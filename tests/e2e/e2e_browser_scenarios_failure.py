"""Lifecycle-controlled recovery journeys owned by the production browser suite."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the lifecycle-controlled native PLE recovery journey."""
	return (
		ScenarioContract(
			scenario_id="learner_native_ple_recovery",
			spec_path="tests/playwright/e2e/learner_native_ple_recovery.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("base_course",),
			ui_creates=("question", "course", "assignment", "invitation", "response"),
			visible_observation="accepted_response_recovers_after_native_ple_worker_interruption",
			fault_transition="native_ple_submission_recovery",
		),
	)
