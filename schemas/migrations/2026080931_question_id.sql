-- One non-sequential, human-facing Crockford Base32 Question ID per problem.
-- Hidden UUID snapshots and the pre-production numeric storage key remain
-- internal while all browser projections move to question_id.

ALTER TABLE public.problem
    ADD COLUMN question_id character(7) NOT NULL,
    ADD CONSTRAINT problem_question_id_format_check
        CHECK (question_id ~ '^[0-9A-HJKMNP-TV-Z]{7}$'),
    ADD CONSTRAINT problem_question_id_key UNIQUE (question_id);

ALTER TABLE public.catalog_search_document
    ADD COLUMN question_id character(7) NOT NULL,
    ADD CONSTRAINT catalog_search_document_question_id_format_check
        CHECK (question_id ~ '^[0-9A-HJKMNP-TV-Z]{7}$');

CREATE INDEX catalog_search_document_question_id_idx
    ON public.catalog_search_document (question_id, version_number DESC);

-- Owner corrections update the mutable assignment definitions used by future
-- runs. Existing assignment_run_item rows remain immutable snapshots, so
-- grading reproducibility is unchanged. The nested trigger exception below is
-- deliberately limited to a same-problem successor linked by previous_version_id.
CREATE OR REPLACE FUNCTION public.ple_guard_assignment_content_lock() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE row_tenant uuid := COALESCE(NEW.tenant_id, OLD.tenant_id);
DECLARE row_assignment uuid := COALESCE(NEW.assignment_id, OLD.assignment_id);
DECLARE content_changed boolean;
DECLARE is_owner_correction boolean := false;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        content_changed := (NEW.problem_id, NEW.version_id)
            IS DISTINCT FROM (OLD.problem_id, OLD.version_id);
        is_owner_correction := content_changed
            AND pg_trigger_depth() > 1
            AND NEW.problem_id = OLD.problem_id
            AND EXISTS (
                SELECT 1
                  FROM public.problem_version AS correction
                 WHERE correction.problem_id = OLD.problem_id
                   AND correction.version_id = NEW.version_id
                   AND correction.previous_version_id = OLD.version_id
                   AND correction.lifecycle = 'published'
            );
    ELSE
        content_changed := true;
    END IF;
    IF content_changed AND NOT is_owner_correction AND EXISTS (
        SELECT 1 FROM public.assignment_run run
         JOIN public.enrollment enrollment
           ON enrollment.tenant_id = run.tenant_id
          AND enrollment.enrollment_id = run.enrollment_id
         WHERE enrollment.tenant_id = row_tenant
           AND enrollment.assignment_id = row_assignment
    ) THEN
        RAISE EXCEPTION 'assignment content is locked after the first student run'
            USING ERRCODE = '55000';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION public.ple_propagate_owner_question_correction() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NEW.previous_version_id IS NULL THEN
        RETURN NEW;
    END IF;

    UPDATE public.problem_version
       SET lifecycle = 'archived',
           lifecycle_reason = 'Superseded by an owner correction'
     WHERE problem_id = NEW.problem_id
       AND version_id = NEW.previous_version_id;

    WITH changed_items AS (
        UPDATE public.assignment_item
           SET version_id = NEW.version_id,
               revision = revision + 1,
               updated_at = transaction_timestamp()
         WHERE problem_id = NEW.problem_id
           AND version_id = NEW.previous_version_id
        RETURNING tenant_id, assignment_id
    ), changed_candidates AS (
        UPDATE public.assignment_selection_candidate
           SET version_id = NEW.version_id,
               updated_at = transaction_timestamp()
         WHERE problem_id = NEW.problem_id
           AND version_id = NEW.previous_version_id
        RETURNING tenant_id, assignment_id
    ), changed_assignments AS (
        SELECT tenant_id, assignment_id FROM changed_items
        UNION
        SELECT tenant_id, assignment_id FROM changed_candidates
    )
    UPDATE public.assignment AS assignment
       SET revision = assignment.revision + 1,
           updated_at = transaction_timestamp()
      FROM changed_assignments AS changed
     WHERE assignment.tenant_id = changed.tenant_id
       AND assignment.assignment_id = changed.assignment_id;

    RETURN NEW;
END
$$;

ALTER FUNCTION public.ple_propagate_owner_question_correction()
    OWNER TO ple_catalog_ownership_broker;
REVOKE ALL ON FUNCTION public.ple_propagate_owner_question_correction() FROM PUBLIC;

GRANT SELECT, UPDATE(lifecycle, lifecycle_reason)
    ON TABLE public.problem_version TO ple_catalog_ownership_broker;
GRANT SELECT, UPDATE(revision, updated_at)
    ON TABLE public.assignment TO ple_catalog_ownership_broker;
GRANT SELECT, UPDATE(version_id, revision, updated_at)
    ON TABLE public.assignment_item TO ple_catalog_ownership_broker;
GRANT SELECT, UPDATE(version_id, updated_at)
    ON TABLE public.assignment_selection_candidate TO ple_catalog_ownership_broker;

CREATE TRIGGER problem_version_owner_correction_propagation
    AFTER INSERT ON public.problem_version
    FOR EACH ROW EXECUTE FUNCTION public.ple_propagate_owner_question_correction();

CREATE TABLE public.question_id_namespace (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    issued_count bigint NOT NULL DEFAULT 0
        CHECK (issued_count BETWEEN 0 AND 100000000)
);

INSERT INTO public.question_id_namespace (singleton, issued_count)
VALUES (true, 0);

ALTER TABLE ONLY public.question_id_namespace FORCE ROW LEVEL SECURITY;
ALTER TABLE public.question_id_namespace ENABLE ROW LEVEL SECURITY;

CREATE POLICY question_id_namespace_app_access
    ON public.question_id_namespace
    FOR ALL TO ple_app
    USING (singleton)
    WITH CHECK (singleton);

GRANT SELECT, UPDATE ON TABLE public.question_id_namespace TO ple_app;

CREATE OR REPLACE FUNCTION public.ple_project_catalog_search_document() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    stable_public_id bigint;
    stable_question_id character(7);
BEGIN
    SELECT public_id, question_id INTO stable_public_id, stable_question_id
      FROM public.problem
     WHERE problem_id = NEW.problem_id;
    INSERT INTO public.catalog_search_document (
        problem_id, version_id, public_id, question_id, version_number, title, backend,
        metadata, publication_scope, lifecycle, lifecycle_reason, authors,
        previous_version_id, derived_from_problem_id, derived_from_version_id,
        published_at, authors_text, question_type, language, license, taxonomy,
        keywords, capabilities, search_text
    ) VALUES (
        NEW.problem_id, NEW.version_id, stable_public_id, stable_question_id,
        NEW.version_number, NEW.title, NEW.backend, NEW.metadata,
        NEW.publication_scope, NEW.lifecycle, NEW.lifecycle_reason, NEW.authors,
        NEW.previous_version_id, NEW.derived_from_problem_id,
        NEW.derived_from_version_id, NEW.created_at, NEW.authors::text,
        NEW.backend, COALESCE(NEW.metadata->>'language', 'und'),
        COALESCE(NEW.metadata #>> '{license,kind}', 'unknown'),
        COALESCE(NEW.metadata->'taxonomy', '[]'::jsonb),
        COALESCE(NEW.metadata->'tags', '[]'::jsonb), NEW.capabilities,
        to_tsvector('simple', concat_ws(' ', NEW.title, NEW.authors::text, NEW.metadata::text))
    )
    ON CONFLICT (problem_id, version_id) DO UPDATE SET
        lifecycle = EXCLUDED.lifecycle,
        lifecycle_reason = EXCLUDED.lifecycle_reason,
        updated_at = transaction_timestamp();
    RETURN NEW;
END
$$;
