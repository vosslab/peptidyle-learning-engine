"""Closed disposable-Compose adapter shared by selected E2E owners."""

import dataclasses
import os
import pathlib
import re
import stat

import local_stack_control.cleanup
import local_stack_control.compose
import local_stack_control.discovery
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process


MANIFEST_KEYS = ("OWNER", "PROJECT", "ENV_FILE", "CAPABILITY_FILE")
CONTAINER_ID_PREFIX_PATTERN = re.compile(r"^[a-f0-9]{12}$")
EVIDENCE_LOG_TAIL_LINES = 5_000
EVIDENCE_LOG_MAX_CHARACTERS = 1_000_000
CANONICAL_IMAGE_SELECTIONS_BY_OWNER = {
	"course-appearance": (
		"PLE_POSTGRES_IMAGE_SHA256",
		"PLE_MINIO_IMAGE_SHA256",
		"PLE_MINIO_MC_IMAGE_SHA256",
	),
	"chapter-one-pilot": (
		"PLE_POSTGRES_IMAGE_SHA256",
		"PLE_MINIO_IMAGE_SHA256",
		"PLE_MINIO_MC_IMAGE_SHA256",
	),
	"database-baseline": ("PLE_POSTGRES_IMAGE_SHA256",),
	"wp-r2-postgres-rls": ("PLE_POSTGRES_IMAGE_SHA256",),
	"wp-rc8-postgres-outbox": ("PLE_POSTGRES_IMAGE_SHA256",),
	"chapter-one-browser": (),
	"webwork-browser": (),
	"wp-r2-host-seed-renderer": (),
	"replica-restart": (),
}


@dataclasses.dataclass(frozen=True)
class DisposableManifest:
	"""Non-secret runner evidence needed to form a disposable target."""

	owner: str
	project: str
	env_file: pathlib.Path
	capability_file: pathlib.Path


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
		if name not in MANIFEST_KEYS or value == "" or name in values:
			raise local_stack_control.models.ControllerError(
				f"disposable target manifest:{line_number} is not an allowed declaration"
			)
		values[name] = value
	if tuple(sorted(values)) != tuple(sorted(MANIFEST_KEYS)):
		raise local_stack_control.models.ControllerError(
			"disposable target manifest must declare its complete ownership evidence"
		)
	return values


#============================================
def load_manifest(repo_root: pathlib.Path, manifest_path: pathlib.Path) -> DisposableManifest:
	"""Load and normalize one runner-owned non-secret target manifest."""
	values = manifest_values(manifest_path)
	env_path = pathlib.Path(values["ENV_FILE"])
	if not env_path.is_absolute():
		env_path = repo_root / env_path
	capability_path = pathlib.Path(values["CAPABILITY_FILE"])
	if not capability_path.is_absolute():
		capability_path = repo_root / capability_path
	manifest = DisposableManifest(
		owner=values["OWNER"],
		project=values["PROJECT"],
		env_file=env_path.absolute(),
		capability_file=capability_path.absolute(),
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
	declared_names = local_stack_control.env_file.add_canonical_selections(
		repo_root,
		manifest.env_file,
		CANONICAL_IMAGE_SELECTIONS_BY_OWNER[policy.owner],
	)
	compose_files = local_stack_control.compose.disposable_policy_compose_files(
		repo_root, policy.owner
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
def lifecycle_options(
	disposable: local_stack_control.models.DisposableComposeTarget,
	timeout_seconds: int,
) -> "local_stack_control.lifecycle.LifecycleOptions":
	"""Form the closed lifecycle request allowed to full-stack disposable owners."""
	policy = disposable_policy(disposable)
	launch_owners = {"chapter-one-browser", "webwork-browser", "wp-r2-host-seed-renderer"}
	if policy.owner not in launch_owners:
		raise local_stack_control.models.ControllerError(
			"closed full-stack owners may use the structured launcher"
		)
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
	if policy.outage_service is None:
		raise local_stack_control.models.ControllerError(
			"this disposable owner cannot create a service outage"
		)
	return policy.outage_service


#============================================
def outage_stop_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> tuple[list[str], dict[str, str]]:
	"""Form the one policy-declared outage command outside generic Compose access."""
	service = outage_service(disposable)
	argv = local_stack_control.compose.compose_argv(disposable.target, ["stop", service])
	return argv, compose_environment(disposable)


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
	project = disposable.target.project
	images: list[str] = []
	if policy.removes_application_image:
		images.append(f"localhost/peptidyle-learning-engine:{project}")
	if policy.removes_gateway_image:
		images.append(f"localhost/{project}_gateway:latest")
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
	if policy.stoppable_service is None or service != policy.stoppable_service:
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
	"""Form one safe generic Compose invocation for a proven target."""
	if not disposable_policy(disposable).allows_generic_compose:
		raise local_stack_control.models.ControllerError(
			"this disposable owner cannot use generic Compose commands"
		)
	require_safe_compose_arguments(arguments)
	argv = local_stack_control.compose.compose_argv(disposable.target, arguments)
	environment = compose_environment(disposable)
	return argv, environment


#============================================
def evidence_log_command(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> tuple[list[str], dict[str, str]]:
	"""Form the one bounded evidence-log read declared by an owner policy."""
	service = disposable_policy(disposable).evidence_log_service
	if service is None:
		raise local_stack_control.models.ControllerError(
			"this disposable owner cannot read application evidence logs"
		)
	arguments = [
		"logs",
		"--no-color",
		"--tail",
		str(EVIDENCE_LOG_TAIL_LINES),
		service,
	]
	argv = local_stack_control.compose.compose_argv(disposable.target, arguments)
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
	if policy.owner != "replica-restart":
		raise local_stack_control.models.ControllerError(
			"diagnostics are not available for this disposable owner"
		)
	allowed = {"api", "gateway"}
	if len(services) == 0 or len(set(services)) != len(services) or not set(services) <= allowed:
		raise local_stack_control.models.ControllerError(
			"replica diagnostics require unique api or gateway services"
		)
	return services


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
