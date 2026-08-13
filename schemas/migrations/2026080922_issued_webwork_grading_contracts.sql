-- PLE is pre-production. Every issued WeBWorK presentation retains the
-- exact server-only definition used for first grading, so submission never
-- resolves a newer catalog definition or rerenders to recover it. There is
-- intentionally no legacy reader, backfill, or source-reissue fallback.

ALTER TABLE public.question_attempt
    ADD COLUMN webwork_grading_required boolean NOT NULL,
    ADD COLUMN webwork_grading_payload jsonb,
    ADD COLUMN webwork_grading_payload_sha256 character(64),
    ADD CONSTRAINT question_attempt_webwork_grading_payload_pair_check CHECK (
        (webwork_grading_required
            AND webwork_grading_payload IS NOT NULL
            AND webwork_grading_payload_sha256 IS NOT NULL
            AND jsonb_typeof(webwork_grading_payload) = 'object')
        OR
        (NOT webwork_grading_required
            AND webwork_grading_payload IS NULL
            AND webwork_grading_payload_sha256 IS NULL)
    );
