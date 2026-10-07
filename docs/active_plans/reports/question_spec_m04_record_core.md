# M04 Record Metadata Core

M04 makes `QuestionMetadata` the authority for Draft and Published Question
record metadata. Native Question JSON remains content-only. `language` is
nullable throughout the record model and exact Published Revision reads.

Draft creation accepts one `{ metadata, source }` JSON envelope and saves both
inside the existing ordinary Draft aggregate. Metadata GET and PUT use the
existing `/api/authoring/drafts/{id}/metadata` route, Edit Number, and ETag.
PUT replaces title, description, tags, license, citation, and language while
retaining the existing general-feedback and paired hint/worked-solution
semantics. Draft metadata can be incomplete; publication requires a nonempty
title and description, a license, and a source-derived Question Type.

Native source saves now update only source binding and type. Record metadata
does not move with source bytes. Publication copies current Draft record
metadata, including citation and nullable language, to the immutable Revision;
fork creation carries the selected exact Revision metadata forward. Successor
publication continues to carry parent metadata forward. Question Library
exact-revision reads return language and citation from the selected Revision,
and both Native and WeBWorK summaries use those record values without source
metadata overlays or fabricated language defaults.

The PostgreSQL authoring test now verifies source replacement leaves record
metadata unchanged, a metadata replacement preserves previously saved hint and
worked-solution text when those fields are omitted, and the request model
accepts nullable language and incomplete Draft metadata.

Validation:

- `source ./source_me.sh && cargo check -p question_model -p learning-data-access -p server_core -p project-tools` - passed.
- `source ./source_me.sh && cargo test -p server_core authoring::tests::published_questions_include_optional_hint_feedback_and_worked_solution --lib` - passed (1 test).
- `source ./source_me.sh && cargo test -p learning-data-access --test authoring_draft_source_postgres --no-run` - passed; compiles the focused PostgreSQL behavior test.
- `source ./source_me.sh && cargo test -p learning-data-access --test blueprint_course_postgres --no-run` - passed; compiles the revision publication metadata test.
- `source ./source_me.sh && rustfmt --edition 2024 --check` over the touched Rust files - passed.
- `git diff --check` over the touched M04 Rust, SQL, and fixture files - passed.

The connected PostgreSQL tests were not run here. They are marked as requiring
the disposable PostgreSQL acceptance runtime and remain part of the root
fresh-database gate. Coordinated M04/M17 contract generation is complete:
`generated/api/QuestionMetadata.ts` now declares `language: string | null`.
The generator report records that `npx tsc --noEmit -p tsconfig.json` passed;
the lint-scope check found an unrelated fixture error in
`tests/support/ribbon_shell_harness.tsx:610`, which remains with its owner.
This report records core implementation evidence and does not claim milestone
acceptance.
