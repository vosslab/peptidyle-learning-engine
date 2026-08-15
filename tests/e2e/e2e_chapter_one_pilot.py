#!/usr/bin/env python3
"""Run the isolated Chapter One publication oracle through typed stack ownership."""

import base64
import dataclasses
import hashlib
import os
import pathlib
import secrets
import subprocess
import sys
import tempfile
import time

# This executable lives below the repository import root rather than in the
# package itself, so direct documented execution needs one explicit path anchor.
SCRIPT_REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SCRIPT_REPOSITORY_ROOT))

import local_stack_control.chapter_one
import local_stack_control.models
import local_stack_control.process


POSTGRES_USER = "ple_chapter_one_pilot"
POSTGRES_DATABASE = "ple_chapter_one_pilot"
TENANT_ID = "00000000-0000-0000-0000-000000000100"
INSTRUCTOR_ID = "00000000-0000-0000-0000-000000000101"
STUDENT_ID = "00000000-0000-0000-0000-000000000102"


#============================================
def repo_root() -> pathlib.Path:
	"""Return the repository root containing this checked-in E2E owner."""
	result = SCRIPT_REPOSITORY_ROOT
	return result


#============================================
def private_file(path: pathlib.Path, content: str | bytes) -> None:
	"""Write one new current-user-only file without a permissive creation window."""
	file_descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
	with os.fdopen(file_descriptor, "wb") as output:
		if isinstance(content, str):
			output.write(content.encode("ascii"))
		else:
			output.write(content)


#============================================
def random_port(base: int) -> int:
	"""Choose a disposable owner port from its documented private range."""
	result = base + secrets.randbelow(500)
	return result


#============================================
def run(argv: list[str], root: pathlib.Path, environment: dict[str, str] | None = None) -> None:
	"""Run one explicit E2E step and stop at its actual failing boundary."""
	completed = subprocess.run(argv, check=False, cwd=root, env=environment)
	if completed.returncode != 0:
		raise local_stack_control.models.ControllerError(
			"Chapter 1 pilot E2E command failed: " + " ".join(argv)
		)


#============================================
def captured(argv: list[str], root: pathlib.Path) -> str:
	"""Run one SQL assertion query and return its scalar output."""
	completed = subprocess.run(argv, check=False, capture_output=True, cwd=root, text=True)
	if completed.returncode != 0:
		raise local_stack_control.models.ControllerError(
			"Chapter 1 pilot E2E query failed: " + " ".join(argv)
		)
	result = completed.stdout.strip()
	return result


#============================================
def compose_argv(manifest_path: pathlib.Path, arguments: list[str]) -> list[str]:
	"""Form an exact typed adapter call for one non-cleanup Compose operation."""
	result = [
		sys.executable,
		"-m",
		"local_stack_control._consumer_cli",
		"compose",
		"--manifest",
		str(manifest_path),
	]
	result.extend(arguments)
	return result


#============================================
def create_buckets_argv(manifest_path: pathlib.Path) -> list[str]:
	"""Run the declared bucket service for its typed topology cleanup."""
	result = compose_argv(manifest_path, ["up", "--no-deps", "createbuckets"])
	return result


#============================================
def wait_for_service(manifest_path: pathlib.Path, root: pathlib.Path, service: str) -> None:
	"""Wait for PostgreSQL or MinIO through the project-scoped adapter."""
	for _ in range(30):
		if service == "postgres":
			argv = compose_argv(
				manifest_path,
				["exec", "-T", "postgres", "pg_isready", "-U", POSTGRES_USER, "-d", POSTGRES_DATABASE],
			)
		else:
			argv = compose_argv(manifest_path, ["exec", "-T", "minio", "mc", "ready", "local"])
		completed = subprocess.run(argv, check=False, cwd=root)
		if completed.returncode == 0:
			return
		time.sleep(1)
	raise local_stack_control.models.ControllerError(
		f"Chapter 1 pilot E2E disposable {service} did not become ready"
	)


#============================================
def expect_query(
	manifest_path: pathlib.Path,
	root: pathlib.Path,
	expected: str,
	query: str,
) -> None:
	"""Require one database fact from the isolated project only."""
	argv = compose_argv(
		manifest_path,
		[
			"exec",
			"-T",
			"postgres",
			"psql",
			"-v",
			"ON_ERROR_STOP=1",
			"-U",
			POSTGRES_USER,
			"-d",
			POSTGRES_DATABASE,
			"-Atc",
			query,
		],
	)
	actual = captured(argv, root)
	if actual != expected:
		raise local_stack_control.models.ControllerError(
			f"Chapter 1 pilot E2E database evidence differed: expected {expected!r}, got {actual!r}"
		)


#============================================
def write_disposable_target(
	directory: pathlib.Path,
	postgres_password: str,
	minio_password: str,
	postgres_port: int,
	minio_port: int,
) -> tuple[str, pathlib.Path, pathlib.Path]:
	"""Create one private typed disposable target and its object-store configuration."""
	project = "ple_chapter_one_pilot_" + secrets.token_hex(12)
	capability_path = directory / "disposable.capability"
	capability = secrets.token_bytes(32)
	private_file(capability_path, capability)
	capability_digest = hashlib.sha256(capability).hexdigest()
	env_path = directory / "env.local"
	env_content = (
		f"POSTGRES_USER={POSTGRES_USER}\n"
		f"POSTGRES_PASSWORD={postgres_password}\n"
		f"POSTGRES_DB={POSTGRES_DATABASE}\n"
		f"PLE_POSTGRES_HOST_PORT={postgres_port}\n"
		"MINIO_ROOT_USER=ple-chapter-one-pilot\n"
		f"MINIO_ROOT_PASSWORD={minio_password}\n"
		f"PLE_MINIO_API_HOST_PORT={minio_port}\n"
		f"PLE_DISPOSABLE_CAPABILITY_SHA256={capability_digest}\n"
	)
	private_file(env_path, env_content)
	manifest_path = directory / "disposable.manifest"
	manifest_content = (
		"OWNER=chapter-one-pilot\n"
		f"PROJECT={project}\n"
		f"ENV_FILE={env_path}\n"
		f"CAPABILITY_FILE={capability_path}\n"
	)
	private_file(manifest_path, manifest_content)
	return project, manifest_path, capability_path


#============================================
def cleanup(manifest_path: pathlib.Path, root: pathlib.Path) -> None:
	"""Run the controller's exact project cleanup rather than direct Compose down."""
	run(
		[
			sys.executable,
			"-m",
			"local_stack_control._consumer_cli",
			"cleanup",
			"--manifest",
			str(manifest_path),
		],
		root,
	)


#============================================
def main() -> None:
	"""Publish, replay, and verify the first teaching corpus in a disposable stack."""
	root = repo_root()
	postgres_password = secrets.token_urlsafe(24)
	minio_password = secrets.token_urlsafe(24)
	postgres_port = random_port(50500)
	minio_port = random_port(51000)
	directory = pathlib.Path(tempfile.mkdtemp(prefix="ple-chapter-one-pilot-"))
	os.chmod(directory, 0o700)
	project, manifest_path, _capability_path = write_disposable_target(
		directory,
		postgres_password,
		minio_password,
		postgres_port,
		minio_port,
	)
	question_secret_path = directory / "question-id-secret"
	question_secret = base64.urlsafe_b64encode(secrets.token_bytes(32)).rstrip(b"=")
	private_file(question_secret_path, question_secret)
	stack_started = False
	publisher_runner = local_stack_control.process.SubprocessRunner()
	keep = os.environ.get("PLE_E2E_KEEP") == "1"
	try:
		print("Chapter 1 pilot E2E: validating the tracked source corpus")
		run(["cargo", "tools", "pilot-content"], root)
		print("Chapter 1 pilot E2E: starting isolated PostgreSQL and MinIO")
		stack_started = True
		run(compose_argv(manifest_path, ["up", "-d", "postgres", "minio"]), root)
		wait_for_service(manifest_path, root, "postgres")
		wait_for_service(manifest_path, root, "minio")
		run(create_buckets_argv(manifest_path), root)
		database_url = (
			f"postgres://{POSTGRES_USER}:{postgres_password}@127.0.0.1:{postgres_port}/"
			f"{POSTGRES_DATABASE}"
		)
		first_manifest = directory / "first.json"
		request = local_stack_control.chapter_one.ChapterOneSeedRequest(
			repo_root=root,
			database_url=database_url,
			tenant_id=TENANT_ID,
			instructor_id=INSTRUCTOR_ID,
			student_id=STUDENT_ID,
			s3_endpoint=f"http://127.0.0.1:{minio_port}",
			aws_access_key_id="ple-chapter-one-pilot",
			aws_secret_access_key=minio_password,
			question_id_secret_file=question_secret_path,
			manifest_path=first_manifest,
			existing_manifest_path=None,
		)
		print("Chapter 1 pilot E2E: publishing two assignments and eight immutable questions")
		local_stack_control.chapter_one.publish_with_runner(request, publisher_runner)
		second_manifest = directory / "second.json"
		replay_request = dataclasses.replace(
			request,
			manifest_path=second_manifest,
			existing_manifest_path=first_manifest,
		)
		print("Chapter 1 pilot E2E: verifying exact idempotent rerun")
		local_stack_control.chapter_one.publish_with_runner(replay_request, publisher_runner)
		run([sys.executable, "tests/e2e/e2e_chapter_one_manifest.py", str(first_manifest), str(second_manifest)], root)
		expect_query(manifest_path, root, "7", f"SELECT count(*) FROM course WHERE tenant_id = '{TENANT_ID}';")
		expect_query(manifest_path, root, "7", f"SELECT count(*) FROM assignment WHERE tenant_id = '{TENANT_ID}';")
		expect_query(manifest_path, root, "13", f"SELECT count(*) FROM assignment_item WHERE tenant_id = '{TENANT_ID}' AND delivery_state = 'active';")
		expect_query(manifest_path, root, "4|4", "SELECT count(*) FILTER (WHERE backend = 'native') || '|' || count(*) FILTER (WHERE backend = 'webwork') FROM problem_version;")
		expect_query(manifest_path, root, "8", "SELECT count(*) FROM published_source_artifact;")
		expect_query(manifest_path, root, "7", f"SELECT count(*) FROM enrollment WHERE tenant_id = '{TENANT_ID}';")
		expect_query(manifest_path, root, "Biochemistry Chapter 1 Mastery|4\nGenetics Chapter 1 Mastery|4", f"SELECT assignment.title || '|' || count(assignment_item.assignment_item_id) FROM assignment JOIN assignment_item USING (tenant_id, assignment_id) WHERE assignment.tenant_id = '{TENANT_ID}' AND assignment.title IN ('Biochemistry Chapter 1 Mastery', 'Genetics Chapter 1 Mastery') GROUP BY assignment.title ORDER BY assignment.title;")
		print("Chapter 1 pilot E2E: PASS")
	finally:
		if keep:
			print(f"Chapter 1 pilot E2E: preserving disposable project {project} ({manifest_path})")
		elif stack_started:
			cleanup(manifest_path, root)
			for child in directory.iterdir():
				child.unlink()
			directory.rmdir()
		else:
			for child in directory.iterdir():
				child.unlink()
			directory.rmdir()


if __name__ == "__main__":
	main()
