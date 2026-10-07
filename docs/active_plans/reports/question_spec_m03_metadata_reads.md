# M03 metadata reads

## Scope completed

The listed SQL consumers now read `ple_data.question_revision_metadata` using the matching
`published_question_id` and `revision_number`. `question_library_entries` and
`load_question_library_revision` retain exact Revision reads; library discovery, the summary view,
current shared metadata, archive confirmation, starred titles, and recognition titles explicitly
select the greatest accepted Revision Number.

Question Pool creation and member admission unnest the Question IDs and Revision Numbers together.
Metadata locks, lookup, and Discipline/Subject comparisons now use each exact pair. Question forks
load the supplied source Revision metadata. Assessment projections and Student response statistics
join metadata to the stored exact tuple.

Rust PostgreSQL fixtures and `tests/e2e/assessment_saved_response/03_course_pool_forks.sql` now insert
metadata with Revision Numbers. The E2E Question with two Revisions receives metadata for both.
Existing M02 tuple renames in shared Rust fixture modules were preserved.

## Validation

Static review confirms no `ple_data.published_question_metadata` relation references remain under
`schemas/base_schema`, `crates`, or `tests` in SQL/Rust sources. Existing bulk metadata operation
names are unchanged and are not table references. `git diff --check` passes.

The focused fresh-PostgreSQL gate remains pending while the M03 schema, write, and runtime lanes
integrate. Run the Question Library and Blueprint PostgreSQL tests plus the saved-response E2E after
those lanes are ready. This report does not claim database or runtime acceptance.
