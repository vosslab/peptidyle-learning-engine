# M11 Blueprint and Course Pool Operations

## Connected publication coverage update (2026-10-06)

The earlier gap recorded below is now closed at source and registration level by the focused
[Course-to-Blueprint acceptance test](output_question_spec/M11_COURSE_BLUEPRINT_ACCEPTANCE.md).
Its target compiles; runtime loading is blocked because the acceptance manifest and database URL
environment variables are unset in this shell.

## Result

Blueprint and Course operations now carry the ordinary `question_pool_id` directly. Saving a
Blueprint, forking or copying a Blueprint, adopting/copying a Course, accepting a Blueprint change,
and applying a Blueprint update do not allocate another Pool ID or copy Pool members. Assessment
`selection_count` and entry policy remain local. Blueprint content does not pin a Pool Edit Number;
the current Pool detail/read model may still show the live Edit Number when useful.

Blueprint persistence verifies that a referenced ordinary Pool exists. It does not reject a save
because the requested selection count exceeds current membership. Release-time adequacy remains the
M12 validation boundary. The Blueprint member editor and member-write path were removed; ordinary
Pool membership remains owned and edited through the ordinary Pool operations.

The explicit ordinary Pool fork remains the only intentional Pool-copy operation. The SQL
constructor creates a new Pool under the acting Instructor at Edit Number 1, points
`source_question_pool_id` at the source, copies the source's current metadata (including both Bloom
dimensions), and copies its exact current Question Revision Tuple set. The connected SQL fixture
also changes the child and parent memberships independently. M09's tuple schema/model/API work is
the prerequisite for this contract; see the [M09 Pool storage report](QUESTION_SPEC_M09_POOL_STORAGE.md)
and its stated runtime limits.

### M11 correction after SPEC review

The fresh SPEC review rejected two paths. `blueprint_lineage.sql` had required a distinct child Pool
linked to the source and with identical members, then rewritten the fork content's Pool ID and Edit
Number. Ordinary Blueprint Course forks now preserve the source Pool ID directly; fork equality
continues to enforce identical authored content and local Blueprint identities still change.
`course_blueprint_publication_assessment_content` had persisted a Pool Edit Number; its projection
now omits that field. The `course_blueprint_publication_snapshot.poolEvidence.questionPoolEditNumber`
value remains transient concurrency evidence and is not stored Blueprint content.

The registered `04_blueprint_pool_forks.sql` fixture directly saves a Blueprint that references the
ordinary Pool ID, forks that Blueprint, and checks that the child revision retains the same Pool ID
with no Pool Edit Number. It then tests the explicit ordinary Pool fork. That SQL fixture does not
call `course_blueprint_publication_assessment_content`; the separate registered Rust oracle now
covers Course-to-Blueprint publication as described above. The SQL fixture was not executed here.

## Implementation evidence

- **Blueprint save and validation:** `50_functions/blueprints.sql` stores `question_pool_id` and
  local assessment policy without a Pool Edit Number. `50_functions/blueprint_pools.sql` validates
  ordinary Pool existence; it no longer requires a child Pool, compares member count during save,
  or writes Blueprint-owned members. `blueprint_pool_members` is read-only and reports live Pool
  detail.
- **Course adoption and updates:** `50_functions/course_blueprint_adoption.sql` and
  `50_functions/assessment_blueprint_updates.sql` preserve the source Pool ID in
  `assessment_entry_pool`. They do not copy Pool membership or validate a historical Pool Edit
  Number. Course creation still creates Course-side Assessment identities. M27's one-time Blueprint
  Theme copy remains in the Course creation path.
- **Rust and API contracts:** Blueprint inputs/views and stored entries in `question_model` and
  `learning-data-access` now use direct Pool IDs without import/retained/member-choice alternatives
  or pinned Edit Numbers. Assessment update views omit Pool Edit Number. Root regenerated the API
  TypeScript output once after the DTO changes (`source ./source_me.sh && cargo tsgen`, exit 0,
  409 types written); no Rust DTO changes followed, so a second generation is not needed.
- **Copy paths:** removed `materialize_imported_pools` and its consumers from Blueprint lineage fork,
  Course/Blueprint publication, Blueprint fork apply, and accepted change proposals. Course
  adoption materialization now creates Assessment identities only and preserves each Pool ID.
  Blueprint Revision compare/update projection also no longer compares Pool Edit Number or member
  tuples as historical content.
- **Browser:** Blueprint entry input, history, fork review, comparison, proposal review, and update
  decoders use ordinary Pool IDs. The Blueprint-only Pool member Save editor and write surface were
  removed. Pool Edit Number remains in current Pool detail and ordinary Pool CAS/read paths.
- **Connected oracles:** `04_blueprint_pool_forks.sql` is included by the canonical
  `assessment_saved_response_oracle.sql` runner. The fixture directly saves a Blueprint reference
  to the source Pool ID without storing an Edit Number and with a selection count above current
  membership, then calls the ordinary Blueprint Course fork and confirms that the new Blueprint
  revision keeps the same Pool ID without a Pool Edit Number. It then calls the explicit
  Instructor-checked ordinary Pool fork. It checks new ID, owner, Edit Number 1, source
  pointer, metadata/Bloom equality, exact tuple equality, and independent parent/child edits. The
  SQL fixture does not call `course_blueprint_publication_assessment_content`. A separate registered
  Rust oracle, `course_publication_preserves_an_ordinary_pool_reference`, now calls the Course
  publication Store and reloads the persisted Blueprint to check the direct Pool ID and requested
  selection count, unchanged Instructor ownership, and absence of a source-linked Pool child. Its
  the root selector stopped before database connection because runtime configuration was unset; see
  the [Course publication acceptance report](output_question_spec/M11_COURSE_BLUEPRINT_ACCEPTANCE.md).
  The connected Rust adoption projection in
  `blueprint_course_postgres/adoption.rs` checks that separate Course adoptions keep the same Pool
  ID. `03_course_pool_forks.sql` retains M10's two-Assessment shared-ID, local-count, and unchanged
  owner coverage. Neither SQL fixture was executed in this task.
- **Unchanged ownership boundaries:** ordinary Pool Watch producers were left alone. The one-time
  Blueprint Theme copy remains. No changes were made to M07 Question-parent/publication ownership,
  M14 Watch/notice ownership, M24 search predicates, or M05 decoders.

## Checks

- The `question_model` Blueprint Pool wire regression checks the direct ordinary Pool ID,
  Assessment-local selection count, and absence of the retired nested Pool, owner, source-Pool,
  and Pool Edit Number fields. `source ./source_me.sh && cargo test -p question_model ordered_content_validation_uses_vector_order_and_pool_meaning`
  passed (1/1), and
  `source ./source_me.sh && cargo test -p question_model` passed (165 unit tests and one doc test).
  `git diff --check -- crates/question_model/src/blueprint_course.rs` passed. Scoped
  `rustfmt --edition 2024 --check --config skip_children=true crates/question_model/src/blueprint_course.rs`
  reports formatting differences in adjacent
  imports and a separate serialization assertion; those concurrent edits were left unchanged.
- `git diff --check` - passed.
- Fresh SPEC review initially found a Critical ordinary Blueprint fork mismatch (automatic distinct
  child Pool and copied tuple check) and a High Course publication mismatch (Pool Edit Number in
  persisted Blueprint content). Both corrections are in source; the in-flight publication snapshot
  remains unchanged as transient CAS evidence. The final fresh SPEC review accepted the corrections.
  A separate QUALITY review found no source mismatch and identified two ledger-quality findings
  (formatting and Pool Edit Number wording); both were corrected, and a different fresh QUALITY
  re-review passed.
- `source ./source_me.sh && devel/generate_schema_tables_doc.py && schema_style/check_schema_style.py` -
  passed; schema style reports `clean`.
- `source ./source_me.sh && node --import tsx --test tests/test_blueprint_course_model.mjs tests/test_question_picker_blueprint_course.mjs` - passed, 11/11.
- `source ./source_me.sh && npx prettier --check src/api/assessment_release.ts src/api/decoders/assessment_blueprint_update.ts src/api/decoders/blueprint_comparison.ts src/api/decoders/blueprint_course.ts src/features/blueprint_change_proposal/proposal_review.tsx src/features/blueprint_course/blueprint_assessment_content_editor.tsx src/features/blueprint_course/blueprint_assessment_entry_content.ts src/features/blueprint_course/blueprint_course_detail_structure.tsx src/features/blueprint_course/blueprint_course_detail_workspace.tsx src/features/blueprint_course/blueprint_history.tsx src/features/blueprint_forks/blueprint_fork_review.tsx tests/test_blueprint_course_model.mjs tests/test_question_picker_blueprint_course.mjs docs/CHANGELOG.md docs/active_plans/reports/QUESTION_SPEC_M11_POOL_OPERATIONS.md docs/active_plans/reports/question_spec_m10_m11_implementation_ledger.md` - passed.
- `source ./source_me.sh && rustfmt --edition 2024 --check crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs crates/learning-data-access/tests/blueprint_course_postgres/support.rs crates/learning-data-access/tests/blueprint_course_postgres/lineage_fork.rs crates/learning-data-access/tests/blueprint_course_postgres/exchange.rs crates/learning-data-access/tests/blueprint_course_postgres/blueprint_pool_members.rs` - passed.
- `source ./source_me.sh && cargo check -p question_model` - passed.
- `source ./source_me.sh && devel/generate_schema_tables_doc.py && schema_style/check_schema_style.py` - passed; schema style reports `clean`.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json` - blocked by concurrent TypeScript errors in `src/components/opaque_webwork_preview_frame.tsx:143` (`contentWindow` nullability) and `src/features/question_draft_preview/draft_preview_client.ts` (`credentials` is not a `RequestOptions` field). No M11 TypeScript diagnostic appeared in that run.
- An earlier `source ./source_me.sh && cargo check -p learning-data-access --test blueprint_course_postgres --no-default-features --features postgres` attempt was blocked by concurrent Pool library errors. The focused check after the publication oracle was added passes with the existing `question_pool_member_pins` dead-code warning; see the [Course publication acceptance report](output_question_spec/M11_COURSE_BLUEPRINT_ACCEPTANCE.md).
- `source ./source_me.sh && cargo check -p learning-data-access --tests --no-default-features --features postgres` - also reports unrelated current-worktree errors: duplicate `pool_selection_uses_injected_entropy_without_replacement` in `assessment_delivery_start.rs`, a missing helper argument in `assessment_attempt.rs`, and two stale `create_course_instance` test calls. The M11-specific `blueprint_pool_members.rs` stale edit-number destructure reported in that run was corrected.
- The Course publication selector compiled but stopped at `AcceptanceRuntime::load()` before database
  connection because `PLE_ACCEPTANCE_RUNTIME_MANIFEST`, `DATABASE_URL`, and
  `PLE_MIGRATION_DATABASE_URL` were unset. No database connection or container was started. The
  other connected SQL/Rust oracles remain source evidence pending their runtime gates.

## Remaining acceptance

Rerun the Course publication selector in the [acceptance report](output_question_spec/M11_COURSE_BLUEPRINT_ACCEPTANCE.md)
after configuring the existing disposable PostgreSQL runtime. The registered test target compiles;
the broader connected-test build and TypeScript workspace check may be rerun after their unrelated
workspace errors recorded above are resolved. M11 source does not require another `cargo tsgen` run
unless its Rust API DTOs change again.
