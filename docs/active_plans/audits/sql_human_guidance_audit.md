# SQL production-readiness audit: Human Guidance and database speed

Status: Historical SQL-only production-readiness audit, dated 2026-09-16, with
dated narrow evidence addenda. Recommendations remain current except where an
addendum explicitly supersedes a historical finding.

## Production-readiness conclusion

**This audit does not approve the SQL for production in its current form.**
The two reproduced database defects that established the original conclusion
are now narrowly corrected in current source and supported by fresh PostgreSQL
17.11 actual-role receipts: retained Question statistics increment from the
private receipt winner rather than rebuilding after deletion, and support
capability issuance no longer takes the redundant Account key-share lock.
Those receipts supersede the two historical blockers only; they do not certify
the remaining SQL or product boundaries below.

The SQL also has remaining unverified or incomplete boundaries, particularly
Course/Pool classification, Bloom metadata, Blueprint Change Proposals, and
archive recovery. A narrower production scope could deliberately exclude an
unfinished capability, but this audit does not assume such an approved scope or
certify those paths.

Historical source-only Course-purge reproduction is now superseded for its
narrow boundary by a fresh canonical PostgreSQL 17.11 actual-role receipt. The
canonical validator privilege correction is included: the retention executor can
run the Course classification CHECK without a workaround grant, while PUBLIC
execution remains revoked. That receipt does not expand the two narrow
corrections above or certify complete purge descendants, worker execution,
deployed behavior, or SQL locking outside the four observed wait orientations.

Successful installation, forced RLS, fixed definer search paths, and immutable
evidence provide a strong foundation. They do not establish correct execution of
every authorized command or protection against every unauthorized command.
Production workload sizing, backup recovery, deployment credentials, and the
rest of the application were not assessed; the conclusion concerns these SQL
sources and their database behavior only.

## Current bounded implementation receipts

Blueprint Change Proposals have separate accepted isolated proof for the production HTTP boundary
and the Instructor component. That is not compiled-main two-Instructor client/API proof, which
remains pending.

Pool stewardship has current canonical SQL/LDA proof for Pool-level Stars and Watches, including
the final ASCII-space-only display-name correction. HTTP/UI controls and the required notification
events remain pending.

Native Student Work recovery has bounded connected evidence on demo `8258`; it does not establish
the retention worker, all-reader coverage, or full screenshots. The Instructor-zone time-display
correction is source-only and not deployed. These receipts neither lock all SQL nor change any
checklist status or count.

## Relevance filter for Human Guidance

Included requirements that SQL must represent, protect, calculate, or expose:
identities and roles; private/public separation; authoring and immutable revisions;
classification and metadata; Course relationships and retention; Blueprint
history/adoption; Assessment content/defaults/disclosure facts; Attempt timing,
submission and scoring evidence; and database-owned installation data.

From interface sections, included only persistent backend implications: initial
avatars and role restrictions, Course settings, Library discovery/bulk metadata,
Student access and progress projections, and answer-free Instructor preview.
Excluded visual layout, typography, colors, navigation placement, drag-and-drop,
keyboard/touch interaction, truncation, confirmation-dialog presentation, and
browser isolation/rendering behavior. Development workflow instructions are not
database production requirements. Future roles are design context, not blockers
requiring implementation now.

Some included requirements span layers. SQL can store passkeys without proving
WebAuthn works, or save response objects without knowing whether a response is
educationally complete. Such requirements receive a boundary judgment rather
than an application-compliance claim. Browser/server email policy is explicitly
outside the readiness verdict.

## Scope and evidence

The controlling authority is [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md).
Reviewed all 69 SQL files in `schemas/base_schema/` and all seven SQL files in
`schemas/installation_data/`, including their installation manifests and the
installation-data README. Related Rust orchestration was inspected only to
distinguish database installation from cross-system provisioning during the
initial pass; application-code observations are excluded from the final verdict
and readiness work below.

The working tree already contained substantial ongoing changes, including
classification SQL. Findings describe that working copy, not a clean release.
The authority's SHA-256 at review was
`9e92c864a019d89ef9952cfb6e3b9c05a4f45d6055b6c3f49520c400f4dc7055`.

An isolated, network-disabled PostgreSQL 17.11 container was used for temporary
tests. The complete base manifest installed successfully under a reconstructed
owner/capability role bootstrap. No existing database, application service, or
tracked SQL was modified. This does not establish success of the supported
cross-system provisioning command, browser behavior, or production performance.

Evidence labels below distinguish measured results, catalog facts, and source
inferences. Speed recommendations are ranked by likely usefulness rather than
by a requirement to add more validation checks.

## Work needed before locking the SQL for production

The intended outcome is a durable SQL model whose identities, historical evidence,
privacy boundaries and public command contracts can remain stable while the
application evolves. Address the work below before production data makes model
changes expensive. This is a readiness outline, not authorization to alter SQL.
The [base-schema README](../../../schemas/base_schema/README.md) already defines
this boundary: owning base modules may change before the first human-approved
production deployment; afterward, structural changes use immutable forward
migrations. Complete the foundational model work before that deployment.

### Historical Priority 1: corrected reproduced defects

| Required change | SQL affected | Finished behavior |
| --- | --- | --- |
| Replace statistics reconstruction from deletable private receipts with a retained aggregate model | `statistics.sql`, retention cascades, `unrelease.sql` | FERPA deletion cannot reduce approved historical totals. New observations update exactly once; duplicate processing and concurrent observations cannot double count or lose increments. Define correction and Unrelease semantics explicitly rather than treating them as ordinary receipt reconstruction. |
| Repair support-capability Account lock/privilege design | `support_repair_capability.sql`, Account grants and narrow repair projections | The legitimate support command succeeds under its actual definer and caller roles. An unrelated Instructor cannot issue access to another Course's records; a Sysadmin cannot substitute a different resource, purpose, recipient or expired/revoked capability. Issuance/use remain audited. |

The two listed changes are now in current source and have the narrow receipts
described below. They remain historical audit recommendations, not proof that
the broader release boundary is ready. Neither correction grants general Account
mutation to the application role.

### Priority 2: settle required data contracts before the schema is frozen

| Decision/change | Durable SQL contract to establish | Why before production |
| --- | --- | --- |
| Course classification | Required Discipline on both Blueprint and Instance; optional hierarchy-consistent Subject/Topic/Subtopic and Tags; Instance classification independent of source Blueprint | Adding mandatory classification later requires populating existing Courses and reconciling saved Blueprint metadata. Resolve this before writing long-lived Course data. |
| Pool metadata and classification | Pool-level Title/Description and classification; first Question establishes Discipline/Subject; every addition matches; exact member Revisions remain pinned | **2026-09-16 current-source addendum:** Pool metadata already exists. Creation establishes the pair; append checks only new admissions. A retained member may reclassify independently; remove/re-add is a new admission; and a source fork preserves Pool metadata and exact member pins. Human Guidance does not require an ongoing metadata-write veto. Historical actual-role/concurrency evidence is not refreshed here; connected and global acceptance remain open. |
| Bloom metadata | Two independent required dimensions keyed to each Question/Pool Revision, with Instructor correction independent of immutable Revision content | This needs a distinct mutable metadata relationship or equivalent model. Placing corrected values inside immutable source would conflict with HG's no-new-Revision correction rule. Persistence must exist before mandatory publication metadata can be preserved. AI classification execution is outside this SQL work. |
| Tags and vocabulary lifecycle | Reconcile the 64-Tag ceiling with HG; establish the promised Sysadmin Discipline lifecycle and how referenced vocabulary changes are handled | Naming, retirement and reference behavior become difficult to change after many objects refer to the vocabulary. Keep stable keys independent of display names; do not remove referenced vocabulary without defined behavior. |
| Later Blueprint adoption | An explicit later-adoption relationship and exact Revision pin compatible with immutable creation/source evidence and complete Assessment adoption | Simply rewriting existing Course Origin would destroy historical meaning. Decide how later adoption and counts relate to original creation before storing production provenance. |
| Blueprint Change Proposals | Persistent source/target exact Revision pins, canonical proposed content, selected acceptance/outcome, and stale-target handling; acceptance produces a target Revision, never direct daughter mutation | **2026-09-16 current-source addendum:** immutable Proposal persistence now retains source/target Revision and metadata-event pins, and reconstructs canonical evidence through the existing exporter; it is not a duplicate JSON baseline. Participant reads permit the receiving owner to inspect an explicitly submitted Private source's exact Proposal evidence while ordinary Private-source and history reads remain denied. Create/read and stale-basis evidence have fresh actual-role proof, but selected/whole acceptance, accepted record/outcome, resulting target Revision, API/UI, and connected acceptance remain open. |
| Blueprint Promoted and discovery | Sysadmin-controlled boolean stored independently from immutable reusable content and available in discovery queries | This required metadata should not force new Blueprint content Revisions or unrestricted Instructor updates. |
| Pool stewardship | Stable Pool-level star/watch identities, public vetted-Instructor endorsements, private subscription data and required notification events | Match the separation already used for Questions/Blueprints. Resolve ownership and visibility before users build durable subscriptions. |
| Required notification kinds | Stable representation for Question/Pool improvement-thread and impact events, in addition to Revision/fork notifications | Question notification kinds are currently restricted to Revision/fork. Resolve the event identities and privacy boundary before subscriptions depend on that closed set. |
| Supported backend set | Consistent allowed backend/source types across authoring, revisions, delivery, reproduction and whole-Attempt completion | The current schema admits iMathAS records that the saved-response completion path cannot finalize, and excludes H5P. Complete required SQL support or deliberately restrict the initial production backend scope; do not claim capability based on a backend CHECK alone. |
| Assessment-only response terminology and parentage | Align the per-Question response record and related grading identities with `question_response`; link the evidence explicitly to its owning Assessment submission | This audit observed names that implied a nonexistent independent Question action. Its recommendation was to resolve that terminology drift before names became frozen production SQL contracts. Preserve per-Question response/credit evidence; only the Assessment Attempt is submitted. |
| Archive recovery | A narrowly authorized SQL recovery/read capability providing sufficient retained responses, exact delivery evidence and outcomes during the archive period | Physical rows and header summaries do not establish recoverability. Recovery need not reverse the archive state; it must provide the required evidence without reopening ordinary access or extending permanent deletion. |
| Canonical Blueprint metadata/content boundary | Define which SQL fields form complete exchange JSON, including names/classification; preserve ordered reusable content and exact pins | Stored Revision content currently contains modules while names are separate. Freeze the database-to-canonical-representation contract; do not assume the content column alone is a complete exchange object. Serialization implementation remains outside scope. |

If a required capability is intentionally deferred, record its approved exclusion
from the production scope. That is a release decision, not evidence that the
current SQL implements it. Optional Hints/Worked Solutions, optional statistics
categories and future roles do not require speculative placeholder tables merely
to make this audit look complete.

### Priority 3: strengthen SQL invariants and command boundaries

- Close remaining NULL holes in mandatory shape checks. The historical Quiz/Exam
  Template Attempt-limit hole is now corrected and has a fresh isolated
  PostgreSQL 17 actual-role receipt: invalid Quiz creation and Exam save reject
  atomically, while a Regular Assignment keeps NULL-as-unlimited behavior. The
  fixed-entry/Pool-entry observations remain separate work. Use explicit
  nullability in conditional shapes. This protects the model as application
  callers change without treating the receipt as a runtime-wide SQL lock.
- Review supported writes under the actual role graph: row-lock privileges,
  cross-schema ownership, fixed definer paths, private table grants and RLS
  predicates. The historical support failure shows why a successful superuser
  install alone is insufficient. Inspect both authorized success and
  unauthorized isolation.
- Confirm that updates cannot replace delivered evidence, revision numbers stay
  sequential, unchanged saves do not create history, stale edits do not overwrite,
  and point changes leave credit fractions unchanged. These contracts are more
  important to freeze than incidental query implementation details.
- Verify complete Course purge and destructive Unrelease against the installed
  dependency graph, including receipts, response/presentation bindings, targets
  and object-reference ownership. Teaching content and global Accounts must
  remain; private Student evidence must not be stranded in ancillary tables.
- Confirm absolute-date retention decisions and repeatability for late execution.
  Resolve any Course/Assessment lock-order inversion before relying on these
  commands under concurrent Instructor and Student writes.
- Set an explicit replay contract for installation data: preserve ordinary edits
  or reject a drifted fixed fixture. Root/pin checks currently do not prove that
  every teaching setting converges. Keep this choice distinct from silently
  overwriting production teaching data.

The existing source supports many of these contracts, as the comparison below
shows. The unfinished work is to confirm the important paths against the real SQL
roles and dependencies, not to require new browser checks or generic validation
features.

### Priority 4: reduce SQL cost before launch

Start with the measured Revision-observation index and exact duplicate issued-
Question index below. Design the statistics correctness fix to avoid repeated
full rebuilds. Then measure bounded Library discovery, latest-state lookups,
Course purge cascades and hot-Assessment contention with realistic data sizes.
Do not add all foreign-key diagnostics as indexes or change lock semantics based
on source inspection alone.

Index and query changes remain possible after launch. Required identities,
nullability, metadata ownership, revision relationships, retention semantics and
capability boundaries deserve the earlier decisions because they determine the
meaning of existing data. Approve the frozen SQL only once the remaining required
production data contract is implemented or its deliberate scope exclusions are
recorded.

## Speed recommendations

### 1. Index statistics observations by Question Revision

**Measured candidate; highest confidence speed improvement.**
[statistics.sql](../../../schemas/base_schema/statistics.sql), lines 59-100 and
118-145, rebuilds aggregate statistics by filtering observation receipts on
`question_id` and `revision_number`. Its primary key and unique Attempt key do
not support that filter.

The production count query was tested on the production receipt table definition
with 130,000 synthetic receipts: 13,000 Question identities, ten receipts each,
one Revision per identity. The requested Revision matched ten receipts.

| Observation | Existing indexes | Candidate Revision index |
| --- | --- | --- |
| Plan | Sequential scan | Bitmap index and heap scans |
| Rows rejected by filter | 129,990 | 0 |
| Shared buffers | 1,478 hits | 10 hits, 2 reads |
| Execution time | 3.906 ms | 0.040 ms |
| Aggregate result | Reference result | Equal in both EXCEPT directions |

The temporary candidate was:

```sql
CREATE INDEX audit_observation_revision_candidate
ON ple_private.question_statistics_observation_receipt
    (question_id, revision_number);
```

The measured query was:

```sql
SELECT question_id, revision_number, count(*),
       count(*) FILTER (WHERE correct)
FROM ple_private.question_statistics_observation_receipt
WHERE question_id = '00000001' AND revision_number = 1
GROUP BY question_id, revision_number;
```

These are single executions on uniform synthetic data, not an application
latency promise. Index size, receipt insertion cost, hot-Revision skew, choice
aggregation, and concurrent grading were not measured. Use representative
data and repeated [EXPLAIN ANALYZE with buffers](https://www.postgresql.org/docs/17/using-explain.html)
before selecting the final index. Address the aggregate-retention defect below
alongside this work; faster reconstruction still loses historical counts.

### 2. Stop rebuilding a growing Revision's entire aggregate on each observation

**Source inference; potentially larger benefit than the index.**
[statistics.sql](../../../schemas/base_schema/statistics.sql), lines 165-245,
captures a receipt and invokes the full rebuild. For a popular Revision, each
new observation revisits earlier observations; cumulative work can grow
quadratically with receipt count. Per-Revision serialization also concentrates
contention on widely used Questions.

Consider incrementing retained aggregates exactly once when an observation
receipt is first accepted, with atomic counters and the existing idempotency
identity. Keep a separate reconciliation operation rather than rebuilding on
every grade. Preserve historical totals when personally identifiable receipts
are deleted. Measure a popular Revision and many independent Revisions under
concurrent grading, including duplicate delivery and rollback. Choice counts
and correction/Unrelease behavior need the same treatment.

### 3. Remove three exact duplicate indexes; assess overlapping lookup indexes

**Confirmed DDL duplication; savings not benchmarked.**
[assessment_attempts.sql](../../../schemas/base_schema/assessment_attempts.sql),
lines 131 and 387, declares both a unique constraint and an explicit B-tree on
`issued_question(assessment_attempt_id, issued_position)`. The unique constraint
already supplies an index. The extra index adds storage and maintenance to every
issued Question without a distinct lookup key.

The later QC catalog pass confirmed two further duplicate access paths in
[assessment_attempt_interaction.sql](../../../schemas/base_schema/assessment_attempt_interaction.sql),
lines 171-172: a nonunique index on the per-Question finalized-response record
duplicates the unique Question Attempt key, and
`assessment_submission_assessment_attempt_idx`
duplicates the unique Assessment Attempt key. Compare access method, key order,
operator classes, collations, expressions, included columns and predicates when
checking duplication; this pass compared those catalog properties. Preserve the
unique indexes. No production write-cost reduction was benchmarked here.

The Attempt unique key at line 65 and lookup index at lines 382-383 also overlap.
The latter changes the final column's direction; with equality on Student and
Assessment, a backward scan of the unique index may satisfy the latest-Attempt
lookup. Confirm actual plans before removing that second index. Constraint
indexes must remain intact. PostgreSQL documents automatic indexes for unique
constraints in [DDL constraints](https://www.postgresql.org/docs/17/ddl-constraints.html).

### 4. Bound Question Library results and connect search to its existing index

**Source inference; important as the catalog grows.**
[question_library_operations.sql](../../../schemas/base_schema/question_library_operations.sql),
lines 1-83, returns an unbounded, title-sorted available-Question projection.
That projection includes author metadata and ownership work. In contrast,
[blueprint_operations.sql](../../../schemas/base_schema/blueprint_operations.sql),
lines 344-395, already has bounded keyset pagination.

Add bounded server-side filtering and pagination to the Question projection
when implementing Human Guidance's searchable/filterable Library. Align the
query with the GIN search index already declared in
[question_lineages.sql](../../../schemas/base_schema/question_lineages.sql),
lines 191-195. An index maintained on writes is not useful to a query that never
uses its search expression. Do not add a second search index before tracing
callers and confirming the required search semantics. Measure visible catalog
size, author count, selective and broad searches, and response volume.

### 5. Investigate latest-state lookups and deletion indexes selectively

**Catalog/source candidates; no speed measurement yet.**
[accounts.sql](../../../schemas/base_schema/accounts.sql) repeatedly selects the
latest Account state by Account identity, descending event time and event ID.
An index beginning with `account_id`, followed by that ordering, is a candidate
for frequent authorization checks. Question ownership has a similar latest-event
access pattern. Confirm their frequency and plans before adding indexes.

The installed catalog produced 144 foreign-key diagnostics without a matching
nonpartial B-tree having the complete FK key as its leading columns. This is an
overinclusive diagnostic, not 144 recommendations: existing shorter prefixes,
small relations, alternate access paths, and immutable parent rows can make
additional indexes unnecessary. PostgreSQL does not automatically index the
referencing side of a foreign key. Focus on actual Course purges and Attempt
cascades in [course_retention_transitions.sql](../../../schemas/base_schema/course_retention_transitions.sql).
Examples worth measuring include Assessment-to-Attempt deletion, Course-to-roster
relations, and correction targets keyed first by correction rather than Attempt.

### 6. Measure transaction contention before changing locking

**Source candidates; preserve lifecycle and concurrency semantics.**
[accounts.sql](../../../schemas/base_schema/accounts.sql), line 37, takes a
table-scoped advisory transaction lock for public-reference allocation. Bulk
creation transactions can serialize unrelated creations for the same table.
Assess whether collision retries around the unique constraint can replace that
serialization while preserving the public-reference contract.

[assessment_attempt_operations.sql](../../../schemas/base_schema/assessment_attempt_operations.sql),
lines 31-40, locks the Assessment row before Attempt work. Many Students saving
work for one Assessment can contend on a common row. Some serialization protects
Release/Unrelease decisions, so simply removing the lock would be unsafe.
Measure concurrent Student saves and expiry/submission alongside Instructor
Unrelease; inspect lock wait time rather than assuming query CPU is the limit.

Retention and Assessment/deadline operations should also be checked for a
consistent Course/Assessment lock order. The source shows both kinds of locks;
a two-session reproduction is needed before claiming a demonstrated deadlock.

No evidence here justifies partitioning, replacing the connection pool, adding
PgBouncer, or changing server-wide memory/autovacuum settings. Those choices need
cluster size, workload, wait events, and operational measurements unavailable
from these SQL directories.

## SQL alignment with Human Guidance

### Historical defect: retained statistics after deletion

Human Guidance's FERPA/statistics sections require aggregate statistics to
survive deletion of Student data. Retention deletes Attempt records and their
dependent observation receipts, while leaving aggregate rows. However,
[statistics.sql](../../../schemas/base_schema/statistics.sql), lines 76-100,
deletes and reconstructs those aggregates from the surviving receipts. Both new
observations and [unrelease.sql](../../../schemas/base_schema/unrelease.sql)
can invoke reconstruction.

A temporary fixture used the exact production aggregate/receipt table definitions
and rebuild function with minimal referenced-table scaffolding:

| Stage | Accepted graded Attempts | Correct |
| --- | --- | --- |
| Two observations captured | 2 | 1 |
| Delete one Attempt and its receipt | 2 | 1 |
| Invoke the production rebuild again | 1 | 0 |

This historical reproduction demonstrates the arithmetic/cascade failure, not
execution of the complete authorized Course retention workflow. Current
`statistics.sql` eliminates the rebuild path: its receipt winner calls the
transactional `ple_data.increment_question_revision_statistics` UPSERT helper,
and `unrelease.sql` no longer reads receipts or reconstructs counts. The fresh
actual-role PostgreSQL 17.11 receipt at `/private/tmp/ple-sql-final-statistics.log`
records retained counts through authorized Unrelease, duplicate refusal,
concurrent increments, and rollback of a forced helper failure. It is narrow:
it does not establish whole-Course retention, a full new-grade lifecycle after
purge, public-statistics disclosure or thresholds, performance, or all SQL
locking.

### Historical defect: support-capability issuance Account lock

The full installed schema was exercised as `ple_app` with an Instructor session
context and an active target Sysadmin. Calling
[support_repair_capability.sql](../../../schemas/base_schema/support_repair_capability.sql),
lines 147-191, failed with `permission denied for table account` at its
`FOR KEY SHARE OF account` statement. The definer's table grant in
[accounts.sql](../../../schemas/base_schema/accounts.sql), line 188, is SELECT
only; the row-locking operation also requires the relevant UPDATE privilege.

Current `support_repair_capability.sql` removes that redundant Account lock
without granting Account UPDATE. It resolves the canonical Student roster
resource, requires the issuing Instructor's exact Course authority, and retains
recipient/resource, expiry, revocation, and immutable-audit checks. The fresh
actual-role PostgreSQL 17.11 receipt at
`/private/tmp/ple-support-exact-authority-result.log` covers allowed issuance
and use plus the named denial and audit cases. This supports the corrected
Student-roster capability only. It does not prove all support access, all
resource classes, concurrency, connected deployment, or a general FERPA
authorization conclusion; canonical installation alone would not prove any of
those boundaries.

### Partial implementation: classification, Pools, and publication metadata

Human Guidance requires exactly one Discipline per Course and Discipline plus
Subject for LibraryObjects, with same-classification Pool membership.
[content_classification.sql](../../../schemas/base_schema/content_classification.sql)
and its operations provide normalized vocabulary, globally unique Subjects,
multi-Discipline Subject associations, and classified published Questions.
The working copy includes recent additions supporting that Question path.

However, [course_core.sql](../../../schemas/base_schema/course_core.sql) and
[blueprints.sql](../../../schemas/base_schema/blueprints.sql) do not provide
the corresponding Course/Blueprint classification fields. **2026-09-16
current-source addendum:** [question_pools.sql](../../../schemas/base_schema/question_pools.sql)
does provide Pool Title, Description, and Discipline/Subject. It establishes
the pair from the first member and checks new admissions only; retained members
may reclassify independently, while remove/re-add is a new admission. A source
fork preserves Pool metadata and exact member pins. This is the Human Guidance
admission rule, not an ongoing metadata-write veto. Vocabulary management does
not expose the complete rename/retirement lifecycle described by the authority.
The addendum does not refresh actual-role/concurrency evidence or establish
connected or global acceptance.

The SQL contains no identifiable pair of required Bloom dimensions or their
publication/correction persistence model. It also lacks full Question/Pool
optional Hint and Worked Solution persistence. Existing Question general
feedback should not be mistaken for the complete support-content contract.

### Partial implementation: stewardship, statistics, and Blueprint workflows

Question stars, watches, ownership/authorship, and revision notifications have
SQL support. Equivalent Pool stewardship/notification support is not evident.
Statistics provide accepted-graded and correct totals plus choice counts;
the full Pool Revision statistics, partial/unanswered categories, and privacy-
thresholded display contract are not represented by those counters alone.
No public statistics disclosure was demonstrated in this audit.

Blueprint immutable revisions, exact Question pins, forks, availability, and
lineage comparisons are present. SQL support for Sysadmin-controlled Promoted
status is not evident. **2026-09-16 current-source addendum:** Change Proposal
create/read persistence now keeps exact Revision and metadata-event pins and
reconstructs canonical evidence through the existing exporter, without a
duplicate JSON baseline. Explicit Proposal submission permits only the
receiving owner to read its exact Private-source evidence; ordinary Private
source and Revision-history access remains denied. Fresh actual-role proof
covers this bounded foundation and stale basis; selected/whole acceptance,
accepted record/outcome, resulting target Revision, API/UI, and connected
acceptance remain open. These remain distinct from Question correction records.

Course creation can adopt a Blueprint. Later adoption into an already created
Course lacks a corresponding operation; the current source fields and Course
Origin are immutable in [course_core.sql](../../../schemas/base_schema/course_core.sql),
lines 161-215. Implement later adoption through an explicit relationship/history
model rather than silently rewriting original creation evidence.

The Blueprint discovery projection derives `total_students_ever_enrolled` from
membership rows. Retention deletes Student memberships, so that derivation cannot
serve as a durable lifetime total. Use privacy-preserving aggregates if the
displayed metric is intended to retain historical enrollment.

### Partial implementation: recovery and secondary backends

Course retention has explicit Active/Archived/Deleted states, bounded lifetime,
deadline synchronization, and purge operations. Physical archived data remains
until deletion. The current bounded recovery slice now supplies an explicit,
current-Course-Instructor select/recover path: `42501` means an ineligible or
unavailable Course and is concealed as HTTP 404, while an eligible exhausted
selection is HTTP 200 with an empty page. Isolated PostgreSQL actual-role proof
covers the SQL guard, pagination, original-cutoff expiry, and deletion races;
the current production server has separate isolated-database HTTP proof; and
the Course workspace has separate actual-component/real-client-fixture proof.
The UI uses deliberate select then recover actions, keeps retained HTML inert,
and displays zero-based issued position 0 as Question 1. These receipts do not
establish an integrated live browser/TLS/demo path, nullable-roster HTTP proof,
all ordinary readers, or the retention worker/email lifecycle. The forward-only
lifecycle and limited executor history projection therefore do not yet establish
complete retention processing or global Human Guidance closure. Receipts:
`/private/tmp/ple-course-recovery-selection-sql-20260916.md`,
`/private/tmp/ple-course-recovery-http-proof-20260916.md`,
`/private/tmp/ple-course-recovery-instructor-ui-20260916.md`, and
`/private/tmp/ple-recovery-ui-independent-review-20260916.md`.

The Question backend constraint in
[question_lineages.sql](../../../schemas/base_schema/question_lineages.sql),
line 29, includes PLE, WeBWorK, and iMathAS, but not H5P. Whole-Attempt finalization
in [assessment_attempt_finalization.sql](../../../schemas/base_schema/assessment_attempt_finalization.sql),
lines 67-86, accepts saved-response grading only for PLE/WeBWorK. Therefore the
SQL does not yet support the complete secondary-backend completion contract.

### Boundary notes, not application defect claims

A direct SQL roster import accepted `audit-student@personal.invalid` and returned
`invitation_pending`. SQL checks canonical formatting but does not establish
institutional-domain eligibility. Browser/server email validation was outside
scope, so this is not a finding that enrollment bypasses the application's email
policy. If the trusted SQL command must independently enforce that policy, use
the configured institutional-domain authority rather than assuming `.edu` is
the only legitimate institutional suffix worldwide.

A minimal reproduction of the Quiz/Exam limit CHECK accepted a NULL limit:
PostgreSQL CHECK constraints permit an unknown result. Current creation/Attempt
operations enforce one Attempt separately, so this does not demonstrate unlimited
Quiz Attempts through the supported path. It is a DDL defense gap, not a speed
recommendation. The same distinction applies to nullable numeric CHECKs.

## PostgreSQL skill reference QC pass

This additional pass applied the SQL-relevant portions of the skill's references
to both audited directories. It is a separate quality review lens from Human
Guidance; a book's recommendation does not override the product authority.

Sources consulted from
`/Users/vosslab/nsh/vosslab-skills/skills/experts/postgresql-expert/references/`:

- `testing_and_oracles.md`: schema/migration checks, query-plan evidence and
  isolated restore guidance.
- `local-only/PostgreSQL_Mistakes_and_How_to_Avoid_Them-2025.md`: chapter 2 SQL
  pitfalls/checkers; chapter 3 types; chapter 4 tables/indexes; sections 5.3-5.4
  JSON and UUID design; sections 6.5 and 6.8-6.12 transactions, plans, locks and
  missing/unused/duplicate indexes.
- `local-only/Mastering_PostgreSQL_Weekend_Projects_and_Scale_to_Millions-2026.md`:
  chapters 3-4 types and table/index mistakes, plus chapter 2 full-text search.
  These chapters overlap substantially with the preceding book's examples;
  agreement between them is not independent validation.
- `reference_survey.md`: topic/source mapping. Version-sensitive conclusions were
  checked against PostgreSQL 17 documentation rather than applying older book
  syntax or benchmarks indiscriminately.

### Conclusions from the reference checks

| QC topic | Observation in this SQL | Conclusion/action |
| --- | --- | --- |
| Naive timestamps and date/time misuse | Installed domain columns contain no timestamp-without-time-zone or time-with-time-zone columns. Deadlines/events use timestamptz; term calendar dates use date. | Good SQL type foundation. Keep instant/calendar meanings distinct. |
| Padded strings, MONEY, XML, SERIAL | Catalog contains no bpchar, MONEY or XML domain columns. Source uses identity rather than SERIAL; catalog found nine GENERATED ALWAYS identity columns. Text limits are explicit domain checks. | No corresponding type anti-pattern found. Valid business length limits are not defects merely because a book prefers unconstrained text. |
| Points/credit precision | Stored point values and normalized credit use numeric; current point bounds and credit 0-1 ranges reject out-of-range finite values. SQL range probes also rejected numeric NaN/Infinity as credit. API projections cast scores to double precision. | Retain exact stored quantities. The range probe does not establish finite-value rejection for every numeric column or certify external formatting/rounding. |
| Primary keys and installed integrity | All 131 data/private/audit tables have a primary key; zero unvalidated constraints; zero invalid/not-ready indexes. | Pass for catalog structure. This does not prove the constraints express every required invariant. |
| Conditional required fields and NULL semantics | Exact Entry CHECK/NOT NULL/generated definitions copied with LIKE accepted fixed Question points=NULL and a Pool entry with selection_count/points_per_item/order=NULL. Negative fixed points were rejected. | Additional reproduced DDL weakness. Add explicit non-NULL conditions to mandatory variants before freezing the schema. Supported command validation is a separate question. |
| Foreign-key indexing | Referencing-side indexes are not implicit. Earlier diagnostic identified 144 overinclusive candidates. | Measure high-volume join/delete paths; prioritize actual Course purge/Attempt cascades. Do not add 144 indexes by rote. |
| Duplicate indexes | Catalog confirmed three exact duplicated access paths on issued Question, per-Question finalized-response, and Assessment-submission records. | Remove redundant nonunique copies after reviewing SQL dependencies/callers; keep unique constraint indexes. Write/storage savings are unquantified. |
| Assessment-only submission boundary | At audit time, the only reviewed INSERT into the per-Question finalized-response table occurred inside whole-Assessment finalization, after creating the Assessment submission record. Its child-state trigger checked Question Attempt state/time but not the parent Assessment submission directly. | The audit found terminology drift and a source-identified parent-invariant weakness. Its recommendation was to use `question_response` and strengthen parent coupling before freezing. No independent Student Question finalization operation was found. This historical finding does not verify a later repair. |
| Unused indexes and partial indexes | Question search GIN has no matching search predicate in the reviewed Library command; active passkey/challenge/expiry indexes already use partial predicates. | Reconcile search query/index usage. Do not drop an index merely because a fresh test database reports zero scans; production/replica usage was not inspected. |
| Index type and predicate match | B-trees cover many exact identity/ordering lookups; GIN is used for Question text search. Statistics Revision filtering lacked a useful leading key and improved in the prior measured experiment. | Keep workload-based index choices. No evidence justifies replacing B-trees wholesale with Hash/BRIN or adding generic GIN indexes to all JSON. |
| Query volume/projection and expression indexing | Explicit public/Student projections coexist with internal SELECT * row locking; Library returns an unbounded catalog with correlated metadata work. | Internal complete-row loading is not automatically a defect. Bound expensive catalog projections and connect search to the existing expression index. |
| NOT IN/NULL pitfalls and COUNT semantics | NOT IN appears chiefly in closed-value validation. Nullable validation arguments and variant CHECKs need three-valued-logic care; aggregates use both count(*) and deliberate filtered counts. | No new wrong-count/nullable-subquery result was reproduced. Do not mechanically rewrite every NOT IN or count(column); address the concrete NULL holes. |
| Relational data hidden in JSON | Accounts, memberships, pins and grading are relational. Canonical Blueprint content and opaque response/backend evidence use JSON with relational pin/member tables alongside it. | No basis for a blanket JSON removal. Keep essential FK/ownership/hierarchy invariants relational and avoid placing new required metadata only in unvalidated JSON. |
| UUID key cost | Many immutable/evidence relationships use UUIDs; selected human-reference objects also have bigint internal identities. | UUID storage cost is a design tradeoff, not a demonstrated bottleneck. Do not change established key types or expose sequential identities based on a book's unrelated large-table benchmark. |
| Table inheritance and rewrite rules | Catalog found no inherited tables or custom non-view rewrite rules. | No corresponding inheritance/CREATE RULE anti-pattern found. No redesign needed. |
| Partitioning and capacity | None of the 131 domain tables is partitioned. Receipt/evidence growth and retention determine future scale. | Absence of partitioning is not a launch blocker by itself. Estimate retained row volume and purge cost before choosing partition keys; partitioning can complicate uniqueness/FKs. |
| Explicit locking, lock order, retries and idempotency | Source has table-wide public-reference locks, hot-Assessment locking and per-Revision statistics serialization; support row-lock privilege fails. | Fix the demonstrated privilege problem; measure contention and deterministic two-session outcomes before weakening locks. Concurrency remains an unverified readiness concern. |
| Fresh install and target version | Full manifest installed again on PostgreSQL 17.11 with the restricted migrator and temporary role bootstrap. | Pass for this fresh-install environment; no supported application/database-initializer execution is claimed. |
| Dump/restore feasibility | pg_dump output restored transactionally into a second disposable database. Both catalog reports matched after normalizing unordered rows, including 131 tables, forced RLS, 402 fixed-path definers and 12 avatar seed rows. | Pass for base schema and structural seed round-trip. Roles already existed in the same cluster. This is not a populated Student-data restore, PITR, cluster-role recovery or proof that every restored command is authorized correctly. |
| Installation replay | SQL preserves ordinary identities/pins and checks important fixture roots, but does not enforce complete equality of every existing teaching setting. | Resolve preserve-versus-reject semantics for fixture edits. Do not infer full convergence solely from root/pin checks. |
| Runtime configuration, pooling, maintenance, backups/HA | These directories do not provide a running production cluster's connection metrics, autovacuum behavior, replica lag or recovery history. | Outside the SQL-only verdict. No claim that these capabilities are absent, and no tuning or infrastructure prescription based only on schema source. |

The QC pass reinforces the production verdict and adds concrete work: two more
redundant indexes and reproduced NULL acceptance in mandatory Entry variants.
It also provides positive evidence for type selection, catalog integrity and a
limited structural restore. It does not turn source-supported command contracts
into tested concurrency or production-performance guarantees.

### Historical schema terminology finding: Assessment submission is the sole boundary

**Questions are not submitted independently.** A Student submits an Assessment
Attempt, and that transaction finalizes its saved responses and outcomes.
At the time of this audit, the per-Question finalized-response table and related
grading identifiers used terminology that did not fit the required
Assessment-only submission model. The observed records meant immutable
per-Question finalized-response evidence belonging to an Assessment submission.
The duplicate-index finding concerns that historical physical table only. This
audit finding does not verify a later terminology or parentage repair.

Source inspection found its only INSERT in
[assessment_attempt_finalization.sql](../../../schemas/base_schema/assessment_attempt_finalization.sql),
lines 335-374. That operation creates the parent Assessment submission first,
then closes the Question Attempts and writes finalized response/outcome evidence
in the same transaction. No separate Student Question submit command was found.

However, the deferred trigger in
[assessment_attempt_interaction.sql](../../../schemas/base_schema/assessment_attempt_interaction.sql),
lines 119-130 and 168-169, checks accepted Question Attempt state and timestamp
without explicitly requiring the owning Assessment's submission record/time.
This is a source-identified invariant weakness, not a demonstrated independent
submission through an application-role command. Before the production freeze,
rename the table to **`question_response`**, align related grading identifiers
with that name, and make its parent
Assessment-submission relationship explicit in SQL. Preserve necessary per-
Question credit/evidence; do not confuse removing misleading terminology with
removing the response/grading facts needed to interpret the submitted Assessment.

PostgreSQL 17 references supporting these judgments:
[data types](https://www.postgresql.org/docs/17/datatype.html),
[numeric types](https://www.postgresql.org/docs/17/datatype-numeric.html),
[constraints](https://www.postgresql.org/docs/17/ddl-constraints.html),
[explicit locking](https://www.postgresql.org/docs/17/explicit-locking.html), and
[SQL dump/restore](https://www.postgresql.org/docs/17/backup-dump.html).

### Additional temporary test evidence

The second network-disabled container used the same PostgreSQL 17.11 image and
fresh SQL files. Temporary scripts/logs used the prefix
`/private/tmp/ple-sql-qc-*`. The tested Entry clone copied actual installed CHECKs,
NOT NULL and generated-column definitions; LIKE deliberately did not copy FKs,
triggers or trusted command validation. Therefore it isolates the DDL shape
failure, not a successful malformed write through a public command.

The restore used `pg_dump`, `createdb`, and
`psql -X --set=ON_ERROR_STOP=1 --single-transaction` in the disposable cluster.
The schema report was rerun against the restored database. No production-shaped
private data was loaded. Concurrency, populated restore and forward-migration
rehearsals remain explicitly unperformed; no existing cluster was touched.

## Disconnected tables, dormant SQL support, and orphan risks

The installed catalog contains **132 PLE tables**, including the migration
ledger, and **244 foreign-key constraints**. An undirected relationship scan
found one 129-table component and three standalone tables. The earlier 131-table
count excludes the migration ledger. The scan used installed constraints,
including late ALTER TABLE declarations, rather than only inline CREATE TABLE
references.

| Fully disconnected table | Purpose | Concern? |
| --- | --- | --- |
| `ple_data.course_retention_policy` | Singleton operational intervals read by retention scheduling | Intentional configuration root. It should not belong to a particular Course. |
| `ple_private.authentication_rate_limit` | Hash-keyed counters by scope/window | Intentional independent security state. Linking each counter to an Account would not fit network/email/principal/service scopes. Window cleanup ownership still needs an operational definition. |
| `ple_migration._sqlx_migrations` | Forward-migration ledger | Intentional tooling metadata, not product data. It should not cascade with Course or Account deletion. |

**No fully FK-disconnected product-data table was identified as an accidental
orphan.** Tables with no outward FKs can be legitimate roots, including Account,
Published Question, vocabulary, avatar catalog and Object Record. Tables with no
incoming FKs can be legitimate leaves, such as responses, audit events and
metadata. Neither property alone justifies deleting a table.

There are narrower pre-freeze concerns:

1. **Dormant authoring structures.** `question_folder`, `question_folder_entry`
   and `saved_question_search` in
   [question_authoring_state.sql](../../../schemas/base_schema/question_authoring_state.sql),
   lines 191-212, are connected by FKs but have no named SQL read/write commands
   in the audited base. Their occurrences are definitions, relationships and
   owner-only RLS policies. Confirm that these are intended retained capabilities
   before freezing them; implement the intended SQL contracts or deliberately
   defer/remove unused pre-production structures. This is SQL-source dormancy,
   not a claim about all application code or a recommendation to drop user data.
2. **Incomplete iMathAS SQL integration.** `imathas_render_cache_entry` has
   Question Revision FKs and expiry, but no cache read/write/prune command in
   these SQL modules. Its backend session table does cascade from Question
   Attempt. This reinforces the existing partial-backend finding; a connected
   cache table alone does not establish a complete supported backend lifecycle.
3. **Potential logical Object Record orphans.**
   [object_records.sql](../../../schemas/base_schema/object_records.sql), lines
   7-33, makes Object Records immutable, including rejecting DELETE. Replacing
   current Draft source can leave old object metadata without a current consumer;
   historical published source must instead remain intentionally referenced.
   Define how unreferenced temporary/authoring objects are recognized and cleaned
   while retained historical source is protected. The generic `student-record`
   storage/data categories also need explicit FERPA ownership/purge semantics if
   used: an identifying JSON Object Address is not rendered safe merely by lack
   of an FK. No actual leaked Student object or blob was reproduced here, and
   external object-store cleanup is outside this SQL audit.
4. **Missing explicit Assessment-submission parent.** The audit's proposed
   `question_response` table connected through Question Attempt to issued
   Question/Assessment Attempt, but did not explicitly require the parent
   Assessment submission. This is the historical parent-invariant concern
   detailed above, rather than a disconnected-table finding; it does not verify
   a later repair.

For deletion, incoming FK edges on Question Attempts, response/grading records,
statistics receipts and correction targets were inspected. Their cascades support
the intended Student Work deletion chain. Course-scoped Student Records,
memberships and accommodations use explicit ordered purge statements where FKs
do not cascade. No missing cascade was proved by this pass. Likewise, no
production row-level orphan census was performed: the test database has structural
seed data, not a representative Student dataset. FK connectivity cannot detect
missing logical ownership, dormant functionality or identifying values in JSON.

## Database QC tools and their limits

| Tool/check | What it can establish | Use in this audit |
| --- | --- | --- |
| PostgreSQL catalogs: `pg_constraint`, `pg_index`, `pg_attribute`, `pg_class`, `pg_proc`, ACL/RLS catalogs | Declared relationships, validation flags, key/index structure, types, function configuration and permission metadata | Used for integrity, duplicate indexes, fixed definer paths, forced RLS and the 132-table/244-FK graph. Requires explicit review queries; not one built-in schema approval command. |
| [pg_amcheck / amcheck](https://www.postgresql.org/docs/17/app-pgamcheck.html) | Physical table/TOAST and supported index consistency, including B-tree uniqueness/heap-to-index checks | Executed successfully in the disposable PostgreSQL 17.11 database. This is not a logical FK, HG, authorization or concurrency test. GIN and other unsupported access methods are not certified by it. |
| [plpgsql_check](https://github.com/okbob/plpgsql_check) | Static analysis of PL/pgSQL functions, including many SQL/type/reference errors and warnings | Useful next analysis on an installed disposable schema; not installed or executed in this audit. It is a third-party extension, not part of default PL/pgSQL execution. Findings need role/context and behavior review. |
| [SQLFluff](https://docs.sqlfluff.com/en/stable/) | SQL parsing/style linting using its PostgreSQL dialect | Not run. Style/parser findings are supplementary; psql includes and PL/pgSQL constructs need tool-appropriate handling. Successful lint cannot certify data semantics. |
| [Squawk](https://squawkhq.com/docs/) | PostgreSQL migration-risk linting | Not run. Most relevant to later forward migrations and changes on populated databases, rather than treating every blocking operation in a fresh base as unsafe. |
| pg_dump plus isolated transactional restore | Whether the selected schema/data can be reconstructed with the roles available in that environment | Already performed for the base/structural seeds. Not PITR or complete production recovery certification. |
| SQL invariant fixtures, real-role calls, concurrency fixtures, EXPLAIN ANALYZE | Intended success/failure behavior, ownership isolation, race handling and workload cost | Selected fixtures and one plan comparison were run. They exposed defects catalogs/physical checks cannot catch; complete command/concurrency coverage remains unperformed. |

For the physical check, only the disposable database was changed by installing
the `amcheck` extension there. The command was:

```bash
pg_amcheck -U postgres -d audit_graph --install-missing \
  --schema=ple_data --schema=ple_private --schema=ple_audit \
  --checkunique --heapallindexed
```

It exited zero without error output. Fresh, mostly empty tables make this a
limited physical-integrity result. PostgreSQL has no built-in command that
answers whether a schema is production-ready or complies with Human Guidance.
The tool results supplement, rather than override, the historical blocker
receipts and the remaining required pre-freeze model work.

## Filtered Human Guidance comparison

This matrix compares database responsibilities, not every Human Guidance bullet.
Repeated requirements are grouped rather than counted multiple times. The HG
column identifies the authoritative section and source line range. Source links
identify the SQL object or operation to inspect. The statuses mean:

- **Supported**: a corresponding SQL representation/enforcement path was found
  in source. This is not a claim that its complete runtime behavior was tested.
- **Partial**: some database support exists, but an important required part is
  missing or a stated invariant is weaker than the requirement.
- **Gap**: required SQL representation or operation was not found in the audited
  directories. Absence from these directories does not establish absence from
  all application code.
- **Defect**: database behavior was reproduced and conflicts with the contract.
- **Boundary**: SQL contributes evidence, but SQL alone cannot resolve compliance.

### Accounts, authorization, and persistent Profile data

| HG requirement and locator | SQL evidence | Assessment |
| --- | --- | --- |
| Account rules, 134-147: global identity; one immutable Student/Instructor/Sysadmin role; separate Accounts for separate roles | [accounts.sql](../../../schemas/base_schema/accounts.sql): `account`, role CHECK, identity trigger; membership role FK | Supported. Role changes cannot rewrite the Account's product identity. |
| Account rules, 134-147: passwordless passkey/email authentication | [authentication.sql](../../../schemas/base_schema/authentication.sql): email challenges, passkeys, ceremonies, hashed sessions | Boundary. SQL supports passwordless persistence; delivery and credential verification are outside this audit. |
| Account rules, 134-147: stronger Sysadmin authentication | [authentication.sql](../../../schemas/base_schema/authentication.sql): encrypted TOTP credentials, used counters, pending attestations, session consumption | Supported database substrate; external encryption and TOTP verification not certified. |
| Account rules, 145-147: deactivate/reactivate Instructor without deleting content or identity | [accounts.sql](../../../schemas/base_schema/accounts.sql): state events and Instructor state commands; [authentication.sql](../../../schemas/base_schema/authentication.sql): session revocation trigger | Supported. State is separate from Account identity and authored content. |
| Instructor role, 148-157: vetted identity before creation; same global capabilities | [accounts.sql](../../../schemas/base_schema/accounts.sql): vetting decision, Instructor creation event and commands | Supported source path. No separate higher teaching Product Role is modeled. |
| Instructor role, 148-157: private Course authority through membership | [authorization.sql](../../../schemas/base_schema/authorization.sql): exact Course Instructor/member predicates | Supported source path. Runtime review did not exhaust every RLS policy/caller. |
| Instructor role, 148-157: answer-free Student view without identity change | [assessment_student_view.sql](../../../schemas/base_schema/assessment_student_view.sql): Instructor-scoped preview source/duration reads | Boundary. SQL authorizes as Instructor; source stripping and rendered absence of answers are application responsibilities. |
| Student role, 158-179: global Account survives Course/term and Course retention | [course_membership.sql](../../../schemas/base_schema/course_membership.sql): separate Account and Course-scoped Student Record; [course_retention_transitions.sql](../../../schemas/base_schema/course_retention_transitions.sql): purge | Accepted narrow actual-role receipt: one global Student Account remained after one adopted daughter was purged, while its Course-scoped memberships, records, profiles, invitations, and events were removed. This is not a complete descendant or deployed purge claim. |
| Student role, 158-179: immutable Student email, multiple passkeys | [authentication.sql](../../../schemas/base_schema/authentication.sql): email-role trigger and many-to-one `passkey.account_id` | Supported. The email trigger rejects Student email updates. |
| Student role, 158-179: find/create Student by institutional email during roster import | [accounts.sql](../../../schemas/base_schema/accounts.sql): Student resolver; [course_operations.sql](../../../schemas/base_schema/course_operations.sql): roster import | Boundary. Identity reuse exists; institutional eligibility remains a browser/server policy question. |
| Student role, 158-179: Instructor can reset login access/send signup code | [authentication.sql](../../../schemas/base_schema/authentication.sql): passkey revocation/session/challenge records; [course_roster.sql](../../../schemas/base_schema/course_roster.sql): invitations | Boundary. Primitives exist; a complete Course-authorized reset operation was not established by SQL review. |
| Student role, 158-179: ending enrollment revokes access without immediately deleting work; later restoration | [course_membership.sql](../../../schemas/base_schema/course_membership.sql): immutable episodes, started/ended events, one active episode | Supported persistence model. Restoration uses a new episode, not rewriting an ended episode. Full command execution was not tested. |
| Student role, 158-179: bulk add, individual removal, no bulk removal product operation | [course_operations.sql](../../../schemas/base_schema/course_operations.sql): array roster import; membership transition model | Boundary. Bulk import exists; UI/command exposure of removals is outside the reviewed SQL guarantee. |
| Sysadmin role, 180-191: platform authority does not automatically confer FERPA access | [authorization.sql](../../../schemas/base_schema/authorization.sql): distinct platform and Course predicates; [support_repair_capability.sql](../../../schemas/base_schema/support_repair_capability.sql) | Partial. The architecture separates scope; repaired support issuance also needs issuer/resource authority review. |
| Sysadmin role, 180-191: task-scoped and audited support access | [support_repair_capability.sql](../../../schemas/base_schema/support_repair_capability.sql): issue command and capability events; roster repair projection | Historical lock defect is corrected narrowly with a PostgreSQL 17.11 actual-role receipt. The current Student-roster capability does not establish all support or FERPA authority. |
| Profile avatar interface, 312-333: random initial gallery avatar; shared gallery; Students cannot upload; Instructor/Sysadmin self images | [profile_media.sql](../../../schemas/base_schema/profile_media.sql): initial-avatar trigger, self-owned image validation and role-scoped operations; [provided_avatar_catalog.sql](../../../schemas/base_schema/provided_avatar_catalog.sql) | Supported database structure. Crop UI, pixel checks and cross-system media execution remain outside SQL certification. |

### Identity, classification, privacy, retention, and time

| HG requirement and locator | SQL evidence | Assessment |
| --- | --- | --- |
| Human-facing reference IDs, 635-643: opaque random prefixed IDs; uniqueness/collision handling | [accounts.sql](../../../schemas/base_schema/accounts.sql): Crockford suffix and reference trigger; Course/Assessment/Blueprint unique keys | Supported for the prefixed DB-issued identities. Account-reference exposure depends on each projection. |
| Published Question identity, 896-904: random checked Question/Pool IDs and collision retries | [question_lineages.sql](../../../schemas/base_schema/question_lineages.sql) and [question_pools.sql](../../../schemas/base_schema/question_pools.sql): shape/unique constraints | Boundary. SQL is the collision authority; the external issuer owns randomness/HMAC validation. SQL shape checks alone do not verify the check character. |
| Content classification, 644-690: one shared vocabulary; globally unique Subject; Subject in multiple Disciplines; Topic/Subtopic parentage | [content_classification.sql](../../../schemas/base_schema/content_classification.sql): vocabulary keys, associations and parent FKs | Supported model. Writer paths preserve nonempty Subject associations; arbitrary owner-level writes were not certified. |
| Content classification, 644-690: normalized progressively longer names | [content_classification_operations.sql](../../../schemas/base_schema/content_classification_operations.sql): normalize helper and 120/240/480 limits | Supported. Normalization precedes command validation. |
| Content classification, 644-690: Sysadmin manages Discipline; Instructor creates/selects other vocabulary; explicit association of existing Subject | [content_classification_operations.sql](../../../schemas/base_schema/content_classification_operations.sql): role guard, create/find/associate/list commands | Partial. Creation/selection/association supported; complete Discipline lifecycle management is absent. Explicit acceptance is a caller interaction. |
| Content classification, 644-690: hierarchy selections; LibraryObject exactly one Discipline/Subject | [question_lineages.sql](../../../schemas/base_schema/question_lineages.sql): required Question fields and composite FKs; [question_pools.sql](../../../schemas/base_schema/question_pools.sql): Pool classification and admission checks | Partial. Published Questions enforce hierarchy. Current Pool source establishes its Discipline/Subject from the first member and checks new admissions; retained members may reclassify independently, remove/re-add is a new admission, and source forks preserve Pool metadata and exact pins. This does not refresh actual-role/concurrency proof or establish connected or global acceptance. |
| Course classification, 1076-1089: required Discipline; optional Subject/Topic/Subtopic/Tags; may differ from parent Blueprint | [course_core.sql](../../../schemas/base_schema/course_core.sql), [blueprints.sql](../../../schemas/base_schema/blueprints.sql) | Gap. Neither Course form has the required classification structure. |
| Content classification/Library metadata, 644-690 and 998-1014: any number of Tags, including none | [question_lineages.sql](../../../schemas/base_schema/question_lineages.sql): metadata Tag validator; [question_pools.sql](../../../schemas/base_schema/question_pools.sql): Pool metadata | Partial. Empty Tags supported, but a 64-Tag ceiling is imposed; Course Tags remain absent and Pool metadata stores Tags. The ceiling needs reconciliation with the authority. |
| Data and history, 627-634: separate public from answer-bearing/identifying/private data | [foundation_roles.sql](../../../schemas/base_schema/foundation_roles.sql), private/data/audit schemas and command grants | Supported architecture; catalog confirms forced RLS. A complete policy adversarial audit is not claimed. |
| Student and FERPA data, 691-709: exact Course membership and Student ownership | [authorization.sql](../../../schemas/base_schema/authorization.sql): exact record ownership; [assessment_attempt_operations.sql](../../../schemas/base_schema/assessment_attempt_operations.sql): current-Attempt ownership assertion | Supported source paths, subject to the support-scope concern already identified. |
| Data/history and FERPA, 627-634 and 691-709: no Student evidence in ordinary logs/analytics/URLs/browser storage | Private evidence tables and narrow projections | Boundary. SQL separation contributes; logging, URLs and browser storage cannot be certified from schema files. |
| FERPA, 691-709: Student Work/Attempts/submissions/grades follow Course purge; teaching definitions remain | [course_retention_transitions.sql](../../../schemas/base_schema/course_retention_transitions.sql): targeted deletion and dependent cascades | Historical source-only assessment is superseded narrowly: fresh canonical actual-role archive/purge removed the observed Course-scoped Student evidence while teaching definitions and the other daughter remained. The fixture had no submitted Student Work, so descendant deletion, object-store cleanup, workers, deployment, and complete purge remain open. |
| FERPA/statistics, 691-709 and 1015-1033: privacy-safe aggregates survive Student deletion | [statistics.sql](../../../schemas/base_schema/statistics.sql): receipt-gated increment; Attempt/receipt cascade | Historical rebuild defect is corrected narrowly with a PostgreSQL 17.11 actual-role receipt. Whole-Course retention and public-statistics policy remain unverified. |
| FERPA/statistics, 691-709 and 1015-1033: no individual reconstruction; privacy thresholds for shared rollups | [statistics.sql](../../../schemas/base_schema/statistics.sql): count tables, private receipts and grants | Boundary/Partial. Aggregates are distinct from evidence, but thresholded shared display is not implemented here. No public disclosure was reproduced. |
| Retention, 710-733: six-month creation-based Active cap; dates cannot extend beyond it | [course_core.sql](../../../schemas/base_schema/course_core.sql): UTC six-month cutoff and immutable schedule; Assessment validation | Supported source enforcement. |
| Retention, 710-733: latest Assessment deadline starts retention; changing deadlines moves it within cap | [assessment_deadline_sync.sql](../../../schemas/base_schema/assessment_deadline_sync.sql): maximum current due instant and Course synchronization | Supported. No-deadline fallback is Active cutoff; that fallback is an implementation choice, not explicit HG wording. |
| Retention, 710-733: inactivity, retention start, archive and deletion are distinct | [course_core.sql](../../../schemas/base_schema/course_core.sql), [course_retention_transitions.sql](../../../schemas/base_schema/course_retention_transitions.sql) | Supported. Inactivity alone does not purge Student Work. |
| Retention, 710-733: configurable intervals; warnings before inactivity/archive | [course_retention.sql](../../../schemas/base_schema/course_retention.sql): policy table; [course_retention_notifications.sql](../../../schemas/base_schema/course_retention_notifications.sql): queued/provider-acceptance evidence | Supported database scheduling/evidence. Actual notification delivery is external. Defaults are operational configuration. |
| Retention, 710-733: archived work hidden ordinarily but recoverable until permanent deletion | [course_retention.sql](../../../schemas/base_schema/course_retention.sql): ordinary-visibility helper; [assessment_attempt_access.sql](../../../schemas/base_schema/assessment_attempt_access.sql): Student context/access and active-reference readers | Ordinary reads remain hidden under fresh actual-role evidence. A separate implemented explicit Instructor recovery path has isolated SQL race/expiry proof, isolated production-server HTTP proof, and actual-component/real-client-fixture proof. Selection corrects unavailable/ineligible Course to concealed `42501`/404 while an eligible exhausted page is empty 200; issued position 0 displays as Question 1 and retained HTML is inert. It does not establish integrated live browser/TLS/demo, nullable-roster HTTP, all-reader, worker/email, or global closure. Receipts: `/private/tmp/ple-ordinary-archive-visibility-20260916.md`, `/private/tmp/ple-course-recovery-selection-sql-20260916.md`, `/private/tmp/ple-course-recovery-http-proof-20260916.md`, and `/private/tmp/ple-course-recovery-instructor-ui-20260916.md`. |
| Retention processing, 734-741: stored-date decisions, late execution gives same decisions, repeated runs safe | [course_retention.sql](../../../schemas/base_schema/course_retention.sql): absolute scheduled actions; transition state checks and notification keys | Narrow receipt observed repeat purge returning false with the captured scalar unchanged. It does not establish complete late/worker scheduling, retry, or deployed execution. |
| Dates/time zones, 762-773: deadlines are instants; IANA zones independent of stored deadline | [assessments.sql](../../../schemas/base_schema/assessments.sql): timestamptz; [accounts.sql](../../../schemas/base_schema/accounts.sql): zone validation | Supported storage model; date-entry interpretation and display are outside SQL. |
| Dates/time zones, 762-773: invited Student defaults to Instructor zone | [course_operations.sql](../../../schemas/base_schema/course_operations.sql): `apply_student_invitation_time_zone_default` invocation | Supported source path. Later zone changes are separate from deadline mutation. |

### Questions, revisions, Pools, and the Library

| HG requirement and locator | SQL evidence | Assessment |
| --- | --- | --- |
| Common history, 742-761: Drafts/Assessments/Course Instances current state; immutable published revisions | [question_authoring_state.sql](../../../schemas/base_schema/question_authoring_state.sql), [assessments.sql](../../../schemas/base_schema/assessments.sql), Question/Pool/Blueprint revision triggers | Supported. Working state and historical revisions are distinct. |
| Common history, 742-761: monotonic Edit Numbers; sequential Revisions start at 1; fork has new identity | Draft/Assessment save CAS; [question_lineages.sql](../../../schemas/base_schema/question_lineages.sql), [question_pools.sql](../../../schemas/base_schema/question_pools.sql), [blueprint_operations.sql](../../../schemas/base_schema/blueprint_operations.sql) | Supported source paths. Blueprint metadata uses an opaque ETag, not a historical Revision identity. |
| Common history, 742-761: exact Question/Pool delivery pins, responses, outcomes; SHA-256 source/assets | [assessment_attempts.sql](../../../schemas/base_schema/assessment_attempts.sql), [assessment_attempt_presentation.sql](../../../schemas/base_schema/assessment_attempt_presentation.sql), [object_records.sql](../../../schemas/base_schema/object_records.sql), [question_assets.sql](../../../schemas/base_schema/question_assets.sql) | Supported evidence structure. Object bytes and renderer reproduction remain external. |
| Draft specifications, 784-795: private current state, save replaces, deletion, excluded from Library | [question_authoring_operations.sql](../../../schemas/base_schema/question_authoring_operations.sql): create/save/delete; [question_library_operations.sql](../../../schemas/base_schema/question_library_operations.sql) | Supported source operations. Abandoned-Draft cleanup is permissive HG language, not a production blocker. |
| Draft/publication and metadata, 784-795 and 998-1014: required metadata before publication | [question_publication_operations.sql](../../../schemas/base_schema/question_publication_operations.sql): required classification, exact source, license/authorship publication | Partial. Question classification is required; Bloom requirements and Pool publication-metadata validation are not established. |
| Question formats, 796-805: eight author-declared immutable Types; native JSON internal source; QTI only interchange | [question_lineages.sql](../../../schemas/base_schema/question_lineages.sql): eight-Type enum-like CHECK; [question_authoring_state.sql](../../../schemas/base_schema/question_authoring_state.sql): explicit format bindings | Supported type/format storage. Import translation and native source-shape validation are application responsibilities. |
| Native JSON, 806-815: unversioned and static, no random seed | Source binding uses `pleQuestionJson`; [assessment_attempt_interaction.sql](../../../schemas/base_schema/assessment_attempt_interaction.sql): PLE seed/parameter hash must be NULL | Supported SQL invariants. No public/native format version column is introduced by these sources. |
| Native JSON, 806-815: external URLs recorded/reviewable | Object/source bindings and asset records | Boundary. SQL stores source references; no dedicated complete external-URL registry was found. Native source validation/extraction was not reviewed. |
| Backend responsibilities and WeBWorK, 843-890: PLE-owned reproducible source, PG/PGML distinction, algorithmic seed separate from Pool selection | [question_authoring_state.sql](../../../schemas/base_schema/question_authoring_state.sql): PG/PGML formats; source objects; Attempt reproduction fields | Supported storage distinctions. Canonical import choice and backend behavior are outside SQL. |
| Supported backends, 845-854: primary PLE/WeBWorK, secondary iMathAS/H5P | Backend CHECKs and [assessment_attempt_finalization.sql](../../../schemas/base_schema/assessment_attempt_finalization.sql) | Partial. H5P absent; iMathAS has storage/delivery bindings but not saved-response whole-Attempt finalization. |
| Published metadata/revisions, 905-933: Title/Description/search metadata on lineage; source/support/assets changes create Revision | [question_lineages.sql](../../../schemas/base_schema/question_lineages.sql): lineage metadata separate from immutable general feedback; [published_question_metadata_operations.sql](../../../schemas/base_schema/published_question_metadata_operations.sql) | Supported for implemented fields. Missing optional support-content structures are assessed separately. |
| Published revisions/forks, 914-933: no silent pin changes; fork starts private Draft; source attribution/license/credit retained | [question_authoring_operations.sql](../../../schemas/base_schema/question_authoring_operations.sql): Draft fork; [question_stewardship.sql](../../../schemas/base_schema/question_stewardship.sql): authorship/license/citation/lineage; publication copy | Supported source paths; no automatic replacement of existing Question pins on successor publication. |
| Published corrections, 914-933: forced critical corrections audited Sysadmin actions | [corrections.sql](../../../schemas/base_schema/corrections.sql): correction records, scoped targets and audit | Supported source mechanism. Human judgment that a flaw warrants forced correction is outside SQL. |
| Question/Pool support content, 905-913, 934-942 and 968-983: optional Hints/Feedback/Worked Solutions, separate disclosure | Immutable Question `general_feedback`; no equivalent full Hint/Worked Solution/Pool support model | Partial capability. Optional content is not itself a blocker for production Questions that omit it. No requirement to populate absent optional content is inferred. |
| Pools, 943-967: published Questions only; immutable own identity/Revisions; backend-independent members | [question_pools.sql](../../../schemas/base_schema/question_pools.sql): exact member FK to Question Revision, contiguous members and revision operations | Supported source model; nested Pools are not a permitted member type. |
| Pools, 943-967 and 1282-1294: fork when reused in another Assessment; preserve exact Question pins | [assessment_pool_forks.sql](../../../schemas/base_schema/assessment_pool_forks.sql), [blueprint_pools.sql](../../../schemas/base_schema/blueprint_pools.sql): owned fork construction | Supported source paths. |
| Pool metadata, 968-983: Title/Description; first Question sets Discipline/Subject; every added member matches | [question_pools.sql](../../../schemas/base_schema/question_pools.sql): Pool schema/create/append/fork | **2026-09-16 current-source addendum:** source provides the required Pool metadata and admission-time matching check. Retained members may reclassify independently; remove/re-add checks current metadata as a new admission, and source forks preserve Pool metadata and exact pins. Human Guidance does not require an ongoing metadata-write veto. This read-only reconciliation adds no fresh actual-role, connected, or global-acceptance proof. |
| Pools, 943-967: resume preserves selections; new Attempt makes fresh selections; retain exact delivered pins | [assessment_pool_selection.sql](../../../schemas/base_schema/assessment_pool_selection.sql), [assessment_attempts.sql](../../../schemas/base_schema/assessment_attempts.sql): retained selection/member evidence | Supported source model. Distribution quality and repeated-start behavior were not runtime-tested. |
| Library, 984-1014: global Questions and Pools, vetted Instructor reuse, no Drafts or Student Library access | [question_library_operations.sql](../../../schemas/base_schema/question_library_operations.sql): Instructor-only Question projection; Pool operations | Partial. Global published Question path exists; the combined discoverable Question-and-Pool projection is incomplete. |
| Library, 984-1014: practical bulk metadata/search/filter/sort | [published_question_metadata_operations.sql](../../../schemas/base_schema/published_question_metadata_operations.sql): bulk CAS writer; Question Library projection/GIN index | Partial. Bulk Question metadata exists; query-side search/filter/paging and Pool coverage remain incomplete. UI search was not audited. |
| Statistics, 1015-1033: Revision-first totals and eligible choice counts, optional partial/unanswered/Pool aggregates | [statistics.sql](../../../schemas/base_schema/statistics.sql): Revision totals and choice tables | Partial optional capability. Missing optional categories/Pool statistics are not by themselves mandatory launch blockers; retained-count loss is. |
| Bloom, 1034-1048: independent required Cognitive Process/Knowledge Dimension per Question/Pool Revision; editable without new Revision | No corresponding fields/relations/commands in either SQL directory | Gap. Required publication metadata and independent correction persistence absent. AI assignment itself is outside SQL. |
| Stewardship, 1049-1061: public vetted-Instructor stars/identities; private watches; no Student/anonymous disclosure | [question_stewardship.sql](../../../schemas/base_schema/question_stewardship.sql): Question star/watch reads/writes | Partial. Question implementation exists; equivalent Pool stewardship was not found. |
| Stewardship/revisions, 914-933, 943-967 and 1049-1061: notifications for Revisions, forks, improvement threads, impact notices | [question_watch_notifications.sql](../../../schemas/base_schema/question_watch_notifications.sql): event kinds restricted to Revision/fork | Partial. Question Revision/fork notifications exist; thread/impact kinds and Pool notification model absent. |

### Courses and Blueprints

| HG requirement and locator | SQL evidence | Assessment |
| --- | --- | --- |
| Courses, 1062-1075: two forms; create empty/adopted; assigned Instructor; equal co-Instructor teaching authority | [course_operations.sql](../../../schemas/base_schema/course_operations.sql): creation/add Instructor; [course_membership.sql](../../../schemas/base_schema/course_membership.sql): deferred assigned-Instructor invariant | Supported. Assignment preserves one required Instructor; teaching predicates grant authority by membership rather than founder status. |
| Courses, 1062-1075: adoption can occur at creation or later | Course creation/adoption commands and immutable Course Origin | Partial. Creation adoption works in source; later adoption lacks an operation/model. |
| Blueprint structure, 1090-1099: reusable only, no Students/dates/relative schedules; published-only content | [blueprints.sql](../../../schemas/base_schema/blueprints.sql): exact-key canonical content validator, immutable Question pins | Supported reusable schema/validator. JSON names/classification completeness is assessed below. |
| Blueprint structure/Instances, 1090-1099 and 1232-1244: publish existing Course structure as new Blueprint | [blueprint_operations.sql](../../../schemas/base_schema/blueprint_operations.sql): creation from caller-supplied canonical content | Boundary. SQL can create the target; full extraction/conversion from a Course requires application-domain verification. |
| Blueprint lifecycle, 1100-1125: Private/Public/Archived, new/forks Private; ownership; public history; no Private/Archived adoption | [blueprint_operations.sql](../../../schemas/base_schema/blueprint_operations.sql), [blueprint_history.sql](../../../schemas/base_schema/blueprint_history.sql), Course creation checks | Supported source lifecycle/read/adoption predicates. |
| Blueprint lifecycle, 1100-1125: adopted Public cannot return Private; archive read-only, excluded normally, forkable/restorable | Availability operation, bounded discovery flags, save/archive checks and fork operation | Supported source path. The explicit archive include flag is honored by discovery. |
| Blueprint revisions, 1126-1136: changed Save creates next Revision, unchanged Save no-op; metadata/name changes independent | [blueprint_operations.sql](../../../schemas/base_schema/blueprint_operations.sql): save checksum/content comparison, metadata ETag and rename | Supported source semantics. SQL stores one saved Revision sequence. |
| Blueprint stewardship, 1137-1148: searchable boolean Promoted controlled only by Sysadmin | [blueprints.sql](../../../schemas/base_schema/blueprints.sql), discovery and metadata operations | Gap. Promoted storage/control/query support absent. |
| Blueprint stewardship, 1137-1148: stars public, watches private, across Revisions; no auto-star on adoption/fork; revision/change notifications | [blueprint_stewardship.sql](../../../schemas/base_schema/blueprint_stewardship.sql): lineaged star/watch, restricted projections and Revision/lifecycle triggers | Supported source paths for existing event kinds; no claim of complete notification delivery. |
| Adoption/daughters, 1149-1159 and 1245-1261: copy every Assessment/settings; exact source Revision; new unreleased/date-unset records | [course_operations.sql](../../../schemas/base_schema/course_operations.sql): complete source-member set check; [course_blueprint_adoption.sql](../../../schemas/base_schema/course_blueprint_adoption.sql) | Supported source operation. Date/source invariants are distinguished from application completeness. |
| Adoption/daughters, 1149-1159 and 1245-1261: auto-add new Assessments, explicit review of existing updates, never silent replacement | [course_blueprint_adoption.sql](../../../schemas/base_schema/course_blueprint_adoption.sql): append paths; [assessment_blueprint_updates.sql](../../../schemas/base_schema/assessment_blueprint_updates.sql): explicit apply | Supported SQL mechanisms. Automatically invoking them on publication is an orchestration responsibility not certified here. |
| Blueprint forks, 1160-1176: new Private owned child, source pin, fresh Assessment/Pool identities, same exact Questions, independent subsequent history | [blueprint_operations.sql](../../../schemas/base_schema/blueprint_operations.sql): fork; [blueprint_pools.sql](../../../schemas/base_schema/blueprint_pools.sql); fork receipt/lineage | Supported source construction paths. |
| Blueprint lineage/comparison, 1160-1176 and 1204-1215: visible forks/owners, visible related current heads, on-request JSON comparison | [blueprint_lineage.sql](../../../schemas/base_schema/blueprint_lineage.sql): fork list and comparison sources | Supported source reads. Semantic difference calculation/presentation belongs to the domain/application. |
| Change Proposals, 1177-1203: exact source/target pins, canonical proposed JSON, selected acceptance, durable outcome, stale-target handling; no direct daughter mutation | [blueprint_change_proposals.sql](../../../schemas/base_schema/blueprint_change_proposals.sql) and the data-access Store persist immutable exact Revision/metadata-event pins, exporter-reconstructed canonical evidence, and a one-final-decision accepted result. The bounded receiving-owner backend accepts both Entire and Selected decisions under the target lock, preserving exact ordered Question/Pool Revision pins; accepted metadata-only changes create the required identical-content successor. | Partial. Fresh isolated actual-role proof accepts both decision forms, verifies final-decision-insert rollback of the successor, metadata, and Pool allocations, and confirms adopted daughter Course/Assessment/entry/Pool records remain unchanged. Source-copy create/read retains explicit Private-source sharing while ordinary Private source/history reads remain denied. API, human-readable proposal review/UI, connected acceptance, and concurrent work remain open; no global checklist-status change follows. |
| Blueprint JSON, 1216-1229: complete metadata/content exchange, ordered Assessments, reusable-only data, reproducible import | [blueprints.sql](../../../schemas/base_schema/blueprints.sql): content JSON has modules; names are separate metadata; comparison loads both | Boundary/Partial. Stored content alone is not complete exchange JSON. Application assembly/round-trip was not reviewed; missing classification limits completeness today. |
| Course Instances, 1232-1244: only co-Instructors/enrolled Students; teaching data; retained inactive metadata; new term uses new Instance | Course membership policies and lifecycle; no distinct rollover object | Supported source representation/access paths, not an exhaustive policy proof. |
| Adoption counts, 1245-1261: count daughter Instances, retain parent/exact source | [course_core.sql](../../../schemas/base_schema/course_core.sql): source relationship; [blueprint_operations.sql](../../../schemas/base_schema/blueprint_operations.sql): discovery count | Narrow actual-role receipt retained each deleted daughter's anonymous distinct Account count exactly once; ordinary reimport/reclaim did not alter it. Broader discovery, worker, and deployed behavior remain unproved. |
| Names, 1262-1271: independent deliberate short/long names, short name preferably under about 16 characters | Course/Blueprint name fields and rename operations | Supported. The suggested compact length is guidance, not a mandatory SQL 16-character constraint. |

### Assessments, Attempts, submission, disclosure, and scoring

| HG requirement and locator | SQL evidence | Assessment |
| --- | --- | --- |
| Assessment model/content, 1272-1294: shared model, five types, ordered Question/Pool positions, exact pins, add/remove/reorder | [assessments.sql](../../../schemas/base_schema/assessments.sql): type CHECK and ordered entries; [assessment_operations.sql](../../../schemas/base_schema/assessment_operations.sql): atomic saves; Blueprint content validator | Supported source model; no separate Assignment category or weights hierarchy. |
| Types, 1295-1318: settings changes do not change Type; Quiz/Exam one Attempt; Bonus zero denominator | [assessments.sql](../../../schemas/base_schema/assessments.sql): save allowed keys exclude Type; start policy; [grading.sql](../../../schemas/base_schema/grading.sql): contribution functions | Supported command semantics. The historical sibling Template nullable-Quiz/Exam CHECK observation is separately corrected; its isolated PostgreSQL 17 actual-role receipt does not prove deployed or runtime-wide SQL acceptance. |
| Blueprint Assessments, 1319-1329: published content/points/reusable defaults; no Students/dates/Templates | [blueprints.sql](../../../schemas/base_schema/blueprints.sql): closed reusable shape and pin integrity | Supported source validator/persistence. |
| Instance Assessments, 1330-1338: delivery settings distinct; daughter copy editable; new Blueprint members copied unreleased | Assessment source/default fields and adoption/update operations | Supported SQL mechanisms; publication-trigger orchestration not certified. |
| Templates, 1339-1351: own editable per-Instructor settings, one Type, no Questions/Pools, copy independent defaults | [assessment_templates.sql](../../../schemas/base_schema/assessment_templates.sql), [assessment_template_copy.sql](../../../schemas/base_schema/assessment_template_copy.sql) | Supported structure/commands. The corrected Template CHECK rejects Quiz NULL creation and Exam NULL save without residue/change, accepts Quiz/Exam limit one, and preserves Regular Assignment NULL-as-unlimited in a fresh canonical PostgreSQL 17 `ple_auth`/`ple_app` rollback proof. Templates are not a live settings link; this is not deployed or runtime-wide SQL acceptance. |
| Release, 1354-1369: start unreleased; Questions/settings valid; dates ordered; due at least 24h ahead and within Active cap | [assessment_creation.sql](../../../schemas/base_schema/assessment_creation.sql), [assessment_release_validation.sql](../../../schemas/base_schema/assessment_release_validation.sql): structured issues and hard validation | Supported source path. Error explanation/retry interaction outside SQL. Raw nullable Entry CHECKs warrant independent hardening review. |
| Submission defaults, 1370-1375: late work rejected; no new starts/submissions beyond due by default | [assessment_creation.sql](../../../schemas/base_schema/assessment_creation.sql): reject default; Attempt start/finalization due logic | Supported source defaults. Live Demo's explicit late-work acceptance is an override, not a global-default defect. |
| Disclosure, 1376-1390: independent policies; Practice answers after submit; Quiz/Exam only after all Students complete, not correctness | Attempt policy snapshots; [assessment_attempt_history.sql](../../../schemas/base_schema/assessment_attempt_history.sql): submission-based cohort predicate | Boundary. SQL supplies immutable policy/cohort facts; adapter/server enforcement of answer disclosure was not audited. Creation's `after_submit` alone is not proof of premature Quiz answers. |
| Disclosure/feedback, 1376-1390: backend feedback independent from correct-answer disclosure | History response/source binding, authored general feedback and separate policy fields | Boundary. SQL distinguishes facts; final feedback extraction/disclosure logic is external. |
| Unrelease, 1391-1401: permanent Student Work deletion, content/settings retained, pre-release state, later release starts clean | [unrelease.sql](../../../schemas/base_schema/unrelease.sql): targeted purge, state and cascade | Supported narrowly: Unrelease preserves retained statistics rather than rebuilding from deleted receipts. Complete destructive-work coverage and later-release lifecycle remain unverified. |
| Attempts, 1402-1414: regular defaults unlimited; controlled repeat count; highest submitted score; fully automatic | NULL regular limit, start gate, [grading_access.sql](../../../schemas/base_schema/grading_access.sql): highest eligible score | Supported source model; backend evaluation/worker scheduling outside SQL execution proof. |
| Response persistence, 1415-1430: complete saved response or none; mutable while open; survives sessions; no grading visible until whole submit | [assessment_attempt_operations.sql](../../../schemas/base_schema/assessment_attempt_operations.sql): upsert/read response, ownership/open/expiry checks; submission projection | Boundary. SQL enforces response-object shape and Attempt state, not type-specific educational completeness. Trusted server/adapter must decide completeness. |
| Whole submission, 1415-1430: finalize all saved responses together; unanswered zero/incorrect and no backend call; no deferred outcome | [assessment_attempt_finalization.sql](../../../schemas/base_schema/assessment_attempt_finalization.sql): complete-evaluation set validation and unanswered closing | Supported SQL evidence/atomicity design for implemented backends. Backend dispatch and no-deferred-result behavior are application responsibilities. |
| Timing, 1431-1454: at most 250 delivered Questions, Pool counts selected size; 90s each rounded whole minute; override <=12h | [assessment_release_validation.sql](../../../schemas/base_schema/assessment_release_validation.sql): delivered count/base duration; Assessment duration CHECK | Supported formula and bounds. Pool member capacity checked independently of selected-count timing. |
| Timing, 1431-1454: accommodation after base, capped 24h | [assessment_student_time_accommodation.sql](../../../schemas/base_schema/assessment_student_time_accommodation.sql), effective-duration helper | Supported source math and Course/Student scope. |
| Timing, 1431-1454: server wall clock, expiry fixed across disconnect/resume, interaction checks and disconnected expiry submission | [assessment_attempt_operations.sql](../../../schemas/base_schema/assessment_attempt_operations.sql): retained start/expiry and resume gate; finalization expiry claim API | Supported database substrate. A worker actually running and succeeding is not certified by SQL availability alone. |
| Student Work, 1455-1462: retain exact Question/Pool pins, finalized response/outcome, interpret past content after edits | Issued Question, selection, source/presentation binding and immutable submission/grading records | Supported evidence structure. Current point values may affect scores without replacing historical delivery/source facts. |
| Scoring, 1463-end: immutable credit, current points, unanswered zero, highest Attempt; no backend recall on point edit | [grading.sql](../../../schemas/base_schema/grading.sql), [grading_access.sql](../../../schemas/base_schema/grading_access.sql): current Entry point join and score function; immutable result constraints | Supported source calculation. A complete edit-and-recalculate runtime scenario was not run in this audit. |
| Scoring, 1463-end: no categories/weights/Course percentages; pilot CSV/TSV point export | Point-based gradebook projection; no category/weight model in these schemas | Supported SQL scope. Export encoding/file behavior and LMS weighting are outside SQL. |

### Production implications of this comparison

The two **Defect** rows are immediate SQL blockers. Required **Gap** rows mean
the full HG-defined data contract is not implemented, particularly mandatory
classification/Bloom metadata and Proposal/recovery capabilities. They should
be addressed or explicitly excluded from an approved release scope before
claiming the SQL implements Human Guidance.

Optional support content, optional statistics categories, future roles,
preferred short-name length, UI interactions and external services are not
automatically SQL launch blockers. **Boundary** rows are intentionally unresolved
by this SQL-only audit. **Supported** rows establish source support, not blanket
production certification. The existing installation/runtime experiments are
stronger evidence for their specific outcomes than source inspection.

## Installation data

Reviewed the manifests, vocabulary seed, prepublication context, Live Demo
construction/oracle, and context revocation. The
[README](../../../schemas/installation_data/README.md) correctly distinguishes
database-owned `apply` from cross-system `provision`. Attempts, renderer effects,
grading, and complete teaching activity are intentionally outside direct SQL.
Their absence from the SQL manifest is not a defect.

The Live Demo is ordinary teaching data using ordinary roles and exact Revision
pins, rather than a separate privileged demo schema. The SQL checks important
root identities, Account roles, roster relationships, Assessment state, and
Question pins on replay. Fixed fixture dates and explicit late-work settings
are fixture choices, not evidence of an incorrect global default.

One SQL replay issue merits attention:

- Replay checks do not establish complete convergence of every teaching setting,
  including points, feedback, instructions, and time limits. Existing records
  can retain altered values while root/pin checks succeed. Clarify whether replay
  preserves ordinary user edits or must reject a drifted fixture; do not silently
  overwrite teaching data without an explicit contract.

No complete installation-data provision was run: it requires owning application
and storage services. Running only the SQL would not test that published Questions
render or that Student Work and grading complete. No additional seed indexing
is justified for this small fixed installation dataset.

## What the SQL already does well

The complete base installation succeeded. Catalog inspection found 131 tables in
the data/private/audit schemas, all with RLS enabled and forced; all 402
SECURITY DEFINER functions inspected had fixed search paths. No inspected PLE
function retained PUBLIC EXECUTE in its effective catalog ACL. These are catalog
facts, not a proof that every policy is correct. PostgreSQL's
[RLS documentation](https://www.postgresql.org/docs/17/ddl-rowsecurity.html)
explains the ownership and execution context boundaries that still matter.

The source provides immutable Account roles, immutable Question/Pool/Blueprint
revisions, exact Revision pins, revision-aware publication/authoring operations,
co-Instructor membership, current Assessment state, whole-Attempt finalization,
server expiry processing, bounded time accommodations, and Course retention
operations. Speed changes should preserve these contracts.

## Verification and limitations

- PostgreSQL: 17.11, isolated container, no network or host port.
- Full base manifest: `psql -X --set=ON_ERROR_STOP=1 --single-transaction`;
  successful installation under the temporary role bootstrap.
- Statistics fixture: exact production rebuild function and receipt/aggregate
  definitions; retained-count loss reproduced; candidate index query results
  matched in both directions.
- Installed-schema support command: Account row-lock privilege failure reproduced.
- Installed-schema roster command: noninstitutional email accepted at SQL boundary;
  browser/server policy not tested.
- Additional QC: fresh catalog inspection, exact Entry shape-clone boundary
  probes and base-schema/structural-seed dump/restore succeeded as described
  above. Two more duplicate indexes and mandatory NULL shape holes were found.
- Performance: one before/after count-query experiment only. No production data,
  concurrent-user benchmark, complete choice-count rebuild benchmark, cluster
  tuning measurement, or cross-system provisioning acceptance was performed.

Temporary evidence was captured under `/private/tmp/ple-sql-guidance-audit-*`.
The tables, query, workload shape, plans, and observed outcomes above preserve
the material evidence without adding permanent test code to the repository.
No SQL, migration, browser code, tracked documentation, changelog, index, or Git
history changes are part of this audit.

## Course-purge evidence addendum

Fresh canonical PostgreSQL 17.11 installation as `ple_migrator` accepted the
canonical `course_classification_tags_are_valid` grant to
`ple_course_retention_executor`; the earlier actual-role archive failure was a
real privilege defect and is now corrected in source, not bypassed in the proof.
The validator remains unavailable to PUBLIC.

The one-time actual-role fixture retained anonymous per-Course enrollment count
`1` for the deleted daughter, retained the same global Student Account and the
other daughter's active membership, and refused import/claim after final purge.
It observed four real lock waits through `pg_stat_activity` and
`pg_blocking_pids`: purge behind Student claim, purge behind Instructor import,
then Student claim and Instructor import behind final purge. Exact labelled
container `ple-course-purge-proof-20260916` used network `none`, tmpfs data, and
auto-remove; it was stopped and removed after the proof.

This receipt is deliberately incomplete. Its Assessment had no submitted Student
Work fixture, so it does not prove every Work/Attempt descendant deletion. It
does not establish complete purge coverage, worker scheduling, deployed behavior,
or a general SQL lock-order proof. Receipt:
`/private/tmp/ple-course-purge-proof-20260916.md`; independent source review:
`/private/tmp/ple-course-purge-sql-review-20260916.md`.

## Archived invitation claim correction addendum

The current claim guard now requires `retention_lifecycle_state = 'active'`
after the Course-root lock and before the active-membership idempotent return.
An Inactive but unarchived Course remains claimable; revocation is unchanged.
Fresh PostgreSQL 17 actual-role `ple_auth`/`ple_app` proof passed: archived
pending and idempotent claims return generic `42501` without child/event rows,
and claim waited behind a real archive lock before the archive committed and
the denial held. This does not prove deployment, touch live `8258`, or close
full retention. Receipt: `/private/tmp/ple-archive-invitation-claim-fix-20260916.md`.
