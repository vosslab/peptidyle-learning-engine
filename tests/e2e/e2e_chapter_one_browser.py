#!/usr/bin/env python3
"""Run the isolated Chapter One browser journey through typed stack ownership."""

import base64
import hashlib
import os
import pathlib
import secrets
import sys
from collections.abc import Mapping, Sequence

SCRIPT_REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_REPOSITORY_ROOT))

import local_stack_control.chapter_one
import local_stack_control.consumer
import local_stack_control.models
import local_stack_control.private_state
import local_stack_control.process


TENANT_ID = "00000000-0000-0000-0000-000000000100"
INSTRUCTOR_ID = "00000000-0000-0000-0000-000000000101"
STUDENT_ID = "00000000-0000-0000-0000-000000000102"
POSTGRES_USER = "ple_chapter_browser"
POSTGRES_DATABASE = "ple_chapter_browser"
PRIVATE_STATE_RELATIVE_DIRECTORY = pathlib.Path("target") / "chapter-one-browser"
PRIVATE_STATE_DIRECTORY_PREFIX = "run-"


class BrowserE2EError(local_stack_control.models.ControllerError):
	"""A concise failure owned by the Chapter One browser E2E."""


#============================================
def repo_root() -> pathlib.Path:
	"""Return the checkout containing this canonical browser E2E owner."""
	result = pathlib.Path(__file__).resolve().parents[2]
	return result


#============================================
def private_file(path: pathlib.Path, content: str | bytes) -> None:
	"""Create one mode-0600 regular file without a permissive creation window."""
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(file_descriptor, "wb") as output:
		if isinstance(content, str):
			output.write(content.encode("ascii"))
		else:
			output.write(content)


#============================================
def random_port(base: int) -> int:
	"""Choose a port in this owner's bounded disposable range."""
	result = base + secrets.randbelow(400)
	return result


#============================================
def base64url_secret(byte_count: int) -> str:
	"""Return one unpadded base64url secret with the requested random entropy."""
	result = base64.urlsafe_b64encode(secrets.token_bytes(byte_count)).decode("ascii")
	return result.rstrip("=")


#============================================
def credential_hash(credential: str) -> str:
	"""Return the SHA-256 digest of the credential's original random bytes."""
	padding = "=" * (-len(credential) % 4)
	raw_credential = base64.urlsafe_b64decode(credential + padding)
	result = hashlib.sha256(raw_credential).hexdigest()
	return result


#============================================
def adapter_argv(
	action: str,
	manifest_path: pathlib.Path,
	arguments: Sequence[str] = (),
) -> list[str]:
	"""Form one exact call into the closed disposable lifecycle adapter."""
	result = [
		sys.executable,
		"-m",
		"local_stack_control._consumer_cli",
		action,
		"--manifest",
		str(manifest_path),
	]
	result.extend(arguments)
	return result


#============================================
def run(
	runner: local_stack_control.process.CommandRunner,
	argv: list[str],
	root: pathlib.Path,
	environment: dict[str, str] | None = None,
) -> None:
	"""Run one visible E2E boundary and preserve its actual failure status."""
	returncode = runner.stream(argv, environment, root)
	if returncode != 0:
		raised = "Chapter 1 browser E2E command failed: " + " ".join(argv)
		raise BrowserE2EError(raised)


#============================================
def write_private_target(
	directory: pathlib.Path,
	postgres_port: int,
	minio_port: int,
	minio_console_port: int,
	gateway_port: int,
	selections: Mapping[str, str],
) -> tuple[str, pathlib.Path, str, str, pathlib.Path]:
	"""Create one complete private stack target and its browser identities."""
	project = "ple-chapter-one-browser-" + secrets.token_hex(6)
	instructor_credential = base64url_secret(32)
	student_credential = base64url_secret(32)
	postgres_password = secrets.token_hex(24)
	minio_password = secrets.token_hex(24)
	grader_password = secrets.token_hex(24)
	invitation_secret = base64url_secret(32)
	question_id_secret = base64url_secret(32)

	login_path = directory / "local-login.txt"
	private_file(
		login_path,
		f"instructor={instructor_credential}\nstudent={student_credential}\n",
	)
	identities_path = directory / "local-identities.json"
	identities_content = (
		'{"credentials":[{"credential_sha256":"'
		+ credential_hash(instructor_credential)
		+ '","learner_alias":"instructor-local","tenant_id":"'
		+ TENANT_ID
		+ '","user_id":"'
		+ INSTRUCTOR_ID
		+ '","display_name":"Dr. Fake Professor","roles":["instructor","sysadmin"]},'
		+ '{"credential_sha256":"'
		+ credential_hash(student_credential)
		+ '","learner_alias":"student-local","tenant_id":"'
		+ TENANT_ID
		+ '","user_id":"'
		+ STUDENT_ID
		+ '","display_name":"Mary Fake Student","roles":["student"]}]}\n'
	)
	private_file(identities_path, identities_content)
	# The unprivileged API runtime reads this answer-free identity-hash file.
	os.chmod(identities_path, 0o644)
	invitation_path = directory / "invitation-secret"
	question_id_secret_path = directory / "question-id-secret"
	renderer_provenance_path = directory / "webwork-renderer.provenance"
	private_file(invitation_path, invitation_secret)
	private_file(question_id_secret_path, question_id_secret)

	capability_path = directory / "disposable.capability"
	capability = secrets.token_bytes(32)
	private_file(capability_path, capability)
	capability_digest = hashlib.sha256(capability).hexdigest()
	env_path = directory / "env.local"
	env_content = (
		f"POSTGRES_USER={POSTGRES_USER}\n"
		f"POSTGRES_PASSWORD={postgres_password}\n"
		f"POSTGRES_DB={POSTGRES_DATABASE}\n"
		f"PLE_POSTGRES_IMAGE_SHA256={selections['PLE_POSTGRES_IMAGE_SHA256']}\n"
		f"PLE_LOCAL_GRADER_PASSWORD={grader_password}\n"
		f"PLE_POSTGRES_HOST_PORT={postgres_port}\n"
		"MINIO_ROOT_USER=ple-chapter-browser\n"
		f"MINIO_ROOT_PASSWORD={minio_password}\n"
		f"PLE_MINIO_API_HOST_PORT={minio_port}\n"
		f"PLE_MINIO_CONSOLE_HOST_PORT={minio_console_port}\n"
		f"PLE_MINIO_IMAGE_SHA256={selections['PLE_MINIO_IMAGE_SHA256']}\n"
		f"PLE_MINIO_MC_IMAGE_SHA256={selections['PLE_MINIO_MC_IMAGE_SHA256']}\n"
		f"PLE_GATEWAY_HOST_PORT={gateway_port}\n"
		f"PLE_GATEWAY_IMAGE_SHA256={selections['PLE_GATEWAY_IMAGE_SHA256']}\n"
		f"PLE_LOCAL_AUTH_HOST_FILE={identities_path}\n"
		f"PLE_PUBLIC_ASSET_BASE_URL=http://127.0.0.1:{minio_port}/public-assets\n"
		"PLE_WEBAUTHN_RP_ID=localhost\n"
		f"PLE_WEBAUTHN_ORIGIN=http://localhost:{gateway_port}\n"
		"PLE_WEBAUTHN_RP_NAME=Peptidyle Learning Engine\n"
		f"PLE_INVITATION_TOKEN_SECRET_HOST_FILE={invitation_path}\n"
		f"PLE_QUESTION_ID_SECRET_HOST_FILE={question_id_secret_path}\n"
		f"PLE_WEBWORK_RENDERER_IMAGE={selections['PLE_WEBWORK_RENDERER_IMAGE']}\n"
		f"PLE_WEBWORK_RENDERER_BASE_URL={selections['PLE_WEBWORK_RENDERER_BASE_URL']}\n"
		f"PLE_WEBWORK_RENDERER_ID={selections['PLE_WEBWORK_RENDERER_ID']}\n"
		f"PLE_WEBWORK_PROVENANCE_FILE={renderer_provenance_path}\n"
		f"PLE_WEBWORK_PROBLEM_JWT_SECRET={secrets.token_hex(32)}\n"
		f"PLE_WEBWORK_SESSION_JWT_SECRET={secrets.token_hex(32)}\n"
		f"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS={selections['PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS']}\n"
		f"PLE_WEBWORK_MAX_RESPONSE_BYTES={selections['PLE_WEBWORK_MAX_RESPONSE_BYTES']}\n"
		f"PLE_SECRET_INIT_IMAGE_SHA256={selections['PLE_SECRET_INIT_IMAGE_SHA256']}\n"
		f"PLE_DISPOSABLE_CAPABILITY_SHA256={capability_digest}\n"
	)
	private_file(env_path, env_content)
	manifest_path = directory / "disposable.manifest"
	manifest_content = (
		"OWNER=chapter-one-browser\n"
		f"PROJECT={project}\n"
		f"ENV_FILE={env_path}\n"
		f"CAPABILITY_FILE={capability_path}\n"
	)
	private_file(manifest_path, manifest_content)
	return project, manifest_path, postgres_password, minio_password, question_id_secret_path


#============================================
def playwright_environment(
	gateway_port: int,
	login_path: pathlib.Path,
) -> dict[str, str]:
	"""Return the only live-browser inputs required by the visible journey."""
	environment = local_stack_control.process.current_environment()
	for name in tuple(environment):
		if name.startswith("PLE_") or name in (
			"AWS_ACCESS_KEY_ID",
			"AWS_SECRET_ACCESS_KEY",
			"AWS_SESSION_TOKEN",
		):
			environment.pop(name)
	environment["PLE_WEBWORK_LIVE_REQUIRED"] = "1"
	environment["PLE_WEBWORK_LIVE_BASE_URL"] = f"http://127.0.0.1:{gateway_port}"
	environment["PLE_WEBWORK_LIVE_STUDENT_CREDENTIAL_FILE"] = str(login_path)
	return environment


#============================================
def cleanup(
	runner: local_stack_control.process.CommandRunner,
	manifest_path: pathlib.Path,
	root: pathlib.Path,
) -> None:
	"""Invoke only the capability-checked exact disposable cleanup adapter."""
	run(runner, adapter_argv("cleanup", manifest_path), root)


#============================================
def main() -> None:
	"""Build, seed, browse, and exactly clean the disposable Chapter One stack."""
	root = repo_root()
	selections = local_stack_control.env_file.canonical_stack_selections(root)
	runner = local_stack_control.process.SubprocessRunner()
	postgres_port = random_port(51500)
	minio_port = random_port(52000)
	minio_console_port = random_port(52500)
	gateway_port = random_port(53000)
	local_stack_control.process.require_available_loopback_ports(
		(postgres_port, minio_port, minio_console_port, gateway_port),
		runner,
		root,
	)
	try:
		private_state = local_stack_control.private_state.prepare(
			root,
			PRIVATE_STATE_RELATIVE_DIRECTORY,
			PRIVATE_STATE_DIRECTORY_PREFIX,
		)
	except local_stack_control.models.ControllerError as error:
		raise BrowserE2EError("could not prepare private Chapter One browser state") from error
	directory = private_state.directory
	project, manifest_path, postgres_password, minio_password, question_id_secret_path = (
		write_private_target(
			directory,
			postgres_port,
			minio_port,
			minio_console_port,
			gateway_port,
			selections,
		)
	)
	login_path = directory / "local-login.txt"
	stack_started = False
	keep = os.environ.get("PLE_E2E_KEEP") == "1"
	try:
		print("Chapter 1 browser E2E: building and starting an isolated complete PLE stack")
		stack_started = True
		run(runner, adapter_argv("launch", manifest_path, ["--timeout-seconds", "240"]), root)
		database_url = (
			f"postgres://{POSTGRES_USER}:{postgres_password}@127.0.0.1:{postgres_port}/"
			f"{POSTGRES_DATABASE}"
		)
		chapter_manifest = directory / "chapter-one.json"
		request = local_stack_control.chapter_one.ChapterOneSeedRequest(
			repo_root=root,
			database_url=database_url,
			tenant_id=TENANT_ID,
			instructor_id=INSTRUCTOR_ID,
			student_id=STUDENT_ID,
			s3_endpoint=f"http://127.0.0.1:{minio_port}",
			aws_access_key_id="ple-chapter-browser",
			aws_secret_access_key=minio_password,
			question_id_secret_file=question_id_secret_path,
			manifest_path=chapter_manifest,
			existing_manifest_path=None,
		)
		print("Chapter 1 browser E2E: publishing the exact two-by-four teaching corpus")
		local_stack_control.chapter_one.publish_with_runner(request, runner)
		print(
			"Chapter 1 browser E2E: browsing disclosed evidence and completing questions "
			"through visible keyboard controls"
		)
		run(
			runner,
			["npx", "playwright", "test", "tests/playwright/chapter_one_run.spec.ts"],
			root,
			playwright_environment(gateway_port, login_path),
		)
		print("Chapter 1 browser E2E: PASS")
	finally:
		if keep:
			print(f"Chapter 1 browser E2E: preserving {project} and {directory}")
		elif stack_started:
			try:
				cleanup(runner, manifest_path, root)
			except BrowserE2EError:
				print(
					"Chapter 1 browser E2E: exact disposable cleanup failed; "
					f"preserving {directory}",
					file=sys.stderr,
				)
				raise
			private_state.remove()
		else:
			private_state.remove()


if __name__ == "__main__":
	main()
