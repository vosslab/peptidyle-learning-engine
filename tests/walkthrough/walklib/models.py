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


@dataclasses.dataclass(frozen=True)
class CommandResult:
	"""Small process result surface that keeps tests independent of Podman."""

	returncode: int
	stdout: str
	stderr: str


CommandRunner = Callable[[list[str], dict[str, str] | None], CommandResult]
