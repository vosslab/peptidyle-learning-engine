# Sysadmin Question Archive and Restore alignment

## Behavior

Sysadmins can archive or restore any Published Question. Instructors can make the same transition
only for a Question they own; Students and other Instructors remain denied. Both roles use the
existing availability Edit Number, exact-title archive confirmation, state transition, and audit
event. The audit records the authenticated actor. The Library affordance is available to the owner
or any Sysadmin for both Available and Archived rows, so it can render Archive or Restore according
to the current state.

The HTTP transition resolves an Instructor or Sysadmin Library session. SQL remains authoritative:
it locks the lineage and permits the owner or Sysadmin, then applies the existing CAS, title, and
state checks before writing the availability event. Archived Questions remain read-only for
metadata editing and preserve exact Revision references, restore, and fork behavior.

## Evidence

- The existing connected `question_revision_metadata` oracle now checks Sysadmin archive/restore,
  other-Instructor and Student denial, exact-title confirmation, stale availability Edit Number,
  the Sysadmin audit actor, and the archive/restore affordance projection. Its archive sequence
  remains last because it changes the shared fixture.
- `instructor_session_hash` was removed after the transition moved to the shared
  Instructor-or-Sysadmin session resolver; the remaining resolver still controls Library reads.
- `source ./source_me.sh && rustfmt --edition 2024 --check crates/server/src/question_library.rs
  crates/learning-data-access/tests/blueprint_course_postgres/question_revision_metadata.rs` passed.
- `source ./source_me.sh && python3 schema_style/check_schema_style.py` passed (`clean`).
- `source ./source_me.sh && cargo clippy -p server_core --lib -- -D warnings` passed.
- `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test
  blueprint_course_postgres --no-run` passed and compiled the connected oracle.
- Scoped `git diff --check` passed.
- The connected PostgreSQL case has not run in this correction. The runtime owner must rebuild or
  reload the changed SQL functions and execute the registered selector before M13 acceptance.

## Projection correction

The earlier report described the archive/restore affordance projection as covered, but source review
found that `question_library_entries` projected owner-only permission into `viewer_may_archive` and
Sysadmin/owner permission into `viewer_may_edit_metadata` without checking availability. The SQL now
projects archive/restore permission for a Sysadmin or owning Instructor on either availability
state, and metadata permission only while the Question is available. The connected PostgreSQL case
has not run, so this source correction is not runtime acceptance evidence.
