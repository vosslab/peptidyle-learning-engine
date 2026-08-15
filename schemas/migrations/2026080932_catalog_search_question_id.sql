-- WP-R2 ledger entry: expose each visible immutable publication by Question ID
-- and its protected internal pair. No latest-version ordering exists.
-- PostgreSQL preserves existing view-column identities under CREATE OR REPLACE.
-- This pre-production baseline intentionally changes the projection shape, so
-- recreate the view at its dated schema boundary before granting its readers.
DROP VIEW public.catalog_search_view;

CREATE VIEW public.catalog_search_view
WITH (security_invoker = true)
AS
SELECT document.problem_id,
       document.version_id,
       document.question_id,
       document.title,
       document.backend,
       document.metadata,
       document.publication_scope,
       document.lifecycle,
       document.lifecycle_reason,
       document.authors,
       document.derived_from_problem_id,
       document.derived_from_version_id,
       document.published_at,
       document.authors_text,
       document.question_type,
       document.language,
       document.license,
       document.taxonomy,
       document.keywords,
       document.capabilities,
       document.search_text,
       document.quality_signal,
       document.updated_at,
       statistics.cohort_size,
       statistics.difficulty_index,
       statistics.attempts_mean,
       statistics.time_median_seconds_estimate,
       statistics.discrimination_index,
       (statistics.cohort_size IS NOT NULL) AS statistics_available
  FROM public.catalog_search_document AS document
  LEFT JOIN LATERAL public.ple_question_statistics_view(
      document.problem_id,
      document.version_id
  ) AS statistics ON true;

GRANT SELECT ON TABLE public.catalog_search_view TO ple_app, ple_student;
