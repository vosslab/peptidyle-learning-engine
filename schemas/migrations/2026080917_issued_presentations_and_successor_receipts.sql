-- PLE is pre-production: fresh disposable ledgers receive the full issuance
-- contract directly. Do not backfill, infer, or add readers for attempts that
-- predate this immutable presentation and successor receipt shape.

ALTER TABLE public.question_attempt
    ADD COLUMN presentation_capability text NOT NULL,
    ADD COLUMN presentation_payload jsonb,
    ADD COLUMN presentation_payload_sha256 character(64),
    ADD CONSTRAINT question_attempt_presentation_capability_check CHECK (
        presentation_capability IN ('envelope_v1', 'not_applicable')
    ),
    ADD CONSTRAINT question_attempt_presentation_snapshot_check CHECK (
        (presentation_capability = 'envelope_v1'
            AND presentation_descriptor_version IS NOT NULL
            AND presentation_payload IS NOT NULL
            AND presentation_payload_sha256 IS NOT NULL)
        OR
        (presentation_capability = 'not_applicable'
            AND presentation_descriptor_version IS NULL
            AND presentation_payload IS NULL
            AND presentation_payload_sha256 IS NULL)
    ),
    ADD CONSTRAINT question_attempt_presentation_payload_check CHECK (
        presentation_payload IS NULL OR jsonb_typeof(presentation_payload) = 'object'
    );

ALTER TABLE public.submission_next_attempt
    ADD COLUMN next_payload jsonb,
    ADD COLUMN next_payload_sha256 character(64),
    ADD CONSTRAINT submission_next_attempt_payload_check CHECK (
        (next_attempt_id IS NULL
            AND next_payload IS NULL
            AND next_payload_sha256 IS NULL)
        OR
        (next_attempt_id IS NOT NULL
            AND next_payload IS NOT NULL
            AND next_payload_sha256 IS NOT NULL
            AND jsonb_typeof(next_payload) = 'object')
    );
