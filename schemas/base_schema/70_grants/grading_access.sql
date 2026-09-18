-- Privileges from grading_access.sql.

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.has_automated_grading_receipt(uuid, uuid),
    ple_api.read_course_gradebook(text)
FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.has_automated_grading_receipt(uuid, uuid)
    TO ple_private_owner;

GRANT EXECUTE ON FUNCTION ple_api.read_course_gradebook(text)
    TO ple_app;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.reject_grading_evidence_change(),
    ple_private.grade_contribution_points_possible(text, text, numeric),
    ple_private.record_direct_automated_grading_result(uuid, uuid, numeric, timestamptz),
    ple_private.read_assessment_gradebook_evidence(uuid, uuid)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.read_assessment_gradebook_evidence(uuid, uuid)
    TO ple_api_owner;

SET LOCAL ROLE ple_audit_owner;

REVOKE ALL ON FUNCTION ple_audit.reject_automated_grading_receipt_change() FROM PUBLIC;

