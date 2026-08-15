"""Run the real-stack public UI walkthrough with fail-closed boundaries."""

import json
import dataclasses
import os
import pathlib
import secrets
import shutil
import stat
import sys
import tempfile

import local_stack_control.compose
import local_stack_control.env_file
import local_stack_control.lifecycle
import local_stack_control.models

import tests.walkthrough.walklib.arrangement_contract as arrangement_contract
import tests.walkthrough.walklib.configuration as configuration
import tests.walkthrough.walklib.instructor_handoff as instructor_handoff
import tests.walkthrough.walklib.models as models
import tests.walkthrough.walklib.podman_ownership as podman_ownership
import tests.walkthrough.walklib.podman_preflight as podman_preflight
import tests.walkthrough.walklib.playwright_boundary as playwright_boundary
import tests.walkthrough.walklib.process as process
import tests.walkthrough.walklib.result_receipt as result_receipt
import tests.walkthrough.walklib.stack_environment as stack_environment
import tests.walkthrough.walklib.v2_report_contract as v2_report_contract

ARRANGER_RELATIVE_PATH = pathlib.Path("node_modules/tsx/dist/cli.mjs")
J1_CHECKPOINT_FILE = "j1-checkpoint.txt"
J2_CHECKPOINT_FILE = "j2-checkpoint.txt"
INSTRUCTOR_SETUP_CHECKPOINT_FILE = "instructor-setup-checkpoint.txt"
CHILD_INPUTS_FILE = "walkthrough-inputs.json"
# Keep the private config extension explicit so Playwright loads it as ESM.
PLAYWRIGHT_CONFIG_FILE = "playwright.walkthrough.config.mts"
J1_CHECKPOINTS = frozenset({
		"signed_in",
		"course_visible",
		"course_opened",
		"assignment_visible",
		"run_controls_visible",
		"feedback_visible",
		"next_question_visible",
	})
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
RunnerError = models.RunnerError
RunnerInputs = models.RunnerInputs
CommandResult = models.CommandResult
CommandRunner = models.CommandRunner
WalkthroughControllerRunner = process.WalkthroughControllerRunner
parse_args = configuration.parse_args
resolve_inputs = configuration.resolve_inputs
validate_regular_readable_file = configuration.validate_regular_readable_file
validate_report_basename = configuration.validate_report_basename
validate_screenshot_directory = configuration.validate_screenshot_directory
has_reusable_dist = configuration.has_reusable_dist
reuse_existing_dist = configuration.reuse_existing_dist
effective_gateway_port = configuration.effective_gateway_port
effective_stack_ports = configuration.effective_stack_ports
reject_external_compose_project_name = configuration.reject_external_compose_project_name
create_compose_project_name = configuration.create_disposable_compose_project_name
validate_credential_file = configuration.validate_credential_file
command_result = process.command_result
assert_no_stale_project_resources = podman_ownership.assert_no_stale_project_resources
keep_instruction = podman_ownership.keep_instruction
assert_no_active_ple_stack = podman_preflight.assert_no_active_ple_stack
assert_ports_available = podman_preflight.assert_ports_available

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
		self.disposable_target: local_stack_control.models.DisposableComposeTarget | None = None
		self.disposable_capability_file: pathlib.Path | None = None
		self.compose_project_name = create_compose_project_name(secrets.token_hex(8))
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
		self.private_env_file: pathlib.Path | None = None
		self.private_state_identity: tuple[int, int] | None = None
		self.journey_state_file: pathlib.Path | None = None
		self.child_inputs_file: pathlib.Path | None = None
		self.playwright_config_file: pathlib.Path | None = None
		self.j1_checkpoint_file: pathlib.Path | None = None
		self.j1_failure_checkpoint: str | None = None
		self.j2_checkpoint_file: pathlib.Path | None = None
		self.j2_failure_checkpoint: str | None = None
		self.instructor_setup_checkpoint_file: pathlib.Path | None = None
		self.instructor_setup_checkpoint_identity: tuple[int, int] | None = None
		self.instructor_setup_failure_checkpoint: str | None = None
	def sanitized_child_environment(self) -> dict[str, str]:
		"""Remove ambient runner controls before every runner-owned child process."""
		environment = {
			name: value
			for name, value in local_stack_control.env_file.sanitized_runtime_environment(self.environ).items()
			if not name.startswith("PLE_") and name != "COMPOSE_PROJECT_NAME"
		}
		return environment

	def compose_child_environment(self) -> dict[str, str]:
		"""Pass the generated project name only to stack-owning child processes."""
		disposable = self.disposable_target
		if disposable is None:
			# Pre-target validation is still isolated from every ambient PLE setting.
			return self.sanitized_child_environment() | dict(
				COMPOSE_PROJECT_NAME=self.compose_project_name
			)
		environment = local_stack_control.compose.target_environment(
			disposable.target,
			self.sanitized_child_environment(),
		)
		environment["PLE_DISPOSABLE_CAPABILITY_FILE"] = str(disposable.capability_file)
		return environment
	def controller_runner(self) -> process.WalkthroughControllerRunner:
		"""Return the shared controller adapter over this runner's child-process seam."""
		return process.WalkthroughControllerRunner(self.run_command)

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
		result = self.run_command(command, environ, None)
		if result.returncode != 0:
			raise RunnerError(
				f"{self.report_stage} command failed with exit status {result.returncode}"
			)
		print(f"UI walkthrough: {self.report_stage} completed")
		return result

	def configure_compose(self) -> None:
		"""Resolve the provider through the shared controller before stack lifecycle work."""
		try:
			local_stack_control.compose.choose_provider(self.controller_runner(), self.repository_root)
			stack_environment.require_rootless_engine(self.controller_runner(), self.repository_root)
		except local_stack_control.models.ControllerError as error:
			raise RunnerError("no usable Podman Compose provider is available") from error

	def assert_no_existing_stack(self) -> None:
		"""Refuse a generated project that has any stale Podman-owned resource."""
		assert_no_stale_project_resources(
			self.compose_project_name,
			self.repository_root,
			self.controller_runner(),
		)

	def prepare_report_directory(self) -> None:
		"""Create a private report parent only after all preflight and ownership checks pass."""
		self.ensure_report_directory()
		self.report_ready = True

	def ensure_report_directory(self) -> None:
		"""Revalidate and recreate only the private report directory without following links."""
		result_receipt.ensure_private_report_directory(
			self.repository_root,
			self.report_directory,
			self.report_path,
		)

	def write_report(self) -> None:
		"""Atomically write the minimal private result record without credentials or service output."""
		if not self.report_ready:
			return
		payload = result_receipt.build_payload(
			self.report_status,
			self.inputs.master_seed,
			self.report_stage,
			self.inputs.student_repeat_only,
			self.visible_outcomes,
			self.arrangements,
			self.j1_failure_checkpoint,
			self.j2_failure_checkpoint,
			self.instructor_setup_failure_checkpoint,
		)
		result_receipt.write_private_receipt(
			self.repository_root,
			self.report_directory,
			self.report_path,
			self.inputs.report_basename,
			payload,
		)

	def compose_down(self) -> None:
		"""Remove the generated stack and volumes that this runner exclusively owns."""
		if not self.stack_launch_attempted or self.inputs.keep:
			return
		disposable = self.disposable_target
		if disposable is None:
			raise RunnerError("walkthrough disposable Compose target is unavailable")
		stack_environment.remove_disposable_stack(
			disposable,
			self.controller_runner(),
		)

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
			self.sanitized_child_environment(), None,
		)
		if result.returncode != 0:
			raise RunnerError("arrangement command failed")
		try:
			self.arrangements, self.instructor_catalog_display_ids = (
				arrangement_contract.parse_runner_arrangement_output(result.stdout)
			)
		except ValueError as error:
			raise RunnerError("arrangement emitted invalid output") from error
		if len(self.arrangements) == 1:
			return

	def arrange_instructor_setup(self) -> None:
		"""Use the launcher-produced four-question Genetics corpus before browser setup."""
		manifest = self.stack_env_file().parent / "local-chapter-one-pilot.json"
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

	def hand_off_instructor_setup(self) -> None:
		"""Pass only validated public J11/J12/J13 identifiers to fixed student children."""
		course_reference, assignment_reference = instructor_handoff.read_handoff(
			self.journey_state_file,
			self.arrangements,
			self.instructor_catalog_display_ids,
		)
		self.write_private_child_inputs(
			models.WalkthroughChildInputs(
				"learner_journey",
				self.base_url(),
				self.inputs.master_seed,
				self.stack_env_file().parent / "local-login.txt",
				journey_state_file=self.journey_state_file,
				j1_checkpoint_file=self.j1_checkpoint_file,
				j2_checkpoint_file=self.j2_checkpoint_file,
				course_reference=course_reference,
				mastery_assignment_reference=assignment_reference,
				screenshot_directory=self.inputs.screenshot_directory,
			)
		)

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
		self.child_inputs_file = directory / CHILD_INPUTS_FILE
		self.playwright_config_file = directory / PLAYWRIGHT_CONFIG_FILE
		self.write_private_playwright_config()

	def create_private_stack_environment(self) -> None:
		"""Copy the selected file into runner state and reserve its application image tag."""
		source = self.inputs.env_file.read_text(encoding="ascii")
		image = stack_environment.application_image(self.compose_project_name)
		directory = self.private_state_directory
		if directory is None:
			raise RunnerError("private walkthrough Compose environment is unavailable")
		capability_file, capability_digest = stack_environment.create_cleanup_capability(
			directory
		)
		self.disposable_capability_file = capability_file
		contents = stack_environment.render_private_environment(
			source,
			image,
			directory,
			capability_digest,
		)
		self.private_env_file = self.write_private_file("compose.env", contents)
		try:
			target = local_stack_control.compose.resolve_target(
				self.controller_runner(),
				self.repository_root,
				str(self.private_env_file),
				False,
				self.compose_project_name,
				required_provider=local_stack_control.models.DISPOSABLE_COMPOSE_PROVIDER,
			)
			self.disposable_target = local_stack_control.compose.new_disposable_target(
				target,
				capability_file,
				"ui-walkthrough",
			)
		except local_stack_control.models.ControllerError as error:
			raise RunnerError("walkthrough disposable Compose target is unavailable") from error

	def stack_env_file(self) -> pathlib.Path:
		"""Return the private Compose configuration used by launcher and cleanup only."""
		if self.private_env_file is None:
			raise RunnerError("private walkthrough Compose environment is unavailable")
		return self.private_env_file

	def base_url(self) -> str:
		"""Return the explicit loopback gateway origin selected by the env file."""
		gateway_port = effective_gateway_port(self.inputs)
		base_url = f"http://127.0.0.1:{gateway_port}"
		return base_url

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

	def child_input_payload(
		self,
		inputs: models.ArrangementChildInputs | models.WalkthroughChildInputs,
	) -> dict[str, object]:
		"""Build one exact stage payload without credentials or answer material."""
		if isinstance(inputs, models.ArrangementChildInputs):
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
				or inputs.instructor_setup_checkpoint_file is None
				or inputs.catalog_display_ids is None
			):
				raise RunnerError("instructor setup inputs are incomplete")
			payload["journeyStateFile"] = str(inputs.journey_state_file)
			payload["instructorSetupCheckpointFile"] = str(inputs.instructor_setup_checkpoint_file)
			payload["catalogDisplayIds"] = list(inputs.catalog_display_ids)
		elif inputs.stage == "learner_journey":
			if (
				inputs.journey_state_file is None
				or inputs.j1_checkpoint_file is None
				or inputs.j2_checkpoint_file is None
				or inputs.course_reference is None
				or inputs.mastery_assignment_reference is None
			):
				raise RunnerError("learner journey inputs are incomplete")
			payload["journeyStateFile"] = str(inputs.journey_state_file)
			payload["j1CheckpointFile"] = str(inputs.j1_checkpoint_file)
			payload["j2CheckpointFile"] = str(inputs.j2_checkpoint_file)
			payload["courseReference"] = inputs.course_reference
			payload["masteryAssignmentReference"] = inputs.mastery_assignment_reference
		else:
			raise RunnerError("walkthrough input stage is invalid")
		payload["screenshotDirectory"] = (
			None if inputs.screenshot_directory is None else str(inputs.screenshot_directory)
		)
		return payload

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

	def write_private_child_inputs(
		self,
		inputs: models.ArrangementChildInputs | models.WalkthroughChildInputs,
	) -> None:
		"""Write the fixed-child argv handoff as canonical, versioned private JSON."""
		payload = self.child_input_payload(inputs)
		encoded = json.dumps(payload, ensure_ascii=True, separators=(",", ":")).encode("ascii")
		path = self.write_private_file(CHILD_INPUTS_FILE, encoded)
		self.child_inputs_file = path

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

	def read_j1_failure_checkpoint(self) -> str:
		"""Read only the bounded J1 learner-stage vocabulary."""
		return self.read_journey_failure_checkpoint(
			self.j1_checkpoint_file,
			J1_CHECKPOINT_FILE,
			J1_CHECKPOINTS,
		)

	def read_j2_failure_checkpoint(self) -> str:
		"""Read only the bounded J2 learner-stage vocabulary."""
		return self.read_journey_failure_checkpoint(
			self.j2_checkpoint_file,
			J2_CHECKPOINT_FILE,
			J2_CHECKPOINTS,
		)

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
			self.sanitized_child_environment(), None,
		)
		if result.returncode != 0 or result.stderr != "":
			raise RunnerError("visible outcome renderer failed")
		try:
			self.visible_outcomes = v2_report_contract.parse_public_v2_report(
				result.stdout,
				self.inputs.master_seed,
			)
		except ValueError as error:
			raise RunnerError("visible outcome renderer emitted invalid output") from error

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
			self.sanitized_child_environment(), None,
		)
		if result.returncode != 0 or result.stdout != "" or result.stderr != "":
			raise RunnerError("cross-actor child failed")

	#============================================
	def run_playwright(self, specification: str) -> None:
		"""Run one fixed browser journey through the private explicit configuration."""
		playwright_boundary.run_specification(
			self.playwright_config_file,
			specification,
			self.run_required,
			self.sanitized_child_environment(),
		)

	#============================================
	def remove_private_state(self) -> None:
		"""Remove only the exact runner-created private state directory after safe cleanup."""
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
		self.private_env_file = None
		self.disposable_capability_file = None
		self.j1_checkpoint_file = None
		self.j2_checkpoint_file = None
		self.instructor_setup_checkpoint_file = None
		self.instructor_setup_checkpoint_identity = None

	#============================================
	def retained_private_state_instruction(self) -> str | None:
		"""Describe a verified private recovery directory without exposing its contents."""
		directory = self.private_state_directory
		identity = self.private_state_identity
		if directory is None or identity is None:
			return None
		try:
			metadata = directory.lstat()
		except OSError:
			return None
		if (
			stat.S_ISLNK(metadata.st_mode)
			or not stat.S_ISDIR(metadata.st_mode)
			or stat.S_IMODE(metadata.st_mode) != 0o700
			or (metadata.st_dev, metadata.st_ino) != identity
		):
			return None
		message = (
			f"UI walkthrough: private recovery state retained at {directory}. "
			"It is mode 0700 and may contain local credentials; remove it after diagnosing cleanup."
		)
		return message

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
		preserve_private_state = self.stack_launch_attempted and self.inputs.keep
		if preserve_private_state:
			print(keep_instruction(self.compose_project_name))
		cleanup_failed = False
		try:
			self.compose_down()
		except (OSError, RunnerError, UnicodeError) as error:
			cleanup_failed = True
			preserve_private_state = True
			self.report_status = "FAIL"
			self.report_stage = "cleanup"
			print(f"FAIL: cleanup failed: {error}", file=sys.stderr)
			instruction = self.retained_private_state_instruction()
			if instruction is not None:
				print(instruction, file=sys.stderr)
		if not preserve_private_state:
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
		effective_gateway_port(self.inputs)
		effective_stack_ports(self.inputs)
		reject_external_compose_project_name(self.inputs, self.environ)
		self.configure_compose()
		assert_no_active_ple_stack(self.repository_root, self.controller_runner())
		assert_ports_available(
			effective_stack_ports(self.inputs),
			self.repository_root,
			self.controller_runner(),
		)
		self.assert_no_existing_stack()
		self.prepare_report_directory()
		self.prepare_journey_state()
		self.create_private_stack_environment()
		disposable = self.disposable_target
		if disposable is None:
			raise RunnerError("walkthrough disposable Compose target is unavailable")
		stack_environment.require_empty_disposable_preflight(
			disposable, self.controller_runner()
		)

		self.report_stage = "lifecycle_validate"
		try:
			local_stack_control.lifecycle.validate_lifecycle(
				dataclasses.replace(disposable.target, env_file=self.inputs.env_file, env_setting_names=local_stack_control.env_file.env_setting_names(self.inputs.env_file)), self.controller_runner(), self.repository_root
			)
		except local_stack_control.models.ControllerError as error:
			raise RunnerError("walkthrough lifecycle validation failed") from error

		self.report_stage = "lifecycle_start"
		self.assert_no_existing_stack()
		self.stack_launch_attempted = True
		try:
			local_stack_control.lifecycle.start_lifecycle(
				disposable,
				self.controller_runner(),
				self.repository_root,
				local_stack_control.lifecycle.LifecycleOptions(
					180.0, not reuse_existing_dist(self.inputs, self.repository_root), False, False
				),
			)
		except local_stack_control.models.ControllerError as error:
			raise RunnerError("walkthrough lifecycle start failed") from error

		self.report_stage = "live_boundary"
		login_file = self.stack_env_file().parent / "local-login.txt"
		validate_credential_file(login_file)
		self.write_private_child_inputs(
			models.ArrangementChildInputs(
				self.stack_env_file().parent / "local-chapter-one-pilot.json"
			)
		)
		self.arrange_instructor_setup()
		if self.instructor_catalog_display_ids is None:
			raise RunnerError("instructor setup arrangement emitted invalid output")
		self.write_private_child_inputs(
			models.WalkthroughChildInputs(
				"instructor_setup",
				self.base_url(),
				self.inputs.master_seed,
				login_file,
				journey_state_file=self.journey_state_file,
				instructor_setup_checkpoint_file=self.instructor_setup_checkpoint_file,
				catalog_display_ids=tuple(self.instructor_catalog_display_ids),
				screenshot_directory=self.inputs.screenshot_directory,
			)
		)
		self.report_stage = "playwright_gateway_smoke"
		self.run_playwright("tests/playwright/ui_walkthrough_smoke.spec.ts")
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
