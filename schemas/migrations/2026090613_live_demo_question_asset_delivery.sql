-- M12 authorized immutable public Question Asset redirect lookup.
--
-- This is deliberately a read-only, opaque registry resolver. The caller
-- supplies only the logical Question Asset ID; it receives no source address,
-- bucket, filename, or mutable delivery selection.

-- The resolver below invokes the canonical M11 Student Assignment Access
-- procedure rather than reproducing availability, close, late, or attempt
-- predicates. Its result remains an internal authorization check.
SET LOCAL ROLE ple_api_owner;
GRANT EXECUTE ON FUNCTION ple_api.live_demo_assignment_access(bigint, bigint)
    TO ple_private_owner;
RESET ROLE;

-- The private resolver needs only the Course Instance reference that supplies
-- the two canonical M11 access-procedure arguments. It receives no browser
-- role grant and forced RLS remains in effect.
SET LOCAL ROLE ple_data_owner;
GRANT SELECT ON TABLE ple_data.course_instance TO ple_private_owner;
CREATE POLICY course_instance_private_owner_question_asset_delivery_read
    ON ple_data.course_instance FOR SELECT TO ple_private_owner USING (true);
-- The asset binder authenticates its Student against this relation while
-- running as the private boundary owner.  It receives no browser-role grant.
GRANT SELECT ON TABLE ple_data.student_record TO ple_private_owner;
CREATE POLICY student_record_private_owner_question_asset_delivery_read
    ON ple_data.student_record FOR SELECT TO ple_private_owner USING (true);
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

-- M11 stores the descriptor binding but not its rendered asset set. Preserve
-- that exact server-built set in the same transaction as issue completion so
-- a later asset route cannot infer authority from an unrelated historical
-- Question Revision.
CREATE TABLE ple_private.question_attempt_presentation_asset_binding (
    question_attempt_id uuid PRIMARY KEY
        REFERENCES ple_private.question_attempt_presentation_binding (question_attempt_id)
);
CREATE TABLE ple_private.question_attempt_presentation_asset_rendition (
    question_attempt_id uuid NOT NULL
        REFERENCES ple_private.question_attempt_presentation_asset_binding (question_attempt_id),
    asset_id uuid NOT NULL,
    rendition_checksum bytea NOT NULL CHECK (pg_catalog.octet_length(rendition_checksum) = 32),
    PRIMARY KEY (question_attempt_id, asset_id)
);

CREATE FUNCTION ple_private.reject_question_attempt_presentation_asset_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514',
        MESSAGE = 'Question Attempt Presentation Asset Binding is immutable';
END
$$;
CREATE TRIGGER question_attempt_presentation_asset_binding_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_attempt_presentation_asset_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_attempt_presentation_asset_change();
CREATE TRIGGER question_attempt_presentation_asset_rendition_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_attempt_presentation_asset_rendition
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_attempt_presentation_asset_change();

ALTER TABLE ple_private.question_attempt_presentation_asset_binding ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_presentation_asset_binding FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_presentation_asset_rendition ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_attempt_presentation_asset_rendition FORCE ROW LEVEL SECURITY;
CREATE POLICY question_attempt_presentation_asset_binding_private_owner_access
    ON ple_private.question_attempt_presentation_asset_binding
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_attempt_presentation_asset_rendition_private_owner_access
    ON ple_private.question_attempt_presentation_asset_rendition
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

RESET ROLE;
-- The private owner needs this one migration-scoped capability to create the
-- externally named binder under its final definer identity.  It is revoked
-- immediately after the function is closed below.
SET LOCAL ROLE ple_api_owner;
GRANT CREATE ON SCHEMA ple_api TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;

-- This is invoked by the same PostgreSQL transaction that called M11's
-- native-issue procedure. The API submits only the server-built descriptor
-- facts and ready renditions; the database rechecks each exact Student,
-- Course, Assignment, Question Attempt, nonce, checksum, and registry row.
CREATE FUNCTION ple_api.bind_live_demo_native_ple_presentation_assets(
    p_course_reference_number bigint,
    p_assignment_reference_number bigint,
    p_presentations jsonb
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_assignment ple_data.assignment%ROWTYPE;
    v_student_record_id uuid;
BEGIN
    IF jsonb_typeof(p_presentations) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Presentation Asset Binding requires a presentation array';
    END IF;
    SELECT assignment.* INTO v_assignment
      FROM ple_data.course_instance AS course
      JOIN ple_data.assignment AS assignment ON assignment.course_id = course.course_id
     WHERE course.reference_number = p_course_reference_number
       AND assignment.reference_number = p_assignment_reference_number;
    SELECT student_record.student_record_id INTO v_student_record_id
      FROM ple_data.student_record AS student_record
     WHERE student_record.course_id = v_assignment.course_id
       AND student_record.student_account_id = ple_api.current_session_account_id();
    IF NOT FOUND
       OR NOT ple_api.current_session_account_owns_student_record(
           v_assignment.course_id, v_student_record_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Presentation Asset Binding is unavailable';
    END IF;

    IF EXISTS (
        WITH supplied AS (
            SELECT item.issued_question_id, item.presentation_nonce,
                   item.presentation_checksum, item.question_assets
              FROM jsonb_to_recordset(p_presentations) AS item(
                  issued_question_id uuid, presentation_nonce text,
                  presentation_checksum text, question_assets jsonb
              )
        ), expected AS (
            SELECT issued.issued_question_id, binding.presentation_nonce,
                   binding.presentation_checksum
              FROM ple_private.assignment_attempt AS attempt
              JOIN ple_private.issued_question AS issued
                ON issued.assignment_attempt_id = attempt.assignment_attempt_id
              JOIN ple_private.question_attempt AS question_attempt
                ON question_attempt.issued_question_id = issued.issued_question_id
              JOIN ple_private.question_attempt_presentation_binding AS binding
                ON binding.question_attempt_id = question_attempt.question_attempt_id
             WHERE attempt.assignment_id = v_assignment.assignment_id
               AND attempt.student_record_id = v_student_record_id
               AND attempt.completed_at IS NULL
        )
        SELECT 1 FROM (
            (SELECT issued_question_id, presentation_nonce, presentation_checksum FROM expected
             EXCEPT ALL
             SELECT issued_question_id, presentation_nonce, presentation_checksum FROM supplied)
            UNION ALL
            (SELECT issued_question_id, presentation_nonce, presentation_checksum FROM supplied
             EXCEPT ALL
             SELECT issued_question_id, presentation_nonce, presentation_checksum FROM expected)
        ) AS difference
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Presentation Asset Binding must cover each exact issued Presentation once';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM jsonb_to_recordset(p_presentations) AS item(question_assets jsonb)
         WHERE jsonb_typeof(item.question_assets) <> 'array'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Presentation Asset Binding requires asset arrays';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM jsonb_to_recordset(p_presentations) AS item(
              issued_question_id uuid, question_assets jsonb
          )
          CROSS JOIN LATERAL jsonb_to_recordset(item.question_assets) AS asset(
              asset_id uuid, rendition_checksum text
          )
          JOIN ple_private.issued_question AS issued
            ON issued.issued_question_id = item.issued_question_id
          LEFT JOIN ple_private.question_asset_publication AS publication
            ON publication.question_id = issued.question_id
           AND publication.revision_number = issued.revision_number
           AND publication.asset_id = asset.asset_id
           AND publication.public_object_checksum = decode(asset.rendition_checksum, 'hex')
           AND publication.publication_state = 'ready'
          LEFT JOIN ple_data.object_delivery AS delivery
            ON delivery.delivery_id = publication.delivery_id
           AND delivery.object_id = publication.public_object_id
           AND delivery.delivery_state = 'available'
         WHERE asset.rendition_checksum !~ '^[0-9a-f]{64}$'
            OR publication.asset_id IS NULL
            OR delivery.delivery_id IS NULL
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Presentation Asset Binding requires exact Ready renditions';
    END IF;

    INSERT INTO ple_private.question_attempt_presentation_asset_binding (question_attempt_id)
    SELECT question_attempt.question_attempt_id
      FROM jsonb_to_recordset(p_presentations) AS item(issued_question_id uuid)
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = item.issued_question_id;

    INSERT INTO ple_private.question_attempt_presentation_asset_rendition (
        question_attempt_id, asset_id, rendition_checksum
    )
    SELECT question_attempt.question_attempt_id, asset.asset_id,
           decode(asset.rendition_checksum, 'hex')
      FROM jsonb_to_recordset(p_presentations) AS item(
          issued_question_id uuid, question_assets jsonb
      )
      CROSS JOIN LATERAL jsonb_to_recordset(item.question_assets) AS asset(
          asset_id uuid, rendition_checksum text
      )
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.issued_question_id = item.issued_question_id;
END
$$;

-- The binder is part of the private presentation boundary: it verifies the
-- immutable Ready registry and writes private attempt bindings.  The API
-- owner retains no direct registry or binding-table capability.
REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.bind_live_demo_native_ple_presentation_assets(bigint, bigint, jsonb)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.bind_live_demo_native_ple_presentation_assets(bigint, bigint, jsonb)
TO ple_app;

RESET ROLE;
SET LOCAL ROLE ple_api_owner;
REVOKE CREATE ON SCHEMA ple_api FROM ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.resolve_ready_question_asset_delivery(
    p_asset_id uuid
) RETURNS TABLE (
    question_id text,
    revision_number integer,
    asset_id uuid,
    public_object_id uuid,
    rendition_checksum bytea
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT publication.question_id,
           publication.revision_number,
           publication.asset_id,
           publication.public_object_id,
           publication.public_object_checksum
      FROM ple_private.question_asset_publication AS publication
      JOIN ple_data.object_delivery AS delivery
        ON delivery.delivery_id = publication.delivery_id
       AND delivery.object_id = publication.public_object_id
      JOIN ple_data.question_asset_delivery AS asset_delivery
        ON asset_delivery.delivery_id = publication.delivery_id
       AND asset_delivery.object_id = publication.public_object_id
       AND asset_delivery.question_id = publication.question_id
       AND asset_delivery.revision_number = publication.revision_number
       AND asset_delivery.asset_id = publication.asset_id
      JOIN ple_private.object_record AS public_record
        ON public_record.object_id = publication.public_object_id
      JOIN ple_private.question_attempt_presentation_asset_rendition AS presented_asset
        ON presented_asset.asset_id = publication.asset_id
       AND presented_asset.rendition_checksum = publication.public_object_checksum
      JOIN ple_private.question_attempt_presentation_asset_binding AS presented_assets
        ON presented_assets.question_attempt_id = presented_asset.question_attempt_id
      JOIN ple_private.question_attempt_presentation_binding AS presentation
        ON presentation.question_attempt_id = presented_asset.question_attempt_id
     WHERE publication.asset_id = p_asset_id
       AND publication.publication_state = 'ready'
       AND delivery.delivery_state = 'available'
       AND delivery.sha256 = publication.public_object_checksum
       AND delivery.media_type = publication.verified_media_type
       AND delivery.byte_length = publication.public_byte_length
       AND public_record.object_storage_area = 'public-assets'
       AND public_record.object_data_class = 'question-asset'
       AND public_record.sha256 = publication.public_object_checksum
       AND public_record.size_bytes = publication.public_byte_length
       AND public_record.media_type = publication.verified_media_type
       AND (
            ple_api.current_session_account_is_instructor()
            OR EXISTS (
                SELECT 1
                  FROM ple_private.issued_question AS issued
                  JOIN ple_private.assignment_attempt AS attempt
                    ON attempt.assignment_attempt_id = issued.assignment_attempt_id
                  JOIN ple_private.question_attempt AS question_attempt
                    ON question_attempt.issued_question_id = issued.issued_question_id
                   AND question_attempt.question_attempt_id = presented_asset.question_attempt_id
                  JOIN ple_data.assignment AS assignment
                    ON assignment.assignment_id = attempt.assignment_id
                  JOIN ple_data.course_instance AS course
                    ON course.course_id = assignment.course_id
                  CROSS JOIN LATERAL ple_api.live_demo_assignment_access(
                      course.reference_number, assignment.reference_number
                  ) AS access_decision
                 WHERE issued.question_id = publication.question_id
                   AND issued.revision_number = publication.revision_number
                   AND attempt.completed_at IS NULL
                   AND access_decision.start_decision = 'may_start'
                   AND ple_api.current_session_account_owns_student_record(
                       assignment.course_id, attempt.student_record_id
                   )
            )
       )
$$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_private.resolve_ready_question_asset_delivery(uuid)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.resolve_ready_question_asset_delivery(uuid)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

-- ASVS 8.2.1/8.3.1: the API role invokes one fixed resolver under an
-- installed session; no route can substitute an Object Address or a Question
-- Revision. The empty result is intentionally shared by absent, pending, and
-- unauthorized assets so the HTTP layer can conceal all three alike.
CREATE FUNCTION ple_api.resolve_ready_live_demo_question_asset(
    p_asset_id uuid
) RETURNS TABLE (
    question_id text,
    revision_number integer,
    asset_id uuid,
    public_object_id uuid,
    rendition_checksum bytea
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question_id, revision_number, asset_id, public_object_id, rendition_checksum
      FROM ple_private.resolve_ready_question_asset_delivery(p_asset_id)
$$;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_api.resolve_ready_live_demo_question_asset(uuid)
FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.resolve_ready_live_demo_question_asset(uuid)
    TO ple_app;

RESET ROLE;
