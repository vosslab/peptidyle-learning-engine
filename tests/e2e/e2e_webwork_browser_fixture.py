"""Mint the private typed stack target for WebWork browser acceptance."""

import base64
import hashlib
import json
import os
import pathlib
import secrets
import sys


SCRIPT_REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_REPOSITORY_ROOT))

import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process


TENANT_ID = "00000000-0000-0000-0000-000000000100"
INSTRUCTOR_ID = "00000000-0000-0000-0000-000000000101"
STUDENT_ID = "00000000-0000-0000-0000-000000000102"
POSTGRES_USER = "ple_webwork_browser"
POSTGRES_DATABASE = "ple_webwork_browser"


#============================================
def private_file(path: pathlib.Path, content: str | bytes, mode: int = 0o600) -> None:
	"""Create one private regular file without a permissive creation window."""
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
	with os.fdopen(file_descriptor, "wb") as output:
		output.write(content.encode("ascii") if isinstance(content, str) else content)


#============================================
def secret_text(byte_count: int = 32) -> str:
	"""Return unpadded base64url private entropy."""
	return base64.urlsafe_b64encode(secrets.token_bytes(byte_count)).decode("ascii").rstrip("=")


#============================================
def credential_hash(credential: str) -> str:
	"""Return the credential's SHA-256 hash without retaining raw entropy."""
	padding = "=" * (-len(credential) % 4)
	return hashlib.sha256(base64.urlsafe_b64decode(credential + padding)).hexdigest()


#============================================
def random_port(base: int) -> int:
	"""Choose a collision-resistant port in this owner's private range."""
	return base + secrets.randbelow(400)


#============================================
def prepare(directory: pathlib.Path) -> dict[str, str]:
	"""Write one full, answer-free local stack target under the WebWork owner."""
	root = SCRIPT_REPOSITORY_ROOT
	selections = local_stack_control.env_file.canonical_stack_selections(root)
	postgres_port = random_port(53500)
	minio_port = random_port(54000)
	minio_console_port = random_port(54500)
	gateway_port = random_port(55000)
	runner = local_stack_control.process.SubprocessRunner()
	local_stack_control.process.require_available_loopback_ports(
		(postgres_port, minio_port, minio_console_port, gateway_port), runner, root
	)
	project = "ple-webwork-browser-" + secrets.token_hex(6)
	instructor_credential = secret_text()
	student_credential = secret_text()
	postgres_password = secrets.token_hex(24)
	minio_password = secrets.token_hex(24)

	login_path = directory / "local-login.txt"
	private_file(login_path, f"instructor={instructor_credential}\nstudent={student_credential}\n")
	identities_path = directory / "local-identities.json"
	identities = {
		"credentials": [
			{
				"credential_sha256": credential_hash(instructor_credential), "learner_alias": "instructor-local",
				"tenant_id": TENANT_ID, "user_id": INSTRUCTOR_ID, "display_name": "Dr. Fake Professor",
				"roles": ["instructor", "sysadmin"],
			},
			{
				"credential_sha256": credential_hash(student_credential), "learner_alias": "student-local",
				"tenant_id": TENANT_ID, "user_id": STUDENT_ID, "display_name": "Mary Fake Student",
				"roles": ["student"],
			},
		],
	}
	private_file(identities_path, json.dumps(identities, separators=(",", ":")) + "\n", 0o644)
	invitation_path = directory / "invitation-secret"
	question_id_path = directory / "question-id-secret"
	private_file(invitation_path, secret_text())
	private_file(question_id_path, secret_text())
	capability_path = directory / "disposable.capability"
	capability = secrets.token_bytes(32)
	private_file(capability_path, capability)
	capability_digest = hashlib.sha256(capability).hexdigest()
	env_path = directory / "env.local"
	env = {
		"POSTGRES_USER": POSTGRES_USER, "POSTGRES_PASSWORD": postgres_password, "POSTGRES_DB": POSTGRES_DATABASE,
		"PLE_POSTGRES_IMAGE_SHA256": selections["PLE_POSTGRES_IMAGE_SHA256"], "PLE_LOCAL_GRADER_PASSWORD": secrets.token_hex(24),
		"PLE_POSTGRES_HOST_PORT": str(postgres_port), "MINIO_ROOT_USER": "ple-webwork-browser",
		"MINIO_ROOT_PASSWORD": minio_password, "PLE_MINIO_API_HOST_PORT": str(minio_port),
		"PLE_MINIO_CONSOLE_HOST_PORT": str(minio_console_port), "PLE_MINIO_IMAGE_SHA256": selections["PLE_MINIO_IMAGE_SHA256"],
		"PLE_MINIO_MC_IMAGE_SHA256": selections["PLE_MINIO_MC_IMAGE_SHA256"], "PLE_GATEWAY_HOST_PORT": str(gateway_port),
		"PLE_GATEWAY_IMAGE_SHA256": selections["PLE_GATEWAY_IMAGE_SHA256"], "PLE_LOCAL_AUTH_HOST_FILE": str(identities_path),
		"PLE_PUBLIC_ASSET_BASE_URL": f"http://127.0.0.1:{minio_port}/public-assets", "PLE_WEBAUTHN_RP_ID": "localhost",
		"PLE_WEBAUTHN_ORIGIN": f"http://localhost:{gateway_port}", "PLE_WEBAUTHN_RP_NAME": "Peptidyle Learning Engine",
		"PLE_INVITATION_TOKEN_SECRET_HOST_FILE": str(invitation_path), "PLE_QUESTION_ID_SECRET_HOST_FILE": str(question_id_path),
		"PLE_WEBWORK_RENDERER_IMAGE": selections["PLE_WEBWORK_RENDERER_IMAGE"], "PLE_WEBWORK_RENDERER_BASE_URL": selections["PLE_WEBWORK_RENDERER_BASE_URL"],
		"PLE_WEBWORK_RENDERER_ID": selections["PLE_WEBWORK_RENDERER_ID"], "PLE_WEBWORK_PROVENANCE_FILE": str(directory / "webwork-renderer.provenance"),
		"PLE_WEBWORK_PROBLEM_JWT_SECRET": secrets.token_hex(32), "PLE_WEBWORK_SESSION_JWT_SECRET": secrets.token_hex(32),
		"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS": selections["PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS"], "PLE_WEBWORK_MAX_RESPONSE_BYTES": selections["PLE_WEBWORK_MAX_RESPONSE_BYTES"],
		"PLE_SECRET_INIT_IMAGE_SHA256": selections["PLE_SECRET_INIT_IMAGE_SHA256"], "PLE_DISPOSABLE_CAPABILITY_SHA256": capability_digest,
	}
	private_file(env_path, "".join(f"{name}={value}\n" for name, value in env.items()))
	manifest_path = directory / "disposable.manifest"
	private_file(manifest_path, f"OWNER=webwork-browser\nPROJECT={project}\nENV_FILE={env_path}\nCAPABILITY_FILE={capability_path}\n")
	return {"project": project, "manifest": str(manifest_path), "env": str(env_path), "login": str(login_path), "gateway_port": str(gateway_port)}


#============================================
def main() -> None:
	"""Prepare a new private target and print its non-secret reference receipt."""
	if len(sys.argv) != 2:
		raise SystemExit("usage: e2e_webwork_browser_fixture.py PRIVATE_DIRECTORY")
	directory = pathlib.Path(sys.argv[1]).absolute()
	if not directory.is_dir() or directory.is_symlink():
		raise local_stack_control.models.ControllerError("private WebWork fixture directory is invalid")
	print(json.dumps(prepare(directory), separators=(",", ":")))


if __name__ == "__main__":
	main()
