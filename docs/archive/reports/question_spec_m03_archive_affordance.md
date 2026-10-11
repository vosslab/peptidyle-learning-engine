# M03 archived Question metadata affordance

## Correction

`viewer_may_edit_metadata` now requires the Question lineage to be available for every role. An
available Question remains editable by its current Instructor owner or a Sysadmin. Archived
Questions are read-only to both roles, matching the metadata save procedure and Human Guidance.

The PostgreSQL behavior test checks the Sysadmin affordance while available and after archival. It
also retains the existing assertion that an archived metadata save is rejected. The archive case
remains last in the test sequence because it changes the shared fixture state.

## Evidence

- Scoped rustfmt passed for `question_revision_metadata.rs`.
- `cargo test -p learning-data-access --features postgres --test blueprint_course_postgres --no-run`
  passed, compiling the new assertions.
- Scoped `git diff --check` passed.
- Fresh database execution remains pending. The shared runtime gate must reload the updated SQL
  function before running the final M03 case; its current blocker is an unrelated M26 fixture role
  failure.
