-- Privileges from assessment_attempt_operations_api.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.lock_assessment_for_student_work(text),
    ple_private.assert_current_student_assessment_attempt(uuid),
    ple_private.assessment_attempt_start_gate(uuid, text),
    ple_private.start_assessment_attempt(uuid, uuid, text, jsonb, jsonb),
    ple_private.prepare_current_assessment_attempt_start_decision(text, text),
    ple_private.prepare_current_assessment_attempt_start(text, text),
    ple_private.read_started_student_assessment_attempt(uuid),
    ple_private.prepare_assessment_attempt_finalization(uuid),
    ple_private.prepare_student_assessment_attempt_finalization(uuid),
    ple_private.commit_student_assessment_attempt_finalization(uuid, text, jsonb),
    ple_private.commit_assessment_attempt_finalization(uuid, text, jsonb, text),
    ple_private.prepare_expired_student_assessment_attempt_finalizations(integer),
    ple_private.commit_expired_student_assessment_attempt_finalization(uuid, jsonb),
    ple_private.save_student_assessment_attempt_response(uuid, integer, jsonb),
    ple_private.checkpoint_student_question_display_duration(uuid, integer, bigint),
    ple_private.read_student_assessment_attempt_progress(uuid),
    ple_private.read_student_assessment_attempt_saved_response(uuid, integer),
    ple_private.lock_question_attempt_for_grading(uuid),
    ple_private.read_student_assessment_attempt_history_evidence(uuid),
    ple_private.save_student_assessment_accommodation(uuid, uuid, text, bigint, timestamptz, timestamptz, timestamptz, numeric, integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.lock_assessment_for_student_work(text),
    ple_private.start_assessment_attempt(uuid, uuid, text, jsonb, jsonb),
    ple_private.prepare_current_assessment_attempt_start_decision(text, text),
    ple_private.prepare_current_assessment_attempt_start(text, text),
    ple_private.read_started_student_assessment_attempt(uuid),
    ple_private.save_student_assessment_attempt_response(uuid, integer, jsonb),
    ple_private.checkpoint_student_question_display_duration(uuid, integer, bigint),
    ple_private.prepare_student_assessment_attempt_finalization(uuid),
    ple_private.commit_student_assessment_attempt_finalization(uuid, text, jsonb),
    ple_private.prepare_expired_student_assessment_attempt_finalizations(integer),
    ple_private.commit_expired_student_assessment_attempt_finalization(uuid, jsonb),
    ple_private.read_student_assessment_attempt_progress(uuid),
    ple_private.read_student_assessment_attempt_saved_response(uuid, integer),
    ple_private.read_student_assessment_attempt_history_evidence(uuid),
    ple_private.save_student_assessment_accommodation(uuid, uuid, text, bigint, timestamptz, timestamptz, timestamptz, numeric, integer) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.start_assessment_attempt(uuid, uuid, text, jsonb, jsonb),
    ple_api.prepare_current_assessment_attempt_start_decision(text, text),
    ple_api.prepare_current_assessment_attempt_start(text, text),
    ple_api.read_started_student_assessment_attempt(uuid),
    ple_api.save_student_assessment_attempt_response(uuid, integer, jsonb),
    ple_api.checkpoint_student_question_display_duration(uuid, integer, bigint),
    ple_api.prepare_student_assessment_attempt_finalization(uuid),
    ple_api.commit_student_assessment_attempt_finalization(uuid, text, jsonb),
    ple_api.prepare_expired_student_assessment_attempt_finalizations(integer),
    ple_api.commit_expired_student_assessment_attempt_finalization(uuid, jsonb),
    ple_api.read_student_assessment_attempt_progress(uuid),
    ple_api.read_student_assessment_attempt_saved_response(uuid, integer),
    ple_api.read_student_assessment_attempt_history_evidence(uuid),
    ple_api.save_student_assessment_accommodation(uuid, uuid, text, bigint, timestamptz, timestamptz, timestamptz, numeric, integer)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.start_assessment_attempt(uuid, uuid, text, jsonb, jsonb),
    ple_api.prepare_current_assessment_attempt_start_decision(text, text),
    ple_api.prepare_current_assessment_attempt_start(text, text),
    ple_api.read_started_student_assessment_attempt(uuid),
    ple_api.save_student_assessment_attempt_response(uuid, integer, jsonb),
    ple_api.checkpoint_student_question_display_duration(uuid, integer, bigint),
    ple_api.prepare_student_assessment_attempt_finalization(uuid),
    ple_api.commit_student_assessment_attempt_finalization(uuid, text, jsonb),
    ple_api.read_student_assessment_attempt_progress(uuid),
    ple_api.read_student_assessment_attempt_saved_response(uuid, integer),
    ple_api.read_student_assessment_attempt_history_evidence(uuid),
    ple_api.save_student_assessment_accommodation(uuid, uuid, text, bigint, timestamptz, timestamptz, timestamptz, numeric, integer) TO ple_app;

GRANT EXECUTE ON FUNCTION ple_api.prepare_expired_student_assessment_attempt_finalizations(integer),
    ple_api.commit_expired_student_assessment_attempt_finalization(uuid, jsonb)
    TO ple_assessment_attempt_expiry_worker;
