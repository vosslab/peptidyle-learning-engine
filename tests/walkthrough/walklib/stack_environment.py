"""Private Compose environment construction for one disposable walkthrough."""

import pathlib
import os
import secrets

import local_stack_control.cleanup
import local_stack_control.commands
import local_stack_control.discovery
import local_stack_control.process

import tests.walkthrough.walklib as walklib
import tests.walkthrough.walklib.models as models


APPLICATION_IMAGE_SETTING = "PLE_APPLICATION_IMAGE"
LOCAL_AUTH_SETTING = "PLE_LOCAL_AUTH_HOST_FILE"
INVITATION_SECRET_SETTING = "PLE_INVITATION_TOKEN_SECRET_HOST_FILE"
QUESTION_ID_SECRET_SETTING = "PLE_QUESTION_ID_SECRET_HOST_FILE"


#============================================
def require_rootless_engine(
	runner: local_stack_control.process.CommandRunner,
	repository_root: pathlib.Path,
) -> None:
	"""Require the shared rootless default engine before walkthrough mutation."""
	local_stack_control.process.require_rootless_local_engine(runner, repository_root)


#============================================
def render_private_environment(
	source: str,
	application_image: str,
	runtime_directory: pathlib.Path,
	capability_digest: str,
) -> bytes:
	"""Copy a selected Compose file while reserving the application image setting."""
	if not application_image.startswith("localhost/peptidyle-learning-engine:"):
		raise models.RunnerError("walkthrough application image is invalid")
	lines = source.splitlines()
	settings = (
		APPLICATION_IMAGE_SETTING,
		LOCAL_AUTH_SETTING,
		INVITATION_SECRET_SETTING,
		QUESTION_ID_SECRET_SETTING,
		local_stack_control.models.DISPOSABLE_CAPABILITY_SETTING,
	)
	retained = [
		line
		for line in lines
		if line
		and not line.lstrip().startswith("#")
		and not line.startswith(tuple(f"{name}=" for name in settings))
	]
	local_auth_file = runtime_directory / "local-identities.json"
	secret_directory = runtime_directory / ".secrets"
	invitation_secret_file = secret_directory / "invitation_token_secret"
	question_id_secret_file = secret_directory / "question_id_secret"
	contents = "\n".join(
		[
			*retained,
			f"{APPLICATION_IMAGE_SETTING}={application_image}",
			f"{LOCAL_AUTH_SETTING}={local_auth_file}",
			f"{INVITATION_SECRET_SETTING}={invitation_secret_file}",
			f"{QUESTION_ID_SECRET_SETTING}={question_id_secret_file}",
			f"{local_stack_control.models.DISPOSABLE_CAPABILITY_SETTING}={capability_digest}",
			"",
		]
	)
	try:
		encoded = contents.encode("ascii")
	except UnicodeEncodeError as error:
		raise walklib.models.RunnerError("selected walkthrough env file must be ASCII") from error
	return encoded


#============================================
def application_image(project_name: str) -> str:
	"""Name the API/worker image so the disposable project cannot reuse a stale build."""
	image = f"localhost/peptidyle-learning-engine:{project_name}"
	return image


#============================================
def gateway_image(project_name: str) -> str:
	"""Return Podman Compose's generated gateway tag for this exact project."""
	image = f"localhost/{project_name}_gateway:latest"
	return image


#============================================
def owned_walkthrough_images(
	disposable: local_stack_control.models.DisposableComposeTarget,
) -> tuple[str, str]:
	"""Return the two exact project tags owned by a verified walkthrough target.

	These are tags, never image IDs: removing them cannot remove the shared
	application image or a default-stack tag that points at the same image.
	"""
	project = disposable.target.project
	if (
		disposable.owner_policy != "ui-walkthrough"
		or disposable.project_prefix != "ple-ui-walkthrough-"
		or not project.startswith(
		disposable.project_prefix
		)
	):
		raise walklib.models.RunnerError("walkthrough image cleanup target is unavailable")
	return (application_image(project), gateway_image(project))


#============================================
def create_cleanup_capability(runtime_directory: pathlib.Path) -> tuple[pathlib.Path, str]:
	"""Create one runner-held capability and return only its commitment."""
	path = runtime_directory / "disposable.capability"
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	try:
		raw = secrets.token_bytes(32)
		if os.write(file_descriptor, raw) != len(raw):
			raise walklib.models.RunnerError("could not write walkthrough cleanup capability")
		os.fsync(file_descriptor)
	finally:
		os.close(file_descriptor)
	try:
		digest = local_stack_control.compose.disposable_capability_digest(path)
	except local_stack_control.models.ControllerError as error:
		raise walklib.models.RunnerError("walkthrough cleanup capability is unavailable") from error
	return path, digest


#============================================
def require_empty_disposable_preflight(
	disposable: local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Prove any retained project resources bind the runner capability."""
	try:
		snapshot = local_stack_control.discovery.discover_snapshot(
			runner, disposable.target.repo_root, disposable.target.project
		)
		local_stack_control.compose.require_disposable_resource_capability(disposable, snapshot)
	except local_stack_control.models.ControllerError as error:
		raise walklib.models.RunnerError("walkthrough cleanup capability preflight failed") from error


#============================================
def remove_disposable_stack(
	disposable: local_stack_control.models.DisposableComposeTarget,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Use shared ownership proof; remove exact tags only after an empty snapshot."""
	target = disposable.target
	try:
		require_rootless_engine(runner, target.repo_root)
		snapshot = local_stack_control.discovery.discover_snapshot(
			runner, target.repo_root, target.project
		)
		local_stack_control.compose.require_disposable_resource_capability(disposable, snapshot)
		resource_count = len(snapshot.containers) + len(snapshot.volumes) + len(snapshot.networks)
		if resource_count > 0:
			plan = local_stack_control.cleanup.disposable_cleanup_plan(
				disposable, snapshot
			)
			status = local_stack_control.commands.execute_cleanup(plan, target, runner, False)
			if status != 0:
				raise walklib.models.RunnerError(f"cleanup command failed with exit status {status}")
		post_snapshot = local_stack_control.discovery.discover_snapshot(
			runner, target.repo_root, target.project
		)
		local_stack_control.compose.require_disposable_resource_capability(
			disposable, post_snapshot
		)
		if len(post_snapshot.containers) + len(post_snapshot.volumes) + len(post_snapshot.networks) != 0:
			raise walklib.models.RunnerError(
				"walkthrough cleanup left labelled resources; retained project image tags"
			)
	except local_stack_control.models.ControllerError as error:
		raise walklib.models.RunnerError(f"walkthrough cleanup ownership check failed: {error}") from error
	for image in owned_walkthrough_images(disposable):
		result = runner.run(["podman", "image", "exists", image], cwd=target.repo_root)
		if result.returncode == 1:
			continue
		if result.returncode != 0:
			raise walklib.models.RunnerError(f"cleanup image inspection failed with exit status {result.returncode}")
		result = runner.run(["podman", "image", "rm", image], cwd=target.repo_root)
		if result.returncode != 0:
			raise walklib.models.RunnerError(f"cleanup image removal failed with exit status {result.returncode}")
