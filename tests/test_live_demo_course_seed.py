"""Convergence planning for the declared Live Demo teaching-data baseline."""

import local_stack_control.live_demo_course_seed


#============================================
def test_fully_provisioned_state_plans_no_stages() -> None:
	"""Every declared product fact closes its corresponding stage."""
	observed = local_stack_control.live_demo_course_seed.ObservedState(
		blueprint_complete=True,
		course_complete=True,
		roster_complete=True,
		claims_complete=True,
		assignment_complete=True,
		selection_complete=True,
		release_complete=True,
		mary_attempt_exists=True,
		jack_attempt_exists=True,
		mary_terminal_submission_count=4,
		jack_terminal_submission_count=2,
	)

	assert local_stack_control.live_demo_course_seed.plan_stages(observed) == ()


#============================================
def test_course_with_roster_but_no_claims_plans_claims_and_later_stages() -> None:
	"""A partial baseline resumes after its already complete Course and roster."""
	observed = local_stack_control.live_demo_course_seed.ObservedState(
		blueprint_complete=True,
		course_complete=True,
		roster_complete=True,
	)

	planned = local_stack_control.live_demo_course_seed.plan_stages(observed)

	assert planned == (
		local_stack_control.live_demo_course_seed.Stage.CLAIMS,
		local_stack_control.live_demo_course_seed.Stage.ASSIGNMENT,
		local_stack_control.live_demo_course_seed.Stage.SELECTION,
		local_stack_control.live_demo_course_seed.Stage.RELEASE,
		local_stack_control.live_demo_course_seed.Stage.ATTEMPTS,
		local_stack_control.live_demo_course_seed.Stage.WORK,
	)


#============================================
def test_partially_answered_mary_attempt_plans_only_remaining_work() -> None:
	"""Two of Mary's four terminal submissions resume at the work stage."""
	observed = local_stack_control.live_demo_course_seed.ObservedState(
		blueprint_complete=True,
		course_complete=True,
		roster_complete=True,
		claims_complete=True,
		assignment_complete=True,
		selection_complete=True,
		release_complete=True,
		mary_attempt_exists=True,
		jack_attempt_exists=True,
		mary_terminal_submission_count=2,
		jack_terminal_submission_count=2,
	)

	planned = local_stack_control.live_demo_course_seed.plan_stages(observed)

	assert planned == (local_stack_control.live_demo_course_seed.Stage.WORK,)
