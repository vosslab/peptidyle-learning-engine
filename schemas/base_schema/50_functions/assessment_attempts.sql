-- Functions, triggers, and views from assessment_attempts.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.assert_student_assessment_accommodation_scope()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_data.student_assessment_has_course_scope(NEW.student_record_id, NEW.assessment_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Accommodation requires a Student and Assessment in one Course';
    END IF;
    IF NEW.assessment_attempt_limit IS NOT NULL
       AND NEW.assessment_attempt_limit <> 1
       AND EXISTS (
           SELECT 1 FROM ple_data.assessment AS assessment
            WHERE assessment.assessment_id = NEW.assessment_id
              AND assessment.assessment_type IN ('quiz', 'exam')
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Quiz and Exam accommodations retain exactly one Assessment Attempt';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.enforce_student_assessment_accommodation_edit()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.accommodation_id IS DISTINCT FROM OLD.accommodation_id
       OR NEW.student_record_id IS DISTINCT FROM OLD.student_record_id
       OR NEW.assessment_id IS DISTINCT FROM OLD.assessment_id
       OR NEW.course_instance_id IS DISTINCT FROM OLD.course_instance_id
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR NEW.accommodation_edit_number <> OLD.accommodation_edit_number + 1 THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Accommodation identity is immutable and changes advance one Edit Number';
    END IF;
    RETURN NEW;
END $$;

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.student_assessment_has_course_scope(
    p_student_record_id uuid, p_assessment_id text
) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ple_data.student_record AS student
          JOIN ple_data.assessment AS assessment ON assessment.course_instance_id = student.course_instance_id
         WHERE student.student_record_id = $1 AND assessment.assessment_id = $2
    )
$$;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.assert_assessment_attempt_scope()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_data.student_assessment_has_course_scope(NEW.student_record_id, NEW.assessment_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Assessment Attempt requires a Student and Assessment in one Course';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.reject_student_work_delete()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    -- The executor deletes only Assessment Attempt roots.  PostgreSQL invokes
    -- descendants through foreign-key cascade triggers at a nested depth; no
    -- application role has DELETE on these tables or an application-deleteable
    -- parent in this ownership graph.
    IF current_user <> 'ple_unrelease_executor' AND pg_catalog.pg_trigger_depth() <= 1 THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Student Work deletion requires the Unrelease executor';
    END IF;
    RETURN OLD;
END $$;

CREATE FUNCTION ple_private.reject_assessment_attempt_rewrite()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.assessment_attempt_id IS DISTINCT FROM OLD.assessment_attempt_id
       OR NEW.course_instance_id IS DISTINCT FROM OLD.course_instance_id
       OR NEW.student_record_id IS DISTINCT FROM OLD.student_record_id
       OR NEW.assessment_id IS DISTINCT FROM OLD.assessment_id
       OR NEW.assessment_attempt_number IS DISTINCT FROM OLD.assessment_attempt_number
       OR NEW.started_at IS DISTINCT FROM OLD.started_at
       OR NEW.expires_at IS DISTINCT FROM OLD.expires_at
       OR ROW(NEW.assessment_policy_snapshot_id,
              NEW.schedule_accommodation_id, NEW.schedule_accommodation_edit_number,
              NEW.time_limit_accommodation_id, NEW.time_limit_accommodation_edit_number,
              NEW.assessment_attempt_limit_accommodation_id, NEW.assessment_attempt_limit_accommodation_edit_number)
           IS DISTINCT FROM ROW(OLD.assessment_policy_snapshot_id,
              OLD.schedule_accommodation_id, OLD.schedule_accommodation_edit_number,
              OLD.time_limit_accommodation_id, OLD.time_limit_accommodation_edit_number,
              OLD.assessment_attempt_limit_accommodation_id, OLD.assessment_attempt_limit_accommodation_edit_number) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Assessment Attempt evidence is immutable after creation';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.reject_immutable_student_work_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Student Work evidence is immutable'; END $$;

CREATE FUNCTION ple_private.validate_question_pool_selected_item_member()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_private.question_pool_selection AS selection
          JOIN ple_data.question_pool_member AS member
            ON member.question_pool_id = selection.question_pool_id
         WHERE selection.question_pool_selection_id = NEW.question_pool_selection_id
           AND member.member_position = NEW.member_position
           AND member.published_question_id = NEW.published_question_id
           AND member.question_revision_number = NEW.revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Selected Question Pool member must match current Pool membership';
    END IF;
    RETURN NEW;
END $$;

CREATE FUNCTION ple_private.validate_question_pool_selected_item_count()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE selection_id uuid := CASE WHEN TG_OP = 'DELETE' THEN OLD.question_pool_selection_id ELSE NEW.question_pool_selection_id END;
BEGIN
    IF EXISTS (SELECT 1 FROM ple_private.question_pool_selection AS selection
        WHERE selection.question_pool_selection_id = selection_id
          AND NOT EXISTS (
              SELECT 1 FROM ple_private.question_pool_selected_item AS item
               WHERE item.question_pool_selection_id = selection.question_pool_selection_id
          )) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Pool Selection requires selected Items';
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION ple_private.validate_question_pool_selection_issues()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE selection_id uuid := CASE WHEN TG_OP = 'DELETE' THEN OLD.question_pool_selection_id ELSE NEW.question_pool_selection_id END;
BEGIN
    IF EXISTS (
        (SELECT item.member_position, item.published_question_id, item.revision_number
           FROM ple_private.question_pool_selected_item AS item
          WHERE item.question_pool_selection_id = selection_id
         EXCEPT
         SELECT issued.question_pool_member_position, issued.published_question_id, issued.revision_number
           FROM ple_private.issued_question AS issued
          WHERE issued.question_pool_selection_id = selection_id)
        UNION ALL
        (SELECT issued.question_pool_member_position, issued.published_question_id, issued.revision_number
           FROM ple_private.issued_question AS issued
          WHERE issued.question_pool_selection_id = selection_id
         EXCEPT
         SELECT item.member_position, item.published_question_id, item.revision_number
           FROM ple_private.question_pool_selected_item AS item
          WHERE item.question_pool_selection_id = selection_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Pool Selection and Issued Questions require one exact shared item set';
    END IF;
    RETURN NULL;
END $$;



-- The source binding is immutable, so this durable Student Work fact may be
-- validated at issue time.  A native PLE source has no generator seed;
-- renderer-backed WeBWorK and iMathAS sources require one.  This trusted
-- database boundary enforces the cross-field reproduction rule (ASVS 2.2.3).
CREATE FUNCTION ple_private.validate_issued_question_reproduction()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE source_backend ple_data.question_backend;
BEGIN
    SELECT backend INTO source_backend
      FROM ple_private.question_revision_source_binding
     WHERE published_question_id = NEW.published_question_id
       AND revision_number = NEW.revision_number;
    IF NOT FOUND OR NOT ple_private.question_backend_is_supported_for_production(source_backend)
       OR (source_backend = 'ple' AND NEW.question_seed IS NOT NULL)
       OR (source_backend = 'webwork' AND NEW.question_seed IS NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Issued Question reproduction must match its source backend';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER student_assessment_accommodation_has_course_scope BEFORE INSERT OR UPDATE
ON ple_private.student_assessment_accommodation FOR EACH ROW EXECUTE FUNCTION ple_private.assert_student_assessment_accommodation_scope();

CREATE TRIGGER assessment_attempt_has_course_scope BEFORE INSERT ON ple_private.assessment_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.assert_assessment_attempt_scope();

CREATE TRIGGER assessment_attempt_retains_evidence BEFORE UPDATE ON ple_private.assessment_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_assessment_attempt_rewrite();

CREATE TRIGGER assessment_attempt_delete_is_guarded BEFORE DELETE ON ple_private.assessment_attempt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

CREATE TRIGGER student_assessment_accommodation_edit_is_guarded BEFORE UPDATE
ON ple_private.student_assessment_accommodation
FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_student_assessment_accommodation_edit();

CREATE TRIGGER question_pool_selection_is_immutable BEFORE UPDATE ON ple_private.question_pool_selection
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_student_work_change();

CREATE TRIGGER question_pool_selection_delete_is_guarded BEFORE DELETE ON ple_private.question_pool_selection
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

CREATE TRIGGER question_pool_selected_item_is_immutable BEFORE UPDATE ON ple_private.question_pool_selected_item
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_student_work_change();

CREATE TRIGGER question_pool_selected_item_delete_is_guarded BEFORE DELETE ON ple_private.question_pool_selected_item
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

CREATE TRIGGER question_pool_selected_item_matches_current_pool_membership BEFORE INSERT
ON ple_private.question_pool_selected_item FOR EACH ROW
EXECUTE FUNCTION ple_private.validate_question_pool_selected_item_member();

CREATE TRIGGER issued_question_is_immutable BEFORE UPDATE ON ple_private.issued_question
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_student_work_change();

CREATE TRIGGER issued_question_reproduction_matches_source BEFORE INSERT OR UPDATE OF published_question_id, revision_number, question_seed
ON ple_private.issued_question FOR EACH ROW EXECUTE FUNCTION ple_private.validate_issued_question_reproduction();

CREATE TRIGGER issued_question_delete_is_guarded BEFORE DELETE ON ple_private.issued_question
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

CREATE CONSTRAINT TRIGGER question_pool_selection_has_exact_item_count AFTER INSERT ON ple_private.question_pool_selection
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selected_item_count();

CREATE CONSTRAINT TRIGGER question_pool_selected_item_count_is_exact AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_pool_selected_item
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selected_item_count();

CREATE CONSTRAINT TRIGGER question_pool_selection_issues_are_exact AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_pool_selected_item
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selection_issues();

CREATE CONSTRAINT TRIGGER issued_question_pool_source_is_exact AFTER INSERT OR UPDATE OR DELETE ON ple_private.issued_question
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_pool_selection_issues();

