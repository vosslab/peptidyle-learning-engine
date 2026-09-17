-- Reusable Blueprint Course lineages and immutable save-created Revisions.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;

-- ASVS 2.2.1/2.2.3: canonical set with no Course Tag-count ceiling.
CREATE FUNCTION ple_data.course_classification_tags_are_valid(p_tags text[])
RETURNS boolean LANGUAGE sql IMMUTABLE SET search_path = pg_catalog AS $$
    SELECT p_tags IS NOT NULL
       AND NOT EXISTS (SELECT 1 FROM unnest(p_tags) AS tag(value)
            WHERE value IS NULL OR value <> btrim(value)
               OR char_length(value) NOT BETWEEN 1 AND 120 OR value ~ '[[:cntrl:]]')
       AND cardinality(p_tags) = cardinality(
           ARRAY(SELECT DISTINCT value FROM unnest(p_tags) AS tag(value)));
$$;
REVOKE ALL ON FUNCTION ple_data.course_classification_tags_are_valid(text[]) FROM PUBLIC;
-- Course retention updates are checked against this shared immutable validator.
GRANT EXECUTE ON FUNCTION ple_data.course_classification_tags_are_valid(text[])
    TO ple_api_owner, ple_course_retention_executor;

CREATE TABLE ple_data.blueprint_course (
    blueprint_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE NOT NULL,
    public_reference text NOT NULL UNIQUE CHECK (
        ple_private.is_canonical_prefixed_public_id(public_reference, 'BP')
    ),
    owner_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    short_name text NOT NULL CHECK (char_length(btrim(short_name)) BETWEEN 1 AND 500),
    long_name text NOT NULL CHECK (char_length(btrim(long_name)) BETWEEN 1 AND 500),
    discipline_uuid uuid NOT NULL REFERENCES ple_data.content_discipline(discipline_uuid),
    subject_uuid uuid,
    topic_uuid uuid,
    subtopic_uuid uuid,
    tags text[] NOT NULL CHECK (ple_data.course_classification_tags_are_valid(tags)),
    CHECK (topic_uuid IS NULL OR subject_uuid IS NOT NULL),
    CHECK (subtopic_uuid IS NULL OR topic_uuid IS NOT NULL),
    FOREIGN KEY (subject_uuid, discipline_uuid)
        REFERENCES ple_data.content_subject_discipline(subject_uuid, discipline_uuid),
    FOREIGN KEY (subject_uuid, topic_uuid)
        REFERENCES ple_data.content_topic(subject_uuid, topic_uuid),
    FOREIGN KEY (topic_uuid, subtopic_uuid)
        REFERENCES ple_data.content_subtopic(topic_uuid, subtopic_uuid),
    -- A new reusable Blueprint is owner-only until its owner explicitly makes
    -- it Public. Public is the sole state eligible for discovery/adoption;
    -- Archived remains readable by exact historical Revision reference only.
    availability text NOT NULL DEFAULT 'private'
        CHECK (availability IN ('private', 'public', 'archived')),
    promoted boolean NOT NULL DEFAULT false,
    metadata_etag uuid NOT NULL,
    current_blueprint_revision_number bigint NOT NULL DEFAULT 1
        CHECK (current_blueprint_revision_number > 0),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT blueprint_course_reference_is_bounded CHECK (
        reference_number BETWEEN 1 AND 2147483647
    )
);

CREATE TRIGGER blueprint_course_human_reference_is_minted
BEFORE INSERT ON ple_data.blueprint_course
FOR EACH ROW EXECUTE FUNCTION ple_private.assign_human_reference('BP');

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

-- Module and Assessment References are durable identities across Revisions.
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

CREATE TABLE ple_data.blueprint_revision_assessment (
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    blueprint_module_reference uuid NOT NULL,
    blueprint_assessment_reference uuid NOT NULL,
    assessment_position integer NOT NULL CHECK (assessment_position BETWEEN 1 AND 1024),
    PRIMARY KEY (
        blueprint_course_reference_number, blueprint_revision_number,
        blueprint_assessment_reference
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
        blueprint_module_reference, assessment_position
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
    -- Immutable exact snapshots, not joins to today's vocabulary associations.
    discipline_uuid uuid NOT NULL,
    subject_uuid uuid,
    topic_uuid uuid,
    subtopic_uuid uuid,
    tags text[] NOT NULL CHECK (ple_data.course_classification_tags_are_valid(tags)),
    availability text NOT NULL CHECK (availability IN ('private', 'public', 'archived')),
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
        SELECT module_ordinality, assessment_ordinality, entry_ordinality, entry
          FROM pg_catalog.jsonb_array_elements(p_content -> 'modules')
                 WITH ORDINALITY AS module_row(module, module_ordinality)
          CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(module_row.module -> 'assessments')
                 WITH ORDINALITY AS assessment_row(assessment, assessment_ordinality)
          CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(
              assessment_row.assessment -> 'content' -> 'entries'
          ) WITH ORDINALITY AS entry_row(entry, entry_ordinality)
    ), pins AS (
        SELECT module_ordinality, assessment_ordinality, entry_ordinality,
               entry -> 'question_revision' AS pin
          FROM entries WHERE entry ->> 'kind' = 'fixed'
    )
    SELECT pg_catalog.format('m%s.a%s.e%s.p%s', module_ordinality,
               assessment_ordinality, entry_ordinality, 0),
           pin ->> 'questionId',
           (pin ->> 'revisionNumber')::bigint
      FROM pins
$$;

-- A Pool entry names one exact reusable Pool Revision. Its ordered Question
-- member pins live in that immutable Pool Revision, rather than being copied
-- into Blueprint JSON as a second representation.
CREATE FUNCTION ple_data.blueprint_content_pool_pins(p_content jsonb)
RETURNS TABLE (content_path text, public_question_pool_id text, question_pool_revision_number bigint)
LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT pg_catalog.format('m%s.a%s.e%s', module_ordinality,
               assessment_ordinality, entry_ordinality),
           entry #>> '{question_pool_revision,questionPoolId}',
           (entry #>> '{question_pool_revision,revisionNumber}')::bigint
      FROM pg_catalog.jsonb_array_elements(p_content -> 'modules')
             WITH ORDINALITY AS module_row(module, module_ordinality)
      CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(module_row.module -> 'assessments')
             WITH ORDINALITY AS assessment_row(assessment, assessment_ordinality)
      CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(
          assessment_row.assessment -> 'content' -> 'entries'
      ) WITH ORDINALITY AS entry_row(entry, entry_ordinality)
     WHERE entry ->> 'kind' = 'pool'
$$;

CREATE FUNCTION ple_data.blueprint_content_assessments(p_content jsonb)
RETURNS TABLE (
    blueprint_module_reference uuid, blueprint_assessment_reference uuid,
    module_position integer, assessment_position integer
)
LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT (module_row.module ->> 'blueprint_module_reference')::uuid,
           (assessment_row.assessment ->> 'blueprint_assessment_reference')::uuid,
           module_row.module_ordinality::integer,
           assessment_row.assessment_ordinality::integer
      FROM pg_catalog.jsonb_array_elements(p_content -> 'modules')
             WITH ORDINALITY AS module_row(module, module_ordinality)
      CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(module_row.module -> 'assessments')
             WITH ORDINALITY AS assessment_row(assessment, assessment_ordinality)
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

-- Blueprint content is a closed, bounded reusable-curriculum shape.  Course
-- Instance delivery, schedule, roster, response, and grade state have no
-- extension point here: the Store's typed codec and this persistence boundary
-- agree on every object node rather than relying on a growing deny-list.
CREATE FUNCTION ple_data.blueprint_content_has_exact_keys(p_value jsonb, p_keys text[])
RETURNS boolean LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
    SELECT CASE WHEN jsonb_typeof(p_value) <> 'object' THEN false ELSE
        NOT EXISTS (
            SELECT 1 FROM jsonb_object_keys(p_value) AS actual WHERE actual <> ALL (p_keys)
        )
        AND NOT EXISTS (SELECT 1 FROM unnest(p_keys) AS required WHERE NOT p_value ? required)
    END
$$;

CREATE FUNCTION ple_data.blueprint_content_is_closed(p_content jsonb)
RETURNS boolean LANGUAGE plpgsql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
DECLARE
    module_value jsonb;
    assessment_value jsonb;
    content_value jsonb;
    entry_value jsonb;
    pin_value jsonb;
    defaults_value jsonb;
    activity_value jsonb;
    feedback_value jsonb;
    limit_value jsonb;
    time_limit_value jsonb;
    selection_value jsonb;
    delivered_question_count bigint;
BEGIN
    IF NOT ple_data.blueprint_content_has_exact_keys(p_content, ARRAY['modules'])
       OR jsonb_typeof(p_content -> 'modules') <> 'array'
       OR jsonb_array_length(p_content -> 'modules') > 1024 THEN
        RETURN false;
    END IF;
    FOR module_value IN SELECT value FROM jsonb_array_elements(p_content -> 'modules') LOOP
        IF NOT ple_data.blueprint_content_has_exact_keys(
            module_value, ARRAY['blueprint_module_reference', 'label', 'assessments']
        ) OR jsonb_typeof(module_value -> 'blueprint_module_reference') <> 'string'
          OR jsonb_typeof(module_value -> 'label') <> 'string'
          OR jsonb_typeof(module_value -> 'assessments') <> 'array'
          OR jsonb_array_length(module_value -> 'assessments') NOT BETWEEN 1 AND 1024 THEN
            RETURN false;
        END IF;
        FOR assessment_value IN
            SELECT value FROM jsonb_array_elements(module_value -> 'assessments') LOOP
            IF NOT ple_data.blueprint_content_has_exact_keys(
                assessment_value, ARRAY['blueprint_assessment_reference', 'content']
            ) OR jsonb_typeof(assessment_value -> 'blueprint_assessment_reference') <> 'string'
              OR jsonb_typeof(assessment_value -> 'content') <> 'object' THEN
                RETURN false;
            END IF;
            content_value := assessment_value -> 'content';
            IF NOT ple_data.blueprint_content_has_exact_keys(
                content_value, ARRAY['assessment_type', 'title', 'instructions', 'entries', 'defaults']
            ) OR content_value ->> 'assessment_type' NOT IN (
                'regular_assignment', 'practice_question_assignment', 'bonus_assignment', 'quiz', 'exam'
            ) OR jsonb_typeof(content_value -> 'title') <> 'string'
              OR jsonb_typeof(content_value -> 'instructions') <> 'string'
              OR jsonb_typeof(content_value -> 'entries') <> 'array'
              OR jsonb_array_length(content_value -> 'entries') > 1024 THEN
                RETURN false;
            END IF;
            defaults_value := content_value -> 'defaults';
            IF NOT ple_data.blueprint_content_has_exact_keys(defaults_value, ARRAY[
                'assessment_attempt_time_limit_seconds', 'assessment_attempt_limit', 'late_work_rule',
                'activity_rules', 'student_feedback_release_rule'
            ]) OR (
                content_value ->> 'assessment_type' IN ('quiz', 'exam')
                AND defaults_value -> 'assessment_attempt_limit' IS DISTINCT FROM '1'::jsonb
            ) THEN
                RETURN false;
            END IF;
            -- ASVS 2.2.1, 2.2.2: NULL means the calculated default, never Unlimited.
            IF defaults_value -> 'assessment_attempt_time_limit_seconds' <> 'null'::jsonb THEN
                IF jsonb_typeof(defaults_value -> 'assessment_attempt_time_limit_seconds') <> 'number'
                   OR defaults_value ->> 'assessment_attempt_time_limit_seconds' !~ '^[1-9][0-9]*$'
                   OR (defaults_value ->> 'assessment_attempt_time_limit_seconds')::numeric > 43200 THEN
                    RETURN false;
                END IF;
            END IF;
            delivered_question_count := 0;
            activity_value := defaults_value -> 'activity_rules';
            feedback_value := defaults_value -> 'student_feedback_release_rule';
            IF NOT ple_data.blueprint_content_has_exact_keys(activity_value, ARRAY[
                'questionVariationRule',
                'assessmentQuestionOrderRule'
            ]) OR NOT ple_data.blueprint_content_has_exact_keys(feedback_value, ARRAY[
                'score', 'per_item_correctness', 'submitted_response',
                'question_answer', 'question_answer_explanation', 'class_statistics'
            ]) THEN
                RETURN false;
            END IF;
            FOR entry_value IN SELECT value FROM jsonb_array_elements(content_value -> 'entries') LOOP
                IF entry_value ->> 'kind' = 'fixed' THEN
                    delivered_question_count := delivered_question_count + 1;
                    IF NOT ple_data.blueprint_content_has_exact_keys(entry_value, ARRAY[
                        'kind', 'question_revision', 'points_possible', 'scoring_rule',
                        'question_attempt_limit', 'question_attempt_time_limit'
                    ]) THEN RETURN false; END IF;
                    pin_value := entry_value -> 'question_revision';
                    IF NOT ple_data.blueprint_content_has_exact_keys(
                        pin_value, ARRAY['questionId', 'revisionNumber']
                    ) THEN RETURN false; END IF;
                ELSIF entry_value ->> 'kind' = 'pool' THEN
                    IF entry_value ->> 'selection_count' !~ '^[1-9][0-9]*$'
                       OR entry_value ->> 'selection_count' IS NULL THEN RETURN false; END IF;
                    IF (entry_value ->> 'selection_count')::numeric > 250 THEN RETURN false; END IF;
                    delivered_question_count := delivered_question_count +
                        (entry_value ->> 'selection_count')::bigint;
                    IF NOT ple_data.blueprint_content_has_exact_keys(entry_value, ARRAY[
                        'kind', 'question_pool_revision', 'selection_count', 'points_per_item',
                        'scoring_rule', 'selection_rule', 'question_attempt_limit',
                        'question_attempt_time_limit'
                    ]) OR jsonb_typeof(entry_value -> 'question_pool_revision') <> 'object' THEN
                        RETURN false;
                    END IF;
                    pin_value := entry_value -> 'question_pool_revision';
                    IF NOT ple_data.blueprint_content_has_exact_keys(
                        pin_value, ARRAY['questionPoolId', 'revisionNumber']
                    ) THEN RETURN false; END IF;
                    selection_value := entry_value -> 'selection_rule';
                    IF NOT ple_data.blueprint_content_has_exact_keys(
                        selection_value, ARRAY['selectedQuestionOrder']
                    ) THEN RETURN false; END IF;
                ELSE
                    RETURN false;
                END IF;
                IF delivered_question_count > 250 THEN RETURN false; END IF;
                limit_value := entry_value -> 'question_attempt_limit';
                time_limit_value := entry_value -> 'question_attempt_time_limit';
                IF NOT ple_data.blueprint_content_has_exact_keys(limit_value, ARRAY['maxAttempts'])
                   OR NOT (
                       ple_data.blueprint_content_has_exact_keys(time_limit_value, ARRAY['kind'])
                       OR ple_data.blueprint_content_has_exact_keys(
                           time_limit_value, ARRAY['kind', 'seconds', 'graceSeconds']
                       )
                   ) THEN
                    RETURN false;
                END IF;
            END LOOP;
        END LOOP;
    END LOOP;
    RETURN true;
END
$$;

CREATE FUNCTION ple_data.validate_blueprint_content(p_content jsonb)
RETURNS void LANGUAGE plpgsql IMMUTABLE STRICT
SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    IF NOT ple_data.blueprint_content_is_closed(p_content)
       OR p_content ? 'title'
       OR jsonb_typeof(p_content -> 'modules') <> 'array'
       OR jsonb_array_length(p_content -> 'modules') > 1024
       OR EXISTS (
           SELECT 1
             FROM jsonb_array_elements(p_content -> 'modules') AS module_row(module)
            WHERE jsonb_typeof(module_row.module) <> 'object'
               OR module_row.module ->> 'blueprint_module_reference' IS NULL
               OR module_row.module ->> 'label' IS NULL
               OR module_row.module ->> 'label' <> btrim(module_row.module ->> 'label')
               OR char_length(module_row.module ->> 'label') NOT BETWEEN 1 AND 500
               OR jsonb_typeof(module_row.module -> 'assessments') <> 'array'
               OR jsonb_array_length(module_row.module -> 'assessments') NOT BETWEEN 1 AND 1024
       )
       OR (SELECT count(*) FROM ple_data.blueprint_content_modules(p_content))
          <> (SELECT count(DISTINCT blueprint_module_reference)
                FROM ple_data.blueprint_content_modules(p_content))
       OR EXISTS (
           SELECT 1
             FROM jsonb_array_elements(p_content -> 'modules') AS module_row(module)
             CROSS JOIN LATERAL jsonb_array_elements(module_row.module -> 'assessments')
                 AS assessment_row(assessment)
            WHERE jsonb_typeof(assessment_row.assessment) <> 'object'
               OR assessment_row.assessment ->> 'blueprint_assessment_reference' IS NULL
               OR jsonb_typeof(assessment_row.assessment -> 'content') <> 'object'
               OR jsonb_typeof(assessment_row.assessment -> 'content' -> 'entries') <> 'array'
       )
       OR (SELECT count(*) FROM ple_data.blueprint_content_assessments(p_content))
          <> (SELECT count(DISTINCT blueprint_assessment_reference)
                FROM ple_data.blueprint_content_assessments(p_content))
       OR EXISTS (
           SELECT 1
             FROM jsonb_array_elements(p_content -> 'modules') AS module_row(module)
             CROSS JOIN LATERAL jsonb_array_elements(module_row.module -> 'assessments')
                 AS assessment_row(assessment)
             CROSS JOIN LATERAL jsonb_array_elements(
                 assessment_row.assessment -> 'content' -> 'entries'
             ) AS entry_row(entry)
            WHERE entry_row.entry ->> 'kind' IS NULL
               OR entry_row.entry ->> 'kind' NOT IN ('fixed', 'pool')
               OR (entry_row.entry ->> 'kind' = 'fixed'
                   AND jsonb_typeof(entry_row.entry -> 'question_revision') <> 'object')
               OR (entry_row.entry ->> 'kind' = 'pool'
                   AND jsonb_typeof(entry_row.entry -> 'question_pool_revision') <> 'object')
       ) THEN
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

ALTER TABLE ple_data.blueprint_course ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_revision FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_question_pin ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_question_pin FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_module ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_module FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_assessment ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_assessment FORCE ROW LEVEL SECURITY;
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
    ple_data.blueprint_revision_assessment,
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
CREATE POLICY blueprint_revision_assessment_api_owner_all ON ple_data.blueprint_revision_assessment TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_revision_event_api_owner_all ON ple_data.blueprint_revision_event TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_metadata_event_api_owner_all ON ple_data.blueprint_metadata_event TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_create_receipt_api_owner_all ON ple_data.blueprint_course_create_receipt TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_save_receipt_api_owner_all ON ple_data.blueprint_course_save_receipt TO ple_api_owner USING (true) WITH CHECK (true);

REVOKE ALL PRIVILEGES ON TABLE
    ple_data.blueprint_course,
    ple_data.blueprint_course_revision,
    ple_data.blueprint_revision_question_pin,
    ple_data.blueprint_revision_module,
    ple_data.blueprint_revision_assessment,
    ple_data.blueprint_revision_event,
    ple_data.blueprint_metadata_event,
    ple_data.blueprint_course_create_receipt,
    ple_data.blueprint_course_save_receipt
FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.blueprint_content_question_pins(jsonb),
    ple_data.blueprint_content_pool_pins(jsonb),
    ple_data.blueprint_content_assessments(jsonb), ple_data.blueprint_content_modules(jsonb),
    ple_data.blueprint_content_has_exact_keys(jsonb, text[]),
    ple_data.blueprint_content_is_closed(jsonb),
    ple_data.validate_blueprint_content(jsonb),
    ple_data.validate_blueprint_question_selection(bigint, bigint, jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.blueprint_content_question_pins(jsonb),
    ple_data.blueprint_content_pool_pins(jsonb),
    ple_data.blueprint_content_assessments(jsonb), ple_data.blueprint_content_modules(jsonb),
    ple_data.blueprint_content_has_exact_keys(jsonb, text[]),
    ple_data.blueprint_content_is_closed(jsonb),
    ple_data.validate_blueprint_content(jsonb),
    ple_data.validate_blueprint_question_selection(bigint, bigint, jsonb)
TO ple_api_owner;

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.blueprint_course_fork (
    blueprint_course_reference_number bigint PRIMARY KEY
        REFERENCES ple_data.blueprint_course (reference_number),
    source_blueprint_course_reference_number bigint NOT NULL,
    source_blueprint_revision_number bigint NOT NULL CHECK (
        source_blueprint_revision_number > 0
    ),
    forked_at timestamp with time zone NOT NULL,
    CHECK (blueprint_course_reference_number <> source_blueprint_course_reference_number),
    FOREIGN KEY (
        source_blueprint_course_reference_number, source_blueprint_revision_number
    ) REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number
    )
);

-- An idempotent fork request is distinct from ordinary Blueprint creation:
-- the receipt preserves the source fact and prevents a retry from creating a
-- second child lineage.
CREATE TABLE ple_data.blueprint_course_fork_receipt (
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    source_blueprint_course_reference_number bigint NOT NULL,
    source_blueprint_revision_number bigint NOT NULL CHECK (
        source_blueprint_revision_number > 0
    ),
    metadata_etag uuid NOT NULL,
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (actor_account_id, request_checksum),
    FOREIGN KEY (
        source_blueprint_course_reference_number, source_blueprint_revision_number
    ) REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number
    )
);
CREATE INDEX blueprint_course_fork_source_idx ON ple_data.blueprint_course_fork (
    source_blueprint_course_reference_number, source_blueprint_revision_number
);

ALTER TABLE ple_data.blueprint_course_fork ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_fork FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_fork_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_course_fork_receipt FORCE ROW LEVEL SECURITY;
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE
    ple_data.blueprint_course_fork, ple_data.blueprint_course_fork_receipt
TO ple_api_owner;
CREATE POLICY blueprint_course_fork_api_owner_all ON ple_data.blueprint_course_fork
    TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY blueprint_course_fork_receipt_api_owner_all
    ON ple_data.blueprint_course_fork_receipt
    TO ple_api_owner USING (true) WITH CHECK (true);

-- The source Course and source Revision are permanent ancestry facts.  The
-- forked Blueprint's own content remains independently editable through its
-- ordinary immutable Revision sequence.
-- ASVS 8.2.2, 8.3.1: enforce this data-specific boundary in trusted PostgreSQL.
CREATE FUNCTION ple_data.reject_blueprint_course_fork_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Blueprint Course fork origin is immutable';
END
$$;
CREATE TRIGGER blueprint_course_fork_origin_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_course_fork
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_course_fork_change();
REVOKE ALL ON FUNCTION ple_data.reject_blueprint_course_fork_change() FROM PUBLIC;

RESET ROLE;
