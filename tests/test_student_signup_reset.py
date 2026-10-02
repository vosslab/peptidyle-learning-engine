"""Instructor signup reset reissues one invitation and restore reuses the Student Record."""

# Standard Library
import pathlib
import re


ACCESS = (
	pathlib.Path(__file__).resolve().parents[1]
	/ "schemas/base_schema/50_functions/course_roster_access.sql"
)


def _function(name: str) -> str:
	text = ACCESS.read_text(encoding="utf-8")
	match = re.search(rf"CREATE FUNCTION {name}\(.*?\n\$\$;", text, re.DOTALL)
	assert match is not None, name
	return match.group(0)


def test_signup_reset_invalidates_prior_access_and_issues_one_invitation() -> None:
	"""The shipped reset ends access, revokes open invitations, and reads the delivery email."""
	body = _function(r"ple_api\.reset_student_signup_access")
	assert "course_membership_event" in body
	assert "'ended'" in body
	assert "'revoked'" in body
	assert "INSERT INTO ple_private.course_invitation" in body
	assert body.count("INSERT INTO ple_private.course_invitation (") == 1
	assert "pending_course_invitation_delivery_email" in body
	assert "INSERT INTO ple_private.account" not in body
	assert "DELETE FROM" not in body


def test_course_access_restore_reuses_the_student_record() -> None:
	"""Restore inserts one membership on the existing Student Record and creates no Account."""
	body = _function(r"ple_api\.restore_student_course_access")
	assert "student_record_id" in body
	assert "INSERT INTO ple_data.course_membership" in body
	assert "INSERT INTO ple_data.student_record" not in body
	assert "INSERT INTO ple_private.account" not in body
	assert "DELETE FROM" not in body
	assert "student_access_restored" in body
