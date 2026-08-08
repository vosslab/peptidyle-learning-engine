-- MOD-UI-ATTEMPT: durable key-free preparation of the next question.
--
-- A reservation is intentionally distinct from question_attempt: it cannot
-- start a timer, accept a response, affect a summary, or carry an attempt ID.
CREATE TABLE question_prefetch (
    tenant_id uuid NOT NULL,
    run_id uuid NOT NULL,
    predecessor_attempt_id uuid NOT NULL,
    predecessor_occurred_at timestamptz NOT NULL,
    assignment_position integer NOT NULL CHECK (assignment_position >= 0),
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    payload jsonb NOT NULL,
    payload_sha256 character(64) NOT NULL,
    PRIMARY KEY (tenant_id, run_id, predecessor_attempt_id, assignment_position),
    FOREIGN KEY (tenant_id, run_id)
        REFERENCES assignment_run(tenant_id, run_id),
    FOREIGN KEY (tenant_id, predecessor_attempt_id, predecessor_occurred_at)
        REFERENCES question_attempt(tenant_id, attempt_id, occurred_at)
);

CREATE INDEX question_prefetch_run_idx
    ON question_prefetch (tenant_id, run_id, predecessor_attempt_id);

ALTER TABLE question_prefetch ENABLE ROW LEVEL SECURITY;
ALTER TABLE question_prefetch FORCE ROW LEVEL SECURITY;
CREATE POLICY question_prefetch_tenant ON question_prefetch
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

GRANT SELECT, INSERT, DELETE ON question_prefetch TO ple_app;

-- The response receipt points at this compact immutable decision rather than
-- discovering whichever attempt happens to be active when it is replayed.
CREATE TABLE submission_next_attempt (
    tenant_id uuid NOT NULL,
    predecessor_attempt_id uuid NOT NULL,
    next_attempt_id uuid,
    next_attempt_occurred_at timestamptz,
    finalized_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, predecessor_attempt_id),
    CHECK ((next_attempt_id IS NULL) = (next_attempt_occurred_at IS NULL)),
    FOREIGN KEY (tenant_id, predecessor_attempt_id)
        REFERENCES submission_idempotency(tenant_id, attempt_id),
    FOREIGN KEY (tenant_id, next_attempt_id, next_attempt_occurred_at)
        REFERENCES question_attempt(tenant_id, attempt_id, occurred_at)
);

ALTER TABLE submission_next_attempt ENABLE ROW LEVEL SECURITY;
ALTER TABLE submission_next_attempt FORCE ROW LEVEL SECURITY;
CREATE POLICY submission_next_attempt_tenant ON submission_next_attempt
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

GRANT SELECT, INSERT ON submission_next_attempt TO ple_app;
