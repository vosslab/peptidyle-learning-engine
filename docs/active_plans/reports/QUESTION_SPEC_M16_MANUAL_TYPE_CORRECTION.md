# M16 Manual WebWork Type Correction Publication

## Status

The approved projection change and focused PostgreSQL regression are implemented and the regression
target compiles. Connected PostgreSQL execution remains pending because this shell has no configured
disposable acceptance runtime.

Fresh SPEC review accepted the approved propagation and confirmed the NULL-Type guard, available
target authorization, exact parent Revision guard, Native source-derived binding, and parent-derived
metadata and lineage fields remain intact.

Fresh QUALITY review accepted the CASE hunk and regression. It confirmed that the Draft save path
persists the Type, the WebWork successor reads that saved value, and the regression asserts both
unchanged and deliberately changed Type outcomes plus same-ID owner and parent Revision state.

## Approved propagation

In `schemas/base_schema/50_functions/question_publication_operations.sql`, the successor
`question_revision_metadata.question_type` expression now takes `binding.question_type` for both
Native and WebWork bindings. WebWork therefore publishes the saved ordinary Draft Type. Other
backend values retain the existing parent-metadata fallback.

The existing non-NULL Draft binding Type guard remains at line 165. The Native source-binding
projection remains at line 331 and sets `native_question_type` only from the saved compiled
`binding.question_type`. The edit does not change the owner predicate, stable Question ID, exact
parent Revision guard, acceptance tuple, authorship, license, citation, classification, Bloom, or
other publication logic.

The shared SQL file also contains separately authorized M16 publication and source-read edits from
the concurrent correction work described in
[QUESTION_SPEC_M16_SYSADMIN_CORRECTION.md](QUESTION_SPEC_M16_SYSADMIN_CORRECTION.md). This slice
changes only the successor Type projection; it does not change that work's content-difference
predicate or other function logic.

## Regression

`crates/learning-data-access/tests/blueprint_course_postgres/question_revision_metadata/manual_type_correction.rs`
adds an ignored disposable-PostgreSQL regression under the existing metadata test module. It creates
a WebWork Question, saves one correction Draft with the exact parent's current Type, and publishes
Revision 2. It then creates a correction Draft initialized from Revision 2's Type, deliberately
saves a different Type, and publishes Revision 3. The test checks the saved Draft binding before
publication, each successor's `question_revision_metadata.question_type`, both exact parent
Revision numbers, the stable Question ID, and unchanged Question owner.

## Verification

- `rustfmt --edition 2024 --check crates/learning-data-access/tests/blueprint_course_postgres/question_revision_metadata.rs crates/learning-data-access/tests/blueprint_course_postgres/question_revision_metadata/manual_type_correction.rs` - passed.
- `source source_me.sh && cargo test -p learning-data-access --features postgres --test blueprint_course_postgres --no-run` - passed; compile-only, with the existing unused `question_pool_member_pins` warning.
- `source source_me.sh && python3 schema_style/check_schema_style.py --verbose` - passed; reported `clean`.
- `git diff --check` - passed.
- Source line counts: SQL 738, parent integration-test module 856, new regression 436; all remain below 1,000.

The disposable PostgreSQL runtime was not configured, so the ignored connected regression was not
executed. The SQL review confirms the Native `native_question_type` projection remains unchanged;
this focused regression covers manual WebWork Type publication only. Fresh SPEC and QUALITY reviews
both accepted the scoped change.

The regression is permanent coverage in the existing ignored connected PostgreSQL target. The
compile-only build, formatter, schema-style check, diff check, and line-count check are one-time
implementation gates.
