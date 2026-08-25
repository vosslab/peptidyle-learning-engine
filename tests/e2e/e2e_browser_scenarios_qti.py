"""Production QTI import scenario facts for the disposable browser suite."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the instructor's visible Canvas QTI conversion journey."""
	return (
		ScenarioContract(
			scenario_id="qti_profile_import",
			spec_path="tests/playwright/e2e/qti_profile_import_real.spec.ts",
			personas=("elena_instructor",),
			baseline_reads=("base_course",),
			ui_creates=("question", "qti_import"),
			visible_observation="converted_qti_draft_persists_after_fresh_elena_session",
		),
	)
