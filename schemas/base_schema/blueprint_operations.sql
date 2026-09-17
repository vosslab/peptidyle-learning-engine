SET LOCAL ROLE ple_private_owner;
GRANT CREATE ON SCHEMA ple_private TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;

-- This private primitive has one narrowly trusted exception for copying a
-- locked source Course's exact retired Discipline. The public wrapper below
-- never supplies that value, so ordinary new Blueprints remain active-only.
CREATE FUNCTION ple_private.create_blueprint_course(
    p_blueprint_id uuid, p_request_checksum bytea, p_short_name text, p_long_name text,
    p_content jsonb, p_content_checksum bytea,
    p_discipline uuid, p_subject uuid, p_topic uuid, p_subtopic uuid, p_tags text[],
    p_retired_source_discipline uuid
)
RETURNS TABLE (
    public_reference text, blueprint_revision_number bigint, metadata_etag uuid,
    accepted_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_actor uuid;
    v_now timestamp with time zone;
    v_metadata_etag uuid;
    v_reference_number bigint;
BEGIN
    IF p_blueprint_id IS NULL OR octet_length(p_request_checksum) <> 32
       OR p_short_name IS NULL OR p_short_name <> btrim(p_short_name)
       OR char_length(p_short_name) NOT BETWEEN 1 AND 500
       OR p_long_name IS NULL OR p_long_name <> btrim(p_long_name)
       OR char_length(p_long_name) NOT BETWEEN 1 AND 500
       OR octet_length(p_content_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course creation is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('ple:blueprint-course-create:%s:%s', v_actor,
            pg_catalog.encode(p_request_checksum, 'hex')), 0));
    SELECT course.public_reference, receipt.blueprint_revision_number,
           receipt.metadata_etag, receipt.accepted_at
      INTO public_reference, blueprint_revision_number, metadata_etag, accepted_at
      FROM ple_data.blueprint_course_create_receipt AS receipt
      JOIN ple_data.blueprint_course AS course
        ON course.reference_number = receipt.blueprint_course_reference_number
     WHERE receipt.actor_account_id = v_actor AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN RETURN NEXT; RETURN; END IF;
    PERFORM ple_data.validate_blueprint_content(p_content);
    PERFORM ple_data.validate_blueprint_question_selection(NULL, NULL, p_content);
    v_now := pg_catalog.clock_timestamp();
    v_metadata_etag := pg_catalog.gen_random_uuid();
    IF p_retired_source_discipline IS NULL
       OR p_discipline IS DISTINCT FROM p_retired_source_discipline
       OR NOT EXISTS (
           SELECT 1 FROM ple_api.get_content_discipline(p_retired_source_discipline) AS discipline
            WHERE discipline.discipline_uuid = p_retired_source_discipline
              AND discipline.is_retired
       ) THEN
        PERFORM ple_api.require_active_content_discipline(p_discipline);
    END IF;
    INSERT INTO ple_data.blueprint_course AS course (
        blueprint_id, owner_account_id, short_name, long_name, metadata_etag, created_at,
        discipline_uuid, subject_uuid, topic_uuid, subtopic_uuid, tags
    ) VALUES (
        p_blueprint_id, v_actor, p_short_name, p_long_name, v_metadata_etag, v_now,
        p_discipline, p_subject, p_topic, p_subtopic, p_tags
    ) RETURNING course.reference_number INTO v_reference_number;
    blueprint_revision_number := 1;
    INSERT INTO ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number,
        content, content_checksum, saved_at
    ) VALUES (
        v_reference_number, blueprint_revision_number, p_content, p_content_checksum, v_now
    );
    INSERT INTO ple_data.blueprint_revision_question_pin
    SELECT v_reference_number, blueprint_revision_number, pins.content_path,
           pins.question_id, pins.question_revision_number
      FROM ple_data.blueprint_content_question_pins(p_content) AS pins;
    INSERT INTO ple_data.blueprint_revision_module
    SELECT v_reference_number, blueprint_revision_number,
           modules.blueprint_module_reference, modules.module_position
      FROM ple_data.blueprint_content_modules(p_content) AS modules;
    INSERT INTO ple_data.blueprint_revision_assessment
    SELECT v_reference_number, blueprint_revision_number,
           assessments.blueprint_module_reference,
           assessments.blueprint_assessment_reference, assessments.assessment_position
      FROM ple_data.blueprint_content_assessments(p_content) AS assessments;
    INSERT INTO ple_data.blueprint_revision_event (
        blueprint_course_reference_number, blueprint_revision_number, actor_account_id,
        request_checksum, occurred_at
    ) VALUES (
        v_reference_number, blueprint_revision_number, v_actor, p_request_checksum, v_now
    );
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_reference_number, actor_account_id, short_name, long_name,
        availability, metadata_etag, occurred_at,
        discipline_uuid, subject_uuid, topic_uuid, subtopic_uuid, tags
    ) VALUES (
        v_reference_number, v_actor, p_short_name, p_long_name,
        'private', v_metadata_etag, v_now,
        p_discipline, p_subject, p_topic, p_subtopic, p_tags
    );
    INSERT INTO ple_data.blueprint_course_create_receipt
    VALUES (
        v_actor, p_request_checksum, v_reference_number, blueprint_revision_number,
        v_metadata_etag, v_now
    );
    SELECT course.public_reference INTO public_reference
      FROM ple_data.blueprint_course AS course
     WHERE course.reference_number = v_reference_number;
    metadata_etag := v_metadata_etag; accepted_at := v_now;
    RETURN NEXT;
END
$$;

REVOKE ALL ON FUNCTION ple_private.create_blueprint_course(
    uuid, bytea, text, text, jsonb, bytea, uuid, uuid, uuid, uuid, text[], uuid
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.create_blueprint_course(
    uuid, bytea, text, text, jsonb, bytea, uuid, uuid, uuid, uuid, text[], uuid
) TO ple_api_owner;

RESET ROLE;
SET LOCAL ROLE ple_private_owner;
REVOKE CREATE ON SCHEMA ple_private FROM ple_api_owner;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.create_blueprint_course(
    p_blueprint_id uuid, p_request_checksum bytea, p_short_name text, p_long_name text,
    p_content jsonb, p_content_checksum bytea,
    p_discipline uuid, p_subject uuid, p_topic uuid, p_subtopic uuid, p_tags text[]
)
RETURNS TABLE (
    public_reference text, blueprint_revision_number bigint, metadata_etag uuid,
    accepted_at timestamp with time zone
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT * FROM ple_private.create_blueprint_course(
        p_blueprint_id, p_request_checksum, p_short_name, p_long_name,
        p_content, p_content_checksum, p_discipline, p_subject, p_topic,
        p_subtopic, p_tags, NULL
    )
$$;

CREATE FUNCTION ple_api.save_blueprint_course(
    p_reference text, p_expected_blueprint_revision_number bigint,
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
    v_actor uuid;
    v_course ple_data.blueprint_course%ROWTYPE;
    v_prior ple_data.blueprint_course_revision%ROWTYPE;
    v_now timestamp with time zone;
    v_reference_number bigint;
BEGIN
    IF NOT ple_private.is_canonical_prefixed_public_id(p_reference, 'BP')
       OR p_expected_blueprint_revision_number <= 0
       OR octet_length(p_request_checksum) <> 32
       OR octet_length(p_content_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Course Save is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    SELECT course.reference_number INTO v_reference_number
      FROM ple_data.blueprint_course AS course
     WHERE course.public_reference = p_reference;
    SELECT * INTO v_course
      FROM ple_data.blueprint_course
     WHERE reference_number = v_reference_number AND owner_account_id = v_actor
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
     WHERE receipt.blueprint_course_reference_number = v_reference_number
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
        v_reference_number, p_expected_blueprint_revision_number, p_content
    );
    SELECT * INTO STRICT v_prior
      FROM ple_data.blueprint_course_revision
     WHERE blueprint_course_reference_number = v_reference_number
       AND blueprint_revision_number = p_expected_blueprint_revision_number;
    v_now := pg_catalog.clock_timestamp();
    changed := v_prior.content IS DISTINCT FROM p_content
        OR v_prior.content_checksum IS DISTINCT FROM p_content_checksum;
    IF changed THEN
        resulting_blueprint_revision_number := p_expected_blueprint_revision_number + 1;
        INSERT INTO ple_data.blueprint_course_revision (
            blueprint_course_reference_number, blueprint_revision_number,
            content, content_checksum, saved_at
        ) VALUES (
            v_reference_number, resulting_blueprint_revision_number,
            p_content, p_content_checksum, v_now
        );
        INSERT INTO ple_data.blueprint_revision_question_pin
        SELECT v_reference_number, resulting_blueprint_revision_number, pins.content_path,
               pins.question_id, pins.question_revision_number
          FROM ple_data.blueprint_content_question_pins(p_content) AS pins;
        INSERT INTO ple_data.blueprint_revision_module
        SELECT v_reference_number, resulting_blueprint_revision_number,
               modules.blueprint_module_reference, modules.module_position
          FROM ple_data.blueprint_content_modules(p_content) AS modules;
        INSERT INTO ple_data.blueprint_revision_assessment
        SELECT v_reference_number, resulting_blueprint_revision_number,
               assessments.blueprint_module_reference,
               assessments.blueprint_assessment_reference, assessments.assessment_position
          FROM ple_data.blueprint_content_assessments(p_content) AS assessments;
        INSERT INTO ple_data.blueprint_revision_event (
            blueprint_course_reference_number, blueprint_revision_number, actor_account_id,
            request_checksum, occurred_at
        ) VALUES (
            v_reference_number, resulting_blueprint_revision_number,
            v_actor, p_request_checksum, v_now
        );
        UPDATE ple_data.blueprint_course
           SET current_blueprint_revision_number = resulting_blueprint_revision_number
         WHERE reference_number = v_reference_number;
    ELSE
        resulting_blueprint_revision_number := p_expected_blueprint_revision_number;
    END IF;
    INSERT INTO ple_data.blueprint_course_save_receipt
    VALUES (
        v_reference_number, v_actor, p_request_checksum,
        resulting_blueprint_revision_number, changed, v_now
    );
    accepted_at := v_now; RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.rename_blueprint_course(
    p_reference text, p_expected_metadata_etag uuid,
    p_short_name text, p_long_name text
)
RETURNS TABLE (
    short_name text, long_name text, availability text, metadata_etag uuid,
    discipline_uuid uuid, subject_uuid uuid, topic_uuid uuid, subtopic_uuid uuid, tags text[]
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_actor uuid;
    v_course ple_data.blueprint_course%ROWTYPE;
    v_next uuid;
    v_reference_number bigint;
BEGIN
    IF NOT ple_private.is_canonical_prefixed_public_id(p_reference, 'BP')
       OR p_expected_metadata_etag IS NULL
       OR p_short_name IS NULL OR p_short_name <> btrim(p_short_name)
       OR char_length(p_short_name) NOT BETWEEN 1 AND 500
       OR p_long_name IS NULL OR p_long_name <> btrim(p_long_name)
       OR char_length(p_long_name) NOT BETWEEN 1 AND 500
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course rename is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    SELECT course.reference_number INTO v_reference_number
      FROM ple_data.blueprint_course AS course
     WHERE course.public_reference = p_reference;
    SELECT * INTO v_course FROM ple_data.blueprint_course
     WHERE reference_number = v_reference_number AND owner_account_id = v_actor FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is not available';
    END IF;
    -- ASVS 8.3.1 / 2.3.1: lifecycle denial precedes CAS and no-op rename.
    IF v_course.availability = 'archived' THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Archived Blueprint Course is read-only';
    END IF;
    IF v_course.metadata_etag <> p_expected_metadata_etag THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint metadata ETag is stale';
    END IF;
    IF v_course.short_name = p_short_name AND v_course.long_name = p_long_name THEN
        RETURN QUERY SELECT v_course.short_name, v_course.long_name,
            v_course.availability, v_course.metadata_etag,
            v_course.discipline_uuid, v_course.subject_uuid, v_course.topic_uuid,
            v_course.subtopic_uuid, v_course.tags;
        RETURN;
    END IF;
    v_next := pg_catalog.gen_random_uuid();
    UPDATE ple_data.blueprint_course AS course
       SET short_name = p_short_name, long_name = p_long_name, metadata_etag = v_next
     WHERE course.reference_number = v_reference_number;
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_reference_number, actor_account_id, short_name, long_name,
        availability, metadata_etag, occurred_at,
        discipline_uuid, subject_uuid, topic_uuid, subtopic_uuid, tags
    ) VALUES (
        v_reference_number, v_actor, p_short_name, p_long_name,
        v_course.availability, v_next, pg_catalog.clock_timestamp(),
        v_course.discipline_uuid, v_course.subject_uuid, v_course.topic_uuid,
        v_course.subtopic_uuid, v_course.tags
    );
    RETURN QUERY SELECT p_short_name, p_long_name, v_course.availability, v_next,
        v_course.discipline_uuid, v_course.subject_uuid, v_course.topic_uuid,
        v_course.subtopic_uuid, v_course.tags;
END
$$;

CREATE FUNCTION ple_api.set_blueprint_availability(
    p_reference text, p_expected_metadata_etag uuid, p_availability text,
    p_archive_confirmation_long_name text
)
RETURNS TABLE (
    short_name text, long_name text, availability text, metadata_etag uuid,
    discipline_uuid uuid, subject_uuid uuid, topic_uuid uuid, subtopic_uuid uuid, tags text[]
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_actor uuid;
    v_course ple_data.blueprint_course%ROWTYPE;
    v_next uuid;
    v_reference_number bigint;
BEGIN
    IF NOT ple_private.is_canonical_prefixed_public_id(p_reference, 'BP')
       OR p_expected_metadata_etag IS NULL
       OR p_availability NOT IN ('private', 'public', 'archived')
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint availability change is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    SELECT course.reference_number INTO v_reference_number
      FROM ple_data.blueprint_course AS course
     WHERE course.public_reference = p_reference;
    SELECT * INTO v_course FROM ple_data.blueprint_course
     WHERE reference_number = v_reference_number AND owner_account_id = v_actor FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is not available';
    END IF;
    IF v_course.metadata_etag <> p_expected_metadata_etag THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint metadata ETag is stale';
    END IF;
    -- Reusable content advances through one directed lifecycle. A Public
    -- lineage may become Private only before its first Adoption, whether that
    -- is a daughter or the originating source Course Instance. Every accepted
    -- transition is recorded below as an immutable metadata event.
    IF (v_course.availability = 'private' AND p_availability <> 'public')
       OR (v_course.availability = 'public' AND p_availability = 'private'
           AND (
               EXISTS (
                   SELECT 1 FROM ple_data.course_instance AS adoption
                    WHERE adoption.blueprint_course_reference_number = v_reference_number
               )
               OR EXISTS (
                   SELECT 1 FROM ple_data.blueprint_course_instance_source AS source
                    WHERE source.blueprint_course_reference_number = v_reference_number
               )
           ))
       OR (v_course.availability = 'public'
           AND p_availability NOT IN ('private', 'archived'))
       OR (v_course.availability = 'archived' AND p_availability <> 'public') THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Blueprint lifecycle transition is not permitted';
    END IF;
    IF p_availability = 'archived'
       AND p_archive_confirmation_long_name IS DISTINCT FROM v_course.long_name THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Archive Blueprint requires exact long name confirmation';
    END IF;
    IF v_course.availability = p_availability THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Blueprint availability already has that state';
    END IF;
    v_next := pg_catalog.gen_random_uuid();
    UPDATE ple_data.blueprint_course AS course
       SET availability = p_availability, metadata_etag = v_next
     WHERE course.reference_number = v_reference_number;
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_reference_number, actor_account_id, short_name, long_name,
        availability, metadata_etag, occurred_at,
        discipline_uuid, subject_uuid, topic_uuid, subtopic_uuid, tags
    ) VALUES (
        v_reference_number, v_actor, v_course.short_name, v_course.long_name,
        p_availability, v_next, pg_catalog.clock_timestamp(),
        v_course.discipline_uuid, v_course.subject_uuid, v_course.topic_uuid,
        v_course.subtopic_uuid, v_course.tags
    );
    RETURN QUERY SELECT v_course.short_name, v_course.long_name, p_availability, v_next,
        v_course.discipline_uuid, v_course.subject_uuid, v_course.topic_uuid,
        v_course.subtopic_uuid, v_course.tags;
END
$$;

CREATE FUNCTION ple_api.update_blueprint_classification(
    p_reference text, p_expected_metadata_etag uuid,
    p_discipline uuid, p_subject uuid, p_topic uuid, p_subtopic uuid, p_tags text[]
)
RETURNS TABLE(metadata_etag uuid, changed boolean)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_course ple_data.blueprint_course%ROWTYPE;
    v_next uuid;
BEGIN
    -- ASVS 8.2.1/8.2.2: ownership, not ambient Product Role, permits mutation.
    SELECT course.* INTO v_course FROM ple_data.blueprint_course AS course
     WHERE course.public_reference = p_reference
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
    IF p_expected_metadata_etag IS DISTINCT FROM v_course.metadata_etag THEN
        RAISE EXCEPTION 'Blueprint metadata ETag is stale' USING ERRCODE = '40001';
    END IF;
    IF ROW(v_course.discipline_uuid, v_course.subject_uuid, v_course.topic_uuid,
           v_course.subtopic_uuid, v_course.tags)
       IS NOT DISTINCT FROM ROW(p_discipline, p_subject, p_topic, p_subtopic, p_tags) THEN
        RETURN QUERY SELECT v_course.metadata_etag, false;
        RETURN;
    END IF;
    v_next := pg_catalog.gen_random_uuid();
    -- Retaining an existing retired Discipline does not make it a new choice.
    -- A true replacement requires an active Discipline and retains its row
    -- lock through this update transaction.
    IF v_course.discipline_uuid IS DISTINCT FROM p_discipline THEN
        PERFORM ple_api.require_active_content_discipline(p_discipline);
    END IF;
    UPDATE ple_data.blueprint_course AS course SET
        discipline_uuid = p_discipline, subject_uuid = p_subject, topic_uuid = p_topic,
        subtopic_uuid = p_subtopic, tags = p_tags, metadata_etag = v_next
     WHERE course.reference_number = v_course.reference_number;
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_reference_number, actor_account_id, short_name, long_name,
        availability, metadata_etag, occurred_at,
        discipline_uuid, subject_uuid, topic_uuid, subtopic_uuid, tags
    ) VALUES (
        v_course.reference_number, ple_api.current_session_account_id(),
        v_course.short_name, v_course.long_name, v_course.availability,
        v_next, pg_catalog.clock_timestamp(), p_discipline, p_subject, p_topic, p_subtopic, p_tags
    );
    RETURN QUERY SELECT v_next, true;
END
$$;

REVOKE ALL ON FUNCTION ple_api.update_blueprint_classification(text, uuid, uuid, uuid, uuid, uuid, text[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.update_blueprint_classification(text, uuid, uuid, uuid, uuid, uuid, text[]) TO ple_app;

CREATE FUNCTION ple_api.list_blueprint_courses(
    p_include_archived boolean, p_public_only boolean, p_promoted_only boolean, p_query text,
    p_after_long_name text, p_after_reference text, p_limit integer,
    p_discipline_uuid uuid, p_subject_uuid uuid, p_topic_uuid uuid,
    p_subtopic_uuid uuid, p_cross_discipline boolean
)
RETURNS TABLE (
    public_reference text, short_name text, long_name text, availability text,
    metadata_etag uuid, current_blueprint_revision_number bigint, is_owner boolean,
    total_adoptions bigint, total_students_ever_enrolled bigint,
    discipline_uuid uuid, subject_uuid uuid, topic_uuid uuid, subtopic_uuid uuid, tags text[]
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    -- ASVS 2.2.1/3: reject contradictory visibility and unbounded pages.
    IF p_limit < 1 OR p_limit > 101 OR (p_public_only AND p_include_archived)
       OR length(p_query) > 256
       OR (p_after_long_name IS NULL) <> (p_after_reference IS NULL) THEN
        RAISE EXCEPTION 'invalid Blueprint discovery page' USING ERRCODE = '22023';
    END IF;
    -- ASVS 2.2.2/8.3.1: validate identity and actual parents through authenticated reads.
    IF (p_subject_uuid IS NOT NULL AND p_discipline_uuid IS NULL)
       OR (p_topic_uuid IS NOT NULL AND p_subject_uuid IS NULL)
       OR (p_subtopic_uuid IS NOT NULL AND p_topic_uuid IS NULL)
       OR (p_cross_discipline AND (p_discipline_uuid IS NULL OR p_subject_uuid IS NULL)) THEN
        RAISE EXCEPTION 'invalid Blueprint classification filter' USING ERRCODE = '22023';
    END IF;
    IF (p_discipline_uuid IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ple_api.list_content_disciplines_including_retired() AS item
             WHERE item.discipline_uuid = p_discipline_uuid))
       OR (p_subject_uuid IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ple_api.list_content_subjects(p_discipline_uuid) AS item
             WHERE item.subject_uuid = p_subject_uuid))
       OR (p_topic_uuid IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ple_api.list_content_topics(p_subject_uuid) AS item
             WHERE item.topic_uuid = p_topic_uuid))
       OR (p_subtopic_uuid IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ple_api.list_content_subtopics(p_topic_uuid) AS item
             WHERE item.subtopic_uuid = p_subtopic_uuid)) THEN
        RAISE EXCEPTION 'invalid Blueprint classification filter' USING ERRCODE = '22023';
    END IF;
    RETURN QUERY SELECT course.public_reference, course.short_name, course.long_name,
           course.availability, course.metadata_etag,
           course.current_blueprint_revision_number,
           course.owner_account_id = ple_api.current_session_account_id(),
           (SELECT count(*) FROM (
                SELECT adoption.course_id
                  FROM ple_data.course_instance AS adoption
                 WHERE adoption.blueprint_course_reference_number = course.reference_number
                UNION
                SELECT source.source_course_id
                  FROM ple_data.blueprint_course_instance_source AS source
                 WHERE source.blueprint_course_reference_number = course.reference_number
           ) AS adopted_course),
           (SELECT COALESCE(sum(CASE
                       WHEN adoption.retention_lifecycle_state = 'deleted'
                           THEN adoption.purged_students_ever_enrolled
                       ELSE (SELECT count(DISTINCT membership.account_id)
                               FROM ple_data.course_membership AS membership
                              WHERE membership.course_id = adoption.course_id
                                AND membership.role = 'student')
                   END), 0)::bigint
              FROM ple_data.course_instance AS adoption
              JOIN (
                    SELECT daughter.course_id
                      FROM ple_data.course_instance AS daughter
                     WHERE daughter.blueprint_course_reference_number = course.reference_number
                    UNION
                    SELECT source.source_course_id
                      FROM ple_data.blueprint_course_instance_source AS source
                     WHERE source.blueprint_course_reference_number = course.reference_number
              ) AS adopted_course ON adopted_course.course_id = adoption.course_id),
           course.discipline_uuid, course.subject_uuid, course.topic_uuid,
           course.subtopic_uuid, course.tags
      FROM ple_data.blueprint_course AS course
     WHERE ple_api.current_session_account_is_instructor()
       AND (NOT p_public_only OR course.availability = 'public')
       AND (NOT p_promoted_only OR course.promoted)
       AND (p_cross_discipline OR p_discipline_uuid IS NULL
            OR course.discipline_uuid = p_discipline_uuid)
       AND (p_subject_uuid IS NULL OR course.subject_uuid = p_subject_uuid)
       AND (p_topic_uuid IS NULL OR course.topic_uuid = p_topic_uuid)
       AND (p_subtopic_uuid IS NULL OR course.subtopic_uuid = p_subtopic_uuid)
       -- ASVS 1.2.4: parameters remain literal text, including LIKE metacharacters.
       AND (p_query = '' OR course.short_name ILIKE
            '%' || replace(replace(replace(p_query, '\', '\\'), '%', '\%'), '_', '\_') || '%'
            OR course.long_name ILIKE
            '%' || replace(replace(replace(p_query, '\', '\\'), '%', '\%'), '_', '\_') || '%')
       AND (p_after_long_name IS NULL OR
            (course.long_name COLLATE "C", course.public_reference COLLATE "C") >
            (p_after_long_name COLLATE "C", p_after_reference COLLATE "C"))
       AND (
           -- ASVS 8.2.2/8.3.1: opt-in history never exposes another owner's Private course.
           course.availability = 'public'
           OR (course.owner_account_id = ple_api.current_session_account_id()
               AND course.availability = 'private')
           OR (p_include_archived AND course.availability = 'archived')
       )
     ORDER BY course.long_name COLLATE "C", course.public_reference COLLATE "C"
     LIMIT p_limit;
END
$$;

CREATE FUNCTION ple_api.load_blueprint_course(p_reference text)
RETURNS TABLE (
    public_reference text, short_name text, long_name text, availability text,
    metadata_etag uuid, current_blueprint_revision_number bigint,
    content jsonb, content_checksum bytea, is_owner boolean,
    fork_source_reference text, fork_source_revision_number bigint,
    discipline_uuid uuid, subject_uuid uuid, topic_uuid uuid, subtopic_uuid uuid, tags text[]
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT course.public_reference, course.short_name, course.long_name,
           course.availability, course.metadata_etag,
           course.current_blueprint_revision_number,
           revision.content, revision.content_checksum,
           course.owner_account_id = ple_api.current_session_account_id(),
           source.public_reference,
           CASE WHEN source.reference_number IS NOT NULL
                THEN ancestry.source_blueprint_revision_number END,
           course.discipline_uuid, course.subject_uuid, course.topic_uuid,
           course.subtopic_uuid, course.tags
      FROM ple_data.blueprint_course AS course
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_reference_number = course.reference_number
       AND revision.blueprint_revision_number = course.current_blueprint_revision_number
      LEFT JOIN ple_data.blueprint_course_fork AS ancestry
        ON ancestry.blueprint_course_reference_number = course.reference_number
      -- ASVS 8.2.2/3, 8.3.1/2: mask ancestry using current source visibility.
      -- Roots and hidden sources share the same two null fields.
      LEFT JOIN ple_data.blueprint_course AS source
        ON source.reference_number = ancestry.source_blueprint_course_reference_number
       AND (source.availability IN ('public', 'archived')
            OR source.owner_account_id = ple_api.current_session_account_id())
     WHERE ple_private.is_canonical_prefixed_public_id(p_reference, 'BP')
       AND course.public_reference = p_reference
       AND ple_api.current_session_account_is_instructor()
       AND (
           course.owner_account_id = ple_api.current_session_account_id()
           OR course.availability IN ('public', 'archived')
       )
$$;

-- Exact historical provenance remains resolvable after later Saves or archive.
CREATE FUNCTION ple_api.load_blueprint_revision(
    p_reference text, p_blueprint_revision_number bigint
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
        ON course.reference_number = revision.blueprint_course_reference_number
     WHERE ple_private.is_canonical_prefixed_public_id(p_reference, 'BP')
       AND p_blueprint_revision_number > 0
       AND course.public_reference = p_reference
       AND revision.blueprint_revision_number = p_blueprint_revision_number
       AND ple_api.current_session_account_is_instructor()
       AND (
           course.owner_account_id = ple_api.current_session_account_id()
           OR course.availability IN ('public', 'archived')
       )
$$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.create_blueprint_course(uuid, bytea, text, text, jsonb, bytea, uuid, uuid, uuid, uuid, text[]),
    ple_api.save_blueprint_course(text, bigint, bytea, jsonb, bytea),
    ple_api.rename_blueprint_course(text, uuid, text, text),
    ple_api.set_blueprint_availability(text, uuid, text, text),
    ple_api.list_blueprint_courses(boolean, boolean, boolean, text, text, text, integer, uuid, uuid, uuid, uuid, boolean), ple_api.load_blueprint_course(text),
    ple_api.load_blueprint_revision(text, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.create_blueprint_course(uuid, bytea, text, text, jsonb, bytea, uuid, uuid, uuid, uuid, text[]),
    ple_api.rename_blueprint_course(text, uuid, text, text),
    ple_api.set_blueprint_availability(text, uuid, text, text),
    ple_api.list_blueprint_courses(boolean, boolean, boolean, text, text, text, integer, uuid, uuid, uuid, uuid, boolean), ple_api.load_blueprint_course(text),
    ple_api.load_blueprint_revision(text, bigint) TO ple_app;

-- ASVS 8.2.1/8.2.3/8.3.1: only the authenticated active Sysadmin may inspect
-- or mutate promotion, regardless of lineage ownership or availability.
CREATE FUNCTION ple_api.load_blueprint_promotion(p_reference text)
RETURNS TABLE(promoted boolean, metadata_etag uuid)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT course.promoted, course.metadata_etag
      FROM ple_data.blueprint_course AS course
     WHERE course.public_reference = p_reference
       AND ple_api.current_session_account_is_sysadmin()
$$;

CREATE FUNCTION ple_api.set_blueprint_promotion(
    p_reference text, p_expected_metadata_etag uuid, p_promoted boolean
)
RETURNS TABLE(promoted boolean, metadata_etag uuid)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_course ple_data.blueprint_course%ROWTYPE;
    v_next uuid;
BEGIN
    IF NOT ple_api.current_session_account_is_sysadmin() THEN
        RAISE EXCEPTION 'Blueprint promotion forbidden' USING ERRCODE = '42501';
    END IF;
    IF NOT ple_private.is_canonical_prefixed_public_id(p_reference, 'BP')
       OR p_expected_metadata_etag IS NULL OR p_promoted IS NULL THEN
        RAISE EXCEPTION 'invalid Blueprint promotion' USING ERRCODE = '22023';
    END IF;
    -- ASVS 15.4.2: the row lock covers the metadata comparison and flag write.
    SELECT course.* INTO v_course FROM ple_data.blueprint_course AS course
     WHERE course.public_reference = p_reference FOR UPDATE;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    IF v_course.metadata_etag <> p_expected_metadata_etag THEN
        RAISE EXCEPTION 'Blueprint Course changed' USING ERRCODE = '40001';
    END IF;
    IF v_course.promoted = p_promoted THEN
        RETURN QUERY SELECT v_course.promoted, v_course.metadata_etag;
        RETURN;
    END IF;
    v_next := pg_catalog.gen_random_uuid();
    UPDATE ple_data.blueprint_course AS course
       SET promoted = p_promoted, metadata_etag = v_next
     WHERE course.blueprint_id = v_course.blueprint_id;
    RETURN QUERY SELECT p_promoted, v_next;
END
$$;

REVOKE ALL ON FUNCTION ple_api.load_blueprint_promotion(text),
    ple_api.set_blueprint_promotion(text, uuid, boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.load_blueprint_promotion(text),
    ple_api.set_blueprint_promotion(text, uuid, boolean) TO ple_app;

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.blueprint_course IS
    'Stable reusable Blueprint Course lineage with names, availability, metadata ETag, and current Revision.';
COMMENT ON TABLE ple_data.blueprint_course_revision IS
    'Immutable complete Blueprint Revision; exact references remain valid after later Saves or archive.';
COMMENT ON TABLE ple_data.blueprint_revision_assessment IS
    'Durable Blueprint Assessment identity and Revision membership retained for provenance and comparison.';

RESET ROLE;
