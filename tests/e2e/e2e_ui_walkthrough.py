#!/usr/bin/env python3
"""Run the real-stack public UI walkthrough smoke with fail-closed boundaries."""

import argparse
import dataclasses
import json
import os
import pathlib
import re
import secrets
import shutil
import stat
import subprocess
import sys
import tempfile
from collections.abc import Callable


UINT32_MAX = 4_294_967_295
SAFE_REPORT_NAME = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*\.json$")
UUID_TEXT = re.compile(
	r"^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
	re.IGNORECASE,
)
MAX_ARRANGEMENT_OUTPUT_BYTES = 2048
MAX_VISIBLE_OUTCOME_OUTPUT_BYTES = 4096
ARRANGER_RELATIVE_PATH = pathlib.Path("node_modules/tsx/dist/cli.mjs")
MAX_JOURNEY_ELAPSED_MS = 30 * 60 * 1000
LOWER_UUID_TEXT = re.compile(
	r"^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$"
)
CATALOG_SEARCH_TITLE = re.compile(
	r"^Pilot retry corpus pilotref[0-9a-f]{32}$"
)


class RunnerError(RuntimeError):
	"""Describe a fail-closed walkthrough preflight or lifecycle problem."""


@dataclasses.dataclass(frozen=True)
class RunnerInputs:
	"""Validated command-line inputs and derived local-stack paths."""

	master_seed: int
	env_file: pathlib.Path
	report_basename: str
	keep: bool
	build_mode: str
	instructor_setup_only: bool
	student_repeat_only: bool


@dataclasses.dataclass(frozen=True)
class CommandResult:
	"""Small subprocess result surface that keeps focused tests independent of Podman."""

	returncode: int
	stdout: str
	stderr: str


CommandRunner = Callable[[list[str], dict[str, str] | None], CommandResult]


# ============================================
def parse_args(argv: list[str]) -> argparse.Namespace:
	"""Parse the small public CLI surface before any filesystem or Podman action."""
	parser = argparse.ArgumentParser(
		description="Run the real-gateway UI walkthrough smoke after fail-closed preflight."
	)
	parser.add_argument(
		"--master-seed",
		required=True,
		metavar="UINT32",
		help="Decimal unsigned 32-bit seed used by the deterministic walkthrough.",
	)
	parser.add_argument(
		"--env-file",
		default="containers/env.local",
		metavar="PATH",
		help="Compose environment file (default: containers/env.local).",
	)
	parser.add_argument(
		"--report-file",
		metavar="BASENAME",
		help="Safe .json basename below test-results/ui_walkthrough/.",
	)
	parser.add_argument(
		"--keep",
		action="store_true",
		help="Preserve a stack started by this runner for diagnosis.",
	)
	build_group = parser.add_mutually_exclusive_group()
	build_group.add_argument(
		"--build",
		action="store_true",
		help="Force launch_local_stack.sh to rebuild dist/.",
	)
	parser.add_argument(
		"--instructor-setup-only",
		action="store_true",
		help="Run only the fixed J11/J12/J13 instructor setup child.",
	)
	parser.add_argument(
		"--student-repeat-only",
		action="store_true",
		help="Run J11/J12/J13 followed by the partial J1/J2/J3/J4 student-repeat slice.",
	)
	build_group.add_argument(
		"--skip-build",
		action="store_true",
		help="Require reusable dist/index.html and dist/main.js, then skip the build.",
	)
	args = parser.parse_args(argv)
	return args


# ============================================
def resolve_inputs(args: argparse.Namespace, repository_root: pathlib.Path) -> RunnerInputs:
	"""Validate CLI values and resolve paths without creating files or calling Podman."""
	seed_text = args.master_seed
	if not re.fullmatch(r"[0-9]+", seed_text):
		raise RunnerError("--master-seed must be a decimal unsigned 32-bit integer")
	master_seed = int(seed_text)
	if master_seed > UINT32_MAX:
		raise RunnerError("--master-seed must be a decimal unsigned 32-bit integer")

	env_file = pathlib.Path(args.env_file)
	if not env_file.is_absolute():
		env_file = repository_root / env_file
	validate_regular_readable_file(env_file, "selected walkthrough env file")

	report_basename = args.report_file
	if report_basename is None:
		report_basename = f"ui_walkthrough_seed_{master_seed}.json"
	validate_report_basename(report_basename)

	build_mode = "auto"
	if args.build:
		build_mode = "build"
	elif args.skip_build:
		build_mode = "skip"
	if args.instructor_setup_only and args.student_repeat_only:
		raise RunnerError("--instructor-setup-only and --student-repeat-only cannot be combined")
	inputs = RunnerInputs(
		master_seed,
		env_file,
		report_basename,
		args.keep,
		build_mode,
		args.instructor_setup_only,
		args.student_repeat_only,
	)
	validate_build_selection(inputs, repository_root)
	return inputs


# ============================================
def validate_regular_readable_file(path: pathlib.Path, description: str) -> None:
	"""Require a readable regular non-symlink file at a security boundary."""
	if path.is_symlink():
		raise RunnerError(f"{description} must not be a symlink")
	if not path.is_file() or not os.access(path, os.R_OK):
		raise RunnerError(f"{description} is missing or unreadable")


# ============================================
def validate_report_basename(report_basename: str) -> None:
	"""Reject traversal and unsafe names before creating the private report directory."""
	if "/" in report_basename or ".." in report_basename:
		raise RunnerError("--report-file must not contain a path or traversal")
	if not SAFE_REPORT_NAME.fullmatch(report_basename):
		raise RunnerError(
			"--report-file must contain only safe ASCII filename characters and end in .json"
		)


# ============================================
def has_reusable_dist(repository_root: pathlib.Path) -> bool:
	"""Return whether the exact publish outputs can safely be reused without building."""
	for relative_path in ("dist/index.html", "dist/main.js"):
		path = repository_root / relative_path
		if path.is_symlink() or not path.is_file() or not os.access(path, os.R_OK):
			return False
	return True


# ============================================
def validate_build_selection(inputs: RunnerInputs, repository_root: pathlib.Path) -> None:
	"""Fail explicit reuse before report or Podman work when required bundle outputs are absent."""
	if inputs.build_mode == "skip" and not has_reusable_dist(repository_root):
		raise RunnerError("--skip-build requires readable non-symlink dist/index.html and dist/main.js")


# ============================================
def launcher_skip_build(inputs: RunnerInputs, repository_root: pathlib.Path) -> bool:
	"""Choose automatic output reuse only when both required outputs are present and safe."""
	if inputs.build_mode == "build":
		return False
	if inputs.build_mode == "skip":
		return True
	return has_reusable_dist(repository_root)


# ============================================
def env_value(env_file: pathlib.Path, setting_name: str) -> str:
	"""Read the last exact assignment from a Compose env file without sourcing it."""
	value = ""
	for line in env_file.read_text(encoding="ascii").splitlines():
		if "=" not in line:
			continue
		key, candidate = line.split("=", 1)
		if key == setting_name:
			value = candidate.strip()
	return value


# ============================================
def effective_gateway_port(inputs: RunnerInputs, environ: dict[str, str]) -> int:
	"""Apply the launcher's inherited-port-over-env-file precedence and bounds checks."""
	configured_port = env_value(inputs.env_file, "PLE_GATEWAY_HOST_PORT")
	inherited_port = environ.get("PLE_GATEWAY_HOST_PORT", "")
	port_text = inherited_port if inherited_port else configured_port
	if not port_text:
		port_text = "8080"
	if not re.fullmatch(r"[0-9]+", port_text):
		raise RunnerError("PLE_GATEWAY_HOST_PORT must be an unquoted integer")
	port = int(port_text)
	if port < 1 or port > 65_535:
		raise RunnerError("PLE_GATEWAY_HOST_PORT must be between 1 and 65535")
	return port


# ============================================
def validate_compose_project_name(inputs: RunnerInputs, environ: dict[str, str]) -> None:
	"""Refuse project-name overrides that would make cleanup ownership ambiguous."""
	configured_name = env_value(inputs.env_file, "COMPOSE_PROJECT_NAME")
	effective_name = environ.get("COMPOSE_PROJECT_NAME", "") or configured_name
	if effective_name and effective_name != "containers":
		raise RunnerError("COMPOSE_PROJECT_NAME must be unset, empty, or exactly containers")


# ============================================
def credential_file(inputs: RunnerInputs) -> pathlib.Path:
	"""Return the selected env file's sibling local login boundary without reading it."""
	path = inputs.env_file.parent / "local-login.txt"
	return path


# ============================================
def validate_credential_file(path: pathlib.Path) -> None:
	"""Require the post-launch login handoff file to be private, regular, and readable."""
	validate_regular_readable_file(path, "required sibling local-login.txt")
	mode = stat.S_IMODE(path.stat().st_mode)
	if mode != 0o600:
		raise RunnerError("required sibling local-login.txt must have mode 0600")


# ============================================
def validate_existing_credential_file(inputs: RunnerInputs) -> None:
	"""Fail early only when an existing sibling credential is malformed or unsafe."""
	path = credential_file(inputs)
	if path.exists() or path.is_symlink():
		validate_credential_file(path)


# ============================================
def command_result(command: list[str], environ: dict[str, str] | None) -> CommandResult:
	"""Run one argv-array command without a shell and capture its redacted process result."""
	completed = subprocess.run(
		command,
		env=environ,
		text=True,
		capture_output=True,
		check=False,
	)
	result = CommandResult(completed.returncode, completed.stdout, completed.stderr)
	return result


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
		# This public title is needed only by the J13 browser child. It never enters a report.
		self.instructor_catalog_search_title: str | None = None
		self.visible_outcomes: dict[str, object] | None = None
		self.private_state_directory: pathlib.Path | None = None
		self.journey_state_file: pathlib.Path | None = None

	# ============================================
	def run_required(
		self,
		command: list[str],
		environ: dict[str, str] | None = None,
	) -> CommandResult:
		"""Run an argv command and emit bounded stage diagnostics without writing process output."""
		print(f"UI walkthrough: {self.report_stage} starting")
		result = self.run_command(command, environ)
		if result.returncode != 0:
			raise RunnerError(
				f"{self.report_stage} command failed with exit status {result.returncode}"
			)
		print(f"UI walkthrough: {self.report_stage} completed")
		return result

	# ============================================
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

	# ============================================
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

	# ============================================
	def prepare_report_directory(self) -> None:
		"""Create a private report parent only after all preflight and ownership checks pass."""
		self.ensure_report_directory()
		self.report_ready = True

	# ============================================
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

	# ============================================
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

	# ============================================
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
		result = self.run_command(command, None)
		if result.returncode != 0:
			raise RunnerError(f"cleanup command failed with exit status {result.returncode}")

	# ============================================
	def parse_arrangement_output(self, stdout: str) -> list[dict[str, str]]:
		"""Accept only the bounded public-reference object emitted by the fixed arranger."""
		try:
			encoded = stdout.encode("ascii")
		except UnicodeEncodeError as error:
			raise RunnerError("arrangement emitted invalid output") from error
		if (
			len(encoded) > MAX_ARRANGEMENT_OUTPUT_BYTES
			or not stdout.endswith("\n")
			or stdout.count("\n") != 1
			or "\r" in stdout
			or stdout[0].isspace()
		):
			raise RunnerError("arrangement emitted invalid output")
		try:
			payload = json.loads(stdout)
		except json.JSONDecodeError as error:
			raise RunnerError("arrangement emitted invalid output") from error
		if not isinstance(payload, dict) or set(payload) != {"arrangements"}:
			raise RunnerError("arrangement emitted invalid output")
		if stdout != json.dumps(payload, separators=(",", ":")) + "\n":
			raise RunnerError("arrangement emitted invalid output")
		arrangements = payload["arrangements"]
		if not isinstance(arrangements, list) or len(arrangements) not in {1, 5}:
			raise RunnerError("arrangement emitted invalid output")
		if len(arrangements) == 1:
			value = arrangements[0]
			if (
				not isinstance(value, dict)
				or set(value) != {"label", "problemId", "versionId", "catalogSearchTitle"}
				or value.get("label") != "api-retry-corpus-publication"
				or not isinstance(value["problemId"], str)
				or not isinstance(value["versionId"], str)
				or not isinstance(value["catalogSearchTitle"], str)
				or not UUID_TEXT.fullmatch(value["problemId"])
				or not UUID_TEXT.fullmatch(value["versionId"])
				or not CATALOG_SEARCH_TITLE.fullmatch(value["catalogSearchTitle"])
			):
				raise RunnerError("arrangement emitted invalid output")
			self.instructor_catalog_search_title = value["catalogSearchTitle"]
			return [
				{
					"label": value["label"],
					"problemId": value["problemId"],
					"versionId": value["versionId"],
				}
			]
		allowed = (
			({"label"}, "launcher-seeded-enrollment"),
			({"label", "baselineAssignmentId"}, "launcher-baseline-assignment"),
			({"label", "problemId", "versionId"}, "api-retry-corpus-publication"),
			(
				{
					"label", "courseId", "masteryAssignmentId",
				},
				"api-mastery-assignment",
			),
			(
				{"label", "courseId", "examAssignmentId"},
				"api-exam-assignment",
			),
		)
		validated: list[dict[str, str]] = []
		for value, (keys, label) in zip(arrangements, allowed, strict=True):
			if not isinstance(value, dict) or set(value) != keys or value.get("label") != label:
				raise RunnerError("arrangement emitted invalid output")
			checked: dict[str, str] = {}
			for key, identifier in value.items():
				if not isinstance(identifier, str):
					raise RunnerError("arrangement emitted invalid output")
				if key != "label" and not UUID_TEXT.fullmatch(identifier):
					raise RunnerError("arrangement emitted invalid output")
				checked[key] = identifier
			validated.append(checked)
		return validated

	# ============================================
	def arrange(self, child_environment: dict[str, str]) -> None:
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
		arranger_environment = child_environment.copy()
		manifest_file = self.inputs.env_file.parent / "local-demo.json"
		arranger_environment["PLE_UI_WALKTHROUGH_ARRANGER_CHILD"] = "1"
		arranger_environment["PLE_UI_WALKTHROUGH_LIVE_MANIFEST_FILE"] = str(manifest_file)
		self.report_stage = "arrangement"
		result = self.run_command(
			[node_path, str(arranger), "tests/e2e/ui_walkthrough_arrange.ts"],
			arranger_environment,
		)
		if result.returncode != 0:
			raise RunnerError("arrangement command failed")
		self.arrangements = self.parse_arrangement_output(result.stdout)
		if len(self.arrangements) == 1:
			return
		mastery = self.arrangements[3]
		exam = self.arrangements[4]
		corpus = self.arrangements[2]
		child_environment.update(
			{
				"PLE_UI_WALKTHROUGH_LIVE_COURSE_ID": mastery["courseId"],
				"PLE_UI_WALKTHROUGH_LIVE_MASTERY_ASSIGNMENT_ID": mastery[
					"masteryAssignmentId"
				],
				"PLE_UI_WALKTHROUGH_LIVE_EXAM_ASSIGNMENT_ID": exam["examAssignmentId"],
				"PLE_UI_WALKTHROUGH_LIVE_MASTERY_PROBLEM_ID": corpus["problemId"],
			}
		)

	# ============================================
	def arrange_instructor_setup(self, child_environment: dict[str, str]) -> None:
		"""Publish only the retry corpus before the browser creates its own course and assignment."""
		child_environment["PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY"] = "1"
		self.arrange(child_environment)
		if self.arrangements is None or len(self.arrangements) != 1:
			raise RunnerError("instructor setup arrangement emitted invalid output")
		if self.instructor_catalog_search_title is None:
			raise RunnerError("instructor setup arrangement emitted invalid output")
		corpus = self.arrangements[0]
		if set(corpus) != {"label", "problemId", "versionId"} or corpus["label"] != "api-retry-corpus-publication":
			raise RunnerError("instructor setup arrangement emitted invalid output")
		child_environment["PLE_UI_WALKTHROUGH_CATALOG_SEARCH_TITLE"] = (
			self.instructor_catalog_search_title
		)

	# ============================================
	def hand_off_instructor_setup(self, child_environment: dict[str, str]) -> None:
		"""Pass only validated public J11/J12/J13 identifiers to fixed student children."""
		if (
			self.journey_state_file is None
			or self.journey_state_file.name != "journeys.json"
			or self.journey_state_file.is_symlink()
		):
			raise RunnerError("instructor setup public-ID handoff is unavailable")
		parent = self.journey_state_file.parent
		parent_descriptor = -1
		file_descriptor = -1
		try:
			parent_descriptor = os.open(
				parent,
				os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0),
			)
			parent_metadata = os.fstat(parent_descriptor)
			file_descriptor = os.open(
				"journeys.json",
				os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0),
				dir_fd=parent_descriptor,
			)
		except OSError as error:
			raise RunnerError("instructor setup public-ID handoff is unavailable") from error
		try:
			metadata = os.fstat(file_descriptor)
			current_parent = parent.lstat()
			if (
				not stat.S_ISDIR(parent_metadata.st_mode)
				or stat.S_IMODE(parent_metadata.st_mode) != 0o700
				or not stat.S_ISREG(metadata.st_mode)
				or stat.S_IMODE(metadata.st_mode) != 0o600
				or metadata.st_size > 4096
				or not stat.S_ISDIR(current_parent.st_mode)
				or stat.S_ISLNK(current_parent.st_mode)
				or stat.S_IMODE(current_parent.st_mode) != 0o700
				or current_parent.st_dev != parent_metadata.st_dev
				or current_parent.st_ino != parent_metadata.st_ino
			):
				raise RunnerError("instructor setup public-ID handoff is unavailable")
			raw = os.read(file_descriptor, metadata.st_size)
		finally:
			os.close(file_descriptor)
			os.close(parent_descriptor)
		try:
			text = raw.decode("ascii")
			value = json.loads(text)
		except (UnicodeDecodeError, json.JSONDecodeError) as error:
			raise RunnerError("instructor setup public-ID handoff is unavailable") from error
		if text != json.dumps(value, separators=(",", ":")) + "\n":
			raise RunnerError("instructor setup public-ID handoff is unavailable")
		if not isinstance(value, list) or len(value) != 3:
			raise RunnerError("instructor setup public-ID handoff is unavailable")
		expected = (
			("J11", {"schemaVersion", "journey", "status", "elapsedMs", "courseId", "visibleOutcomeCodes", "diagnostics"}, ["visible_course_created", "visible_course_opened"]),
			("J12", {"schemaVersion", "journey", "status", "elapsedMs", "courseId", "visibleOutcomeCodes", "diagnostics"}, ["visible_local_student_active"]),
			("J13", {"schemaVersion", "journey", "status", "elapsedMs", "courseId", "assignmentId", "problemId", "versionId", "visibleOutcomeCodes", "diagnostics"}, ["visible_assignment_created", "visible_catalog_problem_selected", "visible_mastery_policy"]),
		)
		course_id: str | None = None
		for fragment, (journey, keys, outcome_codes) in zip(value, expected, strict=True):
			if not isinstance(fragment, dict) or set(fragment) != keys:
				raise RunnerError("instructor setup public-ID handoff is unavailable")
			if (
				fragment.get("schemaVersion") != 2
				or fragment.get("journey") != journey
				or fragment.get("status") != "PASS"
				or not isinstance(fragment.get("elapsedMs"), int)
				or isinstance(fragment.get("elapsedMs"), bool)
				or fragment["elapsedMs"] < 0
				or fragment["elapsedMs"] > MAX_JOURNEY_ELAPSED_MS
				or not isinstance(fragment.get("diagnostics"), list)
				or fragment["diagnostics"] != []
				or fragment.get("visibleOutcomeCodes") != outcome_codes
				or not isinstance(fragment.get("courseId"), str)
				or not LOWER_UUID_TEXT.fullmatch(fragment["courseId"])
			):
				raise RunnerError("instructor setup public-ID handoff is unavailable")
			if course_id is None:
				course_id = fragment["courseId"]
			elif fragment["courseId"] != course_id:
				raise RunnerError("instructor setup public-ID handoff is unavailable")
		j13 = value[2]
		if course_id is None or not isinstance(j13, dict):
			raise RunnerError("instructor setup public-ID handoff is unavailable")
		for key in ("assignmentId", "problemId", "versionId"):
			identifier = j13.get(key)
			if not isinstance(identifier, str) or not LOWER_UUID_TEXT.fullmatch(identifier):
				raise RunnerError("instructor setup public-ID handoff is unavailable")
		if self.arrangements is None or len(self.arrangements) != 1:
			raise RunnerError("instructor setup public-ID handoff is unavailable")
		corpus = self.arrangements[0]
		if j13["problemId"] != corpus["problemId"] or j13["versionId"] != corpus["versionId"]:
			raise RunnerError("instructor setup public-ID handoff is unavailable")
		child_environment.pop("PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY", None)
		child_environment.pop("PLE_UI_WALKTHROUGH_CATALOG_SEARCH_TITLE", None)
		child_environment.update(
			{
				"PLE_UI_WALKTHROUGH_LIVE_COURSE_ID": course_id,
				"PLE_UI_WALKTHROUGH_LIVE_MASTERY_ASSIGNMENT_ID": j13["assignmentId"],
				"PLE_UI_WALKTHROUGH_LIVE_MASTERY_PROBLEM_ID": j13["problemId"],
			}
		)

	# ============================================
	def prepare_journey_state(self, child_environment: dict[str, str]) -> None:
		"""Create one private runner-owned state file outside Playwright's artifact tree."""
		try:
			directory = pathlib.Path(tempfile.mkdtemp(prefix="ple-ui-walkthrough-"))
		except OSError as error:
			raise RunnerError("could not prepare private walkthrough state") from error
		directory.chmod(0o700)
		state_file = directory / "journeys.json"
		file_descriptor = os.open(state_file, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
		os.close(file_descriptor)
		self.private_state_directory = directory
		self.journey_state_file = state_file
		child_environment["PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE"] = str(state_file)
		alias_file = directory / "learner-alias.txt"
		alias_file.write_text("student-local\n", encoding="ascii")
		alias_file.chmod(0o600)
		child_environment["PLE_UI_WALKTHROUGH_LIVE_LEARNER_ALIAS_FILE"] = str(alias_file)
		if self.arrangements is None:
			raise RunnerError("arrangement evidence is unavailable")
		child_environment["PLE_UI_WALKTHROUGH_ARRANGEMENTS_JSON"] = json.dumps(
			[
				{
					"label": item["label"],
					"publicIds": {key: value for key, value in item.items() if key != "label"},
				}
				for item in self.arrangements
			],
			separators=(",", ":"),
		)

	# ============================================
	def collect_visible_outcomes(self, child_environment: dict[str, str]) -> None:
		"""Run only the fixed renderer and retain its bounded public journey result."""
		if self.journey_state_file is None or self.journey_state_file.is_symlink():
			raise RunnerError("visible outcome evidence is unavailable")
		node_path = shutil.which("node")
		arranger = self.repository_root / ARRANGER_RELATIVE_PATH
		if node_path is None or not arranger.is_file() or arranger.is_symlink():
			raise RunnerError("fixed visible-outcome renderer is unavailable")
		self.report_stage = "visible_outcome_report"
		result = self.run_command(
			[node_path, str(arranger), "tests/e2e/ui_walkthrough_report.ts"],
			child_environment,
		)
		if result.returncode != 0:
			raise RunnerError("visible outcome renderer failed")
		self.visible_outcomes = self.parse_visible_outcome_output(result.stdout)

	# ============================================
	def playwright_child_environment(self, child_environment: dict[str, str]) -> dict[str, str]:
		"""Disable Playwright's persisted AI page snapshot only for the sensitive live child."""
		playwright_environment = child_environment.copy()
		playwright_environment["PLAYWRIGHT_NO_COPY_PROMPT"] = "1"
		return playwright_environment

	# ============================================
	def parse_visible_outcome_output(self, stdout: str) -> dict[str, object]:
		"""Accept only the fixed renderer's compact public report envelope."""
		try:
			encoded = stdout.encode("ascii")
			payload = json.loads(stdout)
		except (UnicodeEncodeError, json.JSONDecodeError) as error:
			raise RunnerError("visible outcome renderer emitted invalid output") from error
		if (
			len(encoded) > MAX_VISIBLE_OUTCOME_OUTPUT_BYTES
			or not stdout.endswith("\n")
			or stdout.count("\n") != 1
			or not isinstance(payload, dict)
			or set(payload) != {
				"schemaVersion", "status", "masterSeed", "stage", "elapsedMs", "arrangements", "journeys"
			}
			or payload.get("schemaVersion") != 1
			or payload.get("status") not in {"PASS", "FAIL"}
			or payload.get("masterSeed") != self.inputs.master_seed
			or payload.get("stage") != "complete"
			or not isinstance(payload.get("elapsedMs"), int)
			or not isinstance(payload.get("arrangements"), list)
			or not isinstance(payload.get("journeys"), list)
		):
			raise RunnerError("visible outcome renderer emitted invalid output")
		if stdout != json.dumps(payload, separators=(",", ":")) + "\n":
			raise RunnerError("visible outcome renderer emitted invalid output")
		return payload

	# ============================================
	def remove_private_state(self) -> None:
		"""Remove only the exact runner-created private state directory on every finish path."""
		if self.private_state_directory is None:
			return
		if self.private_state_directory.is_symlink():
			raise RunnerError("walkthrough private state path must not contain a symlink")
		shutil.rmtree(self.private_state_directory)
		self.private_state_directory = None
		self.journey_state_file = None

	# ============================================
	def finish(self, success: bool) -> int:
		"""Perform conservative cleanup, write the redacted report, and return a process status."""
		if success:
			self.report_status = "PASS"
			if not self.inputs.student_repeat_only:
				self.report_stage = "complete"
		if self.stack_launch_attempted and self.inputs.keep:
			print("UI walkthrough: preserving stack started by this runner")
		cleanup_failed = False
		try:
			self.compose_down()
		except (OSError, RunnerError, UnicodeError) as error:
			cleanup_failed = True
			if success:
				self.report_status = "FAIL"
				self.report_stage = "cleanup"
			print(f"FAIL: cleanup failed: {error}", file=sys.stderr)
		try:
			self.remove_private_state()
		except (OSError, RunnerError, UnicodeError) as error:
			cleanup_failed = True
			if success:
				self.report_status = "FAIL"
				self.report_stage = "cleanup"
			print(f"FAIL: private state cleanup failed: {error}", file=sys.stderr)
		try:
			self.write_report()
		except (OSError, UnicodeError, RunnerError):
			print("FAIL: could not write walkthrough report", file=sys.stderr)
			return 1
		return 0 if success and not cleanup_failed else 1

	# ============================================
	def execute(self) -> None:
		"""Run the fail-closed lifecycle from preflight through the WP-O2 Playwright boundary."""
		validate_existing_credential_file(self.inputs)
		effective_gateway_port(self.inputs, self.environ)
		validate_compose_project_name(self.inputs, self.environ)
		self.configure_compose()
		self.assert_no_existing_stack()
		self.prepare_report_directory()

		self.report_stage = "launcher_check"
		launcher = str(self.repository_root / "launch_local_stack.sh")
		self.run_required([launcher, "--check", "--env-file", str(self.inputs.env_file)])

		self.report_stage = "launcher_start"
		self.assert_no_existing_stack()
		start_command = [launcher, "--no-open"]
		if launcher_skip_build(self.inputs, self.repository_root):
			start_command.append("--skip-build")
		start_command.extend(["--env-file", str(self.inputs.env_file)])
		self.stack_launch_attempted = True
		self.run_required(start_command)

		self.report_stage = "live_boundary"
		gateway_port = effective_gateway_port(self.inputs, self.environ)
		login_file = credential_file(self.inputs)
		validate_credential_file(login_file)
		child_environment = self.environ.copy()
		child_environment.update(
			{
				"PLE_UI_WALKTHROUGH_LIVE_REQUIRED": "1",
				"PLE_UI_WALKTHROUGH_LIVE_BASE_URL": f"http://127.0.0.1:{gateway_port}",
				"PLE_UI_WALKTHROUGH_LIVE_CREDENTIAL_FILE": str(login_file),
				"PLE_UI_WALKTHROUGH_MASTER_SEED": str(self.inputs.master_seed),
			}
		)
		self.arrange_instructor_setup(child_environment)
		self.prepare_journey_state(child_environment)
		self.report_stage = "playwright_instructor_setup"
		self.run_required(
			[
				"bash",
				"run_playwright_tests.sh",
				"tests/playwright/ui_walkthrough_instructor_setup.spec.ts",
			],
			self.playwright_child_environment(child_environment),
		)
		if self.inputs.instructor_setup_only:
			return
		self.report_stage = "instructor_setup_handoff"
		self.hand_off_instructor_setup(child_environment)
		self.report_stage = "playwright_j1"
		self.run_required(
			[
				"bash",
				"run_playwright_tests.sh",
				"tests/playwright/ui_walkthrough_keyboard_j1.spec.ts",
			],
			self.playwright_child_environment(child_environment),
		)
		self.report_stage = "playwright_j2"
		self.run_required(
			[
				"bash",
				"run_playwright_tests.sh",
				"tests/playwright/ui_walkthrough_keyboard_j2.spec.ts",
			],
			self.playwright_child_environment(child_environment),
		)
		for stage, specification in (
			("playwright_j3", "tests/playwright/ui_walkthrough_keyboard_j3.spec.ts"),
			("playwright_j4", "tests/playwright/ui_walkthrough_keyboard_j4.spec.ts"),
		):
			self.report_stage = stage
			self.run_required(
				["bash", "run_playwright_tests.sh", specification],
				self.playwright_child_environment(child_environment),
			)
		if self.inputs.student_repeat_only:
			self.report_stage = "student_repeat_complete"
			return
		raise RunnerError("full walkthrough completion awaits WP-S2 and WP-E1")


# ============================================
def main(argv: list[str]) -> int:
	"""Parse inputs, run the lifecycle, and retain a redacted failure record after report setup."""
	args = parse_args(argv)
	repository_root = pathlib.Path(__file__).resolve().parents[2]
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


if __name__ == "__main__":
	raise SystemExit(main(sys.argv[1:]))
