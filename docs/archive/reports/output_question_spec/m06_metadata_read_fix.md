## M06 shared metadata read correction

The shared-metadata Rust loader now selects `question_type`, which its existing row decoder already
requires to construct `PublishedQuestionSharedMetadata`. SQL already returned the field; no schema,
API DTO, or M24 search-predicate change was needed. The existing PostgreSQL scenario now asserts the
returned Type and metadata Edit Number for both a Sysadmin and a different Instructor.

The M06 permission model remains the approved one: the Question Owner or Sysadmin can replace
metadata with the exact Revision and metadata Edit Number; another Instructor retains ordinary
Library read access but cannot edit. Native Type remains derived from its immutable source binding,
while WeBWorK Type is editable ordinary metadata. The generated browser files already include
`questionType`; generation is historical, and final freshness verification remains pending.

## Verification and limits

- The loader SELECT fix is in: source-flow inspection confirms the caller projects `question_type`
  and the row decoder reads it.
- `source ./source_me.sh && cargo test -p learning-data-access --test blueprint_course_postgres --no-run`:
  failed at the time it ran, before compiling `learning-data-access`, due to unrelated in-flight
  missing imports for `AssessmentEntryId` and `AssessmentEditNumber` in
  `crates/question_model/src/question_pool_library.rs`. Those identifiers no longer appear in that
  module now. Cargo has not been rerun against the current source, so this remains historical
  failure evidence rather than a verified current blocker.
- The connected PostgreSQL test now checks Type and Edit Number for Sysadmin and Instructor reads;
  it was not run because the task excludes database startup while the schema is in flight.
- Fresh SPEC review passed; its read-only evidence is in [m06_fix_spec.md](m06_fix_spec.md). The
  initial SPEC BLOCK is historical and superseded by that review.
- Fresh QUALITY review confirmed the source fix. Its initial QUALITY BLOCK was limited to stale
  report status; this status correction was then confirmed by the narrow follow-up review
  [m06_fix_status_quality_final.md](m06_fix_status_quality_final.md). The earlier
  [m06_fix_quality.md](m06_fix_quality.md) is historical and superseded on status by that follow-up.
- Connected PostgreSQL acceptance and generator freshness remain pending.

No Git/index operations were performed. Existing unrelated edits in
`crates/learning-data-access/src/postgres/question_library.rs`, including M24 search and M22/M23
statistics work, were preserved.
