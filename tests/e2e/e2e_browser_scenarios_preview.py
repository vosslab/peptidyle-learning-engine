"""WP-PROF-T3 assignment-delivery preview scenario facts."""

from e2e_browser_scenario_contract import ScenarioContract


def contracts() -> tuple[ScenarioContract, ...]:
	"""Return the independently selectable real-stack preview-plane journey."""
	return (
		ScenarioContract(
			scenario_id="preview_plane",
			spec_path="tests/playwright/e2e/assignment_preview.spec.ts",
			personas=("elena_instructor", "mary_student"),
			baseline_reads=("base_course", "published_peptide_assignment"),
			ui_creates=("assignment", "course_group"),
			sysadmin_requirement="not_required",
			visible_observation=(
				"derived_and_synthetic_delivery_preview_recovers_a_preserved_stale_draft"
			),
			screenshot_states=(
				"schedule_entitlement",
				"derived_resolved",
				"revision_conflict",
				"revision_reloaded",
				"synthetic_resolved",
				"assignment_preview_denial",
			),
		),
	)
