"""Typed data models for local Compose lifecycle control."""

import dataclasses
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

DEFAULT_PROJECT = "containers"
DEFAULT_ENV_FILE = "containers/env.local"
PRIMARY_COMPOSE_FILE = "containers/compose.yaml"
SMTP_COMPOSE_FILE = "containers/compose.smtp.yaml"
DISPOSABLE_COMPOSE_PROVIDER = "podman-compose"
DISPOSABLE_PROVIDER_GLOBAL_ARGS = ("--in-pod", "false")

BASE_LONG_RUNNING_SERVICES = (
	"postgres",
	"minio",
	"webwork-renderer",
	"api",
	"worker",
	"gateway",
)
BASE_ONE_SHOT_SERVICES = (
	"local-data-volume-permissions",
	"createbuckets",
	"identity-secret-init",
)
SMTP_ONE_SHOT_SERVICES = ("smtp-secret-init",)
CLEANUP_ONLY_SERVICES = ("postgres-major-guard",)
RESTARTABLE_SERVICES = ("api", "worker", "gateway", "webwork-renderer")
STOPPABLE_SERVICES = ("webwork-renderer",)

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


class ControllerError(RuntimeError):
	"""A concise operator-facing controller failure."""


@dataclasses.dataclass(frozen=True)
class DisposableOwnerPolicy:
	"""One closed E2E owner namespace, project grammar, and Compose topology."""

	owner: str
	project_prefix: str
	project_pattern: re.Pattern[str]
	compose_relative_paths: tuple[str, ...]
	removes_gateway_image: bool = False
	removes_application_image: bool = False
	stoppable_service: str | None = None


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
		owner="chapter-one-pilot",
		project_prefix="ple_chapter_one_pilot_",
		project_pattern=re.compile(r"^ple_chapter_one_pilot_[A-Za-z0-9]+$"),
		compose_relative_paths=("tests/e2e/compose.course-appearance.yaml",),
	),
	DisposableOwnerPolicy(
		owner="database-baseline",
		project_prefix="ple_database_baseline_",
		project_pattern=re.compile(r"^ple_database_baseline_[A-Za-z0-9]+$"),
		compose_relative_paths=("tests/e2e/compose.database-baseline.yaml",),
	),
	DisposableOwnerPolicy(
		owner="chapter-one-browser",
		project_prefix="ple-chapter-one-browser-",
		project_pattern=re.compile(r"^ple-chapter-one-browser-[a-f0-9]{12}$"),
		compose_relative_paths=(PRIMARY_COMPOSE_FILE,),
		removes_gateway_image=True,
	),
	DisposableOwnerPolicy(
		owner="replica-restart",
		project_prefix="ple-replica-e2e-",
		project_pattern=re.compile(r"^ple-replica-e2e-[a-f0-9]{10}$"),
		compose_relative_paths=(
			PRIMARY_COMPOSE_FILE,
			"tests/e2e/compose.replica-e2e.yaml",
		),
		removes_gateway_image=True,
		removes_application_image=True,
		stoppable_service="api",
	),
	DisposableOwnerPolicy(
		owner="ui-walkthrough",
		project_prefix="ple-ui-walkthrough-",
		project_pattern=re.compile(r"^ple-ui-walkthrough-[a-f0-9]{16}$"),
		compose_relative_paths=(PRIMARY_COMPOSE_FILE,),
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


@dataclasses.dataclass(frozen=True)
class VolumeResource:
	"""One labelled Podman volume."""

	name: str
	project: str | None
	capability_digest: str | None = None


@dataclasses.dataclass(frozen=True)
class NetworkResource:
	"""One labelled Podman network."""

	name: str
	project: str | None
	capability_digest: str | None = None


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


@dataclasses.dataclass(frozen=True)
class ServiceStopPlan:
	"""One narrowly authorized, non-destructive service outage action."""

	project: str
	service: str
	argv: tuple[str, ...]


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
