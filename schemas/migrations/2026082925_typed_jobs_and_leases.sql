-- SD1 typed worker targets, immutable scope, generation fences, and opaque leases.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance, ple_data.assignment,
    ple_data.question_revision TO ple_private_owner;
RESET ROLE;

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.worker_job (
    job_id uuid PRIMARY KEY,
    handler_kind text NOT NULL CHECK (handler_kind IN (
        'grade_accepted_submission', 'recalculate_assignment', 'recalculate_course_item_analysis',
        'auto_submit_attempt', 'retention', 'render', 'export', 'import', 'qti_import',
        'publish_public_assets'
    )),
    target_kind text NOT NULL CHECK (target_kind IN (
        'course_assignment', 'course_attempt', 'course_retention', 'question_revision',
        'export', 'workspace_import', 'qti_import', 'public_asset_publication'
    )),
    course_id uuid REFERENCES ple_data.course_instance (course_id),
    assignment_id uuid,
    attempt_id uuid REFERENCES ple_private.question_attempt (question_attempt_id),
    workspace_id uuid REFERENCES ple_private.authoring_workspace (workspace_id),
    import_id uuid,
    question_id text,
    revision_number integer,
    export_id uuid,
    source_object_id uuid,
    expected_object_id uuid,
    generation bigint NOT NULL CHECK (generation > 0),
    target_digest bytea NOT NULL CHECK (pg_catalog.octet_length(target_digest) = 32),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    state text NOT NULL CHECK (state IN ('ready', 'leased', 'completed', 'dead')),
    available_at timestamp with time zone NOT NULL,
    lease_token uuid,
    lease_expires_at timestamp with time zone,
    attempt_count integer NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    max_attempts integer NOT NULL CHECK (max_attempts BETWEEN 1 AND 20),
    completed_at timestamp with time zone,
    failure_kind text CHECK (failure_kind IN ('transient', 'permanent', 'timed_out')),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT worker_job_assignment_parent_matches FOREIGN KEY (course_id, assignment_id)
        REFERENCES ple_data.assignment (course_id, assignment_id),
    CONSTRAINT worker_job_question_revision_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT worker_job_target_shape_is_exact CHECK (
        (target_kind = 'course_assignment' AND course_id IS NOT NULL AND assignment_id IS NOT NULL
            AND attempt_id IS NULL AND workspace_id IS NULL AND import_id IS NULL
            AND question_id IS NULL AND revision_number IS NULL AND export_id IS NULL
            AND source_object_id IS NULL AND expected_object_id IS NULL)
        OR (target_kind = 'course_attempt' AND course_id IS NOT NULL AND attempt_id IS NOT NULL
            AND assignment_id IS NULL AND workspace_id IS NULL AND import_id IS NULL
            AND question_id IS NULL AND revision_number IS NULL AND export_id IS NULL
            AND source_object_id IS NULL AND expected_object_id IS NULL)
        OR (target_kind = 'course_retention' AND course_id IS NOT NULL AND assignment_id IS NULL
            AND attempt_id IS NULL AND workspace_id IS NULL AND import_id IS NULL
            AND question_id IS NULL AND revision_number IS NULL AND export_id IS NULL
            AND source_object_id IS NULL AND expected_object_id IS NULL)
        OR (target_kind IN ('question_revision', 'public_asset_publication') AND question_id IS NOT NULL
            AND revision_number IS NOT NULL AND course_id IS NULL AND assignment_id IS NULL
            AND attempt_id IS NULL AND workspace_id IS NULL AND import_id IS NULL
            AND export_id IS NULL AND source_object_id IS NULL AND expected_object_id IS NULL)
        OR (target_kind = 'export' AND course_id IS NOT NULL AND assignment_id IS NOT NULL
            AND attempt_id IS NULL AND workspace_id IS NULL AND import_id IS NULL
            AND question_id IS NULL AND revision_number IS NULL AND export_id IS NOT NULL
            AND source_object_id IS NULL AND expected_object_id IS NOT NULL)
        OR (target_kind = 'workspace_import' AND workspace_id IS NOT NULL AND source_object_id IS NOT NULL
            AND course_id IS NULL AND assignment_id IS NULL AND attempt_id IS NULL AND import_id IS NULL
            AND question_id IS NULL AND revision_number IS NULL AND export_id IS NULL AND expected_object_id IS NULL)
        OR (target_kind = 'qti_import' AND workspace_id IS NOT NULL AND import_id IS NOT NULL
            AND source_object_id IS NOT NULL AND course_id IS NULL AND assignment_id IS NULL
            AND attempt_id IS NULL AND question_id IS NULL AND revision_number IS NULL
            AND export_id IS NULL AND expected_object_id IS NULL)
    ),
    CONSTRAINT worker_job_lease_state_matches CHECK (
        (state = 'leased' AND lease_token IS NOT NULL AND lease_expires_at IS NOT NULL AND completed_at IS NULL)
        OR (state = 'ready' AND lease_token IS NULL AND lease_expires_at IS NULL AND completed_at IS NULL)
        OR (state IN ('completed', 'dead') AND lease_token IS NULL AND lease_expires_at IS NULL AND completed_at IS NOT NULL)
    )
);
CREATE FUNCTION ple_private.reject_worker_job_target_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.handler_kind IS DISTINCT FROM OLD.handler_kind
        OR NEW.target_kind IS DISTINCT FROM OLD.target_kind
        OR NEW.course_id IS DISTINCT FROM OLD.course_id
        OR NEW.assignment_id IS DISTINCT FROM OLD.assignment_id
        OR NEW.attempt_id IS DISTINCT FROM OLD.attempt_id
        OR NEW.workspace_id IS DISTINCT FROM OLD.workspace_id
        OR NEW.import_id IS DISTINCT FROM OLD.import_id
        OR NEW.question_id IS DISTINCT FROM OLD.question_id
        OR NEW.revision_number IS DISTINCT FROM OLD.revision_number
        OR NEW.export_id IS DISTINCT FROM OLD.export_id
        OR NEW.source_object_id IS DISTINCT FROM OLD.source_object_id
        OR NEW.expected_object_id IS DISTINCT FROM OLD.expected_object_id
        OR NEW.generation IS DISTINCT FROM OLD.generation
        OR NEW.target_digest IS DISTINCT FROM OLD.target_digest
        OR NEW.payload IS DISTINCT FROM OLD.payload THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'a worker job target is immutable';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER worker_job_target_is_immutable
BEFORE UPDATE ON ple_private.worker_job
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_worker_job_target_change();
CREATE INDEX worker_job_ready_claim_idx
    ON ple_private.worker_job (available_at, job_id) WHERE state = 'ready';
CREATE INDEX worker_job_expired_lease_idx
    ON ple_private.worker_job (lease_expires_at, job_id) WHERE state = 'leased';
ALTER TABLE ple_private.worker_job ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.worker_job FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.worker_job FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_private.reject_worker_job_target_change() FROM PUBLIC;
COMMENT ON TABLE ple_private.worker_job IS 'Server-enqueued work with immutable typed parent, generation fence, and opaque current lease.';
RESET ROLE;
