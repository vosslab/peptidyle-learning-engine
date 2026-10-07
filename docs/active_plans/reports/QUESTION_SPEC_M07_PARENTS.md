# Question Spec M07: ordinary Question parents

## Outcome

Ordinary Draft and Published Question records now store a nullable exact immediate parent ID and
Revision. A fork copies the selected Revision's license, authorship, metadata, Bloom values, and
source bytes into a private Draft and reserves its new public Question ID there. Publication uses
the locked Draft snapshot and creates a new Question at Revision 1 under that reserved ID. A fork
of a fork records its immediate parent. No fork-only source table or fork type was added.

The generic Draft creation receipt binds actor, request key, fingerprint, and Draft. It is deleted
through the existing Draft deletion/publication lifecycle; there is no replay after publication.
Question Watch keeps its existing publication-time behavior and now reads the parent fields from
the ordinary Published Question insert.

## Authority and scope record

- Approved scope: M07 in `docs/active_plans/active/question_spec_implementation_plan.md`.
- Product rules: `docs/HUMAN_GUIDANCE.md` Published Question forks and Draft/publication rules.
- Preserved M06 contract: editable Type is Question metadata; a Native source binding retains its
  immutable source-Type guard. Parent fields are independent of Type. Bloom remains independently
  nullable, and owner/Sysadmin checks remain at their established gates. Fork creation accepts the
  exact available or archived source Revision; archived exact reads remain read-only.
- M14's source handoff was ready in `QUESTION_SPEC_M14_NOTICES.md` and
  `output_question_spec/M14_WATCH_TRIGGER_READY` before the Watch producer edit.
- Graphify context first rejected the existing map because `graphify-out/graph.json` linked to
  missing nodes. `source ./source_me.sh && python3 devel/graphify_map_repo.py --update` rebuilt the
  map (19,653 nodes and 53,085 edges). The checkpoint query was `graphify query "Question fork
  ordinary Draft and Published parent revision fields, source copy, and publication" --budget 1500`;
  it returned 51 candidate nodes, including `question_fork.rs`, `question_source.rs`, authoring
  Drafts, and publication commands. Scope was the listed M07 table/functions, parent projections,
  and focused tests, plus the M14 producer and M15/M16 handoffs. It excluded broad Pool/search
  exploration, UI implementation, and runtime/database acceptance.
- The worktree contains concurrent M01-M27 edits. Shared files were edited only in M07-owned SQL
  functions, parent projections, fixture fields, and tests; unrelated Pool/Blueprint work remains
  untouched.

## Implementation

- `question_authoring.sql` stores parent ID/Revision and the fork's reserved public ID on ordinary
  Drafts, copied author rows, and the generic creation receipt. `published_question.sql` stores
  parent ID/Revision on the ordinary Published Question. Pairing, positive Revision, no-self-parent,
  FK, and useful FK indexes enforce exact valid tuples. Draft parent and reserved ID fields are
  immutable after creation; Published parent fields are guarded as lineage facts.
- Owner-private RLS and existing security-definer functions cover the new Draft rows. The receipt
  can be removed only through authorized Draft deletion or successful fork publication. The old
  `draft_question_fork_source` and `question_fork_source` storage and their policies/triggers are
  removed.
- `fork_published_question_to_draft` resolves and locks the selected available or archived exact
  Revision, reserves a new public Question ID, copies its ordinary metadata (including both
  nullable Bloom dimensions), citation, license, authorship, Type, Backend/format binding, and
  verified source bytes into Draft-owned state, and stores the exact immediate parent tuple. The
  receipt fingerprint binds the actor's request key to the exact source tuple. Retries of the same
  request return its Draft; mismatched reuse is rejected.
- `publish_new_question_lineage` locks the Draft, metadata, and source binding. A fork publishes
  those Draft values, its Draft authorship, and Draft license; it does not refresh values from the
  parent. It reuses the reserved public ID, inserts the new Published Question with the Draft parent
  tuple, starts its own Revision history at 1, and records initial ownership for the Instructor who
  created the fork. Successful publication consumes the ordinary creation receipt. The publication
  source reader rejects that consumed fork Draft, so a second publication attempt cannot replay it.
  Root Question creation keeps its ordinary caller-supplied authoring path.
- Question Library and Draft list/detail projections expose the nullable parent tuple. The exact
  archived-source read remains available and read-only. M07 parent additions did not change M24
  search predicates; concurrent M24 work owns the shared search SQL changes.
- The Question fork Watch producer now runs once on ordinary `published_question` insertion,
  carrying the new Question ID and exact parent Revision. Root Questions emit no fork event; private
  Draft creation and retry do not notify. The existing event and receipt lifecycle stays in place.
- Rust, TypeScript, and API projections decode the parent as a complete nullable Revision tuple and
  reject a self-parent. The focused model and browser decoder tests cover the serialized/read shapes.
- The registered connected PostgreSQL oracle
  `blueprint_course_postgres_question_fork_parents::question_fork_parents_copy_snapshots_and_keep_immediate_parent`
  covers same-key retry, exact copied fields and bytes, later parent metadata edits, the new ID before
  publication and its Revision 1 result, initial child ownership, receipt consumption, independent
  child edits, immediate-parent fork-of-fork, and one Watch event. It is registered with the existing
  database baseline owner and was not executed.

## M15/M16 source handoff

`output_question_spec/M07_DRAFT_SQL_READY` names the finalized M07 tables, functions, wrappers,
triggers, and parent read projections. M15 may edit ordinary Draft `load_authoring_draft`,
`save_authoring_draft`, and their API wrappers in
`schemas/base_schema/50_functions/question_authoring_operations.sql`; it may also continue the
separate source route/store work in `crates/learning-data-access/src/question_source.rs`,
`crates/learning-data-access/src/postgres/question_source.rs`, and the corresponding server source
files described in `M15_M16_BOUNDARY.md`. M07 owns the parent fields, fork creation and read
projections, reserved-ID publication handoff, and published-parent Watch producer. Coordinate any
shared query or struct additions against the marker. The archived exact-source fork predicate and
the M13 UI action now have the M07 source handoff needed to proceed; M13 owns its UI acceptance.

## Verification

- `source ./source_me.sh && cargo tsgen`: passed; generated TypeScript remains root-owned.
- `source ./source_me.sh && cargo check -p question_model`: passed.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json`: passed after root-owned generation.
- `node --import tsx --test tests/test_question_summary_latest_revision_decoder.mjs`: passed, 5/5.
- `source ./source_me.sh && cargo test -p question_model question_library::tests::question_library_detail_wire_shape_has_no_source_or_grading_fields`: passed, 1/1; the test asserts the exact serialized parent tuple.
- `source ./source_me.sh && devel/generate_schema_tables_doc.py && schema_style/check_schema_style.py`: passed, clean.
- `source ./source_me.sh && python3 -m py_compile local_stack_control/database_baseline_owner.py`: passed with bytecode directed to `/private/tmp` and removed afterward.
- `rustfmt --edition 2024 --check` passed for the M07 oracle, its fixture/read-helper module, and the publication implementation/tests. The oracle was split into two focused source files so both remain below 1000 lines.
- Fresh SPEC and QUALITY reviews passed. SPEC confirmed the implementation matches the approved parent/snapshot/lifecycle contract. QUALITY found no code defect and identified stale shared-ledger status text; the M07, M13, and M15 handoff entries were corrected. A fresh focused re-review of the oracle import/scalar correction also passed. Connected acceptance remains pending.
- `npx prettier --check src/api/decoders/question_library.ts tests/test_question_summary_latest_revision_decoder.mjs`: passed. `src/pages/question_drafts_page.tsx` type-checks, but its Prettier check reports one line-wrap difference in the concurrent `creationFormat` code, outside the parent tuple change; that shared edit was preserved.
- M07-scoped `git diff --check` passed. The repository-wide staged and unstaged diff checks currently report only new blank lines at EOF in concurrent `crates/learning-data-access/src/question_metadata.rs:18` and `crates/question_model/src/question_metadata.rs:48`; those files are outside M07 and were preserved. The old `draft_question_fork_source` and `question_fork_source` names are absent from active schema, Rust, API, and test source.
- `source ./source_me.sh && cargo test -p question_model`: 163/164 passed; one unrelated concurrent Blueprint serialization test fails at `blueprint_course.rs:436` (`Null` versus `"import"`). The focused M07 model test passes.
- `source ./source_me.sh && cargo check -p learning-data-access --features postgres`: passed. The focused `server_core` publication unit-test build stops in concurrent Blueprint Pool view field mismatches at `crates/server/src/blueprint_course/views.rs:207,215`. The registered connected oracle `--no-run` now compiles the M07 oracle module, then stops on a concurrent M09 Pool fixture API mismatch at `crates/learning-data-access/tests/blueprint_course_postgres/blueprint_pool_members.rs:39` (`QuestionPoolId::as_string`; the current API exposes `as_str`). No database-backed test was run.
- The registered oracle selector remains
  `blueprint_course_postgres_question_fork_parents::question_fork_parents_copy_snapshots_and_keep_immediate_parent` in `local_stack_control/database_baseline_owner.py`. Its target compile is blocked only by the unrelated sibling Pool fixture error above; the oracle and all connected database/container gates were not run.
- Historical settled design already selected reservation of the public ID at fork Draft creation.
  The implementation and current fork spec now agree: the Draft UUID remains private, while the
  reservation registry holds its distinct public Question ID for publication at Revision 1.
- That early reservation follows the registry's existing permanent no-reuse rule. Deleting an
  unpublished fork Draft therefore does not reclaim its randomly generated public ID. No new
  allocation or replay workflow was added after publication.

## Acceptance boundary

This is an implementation report, not M07 runtime or milestone acceptance. The connected oracle is
registered but unexecuted; the shared integration target and server crate are blocked by the listed
concurrent compile errors. No full database/container or browser milestone was run.
