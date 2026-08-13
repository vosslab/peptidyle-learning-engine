-- PLE is pre-production: the fresh ledger receives one complete immutable
-- issuance contract. Do not backfill, infer, or add compatibility readers for
-- attempts without their server-only grading envelope.
--
-- This answer-free envelope retains durable response IDs only for trusted
-- first-submit validation and grading. Public receipt snapshots continue to
-- expose presentation-scoped IDs and never serialize this payload.

ALTER TABLE public.question_attempt
    ADD COLUMN grading_envelope_payload jsonb,
    ADD COLUMN grading_envelope_payload_sha256 character(64),
    ADD COLUMN issued_feedback_disclosure text NOT NULL CHECK (
        issued_feedback_disclosure IN (
            'immediate_correctness', 'immediate_full', 'deferred', 'on_release'
        )
    ),
    ADD CONSTRAINT question_attempt_grading_envelope_check CHECK (
        (presentation_capability = 'envelope_v1'
            AND grading_envelope_payload IS NOT NULL
            AND grading_envelope_payload_sha256 IS NOT NULL)
        OR
        (presentation_capability = 'not_applicable'
            AND grading_envelope_payload IS NULL
            AND grading_envelope_payload_sha256 IS NULL)
    ),
    ADD CONSTRAINT question_attempt_grading_envelope_payload_check CHECK (
        grading_envelope_payload IS NULL
            OR jsonb_typeof(grading_envelope_payload) = 'object'
    );
