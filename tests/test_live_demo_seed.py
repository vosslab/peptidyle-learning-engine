"""Durable contracts for the private Live Demo seed and its safe manifest."""

import json
import pathlib

import local_stack_control.lifecycle
import local_stack_control.live_demo_seed
import local_stack_control.models
import local_stack_control.process


class SeedSourceRunner(local_stack_control.process.CommandRunner):
	"""Record the closed M4 source upload without contacting MinIO."""

	def __init__(self) -> None:
		self.commands: list[tuple[list[str], str | None]] = []

	def run(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
		stdin: str | None = None,
	) -> local_stack_control.models.CommandResult:
		del environment, cwd
		self.commands.append((argv, stdin))
		if "exec" in argv and any("mc stat" in value for value in argv):
			return local_stack_control.models.CommandResult(tuple(argv), 1, "", "not found")
		if "run" in argv and any("mc cp" in value for value in argv):
			return local_stack_control.models.CommandResult(tuple(argv), 0, "", "")
		raise AssertionError(f"unexpected seed source command: {argv}")

	def stream(
		self,
		argv: list[str],
		environment: dict[str, str] | None = None,
		cwd: pathlib.Path | None = None,
	) -> int:
		del argv, environment, cwd
		raise AssertionError("seed source upload does not stream commands")


#============================================
def write_fixed_question_asset(repo_root: pathlib.Path) -> None:
	"""Place the checked-in seed raster in an isolated fixture repository."""
	asset_path = local_stack_control.live_demo_seed.question_asset_path(repo_root)
	asset_path.parent.mkdir(parents=True, exist_ok=True)
	source_path = (
		pathlib.Path(local_stack_control.live_demo_seed.__file__).parent
		/ "live_demo_seed_data"
		/ local_stack_control.live_demo_seed.SEEDED_QUESTION_ASSET_PUBLICATION.filename
	)
	asset_path.write_bytes(source_path.read_bytes())


#============================================
def test_answer_free_manifest_excludes_private_source_and_grading_fields() -> None:
	"""Question Library seed data exposes metadata without source or answer material."""
	serialized = json.dumps(local_stack_control.live_demo_seed.answer_free_manifest()).lower()
	for forbidden in ("correct", "answer", "feedback", "sourceobject", "sourcefilename"):
		assert forbidden not in serialized


#============================================
def test_seeded_restricted_question_asset_key_includes_asset_and_object_identity() -> None:
	"""The private PNG key follows the canonical revision-owned asset address."""
	asset = local_stack_control.live_demo_seed.SEEDED_QUESTION_ASSET_PUBLICATION
	assert local_stack_control.live_demo_seed.restricted_question_asset_object_path() == (
		"questions/PNE-0004/versions/1/restricted-assets/"
		f"{asset.asset_id}/{asset.private_object_id}"
	)


#============================================
def test_seed_sql_records_private_source_binding_before_publication_event(
	tmp_path: pathlib.Path,
) -> None:
	"""The baseline commits the exact private source and Pending publication chain."""
	data_directory = tmp_path / local_stack_control.live_demo_seed.SEED_SOURCE_DIRECTORY
	data_directory.mkdir(parents=True)
	for question in local_stack_control.live_demo_seed.SEEDED_PUBLISHED_QUESTIONS:
		(data_directory / question.source_filename).write_bytes(b"{}")
	write_fixed_question_asset(tmp_path)

	sql = local_stack_control.live_demo_seed.seed_sql(tmp_path)

	assert sql.index("question_revision_source_binding") < sql.index("question_publication_event")
	assert sql.index("question_asset_delivery") < sql.index("question_asset_publication")
	assert "'publish_public_assets'" in sql
	assert "'pending'" in sql


#============================================
def test_seed_inventory_projection_contains_only_aggregate_counts(
	tmp_path: pathlib.Path,
) -> None:
	"""The M4 checker cannot emit a source byte, answer, Account, or object locator."""
	data_directory = tmp_path / local_stack_control.live_demo_seed.SEED_SOURCE_DIRECTORY
	data_directory.mkdir(parents=True)
	for question in local_stack_control.live_demo_seed.SEEDED_PUBLISHED_QUESTIONS:
		(data_directory / question.source_filename).write_bytes(b"{}")
	write_fixed_question_asset(tmp_path)

	sql = local_stack_control.live_demo_seed.inventory_sql(tmp_path).lower()

	assert "select count" in sql and "correctchoice" not in sql and "elena instructor" not in sql


#============================================
def test_seeded_question_source_uses_known_length_upload_and_declared_media_type(
	tmp_path: pathlib.Path,
) -> None:
	"""The M4 seeder makes only fixed private source and raster uploads."""
	env_file = tmp_path / "env.local"
	env_file.write_text("MINIO_ROOT_PASSWORD=private\n", encoding="ascii")
	env_file.chmod(0o600)
	target = local_stack_control.models.ComposeTarget(
		repo_root=tmp_path,
		project="ple-live-demo-browser",
		env_file=env_file,
		compose_files=(),
		provider=local_stack_control.models.ComposeProvider(("podman", "compose"), "podman compose"),
		with_smtp=False,
		env_setting_names=(),
	)
	data_directory = tmp_path / local_stack_control.live_demo_seed.SEED_SOURCE_DIRECTORY
	data_directory.mkdir(parents=True)
	for question in local_stack_control.live_demo_seed.SEEDED_PUBLISHED_QUESTIONS:
		(data_directory / question.source_filename).write_text("{}", encoding="ascii")
	write_fixed_question_asset(tmp_path)

	runner = SeedSourceRunner()
	local_stack_control.lifecycle.seed_live_demo_source_objects(target, runner)

	uploads = [command for command in runner.commands if "run" in command[0]]
	assert len(uploads) == len(local_stack_control.live_demo_seed.SEEDED_PUBLISHED_QUESTIONS) + 1
	argv, source = uploads[0]
	assert "createbuckets" in argv
	assert any("--disable-multipart" in value for value in argv)
	assert any("Content-Type: $2" in value for value in argv)
	assert any("ple-record-v1=$1" in value for value in argv)
	assert source == "{}"
	asset_upload = uploads[-1]
	assert any("private-content" in value for value in asset_upload[0])
	assert not any("public-assets" in value for value in asset_upload[0])
	assert "base64 -d" in " ".join(asset_upload[0])
