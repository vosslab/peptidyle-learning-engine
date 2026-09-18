-- audit tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.





-- SQLx 0.9 performs CREATE TABLE IF NOT EXISTS on every forward run. Its
-- narrow CREATE grant is limited to this otherwise isolated schema (ASVS 13.2.2).
RESET ROLE;
CREATE TABLE ple_migration._sqlx_migrations (
    version bigint PRIMARY KEY,
    description text NOT NULL,
    installed_on timestamp with time zone NOT NULL DEFAULT now(),
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

ALTER TABLE ple_migration._sqlx_migrations OWNER TO ple_migrator;

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.assessment_unrelease_event (
    event_id uuid PRIMARY KEY,
    assessment_id uuid NOT NULL REFERENCES ple_data.assessment(assessment_id),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    assessment_edit_number bigint NOT NULL CHECK (assessment_edit_number > 0),
    assessment_attempt_count bigint NOT NULL CHECK (assessment_attempt_count >= 0),
    question_response_count bigint NOT NULL CHECK (question_response_count >= 0),
    assessment_submission_count bigint NOT NULL CHECK (assessment_submission_count >= 0),
    grading_result_count bigint NOT NULL CHECK (grading_result_count >= 0),
    outcome text NOT NULL DEFAULT 'completed',
    occurred_at timestamptz NOT NULL
);

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.assessment_unrelease_event IS 'role: event, Redacted completed Assessment Unrelease audit evidence: actor, Assessment, aggregate counts, and time only.';

RESET ROLE;
RESET ROLE;
COMMENT ON TABLE ple_migration._sqlx_migrations IS 'role: aggregate, deleted by retention policy for audit rows; the SQLx ledger is never purged. HUMAN_GUIDANCE.md Audit and Unrelease.';


ALTER TABLE ple_migration._sqlx_migrations OWNER TO ple_migrator;

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.assessment_unrelease_event IS 'role: event, Redacted completed Assessment Unrelease audit evidence: actor, Assessment, aggregate counts, and time only.';

RESET ROLE;
RESET ROLE;
COMMENT ON TABLE ple_migration._sqlx_migrations IS 'role: aggregate, deleted by retention policy for audit rows; the SQLx ledger is never purged. HUMAN_GUIDANCE.md Audit and Unrelease.';



SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.assessment_unrelease_event IS 'role: event, Redacted completed Assessment Unrelease audit evidence: actor, Assessment, aggregate counts, and time only.';

RESET ROLE;
RESET ROLE;
COMMENT ON TABLE ple_migration._sqlx_migrations IS 'role: aggregate, deleted by retention policy for audit rows; the SQLx ledger is never purged. HUMAN_GUIDANCE.md Audit and Unrelease.';

