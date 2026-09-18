-- Privileges from assessment_attempts.sql.

SET LOCAL ROLE ple_data_owner;

-- Student Work roots.  An Assessment Attempt retains the effective facts that
-- later resume, submission, grading, history, disclosure, and statistics reads
-- need; current Assessment rows remain the authority only when an Assessment Attempt starts.

-- The private Student Work root owns FKs to these public Course/Assessment and
-- immutable Question Revision identities.  Current Assessment children are
-- intentionally evidence values below, not FK targets.
GRANT REFERENCES ON TABLE ple_data.student_record, ple_data.assessment,
    ple_data.question_revision, ple_data.question_pool_revision TO ple_private_owner;

REVOKE ALL ON FUNCTION ple_data.student_assessment_has_course_scope(uuid, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_data.student_assessment_has_course_scope(uuid, text) TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.student_assessment_accommodation, ple_private.assessment_attempt,
    ple_private.question_pool_selection, ple_private.question_pool_selected_item, ple_private.issued_question FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.assert_student_assessment_accommodation_scope(), ple_private.assert_assessment_attempt_scope(),
    ple_private.enforce_student_assessment_accommodation_edit(), ple_private.reject_student_work_delete(), ple_private.reject_assessment_attempt_rewrite(),
    ple_private.reject_immutable_student_work_change(),
    ple_private.validate_question_pool_selected_item_count(), ple_private.validate_question_pool_selected_item_member(),
    ple_private.validate_question_pool_selection_issues(),
    ple_private.validate_issued_question_reproduction() FROM PUBLIC;

