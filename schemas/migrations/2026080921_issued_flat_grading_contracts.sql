-- PLE is pre-production. Issued flat attempts retain their exact private
-- grading authority rather than loading a mutable catalog/grader view at
-- first submit. There is intentionally no compatibility reader or backfill.
--
-- The JSON payload contains an answer-free immutable QuestionDefinition plus
-- a base64 private FlatQuestionGradingPayload. It is server-only and never
-- selected into a learner DTO or receipt response.

ALTER TABLE public.question_attempt
    ADD COLUMN flat_grading_required boolean NOT NULL,
    ADD COLUMN flat_grading_payload jsonb,
    ADD COLUMN flat_grading_payload_sha256 character(64),
    ADD CONSTRAINT question_attempt_flat_grading_payload_pair_check CHECK (
        (flat_grading_required
            AND flat_grading_payload IS NOT NULL
            AND flat_grading_payload_sha256 IS NOT NULL
            AND jsonb_typeof(flat_grading_payload) = 'object')
        OR
        (NOT flat_grading_required
            AND flat_grading_payload IS NULL
            AND flat_grading_payload_sha256 IS NULL)
    );
