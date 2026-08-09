# Plan: Build an adaptable, secure, and Blackboard-informed database

## Context

PLE has no durable production data yet. The database can therefore be consolidated into a clean
baseline before the first non-disposable database exists.

The database must serve several legitimate perspectives at once:

| Perspective | Database need |
| --- | --- |
| Instructor | Change points, dates, policies, and remove and regrade bad questions |
| Student | Stable active attempts, fair recalculation, and recovery from technical problems |
| Problem author | Immutable published content, drafts, new versions, and provenance |
| Problem finder | Human-readable IDs and discovery that is not restricted by subject |
| Grader | Raw responses, deterministic evaluation, manual grading, and recalculation |
| Support staff | Force-submit, clear attempts, access logs, and actionable statuses |
| Analyst | Rebuildable item analysis without slowing operational queries |
| Security and privacy | Tenant isolation, least privilege, retention, and access auditing |
| Operations | Shard-ready tenant keys, bounded partitions, and current summaries |
| Import and export | QTI provenance, partial failures, validation, and duplicate warnings |

The design must preserve adaptability without creating assignment-history or scoring-history cruft.
The unifying persistence model is:

- **Immutable facts:** published problem versions, submitted responses, and the questions actually
  delivered in a run.
- **Mutable current state:** assignments, point values, schedules, policies, exclusions, and current
  grades.
- **Replaceable projections:** grade summaries, search documents, and item-analysis results.
- **Minimal audit evidence:** sensitive actions and access, without copies of obsolete assignments
  or scores.

## Objectives

- Produce a clean PostgreSQL baseline before durable data exists.
- Version published problems without versioning ordinary assignment edits.
- Let instructors change points, schedules, time limits, and policies after publication.
- Support Blackboard Original-style **Delete and Regrade** after submissions exist.
- Recalculate all attempts and retakes when current scoring changes; old computed scores disappear.
- Give every published problem a copyable decimal ID while retaining internal UUIDv7 identities.
- Make global, cross-subject problem discovery a primary database concern.
- Protect FERPA education records through tenant isolation and least privilege.
- Keep private records tenant-shardable and high-volume data predictably partitioned.
- Replace the custom migration registry with a maintained raw-SQL migration system.

## Design decisions and alternatives

| Approach | Advantages | Disadvantages | Decision |
| --- | --- | --- | --- |
| Relational PostgreSQL plus versioned JSONB | Strong integrity, predictable queries, RLS, partitioning, and flexible question payloads | Requires deliberate migrations | **Use** |
| Current custom SQL migration registry | Already works and verifies checksums | Manual registration and permanent custom migration machinery | Replace with SQLx migration support |
| ADAPT-style executable migrations and cloned revision tables | Flexible application-driven changes | Model coupling, irreversible behavior, and accumulated versioning cruft | Do not adopt |
| Sinedon runtime schema generation | Excellent universal IDs, timestamps, and typed records | Runtime DDL is unsafe for controlled production evolution | Adopt its record conventions only |
| Xmipp or EAV-style flexible metadata | Fields can be added without normal migrations | Weak constraints, slower filtering, and difficult security classification | Limit this flexibility to problem JSONB |
| Full event sourcing | Complete historical replay | High operational complexity and explicitly unwanted history | Do not adopt |

Use relational columns for authorization, lifecycle, points, dates, constraints, joins, sorting,
frequent filters, retention, and partitioning. Use JSONB only for cohesive problem definitions and
adapter-specific metadata. Every durable JSONB contract has a schema version and checksum.

## Identity and common record rules

- Internal distributed identifiers remain UUIDv7.
- Every durable entity receives an internal ID and database-authored `created_at`.
- Mutable current-state rows also receive `updated_at` and an optimistic-concurrency `revision`.
- Events receive database-authored `occurred_at`.
- Pure join tables use composite keys unless another record must address the relationship directly.
- Point values and credit calculations use fixed-precision `NUMERIC`, not floating point.
- Tenant-owned keys, foreign keys, and important indexes begin with `tenant_id`.

This preserves the useful part of Sinedon's required identity and timestamp convention without
giving every pure relationship a meaningless surrogate ID.

### Human problem IDs

- `problem.problem_id UUID` remains the internal key.
- Add `problem.public_id BIGINT GENERATED ALWAYS AS IDENTITY`, unique and never reused.
- Display a stable problem as `P-123456`.
- Display an exact version as `P-123456-v3`.
- Importing `P-123456` resolves the latest assignable published version and shows the instructor
  which version will be pinned.
- Importing `P-123456-v3` selects that exact published version.
- Sequential IDs are acceptable for catalog content, but never serve as authorization credentials
  or student-record identifiers.

## Catalog, authoring, and discovery

### Published problems

- `problem` stores stable identity, ownership, visibility, license, lifecycle, and public ID.
- `problem_version` stores version number, schema version, checksum, searchable metadata, and
  provenance.
- `problem_version_payload` stores the immutable normalized question definition separately from hot
  search metadata.
- Published versions are immutable. Corrections create a new version.
- Drafts remain private workspace records and receive no public problem ID until publication.

Question-type-specific structures, such as fill-in-multiple-blanks match rules, remain inside the
versioned normalized question payload. They do not require a generic EAV schema.

### Reuse semantics

Replace Blackboard's mutable "copy or link" behavior with two explicit operations:

- **Use existing:** pin an immutable published problem version.
- **Fork:** create a new private draft with lineage back to the source version.

Editing a published catalog problem never silently changes every assignment using it. This is an
intentional departure from Blackboard's linked pool questions. Local evidence:
`OTHER_REPOS/Blackboard_Learn/question-pools.md`.

### Collections and randomized groups

- `problem_collection` is a reusable saved collection or pool with no point values.
- Collections may be private, institution-shared, or public.
- `assignment_selection_group` represents a question set or random block.
- `assignment_selection_candidate` pins eligible problem versions.
- A group stores draw count, uniform points per selected question, ordering or randomization policy,
  and selection algorithm version.
- Search criteria help instructors build the candidate list but are not executed live when a
  student begins an attempt.
- Each run stores the candidates actually selected, their order, and the seed.

This supports the behaviors described in the local Blackboard evidence files
`question-sets.md` and `random-blocks.md` without making historical runs depend on current pool
membership.

### Discovery

Problem discovery is global by default:

- Empty search returns all visible subjects.
- Subject is an optional many-to-many taxonomy facet, not a required filter or partition key.
- Search supports human ID, title, text, author, question type, category, topic, difficulty,
  keywords, capability, language, license, source, lifecycle, and quality signals.
- Maintain a denormalized `catalog_search_document` projection with full-text and trigram indexes.
- Use cursor pagination rather than offset pagination.
- Keep search behind a repository boundary so an external search engine can later consume catalog
  changes without becoming authoritative.
- Public usage statistics are aggregated and disclosure-thresholded; discovery never joins directly
  to student records.

This deliberately improves on Blackboard's course-local, initially blank discovery workflow. Local
evidence: `OTHER_REPOS/Blackboard_Learn/reuse-questions.md` and
`OTHER_REPOS/Blackboard_Learn/question-settings-and-metadata.md`.

## Assignment model

`assignment` is one mutable current-state record containing:

- Tenant and course ownership.
- Lifecycle: `draft`, `published`, `closed`, or `archived`.
- Title and instructions.
- Visibility, availability start, due date, and close date.
- Late-submission policy.
- Assignment time limit and auto-submit behavior.
- Attempt limit and current attempt-selection policy.
- Gradebook inclusion or practice-only status.
- Presentation, randomization, and backtracking policy.
- Feedback-disclosure rules.
- Current `revision`, `scoring_generation`, and `scoring_status`.

Do not create `assignment_version`, assignment-history payloads, or scoring-revision tables.

### Assignment items

`assignment_item` contains:

- Stable UUID.
- Tenant and assignment IDs.
- Pinned problem and version IDs.
- Current position.
- Current `points_possible`.
- `delivery_state`: `active` or `retired`.
- `scoring_mode`: `normal`, `full_credit`, `extra_credit`, or `excluded`.

Problem-authored points are defaults copied into a new assignment item. The assignment item value is
authoritative afterward.

### Publication and locking

Before the first student run:

- Instructors may freely add, replace, remove, and reorder items.
- Replacing a problem creates a new item rather than mutating the pinned version of an existing item.

After any student run has been issued:

- Adding questions or replacing pinned problem versions is blocked.
- Point values and assignment policies remain editable.
- Reordering affects future runs only.
- Existing and active runs retain their issued question order.
- Fixed items and selection candidates may be removed through **Delete and Regrade**.
- A selection candidate may be removed only when enough active candidates remain to satisfy the
  group's draw count.
- An assignment with student records cannot be physically deleted; it may be closed or archived.

## Delete and Regrade

Match Blackboard Original's behavior documented in the local evidence file
`OTHER_REPOS/Blackboard_Learn/edit-tests-and-questions.md`:

- Permit removal after submitted attempts exist.
- Block removal while an affected attempt is actively in progress.
- Offer zero points or full credit as immediate remediation while removal is blocked.
- Retire the referenced `assignment_item`; do not physically delete it.
- Set it excluded from current scoring and omit it from future attempts and retakes.
- Recalculate every affected attempt as though the question had not contributed.
- Remove its possible points from the applicable denominator.
- Hide it from normal student grade and feedback views.
- Retain the raw submitted response as protected student evidence until its retention deadline.

A retired item is a referential tombstone, not an assignment version. It contains only current
exclusion state; no former point value or score is preserved.

## Grading and recalculation

### Separate correctness from assignment points

- Server-side grading produces a normalized `credit_fraction`.
- The normalized value supports partial or negative credit without embedding assignment points in
  the immutable problem result.
- Current earned points derive from normalized credit, current item points, and current scoring mode.
- `full_credit` substitutes full normalized credit.
- `extra_credit` contributes to the numerator without increasing the normal denominator.
- `excluded` contributes to neither numerator nor denominator.

### Current scoring records

- `submission` retains the raw student response.
- `submission_evaluation` retains the current normalized evaluation and manual-grading status.
- `attempt_score_current` contains one current calculated result per attempt.
- `student_assignment_summary` contains one current computed assignment result per student.
- An optional manual override is separate current state. Recalculation replaces computed values
  without silently deleting an explicit current override.
- Do not retain historical grade projections or append obsolete computed values to `grade_event`.

### Recalculation queue

Point changes, zero points, full credit, exclusion, item retirement, or grade-selection-policy
changes use this process:

1. Update current assignment state and increment `scoring_generation`.
2. Mark current grades `recalculating`.
3. Enqueue one idempotent tenant-scoped recalculation job.
4. Recalculate all submitted attempts and retakes.
5. Reapply the current first, last, highest, or lowest attempt-selection policy.
6. Stage results under the new generation.
7. Atomically replace current attempt and assignment summaries.
8. Discard a worker result if a newer generation exists.
9. Delete staging rows and retain only short-lived operational job status.

While recalculation is pending, stale scores are not presented as current. A failed job remains
retryable and visibly failed; partial new grades are never published. After success, old computed
scores are gone.

## Attempts, timing, and student support

### Run evidence

`assignment_run` and `assignment_run_item` store:

- Student, tenant, assignment, and attempt identity.
- Selected assignment items and problem versions.
- Issued order, randomization seed, and start time.
- Delivery status and timestamps.

They do not snapshot points, due dates, or scoring policies.

### Attempt states

Support current states including:

- `in_progress`
- `submitted`
- `auto_submitted`
- `needs_manual_grading`
- `cleared`
- `exempt`

Authorized instructors may force-submit or clear an attempt as described in the local evidence file
`OTHER_REPOS/Blackboard_Learn/resolve-student-issues-with-tests.md`.
Clearing removes the attempt from current scoring but retains protected evidence and an audit action
until retention expires.

### Timing and availability

- Time limits, availability, due dates, and student or group accommodations are mutable current
  state.
- Changes apply immediately, including to active attempts.
- The server automatically submits at the effective deadline.
- Shortening a limit below elapsed time causes immediate auto-submission.
- Extending a limit increases the active deadline.
- The timer continues independently of browser connectivity.
- Per-student and per-group exceptions may override attempts, timer, and availability.
- The resolved effective policy is recorded for operational explanation without creating assignment
  history.

The auto-submit choice is intentional even though Blackboard also supports an overtime
`needs_grading` mode. Local evidence:
`OTHER_REPOS/Blackboard_Learn/test-and-survey-options.md`.

## Item analysis and reporting

Item analysis is a derived, rerunnable projection based on the local evidence file
`OTHER_REPOS/Blackboard_Learn/item-analysis.md`:

- Difficulty.
- Discrimination.
- Average and standard deviation.
- Graded and unanswered counts.
- Response distribution.
- Completion time.
- Flags for incomplete manual grading or recent rescoring.

Analysis is recalculated after grading changes rather than historically versioned. Operational
grading never waits for analytics. Course-local analysis remains tenant-owned. Cross-course problem
statistics become catalog signals only after aggregation and de-identification.

## Import and export

- Imports use an `import_batch` plus per-item result records.
- Validate archives, file count, expanded size, media type, paths, and symlinks in a worker.
- Preserve source format, source identifier, importer, warnings, and provenance.
- Accept partial batch success while clearly reporting every rejected item.
- Detect exact duplicates by normalized checksum and warn on likely duplicates.
- Imported questions remain drafts until reviewed and published.
- QTI incompatibilities produce explicit warnings; fidelity loss is never silent.
- Export published or authorized draft content without exporting student data or protected answer
  keys to unauthorized roles.

These rules improve on the limitations documented in the local Blackboard evidence files
`import-or-export-tests-surveys-and-pools.md` and `upload-questions.md`.

## FERPA and security

FERPA protects education records but does not define one technical certification checklist. PLE
must enforce least privilege and defensible safeguards.

- Separate catalog, authoring, educational-record, grader-secret, and operational schemas and roles.
- Put `tenant_id` first in every private primary key, foreign key, and important index.
- Enable and force PostgreSQL row-level security with default-deny policies.
- Application connections may not own tables, use superuser privileges, or bypass RLS.
- Tenant context comes only from the authenticated server session.
- Separate migration, application, grader, worker, export, analytics, and retention roles.
- Keep correct answers and grader secrets unavailable to browser and catalog roles.
- Store assignment access codes as slow password hashes, never plaintext.
- Do not place student PII, responses, or grades in queue payloads, logs, cache keys, or public
  analytics.
- Require TLS, managed encryption at rest and for backups, point-in-time recovery, secret rotation,
  and tested retention deletion.
- Audit sensitive actions and record access, but do not store obsolete score values in audit payloads.

## Partitioning and distribution

Maximum distributability means preserving clean routing boundaries, not maximizing partition count.

- Start with one PostgreSQL cluster and separate logical schemas.
- Keep the shared catalog central and read-replicable.
- Make private records tenant-shardable from the first schema through leading `tenant_id` keys.
- Introduce a `tenant_placement` routing contract so tenant records can later move to another cluster
  without changing IDs or APIs.
- Avoid cross-tenant business joins and transactions.
- Monthly range-partition only high-volume append-only attempt, submission, access-log, and audit
  detail.
- Use unpartitioned identity or header tables when necessary to preserve stable IDs and foreign keys.
- Keep current grade summaries directly indexed and unpartitioned within each shard.
- Pre-create partitions and alert on default-partition writes.
- Do not partition by subject or create thousands of per-tenant partitions.
- Gradebook request paths read current summaries and never aggregate attempt history.
- Maintain the hot and cold split for a catalog approaching ten million problems.
- Use queue backpressure and per-tenant fairness for large recalculation jobs.

## Migration system

Keep raw SQL while replacing custom orchestration:

- Use SQLx `Migrator` for migration discovery, locking, and applied-checksum validation.
- Apply migrations only through:
  - `cargo tools database status`
  - `cargo tools database migrate`
  - `cargo tools database verify`
- Use a dedicated migration role.
- Application startup performs a read-only compatibility check and never changes the schema.
- Before durable production data, consolidate existing migrations into one reviewed baseline
  representing this design.
- After the baseline freezes, applied migrations are immutable and forward-only.
- Use expand, backfill, switch, and contract changes for populated tables.
- Do not call mutable application models from migrations.
- Do not use runtime schema generation or ORM-owned DDL.
- Production rollback uses application rollback or a compensating forward migration, not destructive
  down migrations.

### Initial epoch files

The pre-data baseline contains exactly six ordered SQLx migrations. Each file owns a durable domain
boundary rather than a chronological implementation slice:

1. `2026080801_principals.sql`: runtime principals, tenant/session helpers, authentication
   sessions, and the narrow read-only migration-state projection used by application compatibility
   checks.
2. `2026080802_catalog_authoring.sql`: immutable problems and versions, payload and answer-key
   separation, catalog grants and search indexes, source artifacts, private workspaces, and QTI
   staging or published-grading tables.
3. `2026080803_courses_assignments.sql`: courses, membership, assignments, ordered immutable
   problem references, enrollment, current summaries, and optimistic assignment revisions.
4. `2026080804_activity_feedback.sql`: runs, attempts, submissions, current evaluations and
   grade evidence, idempotency receipts, feedback, prefetch, audit events, and external-tool
   exchanges. This file creates the four dynamic high-volume partition families from one bounded
   partition helper rather than committing date-specific child-table dumps.
5. `2026080805_operations_analytics.sql`: protected asset delivery, worker jobs and brokers,
   QTI promotion functions, student exports, and identity-free question-statistics aggregation.
6. `2026080806_retention.sql`: retention policy, scheduling, management receipts, archive access
   fences, typed cleanup manifests, private purge work sets, and the final lease-fenced archive and
   deletion functions.

The baseline expresses final object shapes directly. It contains no historical `ALTER`/`DROP`
repair sequence, milestone-named helper such as `r44a_*` or `r44b_*`, source-string assertion, or
dumped `_sqlx_migrations` table. SQLx creates its own ledger before applying the first file. Roles,
tables, functions, policies, triggers, grants, and revocations remain explicit SQL; generated ORM
DDL is not introduced.

The dedicated migration login is deployment configuration, not an application principal created by
the baseline. It applies SQL through `cargo tools database migrate`. `status` reports known applied
and pending versions without changing the database, while `verify` performs the same read-only
checksum, missing-version, dirty-state, and compatibility checks used by application startup. An
unreachable database may leave the stateless API degraded, but a reachable incompatible schema is a
startup error and never triggers automatic migration.

Permanent tests cover Store behavior, cross-backend conformance, compilation, and the application
compatibility decision. Fresh installation, no-op replay, changed or missing historical SQL,
concurrent migration runners, real role/RLS enforcement, partition pruning, and populated payload
upcasts are one-time PostgreSQL acceptance checks. Their temporary databases and inputs are removed
after evidence is recorded rather than becoming committed fixtures.

## Domain interfaces

Introduce or revise these domain concepts:

- `ProblemPublicId` and `ProblemVersionRef`
- `AssignmentItem`
- `AssignmentSelectionGroup` and pinned candidates
- `AssignmentScoringMode`
- `AssignmentRevision`
- `ScoringGeneration` and `ScoringStatus`
- `AttemptStatus`
- `AssignmentPolicies` and resolved student exception

Instructor commands are explicit domain operations:

- Change points.
- Set zero points.
- Give or remove full credit.
- Mark extra credit.
- Delete and regrade.
- Reorder future attempts.
- Force-submit.
- Clear an attempt.

Each command is tenant-scoped, revision-checked, idempotent where retryable, and validated against
active attempts.

## Verification

- Apply the clean baseline to an empty PostgreSQL database and reapply it as a no-op.
- Reject modified historical SQL, missing migrations, and concurrent migration races.
- Verify published problem immutability and human-ID resolution.
- Verify global discovery without a subject filter and authorization-aware direct-ID lookup.
- Verify publish and edit locks before and after student activity.
- Verify point changes recalculate every submitted attempt and retake.
- Verify zero points, full credit, extra credit, and exclusion denominator behavior.
- Verify Delete and Regrade with submitted attempts.
- Verify deletion is rejected during an active attempt and succeeds after it ends.
- Verify future retakes omit retired questions while existing responses remain protected.
- Verify reordering affects only future runs.
- Verify random selection reproducibility and candidate retirement.
- Verify a newer scoring generation supersedes an in-flight job.
- Verify atomic replacement leaves exactly one current score per attempt and student.
- Verify there are no assignment-history, scoring-revision, or old-grade tables.
- Verify active timer changes and server auto-submit.
- Verify force-submit, clear, and access-log authorization.
- Verify manual-grading and mixed automatic and manual assignments.
- Verify item-analysis recalculation after rescoring.
- Verify import partial failures, provenance, hostile archives, and duplicate warnings.
- Verify forced-RLS cross-tenant denial for every application and worker role.
- Verify partition pruning, current-summary gradebook queries, tenant purge, backup restoration, and
  retention deletion.
- Run focused PostgreSQL behavior tests followed by repository formatting, linting, compilation,
  complete tests, `./check_codebase.sh`, and `pytest tests/` when the plan is implemented.

## Resolved decisions

- There is no durable production data to preserve.
- Published problem content is immutable and versioned.
- Assignments and computed grades retain only current state.
- Assignment items have stable identities but no historical versions.
- Delete and Regrade is supported after submissions exist and blocked only by active attempts.
- All affected attempts and retakes are recalculated; old computed scores disappear.
- Runs retain only the execution evidence needed to identify what was delivered.
- Problem IDs are human-readable decimal catalog IDs backed by internal UUIDv7 identities.
- Discovery is global by default and subject is an optional facet.
- PostgreSQL and explicit SQL remain authoritative.
- SQLx replaces the custom migration registry.
- The baseline is consolidated before durable data exists and becomes immutable afterward.

## Explicit non-goals

- Assignment version history.
- Historical computed scores.
- Globally mutable linked published questions.
- Subject-based catalog silos.
- A database per instructor or course.
- Generic EAV storage for educational records.
- Runtime automatic DDL.
- Anonymous surveys in this database decision; a future activity type may reuse the framework
  without distorting assignment and grading records.
