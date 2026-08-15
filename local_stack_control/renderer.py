"""Renderer selection, OCI identity, provenance, and safe probe ownership."""

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
PROVENANCE_NAME = "webwork-renderer.provenance"


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
	# whereas some compatible formatters include it.  Provenance uses one
	# canonical, digest-qualified representation.
	if re.fullmatch(r"[0-9a-f]{64}", oci_id) is not None:
		oci_id = "sha256:" + oci_id
	if not result.ok() or OCI_ID.fullmatch(oci_id) is None:
		raise local_stack_control.models.ControllerError("selected renderer image has no valid OCI ID")
	return oci_id


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
def private_provenance_path(directory: pathlib.Path) -> pathlib.Path:
	"""Return the fixed private provenance path under an owner-controlled directory."""
	if directory.is_symlink() or not directory.is_dir():
		raise local_stack_control.models.ControllerError("renderer provenance directory is unavailable")
	return directory / PROVENANCE_NAME


#============================================
def write_provenance(directory: pathlib.Path, provenance: local_stack_control.models.RendererProvenance) -> pathlib.Path:
	"""Atomically replace the exact two-record private renderer provenance contract."""
	validate_renderer_reference(provenance.reference)
	if OCI_ID.fullmatch(provenance.oci_id) is None:
		raise local_stack_control.models.ControllerError("renderer provenance OCI ID is invalid")
	path = private_provenance_path(directory)
	if path.exists() or path.is_symlink():
		read_provenance(directory)
	content = f"reference={provenance.reference}\noci_id={provenance.oci_id}\n".encode("ascii")
	local_stack_control.private_files.write_atomic_file(path, content, 0o600)
	result = read_provenance(directory)
	return result


#============================================
def read_provenance(directory: pathlib.Path) -> pathlib.Path:
	"""Validate a private provenance boundary and return its fixed path."""
	path = private_provenance_path(directory)
	metadata = path.lstat()
	if not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != os.getuid():
		raise local_stack_control.models.ControllerError("renderer provenance must be a current-user regular file")
	if stat.S_IMODE(metadata.st_mode) != 0o600:
		raise local_stack_control.models.ControllerError("renderer provenance must have mode 0600")
	return path


#============================================
def load_provenance(directory: pathlib.Path) -> local_stack_control.models.RendererProvenance:
	"""Read exactly the two unique, validated records needed before restart."""
	path = read_provenance(directory)
	try:
		content = local_stack_control.private_files.read_current_user_private_file(path, 512)
		lines = content.decode("ascii").splitlines()
	except UnicodeDecodeError as error:
		raise local_stack_control.models.ControllerError("renderer provenance is malformed") from error
	if len(lines) != 2 or any("=" not in line for line in lines):
		raise local_stack_control.models.ControllerError("renderer provenance is malformed")
	values = dict(line.split("=", 1) for line in lines)
	if set(values) != {"reference", "oci_id"}:
		raise local_stack_control.models.ControllerError("renderer provenance records are unsafe")
	return local_stack_control.models.RendererProvenance(
		reference=validate_renderer_reference(values["reference"]),
		oci_id=values["oci_id"] if OCI_ID.fullmatch(values["oci_id"]) else _invalid_oci_id(),
	)


#============================================
def _invalid_oci_id() -> str:
	"""Raise a concise error for malformed private provenance."""
	raise local_stack_control.models.ControllerError("renderer provenance OCI ID is invalid")


#============================================
def require_restart_provenance(directory: pathlib.Path, selected_oci_id: str) -> None:
	"""Require restart provenance to match the freshly selected OCI identity."""
	provenance = load_provenance(directory)
	if provenance.oci_id != selected_oci_id:
		raise local_stack_control.models.ControllerError("renderer provenance does not match selected OCI ID")


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
