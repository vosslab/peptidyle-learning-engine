# Database style

Design rules for PostgreSQL tables in this repository. This document owns how a table is shaped;
[DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) owns which tables exist and what they mean;
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md) owns roles, grants, and row security;
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) is the product authority every rule here serves.

The rules come from the findings of
[docs/active_plans/audits/sql_schema_quality_audit.md](active_plans/audits/sql_schema_quality_audit.md),
the PostgreSQL 17 documentation, and the design chapters of the local PostgreSQL corpus (see
[Sources](#sources)). Cite a rule by its heading when making a schema judgment call.

## Philosophy

- **Tables are forever; functions are replaceable.** A function is `CREATE OR REPLACE`; an index is
  `CREATE INDEX CONCURRENTLY`; a table shape with production rows in it is a migration with locks,
  rewrites, and a rollback plan. Spend design effort on table shape first.
- **Store each fact once.** Every additional copy of a fact is a place it can disagree with itself.
  Redundancy takes exactly one form: a deliberate, named snapshot (see
  [Snapshots](#snapshots-copy-by-reference)).
- **Make invalid states unrepresentable.** Express an invariant with the strongest tool that fits:
  a type or `NOT NULL` first, then a foreign key or `UNIQUE`, then a `CHECK`, then a trigger,
  then application code. This is [Fix the design, not the symptom](REPO_STYLE.md#core-philosophies)
  in SQL.
- **Types carry meaning.** A column whose values come from a closed set of words gets a type that
  names that set.
- **Measure before adding cost.** Justify an index, a denormalized column, or a partition with an
  `EXPLAIN (ANALYZE, BUFFERS)` on representative data ([PYTEST_STYLE.md](PYTEST_STYLE.md) applies
  the same rule to tests).
- **Pre-production means edit the base schema.** Until the first approved production deployment,
  change the owning module under `schemas/base_schema/` directly and reinstall from clean
  (HUMAN_GUIDANCE.md, "Codebase development rules"). Use that freedom to improve foundational
  schemas, contracts, abstractions, and ownership boundaries.

## Table shape

### One fact, one place

- Derive a value that follows from other stored columns: use a `GENERATED ALWAYS AS (...) STORED`
  column when the derivation is row-local and hot, a view otherwise.
- Remove a column whose CHECK admits exactly one value. When a future feature needs that value,
  that feature adds the column with its approved design (HUMAN_GUIDANCE.md:89).
- Reach a parent's value through the foreign key on the same row. `notification.event_id` gives
  the event's kind through one join; the row stores the id and reads the kind.
- Keep one copy of each payload. When a saved value is finalized, mark the row finalized in place
  or move it.

### Snapshots: copy by reference

Human Guidance requires some facts to be frozen at a moment: the exact Question Revision delivered,
the Assessment policy in force when an Attempt started (HUMAN_GUIDANCE.md:844-848). A frozen fact is
still stored once.

- Freeze by referencing an immutable row: `assessment_attempt.policy_id ->
  assessment_policy_snapshot`, one reference per Attempt.
- A snapshot table is immutable, content-addressed (`UNIQUE` on a SHA-256 of its canonical
  content), and shared: two Assessments with the same policy share one snapshot row.
- Freeze exactly what HUMAN_GUIDANCE.md:1556 names: "the additional historical Student Work data
  needed to interpret or grade that work correctly." Read display text, disclosure settings, and
  titles through the reference.
- Copy an *identity* (a plain column carrying an id, without a foreign key) only when the referenced
  row may legitimately be replaced while the copy must survive, and say so in a comment at the
  column. `issued_question.assessment_entry_id` is the existing example.

### Row-multiplying tables stay narrow

Tables whose row count is Students x Assessments x Questions x Attempts (Issued Question, Question
Attempt, saved response, notification fan-out) keep to this budget:

- ids, enums, numbers, and one `timestamptz` per event that happened on that row;
- free text lives on the parent;
- a column group shared by sibling rows (toolchain versions, policy bundles) lives in one
  referenced row.

A narrow table that needs 15 columns is two tables.

### Union tables

A table that holds two kinds of row (fixed Question entry vs Pool entry) with a `kind` column and
nullable columns paired by a CHECK is fine for a small current-state table. Before any other table
snapshots it, split it into a parent with the common columns and one child table per kind.

### Current state, revisions, events

Three table roles, three shapes:

| Role | Mutability | Concurrency token | Timestamps | Examples |
| --- | --- | --- | --- | --- |
| Current state | updated in place | `*_edit_number bigint` (HUMAN_GUIDANCE.md "Common revision and history specifications") | `created_at`, `updated_at` (or `_on`) | `assessment`, `course_instance`, `draft_question`, `question_pool` and its member list; the lineage rows `published_question` (with its metadata) and `blueprint_course`, whose name, tags, classification, and availability change without a Revision |
| Revision | immutable after insert | identified by `(lineage_id, revision_number)` | one `published_on`/`saved_on` (`date`) | `question_revision`, `blueprint_course_revision` |
| Event / receipt | append-only | none needed | one `occurred_at`/`accepted_at` | `*_event`, `*_receipt` |

- The Edit Number is the one concurrency mechanism. If a guessable counter is ever a security
  concern, record the exception in [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) and apply it
  uniformly.
- An event row records what changed and who did it, and references the current row. When the
  current values are exactly the latest event, the current table projects from the event log.
- Notification fan-out is one `(event_id, recipient_id)` row plus the per-recipient state the
  product requires (for example `read_at`).

## Identity

Human Guidance fixes the identity model (HUMAN_GUIDANCE.md, "Human-facing reference IDs"). One
convention, three cases:

| Object | Primary key | Foreign keys reference |
| --- | --- | --- |
| Has a public ID (Account, Course Instance, Blueprint Course, Assessment, Published Question, Question Pool) | the public ID itself: `<entity>_id text PRIMARY KEY` typed by the matching public-ID domain | the public ID |
| Internal aggregate (Assessment Attempt, Draft Question, workspace, job, notification) | `<entity>_id uuid`; Student Work tables key as `(course_instance_id, <entity>_id)` per [Partition readiness](#partition-readiness) | the uuid, with `course_instance_id` for Student Work |
| Owned child (Revision, Pool member, Blueprint module, Issued Question position) | composite natural key: `(published_question_id, revision_number)`, `(question_pool_id, member_position)` | the composite |
| Content-addressed snapshot | the SHA-256 of canonical content: `<entity>_id ple_data.sha256_digest PRIMARY KEY` | the digest |

- The public ID qualifies as a key because it is permanent, issued once, and stored in exactly
  one canonical form (HUMAN_GUIDANCE.md, "Human-facing reference IDs"). Using it as the PK gives
  one identity, one index, 10-11 bytes instead of 16, and the same value in SQL, Rust, JSON, and
  URLs.
- Each table has exactly one key per row: the public ID, the uuid, or the composite. A
  `bigint GENERATED ALWAYS AS IDENTITY` column appears only for a documented internal workflow
  that needs a dense counter, stays internal to the database, and is referenced by nothing.
- Revision numbers are `integer` everywhere; a lineage will see far fewer than two billion
  revisions.
- Random keys (uuid, Crockford public IDs) cost roughly 40% more index space than sequential keys
  (Angelakos, "Putting UUIDs everywhere"). That is the accepted price of opaque identity. A table
  with a natural key uses it. If insert cost is ever measured as a problem, time-ordered UUIDs
  (UUIDv7) are the fix for internal aggregates.

## Types

| Meaning | Type | Replaces |
| --- | --- | --- |
| An instant the server enforces, orders, or audits | `timestamptz` at full precision | `timestamp`, `timetz`, `time`, `timestamptz(n)`, integer epochs |
| A calendar date (terms, authored-content creation, deadlines entered as dates) | `date` | `text`, `timestamptz` at midnight |
| A duration | `interval`, or `integer` seconds with a `_seconds` suffix | `text` |
| Free text | `text` with a `CHECK` on length, whitespace, and control characters, or a domain | `varchar(n)`, `char(n)` |
| A closed vocabulary that only grows | `CREATE TYPE ... AS ENUM` | `text CHECK (col IN (...))` |
| A closed vocabulary whose values retire or carry attributes | a reference table with a `uuid` or `text` key | enum, `text CHECK` |
| Points, credit fractions, multipliers | `numeric` with a `scale()` CHECK | `real`, `double precision`, `money` |
| A checksum | `bytea` with an `octet_length` CHECK | hex `text` |
| A public ID | a `text` domain carrying the canonical-shape and checksum CHECK | per-column regex copies |
| An opaque document owned by another system (backend state, encrypted payloads) | `bytea` or `jsonb` | relational data inside JSON |
| A small set of free tags | `text[]` with a validity CHECK | a comma-joined `text` |
| A yes/no fact | `boolean NOT NULL` | `text` in ('yes','no'), a nullable boolean |
| A generated integer | `GENERATED ALWAYS AS IDENTITY` | `serial` |

Rules behind the table:

- `timestamptz` is the instant type. It supports time arithmetic across zones and DST and costs
  the same 8 bytes as the naive type (Angelakos, "TIMESTAMP (WITHOUT TIME ZONE)").
- `text` plus `CHECK` gives the same storage as a sized varchar, rejects oversize values loudly,
  and lets the limit change by replacing a constraint (Angelakos, "VARCHAR(n)").
- An enum is 4 bytes on disk, sorts by declaration order, and rejects values outside the set at
  the type level. Values can be added (`ALTER TYPE ... ADD VALUE`, committed before use); removing
  or reordering means recreating the type. Use enums for vocabularies that are closed by product
  definition (roles, assessment types, feedback release rules, scoring rules) and a reference
  table for vocabularies that retire members or carry data (themes, disciplines).
- Declare each enum once in `schemas/base_schema/10_types.sql` and use that one type in every
  table that stores the vocabulary.
- A `CREATE DOMAIN` over `text` carries a CHECK shared by many columns: public IDs, SHA-256 hex,
  bounded names, IANA time zone names. Change the rule in one place.
- `jsonb` stores documents PLE reads whole: backend-owned state, signed receipts, a Blueprint
  Revision content body. The moment a query joins on a JSON key or a CHECK inspects one, promote
  that key to a column (Angelakos, "Relational JSON").

## Constraints

Order of preference for expressing an invariant:

1. The column type (enum, domain, `NOT NULL`).
2. `FOREIGN KEY`, including composite keys that bind tenant, owner, or revision together.
3. `UNIQUE` on the natural key.
4. Row-local `CHECK`.
5. A `BEFORE` trigger, for what 1-4 cannot express (state transitions, cross-row rules).
6. Application code.

Specific rules:

- Declare every column `NOT NULL`. When absence is a real domain state, make the column nullable
  and add a comment or a CHECK that says what NULL means.
- Give every reference a foreign key. A copied identity (see
  [Snapshots](#snapshots-copy-by-reference)) carries a comment at the column instead.
- Bind the owner, not just the id: `FOREIGN KEY (assessment_id, course_instance_id) REFERENCES assessment
  (assessment_id, course_instance_id)` proves the Assessment belongs to the Course the row claims.
- **Role-typed foreign keys are the standard way to require a Product Role.**
  `FOREIGN KEY (actor_account_id, actor_role) REFERENCES account (account_id, product_role)` with
  `actor_role` fixed to one enum value makes PostgreSQL enforce "this Account is an Instructor"
  declaratively. Keep the carrier column and give it the enum type.
- Keep every CHECK row-local. Put cross-row rules in a trigger or a deferred constraint, because a
  CHECK is evaluated only when its own row changes.
- Express circular ownership (a Revision that has exactly one Event; an Entry that owns exactly
  one fork) with paired `DEFERRABLE INITIALLY DEFERRED` foreign keys.
- Enforce immutability with privileges: an immutable table has `REVOKE UPDATE` for every runtime
  role and an RLS policy `FOR UPDATE ... WITH CHECK (false)`. A partially mutable row uses one
  generic trigger that compares `to_jsonb(NEW) - mutable_columns` with
  `to_jsonb(OLD) - mutable_columns`, so a newly added column is immutable by default.
- Give a state column (lease state, lifecycle state) exactly one CHECK that pairs it with its
  companion columns (`leased` implies a lease token and expiry) and one trigger for the
  forward-only transition. Tables that share a state machine share its shape.

## Indexes

- Primary keys and `UNIQUE` constraints create their own indexes.
- A foreign key creates an index on the referenced side only. Add a referencing-side index
  whenever the parent is deleted, purged, or unreleased through the child (every cascade path in
  Student Work and every Account purge path), or the child is queried by the parent (Course ->
  Memberships, Assessment -> Attempts). Small, write-once parents (vocabularies, provided
  avatars) are served by their primary key.
- Justify every other index with an `EXPLAIN (ANALYZE, BUFFERS)` on seeded data, recorded in the
  changelog entry that adds it. An index is a write cost on every insert and a maintenance cost
  forever.
- Queue and outbox tables use a partial index on the pending predicate (`WHERE processed_at IS
  NULL`), keyed by the claim order.
- Order composite index columns by equality predicates first, then range, then sort.

## Every table has a clock

- Every table has exactly one creation clock, `NOT NULL`, in one of two types:
  - `timestamptz` at full precision for anything the server enforces, orders within a day, or
    audits: Student Work (`started_at`, `submitted_at`, `expires_at`), sessions, leases, queues,
    events (`occurred_at`), receipts (`accepted_at`), relationship rows (`starred_at`,
    `watched_at`), Course Instances and Accounts (`created_at`; retention math and security audit
    read the instant).
  - `date` for content Instructors author, where the day is the fact and the revision number
    orders the lineage: Published Questions, Question Pools, Blueprint Courses, their Revisions
    (`published_on`, `saved_on`), Draft Questions (`created_on`), and the anonymous statistics
    aggregate (`created_on`). A `date` is 4 bytes and reads as a human expects.
- `timestamptz` keeps full precision. Every precision from 0 to 6 stores the same 8 bytes and
  compares at the same speed, so the choice is full precision or `date`.
- Rows mutated in place (current state, aggregates) carry a second bookkeeping clock,
  `updated_at` or `updated_on`, of the same type as the creation clock, with
  `CHECK (updated_at >= created_at)`. Immutable rows carry the creation clock alone.
- Every other clock on a table is a domain fact named for its event: `started_at`,
  `expires_at`, `submitted_at`, `due_at`, `term_starts_on`. A domain-fact clock takes the type
  its meaning needs regardless of the row's role: deadlines and lease expiries are
  `timestamptz`; term boundaries are `date`.
- Anonymous aggregate tables (`question_revision_statistics`) carry `created_on` and
  `updated_on`, both `date`; `updated_on` is the day of the most recent increment. FERPA attaches
  to identifiability; an increment `timestamptz` on a global aggregate becomes identifying when
  joined to a roster, while a calendar date on a global counter does not. Per-observation
  evidence lives under Student Work and is purged with it. [FERPA_DATA_POLICY.md](FERPA_DATA_POLICY.md) ("Question Library object usage statistics") lists the
  collected facts and the aggregate's exact columns.
- The creation instant is a column, separate from the key. PostgreSQL heaps have no clustered
  primary key, so the identity model in [Identity](#identity) chooses the key on its own terms.

## Organization of the SQL source

A human audits the structure by reading table definitions alone. The source is layered by kind
first, then by domain:

```text
schemas/base_schema/
  install.sql                      ordered \\ir manifest
  00_roles.sql                     cluster roles and schema ownership
  10_types.sql                     every enum and domain, one definition each
  15_table_check_functions.sql     IMMUTABLE helpers used by table CHECKs
  20_tables/<domain>.sql           CREATE TABLE with inline NOT NULL, CHECK, FK, UNIQUE; COMMENT ON
  22_reference_grants.sql          REFERENCES grants needed before late FKs
  30_constraints.sql               late and circular foreign keys
  40_indexes.sql                   every explicit index, each with its justification comment
  50_functions/<domain>.sql        functions, procedures, trigger functions, and CREATE TRIGGER
  60_policies.sql                  include of 60_policies/<domain>.sql
  70_grants.sql                    include of 70_grants/<domain>.sql
```

Rules for `20_tables/`:

- One file per aggregate, named for the root concept (`assessment_attempt.sql` holds every
  Student Work table: Attempt, Pool selection, Issued Question, Question Attempt, saved
  response, submission). A child table lives with its owner.
- Inside a file, tables appear in dependency order, root first. Each table's `COMMENT ON TABLE`
  begins with its role tag (`role: current state`, `role: revision`, `role: event`,
  `role: snapshot`, `role: student work`, `role: aggregate`, `role: vocabulary`), then states
  what deletes its rows and the Human Guidance section it implements.
- `COMMENT ON TABLE` for every table and `COMMENT ON COLUMN` for every column whose meaning goes
  beyond its name and type. Catalog comments appear in `\d+` and in the generated docs.
- A table file contains `CREATE TABLE` and `COMMENT ON` statements. A `CREATE TRIGGER` binding
  lives next to its trigger function in `50_functions/`.
- The manifest lists `20_tables/` files in dependency order; `50_functions/` files follow in any
  order once all tables and constraints exist.

Generated documentation: `docs/SCHEMA_TABLES.md` is produced from the installed catalog (one
section per domain file: table, columns with type and nullability, constraints, FKs, indexes,
catalog comments) so a structural audit reads one document. Regenerate it with the changelog
entry that changes any table.

Enforcement is one Python maintainer tool, `schema_style/check_schema_style.py`, run directly
(`source source_me.sh && python3 schema_style/check_schema_style.py`). It checks the current
`schemas/base_schema/` source today (layout, key names, types, constant columns, duplicate IN
lists, clocks, unindexed FKs). Role-tag rules and catalog-only identity/immutability/null
rules join once comments and `schemas/catalog_snapshot.json` (or `--database`) exist, using
the role tag that begins every table comment
(`role: current state | revision | event | snapshot | student work | aggregate | vocabulary`).
Gate: the schema audit is complete when this layout is in place, every table has a tagged
catalog comment, `docs/SCHEMA_TABLES.md` and the snapshot are regenerated, the checker exits
0 with no findings, and a fresh install passes. Review accepts a new table when it
sits in `20_tables/` with a tagged comment and the checker is clean.
`rule_14_unindexed_fk` fails the checker when any foreign key lacks a covering
index. PRIMARY KEY and UNIQUE constraints cover FKs whose columns lead those
keys; remaining FKs have explicit referencing-side indexes in
`schemas/base_schema/40_indexes.sql`. There are no intentional unindexed-FK
exceptions.

## Partition readiness

Student Work multiplies as Courses x Students x Assessments x Questions x Attempts. At 50
Courses per semester, 100 Students, 20 Assessments, 25 Questions, and an average of five Attempts
(practice after full credit, retries below it), Issued Questions, Question Attempts, responses,
and grading rows each grow by about 12.5M per semester. The 365-day FERPA purge holds each table
to a steady state of roughly 25-40M rows, which PostgreSQL serves with plain B-trees. Partitioning
is cheap to add later only when the keys already include the partition column, and the per-Course
purge (about 250k rows per Course) benefits from `course_instance_id`-led keys today, so Student Work is
built partition-ready from day one:

- The partition key is `course_instance_id`. FERPA purge and Unrelease are per-Course, so a Course
  partition turns the two largest deletes into `DETACH` / `DROP PARTITION`; retention by term
  follows from `course_instance.created_at`.
- Every Student Work table (Assessment Attempt, Pool selection, selected item, Issued Question,
  Question Attempt, saved response, grading result, and their audit targets) carries
  `course_instance_id NOT NULL` and binds it in its FK to the parent. This is the
  [bind the owner](#constraints) rule applied to Student Work.
- Every `PRIMARY KEY` and `UNIQUE` on those tables leads with `course_instance_id`:
  `PRIMARY KEY (course_instance_id, assessment_attempt_id)`, `UNIQUE (course_instance_id,
  student_record_id,
  assessment_id, attempt_number)`. PostgreSQL requires the partition key in every unique
  constraint of a partitioned table and in every FK that targets one.
- Partition on a measured trigger: a Student Work table above ~100M rows, a per-Course purge
  measured in seconds, or autovacuum lagging on `question_attempt`. The change is then
  `CREATE TABLE ... PARTITION BY LIST (course_instance_id)` (or HASH for spread) plus `ATTACH`, with the
  keys already in place.
- Shared content (Questions, Pools, Blueprints, Courses) grows by authoring and stays in plain
  tables.

## Naming

Follows [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md); the database-specific additions are:

- Table: singular noun, `snake_case`, named for the domain concept in
  [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) (`assessment_attempt`, `course_instance`).
- Primary key: `<table>_id`, using the full table name (`course_instance_id`,
  `content_discipline_id`, `published_question_id`).
- Foreign key: `[<role>_]<parent_table>_id`, using the parent's full table name, so the column
  name alone says which table it joins: `course_instance_id`, `object_record_id`,
  `authoring_workspace_id`, `actor_account_id`, `source_course_instance_id`. A composite FK
  carries the same names: `(published_question_id, revision_number)`. JSON and URL field names
  belong to the API contract and are mapped at the boundary.
- `timestamptz` columns end in `_at`; `date` columns end in `_on`; durations end in `_seconds`
  or are `interval`.
- Enum-typed columns are named for the vocabulary: `product_role`, `late_work_rule`, `event_kind`.
  A state column carries its qualifier: `lease_state`, `retention_lifecycle_state`.
- Event tables: `<entity>_<verb>_event` (`question_availability_event`). Snapshot tables:
  `<entity>_snapshot`. Receipt tables: `<operation>_receipt`.
- Booleans name the positive fact: `promoted`, `changed`, `visible`.

## Is my table well designed?

Run this before a table enters `schemas/base_schema/` and again in review. Each item has a
question, the signal that it fails, and the fix. A table passes when every row reads OK.

| # | Question | Fails when | Fix |
| --- | --- | --- | --- |
| 1 | What is this table? | The HUMAN_GUIDANCE.md concept is unnamed, or its role (current state, revision, event) is unclear. | Name it, pick one role, take that role's shape from [Current state, revisions, events](#current-state-revisions-events). |
| 2 | Is every column a fact only this row can state? | A value is reachable by one join through an FK on the same row; a value is computable from other columns; a CHECK admits exactly one value. | Drop the column; derive it; or, when it must be frozen, cite the guidance line and reference a snapshot row. Constant CHECK: `rule_2_constant_columns`. |
| 3 | Is every copied value a deliberate snapshot? | Columns were copied from a parent "so history survives" without a snapshot table. | Content-addressed immutable snapshot row, referenced by id. |
| 4 | Do types say what the data is? | A `text` column has a `CHECK (col IN (...))`; a `timestamp` lacks a zone; a `varchar(n)`; a hex checksum in `text`; relational data inside `jsonb`. | Enum or reference table; `timestamptz`; `text` + CHECK or a domain; `bytea`; real columns. `rule_4_types`. |
| 5 | Is each rule written once? | The same literal list, regex, or length CHECK appears on more than one column. | One enum type or one domain in `10_types.sql`. `rule_5_duplicate_literal_sets`. |
| 6 | Is every reference a foreign key? | A `*_id` column lacks `REFERENCES`; an FK to a parent id leaves the row's tenant, owner, or revision unbound. | Add the FK; make it composite so it binds the owner. A documented copied identity is the one exception. |
| 7 | Does each key column name its table? | A PK is named other than `<table>_id`; an FK column's name ends in something other than the parent's full table name plus `_id` (`course_id` for `course_instance`, `object_id` for `object_record`, `thread_id`). | Rename to `[<role>_]<parent_table>_id`; map API field names at the boundary. `rule_7_key_names`. |
| 8 | Is identity the repository convention? | A public object's PK differs from its public ID; an internal aggregate's PK is something other than `uuid`; an owned child lacks a composite natural key; a table carries a second surrogate such as an identity `bigint`; a revision number differs in width from its siblings. | Public ID as PK, `uuid` for internal aggregates, composite natural keys for children, one key per table, `integer` revision numbers. `rule_8_identity` (Tier 3). |
| 9 | Does `NULL` mean something? | A nullable column lacks both a comment and a CHECK pairing it with the state that makes it absent. | `NOT NULL`, or a CHECK that names when it is NULL. `rule_9_null_meaning` (Tier 3). |
| 10 | Is it narrow enough for its row rate? | The table multiplies with Student Work and has free text, a closed-vocabulary `text`, or a column group repeated across siblings. | Factor the group into a referenced row; move text to the parent. |
| 11 | Is it partition-ready? | A Student Work table lacks `course_instance_id`, or its PK / UNIQUE constraints start with another column. | Add `course_instance_id NOT NULL`, bind it in the parent FK, lead every key with it. `rule_11_student_work_keys` (Tier 2). |
| 12 | Is immutability enforced by privilege? | A trigger enumerates `NEW.x IS DISTINCT FROM OLD.x`. | `REVOKE UPDATE` plus RLS `WITH CHECK (false)`, or the one generic `to_jsonb` trigger. `rule_12_immutability` (Tier 3). |
| 13 | Is the state machine declared once? | Two CHECKs disagree about a state; the same lease or lifecycle shape exists on another table with different column names. | One pairing CHECK, one transition trigger, one shared shape. |
| 14 | Which FK edges need an index? | A child of a table that is purged, unreleased, or closed lacks an index on the referencing columns; a hot parent-to-child lookup lacks one. | Add the referencing-side index; record the `EXPLAIN` that justified any other index. `rule_14_unindexed_fk` fails the checker; no unindexed-FK exceptions. |
| 15 | What deletes these rows? | The delete path (FERPA purge, Unrelease, retention, or none) is unnamed, or it is neither an FK cascade nor an explicit statement. | Name the path in the module comment; make it cascade or explicit. |
| 16 | Does it have a clock? | The table lacks a `NOT NULL` creation clock; the clock is `timestamptz(n)`, an integer epoch, or `date` on an enforced/ordered/audited row; a current-state table lacks `updated_at`; an incremental integer key lacks a documented need. | Add the clock in the type its role requires; drop the counter. `rule_16_clock_present`, `rule_16_updated_clock`, `rule_16_clock_type`. |
| 17 | Is it where a reader expects? | The table sits outside `20_tables/<aggregate>.sql`, beside functions or grants, or lacks `COMMENT ON TABLE`. | Move it; comment it; regenerate `docs/SCHEMA_TABLES.md`. `rule_layout` / `rule_17_role_tag`. |
| 18 | Does it install and seed? | Fresh `install.sql` fails; the Live Demo seed fails; the changelog entry is missing. | Fix, then log. |

### Worked example

`ple_private.assessment_attempt` as of the 2026-09-17 audit, scored against the checklist:

| # | Result | Evidence |
| --- | --- | --- |
| 1 | OK | Assessment Attempt, Student Work root, current state with an immutable start. |
| 2 | FAIL | `assessment_title`, `assessment_instructions`, and nine policy strings are reachable through `assessment_id` at start time. |
| 3 | FAIL | Frozen by copying 15 columns; no snapshot table. |
| 4 | FAIL | Nine `text CHECK (... IN (...))` columns. |
| 5 | FAIL | The five-value feedback list is written six times here and 19 times repo-wide. |
| 6 | OK | Every reference is an FK; accommodation FKs bind `(student_record_id, assessment_id)`. |
| 7 | FAIL | Children carry `assessment_attempt_id` (fine) but the row itself will carry `course_id` for a `course_instance` parent. |
| 8 | FAIL | Carries an unused `reference_number bigint` identity beside the uuid PK. |
| 9 | OK | Every nullable column is paired by a CHECK. |
| 10 | FAIL | 50,000 rows per semester (100 Students x 20 Assessments x 5 Attempts x 50 Courses) each carrying up to 50 KB of instruction text. |
| 11 | FAIL | Lacks `course_id`; FK is to `assessment_id` alone; PK is `assessment_attempt_id` alone. |
| 12 | n/a | Mutable row; no immutability trigger. |
| 13 | OK | One accommodation pairing rule per accommodation kind. |
| 14 | FAIL | `assessment_id` and the three accommodation FKs lack a referencing-side index; Unrelease deletes through this table. |
| 15 | OK | Unrelease and FERPA purge cascade from this root (`unrelease.sql`, `course_retention_transitions.sql`). |
| 16 | FAIL | Has `started_at` but carries an unused `reference_number bigint` identity. |
| 17 | FAIL | Lives in `assessment_attempts.sql` beside 11 functions and 13 triggers; its children are split across three other files; lacks `COMMENT ON`. |
| 18 | OK | Installs and seeds. |

Eleven failures, all fixed by the plan's M0 (organization), M1 (enums, identity, naming, clock), M2
(snapshot and partition-ready keys), and M4 (indexes). A table that passes this checklist on day
one costs nothing to keep passing; one that fails it after a semester of data costs a migration.

## Sources

- PostgreSQL 17 documentation: [Data Types](https://www.postgresql.org/docs/current/datatype.html),
  [Enumerated Types](https://www.postgresql.org/docs/current/datatype-enum.html),
  [Domains](https://www.postgresql.org/docs/current/domains.html),
  [Constraints](https://www.postgresql.org/docs/current/ddl-constraints.html),
  [Generated Columns](https://www.postgresql.org/docs/current/ddl-generated-columns.html),
  [Indexes](https://www.postgresql.org/docs/current/indexes.html).
- Jimmy Angelakos, *PostgreSQL Mistakes and How to Avoid Them* (Manning, 2025): chapter 3
  "Improper data type usage" (timestamps, `char`/`varchar`, `money`, `serial`), chapter 5
  "Improper feature usage" (relational JSON, UUIDs everywhere), chapter 6 (indexes).
- *Mastering PostgreSQL: From Basics to Expert Proficiency* (2024), chapter 5 "Database Design
  and Normalization": normalize first; denormalize for a measured read-path need, and account
  for the update anomalies it introduces.
- The `postgresql-expert` skill's schema quality review reference, which supplied the audit
  method behind these rules.
