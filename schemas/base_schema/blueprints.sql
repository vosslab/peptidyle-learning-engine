-- Reusable Blueprint Course lineages, their private Drafts, and immutable
-- Blueprint Revisions.  A publication is an intentional event: equal content
-- does not suppress a new Revision, while one accepted request is replay-safe.

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
    availability text NOT NULL DEFAULT 'available'
        CHECK (availability IN ('available', 'archived')),
    availability_edit_number bigint NOT NULL DEFAULT 1
        CHECK (availability_edit_number > 0),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT blueprint_course_reference_is_bounded CHECK (
        reference_number BETWEEN 1 AND 2147483647
    )
);

CREATE TABLE ple_data.blueprint_draft (
    blueprint_course_reference_number bigint PRIMARY KEY
        REFERENCES ple_data.blueprint_course (reference_number) ON DELETE CASCADE,
    edit_number bigint NOT NULL DEFAULT 1 CHECK (edit_number > 0),
    content jsonb NOT NULL CHECK (jsonb_typeof(content) = 'object'),
    content_checksum bytea NOT NULL CHECK (octet_length(content_checksum) = 32),
    updated_at timestamp with time zone NOT NULL
);

CREATE TABLE ple_data.blueprint_draft_question_pin (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_draft (blueprint_course_reference_number) ON DELETE CASCADE,
    content_path text NOT NULL CHECK (char_length(content_path) BETWEEN 1 AND 500),
    question_id text NOT NULL,
    question_revision_number bigint NOT NULL CHECK (question_revision_number > 0),
    PRIMARY KEY (blueprint_course_reference_number, content_path),
    FOREIGN KEY (question_id, question_revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);

-- A Module is a stable reusable identity.  Its ordered Assignment members
-- live in a separate relation so a Module can contain more than one
-- Assignment without reusing the Module identity.
CREATE TABLE ple_data.blueprint_draft_module (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_draft (blueprint_course_reference_number) ON DELETE CASCADE,
    blueprint_module_reference uuid NOT NULL,
    module_position integer NOT NULL CHECK (module_position BETWEEN 1 AND 1024),
    PRIMARY KEY (blueprint_course_reference_number, blueprint_module_reference),
    UNIQUE (blueprint_course_reference_number, module_position)
);

CREATE TABLE ple_data.blueprint_draft_assignment (
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_module_reference uuid NOT NULL,
    blueprint_assignment_reference uuid NOT NULL,
    assignment_position integer NOT NULL CHECK (assignment_position BETWEEN 1 AND 1024),
    PRIMARY KEY (blueprint_course_reference_number, blueprint_assignment_reference),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_module_reference)
        REFERENCES ple_data.blueprint_draft_module
            (blueprint_course_reference_number, blueprint_module_reference) ON DELETE CASCADE,
    UNIQUE (blueprint_course_reference_number, blueprint_module_reference, assignment_position)
);

CREATE TABLE ple_data.blueprint_course_revision (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    blueprint_revision_number bigint NOT NULL CHECK (blueprint_revision_number > 0),
    title text NOT NULL CHECK (char_length(btrim(title)) BETWEEN 1 AND 500),
    content jsonb NOT NULL CHECK (jsonb_typeof(content) = 'object'),
    content_checksum bytea NOT NULL CHECK (octet_length(content_checksum) = 32),
    published_at timestamp with time zone NOT NULL,
    PRIMARY KEY (blueprint_course_reference_number, blueprint_revision_number)
);

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

-- This is the stable member identity needed by Assignment provenance.  The
-- Assignment module owns the destination-side BlueprintAssignmentSource FK.
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

CREATE TABLE ple_data.blueprint_publication_event (
    blueprint_publication_event_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
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

CREATE TABLE ple_data.blueprint_availability_event (
    blueprint_availability_event_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    availability text NOT NULL CHECK (availability IN ('available', 'archived')),
    edit_number bigint NOT NULL CHECK (edit_number > 0),
    occurred_at timestamp with time zone NOT NULL,
    UNIQUE (blueprint_course_reference_number, edit_number)
);

CREATE TABLE ple_data.blueprint_draft_create_receipt (
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (actor_account_id, request_checksum)
);

CREATE TABLE ple_data.blueprint_draft_save_receipt (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    resulting_edit_number bigint NOT NULL CHECK (resulting_edit_number > 0),
    changed boolean NOT NULL,
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (blueprint_course_reference_number, actor_account_id, request_checksum)
);

CREATE INDEX blueprint_course_available_owner_idx
    ON ple_data.blueprint_course (availability, owner_account_id, reference_number);
CREATE INDEX blueprint_revision_question_pin_question_idx
    ON ple_data.blueprint_revision_question_pin (question_id, question_revision_number);

-- The content remains the complete answer-free published representation.  The
-- two extraction functions make every Question Revision and reusable
-- Blueprint Assignment identity relational evidence with exact foreign keys.
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
           pin ->> 'questionId',
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
       OR p_content ->> 'title' IS NULL
       OR p_content ->> 'title' <> btrim(p_content ->> 'title')
       OR char_length(p_content ->> 'title') NOT BETWEEN 1 AND 500
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
            MESSAGE = 'Blueprint Draft content is invalid';
    END IF;
END
$$;

-- The Draft pin table preserves exact historical selections.  A create must
-- select only currently Available Question lineages; a changed Draft may keep
-- an exact pin it already owned after that lineage is archived.  Publication
-- copies those retained pins without reinterpreting their current availability.
CREATE FUNCTION ple_data.validate_blueprint_question_selection(
    p_existing_blueprint_course_reference_number bigint,
    p_content jsonb
) RETURNS void
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    -- Hold each selected lineage through the Draft write.  Archive takes the
    -- stronger lineage lock, so it cannot interleave an availability change
    -- between this selection check and relational pin insertion.
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
          LEFT JOIN ple_data.blueprint_draft_question_pin AS existing
            ON existing.blueprint_course_reference_number
                 = p_existing_blueprint_course_reference_number
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
        MESSAGE = 'Blueprint publication and availability evidence is immutable';
END
$$;

CREATE TRIGGER blueprint_course_revision_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_course_revision
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_revision_question_pin_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_question_pin
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_revision_assignment_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_assignment
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_revision_module_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_module
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_change();
CREATE TRIGGER blueprint_publication_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_publication_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_event_change();
CREATE TRIGGER blueprint_availability_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_availability_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_event_change();

ALTER TABLE ple_data.blueprint_course ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft_question_pin ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft_question_pin FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft_module ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft_module FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft_assignment ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft_assignment FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_revision FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_question_pin ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_question_pin FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_module ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_module FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_assignment ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_assignment FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_publication_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_publication_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_availability_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_availability_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft_create_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft_create_receipt FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft_save_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_draft_save_receipt FORCE ROW LEVEL SECURITY;

GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE
    ple_data.blueprint_course,
    ple_data.blueprint_draft,
    ple_data.blueprint_draft_question_pin,
    ple_data.blueprint_draft_module,
    ple_data.blueprint_draft_assignment,
    ple_data.blueprint_course_revision,
    ple_data.blueprint_revision_question_pin,
    ple_data.blueprint_revision_module,
    ple_data.blueprint_revision_assignment,
    ple_data.blueprint_publication_event,
    ple_data.blueprint_availability_event,
    ple_data.blueprint_draft_create_receipt,
    ple_data.blueprint_draft_save_receipt
TO ple_api_owner;
GRANT USAGE ON SEQUENCE ple_data.blueprint_course_reference_number_seq,
    ple_data.blueprint_publication_event_blueprint_publication_event_id_seq,
    ple_data.blueprint_availability_event_blueprint_availability_event_i_seq
TO ple_api_owner;
CREATE POLICY blueprint_api_owner_all ON ple_data.blueprint_course TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_draft_api_owner_all ON ple_data.blueprint_draft TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_draft_pin_api_owner_all ON ple_data.blueprint_draft_question_pin TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_draft_module_api_owner_all ON ple_data.blueprint_draft_module TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_draft_assignment_api_owner_all ON ple_data.blueprint_draft_assignment TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_revision_api_owner_all ON ple_data.blueprint_course_revision TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_revision_pin_api_owner_all ON ple_data.blueprint_revision_question_pin TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_revision_module_api_owner_all ON ple_data.blueprint_revision_module TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_revision_assignment_api_owner_all ON ple_data.blueprint_revision_assignment TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_publication_api_owner_all ON ple_data.blueprint_publication_event TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_availability_api_owner_all ON ple_data.blueprint_availability_event TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_create_receipt_api_owner_all ON ple_data.blueprint_draft_create_receipt TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_save_receipt_api_owner_all ON ple_data.blueprint_draft_save_receipt TO ple_api_owner USING (true) WITH CHECK (true);

REVOKE ALL PRIVILEGES ON TABLE
    ple_data.blueprint_course,
    ple_data.blueprint_draft,
    ple_data.blueprint_draft_question_pin,
    ple_data.blueprint_draft_module,
    ple_data.blueprint_draft_assignment,
    ple_data.blueprint_course_revision,
    ple_data.blueprint_revision_question_pin,
    ple_data.blueprint_revision_module,
    ple_data.blueprint_revision_assignment,
    ple_data.blueprint_publication_event,
    ple_data.blueprint_availability_event,
    ple_data.blueprint_draft_create_receipt,
    ple_data.blueprint_draft_save_receipt
FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.blueprint_content_question_pins(jsonb),
    ple_data.blueprint_content_assignments(jsonb), ple_data.blueprint_content_modules(jsonb),
    ple_data.validate_blueprint_content(jsonb),
    ple_data.validate_blueprint_question_selection(bigint, jsonb),
    ple_data.reject_blueprint_revision_change(), ple_data.reject_blueprint_event_change() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.blueprint_content_question_pins(jsonb),
    ple_data.blueprint_content_assignments(jsonb), ple_data.blueprint_content_modules(jsonb),
    ple_data.validate_blueprint_content(jsonb),
    ple_data.validate_blueprint_question_selection(bigint, jsonb)
TO ple_api_owner;

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.create_blueprint_draft(
    p_blueprint_id uuid, p_request_checksum bytea, p_content jsonb, p_content_checksum bytea
)
RETURNS TABLE (reference_number bigint, draft_edit_number bigint, accepted_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE v_actor uuid; v_now timestamp with time zone;
BEGIN
    IF p_blueprint_id IS NULL OR octet_length(p_request_checksum) <> 32
       OR octet_length(p_content_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Draft creation is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    -- No lineage exists until this command wins.  The command identity is its
    -- natural serialization key, so a concurrent transport replay waits and
    -- then observes the original receipt.
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('ple:blueprint-draft-create:%s:%s', v_actor,
            pg_catalog.encode(p_request_checksum, 'hex')), 0));
    SELECT receipt.blueprint_course_reference_number, 1, receipt.accepted_at
      INTO reference_number, draft_edit_number, accepted_at
      FROM ple_data.blueprint_draft_create_receipt AS receipt
     WHERE receipt.actor_account_id = v_actor AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN RETURN NEXT; RETURN; END IF;
    PERFORM ple_data.validate_blueprint_content(p_content);
    PERFORM ple_data.validate_blueprint_question_selection(NULL, p_content);
    v_now := pg_catalog.clock_timestamp();
    INSERT INTO ple_data.blueprint_course AS course (blueprint_id, owner_account_id, created_at)
    VALUES (p_blueprint_id, v_actor, v_now)
    RETURNING course.reference_number INTO reference_number;
    INSERT INTO ple_data.blueprint_draft (
        blueprint_course_reference_number, content, content_checksum, updated_at
    ) VALUES (reference_number, p_content, p_content_checksum, v_now);
    INSERT INTO ple_data.blueprint_draft_question_pin
    SELECT reference_number, pins.content_path, pins.question_id, pins.question_revision_number
      FROM ple_data.blueprint_content_question_pins(p_content) AS pins;
    INSERT INTO ple_data.blueprint_draft_module
    SELECT reference_number, modules.blueprint_module_reference, modules.module_position
      FROM ple_data.blueprint_content_modules(p_content) AS modules;
    INSERT INTO ple_data.blueprint_draft_assignment
    SELECT reference_number, assignments.blueprint_module_reference,
           assignments.blueprint_assignment_reference, assignments.assignment_position
      FROM ple_data.blueprint_content_assignments(p_content) AS assignments;
    INSERT INTO ple_data.blueprint_draft_create_receipt
    VALUES (v_actor, p_request_checksum, reference_number, v_now);
    draft_edit_number := 1; accepted_at := v_now;
    RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.save_blueprint_draft(
    p_reference_number bigint, p_expected_edit_number bigint, p_request_checksum bytea,
    p_content jsonb, p_content_checksum bytea
)
RETURNS TABLE (resulting_edit_number bigint, changed boolean, accepted_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_actor uuid;
    v_course ple_data.blueprint_course%ROWTYPE;
    v_draft ple_data.blueprint_draft%ROWTYPE;
    v_now timestamp with time zone;
BEGIN
    IF p_reference_number NOT BETWEEN 1 AND 2147483647 OR p_expected_edit_number <= 0
       OR octet_length(p_request_checksum) <> 32 OR octet_length(p_content_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint Draft save is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    -- Course then Draft is the Blueprint write order.  It serializes a
    -- same-command replay before receipt lookup and keeps availability and
    -- content transitions on one lineage lock.
    SELECT * INTO v_course FROM ple_data.blueprint_course AS course
     WHERE course.reference_number = p_reference_number AND course.owner_account_id = v_actor
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Draft is not available'; END IF;
    SELECT * INTO v_draft FROM ple_data.blueprint_draft AS draft
     WHERE draft.blueprint_course_reference_number = p_reference_number
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Draft is not available'; END IF;
    SELECT receipt.resulting_edit_number, receipt.changed, receipt.accepted_at
      INTO resulting_edit_number, changed, accepted_at
      FROM ple_data.blueprint_draft_save_receipt AS receipt
     WHERE receipt.blueprint_course_reference_number = p_reference_number
       AND receipt.actor_account_id = v_actor AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN RETURN NEXT; RETURN; END IF;
    IF v_draft.edit_number <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint Draft Edit Number is stale';
    END IF;
    PERFORM ple_data.validate_blueprint_content(p_content);
    changed := v_draft.content IS DISTINCT FROM p_content OR v_draft.content_checksum IS DISTINCT FROM p_content_checksum;
    v_now := pg_catalog.clock_timestamp();
    IF changed THEN
        PERFORM ple_data.validate_blueprint_question_selection(p_reference_number, p_content);
        resulting_edit_number := v_draft.edit_number + 1;
        UPDATE ple_data.blueprint_draft SET edit_number = resulting_edit_number,
            content = p_content, content_checksum = p_content_checksum, updated_at = v_now
         WHERE blueprint_course_reference_number = p_reference_number;
        DELETE FROM ple_data.blueprint_draft_question_pin WHERE blueprint_course_reference_number = p_reference_number;
        DELETE FROM ple_data.blueprint_draft_assignment WHERE blueprint_course_reference_number = p_reference_number;
        DELETE FROM ple_data.blueprint_draft_module WHERE blueprint_course_reference_number = p_reference_number;
        INSERT INTO ple_data.blueprint_draft_question_pin
        SELECT p_reference_number, pins.content_path, pins.question_id, pins.question_revision_number
          FROM ple_data.blueprint_content_question_pins(p_content) AS pins;
        INSERT INTO ple_data.blueprint_draft_module
        SELECT p_reference_number, modules.blueprint_module_reference, modules.module_position
          FROM ple_data.blueprint_content_modules(p_content) AS modules;
        INSERT INTO ple_data.blueprint_draft_assignment
        SELECT p_reference_number, assignments.blueprint_module_reference,
               assignments.blueprint_assignment_reference, assignments.assignment_position
          FROM ple_data.blueprint_content_assignments(p_content) AS assignments;
    ELSE
        resulting_edit_number := v_draft.edit_number;
    END IF;
    INSERT INTO ple_data.blueprint_draft_save_receipt
    VALUES (p_reference_number, v_actor, p_request_checksum, resulting_edit_number, changed, v_now);
    accepted_at := v_now; RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.publish_blueprint_draft(
    p_reference_number bigint, p_expected_edit_number bigint, p_request_checksum bytea
)
RETURNS TABLE (blueprint_revision_number bigint, accepted_at timestamp with time zone)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_actor uuid;
    v_course ple_data.blueprint_course%ROWTYPE;
    v_draft ple_data.blueprint_draft%ROWTYPE;
    v_now timestamp with time zone;
BEGIN
    IF p_reference_number NOT BETWEEN 1 AND 2147483647 OR p_expected_edit_number <= 0
       OR octet_length(p_request_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint publication is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    SELECT * INTO v_course FROM ple_data.blueprint_course AS course
     WHERE course.reference_number = p_reference_number AND course.owner_account_id = v_actor
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Draft is not available'; END IF;
    SELECT * INTO v_draft FROM ple_data.blueprint_draft AS draft
     WHERE draft.blueprint_course_reference_number = p_reference_number
     FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Draft is not available'; END IF;
    SELECT event.blueprint_revision_number, event.occurred_at
      INTO blueprint_revision_number, accepted_at
      FROM ple_data.blueprint_publication_event AS event
     WHERE event.blueprint_course_reference_number = p_reference_number
       AND event.actor_account_id = v_actor AND event.request_checksum = p_request_checksum;
    IF FOUND THEN RETURN NEXT; RETURN; END IF;
    IF v_draft.edit_number <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint Draft Edit Number is stale';
    END IF;
    PERFORM ple_data.validate_blueprint_content(v_draft.content);
    SELECT COALESCE(MAX(revision.blueprint_revision_number), 0) + 1 INTO blueprint_revision_number
      FROM ple_data.blueprint_course_revision AS revision
     WHERE revision.blueprint_course_reference_number = p_reference_number;
    v_now := pg_catalog.clock_timestamp();
    INSERT INTO ple_data.blueprint_course_revision
    VALUES (p_reference_number, blueprint_revision_number, v_draft.content ->> 'title',
            v_draft.content, v_draft.content_checksum, v_now);
    INSERT INTO ple_data.blueprint_revision_question_pin
    SELECT p_reference_number, blueprint_revision_number, pin.content_path,
           pin.question_id, pin.question_revision_number
      FROM ple_data.blueprint_draft_question_pin AS pin
     WHERE pin.blueprint_course_reference_number = p_reference_number;
    INSERT INTO ple_data.blueprint_revision_module
    SELECT p_reference_number, blueprint_revision_number, item.blueprint_module_reference,
           item.module_position
      FROM ple_data.blueprint_draft_module AS item
     WHERE item.blueprint_course_reference_number = p_reference_number;
    INSERT INTO ple_data.blueprint_revision_assignment
    SELECT p_reference_number, blueprint_revision_number, item.blueprint_module_reference,
           item.blueprint_assignment_reference, item.assignment_position
      FROM ple_data.blueprint_draft_assignment AS item
     WHERE item.blueprint_course_reference_number = p_reference_number;
    INSERT INTO ple_data.blueprint_publication_event (
        blueprint_course_reference_number, blueprint_revision_number, actor_account_id,
        request_checksum, occurred_at
    ) VALUES (p_reference_number, blueprint_revision_number, v_actor, p_request_checksum, v_now);
    accepted_at := v_now; RETURN NEXT;
END
$$;

CREATE FUNCTION ple_api.set_blueprint_availability(
    p_reference_number bigint, p_expected_edit_number bigint, p_availability text,
    p_archive_confirmation_title text
)
RETURNS TABLE (resulting_edit_number bigint, availability text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_actor uuid;
    v_course ple_data.blueprint_course%ROWTYPE;
    v_current_title text;
BEGIN
    IF p_reference_number NOT BETWEEN 1 AND 2147483647 OR p_expected_edit_number <= 0
       OR p_availability NOT IN ('available', 'archived')
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint availability change is invalid';
    END IF;
    v_actor := ple_api.current_session_account_id();
    SELECT * INTO v_course FROM ple_data.blueprint_course
     WHERE reference_number = p_reference_number AND owner_account_id = v_actor FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Blueprint Course is not available'; END IF;
    IF v_course.availability_edit_number <> p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint availability Edit Number is stale';
    END IF;
    SELECT draft.content ->> 'title' INTO v_current_title
      FROM ple_data.blueprint_draft AS draft
     WHERE draft.blueprint_course_reference_number = p_reference_number
       FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'XX000', MESSAGE = 'Blueprint Draft is missing';
    END IF;
    IF p_availability = 'archived'
       AND p_archive_confirmation_title IS DISTINCT FROM v_current_title THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Archive Blueprint requires exact title confirmation';
    END IF;
    IF v_course.availability = p_availability THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Blueprint availability already has that state';
    END IF;
    resulting_edit_number := v_course.availability_edit_number + 1;
    UPDATE ple_data.blueprint_course SET availability = p_availability,
        availability_edit_number = resulting_edit_number WHERE reference_number = p_reference_number;
    INSERT INTO ple_data.blueprint_availability_event (
        blueprint_course_reference_number, actor_account_id, availability, edit_number, occurred_at
    ) VALUES (p_reference_number, v_actor, p_availability, resulting_edit_number, clock_timestamp());
    availability := p_availability; RETURN NEXT;
END
$$;

-- Ordinary discovery sees available published content.  Ownership grants
-- edit access through the targeted functions; it does not keep archived
-- lineages in browsing results.
CREATE FUNCTION ple_api.list_blueprint_courses()
RETURNS TABLE (
    reference_number bigint, title text, availability text, availability_edit_number bigint,
    latest_blueprint_revision_number bigint, is_owner boolean
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT course.reference_number,
           COALESCE(revision.title, draft.content ->> 'title'),
           course.availability,
           course.availability_edit_number,
           revision.blueprint_revision_number,
           course.owner_account_id = ple_api.current_session_account_id()
      FROM ple_data.blueprint_course AS course
      LEFT JOIN ple_data.blueprint_draft AS draft
        ON draft.blueprint_course_reference_number = course.reference_number
      LEFT JOIN LATERAL (
          SELECT candidate.*
            FROM ple_data.blueprint_course_revision AS candidate
           WHERE candidate.blueprint_course_reference_number = course.reference_number
           ORDER BY candidate.blueprint_revision_number DESC
           LIMIT 1
      ) AS revision ON true
     WHERE ple_api.current_session_account_is_instructor()
       AND course.availability = 'available'
       AND revision.blueprint_revision_number IS NOT NULL
     ORDER BY COALESCE(revision.title, draft.content ->> 'title'), course.reference_number
$$;

CREATE FUNCTION ple_api.load_blueprint_course(p_reference_number bigint)
RETURNS TABLE (
    reference_number bigint, title text, availability text, availability_edit_number bigint,
    latest_blueprint_revision_number bigint, content jsonb,
    content_checksum bytea, draft_edit_number bigint, is_owner boolean
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT course.reference_number,
           CASE WHEN course.owner_account_id = ple_api.current_session_account_id()
                THEN draft.content ->> 'title' ELSE revision.title END,
           course.availability, course.availability_edit_number,
           revision.blueprint_revision_number,
           CASE WHEN course.owner_account_id = ple_api.current_session_account_id()
                THEN draft.content ELSE revision.content END,
           CASE WHEN course.owner_account_id = ple_api.current_session_account_id()
                THEN draft.content_checksum ELSE revision.content_checksum END,
           CASE WHEN course.owner_account_id = ple_api.current_session_account_id()
                THEN draft.edit_number END,
           course.owner_account_id = ple_api.current_session_account_id()
      FROM ple_data.blueprint_course AS course
      JOIN ple_data.blueprint_draft AS draft
        ON draft.blueprint_course_reference_number = course.reference_number
      LEFT JOIN LATERAL (
          SELECT candidate.*
            FROM ple_data.blueprint_course_revision AS candidate
           WHERE candidate.blueprint_course_reference_number = course.reference_number
           ORDER BY candidate.blueprint_revision_number DESC
           LIMIT 1
      ) AS revision ON true
     WHERE p_reference_number BETWEEN 1 AND 2147483647
       AND course.reference_number = p_reference_number
       AND ple_api.current_session_account_is_instructor()
       AND (
           course.owner_account_id = ple_api.current_session_account_id()
           OR (course.availability = 'available' AND revision.blueprint_revision_number IS NOT NULL)
       )
$$;

-- Exact provenance remains resolvable to an authorized Instructor after its
-- lineage is archived; archive changes ordinary discovery, not evidence.
CREATE FUNCTION ple_api.load_blueprint_revision(
    p_reference_number bigint, p_blueprint_revision_number bigint
)
RETURNS TABLE (content jsonb, content_checksum bytea, published_at timestamp with time zone)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT revision.content, revision.content_checksum, revision.published_at
      FROM ple_data.blueprint_course_revision AS revision
      JOIN ple_data.blueprint_course AS course
        ON course.reference_number = revision.blueprint_course_reference_number
     WHERE p_reference_number BETWEEN 1 AND 2147483647
       AND p_blueprint_revision_number > 0
       AND revision.blueprint_course_reference_number = p_reference_number
       AND revision.blueprint_revision_number = p_blueprint_revision_number
       AND ple_api.current_session_account_is_instructor()
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.create_blueprint_draft(uuid, bytea, jsonb, bytea),
    ple_api.save_blueprint_draft(bigint, bigint, bytea, jsonb, bytea),
    ple_api.publish_blueprint_draft(bigint, bigint, bytea),
    ple_api.set_blueprint_availability(bigint, bigint, text, text),
    ple_api.list_blueprint_courses(), ple_api.load_blueprint_course(bigint),
    ple_api.load_blueprint_revision(bigint, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.create_blueprint_draft(uuid, bytea, jsonb, bytea),
    ple_api.save_blueprint_draft(bigint, bigint, bytea, jsonb, bytea),
    ple_api.publish_blueprint_draft(bigint, bigint, bytea),
    ple_api.set_blueprint_availability(bigint, bigint, text, text),
    ple_api.list_blueprint_courses(), ple_api.load_blueprint_course(bigint),
    ple_api.load_blueprint_revision(bigint, bigint) TO ple_app;

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.blueprint_course IS
    'Stable reusable Blueprint Course lineage with independent current availability.';
COMMENT ON TABLE ple_data.blueprint_draft IS
    'One owner-private mutable Blueprint Draft; creation does not publish a Revision.';
COMMENT ON TABLE ple_data.blueprint_course_revision IS
    'Immutable complete published Blueprint Revision; exact references remain valid after archive.';
COMMENT ON TABLE ple_data.blueprint_revision_assignment IS
    'Exact reusable Blueprint Assignment membership retained for BlueprintAssignmentSource provenance.';

RESET ROLE;
