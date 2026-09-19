-- Functions, triggers, and views from assessment_attempt_interaction.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.projected_question_attempt_state(
    p_finalized_at timestamptz,
    p_has_saved_response boolean
) RETURNS text
LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog AS $$
    SELECT CASE
               WHEN p_finalized_at IS NULL THEN 'open'
               WHEN p_has_saved_response THEN 'response_finalized'
               ELSE 'closed_unanswered'
           END
$$;

CREATE FUNCTION ple_private.projected_finalization_kind(
    p_authorized_by_account_id text
) RETURNS text
LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog AS $$
    SELECT CASE WHEN p_authorized_by_account_id IS NOT NULL THEN 'student' ELSE 'deadline' END
$$;

CREATE FUNCTION ple_private.ensure_delivery_toolchain(
    p_backend_name ple_data.question_backend,
    p_backend_version text,
    p_renderer_name text,
    p_renderer_version text,
    p_grader_name text,
    p_grader_version text,
    p_issued_capability ple_data.issued_capability
) RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE delivery_toolchain_id_value uuid;
BEGIN
    IF p_backend_name IS NULL
       OR p_backend_version IS NULL OR char_length(btrim(p_backend_version)) NOT BETWEEN 1 AND 100
       OR (p_renderer_name IS NULL) <> (p_renderer_version IS NULL)
       OR (p_renderer_name IS NOT NULL
           AND char_length(btrim(p_renderer_name)) NOT BETWEEN 1 AND 100)
       OR (p_renderer_version IS NOT NULL
           AND char_length(btrim(p_renderer_version)) NOT BETWEEN 1 AND 100)
       OR p_grader_name IS NULL OR char_length(btrim(p_grader_name)) NOT BETWEEN 1 AND 100
       OR p_grader_version IS NULL OR char_length(btrim(p_grader_version)) NOT BETWEEN 1 AND 100
       OR p_issued_capability IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Delivery toolchain facts are invalid';
    END IF;
    INSERT INTO ple_private.delivery_toolchain (
        delivery_toolchain_id, backend_name, backend_version, renderer_name, renderer_version,
        grader_name, grader_version, issued_capability, created_at
    ) VALUES (
        pg_catalog.gen_random_uuid(), p_backend_name, p_backend_version,
        p_renderer_name, p_renderer_version, p_grader_name, p_grader_version,
        p_issued_capability, pg_catalog.clock_timestamp()
    )
    ON CONFLICT ON CONSTRAINT delivery_toolchain_values_key
    DO UPDATE SET delivery_toolchain_id = ple_private.delivery_toolchain.delivery_toolchain_id
    RETURNING delivery_toolchain_id INTO delivery_toolchain_id_value;
    RETURN delivery_toolchain_id_value;
END $$;

CREATE FUNCTION ple_private.enforce_question_attempt_finalization()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.question_attempt_id IS DISTINCT FROM OLD.question_attempt_id
       OR NEW.issued_question_id IS DISTINCT FROM OLD.issued_question_id
       OR NEW.question_seed IS DISTINCT FROM OLD.question_seed
       OR NEW.generated_parameter_sha256 IS DISTINCT FROM OLD.generated_parameter_sha256
       OR NEW.issued_at IS DISTINCT FROM OLD.issued_at
       OR NEW.deadline_at IS DISTINCT FROM OLD.deadline_at
       OR NEW.delivery_toolchain_id IS DISTINCT FROM OLD.delivery_toolchain_id
       OR NEW.source_object_record_id IS DISTINCT FROM OLD.source_object_record_id
       OR NEW.source_object_checksum IS DISTINCT FROM OLD.source_object_checksum
       OR NEW.rendered_question_sha256 IS DISTINCT FROM OLD.rendered_question_sha256
       OR NEW.course_instance_id IS DISTINCT FROM OLD.course_instance_id THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Question Attempt reproduction evidence is immutable';
    END IF;
    IF OLD.finalized_at IS NOT NULL AND NEW.finalized_at IS DISTINCT FROM OLD.finalized_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Attempt finalization is forward-only';
    END IF;
    IF OLD.finalized_at IS NULL AND NEW.finalized_at IS NOT NULL
       AND NEW.finalized_at < NEW.issued_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Attempt finalization time is invalid';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.validate_question_attempt_issue()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private, ple_data AS $$
DECLARE issued ple_private.issued_question%ROWTYPE;
DECLARE snapshot ple_private.assessment_entry_snapshot%ROWTYPE;
DECLARE source_backend ple_data.question_backend;
DECLARE toolchain_backend ple_data.question_backend;
BEGIN
    SELECT * INTO issued FROM ple_private.issued_question
     WHERE course_instance_id = NEW.course_instance_id
       AND issued_question_id = NEW.issued_question_id;
    SELECT * INTO snapshot FROM ple_private.assessment_entry_snapshot
     WHERE assessment_entry_snapshot_id = issued.assessment_entry_snapshot_id;
    SELECT backend INTO source_backend
      FROM ple_private.question_revision_source_binding
     WHERE published_question_id = issued.published_question_id
       AND revision_number = issued.revision_number;
    SELECT backend_name INTO toolchain_backend
      FROM ple_private.delivery_toolchain
     WHERE delivery_toolchain_id = NEW.delivery_toolchain_id;
    IF NOT FOUND
       OR source_backend IS DISTINCT FROM toolchain_backend
       OR issued.question_seed IS DISTINCT FROM NEW.question_seed THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Attempt reproduction must match its Issued Question source';
    END IF;
    IF (toolchain_backend = 'ple' AND NEW.question_seed IS NOT NULL)
       OR (toolchain_backend = 'webwork' AND NEW.question_seed IS NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Attempt seed must match its delivery toolchain';
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

CREATE FUNCTION ple_private.reject_student_work_update()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Student Work evidence is immutable'; END $$;

CREATE FUNCTION ple_private.enforce_saved_response_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.finalized_at IS NOT NULL OR NEW.assessment_submission_id IS NOT NULL THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'Saved Student Response starts unfinalized';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM ple_private.question_attempt
             WHERE course_instance_id = NEW.course_instance_id
               AND question_attempt_id = NEW.question_attempt_id
               AND finalized_at IS NULL
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'Saved Student Response changes require an open Question Attempt';
        END IF;
        RETURN NEW;
    END IF;
    IF OLD.finalized_at IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Finalized Student Response is immutable';
    END IF;
    IF NEW.finalized_at IS NOT NULL THEN
        IF NEW.student_response IS DISTINCT FROM OLD.student_response
           OR NEW.saved_at IS DISTINCT FROM OLD.saved_at
           OR NEW.assessment_submission_id IS NULL THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'Saved Student Response finalization is invalid';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.assessment_submission_id IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Saved Student Response finalization is invalid';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.question_attempt
         WHERE course_instance_id = NEW.course_instance_id
           AND question_attempt_id = NEW.question_attempt_id
           AND finalized_at IS NULL
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Saved Student Response changes require an open Question Attempt';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER question_attempt_finalization_is_forward_only BEFORE UPDATE ON ple_private.question_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_question_attempt_finalization();

CREATE TRIGGER question_attempt_has_exact_issue BEFORE INSERT OR UPDATE OF issued_question_id, question_seed, issued_at, deadline_at, delivery_toolchain_id ON ple_private.question_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_attempt_issue();

CREATE TRIGGER question_attempt_delete_is_guarded BEFORE DELETE ON ple_private.question_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

CREATE TRIGGER saved_response_insert_requires_open_assessment_attempt BEFORE INSERT ON ple_private.assessment_attempt_saved_response
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_saved_response_change();

CREATE TRIGGER saved_response_changes_require_open_assessment_attempt BEFORE UPDATE ON ple_private.assessment_attempt_saved_response
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_saved_response_change();

CREATE TRIGGER saved_response_delete_is_guarded BEFORE DELETE ON ple_private.assessment_attempt_saved_response
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

CREATE TRIGGER assessment_submission_is_immutable BEFORE UPDATE ON ple_private.assessment_submission
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_update();

CREATE TRIGGER assessment_submission_delete_is_guarded BEFORE DELETE ON ple_private.assessment_submission
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
