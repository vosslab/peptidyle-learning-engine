"""Focused offline tests for the public UI walkthrough runner lifecycle."""

import argparse
import dataclasses
import importlib
import io
import json
import os
import pathlib
import shutil
import stat
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from typing import Any
from unittest import mock


E2E_DIRECTORY = pathlib.Path(__file__).resolve().parent / "e2e"
sys.path.insert(0, str(E2E_DIRECTORY))
walkthrough = importlib.import_module("e2e_ui_walkthrough")


class FakeCommands:
	"""Capture argv-array commands and optionally stop at the controlled launcher boundary."""

	def __init__(self, fail_launcher_check: bool = False) -> None:
		self.commands: list[tuple[list[str], dict[str, str] | None]] = []
		self.fail_launcher_check = fail_launcher_check

	def __call__(
		self,
		command: list[str],
		environ: dict[str, str] | None,
	) -> Any:
		self.commands.append((command, environ))
		if self.fail_launcher_check and "--check" in command:
			return walkthrough.CommandResult(1, "", "controlled launcher boundary")
		return walkthrough.CommandResult(0, "", "")


class CleanupFailureCommands(FakeCommands):
	"""Return a controlled cleanup result while keeping every earlier command successful."""

	def __init__(self, cleanup_error: BaseException | None = None) -> None:
		super().__init__()
		self.cleanup_error = cleanup_error

	def __call__(
		self,
		command: list[str],
		environ: dict[str, str] | None,
	) -> Any:
		self.commands.append((command, environ))
		if command[-2:] == ["down", "--remove-orphans"]:
			if self.cleanup_error is not None:
				raise self.cleanup_error
			return walkthrough.CommandResult(7, "", "controlled cleanup failure")
		return walkthrough.CommandResult(0, "", "")


class JourneyFailureCommands(FakeCommands):
	"""Drive the fixed journey sequence through one redacted child failure."""

	def __init__(self, failing_script: str) -> None:
		super().__init__()
		self.failing_script = failing_script

	def __call__(
		self,
		command: list[str],
		environ: dict[str, str] | None,
	) -> Any:
		self.commands.append((command, environ))
		if command[-1] == "tests/e2e/ui_walkthrough_arrange.ts":
			return walkthrough.CommandResult(
				0,
				UiWalkthroughRunnerTests.instructor_arrangement_output(self),
				"",
			)
		if command[-1] == "tests/playwright/ui_walkthrough_instructor_setup.spec.ts":
			if command[-1] == self.failing_script:
				return walkthrough.CommandResult(1, "child-stdout-sentinel", "child-stderr-sentinel")
			if self.failing_script == "instructor_setup_handoff":
				return walkthrough.CommandResult(0, "", "")
			if environ is None:
				raise AssertionError("instructor child needs a private state path")
			state_path = pathlib.Path(environ["PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE"])
			state_path.write_text(UiWalkthroughRunnerTests.instructor_state_json(self), encoding="ascii")
			return walkthrough.CommandResult(0, "", "")
		if command[-1] == self.failing_script:
			return walkthrough.CommandResult(
				1,
				"child-stdout-sentinel",
				"child-stderr-sentinel",
			)
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
	) -> Any:
		"""Resolve a baseline valid CLI plus one focused build-selection variation."""
		args = walkthrough.parse_args(["--master-seed", "42"] + extra_arguments)
		inputs = walkthrough.resolve_inputs(args, repository_root)
		return inputs

	def arrangement_output(self) -> str:
		"""Return the fixed arranger's public-only success object for runner-boundary tests."""
		payload = {
			"arrangements": [
				{"label": "launcher-seeded-enrollment"},
				{
					"label": "launcher-baseline-assignment",
					"baselineAssignmentId": "123e4567-e89b-12d3-a456-426614174000",
				},
				{
					"label": "api-retry-corpus-publication",
					"problemId": "123e4567-e89b-12d3-a456-426614174001",
					"versionId": "123e4567-e89b-12d3-a456-426614174002",
				},
				{
					"label": "api-mastery-assignment",
					"courseId": "123e4567-e89b-12d3-a456-426614174003",
					"masteryAssignmentId": "123e4567-e89b-12d3-a456-426614174004",
				},
				{
					"label": "api-exam-assignment",
					"courseId": "123e4567-e89b-12d3-a456-426614174003",
					"examAssignmentId": "123e4567-e89b-12d3-a456-426614174005",
				},
			]
		}
		return json.dumps(payload, separators=(",", ":")) + "\n"

	def instructor_arrangement_output(self) -> str:
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

	def instructor_state_json(self) -> str:
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

	def visible_outcome_output(self) -> str:
		"""Return the fixed public-only J1 PASS envelope emitted by the renderer child."""
		payload = {
			"schemaVersion": 1,
			"status": "PASS",
			"masterSeed": 42,
			"stage": "complete",
			"elapsedMs": 12,
			"arrangements": [],
			"journeys": [],
		}
		return json.dumps(payload, separators=(",", ":")) + "\n"

	def test_auto_uses_existing_publish_outputs(self) -> None:
		"""AUTO reuse reaches the skip decision only when both exact publish outputs exist."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			dist_directory = repository_root / "dist"
			dist_directory.mkdir()
			(dist_directory / "index.html").write_text("index", encoding="ascii")
			(dist_directory / "main.js").write_text("main", encoding="ascii")
			inputs = self.resolve(repository_root, [])
			self.assertTrue(walkthrough.launcher_skip_build(inputs, repository_root))

	def test_auto_builds_when_publish_output_is_missing(self) -> None:
		"""AUTO chooses a launcher build rather than claiming incomplete dist is reusable."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			self.assertFalse(walkthrough.launcher_skip_build(inputs, repository_root))

	def test_forced_build_ignores_existing_publish_outputs(self) -> None:
		"""--build forces the launcher path even with complete reusable publish outputs."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			dist_directory = repository_root / "dist"
			dist_directory.mkdir()
			(dist_directory / "index.html").write_text("index", encoding="ascii")
			(dist_directory / "main.js").write_text("main", encoding="ascii")
			inputs = self.resolve(repository_root, ["--build"])
			self.assertFalse(walkthrough.launcher_skip_build(inputs, repository_root))

	def test_explicit_skip_fails_when_publish_output_is_missing(self) -> None:
		"""--skip-build rejects missing publish output before report or Podman activity."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			with self.assertRaisesRegex(walkthrough.RunnerError, "--skip-build requires"):
				self.resolve(repository_root, ["--skip-build"])
			self.assertFalse((repository_root / "test-results").exists())

	def test_conflicting_build_flags_fail_during_argparse(self) -> None:
		"""Mutually exclusive build flags are rejected before filesystem or Podman state changes."""
		with self.assertRaises(SystemExit):
			walkthrough.parse_args(["--master-seed", "42", "--build", "--skip-build"])

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
			) -> Any:
				if command[-1] == "label=com.docker.compose.project=containers":
					return walkthrough.CommandResult(0, "stopped-container\n", "")
				return walkthrough.CommandResult(0, "", "")

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

	def test_gateway_port_defaults_to_the_local_gateway_mapping(self) -> None:
		"""An omitted setting follows the launcher default rather than a test-only origin."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			env_file = repository_root / "containers" / "env.local"
			env_file.write_text("POSTGRES_DB=peptidyle\n", encoding="ascii")
			inputs = self.resolve(repository_root, [])
			self.assertEqual(walkthrough.effective_gateway_port(inputs, {}), 8080)

	def test_arrangement_output_accepts_only_ordered_public_uuid_records(self) -> None:
		"""The Python boundary retains only the fixed labels and public UUIDs from its child."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			arrangements = runner.parse_arrangement_output(self.arrangement_output())
			self.assertEqual(arrangements[3]["masteryAssignmentId"], "123e4567-e89b-12d3-a456-426614174004")
			with self.assertRaisesRegex(walkthrough.RunnerError, "invalid output"):
				runner.parse_arrangement_output(self.arrangement_output() + "unexpected\n")
			for hostile in (
				"\n" + self.arrangement_output(),
				self.arrangement_output() + "\n",
				self.arrangement_output().replace("\n", "\r\n"),
				self.arrangement_output().replace("}\n", "} \n"),
				self.arrangement_output().replace(":", ": "),
			):
				with self.assertRaisesRegex(walkthrough.RunnerError, "invalid output"):
					runner.parse_arrangement_output(hostile)

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
			) -> Any:
				if command[-1] == "tests/e2e/ui_walkthrough_arrange.ts":
					return walkthrough.CommandResult(0, self.arrangement_output(), "student-secret")
				return walkthrough.CommandResult(0, "", "")

			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, arrangement_command)
			environment = {"PLE_UI_WALKTHROUGH_LIVE_CREDENTIAL_FILE": "private-login.txt"}
			runner.arrange(environment)
			self.assertEqual(
				environment["PLE_UI_WALKTHROUGH_LIVE_COURSE_ID"],
				"123e4567-e89b-12d3-a456-426614174003",
			)
			self.assertNotIn("PLE_UI_WALKTHROUGH_LIVE_MANIFEST_FILE", environment)
			self.assertNotIn("student-secret", json.dumps(runner.arrangements))

	def test_instructor_arrangement_keeps_unique_catalog_title_out_of_report_state(self) -> None:
		"""The J13-only title is validated, passed privately, and stripped from report arrangements."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.arrangements = runner.parse_arrangement_output(self.instructor_arrangement_output())
			self.assertEqual(
				runner.instructor_catalog_search_title,
				"Pilot retry corpus pilotref123e4567e89b12d3a456426614174000",
			)
			self.assertNotIn("catalogSearchTitle", runner.arrangements[0])
			environment: dict[str, str] = {}
			runner.prepare_journey_state(environment)
			self.assertNotIn("pilotref", environment["PLE_UI_WALKTHROUGH_ARRANGEMENTS_JSON"])
			runner.prepare_report_directory()
			runner.finish(False)
			report_text = runner.report_path.read_text(encoding="ascii")
			self.assertNotIn("Pilot retry corpus", report_text)
			for hostile in (
				self.instructor_arrangement_output().replace("Pilot retry corpus", "peptide"),
				self.instructor_arrangement_output().replace(
					'"catalogSearchTitle"', '"unexpectedTitle"'
				),
			):
				with self.assertRaisesRegex(walkthrough.RunnerError, "invalid output"):
					runner.parse_arrangement_output(hostile)

	def test_private_journey_state_is_mode_0600_and_removed_on_finish(self) -> None:
		"""The renderer handoff stays outside test-results and cleanup removes its exact state root."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.arrangements = runner.parse_arrangement_output(self.arrangement_output())
			environment: dict[str, str] = {}
			runner.prepare_journey_state(environment)
			self.assertNotIn("test-results", environment["PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE"])
			self.assertEqual(stat.S_IMODE(runner.journey_state_file.stat().st_mode), 0o600)
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
			runner.arrangements = runner.parse_arrangement_output(self.instructor_arrangement_output())
			environment: dict[str, str] = {}
			runner.prepare_journey_state(environment)
			state_path = pathlib.Path(environment["PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE"])
			state_path.write_text(self.instructor_state_json(), encoding="ascii")
			runner.hand_off_instructor_setup(environment)
			self.assertEqual(
				environment["PLE_UI_WALKTHROUGH_LIVE_COURSE_ID"],
				"123e4567-e89b-12d3-a456-426614174000",
			)
			self.assertNotIn("PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY", environment)
			for hostile in (
				self.instructor_state_json().replace("visible_mastery_policy", "wrong"),
				self.instructor_state_json().replace('"elapsedMs":3', '"elapsedMs":1800001'),
				self.instructor_state_json().replace("426614174001", "426614174005"),
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
			runner.arrangements = runner.parse_arrangement_output(self.instructor_arrangement_output())
			environment: dict[str, str] = {}
			runner.prepare_journey_state(environment)
			state_path = pathlib.Path(environment["PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE"])
			state_path.write_text(self.instructor_state_json(), encoding="ascii")
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
			runner.arrangements = runner.parse_arrangement_output(self.arrangement_output())
			environment: dict[str, str] = {}
			runner.prepare_journey_state(environment)
			artifact_directory = runner.private_state_directory / "journey-artifacts"
			artifact_directory.mkdir(mode=0o700)
			(artifact_directory / "error-context.md").write_text("sensitive", encoding="ascii")
			self.assertNotIn("test-results", str(artifact_directory))
			runner.prepare_report_directory()
			runner.finish(False)
			self.assertFalse(artifact_directory.exists())

	def test_visible_outcome_renderer_output_is_bounded_and_compatibly_staged(self) -> None:
		"""A fixed renderer can add a journey record without changing top-level success semantics."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			parsed = runner.parse_visible_outcome_output(self.visible_outcome_output())
			self.assertEqual(parsed["status"], "PASS")
			self.assertEqual(parsed["stage"], "complete")
			with self.assertRaisesRegex(walkthrough.RunnerError, "invalid output"):
				runner.parse_visible_outcome_output(self.visible_outcome_output().replace("\n", "\nextra"))

	def test_success_report_is_the_renderer_schema_without_a_nested_duplicate(self) -> None:
		"""A successful J1 run persists the canonical renderer record as the complete report."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			runner.visible_outcomes = runner.parse_visible_outcome_output(self.visible_outcome_output())
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

	def test_valid_preflight_reaches_controlled_launcher_boundary(self) -> None:
		"""A valid preflight invokes launcher --check instead of silently returning failure."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			commands = FakeCommands(fail_launcher_check=True)
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			with mock.patch.object(walkthrough.shutil, "which", return_value="/fake/podman"):
				with self.assertRaisesRegex(walkthrough.RunnerError, "launcher_check"):
					runner.execute()
			self.assertIn("--check", commands.commands[-1][0])
			self.assertTrue(runner.report_ready)

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
			self.assertEqual(set(report), {"status", "masterSeed", "stage"})
			self.assertEqual(stat.S_IMODE(runner.report_path.stat().st_mode), 0o600)

	def test_student_j4_failure_is_redacted_and_still_cleans_the_owned_stack(self) -> None:
		"""A final student child failure records only its stage and removes the owned stack."""
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
			login_file.write_text("learner@example.test\n", encoding="ascii")
			login_file.chmod(0o600)
			inputs = self.resolve(repository_root, [])
			commands = JourneyFailureCommands("tests/playwright/ui_walkthrough_keyboard_j4.spec.ts")
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			with mock.patch.object(
				walkthrough.shutil,
				"which",
				return_value="/bin/echo",
			):
				with self.assertRaisesRegex(walkthrough.RunnerError, "playwright_j4"):
					runner.execute()
			self.assertEqual(runner.finish(False), 1)
			report_text = runner.report_path.read_text(encoding="ascii")
			report = json.loads(report_text)
			self.assertEqual(report["status"], "FAIL")
			self.assertEqual(report["stage"], "playwright_j4")
			self.assertNotIn("child-stdout-sentinel", report_text)
			self.assertNotIn("child-stderr-sentinel", report_text)
			self.assertEqual(stat.S_IMODE(runner.report_path.stat().st_mode), 0o600)
			self.assertIsNone(runner.private_state_directory)
			journey_children = [
				command[-1]
				for command, _ in commands.commands
				if command and command[0] == "bash" and command[-1].startswith("tests/playwright/")
			]
			self.assertEqual(
				journey_children,
				[
					"tests/playwright/ui_walkthrough_instructor_setup.spec.ts",
					"tests/playwright/ui_walkthrough_keyboard_j1.spec.ts",
					"tests/playwright/ui_walkthrough_keyboard_j2.spec.ts",
					"tests/playwright/ui_walkthrough_keyboard_j3.spec.ts",
					"tests/playwright/ui_walkthrough_keyboard_j4.spec.ts",
				],
			)
			cleanup_commands = [
				command for command, _ in commands.commands if command[-2:] == ["down", "--remove-orphans"]
			]
			self.assertEqual(len(cleanup_commands), 1)
			self.assertNotIn("--volumes", cleanup_commands[0])

	def test_each_fixed_student_child_reports_its_exact_failure_stage(self) -> None:
		"""Instructor setup and every fixed student child preserve their redacted stage."""
		for script, expected_stage, error_text in (
			(
				"tests/playwright/ui_walkthrough_instructor_setup.spec.ts",
				"playwright_instructor_setup",
				"playwright_instructor_setup",
			),
			(
				"instructor_setup_handoff",
				"instructor_setup_handoff",
				"public-ID handoff",
			),
			("tests/playwright/ui_walkthrough_keyboard_j1.spec.ts", "playwright_j1", "playwright_j1"),
			("tests/playwright/ui_walkthrough_keyboard_j2.spec.ts", "playwright_j2", "playwright_j2"),
			("tests/playwright/ui_walkthrough_keyboard_j3.spec.ts", "playwright_j3", "playwright_j3"),
			("tests/playwright/ui_walkthrough_keyboard_j4.spec.ts", "playwright_j4", "playwright_j4"),
		):
			with self.subTest(script=script):
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
					login_file.write_text("learner@example.test\n", encoding="ascii")
					login_file.chmod(0o600)
					inputs = self.resolve(repository_root, [])
					commands = JourneyFailureCommands(script)
					runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
					with mock.patch.object(walkthrough.shutil, "which", return_value="/bin/echo"):
						with self.assertRaisesRegex(walkthrough.RunnerError, error_text):
							runner.execute()
					self.assertEqual(runner.finish(False), 1)
					report_text = runner.report_path.read_text(encoding="ascii")
					report = json.loads(report_text)
					self.assertEqual(report["status"], "FAIL")
					self.assertEqual(report["stage"], expected_stage)
					self.assertEqual(stat.S_IMODE(runner.report_path.stat().st_mode), 0o600)
					self.assertIsNone(runner.private_state_directory)
					self.assertNotIn("child-stdout-sentinel", report_text)
					self.assertNotIn("child-stderr-sentinel", report_text)

	def test_instructor_only_failure_runs_one_child_and_redacts_private_alias(self) -> None:
		"""The J11-J13 runner branch has one child, one arrangement, and bounded cleanup."""
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
				def __call__(self, command: list[str], environ: dict[str, str] | None) -> Any:
					self.commands.append((command, environ))
					if command[-1] == "tests/e2e/ui_walkthrough_arrange.ts":
						return walkthrough.CommandResult(
							0,
							'{"arrangements":[{"label":"api-retry-corpus-publication","problemId":"123e4567-e89b-12d3-a456-426614174001","versionId":"123e4567-e89b-12d3-a456-426614174002","catalogSearchTitle":"Pilot retry corpus pilotref123e4567e89b12d3a456426614174000"}]}\n',
							"student-local",
						)
					if command[-1] == "tests/playwright/ui_walkthrough_instructor_setup.spec.ts":
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
			children = [command[-1] for command, _ in commands.commands if command and command[0] == "bash"]
			self.assertEqual(children, ["tests/playwright/ui_walkthrough_instructor_setup.spec.ts"])
			self.assertEqual(runner.report_stage, "playwright_instructor_setup")
			self.assertIsNone(runner.private_state_directory)
			cleanup = [command for command, _ in commands.commands if command[-2:] == ["down", "--remove-orphans"]]
			self.assertEqual(len(cleanup), 1)
			self.assertNotIn("--volumes", cleanup[0])

	def test_student_completion_does_not_claim_a_schema_v2_report_before_wp_e1(self) -> None:
		"""WP-S1 stops after J4 rather than invoking the separately owned report child."""
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
			login_file.write_text("learner@example.test\n", encoding="ascii")
			login_file.chmod(0o600)
			inputs = dataclasses.replace(self.resolve(repository_root, []), student_repeat_only=True)
			commands = JourneyFailureCommands("not-a-child")
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			with mock.patch.object(
				walkthrough.shutil,
				"which",
				return_value="/bin/echo",
			):
				runner.execute()
			self.assertEqual(runner.finish(True), 0)
			report_text = runner.report_path.read_text(encoding="ascii")
			report = json.loads(report_text)
			self.assertEqual(report["status"], "PASS")
			self.assertEqual(report["stage"], "student_repeat_complete")
			self.assertEqual(report["mode"], "student_repeat_only")
			self.assertEqual(stat.S_IMODE(runner.report_path.stat().st_mode), 0o600)
			self.assertIsNone(runner.private_state_directory)
			self.assertNotIn("child-stdout-sentinel", report_text)
			self.assertNotIn("child-stderr-sentinel", report_text)
			cleanup_commands = [
				command for command, _ in commands.commands if command[-2:] == ["down", "--remove-orphans"]
			]
			self.assertEqual(len(cleanup_commands), 1)
			self.assertNotIn("--volumes", cleanup_commands[0])

	def test_cleanup_requires_runner_launch_and_keep_blocks_down(self) -> None:
		"""Cleanup cannot remove an unlaunched stack and --keep preserves a runner-started stack."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			commands = FakeCommands()
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			runner.compose_command = ["podman", "compose"]
			runner.prepare_report_directory()
			runner.finish(False)
			self.assertEqual(commands.commands, [])
			keep_inputs = dataclasses.replace(inputs, keep=True)
			keep_runner = walkthrough.WalkthroughRunner(keep_inputs, repository_root, {}, commands)
			keep_runner.compose_command = ["podman", "compose"]
			keep_runner.stack_launch_attempted = True
			keep_runner.prepare_report_directory()
			keep_runner.finish(False)
			self.assertEqual(commands.commands, [])

	def test_cleanup_nonzero_downgrades_success_to_redacted_failure(self) -> None:
		"""A failed down command cannot leave either a PASS report or a successful process status."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			commands = CleanupFailureCommands()
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			runner.compose_command = ["podman", "compose"]
			runner.stack_launch_attempted = True
			runner.prepare_report_directory()
			stdout = io.StringIO()
			stderr = io.StringIO()
			with redirect_stdout(stdout), redirect_stderr(stderr):
				status = runner.finish(True)
			self.assertEqual(status, 1)
			self.assertNotIn("PASS", stdout.getvalue())
			self.assertIn("cleanup failed", stderr.getvalue())
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual(report, {"status": "FAIL", "masterSeed": 42, "stage": "cleanup"})

	def test_cleanup_os_error_downgrades_success_to_redacted_failure(self) -> None:
		"""A cleanup process-start error also fails closed and leaves a readable failure record."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			commands = CleanupFailureCommands(OSError("controlled cleanup OSError"))
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			runner.compose_command = ["podman", "compose"]
			runner.stack_launch_attempted = True
			runner.prepare_report_directory()
			with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
				status = runner.finish(True)
			self.assertEqual(status, 1)
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual(report["status"], "FAIL")
			self.assertEqual(report["stage"], "cleanup")

	def test_operational_execute_error_runs_runner_owned_cleanup(self) -> None:
		"""A post-launch filesystem or process error reaches no-volume cleanup and a FAIL report."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			commands = FakeCommands()
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, commands)
			runner.compose_command = ["podman", "compose"]
			runner.stack_launch_attempted = True
			runner.prepare_report_directory()
			with mock.patch.object(runner, "execute", side_effect=OSError("controlled execute OSError")):
				with mock.patch.object(walkthrough, "parse_args", return_value=argparse.Namespace()):
					with mock.patch.object(walkthrough, "resolve_inputs", return_value=inputs):
						with mock.patch.object(walkthrough, "WalkthroughRunner", return_value=runner):
							stderr = io.StringIO()
							with redirect_stderr(stderr):
								status = walkthrough.main([])
			self.assertEqual(status, 1)
			self.assertIn("operational error during preflight", stderr.getvalue())
			self.assertNotIn("Traceback", stderr.getvalue())
			self.assertEqual(commands.commands[0][0][-2:], ["down", "--remove-orphans"])
			report = json.loads(runner.report_path.read_text(encoding="ascii"))
			self.assertEqual(report["status"], "FAIL")
			self.assertEqual(report["stage"], "preflight")

	def test_invalid_env_decode_is_a_concise_main_failure(self) -> None:
		"""Malformed selected env bytes do not escape the public CLI with a traceback."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			env_file = repository_root / "containers" / "env.local"
			env_file.write_bytes(b"PLE_GATEWAY_HOST_PORT=3010\xff\n")
			stderr = io.StringIO()
			with redirect_stderr(stderr):
				status = walkthrough.main(
					["--master-seed", "42", "--env-file", str(env_file)]
				)
			self.assertEqual(status, 1)
			self.assertIn("operational error during preflight", stderr.getvalue())
			self.assertNotIn("Traceback", stderr.getvalue())

	def test_report_write_failure_returns_nonzero_without_report_claim(self) -> None:
		"""An atomic report failure stays concise and does not pretend that a report was written."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			stderr = io.StringIO()
			with mock.patch.object(walkthrough.os, "open", side_effect=OSError("full disk")):
				with redirect_stderr(stderr):
					status = runner.finish(False)
			self.assertEqual(status, 1)
			self.assertIn("could not write walkthrough report", stderr.getvalue())
			self.assertFalse(runner.report_path.exists())

	def test_report_directory_is_recreated_after_playwright_artifact_cleanup(self) -> None:
		"""A complete Playwright cleanup cannot erase the final private PASS report."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			shutil.rmtree(repository_root / "test-results")
			status = runner.finish(True)
			self.assertEqual(status, 0)
			self.assertEqual(
				json.loads(runner.report_path.read_text(encoding="ascii")),
				{"status": "PASS", "masterSeed": 42, "stage": "complete"},
			)
			self.assertEqual(stat.S_IMODE(runner.report_directory.stat().st_mode), 0o700)
			self.assertEqual(stat.S_IMODE(runner.report_path.stat().st_mode), 0o600)

	def test_report_symlink_replacement_fails_closed_without_pass_record(self) -> None:
		"""Replacing the report directory with a link cannot redirect a final PASS record."""
		with tempfile.TemporaryDirectory() as temporary_name:
			repository_root = self.make_repository(pathlib.Path(temporary_name))
			inputs = self.resolve(repository_root, [])
			runner = walkthrough.WalkthroughRunner(inputs, repository_root, {}, FakeCommands())
			runner.prepare_report_directory()
			outside_directory = repository_root / "outside"
			outside_directory.mkdir()
			runner.report_directory.rmdir()
			runner.report_directory.symlink_to(outside_directory, target_is_directory=True)
			stdout = io.StringIO()
			stderr = io.StringIO()
			with redirect_stdout(stdout), redirect_stderr(stderr):
				status = runner.finish(True)
			self.assertEqual(status, 1)
			self.assertNotIn("PASS", stdout.getvalue())
			self.assertIn("could not write walkthrough report", stderr.getvalue())
			self.assertFalse((outside_directory / inputs.report_basename).exists())


if __name__ == "__main__":
	unittest.main()
