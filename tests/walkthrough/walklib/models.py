"""Typed values shared across UI walkthrough orchestration capabilities."""

import dataclasses
import pathlib
from collections.abc import Callable


class RunnerError(RuntimeError):
	"""Describe a fail-closed walkthrough preflight or lifecycle problem."""


@dataclasses.dataclass(frozen=True)
class RunnerInputs:
	"""Validated command-line inputs and derived local-stack paths."""

	master_seed: int
	env_file: pathlib.Path
	report_basename: str
	keep: bool
	force_build: bool
	instructor_setup_only: bool
	student_repeat_only: bool
	screenshot_directory: pathlib.Path | None = None


@dataclasses.dataclass(frozen=True)
class ArrangementChildInputs:
	"""Minimal private handoff for the manifest-only Node arrangement child."""

	chapter_one_manifest_file: pathlib.Path


@dataclasses.dataclass(frozen=True)
class WalkthroughChildInputs:
	"""Versioned private values passed explicitly to fixed walkthrough children."""

	stage: str
	base_url: str
	master_seed: int
	credential_file: pathlib.Path
	journey_state_file: pathlib.Path | None = None
	instructor_setup_checkpoint_file: pathlib.Path | None = None
	j1_checkpoint_file: pathlib.Path | None = None
	j2_checkpoint_file: pathlib.Path | None = None
	catalog_display_ids: tuple[str, str, str, str] | None = None
	course_reference: str | None = None
	mastery_assignment_reference: str | None = None
	screenshot_directory: pathlib.Path | None = None


@dataclasses.dataclass(frozen=True)
class CommandResult:
	"""Small process result surface that keeps tests independent of Podman."""

	returncode: int
	stdout: str
	stderr: str


CommandRunner = Callable[[list[str], dict[str, str] | None, str | None], CommandResult]
