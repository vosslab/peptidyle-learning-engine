# M16 Sysadmin Correction-Draft Publication

## Status

The source change and focused connected regression are ready for independent review. Connected
PostgreSQL execution and the browser/HTTP journey remain with root.

## Authorization decision

Sysadmins may correct any eligible Published Question through an ordinary correction Draft and the
existing same-ID `/publish-revision` operation. The Published Question keeps its current Instructor
owner. The correction Draft stays unparented and belongs to the Sysadmin's ordinary Authoring
Workspace.

## Implementation

In `schemas/base_schema/50_functions/question_publication_operations.sql`,
this task changes only the role predicate in
`ple_private.load_draft_question_publication_source`, which now accepts Instructor or Sysadmin.
The current shared `ple_private.publish_question_revision` source already authorizes Instructor or
Sysadmin for an available target and keeps the Published Question owner unchanged; its function
hunk belongs to concurrent work and remains untouched here. The helper's Authoring Workspace access
requirement, Draft Edit Number compare-and-swap, parent tuple, and source binding remain unchanged.

The remaining Type boundary is separate from this helper task: `ple_private.publish_question_revision`
currently copies the parent WebWork Type unconditionally. After its exact helper owner releases
`question_publication_operations.sql`, a fresh narrow owner will change only that CASE and add a
successor regression covering copied versus changed Type.

The connected regression is
`crates/learning-data-access/tests/blueprint_course_postgres/sysadmin_correction_publication.rs`.
It creates an authenticated Sysadmin session, checks that the ordinary workspace belongs to that
Sysadmin, confirms current-source retrieval and stale Edit Number rejection, preserves the Instructor
source-read path, denies cross-workspace access, saves the correction Draft through the ordinary save
operation, and publishes a successor on the same Question ID while asserting the Instructor owner
remains unchanged. The existing `blueprint_course_postgres.rs` registers this focused module.

## Verification

- `rustfmt --edition 2024 --check crates/learning-data-access/tests/blueprint_course_postgres/sysadmin_correction_publication.rs` - passed.
- `source source_me.sh && python3 schema_style/check_schema_style.py --verbose` - passed; schema style reported clean.
- `source source_me.sh && cargo test -p learning-data-access --features postgres --test blueprint_course_postgres --no-run` - passed; the connected test target compiled without executing it. Cargo reported one existing dead-code warning for `question_pool_member_pins` in `blueprint_course_postgres/support.rs:99`.

The initial narrowed schema-style command using `--source-dir schemas/base_schema/50_functions` was
not a valid standalone check because the global table-count rule needs the complete schema source;
the full schema-style command above passed.

## Acceptance boundary

Root owns connected same-ID `/publish-revision` HTTP/database and browser acceptance. This test has
not been executed against PostgreSQL; the successful command above is compile-only evidence. The
separate Type correction remains outside the current helper task.
