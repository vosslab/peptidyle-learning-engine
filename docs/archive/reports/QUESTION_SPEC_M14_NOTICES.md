# M14 impact notice removal

## Early scope checkpoint - 2026-10-06

### Authority and scope

- Follow the approved M14 plan: remove manually written impact notices from the schema, SQL,
  grants/policies, API/types, browser, and tests.
- Keep the settled Watch notifications for Question Revisions, Question forks, Pool Edit Number
  changes, and Pool forks. Keep Change Proposals as a separate approved concept.
- Use fork-and-fix as the constructive workflow. Keep `LibraryObjectKind` as the ordinary shared
  concept formerly colocated with the discussion model and still used by Library lookup.
- Remove notice-only `activity_id` from public and private Watch event paths. Preserve
  `question_revision_number`, `forked_public_id`, and `question_pool_edit_number` event data.
- Preserve all four ordinary event producers: publication events, the current
  `question_fork_source` relation, changed Pool tuples, and `source_question_pool_id` on Pool
  creation. M07 later owns replacing the Question fork-table producer with the ordinary parent
  creation event.

### Shared ownership and dependency

- M06 owns Question Type and permission fields; M09 owns Pool storage and selection; M22/M23 own
  statistics; M27 owns Theme. Leave those fields and paths intact.
- M09 source preparation is underway and can continue independently. Final M14 acceptance depends
  on its completion.
- The worktree already contains extensive concurrent edits. Keep edits within M14-owned notice and
  Watch-event surfaces; record any overlap here before changing shared code.
- Existing worktree edits also touch `schemas/base_schema/10_types.sql`,
  `schemas/base_schema/40_indexes.sql`, `schemas/base_schema/install.sql`,
  `crates/question_model/src/lib.rs`, `crates/learning-data-access/src/lib.rs`,
  `crates/learning-data-access/src/postgres.rs`, and `crates/server/src/composition.rs`.
  The inspected hunks concern separate metadata, Bloom, roster, parent, search, and Sysadmin work;
  preserve those edits while removing M14-owned declarations and wiring.
- `local_stack_control/database_baseline_owner.py` also has concurrent additions for M07 fork and
  Question Revision metadata acceptance plus cargo-result reporting. Three M14 Watch acceptance
  tests have separate command blocks; the existing additions remain intact.
- Generated TypeScript remains root-owned. Request regeneration after the hand-written Rust/API
  contract changes; do not edit generated files during this pass.

### Source trace answers

1. The notice-only lifecycle consisted of `library_impact_notice`, `notice_state`, notice mutation
   functions, its policy and grants, three notice indexes, its Rust store and route, and the
   `ImpactNotice` inbox variant. Those schema, module, API, browser, and notice-only test paths are
   removed.
2. `activity_id` was only the impact-notice UUID. The ordinary event sources provide their own
   evidence: publication events carry the Question Revision Number; the current Question fork
   relation carries source Revision Number and forked Question ID; Pool Edit Number changes carry
   the new number; Pool creation carries `source_question_pool_id` and the source Pool Edit Number.
3. The replacement coverage contains three connected tests in
   `blueprint_course_postgres_question_library_stewardship`: Question fork publication reaches its
   source Watcher; changing a Pool's exact Question Revision tuple set sends `membersChanged` with
   Edit Number 2; and a source-linked Pool fork sends `fork` with source Edit Number 1 and the child
   Pool ID. Each exact test is registered with `local_stack_control/database_baseline_owner.py`.

### Source findings at checkpoint

- The notice-only lifecycle is `library_impact_notice`, its `notice_state`, mutation functions,
  policy, grants, indexes, PostgreSQL store, server routes, and notice inbox variant.
- The private Watch event's `activity_id` carries only the impact-notice UUID. The other Watch
  events use `question_revision_number` or `question_pool_edit_number` and an optional fork ID.
- PostgreSQL `library_object_kind` and Rust `LibraryObjectKind` are also used by ordinary Library
  lookup and Watch behavior. Preserve both concepts.
- The former connected test created a manual notice and checked its inbox delivery. It is replaced
  by ordinary-action Watch event coverage, registered for the coordinated database baseline.
- At this checkpoint, the Question-fork case watched a Question, created and published a normal
  fork Draft, and checked that the source Watcher received the fork ID and source Revision Number.
  It used `question_fork_source`; M07 has since replaced that trigger source with the ordinary
  `published_question` parent fields. M14 preserved the event behavior and did not edit M07's
  producer change.
- The two Pool cases use normal Pool creation and Watch operations, then call the existing SQL
  member-save and source-linked fork operations to exercise the M14-owned event triggers. They do
  not change Pool save or body behavior.
- The connected Question-fork test does not insert invalid event/target rows. Its direct SQL CHECK
  helper was removed after it exercised no concrete producer or demonstrated regression; the
  database CHECK and ordinary Watch delivery assertions remain.

## Implementation and focused evidence

- Removed impact-notice-only table/type/function/policy/grant/index declarations and installation
  references. The Watch event table requires positive Question Revision or Question Pool Edit
  Numbers, has no `activity_id`, and retains only `revision`, `members_changed`, and `fork`. The
  private SQL inbox projection, Rust mapping, server response, and browser decoder
  carry no notice UUID or notice variant.
- Kept `library_object_kind` / `LibraryObjectKind` for ordinary Library lookup. Kept all four
  ordinary Watch producers and their source actions. Public JSON uses nullable
  `questionRevisionNumber` and `questionPoolEditNumber` fields as a pair; the existing target and
  event kind selects which one is populated. Rust uses typed event variants for the same mapping.
  This field-name alignment adds no notification behavior. The Watch event table now encodes the
  settled target mapping: Revision=>Question, membersChanged=>Question Pool, and fork=>either kind. Rust
  and TypeScript enforce the same mapping, including rejecting a Revision for a Pool. Change
  Proposal types and routes were left intact. Rust and browser target/fork IDs use the shared
  `LibraryObjectId`, since Pool events carry Pool IDs. The browser decoder validates the canonical
  shared ID syntax. Removed obsolete notice language from the Watched ribbon description.
- Shared-path edits were limited to M14-owned declarations, wiring, and Watch acceptance tests. M06
  Question Type and permission fields, M09 Pool storage and selection fields, M22/M23 statistics,
  and M27 Theme fields were not edited by M14. M07 has since switched the Question-fork Watch
  producer to ordinary Published Question parent creation; M14 preserved that change.
  The baseline-owner Python file keeps concurrent registrations and cargo-result reporting, with
  separate M14 blocks for each connected Watch test.
- Generated TypeScript was not edited. Request root-owned regeneration after the hand-written
  contract changes land.

### Checks run

- On the initial M14 check snapshot, `source source_me.sh && cargo test -p question_model --lib`
  passed (165 tests), both production `cargo check` commands for learning-data-access and
  server_core passed, and the earlier Question-fork acceptance target compiled with `--no-run`.
- On the latest model check snapshot, `source source_me.sh && cargo test -p question_model --lib`
  passed (164 tests). An earlier production snapshot passed both learning-data-access and server
  `cargo check` commands. The latest checks for both crates stop in shared Blueprint/Pool code with
  two map-type mismatches: `blueprint_change_proposal.rs:118` and `blueprint_lineage.rs:217` provide
  `BTreeMap<QuestionPoolId, Vec<_>>` where the current consumer expects keys of
  `(QuestionPoolId, QuestionPoolEditNumber)`. No error was reported in the M14 Watch source before
  compilation stopped.
- `source source_me.sh && node --import tsx --test tests/test_library_watch_notification_client.mjs`
  - pass, 2 tests, including rejection of a Revision event whose target is a Pool and decoding a
  Pool fork with Pool target and fork IDs.
- `source source_me.sh && node --import tsx --test --test-name-pattern 'UUIDs should never appear' tests/test_frontend_contract.mjs` - pass, 1 test.
- A one-time TypeScript contract probe passed for valid Question/Pool Revisions, member edits, and
  both fork targets; `@ts-expect-error` checks reject a Pool-targeted Revision and a
  Question-targeted member edit. Reproduce the focused API type check with:
  `source source_me.sh && npx tsc --ignoreConfig --noEmit --strict --target es2020 --module esnext --moduleResolution bundler --allowImportingTsExtensions --verbatimModuleSyntax --skipLibCheck src/api/library_watch_notification.ts src/api/decoders/shared.ts src/api/decoders/library_watch_notification.ts src/api/http_client/library_watch_notification.ts`.
  It reaches M14 files, then exits 2 on an unrelated `src/api/decoders/question_library.ts:117`
  field mismatch (`parentPublishedQuestionRevisionTuple` versus
  `publishedQuestionRevisionTuple`).
- An earlier `source source_me.sh && python3 schema_style/check_schema_style.py` snapshot was clean.
  The latest run reports eight findings in concurrent `question_authoring.sql` Draft authorship and
  metadata tables (nullable-column meaning, FK index, and clock rules); it reports no finding for
  the Watch event table. `python3 -m pytest -q tests/test_schema_table_shape.py` parses the current
  SQL source, then fails on three clock findings for the same concurrent Draft authorship table.
  M14 did not edit those Draft tables.
- `source source_me.sh && python3 -m py_compile local_stack_control/database_baseline_owner.py` - pass.
- `rustfmt --check --edition 2024` on the changed Rust model, data-access, server, and integration-test files - pass after formatting.
- `git diff --check` - pass. A scoped source search finds no impact-notice, notice-state, or `activity_id` references in schemas, Rust, browser source, or tests; the only generic `activity_id` match is an unrelated Student Work identifier test.
- Fresh SPEC review initially caught the cross-target database CHECK gap. M14 added matching SQL,
  Rust, and TypeScript validation, including malformed-payload validation. The database CHECK is
  retained without a permanent invalid-row insertion test;
  final live-state SPEC re-review passed after the shared LibraryObjectId correction. QUALITY also
  identified and corrected the stale ledger compile claim, the TypeScript event/target union, and
  Question-only typing of Pool IDs. Rust/server serialization and browser fixtures now cover Pool
  forks. Final QUALITY review passed after the Pool member-change fixture was corrected to use a
  Pool ID.

### Remaining coordinated acceptance

- Canonical integrated retry 19 started (session 78018; `output_question_spec/acceptance_integrated_retry19_20261006.log`)
  and reached the disposable PostgreSQL boundary, but produced no result. Source freeze is active;
  connected runtime acceptance remains pending.
- Fresh SPEC `watch_number_spec` and distinct QUALITY `watch_number_quality` passed. QUALITY
  withdrew its impact-notice concern after confirming the explicit M14 removal and preservation of
  the three current Watch event kinds; HEAD is not authority for this removal.
- The PostgreSQL-feature compile-only target, schema generation/style checks, and focused
  markdown/source-line tests pass. Earlier compile failures in shared Pool/Blueprint code no longer
  describe the current compile state.
- An earlier `cargo test -p learning-data-access --features postgres --lib postgres::library_watch_notification::tests`
  attempt could not compile the shared unit-test crate: concurrent errors were a duplicate
  `pool_selection_uses_injected_entropy_without_replacement` in
  `assessment_delivery_start.rs:389,415` and an `assessment_attempt.rs:276` call to
  `published_question_revision_tuple()` missing its required `&str` argument. This test-target
  attempt predates the current production check blocker.
- An earlier filtered `server_core` unit-test attempt could not compile concurrent Question Library
  tests because they called missing `search_question_library` functions. The current production
  `cargo check -p server_core --lib` passes; its unit-test crate has not been rerun since that
  earlier attempt.
- No full stack rebuild or coordinated database run was performed. The M06/M09/M22/M23/M27 owners'
  in-progress changes and the generated schema catalog remain shared work. The latest schema-style
  findings are confined to concurrent Draft authorship/metadata tables; the Watch event catalog row
  contains no notice-only column.
