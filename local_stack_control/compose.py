"""Compose provider, target, ownership, and argv construction."""

import dataclasses
import hashlib
import pathlib
import re

import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process


#============================================
def repo_root_from_entrypoint(
	entrypoint: pathlib.Path,
	runner: local_stack_control.process.CommandRunner,
) -> pathlib.Path:
	"""Return the Git repository root containing the package entry point."""
	resolved_entrypoint = entrypoint.resolve(strict=True)
	package_directory = resolved_entrypoint.parent
	result = runner.run(
		["git", "rev-parse", "--show-toplevel"],
		cwd=package_directory,
	)
	if not result.ok() or result.stdout.strip() == "":
		raise local_stack_control.models.ControllerError(
			"local stack controller is not inside a Git work tree"
		)
	root = pathlib.Path(result.stdout.strip()).resolve(strict=True)
	public_entrypoint = root / "local_stack.py"
	private_package = root / "local_stack_control"
	if resolved_entrypoint != public_entrypoint and package_directory != private_package:
		raise local_stack_control.models.ControllerError(
			f"{resolved_entrypoint} is not a local-stack controller entry point"
		)
	return root


#============================================
def choose_provider(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	required_name: str | None = None,
) -> local_stack_control.models.ComposeProvider:
	"""Select the first usable Podman Compose provider."""
	environment = local_stack_control.env_file.sanitized_runtime_environment(
		local_stack_control.process.current_environment()
	)
	candidates = (
		local_stack_control.models.ComposeProvider(("podman", "compose"), "podman compose"),
		local_stack_control.models.ComposeProvider(("podman-compose",), "podman-compose"),
	)
	if required_name is not None and required_name not in {item.name for item in candidates}:
		raise local_stack_control.models.ControllerError(
			"requested Compose provider is not supported"
		)
	for provider in candidates:
		if required_name is not None and provider.name != required_name:
			continue
		result = runner.run([*provider.argv, "version"], environment, repo_root)
		if result.ok():
			return provider
	if required_name is not None:
		raise local_stack_control.models.ControllerError(
			f"required Compose provider '{required_name}' is unavailable"
		)

	raise local_stack_control.models.ControllerError(
		"neither 'podman compose' nor 'podman-compose' is usable"
	)


#============================================
def compose_files(repo_root: pathlib.Path, with_smtp: bool) -> tuple[pathlib.Path, ...]:
	"""Return explicit Compose files for the selected topology."""
	files = [
		repo_root / local_stack_control.models.PRIMARY_COMPOSE_FILE,
		repo_root / local_stack_control.models.LOCAL_DEVELOPMENT_COMPOSE_FILE,
	]
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
	required_provider: str | None = None,
) -> local_stack_control.models.ComposeTarget:
	"""Resolve an explicit target without consulting ambient project state."""
	provider = choose_provider(runner, repo_root, required_provider)
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
def require_disposable_no_pod_provider(
	target: local_stack_control.models.ComposeTarget,
) -> None:
	"""Require the exact provider argv that cannot create an unlabelled pod."""
	expected_argv = (
		local_stack_control.models.DISPOSABLE_COMPOSE_PROVIDER,
		*local_stack_control.models.DISPOSABLE_PROVIDER_GLOBAL_ARGS,
	)
	if (
		target.provider.name != local_stack_control.models.DISPOSABLE_COMPOSE_PROVIDER
		or target.provider.argv != expected_argv
	):
		raise local_stack_control.models.ControllerError(
			"disposable targets require the exact no-pod Compose provider"
		)


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
	if (
		target.provider.name != local_stack_control.models.DISPOSABLE_COMPOSE_PROVIDER
		or target.provider.argv
		!= (local_stack_control.models.DISPOSABLE_COMPOSE_PROVIDER,)
	):
		raise local_stack_control.models.ControllerError(
			"disposable targets require the no-pod Compose provider"
		)
	provider = local_stack_control.models.ComposeProvider(
		argv=(
			*target.provider.argv,
			*local_stack_control.models.DISPOSABLE_PROVIDER_GLOBAL_ARGS,
		),
		name=target.provider.name,
	)
	target = dataclasses.replace(target, provider=provider)
	disposable = local_stack_control.models.DisposableComposeTarget(
		target=target,
		owner_policy=owner_policy,
		capability_file=capability_file,
		project_prefix=policy.project_prefix,
		private_environment_file=target.env_file,
	)
	require_disposable_no_pod_provider(disposable.target)
	return disposable


#============================================
def require_disposable_capability_file(capability_file: pathlib.Path) -> bytes:
	"""Return one exact runner-held 32-byte capability from a private file."""
	raw = local_stack_control.private_files.read_current_user_private_file(capability_file, 32)
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
	require_disposable_no_pod_provider(disposable.target)
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
