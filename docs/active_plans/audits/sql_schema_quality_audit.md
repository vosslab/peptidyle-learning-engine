# SQL schema quality and efficiency audit

Read-only audit of the canonical PostgreSQL 17 base schema under
`base_schema`, installed from
[install.sql](../../../schemas/base_schema/install.sql). It extends the
findings the human recorded in
[database_repetition_audit.md](database_repetition_audit.md) and checks each structural
recommendation against [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) and
[DATABASE_STRUCTURE.md](../../DATABASE_STRUCTURE.md).

Audit date: 2026-09-17. Working copy: `main` at `9156ee17` plus the uncommitted UI audit; no SQL
files were modified. No code changed.

## Verdict

**Readiness not established for a production schema freeze.** Nothing reproduced here breaks the
approved data contract, and the manifest installs cleanly in one second. The schema is, however,
carrying a large amount of redundant, derivable, and stringly typed data on exactly the rows that
multiply fastest (Assessment Attempts, Issued Questions, Question Attempts, and notifications), and
several identity and lifecycle shapes contradict current Human Guidance. Because PLE is
pre-production with no durable data (HUMAN_GUIDANCE.md, "Codebase development rules"), these are
cheap to fix now and expensive after the first real semester.

Completion gate for this audit: the layered source layout, catalog comments, and generated
`docs/SCHEMA_TABLES.md` of package 0 are in place, and packages A through E have landed in the
new layout. Package F (indexes, immutability guards) may follow the freeze.

Three headline numbers from the installed catalog:

| Measure | Value | Why it matters |
| --- | --- | --- |
| Native enum types | 3 (all Bloom) | ~110 other closed vocabularies are `text` + `CHECK (... IN (...))` |
| FK edges without a supporting index | 166 of 281 | Every cascade delete, purge, and join on those edges scans the child |
| Hand-written immutability triggers | 92 `IS DISTINCT FROM OLD` clauses + 9 `ROW(...)` compares in 14 files | Column-list drift is a silent correctness risk |

## Scope and method

Scope: what SQL stores, constrains, and computes. Out of scope: Rust/TypeScript validation,
transport, browser behavior, and the object store. A missing SQL check here does not mean the
application lacks it.

Evidence types used below are labeled:

- **catalog**: measured from a fresh disposable install (`postgres:17` container, bootstrap through
  `local_stack_control.lifecycle_database.migration_principal_bootstrap_sql`, then
  `psql --single-transaction -f install.sql` as `ple_migrator`; container removed afterwards).
- **source**: read from the SQL modules; line references are `file:line`.
- **inference**: reasoning from the source and Human Guidance, not yet reproduced.

No production or Live Demo database was touched. No stale `ple-*-pg17` containers were used.

### Installed catalog summary (catalog)

| Item | Count |
| --- | --- |
| Tables | 146 (`ple_data` 67, `ple_private` 66, `ple_audit` 12, `ple_migration` 1) |
| Tables without a primary key | 0 |
| Tables with RLS + FORCE RLS | 144 of 145 product tables (the `_sqlx_migrations` ledger is the exception) |
| CHECK constraints | 497 |
| Foreign keys | 281 |
| UNIQUE constraints | 98 |
| Indexes | 282 (249 unique, 146 PK, 17 partial); only 38 are explicit `CREATE INDEX` |
| Routines | 599 (495 `SECURITY DEFINER`) |
| Triggers | 158 |
| RLS policies | 290 |
| Not-validated constraints / invalid indexes | 0 / 0 |
| Install warnings | 31 lines, all harmless (system-column REVOKE, one identifier truncation, no-op grants) |

Column type histogram: `uuid` 333, `text` 270, `timestamptz` 170, `bigint` 95, `integer` 72,
`bytea` 44, `jsonb` 14, `boolean` 9, `numeric` 8, `text[]` 5, `interval` 4, enums 7, `date` 2,
`smallint` 1, `xid8` 1. No `timestamp without time zone` anywhere (good).

## Part 1: repetition taxonomy and per-row findings

The human's earlier note names the vocabulary. This section applies it to every table where
the pattern was found, ranked by row-multiplication rate. Row-rate estimate for one semester:
one Course of 100 Students x 20 Assessments x 25 Questions x 2 Attempts = 100,000 Issued
Questions and Question Attempts, 4,000 Assessment Attempts.

### 1.1 Assessment Attempt copies the whole current Assessment (source)

`ple_private.assessment_attempt` (assessment_attempts.sql:34) repeats, per Attempt:

- `assessment_title` and `assessment_instructions` (up to 50 KB of free text; assessment.sql:24-32
  bounds the source, the copy at assessment_attempts.sql:43-44 does not even bound its length).
- nine closed-vocabulary strings: `late_work_rule`, `question_variation_rule`,
  `assessment_question_order_rule`, and six `feedback_*` columns (assessment_attempts.sql:50-58).
- schedule and limits: `available_at`, `due_at`, `closes_at`, `assessment_attempt_time_limit_seconds`,
  `assessment_attempt_limit`.
- three accommodation id + edit-number pairs.

That is 29 columns and 23 CHECK constraints on the Student Work root (catalog: widest private
table). The same nine policy strings, with the same 19 repeated literal lists, also live on
`ple_data.assessment` (assessments.sql:41-65) and `ple_private.assessment_template`
(assessment_templates.sql:44-75): three tables, one policy shape, zero shared type.

Human Guidance check:

- "Changes to Assessment settings do not change the recorded history of completed Assessment
  Attempts" (HUMAN_GUIDANCE.md:848) justifies snapshotting *something* at Attempt start.
- "PLE should retain only the additional historical Student Work data needed to interpret or grade
  that work correctly" (HUMAN_GUIDANCE.md:1556) does **not** justify copying the title and a 50 KB
  instruction body per Attempt, and it is at least arguable for the six feedback-disclosure
  columns, which govern what the Student may *see* rather than how the work is interpreted.

Recommendation (plan M2): one immutable, content-addressed
`assessment_policy_snapshot` row referenced by both `assessment` (current) and every
`assessment_attempt`; the Attempt keeps only what the guidance requires (timing instants,
effective limits, exact policy reference). Title and instructions are read from the snapshot.

### 1.2 Question Attempt repeats the toolchain per Question (source)

`ple_private.question_attempt` (assessment_attempt_interaction.sql:5) stores, for every one of the
~100,000 Question Attempts per semester: `backend_name`, `backend_version`, `renderer_name`,
`renderer_version`, `grader_name`, `grader_version`, `issued_capability` (7 text columns) plus a
32-byte `rendered_question_sha256`. Within one deployment these seven strings take a handful of
distinct combinations. This is snapshot configuration stored as repeated categorical text.

Recommendation: a small `ple_private.delivery_toolchain` table (`toolchain_id`, the seven
values, `UNIQUE` over them); `question_attempt.toolchain_id` replaces 7 columns and 6 CHECKs.
The `backend_name` value is also already derivable from
`question_revision_source_binding.backend` for the issued revision (the trigger at
assessment_attempt_interaction.sql:98-110 re-checks exactly that).

### 1.3 Derived state columns (source, confirmed by finalization SQL)

- `question_attempt.question_attempt_state` (`open`, `response_finalized`, `closed_unanswered`) is
  computed at submit time from "does a saved response exist" and "was the Attempt submitted"
  (assessment_attempt_finalization.sql:340-352). It is a materialized boolean pair plus a
  forward-only trigger, all derivable from `finalized_at IS NOT NULL` and the existence of a
  `question_response` row.
- `assessment_submission.finalization_kind` is derivable from
  `authorized_by_account_id IS NOT NULL` (the CHECK at assessment_attempt_interaction.sql:60 says
  so), and the `receipt` JSON repeats both the kind and the constant `submissionState: submitted`
  (finalization.sql:338).
- `question_response_grading.grading_state` is the single literal `'graded'` (grading.sql:11) and
  `completed_at` is forced NOT NULL by a second CHECK (grading.sql:15); the table is a 1:1 wrapper
  between `question_response` and `grading_result` with no information of its own.
- `question_response.student_response` is a byte-for-byte copy of
  `assessment_attempt_saved_response.student_response` made at submit
  (finalization.sql:358-364), and the saved row is never deleted. Two JSONB copies of every
  response for the lifetime of the Attempt.

Recommendation: drop `question_attempt_state`, `finalization_kind`, the `receipt` duplicates,
and the `question_response_grading` wrapper (fold `grading_result` onto `question_response`
or give `question_response` a nullable `normalized_credit`). For the response copy, either
finalize in place (add `finalized_at` to the saved-response row and forbid updates after it)
or delete the saved row on submit; the current contract in DATABASE_STRUCTURE.md
("An internal `question_response` row stores finalized-response evidence") is satisfied by
either.

### 1.4 Issued Question repeats Entry configuration (source)

`ple_private.issued_question` (assessment_attempts.sql:111) copies `scoring_rule`, `point_value`,
`question_attempt_limit`, `question_attempt_time_limit_seconds`, `question_attempt_grace_seconds`
from the Assessment Entry into every issued row. The comment at assessment_attempts.sql:85-87
explains why `assessment_entry_id` is a copied identity rather than an FK (Released saves may
replace entries). The snapshot is legitimate; its shape is not. The scorer at
grading.sql:51 already reads the *current* Entry points (finalization.sql:285-291), so
`point_value` on the issued row is not even the value used for scoring, contradicting
HUMAN_GUIDANCE.md:1563 only if it is ever read for scoring, and wasting 8+ bytes per row if it is
not.

Recommendation: reference an immutable `assessment_entry_snapshot` row (one per distinct Entry
configuration at issue time) instead of copying five columns, and decide whether `point_value` on
the issued row has any reader.

### 1.5 Notification fan-out stores the event three times (source)

For Question Library Watches there are three tables:

1. `ple_data.library_watch_event` (question_watch_notifications.sql:7): the event.
2. `ple_data.library_watch_event_recipient` (question_watch_notifications.sql:55): `(event_id,
   recipient_account_id)`, the frozen recipient snapshot.
3. `ple_private.library_watch_notification` (question_watch_notifications.sql:71): one row per
   recipient that repeats `target_kind`, `target_public_id`, `event_kind`, `revision_number`,
   `forked_public_id`, `activity_id`, `occurred_at`, and the whole four-branch CHECK from table 1.

Table 3 carries no read/dismissed state and no column that table 2 plus a join to table 1 cannot
supply. It also has a copy of the two Crockford checksum CHECKs, so every fan-out row re-runs
`crockford_checksum_character()` twice.

The sibling `ple_private.blueprint_course_watch_notification` (blueprint_stewardship.sql:155)
shows the right shape: recipient, target reference, event kind, source event id, time. Use it as
the model, or better, make table 2 the notification (add `read_at timestamptz` there if a read
state is ever a product requirement) and drop table 3.

### 1.6 Constant columns (catalog + source)

Seventeen columns admit exactly one value (parser over all `CREATE TABLE` bodies):

| Column | Only value | Assessment |
| --- | --- | --- |
| `course_instance.assigned_instructor_role`, `course_invitation.inviting_instructor_role`, `course_retention_notification.recipient_product_role`, `support_repair_capability.sysadmin_role`, `forced_question_correction.approver_role`, three `*_product_role` columns in `ple_audit.instructor_*` | one role | **Keep the mechanism, shrink the column.** These are role-typed foreign keys: `FOREIGN KEY (account_id, role) REFERENCES account (account_id, product_role)` (11 sites) makes PostgreSQL enforce "this account is an Instructor" declaratively, with no trigger. That is good design. The cost is a `text` per row; convert `product_role` and every carrier to one `ple_private.product_role` enum (4 bytes, no CHECK). |
| `course_banner_rendition.rendition_kind`, `course_banner_delivery.rendition_kind` | `'banner'` | Remove; placeholder for a second rendition kind with no approved design (HUMAN_GUIDANCE.md:89). |
| `job.job_kind`, `job.job_target_kind`, `job.worker_kind` | one each | Remove all three; rename the table `public_asset_publication_job` if the name should say what it is. |
| `question_response_grading.grading_state` | `'graded'` | Remove with the table (1.3). |
| `support_repair_capability.resource_class`, `support_repair_capability_event.resource_class` | `'student'` | Remove; same placeholder rule. |
| `assessment_unrelease_event.outcome` | `'completed'` | Remove. |

### 1.7 Stringly typed closed vocabularies (source)

About 110 `text` columns are constrained to a literal set. Only three enums exist. The repeated
five-value feedback list appears 19 times in the DDL alone. Beyond storage (a `text` value is
1 + length bytes and TOAST-able; an enum is 4 bytes), each duplicate literal list is a separate
place to edit when the vocabulary changes, and the CHECK text is not introspectable as a type.

Candidate enum families (one type each, referenced from every table that uses it):

- `product_role` (3 values, 11 role-typed FK sites)
- `assessment_type` (5), `late_work_rule` (3), `question_variation_rule` (2),
  `question_order_rule` (2), `feedback_release` (5, used by 18 columns across 3 tables)
- `scoring_rule` (4, 2 tables), `entry_kind` (2), `entry_availability` (2)
- `question_backend` (`ple`, `webwork`, `imathas`: 4 tables), `question_format` (3 tables),
  `question_type` (8 values, 3 tables)
- `library_object_kind` (`question`, `question_pool`: 5 tables), `library_watch_event_kind` (4)
- `media_type` (3 image types, 4 tables), `object_storage_area` (4), `object_data_class` (8)
- `lease_state` (`ready`, `leased`, `completed`; see 2.4)

Free prose, canonical public IDs, external references, SPDX expressions, and audit reasons stay
`text`. Where a vocabulary is expected to grow at runtime (course themes: 15 values today), a
small reference table beats an enum.

### 1.8 Classification 4-tuple repeated with three composite FKs (source)

`discipline_uuid, subject_uuid, topic_uuid, subtopic_uuid` plus three composite FKs and two
CHECKs appear identically on `course_instance` (course_core.sql:27-40), `blueprint_course`
(blueprints.sql:30-42), `question_pool` (question_pools.sql:33-45),
`published_question_metadata` (question_lineages.sql:107-117), and the two draft/metadata event
tables. The tuple is a materialized path through a four-level hierarchy; storing all four levels
so that each FK can pin the parent is a known PostgreSQL idiom, and vocabulary changes are rare
(HUMAN_GUIDANCE.md:757). This is acceptable **if** the six copies share one definition. Options:

- keep as is, but generate the four-column block from one place (lowest effort);
- store only the most specific node id plus a `classification_node` table with a
  `(node_id, parent_id, level)` shape and derive ancestors on read (one FK instead of three; the
  hierarchy invariant moves into the vocabulary table where it belongs).

### 1.9 Blueprint metadata event duplicates the current row (source)

`ple_data.blueprint_metadata_event` (blueprints.sql:159) stores name, classification 4-tuple,
tags, availability, and `metadata_etag` per metadata change, while `blueprint_course` stores the
same current values. The current row is exactly the latest event. Either the current table
projects from the event log (a view or a `current_metadata_event_id` FK) or the log keeps only
the changed fields. Same shape for `course_instance` vs `ple_audit.course_instance_creation_event`
(course_core.sql:286).

## Part 2: durable-model findings beyond repetition

### 2.1 Three identity conventions (source, pre-freeze decision)

| Aggregate | Primary key | FK target used by children | Public reference |
| --- | --- | --- | --- |
| Published Question | `question_id text` (the public `XXXX-ZXXX`) | the public ID | same column |
| Question Pool | `question_pool_id uuid` | uuid | `public_question_pool_id text` |
| Course Instance, Assessment, Account | `*_id uuid` | uuid | `public_reference text` + `reference_number bigint` |
| Blueprint Course | `blueprint_id uuid` | **`reference_number bigint`** (17 child tables) | `public_reference text` |

`blueprint_course.blueprint_id` is a primary key that nothing references. HUMAN_GUIDANCE.md:687
requires that human-facing IDs not reveal creation order; using an identity `bigint` as the FK
carrier does not leak it to users, but it does make the Blueprint family the only one keyed by a
sequence. Decision (2026-09-18, recorded in HUMAN_GUIDANCE.md "Human-facing reference IDs" and
DATABASE_STYLE.md "Identity"): the public ID is the primary key and sole FK target wherever one
exists; the Question convention becomes the rule and the other three are re-keyed in plan WP-1.4.

Related inconsistency: `revision_number` is `integer` for Question Revisions and `bigint` for
Pool and Blueprint Revisions, and `question_revision_number` is `bigint` in
`blueprint_revision_question_pin` (blueprints.sql:84) while its FK target is `integer`. Pick one
width. `source_object_checksum` is `text` hex in the source-binding tables
(question_authoring_state.sql:102) and `bytea(32)` in `question_attempt`
(assessment_attempt_interaction.sql:19); pick `bytea`.

### 2.2 Two concurrency mechanisms (source, HG alignment)

HUMAN_GUIDANCE.md:836 says "Mutable working state uses a monotonic sequential Edit Number when
needed for concurrency." The schema uses `*_edit_number bigint` on most aggregates **and**
`metadata_etag uuid` on six tables (`course_instance`, `blueprint_course`, `question_pool`,
`blueprint_metadata_event`, and the change-proposal pair, which carry FKs to
`(reference_number, metadata_etag)`). Two tokens for one purpose. Replace `metadata_etag` with an
edit number, or record the etag decision in `docs/DESIGN_DECISIONS.md` if random tokens are
required to defeat guessable CAS values.

### 2.3 Lifecycle vocabularies that Human Guidance retired (source, HG alignment)

- `assessment.assessment_status IN ('unreleased', 'released', 'closed', 'archived')`
  (assessments.sql:66-68). DATABASE_STRUCTURE.md: "Closed and Archived Assessment rows are not
  target lifecycle states." Two dead values in the CHECK, and any code path that writes them is
  drift.
- `course_instance.assigned_instructor_account_id NOT NULL` (course_core.sql:16). HUMAN_GUIDANCE.md
  :1165 says creation "does not give that Instructor greater Course authority than later
  co-Instructors" and DATABASE_STRUCTURE.md says "There is no privileged Course-owner row." The
  column is a privileged owner row by another name; `course_membership(role = 'instructor')` is the
  equal-authority model. Keep creation provenance in `course_instance_creation_event` only.
- `assessment_entry.availability IN ('available', 'retired')` is fine (entries, not Blueprints),
  but `published_question.availability` and `assessment_entry.availability` share a column name
  with different vocabularies.

### 2.4 Four hand-rolled lease/worker patterns (source)

`job` (jobs.sql:6), `course_retention_notification` (course_retention_notifications.sql:7),
`course_banner_work` (course_media.sql:191), and `profile_image_work` (profile_media.sql:95) each
implement lease token, lease expiry, attempt count, and a state CHECK with slightly different
column names and transition triggers. One `ple_private.work_lease` shape (or one composite type)
reused by all four keeps the state machine in one place. HUMAN_GUIDANCE.md:73 also asks that
background processing exist only for a demonstrated need; confirm each of the four is still
needed after the Ollama and iMathAS removals.

### 2.5 Deferred-backend scaffolding still installed (source, HG alignment)

`ple_private.imathas_render_cache_entry` and `ple_private.imathas_question_backend_session`
(delivery_backends.sql:7-57; 26 columns, 19 CHECKs, a 22-column immutability trigger) plus the
`'imathas'` branch in four vocabularies and the four `imathas_*` columns in both source-binding
tables. HUMAN_GUIDANCE.md:34 and :941 defer iMathAS; the changelog (2026-09-17, Decisions and
Failures) records "desired iMathAS/H5P behavior is deferred and implies no current backend work";
HUMAN_GUIDANCE.md:89 forbids placeholder tables before an approved design. Remove the two tables
and the `imathas` vocabulary branches; re-add them with the backend.

### 2.6 Immutability enforced by 92 hand-listed column comparisons (source)

Fourteen modules raise "... is immutable" from triggers that enumerate columns with
`NEW.x IS DISTINCT FROM OLD.x` (92 clauses) or `ROW(NEW.a, ...) IS DISTINCT FROM ROW(OLD.a, ...)`
(9 sites, one of them 22 columns wide). Adding a column to any of those tables silently makes it
mutable unless the trigger is edited too. Two cheaper, drift-proof shapes:

- for fully immutable tables (events, receipts, revisions): no trigger at all; `REVOKE UPDATE`
  from every runtime role and let RLS `WITH CHECK (false)` on UPDATE express it;
- for partially mutable rows: a generic trigger that compares `to_jsonb(NEW) - mutable_columns`
  against `to_jsonb(OLD) - mutable_columns`, where the mutable list is the short one.

### 2.7 Union tables with wide nullable column sets (source)

`assessment_entry` (assessments.sql:98) holds fixed-question and pool entries in one row with
eight columns that are NULL for the other kind and a 22-line CHECK to pair them. `library_watch_
event` uses the same shape with four branches. This is a standard trade-off; it becomes a
finding only because the Attempt-side tables (`question_pool_selection`, `issued_question`) then
copy both halves. If package B introduces entry snapshots, split the entry into
`assessment_entry` (position, kind, points) plus `assessment_entry_question` and
`assessment_entry_pool` children at the same time.

### 2.8 Retention and privacy (inference, not reproduced)

`course_instance.purged_students_ever_enrolled` and the archive/delete state machine
(course_core.sql:56-107) look right for HUMAN_GUIDANCE.md:820-829. Not exercised here: whether a
purge removes `ple_audit.course_roster_event.student_account_id`,
`library_watch_notification.recipient_account_id`, and `course_roster_profile` rows, or whether
those retain identifying evidence past deletion. Record this as an open oracle in package F, not
as a defect.

### 2.9 Tables with no clock (source)

43 of 146 tables have no `timestamptz` column at all. Most are children that inherit their
parent's time (revision members, Blueprint pins, vocabulary hierarchy, delivery bindings), which
is why nobody noticed; but the set also includes `ple_private.assessment_template`
(assessment_templates.sql:23), a current-state table with an Edit Number and no
`created_at`/`updated_at`, `ple_private.issued_question` (assessment_attempts.sql:111), a Student
Work row with no issue instant of its own, `ple_private.question_asset_publication`, and all five
`content_*` vocabulary tables. Without a creation instant a row cannot be ordered, aged, purged
by date, or explained in support. [DATABASE_STYLE.md](../../DATABASE_STYLE.md), "Every table has
a clock", requires one creation instant per table and `updated_at` on current state; plan work
package WP-1.7 adds them. Full list from the parser: `account_time_zone`, the four
`question_attempt_presentation_*` bindings, `question_pool_selected_item`, `issued_question`,
`assessment_template`, `assessment_entry`, `assessment_question_pool_fork`, the three
`blueprint_revision_*` children, five `content_*` tables, two `forced_question_correction_*`
targets, `course_banner`, `course_banner_rendition`, `course_banner_delivery`,
`course_banner_storage_subject`, `course_banner_prepared_presentation`,
`course_retention_policy`, `draft_question_asset`, `course_object_delivery`,
`profile_image_delivery`, `provided_avatar`, `account_avatar`, `question_asset_delivery`,
`question_asset_publication`, `question_revision_bloom`, `question_pool_revision_bloom`,
`bloom_preparation_receipt`, `question_pool_revision_member`, `question_revision_authorship`,
`question_revision_license`, `question_revision_citation`, `library_watch_event_recipient`,
`question_revision_choice_statistics`, `question_statistics_observation_choice`.

### 2.10 Growth outside the FERPA purge (source)

Student Work is bounded by the 365-day purge (about 25-40M rows per table at 50 Courses x 100
Students x 20 Assessments x 25 Questions x 5 Attempts). Tables that grow with Student activity
but sit outside that purge were checked for a delete path:

- `ple_audit.automated_grading_receipt` and `ple_private.question_statistics_observation_receipt`
  / `_choice` cascade from `grading_result` and `question_attempt` (grading.sql:131-133,
  statistics.sql:125-129): purged with the Work. Fine.
- `ple_private.authenticated_session` (authentication.sql:145) is revoked
  (authentication.sql:186, :310) but never deleted, and carries `account_id`. Roughly one row
  per login, ~1.8M per year at 5,000 users: unbounded, and identifying. Needs an expiry sweep.
  `authentication_rate_limit`, `email_authentication_challenge`, and `passkey_ceremony` are the
  same class and were not individually traced.
- `ple_audit.object_delivery_access_event` (object_records.sql:207) has no writer in
  `schemas/base_schema/` or `crates/`. It is a placeholder (HUMAN_GUIDANCE.md:89) that, if ever
  wired, would record every asset load and become the largest table in the system. Remove.
- `ple_data.library_watch_event` and the admin `ple_audit.*_event` tables grow with authoring and
  administration, not Student activity: small, fine.

Published Questions are authored, not generated (~10M rows across all Revision children at 1,000
Instructors x 500 Questions x 3 Revisions): not a size concern and not a partition candidate; the
Library is global by rule (HUMAN_GUIDANCE.md:868). The Question-side risk is discovery search, which
wants a measured `tsvector` + GIN index (plan M4), not partitioning.

### 2.11 Key column names hide their table (source census)

Of 180 single-column foreign keys, 72 have a column name that ends in something other than the
parent's table name plus `_id`: `course_id` -> `course_instance` (14 tables), `question_id` ->
`published_question` (7, plus every `(question_id, revision_number)` pair), `object_id` and
`source_object_id` -> `object_record` (11), `blueprint_course_reference_number` (9, removed by
2.1), `delivery_id` -> `object_delivery`, `workspace_id` -> `authoring_workspace`, the
`*_uuid` classification keys, and short names such as `thread_id`, `folder_id`,
`proposal_id`, `capability_id`, `membership_id`, `invitation_id`. A reader cannot tell from
`course_id` which table it joins. [DATABASE_STYLE.md](../../DATABASE_STYLE.md) "Naming" and
checklist item 7 require `[<role>_]<parent_table>_id`; plan WP-1.8 renames.

## Part 2b: source organization (source census)

The 27,306-line manifest is organized by operation, not by kind, which is why this audit needed a
parser and a container to answer "what tables exist":

| Measure | Value |
| --- | --- |
| Files containing `CREATE TABLE` | 39; 36 of them also contain functions |
| Table-only files | 3 (`content_classification.sql`, `foundation_roles.sql`, `library_discussions.sql`) |
| Lines inside function bodies | 57% |
| `GRANT`/`REVOKE` | 706, interleaved with DDL; 439 `SET LOCAL ROLE` / `RESET ROLE` |
| `ALTER TABLE` | 295, in 40 files, not only `cross_domain_constraints.sql` |
| `COMMENT ON` | 32 for 146 tables and ~1,270 columns; table documentation is `--` prose invisible to the catalog |
| Explicit `CREATE INDEX` | 38, in 17 files |

Concrete consequences: the seven Student Work tables are split across `assessment_attempts.sql`,
`assessment_attempt_interaction.sql`, `assessment_attempt_presentation.sql` (four tables inside an
813-line file whose name says "presentation"), and `grading.sql`; `assessments.sql` puts three
tables among 7 functions, 11 policies, 7 ALTERs, and 7 grants across 963 lines; the manifest order
is dependency order with no index of which table lives where. A reader tracing one FK graph opens
five files.

[DATABASE_STYLE.md](../../DATABASE_STYLE.md), "Organization of the SQL source", is the target
layout. Reaching it is milestone M0 of the plan and is the completion gate for this audit.

## Part 3: remediation

The fixes for every finding above are planned in
[sql_schema_restructure_plan.md](../../archive/sql_schema_restructure_plan.md) as milestones M0
(layered layout and catalog comments), M1 (types and identity), M2 (snapshots), M3 (derived data
and fan-out), and M4 (measured indexes and generic immutability guards). Package letters used in
the findings map as: package 0 = M0, A and D = M1, B = M2, C and E = M3, F = M4. This audit does
not change as the plan executes; the plan's status line records progress.

## Part 4: positive evidence and unverified boundaries

Passed (catalog):

- Every product table has a primary key; every product table except the migration ledger has RLS
  enabled and forced.
- All 281 FKs and 497 CHECKs are validated; no invalid indexes.
- All timestamps are `timestamptz`; calendar dates are `date`; money-like quantities use
  `numeric` with explicit scale CHECKs; checksums are fixed-length `bytea` where the newer
  modules wrote them.
- Public-ID checksum and canonical-shape CHECKs are present on every public reference column.
- The five FK-isolated tables (`_sqlx_migrations`, `public_id_reservation`,
  `authentication_rate_limit`, `course_retention_policy`, `bloom_preparation_receipt`) are all
  legitimately outside the product graph (ledgers, reservations, configuration, one-use receipts).
- The role-typed FK idiom is a strong declarative invariant and should be kept.

Not established here:

- Runtime index usage. A fresh install has zero scan counters; the 166 unindexed edges are
  candidates until `EXPLAIN` on seeded data says otherwise (package F).
- Authorization semantics of the 495 `SECURITY DEFINER` routines and 290 policies; see
  [DATABASE_AUTHORIZATION.md](../../DATABASE_AUTHORIZATION.md) and
  [database_baseline_security_catalog.sql](../../../tests/e2e/database_baseline_security_catalog.sql).
- Application readers of the columns proposed for removal in 1.3. A source grep found two:
  `crates/learning-data-access/src/archived_student_work_recovery.rs:106-141` projects
  `finalization_kind`, `question_attempt_state`, and `grading_state` into the recovery record, and
  `crates/learning-data-access/src/postgres/assessment_delivery_finalization.rs:25-151` reads and
  binds `finalization_kind`. Package C must change those two projections to derive the values.
  The policy columns of 1.1 are read by five `crates/learning-data-access/src/postgres/assessment_*`
  modules; package B touches each.
- Physical integrity (`pg_amcheck`) on a mostly empty database proves little and was not run.

## Relationship to the earlier note

[database_repetition_audit.md](database_repetition_audit.md) items 1, 2, 3, 4, 5, and 6 are
confirmed above as 1.5, 1.1, 1.3, 1.3, 1.3, and 1.4 respectively. One correction to that note:
the fixed-role columns (`assigned_instructor_role` and friends) do carry information to the
database even though they carry none to a reader; they are the composite-FK carrier that makes
PostgreSQL enforce the Product Role of the referenced Account. Remove the `text` cost with an
enum (package A), not the column.
