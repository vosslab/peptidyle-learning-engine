"""Public-presentation response projection for Live Demo Student work."""

# local repo modules
import local_stack_control.live_demo_course_provision
import local_stack_control.live_demo_course_seed


#============================================
def test_pinned_recipes_resolve_only_presented_response_references() -> None:
	"""Choice and hotspot recipes become exact public Student Response shapes."""
	recipes = {
		response.question_id: response
		for response in local_stack_control.live_demo_course_seed.SEEDED_STUDENT_RESPONSES[0].responses
	}
	single_choice = {
		"questionRevision": {"questionId": "PNE-0001"},
		"response": {
			"kind": "singleChoice",
			"choices": [{"id": "a001"}, {"id": "a002"}],
		},
	}
	numerical = {
		"questionRevision": {"questionId": "PNE-0003"},
		"response": {"kind": "numerical"},
	}
	hotspot = {
		"questionRevision": {"questionId": "PNE-0004"},
		"response": {
			"kind": "hotspot",
			"surface": {"regions": [{"id": "b001"}, {"id": "b002"}]},
		},
	}

	assert local_stack_control.live_demo_course_provision._presented_response(
		single_choice, recipes["PNE-0001"]
	) == {"kind": "multipleChoice", "selected": ["a001"]}
	assert local_stack_control.live_demo_course_provision._presented_response(
		numerical, recipes["PNE-0003"]
	) == {"kind": "numeric", "value": 1}
	assert local_stack_control.live_demo_course_provision._presented_response(
		hotspot, recipes["PNE-0004"]
	) == {"kind": "hotspot", "selections": [{"region": "b002"}]}
