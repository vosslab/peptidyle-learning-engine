-- Atomic creation of a new Blueprint Course from current reusable Course structure.

SET LOCAL ROLE ple_data_owner;

-- This relation is source provenance, not daughter-Course adoption provenance.
-- A Course Instance remains unchanged and never acquires a parent Blueprint
-- when its reusable structure is copied into a new lineage.
CREATE TABLE ple_data.blueprint_course_instance_source (
    blueprint_course_reference_number bigint PRIMARY KEY
        REFERENCES ple_data.blueprint_course (reference_number),
    source_course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    recorded_at timestamp with time zone NOT NULL
);

CREATE INDEX blueprint_course_instance_source_course_idx
    ON ple_data.blueprint_course_instance_source (source_course_id);

CREATE FUNCTION ple_data.reject_blueprint_course_instance_source_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'a Blueprint Course source Course Instance is immutable';
END
$$;

CREATE TRIGGER blueprint_course_instance_source_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_course_instance_source
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_course_instance_source_change();

ALTER TABLE ple_data.blueprint_course_instance_source ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_instance_source FORCE ROW LEVEL SECURITY;
GRANT SELECT, INSERT ON TABLE ple_data.blueprint_course_instance_source TO ple_api_owner;
CREATE POLICY blueprint_course_instance_source_api_owner_all
    ON ple_data.blueprint_course_instance_source TO ple_api_owner
    USING (true) WITH CHECK (true);
REVOKE ALL PRIVILEGES ON TABLE ple_data.blueprint_course_instance_source FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.reject_blueprint_course_instance_source_change() FROM PUBLIC;

-- Lock every selected Question lineage in canonical order, then apply the
-- ordinary Available-Published-Question predicate to fixed pins and every
-- exact Pool member. The data-owner capability avoids granting the API owner
-- a general Question update privilege merely to obtain row locks.
CREATE FUNCTION ple_data.lock_course_blueprint_publication_questions(p_course_id uuid)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Instance reusable structure is unavailable';
    END IF;
    PERFORM question.question_id
      FROM ple_data.published_question AS question
     WHERE question.question_id IN (
               SELECT entry.question_id
                 FROM ple_data.assessment AS assessment
                 JOIN ple_data.assessment_entry AS entry
                   ON entry.assessment_id = assessment.assessment_id
                WHERE assessment.course_id = p_course_id
                  AND entry.availability = 'available'
                  AND entry.entry_kind = 'fixed_question'
               UNION
               SELECT member.question_id
                 FROM ple_data.assessment AS assessment
                 JOIN ple_data.assessment_entry AS entry
                   ON entry.assessment_id = assessment.assessment_id
                 JOIN ple_data.question_pool_revision_member AS member
                   ON member.question_pool_id = entry.question_pool_id
                  AND member.revision_number = entry.question_pool_revision_number
                WHERE assessment.course_id = p_course_id
                  AND entry.availability = 'available'
                  AND entry.entry_kind = 'question_pool'
           )
     ORDER BY question.question_id
     FOR SHARE OF question;
    IF EXISTS (
        SELECT 1
          FROM ple_data.assessment AS assessment
          JOIN ple_data.assessment_entry AS entry
            ON entry.assessment_id = assessment.assessment_id
          LEFT JOIN ple_data.published_question AS question
            ON question.question_id = entry.question_id
         WHERE assessment.course_id = p_course_id
           AND entry.availability = 'available'
           AND entry.entry_kind = 'fixed_question'
           AND question.availability IS DISTINCT FROM 'available'
    ) OR EXISTS (
        SELECT 1
          FROM ple_data.assessment AS assessment
          JOIN ple_data.assessment_entry AS entry
            ON entry.assessment_id = assessment.assessment_id
          JOIN ple_data.question_pool_revision_member AS member
            ON member.question_pool_id = entry.question_pool_id
           AND member.revision_number = entry.question_pool_revision_number
          LEFT JOIN ple_data.published_question AS question
            ON question.question_id = member.question_id
         WHERE assessment.course_id = p_course_id
           AND entry.availability = 'available'
           AND entry.entry_kind = 'question_pool'
           AND question.availability IS DISTINCT FROM 'available'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course reusable structure requires Available Published Questions';
    END IF;
END
$$;
REVOKE ALL ON FUNCTION
    ple_data.lock_course_blueprint_publication_questions(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_data.lock_course_blueprint_publication_questions(uuid) TO ple_api_owner;

-- Exact reusable Assessment content. Dates, lifecycle state, origin, Course
-- identity, roster, and Student Work have no representation in this projection.
CREATE FUNCTION ple_data.course_blueprint_publication_assessment_content(p_assessment_id uuid)
RETURNS jsonb LANGUAGE sql STABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT jsonb_build_object(
        'assessment_type', assessment.assessment_type,
        'title', assessment.assessment_title,
        'instructions', assessment.assessment_instructions,
        'entries', COALESCE((
            SELECT jsonb_agg(
                CASE entry.entry_kind
                    WHEN 'fixed_question' THEN jsonb_build_object(
                        'kind', 'fixed',
                        'question_revision', jsonb_build_object(
                            'questionId', ple_data.canonical_public_crockford_display(
                                entry.question_id
                            ),
                            'revisionNumber', entry.question_revision_number
                        ),
                        'points_possible', entry.points_possible::text,
                        'scoring_rule', CASE entry.scoring_rule
                            WHEN 'full_credit' THEN 'fullCredit'
                            WHEN 'extra_credit' THEN 'extraCredit'
                            ELSE entry.scoring_rule
                        END,
                        'question_attempt_limit', jsonb_build_object(
                            'maxAttempts', entry.question_attempt_limit
                        ),
                        'question_attempt_time_limit', CASE
                            WHEN entry.question_attempt_time_limit_seconds IS NULL
                                THEN jsonb_build_object('kind', 'unlimited')
                            ELSE jsonb_build_object(
                                'kind', 'limited',
                                'seconds', entry.question_attempt_time_limit_seconds,
                                'graceSeconds', entry.question_attempt_grace_seconds
                            )
                        END
                    )
                    WHEN 'question_pool' THEN jsonb_build_object(
                        'kind', 'pool',
                        'question_pool_revision', jsonb_build_object(
                            'questionPoolId', ple_data.canonical_public_crockford_display(
                                pool.public_question_pool_id
                            ),
                            'revisionNumber', entry.question_pool_revision_number
                        ),
                        'selection_count', entry.selection_count,
                        'points_per_item', entry.points_per_item::text,
                        'scoring_rule', CASE entry.scoring_rule
                            WHEN 'full_credit' THEN 'fullCredit'
                            WHEN 'extra_credit' THEN 'extraCredit'
                            ELSE entry.scoring_rule
                        END,
                        'selection_rule', jsonb_build_object(
                            'selectedQuestionOrder', CASE entry.selected_question_order
                                WHEN 'question_pool_order' THEN 'questionPoolOrder'
                                WHEN 'random_order' THEN 'randomOrder'
                            END
                        ),
                        'question_attempt_limit', jsonb_build_object(
                            'maxAttempts', entry.question_attempt_limit
                        ),
                        'question_attempt_time_limit', CASE
                            WHEN entry.question_attempt_time_limit_seconds IS NULL
                                THEN jsonb_build_object('kind', 'unlimited')
                            ELSE jsonb_build_object(
                                'kind', 'limited',
                                'seconds', entry.question_attempt_time_limit_seconds,
                                'graceSeconds', entry.question_attempt_grace_seconds
                            )
                        END
                    )
                END ORDER BY entry.authored_position
            )
              FROM ple_data.assessment_entry AS entry
              LEFT JOIN ple_data.question_pool AS pool
                ON pool.question_pool_id = entry.question_pool_id
             WHERE entry.assessment_id = assessment.assessment_id
               AND entry.availability = 'available'
        ), '[]'::jsonb),
        'defaults', jsonb_build_object(
            'assessment_attempt_time_limit_seconds',
                assessment.assessment_attempt_time_limit_seconds,
            'assessment_attempt_limit', assessment.assessment_attempt_limit,
            'late_work_rule', assessment.late_work_rule,
            'activity_rules', jsonb_build_object(
                'questionVariationRule', CASE assessment.question_variation_rule
                    WHEN 'reuse_variation' THEN 'reuseVariation'
                    WHEN 'new_variation' THEN 'newVariation'
                END,
                'assessmentQuestionOrderRule', CASE assessment.assessment_question_order_rule
                    WHEN 'authored_order' THEN 'authoredOrder'
                    WHEN 'shuffled' THEN 'shuffled'
                END
            ),
            'student_feedback_release_rule', jsonb_build_object(
                'score', assessment.feedback_score,
                'per_item_correctness', assessment.feedback_per_item_correctness,
                'submitted_response', assessment.feedback_submitted_response,
                'question_answer', assessment.feedback_question_answer,
                'question_answer_explanation', assessment.feedback_question_answer_explanation,
                'class_statistics', assessment.feedback_class_statistics
            )
        )
    )
      FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = p_assessment_id
$$;

-- Edit Numbers bind all current Assessment content. Pool pins are repeated in
-- the snapshot because Pool membership is copied through a separate immutable
-- fork primitive inside the same transaction.
CREATE FUNCTION ple_data.course_blueprint_publication_snapshot(p_course_id uuid)
RETURNS jsonb LANGUAGE sql STABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT COALESCE(jsonb_agg(jsonb_build_object(
        'assessmentId', assessment.assessment_id,
        'assessmentEditNumber', assessment.assessment_edit_number,
        'poolPins', COALESCE((
            SELECT jsonb_agg(jsonb_build_object(
                'assessmentEntryId', entry.assessment_entry_id,
                'questionPoolId', entry.question_pool_id,
                'revisionNumber', entry.question_pool_revision_number
            ) ORDER BY entry.authored_position)
              FROM ple_data.assessment_entry AS entry
             WHERE entry.assessment_id = assessment.assessment_id
               AND entry.availability = 'available'
               AND entry.entry_kind = 'question_pool'
        ), '[]'::jsonb)
    ) ORDER BY assessment.due_at NULLS LAST, assessment.reference_number), '[]'::jsonb)
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_id = p_course_id
$$;

REVOKE ALL ON FUNCTION
    ple_data.course_blueprint_publication_assessment_content(uuid),
    ple_data.course_blueprint_publication_snapshot(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_data.course_blueprint_publication_assessment_content(uuid),
    ple_data.course_blueprint_publication_snapshot(uuid) TO ple_api_owner;

RESET ROLE;
SET LOCAL ROLE ple_api_owner;

-- ASVS 2.3.1/2.3.3 and 8.2.1/8.2.2: the session-authorized source read
-- obtains the Course and every Assessment row lock before returning reusable
-- structure. A concurrent Course metadata or Assessment save must finish
-- before this snapshot, or wait until the complete publication commits.
CREATE FUNCTION ple_api.load_course_blueprint_publication_source(p_course_reference text)
RETURNS TABLE (
    course_metadata_etag uuid,
    source_snapshot jsonb,
    source_assessments jsonb
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE
    source_course ple_data.course_instance%ROWTYPE;
BEGIN
    SELECT * INTO source_course
      FROM ple_data.course_instance AS course
     WHERE course.public_reference = p_course_reference
     FOR UPDATE;
    IF NOT FOUND
       OR NOT ple_api.current_session_account_is_course_instructor(source_course.course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Instance reusable structure is unavailable';
    END IF;

    PERFORM 1
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_id = source_course.course_id
     ORDER BY assessment.due_at NULLS LAST, assessment.reference_number
     FOR UPDATE;

    -- New Blueprint pins use the same Available-Published-Question predicate
    -- as ordinary Blueprint creation, including every exact Pool member pin.
    PERFORM ple_data.lock_course_blueprint_publication_questions(source_course.course_id);

    course_metadata_etag := source_course.metadata_etag;
    source_snapshot := ple_data.course_blueprint_publication_snapshot(source_course.course_id);
    SELECT COALESCE(jsonb_agg(jsonb_build_object(
        'content', ple_data.course_blueprint_publication_assessment_content(
            assessment.assessment_id
        )
    ) ORDER BY assessment.due_at NULLS LAST, assessment.reference_number), '[]'::jsonb)
      INTO source_assessments
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_id = source_course.course_id;
    RETURN NEXT;
END
$$;

-- Resolve an accepted retry before the Store allocates fresh Blueprint or
-- Pool identities. The route's domain-separated checksum prevents overlap
-- with ordinary Blueprint creation and canonical import.
CREATE FUNCTION ple_api.course_blueprint_publication_receipt(
    p_course_reference text,
    p_request_checksum bytea
)
RETURNS TABLE (
    public_reference text,
    blueprint_revision_number bigint,
    metadata_etag uuid,
    accepted_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE
    actor_id uuid;
    source_course_id uuid;
    prior_source_course_id uuid;
BEGIN
    IF p_request_checksum IS NULL
       OR octet_length(p_request_checksum) <> 32
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Blueprint idempotency lookup is invalid';
    END IF;
    actor_id := ple_api.current_session_account_id();
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('ple:blueprint-course-create:%s:%s', actor_id,
            pg_catalog.encode(p_request_checksum, 'hex')), 0));
    SELECT course.course_id INTO source_course_id
      FROM ple_data.course_instance AS course
     WHERE course.public_reference = p_course_reference;
    SELECT blueprint.public_reference, receipt.blueprint_revision_number,
           receipt.metadata_etag, receipt.accepted_at, source.source_course_id
      INTO public_reference, blueprint_revision_number, metadata_etag, accepted_at,
           prior_source_course_id
      FROM ple_data.blueprint_course_create_receipt AS receipt
      JOIN ple_data.blueprint_course AS blueprint
        ON blueprint.reference_number = receipt.blueprint_course_reference_number
      LEFT JOIN ple_data.blueprint_course_instance_source AS source
        ON source.blueprint_course_reference_number = receipt.blueprint_course_reference_number
     WHERE receipt.actor_account_id = actor_id
       AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN
        IF source_course_id IS NULL
           OR prior_source_course_id IS DISTINCT FROM source_course_id THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Idempotency-Key belongs to another Blueprint operation';
        END IF;
        RETURN NEXT;
    END IF;
END
$$;

-- The Store supplies content derived from the locked source above. This final
-- capability rechecks the exact snapshot, delegates ordinary Revision-1
-- construction, and records source provenance before the transaction commits.
CREATE FUNCTION ple_api.create_blueprint_from_course_instance(
    p_course_reference text,
    p_expected_course_metadata_etag uuid,
    p_expected_source_snapshot jsonb,
    p_blueprint_id uuid,
    p_request_checksum bytea,
    p_short_name text,
    p_long_name text,
    p_content jsonb,
    p_content_checksum bytea,
    p_discipline uuid,
    p_subject uuid,
    p_topic uuid,
    p_subtopic uuid,
    p_tags text[]
)
RETURNS TABLE (
    public_reference text,
    blueprint_revision_number bigint,
    metadata_etag uuid,
    accepted_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    actor_id uuid;
    source_course ple_data.course_instance%ROWTYPE;
    prior_receipt record;
    created record;
    created_reference_number bigint;
BEGIN
    IF p_request_checksum IS NULL
       OR octet_length(p_request_checksum) <> 32
       OR p_expected_course_metadata_etag IS NULL
       OR jsonb_typeof(p_expected_source_snapshot) IS DISTINCT FROM 'array'
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Blueprint creation is invalid';
    END IF;
    actor_id := ple_api.current_session_account_id();
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('ple:blueprint-course-create:%s:%s', actor_id,
            pg_catalog.encode(p_request_checksum, 'hex')), 0));

    SELECT blueprint.public_reference, receipt.blueprint_revision_number,
           receipt.metadata_etag, receipt.accepted_at, source.source_course_id
      INTO prior_receipt
      FROM ple_data.blueprint_course_create_receipt AS receipt
      JOIN ple_data.blueprint_course AS blueprint
        ON blueprint.reference_number = receipt.blueprint_course_reference_number
      LEFT JOIN ple_data.blueprint_course_instance_source AS source
        ON source.blueprint_course_reference_number = receipt.blueprint_course_reference_number
     WHERE receipt.actor_account_id = actor_id
       AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN
        SELECT * INTO source_course
          FROM ple_data.course_instance AS course
         WHERE course.public_reference = p_course_reference;
        IF NOT FOUND OR prior_receipt.source_course_id IS DISTINCT FROM source_course.course_id THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Idempotency-Key belongs to another Blueprint operation';
        END IF;
        public_reference := prior_receipt.public_reference;
        blueprint_revision_number := prior_receipt.blueprint_revision_number;
        metadata_etag := prior_receipt.metadata_etag;
        accepted_at := prior_receipt.accepted_at;
        RETURN NEXT;
        RETURN;
    END IF;

    SELECT * INTO source_course
      FROM ple_data.course_instance AS course
     WHERE course.public_reference = p_course_reference
     FOR UPDATE;
    IF NOT FOUND
       OR NOT ple_api.current_session_account_is_course_instructor(source_course.course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Instance reusable structure is unavailable';
    END IF;
    PERFORM 1
      FROM ple_data.assessment AS assessment
     WHERE assessment.course_id = source_course.course_id
     ORDER BY assessment.due_at NULLS LAST, assessment.reference_number
     FOR UPDATE;
    IF source_course.metadata_etag IS DISTINCT FROM p_expected_course_metadata_etag
       OR ple_data.course_blueprint_publication_snapshot(source_course.course_id)
            IS DISTINCT FROM p_expected_source_snapshot THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Course reusable structure changed during Blueprint creation';
    END IF;
    -- Recheck and lock eligibility at the final write boundary. The earlier
    -- projection check is not accepted as authority because Question
    -- availability changes independently of the locked Course rows.
    PERFORM ple_data.lock_course_blueprint_publication_questions(source_course.course_id);

    SELECT * INTO created FROM ple_api.create_blueprint_course(
        p_blueprint_id, p_request_checksum, p_short_name, p_long_name,
        p_content, p_content_checksum, p_discipline, p_subject, p_topic,
        p_subtopic, p_tags
    );
    SELECT blueprint.reference_number INTO created_reference_number
      FROM ple_data.blueprint_course AS blueprint
     WHERE blueprint.public_reference = created.public_reference;
    INSERT INTO ple_data.blueprint_course_instance_source (
        blueprint_course_reference_number, source_course_id, recorded_at
    ) VALUES (
        created_reference_number, source_course.course_id, created.accepted_at
    );
    public_reference := created.public_reference;
    blueprint_revision_number := created.blueprint_revision_number;
    metadata_etag := created.metadata_etag;
    accepted_at := created.accepted_at;
    RETURN NEXT;
END
$$;

REVOKE ALL ON FUNCTION
    ple_api.load_course_blueprint_publication_source(text),
    ple_api.course_blueprint_publication_receipt(text, bytea),
    ple_api.create_blueprint_from_course_instance(
        text, uuid, jsonb, uuid, bytea, text, text, jsonb, bytea,
        uuid, uuid, uuid, uuid, text[]
    ) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.load_course_blueprint_publication_source(text),
    ple_api.course_blueprint_publication_receipt(text, bytea),
    ple_api.create_blueprint_from_course_instance(
        text, uuid, jsonb, uuid, bytea, text, text, jsonb, bytea,
        uuid, uuid, uuid, uuid, text[]
    ) TO ple_app;

RESET ROLE;
