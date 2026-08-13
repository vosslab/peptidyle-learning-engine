"""Private Compose environment construction for one disposable walkthrough."""

import pathlib

import walklib.models


APPLICATION_IMAGE_SETTING = "PLE_APPLICATION_IMAGE"
LOCAL_AUTH_SETTING = "PLE_LOCAL_AUTH_HOST_FILE"
INVITATION_SECRET_SETTING = "PLE_INVITATION_TOKEN_SECRET_HOST_FILE"


#============================================
def render_private_environment(
	source: str, application_image: str, runtime_directory: pathlib.Path
) -> bytes:
	"""Copy a selected Compose file while reserving the application image setting."""
	if not application_image.startswith("localhost/peptidyle-learning-engine:"):
		raise walklib.models.RunnerError("walkthrough application image is invalid")
	lines = source.splitlines()
	settings = (APPLICATION_IMAGE_SETTING, LOCAL_AUTH_SETTING, INVITATION_SECRET_SETTING)
	retained = [line for line in lines if not line.startswith(tuple(f"{name}=" for name in settings))]
	local_auth_file = runtime_directory / "local-identities.json"
	secret_file = runtime_directory / ".secrets" / "invitation_token_secret"
	contents = "\n".join(
		[
			*retained,
			f"{APPLICATION_IMAGE_SETTING}={application_image}",
			f"{LOCAL_AUTH_SETTING}={local_auth_file}",
			f"{INVITATION_SECRET_SETTING}={secret_file}",
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
def remove_disposable_stack(
	compose_command: list[str],
	env_file: pathlib.Path,
	project_name: str,
	environment: dict[str, str],
	run_command: walklib.models.CommandRunner,
) -> None:
	"""Remove only the current generated Compose project and its named volumes."""
	command = compose_command + [
		"-f", "containers/compose.yaml", "--env-file", str(env_file),
		"down", "--volumes", "--remove-orphans",
	]
	result = run_command(command, environment)
	if result.returncode != 0:
		raise walklib.models.RunnerError(f"cleanup command failed with exit status {result.returncode}")
	for image in (application_image(project_name), gateway_image(project_name)):
		result = run_command(["podman", "image", "exists", image], environment)
		if result.returncode == 1:
			continue
		if result.returncode != 0:
			raise walklib.models.RunnerError(f"cleanup image inspection failed with exit status {result.returncode}")
		result = run_command(["podman", "image", "rm", image], environment)
		if result.returncode != 0:
			raise walklib.models.RunnerError(f"cleanup image removal failed with exit status {result.returncode}")
