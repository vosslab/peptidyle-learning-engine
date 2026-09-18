-- Functions, triggers, and views from corrections.sql.

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.reject_forced_question_correction_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Forced Question Correction evidence is immutable';
END
$$;

CREATE TRIGGER forced_question_correction_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.forced_question_correction
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_forced_question_correction_change();

CREATE TRIGGER forced_question_correction_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_change_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_forced_question_correction_change();

SET LOCAL ROLE ple_audit_owner;

CREATE FUNCTION ple_audit.reject_forced_question_correction_target_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    IF TG_OP = 'DELETE'
       AND TG_TABLE_NAME IN (
           'forced_question_correction_assessment_attempt_target',
           'forced_question_correction_issued_question_target'
       ) THEN
        -- FK actions run under the referenced-table owner.  The ordinary
        -- direct-delete path is still restricted to the executor; a rooted
        -- Student Work cascade reaches this trigger at nested depth.
        IF current_user = 'ple_unrelease_executor'
           OR pg_catalog.pg_trigger_depth() > 1 THEN
            RETURN OLD;
        END IF;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Forced Question Correction target evidence is immutable';
END
$$;

CREATE TRIGGER forced_question_correction_assessment_attempt_target_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.forced_question_correction_assessment_attempt_target
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_forced_question_correction_target_change();

CREATE TRIGGER forced_question_correction_issued_question_target_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.forced_question_correction_issued_question_target
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_forced_question_correction_target_change();

