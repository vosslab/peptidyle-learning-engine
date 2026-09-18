-- Functions, triggers, and views from assessment_blueprint_updates.sql.

SET LOCAL ROLE ple_data_owner;

-- Derived retained-Assessment review and one explicit, atomic reusable-content update.
-- No offers, approval receipts or merge baselines are persisted.
CREATE FUNCTION ple_data.lock_assessment_blueprint_update_destination(
    p_course_reference text, p_assessment_reference text, p_parent_reference text
) RETURNS ple_data.assessment LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE course_row ple_data.course_instance%ROWTYPE; assessment_row ple_data.assessment%ROWTYPE;
BEGIN
    SELECT * INTO course_row FROM ple_data.course_instance
     WHERE course_instance_id = p_course_reference
       AND blueprint_course_id = p_parent_reference
       AND ple_api.current_session_account_is_course_instructor(course_instance_id)
     FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    SELECT * INTO assessment_row FROM ple_data.assessment
     WHERE course_instance_id = course_row.course_instance_id AND assessment_id = p_assessment_reference
       AND origin_kind = 'adopted'
       AND source_blueprint_course_id = p_parent_reference
     FOR UPDATE;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course_row.course_instance_id)
        THEN RETURN NULL; END IF;
    RETURN assessment_row;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.load_assessment_blueprint_update(
    p_course_reference text, p_assessment_reference text
) RETURNS TABLE (
    source_reference text, source_revision bigint, source_assessment_reference uuid,
    content jsonb, content_checksum bytea, source_assessment_content jsonb,
    cannot_apply_reason text
)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    parent ple_data.blueprint_course%ROWTYPE;
    assessment_row ple_data.assessment%ROWTYPE;
BEGIN
    -- ASVS 8.2.1-8.2.3, 8.3.1: derive the parent and retained source from
    -- immutable daughter origin, never from a browser-selected Blueprint ID.
    SELECT blueprint.* INTO parent
      FROM ple_data.blueprint_course AS blueprint
      JOIN ple_data.course_instance AS course
        ON course.blueprint_course_id = blueprint.blueprint_course_id
      JOIN ple_data.assessment AS assessment ON assessment.course_instance_id = course.course_instance_id
       AND assessment.source_blueprint_course_id = blueprint.blueprint_course_id
     WHERE course.course_instance_id = p_course_reference
       AND assessment.assessment_id = p_assessment_reference
       AND assessment.origin_kind = 'adopted'
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
       AND ple_api.current_session_account_is_instructor()
       AND (blueprint.availability IN ('public', 'archived')
            OR blueprint.owner_account_id = ple_api.current_session_account_id())
     FOR UPDATE OF blueprint;
    IF NOT FOUND THEN RETURN; END IF;
    -- ASVS 2.3.3, 15.4.2-15.4.3: parent -> Course -> Assessment lock order
    -- matches Blueprint Save/adoption. Reauthorize after each lock is acquired.
    assessment_row := ple_data.lock_assessment_blueprint_update_destination(
        p_course_reference, p_assessment_reference, parent.blueprint_course_id);
    IF assessment_row.assessment_id IS NULL
       OR NOT ple_api.current_session_account_is_course_instructor(assessment_row.course_instance_id)
       OR NOT ple_api.current_session_account_is_instructor()
       OR NOT (parent.availability IN ('public', 'archived')
               OR parent.owner_account_id = ple_api.current_session_account_id()) THEN RETURN; END IF;
    source_reference := parent.blueprint_course_id;
    source_revision := parent.current_blueprint_revision_number;
    source_assessment_reference := assessment_row.source_blueprint_assessment_reference;
    SELECT revision.content, revision.content_checksum INTO content, content_checksum
      FROM ple_data.blueprint_course_revision AS revision
     WHERE revision.blueprint_course_id = source_reference
       AND revision.blueprint_revision_number = source_revision;
    SELECT member.value -> 'content' INTO source_assessment_content
      FROM jsonb_array_elements(content -> 'modules') AS module
      CROSS JOIN LATERAL jsonb_array_elements(module.value -> 'assessments') AS member
     WHERE member.value ->> 'blueprint_assessment_reference' = source_assessment_reference::text;
    cannot_apply_reason := CASE
        WHEN source_assessment_content IS NULL THEN 'retained_source_missing'
        WHEN source_assessment_content ->> 'assessment_type' <> assessment_row.assessment_type
            THEN 'assessment_type_mismatch'
        ELSE NULL END;
    RETURN NEXT;
END
$$;

SET LOCAL ROLE ple_data_owner;






-- Compare ordered reusable semantics, not destination Entry or fork identities.
CREATE FUNCTION ple_data.assessment_blueprint_update_entries_semantics(
    p_entries jsonb
) RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE entry_json jsonb; normalized jsonb; members jsonb; result jsonb := '[]'::jsonb;
BEGIN
    FOR entry_json IN SELECT value FROM jsonb_array_elements(p_entries) LOOP
        normalized := entry_json - 'assessmentEntryId' - 'authoredPosition'
            - 'forkQuestionPoolId' - 'forkPublicQuestionPoolId';
        IF entry_json ->> 'kind' = 'question_pool' THEN
            SELECT jsonb_agg(jsonb_build_array(member.published_question_id, member.question_revision_number)
                             ORDER BY member.member_position) INTO members
              FROM ple_data.question_pool AS pool
              JOIN ple_data.question_pool_member AS member
                ON member.question_pool_id = pool.question_pool_id
             WHERE pool.question_pool_id = COALESCE(entry_json ->> 'sourceQuestionPoolId',
                                                           entry_json ->> 'questionPoolId')
               AND member.revision_number = COALESCE(
                   entry_json ->> 'sourceQuestionPoolRevisionNumber',
                   entry_json ->> 'questionPoolRevisionNumber')::bigint;
            normalized := normalized - 'sourceQuestionPoolId' - 'sourceQuestionPoolRevisionNumber'
                - 'questionPoolId' - 'questionPoolRevisionNumber'
                || jsonb_build_object('members', members,
                    'pointsPerItem', (entry_json ->> 'pointsPerItem')::numeric);
        ELSE
            normalized := normalized || jsonb_build_object(
                'pointsPossible', (entry_json ->> 'pointsPossible')::numeric);
        END IF;
        result := result || jsonb_build_array(normalized);
    END LOOP;
    RETURN result;
END
$$;

CREATE FUNCTION ple_data.assessment_blueprint_update_equivalent(
    p_assessment_id text, p_values jsonb, p_entries jsonb
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT NOT EXISTS (
        SELECT 1 FROM jsonb_each(p_values) AS proposed
         WHERE proposed.key NOT IN ('assessment_type', 'available_at', 'due_at', 'closes_at')
           AND proposed.value IS DISTINCT FROM COALESCE(
               to_jsonb(policy) -> proposed.key, to_jsonb(assessment) -> proposed.key)
    ) AND ple_data.assessment_blueprint_update_entries_semantics(p_entries) =
        ple_data.assessment_blueprint_update_entries_semantics(COALESCE((
            SELECT jsonb_agg(
                jsonb_build_object('kind', entry.entry_kind, 'availability', entry.availability,
                    'scoringRule', entry.scoring_rule,
                    'questionAttemptLimit', entry.question_attempt_limit,
                    'questionAttemptTimeLimitSeconds', entry.question_attempt_time_limit_seconds,
                    'questionAttemptGraceSeconds', entry.question_attempt_grace_seconds)
                || CASE WHEN entry.entry_kind = 'fixed_question' THEN jsonb_build_object(
                    'questionId', question.published_question_id, 'revisionNumber', question.question_revision_number,
                    'pointsPossible', question.points_possible::text)
                ELSE jsonb_build_object('questionPoolId', pool.question_pool_id,
                    'questionPoolRevisionNumber', pool.question_pool_edit_number,
                    'selectionCount', pool_entry.selection_count, 'pointsPerItem', pool_entry.points_per_item::text,
                    'selectedQuestionOrder', pool_entry.selected_question_order) END
                ORDER BY entry.authored_position)
              FROM ple_data.assessment_entry AS entry
              LEFT JOIN ple_data.assessment_entry_question AS question
                ON question.assessment_entry_id = entry.assessment_entry_id
              LEFT JOIN ple_data.assessment_entry_pool AS pool_entry
                ON pool_entry.assessment_entry_id = entry.assessment_entry_id
              LEFT JOIN ple_data.question_pool AS pool ON pool.question_pool_id = pool_entry.question_pool_id
             WHERE entry.assessment_id = p_assessment_id AND entry.availability = 'available'
        ), '[]'::jsonb))
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = assessment.assessment_policy_snapshot_id
     WHERE assessment.assessment_id = p_assessment_id
$$;

CREATE FUNCTION ple_data.apply_assessment_blueprint_update(
    p_course_reference text, p_assessment_reference text,
    p_expected_source_revision bigint, p_expected_edit_number bigint, p_member jsonb
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    source record; assessment_row ple_data.assessment%ROWTYPE; course_row ple_data.course_instance%ROWTYPE;
    policy ple_data.assessment_policy_snapshot%ROWTYPE;
    values_json jsonb; entries_json jsonb := '[]'::jsonb; entry_json jsonb;
    source_pool_id text; forked record;
BEGIN
    SELECT * INTO source FROM ple_api.load_assessment_blueprint_update(
        p_course_reference, p_assessment_reference);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment is unavailable';
    END IF;
    SELECT * INTO course_row FROM ple_data.course_instance WHERE course_instance_id = p_course_reference;
    SELECT * INTO assessment_row FROM ple_data.assessment
     WHERE course_instance_id = course_row.course_instance_id AND assessment_id = p_assessment_reference;
    SELECT * INTO policy FROM ple_data.assessment_policy_snapshot
     WHERE assessment_policy_snapshot_id = assessment_row.assessment_policy_snapshot_id;
    IF source.source_revision IS DISTINCT FROM p_expected_source_revision
       OR assessment_row.assessment_edit_number IS DISTINCT FROM p_expected_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Blueprint update precondition is stale';
    END IF;
    IF source.cannot_apply_reason IS NOT NULL THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Retained Blueprint Assessment cannot be applied';
    END IF;
    IF p_member ->> 'source' IS DISTINCT FROM source.source_assessment_reference::text THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint update source is invalid';
    END IF;
    -- ASVS 2.2.2, 15.3.3: trusted exact-source validation precedes any fork.
    PERFORM ple_data.validate_course_blueprint_adoption(
        source.source_reference, source.source_revision, jsonb_build_array(p_member));
    IF ple_data.assessment_blueprint_update_equivalent(
        assessment_row.assessment_id, p_member -> 'values', p_member -> 'entries') THEN
        IF assessment_row.assessment_status = 'released' THEN
            PERFORM ple_data.validate_assessment_release(
                assessment_row.assessment_id, transaction_timestamp(), false);
        END IF;
        RETURN;
    END IF;
    IF EXISTS (
        SELECT 1 FROM jsonb_array_elements(p_member -> 'entries') AS proposed
         WHERE proposed.value ->> 'assessmentEntryId' IS NULL
            OR proposed.value ->> 'assessmentEntryId' !~*
                '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            OR EXISTS (SELECT 1 FROM ple_data.assessment_entry AS old
                       WHERE old.assessment_entry_id = (proposed.value ->> 'assessmentEntryId')::uuid)
            OR (proposed.value ->> 'kind' = 'question_pool' AND (
                proposed.value ->> 'forkQuestionPoolId' IS NULL
                OR proposed.value ->> 'forkPublicQuestionPoolId' IS NULL))
    ) OR (SELECT count(*) FROM jsonb_array_elements(p_member -> 'entries')) <>
         (SELECT count(DISTINCT value ->> 'assessmentEntryId')
            FROM jsonb_array_elements(p_member -> 'entries')) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint update requires fresh Entry identities';
    END IF;
    values_json := (p_member -> 'values') - 'assessment_type' || jsonb_build_object(
        'available_at', policy.available_at, 'due_at', policy.due_at,
        'closes_at', policy.closes_at);
    FOR entry_json IN SELECT value FROM jsonb_array_elements(p_member -> 'entries') LOOP
        IF entry_json ->> 'kind' = 'question_pool' THEN
            SELECT question_pool_id INTO source_pool_id FROM ple_data.question_pool
             WHERE question_pool_id = entry_json ->> 'sourceQuestionPoolId';
            SELECT * INTO forked FROM ple_data.fork_question_pool_revision_for_course_adoption(
                entry_json ->> 'forkPublicQuestionPoolId', source_pool_id);
            -- Create the owned Entry and fork association without an intermediate
            -- Assessment edit. The one normal save below owns the complete edit.
            INSERT INTO ple_data.assessment_entry (
                assessment_entry_id, assessment_id, authored_position, entry_kind, availability,
                scoring_rule, question_attempt_limit,
                question_attempt_time_limit_seconds, question_attempt_grace_seconds
            ) VALUES (
                (entry_json ->> 'assessmentEntryId')::uuid, assessment_row.assessment_id,
                (entry_json ->> 'authoredPosition')::integer, 'question_pool', 'retired',
                entry_json ->> 'scoringRule',
                NULLIF(entry_json ->> 'questionAttemptLimit', '')::integer,
                NULLIF(entry_json ->> 'questionAttemptTimeLimitSeconds', '')::integer,
                NULLIF(entry_json ->> 'questionAttemptGraceSeconds', '')::integer);
            INSERT INTO ple_data.assessment_entry_pool (
                assessment_entry_id, assessment_id, question_pool_id,
                selection_count, points_per_item, selected_question_order
            ) VALUES (
                (entry_json ->> 'assessmentEntryId')::uuid, assessment_row.assessment_id,
                forked.question_pool_id,
                (entry_json ->> 'selectionCount')::integer, (entry_json ->> 'pointsPerItem')::numeric,
                entry_json ->> 'selectedQuestionOrder');
            INSERT INTO ple_data.assessment_question_pool_fork (
                assessment_entry_id, assessment_id, question_pool_id
            ) VALUES ((entry_json ->> 'assessmentEntryId')::uuid, assessment_row.assessment_id,
                      forked.question_pool_id);
            entry_json := entry_json - 'sourceQuestionPoolId' - 'sourceQuestionPoolRevisionNumber'
                - 'forkQuestionPoolId' - 'forkPublicQuestionPoolId' || jsonb_build_object(
                    'questionPoolId', forked.question_pool_id, 'questionPoolRevisionNumber', 1);
        END IF;
        entries_json := entries_json || jsonb_build_array(entry_json);
    END LOOP;
    PERFORM ple_data.save_assessment(course_row.blueprint_course_id, assessment_row.assessment_id,
        p_expected_edit_number, values_json, entries_json);
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.apply_assessment_blueprint_update(
    p_course_reference text, p_assessment_reference text,
    p_expected_source_revision bigint, p_expected_edit_number bigint, p_member jsonb
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT ple_data.apply_assessment_blueprint_update($1, $2, $3, $4, $5)
$$;



-- A read-only semantic projection can use this before minting new Entry/fork IDs.
-- The Store supplies only its typed reusable projection; source eligibility
-- and destination ownership are independently resolved at this boundary.
CREATE FUNCTION ple_api.assessment_blueprint_update_equivalent(
    p_course_reference text, p_assessment_reference text, p_values jsonb, p_entries jsonb
) RETURNS boolean LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE source record; destination_id text;
BEGIN
    SELECT * INTO source FROM ple_api.load_assessment_blueprint_update($1, $2);
    IF NOT FOUND OR source.cannot_apply_reason IS NOT NULL THEN RETURN false; END IF;
    SELECT assessment.assessment_id INTO destination_id
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_instance_id = course.course_instance_id
     WHERE course.course_instance_id = $1 AND assessment.assessment_id = $2;
    RETURN ple_data.assessment_blueprint_update_equivalent(destination_id, p_values, p_entries);
END
$$;

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.lock_course_blueprint_update_destination(
    p_course_reference text, p_parent_reference text
) RETURNS ple_data.course_instance LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE course_row ple_data.course_instance%ROWTYPE;
BEGIN
    SELECT * INTO course_row FROM ple_data.course_instance
     WHERE course_instance_id = p_course_reference AND source_kind = 'adopted'
       AND blueprint_course_id = p_parent_reference
       AND ple_api.current_session_account_is_course_instructor(course_instance_id)
     FOR UPDATE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    -- ASVS 15.4.2-15.4.3: stable lock order protects the complete current
    -- adopted membership and each reusable aggregate during summary derivation.
    PERFORM assessment_id FROM ple_data.assessment
     WHERE course_instance_id = course_row.course_instance_id AND origin_kind = 'adopted'
     ORDER BY assessment_id FOR UPDATE;
    IF NOT ple_api.current_session_account_is_course_instructor(course_row.course_instance_id)
        THEN RETURN NULL; END IF;
    RETURN course_row;
END
$$;

SET LOCAL ROLE ple_api_owner;





-- First read seals the typed source and locks the complete destination. The
-- second supplies its identity-free reusable projections for one batch comparison.
-- Neither invocation writes content, origin pins, offers, receipts or Student Work.
CREATE FUNCTION ple_api.load_course_blueprint_update(
    p_course_reference text, p_members jsonb
) RETURNS TABLE (
    blueprint_reference text, adopted_revision bigint, source_revision bigint,
    content jsonb, content_checksum bytea, assessments jsonb
) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE parent ple_data.blueprint_course%ROWTYPE; course_row ple_data.course_instance%ROWTYPE;
BEGIN
    -- ASVS 8.2.1-8.2.3, 8.3.1: origin selects the parent, including an empty
    -- adopted Course. Empty-source Courses and unreadable parents are unavailable.
    SELECT blueprint.* INTO parent
      FROM ple_data.blueprint_course AS blueprint
      JOIN ple_data.course_instance AS course
        ON course.blueprint_course_id = blueprint.blueprint_course_id
     WHERE course.course_instance_id = p_course_reference AND course.source_kind = 'adopted'
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
       AND ple_api.current_session_account_is_instructor()
       AND (blueprint.availability IN ('public', 'archived')
            OR blueprint.owner_account_id = ple_api.current_session_account_id())
     FOR UPDATE OF blueprint;
    IF NOT FOUND THEN RETURN; END IF;
    course_row := ple_data.lock_course_blueprint_update_destination(
        p_course_reference, parent.blueprint_course_id);
    IF course_row.course_instance_id IS NULL
       OR NOT ple_api.current_session_account_is_course_instructor(course_row.course_instance_id)
       OR NOT ple_api.current_session_account_is_instructor()
       OR NOT (parent.availability IN ('public', 'archived')
               OR parent.owner_account_id = ple_api.current_session_account_id()) THEN RETURN; END IF;
    blueprint_reference := parent.blueprint_course_id;
    adopted_revision := course_row.blueprint_revision_number;
    source_revision := parent.current_blueprint_revision_number;
    SELECT revision.content, revision.content_checksum INTO content, content_checksum
      FROM ple_data.blueprint_course_revision AS revision
     WHERE revision.blueprint_course_id = parent.blueprint_course_id
       AND revision.blueprint_revision_number = source_revision;
    IF p_members IS NOT NULL AND jsonb_typeof(p_members) IS DISTINCT FROM 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Blueprint update projections are invalid';
    END IF;
    SELECT COALESCE(jsonb_agg(jsonb_build_object(
        'assessmentReference', destination.assessment_id,
        'title', policy.assessment_title,
        'assessmentType', destination.assessment_type,
        'cannotApplyReason', CASE
            WHEN source_member.source_content IS NULL THEN 'retainedSourceMissing'
            WHEN source_member.source_content ->> 'assessment_type' <> destination.assessment_type
                THEN 'assessmentTypeMismatch' ELSE NULL END,
        'matchesSource', CASE
            WHEN source_member.source_content IS NULL
              OR source_member.source_content ->> 'assessment_type' <> destination.assessment_type
              OR projection.member IS NULL THEN false
            ELSE COALESCE(ple_data.assessment_blueprint_update_equivalent(
                destination.assessment_id, projection.member -> 'values',
                projection.member -> 'entries'), false) END
        ) ORDER BY destination.assessment_id), '[]'::jsonb) INTO assessments
      FROM ple_data.assessment AS destination
      JOIN ple_data.assessment_policy_snapshot AS policy
        ON policy.assessment_policy_snapshot_id = destination.assessment_policy_snapshot_id
      LEFT JOIN LATERAL (
          SELECT member.value -> 'content' AS source_content
            FROM jsonb_array_elements(content -> 'modules') AS module
            CROSS JOIN LATERAL jsonb_array_elements(module.value -> 'assessments') AS member
           WHERE member.value ->> 'blueprint_assessment_reference' =
                 destination.source_blueprint_assessment_reference::text
             AND destination.source_blueprint_course_id = parent.blueprint_course_id
      ) AS source_member ON true
      LEFT JOIN LATERAL (
          SELECT value AS member FROM jsonb_array_elements(p_members)
           WHERE value ->> 'source' = destination.source_blueprint_assessment_reference::text
      ) AS projection ON true
     WHERE destination.course_instance_id = course_row.course_instance_id AND destination.origin_kind = 'adopted';
    RETURN NEXT;
END
$$;

