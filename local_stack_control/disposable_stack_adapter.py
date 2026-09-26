"""Closed disposable-Compose adapter shared by selected E2E owners."""

import dataclasses
import os
import pathlib
import stat

import local_stack_control.compose
import local_stack_control.disposable_stack_cleanup
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process
import local_stack_control.runtime_manifest
from local_stack_control.readiness_faults import READINESS_DEPENDENCIES
from local_stack_control.worker_lifecycle import (
	replace_worker_service,
	require_worker_replaced,
	require_worker_stopped,
	stop_worker_service,
	worker_replacement_command,
	worker_replacement_plan,
	worker_service,
	worker_stop_command,
	worker_stop_plan,
)


__all__ = (
	"READINESS_DEPENDENCIES",
	"replace_worker_service",
	"require_worker_replaced",
	"require_worker_stopped",
	"stop_worker_service",
	"worker_replacement_command",
	"worker_replacement_plan",
	"worker_service",
	"worker_stop_command",
	"worker_stop_plan",
)


MANIFEST_KEYS = ("OWNER", "PROJECT", "ENV_FILE", "CAPABILITY_FILE")
LIVE_DEMO_MANIFEST_KEYS = (*MANIFEST_KEYS, "PROFILE")

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
	declared_names = local_stack_control.env_file.env_setting_names(manifest.env_file)
	compose_files = local_stack_control.compose.disposable_policy_compose_files(
		repo_root, policy.owner, manifest.live_demo_profile
	)
	provider = local_stack_control.compose.choose_provider(
		runner,
		repo_root,
		True,
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
def require_browser_profile(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> None:
	"""Require the one full teaching profile before an installation-data action."""
	if live_demo_profile_policy(disposable).profile is not local_stack_control.models.LiveDemoProfile.BROWSER:
		raise local_stack_control.models.ControllerError(
			"installation-data acceptance requires the fixed browser profile"
		)


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
		float(timeout_seconds), True, False, False,
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


# Cleanup, outage proof, replica stop, Compose/evidence, and redaction stay on
# this module's public facade so existing callers keep one import path.
cleanup_plan = local_stack_control.disposable_stack_cleanup.cleanup_plan
compose_command = local_stack_control.disposable_stack_cleanup.compose_command
declared_outage_stop_plan = local_stack_control.disposable_stack_cleanup.declared_outage_stop_plan
diagnostic_commands = local_stack_control.disposable_stack_cleanup.diagnostic_commands
diagnostic_services = local_stack_control.disposable_stack_cleanup.diagnostic_services
evidence_log_command = local_stack_control.disposable_stack_cleanup.evidence_log_command
evidence_log_service = local_stack_control.disposable_stack_cleanup.evidence_log_service
owned_project_images = local_stack_control.disposable_stack_cleanup.owned_project_images
persistent_scope = local_stack_control.disposable_stack_cleanup.persistent_scope
postgresql_count_command = local_stack_control.disposable_stack_cleanup.postgresql_count_command
private_environment_values = local_stack_control.disposable_stack_cleanup.private_environment_values
redact_diagnostics = local_stack_control.disposable_stack_cleanup.redact_diagnostics
redact_evidence_logs = local_stack_control.disposable_stack_cleanup.redact_evidence_logs
redact_private_values = local_stack_control.disposable_stack_cleanup.redact_private_values
replica_stop_container = local_stack_control.disposable_stack_cleanup.replica_stop_container
require_capability_snapshot = local_stack_control.disposable_stack_cleanup.require_capability_snapshot
require_current_resource_capability = (
	local_stack_control.disposable_stack_cleanup.require_current_resource_capability
)
require_declared_outage_snapshot = (
	local_stack_control.disposable_stack_cleanup.require_declared_outage_snapshot
)
require_declared_outage_stopped = (
	local_stack_control.disposable_stack_cleanup.require_declared_outage_stopped
)
require_empty_post_cleanup_snapshot = (
	local_stack_control.disposable_stack_cleanup.require_empty_post_cleanup_snapshot
)
require_lowercase_uuid = local_stack_control.disposable_stack_cleanup.require_lowercase_uuid
require_mutating_capability = local_stack_control.disposable_stack_cleanup.require_mutating_capability
require_replica_stopped = local_stack_control.disposable_stack_cleanup.require_replica_stopped
stop_declared_outage_service = local_stack_control.disposable_stack_cleanup.stop_declared_outage_service
unrelated_containers = local_stack_control.disposable_stack_cleanup.unrelated_containers
