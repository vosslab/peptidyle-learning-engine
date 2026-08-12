"""Run the real-stack public UI walkthrough with fail-closed boundaries."""

import json
import os
import pathlib
import secrets
import shutil
import stat
import sys
import tempfile

import walklib.arrangement_contract
import walklib.configuration
import walklib.instructor_handoff
import walklib.models
import walklib.playwright_boundary
import walklib.process
import walklib.v2_report_contract

ARRANGER_RELATIVE_PATH = pathlib.Path("node_modules/tsx/dist/cli.mjs")
J1_CHECKPOINT_FILE = "j1-checkpoint.txt"
J2_CHECKPOINT_FILE = "j2-checkpoint.txt"
INSTRUCTOR_SETUP_CHECKPOINT_FILE = "instructor-setup-checkpoint.txt"
CHILD_INPUTS_FILE = "walkthrough-inputs.json"
# The private config is outside the repository's package boundary.  Keep the
# extension explicit so Playwright loads it as ESM and its imported helpers share
# the same module semantics as the ESM walkthrough specifications.
PLAYWRIGHT_CONFIG_FILE = "playwright.walkthrough.config.mts"
J1_CHECKPOINTS = frozenset(
	{
		"signed_in",
		"course_visible",
		"course_opened",
		"assignment_visible",
		"run_controls_visible",
		"feedback_visible",
		"next_question_visible",
	}
)
J2_CHECKPOINTS = frozenset(
	{
		"signed_in",
		"active_run_visible",
		"response_selected",
		"feedback_visible",
		"first_run_completed",
		"fresh_practice_visible",
	}
)
INSTRUCTOR_SETUP_CHECKPOINTS = frozenset(
	{
		"browser_ready",
		"login_visible",
		"signed_in",
		"course_created",
		"course_opened",
		"student_active",
		"assignment_editor_opened",
		"catalog_result_selected",
		"assignment_created",
	}
)


RunnerError = walklib.models.RunnerError
RunnerInputs = walklib.models.RunnerInputs
CommandResult = walklib.models.CommandResult
CommandRunner = walklib.models.CommandRunner


parse_args = walklib.configuration.parse_args
resolve_inputs = walklib.configuration.resolve_inputs
validate_regular_readable_file = walklib.configuration.validate_regular_readable_file
validate_report_basename = walklib.configuration.validate_report_basename
validate_screenshot_directory = walklib.configuration.validate_screenshot_directory
has_reusable_dist = walklib.configuration.has_reusable_dist
reuse_existing_dist = walklib.configuration.reuse_existing_dist
env_value = walklib.configuration.env_value
effective_gateway_port = walklib.configuration.effective_gateway_port
validate_compose_project_name = walklib.configuration.validate_compose_project_name
credential_file = walklib.configuration.credential_file
validate_credential_file = walklib.configuration.validate_credential_file
validate_existing_credential_file = walklib.configuration.validate_existing_credential_file
command_result = walklib.process.command_result

class WalkthroughRunner:
	"""Own preflight, launcher lifecycle, secure report creation, and conservative cleanup."""

	def __init__(
		self,
		inputs: RunnerInputs,
		repository_root: pathlib.Path,
		environ: dict[str, str],
		run_command: CommandRunner = command_result,
	) -> None:
		self.inputs = inputs
		self.repository_root = repository_root
		self.environ = environ
		self.run_command = run_command
		self.compose_command: list[str] = []
		self.stack_launch_attempted = False
		self.report_directory = repository_root / "test-results" / "ui_walkthrough"
		self.report_path = self.report_directory / inputs.report_basename
		self.report_ready = False
		self.report_status = "FAIL"
		self.report_stage = "preflight"
		self.arrangements: list[dict[str, str]] | None = None
		# Four human-readable references are needed only by the J13 browser child.
		self.instructor_catalog_display_ids: list[str] | None = None
		self.visible_outcomes: dict[str, object] | None = None
		self.private_state_directory: pathlib.Path | None = None
		self.private_state_identity: tuple[int, int] | None = None
		self.journey_state_file: pathlib.Path | None = None
		self.child_inputs_file: pathlib.Path | None = None
		self.playwright_config_file: pathlib.Path | None = None
		self.learner_alias_file: pathlib.Path | None = None
		self.j1_checkpoint_file: pathlib.Path | None = None
		self.j1_failure_checkpoint: str | None = None
		self.j2_checkpoint_file: pathlib.Path | None = None
		self.j2_failure_checkpoint: str | None = None
		self.instructor_setup_checkpoint_file: pathlib.Path | None = None
		self.instructor_setup_checkpoint_identity: tuple[int, int] | None = None
		self.instructor_setup_failure_checkpoint: str | None = None

	#============================================
	def sanitized_child_environment(self) -> dict[str, str]:
		"""Remove inherited PLE switches before every runner-owned child process."""
		environment = {
			name: value for name, value in self.environ.items() if not name.startswith("PLE_")
		}
		return environment

	#============================================
	def run_required(
		self,
		command: list[str],
		environ: dict[str, str] | None = None,
	) -> CommandResult:
		"""Run one child command with bounded public stage diagnostics.

		Args:
			command: Exact argument vector to execute without shell parsing.
			environ: Optional child environment; ``None`` inherits the current process.

		Returns:
			The successful child result.

		Raises:
			RunnerError: The child exits with a nonzero status.
		"""
		print(f"UI walkthrough: {self.report_stage} starting")
		result = self.run_command(command, environ)
		if result.returncode != 0:
			raise RunnerError(
				f"{self.report_stage} command failed with exit status {result.returncode}"
			)
		print(f"UI walkthrough: {self.report_stage} completed")
		return result

	#============================================
	def configure_compose(self) -> None:
		"""Select a usable Podman Compose provider before touching report or stack state."""
		podman_available = shutil.which("podman")
		podman_compose_available = (
			podman_available is not None
			and self.run_command(["podman", "compose", "version"], None).returncode == 0
		)
		if podman_compose_available:
			self.compose_command = ["podman", "compose"]
			return
		if (
			shutil.which("podman-compose")
			and self.run_command(["podman-compose", "version"], None).returncode == 0
		):
			self.compose_command = ["podman-compose"]
			return
		raise RunnerError("no usable Podman Compose provider is available")

	#============================================
	def assert_no_existing_stack(self) -> None:
		"""Refuse to claim cleanup ownership when either exact containers label already exists."""
		labels = (
			"io.podman.compose.project=containers",
			"com.docker.compose.project=containers",
		)
		for label in labels:
			result = self.run_command(
				["podman", "ps", "--all", "--quiet", "--filter", f"label={label}"],
				None,
			)
			if result.returncode != 0:
				raise RunnerError("cannot inspect selected Podman project before walkthrough E2E")
			if result.stdout.strip():
				raise RunnerError(
					"selected Podman project already has containers; remove it before walkthrough E2E"
				)

	#============================================
	def prepare_report_directory(self) -> None:
		"""Create a private report parent only after all preflight and ownership checks pass."""
		self.ensure_report_directory()
		self.report_ready = True

	#============================================
	def ensure_report_directory(self) -> None:
		"""Revalidate and recreate only the private report directory without following links."""
		report_root = self.repository_root / "test-results"
		for path, description, private in (
			(report_root, "test-results", False),
			(self.report_directory, "ui_walkthrough report directory", True),
		):
			if path.is_symlink():
				raise RunnerError("walkthrough report path must not contain a symlink")
			if path.exists():
				if not path.is_dir():
					raise RunnerError(f"{description} must be a directory")
			else:
				try:
					path.mkdir()
				except FileExistsError:
					pass
				if path.is_symlink() or not path.is_dir():
					raise RunnerError("walkthrough report path must not contain a symlink")
			if private:
				path.chmod(0o700)
		if self.report_path.is_symlink():
			raise RunnerError("walkthrough report path must not contain a symlink")
		if self.report_path.exists() and not self.report_path.is_file():
			raise RunnerError("walkthrough report path must be a regular file")

	#============================================
	def write_report(self) -> None:
		"""Atomically write the minimal private result record without credentials or service output."""
		if not self.report_ready:
			return
		self.ensure_report_directory()
		payload: dict[str, object] = {
			"status": self.report_status,
			"masterSeed": self.inputs.master_seed,
			"stage": self.report_stage,
		}
		if self.report_status == "PASS" and self.visible_outcomes is not None:
			payload = self.visible_outcomes
		elif self.report_status == "PASS" and self.inputs.student_repeat_only:
			payload["mode"] = "student_repeat_only"
		elif self.report_status == "FAIL" and self.report_stage == "playwright_j1":
			payload["j1Checkpoint"] = self.j1_failure_checkpoint or "unavailable"
		elif self.report_status == "FAIL" and self.report_stage == "playwright_j2":
			payload["j2Checkpoint"] = self.j2_failure_checkpoint or "unavailable"
		elif self.report_status == "FAIL" and self.report_stage == "playwright_instructor_setup":
			payload["instructorCheckpoint"] = (
				self.instructor_setup_failure_checkpoint or "unavailable"
			)
		elif self.arrangements is not None:
			payload["arrangements"] = self.arrangements
		directory_flags = os.O_RDONLY | os.O_DIRECTORY
		if hasattr(os, "O_NOFOLLOW"):
			directory_flags |= os.O_NOFOLLOW
		directory_descriptor = os.open(self.report_directory, directory_flags)
		temporary_name = f".ui_walkthrough_report.{secrets.token_hex(16)}"
		file_descriptor = -1
		try:
			file_descriptor = os.open(
				temporary_name,
				os.O_WRONLY | os.O_CREAT | os.O_EXCL,
				0o600,
				dir_fd=directory_descriptor,
			)
			os.fchmod(file_descriptor, 0o600)
			with os.fdopen(file_descriptor, "w", encoding="ascii") as report_file:
				file_descriptor = -1
				json.dump(payload, report_file, separators=(",", ":"))
				report_file.write("\n")
			os.replace(
				temporary_name,
				self.inputs.report_basename,
				src_dir_fd=directory_descriptor,
				dst_dir_fd=directory_descriptor,
			)
			os.chmod(self.inputs.report_basename, 0o600, dir_fd=directory_descriptor)
			self.ensure_report_directory()
		finally:
			if file_descriptor >= 0:
				os.close(file_descriptor)
			try:
				os.unlink(temporary_name, dir_fd=directory_descriptor)
			except FileNotFoundError:
				pass
			os.close(directory_descriptor)

	#============================================
	def compose_down(self) -> None:
		"""Remove only a runner-owned selected stack and never request volume removal."""
		if not self.stack_launch_attempted or self.inputs.keep:
			return
		command = self.compose_command + [
			"-f",
			"containers/compose.yaml",
			"--env-file",
			str(self.inputs.env_file),
			"down",
			"--remove-orphans",
		]
		result = self.run_command(command, self.sanitized_child_environment())
		if result.returncode != 0:
			raise RunnerError(f"cleanup command failed with exit status {result.returncode}")

	#============================================
	def parse_arrangement_output(self, stdout: str) -> list[dict[str, object]]:
		"""Accept only the bounded public-reference object emitted by the fixed arranger."""
		try:
			arrangements, catalog_questions = (
				walklib.arrangement_contract.parse_arrangement_output(stdout)
			)
		except ValueError as error:
			raise RunnerError("arrangement emitted invalid output") from error
		if catalog_questions is not None:
			self.instructor_catalog_display_ids = [
				question["displayId"] for question in catalog_questions
			]
		return arrangements

	#============================================
	def arrange(self) -> None:
		"""Run only the repository-fixed arranger and retain its public IDs after strict parsing."""
		arranger = self.repository_root / ARRANGER_RELATIVE_PATH
		try:
			resolved_arranger = arranger.resolve(strict=True)
		except OSError as error:
			raise RunnerError("fixed walkthrough arranger is unavailable") from error
		expected_arranger = self.repository_root.resolve() / ARRANGER_RELATIVE_PATH
		if resolved_arranger != expected_arranger or not arranger.is_file() or arranger.is_symlink():
			raise RunnerError("fixed walkthrough arranger is unavailable")
		bin_link = self.repository_root / "node_modules" / ".bin" / "tsx"
		try:
			resolved_link = bin_link.resolve(strict=True)
		except OSError as error:
			raise RunnerError("fixed walkthrough arranger is unavailable") from error
		if not bin_link.is_symlink() or resolved_link != expected_arranger:
			raise RunnerError("fixed walkthrough arranger is unavailable")
		node_path = shutil.which("node")
		if node_path is None or not os.access(node_path, os.X_OK):
			raise RunnerError("fixed walkthrough arranger is unavailable")
		if self.child_inputs_file is None:
			raise RunnerError("fixed walkthrough arranger inputs are unavailable")
		self.report_stage = "arrangement"
		result = self.run_command(
			[
				node_path,
				str(arranger),
				"tests/walkthrough/children/arrange.ts",
				"--inputs",
				str(self.child_inputs_file),
			],
			self.sanitized_child_environment(),
		)
		if result.returncode != 0:
			raise RunnerError("arrangement command failed")
		self.arrangements = self.parse_arrangement_output(result.stdout)
		if len(self.arrangements) == 1:
			return

	#============================================
	def arrange_instructor_setup(self) -> None:
		"""Use the launcher-produced four-question Genetics corpus before browser setup."""
		manifest = self.inputs.env_file.parent / "local-chapter-one-pilot.json"
		if (
			manifest.is_symlink()
			or not manifest.is_file()
			or stat.S_IMODE(manifest.stat().st_mode) != 0o600
		):
			raise RunnerError(
				"canonical instructor setup requires the launcher-produced Chapter 1 manifest"
			)
		self.arrange()
		if self.arrangements is None or len(self.arrangements) != 1:
			raise RunnerError("instructor setup arrangement emitted invalid output")
		if self.instructor_catalog_display_ids is None:
			raise RunnerError("instructor setup arrangement emitted invalid output")
		corpus = self.arrangements[0]
		if set(corpus) != {"label"} or corpus["label"] != "launcher-chapter-one-genetics":
			raise RunnerError("instructor setup arrangement emitted invalid output")

	#============================================
	def hand_off_instructor_setup(self) -> None:
		"""Pass only validated public J11/J12/J13 identifiers to fixed student children."""
		course_id, assignment_id = walklib.instructor_handoff.read_handoff(
			self.journey_state_file,
			self.arrangements,
			self.instructor_catalog_display_ids,
		)
		self.write_private_child_inputs(
			walklib.models.WalkthroughChildInputs(
				"learner_journey",
				self.base_url(),
				self.inputs.master_seed,
				credential_file(self.inputs),
				journey_state_file=self.journey_state_file,
				j1_checkpoint_file=self.j1_checkpoint_file,
				j2_checkpoint_file=self.j2_checkpoint_file,
				course_id=course_id,
				mastery_assignment_id=assignment_id,
				screenshot_directory=self.inputs.screenshot_directory,
			)
		)

	#============================================
	def prepare_journey_state(self) -> None:
		"""Create one private runner-owned state file outside Playwright's artifact tree."""
		try:
			directory = pathlib.Path(tempfile.mkdtemp(prefix="ple-ui-walkthrough-"))
		except OSError as error:
			raise RunnerError("could not prepare private walkthrough state") from error
		directory.chmod(0o700)
		directory_metadata = directory.lstat()
		if (
			not stat.S_ISDIR(directory_metadata.st_mode)
			or stat.S_ISLNK(directory_metadata.st_mode)
			or stat.S_IMODE(directory_metadata.st_mode) != 0o700
		):
			raise RunnerError("could not prepare private walkthrough state")
		state_file = directory / "journeys.json"
		file_descriptor = os.open(state_file, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
		os.close(file_descriptor)
		checkpoint_file = directory / J1_CHECKPOINT_FILE
		file_descriptor = os.open(checkpoint_file, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
		os.close(file_descriptor)
		j2_checkpoint_file = directory / J2_CHECKPOINT_FILE
		file_descriptor = os.open(j2_checkpoint_file, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
		os.close(file_descriptor)
		instructor_checkpoint_file = directory / INSTRUCTOR_SETUP_CHECKPOINT_FILE
		file_descriptor = os.open(
			instructor_checkpoint_file,
			os.O_WRONLY | os.O_CREAT | os.O_EXCL,
			0o600,
		)
		instructor_checkpoint_metadata = os.fstat(file_descriptor)
		os.close(file_descriptor)
		if (
			not stat.S_ISREG(instructor_checkpoint_metadata.st_mode)
			or stat.S_IMODE(instructor_checkpoint_metadata.st_mode) != 0o600
		):
			raise RunnerError("could not prepare private walkthrough state")
		self.private_state_directory = directory
		self.private_state_identity = (directory_metadata.st_dev, directory_metadata.st_ino)
		self.journey_state_file = state_file
		self.j1_checkpoint_file = checkpoint_file
		self.j2_checkpoint_file = j2_checkpoint_file
		self.instructor_setup_checkpoint_file = instructor_checkpoint_file
		self.instructor_setup_checkpoint_identity = (
			instructor_checkpoint_metadata.st_dev,
			instructor_checkpoint_metadata.st_ino,
		)
		alias_file = directory / "learner-alias.txt"
		alias_file.write_text("student-local\n", encoding="ascii")
		alias_file.chmod(0o600)
		self.learner_alias_file = alias_file
		self.child_inputs_file = directory / CHILD_INPUTS_FILE
		self.playwright_config_file = directory / PLAYWRIGHT_CONFIG_FILE
		self.write_private_playwright_config()

	#============================================
	def base_url(self) -> str:
		"""Return the explicit loopback gateway origin selected by the env file."""
		gateway_port = effective_gateway_port(self.inputs)
		base_url = f"http://127.0.0.1:{gateway_port}"
		return base_url

	#============================================
	def private_state_descriptor(self) -> int:
		"""Open the exact private directory without following a replacement symlink."""
		directory = self.private_state_directory
		identity = self.private_state_identity
		if directory is None or identity is None:
			raise RunnerError("private walkthrough input directory is unavailable")
		descriptor = os.open(
			directory,
			os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0),
		)
		metadata = os.fstat(descriptor)
		if (
			not stat.S_ISDIR(metadata.st_mode)
			or stat.S_IMODE(metadata.st_mode) != 0o700
			or (metadata.st_dev, metadata.st_ino) != identity
		):
			os.close(descriptor)
			raise RunnerError("private walkthrough input directory is unavailable")
		return descriptor

	#============================================
	def child_input_payload(
		self,
		inputs: walklib.models.ArrangementChildInputs | walklib.models.WalkthroughChildInputs,
	) -> dict[str, object]:
		"""Build one exact stage payload without credentials or answer material."""
		if isinstance(inputs, walklib.models.ArrangementChildInputs):
			return {
				"schemaVersion": 1,
				"stage": "arrangement",
				"chapterOneManifestFile": str(inputs.chapter_one_manifest_file),
			}
		payload: dict[str, object] = {
			"schemaVersion": 1,
			"stage": inputs.stage,
			"baseUrl": inputs.base_url,
			"masterSeed": inputs.master_seed,
			"credentialFile": str(inputs.credential_file),
		}
		if inputs.stage == "instructor_setup":
			if (
				inputs.journey_state_file is None
				or inputs.learner_alias_file is None
				or inputs.instructor_setup_checkpoint_file is None
				or inputs.catalog_display_ids is None
			):
				raise RunnerError("instructor setup inputs are incomplete")
			payload["journeyStateFile"] = str(inputs.journey_state_file)
			payload["learnerAliasFile"] = str(inputs.learner_alias_file)
			payload["instructorSetupCheckpointFile"] = str(inputs.instructor_setup_checkpoint_file)
			payload["catalogDisplayIds"] = list(inputs.catalog_display_ids)
		elif inputs.stage == "learner_journey":
			if (
				inputs.journey_state_file is None
				or inputs.j1_checkpoint_file is None
				or inputs.j2_checkpoint_file is None
				or inputs.course_id is None
				or inputs.mastery_assignment_id is None
			):
				raise RunnerError("learner journey inputs are incomplete")
			payload["journeyStateFile"] = str(inputs.journey_state_file)
			payload["j1CheckpointFile"] = str(inputs.j1_checkpoint_file)
			payload["j2CheckpointFile"] = str(inputs.j2_checkpoint_file)
			payload["courseId"] = inputs.course_id
			payload["masteryAssignmentId"] = inputs.mastery_assignment_id
		else:
			raise RunnerError("walkthrough input stage is invalid")
		payload["screenshotDirectory"] = (
			None if inputs.screenshot_directory is None else str(inputs.screenshot_directory)
		)
		return payload

	#============================================
	def write_private_file(self, filename: str, contents: bytes) -> pathlib.Path:
		"""Atomically replace one bounded canonical file in the private state directory."""
		if len(contents) == 0 or len(contents) > 8192 or any(byte > 0x7f for byte in contents):
			raise RunnerError("private walkthrough input is not bounded ASCII")
		directory_descriptor = self.private_state_descriptor()
		temporary_name = f".{filename}.{secrets.token_hex(16)}"
		file_descriptor = -1
		try:
			file_descriptor = os.open(
				temporary_name,
				os.O_WRONLY | os.O_CREAT | os.O_EXCL,
				0o600,
				dir_fd=directory_descriptor,
			)
			os.fchmod(file_descriptor, 0o600)
			os.write(file_descriptor, contents)
			os.fsync(file_descriptor)
			os.close(file_descriptor)
			file_descriptor = -1
			os.replace(
				temporary_name,
				filename,
				src_dir_fd=directory_descriptor,
				dst_dir_fd=directory_descriptor,
			)
			os.chmod(filename, 0o600, dir_fd=directory_descriptor)
		finally:
			if file_descriptor >= 0:
				os.close(file_descriptor)
			try:
				os.unlink(temporary_name, dir_fd=directory_descriptor)
			except FileNotFoundError:
				pass
			os.close(directory_descriptor)
		directory = self.private_state_directory
		if directory is None:
			raise RunnerError("private walkthrough input directory is unavailable")
		path = directory / filename
		metadata = path.lstat()
		if (
			not stat.S_ISREG(metadata.st_mode)
			or stat.S_ISLNK(metadata.st_mode)
			or stat.S_IMODE(metadata.st_mode) != 0o600
		):
			raise RunnerError("private walkthrough input file is unavailable")
		return path

	#============================================
	def write_private_child_inputs(
		self,
		inputs: walklib.models.ArrangementChildInputs | walklib.models.WalkthroughChildInputs,
	) -> None:
		"""Write the fixed-child argv handoff as canonical, versioned private JSON."""
		payload = self.child_input_payload(inputs)
		encoded = json.dumps(payload, ensure_ascii=True, separators=(",", ":")).encode("ascii")
		path = self.write_private_file(CHILD_INPUTS_FILE, encoded)
		self.child_inputs_file = path

	#============================================
	def write_private_playwright_config(self) -> None:
		"""Point Playwright's standard config argument at the current private input path."""
		if self.child_inputs_file is None:
			self.child_inputs_file = self.private_state_directory / CHILD_INPUTS_FILE
		factory = self.repository_root / "tests/playwright/ui_walkthrough_config_factory.ts"
		test_directory = self.repository_root / "tests/playwright"
		content = (
			"import { createUiWalkthroughConfig } from "
			# The ESM config loader resolves this absolute repository module without
			# depending on the package boundary that owns the private config file.
			+ json.dumps(str(factory))
			+ ";\nexport default createUiWalkthroughConfig("
			+ json.dumps(str(self.child_inputs_file))
			+ ", "
			+ json.dumps(str(test_directory))
			+ ");\n"
		)
		path = self.write_private_file(PLAYWRIGHT_CONFIG_FILE, content.encode("ascii"))
		self.playwright_config_file = path

	#============================================
	def read_journey_failure_checkpoint(
		self,
		path: pathlib.Path | None,
		filename: str,
		allowed_stages: frozenset[str],
	) -> str:
		"""Read one canonical private journey stage without retaining child output."""
		state_identity = self.private_state_identity
		if (
			path is None
			or state_identity is None
			or path.name != filename
			or path.is_symlink()
		):
			return "unavailable"
		parent = path.parent
		directory_descriptor = -1
		file_descriptor = -1
		try:
			directory_descriptor = os.open(
				parent,
				os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0),
			)
			parent_metadata = os.fstat(directory_descriptor)
			file_descriptor = os.open(
				filename,
				os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0),
				dir_fd=directory_descriptor,
			)
			metadata = os.fstat(file_descriptor)
			current_parent = parent.lstat()
			if (
				not stat.S_ISDIR(parent_metadata.st_mode)
				or stat.S_IMODE(parent_metadata.st_mode) != 0o700
				or not stat.S_ISREG(metadata.st_mode)
				or stat.S_IMODE(metadata.st_mode) != 0o600
				or metadata.st_size > 64
				or not stat.S_ISDIR(current_parent.st_mode)
				or stat.S_ISLNK(current_parent.st_mode)
				or stat.S_IMODE(current_parent.st_mode) != 0o700
				or current_parent.st_dev != parent_metadata.st_dev
				or current_parent.st_ino != parent_metadata.st_ino
				or (parent_metadata.st_dev, parent_metadata.st_ino) != state_identity
			):
				return "unavailable"
			raw = os.read(file_descriptor, metadata.st_size)
			try:
				value = raw.decode("ascii")
			except UnicodeDecodeError:
				return "unavailable"
			if value.count("\n") != 1 or not value.endswith("\n") or "\r" in value:
				return "unavailable"
			checkpoint = value[:-1]
			return checkpoint if checkpoint in allowed_stages else "unavailable"
		except OSError:
			return "unavailable"
		finally:
			if file_descriptor >= 0:
				os.close(file_descriptor)
			if directory_descriptor >= 0:
				os.close(directory_descriptor)

	#============================================
	def read_j1_failure_checkpoint(self) -> str:
		"""Read only the bounded J1 learner-stage vocabulary."""
		return self.read_journey_failure_checkpoint(
			self.j1_checkpoint_file,
			J1_CHECKPOINT_FILE,
			J1_CHECKPOINTS,
		)

	#============================================
	def read_j2_failure_checkpoint(self) -> str:
		"""Read only the bounded J2 learner-stage vocabulary."""
		return self.read_journey_failure_checkpoint(
			self.j2_checkpoint_file,
			J2_CHECKPOINT_FILE,
			J2_CHECKPOINTS,
		)

	#============================================
	def read_instructor_setup_failure_checkpoint(self) -> str:
		"""Read only a canonical runner-owned instructor stage without retaining child output."""
		path = self.instructor_setup_checkpoint_file
		state_identity = self.private_state_identity
		checkpoint_identity = self.instructor_setup_checkpoint_identity
		if (
			path is None
			or state_identity is None
			or checkpoint_identity is None
			or path.name != INSTRUCTOR_SETUP_CHECKPOINT_FILE
			or path.is_symlink()
		):
			return "unavailable"
		parent = path.parent
		directory_descriptor = -1
		file_descriptor = -1
		try:
			directory_descriptor = os.open(
				parent,
				os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0),
			)
			parent_metadata = os.fstat(directory_descriptor)
			file_descriptor = os.open(
				INSTRUCTOR_SETUP_CHECKPOINT_FILE,
				os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0),
				dir_fd=directory_descriptor,
			)
			metadata = os.fstat(file_descriptor)
			current_parent = parent.lstat()
			if (
				not stat.S_ISDIR(parent_metadata.st_mode)
				or stat.S_IMODE(parent_metadata.st_mode) != 0o700
				or not stat.S_ISREG(metadata.st_mode)
				or stat.S_IMODE(metadata.st_mode) != 0o600
				or metadata.st_size > 64
				or not stat.S_ISDIR(current_parent.st_mode)
				or stat.S_ISLNK(current_parent.st_mode)
				or stat.S_IMODE(current_parent.st_mode) != 0o700
				or current_parent.st_dev != parent_metadata.st_dev
				or current_parent.st_ino != parent_metadata.st_ino
				or (parent_metadata.st_dev, parent_metadata.st_ino) != state_identity
				or (metadata.st_dev, metadata.st_ino) != checkpoint_identity
			):
				return "unavailable"
			raw = os.read(file_descriptor, metadata.st_size)
			try:
				value = raw.decode("ascii")
			except UnicodeDecodeError:
				return "unavailable"
			if value.count("\n") != 1 or not value.endswith("\n") or "\r" in value:
				return "unavailable"
			checkpoint = value[:-1]
			return checkpoint if checkpoint in INSTRUCTOR_SETUP_CHECKPOINTS else "unavailable"
		except OSError:
			return "unavailable"
		finally:
			if file_descriptor >= 0:
				os.close(file_descriptor)
			if directory_descriptor >= 0:
				os.close(directory_descriptor)

	#============================================
	def collect_visible_outcomes(self) -> None:
		"""Run only the fixed v2 renderer and retain its bounded public journey result."""
		if self.journey_state_file is None or self.journey_state_file.is_symlink():
			raise RunnerError("visible outcome evidence is unavailable")
		node_path = shutil.which("node")
		arranger = self.repository_root / ARRANGER_RELATIVE_PATH
		if node_path is None or not arranger.is_file() or arranger.is_symlink():
			raise RunnerError("fixed visible-outcome renderer is unavailable")
		self.report_stage = "visible_outcome_report"
		result = self.run_command(
			[
				node_path,
				str(arranger),
				"tests/walkthrough/children/v2_report.ts",
				"--inputs",
				str(self.child_inputs_file),
			],
			self.sanitized_child_environment(),
		)
		if result.returncode != 0 or result.stderr != "":
			raise RunnerError("visible outcome renderer failed")
		try:
			self.visible_outcomes = walklib.v2_report_contract.parse_public_v2_report(
				result.stdout,
				self.inputs.master_seed,
			)
		except ValueError as error:
			raise RunnerError("visible outcome renderer emitted invalid output") from error

	#============================================
	def append_cross_actor_evidence(self) -> None:
		"""Run the fixed silent J8 child after the browser commits the J5 evidence."""
		if self.journey_state_file is None or self.journey_state_file.is_symlink():
			raise RunnerError("cross-actor evidence is unavailable")
		node_path = shutil.which("node")
		arranger = self.repository_root / ARRANGER_RELATIVE_PATH
		if node_path is None or not arranger.is_file() or arranger.is_symlink():
			raise RunnerError("fixed cross-actor child is unavailable")
		self.report_stage = "cross_actor"
		result = self.run_command(
			[
				node_path,
				str(arranger),
				"tests/walkthrough/children/v2_cross_actor.ts",
				"--inputs",
				str(self.child_inputs_file),
			],
			self.sanitized_child_environment(),
		)
		if result.returncode != 0 or result.stdout != "" or result.stderr != "":
			raise RunnerError("cross-actor child failed")

	#============================================
	def run_playwright(self, specification: str) -> None:
		"""Run one fixed browser journey through the private explicit configuration."""
		walklib.playwright_boundary.run_specification(
			self.playwright_config_file,
			specification,
			self.run_required,
			self.sanitized_child_environment(),
		)

	#============================================
	def remove_private_state(self) -> None:
		"""Remove only the exact runner-created private state directory on every finish path."""
		if self.private_state_directory is None:
			return
		identity = self.private_state_identity
		metadata = self.private_state_directory.lstat()
		if (
			identity is None
			or stat.S_ISLNK(metadata.st_mode)
			or not stat.S_ISDIR(metadata.st_mode)
			or stat.S_IMODE(metadata.st_mode) != 0o700
			or (metadata.st_dev, metadata.st_ino) != identity
		):
			raise RunnerError("walkthrough private state path must not contain a symlink")
		shutil.rmtree(self.private_state_directory)
		self.private_state_directory = None
		self.private_state_identity = None
		self.journey_state_file = None
		self.child_inputs_file = None
		self.playwright_config_file = None
		self.learner_alias_file = None
		self.j1_checkpoint_file = None
		self.j2_checkpoint_file = None
		self.instructor_setup_checkpoint_file = None
		self.instructor_setup_checkpoint_identity = None

	#============================================
	def finish(self, success: bool) -> int:
		"""Perform conservative cleanup, write the redacted report, and return a process status."""
		if success:
			self.report_status = "PASS"
			if not self.inputs.student_repeat_only:
				self.report_stage = "complete"
		if not success and self.report_stage == "playwright_j1":
			self.j1_failure_checkpoint = self.read_j1_failure_checkpoint()
		if not success and self.report_stage == "playwright_j2":
			self.j2_failure_checkpoint = self.read_j2_failure_checkpoint()
		if not success and self.report_stage == "playwright_instructor_setup":
			self.instructor_setup_failure_checkpoint = self.read_instructor_setup_failure_checkpoint()
		if self.stack_launch_attempted and self.inputs.keep:
			print("UI walkthrough: preserving stack started by this runner")
		cleanup_failed = False
		try:
			self.compose_down()
		except (OSError, RunnerError, UnicodeError) as error:
			cleanup_failed = True
			self.report_status = "FAIL"
			self.report_stage = "cleanup"
			print(f"FAIL: cleanup failed: {error}", file=sys.stderr)
		try:
			self.remove_private_state()
		except (OSError, RunnerError, UnicodeError) as error:
			cleanup_failed = True
			self.report_status = "FAIL"
			self.report_stage = "cleanup"
			print(f"FAIL: private state cleanup failed: {error}", file=sys.stderr)
		try:
			self.write_report()
		except (OSError, UnicodeError, RunnerError):
			print("FAIL: could not write walkthrough report", file=sys.stderr)
			return 1
		return 0 if success and not cleanup_failed else 1

	#============================================
	def execute(self) -> None:
		"""Run the fail-closed lifecycle from preflight through the Playwright boundary."""
		validate_existing_credential_file(self.inputs)
		effective_gateway_port(self.inputs)
		validate_compose_project_name(self.inputs, self.environ)
		self.configure_compose()
		self.assert_no_existing_stack()
		self.prepare_report_directory()

		self.report_stage = "launcher_check"
		launcher = str(self.repository_root / "launch_local_stack.sh")
		self.run_required(
			[launcher, "--check", "--env-file", str(self.inputs.env_file)],
			self.sanitized_child_environment(),
		)

		self.report_stage = "launcher_start"
		self.assert_no_existing_stack()
		start_command = [launcher, "--no-open"]
		if reuse_existing_dist(self.inputs, self.repository_root):
			start_command.append("--skip-build")
		start_command.extend(["--env-file", str(self.inputs.env_file)])
		self.stack_launch_attempted = True
		self.run_required(start_command, self.sanitized_child_environment())

		self.report_stage = "live_boundary"
		login_file = credential_file(self.inputs)
		validate_credential_file(login_file)
		self.prepare_journey_state()
		self.write_private_child_inputs(
			walklib.models.ArrangementChildInputs(
				self.inputs.env_file.parent / "local-chapter-one-pilot.json"
			)
		)
		self.arrange_instructor_setup()
		if self.instructor_catalog_display_ids is None:
			raise RunnerError("instructor setup arrangement emitted invalid output")
		self.write_private_child_inputs(
			walklib.models.WalkthroughChildInputs(
				"instructor_setup",
				self.base_url(),
				self.inputs.master_seed,
				login_file,
				journey_state_file=self.journey_state_file,
				learner_alias_file=self.learner_alias_file,
				instructor_setup_checkpoint_file=self.instructor_setup_checkpoint_file,
				catalog_display_ids=tuple(self.instructor_catalog_display_ids),
				screenshot_directory=self.inputs.screenshot_directory,
			)
		)
		self.report_stage = "playwright_instructor_setup"
		self.run_playwright(
			"tests/playwright/ui_walkthrough_instructor_setup.spec.ts"
		)
		if self.inputs.instructor_setup_only:
			return
		self.report_stage = "instructor_setup_handoff"
		self.hand_off_instructor_setup()
		if self.j1_checkpoint_file is None:
			raise RunnerError("J1 checkpoint is unavailable")
		self.report_stage = "playwright_j1"
		self.run_playwright("tests/playwright/ui_walkthrough_keyboard_j1.spec.ts")
		self.report_stage = "playwright_j2"
		self.run_playwright("tests/playwright/ui_walkthrough_keyboard_j2.spec.ts")
		for stage, specification in (
			("playwright_j3", "tests/playwright/ui_walkthrough_keyboard_j3.spec.ts"),
			("playwright_j4", "tests/playwright/ui_walkthrough_keyboard_j4.spec.ts"),
		):
			self.report_stage = stage
			self.run_playwright(specification)
		if self.inputs.student_repeat_only:
			self.report_stage = "student_repeat_complete"
			return
		self.report_stage = "playwright_j5"
		self.run_playwright("tests/playwright/ui_walkthrough_keyboard_j5.spec.ts")
		self.append_cross_actor_evidence()
		self.collect_visible_outcomes()


#============================================
def main(argv: list[str]) -> int:
	"""Parse inputs, run the lifecycle, and retain a redacted failure record after report setup."""
	args = parse_args(argv)
	repository_root = pathlib.Path(__file__).resolve().parents[3]
	try:
		inputs = resolve_inputs(args, repository_root)
	except RunnerError as error:
		print(f"FAIL: {error}", file=sys.stderr)
		return 1
	os.chdir(repository_root)
	runner = WalkthroughRunner(inputs, repository_root, os.environ.copy())
	try:
		runner.execute()
	except RunnerError as error:
		print(f"FAIL: {error}", file=sys.stderr)
		return runner.finish(False)
	except (OSError, UnicodeError):
		print(
			f"FAIL: operational error during {runner.report_stage}",
			file=sys.stderr,
		)
		return runner.finish(False)
	status = runner.finish(True)
	if status == 0:
		print("PASS: UI walkthrough live smoke completed.")
	return status
