"""Question Renderer Version selection, OCI identity, and safe probe ownership."""

import os
import pathlib
import re
import stat

import local_stack_control.models
import local_stack_control.private_files
import local_stack_control.process
import local_stack_control.status


LOCAL_REFERENCE = re.compile(r"^localhost/[a-z0-9][a-z0-9._/-]*:[A-Za-z0-9][A-Za-z0-9._-]*$")
IMMUTABLE_REFERENCE = re.compile(r"^[a-z0-9][a-z0-9._/-]*@sha256:[0-9a-f]{64}$")
OCI_ID = re.compile(r"^sha256:[0-9a-f]{64}$")
QUESTION_RENDERER_VERSION_NAME = "question-renderer-version"
LOCAL_REVIEWED_REFERENCE = "localhost/pg-renderer:reviewed"
LOCAL_SOURCE_DIRECTORY_NAME = "webwork-pg-renderer"


#============================================
def validate_renderer_reference(reference: str) -> str:
	"""Accept only a local tag or an immutable digest-qualified renderer reference."""
	if LOCAL_REFERENCE.fullmatch(reference) is None and IMMUTABLE_REFERENCE.fullmatch(reference) is None:
		raise local_stack_control.models.ControllerError("renderer image reference is unsafe or invalid")
	return reference


#============================================
def inspect_renderer_oci_id(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	reference: str,
	environment: dict[str, str],
) -> str:
	"""Inspect one selected renderer image and return its exact OCI configuration ID."""
	validate_renderer_reference(reference)
	result = runner.run(
		["podman", "image", "inspect", reference, "--format", "{{.Id}}"], environment, repo_root
	)
	oci_id = result.stdout.strip()
	# Podman reports the OCI configuration ID without its algorithm prefix,
	# whereas some compatible formatters include it. Question Renderer Version uses one
	# canonical, digest-qualified OCI identity.
	if re.fullmatch(r"[0-9a-f]{64}", oci_id) is not None:
		oci_id = "sha256:" + oci_id
	if not result.ok() or OCI_ID.fullmatch(oci_id) is None:
		raise local_stack_control.models.ControllerError("selected renderer image has no valid OCI ID")
	return oci_id


#============================================
def ensure_renderer_oci_id(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	reference: str,
	environment: dict[str, str],
	build: bool,
) -> str:
	"""Return the selected image identity, creating a missing build-mode image."""
	validate_renderer_reference(reference)
	if not build:
		return inspect_renderer_oci_id(runner, repo_root, reference, environment)
	exists = runner.run(["podman", "image", "exists", reference], environment, repo_root)
	if exists.returncode == 0:
		return inspect_renderer_oci_id(runner, repo_root, reference, environment)
	if exists.returncode != 1:
		raise local_stack_control.models.ControllerError(
			"selected renderer image availability could not be determined"
		)
	if reference == LOCAL_REVIEWED_REFERENCE:
		build_local_renderer(runner, repo_root, reference, environment)
	else:
		# ASVS 3.4.2: immutable digest validation occurs before a registry pull.
		result = runner.stream(["podman", "pull", reference], environment, repo_root)
		if result != 0:
			raise local_stack_control.models.ControllerError(
				"selected immutable renderer image could not be pulled"
			)
	return inspect_renderer_oci_id(runner, repo_root, reference, environment)


#============================================
def build_local_renderer(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	reference: str,
	environment: dict[str, str],
) -> None:
	"""Build the canonical reviewed local image from its maintained sibling."""
	source = repo_root.parent / LOCAL_SOURCE_DIRECTORY_NAME
	dockerfile = source / "Dockerfile"
	if source.is_symlink() or not source.is_dir() or not dockerfile.is_file():
		raise local_stack_control.models.ControllerError(
			"canonical renderer source checkout is unavailable beside the PLE repository"
		)
	# ASVS 13.3.2: the fixed sibling path and selected local tag form the complete
	# build authority; private stack state and caller-provided build contexts stay out.
	result = runner.stream(
		["podman", "build", "--tag", reference, "--file", str(dockerfile), str(source)],
		environment,
		repo_root,
	)
	if result != 0:
		raise local_stack_control.models.ControllerError(
			"canonical reviewed renderer image build failed"
		)


#============================================
def require_running_renderer(
	report: local_stack_control.models.StatusReport,
	selected_oci_id: str,
) -> local_stack_control.models.ContainerResource:
	"""Prove the one label-resolved renderer is running from the selected image."""
	if OCI_ID.fullmatch(selected_oci_id) is None:
		raise local_stack_control.models.ControllerError("selected renderer OCI ID is invalid")
	containers = tuple(
		item for item in report.snapshot.containers if item.service == "webwork-renderer"
	)
	if len(containers) != 1:
		raise local_stack_control.models.ControllerError("renderer service is missing or ambiguous")
	container = containers[0]
	if not container.running or container.image_id != selected_oci_id:
		raise local_stack_control.models.ControllerError(
			"running renderer does not match the selected OCI configuration"
		)
	return container


#============================================
def question_renderer_version_path(directory: pathlib.Path) -> pathlib.Path:
	"""Return the fixed private Question Renderer Version path."""
	if directory.is_symlink() or not directory.is_dir():
		raise local_stack_control.models.ControllerError("Question Renderer Version directory is unavailable")
	return directory / QUESTION_RENDERER_VERSION_NAME


#============================================
def write_question_renderer_version(directory: pathlib.Path, version: local_stack_control.models.QuestionRendererVersion) -> pathlib.Path:
	"""Atomically replace the exact private Question Renderer Version."""
	validate_renderer_reference(version.reference)
	if OCI_ID.fullmatch(version.oci_id) is None:
		raise local_stack_control.models.ControllerError("Question Renderer Version OCI ID is invalid")
	path = question_renderer_version_path(directory)
	if path.exists() or path.is_symlink():
		read_question_renderer_version(directory)
	content = f"reference={version.reference}\noci_id={version.oci_id}\n".encode("ascii")
	local_stack_control.private_files.write_atomic_file(path, content, 0o600)
	result = read_question_renderer_version(directory)
	return result


#============================================
def read_question_renderer_version(directory: pathlib.Path) -> pathlib.Path:
	"""Validate a private Question Renderer Version and return its fixed path."""
	path = question_renderer_version_path(directory)
	metadata = path.lstat()
	if not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != os.getuid():
		raise local_stack_control.models.ControllerError("Question Renderer Version must be a current-user regular file")
	if stat.S_IMODE(metadata.st_mode) != 0o600:
		raise local_stack_control.models.ControllerError("Question Renderer Version must have mode 0600")
	return path


#============================================
def load_question_renderer_version(directory: pathlib.Path) -> local_stack_control.models.QuestionRendererVersion:
	"""Read the exact private Question Renderer Version needed before restart."""
	path = read_question_renderer_version(directory)
	try:
		content = local_stack_control.private_files.read_current_user_private_file(path, 512)
		lines = content.decode("ascii").splitlines()
	except UnicodeDecodeError as error:
		raise local_stack_control.models.ControllerError("Question Renderer Version is malformed") from error
	if len(lines) != 2 or any("=" not in line for line in lines):
		raise local_stack_control.models.ControllerError("Question Renderer Version is malformed")
	values = dict(line.split("=", 1) for line in lines)
	if set(values) != {"reference", "oci_id"}:
		raise local_stack_control.models.ControllerError("Question Renderer Version records are unsafe")
	return local_stack_control.models.QuestionRendererVersion(
		reference=validate_renderer_reference(values["reference"]),
		oci_id=values["oci_id"] if OCI_ID.fullmatch(values["oci_id"]) else _invalid_oci_id(),
	)


#============================================
def _invalid_oci_id() -> str:
	"""Raise a concise error for a malformed Question Renderer Version OCI ID."""
	raise local_stack_control.models.ControllerError("Question Renderer Version OCI ID is invalid")


#============================================
def require_question_renderer_version(directory: pathlib.Path, selected_oci_id: str) -> None:
	"""Require the stored Question Renderer Version to match selected OCI identity."""
	version = load_question_renderer_version(directory)
	if version.oci_id != selected_oci_id:
		raise local_stack_control.models.ControllerError("Question Renderer Version does not match selected OCI ID")


#============================================
def run_renderer_probe(
	runner: local_stack_control.process.CommandRunner,
	repo_root: pathlib.Path,
	argv: list[str],
	environment: dict[str, str],
	request_json: str,
) -> None:
	"""Run the real renderer probe with explicit stdin and a minimal child environment."""
	if len(argv) == 0 or any(value == "" for value in argv) or "\n" in request_json:
		raise local_stack_control.models.ControllerError("renderer probe request is invalid")
	allowed = {name: value for name, value in environment.items() if name in ("PATH", "HOME")}
	result = runner.run(argv, allowed, repo_root, request_json)
	if not result.ok():
		raise local_stack_control.models.ControllerError(
			"renderer probe failed; inspect the retained renderer container logs"
		)
