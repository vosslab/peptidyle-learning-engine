"""Private Chapter One publication and replay boundary."""

import dataclasses
import os
import pathlib

import local_stack_control.consumer
import local_stack_control.chapter_one_manifest
import local_stack_control.env_file
import local_stack_control.models
import local_stack_control.process
import local_stack_control.private_files


PUBLISHER_RUNTIME_ENVIRONMENT_NAMES = (
	"PATH",
	"HOME",
	"CARGO_HOME",
	"RUSTUP_HOME",
	"TMPDIR",
	"LANG",
	"LC_ALL",
	"LC_CTYPE",
)


@dataclasses.dataclass(frozen=True)
class ChapterOneSeedRequest:
	"""One explicit, host-owned invocation of the immutable Chapter One publisher."""

	repo_root: pathlib.Path
	database_url: str
	instructor_id: str
	student_id: str
	s3_endpoint: str
	aws_access_key_id: str
	aws_secret_access_key: str
	question_id_secret_file: pathlib.Path
	manifest_path: pathlib.Path
	existing_manifest_path: pathlib.Path | None


#============================================
def require_private_existing_manifest(path: pathlib.Path | None) -> None:
	"""Require replay evidence to be a current-user private regular file."""
	if path is not None:
		local_stack_control.consumer.require_private_regular_file(
			path,
			"Chapter One replay manifest",
		)


#============================================
def require_private_output_target(path: pathlib.Path) -> None:
	"""Reject a non-private pre-existing destination before atomic replacement."""
	if path.exists() or path.is_symlink():
		local_stack_control.consumer.require_private_regular_file(
			path,
			"Chapter One manifest output",
		)
	if not path.parent.is_dir():
		raise local_stack_control.models.ControllerError(
			"Chapter One manifest output directory is unavailable"
		)


#============================================
def publication_environment(request: ChapterOneSeedRequest) -> dict[str, str]:
	"""Return Cargo runtime context plus the publisher's three explicit capabilities."""
	base_environment = local_stack_control.env_file.sanitized_runtime_environment(
		dict(os.environ)
	)
	environment = {
		name: base_environment[name]
		for name in PUBLISHER_RUNTIME_ENVIRONMENT_NAMES
		if name in base_environment
	}
	environment["AWS_ACCESS_KEY_ID"] = request.aws_access_key_id
	environment["AWS_SECRET_ACCESS_KEY"] = request.aws_secret_access_key
	environment["PLE_QUESTION_ID_SECRET_FILE"] = str(request.question_id_secret_file)
	return environment


#============================================
def safe_seed_argv(request: ChapterOneSeedRequest) -> list[str]:
	"""Build the publisher invocation without database URLs or object credentials in argv."""
	argv = [
		"cargo", "tools", "e2e-seed", "--chapter-one-pilot", "--apply-migrations",
		"--instructor", request.instructor_id,
		"--student", request.student_id, "--s3-endpoint", request.s3_endpoint,
		"--s3-region", "us-east-1", "--private-content-bucket", "private-content",
	]
	if request.existing_manifest_path is not None:
		argv.extend(("--chapter-one-existing-manifest", str(request.existing_manifest_path)))
	return argv


#============================================
def publish_with_runner(
	request: ChapterOneSeedRequest,
	runner: local_stack_control.process.CommandRunner,
) -> None:
	"""Publish through the injected process boundary with secrets limited to child environment."""
	require_private_existing_manifest(request.existing_manifest_path)
	local_stack_control.consumer.require_private_regular_file(
		request.question_id_secret_file, "Question ID secret file"
	)
	require_private_output_target(request.manifest_path)
	environment = publication_environment(request)
	environment["PLE_MIGRATION_DATABASE_URL"] = request.database_url
	result = runner.run(safe_seed_argv(request), environment, request.repo_root)
	if not result.ok():
		raise local_stack_control.models.ControllerError(
			"Chapter One publisher failed; retained stack resources are available for diagnostics"
		)
	manifest_bytes = result.stdout.encode("utf-8")
	local_stack_control.chapter_one_manifest.parse_manifest_bytes(manifest_bytes)
	local_stack_control.private_files.write_atomic_file(request.manifest_path, manifest_bytes, 0o600)
