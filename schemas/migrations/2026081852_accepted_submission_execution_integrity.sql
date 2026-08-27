-- WP-PROF-G1 / G1-W4: immutable automated-grading evidence.
--
-- 1851 provides the roles and structural canonical-evidence columns.  This
-- migration owns only the write-time integrity boundary: normalized receipt
-- evidence, append-only automated results, and the sealed writer for rows
-- belonging to an accepted-submission execution.  RLS and table privileges
-- belong to 1854 so this migration stays independently reviewable.

BEGIN;

-- A receipt contains the exact terminal, answer-free attempt representation
-- that existed when the writer completed its work.  The canonical-source
-- constraints installed in 1851 verify source/projection/digest coherence;
-- this trigger binds the projection to the row identity and prevents later
-- substitution of receipt evidence.  ASVS 1.5.2-1.5.3 and 2.2.1-2.2.3.
CREATE FUNCTION public.ple_guard_receipt_attempt_snapshot()
RETURNS trigger
LANGUAGE plpgsql
SECURITY INVOKER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF current_user = 'ple_retention_broker' THEN
            RETURN OLD;
        END IF;

        RAISE EXCEPTION 'receipt attempt snapshot is retention-deleted only'
            USING ERRCODE = '42501';
    END IF;

    IF NEW.receipt_attempt_payload ->> 'id' IS DISTINCT FROM NEW.attempt_id::text
       OR NEW.receipt_attempt_payload ->> 'tenant' IS DISTINCT FROM NEW.tenant_id::text
       OR NEW.receipt_attempt_payload -> 'response' <> 'null'::jsonb
       OR NEW.receipt_attempt_payload ->> 'status' NOT IN (
            'submitted',
            'auto_submitted',
            'needs_manual_grading',
            'exempt'
       )
    THEN
        RAISE EXCEPTION 'receipt attempt snapshot is not answer-free terminal evidence'
            USING ERRCODE = '22023';
    END IF;

    IF TG_OP = 'UPDATE'
       AND (
            NEW.receipt_attempt_payload IS DISTINCT FROM OLD.receipt_attempt_payload
            OR NEW.receipt_attempt_canonical_json
                IS DISTINCT FROM OLD.receipt_attempt_canonical_json
            OR NEW.receipt_attempt_payload_sha256
                IS DISTINCT FROM OLD.receipt_attempt_payload_sha256
            OR NEW.run_canonical_json IS DISTINCT FROM OLD.run_canonical_json
            OR NEW.run_payload IS DISTINCT FROM OLD.run_payload
            OR NEW.run_payload_sha256 IS DISTINCT FROM OLD.run_payload_sha256
            OR NEW.summary_canonical_json IS DISTINCT FROM OLD.summary_canonical_json
            OR NEW.summary_payload IS DISTINCT FROM OLD.summary_payload
            OR NEW.summary_payload_sha256 IS DISTINCT FROM OLD.summary_payload_sha256
            OR NEW.presentation_canonical_json
                IS DISTINCT FROM OLD.presentation_canonical_json
            OR NEW.presentation_payload IS DISTINCT FROM OLD.presentation_payload
            OR NEW.presentation_payload_sha256
                IS DISTINCT FROM OLD.presentation_payload_sha256
            OR NEW.canonical_json_version IS DISTINCT FROM OLD.canonical_json_version
       )
    THEN
        RAISE EXCEPTION 'receipt attempt snapshot is immutable' USING ERRCODE = '42501';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER submission_receipt_snapshot_attempt_guard
    BEFORE INSERT OR UPDATE OR DELETE ON public.submission_receipt_snapshot
    FOR EACH ROW
    EXECUTE FUNCTION public.ple_guard_receipt_attempt_snapshot();

-- A persisted automated result becomes immutable evidence.  Retention is the
-- only deletion authority, while a pending evaluation can still transition to
-- its first result through the sealed accepted-submission writer.  ASVS 2.3.1,
-- 2.3.3, 8.2.2, and 15.4.2.
CREATE FUNCTION public.ple_forbid_automated_result_mutation()
RETURNS trigger
LANGUAGE plpgsql
SECURITY INVOKER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF current_user = 'ple_retention_broker' THEN
            RETURN OLD;
        END IF;

        RAISE EXCEPTION 'automated result evidence is retention-deleted only'
            USING ERRCODE = '42501';
    END IF;

    IF TG_OP = 'UPDATE'
       AND OLD.automated_result_canonical_json IS NOT NULL
       AND (
            NEW.grading_status IS DISTINCT FROM OLD.grading_status
            OR NEW.credit_fraction IS DISTINCT FROM OLD.credit_fraction
            OR NEW.correct IS DISTINCT FROM OLD.correct
            OR NEW.payload IS DISTINCT FROM OLD.payload
            OR NEW.payload_sha256 IS DISTINCT FROM OLD.payload_sha256
            OR NEW.automated_result_canonical_json
                IS DISTINCT FROM OLD.automated_result_canonical_json
            OR NEW.automated_result_sha256 IS DISTINCT FROM OLD.automated_result_sha256
            OR NEW.automated_result_canonical_json_version
                IS DISTINCT FROM OLD.automated_result_canonical_json_version
            OR NEW.evaluated_at IS DISTINCT FROM OLD.evaluated_at
            OR NEW.evaluation_revision IS DISTINCT FROM OLD.evaluation_revision
       )
    THEN
        RAISE EXCEPTION 'automated result evidence is immutable' USING ERRCODE = '42501';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER submission_evaluation_automated_result_append_only
    BEFORE UPDATE OR DELETE ON public.submission_evaluation
    FOR EACH ROW
    EXECUTE FUNCTION public.ple_forbid_automated_result_mutation();

-- An attempt with a grading_execution row is an accepted-submission attempt.
-- Its evaluation, feedback, and completed receipt all have one writer.  The
-- later authority migration grants that writer only the rows and columns it
-- needs; this guard independently prevents a differently privileged writer
-- from bypassing the lifecycle.  ASVS 2.3.1, 8.2.1-8.2.3, and 15.3.3.
CREATE FUNCTION public.ple_guard_accepted_execution_evidence_writer()
RETURNS trigger
LANGUAGE plpgsql
SECURITY INVOKER
SET search_path TO 'pg_catalog', 'public', pg_temp
AS $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM public.grading_execution AS execution
        WHERE execution.tenant_id = NEW.tenant_id
          AND execution.attempt_id = NEW.attempt_id
    )
       AND current_user <> 'ple_accepted_submission_execution_worker'
    THEN
        RAISE EXCEPTION 'accepted-submission evidence requires its sealed worker'
            USING ERRCODE = '42501';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER submission_evaluation_accepted_execution_writer
    BEFORE INSERT OR UPDATE ON public.submission_evaluation
    FOR EACH ROW
    EXECUTE FUNCTION public.ple_guard_accepted_execution_evidence_writer();

CREATE TRIGGER attempt_feedback_accepted_execution_writer
    BEFORE INSERT ON public.attempt_feedback
    FOR EACH ROW
    EXECUTE FUNCTION public.ple_guard_accepted_execution_evidence_writer();

CREATE TRIGGER submission_receipt_snapshot_accepted_execution_writer
    BEFORE INSERT ON public.submission_receipt_snapshot
    FOR EACH ROW
    EXECUTE FUNCTION public.ple_guard_accepted_execution_evidence_writer();

-- Guard code is owned by the sealed definer and never presents a callable
-- application surface.  PostgreSQL invokes each guard through its trigger;
-- these explicit per-function statements make the capability auditable.
ALTER FUNCTION public.ple_guard_receipt_attempt_snapshot()
    OWNER TO ple_accepted_submission_execution_worker;

ALTER FUNCTION public.ple_forbid_automated_result_mutation()
    OWNER TO ple_accepted_submission_execution_worker;

ALTER FUNCTION public.ple_guard_accepted_execution_evidence_writer()
    OWNER TO ple_accepted_submission_execution_worker;

REVOKE ALL ON FUNCTION public.ple_guard_receipt_attempt_snapshot() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_guard_receipt_attempt_snapshot() FROM ple_app;
REVOKE ALL ON FUNCTION public.ple_guard_receipt_attempt_snapshot()
    FROM ple_accepted_submission_execution;

REVOKE ALL ON FUNCTION public.ple_forbid_automated_result_mutation() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_forbid_automated_result_mutation() FROM ple_app;
REVOKE ALL ON FUNCTION public.ple_forbid_automated_result_mutation()
    FROM ple_accepted_submission_execution;

REVOKE ALL ON FUNCTION public.ple_guard_accepted_execution_evidence_writer()
    FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_guard_accepted_execution_evidence_writer()
    FROM ple_app;
REVOKE ALL ON FUNCTION public.ple_guard_accepted_execution_evidence_writer()
    FROM ple_accepted_submission_execution;

-- Make this integrity layer self-describing at install time.  The later
-- authority migration verifies table access; this check verifies only the
-- functions and triggers that this migration owns.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM unnest(ARRAY[
            'public.ple_guard_receipt_attempt_snapshot()'::regprocedure,
            'public.ple_forbid_automated_result_mutation()'::regprocedure,
            'public.ple_guard_accepted_execution_evidence_writer()'::regprocedure
        ]) AS expected(procedure_id)
        JOIN pg_catalog.pg_proc AS procedure_row
            ON procedure_row.oid = expected.procedure_id
        WHERE procedure_row.proowner <>
                  'ple_accepted_submission_execution_worker'::regrole
           OR procedure_row.prosecdef
           OR procedure_row.proconfig IS DISTINCT FROM
                  ARRAY['search_path=pg_catalog, public, pg_temp']
    ) THEN
        RAISE EXCEPTION 'accepted-submission integrity function authority is unsafe';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM unnest(ARRAY[
            'public.ple_guard_receipt_attempt_snapshot()',
            'public.ple_forbid_automated_result_mutation()',
            'public.ple_guard_accepted_execution_evidence_writer()'
        ]) AS expected(function_name)
        WHERE has_function_privilege('public', expected.function_name, 'EXECUTE')
           OR has_function_privilege('ple_app', expected.function_name, 'EXECUTE')
           OR has_function_privilege(
                  'ple_accepted_submission_execution',
                  expected.function_name,
                  'EXECUTE'
              )
    ) THEN
        RAISE EXCEPTION 'accepted-submission integrity guard is callable';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM (
            VALUES
                (
                    'public.submission_receipt_snapshot'::regclass,
                    'submission_receipt_snapshot_attempt_guard',
                    'public.ple_guard_receipt_attempt_snapshot()'::regprocedure,
                    31::smallint
                ),
                (
                    'public.submission_evaluation'::regclass,
                    'submission_evaluation_automated_result_append_only',
                    'public.ple_forbid_automated_result_mutation()'::regprocedure,
                    27::smallint
                ),
                (
                    'public.submission_evaluation'::regclass,
                    'submission_evaluation_accepted_execution_writer',
                    'public.ple_guard_accepted_execution_evidence_writer()'::regprocedure,
                    23::smallint
                ),
                (
                    'public.attempt_feedback'::regclass,
                    'attempt_feedback_accepted_execution_writer',
                    'public.ple_guard_accepted_execution_evidence_writer()'::regprocedure,
                    7::smallint
                ),
                (
                    'public.submission_receipt_snapshot'::regclass,
                    'submission_receipt_snapshot_accepted_execution_writer',
                    'public.ple_guard_accepted_execution_evidence_writer()'::regprocedure,
                    7::smallint
                )
        ) AS expected(relation_id, trigger_name, procedure_id, trigger_type)
        WHERE NOT EXISTS (
            SELECT 1
            FROM pg_catalog.pg_trigger AS trigger_row
            WHERE trigger_row.tgrelid = expected.relation_id
              AND trigger_row.tgname = expected.trigger_name
              AND trigger_row.tgfoid = expected.procedure_id
              AND trigger_row.tgtype = expected.trigger_type
              AND trigger_row.tgenabled = 'O'
              AND NOT trigger_row.tgisinternal
        )
    ) THEN
        RAISE EXCEPTION 'accepted-submission integrity trigger is unsafe';
    END IF;
END;
$$;

COMMIT;
