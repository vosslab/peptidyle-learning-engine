-- WP-R0: PostgreSQL-owned ranked catalog discovery.
-- `catalog_sequence` is allocated once at first projection.  It deliberately
-- does not change on lifecycle transitions: continuations exclude later
-- publications, while current lifecycle/RLS checks remain immediate.
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE SEQUENCE public.catalog_search_publication_sequence AS bigint;

ALTER TABLE public.catalog_search_document
    ADD COLUMN catalog_sequence bigint;

UPDATE public.catalog_search_document
   SET catalog_sequence = nextval('public.catalog_search_publication_sequence')
 WHERE catalog_sequence IS NULL;

ALTER TABLE public.catalog_search_document
    ALTER COLUMN catalog_sequence SET NOT NULL,
    ALTER COLUMN catalog_sequence SET DEFAULT nextval('public.catalog_search_publication_sequence');

ALTER SEQUENCE public.catalog_search_publication_sequence
    OWNED BY public.catalog_search_document.catalog_sequence;

CREATE INDEX catalog_search_document_catalog_sequence_idx
    ON public.catalog_search_document (catalog_sequence);

-- Availability is a disclosure event, not a value reconstructed from a
-- later aggregate.  The same event sequence as publication lets a cursor
-- honestly retain the first page's answer to "were statistics disclosed?".
CREATE TABLE public.catalog_statistics_disclosure (
    problem_id uuid NOT NULL,
    version_id uuid NOT NULL,
    disclosed_sequence bigint NOT NULL,
    PRIMARY KEY (problem_id, version_id),
    FOREIGN KEY (problem_id, version_id)
        REFERENCES public.problem_version(problem_id, version_id) ON DELETE CASCADE
);

-- This relation is an implementation detail of the catalog projection, not a
-- second enumeration surface.  Direct readers can observe only the same
-- published versions already visible through catalog_search_document RLS.
ALTER TABLE public.catalog_statistics_disclosure ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.catalog_statistics_disclosure FORCE ROW LEVEL SECURITY;

CREATE POLICY catalog_statistics_disclosure_visible_select
    ON public.catalog_statistics_disclosure FOR SELECT TO ple_app, ple_student
    USING (EXISTS (
        SELECT 1
          FROM public.catalog_search_document AS visible_document
         WHERE visible_document.problem_id = catalog_statistics_disclosure.problem_id
           AND visible_document.version_id = catalog_statistics_disclosure.version_id
    ));

CREATE POLICY catalog_statistics_disclosure_statistics_broker_insert
    ON public.catalog_statistics_disclosure FOR INSERT TO ple_statistics_broker
    WITH CHECK (true);

CREATE FUNCTION public.ple_record_catalog_statistics_disclosure() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
AS $$
BEGIN
    IF NEW.cohort_size >= 5 AND (TG_OP = 'INSERT' OR OLD.cohort_size < 5) THEN
        INSERT INTO public.catalog_statistics_disclosure (
            problem_id, version_id, disclosed_sequence
        ) VALUES (
            NEW.problem_id, NEW.version_id,
            nextval('public.catalog_search_publication_sequence')
        );
    END IF;
    RETURN NEW;
END
$$;

ALTER FUNCTION public.ple_record_catalog_statistics_disclosure()
    OWNER TO ple_statistics_broker;
REVOKE ALL ON FUNCTION public.ple_record_catalog_statistics_disclosure() FROM PUBLIC;

CREATE TRIGGER question_statistics_disclosure_projection
AFTER INSERT OR UPDATE OF cohort_size ON public.question_statistics_aggregate
FOR EACH ROW EXECUTE FUNCTION public.ple_record_catalog_statistics_disclosure();

-- Existing aggregates may already have crossed the disclosure threshold when
-- this forward migration is installed.  They become visible at this migration
-- event; later cursor pages cannot retroactively include them.
INSERT INTO public.catalog_statistics_disclosure (
    problem_id, version_id, disclosed_sequence
)
SELECT aggregate.problem_id, aggregate.version_id,
       nextval('public.catalog_search_publication_sequence')
  FROM public.question_statistics_aggregate AS aggregate
 WHERE aggregate.cohort_size >= 5
ON CONFLICT (problem_id, version_id) DO NOTHING;

-- PostgreSQL rejects non-IMMUTABLE index expressions.  Keep the indexed
-- projection explicit so the `<%` word-similarity predicate and its index
-- cannot drift apart.
CREATE FUNCTION public.ple_catalog_normalized_search_text(
    p_title text, p_authors text, p_metadata jsonb
) RETURNS text
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    RETURN lower(coalesce(p_title, '') || ' ' || coalesce(p_authors, '') || ' ' || coalesce(p_metadata::text, ''));

ALTER TABLE public.catalog_search_document
    ADD COLUMN normalized_search_text text GENERATED ALWAYS AS (
        public.ple_catalog_normalized_search_text(title, authors_text, metadata)
    ) STORED;

-- gin_trgm_ops supports the `<%` word-similarity operator used by the Store
-- query.  The Store pins pg_trgm.word_similarity_threshold per transaction.
CREATE INDEX catalog_search_document_trigram_text_idx
    ON public.catalog_search_document USING gin
    (normalized_search_text gin_trgm_ops);

CREATE OR REPLACE VIEW public.catalog_search_view
WITH (security_invoker = true)
AS
SELECT document.problem_id, document.version_id, document.question_id,
       document.title, document.backend,
       document.metadata, document.publication_scope, document.lifecycle,
       document.lifecycle_reason, document.authors, document.derived_from_problem_id,
       document.derived_from_version_id,
       document.published_at, document.authors_text, document.question_type,
       document.language, document.license, document.taxonomy, document.keywords,
       document.capabilities, document.search_text, document.quality_signal,
       document.updated_at, statistics.cohort_size, statistics.difficulty_index,
       statistics.attempts_mean, statistics.time_median_seconds_estimate,
       statistics.discrimination_index,
       (statistics.cohort_size IS NOT NULL) AS statistics_available,
       document.catalog_sequence,
       disclosure.disclosed_sequence AS statistics_disclosed_sequence,
       document.normalized_search_text
  FROM public.catalog_search_document AS document
  LEFT JOIN LATERAL public.ple_question_statistics_view(
      document.problem_id, document.version_id
  ) AS statistics ON true
  LEFT JOIN public.catalog_statistics_disclosure AS disclosure
    ON disclosure.problem_id = document.problem_id
   AND disclosure.version_id = document.version_id;

GRANT SELECT ON TABLE public.catalog_search_view TO ple_app, ple_student;
GRANT SELECT ON TABLE public.catalog_statistics_disclosure TO ple_app, ple_student;
GRANT USAGE, SELECT ON SEQUENCE public.catalog_search_publication_sequence TO ple_app;
GRANT INSERT ON TABLE public.catalog_statistics_disclosure TO ple_statistics_broker;
GRANT USAGE ON SEQUENCE public.catalog_search_publication_sequence TO ple_statistics_broker;
