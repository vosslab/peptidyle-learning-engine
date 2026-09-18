-- Functions, triggers, and views from assessment_attempt_interaction.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.enforce_question_attempt_state_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.question_attempt_id IS DISTINCT FROM OLD.question_attempt_id
       OR NEW.issued_question_id IS DISTINCT FROM OLD.issued_question_id
       OR NEW.question_seed IS DISTINCT FROM OLD.question_seed
       OR NEW.generated_parameter_sha256 IS DISTINCT FROM OLD.generated_parameter_sha256
       OR NEW.issued_at IS DISTINCT FROM OLD.issued_at
       OR NEW.deadline_at IS DISTINCT FROM OLD.deadline_at
       OR NEW.backend_name IS DISTINCT FROM OLD.backend_name
       OR NEW.backend_version IS DISTINCT FROM OLD.backend_version
       OR NEW.renderer_name IS DISTINCT FROM OLD.renderer_name
       OR NEW.renderer_version IS DISTINCT FROM OLD.renderer_version
       OR NEW.source_object_record_id IS DISTINCT FROM OLD.source_object_record_id
       OR NEW.source_object_checksum IS DISTINCT FROM OLD.source_object_checksum
       OR NEW.grader_name IS DISTINCT FROM OLD.grader_name
       OR NEW.grader_version IS DISTINCT FROM OLD.grader_version
       OR NEW.rendered_question_sha256 IS DISTINCT FROM OLD.rendered_question_sha256
       OR NEW.issued_capability IS DISTINCT FROM OLD.issued_capability THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Question Attempt reproduction evidence is immutable';
    END IF;
    IF OLD.question_attempt_state <> 'open' AND NEW.question_attempt_state <> OLD.question_attempt_state THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Attempt State is forward-only';
    END IF;
    IF NEW.question_attempt_state = 'open' AND NEW.finalized_at IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Open Question Attempt has no finalization time';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.validate_question_attempt_issue()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
DECLARE issued ple_private.issued_question%ROWTYPE;
DECLARE snapshot ple_private.assessment_entry_snapshot%ROWTYPE;
DECLARE source_backend text;
BEGIN
    SELECT * INTO issued FROM ple_private.issued_question
     WHERE issued_question_id = NEW.issued_question_id;
    SELECT * INTO snapshot FROM ple_private.assessment_entry_snapshot
     WHERE assessment_entry_snapshot_id = issued.assessment_entry_snapshot_id;
    SELECT backend INTO source_backend
      FROM ple_private.question_revision_source_binding
     WHERE published_question_id = issued.published_question_id
       AND revision_number = issued.revision_number;
    IF NOT FOUND
       OR source_backend IS DISTINCT FROM NEW.backend_name
       OR issued.question_seed IS DISTINCT FROM NEW.question_seed THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Attempt reproduction must match its Issued Question source';
    END IF;
    IF snapshot.question_attempt_time_limit_seconds IS NOT NULL
       AND NEW.deadline_at IS DISTINCT FROM NEW.issued_at
            + pg_catalog.make_interval(secs => snapshot.question_attempt_time_limit_seconds
                + snapshot.question_attempt_grace_seconds) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Attempt deadline must retain its issued per-question limit and grace';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.enforce_question_response_state()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE question_attempt ple_private.question_attempt%ROWTYPE;
BEGIN
    SELECT * INTO question_attempt FROM ple_private.question_attempt WHERE question_attempt_id = NEW.question_attempt_id;
    IF NOT FOUND OR question_attempt.question_attempt_state <> 'response_finalized'
       OR question_attempt.finalized_at IS DISTINCT FROM NEW.finalized_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'finalized Question response requires its exact accepted Question Attempt state';
    END IF;
    -- ASVS 2.3.1: Question evidence cannot skip whole-Assessment finalization
    -- or attach to another Attempt's Assessment submission.
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.issued_question AS issued
        JOIN ple_private.assessment_submission AS assessment_submission
          ON assessment_submission.assessment_attempt_id = issued.assessment_attempt_id
        WHERE issued.issued_question_id = question_attempt.issued_question_id
          AND assessment_submission.assessment_submission_id = NEW.assessment_submission_id
          AND assessment_submission.submitted_at = NEW.finalized_at
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Finalized Question response requires its exact owning Assessment submission';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.reject_student_work_update()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Student Work evidence is immutable'; END $$;

CREATE FUNCTION ple_private.enforce_saved_response_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.question_attempt
         WHERE question_attempt_id = NEW.question_attempt_id
           AND question_attempt_state = 'open'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Saved Student Response changes require an open Question Attempt';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER question_attempt_state_is_forward_only BEFORE UPDATE ON ple_private.question_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_question_attempt_state_transition();

CREATE TRIGGER question_attempt_has_exact_issue BEFORE INSERT OR UPDATE OF issued_question_id, question_seed, issued_at, deadline_at ON ple_private.question_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_attempt_issue();

CREATE TRIGGER question_attempt_delete_is_guarded BEFORE DELETE ON ple_private.question_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

CREATE TRIGGER saved_response_changes_require_open_assessment_attempt BEFORE UPDATE ON ple_private.assessment_attempt_saved_response
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_saved_response_change();

CREATE TRIGGER saved_response_delete_is_guarded BEFORE DELETE ON ple_private.assessment_attempt_saved_response
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

CREATE TRIGGER question_response_is_immutable BEFORE UPDATE ON ple_private.question_response
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_update();

CREATE TRIGGER question_response_delete_is_guarded BEFORE DELETE ON ple_private.question_response
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

CREATE TRIGGER assessment_submission_is_immutable BEFORE UPDATE ON ple_private.assessment_submission
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_update();

CREATE TRIGGER assessment_submission_delete_is_guarded BEFORE DELETE ON ple_private.assessment_submission
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

CREATE CONSTRAINT TRIGGER question_response_matches_assessment_attempt AFTER INSERT OR UPDATE ON ple_private.question_response
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_question_response_state();

