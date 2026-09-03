"""Closed disposable-Compose adapter shared by selected E2E owners."""

import dataclasses
import os
import pathlib
import re
import stat
import uuid

import local_stack_control.cleanup
import local_stack_control.browser_suite_ownership
import local_stack_control.compose
import local_stack_control.discovery
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process
import local_stack_control.runtime_manifest


MANIFEST_KEYS = ("OWNER", "PROJECT", "ENV_FILE", "CAPABILITY_FILE")
LIVE_DEMO_MANIFEST_KEYS = (*MANIFEST_KEYS, "PROFILE")
CONTAINER_ID_PREFIX_PATTERN = re.compile(r"^[a-f0-9]{12}$")
POSTGRESQL_COUNT_FIELDS = (
	"question_attempt",
	"submission",
	"submission_idempotency",
	"submission_evaluation",
	"attempt_score_current",
)
EVIDENCE_LOG_TAIL_LINES = 5_000
EVIDENCE_LOG_MAX_CHARACTERS = 1_000_000
TRACKED_IMAGE_SELECTIONS_BY_OWNER = {
	"course-appearance": (
		"PLE_POSTGRES_IMAGE_SHA256",
		"PLE_MINIO_IMAGE_SHA256",
		"PLE_MINIO_MC_IMAGE_SHA256",
	),
	"live-demo-baseline": (
		"PLE_POSTGRES_IMAGE_SHA256",
		"PLE_MINIO_IMAGE_SHA256",
		"PLE_MINIO_MC_IMAGE_SHA256",
	),
	local_stack_control.models.LIVE_DEMO_BROWSER_OWNER: ("PLE_POSTGRES_IMAGE_SHA256",),
}


#============================================
def tracked_image_selections(manifest: "DisposableManifest") -> tuple[str, ...]:
	"""Return the tracked image selections allowed by the selected profile."""
	if manifest.owner != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		return TRACKED_IMAGE_SELECTIONS_BY_OWNER[manifest.owner]
	if manifest.live_demo_profile is local_stack_control.models.LiveDemoProfile.DATABASE_BASELINE:
		return ("PLE_POSTGRES_IMAGE_SHA256",)
	if manifest.live_demo_profile is local_stack_control.models.LiveDemoProfile.COURSE_APPEARANCE_CROSS_STORE:
		return (
			"PLE_POSTGRES_IMAGE_SHA256",
			"PLE_MINIO_IMAGE_SHA256",
			"PLE_MINIO_MC_IMAGE_SHA256",
		)
	return TRACKED_IMAGE_SELECTIONS_BY_OWNER[manifest.owner]


@dataclasses.dataclass(frozen=True)
class DisposableManifest:
	"""Non-secret runner evidence needed to form a disposable target."""

	owner: str
	project: str
	env_file: pathlib.Path
	capability_file: pathlib.Path
	live_demo_profile: local_stack_control.models.LiveDemoProfile | None = None
	acceptance_runtime_workspace: pathlib.Path | None = None


#============================================
def require_private_regular_file(path: pathlib.Path, description: str) -> None:
	"""Require a current-user private regular file without revealing contents."""
	if path.is_symlink() or not path.is_file():
		raise local_stack_control.models.ControllerError(
			f"{description} must be a regular file"
		)
	file_stat = path.stat()
	if file_stat.st_uid != os.getuid():
		raise local_stack_control.models.ControllerError(
			f"{description} must be owned by the current user"
		)
	if stat.S_IMODE(file_stat.st_mode) != 0o600:
		raise local_stack_control.models.ControllerError(
			f"{description} must have mode 0600"
		)


#============================================
def owner_policy(owner: str) -> local_stack_control.models.DisposableOwnerPolicy:
	"""Return the one closed policy allowed to use an adapter action."""
	policy = local_stack_control.models.disposable_owner_policy(owner)
	return policy


#============================================
def manifest_values(manifest_path: pathlib.Path) -> dict[str, str]:
	"""Read a fixed non-secret manifest format after private-file validation."""
	require_private_regular_file(manifest_path, "disposable target manifest")
	values: dict[str, str] = {}
	for line_number, line in enumerate(manifest_path.read_text(encoding="ascii").splitlines(), start=1):
		if "=" not in line:
			raise local_stack_control.models.ControllerError(
				f"disposable target manifest:{line_number} is not NAME=value"
			)
		name, value = line.split("=", 1)
		if name not in LIVE_DEMO_MANIFEST_KEYS or value == "" or name in values:
			raise local_stack_control.models.ControllerError(
				f"disposable target manifest:{line_number} is not an allowed declaration"
			)
		values[name] = value
	expected_keys = (
		LIVE_DEMO_MANIFEST_KEYS
		if values.get("OWNER") == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER
		else MANIFEST_KEYS
	)
	if tuple(sorted(values)) != tuple(sorted(expected_keys)):
		raise local_stack_control.models.ControllerError(
			"disposable target manifest must declare its complete ownership evidence"
		)
	return values


#============================================
def load_manifest(repo_root: pathlib.Path, manifest_path: pathlib.Path) -> DisposableManifest:
	"""Load and normalize one runner-owned non-secret target manifest."""
	manifest_path = manifest_path.absolute()
	if manifest_path.name == local_stack_control.runtime_manifest.MANIFEST_NAME:
		profile = local_stack_control.runtime_manifest.acceptance_runtime_profile(manifest_path.parent)
		if profile is local_stack_control.models.LiveDemoProfile.DATABASE_BASELINE:
			runtime = local_stack_control.runtime_manifest.load_database_baseline_runtime(manifest_path.parent)
		elif profile is local_stack_control.models.LiveDemoProfile.COURSE_APPEARANCE_CROSS_STORE:
			runtime = local_stack_control.runtime_manifest.load_course_appearance_cross_store_runtime(manifest_path.parent)
		else:
			raise local_stack_control.models.ControllerError("acceptance runtime profile is invalid")
		return DisposableManifest(
			owner=local_stack_control.models.LIVE_DEMO_BROWSER_OWNER,
			project=local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT,
			env_file=runtime.compose_environment_path,
			capability_file=runtime.cleanup_capability_path,
			live_demo_profile=profile,
			acceptance_runtime_workspace=runtime.workspace,
		)
	values = manifest_values(manifest_path)
	env_path = pathlib.Path(values["ENV_FILE"])
	if not env_path.is_absolute():
		env_path = repo_root / env_path
	capability_path = pathlib.Path(values["CAPABILITY_FILE"])
	if not capability_path.is_absolute():
		capability_path = repo_root / capability_path
	profile = None
	if values["OWNER"] == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		profile = local_stack_control.models.live_demo_profile(values["PROFILE"])
	manifest = DisposableManifest(
		owner=values["OWNER"],
		project=values["PROJECT"],
		env_file=env_path.absolute(),
		capability_file=capability_path.absolute(),
		live_demo_profile=profile,
	)
	return manifest


#============================================
def disposable_target(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	manifest: DisposableManifest,
) -> local_stack_control.models.DisposableComposeTarget:
	"""Build a provider-backed target only under the manifest's closed policy."""
	policy = owner_policy(manifest.owner)
	local_stack_control.compose.require_disposable_capability_file(manifest.capability_file)
	local_stack_control.env_file.require_mutation_env_file(manifest.env_file)
	declared_names = local_stack_control.env_file.add_tracked_selections(
		repo_root, manifest.env_file, tracked_image_selections(manifest)
	)
	compose_files = local_stack_control.compose.disposable_policy_compose_files(
		repo_root, policy.owner, manifest.live_demo_profile
	)
	provider = local_stack_control.compose.choose_provider(
		runner,
		repo_root,
		local_stack_control.models.DISPOSABLE_COMPOSE_PROVIDER,
	)
	target = local_stack_control.models.ComposeTarget(
		repo_root=repo_root,
		project=manifest.project,
		env_file=manifest.env_file,
		compose_files=compose_files,
		provider=provider,
		with_smtp=False,
		env_setting_names=declared_names,
	)
	result = local_stack_control.compose.new_disposable_target(
		target,
		manifest.capability_file,
		policy.owner,
		manifest.live_demo_profile,
	)
	if manifest.acceptance_runtime_workspace is not None:
		result = dataclasses.replace(
			result,
			acceptance_runtime_workspace=manifest.acceptance_runtime_workspace,
		)
	return result


#============================================
def require_safe_compose_arguments(arguments: list[str]) -> None:
	"""Keep target/provider/environment authority in the typed adapter."""
	for argument in arguments:
		if argument in (
			"-p",
			"--project-name",
			"-f",
			"--file",
			"--env-file",
			"--project-directory",
		):
			raise local_stack_control.models.ControllerError(
				"Compose target, files, environment, and directory are fixed by the disposable manifest"
			)
		if argument.startswith((
			"--project-name=",
			"--file=",
			"--env-file=",
			"--project-directory=",
			"-p",
			"-f",
		)):
			raise local_stack_control.models.ControllerError(
				"Compose target, files, environment, and directory are fixed by the disposable manifest"
			)
		if argument in ("down", "rm", "stop", "kill"):
			raise local_stack_control.models.ControllerError(
				"disposable removal must use the adapter cleanup action"
			)


#============================================
def compose_environment(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> dict[str, str]:
	"""Return the target-controlled process environment for Compose."""
	environment = local_stack_control.process.current_environment()
	result = local_stack_control.compose.target_environment(disposable.target, environment)
	for name in tuple(result):
		if name.startswith("PLE_DISPOSABLE_CAPABILITY_"):
			result.pop(name)
	return result


#============================================
def disposable_policy(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.DisposableOwnerPolicy:
	"""Recover the closed owner policy from typed disposable metadata."""
	result = local_stack_control.models.disposable_owner_policy(disposable.owner_policy)
	return result


#============================================
def live_demo_profile_policy(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.LiveDemoProfilePolicy:
	"""Require and return the fixed owner's selected closed profile policy."""
	if disposable.owner_policy != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		raise local_stack_control.models.ControllerError(
			"this disposable owner does not use a live-demo profile"
		)
	if disposable.live_demo_profile is None:
		raise local_stack_control.models.ControllerError(
			"live-demo target must declare its closed profile"
		)
	return local_stack_control.models.live_demo_profile_policy(disposable.live_demo_profile)


#============================================
def lifecycle_options(
	disposable: local_stack_control.models.DisposableComposeTarget,
	timeout_seconds: int,
) -> "local_stack_control.lifecycle.LifecycleOptions":
	"""Form the closed lifecycle request allowed to full-stack disposable owners."""
	policy = disposable_policy(disposable)
	if policy.owner != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		raise local_stack_control.models.ControllerError(
			"closed full-stack owners may use the structured launcher"
		)
	live_demo_profile_policy(disposable)
	if timeout_seconds < 1 or timeout_seconds > 600:
		raise local_stack_control.models.ControllerError(
			"disposable launcher timeout must be between 1 and 600 seconds"
		)
	from local_stack_control import lifecycle
	return lifecycle.LifecycleOptions(
		float(timeout_seconds), True, False, False
	)


#============================================
def restart_options(
	disposable: local_stack_control.models.DisposableComposeTarget,
	timeout_seconds: int,
) -> "local_stack_control.lifecycle.LifecycleOptions":
	"""Form the no-build stateless-restart request for a declared outage owner."""
	outage_service(disposable)
	if timeout_seconds < 1 or timeout_seconds > 600:
		raise local_stack_control.models.ControllerError(
			"disposable restart timeout must be between 1 and 600 seconds"
		)
	from local_stack_control import lifecycle
	return lifecycle.LifecycleOptions(float(timeout_seconds), False, False, False)


#============================================
def outage_service(disposable: local_stack_control.models.DisposableComposeTarget) -> str:
	"""Return the one deliberate outage service owned by this browser fixture."""
	policy = disposable_policy(disposable)
	if policy.owner != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		raise local_stack_control.models.ControllerError(
			"this disposable owner cannot create a service outage"
		)
	service = live_demo_profile_policy(disposable).outage_service
	if service is None:
		raise local_stack_control.models.ControllerError(
			"this disposable owner cannot create a service outage"
		)
	return service


#============================================
def outage_stop_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> tuple[list[str], dict[str, str]]:
	"""Form the one policy-declared outage command outside generic Compose access."""
	service = outage_service(disposable)
	argv = local_stack_control.compose.compose_argv(disposable.target, ["stop", service])
	return argv, compose_environment(disposable)


#============================================
def require_declared_outage_snapshot(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Require a complete snapshot to describe only the selected labelled project."""
	project = disposable.target.project
	if snapshot.project != project:
		raise local_stack_control.models.ControllerError(
			"declared outage snapshot does not match its selected project"
		)
	resources = (*snapshot.containers, *snapshot.volumes, *snapshot.networks)
	if any(resource.project != project for resource in resources):
		raise local_stack_control.models.ControllerError(
			"declared outage snapshot contains a foreign or malformed resource"
		)
	if disposable.owner_policy == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		if (
			disposable.target.project != local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
			or disposable.project_prefix != local_stack_control.models.LIVE_DEMO_BROWSER_PROJECT
		):
			raise local_stack_control.models.ControllerError(
				"live-demo browser outage has an invalid fixed project selection"
			)
		local_stack_control.browser_suite_ownership.require_live_demo_browser_ownership(snapshot)


#============================================
def declared_outage_stop_plan(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> local_stack_control.models.ServiceStopPlan:
	"""Select exactly one running policy-declared service from one owned snapshot."""
	require_declared_outage_snapshot(disposable, snapshot)
	service = outage_service(disposable)
	selected = tuple(container for container in snapshot.containers if container.service == service)
	if len(selected) != 1 or not selected[0].running:
		raise local_stack_control.models.ControllerError(
			"declared outage requires exactly one running labelled service instance"
		)
	argv, _environment = outage_stop_command(disposable)
	return local_stack_control.models.ServiceStopPlan(
		project=disposable.target.project,
		service=service,
		argv=tuple(argv),
	)


#============================================
def persistent_scope(
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> tuple[tuple[str, ...], tuple[str, ...]]:
	"""Return order-independent exact-project persistent resource identities."""
	result = (
		tuple(sorted(resource.name for resource in snapshot.volumes)),
		tuple(sorted(resource.name for resource in snapshot.networks)),
	)
	return result


#============================================
def unrelated_containers(
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
) -> tuple[local_stack_control.models.ContainerResource, ...]:
	"""Return the complete unchanged-container comparison scope outside one service."""
	result = tuple(sorted(
		(container for container in snapshot.containers if container.service != service),
		key=lambda container: container.id,
	))
	return result


#============================================
def require_declared_outage_stopped(
	disposable: local_stack_control.models.DisposableComposeTarget,
	before: local_stack_control.models.ProjectSnapshot,
	after: local_stack_control.models.ProjectSnapshot,
	plan: local_stack_control.models.ServiceStopPlan,
) -> None:
	"""Prove only the selected declared service changed from running to stopped."""
	require_declared_outage_snapshot(disposable, before)
	require_declared_outage_snapshot(disposable, after)
	if plan.project != disposable.target.project or plan.service != outage_service(disposable):
		raise local_stack_control.models.ControllerError(
			"declared outage plan does not match its selected policy"
		)
	expected_argv, _environment = outage_stop_command(disposable)
	if plan.argv != tuple(expected_argv):
		raise local_stack_control.models.ControllerError(
			"declared outage plan does not match its closed command"
		)
	if persistent_scope(before) != persistent_scope(after):
		raise local_stack_control.models.ControllerError(
			"declared outage changed labelled persistent resource scope"
		)
	if unrelated_containers(before, plan.service) != unrelated_containers(after, plan.service):
		raise local_stack_control.models.ControllerError(
			"declared outage changed an unrelated labelled container"
		)
	stopped = tuple(
		container for container in after.containers if container.service == plan.service
	)
	selected_before = tuple(
		container for container in before.containers if container.service == plan.service
	)
	if len(selected_before) != 1 or not selected_before[0].running:
		raise local_stack_control.models.ControllerError(
			"declared outage preselection is not exactly one running labelled service"
		)
	if len(stopped) != 1 or stopped[0].running or stopped[0].id != selected_before[0].id:
		raise local_stack_control.models.ControllerError(
			"declared outage did not leave exactly one labelled service instance stopped"
		)


#============================================
def stop_declared_outage_service(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.DeclaredOutageStop:
	"""Stop one policy-declared service and prove its exact labelled postcondition."""
	before = require_current_resource_capability(runner, disposable)
	plan = declared_outage_stop_plan(disposable, before)
	environment = compose_environment(disposable)
	result = runner.stream(list(plan.argv), environment, disposable.target.repo_root)
	if result != 0:
		raise local_stack_control.models.ControllerError("declared outage stop command failed")
	after = require_current_resource_capability(runner, disposable)
	require_declared_outage_stopped(disposable, before, after, plan)
	completed = local_stack_control.models.DeclaredOutageStop(plan.project, plan.service)
	return completed


#============================================
def owned_project_images(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> tuple[str, ...]:
	"""Return policy-derived image *tags* removable after an empty resource proof.

	The closed policy owns these generated tags, not the underlying image ID.  A
	plain ``podman image rm <tag>`` consequently leaves a shared image and any
	default-project tag intact.  Callers must prove that the labelled resource
	snapshot is empty before using this result; image cleanup never substitutes
	for Compose cleanup.
	"""
	policy = disposable_policy(disposable)
	images: list[str] = []
	application_image: str | None = None
	if policy.owner == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		application_image = live_demo_profile_policy(
			disposable
		).application_image
	if application_image is not None:
		images.append(application_image)
	if policy.removes_gateway_image:
		images.append(f"localhost/{disposable.target.project}_gateway:latest")
	return tuple(images)


#============================================
def require_empty_post_cleanup_snapshot(
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Fail closed when Compose cleanup leaves any labelled resource behind."""
	remaining = len(snapshot.containers) + len(snapshot.volumes) + len(snapshot.networks)
	if remaining != 0:
		raise local_stack_control.models.ControllerError(
			"disposable cleanup left labelled resources; retained the owned image"
		)


#============================================
def replica_stop_container(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
	service: str,
	id_prefix: str,
) -> local_stack_control.models.ContainerResource:
	"""Resolve one running replica strictly within its typed labelled project."""
	policy = disposable_policy(disposable)
	stoppable_service = None
	if policy.owner == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		stoppable_service = live_demo_profile_policy(disposable).stoppable_service
	if stoppable_service is None or service != stoppable_service:
		raise local_stack_control.models.ControllerError(
			"this disposable owner cannot stop the requested service"
		)
	if CONTAINER_ID_PREFIX_PATTERN.fullmatch(id_prefix) is None:
		raise local_stack_control.models.ControllerError(
			"replica container prefix must be twelve lowercase hexadecimal characters"
		)
	if snapshot.project != disposable.target.project:
		raise local_stack_control.models.ControllerError(
			"replica snapshot does not match its owned project"
		)
	running = tuple(
		container
		for container in snapshot.containers
		if container.service == service and container.running
	)
	matches = tuple(container for container in running if container.id.startswith(id_prefix))
	if len(running) < 2 or len(matches) != 1:
		raise local_stack_control.models.ControllerError(
			"replica stop requires one matching instance within at least two running API replicas"
		)
	return matches[0]


#============================================
def require_lowercase_uuid(value: str, label: str) -> str:
	"""Require a lowercase UUID before forming a PostgreSQL variable."""
	# ASVS 1.2.4 and 2.2.1: only typed UUID values may reach the fixed SQL statement.
	if not isinstance(value, str):
		raise local_stack_control.models.ControllerError(f"{label} must be a lowercase UUID")
	try:
		parsed = uuid.UUID(value)
	except ValueError as error:
		raise local_stack_control.models.ControllerError(
			f"{label} must be a lowercase UUID"
		) from error
	if str(parsed) != value:
		raise local_stack_control.models.ControllerError(f"{label} must be a lowercase UUID")
	return value


#============================================
def postgresql_count_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
	attempt_id: str,
) -> tuple[list[str], dict[str, str], str]:
	"""Form the replica profile's one fixed scoped durability-count query."""
	# ASVS 8.2.2: this adapter exposes only the one oracle-scoped data projection.
	if disposable.owner_policy != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		raise local_stack_control.models.ControllerError(
			"PostgreSQL count is limited to the fixed replica profile"
		)
	profile = live_demo_profile_policy(disposable)
	if (
		profile.profile is not local_stack_control.models.LiveDemoProfile.REPLICA_RESTART
		or "postgresql_count" not in profile.child_capabilities
	):
		raise local_stack_control.models.ControllerError(
			"PostgreSQL count is limited to the fixed replica profile"
		)
	attempt = require_lowercase_uuid(attempt_id, "PostgreSQL count attempt")
	values = local_stack_control.env_file.env_settings(disposable.target.env_file)
	postgres_user = values.get("POSTGRES_USER")
	postgres_database = values.get("POSTGRES_DB")
	if not postgres_user or not postgres_database:
		raise local_stack_control.models.ControllerError(
			"PostgreSQL count target omits its database selection"
		)
	sql = "SELECT " + ",".join(
		f"(SELECT count(*) FROM {table} "
		"WHERE attempt_id = :'attempt_id'::uuid)"
		for table in POSTGRESQL_COUNT_FIELDS
	) + ";\n"
	argv = local_stack_control.compose.compose_argv(
		disposable.target,
		[
			"exec",
			"-T",
			"postgres",
			"psql",
			"-v",
			"ON_ERROR_STOP=1",
			"-v",
			f"attempt_id={attempt}",
			"-U",
			postgres_user,
			"-d",
			postgres_database,
			"-tA",
			"-F",
			"|",
		],
	)
	return argv, compose_environment(disposable), sql


#============================================
def require_replica_stopped(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
	container_id: str,
) -> None:
	"""Prove the selected replica stopped while another API remains available."""
	if snapshot.project != disposable.target.project:
		raise local_stack_control.models.ControllerError(
			"replica post-stop snapshot does not match its owned project"
		)
	selected = tuple(container for container in snapshot.containers if container.id == container_id)
	running_peers = tuple(
		container
		for container in snapshot.containers
		if container.service == "api" and container.running and container.id != container_id
	)
	if len(selected) != 1 or selected[0].running or len(running_peers) < 1:
		raise local_stack_control.models.ControllerError(
			"replica stop did not leave the selected API stopped with a running peer"
		)


#============================================
def compose_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
	arguments: list[str],
) -> tuple[list[str], dict[str, str]]:
	"""Form one policy-authorized Compose invocation for a proven target."""
	policy = disposable_policy(disposable)
	require_safe_compose_arguments(arguments)
	if policy.owner == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		profile = live_demo_profile_policy(disposable)
		if profile.profile is local_stack_control.models.LiveDemoProfile.DATABASE_BASELINE:
			if "database_baseline_oracle" not in profile.child_capabilities:
				raise local_stack_control.models.ControllerError("live-demo profile capability is invalid")
			if arguments != ["up", "-d", "postgres"] and not (
				len(arguments) >= 5 and arguments[:4] == ["exec", "-T", "postgres", "psql"]
			):
				raise local_stack_control.models.ControllerError(
					"database baseline Compose commands are limited to PostgreSQL startup and psql"
				)
			if disposable.acceptance_runtime_workspace is not None:
				local_stack_control.runtime_manifest.require_database_baseline_compose_password(
					disposable.acceptance_runtime_workspace
				)
		elif profile.profile is local_stack_control.models.LiveDemoProfile.COURSE_APPEARANCE_CROSS_STORE:
			if "course_appearance_cross_store_oracle" not in profile.child_capabilities:
				raise local_stack_control.models.ControllerError("live-demo profile capability is invalid")
			is_postgres_ready = arguments[:4] == ["exec", "-T", "postgres", "pg_isready"]
			is_minio_ready = arguments == ["exec", "-T", "minio", "mc", "ready", "local"]
			is_postgres_psql = len(arguments) >= 5 and arguments[:4] == ["exec", "-T", "postgres", "psql"]
			if arguments != ["up", "-d", "postgres", "minio"] and not (
				is_postgres_ready or is_minio_ready or is_postgres_psql or arguments == ["run", "--rm", "createbuckets"]
			):
				raise local_stack_control.models.ControllerError(
					"cross-store Compose commands are limited to startup, readiness, bucket creation, and PostgreSQL psql"
				)
			if disposable.acceptance_runtime_workspace is not None:
				local_stack_control.runtime_manifest.require_course_appearance_cross_store_compose_credentials(
					disposable.acceptance_runtime_workspace
				)
		else:
			raise local_stack_control.models.ControllerError(
				"this fixed live-demo profile cannot use generic Compose commands"
			)
	elif not policy.allows_generic_compose:
		raise local_stack_control.models.ControllerError(
			"this disposable owner cannot use generic Compose commands"
		)
	argv = local_stack_control.compose.compose_argv(disposable.target, arguments)
	environment = compose_environment(disposable)
	return argv, environment


#============================================
def evidence_log_service(
	disposable: local_stack_control.models.DisposableComposeTarget,
	receipt_claim: str,
) -> str:
	"""Resolve one receipt claim to its policy-owned service without a generic selector."""
	if not isinstance(receipt_claim, str):
		raise local_stack_control.models.ControllerError("evidence receipt claim is invalid")
	policy = disposable_policy(disposable)
	mapping: tuple[tuple[str, str], ...] = ()
	if policy.owner == local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		mapping = live_demo_profile_policy(disposable).evidence_log_services
	claims = tuple(item[0] for item in mapping)
	if len(claims) != len(set(claims)):
		raise local_stack_control.models.ControllerError("evidence receipt policy is invalid")
	for claim, service in mapping:
		if claim == receipt_claim:
			return service
	raise local_stack_control.models.ControllerError(
		"this disposable owner cannot read the requested evidence receipt"
	)


def evidence_log_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
	receipt_claim: str,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> tuple[list[str], dict[str, str]]:
	"""Read one claim-selected, label-resolved running container with bounded logs."""
	service = evidence_log_service(disposable, receipt_claim)
	if snapshot.project != disposable.target.project:
		raise local_stack_control.models.ControllerError(
			"evidence-log snapshot does not match its selected project"
		)
	selected = tuple(
		container
		for container in snapshot.containers
		if container.service == service and container.running
	)
	if len(selected) != 1:
		raise local_stack_control.models.ControllerError(
			"evidence receipt requires exactly one running labelled service container"
		)
	argv = ["podman", "logs", "--tail", str(EVIDENCE_LOG_TAIL_LINES), selected[0].id]
	return argv, compose_environment(disposable)


#============================================
def cleanup_plan(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.CleanupPlan:
	"""Discover labels and form the only destructive operation this adapter owns."""
	snapshot = local_stack_control.discovery.discover_snapshot(
		runner,
		disposable.target.repo_root,
		disposable.target.project,
	)
	plan = local_stack_control.cleanup.disposable_cleanup_plan(
		disposable,
		snapshot,
	)
	return plan


#============================================
def require_mutating_capability(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.ProjectSnapshot:
	"""Prove runner capability against all current labels before a mutation."""
	return require_current_resource_capability(runner, disposable)


#============================================
def require_current_resource_capability(
	runner: local_stack_control.process.CommandRunner,
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> local_stack_control.models.ProjectSnapshot:
	"""Prove the manifest capability against every current project resource."""
	snapshot = local_stack_control.discovery.discover_snapshot(
		runner,
		disposable.target.repo_root,
		disposable.target.project,
	)
	local_stack_control.compose.require_disposable_resource_capability(disposable, snapshot)
	return snapshot


#============================================
def require_capability_snapshot(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Require every existing resource to bind the runner capability digest."""
	local_stack_control.compose.require_disposable_resource_capability(disposable, snapshot)


#============================================
def diagnostic_services(
	disposable: local_stack_control.models.DisposableComposeTarget,
	services: tuple[str, ...],
) -> tuple[str, ...]:
	"""Validate the bounded service set exposed by replica diagnostics."""
	policy = disposable_policy(disposable)
	if policy.owner != local_stack_control.models.LIVE_DEMO_BROWSER_OWNER:
		raise local_stack_control.models.ControllerError(
			"diagnostics are not available for this disposable owner"
		)
	allowed = set(live_demo_profile_policy(disposable).diagnostic_services)
	if len(services) == 0 or len(set(services)) != len(services) or not set(services) <= allowed:
		raise local_stack_control.models.ControllerError(
			"replica diagnostics require unique api or gateway services"
		)
	return services


#============================================
def diagnostic_commands(
	disposable: local_stack_control.models.DisposableComposeTarget,
	services: tuple[str, ...],
) -> tuple[list[str], list[str]]:
	"""Form the two bounded replica diagnostic commands without generic authority."""
	selected = diagnostic_services(disposable, services)
	status = local_stack_control.compose.compose_argv(disposable.target, ["ps"])
	logs = local_stack_control.compose.compose_argv(
		disposable.target,
		["logs", "--no-color", "--tail", "80", *selected],
	)
	return status, logs


#============================================
def private_environment_values(env_file: pathlib.Path) -> tuple[str, ...]:
	"""Read only values needed to redact private disposable diagnostics."""
	local_stack_control.env_file.require_mutation_env_file(env_file)
	values: list[str] = []
	for line in env_file.read_text(encoding="utf-8").splitlines():
		stripped = line.strip()
		if stripped == "" or stripped.startswith("#"):
			continue
		value = line.split("=", 1)[1]
		if len(value) >= 4:
			values.append(value)
	return tuple(values)


#============================================
def redact_diagnostics(text: str, private_values: tuple[str, ...]) -> str:
	"""Return bounded diagnostic text without values from the private env file."""
	return redact_private_values(text, private_values)[-4_000:]


#============================================
def redact_evidence_logs(text: str, private_values: tuple[str, ...]) -> str:
	"""Return the bounded evidence log with every private environment value removed."""
	return redact_private_values(text, private_values)[-EVIDENCE_LOG_MAX_CHARACTERS:]


#============================================
def redact_private_values(text: str, private_values: tuple[str, ...]) -> str:
	"""Remove private environment values and credential-bearing PostgreSQL URLs."""
	redacted = text
	for value in sorted(set(private_values), key=len, reverse=True):
		redacted = redacted.replace(value, "[redacted]")
	redacted = re.sub(r"postgres://[^@\s]+@", "postgres://[redacted]@", redacted)
	return redacted
