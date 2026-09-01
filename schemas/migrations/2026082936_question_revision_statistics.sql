-- SD1 exact global Question Revision Statistics from accepted automated grades.

SET LOCAL ROLE ple_audit_owner;
GRANT USAGE ON SCHEMA ple_audit TO ple_private_owner, ple_api_owner;
GRANT REFERENCES ON TABLE ple_audit.automated_grading_receipt TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.question_revision_statistics (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    accepted_graded_attempt_count bigint NOT NULL DEFAULT 0
        CHECK (accepted_graded_attempt_count >= 0),
    correct_count bigint NOT NULL DEFAULT 0
        CHECK (correct_count >= 0 AND correct_count <= accepted_graded_attempt_count),
    updated_at timestamp with time zone NOT NULL,
    PRIMARY KEY (question_id, revision_number),
    CONSTRAINT question_revision_statistics_version_matches
        FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);
CREATE TABLE ple_data.question_revision_choice_statistics (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    choice_id text NOT NULL CHECK (choice_id <> ''),
    selected_count bigint NOT NULL CHECK (selected_count >= 0),
    PRIMARY KEY (question_id, revision_number, choice_id),
    CONSTRAINT question_revision_choice_statistics_version_matches
        FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT question_revision_choice_statistics_summary_matches
        FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision_statistics (question_id, revision_number)
);
ALTER TABLE ple_data.question_revision_statistics ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_statistics FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_choice_statistics ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_choice_statistics FORCE ROW LEVEL SECURITY;
CREATE POLICY question_revision_statistics_api_owner_access
    ON ple_data.question_revision_statistics
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY question_revision_choice_statistics_api_owner_access
    ON ple_data.question_revision_choice_statistics
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;
GRANT SELECT, INSERT, UPDATE ON TABLE ple_data.question_revision_statistics,
    ple_data.question_revision_choice_statistics TO ple_api_owner;
REVOKE ALL PRIVILEGES ON TABLE ple_data.question_revision_statistics,
    ple_data.question_revision_choice_statistics FROM PUBLIC;
COMMENT ON TABLE ple_data.question_revision_statistics IS
    'Identity-free accepted graded-attempt and correct counts for one immutable Question Revision.';
COMMENT ON TABLE ple_data.question_revision_choice_statistics IS
    'Identity-free eligible-choice selection counts for one immutable Question Revision.';
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.question_statistics_observation_receipt (
    automated_grading_receipt_id uuid PRIMARY KEY
        REFERENCES ple_audit.automated_grading_receipt (automated_grading_receipt_id),
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt (question_attempt_id),
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    correct boolean NOT NULL,
    observed_at timestamp with time zone NOT NULL,
    CONSTRAINT question_statistics_observation_receipt_version_matches
        FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);
ALTER TABLE ple_private.question_statistics_observation_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_statistics_observation_receipt FORCE ROW LEVEL SECURITY;
CREATE POLICY question_statistics_observation_receipt_api_owner_access
    ON ple_private.question_statistics_observation_receipt
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY question_attempt_statistics_api_owner_read
    ON ple_private.question_attempt
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_submission_statistics_api_owner_read
    ON ple_private.question_submission
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_submission_grading_statistics_api_owner_read
    ON ple_private.question_submission_grading
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY job_question_statistics_api_owner_read
    ON ple_private.job
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY issued_question_statistics_api_owner_read
    ON ple_private.issued_question
    FOR SELECT TO ple_api_owner USING (true);
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner;
GRANT SELECT ON TABLE ple_private.question_attempt, ple_private.question_submission,
    ple_private.question_submission_grading, ple_private.job, ple_private.issued_question TO ple_api_owner;
GRANT INSERT, SELECT ON TABLE ple_private.question_statistics_observation_receipt TO ple_api_owner;
REVOKE ALL PRIVILEGES ON TABLE ple_private.question_statistics_observation_receipt FROM PUBLIC;
COMMENT ON TABLE ple_private.question_statistics_observation_receipt IS
    'One private idempotency witness for a global Question Statistics Observation.';
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE POLICY automated_grading_receipt_question_statistics_api_owner_read
    ON ple_audit.automated_grading_receipt
    FOR SELECT TO ple_api_owner USING (true);
GRANT USAGE ON SCHEMA ple_audit TO ple_api_owner;
GRANT SELECT ON TABLE ple_audit.automated_grading_receipt TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.record_question_statistics_observation(
    p_automated_grading_receipt_id uuid,
    p_correct boolean,
    p_eligible_choice_ids text[]
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private, ple_data, ple_audit
AS $$
DECLARE
    v_question_attempt_id uuid;
    v_question_id text;
    v_revision_number integer;
    v_observed_at timestamp with time zone;
    v_inserted boolean;
BEGIN
    IF p_eligible_choice_ids IS NULL
       OR EXISTS (
           SELECT 1
             FROM unnest(p_eligible_choice_ids) AS choice(choice_id)
            WHERE choice.choice_id = ''
       )
       OR (SELECT count(*) FROM unnest(p_eligible_choice_ids))
          <> (SELECT count(DISTINCT choice_id) FROM unnest(p_eligible_choice_ids) AS choice(choice_id)) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Statistics Observation choices must be distinct nonempty eligible IDs';
    END IF;

    SELECT question_attempt.question_attempt_id, issued.question_id, issued.revision_number,
           receipt.committed_at
      INTO v_question_attempt_id, v_question_id, v_revision_number, v_observed_at
      FROM ple_audit.automated_grading_receipt AS receipt
      JOIN ple_private.grading_result AS result
        ON result.grading_result_id = receipt.grading_result_id
      JOIN ple_private.question_submission_grading AS grading
        ON grading.question_submission_grading_id = result.question_submission_grading_id
      JOIN ple_private.job AS job
        ON job.job_id = grading.job_id
      JOIN ple_private.question_submission AS submission
        ON submission.submission_id = grading.submission_id
      JOIN ple_private.question_attempt AS question_attempt
        ON question_attempt.question_attempt_id = submission.question_attempt_id
      JOIN ple_private.issued_question AS issued
        ON issued.issued_question_id = question_attempt.issued_question_id
     WHERE receipt.automated_grading_receipt_id = p_automated_grading_receipt_id
       AND grading.grading_state = 'graded'
       AND job.state = 'completed'
       AND issued.statistics_eligible;
    IF v_question_attempt_id IS NULL THEN
        RETURN;
    END IF;

    INSERT INTO ple_private.question_statistics_observation_receipt (
        automated_grading_receipt_id, question_attempt_id, question_id, revision_number, correct, observed_at
    ) VALUES (
        p_automated_grading_receipt_id, v_question_attempt_id, v_question_id, v_revision_number, p_correct, v_observed_at
    ) ON CONFLICT (automated_grading_receipt_id) DO NOTHING
    RETURNING true INTO v_inserted;
    IF COALESCE(v_inserted, false) IS NOT TRUE THEN
        RETURN;
    END IF;

    INSERT INTO ple_data.question_revision_statistics (
        question_id, revision_number, accepted_graded_attempt_count, correct_count, updated_at
    ) VALUES (
        v_question_id, v_revision_number, 1, CASE WHEN p_correct THEN 1 ELSE 0 END, v_observed_at
    ) ON CONFLICT (question_id, revision_number) DO UPDATE
        SET accepted_graded_attempt_count =
                ple_data.question_revision_statistics.accepted_graded_attempt_count + 1,
            correct_count = ple_data.question_revision_statistics.correct_count
                + CASE WHEN p_correct THEN 1 ELSE 0 END,
            updated_at = EXCLUDED.updated_at;

    INSERT INTO ple_data.question_revision_choice_statistics (
        question_id, revision_number, choice_id, selected_count
    ) SELECT v_question_id, v_revision_number, choice.choice_id, 1
        FROM unnest(p_eligible_choice_ids) AS choice(choice_id)
    ON CONFLICT (question_id, revision_number, choice_id) DO UPDATE
        SET selected_count = ple_data.question_revision_choice_statistics.selected_count + 1;
END
$$;
REVOKE ALL PRIVILEGES ON FUNCTION ple_api.record_question_statistics_observation(
    uuid, boolean, text[]
) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.record_question_statistics_observation(
    uuid, boolean, text[]
) TO ple_app;
RESET ROLE;
