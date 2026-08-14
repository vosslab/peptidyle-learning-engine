-- Question ID was added to the catalog projection after catalog_search_view
-- was established. Keep the established search-view contract in its original
-- order, and append the new human-facing identity at the end.
CREATE OR REPLACE VIEW public.catalog_search_view
WITH (security_invoker = true)
AS
SELECT document.problem_id,
       document.version_id,
       document.public_id,
       document.version_number,
       document.title,
       document.backend,
       document.metadata,
       document.publication_scope,
       document.lifecycle,
       document.lifecycle_reason,
       document.authors,
       document.previous_version_id,
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
       (statistics.cohort_size IS NOT NULL) AS statistics_available,
       document.question_id
  FROM public.catalog_search_document AS document
  LEFT JOIN LATERAL public.ple_question_statistics_view(
      document.problem_id,
      document.version_id
  ) AS statistics ON true;

GRANT SELECT ON TABLE public.catalog_search_view TO ple_app, ple_student;
