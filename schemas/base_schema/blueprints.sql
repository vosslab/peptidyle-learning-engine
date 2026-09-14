-- Reusable Blueprint Course lineages and immutable save-created Revisions.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner, ple_api_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;

CREATE TABLE ple_data.blueprint_course (
    blueprint_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE NOT NULL,
    owner_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    short_name text NOT NULL CHECK (char_length(btrim(short_name)) BETWEEN 1 AND 500),
    long_name text NOT NULL CHECK (char_length(btrim(long_name)) BETWEEN 1 AND 500),
    availability text NOT NULL DEFAULT 'available'
        CHECK (availability IN ('available', 'archived')),
    metadata_etag uuid NOT NULL,
    current_blueprint_revision_number bigint NOT NULL DEFAULT 1
        CHECK (current_blueprint_revision_number > 0),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT blueprint_course_reference_is_bounded CHECK (
        reference_number BETWEEN 1 AND 2147483647
    )
);

CREATE TABLE ple_data.blueprint_course_revision (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    blueprint_revision_number bigint NOT NULL CHECK (blueprint_revision_number > 0),
    content jsonb NOT NULL CHECK (jsonb_typeof(content) = 'object'),
    content_checksum bytea NOT NULL CHECK (octet_length(content_checksum) = 32),
    saved_at timestamp with time zone NOT NULL,
    PRIMARY KEY (blueprint_course_reference_number, blueprint_revision_number)
);

ALTER TABLE ple_data.blueprint_course
    ADD CONSTRAINT blueprint_course_current_revision_fk
    FOREIGN KEY (reference_number, current_blueprint_revision_number)
    REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number
    ) DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE ple_data.blueprint_revision_question_pin (
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    content_path text NOT NULL CHECK (char_length(content_path) BETWEEN 1 AND 500),
    question_id text NOT NULL,
    question_revision_number bigint NOT NULL CHECK (question_revision_number > 0),
    PRIMARY KEY (
        blueprint_course_reference_number, blueprint_revision_number, content_path
    ),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number),
    FOREIGN KEY (question_id, question_revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);

-- Module and Assignment References are durable identities across Revisions.
-- Their positions and Module membership belong to each immutable Revision.
CREATE TABLE ple_data.blueprint_revision_module (
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    blueprint_module_reference uuid NOT NULL,
    module_position integer NOT NULL CHECK (module_position BETWEEN 1 AND 1024),
    PRIMARY KEY (
        blueprint_course_reference_number, blueprint_revision_number,
        blueprint_module_reference
    ),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number),
    UNIQUE (blueprint_course_reference_number, blueprint_revision_number, module_position)
);

CREATE TABLE ple_data.blueprint_revision_assignment (
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    blueprint_module_reference uuid NOT NULL,
    blueprint_assignment_reference uuid NOT NULL,
    assignment_position integer NOT NULL CHECK (assignment_position BETWEEN 1 AND 1024),
    PRIMARY KEY (
        blueprint_course_reference_number, blueprint_revision_number,
        blueprint_assignment_reference
    ),
    FOREIGN KEY (
        blueprint_course_reference_number, blueprint_revision_number,
        blueprint_module_reference
    ) REFERENCES ple_data.blueprint_revision_module (
        blueprint_course_reference_number, blueprint_revision_number,
        blueprint_module_reference
    ),
    UNIQUE (
        blueprint_course_reference_number, blueprint_revision_number,
        blueprint_module_reference, assignment_position
    )
);

CREATE TABLE ple_data.blueprint_revision_event (
    blueprint_revision_event_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    occurred_at timestamp with time zone NOT NULL,
    UNIQUE (blueprint_course_reference_number, blueprint_revision_number),
    UNIQUE (blueprint_course_reference_number, actor_account_id, request_checksum),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number)
);

-- A complete immutable Revision has exactly one immutable receipt event.  The
-- circular deferred foreign keys let one transaction insert the Revision,
-- children, and its event in that order without an RLS-sensitive trigger.
ALTER TABLE ple_data.blueprint_course_revision
    ADD CONSTRAINT blueprint_revision_requires_event_fk
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
    REFERENCES ple_data.blueprint_revision_event (
        blueprint_course_reference_number, blueprint_revision_number
    ) DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE ple_data.blueprint_metadata_event (
    blueprint_metadata_event_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    short_name text NOT NULL,
    long_name text NOT NULL,
    availability text NOT NULL CHECK (availability IN ('available', 'archived')),
    metadata_etag uuid NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    UNIQUE (blueprint_course_reference_number, metadata_etag)
);

CREATE TABLE ple_data.blueprint_course_create_receipt (
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    metadata_etag uuid NOT NULL,
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (actor_account_id, request_checksum),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number)
);

CREATE TABLE ple_data.blueprint_course_save_receipt (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    resulting_blueprint_revision_number bigint NOT NULL CHECK (
        resulting_blueprint_revision_number > 0
    ),
    changed boolean NOT NULL,
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (blueprint_course_reference_number, actor_account_id, request_checksum)
);

CREATE INDEX blueprint_course_available_owner_idx
    ON ple_data.blueprint_course (availability, owner_account_id, reference_number);
CREATE INDEX blueprint_revision_question_pin_question_idx
    ON ple_data.blueprint_revision_question_pin (question_id, question_revision_number);

CREATE FUNCTION ple_data.blueprint_content_question_pins(p_content jsonb)
RETURNS TABLE (content_path text, question_id text, question_revision_number bigint)
LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    WITH entries AS (
        SELECT module_ordinality, assignment_ordinality, entry_ordinality, entry
          FROM pg_catalog.jsonb_array_elements(p_content -> 'modules')
                 WITH ORDINALITY AS module_row(module, module_ordinality)
          CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(module_row.module -> 'assignments')
                 WITH ORDINALITY AS assignment_row(assignment, assignment_ordinality)
          CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(
              assignment_row.assignment -> 'content' -> 'entries'
          ) WITH ORDINALITY AS entry_row(entry, entry_ordinality)
    ), pins AS (
        SELECT module_ordinality, assignment_ordinality, entry_ordinality, 0 AS pin_ordinality,
               entry -> 'question_revision' AS pin
          FROM entries WHERE entry ->> 'kind' = 'fixed'
        UNION ALL
        SELECT entries.module_ordinality, entries.assignment_ordinality,
               entries.entry_ordinality, pool.pin_ordinality, pool.pin
          FROM entries
          CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(
              entries.entry -> 'question_revisions'
          ) WITH ORDINALITY AS pool(pin, pin_ordinality)
         WHERE entries.entry ->> 'kind' = 'pool'
    )
    SELECT pg_catalog.format('m%s.a%s.e%s.p%s', module_ordinality,
               assignment_ordinality, entry_ordinality, pin_ordinality),
           pg_catalog.replace(pin ->> 'questionId', '-', ''),
           (pin ->> 'revisionNumber')::bigint
      FROM pins
$$;

CREATE FUNCTION ple_data.blueprint_content_assignments(p_content jsonb)
RETURNS TABLE (
    blueprint_module_reference uuid, blueprint_assignment_reference uuid,
    module_position integer, assignment_position integer
)
LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT (module_row.module ->> 'blueprint_module_reference')::uuid,
           (assignment_row.assignment ->> 'blueprint_assignment_reference')::uuid,
           module_row.module_ordinality::integer,
           assignment_row.assignment_ordinality::integer
      FROM pg_catalog.jsonb_array_elements(p_content -> 'modules')
             WITH ORDINALITY AS module_row(module, module_ordinality)
      CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(module_row.module -> 'assignments')
             WITH ORDINALITY AS assignment_row(assignment, assignment_ordinality)
$$;

CREATE FUNCTION ple_data.blueprint_content_modules(p_content jsonb)
RETURNS TABLE (blueprint_module_reference uuid, module_position integer)
LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT (module_row.module ->> 'blueprint_module_reference')::uuid,
           module_row.module_ordinality::integer
      FROM pg_catalog.jsonb_array_elements(p_content -> 'modules')
             WITH ORDINALITY AS module_row(module, module_ordinality)
$$;

CREATE FUNCTION ple_data.validate_blueprint_content(p_content jsonb)
RETURNS void LANGUAGE plpgsql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    IF jsonb_typeof(p_content) <> 'object'
       OR p_content ? 'title'
       OR jsonb_typeof(p_content -> 'modules') <> 'array'
       OR jsonb_array_length(p_content -> 'modules') NOT BETWEEN 1 AND 1024
       OR EXISTS (
           SELECT 1
             FROM jsonb_array_elements(p_content -> 'modules') AS module_row(module)
            WHERE jsonb_typeof(module_row.module) <> 'object'
               OR module_row.module ->> 'blueprint_module_reference' IS NULL
               OR module_row.module ->> 'label' IS NULL
               OR module_row.module ->> 'label' <> btrim(module_row.module ->> 'label')
               OR char_length(module_row.module ->> 'label') NOT BETWEEN 1 AND 500
               OR jsonb_typeof(module_row.module -> 'assignments') <> 'array'
               OR jsonb_array_length(module_row.module -> 'assignments') NOT BETWEEN 1 AND 1024
       )
       OR (SELECT count(*) FROM ple_data.blueprint_content_modules(p_content))
          <> (SELECT count(DISTINCT blueprint_module_reference)
                FROM ple_data.blueprint_content_modules(p_content))
       OR EXISTS (
           SELECT 1
             FROM jsonb_array_elements(p_content -> 'modules') AS module_row(module)
             CROSS JOIN LATERAL jsonb_array_elements(module_row.module -> 'assignments')
                 AS assignment_row(assignment)
            WHERE jsonb_typeof(assignment_row.assignment) <> 'object'
               OR assignment_row.assignment ->> 'blueprint_assignment_reference' IS NULL
               OR jsonb_typeof(assignment_row.assignment -> 'content') <> 'object'
               OR jsonb_typeof(assignment_row.assignment -> 'content' -> 'entries') <> 'array'
               OR jsonb_array_length(assignment_row.assignment -> 'content' -> 'entries') = 0
       )
       OR (SELECT count(*) FROM ple_data.blueprint_content_assignments(p_content))
          <> (SELECT count(DISTINCT blueprint_assignment_reference)
                FROM ple_data.blueprint_content_assignments(p_content))
       OR EXISTS (
           SELECT 1
             FROM jsonb_array_elements(p_content -> 'modules') AS module_row(module)
             CROSS JOIN LATERAL jsonb_array_elements(module_row.module -> 'assignments')
                 AS assignment_row(assignment)
             CROSS JOIN LATERAL jsonb_array_elements(
                 assignment_row.assignment -> 'content' -> 'entries'
             ) AS entry_row(entry)
            WHERE entry_row.entry ->> 'kind' IS NULL
               OR entry_row.entry ->> 'kind' NOT IN ('fixed', 'pool')
               OR (entry_row.entry ->> 'kind' = 'fixed'
                   AND jsonb_typeof(entry_row.entry -> 'question_revision') <> 'object')
               OR (entry_row.entry ->> 'kind' = 'pool'
                   AND (jsonb_typeof(entry_row.entry -> 'question_revisions') <> 'array'
                        OR jsonb_array_length(entry_row.entry -> 'question_revisions') = 0))
       )
       OR NOT EXISTS (SELECT 1 FROM ple_data.blueprint_content_question_pins(p_content)) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint Course content is invalid';
    END IF;
END
$$;

-- New pins must select an Available Published Question. A Save may retain an
-- exact pin already owned by its expected immutable head Revision.
CREATE FUNCTION ple_data.validate_blueprint_question_selection(
    p_existing_blueprint_course_reference_number bigint,
    p_existing_blueprint_revision_number bigint,
    p_content jsonb
) RETURNS void
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    PERFORM 1
      FROM ple_data.published_question AS question
      JOIN ple_data.blueprint_content_question_pins(p_content) AS pin
        ON pin.question_id = question.question_id
     FOR KEY SHARE OF question;
    IF EXISTS (
        SELECT 1
          FROM ple_data.blueprint_content_question_pins(p_content) AS pin
          LEFT JOIN ple_data.published_question AS question
            ON question.question_id = pin.question_id
          LEFT JOIN ple_data.question_revision AS revision
            ON revision.question_id = pin.question_id
           AND revision.revision_number = pin.question_revision_number
          LEFT JOIN ple_data.blueprint_revision_question_pin AS existing
            ON existing.blueprint_course_reference_number
                 = p_existing_blueprint_course_reference_number
           AND existing.blueprint_revision_number = p_existing_blueprint_revision_number
           AND existing.question_id = pin.question_id
           AND existing.question_revision_number = pin.question_revision_number
         WHERE (question.availability IS DISTINCT FROM 'available'
                OR revision.question_id IS NULL)
           AND existing.blueprint_course_reference_number IS NULL
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'New Blueprint Question pins require an Available Question';
    END IF;
END
$$;

CREATE FUNCTION ple_data.reject_blueprint_revision_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'a Blueprint Revision is immutable';
END
$$;

CREATE FUNCTION ple_data.reject_blueprint_event_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Blueprint lifecycle evidence is immutable';
END
$$;

-- The receipt event seals a complete child aggregate.  The creation and Save
-- transactions insert children first and the event last.  Child insertion
-- locks the parent Revision before looking for its event: a concurrent
-- transaction cannot miss an uncommitted seal and later append a child.
CREATE FUNCTION ple_data.reject_sealed_blueprint_revision_child_insert()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    PERFORM 1 FROM ple_data.blueprint_course_revision AS revision
     WHERE revision.blueprint_course_reference_number = NEW.blueprint_course_reference_number
       AND revision.blueprint_revision_number = NEW.blueprint_revision_number
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23503',
            MESSAGE = 'Blueprint Revision parent is unavailable for child insertion';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.blueprint_revision_event AS event
         WHERE event.blueprint_course_reference_number = NEW.blueprint_course_reference_number
           AND event.blueprint_revision_number = NEW.blueprint_revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'a Blueprint Revision is immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER blueprint_course_revision_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_course_revision
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_revision_question_pin_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_question_pin
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_revision_question_pin_is_sealed
BEFORE INSERT ON ple_data.blueprint_revision_question_pin
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_sealed_blueprint_revision_child_insert();
CREATE TRIGGER blueprint_revision_assignment_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_assignment
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_revision_assignment_is_sealed
BEFORE INSERT ON ple_data.blueprint_revision_assignment
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_sealed_blueprint_revision_child_insert();
CREATE TRIGGER blueprint_revision_module_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_module
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_revision_module_is_sealed
BEFORE INSERT ON ple_data.blueprint_revision_module
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_sealed_blueprint_revision_child_insert();
CREATE TRIGGER blueprint_revision_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_event_change();
CREATE TRIGGER blueprint_metadata_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_metadata_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_event_change();

ALTER TABLE ple_data.blueprint_course ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_revision FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_question_pin ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_question_pin FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_module ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_module FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_assignment ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_assignment FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_metadata_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_metadata_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_create_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_create_receipt FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_save_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_save_receipt FORCE ROW LEVEL SECURITY;

GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE
    ple_data.blueprint_course,
    ple_data.blueprint_course_revision,
    ple_data.blueprint_revision_question_pin,
    ple_data.blueprint_revision_module,
    ple_data.blueprint_revision_assignment,
    ple_data.blueprint_revision_event,
    ple_data.blueprint_metadata_event,
    ple_data.blueprint_course_create_receipt,
    ple_data.blueprint_course_save_receipt
TO ple_api_owner;
GRANT USAGE ON SEQUENCE ple_data.blueprint_course_reference_number_seq,
    ple_data.blueprint_revision_event_blueprint_revision_event_id_seq,
    ple_data.blueprint_metadata_event_blueprint_metadata_event_id_seq
TO ple_api_owner;
CREATE POLICY blueprint_api_owner_all ON ple_data.blueprint_course TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_revision_api_owner_all ON ple_data.blueprint_course_revision TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_revision_pin_api_owner_all ON ple_data.blueprint_revision_question_pin TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_revision_module_api_owner_all ON ple_data.blueprint_revision_module TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_revision_assignment_api_owner_all ON ple_data.blueprint_revision_assignment TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_revision_event_api_owner_all ON ple_data.blueprint_revision_event TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_metadata_event_api_owner_all ON ple_data.blueprint_metadata_event TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_create_receipt_api_owner_all ON ple_data.blueprint_course_create_receipt TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_save_receipt_api_owner_all ON ple_data.blueprint_course_save_receipt TO ple_api_owner USING (true) WITH CHECK (true);

REVOKE ALL PRIVILEGES ON TABLE
    ple_data.blueprint_course,
    ple_data.blueprint_course_revision,
    ple_data.blueprint_revision_question_pin,
    ple_data.blueprint_revision_module,
    ple_data.blueprint_revision_assignment,
    ple_data.blueprint_revision_event,
    ple_data.blueprint_metadata_event,
    ple_data.blueprint_course_create_receipt,
    ple_data.blueprint_course_save_receipt
FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.blueprint_content_question_pins(jsonb),
    ple_data.blueprint_content_assignments(jsonb), ple_data.blueprint_content_modules(jsonb),
    ple_data.validate_blueprint_content(jsonb),
    ple_data.validate_blueprint_question_selection(bigint, bigint, jsonb),
    ple_data.reject_blueprint_revision_change(), ple_data.reject_blueprint_event_change(),
    ple_data.reject_sealed_blueprint_revision_child_insert() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.blueprint_content_question_pins(jsonb),
    ple_data.blueprint_content_assignments(jsonb), ple_data.blueprint_content_modules(jsonb),
    ple_data.validate_blueprint_content(jsonb),
    ple_data.validate_blueprint_question_selection(bigint, bigint, jsonb)
TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.create_blueprint_course(
    p_blueprint_id uuid, p_request_checksum bytea, p_short_name text, p_long_name text,
    p_content jsonb, p_content_checksum bytea
)
RETURNS TABLE (
    reference_number bigint, blueprint_revision_number bigint, metadata_etag uuid,
    accepted_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE v_actor uuid; v_now timestamp with time zone; v_metadata_etag uuid;
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
    SELECT receipt.blueprint_course_reference_number, receipt.blueprint_revision_number,
           receipt.metadata_etag, receipt.accepted_at
      INTO reference_number, blueprint_revision_number, metadata_etag, accepted_at
      FROM ple_data.blueprint_course_create_receipt AS receipt
     WHERE receipt.actor_account_id = v_actor AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN RETURN NEXT; RETURN; END IF;
    PERFORM ple_data.validate_blueprint_content(p_content);
    PERFORM ple_data.validate_blueprint_question_selection(NULL, NULL, p_content);
    v_now := pg_catalog.clock_timestamp();
    v_metadata_etag := pg_catalog.gen_random_uuid();
    INSERT INTO ple_data.blueprint_course AS course (
        blueprint_id, owner_account_id, short_name, long_name, metadata_etag, created_at
    ) VALUES (
        p_blueprint_id, v_actor, p_short_name, p_long_name, v_metadata_etag, v_now
    ) RETURNING course.reference_number INTO reference_number;
    blueprint_revision_number := 1;
    INSERT INTO ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number,
        content, content_checksum, saved_at
    ) VALUES (
        reference_number, blueprint_revision_number, p_content, p_content_checksum, v_now
    );
    INSERT INTO ple_data.blueprint_revision_question_pin
    SELECT reference_number, blueprint_revision_number, pins.content_path,
           pins.question_id, pins.question_revision_number
      FROM ple_data.blueprint_content_question_pins(p_content) AS pins;
    INSERT INTO ple_data.blueprint_revision_module
    SELECT reference_number, blueprint_revision_number,
           modules.blueprint_module_reference, modules.module_position
      FROM ple_data.blueprint_content_modules(p_content) AS modules;
    INSERT INTO ple_data.blueprint_revision_assignment
    SELECT reference_number, blueprint_revision_number,
           assignments.blueprint_module_reference,
           assignments.blueprint_assignment_reference, assignments.assignment_position
      FROM ple_data.blueprint_content_assignments(p_content) AS assignments;
    INSERT INTO ple_data.blueprint_revision_event (
        blueprint_course_reference_number, blueprint_revision_number, actor_account_id,
        request_checksum, occurred_at
    ) VALUES (
        reference_number, blueprint_revision_number, v_actor, p_request_checksum, v_now
    );
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_reference_number, actor_account_id, short_name, long_name,
        availability, metadata_etag, occurred_at
    ) VALUES (
        reference_number, v_actor, p_short_name, p_long_name,
        'available', v_metadata_etag, v_now
    );
    INSERT INTO ple_data.blueprint_course_create_receipt
    VALUES (
        v_actor, p_request_checksum, reference_number, blueprint_revision_number,
        v_metadata_etag, v_now
    );
    metadata_etag := v_metadata_etag; accepted_at := v_now;
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.save_blueprint_course(
    p_reference_number bigint, p_expected_blueprint_revision_number bigint,
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
BEGIN
    IF p_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_blueprint_revision_number <= 0
       OR octet_length(p_request_checksum) <> 32
       OR octet_length(p_content_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Course Save is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    SELECT * INTO v_course
      FROM ple_data.blueprint_course
     WHERE reference_number = p_reference_number AND owner_account_id = v_actor
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is not available';
    END IF;
    SELECT receipt.resulting_blueprint_revision_number, receipt.changed, receipt.accepted_at
      INTO resulting_blueprint_revision_number, changed, accepted_at
      FROM ple_data.blueprint_course_save_receipt AS receipt
     WHERE receipt.blueprint_course_reference_number = p_reference_number
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
        p_reference_number, p_expected_blueprint_revision_number, p_content
    );
    SELECT * INTO STRICT v_prior
      FROM ple_data.blueprint_course_revision
     WHERE blueprint_course_reference_number = p_reference_number
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
            p_reference_number, resulting_blueprint_revision_number,
            p_content, p_content_checksum, v_now
        );
        INSERT INTO ple_data.blueprint_revision_question_pin
        SELECT p_reference_number, resulting_blueprint_revision_number, pins.content_path,
               pins.question_id, pins.question_revision_number
          FROM ple_data.blueprint_content_question_pins(p_content) AS pins;
        INSERT INTO ple_data.blueprint_revision_module
        SELECT p_reference_number, resulting_blueprint_revision_number,
               modules.blueprint_module_reference, modules.module_position
          FROM ple_data.blueprint_content_modules(p_content) AS modules;
        INSERT INTO ple_data.blueprint_revision_assignment
        SELECT p_reference_number, resulting_blueprint_revision_number,
               assignments.blueprint_module_reference,
               assignments.blueprint_assignment_reference, assignments.assignment_position
          FROM ple_data.blueprint_content_assignments(p_content) AS assignments;
        INSERT INTO ple_data.blueprint_revision_event (
            blueprint_course_reference_number, blueprint_revision_number, actor_account_id,
            request_checksum, occurred_at
        ) VALUES (
            p_reference_number, resulting_blueprint_revision_number,
            v_actor, p_request_checksum, v_now
        );
        UPDATE ple_data.blueprint_course
           SET current_blueprint_revision_number = resulting_blueprint_revision_number
         WHERE reference_number = p_reference_number;
    ELSE
        resulting_blueprint_revision_number := p_expected_blueprint_revision_number;
    END IF;
    INSERT INTO ple_data.blueprint_course_save_receipt
    VALUES (
        p_reference_number, v_actor, p_request_checksum,
        resulting_blueprint_revision_number, changed, v_now
    );
    accepted_at := v_now; RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.rename_blueprint_course(
    p_reference_number bigint, p_expected_metadata_etag uuid,
    p_short_name text, p_long_name text
)
RETURNS TABLE (
    short_name text, long_name text, availability text, metadata_etag uuid
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE v_actor uuid; v_course ple_data.blueprint_course%ROWTYPE; v_next uuid;
BEGIN
    IF p_reference_number NOT BETWEEN 1 AND 2147483647
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
    SELECT * INTO v_course FROM ple_data.blueprint_course
     WHERE reference_number = p_reference_number AND owner_account_id = v_actor FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is not available';
    END IF;
    IF v_course.metadata_etag <> p_expected_metadata_etag THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint metadata ETag is stale';
    END IF;
    IF v_course.short_name = p_short_name AND v_course.long_name = p_long_name THEN
        RETURN QUERY SELECT v_course.short_name, v_course.long_name,
            v_course.availability, v_course.metadata_etag;
        RETURN;
    END IF;
    v_next := pg_catalog.gen_random_uuid();
    UPDATE ple_data.blueprint_course AS course
       SET short_name = p_short_name, long_name = p_long_name, metadata_etag = v_next
     WHERE course.reference_number = p_reference_number;
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_reference_number, actor_account_id, short_name, long_name,
        availability, metadata_etag, occurred_at
    ) VALUES (
        p_reference_number, v_actor, p_short_name, p_long_name,
        v_course.availability, v_next, pg_catalog.clock_timestamp()
    );
    RETURN QUERY SELECT p_short_name, p_long_name, v_course.availability, v_next;
END
$$;

CREATE FUNCTION ple_api.set_blueprint_availability(
    p_reference_number bigint, p_expected_metadata_etag uuid, p_availability text,
    p_archive_confirmation_long_name text
)
RETURNS TABLE (
    short_name text, long_name text, availability text, metadata_etag uuid
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE v_actor uuid; v_course ple_data.blueprint_course%ROWTYPE; v_next uuid;
BEGIN
    IF p_reference_number NOT BETWEEN 1 AND 2147483647
       OR p_expected_metadata_etag IS NULL
       OR p_availability NOT IN ('available', 'archived')
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Blueprint availability change is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    SELECT * INTO v_course FROM ple_data.blueprint_course
     WHERE reference_number = p_reference_number AND owner_account_id = v_actor FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is not available';
    END IF;
    IF v_course.metadata_etag <> p_expected_metadata_etag THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint metadata ETag is stale';
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
     WHERE course.reference_number = p_reference_number;
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_reference_number, actor_account_id, short_name, long_name,
        availability, metadata_etag, occurred_at
    ) VALUES (
        p_reference_number, v_actor, v_course.short_name, v_course.long_name,
        p_availability, v_next, pg_catalog.clock_timestamp()
    );
    RETURN QUERY SELECT v_course.short_name, v_course.long_name, p_availability, v_next;
END
$$;

CREATE FUNCTION ple_api.list_blueprint_courses()
RETURNS TABLE (
    reference_number bigint, short_name text, long_name text, availability text,
    metadata_etag uuid, current_blueprint_revision_number bigint, is_owner boolean,
    total_adoptions bigint, total_students_ever_enrolled bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    RETURN QUERY SELECT course.reference_number, course.short_name, course.long_name,
           course.availability, course.metadata_etag,
           course.current_blueprint_revision_number,
           course.owner_account_id = ple_api.current_session_account_id(),
           (SELECT count(*) FROM ple_data.course_instance AS adoption
             WHERE adoption.blueprint_course_reference_number = course.reference_number),
           (SELECT count(DISTINCT (membership.course_id, membership.account_id))
              FROM ple_data.course_instance AS adoption
              JOIN ple_data.course_membership AS membership ON membership.course_id = adoption.course_id
             WHERE adoption.blueprint_course_reference_number = course.reference_number
               AND membership.role = 'student')
      FROM ple_data.blueprint_course AS course
     WHERE ple_api.current_session_account_is_instructor()
       AND course.availability = 'available'
     ORDER BY course.long_name, course.reference_number;
END
$$;

CREATE FUNCTION ple_api.load_blueprint_course(p_reference_number bigint)
RETURNS TABLE (
    reference_number bigint, short_name text, long_name text, availability text,
    metadata_etag uuid, current_blueprint_revision_number bigint,
    content jsonb, content_checksum bytea, is_owner boolean
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT course.reference_number, course.short_name, course.long_name,
           course.availability, course.metadata_etag,
           course.current_blueprint_revision_number,
           revision.content, revision.content_checksum,
           course.owner_account_id = ple_api.current_session_account_id()
      FROM ple_data.blueprint_course AS course
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_reference_number = course.reference_number
       AND revision.blueprint_revision_number = course.current_blueprint_revision_number
     WHERE p_reference_number BETWEEN 1 AND 2147483647
       AND course.reference_number = p_reference_number
       AND ple_api.current_session_account_is_instructor()
       AND (
           course.owner_account_id = ple_api.current_session_account_id()
           OR course.availability = 'available'
       )
$$;

-- Exact historical provenance remains resolvable after later Saves or archive.
CREATE FUNCTION ple_api.load_blueprint_revision(
    p_reference_number bigint, p_blueprint_revision_number bigint
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
     WHERE p_reference_number BETWEEN 1 AND 2147483647
       AND p_blueprint_revision_number > 0
       AND revision.blueprint_course_reference_number = p_reference_number
       AND revision.blueprint_revision_number = p_blueprint_revision_number
       AND ple_api.current_session_account_is_instructor()
$$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.create_blueprint_course(uuid, bytea, text, text, jsonb, bytea),
    ple_api.save_blueprint_course(bigint, bigint, bytea, jsonb, bytea),
    ple_api.rename_blueprint_course(bigint, uuid, text, text),
    ple_api.set_blueprint_availability(bigint, uuid, text, text),
    ple_api.list_blueprint_courses(), ple_api.load_blueprint_course(bigint),
    ple_api.load_blueprint_revision(bigint, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.create_blueprint_course(uuid, bytea, text, text, jsonb, bytea),
    ple_api.save_blueprint_course(bigint, bigint, bytea, jsonb, bytea),
    ple_api.rename_blueprint_course(bigint, uuid, text, text),
    ple_api.set_blueprint_availability(bigint, uuid, text, text),
    ple_api.list_blueprint_courses(), ple_api.load_blueprint_course(bigint),
    ple_api.load_blueprint_revision(bigint, bigint) TO ple_app;

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.blueprint_course IS
    'Stable reusable Blueprint Course lineage with names, availability, metadata ETag, and current Revision.';
COMMENT ON TABLE ple_data.blueprint_course_revision IS
    'Immutable complete Blueprint Revision; exact references remain valid after later Saves or archive.';
COMMENT ON TABLE ple_data.blueprint_revision_assignment IS
    'Durable Blueprint Assignment identity and Revision membership retained for provenance and comparison.';

RESET ROLE;
