-- Immutable presentation/replay bindings.  Format-specific private payloads
-- remain below a Question Attempt so Unrelease can remove them with Student Work.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_attempt_presentation_binding (
    question_attempt_id uuid PRIMARY KEY REFERENCES ple_private.question_attempt(question_attempt_id) ON DELETE CASCADE,
    descriptor_version smallint NOT NULL CHECK (descriptor_version = 1),
    presentation_nonce text NOT NULL CHECK (presentation_nonce ~ '^[0-9a-f]{32,128}$'),
    presentation_checksum bytea NOT NULL CHECK (octet_length(presentation_checksum) = 32),
    presentation jsonb NOT NULL CHECK (jsonb_typeof(presentation) = 'object'),
    UNIQUE (question_attempt_id, presentation_nonce)
);

-- The public descriptor determines presentation references; this normalized
-- child retains only the durable response identity needed to interpret saved
-- Student Work after source content is gone.
CREATE TABLE ple_private.question_attempt_response_item_binding (
    question_attempt_id uuid NOT NULL REFERENCES ple_private.question_attempt_presentation_binding(question_attempt_id) ON DELETE CASCADE,
    presentation_response_item_reference text NOT NULL CHECK (presentation_response_item_reference ~ '^[0-9a-f]{4}$'),
    response_item_reference text NOT NULL CHECK (char_length(btrim(response_item_reference)) > 0),
    PRIMARY KEY (question_attempt_id, presentation_response_item_reference),
    UNIQUE (question_attempt_id, response_item_reference)
);

CREATE TABLE ple_private.question_attempt_presentation_asset_binding (
    question_attempt_id uuid PRIMARY KEY REFERENCES ple_private.question_attempt_presentation_binding(question_attempt_id) ON DELETE CASCADE
);

CREATE TABLE ple_private.question_attempt_presentation_asset_rendition (
    question_attempt_id uuid NOT NULL REFERENCES ple_private.question_attempt_presentation_asset_binding(question_attempt_id) ON DELETE CASCADE,
    asset_id uuid NOT NULL,
    question_asset_checksum bytea NOT NULL CHECK (octet_length(question_asset_checksum) = 32),
    rendition_checksum bytea NOT NULL CHECK (octet_length(rendition_checksum) = 32),
    intrinsic_width integer NOT NULL CHECK (intrinsic_width > 0),
    intrinsic_height integer NOT NULL CHECK (intrinsic_height > 0),
    PRIMARY KEY (question_attempt_id, asset_id)
);

CREATE TABLE ple_private.question_attempt_webwork_replay (
    question_attempt_id uuid PRIMARY KEY REFERENCES ple_private.question_attempt(question_attempt_id) ON DELETE CASCADE,
    replay_version smallint NOT NULL CHECK (replay_version = 1),
    replay_details jsonb NOT NULL CHECK (jsonb_typeof(replay_details) = 'object')
);

CREATE FUNCTION ple_private.reject_attempt_presentation_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Question Attempt presentation evidence is immutable'; END $$;

-- Delivery writes a complete reproduction bundle in its one transaction.  This
-- deferred check makes missing evidence fail closed before an Attempt can be
-- committed or finalized; no reader reconstructs a past presentation from
-- mutable Assignment or Question configuration.
CREATE FUNCTION ple_private.validate_question_attempt_reproduction()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
DECLARE question_attempt_id_value uuid := CASE WHEN TG_OP = 'DELETE' THEN OLD.question_attempt_id ELSE NEW.question_attempt_id END;
DECLARE capability text;
DECLARE has_presentation boolean;
DECLARE has_webwork_replay boolean;
BEGIN
    SELECT issued_capability INTO capability
      FROM ple_private.question_attempt
     WHERE question_attempt_id = question_attempt_id_value;
    IF NOT FOUND THEN RETURN NULL; END IF;
    SELECT EXISTS (
        SELECT 1 FROM ple_private.question_attempt_presentation_binding
         WHERE question_attempt_id = question_attempt_id_value
    ) INTO has_presentation;
    SELECT EXISTS (
        SELECT 1 FROM ple_private.question_attempt_webwork_replay
         WHERE question_attempt_id = question_attempt_id_value
    ) INTO has_webwork_replay;
    IF capability IN ('question_presentation', 'ple_question_json_presentation')
       AND (NOT has_presentation OR has_webwork_replay) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Attempt presentation capability requires one presentation bundle and no WeBWorK replay';
    END IF;
    IF capability = 'webwork_presentation'
       AND (NOT has_presentation OR NOT has_webwork_replay) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'WeBWorK Question Attempt requires presentation and replay evidence';
    END IF;
    IF capability = 'not_applicable' AND (has_presentation OR has_webwork_replay) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Non-presented Question Attempt has no presentation or replay evidence';
    END IF;
    RETURN NULL;
END $$;

CREATE TRIGGER question_attempt_presentation_binding_is_immutable BEFORE UPDATE ON ple_private.question_attempt_presentation_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_attempt_presentation_change();
CREATE TRIGGER question_attempt_presentation_binding_delete_is_guarded BEFORE DELETE ON ple_private.question_attempt_presentation_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER question_attempt_response_item_binding_is_immutable BEFORE UPDATE ON ple_private.question_attempt_response_item_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_attempt_presentation_change();
CREATE TRIGGER question_attempt_response_item_binding_delete_is_guarded BEFORE DELETE ON ple_private.question_attempt_response_item_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER question_attempt_presentation_asset_binding_is_immutable BEFORE UPDATE ON ple_private.question_attempt_presentation_asset_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_attempt_presentation_change();
CREATE TRIGGER question_attempt_presentation_asset_binding_delete_is_guarded BEFORE DELETE ON ple_private.question_attempt_presentation_asset_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER question_attempt_presentation_asset_rendition_is_immutable BEFORE UPDATE ON ple_private.question_attempt_presentation_asset_rendition
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_attempt_presentation_change();
CREATE TRIGGER question_attempt_presentation_asset_rendition_delete_is_guarded BEFORE DELETE ON ple_private.question_attempt_presentation_asset_rendition
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER question_attempt_webwork_replay_is_immutable BEFORE UPDATE ON ple_private.question_attempt_webwork_replay
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_attempt_presentation_change();
CREATE TRIGGER question_attempt_webwork_replay_delete_is_guarded BEFORE DELETE ON ple_private.question_attempt_webwork_replay
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE CONSTRAINT TRIGGER question_attempt_reproduction_is_complete AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_attempt
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_attempt_reproduction();
CREATE CONSTRAINT TRIGGER presentation_binding_matches_capability AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_attempt_presentation_binding
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_attempt_reproduction();
CREATE CONSTRAINT TRIGGER webwork_replay_matches_capability AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_attempt_webwork_replay
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_attempt_reproduction();

ALTER TABLE ple_private.question_attempt_presentation_binding ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_presentation_binding FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_response_item_binding ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_response_item_binding FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_presentation_asset_binding ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_presentation_asset_binding FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_presentation_asset_rendition ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_presentation_asset_rendition FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_webwork_replay ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_webwork_replay FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.question_attempt_presentation_binding,
    ple_private.question_attempt_response_item_binding,
    ple_private.question_attempt_presentation_asset_binding,
    ple_private.question_attempt_presentation_asset_rendition,
    ple_private.question_attempt_webwork_replay FROM PUBLIC;
CREATE POLICY question_attempt_presentation_binding_private_owner_access ON ple_private.question_attempt_presentation_binding FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_attempt_response_item_binding_private_owner_access ON ple_private.question_attempt_response_item_binding FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_attempt_presentation_asset_binding_private_owner_access ON ple_private.question_attempt_presentation_asset_binding FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_attempt_presentation_asset_rendition_private_owner_access ON ple_private.question_attempt_presentation_asset_rendition FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_attempt_webwork_replay_private_owner_access ON ple_private.question_attempt_webwork_replay FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
REVOKE ALL ON FUNCTION ple_private.reject_attempt_presentation_change(),
    ple_private.validate_question_attempt_reproduction() FROM PUBLIC;

-- The renderer is outside PostgreSQL, but it receives only immutable Issued
-- Question facts through this authenticated seam.  The commit routine writes
-- the renderer result once, below that exact issue.  ASVS 2.3.1/2.3.3 and
-- 8.2.1/8.2.2: ownership, ordering, and the Assignment-first lock are
-- enforced here rather than reconstructed in a caller.
CREATE FUNCTION ple_private.require_owned_attempt_for_presentation(
    p_assignment_attempt_id uuid
) RETURNS ple_private.assignment_attempt LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE result ple_private.assignment_attempt%ROWTYPE;
DECLARE course_id_value uuid;
BEGIN
    IF p_assignment_attempt_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assignment Attempt presentation identity is invalid';
    END IF;
    SELECT attempt.* INTO result
      FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.assignment_attempt_id = p_assignment_attempt_id
       AND attempt.completed_at IS NULL;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment Attempt presentation is unavailable';
    END IF;
    SELECT assignment.course_id INTO course_id_value
      FROM ple_data.assignment AS assignment
     WHERE assignment.assignment_id = result.assignment_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_owns_student_record(
        course_id_value, result.student_record_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment Attempt presentation is unavailable';
    END IF;
    -- This is deliberately the first lock, matching all Student Work writes
    -- and guarded Unrelease.  A concurrent Unrelease therefore wins cleanly.
    PERFORM 1 FROM ple_data.assignment
     WHERE assignment_id = result.assignment_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment Attempt presentation is unavailable';
    END IF;
    RETURN result;
END $$;

CREATE FUNCTION ple_private.prepare_student_assignment_attempt_presentation(
    p_assignment_attempt_id uuid
) RETURNS TABLE (
    assignment_attempt_id uuid, issued_question_id uuid, issued_position integer,
    assignment_entry_id uuid, assignment_content_entry_index integer,
    question_id text, revision_number integer, question_seed numeric, backend text,
    issued_capability text, source_object_id uuid, source_object_address jsonb,
    source_object_checksum text, webwork_pg_path text,
    question_attempt_id uuid, presentation_nonce text, presentation_checksum text,
    presentation jsonb, replay_details jsonb, question_asset_renditions jsonb
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_owned_attempt_for_presentation(p_assignment_attempt_id);
    RETURN QUERY
    SELECT issued.assignment_attempt_id, issued.issued_question_id, issued.issued_position,
           issued.assignment_entry_id, issued.assignment_content_entry_index,
           issued.question_id, issued.revision_number, issued.question_seed, source.backend,
           CASE source.backend WHEN 'ple' THEN 'ple_question_json_presentation'
                               WHEN 'webwork' THEN 'webwork_presentation' END,
           source.source_object_id, object_record.object_address,
           source.source_object_checksum, source.webwork_pg_path,
           attempt.question_attempt_id, binding.presentation_nonce,
           CASE WHEN binding.question_attempt_id IS NULL THEN NULL
                ELSE encode(binding.presentation_checksum, 'hex') END,
           binding.presentation,
           replay.replay_details,
           COALESCE(jsonb_agg(jsonb_build_object(
               'asset_id', rendition.asset_id,
               'question_asset_checksum', encode(rendition.question_asset_checksum, 'hex'),
               'rendition_checksum', encode(rendition.rendition_checksum, 'hex'),
               'intrinsic_width', rendition.intrinsic_width,
               'intrinsic_height', rendition.intrinsic_height
           ) ORDER BY rendition.asset_id) FILTER (WHERE rendition.asset_id IS NOT NULL), '[]'::jsonb)
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_revision_source_binding AS source
        ON source.question_id = issued.question_id
       AND source.revision_number = issued.revision_number
      JOIN ple_private.object_record AS object_record
        ON object_record.object_id = source.source_object_id
      LEFT JOIN ple_private.question_attempt AS attempt
        ON attempt.issued_question_id = issued.issued_question_id
      LEFT JOIN ple_private.question_attempt_presentation_binding AS binding
        ON binding.question_attempt_id = attempt.question_attempt_id
      LEFT JOIN ple_private.question_attempt_webwork_replay AS replay
        ON replay.question_attempt_id = attempt.question_attempt_id
      LEFT JOIN ple_private.question_attempt_presentation_asset_rendition AS rendition
        ON rendition.question_attempt_id = attempt.question_attempt_id
     WHERE issued.assignment_attempt_id = p_assignment_attempt_id
       AND source.backend IN ('ple', 'webwork')
     GROUP BY issued.assignment_attempt_id, issued.issued_question_id, issued.issued_position,
              issued.assignment_entry_id, issued.assignment_content_entry_index,
              issued.question_id, issued.revision_number, issued.question_seed, source.backend,
              source.source_object_id, object_record.object_address, source.source_object_checksum,
              source.webwork_pg_path, attempt.question_attempt_id, binding.presentation_nonce,
              binding.question_attempt_id, binding.presentation_checksum, binding.presentation,
              replay.replay_details
     ORDER BY issued.issued_position;
END $$;

CREATE FUNCTION ple_private.commit_student_assignment_attempt_presentation(
    p_assignment_attempt_id uuid, p_presentations jsonb
) RETURNS TABLE (
    issued_question_id uuid, issued_position integer, question_attempt_id uuid,
    presentation_nonce text, presentation_checksum text, resumed boolean
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE attempt_row ple_private.assignment_attempt%ROWTYPE;
DECLARE expected_count integer;
DECLARE supplied_count integer;
DECLARE existing_count integer;
DECLARE item jsonb;
DECLARE issued_row ple_private.issued_question%ROWTYPE;
DECLARE source_row ple_private.question_revision_source_binding%ROWTYPE;
DECLARE item_question_attempt_id uuid;
DECLARE item_issued_question_id uuid;
DECLARE item_parameter_hash text;
DECLARE item_renderer_name text;
DECLARE item_renderer_version text;
DECLARE item_backend_version text;
DECLARE item_grader_name text;
DECLARE item_grader_version text;
DECLARE item_rendered_hash text;
DECLARE item_capability text;
DECLARE item_nonce text;
DECLARE item_checksum text;
DECLARE item_presentation jsonb;
DECLARE item_replay jsonb;
DECLARE item_response_item_bindings jsonb;
BEGIN
    IF jsonb_typeof(p_presentations) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question presentation commit requires an array';
    END IF;
    attempt_row := ple_private.require_owned_attempt_for_presentation(p_assignment_attempt_id);
    SELECT count(*) INTO expected_count
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_revision_source_binding AS source
        ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
       AND source.backend IN ('ple', 'webwork');
    SELECT count(*) INTO existing_count
      FROM ple_private.question_attempt AS question_attempt
      JOIN ple_private.issued_question AS issued ON issued.issued_question_id = question_attempt.issued_question_id
      JOIN ple_private.question_revision_source_binding AS source
        ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
       AND source.backend IN ('ple', 'webwork');
    IF existing_count <> 0 THEN
        IF existing_count <> expected_count OR EXISTS (
            SELECT 1 FROM ple_private.issued_question AS issued
            JOIN ple_private.question_revision_source_binding AS source
              ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
            LEFT JOIN ple_private.question_attempt AS question_attempt
              ON question_attempt.issued_question_id = issued.issued_question_id
            LEFT JOIN ple_private.question_attempt_presentation_binding AS binding
              ON binding.question_attempt_id = question_attempt.question_attempt_id
            LEFT JOIN ple_private.question_attempt_webwork_replay AS replay
              ON replay.question_attempt_id = question_attempt.question_attempt_id
            WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id
              AND source.backend IN ('ple', 'webwork')
              AND (binding.question_attempt_id IS NULL
                OR (source.backend = 'webwork' AND replay.question_attempt_id IS NULL)
                OR (source.backend = 'ple' AND replay.question_attempt_id IS NOT NULL))
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Assignment Attempt presentation evidence is incomplete';
        END IF;
        RETURN QUERY
        SELECT issued.issued_question_id, issued.issued_position, question_attempt.question_attempt_id,
               binding.presentation_nonce, encode(binding.presentation_checksum, 'hex'), true
          FROM ple_private.issued_question AS issued
          JOIN ple_private.question_revision_source_binding AS source
            ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
          JOIN ple_private.question_attempt AS question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
          JOIN ple_private.question_attempt_presentation_binding AS binding ON binding.question_attempt_id = question_attempt.question_attempt_id
         WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id AND source.backend IN ('ple', 'webwork')
         ORDER BY issued.issued_position;
        RETURN;
    END IF;
    SELECT count(*) INTO supplied_count FROM jsonb_array_elements(p_presentations);
    IF supplied_count <> expected_count OR EXISTS (
        SELECT 1 FROM jsonb_to_recordset(p_presentations) AS supplied(issued_question_id uuid)
         GROUP BY supplied.issued_question_id HAVING count(*) <> 1
    ) OR EXISTS (
        SELECT issued.issued_question_id FROM ple_private.issued_question AS issued
        JOIN ple_private.question_revision_source_binding AS source
          ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
        WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id AND source.backend IN ('ple', 'webwork')
        EXCEPT
        SELECT supplied.issued_question_id FROM jsonb_to_recordset(p_presentations) AS supplied(issued_question_id uuid)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question presentations must cover each issued native Question exactly once';
    END IF;
    FOR item IN SELECT value FROM jsonb_array_elements(p_presentations) LOOP
        item_question_attempt_id := (item ->> 'question_attempt_id')::uuid;
        item_issued_question_id := (item ->> 'issued_question_id')::uuid;
        item_parameter_hash := item ->> 'generated_parameter_sha256';
        item_backend_version := item ->> 'backend_version';
        item_renderer_name := item ->> 'renderer_name';
        item_renderer_version := item ->> 'renderer_version';
        item_grader_name := item ->> 'grader_name';
        item_grader_version := item ->> 'grader_version';
        item_rendered_hash := item ->> 'rendered_question_sha256';
        item_capability := item ->> 'issued_capability';
        item_nonce := item ->> 'presentation_nonce';
        item_checksum := item ->> 'presentation_checksum';
        item_presentation := item -> 'presentation';
        item_replay := item -> 'webwork_replay';
        item_response_item_bindings := item -> 'response_item_bindings';
        SELECT issued.* INTO issued_row FROM ple_private.issued_question AS issued
         WHERE issued.issued_question_id = item_issued_question_id
           AND issued.assignment_attempt_id = attempt_row.assignment_attempt_id;
        SELECT source.* INTO source_row FROM ple_private.question_revision_source_binding AS source
         WHERE source.question_id = issued_row.question_id AND source.revision_number = issued_row.revision_number;
        IF NOT FOUND OR item_question_attempt_id IS NULL OR item_parameter_hash !~ '^[0-9a-f]{64}$'
           OR item_backend_version IS NULL OR char_length(btrim(item_backend_version)) NOT BETWEEN 1 AND 100
           OR (item_renderer_name IS NULL) <> (item_renderer_version IS NULL)
           OR (item_renderer_name IS NOT NULL
               AND char_length(btrim(item_renderer_name)) NOT BETWEEN 1 AND 100)
           OR (item_renderer_version IS NOT NULL
               AND char_length(btrim(item_renderer_version)) NOT BETWEEN 1 AND 100)
           OR item_grader_name IS NULL OR char_length(btrim(item_grader_name)) NOT BETWEEN 1 AND 100
           OR item_grader_version IS NULL OR char_length(btrim(item_grader_version)) NOT BETWEEN 1 AND 100
           OR item_rendered_hash !~ '^[0-9a-f]{64}$' OR item_nonce !~ '^[0-9a-f]{32,128}$'
           OR item_checksum !~ '^[0-9a-f]{64}$' OR jsonb_typeof(item_presentation) <> 'object'
           OR jsonb_typeof(item_response_item_bindings) <> 'array'
           OR (source_row.backend = 'ple' AND item_capability NOT IN ('question_presentation', 'ple_question_json_presentation'))
           OR (source_row.backend = 'webwork' AND item_capability <> 'webwork_presentation')
           OR (source_row.backend = 'ple' AND item_replay IS NOT NULL)
           OR (source_row.backend = 'webwork' AND jsonb_typeof(item_replay) <> 'object') THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question presentation evidence is invalid';
        END IF;
        IF EXISTS (
            SELECT 1
              FROM jsonb_array_elements(item_response_item_bindings) AS supplied(value)
             WHERE jsonb_typeof(supplied.value) <> 'object'
                OR NOT (supplied.value ? 'presentation_response_item_reference'
                    AND supplied.value ? 'response_item_reference')
                OR EXISTS (
                    SELECT 1 FROM jsonb_object_keys(
                        CASE WHEN jsonb_typeof(supplied.value) = 'object'
                             THEN supplied.value ELSE '{}'::jsonb END
                    ) AS key
                     WHERE key NOT IN ('presentation_response_item_reference', 'response_item_reference')
                )
        ) OR EXISTS (
            SELECT 1
              FROM jsonb_to_recordset(item_response_item_bindings) AS supplied(
                  presentation_response_item_reference text, response_item_reference text
              )
             WHERE supplied.presentation_response_item_reference !~ '^[0-9a-f]{4}$'
                OR char_length(btrim(supplied.response_item_reference)) = 0
        ) OR EXISTS (
            SELECT 1
              FROM jsonb_to_recordset(item_response_item_bindings) AS supplied(
                  presentation_response_item_reference text, response_item_reference text
              )
             GROUP BY supplied.presentation_response_item_reference HAVING count(*) <> 1
        ) OR EXISTS (
            SELECT 1
              FROM jsonb_to_recordset(item_response_item_bindings) AS supplied(
                  presentation_response_item_reference text, response_item_reference text
              )
             GROUP BY supplied.response_item_reference HAVING count(*) <> 1
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question presentation response-item bindings are invalid';
        END IF;
        INSERT INTO ple_private.question_attempt(
            question_attempt_id, issued_question_id, question_seed, generated_parameter_sha256,
            issued_at, deadline_at, question_attempt_state, backend_name, backend_version,
            renderer_name, renderer_version, source_object_id, source_object_checksum,
            grader_name, grader_version, rendered_question_sha256, issued_capability
        ) VALUES (
            item_question_attempt_id, issued_row.issued_question_id, issued_row.question_seed,
            item_parameter_hash, clock_timestamp(),
            CASE WHEN issued_row.question_attempt_time_limit_seconds IS NULL THEN NULL
                 ELSE clock_timestamp() + make_interval(secs => issued_row.question_attempt_time_limit_seconds + issued_row.question_attempt_grace_seconds) END,
            'open', source_row.backend, item_backend_version, item_renderer_name, item_renderer_version,
            source_row.source_object_id, decode(source_row.source_object_checksum, 'hex'),
            item_grader_name, item_grader_version, decode(item_rendered_hash, 'hex'), item_capability
        );
        INSERT INTO ple_private.question_attempt_presentation_binding(
            question_attempt_id, descriptor_version, presentation_nonce, presentation_checksum, presentation
        ) VALUES (item_question_attempt_id, 1, item_nonce, decode(item_checksum, 'hex'), item_presentation);
        INSERT INTO ple_private.question_attempt_response_item_binding(
            question_attempt_id, presentation_response_item_reference, response_item_reference
        )
        SELECT item_question_attempt_id, supplied.presentation_response_item_reference,
               supplied.response_item_reference
          FROM jsonb_to_recordset(item_response_item_bindings) AS supplied(
              presentation_response_item_reference text, response_item_reference text
          );
        IF item ? 'question_assets' AND jsonb_typeof(item -> 'question_assets') <> 'array' THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question presentation assets are invalid';
        END IF;
        IF jsonb_array_length(COALESCE(item -> 'question_assets', '[]'::jsonb)) > 0 THEN
            IF EXISTS (
                SELECT 1 FROM jsonb_to_recordset(item -> 'question_assets') AS supplied(asset_id uuid, question_asset_checksum text, rendition_checksum text, intrinsic_width integer, intrinsic_height integer)
                 WHERE supplied.asset_id IS NULL OR supplied.question_asset_checksum !~ '^[0-9a-f]{64}$' OR supplied.rendition_checksum !~ '^[0-9a-f]{64}$'
                    OR supplied.intrinsic_width IS NULL OR supplied.intrinsic_width <= 0 OR supplied.intrinsic_height IS NULL OR supplied.intrinsic_height <= 0
                    OR NOT EXISTS (
                        SELECT 1 FROM ple_private.question_asset_publication AS publication
                         WHERE publication.question_id = issued_row.question_id
                           AND publication.revision_number = issued_row.revision_number
                           AND publication.asset_id = supplied.asset_id
                           AND publication.publication_state = 'ready'
                           AND encode(publication.source_object_checksum, 'hex') = supplied.question_asset_checksum
                           AND encode(publication.public_object_checksum, 'hex') = supplied.rendition_checksum
                    )
            ) OR EXISTS (
                SELECT 1 FROM jsonb_to_recordset(item -> 'question_assets') AS supplied(asset_id uuid)
                 GROUP BY supplied.asset_id HAVING count(*) <> 1
            ) THEN
                RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question presentation assets are invalid';
            END IF;
            INSERT INTO ple_private.question_attempt_presentation_asset_binding(question_attempt_id)
            VALUES (item_question_attempt_id);
            INSERT INTO ple_private.question_attempt_presentation_asset_rendition(question_attempt_id, asset_id, question_asset_checksum, rendition_checksum, intrinsic_width, intrinsic_height)
            SELECT item_question_attempt_id, supplied.asset_id, decode(supplied.question_asset_checksum, 'hex'), decode(supplied.rendition_checksum, 'hex'), supplied.intrinsic_width, supplied.intrinsic_height
              FROM jsonb_to_recordset(item -> 'question_assets') AS supplied(asset_id uuid, question_asset_checksum text, rendition_checksum text, intrinsic_width integer, intrinsic_height integer);
        END IF;
        IF source_row.backend = 'webwork' THEN
            INSERT INTO ple_private.question_attempt_webwork_replay(question_attempt_id, replay_version, replay_details)
            VALUES (item_question_attempt_id, 1, item_replay);
        END IF;
    END LOOP;
    SET CONSTRAINTS ALL IMMEDIATE;
    RETURN QUERY
    SELECT issued.issued_question_id, issued.issued_position, question_attempt.question_attempt_id,
           binding.presentation_nonce, encode(binding.presentation_checksum, 'hex'), false
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_revision_source_binding AS source
        ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
      JOIN ple_private.question_attempt AS question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_attempt_presentation_binding AS binding ON binding.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assignment_attempt_id = attempt_row.assignment_attempt_id AND source.backend IN ('ple', 'webwork')
     ORDER BY issued.issued_position;
END $$;

-- Server-only read after a presentation has been committed.  It reads the
-- retained bundle, never a mutable Assignment Entry or Question configuration.
CREATE FUNCTION ple_private.read_student_assignment_attempt_presentation_evidence(
    p_assignment_attempt_reference_number bigint, p_issued_position integer
) RETURNS TABLE (
    question_id text, revision_number integer, question_seed numeric,
    presentation_nonce text, presentation_checksum text, presentation jsonb,
    question_asset_renditions jsonb, response_item_bindings jsonb
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE assignment_attempt_id_value uuid;
BEGIN
    IF p_assignment_attempt_reference_number IS NULL OR p_issued_position < 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Issued Question position is invalid';
    END IF;
    SELECT assignment_attempt_id INTO assignment_attempt_id_value
      FROM ple_private.assignment_attempt
     WHERE reference_number = p_assignment_attempt_reference_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assignment Attempt presentation is unavailable';
    END IF;
    PERFORM ple_private.require_owned_attempt_for_presentation(assignment_attempt_id_value);
    RETURN QUERY
    SELECT issued.question_id, issued.revision_number, question_attempt.question_seed,
           binding.presentation_nonce,
           encode(binding.presentation_checksum, 'hex'),
           binding.presentation,
           COALESCE(jsonb_agg(jsonb_build_object(
               'asset_id', rendition.asset_id,
               'question_asset_checksum', encode(rendition.question_asset_checksum, 'hex'),
               'rendition_checksum', encode(rendition.rendition_checksum, 'hex'),
               'intrinsic_width', rendition.intrinsic_width,
               'intrinsic_height', rendition.intrinsic_height
           ) ORDER BY rendition.asset_id) FILTER (WHERE rendition.asset_id IS NOT NULL), '[]'::jsonb)
           , COALESCE(response_item_bindings.response_item_bindings, '[]'::jsonb)
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_attempt_presentation_binding AS binding
        ON binding.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.question_attempt_presentation_asset_rendition AS rendition
        ON rendition.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN LATERAL (
          SELECT jsonb_agg(jsonb_build_object(
              'presentation_response_item_reference', response_item.presentation_response_item_reference,
              'response_item_reference', response_item.response_item_reference
          ) ORDER BY response_item.presentation_response_item_reference) AS response_item_bindings
            FROM ple_private.question_attempt_response_item_binding AS response_item
           WHERE response_item.question_attempt_id = question_attempt.question_attempt_id
      ) AS response_item_bindings ON true
     WHERE issued.assignment_attempt_id = assignment_attempt_id_value
       AND issued.issued_position = p_issued_position
     GROUP BY issued.question_id, issued.revision_number, question_attempt.question_seed,
              binding.presentation_nonce, binding.presentation_checksum, binding.presentation,
              response_item_bindings.response_item_bindings;
END $$;

CREATE FUNCTION ple_private.read_student_assignment_attempt_presentation_evidence_set(
    p_assignment_attempt_id uuid
) RETURNS TABLE (
    assignment_entry_id uuid, issued_position integer, question_id text, revision_number integer,
    question_seed numeric, presentation_nonce text, presentation_checksum text, presentation jsonb,
    question_asset_renditions jsonb, response_item_bindings jsonb
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE expected_count integer;
DECLARE attempt_count integer;
DECLARE complete_count integer;
BEGIN
    PERFORM ple_private.require_owned_attempt_for_presentation(p_assignment_attempt_id);
    SELECT count(*) INTO expected_count
     FROM ple_private.issued_question AS issued
      JOIN ple_private.question_revision_source_binding AS source
        ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
     WHERE issued.assignment_attempt_id = p_assignment_attempt_id
       AND source.backend IN ('ple', 'webwork');
    SELECT count(*) INTO attempt_count
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_revision_source_binding AS source
        ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
     WHERE issued.assignment_attempt_id = p_assignment_attempt_id
       AND source.backend IN ('ple', 'webwork');
    SELECT count(*) INTO complete_count
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_revision_source_binding AS source
        ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
      JOIN ple_private.question_attempt_presentation_binding AS binding
        ON binding.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.question_attempt_webwork_replay AS replay
        ON replay.question_attempt_id = question_attempt.question_attempt_id
     WHERE issued.assignment_attempt_id = p_assignment_attempt_id
       AND source.backend IN ('ple', 'webwork')
       AND ((source.backend = 'ple' AND replay.question_attempt_id IS NULL)
         OR (source.backend = 'webwork' AND replay.question_attempt_id IS NOT NULL));
    IF attempt_count <> 0 AND (attempt_count <> expected_count OR complete_count <> expected_count) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Assignment Attempt presentation evidence is incomplete';
    END IF;
    RETURN QUERY
    SELECT issued.assignment_entry_id, issued.issued_position, issued.question_id, issued.revision_number,
           question_attempt.question_seed, binding.presentation_nonce, encode(binding.presentation_checksum, 'hex'), binding.presentation,
           COALESCE(jsonb_agg(jsonb_build_object('asset_id', rendition.asset_id, 'question_asset_checksum', encode(rendition.question_asset_checksum, 'hex'), 'rendition_checksum', encode(rendition.rendition_checksum, 'hex'), 'intrinsic_width', rendition.intrinsic_width, 'intrinsic_height', rendition.intrinsic_height) ORDER BY rendition.asset_id) FILTER (WHERE rendition.asset_id IS NOT NULL), '[]'::jsonb)
           , COALESCE(response_item_bindings.response_item_bindings, '[]'::jsonb)
      FROM ple_private.issued_question AS issued
      JOIN ple_private.question_attempt AS question_attempt ON question_attempt.issued_question_id = issued.issued_question_id
      JOIN ple_private.question_revision_source_binding AS source ON source.question_id = issued.question_id AND source.revision_number = issued.revision_number
      JOIN ple_private.question_attempt_presentation_binding AS binding ON binding.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN ple_private.question_attempt_presentation_asset_rendition AS rendition ON rendition.question_attempt_id = question_attempt.question_attempt_id
      LEFT JOIN LATERAL (
          SELECT jsonb_agg(jsonb_build_object(
              'presentation_response_item_reference', response_item.presentation_response_item_reference,
              'response_item_reference', response_item.response_item_reference
          ) ORDER BY response_item.presentation_response_item_reference) AS response_item_bindings
            FROM ple_private.question_attempt_response_item_binding AS response_item
           WHERE response_item.question_attempt_id = question_attempt.question_attempt_id
      ) AS response_item_bindings ON true
     WHERE issued.assignment_attempt_id = p_assignment_attempt_id
       AND source.backend IN ('ple', 'webwork')
     GROUP BY issued.assignment_entry_id, issued.issued_position, issued.question_id, issued.revision_number, question_attempt.question_seed, binding.presentation_nonce, binding.presentation_checksum, binding.presentation,
              response_item_bindings.response_item_bindings
     ORDER BY issued.issued_position;
END $$;

REVOKE ALL ON FUNCTION ple_private.require_owned_attempt_for_presentation(uuid),
    ple_private.prepare_student_assignment_attempt_presentation(uuid),
    ple_private.commit_student_assignment_attempt_presentation(uuid, jsonb),
    ple_private.read_student_assignment_attempt_presentation_evidence(bigint, integer),
    ple_private.read_student_assignment_attempt_presentation_evidence_set(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.prepare_student_assignment_attempt_presentation(uuid),
    ple_private.commit_student_assignment_attempt_presentation(uuid, jsonb),
    ple_private.read_student_assignment_attempt_presentation_evidence(bigint, integer),
    ple_private.read_student_assignment_attempt_presentation_evidence_set(uuid) TO ple_api_owner;

COMMENT ON TABLE ple_private.question_attempt_presentation_binding IS 'Checksummed issued presentation retained for one Question Attempt.';
COMMENT ON TABLE ple_private.question_attempt_webwork_replay IS 'Exact private WeBWorK replay evidence retained for one Question Attempt.';

RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.prepare_student_assignment_attempt_presentation(uuid)
RETURNS TABLE (
    assignment_attempt_id uuid, issued_question_id uuid, issued_position integer,
    assignment_entry_id uuid, assignment_content_entry_index integer,
    question_id text, revision_number integer, question_seed numeric, backend text,
    issued_capability text, source_object_id uuid, source_object_address jsonb,
    source_object_checksum text, webwork_pg_path text,
    question_attempt_id uuid, presentation_nonce text, presentation_checksum text,
    presentation jsonb, replay_details jsonb, question_asset_renditions jsonb
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT assignment_attempt_id, issued_question_id, issued_position + 1,
           assignment_entry_id, assignment_content_entry_index,
           question_id, revision_number, question_seed, backend,
           issued_capability, source_object_id, source_object_address,
           source_object_checksum, webwork_pg_path,
           question_attempt_id, presentation_nonce, presentation_checksum,
           presentation, replay_details, question_asset_renditions
      FROM ple_private.prepare_student_assignment_attempt_presentation($1)
$$;
CREATE FUNCTION ple_api.commit_student_assignment_attempt_presentation(uuid, jsonb)
RETURNS TABLE (
    issued_question_id uuid, issued_position integer, question_attempt_id uuid,
    presentation_nonce text, presentation_checksum text, resumed boolean
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT issued_question_id, issued_position + 1, question_attempt_id,
           presentation_nonce, presentation_checksum, resumed
      FROM ple_private.commit_student_assignment_attempt_presentation($1, $2)
$$;
CREATE FUNCTION ple_api.read_student_assignment_attempt_presentation_evidence(bigint, integer)
RETURNS TABLE (
    question_id text, revision_number integer, question_seed numeric,
    presentation_nonce text, presentation_checksum text, presentation jsonb,
    question_asset_renditions jsonb, response_item_bindings jsonb
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT question_id, revision_number, question_seed,
           presentation_nonce, presentation_checksum, presentation,
           question_asset_renditions, response_item_bindings
      FROM ple_private.read_student_assignment_attempt_presentation_evidence($1, $2 - 1)
$$;
CREATE FUNCTION ple_api.read_student_assignment_attempt_presentation_evidence_set(uuid)
RETURNS TABLE (assignment_entry_id uuid, issued_position integer, question_id text, revision_number integer, question_seed numeric, presentation_nonce text, presentation_checksum text, presentation jsonb, question_asset_renditions jsonb, response_item_bindings jsonb)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_private, ple_api AS $$
    SELECT assignment_entry_id, issued_position + 1, question_id, revision_number, question_seed, presentation_nonce, presentation_checksum, presentation, question_asset_renditions, response_item_bindings
      FROM ple_private.read_student_assignment_attempt_presentation_evidence_set($1)
$$;
REVOKE ALL ON FUNCTION ple_api.prepare_student_assignment_attempt_presentation(uuid),
    ple_api.commit_student_assignment_attempt_presentation(uuid, jsonb),
    ple_api.read_student_assignment_attempt_presentation_evidence(bigint, integer),
    ple_api.read_student_assignment_attempt_presentation_evidence_set(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.prepare_student_assignment_attempt_presentation(uuid),
    ple_api.commit_student_assignment_attempt_presentation(uuid, jsonb),
    ple_api.read_student_assignment_attempt_presentation_evidence(bigint, integer),
    ple_api.read_student_assignment_attempt_presentation_evidence_set(uuid) TO ple_app;
RESET ROLE;
