#!/usr/bin/env python3
"""Prove fresh host publication reaches the isolated live WebWork renderer."""

import base64
import hashlib
import json
import os
import pathlib
import re
import secrets
import shutil
import sys
import tempfile
import uuid
from collections.abc import Mapping, Sequence
from dataclasses import dataclass

SCRIPT_REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_REPOSITORY_ROOT))

import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process


TENANT_ID = "00000000-0000-0000-0000-000000000100"
INSTRUCTOR_ID = "00000000-0000-0000-0000-000000000101"
STUDENT_ID = "00000000-0000-0000-0000-000000000102"
POSTGRES_USER = "ple_wp_r2_host_seed"
POSTGRES_DATABASE = "ple_wp_r2_host_seed"
QUESTION_ID_PATTERN = "^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$"


class HostSeedE2EError(local_stack_control.models.ControllerError):
	"""A concise failure owned by this complete-stack acceptance runner."""


@dataclass(frozen=True)
class SeedManifest:
	"""Private host-only identity evidence from one verified seed invocation."""

	assignment_id: str
	enrollment_id: str
	question_id: str
	problem_id: str
	version_id: str


#============================================
def repo_root() -> pathlib.Path:
	"""Return the checkout containing this canonical E2E owner."""
	return SCRIPT_REPOSITORY_ROOT


#============================================
def private_file(path: pathlib.Path, content: str | bytes) -> None:
	"""Create one regular mode-0600 file without a permissive creation window."""
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(file_descriptor, "wb") as output:
		output.write(content.encode("ascii") if isinstance(content, str) else content)


#============================================
def random_port(base: int) -> int:
	"""Choose a port from this owner's bounded private range."""
	return base + secrets.randbelow(400)


#============================================
def base64url_secret(byte_count: int) -> str:
	"""Return an unpadded base64url secret with the requested entropy."""
	return base64.urlsafe_b64encode(secrets.token_bytes(byte_count)).decode("ascii").rstrip("=")


#============================================
def credential_hash(credential: str) -> str:
	"""Return the digest used by the local identity provider without logging it."""
	padding = "=" * (-len(credential) % 4)
	return hashlib.sha256(base64.urlsafe_b64decode(credential + padding)).hexdigest()


#============================================
def adapter_argv(action: str, manifest_path: pathlib.Path, arguments: Sequence[str] = ()) -> list[str]:
	"""Form one exact invocation of the closed disposable lifecycle adapter."""
	return [
		sys.executable,
		"-m",
		"local_stack_control._consumer_cli",
		action,
		"--manifest",
		str(manifest_path),
		*arguments,
	]


#============================================
def run(
	runner: local_stack_control.process.CommandRunner,
	argv: list[str],
	root: pathlib.Path,
	environment: dict[str, str] | None = None,
) -> None:
	"""Run one external boundary without flooding the durable E2E receipt."""
	completed = runner.run(argv, environment, root)
	if not completed.ok():
		detail = (completed.stderr + "\n" + completed.stdout).strip()[-2_000:]
		raise HostSeedE2EError(
			"WP-R2 host-seed E2E command failed: " + " ".join(argv) + "\n" + detail
		)


#============================================
def captured(
	runner: local_stack_control.process.CommandRunner,
	argv: list[str],
	root: pathlib.Path,
	environment: dict[str, str],
) -> str:
	"""Run one host-only command and return its private, transient stdout."""
	completed = runner.run(argv, environment, root)
	if not completed.ok():
		raise HostSeedE2EError("WP-R2 host-seed command failed: " + " ".join(argv))
	return completed.stdout.strip()


#============================================
def parse_seed_manifest(text: str) -> SeedManifest:
	"""Validate one bounded host-only seed record without retaining a snapshot."""
	try:
		value = json.loads(text)
	except json.JSONDecodeError as error:
		raise HostSeedE2EError("host seed did not emit JSON evidence") from error
	if not isinstance(value, dict) or set(value) != {
		"assignmentId",
		"enrollmentId",
		"questionId",
		"problemId",
		"versionId",
	}:
		raise HostSeedE2EError("host seed emitted an incomplete identity record")
	fields = {name: value[name] for name in value}
	if not all(isinstance(item, str) and item != "" for item in fields.values()):
		raise HostSeedE2EError("host seed emitted an invalid identity value")
	question_id = fields["questionId"]
	if re.fullmatch(QUESTION_ID_PATTERN, question_id) is None:
		raise HostSeedE2EError("host seed emitted a malformed Question ID")
	return SeedManifest(
		assignment_id=canonical_uuid("assignmentId", fields["assignmentId"]),
		enrollment_id=canonical_uuid("enrollmentId", fields["enrollmentId"]),
		question_id=question_id,
		problem_id=canonical_uuid("problemId", fields["problemId"]),
		version_id=canonical_uuid("versionId", fields["versionId"]),
	)


#============================================
def canonical_uuid(field_name: str, value: str) -> str:
	"""Require one opaque host record field to use the canonical UUID spelling."""
	try:
		parsed = uuid.UUID(value)
	except ValueError as error:
		raise HostSeedE2EError(f"host seed emitted a malformed {field_name}") from error
	if str(parsed) != value:
		raise HostSeedE2EError(f"host seed emitted a non-canonical {field_name}")
	return value


#============================================
def require_malformed_manifest_refusal() -> None:
	"""Exercise fail-closed parsing before a manifest value can reach SQL cleanup."""
	valid_identifier = str(uuid.uuid4())
	malformed = {
		"assignmentId": "not-a-uuid",
		"enrollmentId": valid_identifier,
		"questionId": "ABC-2345",
		"problemId": valid_identifier,
		"versionId": valid_identifier,
	}
	try:
		parse_seed_manifest(json.dumps(malformed))
	except HostSeedE2EError:
		return
	raise HostSeedE2EError("host seed malformed assignment evidence was accepted")


#============================================
def require_replay(first: SeedManifest, replay: SeedManifest, mode: str) -> None:
	"""Require retained-marker convergence by relational identity equality."""
	if replay != first:
		raise HostSeedE2EError(f"{mode} seed replay did not resolve its retained publication")


#============================================
def require_distinct_modes(native: SeedManifest, webwork: SeedManifest) -> None:
	"""Require native and WebWork to retain independently minted publications."""
	if (
		native.question_id == webwork.question_id
		or native.problem_id == webwork.problem_id
		or native.version_id == webwork.version_id
	):
		raise HostSeedE2EError("native and WebWork host publications were not relationally distinct")


#============================================
def write_private_target(
	directory: pathlib.Path,
	postgres_port: int,
	minio_port: int,
	minio_console_port: int,
	gateway_port: int,
	selections: Mapping[str, str],
) -> tuple[str, pathlib.Path, pathlib.Path, str, str]:
	"""Create the one complete private stack target and host-only credentials."""
	project = "ple-wp-r2-host-seed-renderer-" + secrets.token_hex(6)
	instructor_credential = base64url_secret(32)
	student_credential = base64url_secret(32)
	postgres_password = secrets.token_hex(24)
	minio_password = secrets.token_hex(24)
	grader_password = secrets.token_hex(24)
	invitation_secret = base64url_secret(32)
	question_secret = base64url_secret(32)
	login_path = directory / "local-login.txt"
	private_file(login_path, f"instructor={instructor_credential}\nstudent={student_credential}\n")
	identities_path = directory / "local-identities.json"
	private_file(
		identities_path,
		'{"credentials":[{"credential_sha256":"'
		+ credential_hash(instructor_credential)
		+ '","learner_alias":"instructor-local","tenant_id":"'
		+ TENANT_ID
		+ '","user_id":"'
		+ INSTRUCTOR_ID
		+ '","display_name":"Dr. Local Instructor","roles":["instructor","sysadmin"]},'
		+ '{"credential_sha256":"'
		+ credential_hash(student_credential)
		+ '","learner_alias":"student-local","tenant_id":"'
		+ TENANT_ID
		+ '","user_id":"'
		+ STUDENT_ID
		+ '","display_name":"Local Learner","roles":["student"]}]}\n',
	)
	os.chmod(identities_path, 0o644)
	invitation_path = directory / "invitation-secret"
	question_secret_path = directory / "question-id-secret"
	private_file(invitation_path, invitation_secret)
	private_file(question_secret_path, question_secret)
	capability_path = directory / "disposable.capability"
	capability = secrets.token_bytes(32)
	private_file(capability_path, capability)
	env_path = directory / "env.local"
	private_file(
		env_path,
		f"POSTGRES_USER={POSTGRES_USER}\n"
		f"POSTGRES_PASSWORD={postgres_password}\n"
		f"POSTGRES_DB={POSTGRES_DATABASE}\n"
		f"PLE_POSTGRES_IMAGE_SHA256={selections['PLE_POSTGRES_IMAGE_SHA256']}\n"
		f"PLE_POSTGRES_HOST_PORT={postgres_port}\n"
		"MINIO_ROOT_USER=ple-wp-r2-host-seed\n"
		f"MINIO_ROOT_PASSWORD={minio_password}\n"
		f"PLE_MINIO_API_HOST_PORT={minio_port}\n"
		f"PLE_MINIO_CONSOLE_HOST_PORT={minio_console_port}\n"
		f"PLE_MINIO_IMAGE_SHA256={selections['PLE_MINIO_IMAGE_SHA256']}\n"
		f"PLE_MINIO_MC_IMAGE_SHA256={selections['PLE_MINIO_MC_IMAGE_SHA256']}\n"
		f"PLE_GATEWAY_HOST_PORT={gateway_port}\n"
		f"PLE_GATEWAY_IMAGE_SHA256={selections['PLE_GATEWAY_IMAGE_SHA256']}\n"
		f"PLE_LOCAL_GRADER_PASSWORD={grader_password}\n"
		f"PLE_LOCAL_AUTH_HOST_FILE={identities_path}\n"
		f"PLE_PUBLIC_ASSET_BASE_URL=http://127.0.0.1:{minio_port}/public-assets\n"
		"PLE_WEBAUTHN_RP_ID=localhost\n"
		f"PLE_WEBAUTHN_ORIGIN=http://localhost:{gateway_port}\n"
		"PLE_WEBAUTHN_RP_NAME=Peptidyle Learning Engine\n"
		f"PLE_INVITATION_TOKEN_SECRET_HOST_FILE={invitation_path}\n"
		f"PLE_QUESTION_ID_SECRET_HOST_FILE={question_secret_path}\n"
		f"PLE_WEBWORK_RENDERER_IMAGE={selections['PLE_WEBWORK_RENDERER_IMAGE']}\n"
		f"PLE_WEBWORK_RENDERER_BASE_URL={selections['PLE_WEBWORK_RENDERER_BASE_URL']}\n"
		f"PLE_WEBWORK_RENDERER_ID={selections['PLE_WEBWORK_RENDERER_ID']}\n"
		f"PLE_WEBWORK_PROBLEM_JWT_SECRET={secrets.token_hex(32)}\n"
		f"PLE_WEBWORK_SESSION_JWT_SECRET={secrets.token_hex(32)}\n"
		f"PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS={selections['PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS']}\n"
		f"PLE_WEBWORK_MAX_RESPONSE_BYTES={selections['PLE_WEBWORK_MAX_RESPONSE_BYTES']}\n"
		f"PLE_SECRET_INIT_IMAGE_SHA256={selections['PLE_SECRET_INIT_IMAGE_SHA256']}\n"
		f"PLE_DISPOSABLE_CAPABILITY_SHA256={hashlib.sha256(capability).hexdigest()}\n",
	)
	manifest_path = directory / "disposable.manifest"
	private_file(
		manifest_path,
		"OWNER=wp-r2-host-seed-renderer\n"
		f"PROJECT={project}\nENV_FILE={env_path}\nCAPABILITY_FILE={capability_path}\n",
	)
	return project, manifest_path, question_secret_path, postgres_password, minio_password


#============================================
def clean_environment() -> dict[str, str]:
	"""Return an E2E environment without inherited PLE, Compose, or AWS state."""
	environment = local_stack_control.process.current_environment()
	for name in tuple(environment):
		if name.startswith(("PLE_", "COMPOSE_")) or name in {
			"AWS_ACCESS_KEY_ID",
			"AWS_SECRET_ACCESS_KEY",
			"AWS_SESSION_TOKEN",
		}:
			environment.pop(name)
	return environment


#============================================
def host_environment(
	question_secret_path: pathlib.Path,
	minio_password: str,
	database_url: str,
) -> dict[str, str]:
	"""Return a scrubbed host-publication environment with private S3 credentials."""
	environment = clean_environment()
	environment["PLE_QUESTION_ID_SECRET_FILE"] = str(question_secret_path)
	environment["PLE_MIGRATION_DATABASE_URL"] = database_url
	environment["AWS_ACCESS_KEY_ID"] = "ple-wp-r2-host-seed"
	environment["AWS_SECRET_ACCESS_KEY"] = minio_password
	return environment


#============================================
def seed_argv(webwork: bool, minio_port: int) -> list[str]:
	"""Form one explicit native or WebWork host publication command."""
	argv = [
		"cargo", "tools", "e2e-seed",
		"--tenant", TENANT_ID, "--instructor", INSTRUCTOR_ID, "--student", STUDENT_ID,
		"--apply-migrations",
	]
	if webwork:
		argv.extend([
			"--webwork-pilot", "--s3-endpoint", f"http://127.0.0.1:{minio_port}",
			"--s3-region", "us-east-1", "--private-content-bucket", "private-content",
		])
	return argv


#============================================
def browser_environment(gateway_port: int, login_path: pathlib.Path, question_id: str) -> dict[str, str]:
	"""Return the explicit, isolated live browser inputs for the renderer proof."""
	environment = clean_environment()
	environment["PLE_WEBWORK_LIVE_REQUIRED"] = "1"
	environment["PLE_WEBWORK_LIVE_BASE_URL"] = f"http://127.0.0.1:{gateway_port}"
	environment["PLE_WEBWORK_LIVE_STUDENT_CREDENTIAL_FILE"] = str(login_path)
	environment["PLE_WP_R2_WEBWORK_QUESTION_ID"] = question_id
	return environment


#============================================
def create_partial_native_marker(
	runner: local_stack_control.process.CommandRunner,
	manifest_path: pathlib.Path,
	root: pathlib.Path,
	assignment_id: str,
) -> None:
	"""Leave the durable native course marker while removing its protected assignment."""
	sql = (
		f"DELETE FROM public.enrollment WHERE tenant_id = '{TENANT_ID}' "
		f"AND assignment_id = '{assignment_id}'; "
		f"DELETE FROM public.assignment WHERE tenant_id = '{TENANT_ID}' "
		f"AND assignment_id = '{assignment_id}';"
	)
	run(
		runner,
		adapter_argv(
			"compose",
			manifest_path,
			[
				"exec", "-T", "postgres", "psql", "-v", "ON_ERROR_STOP=1",
				"-U", POSTGRES_USER, "-d", POSTGRES_DATABASE, "-c", sql,
			],
		),
		root,
	)


#============================================
def require_partial_marker_refusal(
	runner: local_stack_control.process.CommandRunner,
	argv: list[str],
	root: pathlib.Path,
	environment: dict[str, str],
) -> None:
	"""Require an interrupted marker to stop before it can mint another question."""
	completed = runner.run(argv, environment, root)
	if completed.ok():
		raise HostSeedE2EError("a partial host marker minted a replacement publication")


#============================================
def cleanup(runner: local_stack_control.process.CommandRunner, manifest_path: pathlib.Path, root: pathlib.Path) -> None:
	"""Use the capability-checked adapter for exact project cleanup."""
	run(runner, adapter_argv("cleanup", manifest_path), root)


#============================================
def main() -> None:
	"""Launch, publish/replay native and WebWork, render, and exactly clean."""
	require_malformed_manifest_refusal()
	root = repo_root()
	selections = local_stack_control.env_file.canonical_stack_selections(root)
	runner = local_stack_control.process.SubprocessRunner()
	postgres_port = random_port(53500)
	minio_port = random_port(54000)
	minio_console_port = random_port(54500)
	gateway_port = random_port(55000)
	local_stack_control.process.require_available_loopback_ports(
		(postgres_port, minio_port, minio_console_port, gateway_port), runner, root
	)
	directory = pathlib.Path(tempfile.mkdtemp(prefix="ple-wp-r2-host-seed-renderer-"))
	os.chmod(directory, 0o700)
	project, manifest_path, question_secret_path, postgres_password, minio_password = write_private_target(
		directory, postgres_port, minio_port, minio_console_port, gateway_port, selections
	)
	keep = os.environ.get("PLE_E2E_KEEP") == "1"
	stack_started = False
	try:
		print("WP-R2 host seed: starting isolated full PLE stack")
		stack_started = True
		run(runner, adapter_argv("launch", manifest_path, ["--timeout-seconds", "240"]), root)
		database_url = f"postgres://{POSTGRES_USER}:{postgres_password}@127.0.0.1:{postgres_port}/{POSTGRES_DATABASE}"
		environment = host_environment(question_secret_path, minio_password, database_url)
		native = parse_seed_manifest(captured(runner, seed_argv(False, minio_port), root, environment))
		native_replay = parse_seed_manifest(captured(runner, seed_argv(False, minio_port), root, environment))
		require_replay(native, native_replay, "native")
		webwork = parse_seed_manifest(captured(runner, seed_argv(True, minio_port), root, environment))
		webwork_replay = parse_seed_manifest(captured(runner, seed_argv(True, minio_port), root, environment))
		require_replay(webwork, webwork_replay, "WebWork")
		require_distinct_modes(native, webwork)
		print("WP-R2 host seed: issuing and rendering the retained WebWork publication through PLE")
		run(
			runner,
			["npx", "playwright", "test", "tests/playwright/e2e/wp_r2_host_seed_renderer.spec.ts"],
			root,
			browser_environment(gateway_port, directory / "local-login.txt", webwork.question_id),
		)
		print("WP-R2 host seed: proving an interrupted native marker stops safely")
		create_partial_native_marker(runner, manifest_path, root, native.assignment_id)
		require_partial_marker_refusal(
			runner, seed_argv(False, minio_port), root, environment
		)
		print("WP-R2 host seed and renderer: PASS")
	finally:
		if keep:
			print(f"WP-R2 host seed: preserving {project} and {directory}")
		elif stack_started:
			try:
				cleanup(runner, manifest_path, root)
			except HostSeedE2EError:
				print(f"WP-R2 host seed: cleanup failed; preserving {directory}", file=sys.stderr)
				raise
			shutil.rmtree(directory)
		else:
			shutil.rmtree(directory)


if __name__ == "__main__":
	main()
