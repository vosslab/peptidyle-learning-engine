-- Privileges from assessment_attempt_presentation.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.question_attempt_presentation_binding,
    ple_private.question_attempt_response_item_binding,
    ple_private.question_attempt_presentation_asset_binding,
    ple_private.question_attempt_presentation_asset_rendition FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.reject_question_attempt_presentation_change(),
    ple_private.validate_question_attempt_reproduction(),
    ple_private.validate_author_content_presentation() FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.require_owned_assessment_attempt_for_presentation(uuid),
    ple_private.prepare_student_assessment_attempt_presentation(uuid),
    ple_private.commit_student_assessment_attempt_presentation(uuid, jsonb),
    ple_private.read_student_assessment_attempt_presentation_evidence(uuid, integer),
    ple_private.read_student_assessment_attempt_presentation_evidence_set(uuid),
    ple_private.read_student_assessment_attempt_backend_document(uuid, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.prepare_student_assessment_attempt_presentation(uuid),
    ple_private.commit_student_assessment_attempt_presentation(uuid, jsonb),
    ple_private.read_student_assessment_attempt_presentation_evidence(uuid, integer),
    ple_private.read_student_assessment_attempt_presentation_evidence_set(uuid),
    ple_private.read_student_assessment_attempt_backend_document(uuid, integer) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.prepare_student_assessment_attempt_presentation(uuid),
    ple_api.commit_student_assessment_attempt_presentation(uuid, jsonb),
    ple_api.read_student_assessment_attempt_presentation_evidence(uuid, integer),
    ple_api.read_student_assessment_attempt_presentation_evidence_set(uuid),
    ple_api.read_student_assessment_attempt_backend_document(uuid, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.prepare_student_assessment_attempt_presentation(uuid),
    ple_api.commit_student_assessment_attempt_presentation(uuid, jsonb),
    ple_api.read_student_assessment_attempt_presentation_evidence(uuid, integer),
    ple_api.read_student_assessment_attempt_presentation_evidence_set(uuid),
    ple_api.read_student_assessment_attempt_backend_document(uuid, integer) TO ple_app;

