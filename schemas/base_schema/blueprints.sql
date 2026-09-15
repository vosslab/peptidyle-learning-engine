-- Reusable Blueprint Course lineages and immutable save-created Revisions.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner, ple_api_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
-- The table-local human-reference trigger is created by ple_data_owner below.
-- Keep this grant adjacent to its first use so a fresh install cannot depend
-- on a later privilege bundle.
GRANT EXECUTE ON FUNCTION ple_private.assign_human_reference() TO ple_data_owner, ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;

CREATE TABLE ple_data.blueprint_course (
    blueprint_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE NOT NULL,
    public_reference text NOT NULL UNIQUE CHECK (public_reference ~ '^BP[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$'),
    owner_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    short_name text NOT NULL CHECK (char_length(btrim(short_name)) BETWEEN 1 AND 500),
    long_name text NOT NULL CHECK (char_length(btrim(long_name)) BETWEEN 1 AND 500),
    -- A new reusable Blueprint is owner-only until its owner explicitly makes
    -- it Public. Public is the sole state eligible for discovery/adoption;
    -- Archived remains readable by exact historical Revision reference only.
    availability text NOT NULL DEFAULT 'private'
        CHECK (availability IN ('private', 'public', 'archived')),
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
        SELECT module_ordinality, assessment_ordinality, entry_ordinality, 0 AS pin_ordinality,
               entry -> 'question_revision' AS pin
          FROM entries WHERE entry ->> 'kind' = 'fixed'
        UNION ALL
        SELECT entries.module_ordinality, entries.assessment_ordinality,
               entries.entry_ordinality, pool.pin_ordinality, pool.pin
          FROM entries
          CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(
              entries.entry -> 'question_revisions'
          ) WITH ORDINALITY AS pool(pin, pin_ordinality)
         WHERE entries.entry ->> 'kind' = 'pool'
    )
    SELECT pg_catalog.format('m%s.a%s.e%s.p%s', module_ordinality,
               assessment_ordinality, entry_ordinality, pin_ordinality),
           pg_catalog.replace(pin ->> 'questionId', '-', ''),
           (pin ->> 'revisionNumber')::bigint
      FROM pins
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
    completion_value jsonb;
    continuation_value jsonb;
BEGIN
    IF NOT ple_data.blueprint_content_has_exact_keys(p_content, ARRAY['modules'])
       OR jsonb_typeof(p_content -> 'modules') <> 'array'
       OR jsonb_array_length(p_content -> 'modules') NOT BETWEEN 1 AND 1024 THEN
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
                content_value, ARRAY['title', 'instructions', 'entries', 'defaults']
            ) OR jsonb_typeof(content_value -> 'title') <> 'string'
              OR jsonb_typeof(content_value -> 'instructions') <> 'string'
              OR jsonb_typeof(content_value -> 'entries') <> 'array'
              OR jsonb_array_length(content_value -> 'entries') NOT BETWEEN 1 AND 1024 THEN
                RETURN false;
            END IF;
            defaults_value := content_value -> 'defaults';
            IF NOT ple_data.blueprint_content_has_exact_keys(defaults_value, ARRAY[
                'assessment_attempt_time_limit_seconds', 'assessment_attempt_limit', 'late_work_rule',
                'activity_rules', 'student_feedback_release_rule'
            ]) THEN
                RETURN false;
            END IF;
            activity_value := defaults_value -> 'activity_rules';
            feedback_value := defaults_value -> 'student_feedback_release_rule';
            IF NOT ple_data.blueprint_content_has_exact_keys(activity_value, ARRAY[
                'assessmentCompletionRule', 'assessmentAttemptGradeRule',
                'assessmentAttemptContinuationRule', 'questionPoolReuseRule',
                'questionVariationRule', 'assessmentAttemptResumeRule',
                'assessmentQuestionDisplayRule', 'assessmentNavigationRule',
                'assessmentQuestionOrderRule'
            ]) OR NOT ple_data.blueprint_content_has_exact_keys(feedback_value, ARRAY[
                'score', 'per_item_correctness', 'submitted_response', 'question_feedback',
                'question_answer', 'question_answer_explanation', 'class_statistics'
            ]) THEN
                RETURN false;
            END IF;
            completion_value := activity_value -> 'assessmentCompletionRule';
            continuation_value := activity_value -> 'assessmentAttemptContinuationRule';
            IF NOT (
                ple_data.blueprint_content_has_exact_keys(completion_value, ARRAY['kind'])
                OR ple_data.blueprint_content_has_exact_keys(completion_value, ARRAY['kind', 'fraction'])
            ) OR NOT (
                ple_data.blueprint_content_has_exact_keys(continuation_value, ARRAY['kind'])
                OR ple_data.blueprint_content_has_exact_keys(
                    continuation_value, ARRAY['kind', 'maxAdditionalAssessmentAttempts']
                )
            ) THEN
                RETURN false;
            END IF;
            FOR entry_value IN SELECT value FROM jsonb_array_elements(content_value -> 'entries') LOOP
                IF entry_value ->> 'kind' = 'fixed' THEN
                    IF NOT ple_data.blueprint_content_has_exact_keys(entry_value, ARRAY[
                        'kind', 'question_revision', 'points_possible', 'scoring_rule',
                        'question_attempt_limit', 'question_attempt_time_limit'
                    ]) THEN RETURN false; END IF;
                    pin_value := entry_value -> 'question_revision';
                    IF NOT ple_data.blueprint_content_has_exact_keys(
                        pin_value, ARRAY['questionId', 'revisionNumber']
                    ) THEN RETURN false; END IF;
                ELSIF entry_value ->> 'kind' = 'pool' THEN
                    IF NOT ple_data.blueprint_content_has_exact_keys(entry_value, ARRAY[
                        'kind', 'question_revisions', 'selection_count', 'points_per_item',
                        'scoring_rule', 'selection_rule', 'question_attempt_limit',
                        'question_attempt_time_limit'
                    ]) OR jsonb_typeof(entry_value -> 'question_revisions') <> 'array'
                      OR jsonb_array_length(entry_value -> 'question_revisions') NOT BETWEEN 1 AND 1024 THEN
                        RETURN false;
                    END IF;
                    FOR pin_value IN
                        SELECT value FROM jsonb_array_elements(entry_value -> 'question_revisions') LOOP
                        IF NOT ple_data.blueprint_content_has_exact_keys(
                            pin_value, ARRAY['questionId', 'revisionNumber']
                        ) THEN RETURN false; END IF;
                    END LOOP;
                    selection_value := entry_value -> 'selection_rule';
                    IF NOT ple_data.blueprint_content_has_exact_keys(
                        selection_value, ARRAY['selectedQuestionOrder']
                    ) THEN RETURN false; END IF;
                ELSE
                    RETURN false;
                END IF;
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
       OR jsonb_array_length(p_content -> 'modules') NOT BETWEEN 1 AND 1024
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
               OR jsonb_array_length(assessment_row.assessment -> 'content' -> 'entries') = 0
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
    ple_data.blueprint_content_assessments(jsonb), ple_data.blueprint_content_modules(jsonb),
    ple_data.blueprint_content_has_exact_keys(jsonb, text[]),
    ple_data.blueprint_content_is_closed(jsonb),
    ple_data.validate_blueprint_content(jsonb),
    ple_data.validate_blueprint_question_selection(bigint, bigint, jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.blueprint_content_question_pins(jsonb),
    ple_data.blueprint_content_assessments(jsonb), ple_data.blueprint_content_modules(jsonb),
    ple_data.blueprint_content_has_exact_keys(jsonb, text[]),
    ple_data.blueprint_content_is_closed(jsonb),
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
    INSERT INTO ple_data.blueprint_course AS course (
        blueprint_id, owner_account_id, short_name, long_name, metadata_etag, created_at
    ) VALUES (
        p_blueprint_id, v_actor, p_short_name, p_long_name, v_metadata_etag, v_now
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
        availability, metadata_etag, occurred_at
    ) VALUES (
        v_reference_number, v_actor, p_short_name, p_long_name,
        'private', v_metadata_etag, v_now
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
    IF p_reference !~ '^BP[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$'
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
    short_name text, long_name text, availability text, metadata_etag uuid
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
    IF p_reference !~ '^BP[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$'
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
     WHERE course.reference_number = v_reference_number;
    INSERT INTO ple_data.blueprint_metadata_event (
        blueprint_course_reference_number, actor_account_id, short_name, long_name,
        availability, metadata_etag, occurred_at
    ) VALUES (
        v_reference_number, v_actor, p_short_name, p_long_name,
        v_course.availability, v_next, pg_catalog.clock_timestamp()
    );
    RETURN QUERY SELECT p_short_name, p_long_name, v_course.availability, v_next;
END
$$;

CREATE FUNCTION ple_api.set_blueprint_availability(
    p_reference text, p_expected_metadata_etag uuid, p_availability text,
    p_archive_confirmation_long_name text
)
RETURNS TABLE (
    short_name text, long_name text, availability text, metadata_etag uuid
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
    IF p_reference !~ '^BP[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$'
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
    -- lineage may become Private only before its first daughter Course
    -- Instance; otherwise its adopted source remains Public. Every accepted
    -- transition is recorded below as an immutable metadata event.
    IF (v_course.availability = 'private' AND p_availability <> 'public')
       OR (v_course.availability = 'public' AND p_availability = 'private'
           AND EXISTS (
               SELECT 1 FROM ple_data.course_instance AS adoption
                WHERE adoption.blueprint_course_reference_number = v_reference_number
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
        availability, metadata_etag, occurred_at
    ) VALUES (
        v_reference_number, v_actor, v_course.short_name, v_course.long_name,
        p_availability, v_next, pg_catalog.clock_timestamp()
    );
    RETURN QUERY SELECT v_course.short_name, v_course.long_name, p_availability, v_next;
END
$$;

CREATE FUNCTION ple_api.list_blueprint_courses()
RETURNS TABLE (
    public_reference text, short_name text, long_name text, availability text,
    metadata_etag uuid, current_blueprint_revision_number bigint, is_owner boolean,
    total_adoptions bigint, total_students_ever_enrolled bigint
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    RETURN QUERY SELECT course.public_reference, course.short_name, course.long_name,
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
       AND (
           course.owner_account_id = ple_api.current_session_account_id()
           OR course.availability = 'public'
       )
     ORDER BY course.long_name, course.reference_number;
END
$$;

CREATE FUNCTION ple_api.load_blueprint_course(p_reference text)
RETURNS TABLE (
    public_reference text, short_name text, long_name text, availability text,
    metadata_etag uuid, current_blueprint_revision_number bigint,
    content jsonb, content_checksum bytea, is_owner boolean
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
    SELECT course.public_reference, course.short_name, course.long_name,
           course.availability, course.metadata_etag,
           course.current_blueprint_revision_number,
           revision.content, revision.content_checksum,
           course.owner_account_id = ple_api.current_session_account_id()
      FROM ple_data.blueprint_course AS course
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_reference_number = course.reference_number
       AND revision.blueprint_revision_number = course.current_blueprint_revision_number
     WHERE p_reference ~ '^BP[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$'
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
     WHERE p_reference ~ '^BP[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$'
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
    ple_api.create_blueprint_course(uuid, bytea, text, text, jsonb, bytea),
    ple_api.save_blueprint_course(text, bigint, bytea, jsonb, bytea),
    ple_api.rename_blueprint_course(text, uuid, text, text),
    ple_api.set_blueprint_availability(text, uuid, text, text),
    ple_api.list_blueprint_courses(), ple_api.load_blueprint_course(text),
    ple_api.load_blueprint_revision(text, bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_api.create_blueprint_course(uuid, bytea, text, text, jsonb, bytea),
    ple_api.save_blueprint_course(text, bigint, bytea, jsonb, bytea),
    ple_api.rename_blueprint_course(text, uuid, text, text),
    ple_api.set_blueprint_availability(text, uuid, text, text),
    ple_api.list_blueprint_courses(), ple_api.load_blueprint_course(text),
    ple_api.load_blueprint_revision(text, bigint) TO ple_app;

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.blueprint_course IS
    'Stable reusable Blueprint Course lineage with names, availability, metadata ETag, and current Revision.';
COMMENT ON TABLE ple_data.blueprint_course_revision IS
    'Immutable complete Blueprint Revision; exact references remain valid after later Saves or archive.';
COMMENT ON TABLE ple_data.blueprint_revision_assessment IS
    'Durable Blueprint Assessment identity and Revision membership retained for provenance and comparison.';

RESET ROLE;
