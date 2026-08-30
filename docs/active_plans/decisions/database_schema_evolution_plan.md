# Plan: Build an adaptable, secure, and Blackboard-informed database

Status: implemented and accepted on 2026-08-08 as the six-file SQLx baseline. The complete fresh
PostgreSQL 17 gate passed again during WP-QTI-11 acceptance on 2026-08-09. This document retains
the accepted baseline design as historical evidence, not as an implementation package to replay.
The baseline is frozen: every later schema change uses a new forward migration. WP-CA3 followed that
rule with the accepted `2026080907_course_appearance.sql` forward migration and a fresh
seven-migration PostgreSQL/RLS gate on 2026-08-09. WP-RC1 acceptance strengthened that migration
with a trigger enforcing exact current-banner delivery kind and legacy scope/course ownership; the real
`ple_app` negative probe and combined PostgreSQL/MinIO cleanup oracle passed.

## Context

Historical pre-baseline context: PLE had no durable production data, so the initial design could be
consolidated into one reviewed SQLx baseline. That consolidation is complete and is no longer an
available schema-evolution path.

The owner anchors for this plan are [Data philosophy](../../HUMAN_GUIDANCE.md#data-philosophy),
[Question content philosophy](../../HUMAN_GUIDANCE.md#question-content-philosophy),
[Course content philosophy](../../HUMAN_GUIDANCE.md#course-content-philosophy), and
[Sysadmin philosophy](../../HUMAN_GUIDANCE.md#sysadmin-philosophy). The binding technical
interpretation is in [DATABASE_AUTHORIZATION.md](../../DATABASE_AUTHORIZATION.md),
[PROBLEM_IDENTITY.md](../../PROBLEM_IDENTITY.md), and the SD1 plans; this document does not alter
Human Guidance.

The database must serve several legitimate perspectives at once:

| Perspective          | Database need                                                                         |
| -------------------- | ------------------------------------------------------------------------------------- |
| Instructor           | Change points, dates, policies, and remove and regrade bad questions                  |
| Student              | Stable active attempts, fair recalculation, and recovery from technical problems      |
| Problem author       | Immutable published questions, drafts, Question ID lineages, versions, and provenance |
| Problem finder       | Human-readable IDs and discovery that is not restricted by subject                    |
| Grading service      | Raw responses, deterministic automated evaluation, and recalculation                  |
| Support staff        | Force-submit, clear attempts, access logs, and actionable statuses                    |
| Analyst              | Rebuildable item analysis without slowing operational queries                         |
| Security and privacy | Exact actor, relationship ownership, least privilege, retention, and access auditing  |
| Operations           | One-installation routing, bounded partitions, typed leases, and current summaries     |
| Import and export    | QTI provenance, partial failures, validation, and duplicate warnings                  |

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

- Historical implementation objective (completed): produce the reviewed PostgreSQL baseline.
- Version published questions without versioning ordinary assignment edits.
- Let instructors change points, schedules, time limits, and policies after publication.
- Support Blackboard Original-style **Delete and Regrade** after submissions exist.
- Recalculate all attempts and retakes when current scoring changes; old computed scores disappear.
- Give every published question one copyable `AAA-BBBB` Question ID while retaining internal UUIDv7 identities and immutable versions.
- Make global, cross-subject problem discovery a primary database concern.
- Protect FERPA education records through `ActorContext`, exact course/Student ownership, forced RLS, and least privilege.
- Keep one installation's high-volume data predictably partitioned and worker work bounded by typed leases.
- Replace the custom migration registry with a maintained raw-SQL migration system.

## Design decisions and alternatives

| Approach                                                     | Advantages                                                                               | Disadvantages                                                             | Decision                                |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | --------------------------------------- |
| Relational PostgreSQL plus versioned JSONB                   | Strong integrity, predictable queries, RLS, partitioning, and flexible question payloads | Requires deliberate migrations                                            | **Use**                                 |
| Current custom SQL migration registry                        | Already works and verifies checksums                                                     | Manual registration and permanent custom migration machinery              | Replace with SQLx migration support     |
| ADAPT-style executable migrations and cloned revision tables | Flexible application-driven changes                                                      | Model coupling, irreversible behavior, and accumulated versioning cruft   | Do not adopt                            |
| Sinedon runtime schema generation                            | Excellent universal IDs, timestamps, and typed records                                   | Runtime DDL is unsafe for controlled production evolution                 | Adopt its record conventions only       |
| Xmipp or EAV-style flexible metadata                         | Fields can be added without normal migrations                                            | Weak constraints, slower filtering, and difficult security classification | Limit this flexibility to problem JSONB |
| Full event sourcing                                          | Complete historical replay                                                               | High operational complexity and explicitly unwanted history               | Do not adopt                            |

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
- Name keys and foreign keys for their actual owner: `user_id`, `workspace_id`, `course_id`, `student_id`,
  `assignment_id`, `run_id`, `question_attempt_id`, or immutable catalog identity.
- Transaction authority is the server-derived `ActorContext { user_id, session_id }`; it is not a caller-
  selected installation scope or database context.

This preserves the useful part of Sinedon's required identity and timestamp convention without
giving every pure relationship a meaningless surrogate ID.

### Current question identity and evolution

[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md#question-content-philosophy),
[PROBLEM_IDENTITY.md](../../PROBLEM_IDENTITY.md#human-facing-question-id), and
[QUESTION_MODEL.md](../../QUESTION_MODEL.md#identity) are the current authority. One stable,
non-sequential Crockford Base32 `AAA-BBBB` `QuestionId` names a published question lineage. Each
publication in that lineage has a fresh immutable `VersionId`; `ProblemVersionRef { problem, version }`
is the hidden exact evidence used by assignments, issued attempts, grading, replay, audit, and
provenance. Browser and instructor-safe projections carry the Question ID and safe metadata, never
the hidden pair as a locator or authority.

- A private draft belongs to an Instructor-owned `WorkspaceId` and has no published identity.
- Initial publication mints the Question ID lineage and first immutable version.
- Editorial/accessibility work or a grading-semantic correction may publish another immutable version
  under the same Question ID when the semantic stewardship rules permit it; an assignment never moves
  automatically.
- A major objective, task, or response-family change is a private fork; publication gives the fork a
  new Question ID, immutable version, and exact visible ancestry to its source.
- `ForcedQuestionCorrection` is the Sysadmin-approved emergency path for only `security_flaw` or
  `critical_correctness_flaw`: the replacement is validated first; approval withdraws the flawed
  version from ordinary new selection and issuance; the flawed version remains immutable historical
  evidence; and a closed manifest records mapping, impact, generation, and deterministic remediation
  without silently swapping issued work.
- Question IDs are not credentials. No request resolves an implicit latest version or selects a hidden
  pair; the server resolves and pins an exact version under the relevant authority.

## Catalog, authoring, and discovery

### Published problems

- `problem` stores one global immutable Question ID lineage and lifecycle; it has no publication-
  scope or institution selector.
- `problem_version` stores one immutable publication's hidden exact evidence, schema version,
  checksum, searchable metadata, and optional one-way provenance.
- `problem_version_payload` stores the immutable normalized question definition separately from hot
  search metadata; answer keys and grading secrets remain in their server-only boundary.
- Published versions are immutable. Allowed compatible improvements and grading-semantic corrections
  create a fresh version in the stewarded lineage; major semantic changes fork to a new Question ID.
- Drafts remain private workspace records until validation and publication; they receive no published
  identity before that transition.

Question-type-specific structures, such as fill-in-multiple-blanks match rules, remain inside the
versioned normalized question payload. They do not require a generic EAV schema.

### Reuse semantics

Replace Blackboard's mutable "copy or link" behavior with explicit operations:

- **Use existing:** select a published Question ID and pin the resolved immutable version.
- **Update:** deliberately opt an affected assignment into a selected exact version through its
  revision-checked operation; existing pins remain authoritative.
- **Fork:** create a new private draft with Question ID/version ancestry back to the source version.
- **ForcedQuestionCorrection:** let a Sysadmin-approved closed manifest stop new use of a flawed
  version and apply deterministic, generation-fenced remediation while preserving issued evidence.

Editing a published catalog question never silently changes every assignment using it. This is an
intentional departure from Blackboard's linked pool questions. Local evidence:
`OTHER_REPOS/Blackboard_Learn/question-pools.md`.

### Collections and randomized groups

- `problem_collection` is a reusable saved collection or pool with no point values.
- Collections may be private, explicitly shared, or public.
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

- Exact `CourseId` ownership through the CourseInstance and current direct Instructor membership.
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
- Exact `CourseId` and `AssignmentId` parent IDs.
- Hidden pinned problem and version IDs for server-side exact replay, grading, audit, and
  provenance; browser summaries use the Question ID and safe display metadata.
- Current position.
- Current `points_possible`.
- `delivery_state`: `active` or `retired`.
- `scoring_mode`: `normal`, `full_credit`, `extra_credit`, or `excluded`.

Problem-authored points are defaults copied into a new assignment item. The assignment item value is
authoritative afterward.

### Publication and locking

Before the first Student run, authorized current course Instructors may add, replace, remove, and
reorder items through the
assignment's strong revision boundary. A focused replacement selects a published Question ID and
pins its selected immutable version while keeping the item's ordinary assignment configuration.

After any Student run has been issued:

- Point values, assignment policies, and reordering remain editable for future runs.
- A fixed item may be deliberately replaced through the same revision-checked Question-ID operation.
  The updated definition affects future runs only; existing and active runs retain their issued
  question order and exact hidden evidence.
- Add and remove operations remain pre-evidence edits. A selection candidate may be removed only
  before issued evidence and only when enough active candidates remain to satisfy the group's draw
  count.
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
- `submission_evaluation` retains the current normalized automated evaluation and policy-controlled
  feedback basis.
- `attempt_score_current` contains one current calculated result per attempt.
- `student_assignment_summary` contains one current computed assignment result per student.
- No manual-grading or manual-score authority is introduced. Recalculation replaces computed values
  under the current scoring generation.
- Do not retain historical grade projections or append obsolete computed values to `grade_event`.

### Recalculation queue

Point changes, zero points, full credit, exclusion, item retirement, or grade-selection-policy
changes use this process:

1. Update current assignment state and increment `scoring_generation`.
2. Mark current grades `recalculating`.
3. Enqueue one idempotent course/assignment-scoped recalculation job.
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

- Student, exact course, assignment, and attempt identity.
- Selected assignment items and problem versions.
- Issued order, randomization seed, and start time.
- Delivery status and timestamps.

They do not snapshot points, due dates, or scoring policies.

### Attempt states

Support current states including:

- `in_progress`
- `submitted`
- `auto_submitted`
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
- Flags for incomplete automated grading or recent rescoring.

Analysis is recalculated after grading changes rather than historically versioned. Operational
grading never waits for analytics. Course-local analysis remains owned by its exact CourseInstance.
Cross-course problem statistics become catalog signals only after aggregation and de-identification.

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

## External metadata and historical evidence

Institution names, institutional roster IDs, display labels, provider IDs, renderer IDs, source
identifiers, and external protocol fields may be retained for presentation, audit, provenance,
routing, or adapter exchange. They are metadata only: none can establish `UserId`, `ActorContext`,
workspace, course, Student, catalog, assignment, or lease authority. PLE remains the education-record
and grading authority; a provider is a private server-side adapter and its credentials, sessions,
raw responses, and answer-bearing material never cross the browser boundary.

The checked-in pre-SD1 migration files and current Rust Store names are migration/source evidence for
the rebase. Their historical global-scope, publication-scope, Alpha, and provider vocabulary remains
migration evidence, but does not define
the fresh schema, authorization, route, or API contract. One-time Graphify maps, source inventories,
migration matrices, schema fingerprints, and clean-volume receipts record this distinction; they are
not permanent tests or a second compatibility model.

## FERPA and security

FERPA protects education records but does not define one technical certification checklist. PLE
must enforce least privilege and defensible safeguards.

- Separate catalog, authoring, educational-record, grading-secret, and operational schemas and roles.
- Derive `ActorContext { user_id, session_id }` from the authenticated server session. It has no
  caller-controlled constructor and no institution or installation-scope selector.
- Author private drafts through the exact workspace owner/collaborator relationship; authorize
  courses and assignments through current direct Instructor membership; bind Student records to the
  exact course and Student owner; derive worker, object, export, retention, and provider work from a
  locked typed lease and its durable target.
- Enable and force PostgreSQL row-level security with default-deny policies.
- Application connections may not own tables, use superuser privileges, or bypass RLS.
- The protected transaction sets only transaction-local actor context. Route IDs, browser fields,
  queue payloads, object keys, provider fields, and catalog Question IDs are input or evidence, not
  authority.
- Separate migration, application, grader, worker, export, analytics, and retention roles.
- Keep correct answers and grader secrets unavailable to browser and catalog roles.
- Store assignment access codes as slow password hashes, never plaintext.
- Do not place student PII, responses, or grades in queue payloads, logs, cache keys, or public
  analytics.
- Require TLS, managed encryption at rest and for backups, point-in-time recovery, secret rotation,
  and tested retention deletion.
- Audit sensitive actions and record access, but do not store obsolete score values in audit payloads.

## Partitioning and distribution

Maximum distributability means preserving clean typed ownership and routing boundaries, not inventing
an institution or installation-scope hierarchy.

- Start with one PostgreSQL cluster and separate logical schemas.
- Keep the shared catalog central and read-replicable.
- Keep private records addressable by their exact `UserId`, `WorkspaceId`, `CourseId`, `StudentId`,
  and child identities. Do not add scope-leading keys or a placement contract.
- Avoid cross-course business joins and transactions where a typed course-owned operation suffices.
- Monthly range-partition only high-volume append-only attempt, submission, access-log, and audit
  detail.
- Use unpartitioned identity or header tables when necessary to preserve stable IDs and foreign keys.
- Keep current grade summaries directly indexed and unpartitioned within each shard.
- Pre-create partitions and alert on default-partition writes.
- Do not partition by subject or create thousands of per-course partitions.
- Gradebook request paths read current summaries and never aggregate attempt history.
- Maintain the hot and cold split for a catalog approaching ten million problems.
- Use queue backpressure and fairness across typed course/workspace/system targets for large
  recalculation jobs.

## Migration system

Keep raw SQL while replacing custom orchestration:

- Use SQLx `Migrator` for migration discovery, locking, and applied-checksum validation.
- Apply migrations only through:
  - `cargo tools database status`
  - `cargo tools database migrate`
  - `cargo tools database verify`
- Use a dedicated migration role.
- Application startup performs a read-only compatibility check and never changes the schema.
- Historical baseline-creation rule (completed): consolidate the pre-data design into one reviewed
  baseline.
- The accepted baseline and every applied migration are immutable. All later schema changes are new,
  forward-only migrations.
- Use expand, backfill, switch, and contract changes for populated tables.
- Do not call mutable application models from migrations.
- Do not use runtime schema generation or ORM-owned DDL.
- Production rollback uses application rollback or a compensating forward migration, not destructive
  down migrations.

### Initial epoch files

The historical pre-SD1 baseline contains exactly six ordered SQLx migrations. Each file owns a
durable domain boundary rather than a chronological implementation slice:

1. `2026080801_principals.sql`: historical runtime principals, scope/session helpers, authentication
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

The historical baseline expresses final object shapes directly. It contains no historical
`ALTER`/`DROP` repair sequence, milestone-named helper such as `r44a_*` or `r44b_*`, source-string
assertion, or dumped `_sqlx_migrations` table. SQLx creates its own ledger before applying the first
file. Roles, tables, functions, policies, triggers, grants, and revocations remain explicit SQL;
generated ORM DDL is not introduced.

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

### Fresh SD1-C epoch

The current single-installation target is a fresh disposable PostgreSQL epoch owned by `WP-SD1-C`,
not a compatibility migration over the historical global-scope schema. The exact allocation remains
in [implementation_status.md](../implementation_status.md) and the single-installation scope
register. The range is `2026082901` through `2026082932`, with these capability families:

| Range                     | Capability family                                                            |
| ------------------------- | ---------------------------------------------------------------------------- |
| `2026082901`              | Principal baseline, schemas, capability roles, and default ACLs              |
| `2026082902`-`2026082906` | Accounts, passwordless identity, Instructor vetting, and actor resolution    |
| `2026082907`-`2026082909` | Global immutable catalog, publication, discovery, and stewardship            |
| `2026082910`-`2026082912` | Private authoring, Blueprints, collections, and saved searches               |
| `2026082913`-`2026082916` | Courses, equal co-Instructors, Students, invitations, and reusable curricula |
| `2026082917`-`2026082920` | Assignments, schedules, runs, attempts, submissions, and artifacts           |
| `2026082921`-`2026082924` | Automated grading, Gradebook, analysis, and improvement threads              |
| `2026082925`-`2026082928` | Typed jobs, exports, objects, retention, and external-tool state             |
| `2026082929`-`2026082932` | Capability brokers, forced RLS, grants, and schema acceptance helpers        |

Each migration owns its local relations, keys, constraints, indexes, functions, policies, grants,
and comments. It uses global content identities and exact user, workspace, course, membership,
Student, lease, and immutable-version relationships. It creates no scope compatibility column,
scope RLS predicate, Alpha compatibility table/function, or latest-version reader. Historical
migrations remain unchanged and are one-time migration/source evidence; their obsolete scope or provider
spelling does not establish the current schema. Fresh-install convergence, no-op replay, checksum
status, missing-actor refusal, exact Student/course authorization, equal co-Instructor behavior,
typed lease scope, and forced-RLS denial are the required SD1-C PostgreSQL acceptance boundaries.

## Domain interfaces

Introduce or revise these domain concepts:

- `ActorContext { user_id, session_id }`
- `QuestionId`, immutable `QuestionVersion`, and internal `ProblemVersionRef`
- `ForcedQuestionCorrection` manifest and generation
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

Each command derives the actor from the authenticated session, authorizes its exact workspace,
course, Student, catalog, or typed lease scope, is revision-checked, idempotent where retryable,
and validated against active attempts. No command accepts a caller-selected installation scope or institution as
authority.

## Verification

- Retain the recorded fresh-install and no-op replay evidence for the frozen six-file baseline.
- Apply and verify each new forward migration with its package's focused PostgreSQL acceptance gate.
- Reject modified historical SQL, missing migrations, and concurrent migration races.
- Verify published question immutability, stable Question ID lineage, semantic fork/version rules,
  and authorized exact-version resolution.
- Verify `ForcedQuestionCorrection`: Sysadmin-only approval, allowed reason, validated replacement,
  closed manifest, immediate withdrawal from new selection, generation-fenced remediation, and
  preservation of issued/graded evidence.
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
- Verify there are no assignment-history, scoring-revision, old-grade, scope-compatibility, or
  latest-version tables/readers in the fresh SD1-C epoch.
- Verify active timer changes and server auto-submit.
- Verify force-submit, clear, and access-log authorization.
- Verify that supported question families remain strictly and deterministically automated.
- Verify item-analysis recalculation after rescoring.
- Verify import partial failures, external source/provider provenance, hostile archives, and duplicate
  warnings without allowing external metadata to establish PLE ownership or authority.
- Verify missing-actor refusal, forced-RLS denial across courses and Student owners, exact workspace
  and catalog predicates, and typed worker-lease scope for every application and worker role.
- Verify partition pruning, current-summary gradebook queries, course purge, backup restoration, and
  retention deletion.
- Run focused PostgreSQL behavior tests followed by repository formatting, linting, compilation,
  complete tests, `./check_codebase.sh`, and `pytest tests/` when the plan is implemented.

## Resolved decisions

- Historical implementation condition: no durable production data required preservation during
  baseline creation.
- Published question content is immutable and versioned; a stable Question ID names its lineage.
- Compatible improvements and grading-semantic corrections may create same-lineage versions; major
  semantic changes fork to a new Question ID with visible ancestry.
- `ForcedQuestionCorrection` is a Sysadmin-approved, closed, generation-fenced correction manifest;
  it never mutates an immutable version or silently rewrites issued evidence.
- Assignments and computed grades retain only current state.
- Assignment items have stable identities but no historical versions.
- Delete and Regrade is supported after submissions exist and blocked only by active attempts.
- All affected attempts and retakes are recalculated; old computed scores disappear.
- Runs retain only the execution evidence needed to identify what was delivered.
- Question IDs are human-readable `AAA-BBBB` Crockford Base32 identities backed by internal UUIDv7
  `ProblemId`/`VersionId` evidence.
- Discovery is global by default and subject is an optional facet.
- PostgreSQL and explicit SQL remain authoritative.
- SQLx replaces the custom migration registry.
- The accepted six-file baseline is immutable; all schema evolution now uses new forward migrations.
- The fresh `WP-SD1-C` epoch owns `2026082901`-`2026082932`; historical global-scope migrations are
  unchanged migration evidence and are not a compatibility authority.

## Explicit non-goals

- Assignment version history.
- Historical computed scores.
- Globally mutable linked published questions.
- A new Question ID for every compatible content change.
- Subject-based catalog silos.
- Institution or installation-scope boundaries, selectors, scope-leading keys, or scope-based RLS.
- A database per Instructor or CourseInstance.
- Generic EAV storage for educational records.
- Runtime automatic DDL.
- Anonymous surveys in this database decision; a future activity type may reuse the framework
  without distorting assignment and grading records.
