# Fork model code audit

## Result and scope

The code only partly follows the intended model. Fork creation generally creates an ordinary
object with a new identity. Assessment and Blueprint Pool handling then adds special ownership
and attachment restrictions that prevent normal Pool reuse. Question and Blueprint parent links
also live in separate fork tables rather than on their ordinary records.

Neil's rule for this review: forking creates another object of the same type, with its own ID,
ordinary lifecycle, and a parent/source reference. A fork is not a separate object type.
The existing Question Draft/publication lifecycle and exact source Revision reference still apply.
The settled Pool model uses direct Assessment references and explicit forks.

This is a static review of schema source, SQL functions and constraints, Rust types and callers,
TypeScript clients, and selected tests. It did not inspect a running database or execute application
tests. Code and schema are unchanged. Fixes are recorded in [TODO.md](../../TODO.md#question-spec-implementation-follow-up).

## Tables with fork names

The schema contains exactly five `CREATE TABLE` declarations whose table names contain `fork`.

| Table | Current purpose | Finding and correction direction |
| --- | --- | --- |
| `ple_data.assessment_question_pool_fork` | Associates a Pool with exactly one Assessment entry | Direct model conflict. Use the ordinary Assessment Pool reference and the Pool's Instructor owner. |
| `ple_data.question_fork_source` | Stores one exact source Question ID and Revision for a new Published Question | Parent information in a separate table, not a separate Question body. Move the source reference onto the ordinary Question record while retaining the exact source Revision. |
| `ple_private.draft_question_fork_source` | Stores a Draft's source reference together with actor and retry-key data | Combines parent information and request handling. Keep the source reference with ordinary Draft state and carry it through publication; assess retry handling separately. |
| `ple_data.blueprint_course_fork` | Stores one exact source Blueprint Revision per child Blueprint, plus a separate UUID for that relationship | The child is an ordinary Blueprint, but its parent relation is a separate object. Put the source reference on the ordinary Blueprint record. |
| `ple_data.blueprint_course_fork_receipt` | Remembers an accepted request so repeating it returns the same child | Request bookkeeping, not a fork subtype or copied history. Review how ordinary creation can retain this protection without a dedicated Fork table. |

Definitions are in [assessment.sql](../../../schemas/base_schema/20_tables/assessment.sql),
[published_question.sql](../../../schemas/base_schema/20_tables/published_question.sql),
[question_authoring.sql](../../../schemas/base_schema/20_tables/question_authoring.sql), and
[blueprint_course.sql](../../../schemas/base_schema/20_tables/blueprint_course.sql).

## Highest-impact drift: Pool reuse

`assessment_question_pool_fork.question_pool_id` is unique, so a Pool can belong to only one row
in that association. The deferred foreign key `assessment_entry_owns_exact_question_pool_fork`
requires every Assessment Pool entry to have that association. The trigger
`validate_assessment_question_pool_fork` also requires a nonempty source-Pool reference.
Consequently, an Assessment cannot simply reference an original Pool, and the same fork cannot
serve multiple Assessment entries as an ordinary reusable Pool.

Evidence:

- [assessment.sql](../../../schemas/base_schema/20_tables/assessment.sql),
  `assessment_question_pool_fork` declaration and its one-entry ownership comment.
- [30_constraints.sql](../../../schemas/base_schema/30_constraints.sql),
  `assessment_entry_owns_exact_question_pool_fork`.
- [assessments.sql](../../../schemas/base_schema/50_functions/assessments.sql),
  `validate_assessment_question_pool_fork` and current-content save checks.
- [assessment_pool_forks.sql](../../../schemas/base_schema/50_functions/assessment_pool_forks.sql),
  `import_assessment_question_pool_fork` creates a child Pool while attaching it; member editing
  and reads join through the ownership association.

This behavior extends beyond SQL. `AssessmentQuestionPoolForkView` describes an Assessment entry
as owning the fork and includes Assessment-specific selection count alongside Pool state.
The API and editor follow that model:

- [question_pool_library.rs](../../../crates/question_model/src/question_pool_library.rs).
- [assessment_pool_fork.rs](../../../crates/server/src/assessment_pool_fork.rs), Course/Assessment
  routes ending in `question-pool-forks` and automatic new Pool ID allocation.
- [assessment_pool_fork.rs](../../../crates/learning-data-access/src/postgres/assessment_pool_fork.rs),
  Store calls to the SQL import operation.
- [assessment_pool_entry_editor.tsx](../../../src/pages/assessment_workspace/assessment_pool_entry_editor.tsx)
  and [assessment_pool_fork.ts](../../../src/api/assessment_pool_fork.ts).

Existing tests reinforce the old rule. For example,
[03_course_pool_forks.sql](../../../tests/e2e/assessment_saved_response/03_course_pool_forks.sql)
expects attachment without a fork to fail. Updating names alone would leave the model wrong.

## Blueprint and Course Pool copies

The same special handling exists in Blueprint and Course assembly:

- [blueprint_pools.sql](../../../schemas/base_schema/50_functions/blueprint_pools.sql) rejects a
  Pool appearing more than once in a Blueprint and requires a new attachment to use a fork
  created in the current transaction. It also checks that another Assessment/Blueprint has not
  already claimed that Pool.
- [course_blueprint_adoption.sql](../../../schemas/base_schema/50_functions/course_blueprint_adoption.sql)
  creates Pool forks and Assessment ownership associations during adoption.
- [assessment_blueprint_updates.sql](../../../schemas/base_schema/50_functions/assessment_blueprint_updates.sql)
  does the same when applying Blueprint content to an Assessment.
- [blueprint_lineage.sql](../../../schemas/base_schema/50_functions/blueprint_lineage.sql),
  `fork_blueprint_course`, verifies replacement child Pools rather than retaining existing Pool
  references in the new Blueprint.

These paths need the same direct-reference correction as Assessment Add. Explicitly creating a
separate Pool remains available when an Instructor wants independent membership. Preserve
existing Attempt selections and the established post-issue editing restrictions.

## Parent storage and ordinary lifecycle

The parent-table findings do not mean that Question and Blueprint forks have separate bodies or
copied Revision histories:

- [question_authoring_operations.sql](../../../schemas/base_schema/50_functions/question_authoring_operations.sql)
  inserts into ordinary `draft_question` and its normal metadata/source tables. It then adds
  `draft_question_fork_source`. The inspected path copies the selected source, not all Revisions.
- [question_publication_operations.sql](../../../schemas/base_schema/50_functions/question_publication_operations.sql)
  inserts an ordinary `published_question`, Revision 1, initial ownership, and the source relation.
- [blueprint_lineage.sql](../../../schemas/base_schema/50_functions/blueprint_lineage.sql) creates
  an ordinary private `blueprint_course`, sets the child's Revision to 1, and adds its source
  relation and retry receipt. Reads use the ordinary Blueprint availability rules.
- [question_pool.sql](../../../schemas/base_schema/20_tables/question_pool.sql) already has the
  desired nullable `source_question_pool_id` self-reference. In
  [question_pools.sql](../../../schemas/base_schema/50_functions/question_pools.sql),
  `construct_question_pool_fork` creates an ordinary Pool with its owner, Edit Number 1, copied
  current members and metadata, and source ID. It does not create Pool Revision history.

Move parent information rather than creating replacement fork objects. Retain the exact source
Revision for Questions and Blueprints. A source pointer describes where the new object came from;
it does not require copying the source's history into the child.

Operation names such as `ForkPublishedQuestionInput` describe an action and its input. The name
alone does not establish a separate persistent object type. The Assessment-owned Pool view is
different: its fields and SQL constraints enforce special ownership and use.

## Other confirmed follow-up

- The inspected Pool constructor copies the ordinary metadata columns but does not copy
  `question_pool_bloom`. Q29 requires a Pool fork to copy existing Pool metadata, including Bloom.
  Reconcile this with the already queued ordinary-metadata cleanup.
- Question publication checks for a source relation and rejects any different license. Neil
  clarified that a fork starts with the source Question's license, with no license choice during
  forking. The proposed license correction was based on a misunderstanding; F06 in the
  [authority audit](QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md) is withdrawn.

No full Revision-history copying was found in the creation paths inspected. This is a bounded
source finding, not runtime verification or a claim that every fork-related operation is correct.

## Broad search and correction scope

A broader search for `fork` and `assessment_owned` across schemas, Rust, TypeScript, tests,
development tools, and launchers found 150 source/test files with mentions: 45 schema files,
51 Rust files, 34 TypeScript application files, and 20 test files. These are search hits, not
150 defects. Inspection focused on the creation, storage, attachment, and read paths above.

Additional concrete evidence:

- `BlueprintPoolInputChoice` in
  [assessment_content.rs](../../../crates/question_model/src/blueprint_course/assessment_content.rs)
  offers `Import` and `Retained` alternatives built around Assessment ownership. Its `Import`
  branch in [blueprint_pools.rs](../../../crates/learning-data-access/src/postgres/blueprint_pools.rs)
  allocates a child Pool ID, calls `fork_blueprint_question_pool`, and replaces the entry's Pool
  ID. This is executable copy-on-add logic, not merely stale comments.
- The browser imports generated `AssessmentQuestionPoolForkView` definitions. Generated API
  files are absent from this checkout; their Rust definition and TypeScript consumers were
  inspected. Regenerate the API types when correcting those definitions.
- [support.rs](../../../crates/learning-data-access/tests/blueprint_course_postgres/support.rs),
  [adoption.rs](../../../crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs),
  [04_blueprint_pool_forks.sql](../../../tests/e2e/assessment_saved_response/04_blueprint_pool_forks.sql),
  and [test_assessment_pool_fork_import_client.mjs](../../../tests/test_assessment_pool_fork_import_client.mjs)
  are part of the affected test and fixture setup.
- [bloom_classification_workflow_harness.tsx](../../../tests/support/bloom_classification_workflow_harness.tsx)
  contains an Assessment-owned Pool fork fixture. Fixture setup needs the same ordinary-Pool
  correction as production code.
- Current generated [SCHEMA_TABLES.md](../../SCHEMA_TABLES.md), SQL comments, and active API/code
  documentation need regeneration or alignment after implementation. Archived reports and
  changelogs preserve historical evidence; their old assertions are not current authority.

Treat this as a direct pre-production schema correction. Keep the ordinary object creation and
Revision behavior that already fits. Correct the special Pool ownership, forced copies, parent
storage, and their callers together. Completion requires source and behavior checks across SQL
constraints, policies and grants, Rust operations/types, generated API types, browser clients and
editors, tests, fixtures, and current documentation. A table rename alone does not complete it.

## How the model developed

This history was checked against the local Git history, which is not shallow, and the relevant
September 15 Codex session records. Dates and times below use America/Chicago (CDT, UTC-05:00).
Commit dates establish when changes were recorded; the session timestamp identifies the actual
table-writing tool call. Git author names alone do not identify who proposed or approved a rule.

### September 14: wording in HG

Commit `9fc51c0aa4eec6b5524f1509d48a273a66714169`, September 14 at 14:10:57, adds a
Question Pools section to HG during a much larger rewrite. It includes these adjacent bullets:

> Importing a Question Pool into a new Assignment automatically forks the Question Pool.
>
> The fork belongs to the new Assignment and can be changed without changing the source Question Pool.

The same section calls a Pool an independently reusable Question Library object. The recorded
reason for the copy is independent editing without changing the source. The phrase "belongs to"
does not explain whether the copy can be reused by another Assessment.

Commit `9e8b992f`, September 14 at 21:58:16, changes Assignment to Assessment in those two
bullets. It preserves the ownership wording. The relevant HG history starts with the September 14
addition, not the October Question-spec refactor. This establishes the wording's entry into the
tracked document; it does not establish who first proposed the sentence or a separate human
approval of exclusive Assessment ownership.

### September 15: exclusive SQL ownership

Commit `a49844cc25da1b63c77a5970db5c1d41fb1ab7d5`, September 15 at 08:25:34, adds
`assessment_question_pool_fork` in the then-current `schemas/base_schema/assessments.sql`.
It includes all of the decisive restrictions:

- `question_pool_id NOT NULL UNIQUE`: one Pool can belong to only one association.
- `assessment_entry_owns_exact_question_pool_fork`: Assessment entries must use that association.
- A trigger requires a source Pool reference, so an ordinary original Pool is rejected.
- The creation comment says the Pool is "owned by exactly one Assessment Entry" and deliberately
  prevents attaching a published Pool or another fork through ordinary Assessment saving.

The parent commit has `question_pool_item` rows attached directly to an Assignment entry. The
new work replaces those local member lists with ordinary Pool records plus the special exclusive
association. It fixes the missing Pool identity and source evidence while preserving local ownership.

The session records narrow the timing and explain the engineering rationale:

- At 08:09:47, the manager reports that Assessment Pools are mutable local item lists while
  reusable Pools have immutable records. It assigns SQL and typed-contract corrections.
- At 08:11:29, agent `/root/immutable_pool_forks/c905_sql_pool_fork_provenance` submits the patch
  creating the association, unique constraint, and foreign key.
- At 08:15:41, the manager reports that importing a reusable Pool now creates an Assessment-owned
  child instead of referencing the original. It presents this as a provenance correction.
- At 08:19:34, it describes the API restriction: the browser can supply a source Pool but cannot
  supply another Assessment's fork. At 08:22:50, it closes the ordinary-save path for changing a
  fork's Revision, leaving membership changes to a dedicated operation.

These are recorded agent actions and progress messages, not inferred human decisions. Their
local session IDs are `01a0a52c-f816-7671-a323-92680a518ba7` (manager) and
`01a0a52e-2d46-74c3-b4ec-e8c8c354d3c5` (SQL worker). The relevant records are under
`/Users/vosslab/.codex/sessions/2026/09/15/`; the SQL patch is line 281 of the worker's JSONL file.

The surrounding HG-compliance plan already assigned C905-C909 to Pool provenance, selection,
and delivery. Commit `6e916396` records that plan at 07:17:42. Later plan language calls the
Assessment-entry selection count an engineering decision consistent with HG. That count belongs
on the Assessment entry under today's model too; it does not require exclusive Pool ownership.

### September 16: Blueprint restrictions

Commit `17146319`, September 16 at 07:21:02, reinforces automatic fork-on-add in HG and says
the child can change independently. It does not separately explain a ban on reusing the child.

Commit `2c9b700b25dc3378a7933f95293f541591dcd5bb`, September 16 at 07:29:33, adds
`schemas/base_schema/blueprint_pools.sql`. Its validation permits a newly attached Pool only
when it is a fresh fork created in that transaction and has no previous Assessment or Blueprint
attachment. It also rejects repeated use of a Pool in the Blueprint with the message
"A Blueprint owned Pool may occur only once". The exclusive model has now spread beyond the
Course Assessment table into Blueprint persistence.

### September 18: ownership survives simplification

Commit `b01182b8`, September 18 at 10:01:36, records HG's current-state Pool direction.
Commit `a2eee8ac`, September 18 at 17:32:35, removes Pool Revision storage and changes the
association to reference the current Pool row. It retains the unique Pool constraint and the
Assessment-owned association. Its comment changes from creating Revision 1 to copying current
members once. The historical-storage model changes; exclusive ownership survives.

### October 5: two ownership models coexist

Commit `f7d8fb7d`, October 5 at 12:03:52, adds `owner_account_id` to the ordinary Pool table.
It leaves the older Assessment-owned association in place. This explains the present contradiction:
the Pool has an Instructor owner, but separate constraints still limit its reuse as if an Assessment
owned it. The current documentation corrections supersede automatic fork-on-add as well.

### Cause and confidence

The evidenced chain is: HG describes automatic copying and says the copy "belongs to" the
Assessment; compliance implementation turns that into exclusive database ownership; Blueprint
code repeats the restriction; later schema work preserves it. Independent editing and exact
Student Work evidence are the recorded reasons. Neither requires a one-Assessment-only Pool.

The likely overreach is treating the location where a copy was created as permanent, exclusive
ownership of that copy. That is an interpretation supported by the sequence, not a recovered human
decision. Automatic fork-on-add and exclusive ownership are distinct: the former was explicitly in
historical HG and was also supported earlier in the current conversation; the latter is the added
restriction under review. Current direct references and explicit forking supersede the older Add
behavior. The history does not support saying the whole model appeared from nowhere, or blaming
the original ownership restriction on the recent shared-search work.

## Validation

The documentation format and local-link checks passed: 450 tests. `git diff --check` passed.
These checks validate the audit documents, not runtime fork behavior. No application build,
database installation, or end-to-end test was run for this read-only code review.
