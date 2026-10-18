-- Functions, triggers, and views from blueprint_operations.sql.

SET LOCAL ROLE ple_api_owner;






-- This private primitive has one narrowly trusted exception for copying a
-- locked source Course's exact retired Discipline. The public wrapper below
-- never supplies that value, so ordinary new Blueprints remain active-only.
CREATE FUNCTION ple_private.create_blueprint_course(
    p_blueprint_course_id text, p_request_checksum bytea, p_short_name text, p_long_name text, p_theme text,
    p_content jsonb, p_content_checksum bytea,
    p_discipline uuid, p_subject uuid, p_topic uuid, p_subtopic uuid, p_tags text[],
    p_retired_source_discipline uuid
)
RETURNS TABLE (
    blueprint_course_id text, blueprint_revision_number bigint, blueprint_edit_number bigint,
    accepted_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_actor text;
    v_now timestamp with time zone;
    v_blueprint_edit_number bigint;
    v_blueprint_course_id text;
BEGIN
    IF octet_length(p_request_checksum) <> 32
       OR p_short_name IS NULL OR p_short_name <> btrim(p_short_name)
       OR char_length(p_short_name) NOT BETWEEN 1 AND 500
       OR p_long_name IS NULL OR p_long_name <> btrim(p_long_name)
       OR char_length(p_long_name) NOT BETWEEN 1 AND 500
       OR NOT EXISTS (SELECT 1 FROM ple_data.theme WHERE theme_id = p_theme)
       OR octet_length(p_content_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course creation is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('ple:blueprint-course-create:%s:%s', v_actor,
            pg_catalog.encode(p_request_checksum, 'hex')), 0));
    SELECT course.blueprint_course_id, receipt.blueprint_revision_number,
           receipt.blueprint_edit_number, receipt.accepted_at
      INTO blueprint_course_id, blueprint_revision_number, blueprint_edit_number, accepted_at
      FROM ple_data.blueprint_course_create_receipt AS receipt
      JOIN ple_data.blueprint_course AS course
        ON course.blueprint_course_id = receipt.blueprint_course_id
     WHERE receipt.actor_account_id = v_actor AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN RETURN NEXT; RETURN; END IF;
    PERFORM ple_data.validate_blueprint_content(p_content);
    PERFORM ple_data.validate_blueprint_question_selection(NULL, NULL, p_content);
    v_now := pg_catalog.clock_timestamp();
    v_blueprint_edit_number := 1;
    IF p_retired_source_discipline IS NULL
       OR p_discipline IS DISTINCT FROM p_retired_source_discipline
       OR NOT EXISTS (
           SELECT 1 FROM ple_api.get_content_discipline(p_retired_source_discipline) AS discipline
            WHERE discipline.content_discipline_id = p_retired_source_discipline
              AND discipline.is_retired
       ) THEN
        PERFORM ple_api.require_active_content_discipline(p_discipline);
    END IF;
    INSERT INTO ple_data.blueprint_course AS course (
        blueprint_course_id, owner_account_id, short_name, long_name, blueprint_edit_number, created_at,
        content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, tags, theme_id
    ) VALUES (
        COALESCE(p_blueprint_course_id, 'BP0000000C'), v_actor, p_short_name, p_long_name, v_blueprint_edit_number, v_now,
        p_discipline, p_subject, p_topic, p_subtopic, p_tags, p_theme
    ) RETURNING course.blueprint_course_id INTO v_blueprint_course_id;
    blueprint_revision_number := 1;
    INSERT INTO ple_data.blueprint_course_revision (
        blueprint_course_id, blueprint_revision_number,
        content, content_checksum, saved_at
    ) VALUES (
        v_blueprint_course_id, blueprint_revision_number, p_content, p_content_checksum, v_now
    );
    INSERT INTO ple_data.blueprint_revision_question_pin
    SELECT v_blueprint_course_id, blueprint_revision_number, pins.content_path,
           pins.published_question_id, pins.question_revision_number
      FROM ple_data.blueprint_content_question_pins(p_content) AS pins;
    INSERT INTO ple_data.blueprint_revision_module
    SELECT v_blueprint_course_id, blueprint_revision_number,
           modules.blueprint_module_id, modules.module_position
      FROM ple_data.blueprint_content_modules(p_content) AS modules;
    INSERT INTO ple_data.blueprint_revision_assessment
    SELECT v_blueprint_course_id, blueprint_revision_number,
           assessments.blueprint_module_id,
           assessments.blueprint_assessment_id, assessments.assessment_position
      FROM ple_data.blueprint_content_assessments(p_content) AS assessments;
    INSERT INTO ple_data.blueprint_revision_event (
        blueprint_course_id, blueprint_revision_number, actor_account_id,
        request_checksum, occurred_at
    ) VALUES (
        v_blueprint_course_id, blueprint_revision_number, v_actor, p_request_checksum, v_now
    );
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_id, actor_account_id, short_name, long_name,
        availability, blueprint_edit_number, occurred_at,
        content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, tags, theme_id
    ) VALUES (
        v_blueprint_course_id, v_actor, p_short_name, p_long_name,
        'public', v_blueprint_edit_number, v_now,
        p_discipline, p_subject, p_topic, p_subtopic, p_tags, p_theme
    );
    INSERT INTO ple_data.blueprint_course_create_receipt
    VALUES (
        v_actor, p_request_checksum, v_blueprint_course_id, blueprint_revision_number,
        v_blueprint_edit_number, v_now
    );
    SELECT course.blueprint_course_id INTO blueprint_course_id
      FROM ple_data.blueprint_course AS course
     WHERE course.blueprint_course_id = v_blueprint_course_id;
    blueprint_edit_number := v_blueprint_edit_number; accepted_at := v_now;
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.create_blueprint_course(
    p_blueprint_course_id text, p_request_checksum bytea, p_short_name text, p_long_name text, p_theme text,
    p_content jsonb, p_content_checksum bytea,
    p_discipline uuid, p_subject uuid, p_topic uuid, p_subtopic uuid, p_tags text[]
)
RETURNS TABLE (
    blueprint_course_id text, blueprint_revision_number bigint, blueprint_edit_number bigint,
    accepted_at timestamp with time zone
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT * FROM ple_private.create_blueprint_course(
        p_blueprint_course_id, p_request_checksum, p_short_name, p_long_name, p_theme,
        p_content, p_content_checksum, p_discipline, p_subject, p_topic,
        p_subtopic, p_tags, NULL
    )
$$;

CREATE FUNCTION ple_api.save_blueprint_course(
    p_blueprint_course_id text, p_expected_blueprint_revision_number bigint,
    p_request_checksum bytea, p_content jsonb, p_content_checksum bytea
)
RETURNS TABLE (
    resulting_blueprint_revision_number bigint, changed boolean,
    accepted_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_actor text;
    v_course ple_data.blueprint_course%ROWTYPE;
    v_prior ple_data.blueprint_course_revision%ROWTYPE;
    v_now timestamp with time zone;
    v_blueprint_course_id text;
BEGIN
    IF NOT ple_private.is_canonical_prefixed_public_id(p_blueprint_course_id, 'BP')
       OR p_expected_blueprint_revision_number <= 0
       OR octet_length(p_request_checksum) <> 32
       OR octet_length(p_content_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Course Save is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    SELECT course.blueprint_course_id INTO v_blueprint_course_id
      FROM ple_data.blueprint_course AS course
     WHERE course.blueprint_course_id = p_blueprint_course_id;
    SELECT * INTO v_course
      FROM ple_data.blueprint_course
     WHERE blueprint_course_id = v_blueprint_course_id AND owner_account_id = v_actor
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is not available';
    END IF;
    -- ASVS 8.3.1 / 2.3.1: enforce the lifecycle under the owner lock,
    -- including retries and no-op Saves; restore to Public before writing.
    IF v_course.availability = 'archived' THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Archived Blueprint Course is read-only';
    END IF;
    SELECT receipt.resulting_blueprint_revision_number, receipt.changed, receipt.accepted_at
      INTO resulting_blueprint_revision_number, changed, accepted_at
      FROM ple_data.blueprint_course_save_receipt AS receipt
     WHERE receipt.blueprint_course_id = v_blueprint_course_id
       AND receipt.actor_account_id = v_actor
       AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN RETURN NEXT; RETURN; END IF;
    IF v_course.current_blueprint_revision_number
       <> p_expected_blueprint_revision_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Blueprint Revision precondition is stale';
    END IF;
    PERFORM ple_data.validate_blueprint_content(p_content);
    PERFORM ple_data.validate_blueprint_question_selection(
        v_blueprint_course_id, p_expected_blueprint_revision_number, p_content
    );
    SELECT * INTO STRICT v_prior
      FROM ple_data.blueprint_course_revision
     WHERE blueprint_course_id = v_blueprint_course_id
       AND blueprint_revision_number = p_expected_blueprint_revision_number;
    v_now := pg_catalog.clock_timestamp();
    changed := v_prior.content IS DISTINCT FROM p_content
        OR v_prior.content_checksum IS DISTINCT FROM p_content_checksum;
    IF changed THEN
        resulting_blueprint_revision_number := p_expected_blueprint_revision_number + 1;
        INSERT INTO ple_data.blueprint_course_revision (
            blueprint_course_id, blueprint_revision_number,
            content, content_checksum, saved_at
        ) VALUES (
            v_blueprint_course_id, resulting_blueprint_revision_number,
            p_content, p_content_checksum, v_now
        );
        INSERT INTO ple_data.blueprint_revision_question_pin
        SELECT v_blueprint_course_id, resulting_blueprint_revision_number, pins.content_path,
               pins.published_question_id, pins.question_revision_number
          FROM ple_data.blueprint_content_question_pins(p_content) AS pins;
        INSERT INTO ple_data.blueprint_revision_module
        SELECT v_blueprint_course_id, resulting_blueprint_revision_number,
               modules.blueprint_module_id, modules.module_position
          FROM ple_data.blueprint_content_modules(p_content) AS modules;
        INSERT INTO ple_data.blueprint_revision_assessment
        SELECT v_blueprint_course_id, resulting_blueprint_revision_number,
               assessments.blueprint_module_id,
               assessments.blueprint_assessment_id, assessments.assessment_position
          FROM ple_data.blueprint_content_assessments(p_content) AS assessments;
        INSERT INTO ple_data.blueprint_revision_event (
            blueprint_course_id, blueprint_revision_number, actor_account_id,
            request_checksum, occurred_at
        ) VALUES (
            v_blueprint_course_id, resulting_blueprint_revision_number,
            v_actor, p_request_checksum, v_now
        );
        UPDATE ple_data.blueprint_course
           SET current_blueprint_revision_number = resulting_blueprint_revision_number
         WHERE blueprint_course_id = v_blueprint_course_id;
    ELSE
        resulting_blueprint_revision_number := p_expected_blueprint_revision_number;
    END IF;
    INSERT INTO ple_data.blueprint_course_save_receipt
    VALUES (
        v_blueprint_course_id, v_actor, p_request_checksum,
        resulting_blueprint_revision_number, changed, v_now
    );
    accepted_at := v_now; RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.rename_blueprint_course(
    p_blueprint_course_id text, p_expected_blueprint_edit_number bigint,
    p_short_name text, p_long_name text
)
RETURNS TABLE (
    short_name text, long_name text, availability text, blueprint_edit_number bigint,
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid, tags text[], theme_id text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_actor text;
    v_course ple_data.blueprint_course%ROWTYPE;
    v_next_blueprint_edit_number bigint;
    v_blueprint_course_id text;
BEGIN
    IF NOT ple_private.is_canonical_prefixed_public_id(p_blueprint_course_id, 'BP')
       OR p_expected_blueprint_edit_number IS NULL
       OR p_short_name IS NULL OR p_short_name <> btrim(p_short_name)
       OR char_length(p_short_name) NOT BETWEEN 1 AND 500
       OR p_long_name IS NULL OR p_long_name <> btrim(p_long_name)
       OR char_length(p_long_name) NOT BETWEEN 1 AND 500
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course rename is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    SELECT course.blueprint_course_id INTO v_blueprint_course_id
      FROM ple_data.blueprint_course AS course
     WHERE course.blueprint_course_id = p_blueprint_course_id;
    SELECT * INTO v_course FROM ple_data.blueprint_course
     WHERE blueprint_course_id = v_blueprint_course_id AND owner_account_id = v_actor FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is not available';
    END IF;
    -- ASVS 8.3.1 / 2.3.1: lifecycle denial precedes CAS and no-op rename.
    IF v_course.availability = 'archived' THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Archived Blueprint Course is read-only';
    END IF;
    IF v_course.blueprint_edit_number <> p_expected_blueprint_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint metadata Edit Number is stale';
    END IF;
    IF v_course.short_name = p_short_name AND v_course.long_name = p_long_name THEN
        RETURN QUERY SELECT v_course.short_name, v_course.long_name,
            v_course.availability::text, v_course.blueprint_edit_number,
            v_course.content_discipline_id, v_course.content_subject_id, v_course.content_topic_id,
            v_course.content_subtopic_id, v_course.tags, v_course.theme_id;
        RETURN;
    END IF;
    v_next_blueprint_edit_number := v_course.blueprint_edit_number + 1;
    UPDATE ple_data.blueprint_course AS course
       SET short_name = p_short_name, long_name = p_long_name, blueprint_edit_number = v_next_blueprint_edit_number
     WHERE course.blueprint_course_id = v_blueprint_course_id;
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_id, actor_account_id, short_name, long_name,
        availability, blueprint_edit_number, occurred_at,
        content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, tags, theme_id
    ) VALUES (
        v_blueprint_course_id, v_actor, p_short_name, p_long_name,
        v_course.availability, v_next_blueprint_edit_number, pg_catalog.clock_timestamp(),
        v_course.content_discipline_id, v_course.content_subject_id, v_course.content_topic_id,
        v_course.content_subtopic_id, v_course.tags, v_course.theme_id
    );
    RETURN QUERY SELECT p_short_name, p_long_name, v_course.availability::text, v_next_blueprint_edit_number,
        v_course.content_discipline_id, v_course.content_subject_id, v_course.content_topic_id,
        v_course.content_subtopic_id, v_course.tags, v_course.theme_id;
END
$$;

CREATE FUNCTION ple_api.set_blueprint_availability(
    p_blueprint_course_id text, p_expected_blueprint_edit_number bigint, p_availability text,
    p_archive_confirmation_long_name text
)
RETURNS TABLE (
    short_name text, long_name text, availability text, blueprint_edit_number bigint,
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid, tags text[], theme_id text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_actor text;
    v_course ple_data.blueprint_course%ROWTYPE;
    v_next_blueprint_edit_number bigint;
    v_blueprint_course_id text;
    v_availability ple_data.blueprint_availability;
BEGIN
    IF NOT ple_private.is_canonical_prefixed_public_id(p_blueprint_course_id, 'BP')
       OR p_expected_blueprint_edit_number IS NULL
       OR p_availability NOT IN ('public', 'archived')
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint availability change is invalid';
    END IF;
    v_availability := p_availability::ple_data.blueprint_availability;
    v_actor := ple_api.current_session_account_id();
    SELECT course.blueprint_course_id INTO v_blueprint_course_id
      FROM ple_data.blueprint_course AS course
     WHERE course.blueprint_course_id = p_blueprint_course_id;
    SELECT * INTO v_course FROM ple_data.blueprint_course
     WHERE blueprint_course_id = v_blueprint_course_id AND owner_account_id = v_actor FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is not available';
    END IF;
    IF v_course.blueprint_edit_number <> p_expected_blueprint_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint metadata Edit Number is stale';
    END IF;
    IF (v_course.availability = 'public' AND p_availability <> 'archived')
       OR (v_course.availability = 'archived' AND p_availability <> 'public') THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Blueprint lifecycle transition is not permitted';
    END IF;
    IF p_availability = 'archived'
       AND p_archive_confirmation_long_name IS DISTINCT FROM v_course.long_name THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Archive Blueprint requires exact long name confirmation';
    END IF;
    IF v_course.availability = v_availability THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Blueprint availability already has that state';
    END IF;
    v_next_blueprint_edit_number := v_course.blueprint_edit_number + 1;
    UPDATE ple_data.blueprint_course AS course
       SET availability = v_availability, blueprint_edit_number = v_next_blueprint_edit_number
     WHERE course.blueprint_course_id = v_blueprint_course_id;
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_id, actor_account_id, short_name, long_name,
        availability, blueprint_edit_number, occurred_at,
        content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, tags, theme_id
    ) VALUES (
        v_blueprint_course_id, v_actor, v_course.short_name, v_course.long_name,
        v_availability, v_next_blueprint_edit_number, pg_catalog.clock_timestamp(),
        v_course.content_discipline_id, v_course.content_subject_id, v_course.content_topic_id,
        v_course.content_subtopic_id, v_course.tags, v_course.theme_id
    );
    RETURN QUERY SELECT v_course.short_name, v_course.long_name, p_availability, v_next_blueprint_edit_number,
        v_course.content_discipline_id, v_course.content_subject_id, v_course.content_topic_id,
        v_course.content_subtopic_id, v_course.tags, v_course.theme_id;
END
$$;

CREATE FUNCTION ple_api.update_blueprint_classification(
    p_blueprint_course_id text, p_expected_blueprint_edit_number bigint,
    p_discipline uuid, p_subject uuid, p_topic uuid, p_subtopic uuid, p_tags text[]
)
RETURNS TABLE(blueprint_edit_number bigint, changed boolean)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_course ple_data.blueprint_course%ROWTYPE;
    v_next_blueprint_edit_number bigint;
BEGIN
    -- ASVS 8.2.1/8.2.2: ownership, not ambient User Role, permits mutation.
    SELECT course.* INTO v_course FROM ple_data.blueprint_course AS course
     WHERE course.blueprint_course_id = p_blueprint_course_id
       AND course.owner_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_is_instructor()
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Blueprint Course is not available' USING ERRCODE = '42501';
    END IF;
    IF v_course.availability = 'archived' THEN
        RAISE EXCEPTION 'Archived Blueprint Course is read-only' USING ERRCODE = '55000';
    END IF;
    -- ASVS 2.3.3: row lock and validator reject concurrent stale metadata writes.
    IF p_expected_blueprint_edit_number IS DISTINCT FROM v_course.blueprint_edit_number THEN
        RAISE EXCEPTION 'Blueprint metadata Edit Number is stale' USING ERRCODE = '40001';
    END IF;
    IF ROW(v_course.content_discipline_id, v_course.content_subject_id, v_course.content_topic_id,
           v_course.content_subtopic_id, v_course.tags)
       IS NOT DISTINCT FROM ROW(p_discipline, p_subject, p_topic, p_subtopic, p_tags) THEN
        RETURN QUERY SELECT v_course.blueprint_edit_number, false;
        RETURN;
    END IF;
    v_next_blueprint_edit_number := v_course.blueprint_edit_number + 1;
    -- Retaining an existing retired Discipline does not make it a new choice.
    -- A true replacement requires an active Discipline and retains its row
    -- lock through this update transaction.
    IF v_course.content_discipline_id IS DISTINCT FROM p_discipline THEN
        PERFORM ple_api.require_active_content_discipline(p_discipline);
    END IF;
    UPDATE ple_data.blueprint_course AS course SET
        content_discipline_id = p_discipline, content_subject_id = p_subject, content_topic_id = p_topic,
        content_subtopic_id = p_subtopic, tags = p_tags, blueprint_edit_number = v_next_blueprint_edit_number
     WHERE course.blueprint_course_id = v_course.blueprint_course_id;
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_id, actor_account_id, short_name, long_name,
        availability, blueprint_edit_number, occurred_at,
        content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, tags, theme_id
    ) VALUES (
        v_course.blueprint_course_id, ple_api.current_session_account_id(),
        v_course.short_name, v_course.long_name, v_course.availability,
        v_next_blueprint_edit_number, pg_catalog.clock_timestamp(), p_discipline, p_subject, p_topic, p_subtopic, p_tags,
        v_course.theme_id
    );
    RETURN QUERY SELECT v_next_blueprint_edit_number, true;
END
$$;

CREATE FUNCTION ple_api.update_blueprint_theme(
    p_blueprint_course_id text, p_expected_blueprint_edit_number bigint, p_theme_id text
)
RETURNS TABLE (
    short_name text, long_name text, availability text, blueprint_edit_number bigint,
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid,
    content_subtopic_id uuid, tags text[], theme_id text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_course ple_data.blueprint_course%ROWTYPE;
    v_next_blueprint_edit_number bigint;
BEGIN
    IF NOT ple_private.is_canonical_prefixed_public_id(p_blueprint_course_id, 'BP')
       OR p_expected_blueprint_edit_number IS NULL
       OR NOT EXISTS (SELECT 1 FROM ple_data.theme WHERE theme_id = p_theme_id)
       OR NOT (ple_api.current_session_account_is_sysadmin()
               OR ple_api.current_session_account_is_instructor()) THEN
        RAISE EXCEPTION 'Blueprint Theme update is invalid' USING ERRCODE = '22023';
    END IF;
    SELECT course.* INTO v_course FROM ple_data.blueprint_course AS course
     WHERE course.blueprint_course_id = p_blueprint_course_id
       AND (ple_api.current_session_account_is_sysadmin()
            OR course.owner_account_id = ple_api.current_session_account_id())
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Blueprint Course is not available' USING ERRCODE = '42501';
    END IF;
    IF v_course.availability = 'archived'
       AND NOT ple_api.current_session_account_is_sysadmin() THEN
        RAISE EXCEPTION 'Archived Blueprint Course is read-only' USING ERRCODE = '55000';
    END IF;
    IF p_expected_blueprint_edit_number IS DISTINCT FROM v_course.blueprint_edit_number THEN
        RAISE EXCEPTION 'Blueprint metadata Edit Number is stale' USING ERRCODE = '40001';
    END IF;
    IF v_course.theme_id = p_theme_id THEN
        RETURN QUERY SELECT v_course.short_name, v_course.long_name,
            v_course.availability::text, v_course.blueprint_edit_number,
            v_course.content_discipline_id, v_course.content_subject_id,
            v_course.content_topic_id, v_course.content_subtopic_id,
            v_course.tags, v_course.theme_id;
        RETURN;
    END IF;
    v_next_blueprint_edit_number := v_course.blueprint_edit_number + 1;
    UPDATE ple_data.blueprint_course SET theme_id = p_theme_id,
        blueprint_edit_number = v_next_blueprint_edit_number
     WHERE blueprint_course_id = p_blueprint_course_id;
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_id, actor_account_id, short_name, long_name,
        availability, blueprint_edit_number, occurred_at,
        content_discipline_id, content_subject_id, content_topic_id,
        content_subtopic_id, tags, theme_id
    ) VALUES (
        p_blueprint_course_id, ple_api.current_session_account_id(),
        v_course.short_name, v_course.long_name, v_course.availability,
        v_next_blueprint_edit_number, pg_catalog.clock_timestamp(),
        v_course.content_discipline_id, v_course.content_subject_id,
        v_course.content_topic_id, v_course.content_subtopic_id,
        v_course.tags, p_theme_id
    );
    RETURN QUERY SELECT v_course.short_name, v_course.long_name,
        v_course.availability::text, v_next_blueprint_edit_number,
        v_course.content_discipline_id, v_course.content_subject_id,
        v_course.content_topic_id, v_course.content_subtopic_id,
        v_course.tags, p_theme_id;
END
$$;

DROP FUNCTION IF EXISTS ple_api.list_blueprint_courses(
    boolean, boolean, boolean, text, text, bigint, text, text, integer,
    uuid, uuid, uuid, uuid, boolean
);
DROP FUNCTION IF EXISTS ple_api.list_blueprint_courses(
    boolean, boolean, boolean, text, text, bigint, text, text, integer,
    uuid, uuid, uuid, uuid, boolean, text
);

DROP FUNCTION IF EXISTS ple_api.list_blueprint_courses(
    boolean, boolean, boolean, text, text, bigint, text, text, integer,
    uuid, uuid, uuid, uuid, boolean, text, uuid[], uuid[], uuid[], uuid[], text[]
);

DROP FUNCTION IF EXISTS ple_api.list_blueprint_courses(
    boolean, boolean, boolean, text, text, bigint, integer,
    uuid, uuid, uuid, uuid, boolean, text, uuid[], uuid[], uuid[], uuid[], text[]
);

CREATE FUNCTION ple_api.list_blueprint_courses(
    p_include_archived boolean, p_public_only boolean, p_promoted_only boolean, p_query text,
    p_sort text, p_page_number bigint, p_page_size integer,
    p_discipline_uuid uuid, p_subject_uuid uuid, p_topic_uuid uuid,
    p_subtopic_uuid uuid, p_cross_discipline boolean, p_tag text,
    p_discipline_uuids uuid[], p_subject_uuids uuid[], p_topic_uuids uuid[],
    p_subtopic_uuids uuid[], p_tags text[]
)
RETURNS TABLE (
    blueprint_course_id text, short_name text, long_name text, availability text,
    blueprint_edit_number bigint, current_blueprint_revision_number bigint, is_owner boolean,
    owner_account_id text, owner_display_name text, owner_affiliation text,
    total_adoptions bigint, total_students_ever_enrolled bigint, star_count bigint, watcher_count bigint,
    last_edited_at_millis bigint,
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid, tags text[],
    theme_id text, page_number bigint, total_count bigint
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    -- ASVS 2.2.1/3: reject contradictory visibility and unbounded pages.
    IF p_page_number IS NULL OR p_page_number < 1 OR p_page_number > 4294967295
       OR p_page_size IS NULL OR p_page_size < 1 OR p_page_size > 250
       OR (p_public_only AND p_include_archived)
       OR length(p_query) > 256
       OR p_sort NOT IN ('name', 'nameDescending', 'adoptions', 'adoptionsAscending',
                         'students', 'studentsAscending', 'stars', 'starsAscending',
                         'watches', 'watchesAscending', 'recentEdits', 'recentEditsAscending') THEN
        RAISE EXCEPTION 'invalid Blueprint discovery page' USING ERRCODE = '22023';
    END IF;
    RETURN QUERY
    WITH matching AS MATERIALIZED (
        SELECT course.blueprint_course_id::text AS blueprint_course_id, course.short_name,
           course.long_name, course.availability::text AS availability, course.blueprint_edit_number,
           course.current_blueprint_revision_number::bigint AS current_blueprint_revision_number,
           course.owner_account_id = ple_api.current_session_account_id() AS is_owner,
           course.owner_account_id::text AS owner_account_id,
           ple_private.instructor_display_name(course.owner_account_id::text)
               AS owner_display_name,
           ple_private.instructor_affiliation(course.owner_account_id::text) AS owner_affiliation,
           (SELECT count(*) FROM (
                SELECT adoption.course_instance_id
                  FROM ple_data.course_instance AS adoption
                 WHERE adoption.blueprint_course_id = course.blueprint_course_id
                UNION
                SELECT source.source_course_instance_id
                  FROM ple_data.blueprint_course_instance_source AS source
                 WHERE source.blueprint_course_id = course.blueprint_course_id
           ) AS adopted_course)::bigint AS total_adoptions,
           (SELECT COALESCE(sum(CASE
                       WHEN adoption.retention_lifecycle_state = 'deleted'
                           THEN adoption.purged_students_ever_enrolled
                       ELSE (SELECT count(DISTINCT membership.account_id)
                               FROM ple_data.course_membership AS membership
                              WHERE membership.course_instance_id = adoption.course_instance_id
                                AND membership.role = 'student')
                   END), 0)::bigint
              FROM ple_data.course_instance AS adoption
              JOIN (
                    SELECT daughter.course_instance_id
                      FROM ple_data.course_instance AS daughter
                     WHERE daughter.blueprint_course_id = course.blueprint_course_id
                    UNION
                    SELECT source.source_course_instance_id
                      FROM ple_data.blueprint_course_instance_source AS source
                     WHERE source.blueprint_course_id = course.blueprint_course_id
              ) AS adopted_course ON adopted_course.course_instance_id = adoption.course_instance_id)
              AS total_students_ever_enrolled,
           (SELECT count(*)::bigint
              FROM ple_data.blueprint_course_star AS star
             WHERE star.blueprint_course_id = course.blueprint_course_id) AS star_count,
           (SELECT count(*)::bigint
              FROM ple_data.blueprint_course_watch AS watch
             WHERE watch.blueprint_course_id = course.blueprint_course_id) AS watcher_count,
           (EXTRACT(EPOCH FROM GREATEST(
                course.created_at,
                COALESCE((SELECT max(revision.saved_at)
                            FROM ple_data.blueprint_course_revision AS revision
                           WHERE revision.blueprint_course_id = course.blueprint_course_id),
                         course.created_at),
                COALESCE((SELECT max(event.occurred_at)
                            FROM ple_data.blueprint_metadata_event AS event
                           WHERE event.blueprint_course_id = course.blueprint_course_id),
                         course.created_at)
            )) * 1000)::bigint AS last_edited_at_millis,
           course.content_discipline_id, course.content_subject_id, course.content_topic_id,
           course.content_subtopic_id, course.tags, course.theme_id
      FROM ple_data.blueprint_course AS course
      JOIN ple_api.matching_blueprint_course_ids(
          p_include_archived, p_public_only, p_promoted_only, p_query,
          p_discipline_uuid, p_subject_uuid, p_topic_uuid, p_subtopic_uuid,
          p_cross_discipline, p_tag, p_discipline_uuids, p_subject_uuids,
          p_topic_uuids, p_subtopic_uuids, p_tags
      ) AS hit ON hit.blueprint_course_id = course.blueprint_course_id
    )
    , ordered AS MATERIALIZED (
        SELECT matched.*,
               row_number() OVER (
     ORDER BY
       CASE WHEN p_sort = 'name' THEN matched.long_name END COLLATE "C" ASC,
       CASE WHEN p_sort = 'nameDescending' THEN matched.long_name END COLLATE "C" DESC,
       CASE WHEN p_sort = 'adoptions' THEN matched.total_adoptions END DESC,
       CASE WHEN p_sort = 'adoptionsAscending' THEN matched.total_adoptions END ASC,
       CASE WHEN p_sort = 'students' THEN matched.total_students_ever_enrolled END DESC,
       CASE WHEN p_sort = 'studentsAscending' THEN matched.total_students_ever_enrolled END ASC,
       CASE WHEN p_sort = 'stars' THEN matched.star_count END DESC,
       CASE WHEN p_sort = 'starsAscending' THEN matched.star_count END ASC,
       CASE WHEN p_sort = 'watches' THEN matched.watcher_count END DESC,
       CASE WHEN p_sort = 'watchesAscending' THEN matched.watcher_count END ASC,
       CASE WHEN p_sort = 'recentEdits' THEN matched.last_edited_at_millis END DESC,
       CASE WHEN p_sort = 'recentEditsAscending' THEN matched.last_edited_at_millis END ASC,
       CASE WHEN p_sort <> 'nameDescending' THEN matched.long_name END COLLATE "C" ASC,
       CASE WHEN p_sort = 'nameDescending' THEN matched.long_name END COLLATE "C" DESC,
       CASE WHEN p_sort <> 'nameDescending' THEN matched.blueprint_course_id END COLLATE "C" ASC,
       CASE WHEN p_sort = 'nameDescending' THEN matched.blueprint_course_id END COLLATE "C" DESC
               ) AS row_position
          FROM matching AS matched
    ), page_bounds AS (
        SELECT count(*)::bigint AS total_count,
               LEAST(p_page_number,
                     GREATEST(1::bigint, ceil(count(*)::numeric / p_page_size)::bigint)) AS page_number
          FROM ordered
    ), page_rows AS (
        SELECT ordered.*
          FROM ordered
          CROSS JOIN page_bounds AS bounds
         WHERE ordered.row_position > (bounds.page_number - 1) * p_page_size
           AND ordered.row_position <= bounds.page_number * p_page_size
    )
    SELECT page_rows.blueprint_course_id, page_rows.short_name, page_rows.long_name,
           page_rows.availability, page_rows.blueprint_edit_number,
           page_rows.current_blueprint_revision_number, page_rows.is_owner,
           page_rows.owner_account_id, page_rows.owner_display_name, page_rows.owner_affiliation,
           page_rows.total_adoptions, page_rows.total_students_ever_enrolled,
           page_rows.star_count, page_rows.watcher_count, page_rows.last_edited_at_millis,
           page_rows.content_discipline_id, page_rows.content_subject_id, page_rows.content_topic_id,
           page_rows.content_subtopic_id, page_rows.tags, page_rows.theme_id,
           bounds.page_number, bounds.total_count
      FROM page_bounds AS bounds
      LEFT JOIN page_rows ON true
     ORDER BY page_rows.row_position;
END
$$;

CREATE FUNCTION ple_api.load_blueprint_course(p_blueprint_course_id text)
RETURNS TABLE (
    blueprint_course_id text, short_name text, long_name text, availability text,
    blueprint_edit_number bigint, current_blueprint_revision_number bigint,
    content jsonb, content_checksum bytea, is_owner boolean,
    fork_source_blueprint_course_id text, fork_source_revision_number bigint,
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid, tags text[],
    theme_id text
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT course.blueprint_course_id::text, course.short_name, course.long_name,
           course.availability::text, course.blueprint_edit_number,
           course.current_blueprint_revision_number::bigint,
           revision.content, revision.content_checksum,
           course.owner_account_id = ple_api.current_session_account_id(),
           source.blueprint_course_id,
           CASE WHEN source.blueprint_course_id IS NOT NULL
                THEN course.parent_blueprint_revision_number::bigint END,
           course.content_discipline_id, course.content_subject_id, course.content_topic_id,
           course.content_subtopic_id, course.tags, course.theme_id
      FROM ple_data.blueprint_course AS course
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_id = course.blueprint_course_id
       AND revision.blueprint_revision_number = course.current_blueprint_revision_number
      -- All Blueprint ancestors are readable by active Instructors; roots have no source.
      LEFT JOIN ple_data.blueprint_course AS source
        ON source.blueprint_course_id = course.parent_blueprint_course_id
       AND source.availability IN ('public', 'archived')
     WHERE ple_private.is_canonical_prefixed_public_id(p_blueprint_course_id, 'BP')
       AND course.blueprint_course_id = p_blueprint_course_id
       AND ple_api.current_session_account_is_instructor()
       AND course.availability IN ('public', 'archived')
$$;



-- Exact historical provenance remains resolvable after later Saves or archive.
CREATE FUNCTION ple_api.load_blueprint_revision(
    p_blueprint_course_id text, p_blueprint_revision_number bigint
)
RETURNS TABLE (
    content jsonb, content_checksum bytea, saved_at timestamp with time zone
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT revision.content, revision.content_checksum, revision.saved_at
      FROM ple_data.blueprint_course_revision AS revision
      JOIN ple_data.blueprint_course AS course
        ON course.blueprint_course_id = revision.blueprint_course_id
     WHERE ple_private.is_canonical_prefixed_public_id(p_blueprint_course_id, 'BP')
       AND p_blueprint_revision_number > 0
       AND course.blueprint_course_id = p_blueprint_course_id
       AND revision.blueprint_revision_number = p_blueprint_revision_number
       AND ple_api.current_session_account_is_instructor()
       AND course.availability IN ('public', 'archived')
$$;



-- ASVS 8.2.1/8.2.3/8.3.1: only the authenticated active Sysadmin may inspect
-- or mutate promotion, regardless of lineage ownership or availability.
CREATE FUNCTION ple_api.load_blueprint_promotion(p_blueprint_course_id text)
RETURNS TABLE(promoted boolean, blueprint_edit_number bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT course.promoted, course.blueprint_edit_number
      FROM ple_data.blueprint_course AS course
     WHERE course.blueprint_course_id = p_blueprint_course_id
       AND ple_api.current_session_account_is_sysadmin()
$$;

CREATE FUNCTION ple_api.set_blueprint_promotion(
    p_blueprint_course_id text, p_expected_blueprint_edit_number bigint, p_promoted boolean
)
RETURNS TABLE(promoted boolean, blueprint_edit_number bigint)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_course ple_data.blueprint_course%ROWTYPE;
    v_next_blueprint_edit_number bigint;
BEGIN
    IF NOT ple_api.current_session_account_is_sysadmin() THEN
        RAISE EXCEPTION 'Blueprint promotion forbidden' USING ERRCODE = '42501';
    END IF;
    IF NOT ple_private.is_canonical_prefixed_public_id(p_blueprint_course_id, 'BP')
       OR p_expected_blueprint_edit_number IS NULL OR p_promoted IS NULL THEN
        RAISE EXCEPTION 'invalid Blueprint promotion' USING ERRCODE = '22023';
    END IF;
    -- ASVS 15.4.2: the row lock covers the metadata comparison and flag write.
    SELECT course.* INTO v_course FROM ple_data.blueprint_course AS course
     WHERE course.blueprint_course_id = p_blueprint_course_id FOR UPDATE;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    IF v_course.blueprint_edit_number <> p_expected_blueprint_edit_number THEN
        RAISE EXCEPTION 'Blueprint Course changed' USING ERRCODE = '40001';
    END IF;
    IF v_course.promoted = p_promoted THEN
        RETURN QUERY SELECT v_course.promoted, v_course.blueprint_edit_number;
        RETURN;
    END IF;
    v_next_blueprint_edit_number := v_course.blueprint_edit_number + 1;
    UPDATE ple_data.blueprint_course AS course
       SET promoted = p_promoted, blueprint_edit_number = v_next_blueprint_edit_number
     WHERE course.blueprint_course_id = v_course.blueprint_course_id;
    RETURN QUERY SELECT p_promoted, v_next_blueprint_edit_number;
END
$$;
