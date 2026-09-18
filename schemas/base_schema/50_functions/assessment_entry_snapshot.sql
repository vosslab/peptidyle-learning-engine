-- Functions, triggers, and views from assessment_entry_snapshot.sql.

SET LOCAL ROLE ple_private_owner;

-- Content-addressed Assessment Entry facts frozen at issue. Two equal Entry
-- configurations share one immutable row. Kind pairing lives on the table.
CREATE FUNCTION ple_private.ensure_assessment_entry_snapshot(
    p_entry_kind ple_data.entry_kind,
    p_scoring_rule ple_data.scoring_rule,
    p_points numeric,
    p_published_question_id text,
    p_question_revision_number integer,
    p_question_pool_id text,
    p_question_attempt_limit integer,
    p_question_attempt_time_limit_seconds integer,
    p_question_attempt_grace_seconds integer
) RETURNS ple_data.sha256_digest
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    snapshot_id ple_data.sha256_digest;
    canonical_jsonb jsonb;
BEGIN
    canonical_jsonb := jsonb_build_object(
        'entry_kind', p_entry_kind,
        'scoring_rule', p_scoring_rule,
        'points', to_jsonb(p_points),
        'published_question_id', to_jsonb(p_published_question_id),
        'question_revision_number', to_jsonb(p_question_revision_number),
        'question_pool_id', to_jsonb(p_question_pool_id),
        'question_attempt_limit', to_jsonb(p_question_attempt_limit),
        'question_attempt_time_limit_seconds', to_jsonb(p_question_attempt_time_limit_seconds),
        'question_attempt_grace_seconds', to_jsonb(p_question_attempt_grace_seconds)
    );
    snapshot_id := sha256(convert_to(canonical_jsonb::text, 'UTF8'));
    INSERT INTO ple_private.assessment_entry_snapshot (
        assessment_entry_snapshot_id,
        entry_kind,
        scoring_rule,
        points,
        published_question_id,
        question_revision_number,
        question_pool_id,
        question_attempt_limit,
        question_attempt_time_limit_seconds,
        question_attempt_grace_seconds,
        created_at
    ) VALUES (
        snapshot_id,
        p_entry_kind,
        p_scoring_rule,
        p_points,
        p_published_question_id,
        p_question_revision_number,
        p_question_pool_id,
        p_question_attempt_limit,
        p_question_attempt_time_limit_seconds,
        p_question_attempt_grace_seconds,
        clock_timestamp()
    ) ON CONFLICT (assessment_entry_snapshot_id) DO NOTHING;
    RETURN snapshot_id;
END
$$;
