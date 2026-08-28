"""UI-first production delivery facts for the WP-INST-T5 item-pool journey."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the independently selectable Instructor-to-Student pool journey."""
	return (
		ScenarioContract(
			scenario_id="item_pool_delivery",
			spec_path="tests/playwright/e2e/item_pool_delivery.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("base_course",),
			ui_creates=("question", "course", "assignment", "invitation", "response"),
			visible_observation=(
				"server_sampled_pool_draw_delivers_fixed_then_ordered_membership_and_preserves_issued_work"
			),
			screenshot_states=("pool_preview", "learner_delivered_pool"),
		),
	)
