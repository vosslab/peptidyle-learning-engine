-- Preserve immutable v2 Blueprint revision content while making the independent
-- submitted-response disclosure timing explicit in every new v3 revision.

DO $$
BEGIN
    IF current_user <> 'ple_migrator' THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'migration 2026091018 must run as ple_migrator';
    END IF;
END
$$;

SET LOCAL ROLE ple_data_owner;

DO $$
DECLARE
    v_legacy_constraint_name name;
BEGIN
    SELECT constraint_record.conname
      INTO STRICT v_legacy_constraint_name
      FROM pg_catalog.pg_constraint AS constraint_record
      JOIN pg_catalog.pg_attribute AS checked_column
        ON checked_column.attrelid = constraint_record.conrelid
       AND checked_column.attnum = constraint_record.conkey[1]
     WHERE constraint_record.conrelid = 'ple_data.blueprint_course_revision'::pg_catalog.regclass
       AND constraint_record.contype = 'c'
       AND pg_catalog.cardinality(constraint_record.conkey) = 1
       AND checked_column.attname = 'blueprint_content_encoding_version'
       AND pg_catalog.pg_get_expr(
            constraint_record.conbin, constraint_record.conrelid
       ) = '(blueprint_content_encoding_version = 2)';

    EXECUTE pg_catalog.format(
        'ALTER TABLE ple_data.blueprint_course_revision DROP CONSTRAINT %I',
        v_legacy_constraint_name
    );
END
$$;

ALTER TABLE ple_data.blueprint_course_revision
    ADD CONSTRAINT blueprint_course_revision_content_encoding_version_check
        CHECK (blueprint_content_encoding_version IN (2, 3));

RESET ROLE;
SET LOCAL ROLE ple_api_owner;

CREATE OR REPLACE FUNCTION ple_api.create_live_demo_blueprint_course(
    p_blueprint_id uuid,
    p_publication_event_id uuid,
    p_availability_event_id uuid,
    p_blueprint_course_content jsonb,
    p_blueprint_content_checksum bytea
)
RETURNS bigint LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE
    v_account_id uuid;
    v_reference_number bigint;
    v_title text;
    v_occurred_at timestamp with time zone;
BEGIN
    IF p_blueprint_id IS NULL OR p_publication_event_id IS NULL OR p_availability_event_id IS NULL
       OR jsonb_typeof(p_blueprint_course_content) <> 'object'
       OR octet_length(p_blueprint_content_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course creation arguments are invalid';
    END IF;
    v_title := p_blueprint_course_content ->> 'title';
    IF v_title IS NULL OR v_title <> btrim(v_title)
       OR char_length(v_title) NOT BETWEEN 1 AND 200
       OR jsonb_typeof(p_blueprint_course_content -> 'modules') <> 'array'
       OR jsonb_array_length(p_blueprint_course_content -> 'modules') NOT BETWEEN 1 AND 1024 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course content is invalid';
    END IF;
    v_account_id := ple_api.current_session_account_id();
    v_occurred_at := pg_catalog.clock_timestamp();
    INSERT INTO ple_data.blueprint_course (
        blueprint_id, blueprint_course_owner_account_id, created_at
    ) VALUES (p_blueprint_id, v_account_id, v_occurred_at)
    RETURNING reference_number INTO v_reference_number;
    INSERT INTO ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number, title,
        blueprint_course_content, blueprint_content_encoding_version,
        blueprint_content_checksum, created_at
    ) VALUES (
        v_reference_number, 1, v_title, p_blueprint_course_content, 3,
        p_blueprint_content_checksum, v_occurred_at
    );
    INSERT INTO ple_data.blueprint_publication_event (
        blueprint_publication_event_id, blueprint_course_reference_number,
        blueprint_revision_number, published_by_account_id, occurred_at
    ) VALUES (
        p_publication_event_id, v_reference_number, 1, v_account_id, v_occurred_at
    );
    INSERT INTO ple_data.blueprint_revision_availability_event (
        blueprint_revision_availability_event_id, blueprint_course_reference_number,
        blueprint_revision_number, recorded_by_account_id, event_kind, occurred_at
    ) VALUES (
        p_availability_event_id, v_reference_number, 1, v_account_id, 'available', v_occurred_at
    );
    RETURN v_reference_number;
END
$$;

CREATE OR REPLACE FUNCTION ple_api.replace_live_demo_blueprint_course(
    p_reference_number bigint,
    p_expected_revision_number bigint,
    p_publication_event_id uuid,
    p_availability_event_id uuid,
    p_blueprint_course_content jsonb,
    p_blueprint_content_checksum bytea
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE
    v_account_id uuid;
    v_owner_account_id uuid;
    v_current_revision_number bigint;
    v_next_revision_number bigint;
    v_title text;
    v_occurred_at timestamp with time zone;
BEGIN
    IF p_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_revision_number IS NULL OR p_expected_revision_number <= 0
       OR p_publication_event_id IS NULL OR p_availability_event_id IS NULL
       OR jsonb_typeof(p_blueprint_course_content) <> 'object'
       OR octet_length(p_blueprint_content_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course replacement arguments are invalid';
    END IF;
    v_title := p_blueprint_course_content ->> 'title';
    IF v_title IS NULL OR v_title <> btrim(v_title)
       OR char_length(v_title) NOT BETWEEN 1 AND 200
       OR jsonb_typeof(p_blueprint_course_content -> 'modules') <> 'array'
       OR jsonb_array_length(p_blueprint_course_content -> 'modules') NOT BETWEEN 1 AND 1024 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course content is invalid';
    END IF;
    v_account_id := ple_api.current_session_account_id();
    SELECT blueprint.blueprint_course_owner_account_id
      INTO v_owner_account_id
      FROM ple_data.blueprint_course AS blueprint
     WHERE blueprint.reference_number = p_reference_number
     FOR UPDATE;
    IF NOT FOUND OR v_owner_account_id <> v_account_id THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Blueprint Course replacement requires the current Blueprint Course Owner';
    END IF;
    SELECT MAX(revision.blueprint_revision_number)
      INTO v_current_revision_number
      FROM ple_data.blueprint_course_revision AS revision
     WHERE revision.blueprint_course_reference_number = p_reference_number;
    IF v_current_revision_number <> p_expected_revision_number THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Blueprint Course Revision is stale';
    END IF;
    v_next_revision_number := v_current_revision_number + 1;
    v_occurred_at := pg_catalog.clock_timestamp();
    INSERT INTO ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number, title,
        blueprint_course_content, blueprint_content_encoding_version,
        blueprint_content_checksum, created_at
    ) VALUES (
        p_reference_number, v_next_revision_number, v_title, p_blueprint_course_content, 3,
        p_blueprint_content_checksum, v_occurred_at
    );
    INSERT INTO ple_data.blueprint_publication_event (
        blueprint_publication_event_id, blueprint_course_reference_number,
        blueprint_revision_number, published_by_account_id, occurred_at
    ) VALUES (
        p_publication_event_id, p_reference_number, v_next_revision_number,
        v_account_id, v_occurred_at
    );
    INSERT INTO ple_data.blueprint_revision_availability_event (
        blueprint_revision_availability_event_id, blueprint_course_reference_number,
        blueprint_revision_number, recorded_by_account_id, event_kind, occurred_at
    ) VALUES (
        p_availability_event_id, p_reference_number, v_next_revision_number,
        v_account_id, 'available', v_occurred_at
    );
END
$$;

DROP FUNCTION ple_api.load_live_demo_blueprint_course(bigint);
CREATE FUNCTION ple_api.load_live_demo_blueprint_course(p_reference_number bigint)
RETURNS TABLE (
    reference_number bigint,
    revision_number bigint,
    title text,
    blueprint_content_encoding_version smallint,
    blueprint_course_content jsonb,
    blueprint_content_checksum bytea,
    is_owner boolean
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT blueprint.reference_number,
           revision.blueprint_revision_number,
           revision.title,
           revision.blueprint_content_encoding_version,
           revision.blueprint_course_content,
           revision.blueprint_content_checksum,
           blueprint.blueprint_course_owner_account_id = ple_api.current_session_account_id()
      FROM ple_data.blueprint_course AS blueprint
      JOIN LATERAL (
          SELECT candidate.*
            FROM ple_data.blueprint_course_revision AS candidate
            JOIN ple_data.blueprint_publication_event AS publication
              ON publication.blueprint_course_reference_number
                    = candidate.blueprint_course_reference_number
             AND publication.blueprint_revision_number = candidate.blueprint_revision_number
           WHERE candidate.blueprint_course_reference_number = blueprint.reference_number
           ORDER BY candidate.blueprint_revision_number DESC
           LIMIT 1
      ) AS revision ON true
     WHERE p_reference_number BETWEEN 1 AND 2147483647
       AND blueprint.reference_number = p_reference_number
       AND ple_api.current_session_account_is_instructor()
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.create_live_demo_blueprint_course(
    uuid, uuid, uuid, jsonb, bytea
) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.replace_live_demo_blueprint_course(
    bigint, bigint, uuid, uuid, jsonb, bytea
) FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.load_live_demo_blueprint_course(bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.create_live_demo_blueprint_course(
    uuid, uuid, uuid, jsonb, bytea
) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.replace_live_demo_blueprint_course(
    bigint, bigint, uuid, uuid, jsonb, bytea
) TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.load_live_demo_blueprint_course(bigint) TO ple_app;

COMMENT ON FUNCTION ple_api.load_live_demo_blueprint_course(bigint) IS
    'Returns one answer-free current published Blueprint Course with its immutable content encoding version to an active Instructor.';

RESET ROLE;
