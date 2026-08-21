"""Invoke the fixed browser journey through its explicit private configuration."""

import collections.abc
import pathlib

import tests.walkthrough.walklib as walklib
import tests.walkthrough.walklib.models as models


WALKTHROUGH_SPECIFICATIONS_BY_STAGE = {
	"gateway_smoke": "tests/playwright/ui_walkthrough_smoke.spec.ts",
	"instructor_setup": "tests/playwright/ui_walkthrough_instructor_setup.spec.ts",
	"j1": "tests/playwright/ui_walkthrough_keyboard_j1.spec.ts",
	"j2": "tests/playwright/ui_walkthrough_keyboard_j2.spec.ts",
	"j3": "tests/playwright/ui_walkthrough_keyboard_j3.spec.ts",
	"j4": "tests/playwright/ui_walkthrough_keyboard_j4.spec.ts",
	"j5": "tests/playwright/ui_walkthrough_keyboard_j5.spec.ts",
}
WALKTHROUGH_SPECIFICATIONS = frozenset(WALKTHROUGH_SPECIFICATIONS_BY_STAGE.values())


#============================================
def specification_for_stage(stage: str) -> str:
	"""Return one authored walkthrough spec from the canonical closed inventory."""
	try:
		result = WALKTHROUGH_SPECIFICATIONS_BY_STAGE[stage]
	except KeyError as error:
		raise walklib.models.RunnerError("walkthrough Playwright stage is unavailable") from error
	return result


#============================================
def run_specification(
	playwright_config_file: pathlib.Path | None,
	specification: str,
	run_required: collections.abc.Callable[
		[list[str], dict[str, str]], models.CommandResult
	],
	environment: dict[str, str],
) -> None:
	"""Run one fixed journey without ambient walkthrough configuration.

	Args:
		playwright_config_file: Runner-owned private ESM config passed with ``--config``.
		specification: Repository-fixed Playwright specification path.
		run_required: Runner process boundary that reports the active public stage.
		environment: Already-sanitized environment for this child process.

	Raises:
		walklib.models.RunnerError: The private config is unavailable.
	"""
	if playwright_config_file is None:
		raise walklib.models.RunnerError("private Playwright configuration is unavailable")
	if specification not in WALKTHROUGH_SPECIFICATIONS:
		raise walklib.models.RunnerError("walkthrough Playwright specification is unavailable")
	run_required(
		[
			"npx",
			"playwright",
			"test",
			"--config",
			str(playwright_config_file),
			specification,
		],
		environment,
	)
