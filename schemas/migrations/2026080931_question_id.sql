-- WP-R2: Question ID is the sole durable human identity for one immutable
-- publication. The opaque problem/version pair remains an exact internal
-- reference for grading, audit, and optional source provenance.

ALTER TABLE public.problem
    ADD COLUMN question_id character(7) NOT NULL,
    ADD CONSTRAINT problem_question_id_format_check
        CHECK (question_id ~ '^[0-9A-HJKMNP-TV-Z]{7}$'),
    ADD CONSTRAINT problem_question_id_key UNIQUE (question_id);

ALTER TABLE public.catalog_search_document
    ADD COLUMN question_id character(7) NOT NULL,
    ADD CONSTRAINT catalog_search_document_question_id_format_check
        CHECK (question_id ~ '^[0-9A-HJKMNP-TV-Z]{7}$');

CREATE UNIQUE INDEX catalog_search_document_question_id_key
    ON public.catalog_search_document (question_id);

-- Allocation records only namespace capacity. A publisher allocates a fresh
-- Question ID together with its fresh opaque pair; this relation never maps a
-- Question ID to another publication.
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
DECLARE stable_question_id character(7);
BEGIN
    SELECT question_id INTO stable_question_id
      FROM public.problem
     WHERE problem_id = NEW.problem_id;

    INSERT INTO public.catalog_search_document (
        problem_id, version_id, question_id, title, backend, metadata,
        publication_scope, lifecycle, lifecycle_reason, authors,
        derived_from_problem_id, derived_from_version_id, published_at,
        authors_text, question_type, language, license, taxonomy, keywords,
        capabilities, search_text
    ) VALUES (
        NEW.problem_id, NEW.version_id, stable_question_id, NEW.title,
        NEW.backend, NEW.metadata, NEW.publication_scope, NEW.lifecycle,
        NEW.lifecycle_reason, NEW.authors, NEW.derived_from_problem_id,
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
