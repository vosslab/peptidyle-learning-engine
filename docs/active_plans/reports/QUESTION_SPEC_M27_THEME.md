# M27 Blueprint and Course Theme

The approved [Question-spec implementation plan](../active/question_spec_implementation_plan.md)
defines a Course adopted from a Blueprint as starting with its Theme, then changing Theme
independently. Blueprint Theme is ordinary mutable metadata that uses the existing Blueprint Edit
Number.

## Implemented behavior

- New Blueprints persist the selected Theme; a fork copies the source Blueprint's current Theme.
- Instructors edit Blueprint Theme through the dedicated metadata route. The update follows existing
  Blueprint ownership/Sysadmin authorization, advances the Blueprint Edit Number only when the Theme
  changes, and records the resulting Theme in metadata history.
- Blueprint detail, discovery, history, comparison, and proposal views carry the persisted Theme.
- Course creation from a Blueprint reads the locked Blueprint's stored Theme. The create request
  carries no Course Theme override. Empty Courses start with `grass`; adopted Courses start with the
  source Theme and can later change Theme independently.
- `TERMINOLOGY_CONTRACT.md` now records the Blueprint Theme and one-time adoption projection.

## Focused evidence

- Added a browser API client check for the Theme metadata route, exact request body, and existing
  Blueprint Edit Number `If-Match` validator. The focused client suite passes all 16 tests.
- Added a PostgreSQL lifecycle oracle that adopts a Blueprint with Theme `forest`, then changes only
  the Course to `ocean` and verifies that the Blueprint remains `forest`.
- Added Theme foreign-key indexes for the Blueprint current row and metadata event. The schema style
  check no longer reports Theme foreign keys; its remaining finding is on the separately owned
  `issued_question` table.

## Verification and limits

- `schema_style/check_schema_style.py` - Theme FK findings resolved; one unrelated advisory remains
  for `ple_private.issued_question`.
- `source ./source_me.sh && node --import tsx --test tests/test_blueprint_course_client.mjs` - passed
  all 16 tests.
- Prettier check over M27 TypeScript/TSX, browser client test, report, terminology, and changelog -
  passed; `git diff --check` - passed.
- `rustfmt --check --edition 2024` over M27-owned Rust source files - passed.
- `source ./source_me.sh && cargo check -p question_model -p learning-data-access -p browser-api-contract -p server_core`
  - passed.
- `source ./source_me.sh && cargo test -p learning-data-access --test blueprint_course_postgres --no-run`
  - passed; the PostgreSQL lifecycle oracle compiles.
- `source ./source_me.sh && npx tsc --noEmit` - M27-owned files are clean; the full check still fails
  on concurrent Pool model/decoder and Bloom metadata errors, plus ES library `toSorted` errors in
  Pool editors. These are outside M27 ownership.
- `cargo check --workspace` previously stopped at concurrent QuestionType edits in
  `crates/learning-data-access/src/postgres/question_library.rs`; the M27 dependency crates now pass
  their scoped check. A fresh workspace-wide check remains pending coordinator integration.
- Canonical retry 2 exited 1 after the Blueprint Assessment Pool key correction succeeded. The
  existing PostgreSQL lifecycle oracle then exposed that `CreatedCourseInstance` returned Grass
  after persisting the adopted Forest Theme. The SQL creation function now returns the saved
  `course_theme`, and Rust decodes it through the existing Theme parser. Fresh source-only SPEC
  `course_theme_response_spec` and distinct QUALITY `course_theme_response_quality` passed with no
  further defect found. Canonical retry 3 is running in session 37923; its runtime result remains
  pending in `output_question_spec/acceptance_integrated_retry3_20261006.log`. TypeScript compilation
  after contract generation, live UI acceptance, M29 integration, and overall acceptance remain
  pending.
