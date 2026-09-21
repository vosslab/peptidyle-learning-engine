-- Functions, triggers, and views from question_lineages.sql.

SET LOCAL ROLE ple_private_owner;



-- Current admission policy is the production Question Backend vocabulary.
CREATE FUNCTION ple_private.question_backend_is_supported_for_production(
    p_backend ple_data.question_backend
)
RETURNS boolean LANGUAGE sql IMMUTABLE SET search_path = pg_catalog AS $$
    SELECT COALESCE(p_backend IN ('ple', 'webwork'), false)
$$;

SET LOCAL ROLE ple_data_owner;

CREATE TRIGGER published_question_public_id_is_reserved
BEFORE INSERT ON ple_data.published_question
FOR EACH ROW EXECUTE FUNCTION ple_private.reserve_public_id_from_trigger(
    'published_question', 'published_question_id'
);

CREATE FUNCTION ple_data.reject_question_lineage_immutable_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question Revision and Question event records are immutable';
END
$$;

CREATE FUNCTION ple_data.validate_question_availability_event()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE
    current_availability text;
    v_current_availability_edit_number bigint;
    has_prior_event boolean;
BEGIN
    SELECT availability, availability_edit_number
      INTO current_availability, v_current_availability_edit_number
      FROM ple_data.published_question
     WHERE published_question_id = NEW.published_question_id
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Published Question Availability Edit Number is stale';
    END IF;
    SELECT EXISTS (
        SELECT 1 FROM ple_data.question_availability_event
         WHERE published_question_id = NEW.published_question_id
    ) INTO has_prior_event;
    IF NOT has_prior_event THEN
        IF NEW.availability::text <> 'available' OR NEW.edit_number <> 1 THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'Question lineage publication records Available at Edit Number 1';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.edit_number <> v_current_availability_edit_number + 1 THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Published Question Availability Edit Number is stale';
    END IF;
    IF NEW.availability::text = current_availability THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Published Question Availability transition must change state';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_revision_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_lineage_immutable_change();

CREATE TRIGGER question_publication_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_publication_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_lineage_immutable_change();

CREATE TRIGGER question_availability_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_availability_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_lineage_immutable_change();

CREATE TRIGGER question_availability_event_is_current
BEFORE INSERT ON ple_data.question_availability_event
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_availability_event();

CREATE FUNCTION ple_data.set_question_availability(
    p_published_question_id text,
    p_expected_question_availability_edit_number bigint,
    p_target_availability text,
    p_archive_confirmation_title text,
    p_event_id uuid
) RETURNS TABLE (availability text, availability_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    current_question ple_data.published_question%ROWTYPE;
    current_title text;
    actor_id text;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Instructor authority is required';
    END IF;
    IF p_target_availability NOT IN ('available', 'archived')
       OR p_expected_question_availability_edit_number IS NULL OR p_expected_question_availability_edit_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'availability transition input is invalid';
    END IF;
    SELECT * INTO current_question FROM ple_data.published_question
     WHERE published_question_id = p_published_question_id FOR UPDATE;
    IF NOT FOUND OR NOT EXISTS (SELECT 1 FROM ple_data.question_current_owner
        WHERE published_question_id = p_published_question_id AND owner_account_id = actor_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Owner authority is required';
    END IF;
    IF current_question.availability_edit_number <> p_expected_question_availability_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Published Question Availability Edit Number is stale';
    END IF;
    SELECT question_title INTO current_title FROM ple_data.published_question_metadata
     WHERE published_question_id = p_published_question_id;
    IF p_target_availability = 'archived'
       AND p_archive_confirmation_title IS DISTINCT FROM current_title THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Archive Published Question requires exact title confirmation';
    END IF;
    IF (p_target_availability = 'archived' AND current_question.availability <> 'available')
       OR (p_target_availability = 'available' AND current_question.availability <> 'archived') THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Question availability transition conflicts with its current state';
    END IF;
    INSERT INTO ple_data.question_availability_event(
        event_id, published_question_id, actor_account_id, availability, edit_number, reason, occurred_at
    ) VALUES (
        p_event_id, p_published_question_id, actor_id, p_target_availability::ple_data.question_availability,
        p_expected_question_availability_edit_number + 1,
        CASE WHEN p_target_availability = 'archived' THEN 'archived by Question Owner' END,
        pg_catalog.clock_timestamp()
    );
    UPDATE ple_data.published_question
       SET availability = p_target_availability::ple_data.question_availability,
           availability_edit_number = p_expected_question_availability_edit_number + 1
     WHERE published_question_id = p_published_question_id
     RETURNING published_question.availability, published_question.availability_edit_number
       INTO availability, availability_edit_number;
    availability := availability::text;
    RETURN NEXT;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.set_question_availability(
    p_published_question_id text, p_expected_question_availability_edit_number bigint, p_target_availability text,
    p_archive_confirmation_title text, p_event_id uuid
) RETURNS TABLE (availability text, availability_edit_number bigint)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_api AS $$
    SELECT * FROM ple_data.set_question_availability(
        p_published_question_id, p_expected_question_availability_edit_number, p_target_availability,
        p_archive_confirmation_title, p_event_id)
$$;
