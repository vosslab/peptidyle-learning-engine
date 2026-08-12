"""Invoke the fixed browser journey through its explicit private configuration."""

import pathlib
import collections.abc

import walklib.models


#============================================
def run_specification(
	playwright_config_file: pathlib.Path | None,
	specification: str,
	run_required: collections.abc.Callable[
		[list[str], dict[str, str]], walklib.models.CommandResult
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
	run_required(
		[
			"bash",
			"run_playwright_tests.sh",
			"--config",
			str(playwright_config_file),
			specification,
		],
		environment,
	)
