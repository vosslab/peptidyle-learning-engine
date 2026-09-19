-- Functions, triggers, and views from course_blueprint_publication.sql.

SET LOCAL ROLE ple_data_owner;

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



-- Lock every selected Question lineage in canonical order, then apply the
-- ordinary Available-Published-Question predicate to fixed pins and every
-- exact Pool member. The data-owner capability avoids granting the API owner
-- a general Question update privilege merely to obtain row locks.
CREATE FUNCTION ple_data.lock_course_blueprint_publication_questions(p_course_instance_id text)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_course_instructor(p_course_instance_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Instance reusable structure is unavailable';
    END IF;
    PERFORM question.published_question_id
      FROM ple_data.published_question AS question
     WHERE question.published_question_id IN (
               SELECT question.published_question_id
                 FROM ple_data.assessment AS assessment
                 JOIN ple_data.assessment_entry AS entry
                   ON entry.assessment_id = assessment.assessment_id
                 JOIN ple_data.assessment_entry_question AS question
                   ON question.assessment_entry_id = entry.assessment_entry_id
                WHERE assessment.course_instance_id = p_course_instance_id
                  AND entry.availability = 'available'
                  AND entry.entry_kind = 'fixed_question'
               UNION
               SELECT member.published_question_id
                 FROM ple_data.assessment AS assessment
                 JOIN ple_data.assessment_entry AS entry
                   ON entry.assessment_id = assessment.assessment_id
                 JOIN ple_data.assessment_entry_pool AS pool_entry
                   ON pool_entry.assessment_entry_id = entry.assessment_entry_id
                 JOIN ple_data.question_pool_member AS member
                   ON member.question_pool_id = pool_entry.question_pool_id
                WHERE assessment.course_instance_id = p_course_instance_id
                  AND entry.availability = 'available'
                  AND entry.entry_kind = 'question_pool'
           )
     ORDER BY question.published_question_id
     FOR SHARE OF question;
    IF EXISTS (
        SELECT 1
          FROM ple_data.assessment AS assessment
          JOIN ple_data.assessment_entry AS entry
            ON entry.assessment_id = assessment.assessment_id
          JOIN ple_data.assessment_entry_question AS entry_question
            ON entry_question.assessment_entry_id = entry.assessment_entry_id
          LEFT JOIN ple_data.published_question AS question
            ON question.published_question_id = entry_question.published_question_id
         WHERE assessment.course_instance_id = p_course_instance_id
           AND entry.availability = 'available'
           AND entry.entry_kind = 'fixed_question'
           AND question.availability IS DISTINCT FROM 'available'
    ) OR EXISTS (
        SELECT 1
          FROM ple_data.assessment AS assessment
          JOIN ple_data.assessment_entry AS entry
            ON entry.assessment_id = assessment.assessment_id
          JOIN ple_data.assessment_entry_pool AS pool_entry
            ON pool_entry.assessment_entry_id = entry.assessment_entry_id
          JOIN ple_data.question_pool_member AS member
            ON member.question_pool_id = pool_entry.question_pool_id
          LEFT JOIN ple_data.published_question AS question
            ON question.published_question_id = member.published_question_id
         WHERE assessment.course_instance_id = p_course_instance_id
           AND entry.availability = 'available'
           AND entry.entry_kind = 'question_pool'
           AND question.availability IS DISTINCT FROM 'available'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course reusable structure requires Available Published Questions';
    END IF;
END
$$;



-- Exact reusable Assessment content. Dates, lifecycle state, origin, Course
-- identity, roster, and Student Work have no representation in this projection.
CREATE FUNCTION ple_data.course_blueprint_publication_assessment_content(p_assessment_id text)
RETURNS jsonb LANGUAGE sql STABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT jsonb_build_object(
        'assessment_type', assessment.assessment_type,
        'title', policy.assessment_title,
        'instructions', policy.assessment_instructions,
        'entries', COALESCE((
            SELECT jsonb_agg(
                CASE entry.entry_kind
                    WHEN 'fixed_question' THEN jsonb_build_object(
                        'kind', 'fixed',
                        'question_revision', jsonb_build_object(
                            'questionId', question.published_question_id,
                            'revisionNumber', question.question_revision_number
                        ),
                        'points_possible', question.points_possible::text,
                        'scoring_rule', CASE entry.scoring_rule::text
                            WHEN 'full_credit' THEN 'fullCredit'
                            WHEN 'extra_credit' THEN 'extraCredit'
                            ELSE entry.scoring_rule::text
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
                        'question_pool_id', pool.question_pool_id,
                        'question_pool_edit_number', pool.question_pool_edit_number,
                        'selection_count', pool_entry.selection_count,
                        'points_per_item', pool_entry.points_per_item::text,
                        'scoring_rule', CASE entry.scoring_rule::text
                            WHEN 'full_credit' THEN 'fullCredit'
                            WHEN 'extra_credit' THEN 'extraCredit'
                            ELSE entry.scoring_rule::text
                        END,
                        'selection_rule', jsonb_build_object(
                            'selectedQuestionOrder', CASE pool_entry.selected_question_order
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
              LEFT JOIN ple_data.assessment_entry_question AS question
                ON question.assessment_entry_id = entry.assessment_entry_id
              LEFT JOIN ple_data.assessment_entry_pool AS pool_entry
                ON pool_entry.assessment_entry_id = entry.assessment_entry_id
              LEFT JOIN ple_data.question_pool AS pool
                ON pool.question_pool_id = pool_entry.question_pool_id
             WHERE entry.assessment_id = assessment.assessment_id
               AND entry.availability = 'available'
        ), '[]'::jsonb),
        'defaults', jsonb_build_object(
            'assessment_attempt_time_limit_seconds',
                policy.assessment_attempt_time_limit_seconds,
            'assessment_attempt_limit', policy.assessment_attempt_limit,
            'late_work_rule', policy.late_work_rule,
            'activity_rules', jsonb_build_object(
                'questionVariationRule', CASE policy.question_variation_rule
                    WHEN 'reuse_variation' THEN 'reuseVariation'
                    WHEN 'new_variation' THEN 'newVariation'
                END,
                'assessmentQuestionOrderRule', CASE policy.assessment_question_order_rule
                    WHEN 'authored_order' THEN 'authoredOrder'
                    WHEN 'shuffled' THEN 'shuffled'
                END
            ),
            'student_feedback_release_rule', jsonb_build_object(
                'score', policy.feedback_score,
                'per_item_correctness', policy.feedback_per_item_correctness,
                'submitted_response', policy.feedback_submitted_response,
                'question_answer', policy.feedback_question_answer,
                'question_answer_explanation', policy.feedback_question_answer_explanation,
                'class_statistics', policy.feedback_class_statistics
            )
        )
    )
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.assessment_id = p_assessment_id
$$;



-- Edit Numbers bind all current Assessment content. Pool ID plus Pool Edit
-- Number are recorded in the snapshot as concurrency evidence, not as a
-- historical Pool membership object.
CREATE FUNCTION ple_data.course_blueprint_publication_snapshot(p_course_instance_id text)
RETURNS jsonb LANGUAGE sql STABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT COALESCE(jsonb_agg(jsonb_build_object(
        'assessmentId', assessment.assessment_id,
        'assessmentEditNumber', assessment.assessment_edit_number,
        'poolEvidence', COALESCE((
            SELECT jsonb_agg(jsonb_build_object(
                'assessmentEntryId', entry.assessment_entry_id,
                'questionPoolId', pool_entry.question_pool_id,
                'questionPoolEditNumber', (
                    SELECT pool.question_pool_edit_number
                      FROM ple_data.question_pool AS pool
                     WHERE pool.question_pool_id = pool_entry.question_pool_id
                )
            ) ORDER BY entry.authored_position)
              FROM ple_data.assessment_entry AS entry
              JOIN ple_data.assessment_entry_pool AS pool_entry
                ON pool_entry.assessment_entry_id = entry.assessment_entry_id
             WHERE entry.assessment_id = assessment.assessment_id
               AND entry.availability = 'available'
               AND entry.entry_kind = 'question_pool'
        ), '[]'::jsonb)
    ) ORDER BY policy.due_at NULLS LAST, assessment.assessment_id), '[]'::jsonb)
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.course_instance_id = p_course_instance_id
$$;

SET LOCAL ROLE ple_api_owner;






-- ASVS 2.3.1/2.3.3 and 8.2.1/8.2.2: the session-authorized source read
-- obtains the Course and every Assessment row lock before returning reusable
-- structure. A concurrent Course metadata or Assessment save must finish
-- before this snapshot, or wait until the complete publication commits.
CREATE FUNCTION ple_api.load_course_blueprint_publication_source(p_course_instance_id text)
RETURNS TABLE (
    course_edit_number bigint,
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
     WHERE course.course_instance_id = p_course_instance_id
     FOR UPDATE;
    IF NOT FOUND
       OR NOT ple_api.current_session_account_is_course_instructor(source_course.course_instance_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Instance reusable structure is unavailable';
    END IF;

    PERFORM 1
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.course_instance_id = source_course.course_instance_id
     ORDER BY policy.due_at NULLS LAST, assessment.assessment_id
     FOR UPDATE OF assessment;

    -- New Blueprint pins use the same Available-Published-Question predicate
    -- as ordinary Blueprint creation, including every exact Pool member pin.
    PERFORM ple_data.lock_course_blueprint_publication_questions(source_course.course_instance_id);

    course_edit_number := source_course.blueprint_edit_number;
    source_snapshot := ple_data.course_blueprint_publication_snapshot(source_course.course_instance_id);
    SELECT COALESCE(jsonb_agg(jsonb_build_object(
        'content', ple_data.course_blueprint_publication_assessment_content(
            assessment.assessment_id
        )
    ) ORDER BY policy.due_at NULLS LAST, assessment.assessment_id), '[]'::jsonb)
      INTO source_assessments
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.course_instance_id = source_course.course_instance_id;
    RETURN NEXT;
END
$$;



-- Resolve an accepted retry before the Store allocates fresh Blueprint or
-- Pool identities. The route's domain-separated checksum prevents overlap
-- with ordinary Blueprint creation and canonical import.
CREATE FUNCTION ple_api.course_blueprint_publication_receipt(
    p_course_instance_id text,
    p_request_checksum bytea
)
RETURNS TABLE (
    blueprint_course_id text,
    blueprint_revision_number bigint,
    blueprint_edit_number bigint,
    accepted_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
DECLARE
    actor_id text;
    source_course_instance_id text;
    prior_source_course_id text;
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
    SELECT course.course_instance_id INTO source_course_instance_id
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id;
    SELECT blueprint.blueprint_course_id, receipt.blueprint_revision_number,
           receipt.blueprint_edit_number, receipt.accepted_at, source.source_course_instance_id
      INTO blueprint_course_id, blueprint_revision_number, blueprint_edit_number, accepted_at,
           prior_source_course_id
      FROM ple_data.blueprint_course_create_receipt AS receipt
      JOIN ple_data.blueprint_course AS blueprint
        ON blueprint.blueprint_course_id = receipt.blueprint_course_id
      LEFT JOIN ple_data.blueprint_course_instance_source AS source
        ON source.blueprint_course_id = receipt.blueprint_course_id
     WHERE receipt.actor_account_id = actor_id
       AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN
        IF source_course_instance_id IS NULL
           OR prior_source_course_id IS DISTINCT FROM source_course_instance_id THEN
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
    p_course_instance_id text,
    p_expected_course_edit_number bigint,
    p_expected_source_snapshot jsonb,
    p_blueprint_course_id text,
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
    blueprint_course_id text,
    blueprint_revision_number bigint,
    blueprint_edit_number bigint,
    accepted_at timestamp with time zone
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    actor_id text;
    source_course ple_data.course_instance%ROWTYPE;
    prior_receipt record;
    created record;
    created_blueprint_course_id text;
BEGIN
    IF p_request_checksum IS NULL
       OR octet_length(p_request_checksum) <> 32
       OR p_expected_course_edit_number IS NULL
       OR jsonb_typeof(p_expected_source_snapshot) IS DISTINCT FROM 'array'
       OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Blueprint creation is invalid';
    END IF;
    actor_id := ple_api.current_session_account_id();
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('ple:blueprint-course-create:%s:%s', actor_id,
            pg_catalog.encode(p_request_checksum, 'hex')), 0));

    SELECT blueprint.blueprint_course_id, receipt.blueprint_revision_number,
           receipt.blueprint_edit_number, receipt.accepted_at, source.source_course_instance_id
      INTO prior_receipt
      FROM ple_data.blueprint_course_create_receipt AS receipt
      JOIN ple_data.blueprint_course AS blueprint
        ON blueprint.blueprint_course_id = receipt.blueprint_course_id
      LEFT JOIN ple_data.blueprint_course_instance_source AS source
        ON source.blueprint_course_id = receipt.blueprint_course_id
     WHERE receipt.actor_account_id = actor_id
       AND receipt.request_checksum = p_request_checksum;
    IF FOUND THEN
        SELECT * INTO source_course
          FROM ple_data.course_instance AS course
         WHERE course.course_instance_id = p_course_instance_id;
        IF NOT FOUND OR prior_receipt.source_course_instance_id IS DISTINCT FROM source_course.course_instance_id THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Idempotency-Key belongs to another Blueprint operation';
        END IF;
        blueprint_course_id := prior_receipt.blueprint_course_id;
        blueprint_revision_number := prior_receipt.blueprint_revision_number;
        blueprint_edit_number := prior_receipt.blueprint_edit_number;
        accepted_at := prior_receipt.accepted_at;
        RETURN NEXT;
        RETURN;
    END IF;

    SELECT * INTO source_course
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id
     FOR UPDATE;
    IF NOT FOUND
       OR NOT ple_api.current_session_account_is_course_instructor(source_course.course_instance_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Instance reusable structure is unavailable';
    END IF;
    PERFORM 1
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.course_instance_id = source_course.course_instance_id
     ORDER BY policy.due_at NULLS LAST, assessment.assessment_id
     FOR UPDATE OF assessment;
    IF source_course.blueprint_edit_number IS DISTINCT FROM p_expected_course_edit_number
       OR ple_data.course_blueprint_publication_snapshot(source_course.course_instance_id)
            IS DISTINCT FROM p_expected_source_snapshot THEN
        RAISE EXCEPTION USING ERRCODE = '40001',
            MESSAGE = 'Course reusable structure changed during Blueprint creation';
    END IF;
    -- Recheck and lock eligibility at the final write boundary. The earlier
    -- projection check is not accepted as authority because Question
    -- availability changes independently of the locked Course rows.
    PERFORM ple_data.lock_course_blueprint_publication_questions(source_course.course_instance_id);

    -- Only this locked Course-copy path can retain an exact retired source
    -- Discipline. The private primitive is unavailable to ple_app, and an
    -- altered caller-supplied UUID follows the ordinary active-only wrapper.
    IF p_discipline IS NOT DISTINCT FROM source_course.content_discipline_id
       AND EXISTS (
           SELECT 1 FROM ple_api.get_content_discipline(source_course.content_discipline_id) AS discipline
            WHERE discipline.content_discipline_id = source_course.content_discipline_id
              AND discipline.is_retired
       ) THEN
        SELECT * INTO created FROM ple_private.create_blueprint_course(
            p_blueprint_course_id, p_request_checksum, p_short_name, p_long_name,
            p_content, p_content_checksum, p_discipline, p_subject, p_topic,
            p_subtopic, p_tags, source_course.content_discipline_id
        );
    ELSE
        SELECT * INTO created FROM ple_api.create_blueprint_course(
            p_blueprint_course_id, p_request_checksum, p_short_name, p_long_name,
            p_content, p_content_checksum, p_discipline, p_subject, p_topic,
            p_subtopic, p_tags
        );
    END IF;
    SELECT blueprint.blueprint_course_id INTO created_blueprint_course_id
      FROM ple_data.blueprint_course AS blueprint
     WHERE blueprint.blueprint_course_id = created.blueprint_course_id;
    INSERT INTO ple_data.blueprint_course_instance_source (
        blueprint_course_id, source_course_instance_id, recorded_at
    ) VALUES (
        created_blueprint_course_id, source_course.course_instance_id, created.accepted_at
    );
    blueprint_course_id := created.blueprint_course_id;
    blueprint_revision_number := created.blueprint_revision_number;
    blueprint_edit_number := created.blueprint_edit_number;
    accepted_at := created.accepted_at;
    RETURN NEXT;
END
$$;

