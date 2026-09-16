-- Immutable grading evidence.  Every result is rooted at one accepted
-- Submission and retained Assessment Attempt evidence; Assessment Revision is not an
-- interpretation source.

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_response_grading (
    question_response_grading_id uuid PRIMARY KEY,
    question_response_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_response(question_response_id)
        ON DELETE CASCADE,
    grading_state text NOT NULL DEFAULT 'graded' CHECK (grading_state = 'graded'),
    created_at timestamptz NOT NULL,
    completed_at timestamptz,
    UNIQUE (question_response_grading_id, question_response_id),
    CHECK (completed_at IS NOT NULL),
    CHECK (completed_at IS NULL OR completed_at >= created_at)
);

CREATE TABLE ple_private.grading_result (
    grading_result_id uuid PRIMARY KEY,
    question_response_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_response(question_response_id)
        ON DELETE CASCADE,
    question_response_grading_id uuid NOT NULL UNIQUE,
    question_attempt_id uuid NOT NULL UNIQUE REFERENCES ple_private.question_attempt(question_attempt_id)
        ON DELETE CASCADE,
    -- The Question Backend owns response interpretation.  PLE retains only
    -- its normalized immutable outcome; point values remain Assessment
    -- configuration and are applied by score readers.
    normalized_credit numeric NOT NULL CHECK (
        normalized_credit >= 0 AND normalized_credit <= 1
    ),
    recorded_at timestamptz NOT NULL,
    FOREIGN KEY (question_response_id, question_attempt_id)
        REFERENCES ple_private.question_response(question_response_id, question_attempt_id)
        ON DELETE CASCADE,
    FOREIGN KEY (question_response_grading_id, question_response_id)
        REFERENCES ple_private.question_response_grading(question_response_grading_id, question_response_id)
        ON DELETE CASCADE,
    UNIQUE (question_response_grading_id, grading_result_id)
);

-- Applies current Assessment Entry points to retained backend credit.  The
-- stable Entry identifier remains present after a released Assessment save,
-- including when an Entry is retired, so historical Student Work continues
-- to have a current score without consulting a Question Backend.
-- NULL credit means no retained backend outcome (an unanswered Question),
-- not an evaluated incorrect response with zero credit. Unanswered work earns
-- zero even under full_credit; its current points remain in the denominator.
-- ASVS 1.2.4, 2.3.2: fixed parameters preserve the unanswered-zero rule without
-- changing immutable grading evidence or consulting a backend.
CREATE FUNCTION ple_private.score_recorded_credit(
    p_normalized_credit numeric,
    p_scoring_rule text,
    p_point_value numeric
) RETURNS TABLE (points_earned numeric, points_possible numeric)
LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog AS $$
    SELECT CASE
               WHEN p_normalized_credit IS NULL THEN 0::numeric
               WHEN p_scoring_rule = 'full_credit' THEN p_point_value
               WHEN p_scoring_rule = 'excluded' THEN 0::numeric
               ELSE p_point_value * p_normalized_credit
           END,
           p_point_value
     WHERE (p_normalized_credit IS NULL OR p_normalized_credit BETWEEN 0 AND 1)
       AND p_scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')
       AND p_point_value >= 0
$$;

-- Applies the Assessment Type and Entry scoring treatment only to Course-grade
-- denominator contribution. Earned points remain owned by score_recorded_credit,
-- and Assessment Attempt performance continues to use its raw points possible.
-- ASVS 1.2.4, 2.1.1: fixed parameters and closed stored values define the
-- complete grade-contribution rule without dynamic SQL.
CREATE FUNCTION ple_private.grade_contribution_points_possible(
    p_assessment_type text,
    p_scoring_rule text,
    p_authored_points_possible numeric
) RETURNS numeric
LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog AS $$
    SELECT CASE
               WHEN p_assessment_type = 'bonus_assignment'
                    OR p_scoring_rule IN ('extra_credit', 'excluded')
                   THEN 0::numeric
               ELSE p_authored_points_possible
           END
     WHERE p_assessment_type IN (
               'regular_assignment', 'practice_question_assignment',
               'bonus_assignment', 'quiz', 'exam'
           )
       AND p_scoring_rule IN ('normal', 'full_credit', 'extra_credit', 'excluded')
       AND p_authored_points_possible >= 0
$$;

CREATE FUNCTION ple_private.reject_grading_evidence_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Grading evidence is immutable';
END $$;

CREATE TRIGGER question_response_grading_is_immutable
BEFORE UPDATE ON ple_private.question_response_grading
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_grading_evidence_change();
CREATE TRIGGER question_response_grading_delete_is_guarded
BEFORE DELETE ON ple_private.question_response_grading
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();
CREATE TRIGGER grading_result_is_immutable
BEFORE UPDATE ON ple_private.grading_result
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_grading_evidence_change();
CREATE TRIGGER grading_result_delete_is_guarded
BEFORE DELETE ON ple_private.grading_result
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

-- The audit owner owns the receipt relation but its receipt is an exclusive
-- child of private Student Work.  It needs only the FK/trigger capability.
GRANT USAGE ON SCHEMA ple_private TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_private.grading_result TO ple_audit_owner;
GRANT EXECUTE ON FUNCTION ple_private.reject_student_work_delete() TO ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.automated_grading_receipt (
    automated_grading_receipt_id uuid PRIMARY KEY,
    question_response_grading_id uuid NOT NULL,
    grading_result_id uuid NOT NULL UNIQUE,
    committed_at timestamptz NOT NULL,
    automated_grading_receipt_checksum bytea NOT NULL
        CHECK (octet_length(automated_grading_receipt_checksum) = 32),
    FOREIGN KEY (question_response_grading_id, grading_result_id)
        REFERENCES ple_private.grading_result(question_response_grading_id, grading_result_id)
        ON DELETE CASCADE
);
CREATE FUNCTION ple_audit.reject_automated_grading_receipt_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_audit AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Automated Grading Receipt is immutable';
END $$;
CREATE TRIGGER automated_grading_receipt_is_immutable
BEFORE UPDATE ON ple_audit.automated_grading_receipt
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_automated_grading_receipt_change();
CREATE TRIGGER automated_grading_receipt_delete_is_guarded
BEFORE DELETE ON ple_audit.automated_grading_receipt
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_student_work_delete();

ALTER TABLE ple_audit.automated_grading_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.automated_grading_receipt FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_audit.automated_grading_receipt FROM PUBLIC;
CREATE POLICY automated_grading_receipt_audit_owner_access
    ON ple_audit.automated_grading_receipt
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);
CREATE POLICY automated_grading_receipt_api_owner_read
    ON ple_audit.automated_grading_receipt
    FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY automated_grading_receipt_private_owner_insert
    ON ple_audit.automated_grading_receipt
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY automated_grading_receipt_private_owner_read
    ON ple_audit.automated_grading_receipt
    FOR SELECT TO ple_private_owner USING (true);
GRANT USAGE ON SCHEMA ple_audit TO ple_api_owner;
GRANT SELECT ON ple_audit.automated_grading_receipt TO ple_api_owner;
GRANT SELECT, INSERT ON ple_audit.automated_grading_receipt TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.question_response_grading ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_response_grading FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.grading_result ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.grading_result FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.question_response_grading, ple_private.grading_result FROM PUBLIC;
CREATE POLICY question_response_grading_private_owner_access
    ON ple_private.question_response_grading FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY grading_result_private_owner_access
    ON ple_private.grading_result FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;

-- Inserts the immutable evidence for one backend result that was obtained
-- before the finalization transaction. The caller has already revalidated the
-- saved-response snapshot and locked the Assessment root.
CREATE FUNCTION ple_private.record_direct_automated_grading_result(
    p_question_response_id uuid,
    p_question_attempt_id uuid,
    p_normalized_credit numeric,
    p_recorded_at timestamptz
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private, ple_audit AS $$
DECLARE grading_id uuid := pg_catalog.gen_random_uuid();
DECLARE result_id uuid := pg_catalog.gen_random_uuid();
DECLARE receipt_id uuid := pg_catalog.gen_random_uuid();
DECLARE calculated_checksum bytea;
BEGIN
    IF p_question_response_id IS NULL OR p_question_attempt_id IS NULL
       OR p_normalized_credit IS NULL OR p_normalized_credit < 0
       OR p_normalized_credit > 1 OR p_recorded_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Direct automated grading facts are invalid';
    END IF;
    PERFORM 1 FROM ple_private.question_response AS submission
     WHERE submission.question_response_id = p_question_response_id
       AND submission.question_attempt_id = p_question_attempt_id
     FOR KEY SHARE;
    IF NOT FOUND OR EXISTS (
        SELECT 1 FROM ple_private.grading_result AS result
         WHERE result.question_attempt_id = p_question_attempt_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Direct automated grading target is unavailable';
    END IF;
    INSERT INTO ple_private.question_response_grading (
        question_response_grading_id, question_response_id, grading_state, created_at, completed_at
    ) VALUES (
        grading_id, p_question_response_id, 'graded', p_recorded_at, p_recorded_at
    );
    INSERT INTO ple_private.grading_result (
        grading_result_id, question_response_id, question_response_grading_id,
        question_attempt_id, normalized_credit, recorded_at
    ) VALUES (
        result_id, p_question_response_id, grading_id, p_question_attempt_id,
        p_normalized_credit, p_recorded_at
    );
    calculated_checksum := pg_catalog.sha256(
        pg_catalog.convert_to('ple:automated-grading-receipt:v1', 'UTF8')
        || pg_catalog.uuid_send(receipt_id)
        || pg_catalog.uuid_send(result_id)
        || pg_catalog.uuid_send(grading_id)
        || pg_catalog.uuid_send(p_question_response_id)
        || pg_catalog.uuid_send(p_question_attempt_id)
        || pg_catalog.numeric_send(p_normalized_credit)
        || pg_catalog.int8send((extract(epoch FROM p_recorded_at) * 1000)::bigint)
    );
    INSERT INTO ple_audit.automated_grading_receipt (
        automated_grading_receipt_id, question_response_grading_id,
        grading_result_id, committed_at, automated_grading_receipt_checksum
    ) VALUES (
        receipt_id, grading_id, result_id, p_recorded_at, calculated_checksum
    );
    PERFORM ple_private.capture_question_statistics_observation(receipt_id, ARRAY[]::text[]);
END $$;
