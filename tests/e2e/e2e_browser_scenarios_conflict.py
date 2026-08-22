"""Two-session optimistic-concurrency browser scenario facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the ordinary instructor conflict-recovery journey."""
	return (
		ScenarioContract(
			scenario_id="grade_settings_conflict",
			spec_path="tests/playwright/e2e/instructor_grade_settings_conflict.spec.ts",
			personas=("elena_instructor",),
			baseline_reads=("base_course",),
			ui_creates=("course", "grade_scheme"),
			sysadmin_requirement="not_required",
			visible_observation="stale_grade_settings_preserve_local_draft_and_retry_persists",
			screenshot_states=("conflict_detected", "retry_saved", "remote_observed"),
		),
	)
