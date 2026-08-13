-- Forward migration: every first submission receipt retains the exact
-- answer-free presentation and disclosure decision used for its response.
--
-- PLE is pre-production and the baseline migration ledger is rebuilt for
-- disposable databases. There is intentionally no compatibility reader or
-- synthetic backfill for receipts created without these fields.

ALTER TABLE public.submission_receipt_snapshot
    ADD COLUMN presentation_payload jsonb,
    ADD COLUMN presentation_payload_sha256 character(64),
    ADD COLUMN presentation_required boolean NOT NULL,
    ADD COLUMN feedback_disclosure text NOT NULL,
    ADD CONSTRAINT submission_receipt_snapshot_presentation_requirement_check CHECK (
        (presentation_required
            AND presentation_payload IS NOT NULL
            AND presentation_payload_sha256 IS NOT NULL)
        OR
        (NOT presentation_required
            AND presentation_payload IS NULL
            AND presentation_payload_sha256 IS NULL)
    ),
    ADD CONSTRAINT submission_receipt_snapshot_presentation_payload_check CHECK (
        presentation_payload IS NULL OR jsonb_typeof(presentation_payload) = 'object'
    ),
    ADD CONSTRAINT submission_receipt_snapshot_feedback_disclosure_check CHECK (
        feedback_disclosure IN (
            'immediate_full',
            'immediate_correctness',
            'deferred',
            'on_release'
        )
    );
