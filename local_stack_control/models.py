"""Typed data models for local Compose lifecycle control."""

import dataclasses
import enum
import pathlib
import re


COMPOSE_PROJECT_LABELS = (
	"io.podman.compose.project",
	"com.docker.compose.project",
)
COMPOSE_SERVICE_LABELS = (
	"io.podman.compose.service",
	"com.docker.compose.service",
)
DISPOSABLE_CAPABILITY_LABEL = "org.peptidyle.disposable.capability-sha256"
DISPOSABLE_CAPABILITY_SETTING = "PLE_DISPOSABLE_CAPABILITY_SHA256"
E2E_OWNER_LABEL = "org.peptidyle.e2e.owner"
LIVE_DEMO_BROWSER_OWNER = "live-demo-browser"

DEFAULT_PROJECT = "containers"
LIVE_DEMO_BROWSER_PROJECT = "ple-live-demo-browser"
LIVE_DEMO_REPLICA_APPLICATION_IMAGE = (
	"localhost/peptidyle-learning-engine:ple-live-demo-browser"
)
DEFAULT_ENV_FILE = "containers/env.local"
PRIMARY_COMPOSE_FILE = "containers/compose.yaml"
DISPOSABLE_COMPOSE_PROVIDER = "podman-compose"
DISPOSABLE_PROVIDER_GLOBAL_ARGS = ("--in-pod", "false")

BASE_LONG_RUNNING_SERVICES = (
	"postgres",
	"minio",
	"webwork-renderer",
	"api",
	"gateway",
)
BASE_ONE_SHOT_SERVICES = (
	"local-data-volume-permissions",
	"createbuckets",
	"identity-secret-init",
)
CLEANUP_ONLY_SERVICES = ("postgres-major-guard",)
RESTARTABLE_SERVICES = ("api", "gateway", "webwork-renderer")
STOPPABLE_SERVICES = ("webwork-renderer",)


class ControllerError(RuntimeError):
	"""A concise operator-facing controller failure."""


class LiveDemoProfile(enum.StrEnum):
	"""Closed fixed-stack topology selected by a lifecycle owner."""

	BROWSER = "browser"
	WEBWORK_RENDER_RPC = "webwork_render_rpc"
	REPLICA_RESTART = "replica_restart"
	DATABASE_BASELINE = "database_baseline"
	COURSE_APPEARANCE_CROSS_STORE = "course_appearance_cross_store"


@dataclasses.dataclass(frozen=True)
class LiveDemoProfilePolicy:
	"""Exact topology and bounded child authority for one fixed-stack profile."""

	profile: LiveDemoProfile
	compose_relative_paths: tuple[str, ...]
	child_capabilities: tuple[str, ...]
	evidence_log_services: tuple[tuple[str, str], ...] = ()
	outage_service: str | None = None
	stoppable_service: str | None = None
	diagnostic_services: tuple[str, ...] = ()
	application_image: str | None = None
	service_replica_counts: tuple[tuple[str, int], ...] = ()


LIVE_DEMO_PROFILE_POLICIES = (
	LiveDemoProfilePolicy(
		profile=LiveDemoProfile.BROWSER,
		compose_relative_paths=(
			PRIMARY_COMPOSE_FILE,
			"tests/e2e/compose.live-demo-browser.yaml",
		),
		child_capabilities=("canonical_browser_lifecycle",),
		evidence_log_services=(("renderer_delivery", "api"),),
		outage_service="gateway",
	),
	LiveDemoProfilePolicy(
		profile=LiveDemoProfile.WEBWORK_RENDER_RPC,
		compose_relative_paths=(
			PRIMARY_COMPOSE_FILE,
			"tests/e2e/compose.live-demo-browser.yaml",
		),
		child_capabilities=("bounded_renderer_log", "webwork_service_client"),
		evidence_log_services=(("renderer_delivery", "api"),),
		outage_service="webwork-renderer",
	),
	LiveDemoProfilePolicy(
		profile=LiveDemoProfile.REPLICA_RESTART,
		compose_relative_paths=(
			PRIMARY_COMPOSE_FILE,
			"tests/e2e/compose.live-demo-browser.yaml",
			"tests/e2e/compose.replica-e2e.yaml",
		),
		child_capabilities=(
			"bounded_replica_restart",
			"postgresql_count",
			"replica_service_client",
		),
		stoppable_service="api",
		diagnostic_services=("api", "gateway"),
		application_image=LIVE_DEMO_REPLICA_APPLICATION_IMAGE,
		service_replica_counts=(("api", 2),),
	),
	LiveDemoProfilePolicy(
		profile=LiveDemoProfile.DATABASE_BASELINE,
		compose_relative_paths=("tests/e2e/compose.database-baseline.yaml",),
		child_capabilities=("database_baseline_oracle",),
	),
	LiveDemoProfilePolicy(
		profile=LiveDemoProfile.COURSE_APPEARANCE_CROSS_STORE,
		compose_relative_paths=(
			"tests/e2e/compose.database-baseline.yaml",
			"tests/e2e/compose.course-appearance-cross-store.yaml",
		),
		child_capabilities=("course_appearance_cross_store_oracle",),
	),
)


#============================================
def live_demo_profile(value: str) -> LiveDemoProfile:
	"""Parse one exact fixed-stack profile name at the manifest boundary."""
	try:
		profile = LiveDemoProfile(value)
	except ValueError as error:
		raise ControllerError("live-demo target does not declare a supported profile") from error
	return profile


#============================================
def live_demo_profile_policy(profile: LiveDemoProfile) -> LiveDemoProfilePolicy:
	"""Return the exact topology and capabilities for one closed profile."""
	if not isinstance(profile, LiveDemoProfile):
		raise ControllerError("live-demo target does not declare a supported profile")
	for policy in LIVE_DEMO_PROFILE_POLICIES:
		if policy.profile is profile:
			return policy
	raise ControllerError("live-demo target does not declare a supported profile")

DECLARED_BASE_VOLUMES = (
	"ple_pgdata",
	"ple_miniodata",
	"ple_identity_runtime",
)
DECLARED_SMTP_VOLUMES = ("ple_smtp_runtime",)

# Compose assigns the project prefix to each declared network.  Keep this
# topology beside the declared persistent volumes so cleanup can prove that a
# labelled network belongs to the selected normal stack before mutating it.
DECLARED_BASE_NETWORKS = (
	"default",
	"gateway_api",
	"renderer_private",
	"api_outbound",
)


#============================================
def restartable_services() -> tuple[str, ...]:
	"""Return the stateless services authorized by the selected topology."""
	return RESTARTABLE_SERVICES


@dataclasses.dataclass(frozen=True)
class DisposableOwnerPolicy:
	"""One closed E2E owner namespace, project grammar, and Compose topology."""

	owner: str
	project_prefix: str
	project_pattern: re.Pattern[str]
	compose_relative_paths: tuple[str, ...]
	removes_gateway_image: bool = False
	allows_generic_compose: bool = True


# Every disposable mutation carries one of these stable, closed identities.
# This is the one registry for an owner's namespace, grammar, and Compose
# topology. Consumers may add application assertions, never target shape.
DISPOSABLE_OWNER_POLICIES = (
	DisposableOwnerPolicy(
		owner="course-appearance",
		project_prefix="ple_course_appearance_",
		project_pattern=re.compile(r"^ple_course_appearance_[A-Za-z0-9]+$"),
		compose_relative_paths=("tests/e2e/compose.course-appearance.yaml",),
	),
	DisposableOwnerPolicy(
		owner="live-demo-baseline",
		project_prefix="ple_live_demo_baseline_",
		project_pattern=re.compile(r"^ple_live_demo_baseline_[A-Za-z0-9]+$"),
		compose_relative_paths=("tests/e2e/compose.course-appearance.yaml",),
	),
	DisposableOwnerPolicy(
		owner="wp-r2-postgres-rls",
		project_prefix="ple_wp_r2_postgres_rls_",
		project_pattern=re.compile(r"^ple_wp_r2_postgres_rls_[A-Za-z0-9]+$"),
		compose_relative_paths=("tests/e2e/compose.database-baseline.yaml",),
	),
	DisposableOwnerPolicy(
		owner="wp-rc8-postgres-outbox",
		project_prefix="ple_wp_rc8_postgres_outbox_",
		project_pattern=re.compile(r"^ple_wp_rc8_postgres_outbox_[A-Za-z0-9]+$"),
		compose_relative_paths=("tests/e2e/compose.database-baseline.yaml",),
	),
	DisposableOwnerPolicy(
		owner=LIVE_DEMO_BROWSER_OWNER,
		project_prefix=LIVE_DEMO_BROWSER_PROJECT,
		project_pattern=re.compile(r"^ple-live-demo-browser$"),
		compose_relative_paths=(
			PRIMARY_COMPOSE_FILE,
			"tests/e2e/compose.live-demo-browser.yaml",
		),
		removes_gateway_image=True,
		allows_generic_compose=False,
	),
)


#============================================
def disposable_owner_policy(owner: str) -> DisposableOwnerPolicy:
	"""Return one supported disposable owner policy by its stable identity."""
	for policy in DISPOSABLE_OWNER_POLICIES:
		if policy.owner == owner:
			return policy
	raise ControllerError("disposable target does not declare a supported owner policy")


@dataclasses.dataclass(frozen=True)
class CommandResult:
	"""Completed subprocess result."""

	argv: tuple[str, ...]
	returncode: int
	stdout: str
	stderr: str

	#============================================
	def ok(self) -> bool:
		"""Return whether the process exited successfully."""
		result = self.returncode == 0
		return result


@dataclasses.dataclass(frozen=True)
class ComposeProvider:
	"""Selected Compose provider command."""

	argv: tuple[str, ...]
	name: str


@dataclasses.dataclass(frozen=True)
class ComposeTarget:
	"""Resolved normal or read-only Compose target."""

	repo_root: pathlib.Path
	project: str
	env_file: pathlib.Path
	compose_files: tuple[pathlib.Path, ...]
	provider: ComposeProvider
	with_smtp: bool
	env_setting_names: tuple[str, ...]


@dataclasses.dataclass(frozen=True)
class DisposableComposeTarget:
	"""Runner-owned disposable target with explicit cleanup authority."""

	target: ComposeTarget
	owner_policy: str
	capability_file: pathlib.Path
	project_prefix: str
	private_environment_file: pathlib.Path
	live_demo_profile: LiveDemoProfile | None = None
	acceptance_runtime_workspace: pathlib.Path | None = None


@dataclasses.dataclass(frozen=True)
class PortBinding:
	"""One host-to-container port binding."""

	host_ip: str
	host_port: int
	container_port: int
	protocol: str


@dataclasses.dataclass(frozen=True)
class ContainerResource:
	"""One labelled Podman container with inspected state."""

	id: str
	names: tuple[str, ...]
	project: str | None
	service: str | None
	state: str
	running: bool
	exit_code: int | None
	health: str | None
	image: str
	ports: tuple[PortBinding, ...]
	capability_digest: str | None = None
	image_id: str | None = None
	owner: str | None = None


@dataclasses.dataclass(frozen=True)
class VolumeResource:
	"""One labelled Podman volume."""

	name: str
	project: str | None
	capability_digest: str | None = None
	owner: str | None = None


@dataclasses.dataclass(frozen=True)
class NetworkResource:
	"""One labelled Podman network."""

	name: str
	project: str | None
	capability_digest: str | None = None
	owner: str | None = None


@dataclasses.dataclass(frozen=True)
class ProjectSnapshot:
	"""Label-derived resources for one Compose project."""

	project: str
	containers: tuple[ContainerResource, ...]
	volumes: tuple[VolumeResource, ...]
	networks: tuple[NetworkResource, ...]


@dataclasses.dataclass(frozen=True)
class ProjectSummary:
	"""Non-secret resource counts for one Compose project."""

	project: str
	containers: int
	running: int
	volumes: int
	networks: int
	state: str


@dataclasses.dataclass(frozen=True)
class StackServiceStatus:
	"""Computed status for one required service."""

	service: str
	instances: int
	present: bool
	running: bool
	healthy: bool
	complete: bool
	state: str
	health: str | None
	exit_code: int | None


@dataclasses.dataclass(frozen=True)
class StatusReport:
	"""Semantic readiness report for a project snapshot."""

	project: str
	with_smtp: bool
	snapshot: ProjectSnapshot
	services: tuple[StackServiceStatus, ...]
	ok: bool
	state: str
	message: str


@dataclasses.dataclass(frozen=True)
class CleanupPlan:
	"""Exact project-scoped cleanup operation."""

	project: str
	snapshot: ProjectSnapshot
	argv: tuple[str, ...]
	removes_volumes: bool
	host_paths_to_remove: tuple[pathlib.Path, ...] = ()


@dataclasses.dataclass(frozen=True)
class ServiceStopPlan:
	"""One narrowly authorized, non-destructive service outage action."""

	project: str
	service: str
	argv: tuple[str, ...]


@dataclasses.dataclass(frozen=True)
class DeclaredOutageStop:
	"""Completed policy-declared service stop proved against labelled state."""

	project: str
	service: str


@dataclasses.dataclass(frozen=True)
class ConflictPreflight:
	"""Read-only conflict decision for an acceptance owner."""

	conflicting_projects: tuple[str, ...]
	ok: bool


@dataclasses.dataclass(frozen=True)
class DoctorCheck:
	"""One diagnostic line from the doctor command."""

	name: str
	status: str
	detail: str


@dataclasses.dataclass(frozen=True)
class QuestionRendererVersion:
	"""Private exact renderer version selected for one local-stack lifecycle."""

	reference: str
	oci_id: str
