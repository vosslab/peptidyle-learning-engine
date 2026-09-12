-- Privacy-safe global statistics for immutable Question Revisions.  The
-- receipt rows retain no Student, Account, Course, response, or grade identity;
-- they are rooted in the grading receipt solely so Unrelease can remove an
-- observation and rebuild the affected aggregate from surviving evidence.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.question_revision_statistics (
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    accepted_graded_attempt_count bigint NOT NULL DEFAULT 0
        CHECK (accepted_graded_attempt_count >= 0),
    correct_count bigint NOT NULL DEFAULT 0
        CHECK (correct_count BETWEEN 0 AND accepted_graded_attempt_count),
    updated_at timestamptz NOT NULL,
    PRIMARY KEY (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_data.question_revision_choice_statistics (
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    choice_id text NOT NULL CHECK (
        choice_id = btrim(choice_id) AND char_length(choice_id) BETWEEN 1 AND 256
    ),
    selected_count bigint NOT NULL CHECK (selected_count >= 0),
    PRIMARY KEY (question_id, revision_number, choice_id),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision_statistics(question_id, revision_number)
);

ALTER TABLE ple_data.question_revision_statistics ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_statistics FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_choice_statistics ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_choice_statistics FORCE ROW LEVEL SECURITY;

CREATE POLICY question_revision_statistics_data_owner_access
    ON ple_data.question_revision_statistics
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_revision_choice_statistics_data_owner_access
    ON ple_data.question_revision_choice_statistics
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_revision_statistics_api_read
    ON ple_data.question_revision_statistics FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_revision_choice_statistics_api_read
    ON ple_data.question_revision_choice_statistics FOR SELECT TO ple_api_owner USING (true);

REVOKE ALL ON TABLE ple_data.question_revision_statistics,
    ple_data.question_revision_choice_statistics FROM PUBLIC;
GRANT SELECT ON TABLE ple_data.question_revision_statistics,
    ple_data.question_revision_choice_statistics TO ple_api_owner;

-- Rebuild only one exact Revision.  This is called after Unrelease removes
-- rooted receipts, rather than trusting a mutable counter to survive deletion.
CREATE FUNCTION ple_data.rebuild_question_revision_statistics(
    p_question_id text,
    p_revision_number integer,
    p_rebuilt_at timestamptz
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF p_question_id !~ '^[0-9A-HJKMNP-TV-Z]{7}$'
       OR p_revision_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Revision Statistics rebuild target is invalid';
    END IF;

    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(p_question_id || ':' || p_revision_number::text, 0));

    DELETE FROM ple_data.question_revision_choice_statistics
     WHERE question_id = p_question_id AND revision_number = p_revision_number;
    DELETE FROM ple_data.question_revision_statistics
     WHERE question_id = p_question_id AND revision_number = p_revision_number;

    INSERT INTO ple_data.question_revision_statistics (
        question_id, revision_number, accepted_graded_attempt_count, correct_count, updated_at
    ) SELECT observation.question_id, observation.revision_number, count(*),
             count(*) FILTER (WHERE observation.correct), p_rebuilt_at
        FROM ple_private.question_statistics_observation_receipt AS observation
       WHERE observation.question_id = p_question_id
         AND observation.revision_number = p_revision_number
       GROUP BY observation.question_id, observation.revision_number;

    INSERT INTO ple_data.question_revision_choice_statistics (
        question_id, revision_number, choice_id, selected_count
    ) SELECT observation.question_id, observation.revision_number, choice.choice_id, count(*)
        FROM ple_private.question_statistics_observation_receipt AS observation
        JOIN ple_private.question_statistics_observation_choice AS choice
          ON choice.automated_grading_receipt_id = observation.automated_grading_receipt_id
       WHERE observation.question_id = p_question_id
         AND observation.revision_number = p_revision_number
       GROUP BY observation.question_id, observation.revision_number, choice.choice_id;
END
$$;

REVOKE ALL ON FUNCTION ple_data.rebuild_question_revision_statistics(text, integer, timestamptz)
    FROM PUBLIC;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
GRANT USAGE ON SCHEMA ple_audit TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_audit.automated_grading_receipt TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.question_revision TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_statistics_observation_receipt (
    automated_grading_receipt_id uuid PRIMARY KEY
        REFERENCES ple_audit.automated_grading_receipt(automated_grading_receipt_id)
        ON DELETE CASCADE,
    question_attempt_id uuid NOT NULL UNIQUE
        REFERENCES ple_private.question_attempt(question_attempt_id) ON DELETE CASCADE,
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    correct boolean NOT NULL,
    observed_at timestamptz NOT NULL,
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_private.question_statistics_observation_choice (
    automated_grading_receipt_id uuid NOT NULL
        REFERENCES ple_private.question_statistics_observation_receipt(automated_grading_receipt_id)
        ON DELETE CASCADE,
    choice_id text NOT NULL CHECK (
        choice_id = btrim(choice_id) AND char_length(choice_id) BETWEEN 1 AND 256
    ),
    PRIMARY KEY (automated_grading_receipt_id, choice_id)
);

ALTER TABLE ple_private.question_statistics_observation_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_statistics_observation_receipt FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_statistics_observation_choice ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_statistics_observation_choice FORCE ROW LEVEL SECURITY;

CREATE POLICY question_statistics_receipt_private_owner_access
    ON ple_private.question_statistics_observation_receipt
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_statistics_choice_private_owner_access
    ON ple_private.question_statistics_observation_choice
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_statistics_receipt_data_rebuild_read
    ON ple_private.question_statistics_observation_receipt
    FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY question_statistics_choice_data_rebuild_read
    ON ple_private.question_statistics_observation_choice
    FOR SELECT TO ple_data_owner USING (true);

REVOKE ALL ON TABLE ple_private.question_statistics_observation_receipt,
    ple_private.question_statistics_observation_choice FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT SELECT ON TABLE ple_private.question_statistics_observation_receipt,
    ple_private.question_statistics_observation_choice TO ple_data_owner;

-- The trusted grading commit has already locked and accepted its grading
-- lineage.  This function derives the exact immutable Revision and eligibility
-- from that lineage; callers supply only distinct opaque choice identifiers.
CREATE FUNCTION ple_private.capture_question_statistics_observation(
    p_automated_grading_receipt_id uuid,
    p_eligible_choice_ids text[]
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_data, ple_audit AS $$
DECLARE
    v_question_attempt_id uuid;
    v_question_id text;
    v_revision_number integer;
    v_correct boolean;
    v_observed_at timestamptz;
    v_inserted boolean := false;
BEGIN
    IF p_eligible_choice_ids IS NULL
       OR EXISTS (
           SELECT 1 FROM unnest(p_eligible_choice_ids) AS choice(choice_id)
            WHERE choice.choice_id <> btrim(choice.choice_id)
               OR char_length(choice.choice_id) NOT BETWEEN 1 AND 256
       )
       OR (SELECT count(*) FROM unnest(p_eligible_choice_ids))
          <> (SELECT count(DISTINCT choice_id)
                FROM unnest(p_eligible_choice_ids) AS choice(choice_id)) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question Statistics Observation choices must be distinct nonempty IDs';
    END IF;

    SELECT attempt.question_attempt_id, issued.question_id, issued.revision_number,
           result.correct, receipt.committed_at
      INTO v_question_attempt_id, v_question_id, v_revision_number, v_correct, v_observed_at
      FROM ple_audit.automated_grading_receipt AS receipt
      JOIN ple_private.grading_result AS result
        ON result.grading_result_id = receipt.grading_result_id
      JOIN ple_private.question_submission_grading AS grading
        ON grading.question_submission_grading_id = result.question_submission_grading_id
      JOIN ple_private.question_submission AS submission
        ON submission.submission_id = result.submission_id
      JOIN ple_private.question_attempt AS attempt
        ON attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = attempt.issued_question_id
     WHERE receipt.automated_grading_receipt_id = p_automated_grading_receipt_id
       AND grading.grading_state = 'graded'
       AND issued.question_statistics_eligibility;
    IF NOT FOUND THEN
        RETURN;
    END IF;

    INSERT INTO ple_private.question_statistics_observation_receipt (
        automated_grading_receipt_id, question_attempt_id, question_id,
        revision_number, correct, observed_at
    ) VALUES (
        p_automated_grading_receipt_id, v_question_attempt_id, v_question_id,
        v_revision_number, v_correct, v_observed_at
    ) ON CONFLICT (automated_grading_receipt_id) DO NOTHING
      RETURNING true INTO v_inserted;
    IF COALESCE(v_inserted, false) IS NOT TRUE THEN
        RETURN;
    END IF;

    INSERT INTO ple_private.question_statistics_observation_choice (
        automated_grading_receipt_id, choice_id
    ) SELECT p_automated_grading_receipt_id, choice.choice_id
        FROM unnest(p_eligible_choice_ids) AS choice(choice_id);

    PERFORM ple_data.rebuild_question_revision_statistics(
        v_question_id, v_revision_number, v_observed_at);
END
$$;

REVOKE ALL ON FUNCTION ple_private.capture_question_statistics_observation(uuid, text[])
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.capture_question_statistics_observation(uuid, text[])
    TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE POLICY automated_grading_receipt_private_statistics_read
    ON ple_audit.automated_grading_receipt FOR SELECT TO ple_private_owner USING (true);
GRANT USAGE ON SCHEMA ple_audit TO ple_private_owner;
GRANT SELECT ON TABLE ple_audit.automated_grading_receipt TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
GRANT EXECUTE ON FUNCTION ple_data.rebuild_question_revision_statistics(text, integer, timestamptz)
    TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.record_question_statistics_observation(
    p_automated_grading_receipt_id uuid,
    p_eligible_choice_ids text[]
) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    PERFORM ple_private.capture_question_statistics_observation(
        p_automated_grading_receipt_id, p_eligible_choice_ids);
END
$$;
REVOKE ALL ON FUNCTION ple_api.record_question_statistics_observation(uuid, text[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.record_question_statistics_observation(uuid, text[]) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.question_revision_statistics IS
    'Identity-free accepted-grade and correct counts for one immutable Question Revision.';
COMMENT ON TABLE ple_data.question_revision_choice_statistics IS
    'Identity-free selected eligible-choice counts for one immutable Question Revision.';
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.question_statistics_observation_receipt IS
    'One immutable accepted-grade observation, retained only as rebuildable aggregate evidence.';
COMMENT ON TABLE ple_private.question_statistics_observation_choice IS
    'Normalized opaque eligible-choice evidence for one statistics observation.';
RESET ROLE;
