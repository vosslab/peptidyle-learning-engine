"""Compose provider, target, ownership, and argv construction."""

import hashlib
import os
import pathlib
import re
import stat

import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process


#============================================
def repo_root_from_entrypoint(entrypoint: pathlib.Path) -> pathlib.Path:
	"""Return the repository root anchored to the executable entry point."""
	resolved_entrypoint = entrypoint.resolve(strict=True)
	root = resolved_entrypoint.parent
	if not (root / ".git").exists():
		raise local_stack_control.models.ControllerError(
			f"{resolved_entrypoint} is not located at the repository root"
		)
	return root


#============================================
def choose_provider(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
) -> local_stack_control.models.ComposeProvider:
	"""Select the first usable Podman Compose provider."""
	environment = local_stack_control.env_file.sanitized_runtime_environment(
		local_stack_control.process.current_environment()
	)
	podman_result = runner.run(
		["podman", "compose", "version"], environment, repo_root
	)
	if podman_result.ok():
		provider = local_stack_control.models.ComposeProvider(
			argv=("podman", "compose"),
			name="podman compose",
		)
		return provider

	legacy_result = runner.run(["podman-compose", "version"], environment, repo_root)
	if legacy_result.ok():
		provider = local_stack_control.models.ComposeProvider(
			argv=("podman-compose",),
			name="podman-compose",
		)
		return provider

	raise local_stack_control.models.ControllerError(
		"neither 'podman compose' nor 'podman-compose' is usable"
	)


#============================================
def compose_files(repo_root: pathlib.Path, with_smtp: bool) -> tuple[pathlib.Path, ...]:
	"""Return explicit Compose files for the selected topology."""
	files = [repo_root / local_stack_control.models.PRIMARY_COMPOSE_FILE]
	if with_smtp:
		files.append(repo_root / local_stack_control.models.SMTP_COMPOSE_FILE)
	result = tuple(files)
	return result


#============================================
def resolve_path(repo_root: pathlib.Path, selected_path: str) -> pathlib.Path:
	"""Resolve an explicit path relative to the repository root."""
	path = pathlib.Path(selected_path)
	if not path.is_absolute():
		path = repo_root / path
	# Keep the final path component unresolved so mutation validation can detect
	# and reject a caller-selected symbolic link.
	result = path.absolute()
	return result


#============================================
def resolve_target(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	env_file: str,
	with_smtp: bool,
	project: str | None = None,
	allow_missing_env: bool = False,
) -> local_stack_control.models.ComposeTarget:
	"""Resolve an explicit target without consulting ambient project state."""
	provider = choose_provider(runner, repo_root)
	selected_project = local_stack_control.models.DEFAULT_PROJECT
	if project is not None:
		selected_project = project
	if selected_project.strip() == "":
		raise local_stack_control.models.ControllerError("Compose project must not be empty")

	selected_env_file = resolve_path(repo_root, env_file)
	env_names = local_stack_control.env_file.env_setting_names(
		selected_env_file,
		allow_missing=allow_missing_env,
	)
	target = local_stack_control.models.ComposeTarget(
		repo_root=repo_root,
		project=selected_project,
		env_file=selected_env_file,
		compose_files=compose_files(repo_root, with_smtp),
		provider=provider,
		with_smtp=with_smtp,
		env_setting_names=env_names,
	)
	return target


#============================================
def require_default_mutation_target(target: local_stack_control.models.ComposeTarget) -> None:
	"""Fail unless a target is the supported private default project."""
	if target.project != local_stack_control.models.DEFAULT_PROJECT:
		raise local_stack_control.models.ControllerError(
			"mutating commands are limited to the containers project"
		)
	local_stack_control.env_file.require_mutation_env_file(target.env_file)
	for compose_file in target.compose_files:
		if not compose_file.is_file():
			raise local_stack_control.models.ControllerError(
				f"{compose_file} does not exist"
			)


#============================================
def disposable_policy_compose_files(
	repo_root: pathlib.Path,
	owner_policy: str,
) -> tuple[pathlib.Path, ...]:
	"""Resolve one declared owner's exact Compose file sequence."""
	policy = local_stack_control.models.disposable_owner_policy(owner_policy)
	files: list[pathlib.Path] = []
	for relative_path in policy.compose_relative_paths:
		path = repo_root / relative_path
		if not path.is_file() or path.is_symlink():
			raise local_stack_control.models.ControllerError(
				"declared disposable Compose file is unavailable"
			)
		files.append(path.resolve(strict=True))
	return tuple(files)


#============================================
def require_disposable_target_policy(
	target: local_stack_control.models.ComposeTarget,
	owner_policy: str,
) -> local_stack_control.models.DisposableOwnerPolicy:
	"""Require a declared owner, project grammar, and exact Compose topology."""
	policy = local_stack_control.models.disposable_owner_policy(owner_policy)
	if policy.project_pattern.fullmatch(target.project) is None:
		raise local_stack_control.models.ControllerError(
			"disposable project does not match its declared owner format"
		)
	if target.compose_files != disposable_policy_compose_files(target.repo_root, owner_policy):
		raise local_stack_control.models.ControllerError(
			"disposable target Compose files do not match its declared owner policy"
		)
	return policy


#============================================
def new_disposable_target(
	target: local_stack_control.models.ComposeTarget,
	capability_file: pathlib.Path,
	owner_policy: str,
) -> local_stack_control.models.DisposableComposeTarget:
	"""Create a typed disposable-owner contract for a private runner."""
	policy = require_disposable_target_policy(target, owner_policy)
	local_stack_control.env_file.require_mutation_env_file(target.env_file)
	require_disposable_capability_file(capability_file)
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy=owner_policy,
		capability_file=capability_file,
		project_prefix=policy.project_prefix,
		private_environment_file=target.env_file,
	)
	return disposable


#============================================
def require_disposable_capability_file(capability_file: pathlib.Path) -> bytes:
	"""Return one exact runner-held 32-byte capability from a private file."""
	if capability_file.is_symlink() or not capability_file.is_file():
		raise local_stack_control.models.ControllerError(
			"disposable capability file must be a regular file"
		)
	metadata = capability_file.stat()
	if metadata.st_uid != os.getuid() or stat.S_IMODE(metadata.st_mode) != 0o600:
		raise local_stack_control.models.ControllerError(
			"disposable capability file must be current-user mode 0600"
		)
	raw = capability_file.read_bytes()
	if len(raw) != 32:
		raise local_stack_control.models.ControllerError(
			"disposable capability file must contain exactly 32 bytes"
		)
	return raw


#============================================
def disposable_capability_digest(capability_file: pathlib.Path) -> str:
	"""Hash a validated raw capability without exposing its material."""
	raw = require_disposable_capability_file(capability_file)
	return hashlib.sha256(raw).hexdigest()


#============================================
def disposable_capability_commitment(env_file: pathlib.Path) -> str:
	"""Read the private environment's non-secret capability commitment."""
	local_stack_control.env_file.require_mutation_env_file(env_file)
	prefix = local_stack_control.models.DISPOSABLE_CAPABILITY_SETTING + "="
	value: str | None = None
	for line in env_file.read_text(encoding="utf-8").splitlines():
		if line.startswith(prefix):
			if value is not None:
				raise local_stack_control.models.ControllerError(
					"disposable environment duplicates its capability commitment"
				)
			value = line[len(prefix):]
	if value is None or re.fullmatch(r"[0-9a-f]{64}", value) is None:
		raise local_stack_control.models.ControllerError(
			"disposable environment must declare a SHA-256 capability commitment"
		)
	return value


#============================================
def require_disposable_ownership(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> None:
	"""Verify runner-held capability commitment before a disposable mutation."""
	policy = require_disposable_target_policy(disposable.target, disposable.owner_policy)
	if disposable.project_prefix != policy.project_prefix:
		raise local_stack_control.models.ControllerError(
			"disposable owner policy does not match its project namespace"
		)
	digest = disposable_capability_digest(disposable.capability_file)
	commitment = disposable_capability_commitment(disposable.private_environment_file)
	if digest != commitment:
		raise local_stack_control.models.ControllerError(
			"disposable capability does not match its private environment commitment"
		)
	if disposable.private_environment_file != disposable.target.env_file:
		raise local_stack_control.models.ControllerError("disposable env ownership does not match")


#============================================
def require_disposable_resource_capability(
	disposable: local_stack_control.models.DisposableComposeTarget,
	snapshot: local_stack_control.models.ProjectSnapshot,
) -> None:
	"""Bind every extant disposable resource to the runner-held capability."""
	require_disposable_ownership(disposable)
	if snapshot.project != disposable.target.project:
		raise local_stack_control.models.ControllerError(
			"disposable capability snapshot does not match its selected project"
		)
	resources = (*snapshot.containers, *snapshot.volumes, *snapshot.networks)
	if len(resources) == 0:
		return
	expected_digest = disposable_capability_digest(disposable.capability_file)
	if any(item.capability_digest != expected_digest for item in resources):
		raise local_stack_control.models.ControllerError(
			"disposable resources do not all carry the runner capability commitment"
		)


#============================================
def compose_argv(
	target: local_stack_control.models.ComposeTarget,
	extra_args: list[str],
) -> list[str]:
	"""Build a self-contained Compose argument vector."""
	argv = list(target.provider.argv)
	argv.extend(["-p", target.project])
	for compose_file in target.compose_files:
		argv.extend(["-f", str(compose_file)])
	argv.extend(["--env-file", str(target.env_file)])
	argv.extend(extra_args)
	return argv


#============================================
def target_environment(
	target: local_stack_control.models.ComposeTarget,
	base_environment: dict[str, str],
) -> dict[str, str]:
	"""Return a child environment controlled by target metadata."""
	environment = local_stack_control.env_file.sanitized_environment(
		base_environment,
		target.env_setting_names,
		target.project,
	)
	return environment
