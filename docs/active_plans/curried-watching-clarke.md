# Plan: base schema compression and revision unwind

<!-- Drafted with blueprint-plan-drafter and postgresql-expert. Supersedes
docs/active_plans/active/backend_terminology_reconciliation_2026_09.md, whose scope this plan
absorbs. Published copy lives at docs/active_plans/active/base_schema_and_revision_unwind_2026_09.md
after M0. -->

## Context

`schemas/migrations/` holds 101 forward migrations (20,724 lines, `2026082901` through
`2026091029`) for a product that has never shipped. The chain carries 293 `ALTER TABLE`, 23
`DROP FUNCTION`, and 30 files that `CREATE OR REPLACE` earlier objects: updates on updates on
updates. Every reader of the schema, human or tool, has to replay the chain mentally to learn a
table's current shape. Graphify maps the repository from these patches and reports the schema as a
web of superseded definitions. `docs/DATABASE_STRUCTURE.md` keys its ownership map by migration
number range instead of by table family.

The chain also carries four revision families that
[docs/TERMINOLOGY_CONTRACT.md](../TERMINOLOGY_CONTRACT.md) and
[docs/HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md) now reject: `course_schedule_revision`,
`assignment_revision` plus its four entry-snapshot tables, `question_change_proposal_revision`, and
`course_retention_plan_revision`. The owner's triage in
[docs/active_plans/revision_concerns.txt](../revision_concerns.txt) named them; the audit behind this
plan confirmed each one. `course_schedule_revision` is written exactly once per Course (revision 1)
and every reader either hardcodes `revision_number = 1` or takes the latest.
`course_retention_plan_revision` has no writer at all. `assignment_revision` is read by twenty-two
SQL functions, but the facts they consume reduce to a short list that belongs on
`ple_private.assignment_attempt` and `ple_private.issued_question` directly. The Attempt already
captures its started title and due instant; this plan finishes that move.

[docs/ROADMAP.md](../../ROADMAP.md) already states the governing policy: before v1 ships, disposable
databases may be recreated from the reviewed baseline; after v1 ships, no migration is ever edited.
This plan is the pre-v1 recreation. It replaces the chain with one authored base schema in
`schemas/base_schema/`, split by table family so every file stays under the 1000-line source cap,
applied to an empty database by `psql --single-transaction`, and keeps `schemas/migrations/` as
the SQLx forward-migration folder that starts empty and receives every post-launch change. The two
concerns use the simplest tool each: PostgreSQL's own client initializes; SQLx migrates. The base is
authored directly in its final revision-free shape; there is no intermediate faithful squash,
because the owner selected single combined authoring and because the revision tables have no
production data to carry.

The superseded reconciliation plan also owned four capabilities that sit on the same tables: stable
lineage-level Published Question and Blueprint Course availability with their Danger Zone
archive/restore actions, a mutable Blueprint Draft with explicit publication, atomic Assignment
Unrelease, and a Student-owned Account Time Zone route. The owner folded them in. They land after the
schema cutover, each as its own milestone.

## Objectives

- Replace `schemas/migrations/` with `schemas/base_schema/` (numbered, family-scoped files) and
  `schemas/schema_updates/` (post-launch forward migrations), embedded by one generated migrator.
- Make Question Revision and Blueprint Revision the only revision concepts in the schema, the Rust
  model, the API, the generated TypeScript, the tests, and the current documentation.
- Give Assignments, Course Instances, Question Change Proposals, and Course Retention Plans one
  mutable current state, with Edit Numbers only where a concurrent save exists.
- Make every Assignment Attempt and Issued Question retain the exact delivery, policy, disclosure,
  and Question Revision facts it needs, with no join to mutable Assignment state after start.
- Deliver the three HUMAN_GUIDANCE Danger Zone actions end to end: Assignment Unrelease, Archive
  Published Question, Archive Blueprint Course, each with restore where guidance names one.
- Separate Blueprint Draft saves from Blueprint publication.
- Give Students the same self-owned time-zone read/write path Instructors already have.
- Remove every e2e oracle that proves old revision logic; rewrite only the oracles that protect a
  durable contract on the new model.
- Leave `docs/DATABASE_STRUCTURE.md`, `docs/DATABASE_AUTHORIZATION.md`, `docs/ROADMAP.md`, and
  every citing document describing the base-plus-updates layout, with no migration-number citations.

## Design philosophy

Author once, in the final shape. **Fix the design, not the symptom**: a faithful squash would
reproduce four revision families the contract rejects and then delete them in the same plan. The
audit already inventoried every consumer of every retired table, so the base files are written
directly to the current-state and evidence model, and the fresh database is the oracle.

Evidence goes on the record that owns it. Attempts copy the facts they use; Issued Questions copy
theirs. No generic snapshot, no policy JSON blob, no `assignment_revision_id` in disguise. That is
the owner's explicit rule from `revision_concerns.txt` and it is the difference between this plan
and a rename.

Many small milestones. Each base-schema milestone authors a bounded family of files and proves them
with a fresh apply, a no-op reapply, and one behavior oracle for that family. Rust, API, and browser
work land only after the schema cutover, each capability as its own milestone, so an unfinished
capability never blocks the schema.

Tests are liabilities. Every e2e oracle that asserts revision tables, column inventories, policy
names, or role membership counts is deleted rather than ported. The retained set asserts contracts:
`ple_app` cannot reach private tables, evidence rows are immutable, an old Attempt survives an
Assignment edit, Unrelease leaves no Student row. Per PYTEST_STYLE.md, when in doubt the oracle goes.

No human gate. Every milestone finishes on captured fixtures, disposable PostgreSQL 17 stacks, the
existing acceptance applier, and `reviewer` or `architect` subagents. Only humans commit, per
REPO_STYLE.md, so no gate depends on a commit; the plan ends with a passing working tree and a
changelog entry.

- Evidence strategy for uncertain methods: the sqlx 0.9 `Migrator::with_migrations` and
  `Migration::new` constructors are confirmed in current docs; WP-LAY1 proves them with a compiled
  embed before any base file beyond `0001` is written. The Attempt fact list is confirmed by reading
  every current function that touches `assignment_revision` (recorded under Data inventory). The
  Unrelease deletion graph is confirmed by a foreign-key walk from `assignment_attempt` outward,
  captured as an oracle before the deletion function is written.

## Scope

- Add `schemas/base_schema/` with consecutively numbered `NNNN_<family>.sql` files and
  `schemas/schema_updates/` with its README and no migrations.
- Generate the embedded migrator from both folders in `crates/learning-data-access/build.rs`; point
  the acceptance applier, the live-demo migrator image, and `.containerignore` at the new layout.
- Author every base file to the revision-free model: Assignment current content with exact Question
  Revision pins, Attempt and Issued Question copied facts, Course Instance term dates, current
  Question Change Proposal and Course Retention Plan rows, lineage-level availability events, and a
  current Blueprint Draft.
- Rewrite every `ple_api` and `ple_private` function that read a retired table to read current
  state or copied evidence.
- Delete `schemas/migrations/` and the three stale entries in
  `tests/source_file_line_limit_overrides.txt`.
- Remove the retired Rust types, decoders, generated TypeScript, fixtures, and tests; regenerate
  `generated/api/`.
- Drop `revisionNumber` from the release response and its UI message; release advances the
  Assignment Edit Number.
- Implement Unrelease impact and action routes, the typed-title Danger Zone confirmation, complete
  Student Work deletion, and Question Revision Statistics recompute.
- Implement Published Question archive/restore and Blueprint Course archive/restore with Danger Zone
  confirmation and ordinary restore controls.
- Implement Blueprint Draft GET/PUT and explicit `publish`.
- Implement Student self time-zone GET/PUT with a minimal Student profile page.
- Replace old-logic e2e SQL oracles with family-scoped contract oracles; split the 999-line
  `postgres_migration_acceptance_catalog.sql` out of existence.
- Add one fast offline pytest for the schema layout contract.
- Rewrite `docs/DATABASE_STRUCTURE.md`, `docs/DATABASE_AUTHORIZATION.md`, `docs/ROADMAP.md`,
  `docs/DESIGN_DECISIONS.md`, and every document that cites a migration filename.
- Archive the superseded reconciliation plan, close `revision_concerns.txt`, regenerate Graphify
  output, and record evidence in `docs/CHANGELOG.md`.

## Non-goals

- Preserve any existing database volume, ledger row, or migration checksum. Pre-v1 policy allows
  recreation and the compatibility check makes old volumes incompatible by design.
- Add a Course Instance Edit Number. Every current Course Instance mutation (theme, banner, names)
  is a single-field replace with no concurrent form; the terminology contract adds Edit Numbers
  only where concurrency needs one. This version succeeds because no two Instructors edit the same
  Course field through the same form.
- Add product routes for Question Change Proposals or Course Retention Plans. Both become current
  rows with Edit Numbers so the schema is honest; their workflows are unimplemented today and this
  version succeeds without inventing them.
- Add Student Accommodation routes, Blueprint copy/update routes, or grading-operation routes. Their
  retired revision scaffolding is deleted; no replacement API is built because no product surface
  consumes one.
- Change Question Revision or Blueprint Revision semantics, checksums, or encodings.
- Add a compatibility view, alias, dual-write, fallback reader, or create-then-drop migration.
- Change Close or Archive into destructive Assignment actions. Only Unrelease deletes Student Work.
- Run `git commit`. Humans commit; the plan ends with a clean, gated working tree.

## Current state summary

Layout and runtime (from the audit):

- `crates/learning-data-access/src/postgres/migrations.rs:16` embeds
  `sqlx::migrate!("../../schemas/migrations")`; `build.rs:2` reruns on that directory.
  `apply_migrations` (`:372-398`) runs under advisory lock `0x504c455f5343484d` and tolerates the
  post-run ledger recheck failing because `2026082901` revokes `CREATE ON SCHEMA public` from
  `ple_migrator`. `evaluate_migration_check` (`:148-181`) marks any ledger version absent from the
  embed as unexpected, so a re-based schema needs a fresh volume.
- `crates/project-tools/src/database_postgres_migration_acceptance.rs:12-14` hardcodes
  `schemas/migrations`, role `ple_migrator`, and first version `2026082901`; `:119-193` splits each
  file into top-level statements so `SET LOCAL ROLE` precedes policy compilation; `:344-356` refuses
  any other directory.
- `tests/e2e/e2e_postgres_migration_acceptance.sh:26,160-168` asserts `2026082901` pending, fresh
  apply, no-op reapply, verify, then pipes 24 SQL oracles (`:169-215`) through psql.
- `local_stack_control/lifecycle_migrations.py` runs `database migrate` inside the
  `database-migrator` Compose image built from `containers/Containerfile.api`, which copies
  `schemas/migrations` at `:24`; `.containerignore:12-14` whitelists it.
- Bootstrap roles (`ple_database_owner`, `ple_migrator`, three capability roles) are created by
  superuser psql before migrations in `local_stack_control/lifecycle_database.py:12-41` and
  `runtime_manifest.py:795-817`; the base must assume them, not create them.
- `tests/test_source_file_line_limit.py` caps every tracked `.sql` at 999 lines;
  `tests/source_file_line_limit_overrides.txt:3-5` lists three files that no longer exist.
- No `pg_dump`, ERD, or schema generator exists; `docs/DATABASE_STRUCTURE.md` is hand-maintained.

Revision machinery (from the audit; full inventory under Data inventory):

- `course_schedule_revision`: one writer (`create_live_demo_course_instance`, revision 1), five
  readers, all effectively "the current row". Term dates exist nowhere else.
- `assignment_revision` and entries: written only by `release_live_demo_assignment`; only
  `fixed_question` entries are ever written; the pool tables are never inserted. Twenty-two functions
  read it. Rust never touches the tables directly; it calls `ple_api.*`.
- `course_retention_plan_revision`: no writer; FK targets from `ple_private.job` and
  `ple_audit.course_retention_event`.
- `question_change_proposal_revision`: FK target from `ple_audit.question_change_event`; no route.
- Wire leaks: `ReleasedLiveAssignment.revisionNumber` (release response and the UI string
  `Assignment released as revision N.`), `AssignmentAttempt.assignmentRevision` (generated type,
  decoded, consumed only by a Node test), `SuccessorAssignmentRevisionRequired` (409 body),
  grading-operation `assignment_revision` (dead contract). `assignmentRevisionEtag` parameters in
  `src/api/client.ts` and `request.ts` carry the Edit Number ETag and only need renaming.
- Already done and reused: Account Time Zone storage with IANA enforcement (`2026091001`),
  Instructor profile GET/PUT (`2026091014`, `/api/instructor-profile`), Course time zone retired
  (`2026091013`), Attempt-captured title and due (`2026091023`).

## Architecture boundaries and ownership

- `schemas/base_schema/` is the sole physical schema authority. Each file owns one table family:
  its tables, keys, constraints, triggers, functions, grants, RLS policies, and comments.
- `schemas/schema_updates/` holds post-launch forward migrations only. Its README states the rule.
- `crates/learning-data-access/build.rs` owns the generated embed; `migrations.rs` owns the
  `Migrator`, advisory lock, and compatibility check; `project-tools` owns the acceptance applier
  and CLI.
- `question_model` owns Edit Number, status, availability, and evidence types.
- `learning-data-access` owns SQL call sites; `server` owns DTOs, ETags, status codes, and Danger
  Zone precondition checks.
- The browser owns Danger Zone presentation and typed-title confirmation; the server verifies.
- `tests/e2e/` owns family-scoped contract oracles; `tests/` owns the layout contract pytest.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component | Review boundary |
| --- | --- | --- |
| M0 / WS-DOC | plan publication, `docs/DESIGN_DECISIONS.md`, archive move | `architect` |
| M1 / WS-LAY | `schemas/base_schema/0001`, `schema_updates/README.md`, `build.rs`, `migrations.rs`, acceptance applier, Containerfile, `.containerignore`, layout pytest | `reviewer` on the embed and applier |
| M2-M9 / WS-BASE | `schemas/base_schema/0002-0020`, one oracle per family | `reviewer` per family; `architect` on M7 and M8 |
| M10 / WS-CLOSE | `0021` RLS closure and witness, renumbering, oracle set | `reviewer` |
| M11 / WS-CUT | delete `schemas/migrations/`, `learning-data-access` call sites, decoders, lanes | `integrator` |
| M12 / WS-RUST | `question_model`, `browser-api-contract`, `generated/api/`, Node tests | `reviewer` |
| M13 / WS-PQ | Published Question archive/restore, Danger Zone | `reviewer` plus security review |
| M14 / WS-BP | Blueprint Draft, publish, archive/restore | `reviewer` |
| M15 / WS-UNR | Unrelease impact, action, deletion, statistics recompute | `architect` plus security review |
| M16 / WS-STZ | Student time zone route and page | `reviewer` |
| M17 / WS-EVI | docs, changelog, Graphify, plan archive | `maintainer` |

## Milestone plan

Numbers are labels; `Depends on` is the order.

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M0 | Publish and supersede | Publish this plan, archive the reconciliation plan, record decisions | One plan of record |
| M1 | Layout and embed | Two folders, generated migrator, applier, `0001` principals | Runtime proven on one base file |
| M2 | Accounts and sessions | `0002`-`0003` | Identity family |
| M3 | Question Library | `0004`-`0005`, `0007` | Published content with lineage availability |
| M4 | Authoring and stewardship | `0006`, `0008` | Drafts, imports, current proposals |
| M5 | Blueprint Courses | `0009` | Draft, revisions, lineage availability |
| M6 | Course Instances | `0010`-`0012` | Term dates on the Course, roster, appearance |
| M7 | Assignments | `0013` | Current content with exact pins; release without revision |
| M8 | Student work | `0014`-`0016` | Copied Attempt and Issued Question facts |
| M9 | Grading, analysis, jobs, retention, iMathAS | `0017`-`0020` | Remaining families; statistics recompute |
| M10 | Closure and renumber | `0021` RLS closure and witness, consecutive numbering | Complete base, old oracles gone |
| M11 | Cutover | Delete chain, adapt call sites, all lanes green | Live demo runs on the base |
| M12 | Rust and TypeScript cleanup | Retired types and decoders removed | No revision noun outside Question/Blueprint |
| M13 | Published Question availability actions | Archive, restore, Danger Zone | HUMAN_GUIDANCE Danger Zone item 2 |
| M14 | Blueprint Draft and availability actions | Draft save, publish, archive, restore | Danger Zone item 3; no implicit publish |
| M15 | Assignment Unrelease | Impact, typed-title action, deletion, recompute | Danger Zone item 1 |
| M16 | Student time zone | GET/PUT route and page | Students own their display zone |
| M17 | Documentation close-out | Docs, changelog, Graphify, archive | Current docs, no stale citations |

### Milestone: M0 publish and supersede

- Depends on: none.
- Deliverables: this plan copied to
  `docs/active_plans/active/base_schema_and_revision_unwind_2026_09.md`;
  `git mv docs/active_plans/active/backend_terminology_reconciliation_2026_09.md docs/archive/`
  with a one-line superseded note at its top; four `docs/DESIGN_DECISIONS.md` entries (Decision,
  Why, Consequence, Owner): base-plus-updates schema layout and post-launch freeze; Attempt and
  Issued Question copied facts as the sole delivery evidence; no Course Instance Edit Number;
  availability on lineages.
- Workstreams: WS-DOC.
- Entry criteria: none.
- Exit criteria: `source source_me.sh && python3 -m pytest tests/test_markdown_links.py` passes;
  the archived plan's first line names the superseding plan.
- Parallel-plan ready: no. One owner writes all three documents.

### Milestone: M1 layout and embed

- Depends on: M0 for the recorded layout decision.
- Deliverables: `schemas/base_schema/0001_principals_and_schemas.sql`;
  `schemas/schema_updates/README.md`; generated embed in `build.rs`; `Migrator` built with
  `with_migrations`; acceptance applier iterating the embedded list with first version `1`;
  `Containerfile.api` and `.containerignore` copying both folders; `--migrations-dir` accepting
  either folder; `tests/test_schema_layout_contract.py`; the acceptance script's first-version
  assertion updated and an `--oracles` flag added.
- Workstreams: WS-LAY.
- Entry criteria: M0 exit.
- Exit criteria: `./check_rust.sh` passes with the embed containing exactly `0001`; on a fresh
  disposable PostgreSQL 17 database the acceptance lane applies `0001`, reapplies as a no-op, and
  `ple_app` can read `ple_api.ple_migration_state` and cannot read `public._sqlx_migrations`; the
  layout pytest passes.
- Parallel-plan ready: no. Runtime and first file are one slice.

### Milestone: M2 accounts and sessions

- Depends on: M1.
- Deliverables: `0002_accounts_and_sessions.sql` (account, Product Role, Account State Events,
  authenticated session, authentication emails, credentials, credential completion, account
  preferences with the exact IANA check); `0003_instructor_accounts_and_audit.sql` (Create,
  Deactivate, Reactivate Instructor Account, `instructor_account_creation_event`, Instructor
  Accounts readers); `tests/e2e/base_schema_accounts_oracle.sql`.
- Workstreams: WS-BASE.
- Entry criteria: M1 exit.
- Exit criteria: fresh and no-op apply of `0001`-`0003`; the oracle proves an Account State Event
  drives Active/Deactivated, a Deactivated Account's sessions are revoked, a Student preference is
  defaulted once from the importing Instructor and thereafter owned by the Student, and an invalid
  IANA name is refused by SQL.
- Parallel-plan ready: no.

### Milestone: M3 Question Library

- Depends on: M2 for Account foreign keys.
- Deliverables: `0004_question_library_lineage.sql` (published question, question revision,
  acceptance, authorship, license, citation, ownership events, publication events,
  `published_question_availability_event` keyed by `question_id`, published metadata, fork source);
  `0005_question_source_bindings_and_objects.sql` (object records, deliveries, storage checks,
  cleanup manifests, revision source binding, asset publication and delivery);
  `0007_question_publication_operations.sql` (new-lineage publication transaction, draft source
  resolution, `published_question_summary` deriving Latest Question Revision and current lineage
  availability, browse, lookup, detail readers); `tests/e2e/base_schema_question_library_oracle.sql`.
- Workstreams: WS-BASE.
- Entry criteria: M2 exit.
- Exit criteria: fresh and no-op apply through `0007` (with `0006` absent, `0007` depends only on
  `0004`-`0005`; if the resolver needs draft tables, `0007` moves after `0006` in M4 and the exit
  check runs there); the oracle proves initial publication starts Available, an archive event hides
  the lineage from the summary while the exact revision still resolves, and a later publication does
  not reset availability.
- Parallel-plan ready: no.

### Milestone: M4 authoring and stewardship

- Depends on: M3 for Question lineage foreign keys.
- Deliverables: `0006_authoring_workspace.sql` (workspace, collaborator events, draft question, draft
  metadata, draft source binding, workspace imports, QTI import evidence, bind source, list, load,
  create, save at exact Edit Number); `0008_question_stewardship.sql` (`question_change_proposal`
  as one current row with `question_change_proposal_edit_number`, `question_change_event` copying
  the acted-on Edit Number, base revision, payload checksum, validation, and impact; forced
  correction manifests; folders; saved searches); `tests/e2e/base_schema_authoring_oracle.sql`.
- Workstreams: WS-BASE. Runs beside M5.
- Entry criteria: M3 exit.
- Exit criteria: fresh and no-op apply; the oracle proves a stale Draft Edit Number is refused, a
  proposal edit at a stale Edit Number is refused, and a `question_change_event` row reconstructs
  the acted-on facts after the proposal row changes.
- Parallel-plan ready: yes, beside M5, with M4 owning `0006` and `0008` and M5 owning `0009`.

### Milestone: M5 Blueprint Courses

- Depends on: M3 for Question Revision pins.
- Deliverables: `0009_blueprint_courses.sql` (blueprint course lineage; `blueprint_draft` with
  `blueprint_draft_edit_number`, v3 content, and `last_published_edit_number`;
  `blueprint_course_revision`; publication and collaborator events;
  `blueprint_course_availability_event` keyed by reference; fork and creation reservations; create
  as an atomic lineage-plus-draft-plus-Revision 1; draft load and save; publish; archive; restore;
  list and load for Active Instructors); `tests/e2e/base_schema_blueprint_oracle.sql`.
- Workstreams: WS-BASE.
- Entry criteria: M3 exit.
- Exit criteria: fresh and no-op apply; the oracle proves a draft save creates no revision, a
  publish at a stale draft ETag is refused, a publish with no edit since the last publish is
  refused, one publish creates exactly one revision and one event, and an archived lineage still
  resolves its exact revisions.
- Parallel-plan ready: yes, beside M4.

### Milestone: M6 Course Instances

- Depends on: M2 and M5.
- Deliverables: `0010_course_instances.sql` (`course_instance` with `term_starts_on`,
  `term_ends_on` NOT NULL and an ordered CHECK, short and long names, theme, banner pointer;
  `course_origin`; creation reservation and event; membership events; teaching team; course summary
  and C-reference readers; theme read and replace); `0011_course_roster_and_invitations.sql`
  (invitations, email rule, student record, roster import, claim, revoke, export);
  `0012_course_banner_and_profile_thumbnail.sql` (banner uploads, renditions, pointer, thumbnail
  pointer, put and delete work); `tests/e2e/base_schema_course_oracle.sql`.
- Workstreams: WS-BASE.
- Entry criteria: M2 and M5 exit.
- Exit criteria: fresh and no-op apply; the oracle proves Course creation from an Archived Blueprint
  is refused while an existing Course Origin still resolves, term dates live on the Course row with
  no schedule table in the catalog, and a revoked membership loses Course reads.
- Parallel-plan ready: no.

### Milestone: M7 Assignments

- Depends on: M6 for Course foreign keys; M3 for Question Revision pins.
- Deliverables: `0013_assignments.sql` (`assignment` current row with every policy, disclosure, and
  schedule column and `assignment_status` governed by `assignment_edit_number`;
  `assignment_entry`, `assignment_fixed_question` pinning `(question_id, revision_number)`,
  `assignment_question_pool`, `assignment_question_pool_item`; the edit trigger covering status;
  create, save, inline save, list, preview, release validation requiring a positive time limit,
  release as `unreleased -> released` plus one Edit Number advance and no snapshot; unrelease impact
  and unrelease action functions (deletion graph completed in M8 and M9, see WP-ASG3); Due Soon
  reader); `tests/e2e/base_schema_assignment_oracle.sql`.
- Workstreams: WS-BASE.
- Entry criteria: M6 exit.
- Exit criteria: fresh and no-op apply; the oracle proves release creates no new table row outside
  `assignment` and advances the Edit Number by one, a Released save that fails release validation
  is refused atomically, a reused pin keeps its revision after a newer publication, a newly added
  pin to an Archived lineage is refused, and `ple_student` cannot read `assignment_entry`.
- Parallel-plan ready: no.

### Milestone: M8 Student work

- Depends on: M7.
- Deliverables: `0014_assignment_attempts_and_issued_questions.sql` (`assignment_attempt` with the
  copied fact columns listed under Data inventory, `issued_question` with copied entry facts and no
  FK to a mutable pool item, `question_pool_selection` and selected items copying facts, `R-n`
  reference, start with supplied order, access decision, context, progress);
  `0015_question_attempts_and_submissions.sql` (question attempts, presentation bindings, saved
  responses, submissions, finalize); `0016_grading_and_completion.sql` (grading results, receipts,
  grading state, recovery, completion trigger reading Attempt-copied completion rule, feedback
  release reads, history readers, landing score disclosure, source reproduction);
  `tests/e2e/base_schema_student_work_oracle.sql`.
- Workstreams: WS-BASE.
- Entry criteria: M7 exit.
- Exit criteria: fresh and no-op apply; the oracle starts an Attempt, edits the Assignment's due,
  time limit, disclosure, and one pin, then proves the started Attempt's access, timer, feedback
  release, and issued revision are unchanged while a second Attempt uses the edits; `ple_app` cannot
  update `assignment_attempt` copied columns; `ple_student` cannot read `assignment`.
- Parallel-plan ready: no.

### Milestone: M9 grading, analysis, jobs, retention, iMathAS

- Depends on: M8.
- Deliverables: `0017_gradebook_and_analysis.sql` (grades, events, gradebook reads, analysis jobs,
  question analysis); `0018_question_statistics.sql` (revision and choice statistics, observation
  recording, `ple_private.recompute_question_revision_statistics(question_id, revision_number)`
  rebuilding both tables from retained submissions and grading results);
  `0019_jobs_leases_and_retention.sql` (typed jobs and leases; `course_retention_plan` current row
  per Course with `notice_after_days 30`, `archive_after_days 100`, `delete_after_days 365`, and
  `course_retention_plan_edit_number`; retention events and jobs copying effective values;
  authorization checks); `0020_imathas_question_backend_session.sql` (the 929-line family, split
  into `0020a` is not allowed, so if it exceeds 900 lines after edits it becomes `0020` and `0021`
  and later files shift by one); `tests/e2e/base_schema_operations_oracle.sql`.
- Workstreams: WS-BASE. Three doers: gradebook plus statistics, jobs plus retention, iMathAS.
- Entry criteria: M8 exit.
- Exit criteria: fresh and no-op apply; the oracle proves recompute reproduces the incrementally
  accumulated statistics for a seeded Question Revision exactly, a retention job row copies the
  effective days and ignores a later plan edit, and the iMathAS session lifecycle proof from the
  current `imathas_question_backend_session_postgres_oracle.sql` passes after its fixture inserts
  are rewritten to current Assignment rows.
- Parallel-plan ready: yes, three doers by file, integrator renumbers at M10.

### Milestone: M10 closure and renumber

- Depends on: M9.
- Deliverables: the final `NNNN_forced_rls_and_acl_closure.sql` (`ENABLE` and `FORCE ROW LEVEL
  SECURITY` on every `ple_data`, `ple_private`, `ple_audit` relation not already forced, final
  grants and revokes, `ple_api.assert_baseline_security_audit()`, baseline witness); consecutive
  renumbering `1..N`; deletion of every old oracle named under Test and verification strategy; the
  acceptance script's default oracle list set to the family oracles.
- Workstreams: WS-CLOSE.
- Entry criteria: M9 exit.
- Exit criteria: `bash tests/e2e/e2e_postgres_migration_acceptance.sh` passes end to end: fresh,
  no-op, verify, every family oracle, `assert_baseline_security_audit()`, restricted logins;
  `git ls-files tests/e2e | grep -c revision` reports zero; the layout pytest passes.
- Parallel-plan ready: no.

### Milestone: M11 cutover

- Depends on: M10.
- Deliverables: `git rm -r schemas/migrations`; the three stale override lines removed;
  `learning-data-access` call sites adapted to changed function signatures (release returns
  `assignment_edit_number`; course creation binds no schedule revision id; attempt start binds no
  revision id); server DTOs and TypeScript decoders adapted; the release UI message
  `Assignment released.`; Playwright and shell journeys updated; `./launchers/all_test.sh` green.
- Workstreams: WS-CUT.
- Entry criteria: M10 exit.
- Exit criteria: `./check_rust.sh`, `./check_codebase.sh`, `source source_me.sh && pytest tests/`,
  `python3 local_stack.py acceptance`, and `./devel/run_playwright_tests.sh` all exit 0 on a fresh
  disposable stack; `grep -rn "schemas/migrations" --include=*.rs --include=*.py --include=*.sh
  --include=*.yaml --include=*.toml .` outside `docs/archive/` and changelog archives reports zero.
- Parallel-plan ready: no. One integrator owns the switch.

### Milestone: M12 Rust and TypeScript cleanup

- Depends on: M11.
- Deliverables: removal of `AssignmentRevisionNumber`, `AssignmentRevisionReference`,
  `BoundedAssignmentRevisionReferences`, `CourseInstanceSnapshot`, `CourseScheduleRevisionNumber`,
  `CourseScheduleRevisionReference`, `SuccessorAssignmentRevisionRequired`, grading-operation
  `assignment_revision`, `AssignmentAttempt.assignment_revision`, and their decoders and Node tests;
  `assignmentRevisionEtag` renamed `assignmentEditEtag`; `AssignmentAttempt` and `IssuedQuestion`
  exposing the copied facts; regenerated `generated/api/`.
- Workstreams: WS-RUST.
- Entry criteria: M11 exit.
- Exit criteria: `./check_rust.sh` and `./check_codebase.sh` pass; `grep -rn "Revision" crates
  src generated tests --include=*.rs --include=*.ts --include=*.tsx --include=*.mjs` matches only
  Question Revision, Blueprint Revision, and their references, recorded as a reviewed list.
- Parallel-plan ready: yes, Rust and TypeScript as two doers after the Rust types settle.

### Milestone: M13 Published Question availability actions

- Depends on: M12.
- Deliverables: `POST /api/questions/by-id/{question_id}/archive` and `/restore` (owner only,
  `If-Match` on the current summary ETag, strict `{ "questionId": "AAA-BBBB" }` confirmation on
  archive, no confirmation on restore); Danger Zone section on the owner's Published Question page
  with the shared-availability consequence; restore as an ordinary availability control; picker and
  search restricted to Available lineages; redacted audit row.
- Workstreams: WS-PQ.
- Entry criteria: M12 exit.
- Exit criteria: a connected e2e proves archive, restore, stale ETag `412`, foreign owner concealed
  `404`, wrong confirmation `409`, browse filtering, and an existing Assignment pin resolving after
  archive; axe reports no serious or critical finding on the Danger Zone surface.
- Parallel-plan ready: yes, beside M14 and M16.

### Milestone: M14 Blueprint Draft and availability actions

- Depends on: M12.
- Deliverables: `GET`/`PUT /api/course-blueprints/{reference}/draft` (`If-Match` on the Draft Edit
  Number), `POST .../publish`, `POST .../archive` (owner only, strict `{ "reference": "BP-..." }`
  confirmation), `POST .../restore`; the Blueprint editing page saving to the draft and publishing
  explicitly with a visible "unpublished changes" state; Danger Zone section; Course creation
  refusing an Archived Blueprint.
- Workstreams: WS-BP.
- Entry criteria: M12 exit.
- Exit criteria: a connected e2e proves draft save without publication, stale draft `412`, duplicate
  publish `409`, one revision per publish, archive and restore, and refused Course creation from an
  Archived Blueprint; axe passes on the changed page.
- Parallel-plan ready: yes.

### Milestone: M15 Assignment Unrelease

- Depends on: M12; M9 for the recompute function.
- Deliverables: `GET /api/.../assignments/{reference}/unrelease-impact` returning title, Edit
  Number, status, and counts of Attempts, Issued Questions, submissions, and grading results;
  `POST .../unrelease` with `If-Match` and strict `{ "assignmentTitle": "exact title" }`; the
  private-owner SECURITY DEFINER deletion function walking the complete Student Work graph inside
  one transaction, then recomputing statistics for every affected Question Revision, then setting
  `unreleased` with one Edit Number advance; a redacted `ple_audit` event with aggregate counts; the
  Danger Zone section in the Assignment Properties Editor with typed-title confirmation.
- Workstreams: WS-UNR.
- Entry criteria: M12 exit.
- Exit criteria: a connected e2e seeds two Students with Attempts, submissions, grading, saved
  responses, presentation bindings, and an iMathAS session on one Assignment, runs Unrelease, and
  proves zero Student-work rows remain for that Assignment, other Assignments' rows are untouched,
  the affected Question Revision Statistics equal a fresh recompute, a concurrent Attempt start
  during Unrelease is refused, a wrong title returns `409` and changes nothing, and re-release then
  a new Attempt succeeds; security review finds no unresolved blocker.
- Parallel-plan ready: no. One transaction boundary, one owner.

### Milestone: M16 Student time zone

- Depends on: M12.
- Deliverables: `GET`/`PUT /api/student-profile` returning and replacing `{ "timeZone": "<IANA>" }`
  with `If-Match`; `ple_api` read and replace functions authorized for the Student's own Account;
  `src/pages/student_profile_page.tsx` at the Student `/profile` route reached from the Student
  Ribbon's Profile control, stating that changing the zone re-renders deadlines without moving them.
- Workstreams: WS-STZ.
- Entry criteria: M12 exit.
- Exit criteria: a connected e2e proves a Student in `America/Chicago` and one in `Europe/Berlin`
  see the same due instant as different wall clocks, a `PUT` with an invalid name is refused, and
  the stored instant is unchanged after the preference changes; keyboard-only operation; axe passes.
- Parallel-plan ready: yes.

### Milestone: M17 documentation close-out

- Depends on: M13, M14, M15, M16.
- Deliverables: rewritten `docs/DATABASE_STRUCTURE.md` (ownership map keyed by base file family,
  new relational chains, layout and freeze rule), `docs/DATABASE_AUTHORIZATION.md` ("Schema layout
  and update rule" replacing the allocation registry), `docs/ROADMAP.md` (baseline paragraph, D2 and
  D3 wording, durable policy naming both folders), `docs/CONTRACTS.md`, `docs/DESIGN_DECISIONS.md`,
  `docs/CODE_ARCHITECTURE.md`, `docs/FILE_STRUCTURE.md`, `docs/TEST_EVIDENCE_MODEL.md`,
  `docs/DEVELOPMENT.md`, `docs/LOCAL_STACK_ARCHITECTURE.md`, `docs/LOCAL_STACK_OPERATIONS.md`,
  `docs/NAMING_CONVENTIONS.md`, `docs/USER_ROLES.md`, `docs/RETENTION_POLICY.md`, `docs/TODO.md`
  with every migration-filename citation replaced by a base-file citation; `revision_concerns.txt`
  moved by `git mv` to `docs/archive/` with a closing paragraph linking the evidence; regenerated
  Graphify output; one `docs/CHANGELOG.md` day block.
- Workstreams: WS-EVI.
- Entry criteria: every capability milestone exit.
- Exit criteria: `grep -rn "2026[01][0-9][0-9][0-9][0-9][0-9]_" docs --include=*.md` outside
  `docs/archive/` and `docs/CHANGELOG*.md` reports zero; `tests/test_markdown_links.py`,
  `tests/test_ascii_compliance.py`, and `tests/test_whitespace.py` pass; `graphify query "what
  creates assignment_revision"` returns no live node.
- Parallel-plan ready: yes, per document.

## Workstream breakdown

### Workstream: WS-DOC

- Goal: one plan of record and the settled decisions written before code.
- Owner: `architect`.
- Work packages: WP-DOC1.
- Needs: this plan.
- Provides: the layout, evidence, and availability decisions every later package cites.
- Review boundary, when modifying the repository: documentation only.

### Workstream: WS-LAY

- Goal: a two-folder schema layout embedded by one generated migrator, proven on one file.
- Owner: `expert_coder` (Rust).
- Work packages: WP-LAY1, WP-LAY2, WP-LAY3.
- Needs: WP-DOC1.
- Provides: the runtime every base-file milestone applies through.
- Review boundary, when modifying the repository: `reviewer` on `build.rs`, `migrations.rs`, the
  applier, and the container files.

### Workstream: WS-BASE

- Goal: the complete base schema in its final shape, one family per file, one oracle per family.
- Owner: `expert_coder` (PostgreSQL), with the `postgresql-expert` skill.
- Work packages: WP-B02 through WP-B09.
- Needs: WS-LAY runtime; the Data inventory below.
- Provides: every table, function, and policy the cutover and capabilities use.
- Review boundary, when modifying the repository: `reviewer` per family; `architect` on WP-B07 and
  WP-B08 because they own the Attempt evidence boundary.

### Workstream: WS-CLOSE

- Goal: RLS closure, consecutive numbering, and the old-oracle purge.
- Owner: `expert_coder`.
- Work packages: WP-CL1, WP-CL2.
- Needs: every WS-BASE file.
- Provides: the complete acceptance lane on the base.
- Review boundary, when modifying the repository: `reviewer` on the deleted oracle list.

### Workstream: WS-CUT

- Goal: the live demo and every lane running on the base with no old chain present.
- Owner: `integrator`.
- Work packages: WP-CUT1, WP-CUT2, WP-CUT3.
- Needs: WS-CLOSE exit.
- Provides: a green tree for the capability milestones.
- Review boundary, when modifying the repository: full lane receipts.

### Workstream: WS-RUST

- Goal: no retired revision type, decoder, or generated file survives.
- Owner: `expert_coder` (Rust) and `coder` (TypeScript).
- Work packages: WP-RS1, WP-RS2.
- Needs: WS-CUT exit.
- Provides: the domain types the capability milestones extend.
- Review boundary, when modifying the repository: `reviewer` semantic scan.

### Workstream: WS-PQ, WS-BP, WS-UNR, WS-STZ

- Goal: one capability each, as named in M13 through M16.
- Owner: `expert_coder` for WS-UNR; `coder` with the `ui-ux-engineer` skill for the three others.
- Work packages: WP-PQ1, WP-PQ2; WP-BP1, WP-BP2; WP-UNR1, WP-UNR2, WP-UNR3; WP-STZ1.
- Needs: WS-RUST exit.
- Provides: the Danger Zone triad and Student zone ownership.
- Review boundary, when modifying the repository: `reviewer`; security review on WS-PQ archive and
  WS-UNR.

### Workstream: WS-EVI

- Goal: current documentation, evidence record, Graphify regeneration, closure.
- Owner: `maintainer`.
- Work packages: WP-EVI1, WP-EVI2.
- Needs: every capability exit.
- Provides: the acceptance record.
- Review boundary, when modifying the repository: markdown gates.

## Work packages

### Work package: WP-DOC1 publish the plan and record decisions

- Owner: `architect`.
- Touch points: `docs/active_plans/active/base_schema_and_revision_unwind_2026_09.md`,
  `docs/archive/backend_terminology_reconciliation_2026_09.md`, `docs/DESIGN_DECISIONS.md`.
- Depends on: none.
- Acceptance criteria: the four decisions under Resolved decisions appear as `###` entries with
  Decision, Why, Consequence, Owner; the archived plan's first line reads "Superseded by
  base_schema_and_revision_unwind_2026_09.md"; markdown link test passes.
- Evidence or review, when useful: `tests/test_markdown_links.py`.
- Obvious follow-ons: WP-LAY1.

### Work package: WP-LAY1 generate the two-folder embedded migrator

- Owner: `expert_coder`.
- Touch points: `crates/learning-data-access/build.rs`,
  `crates/learning-data-access/src/postgres/migrations.rs`, `crates/learning-data-access/Cargo.toml`.
- Depends on: WP-DOC1.
- Acceptance criteria: `build.rs` scans `../../schemas/base_schema` and
  `../../schemas/schema_updates`, sorts by parsed leading integer, refuses duplicate versions and any
  base version that is not consecutive from 1, refuses an update version at or below the largest base
  version, and writes `$OUT_DIR/embedded_schema.rs` containing one
  `sqlx::migrate::Migration::new(version, description, MigrationType::Simple, include_str!(path)
  .into_sql_str(), false)` per file plus `rerun-if-changed` for both folders; `migrations.rs`
  replaces `sqlx::migrate!` with a `LazyLock<Migrator>` built by `Migrator::with_migrations`;
  `migration_status_from_directory` keeps `Migrator::new(directory)` for `--migrations-dir` and
  accepts either folder; every existing unit test in `migrations.rs` passes; `cargo test -p
  learning-data-access` passes with `0001` as the only file.
- Evidence or review, when useful: `reviewer` on the generator's refusal paths; a compile with a
  deliberately duplicated version failing the build.
- Obvious follow-ons: WP-LAY2.

### Work package: WP-LAY2 author the principals file and the updates README

- Owner: `expert_coder`.
- Touch points: `schemas/base_schema/0001_principals_and_schemas.sql`,
  `schemas/schema_updates/README.md`.
- Depends on: WP-LAY1.
- Acceptance criteria: `0001` assumes the bootstrap roles (`ple_database_owner`, `ple_migrator`,
  `ple_public_asset_publisher`, `ple_native_ple_grading_worker`, `ple_webwork_grading_worker`)
  exist; creates `ple_data_owner`, `ple_private_owner`, `ple_audit_owner`, `ple_api_owner`,
  `ple_app`, `ple_auth`, `ple_student`, `ple_imathas_question_backend_grading_worker`; creates the
  four schemas with owner authorization and default-deny default privileges; grants `SELECT` on
  `public._sqlx_migrations` to `ple_api_owner` and creates `ple_api.ple_migration_state`; revokes
  `CREATE ON SCHEMA public` from `ple_migrator` as its last statement so `apply_migrations`' ledger
  fallback keeps its reason; opens with one `DO` guard requiring `current_user = 'ple_migrator'`
  and carries no version literal; drops the exact membership-count `DO` blocks of `2026082901`
  (role verification belongs to the accounts oracle, not to DDL); stays under 400 lines. The README
  states in under 20 lines: the folder holds forward migrations added after the v1 launch tag, named
  `YYYYMMDDNN_<description>.sql`, one per bounded work item, never edited after acceptance, applied
  after every base file, and links `docs/DATABASE_STRUCTURE.md`.
- Evidence or review, when useful: fresh apply on a disposable database; `ple_app` reads
  `ple_migration_state` and is denied `public._sqlx_migrations`.
- Obvious follow-ons: WP-LAY3.

### Work package: WP-LAY3 point the applier, lanes, and container at the layout

- Owner: `expert_coder`.
- Touch points: `crates/project-tools/src/database_postgres_migration_acceptance.rs`,
  `crates/project-tools/src/database.rs`, `tests/e2e/e2e_postgres_migration_acceptance.sh`,
  `containers/Containerfile.api`, `.containerignore`, `tests/test_schema_layout_contract.py`.
- Depends on: WP-LAY2.
- Acceptance criteria: the applier iterates the embedded migrator's list instead of reading
  `schemas/migrations`, keeps the statement splitter and per-file transaction, and requires first
  version `1`; `--migrations-dir` validation accepts `schemas/base_schema` or
  `schemas/schema_updates`; the acceptance script asserts version `1` pending before the first apply,
  gains `--oracles <comma-separated paths>` defaulting to the family oracle list, and keeps the
  fresh, no-op, verify, and restricted-login steps; `Containerfile.api` copies `schemas/base_schema`
  and `schemas/schema_updates`; `.containerignore` whitelists both; the pytest asserts base files
  are `NNNN_` zero-padded, consecutive from 1, and update files are ten-digit and greater than every
  base version, using the repository's `file_utils.get_repo_root()` and no PostgreSQL.
- Evidence or review, when useful: `bash tests/e2e/e2e_postgres_migration_acceptance.sh --oracles
  ""` passing on `0001` alone; `./check_rust.sh`; `pytest tests/test_schema_layout_contract.py`.
- Obvious follow-ons: WP-B02.

### Work package: WP-B02 accounts, sessions, and Instructor Accounts

- Owner: `expert_coder`.
- Touch points: `schemas/base_schema/0002_accounts_and_sessions.sql`,
  `0003_instructor_accounts_and_audit.sql`, `tests/e2e/base_schema_accounts_oracle.sql`.
- Depends on: WP-LAY3.
- Acceptance criteria: every table, function, trigger, and policy from the current
  `2026082902`-`2026082904`, `2026082906`, `2026082933`-`2026082934`, `2026090401`,
  `2026090610`, `2026091001`, and `2026091014` lands in its final shape, with later `CREATE OR
  REPLACE` bodies as the only definition and every `ALTER TABLE ... ADD COLUMN` folded into the
  `CREATE TABLE`; no version literal appears in any message; each file stays under 900 lines; the
  oracle covers the four behaviors named in M2.
- Evidence or review, when useful: fresh and no-op apply; oracle; `reviewer` diff read against the
  source migrations for dropped grants.
- Obvious follow-ons: WP-B03.

### Work package: WP-B03 Question Library, sources, objects, and publication operations

- Owner: `expert_coder`.
- Touch points: `0004_question_library_lineage.sql`, `0005_question_source_bindings_and_objects.sql`,
  `0007_question_publication_operations.sql`, `tests/e2e/base_schema_question_library_oracle.sql`.
- Depends on: WP-B02.
- Acceptance criteria: `question_revision_availability_event` is replaced by
  `ple_data.published_question_availability_event (question_id, availability, occurred_at,
  acting_account_id)` with an immutability trigger and a current-availability index;
  `published_question_summary` derives Latest Question Revision from acceptance evidence and current
  availability from the lineage event; browse, search, picker, and detail readers filter on
  lineage availability; `2026090101`, `2026090301`-`2026090304`, `2026090601`, `2026090611`-
  `2026090613`, `2026082907`-`2026082910` (published parts), `2026082936`, `2026082940`,
  `2026082943`-`2026082945` fold in; the oracle covers the three behaviors named in M3.
- Evidence or review, when useful: fresh and no-op apply; oracle; the existing
  `tests/e2e/question_records.sql`, `question_publication_operation.sql`,
  `question_publication_credit_catalog.sql`, `question_asset_publication_catalog.sql` rewritten to
  the lineage event and retained only where they assert publication contracts.
- Obvious follow-ons: WP-B04, WP-B05.

### Work package: WP-B04 Authoring Workspace and stewardship

- Owner: `expert_coder`.
- Touch points: `0006_authoring_workspace.sql`, `0008_question_stewardship.sql`,
  `tests/e2e/base_schema_authoring_oracle.sql`.
- Depends on: WP-B03.
- Acceptance criteria: `question_change_proposal_revision` is replaced by
  `ple_data.question_change_proposal` with `proposal_id`, `question_id`, `base_revision_number`,
  proposer account, proposed source object reference and checksum, `question_publication_validation`,
  semantic and grading impact, `question_change_proposal_edit_number`, `updated_at`, and an edit
  trigger requiring the exact prior Edit Number; `ple_audit.question_change_event` copies
  `proposal_edit_number`, base revision, payload checksum, validation, impact, and outcome and drops
  its FK to a revision row; forced correction manifests reference the proposal id and copied Edit
  Number; `2026082909`, `2026082910` (draft parts), `2026082912`, `2026082924`, `2026082942`,
  `2026090304`, `2026090602` fold in; the oracle covers the three behaviors named in M4.
- Evidence or review, when useful: fresh and no-op apply; oracle.
- Obvious follow-ons: WP-B06.

### Work package: WP-B05 Blueprint Courses

- Owner: `expert_coder`.
- Touch points: `0009_blueprint_courses.sql`, `tests/e2e/base_schema_blueprint_oracle.sql`.
- Depends on: WP-B03.
- Acceptance criteria: `ple_data.blueprint_draft (blueprint_course_reference_number PK,
  draft_content jsonb, draft_content_encoding_version smallint DEFAULT 3, blueprint_draft_edit_number,
  last_published_edit_number, updated_at)` with an edit trigger; create is one transaction writing
  the lineage, the draft at Edit Number 1, Revision 1, and its publication event; `publish` requires
  the supplied Edit Number to equal the current one and to exceed `last_published_edit_number`,
  creates one revision and one event, and sets `last_published_edit_number`;
  `blueprint_revision_availability_event` is replaced by
  `ple_data.blueprint_course_availability_event` keyed by reference; archive and restore functions
  require the Blueprint Course Owner; list and load expose only Available lineages to non-owners
  while exact revisions still resolve for Course Origins; `2026082911`, `2026082935`, `2026090603`,
  `2026091018` fold in; the oracle covers the five behaviors named in M5.
- Evidence or review, when useful: fresh and no-op apply; oracle.
- Obvious follow-ons: WP-B06.

### Work package: WP-B06 Course Instances, roster, banner, thumbnail

- Owner: `expert_coder`.
- Touch points: `0010_course_instances.sql`, `0011_course_roster_and_invitations.sql`,
  `0012_course_banner_and_profile_thumbnail.sql`, `tests/e2e/base_schema_course_oracle.sql`.
- Depends on: WP-B02, WP-B05.
- Acceptance criteria: `course_schedule_revision` does not exist; `course_instance` carries
  `term_starts_on date NOT NULL`, `term_ends_on date NOT NULL`, `CHECK (term_starts_on <=
  term_ends_on)`; `create_live_demo_course_instance` takes term dates and no schedule revision id;
  `list_live_demo_course_instances`, `load_live_demo_course_instance`, `read_course_summary`, and
  `load_live_demo_assignment_schedule_context` read the Course row; `2026082913`-`2026082915`,
  `2026090604`-`2026090605`, `2026090609`, `2026090902`-`2026090904`, `2026091002`-`2026091003`,
  `2026091013`, `2026091015` fold in; the oracle covers the three behaviors named in M6.
- Evidence or review, when useful: fresh and no-op apply; oracle; `reviewer` confirms no
  `revision_number = 1` literal survives in Course readers.
- Obvious follow-ons: WP-B07.

### Work package: WP-B07 Assignments as current content

- Owner: `expert_coder`.
- Touch points: `0013_assignments.sql`, `tests/e2e/base_schema_assignment_oracle.sql`.
- Depends on: WP-B06.
- Acceptance criteria: `assignment` keeps every current column except
  `released_assignment_revision_id` and `live_demo_question_selection_version`; `ple_private.
  live_demo_assignment_question` is replaced by `ple_data.assignment_entry (assignment_entry_id,
  assignment_id, entry_position, entry_kind, point_value, scoring_rule, availability,
  question_attempt_limit, question_attempt_time_limit_seconds)`, `ple_data.assignment_fixed_question
  (assignment_entry_id PK, question_id, revision_number)` with FK to `question_revision`,
  `ple_data.assignment_question_pool (assignment_entry_id PK, selection_count,
  selected_question_order)`, and `ple_data.assignment_question_pool_item (pool_item_id,
  assignment_entry_id, item_position, question_id, revision_number, availability)`; the edit
  trigger compares the whole row including `assignment_status` and requires exactly one Edit Number
  advance per accepted change; `save_live_demo_assignment` replaces entries in the same transaction,
  keeps an existing pin's revision when the same `question_id` is resubmitted, and resolves a newly
  introduced `question_id` to its Latest Question Revision only when the lineage is Available;
  `release_live_demo_assignment` validates the current row, sets `released`, advances the Edit
  Number, and returns `assignment_edit_number`; `save_live_demo_assignment_inline` and released saves
  run release validation when status is `released`; `list_assignments_due_soon` and
  `list_course_assignments` read current rows; `2026082916` (assignment parts), `2026082937`
  (assignment parts), `2026090606`, `2026090701`, `2026091008`, `2026091017`, `2026091019`-
  `2026091020`, `2026091022`-`2026091023` (assignment parts), `2026091028` fold in; the oracle
  covers the five behaviors named in M7.
- Evidence or review, when useful: fresh and no-op apply; oracle; `architect` on the entry model.
- Obvious follow-ons: WP-B08.

### Work package: WP-B08 Attempts, Issued Questions, submissions, grading, completion

- Owner: `expert_coder`.
- Touch points: `0014_assignment_attempts_and_issued_questions.sql`,
  `0015_question_attempts_and_submissions.sql`, `0016_grading_and_completion.sql`,
  `tests/e2e/base_schema_student_work_oracle.sql`.
- Depends on: WP-B07.
- Acceptance criteria: `assignment_attempt` has no `assignment_revision_id`; it carries the copied
  columns `assignment_title`, `assignment_instructions`, `available_at`, `due_at`, `closes_at`,
  `late_work_rule`, `assignment_attempt_time_limit_seconds`, `assignment_question_order_rule`,
  `question_pool_reuse_rule`, `question_variation_rule`, `assignment_completion_rule`,
  `assignment_completion_score_threshold`, and the seven `feedback_*` columns, all protected by the
  existing immutability trigger; `issued_question` carries `assignment_entry_id`,
  `assignment_entry_position`, `question_id`, `revision_number`, `issued_position`, `point_value`,
  `scoring_rule`, `question_statistics_eligibility`, `question_attempt_limit`,
  `question_attempt_time_limit_seconds`, `question_pool_selection_id`, and `pool_item_position`,
  with FKs only to `assignment_attempt`, `question_revision`, and `question_pool_selection`;
  `question_pool_selected_item` copies `question_id`, `revision_number`, `item_position` and has no
  FK to a pool item; `start_assignment_attempt_with_supplied_order` locks the current `assignment`
  row `FOR SHARE`, refuses unless `released`, copies the facts, and issues from current entries; the
  `require_released_assignment_revision` trigger is replaced by a `SECURITY DEFINER` check that the
  Assignment is `released` at insert; every reader in `2026090607`-`2026090608`, `2026090612`,
  `2026090614`-`2026090620`, `2026090802`, `2026090901`, `2026091010`-`2026091012`, `2026091016`,
  `2026091021`, `2026091023`-`2026091027`, `2026091029` reads Attempt-copied facts with no
  `CASE WHEN delivery_assignment_title IS NOT NULL` fallback; the deletion function
  `ple_private.delete_assignment_student_work(assignment_id)` walks Attempts, Issued Questions,
  pool selections, Question Attempts, presentation bindings, saved responses, submissions, grading
  results and receipts, completion rows, and iMathAS sessions in dependency order and returns the
  counts; the oracle covers the behaviors named in M8.
- Evidence or review, when useful: fresh and no-op apply; oracle; `architect` on the copied-fact
  list against the Data inventory.
- Obvious follow-ons: WP-B09.

### Work package: WP-B09 gradebook, analysis, statistics, jobs, retention, iMathAS

- Owner: three `expert_coder` doers by file; `integrator` reconciles.
- Touch points: `0017_gradebook_and_analysis.sql`, `0018_question_statistics.sql`,
  `0019_jobs_leases_and_retention.sql`, `0020_imathas_question_backend_session.sql`,
  `tests/e2e/base_schema_operations_oracle.sql`, rewritten
  `tests/e2e/imathas_question_backend_session_postgres_oracle.sql` fixtures.
- Depends on: WP-B08.
- Acceptance criteria: `read_live_demo_gradebook` and analysis rows count issued work from current
  entries and Attempt evidence, never from a revision; `recompute_question_revision_statistics`
  truncates and rebuilds the revision and choice statistics rows for one Question Revision from
  retained `question_submission` and `grading_result` rows and is `SECURITY DEFINER` owned by
  `ple_private_owner`; `course_retention_plan_revision` is replaced by
  `ple_private.course_retention_plan (course_id PK, notice_after_days 30, archive_after_days 100,
  delete_after_days 365, course_retention_plan_edit_number, updated_at)` created by Course creation;
  `ple_private.job` and `ple_audit.course_retention_event` copy `effective_notice_after_days`,
  `effective_archive_after_days`, `effective_delete_after_days` and drop the revision FK, unique,
  and check constraints, with `job_target_shape_is_exact` rewritten without the column; the iMathAS
  file stays under 999 lines or splits into two consecutive files; the oracle covers the three
  behaviors named in M9.
- Evidence or review, when useful: fresh and no-op apply; oracle; the rewritten iMathAS oracle.
- Obvious follow-ons: WP-CL1.

### Work package: WP-CL1 RLS closure, witness, renumbering

- Owner: `expert_coder`.
- Touch points: the final `NNNN_forced_rls_and_acl_closure.sql`; every base file's number.
- Depends on: WP-B09.
- Acceptance criteria: every `ple_data`, `ple_private`, `ple_audit` relation has forced RLS and
  PUBLIC holds no privilege, proven by `ple_api.assert_baseline_security_audit()`; `ple_app` and
  `ple_student` are procedure-only outside the explicit read views; base files are consecutive from
  `0001`; every file under 999 lines; `tests/source_file_line_limit_overrides.txt` holds no schema
  entry.
- Evidence or review, when useful: full acceptance lane; layout pytest.
- Obvious follow-ons: WP-CL2.

### Work package: WP-CL2 purge old-logic oracles

- Owner: `expert_coder`.
- Touch points: `tests/e2e/*.sql`, `tests/e2e/e2e_postgres_migration_acceptance.sh`,
  `tests/e2e/e2e_live_demo_assignment_attempt.sh`, `e2e_live_demo_assignment_release.sh`,
  `e2e_live_demo_course_seed.sh`.
- Depends on: WP-CL1.
- Acceptance criteria: the files listed under Test and verification strategy as deleted are
  removed with `git rm`; the files listed as rewritten assert only the contracts named there; no
  oracle asserts a column inventory, a policy name, or a role membership count; the shell journeys
  drop their psql assertions on `_m11_read` policies and `assignment_revision_fixed_question` pins;
  the acceptance script's default oracle list names only surviving files.
- Evidence or review, when useful: full acceptance lane green; `reviewer` confirms every deletion
  against the decision rule.
- Obvious follow-ons: WP-CUT1.

### Work package: WP-CUT1 delete the chain and adapt Rust call sites

- Owner: `integrator`.
- Touch points: `schemas/migrations/` (removed), `crates/learning-data-access/src/postgres/
  assignment_release.rs`, `course_instance.rs`, `assignment_attempt.rs`, `assignment_delivery.rs`,
  `crates/learning-data-access/src/assignment_release.rs`, `crates/server/src/assignment_release.rs`,
  `crates/learning-data-access/tests/*_postgres.rs` fixture inserts.
- Depends on: WP-CL2.
- Acceptance criteria: `git rm -r schemas/migrations` done; `release_live_assignment` reads
  `assignment_edit_number` into `ReleasedLiveAssignment.assignment_edit_number` and the route
  returns the updated Assignment ETag; `create_live_demo_course_instance` binds term dates only;
  `start_assignment_attempt` binds no revision id; Rust integration test fixtures insert current
  Assignment rows and entries; `./check_rust.sh` passes.
- Evidence or review, when useful: `cargo test -p learning-data-access --features postgres` on a
  disposable stack.
- Obvious follow-ons: WP-CUT2.

### Work package: WP-CUT2 adapt TypeScript decoders, UI copy, and browser journeys

- Owner: `coder`.
- Touch points: `src/api/assignment_release.ts`, `src/api/decoders/assignment_release.ts`,
  `src/api/http_client/assignment_release.ts`,
  `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx`,
  `tests/playwright/e2e_live_demo_assignment_release_browser.mjs`, `tests/test_http_client.mjs`.
- Depends on: WP-CUT1.
- Acceptance criteria: `ReleasedLiveAssignment` carries `assignmentEditNumber`; the policies page
  says `Assignment released.`; the Playwright journey waits for that text; `./check_codebase.sh`
  passes.
- Evidence or review, when useful: `./check_codebase.sh`.
- Obvious follow-ons: WP-CUT3.

### Work package: WP-CUT3 run every lane on a fresh stack

- Owner: `integrator`.
- Touch points: none new; receipts.
- Depends on: WP-CUT2.
- Acceptance criteria: `./launchers/all_test.sh` exits 0 after `python3 local_stack.py` recreates
  the disposable stack; `./devel/run_playwright_tests.sh` exits 0; the grep named in M11 exit
  criteria reports zero.
- Evidence or review, when useful: lane logs under the scratchpad, cited in the changelog.
- Obvious follow-ons: WP-RS1.

### Work package: WP-RS1 remove retired Rust revision types

- Owner: `expert_coder`.
- Touch points: `crates/question_model/src/assignment/revision.rs`,
  `crates/question_model/src/blueprint_operations.rs`,
  `crates/question_model/src/blueprint_operations/contracts/course_instance.rs`,
  `course_instance_receipts.rs`, `crates/question_model/src/student_work.rs`,
  `crates/question_model/src/assignment_workspace.rs`, `crates/question_model/src/
  grading_operations.rs`, `crates/browser-api-contract/src/grading_operations.rs`,
  `crates/question_model/src/lib.rs`, `assignment.rs`.
- Depends on: WP-CUT3.
- Acceptance criteria: `AssignmentEditNumber` keeps its own error type; every type named in M12 is
  deleted with its tests; `AssignmentAttempt` exposes the copied facts and `IssuedQuestion` exposes
  its copied entry facts as typed fields; `CourseInstanceSnapshot`, shift-dates and apply-update
  receipts, and `Recalculation` are deleted rather than rewritten because no route consumes them;
  `cargo tools tsgen` regenerates `generated/api/` with no retired file remaining.
- Evidence or review, when useful: `./check_rust.sh`; `reviewer` semantic scan.
- Obvious follow-ons: WP-RS2.

### Work package: WP-RS2 remove retired TypeScript decoders and rename the ETag parameter

- Owner: `coder`.
- Touch points: `src/api/decoders/assignment_attempt.ts`, `src/api/decoders/assignment_workspace.ts`,
  `src/api/decoders/grading_operations.ts`, `src/api/http_client/request.ts`, `response.ts`,
  `error.ts`, `grading_operations.ts`, `src/api/client.ts`, `tests/test_http_client.mjs`,
  `tests/test_assignment_workspace_content_conflict_client.mjs`,
  `tests/test_grading_operations_client.mjs`, `tests/test_calculated_gradebook_client.mjs`.
- Depends on: WP-RS1.
- Acceptance criteria: no decoder requires `assignmentRevision`, `assignment_revision`, or a
  successor-revision 409 body; `assignmentRevisionEtag` is `assignmentEditEtag` everywhere; the four
  Node tests are deleted or rewritten to the current contract; `./check_codebase.sh` passes.
- Evidence or review, when useful: `./check_codebase.sh`.
- Obvious follow-ons: WP-PQ1, WP-BP1, WP-UNR1, WP-STZ1.

### Work package: WP-PQ1 Published Question archive and restore routes

- Owner: `coder`.
- Touch points: `crates/server/src/question_library.rs` (or the route module that owns
  `/api/questions`), `crates/learning-data-access/src/postgres/question_library.rs`,
  `src/api/http_client/question_library.ts`, decoders.
- Depends on: WP-RS2.
- Acceptance criteria: the two routes behave as M13 states; concealed `404` for a non-owner; the
  archive route writes a redacted `ple_audit` row; picker and search results contain only Available
  lineages.
- Evidence or review, when useful: connected e2e `tests/e2e/e2e_question_availability.sh`; security
  review on owner authority.
- Obvious follow-ons: WP-PQ2.

### Work package: WP-PQ2 Published Question Danger Zone surface

- Owner: `coder` with `ui-ux-engineer`.
- Touch points: the owner's Published Question page, `src/route_contract.ts`, shared Danger Zone
  component `src/components/danger_zone.tsx` created here and reused by WP-BP2 and WP-UNR3.
- Depends on: WP-PQ1.
- Acceptance criteria: a visibly separated Danger Zone with the consequence text, the exact target,
  typed confirmation, a dedicated confirm control, keyboard cancel and focus return, busy and error
  states; restore is an ordinary control outside the zone.
- Evidence or review, when useful: Playwright journey; axe; `image_evaluator` at 1280 by 800.
- Obvious follow-ons: none.

### Work package: WP-BP1 Blueprint Draft, publish, archive, restore routes

- Owner: `expert_coder`.
- Touch points: `crates/server/src/blueprint_course.rs`, `crates/learning-data-access/src/postgres/
  blueprint_course.rs`, `crates/question_model` draft types, generated TypeScript, decoders.
- Depends on: WP-RS2.
- Acceptance criteria: the routes behave as M14 states; the former revision-creating base `PUT` is
  removed; `GET` list and detail expose availability; Course creation refuses an Archived Blueprint
  with `409`.
- Evidence or review, when useful: connected e2e `tests/e2e/e2e_blueprint_draft.sh`.
- Obvious follow-ons: WP-BP2.

### Work package: WP-BP2 Blueprint editing page and Danger Zone

- Owner: `coder` with `ui-ux-engineer`.
- Touch points: the Blueprint editing page, `src/api/http_client/blueprint_course.ts`, Danger Zone
  component reuse.
- Depends on: WP-BP1, WP-PQ2.
- Acceptance criteria: save writes the draft; a visible unpublished-changes state; an explicit
  Publish action with its own confirmation; Danger Zone archive with typed reference; restore as an
  ordinary control.
- Evidence or review, when useful: Playwright journey; axe.
- Obvious follow-ons: none.

### Work package: WP-UNR1 Unrelease impact and action routes

- Owner: `expert_coder`.
- Touch points: `crates/server/src/assignment_release.rs`, `crates/learning-data-access/src/postgres/
  assignment_release.rs`, `ple_api.read_assignment_unrelease_impact` and
  `ple_api.unrelease_assignment` (authored in WP-B07, wired here), generated TypeScript.
- Depends on: WP-RS2.
- Acceptance criteria: `GET` returns the fields M15 names; `POST` requires `If-Match`, exact title,
  `released` status, and current Teaching Team authority, then calls one transaction that deletes
  the Student Work graph through `delete_assignment_student_work`, recomputes statistics for every
  affected Question Revision, sets `unreleased` with one Edit Number advance, and writes the redacted
  audit row; any failure leaves every row unchanged.
- Evidence or review, when useful: connected e2e `tests/e2e/e2e_assignment_unrelease.sh` covering
  the M15 scenarios; security review.
- Obvious follow-ons: WP-UNR2.

### Work package: WP-UNR2 deletion graph oracle

- Owner: `tester`.
- Touch points: `tests/e2e/e2e_assignment_unrelease.sh`, seeded SQL fixture.
- Depends on: WP-UNR1.
- Acceptance criteria: the fixture seeds every Student Work table the foreign-key walk from
  `assignment_attempt` reaches, including iMathAS session rows and presentation bindings; after
  Unrelease a `SELECT count(*)` per table for that Assignment is zero and per-table counts for a
  sibling Assignment are unchanged; the statistics equality check passes.
- Evidence or review, when useful: the script output.
- Obvious follow-ons: WP-UNR3.

### Work package: WP-UNR3 Unrelease Danger Zone in the Assignment Properties Editor

- Owner: `coder` with `ui-ux-engineer`.
- Touch points: `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx`, Danger Zone
  component reuse, `src/api/http_client/assignment_release.ts`.
- Depends on: WP-UNR1, WP-PQ2.
- Acceptance criteria: the zone shows current counts from the impact route, requires the exact
  title, disables confirm until it matches, shows busy and `412`/`409` recovery states, and returns
  the page to `unreleased` on success.
- Evidence or review, when useful: Playwright journey; axe; `image_evaluator`.
- Obvious follow-ons: none.

### Work package: WP-STZ1 Student time zone route and page

- Owner: `coder`.
- Touch points: `crates/server/src/student_profile.rs` (new), `crates/learning-data-access/src/
  postgres/student_profile.rs` (new), `ple_api.read_student_profile` and
  `replace_student_profile_time_zone` (authored in WP-B02), `src/pages/student_profile_page.tsx`,
  `src/route_contract.ts`, `src/routes.ts`, the Student Ribbon Profile control.
- Depends on: WP-RS2.
- Acceptance criteria: as M16 states; the page uses the existing IANA select pattern from the
  Instructor profile page and Student-facing language only.
- Evidence or review, when useful: connected e2e `tests/e2e/e2e_student_time_zone.sh`; Playwright
  keyboard journey; axe.
- Obvious follow-ons: none.

### Work package: WP-EVI1 rewrite database and policy documentation

- Owner: `maintainer`.
- Touch points: every document named in M17.
- Depends on: WP-PQ2, WP-BP2, WP-UNR3, WP-STZ1.
- Acceptance criteria: as M17 states; `docs/DATABASE_STRUCTURE.md` gains a "Schema layout" section
  naming both folders, the consecutive base numbering, the `YYYYMMDDNN` update numbering, the launch
  freeze, and the ownership map table keyed by base file; `docs/ROADMAP.md` lines 12-20 describe the
  base, and lines 64-69 no longer mention `--apply-migrations`.
- Evidence or review, when useful: the three markdown gates; the citation grep.
- Obvious follow-ons: WP-EVI2.

### Work package: WP-EVI2 changelog, Graphify, archive

- Owner: `maintainer`.
- Touch points: `docs/CHANGELOG.md`, `docs/archive/revision_concerns.txt`,
  `docs/active_plans/active/base_schema_and_revision_unwind_2026_09.md` (moved to `docs/archive/`
  at closure), `graphify-out/`.
- Depends on: WP-EVI1.
- Acceptance criteria: one day block with all six categories; Graphify regenerated and the M17 query
  check passing; both plan files archived by `git mv`.
- Evidence or review, when useful: `tests/test_markdown_links.py`.
- Obvious follow-ons: none.

## Acceptance criteria and gates

- Per-patch gate: `./check_rust.sh` when a crate changed; `./check_codebase.sh` when TypeScript
  changed; `source source_me.sh && python3 -m pytest tests/test_schema_layout_contract.py
  tests/test_source_file_line_limit.py tests/test_sql_source_line_length.py
  tests/test_ascii_compliance.py tests/test_markdown_links.py`; `git diff --check`; for every
  base-file package, `bash tests/e2e/e2e_postgres_migration_acceptance.sh --oracles <family
  oracle>` on a fresh disposable database.
- Integration gate: M10 full acceptance lane; M11 `./launchers/all_test.sh` and
  `./devel/run_playwright_tests.sh`; each capability milestone's connected e2e.
- Independent review gate: `architect` on WP-B07 and WP-B08; security review on WP-PQ1 and
  WP-UNR1; `reviewer` on every deleted oracle and every deleted Rust type.
- Failure semantics: a failed per-patch gate blocks that package; a failed family oracle blocks the
  next base-file milestone; a failed lane at M11 blocks every capability milestone; a security
  finding returns to its package and reruns its e2e.

## Test and verification strategy

Match the layer to the claim, per `docs/TEST_EVIDENCE_MODEL.md` and PYTEST_STYLE.md.

Deleted (old logic, no port): `tests/e2e/assignment_revision_entry_snapshot_catalog.sql`,
`postgres_migration_acceptance_catalog.sql`,
`postgres_migration_acceptance_assignment_release_compatibility.sql`,
`postgres_migration_acceptance_assignment_selection_compatibility.sql`,
`postgres_migration_acceptance_assignment_schedule_context.sql`,
`postgres_migration_acceptance_inline_assignment_retime.sql`,
`postgres_migration_acceptance_student_landing_score_disclosure.sql`,
`postgres_migration_acceptance_assignments_due_soon.sql`, and every other
`postgres_migration_acceptance_*.sql` whose assertions are column inventories, policy names, role
membership counts, or revision joins. Decision rule: an oracle survives only if it asserts a
security boundary, an evidence immutability, or a Student-visible behavior on the new model.

Rewritten to contracts: `question_records.sql`, `question_publication_operation.sql`,
`question_publication_credit_catalog.sql`, `question_asset_publication_catalog.sql`,
`question_asset_delivery_oracle.sql`, `imathas_question_backend_session_postgres_oracle.sql`,
`assignment_attempt_completion_authority_catalog.sql`, `assignment_question_analysis_job_catalog.sql`.

New, one per family, each under 400 lines: `base_schema_accounts_oracle.sql`,
`base_schema_question_library_oracle.sql`, `base_schema_authoring_oracle.sql`,
`base_schema_blueprint_oracle.sql`, `base_schema_course_oracle.sql`,
`base_schema_assignment_oracle.sql`, `base_schema_student_work_oracle.sql`,
`base_schema_operations_oracle.sql`. Each asserts behaviors named in its milestone, not shapes.

Fast pytest: `tests/test_schema_layout_contract.py` only. It protects the ordering contract both
migrators depend on; nothing else in this plan earns a permanent fast test.

Connected e2e added: `e2e_question_availability.sh`, `e2e_blueprint_draft.sh`,
`e2e_assignment_unrelease.sh`, `e2e_student_time_zone.sh`, each registered in
`tests/e2e/e2e_run_all.sh` and `local_stack_control/acceptance_lanes.py`.

Playwright: the release journey text change; one Danger Zone keyboard journey reused across the
three zones; a Student time-zone journey. axe on every changed surface.

## Migration and compatibility policy

- The base is authored directly; `schemas/migrations/` is deleted at M11 and never re-read.
- Every existing disposable volume is incompatible by design and is recreated through
  `local_stack.py`; image pruning is pre-approved by HUMAN_GUIDANCE.
- No compatibility view, alias, dual write, fallback reader, or create-then-drop file exists.
- Until the v1 launch tag, a wrong base file is corrected in place and the disposable database
  rebuilt. After the tag, base files are frozen and every change is one file in
  `schemas/schema_updates/`; the existing checksum `Changed` detection is the enforcement.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| A folded `CREATE OR REPLACE` body is the wrong (earlier) version | Behavior regression hidden by a green apply | Two definitions of one function in the old chain | `expert_coder` | Take the definition from the highest-numbered file; family oracle asserts the latest behavior |
| A grant or policy from a late migration is dropped in the fold | `ple_app` loses a read; live demo 500s | Missing `GRANT` in the family file | `reviewer` | Diff the old chain's grant lines per family before closing the package; restricted-login probe at M10 |
| Copied-fact list misses a consumer | Attempt reads mutable state after edit | A function still joins `assignment` after start | `architect` | Data inventory is the checklist; the M8 oracle edits every copied field then rereads |
| Unrelease misses a table | Student rows survive | New Student Work table added later without deletion | `expert_coder` | Deletion walks the FK graph from `assignment_attempt`; WP-UNR2 counts every reached table |
| Statistics recompute disagrees with accumulation | Wrong public counts | Choice counts need a fact not retained | `expert_coder` | M9 oracle compares accumulated against recomputed on a seeded revision before Unrelease exists |
| Generated embed and applier disagree on order | Fresh and live demo diverge | Applier reads disk while embed reads generated list | `expert_coder` | Applier iterates the embedded list; one source |
| Base file crosses 999 lines late | Line-limit gate fails | Fold grows a family | `expert_coder` | Split at 900 and renumber at M10; layout pytest enforces consecutiveness |
| Old chain and base coexist in one tree during authoring | Confusion; wrong file edited | M2-M10 window | `integrator` | The embed points at the base from M1; the old chain is read-only reference until `git rm` at M11 |
| Capability milestones widen scope | Plan never closes | Accommodation or copy/update routes appear | manager | Non-goals name them; delete scaffolding without replacement |
| Docs cite a base file line that later shifts | Broken citation | Renumber at M10 | `maintainer` | Docs cite family files by name, not line, and are written at M17 after numbering settles |

## Rollout and release checklist

- [ ] M0 plan published; reconciliation plan archived; decisions recorded.
- [ ] M1 embed generates from two folders; `0001` applies fresh and no-op.
- [ ] M2-M9 each family file applies and its oracle passes.
- [ ] M10 full acceptance lane passes on the base; old oracles gone; numbering consecutive.
- [ ] M11 `schemas/migrations/` removed; `all_test.sh` and Playwright green on a fresh stack.
- [ ] M12 no retired revision type, decoder, or generated file remains.
- [ ] M13-M16 each connected e2e passes; three Danger Zones live; Student time zone live.
- [ ] M17 documentation cites base files only; Graphify regenerated; changelog written.
- [ ] Release acceptance remains governed by `docs/ROADMAP.md`; this plan authorizes no deployment.

## Documentation close-out requirements

- Active plan / progress tracker: the published plan records each milestone's evidence path as it
  exits; archived by `git mv` at closure.
- docs/CHANGELOG.md entry: one day block; Decisions and Failures records the single-authoring choice,
  the deleted oracle list, and any family whose oracle needed a second attempt.
- Archive / closure notes: `revision_concerns.txt` closes with links to the Data inventory and the
  M8 oracle; the superseded plan carries its supersession line.

## Patch plan and reporting format

- Patch 1: M0 and M1.
- Patch 2: M2.
- Patch 3: M3.
- Patch 4: M4 and M5 (two doers).
- Patch 5: M6.
- Patch 6: M7.
- Patch 7: M8.
- Patch 8: M9 (three doers).
- Patch 9: M10.
- Patch 10: M11.
- Patch 11: M12.
- Patch 12: M13, M14, M16 (three doers).
- Patch 13: M15.
- Patch N: M17 and remaining repository-required work.

Each patch report states: files touched, gates run with exit codes, oracle names passed, findings
returned to a package, and the next milestone unlocked.

## Data inventory

Facts the current functions read from `assignment_revision` and its entries, and where each lands:

| Fact | Readers today | New home |
| --- | --- | --- |
| `available_at`, `closes_at`, `late_work_rule` | start, prepare, access, save response, history | `assignment_attempt` copied |
| `due_at`, `assignment_title` | every reader via the delivery fallback | `assignment_attempt` (already captured; fallback removed) |
| `assignment_instructions` | native PLE start, legacy start | `assignment_attempt` copied |
| `assignment_attempt_time_limit_seconds` | save, finalize, progress, context, access | `assignment_attempt` copied |
| `attempt_limit`, `assignment_attempt_continuation_rule`, `max_additional_assignment_attempts` | start only | stay on `assignment`; start-time decision |
| `question_pool_reuse_rule`, `question_variation_rule` | start | `assignment_attempt` (already present) |
| `assignment_question_order_rule` | order wrapper | `assignment_attempt` copied |
| `assignment_completion_rule`, `assignment_completion_score_threshold` | completion trigger | `assignment_attempt` copied |
| seven `feedback_*` rules | history, access, landing | `assignment_attempt` copied |
| entry `point_value`, `scoring_rule`, `assignment_entry_id`, `assignment_content_entry_index` | start, issuance source binding, prepare | `issued_question` (position renamed `assignment_entry_position`) |
| fixed `question_id`, `revision_number` | start expected set, source binding | `issued_question` (already present) |
| pool `selection_count`, item `availability` | start coverage, triggers | validated against current entries at start; selected items copy facts |
| `assignment_deadline_rule`, `assignment_attempt_grade_rule`, `assignment_attempt_resume_rule`, `assignment_question_display_rule`, `assignment_navigation_rule` | none | stay on `assignment` only |
| `revision_number` | release response echo | removed; response carries `assignment_edit_number` |

Retired objects and their replacements:

| Retired | Replacement |
| --- | --- |
| `ple_data.course_schedule_revision` and its triggers | `course_instance.term_starts_on`, `term_ends_on` |
| `ple_data.assignment_revision`, `assignment_revision_entry`, `_fixed_question`, `_question_pool`, `_question_pool_item`, their triggers, indexes, `require_released_assignment_revision`, `assignment.released_assignment_revision_id` | `assignment_entry`, `assignment_fixed_question`, `assignment_question_pool`, `assignment_question_pool_item`; released check at Attempt insert |
| `ple_private.assignment_attempt.assignment_revision_id`, `delivery_*` naming | copied fact columns |
| `ple_private.live_demo_assignment_question`, `assignment.live_demo_question_selection_version` | `assignment_entry` family |
| `ple_private.course_retention_plan_revision`, job and event FKs and constraints | `ple_private.course_retention_plan`; copied effective values |
| `ple_data.question_change_proposal_revision`, event FK | `ple_data.question_change_proposal`; event copies facts |
| `question_revision_availability_event`, `blueprint_revision_availability_event` | `published_question_availability_event`, `blueprint_course_availability_event` |
| Blueprint base `PUT` creating a revision | `blueprint_draft` plus explicit publish |

## Resolved decisions

- Schema layout: `schemas/base_schema/NNNN_<family>.sql` with consecutive integer versions from 1,
  one table family per file, each under 999 lines; `schemas/schema_updates/YYYYMMDDNN_<name>.sql`
  for every change after the v1 launch tag; one generated embed reads both; the base is frozen at
  the tag and the checksum check enforces it. Owner: `crates/learning-data-access/build.rs` and
  `docs/DATABASE_STRUCTURE.md`.
- Single combined authoring: the base is written directly to the revision-free model; no faithful
  squash step exists. Owner: WS-BASE.
- All four retired revision families are unwound in this plan: Course Schedule, Assignment and its
  entries, Question Change Proposal, Course Retention Plan. Owner: WS-BASE.
- Attempt and Issued Question evidence is explicit columns copied at start; no snapshot table, no
  JSON policy blob, no fallback to mutable state. Owner: `0014_assignment_attempts_and_issued_
  questions.sql`.
- Assignment status is current state governed by the Assignment Edit Number; release and unrelease
  each advance it by one. Owner: `0013_assignments.sql`.
- Course Instance has no Edit Number in this version; term dates live on the Course row. Owner:
  `0010_course_instances.sql`.
- Availability is a lineage fact for Published Questions and Blueprint Courses; publication never
  resets it. Owner: `0004` and `0009`.
- The reconciliation plan's capabilities (availability actions, Blueprint Draft, Unrelease, Student
  time zone) are in scope as M13-M16; its Accommodation, copy/update, and grading-operation
  scaffolding is deleted without replacement. Owner: this plan.
- Old-logic e2e oracles are deleted, not ported; survivors assert contracts only. Owner: WP-CL2.
- Statistics after Unrelease are recomputed from retained submissions and grading results by a
  function authored with the statistics family, proven equal to accumulation before Unrelease
  exists. Owner: `0018_question_statistics.sql`.

## Open questions and decisions needed

This plan is finishable by the manager and subagents without another owner question.

- Manager/subagent decision procedure:
  - Decision owner or dedicated class: `architect` for any newly found `revision` or `snapshot`
    noun; `expert_coder` plus `reviewer` for any oracle whose survival is unclear.
  - Evidence and decision rule: a noun survives only if it names immutable published Question or
    Blueprint content or an exact reference to it; otherwise it becomes current state with an Edit
    Number when a concurrent save exists, or copied evidence when a Student record depends on it. An
    oracle survives only if it asserts a security boundary, an evidence immutability, or a
    Student-visible behavior on the new model; otherwise it is deleted.
- Non-blocking follow-up:
  - Whether `0020` iMathAS splits into two files is decided by its line count at WP-B09; the layout
    pytest and renumbering at M10 absorb either outcome.
  - Whether `0007` publication operations precede or follow `0006` is decided at WP-B03 by whether
    the draft source resolver needs draft tables; both orders are consecutive at M10.
