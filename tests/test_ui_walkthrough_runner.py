"""Focused offline tests for the public UI walkthrough runner lifecycle."""

import dataclasses
import importlib
import json
import os
import pathlib
import shutil
import stat
import sys
import tempfile
import unittest
from unittest import mock


WALKTHROUGH_DIRECTORY = pathlib.Path(__file__).resolve().parent / "walkthrough"
sys.path.insert(0, str(WALKTHROUGH_DIRECTORY))
walkthrough = importlib.import_module("walklib.runner")
report_contract = importlib.import_module("walklib.v2_report_contract")


def instructor_arrangement_output() -> str:
	"""Return one canonical public arrangement as inline test input."""
	payload = {
		"arrangements": [
			{
				"label": "api-retry-corpus-publication",
				"problemId": "123e4567-e89b-12d3-a456-426614174001",
				"versionId": "123e4567-e89b-12d3-a456-426614174002",
				"catalogSearchTitle": "Pilot retry corpus pilotref123e4567e89b12d3a456426614174000",
			}
		]
	}
	return json.dumps(payload, separators=(",", ":")) + "\n"


def instructor_state_json() -> str:
	"""Return canonical public course, roster, and assignment evidence."""
	fragments = [
		{
			"schemaVersion": 2,
			"journey": "J11",
			"status": "PASS",
			"elapsedMs": 1,
			"courseId": "123e4567-e89b-12d3-a456-426614174000",
			"visibleOutcomeCodes": ["visible_course_created", "visible_course_opened"],
			"diagnostics": [],
		},
		{
			"schemaVersion": 2,
			"journey": "J12",
			"status": "PASS",
			"elapsedMs": 2,
			"courseId": "123e4567-e89b-12d3-a456-426614174000",
			"visibleOutcomeCodes": ["visible_local_student_active"],
			"diagnostics": [],
		},
		{
			"schemaVersion": 2,
			"journey": "J13",
			"status": "PASS",
			"elapsedMs": 3,
			"courseId": "123e4567-e89b-12d3-a456-426614174000",
			"assignmentId": "123e4567-e89b-12d3-a456-426614174004",
			"problemId": "123e4567-e89b-12d3-a456-426614174001",
			"versionId": "123e4567-e89b-12d3-a456-426614174002",
			"visibleOutcomeCodes": [
				"visible_assignment_created",
				"visible_catalog_problem_selected",
				"visible_mastery_policy",
			],
			"diagnostics": [],
		},
	]
	return json.dumps(fragments, separators=(",", ":")) + "\n"


def visible_outcome_output() -> str:
	"""Return one canonical public report as inline test input."""
	journeys = [
		{
			"journey": journey,
			"status": "PASS",
			"elapsedMs": index + 1,
			"visibleOutcomeCodes": codes,
			"diagnostics": [],
		}
		for index, (journey, codes) in enumerate(report_contract.EXPECTED_JOURNEYS)
	]
	payload = {
		"schemaVersion": 2,
		"status": "PASS",
		"masterSeed": 42,
		"stage": "complete",
		"elapsedMs": sum(row["elapsedMs"] for row in journeys),
		"arrangements": [{"label": "api-retry-corpus-publication"}],
		"journeys": journeys,
	}
	return json.dumps(payload, separators=(",", ":")) + "\n"


class FakeCommands:
	"""Capture offline command requests while returning successful results."""

	def __init__(self) -> None:
		self.commands: list[tuple[list[str], dict[str, str] | None]] = []

	def __call__(
		self,
		command: list[str],
		environ: dict[str, str] | None,
	) -> object:
		self.commands.append((command, environ))
		return walkthrough.CommandResult(0, "", "")


class UiWalkthroughRunnerTests(unittest.TestCase):
	"""Prove preflight and cleanup decisions without launching Podman or Playwright."""

	def make_repository(self, temporary_directory: pathlib.Path) -> pathlib.Path:
		"""Create only the small local files consumed during offline preflight."""
		repository_root = temporary_directory / "repository"
		env_directory = repository_root / "containers"
		env_directory.mkdir(parents=True)
		(env_directory / "env.local").write_text("PLE_GATEWAY_HOST_PORT=3010\n", encoding="ascii")
		return repository_root

	def resolve(
		self,
		repository_root: pathlib.Path,
		extra_arguments: list[str],
	) -> object:
		"""Resolve a baseline valid CLI plus one focused build-selection variation."""
		args = walkthrough.parse_args(["--master-seed", "42"] + extra_arguments)
		inputs = walkthrough.resolve_inputs(args, repository_root)
		return inputs


	def test_auto_reuses_safe_existing_publish_outputs(self) -> None:
		"""AUTO reuses a complete safe browser bundle."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			dist_directory = repository_root / "dist"
			dist_directory.mkdir()
			(dist_directory / "index.html").write_text("index", encoding="ascii")
			(dist_directory / "main.js").write_text("main", encoding="ascii")
			inputs = self.resolve(repository_root, [])
			self.assertTrue(walkthrough.reuse_existing_dist(inputs, repository_root))

	def test_auto_builds_when_publish_output_is_missing(self) -> None:
		"""AUTO chooses a launcher build rather than claiming incomplete dist is reusable."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			self.assertFalse(walkthrough.reuse_existing_dist(inputs, repository_root))

	def test_build_refreshes_a_safe_existing_publish_output(self) -> None:
		"""--build overrides AUTO reuse for a fresh browser bundle."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			dist_directory = repository_root / "dist"
			dist_directory.mkdir()
			(dist_directory / "index.html").write_text("index", encoding="ascii")
			(dist_directory / "main.js").write_text("main", encoding="ascii")
			inputs = self.resolve(repository_root, ["--build"])
			self.assertFalse(walkthrough.reuse_existing_dist(inputs, repository_root))

	def test_invalid_boundaries_fail_before_podman(self) -> None:
		"""Malformed seed, report, port, and existing credential inputs each fail closed offline."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			with self.assertRaisesRegex(walkthrough.RunnerError, "master-seed"):
				self.resolve(repository_root, ["--master-seed", "not-a-number"])
			with self.assertRaisesRegex(walkthrough.RunnerError, "report-file"):
				self.resolve(repository_root, ["--report-file", "../report.json"])
			inputs = self.resolve(repository_root, [])
			with self.assertRaisesRegex(walkthrough.RunnerError, "PLE_GATEWAY_HOST_PORT"):
				walkthrough.effective_gateway_port(inputs, {"PLE_GATEWAY_HOST_PORT": "not-a-port"})
			with self.assertRaisesRegex(walkthrough.RunnerError, "COMPOSE_PROJECT_NAME"):
				walkthrough.validate_compose_project_name(
					inputs,
					{"COMPOSE_PROJECT_NAME": "different-project"},
				)
			login_file = inputs.env_file.parent / "local-login.txt"
			login_file.write_text("student@example.test\n", encoding="ascii")
			login_file.chmod(0o644)
			with self.assertRaisesRegex(walkthrough.RunnerError, "mode 0600"):
				walkthrough.validate_existing_credential_file(inputs)
			login_file.unlink()
			login_file.symlink_to("outside-login.txt")
			with self.assertRaisesRegex(walkthrough.RunnerError, "must not be a symlink"):
				walkthrough.validate_existing_credential_file(inputs)
			inputs.env_file.unlink()
			inputs.env_file.symlink_to("outside-env.local")
			args = walkthrough.parse_args(["--master-seed", "42"])
			with self.assertRaisesRegex(walkthrough.RunnerError, "env file must not be a symlink"):
				walkthrough.resolve_inputs(args, repository_root)

	def test_exact_existing_container_label_is_refused(self) -> None:
		"""Either exact containers project label prevents cleanup ownership before launch."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])

			def existing_container(
				command: list[str],
				environ: dict[str, str] | None,
			) -> object:
				return walkthrough.CommandResult(0, "stopped-container\n", "")

			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, existing_container)
			with self.assertRaisesRegex(walkthrough.RunnerError, "already has containers"):
				runner.assert_no_existing_stack()

	def test_inherited_gateway_port_overrides_selected_env_file(self) -> None:
		"""The launcher-compatible inherited gateway port takes precedence over the env file value."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			port = walkthrough.effective_gateway_port(inputs, {"PLE_GATEWAY_HOST_PORT": "3020"})
			self.assertEqual(port, 3020)

	def test_arrangement_parser_accepts_canonical_public_input_and_rejects_trailing_data(self) -> None:
		"""The Python boundary accepts public evidence and rejects extra child output."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			arrangements = runner.parse_arrangement_output(instructor_arrangement_output())
			self.assertEqual(arrangements[0]["label"], "api-retry-corpus-publication")
			with self.assertRaisesRegex(walkthrough.RunnerError, "invalid output"):
				runner.parse_arrangement_output(instructor_arrangement_output() + "unexpected\n")

	def test_arrangement_handoff_keeps_child_output_and_credentials_private(self) -> None:
		"""The fixed child passes only parsed public IDs to Playwright after successful arrangement."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			arranger = repository_root / "node_modules" / "tsx" / "dist"
			arranger.mkdir(parents=True)
			tsx = arranger / "cli.mjs"
			tsx.write_text("export {};\n", encoding="ascii")
			bin_directory = repository_root / "node_modules" / ".bin"
			bin_directory.mkdir()
			(bin_directory / "tsx").symlink_to("../tsx/dist/cli.mjs")
			inputs = self.resolve(repository_root, [])

			def arrangement_command(
				command: list[str],
				environ: dict[str, str] | None,
			) -> object:
				if environ is not None and environ.get("PLE_UI_WALKTHROUGH_ARRANGER_CHILD") == "1":
					return walkthrough.CommandResult(0, instructor_arrangement_output(), "student-secret")
				return walkthrough.CommandResult(0, "", "")

			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, arrangement_command)
			environment = {"PLE_UI_WALKTHROUGH_LIVE_CREDENTIAL_FILE": "private-login.txt"}
			runner.arrange(environment)
			self.assertNotIn("PLE_UI_WALKTHROUGH_LIVE_MANIFEST_FILE", environment)
			self.assertNotIn("student-secret", json.dumps(runner.arrangements))

	def test_instructor_arrangement_keeps_unique_catalog_title_out_of_report_state(self) -> None:
		"""The J13-only title is validated, passed privately, and stripped from report arrangements."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.arrangements = runner.parse_arrangement_output(instructor_arrangement_output())
			environment: dict[str, str] = {}
			runner.prepare_journey_state(environment)
			runner.prepare_report_directory()
			runner.finish(False)
			report_text = runner.report_path.read_text(encoding="ascii")
			public_state = environment["PLE_UI_WALKTHROUGH_ARRANGEMENTS_JSON"] + report_text
			self.assertNotIn("Pilot retry corpus", public_state)
			with self.assertRaisesRegex(walkthrough.RunnerError, "invalid output"):
				runner.parse_arrangement_output(
					instructor_arrangement_output().replace("Pilot retry corpus", "peptide")
				)

	def test_private_journey_state_is_mode_0600_and_removed_on_finish(self) -> None:
		"""The renderer handoff stays outside test-results and cleanup removes its exact state root."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.arrangements = runner.parse_arrangement_output(instructor_arrangement_output())
			environment: dict[str, str] = {}
			runner.prepare_journey_state(environment)
			self.assertNotIn("test-results", environment["PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE"])
			self.assertEqual(stat.S_IMODE(runner.journey_state_file.stat().st_mode), 0o600)
			self.assertEqual(stat.S_IMODE(runner.j1_checkpoint_file.stat().st_mode), 0o600)
			self.assertEqual(
				stat.S_IMODE(runner.instructor_setup_checkpoint_file.stat().st_mode),
				0o600,
			)
			self.assertEqual(stat.S_IMODE(runner.private_state_directory.stat().st_mode), 0o700)
			self.assertNotIn("student", environment["PLE_UI_WALKTHROUGH_ARRANGEMENTS_JSON"])
			runner.prepare_report_directory()
			runner.finish(False)
			self.assertFalse(runner.private_state_directory is not None)

	def test_instructor_handoff_exports_only_matching_bounded_public_ids(self) -> None:
		"""The runner rejects altered outcome, time, and corpus values before student children exist."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.arrangements = runner.parse_arrangement_output(instructor_arrangement_output())
			environment: dict[str, str] = {}
			runner.prepare_journey_state(environment)
			state_path = pathlib.Path(environment["PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE"])
			state_path.write_text(instructor_state_json(), encoding="ascii")
			runner.hand_off_instructor_setup(environment)
			self.assertEqual(
				environment["PLE_UI_WALKTHROUGH_LIVE_COURSE_ID"],
				"123e4567-e89b-12d3-a456-426614174000",
			)
			self.assertNotIn("PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY", environment)
			for hostile in (
				instructor_state_json().replace("visible_mastery_policy", "wrong"),
				instructor_state_json().replace('"elapsedMs":3', '"elapsedMs":1800001'),
				instructor_state_json().replace("426614174001", "426614174005"),
			):
				state_path.write_text(hostile, encoding="ascii")
				with self.assertRaisesRegex(walkthrough.RunnerError, "public-ID handoff"):
					runner.hand_off_instructor_setup({})
			runner.prepare_report_directory()
			runner.finish(False)

	def test_instructor_handoff_rejects_parent_replacement_before_descriptor_child_open(self) -> None:
		"""The descriptor-relative child open cannot be redirected by replacing its named parent."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			runner = walkthrough.WalkthroughRunner(self.resolve(repository_root, []), repository_root, {}, FakeCommands())
			runner.arrangements = runner.parse_arrangement_output(instructor_arrangement_output())
			environment: dict[str, str] = {}
			runner.prepare_journey_state(environment)
			state_path = pathlib.Path(environment["PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE"])
			state_path.write_text(instructor_state_json(), encoding="ascii")
			original_parent = state_path.parent
			moved_parent = original_parent.with_name(f"{original_parent.name}-moved")
			replacement_parent = original_parent.with_name(f"{original_parent.name}-replacement")
			original_open = walkthrough.os.open
			replaced = False

			def replace_parent_before_child(
				path: os.PathLike[str] | str,
				flags: int,
				mode: int = 0o777,
				*,
				dir_fd: int | None = None,
			) -> int:
				nonlocal replaced
				if path == "journeys.json" and dir_fd is not None and not replaced:
					replaced = True
					original_parent.rename(moved_parent)
					replacement_parent.mkdir(mode=0o700)
					replacement_parent.rename(original_parent)
				return original_open(path, flags, mode, dir_fd=dir_fd)

			with mock.patch.object(walkthrough.os, "open", side_effect=replace_parent_before_child):
				with self.assertRaisesRegex(walkthrough.RunnerError, "public-ID handoff"):
					runner.hand_off_instructor_setup({})
			self.assertTrue(replaced)
			shutil.rmtree(moved_parent)
			shutil.rmtree(original_parent)

	def test_private_playwright_artifacts_are_siblings_of_state_and_removed_with_it(self) -> None:
		"""A failed walkthrough artifact stays in runner-private state rather than shared test-results."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.arrangements = runner.parse_arrangement_output(instructor_arrangement_output())
			environment: dict[str, str] = {}
			runner.prepare_journey_state(environment)
			artifact_directory = runner.private_state_directory / "journey-artifacts"
			artifact_directory.mkdir(mode=0o700)
			(artifact_directory / "error-context.md").write_text("sensitive", encoding="ascii")
			self.assertNotIn("test-results", str(artifact_directory))
			runner.prepare_report_directory()
			runner.finish(False)
			self.assertFalse(artifact_directory.exists())

	def test_success_report_is_the_renderer_schema_without_a_nested_duplicate(self) -> None:
		"""A successful J1 run persists the canonical renderer record as the complete report."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			runner.visible_outcomes = runner.parse_visible_outcome_output(visible_outcome_output())
			self.assertEqual(runner.finish(True), 0)
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual(report["stage"], "complete")
			self.assertEqual(report["status"], "PASS")
			self.assertNotIn("visibleOutcomes", report)

	def test_playwright_child_disables_persisted_ai_page_snapshots_only_for_that_child(self) -> None:
		"""Sensitive visible feedback cannot leak through Playwright's automatic error context."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			child_environment = {"PLE_UI_WALKTHROUGH_MASTER_SEED": "42"}
			playwright_environment = runner.playwright_child_environment(child_environment)
			self.assertEqual(playwright_environment["PLAYWRIGHT_NO_COPY_PROMPT"], "1")
			self.assertNotIn("PLAYWRIGHT_NO_COPY_PROMPT", child_environment)

	def test_arranger_rejects_a_bin_symlink_that_escapes_the_exact_ttsx_cli(self) -> None:
		"""A normal npm link is accepted only when it resolves to the fixed repository CLI target."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			arranger = repository_root / "node_modules" / "tsx" / "dist"
			arranger.mkdir(parents=True)
			(arranger / "cli.mjs").write_text("export {};\n", encoding="ascii")
			bin_directory = repository_root / "node_modules" / ".bin"
			bin_directory.mkdir()
			(bin_directory / "tsx").symlink_to("../../outside-cli.mjs")
			(repository_root / "outside-cli.mjs").write_text("export {};\n", encoding="ascii")
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			with self.assertRaisesRegex(walkthrough.RunnerError, "arranger is unavailable"):
				runner.arrange({})

	def test_failure_cleanup_is_no_volume_and_report_is_private(self) -> None:
		"""A started stack is removed without volumes and its redacted failure report is mode 0600."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			commands = FakeCommands()
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			runner.compose_command = ["podman", "compose"]
			runner.stack_launch_attempted = True
			runner.prepare_report_directory()
			runner.finish(False)
			cleanup_command = commands.commands[0][0]
			self.assertEqual(cleanup_command[-2:], ["down", "--remove-orphans"])
			self.assertNotIn("--volumes", cleanup_command)
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual(report["status"], "FAIL")
			self.assertEqual(stat.S_IMODE(runner.report_path.stat().st_mode), 0o600)

	def test_j1_failure_reports_only_the_closed_checkpoint_and_not_child_output(self) -> None:
		"""The J1 failure receipt preserves one safe stage while state cleanup remains complete."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			environment: dict[str, str] = {}
			runner.arrangements = [{"label": "api-retry-corpus-publication"}]
			runner.prepare_journey_state(environment)
			checkpoint = runner.j1_checkpoint_file
			self.assertIsNotNone(checkpoint)
			if checkpoint is None:
				raise AssertionError("J1 checkpoint was not prepared")
			checkpoint.write_text("course_opened\n", encoding="ascii")
			checkpoint.chmod(0o600)
			runner.report_stage = "playwright_j1"
			self.assertEqual(runner.finish(False), 1)
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual(
				report,
				{
					"status": "FAIL",
					"masterSeed": 42,
					"stage": "playwright_j1",
					"j1Checkpoint": "course_opened",
				},
			)
			self.assertFalse(checkpoint.exists())

	def test_j1_checkpoint_reader_rejects_unsafe_files_and_unbounded_values(self) -> None:
		"""Only an exact private regular file with one closed stage enters a J1 FAIL receipt."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			environment: dict[str, str] = {}
			runner.arrangements = [{"label": "api-retry-corpus-publication"}]
			runner.prepare_journey_state(environment)
			checkpoint = runner.j1_checkpoint_file
			self.assertIsNotNone(checkpoint)
			if checkpoint is None:
				raise AssertionError("J1 checkpoint was not prepared")
			for value in ("answer-visible\n", "retry_visible\r\n", "retry_visible\nextra\n", "\xff"):
				checkpoint.write_bytes(value.encode("latin1"))
				checkpoint.chmod(0o600)
				self.assertEqual(runner.read_j1_failure_checkpoint(), "unavailable")
			checkpoint.write_text("retry_visible\n", encoding="ascii")
			checkpoint.chmod(0o644)
			self.assertEqual(runner.read_j1_failure_checkpoint(), "unavailable")
			checkpoint.unlink()
			checkpoint.symlink_to(repository_root / "containers" / "env.local")
			self.assertEqual(runner.read_j1_failure_checkpoint(), "unavailable")
			runner.remove_private_state()

	def test_instructor_setup_failure_reports_only_the_closed_checkpoint_and_not_child_output(self) -> None:
		"""The instructor failure receipt exposes one approved stage and cleans private state."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			environment: dict[str, str] = {}
			runner.arrangements = [{"label": "api-retry-corpus-publication"}]
			runner.prepare_journey_state(environment)
			checkpoint = pathlib.Path(
				environment["PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_CHECKPOINT_FILE"]
			)
			checkpoint.write_text("catalog_result_selected\n", encoding="ascii")
			checkpoint.chmod(0o600)
			runner.report_stage = "playwright_instructor_setup"
			self.assertEqual(runner.finish(False), 1)
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual(
				report,
				{
					"status": "FAIL",
					"masterSeed": 42,
					"stage": "playwright_instructor_setup",
					"instructorCheckpoint": "catalog_result_selected",
				},
			)
			self.assertFalse(checkpoint.exists())

	def test_instructor_setup_checkpoint_reader_rejects_unsafe_files_and_unbounded_values(self) -> None:
		"""Only an exact private regular file with a closed instructor stage enters the receipt."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			environment: dict[str, str] = {}
			runner.arrangements = [{"label": "api-retry-corpus-publication"}]
			runner.prepare_journey_state(environment)
			checkpoint = pathlib.Path(
				environment["PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_CHECKPOINT_FILE"]
			)
			for value in (
				"answer_visible\n",
				"assignment_created\r\n",
				"assignment_created\nextra\n",
				"caf\xe9\n",
				"signed_in" * 20,
			):
				checkpoint.write_bytes(value.encode("latin1"))
				checkpoint.chmod(0o600)
				self.assertEqual(runner.read_instructor_setup_failure_checkpoint(), "unavailable")
			checkpoint.write_text("assignment_created\n", encoding="ascii")
			checkpoint.chmod(0o644)
			self.assertEqual(runner.read_instructor_setup_failure_checkpoint(), "unavailable")
			checkpoint.unlink()
			checkpoint.symlink_to(repository_root / "containers" / "env.local")
			self.assertEqual(runner.read_instructor_setup_failure_checkpoint(), "unavailable")
			runner.remove_private_state()

	def test_instructor_setup_checkpoint_file_replacement_is_unavailable_and_redacted(self) -> None:
		"""A same-mode replacement after the child checkpoint bind cannot enter a failure receipt."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			environment: dict[str, str] = {}
			runner.arrangements = [{"label": "api-retry-corpus-publication"}]
			runner.prepare_journey_state(environment)
			checkpoint = pathlib.Path(
				environment["PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_CHECKPOINT_FILE"]
			)
			checkpoint.write_text("course_opened\n", encoding="ascii")
			checkpoint.chmod(0o600)
			replacement = checkpoint.with_name("replacement-checkpoint.txt")
			replacement.write_text("assignment_created\n", encoding="ascii")
			replacement.chmod(0o600)
			replacement.replace(checkpoint)
			runner.report_stage = "playwright_instructor_setup"
			self.assertEqual(runner.finish(False), 1)
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual(report["instructorCheckpoint"], "unavailable")
			self.assertNotIn("assignment_created", json.dumps(report))
			self.assertIsNone(runner.private_state_directory)

	def test_instructor_checkpoint_parent_replacement_is_redacted_and_cleanup_preserves_replacement(self) -> None:
		"""A forged private parent cannot enter the receipt or become a cleanup target."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			environment: dict[str, str] = {}
			runner.arrangements = [{"label": "api-retry-corpus-publication"}]
			runner.prepare_journey_state(environment)
			original = runner.private_state_directory
			self.assertIsNotNone(original)
			if original is None:
				raise AssertionError("private state directory was not prepared")
			parked = original.with_name(f"{original.name}-parked")
			original.rename(parked)
			original.mkdir(mode=0o700)
			replacement_checkpoint = original / walkthrough.INSTRUCTOR_SETUP_CHECKPOINT_FILE
			replacement_checkpoint.write_text("assignment_created\n", encoding="ascii")
			replacement_checkpoint.chmod(0o600)
			runner.report_stage = "playwright_instructor_setup"
			self.assertEqual(runner.finish(False), 1)
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual(report["stage"], "cleanup")
			self.assertNotIn("assignment_created", json.dumps(report))
			self.assertTrue(replacement_checkpoint.exists())
			self.assertEqual(runner.private_state_directory, original)
			shutil.rmtree(original)
			shutil.rmtree(parked)

	def test_private_state_replacement_fails_closed_without_reading_or_deleting_replacement(self) -> None:
		"""A same-mode replacement cannot become a trusted J1 checkpoint or cleanup target."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			environment: dict[str, str] = {}
			runner.arrangements = [{"label": "api-retry-corpus-publication"}]
			runner.prepare_journey_state(environment)
			original = runner.private_state_directory
			self.assertIsNotNone(original)
			if original is None:
				raise AssertionError("private state directory was not prepared")
			parked = original.with_name(f"{original.name}-parked")
			original.rename(parked)
			original.mkdir(mode=0o700)
			replacement_checkpoint = original / walkthrough.J1_CHECKPOINT_FILE
			replacement_checkpoint.write_text("retry_visible\n", encoding="ascii")
			replacement_checkpoint.chmod(0o600)
			runner.report_stage = "playwright_j1"
			self.assertEqual(runner.read_j1_failure_checkpoint(), "unavailable")
			self.assertEqual(runner.finish(False), 1)
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual(report["stage"], "cleanup")
			self.assertTrue(replacement_checkpoint.exists())
			self.assertEqual(runner.private_state_directory, original)
			shutil.rmtree(original)
			shutil.rmtree(parked)

	def test_instructor_checkpoint_replacement_before_command_return_is_untrusted(self) -> None:
		"""A forged checkpoint installed before child return cannot become trusted evidence."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			(repository_root / "dist").mkdir()
			(repository_root / "dist" / "index.html").write_text("index", encoding="ascii")
			(repository_root / "dist" / "main.js").write_text("main", encoding="ascii")
			arranger_directory = repository_root / "node_modules" / "tsx" / "dist"
			arranger_directory.mkdir(parents=True)
			(arranger_directory / "cli.mjs").write_text("export {};\n", encoding="ascii")
			bin_directory = repository_root / "node_modules" / ".bin"
			bin_directory.mkdir()
			(bin_directory / "tsx").symlink_to("../tsx/dist/cli.mjs")
			login_file = repository_root / "containers" / "local-login.txt"
			login_file.write_text("instructor=fixture\nstudent=fixture\n", encoding="ascii")
			login_file.chmod(0o600)
			inputs = dataclasses.replace(self.resolve(repository_root, []), instructor_setup_only=True)

			class InstructorCommands(FakeCommands):
				def __call__(self, command: list[str], environ: dict[str, str] | None) -> object:
					self.commands.append((command, environ))
					if environ is not None and environ.get("PLE_UI_WALKTHROUGH_ARRANGER_CHILD") == "1":
						return walkthrough.CommandResult(
							0,
							'{"arrangements":[{"label":"api-retry-corpus-publication","problemId":"123e4567-e89b-12d3-a456-426614174001","versionId":"123e4567-e89b-12d3-a456-426614174002","catalogSearchTitle":"Pilot retry corpus pilotref123e4567e89b12d3a456426614174000"}]}\n',
							"student-local",
						)
					if environ is not None and "PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_CHECKPOINT_FILE" in environ:
						checkpoint = pathlib.Path(
							environ["PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_CHECKPOINT_FILE"]
						)
						checkpoint.write_text("course_opened\n", encoding="ascii")
						checkpoint.chmod(0o600)
						forged = repository_root / "forged-instructor-checkpoint.txt"
						forged.write_text("assignment_created\n", encoding="ascii")
						forged.chmod(0o600)
						checkpoint.unlink()
						os.link(forged, checkpoint)
						return walkthrough.CommandResult(1, "student-local", "student-local")
					return walkthrough.CommandResult(0, "", "")

			commands = InstructorCommands()
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			with mock.patch.object(walkthrough.shutil, "which", return_value="/bin/echo"):
				with self.assertRaisesRegex(walkthrough.RunnerError, "playwright_instructor_setup"):
					runner.execute()
			self.assertEqual(runner.finish(False), 1)
			report_text = runner.report_path.read_text(encoding="ascii")
			self.assertNotIn("student-local", report_text)
			report = json.loads(report_text)
			self.assertEqual(report["instructorCheckpoint"], "unavailable")
			self.assertNotIn("assignment_created", report_text)
			self.assertEqual(
				(repository_root / "forged-instructor-checkpoint.txt").read_text(encoding="ascii"),
				"assignment_created\n",
			)
			self.assertIsNone(runner.private_state_directory)

if __name__ == "__main__":
	unittest.main()
