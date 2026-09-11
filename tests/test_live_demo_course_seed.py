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
		mary_attempt_completed=True,
		mary_graded_question_count=4,
		jack_saved_response_count=2,
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
def test_open_saved_jack_work_plans_only_remaining_work() -> None:
	"""Jack's open saved work remains the only incomplete baseline stage."""
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
		mary_attempt_completed=True,
		mary_graded_question_count=4,
		jack_saved_response_count=1,
	)

	planned = local_stack_control.live_demo_course_seed.plan_stages(observed)

	assert planned == (local_stack_control.live_demo_course_seed.Stage.WORK,)


#============================================
def test_graded_mary_work_requires_completed_assignment_attempt() -> None:
	"""Gradebook counts alone cannot prove Mary's whole-Attempt finalization."""
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
		mary_graded_question_count=4,
		jack_saved_response_count=2,
	)

	assert observed.work_complete is False
	assert local_stack_control.live_demo_course_seed.plan_stages(observed) == (
		local_stack_control.live_demo_course_seed.Stage.WORK,
	)
