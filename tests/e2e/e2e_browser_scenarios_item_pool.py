"""Assignment Question Pool delivery browser scenario contract."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the independently selectable Instructor-to-Student pool journey."""
	return (
		ScenarioContract(
			scenario_id="item_pool_delivery",
			spec_path="tests/playwright/e2e/item_pool_delivery.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("seeded_accounts",),
			ui_creates=("question", "blueprint", "course", "assignment", "invitation"),
			visible_observation=(
				"student_receives_fixed_then_ordered_pool_membership_and_issued_work_blocks_pool_edits"
			),
		),
	)
