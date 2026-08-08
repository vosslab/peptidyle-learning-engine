-- MOD-API-RUN: authenticated ownership, logical positions, and replay-safe submissions.

-- Existing databases may contain pre-auth enrollment payloads. New writes must
-- carry the authenticated user link; legacy ownership requires an explicit
-- institution-led migration rather than equating UserId with StudentId.
ALTER TABLE enrollment
    ADD COLUMN user_id uuid,
    ADD CONSTRAINT enrollment_user_required_check
        CHECK (user_id IS NOT NULL) NOT VALID;

CREATE UNIQUE INDEX enrollment_user_assignment_idx
    ON enrollment (tenant_id, assignment_id, user_id)
    WHERE user_id IS NOT NULL;

-- Position distinguishes repeated problem/version references and groups
-- retries under the same logical assignment item.
ALTER TABLE question_attempt
    ADD COLUMN assignment_position integer,
    ADD CONSTRAINT question_attempt_position_required_check
        CHECK (assignment_position IS NOT NULL AND assignment_position >= 0)
        NOT VALID;

CREATE INDEX question_attempt_run_position_idx
    ON question_attempt (tenant_id, run_id, assignment_position, occurred_at, attempt_id);

-- The monthly submission stream cannot enforce a global idempotency key
-- without putting time in the key. This compact hash-partitioned table owns the
-- durable first-result decision; the high-volume event tables remain monthly.
CREATE TABLE submission_idempotency (
    tenant_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    idempotency_key text NOT NULL
        CHECK (
            octet_length(idempotency_key) BETWEEN 1 AND 200
            AND idempotency_key !~ '[[:space:][:cntrl:]]'
        ),
    response_sha256 character(64) NOT NULL,
    submitted_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, attempt_id)
) PARTITION BY HASH (tenant_id);

DO $$
DECLARE
    remainder integer;
BEGIN
    FOR remainder IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE submission_idempotency_p%s PARTITION OF submission_idempotency '
            'FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            remainder,
            remainder
        );
    END LOOP;
END
$$;

CREATE INDEX submission_idempotency_time_idx
    ON submission_idempotency (tenant_id, submitted_at, attempt_id);

ALTER TABLE submission
    ADD COLUMN attempt_id uuid,
    ADD COLUMN idempotency_key text,
    ADD CONSTRAINT submission_attempt_required_check
        CHECK (attempt_id IS NOT NULL) NOT VALID,
    ADD CONSTRAINT submission_idempotency_required_check
        CHECK (idempotency_key IS NOT NULL) NOT VALID;

ALTER TABLE grade_event
    ADD COLUMN attempt_id uuid,
    ADD CONSTRAINT grade_event_attempt_required_check
        CHECK (attempt_id IS NOT NULL) NOT VALID;

ALTER TABLE submission_idempotency ENABLE ROW LEVEL SECURITY;
ALTER TABLE submission_idempotency FORCE ROW LEVEL SECURITY;
CREATE POLICY submission_idempotency_tenant ON submission_idempotency
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

GRANT SELECT, INSERT ON submission_idempotency TO ple_app;
