"""Offline contracts for private Chapter One publication evidence."""

import json
import pathlib
import stat

import pytest

import local_stack_control.chapter_one
import local_stack_control.models
import local_stack_control.process


class PublisherRunner(local_stack_control.process.CommandRunner):
	"""Record one safe publisher call and return injected output."""

	def __init__(self, returncode: int, stdout: str) -> None:
		self.returncode = returncode
		self.stdout = stdout
		self.argv: tuple[str, ...] | None = None
		self.environment: dict[str, str] | None = None

	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		"""Record the captured publisher boundary."""
		if stdin is not None:
			raise AssertionError("publisher stdin must be absent")
		self.argv = tuple(argv)
		self.environment = None if environment is None else dict(environment)
		return local_stack_control.models.CommandResult(
			tuple(argv),
			self.returncode,
			self.stdout,
			"",
		)

	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		"""Refuse an unowned streaming publisher path."""
		raise AssertionError("publisher must use captured execution")


#============================================
def request(tmp_path: pathlib.Path, existing: pathlib.Path | None = None) -> local_stack_control.chapter_one.ChapterOneSeedRequest:
	"""Build one private publisher request with no real service dependency."""
	secret = tmp_path / "question-secret"
	secret.write_text("secret", encoding="ascii")
	secret.chmod(0o600)
	result = local_stack_control.chapter_one.ChapterOneSeedRequest(
		repo_root=tmp_path,
		database_url="postgres://private",
        instructor_id="instructor",
		student_id="student",
		s3_endpoint="http://127.0.0.1:9000",
		aws_access_key_id="minio",
		aws_secret_access_key="private",
		question_id_secret_file=secret,
		manifest_path=tmp_path / "published.json",
		existing_manifest_path=existing,
	)
	return result


#============================================
def valid_manifest_bytes() -> bytes:
	"""Build one complete answer-free manifest for publisher-output behavior tests."""
	chapters: list[dict[str, object]] = []
	question_number = 1
	for chapter_number, (slug, question_slugs) in enumerate(
		local_stack_control.chapter_one_manifest.EXPECTED_CHAPTERS,
		start=1,
	):
		questions: list[dict[str, str]] = []
		for question_slug in question_slugs:
			questions.append(
				{
					"slug": question_slug,
					"displayId": f"000-000{question_number}",
					"problemId": f"00000000-0000-0000-0000-{question_number:012d}",
					"versionId": f"10000000-0000-0000-0000-{question_number:012d}",
				}
			)
			question_number += 1
		chapters.append(
			{
				"slug": slug,
				"courseId": f"20000000-0000-0000-0000-{chapter_number:012d}",
				"assignmentId": f"30000000-0000-0000-0000-{chapter_number:012d}",
				"enrollmentId": f"40000000-0000-0000-0000-{chapter_number:012d}",
				"questions": questions,
			}
		)
	encoded = json.dumps({"chapters": chapters}, separators=(",", ":")).encode("ascii")
	return encoded


#============================================
def test_publication_environment_keeps_runtime_context_and_explicit_capabilities(
	tmp_path: pathlib.Path,
	monkeypatch: pytest.MonkeyPatch,
) -> None:
	"""The publisher receives its Cargo context and its three supplied capabilities."""
	runtime_environment = {
		"PATH": "/toolchain/bin",
		"HOME": "/toolchain/home",
		"CARGO_HOME": "/toolchain/cargo",
		"RUSTUP_HOME": "/toolchain/rustup",
		"TMPDIR": "/toolchain/tmp",
		"LANG": "C",
		"LC_ALL": "C",
		"LC_CTYPE": "C",
	}
	for name, value in runtime_environment.items():
		monkeypatch.setenv(name, value)
	monkeypatch.setenv("AWS_SESSION_TOKEN", "ambient-session")
	monkeypatch.setenv("PLE_GATEWAY_SECRET", "ambient-gateway-secret")
	monkeypatch.setenv("DEPLOYMENT_TOKEN", "ambient-deployment-token")

	environment = local_stack_control.chapter_one.publication_environment(request(tmp_path))

	assert {
		name: environment[name]
		for name in (*runtime_environment, *(
			"AWS_ACCESS_KEY_ID",
			"AWS_SECRET_ACCESS_KEY",
			"PLE_QUESTION_ID_SECRET_FILE",
		))
	} == {
		**runtime_environment,
		"AWS_ACCESS_KEY_ID": "minio",
		"AWS_SECRET_ACCESS_KEY": "private",
		"PLE_QUESTION_ID_SECRET_FILE": str(tmp_path / "question-secret"),
	}
	assert all(
		name not in environment
		for name in ("AWS_SESSION_TOKEN", "PLE_GATEWAY_SECRET", "DEPLOYMENT_TOKEN")
	)


#============================================
def test_publish_uses_list_argv_and_atomically_replaces_private_manifest(
	tmp_path: pathlib.Path,
) -> None:
	"""The only publisher path keeps credentials off argv and writes private evidence."""
	existing = tmp_path / "existing.json"
	existing.write_text("{}", encoding="ascii")
	existing.chmod(0o600)
	publication = request(tmp_path, existing)
	runner = PublisherRunner(0, valid_manifest_bytes().decode("ascii"))
	local_stack_control.chapter_one.publish_with_runner(publication, runner)

	assert runner.argv is not None
	assert "--chapter-one-existing-manifest" in runner.argv
	assert "--database-url" not in runner.argv
	assert publication.database_url not in runner.argv
	assert publication.aws_secret_access_key not in runner.argv
	assert runner.environment is not None
	assert runner.environment["PLE_MIGRATION_DATABASE_URL"] == publication.database_url
	assert stat.S_IMODE(publication.manifest_path.stat().st_mode) == 0o600


#============================================
def test_publish_removes_partial_private_output_after_a_publisher_failure(
	tmp_path: pathlib.Path,
) -> None:
	"""A failed publisher cannot leave an ambiguous partial replay manifest behind."""
	publication = request(tmp_path)
	publication.manifest_path.write_text("previous", encoding="ascii")
	publication.manifest_path.chmod(0o600)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.chapter_one.publish_with_runner(
			publication,
			PublisherRunner(1, "partial"),
		)

	assert publication.manifest_path.read_text(encoding="ascii") == "previous"


#============================================
def test_publish_rejects_malformed_success_output_and_retains_prior_manifest(
	tmp_path: pathlib.Path,
) -> None:
	"""A zero-exit publisher result needs complete manifest evidence before replacement."""
	publication = request(tmp_path)
	publication.manifest_path.write_text("previous", encoding="ascii")
	publication.manifest_path.chmod(0o600)

	with pytest.raises(local_stack_control.models.ControllerError):
		local_stack_control.chapter_one.publish_with_runner(
			publication,
			PublisherRunner(0, '{"chapters":['),
		)

	assert publication.manifest_path.read_text(encoding="ascii") == "previous"
