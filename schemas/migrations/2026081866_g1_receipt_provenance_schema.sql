-- WP-PROF-G1 / G1-W7: receipt provenance schema for a disposable baseline.
BEGIN;

-- Serialize the fail-closed clean-volume preflight against receipt writers before
-- tightening either append-only table with non-null provenance.
LOCK TABLE public.grading_execution_receipt, public.grading_operation_receipt
    IN ACCESS EXCLUSIVE MODE;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.grading_execution_receipt)
       OR EXISTS (SELECT 1 FROM public.grading_operation_receipt) THEN
        RAISE EXCEPTION
            'G1 receipt provenance migration requires empty grading receipt tables; rebuild the disposable pre-production volume or plan immutable receipt augmentation';
    END IF;
END;
$$;

ALTER TABLE public.grading_execution_receipt
    ADD COLUMN safe_category text NOT NULL,
    ADD COLUMN actor_id uuid,
    ADD CONSTRAINT grading_execution_receipt_identity_check CHECK (
        (actor_id IS NULL) <> (worker_id IS NULL)
    ),
    ADD CONSTRAINT grading_execution_receipt_safe_category_check CHECK (
        (safe_category = 'accepted_submission'
            AND resulting_state = 'ready' AND actor_id IS NOT NULL AND worker_id IS NULL)
        OR (safe_category = 'instructor_retry'
            AND resulting_state = 'ready' AND actor_id IS NOT NULL AND worker_id IS NULL)
        OR (safe_category = 'worker_claim'
            AND resulting_state = 'running' AND actor_id IS NULL AND worker_id IS NOT NULL)
        OR (safe_category = 'graded'
            AND resulting_state = 'completed' AND actor_id IS NULL AND worker_id IS NOT NULL)
        OR (safe_category = 'dependency_retry'
            AND resulting_state = 'retry_wait' AND actor_id IS NULL AND worker_id IS NOT NULL)
        OR (safe_category = ANY ('{grader_contract_failure,grader_execution_failure,issued_evidence_integrity,retry_exhausted}')
            AND resulting_state = 'exception' AND actor_id IS NULL AND worker_id IS NOT NULL)
    );

ALTER TABLE public.grading_operation_receipt
    ADD COLUMN safe_category text NOT NULL,
    ADD CONSTRAINT grading_operation_receipt_safe_category_check CHECK (
        (action_kind = 'retry' AND safe_category = 'instructor_retry')
        OR (action_kind = 'recalculate' AND safe_category = 'instructor_recalculation')
    );

COMMIT;
