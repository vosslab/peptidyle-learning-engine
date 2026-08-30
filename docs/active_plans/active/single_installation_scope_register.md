# Single-installation scope register

## Purpose

This register assigns the single-installation correction to exact repository owners. It supports
[single_installation_authorization_plan.md](single_installation_authorization_plan.md) and is the
handoff contract for `WP-SD1` implementation packages.

Graphify at commit `dc227871d18d` identifies the legacy request context, legacy scope identifier, and `CourseId` as major
cross-area connectors. One-time direct inspection found the historical scope-shaped identity in 465
Rust files, 85 of 111 active migration files, and 113 documentation files. These counts are impact
evidence rather than permanent acceptance targets. Current source remains authoritative for each
package's exact file boundary.

The 2026-08-29 BlueprintCourse/Alpha consolidation inventory is also one-time impact evidence. It
identified the existing one-assignment Blueprint and multi-module Alpha branches as one reusable
course concept. The cutover below retains the Alpha tree's cardinality as `BlueprintCourse`, retains
the Blueprint revision/pin machinery, and keeps the separate `CourseInstance` delivery aggregate.

## Binding product decisions

- PLE is one installation with global accounts.
- Every approved Instructor has the same global product capabilities.
- A course supports multiple equal co-Instructors. Creation creates its first ordinary Instructor
  membership.
- Every published assignment question remains discoverable and resolvable to every approved Instructor
  across its lifecycle, with visible lifecycle state. Active questions are ordinarily selectable;
  deprecated and archived questions remain available for evidence/history but are excluded from
  ordinary new selection.
- Drafts remain private until validated publication.
- `BlueprintCourse` is the only reusable course-level model: one ordered module/assignment tree,
  one aggregate revision, exact published question pins, and relative schedule intent.
- A published BlueprintCourse is answer-free and visible/reusable to every vetted Instructor; a
  draft is owner/workspace-collaborator scoped. It has no Students, deadlines, releases,
  accommodations, grades, or activity.
- `CourseInstance` is the exact teaching `CourseId` aggregate. It has exactly one immutable
  Blueprint parent/applied revision and owns copied definitions, Students, releases, live deadlines,
  accommodations, grades, and delivery settings.
- Blank-course creation uses a minimal Blueprint. Relative intent resolves against the destination
  term/time zone; new upstream assignments propagate unreleased and require explicit release.
- Archived referenced Blueprints remain resolvable for evidence/history. `BlueprintReference` (`BP-*`)
  is the only reusable-course locator; Alpha is not a type, route, schema branch, capability, or alias.
- Student records use exact course membership and Student ownership.
- A Course Observer uses an exact-course audited relationship that can show named assignment
  completion and privacy-safe anonymous aggregate grades, while individual Student scores remain
  unavailable.
- An approved Instructor creates a CourseInstance as its first ordinary co-Instructor. A Sysadmin may
  create one on behalf of an explicitly assigned approved Instructor; that operation creates the
  assigned Instructor's first ordinary membership and grants the Sysadmin neither teaching nor FERPA
  authority. Sysadmin FERPA operations use narrow audited capabilities.
- Current live personas are Student, Instructor, and Sysadmin.
- Future Grader, Course Observer, and Student Observer access uses explicit course relationships and
  capability grants.

## Historical shapes and canonical replacements

The left column records pre-SD1 implementation or protocol vocabulary. Legacy-scope descriptions
identify historical migration/source shapes; the right column is the binding target.

| Current owner or shape                                | Intended owner or shape                                                             | Package    |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------- | ---------- |
| Legacy global scope identifier                        | Exact `UserId`, `WorkspaceId`, `CourseId`, or immutable content identity            | SD1-B      |
| Legacy request context                                | Server-derived `ActorContext { user_id, session_id }`, defined only in `session.rs` | SD1-B1     |
| Browser session installation-scope field              | Account/session projection plus course memberships loaded by course APIs            | SD1-B5/F   |
| Legacy session-scope GUC and resolver                 | Transaction-local `ple.actor_user_id` and operation-specific predicates             | SD1-C      |
| Scope-leading keys and foreign keys                   | Globally unique identity plus exact user/workspace/course parent                    | SD1-C      |
| Global-scope RLS predicates                           | Current actor, membership, Student owner, workspace relation, or leased capability  | SD1-C      |
| Historical `PublicationScope`                         | One shared published state with visible lifecycle status                            | SD1-B3/C   |
| Historical published visibility facets/request fields | Shared-catalog query over every published question                                  | SD1-B3/F   |
| Historical collection visibility `institution`        | Explicit shared collection relationship                                             | SD1-B3/F   |
| Private collection/Star                               | One UserId-owned Star; optional explicit sharing for named collections              | SD1-B3/F   |
| Scope-shaped Student identity                         | Course membership/enrollment plus Student `UserId` where required                   | SD1-B2/C   |
| One-assignment Blueprint versus multi-module Alpha    | One ordered `BlueprintCourse` tree with bounded assignment projection               | SD1-B3     |
| Alpha fork/instantiation source variants              | Destination-specific Blueprint fork/adoption/instantiation operations               | SD1-B3/B2  |
| Reusable relative schedule versus live deadline       | Relative intent on BlueprintCourse; resolved/editable deadline on CourseInstance    | SD1-B2/B2  |
| Upstream assignment addition                          | Daughter CourseInstance projection marked unreleased until explicit release         | SD1-B2/E/F |
| Alpha reference `AC-*`                                | `BlueprintReference` `BP-*` only; reject AC references after cutover                | SD1-B3/B5  |
| Scope-prefixed object key                             | Typed opaque catalog, workspace, or course-record object identity                   | SD1-B4/E   |
| Legacy job scope                                      | Typed course, workspace, catalog, or system lease scope                             | SD1-B4/E   |
| Legacy launch/export/retention scope                  | Exact course, assignment, attempt, export, or retention target                      | SD1-B4/E   |

## Historical source ownership and successors

| Capability                            | Primary files and symbols                                                                                                                                                                                                                                                                                                     | Successor       |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| Fundamental identity                  | Legacy scope identity; `crates/learning-data-access/src/session.rs::{SessionSubject,ActorContext}`; `crates/server/src/auth.rs::{AuthenticatedSession,AuthSessionResponse}`                                                                                                                                                   | SD1-B1          |
| Actor transaction context             | `crates/learning-data-access/src/session.rs` is the sole `ActorContext` definition owner; `src/rls.rs` is transaction-adapter-only, adapting the resolved value to transaction-local PostgreSQL state and defining no ActorContext type; `src/postgres/transaction_context.rs` and session/account Store contracts consume it | SD1-B1/C1/D     |
| Instructor approval and equality      | `crates/domain/src/teaching_authority.rs`; `crates/learning-data-access/src/teaching_authority_store.rs`; course creation and teaching-authority Store contracts                                                                                                                                                              | SD1-B2/D        |
| Co-Instructor membership              | `crates/server/src/course/tests/course_creation.rs`; `crates/server/src/course/teaching_operations/`; `src/pages/teaching_team_panel.tsx`                                                                                                                                                                                     | SD1-B2/E/F      |
| Course and Student ownership          | `crates/question_model/src/course.rs`; `crates/domain/src/entitlement.rs`; course, roster, enrollment, run, attempt, feedback, and Gradebook Store families                                                                                                                                                                   | SD1-B2/D        |
| Publication contract                  | `crates/question_model/src/catalog.rs::PublicationScope`; `crates/question_model/src/catalog_facets.rs`; `crates/learning-data-access/src/contracts/catalog.rs`                                                                                                                                                               | SD1-B3          |
| Shared catalog persistence            | `crates/learning-data-access/src/{in_memory,postgres}/catalog*`; catalog search, evidence, usage, publication, byline, and asset modules                                                                                                                                                                                      | SD1-C2/D        |
| Curation, discovery, and proposals    | `crates/question_model/src/curation.rs::ProblemCollectionVisibility`; `crates/learning-data-access/src/contracts/problem_curation.rs`; Memory/PostgreSQL curation and QuestionChangeProposal modules                                                                                                                          | SD1-B3/C3/D     |
| Private authoring and reusable source | workspace, QTI ingress/import, flat-question source/grading, author preview, and BlueprintCourse module/assignment tree                                                                                                                                                                                                       | SD1-B3/C3/D/E   |
| CourseInstance adoption               | curriculum adoption, fork, assignment/whole-course instantiation, rollover, term shift, fast-forward, and divergence recovery                                                                                                                                                                                                 | SD1-B2/C4/D/E/F |
| Runs and grading                      | `crates/domain/src/{run,scoring,item_analysis}.rs`; run, attempt, submission, evaluation, receipt, Gradebook, and analysis Store modules                                                                                                                                                                                      | SD1-B2/C5/C6/D  |
| Jobs and workers                      | `crates/learning-data-access/src/jobs.rs`; PostgreSQL jobs; accepted-submission, scoring, item-analysis, export, retention, and publication workers                                                                                                                                                                           | SD1-B4/C7/D/E   |
| Objects and delivery                  | `crates/objects/src/bucket.rs`; Memory/S3 object implementations; asset delivery, reconciliation, appearance, upload, and retention modules                                                                                                                                                                                   | SD1-B4/C7/E     |
| External adapters                     | `crates/adapters/imathas/`; `crates/adapters/webwork/`; server provider backends; external-tool Store modules                                                                                                                                                                                                                 | SD1-B4/C7/E     |
| Base-course installer                 | `crates/base-course-installation/`; PostgreSQL base-course install; project-tools seed; `local_stack_control/chapter_one.py`                                                                                                                                                                                                  | SD1-E/F         |
| Browser contracts                     | `generated/api/`; `src/api/`; `src/features/`; `src/pages/`; generated Rust Serde roots                                                                                                                                                                                                                                       | SD1-B5/F        |
| Live-stack orchestration              | `local_stack_control/`; `containers/`; `tests/e2e/`; `run_live_demo.sh`; screenshot/browser owners                                                                                                                                                                                                                            | SD1-F/G         |
| Deployment                            | `deploy/opentofu/` storage, compute, edge, identity, queue, and cost-control modules                                                                                                                                                                                                                                          | SD1-E/G         |
| Product documentation                 | architecture, authorization, security, data, object, identity, enrollment, retention, local-stack, install, usage, release, and active-plan documents                                                                                                                                                                         | SD1-A4/G        |

## BlueprintCourse cutover ownership

The following matrix binds the source-to-consumer cutover to the SD1 dependency order. Each owner
removes the old paired branch in its boundary and hands the single BlueprintCourse contract to the
next milestone. Exact current files remain subject to source inspection; the matrix is the ownership
contract, not a generated inventory.

| Milestone | Owned boundary                                                                            | Required outcome and handoff                                                                                                                                                                                                                                                                                                                                                                                                              |
| --------- | ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SD1-B     | QModel/domain, adoption contracts, Rust Serde roots, route/reference contracts            | Define `BlueprintCourseDefinitionInput`, ordered module/assignment projections, one revision/access/source family, `BlueprintReference` only, destination-specific fork/assignment/course instantiation commands, and `QuestionChangeProposal` pinned to an exact QuestionVersion with validation, semantic-impact, lifecycle, and contributor-credit fields. Preserve relative-intent and exact-pin invariants.                          |
| SD1-C     | Fresh PostgreSQL epoch, migrations, RLS, brokers, grants                                  | Under the status-owned `2026082901`-`2026082932` epoch, use the course/curriculum slice `2026082913`-`2026082916` for one BlueprintCourse root/tree and CourseInstance parent binding, the proposal/audit relations for exact-version QuestionChangeProposal review and immutable acceptance/rejection receipts, and the broker/RLS slice `2026082929`-`2026082932` for one capability family. Historical `1837`-`1847` remains evidence. |
| SD1-D     | Memory/PostgreSQL Store contracts, decoders, SQL function callers, adoption materializers | Merge stored Blueprint/Alpha implementations into one BlueprintCourse Store family; decode one complete ordered tree; preserve no-op/stale/atomic revision behavior; record immutable parent/revision receipts, CourseInstance-owned current projections, and one proposal Store contract with exact-base conflict/rebase handling and append-only audit results.                                                                         |
| SD1-E     | Server services, workers, objects, adapters, seed/materialization helpers                 | Enforce approved-Instructor and exact destination authorization; create blank CourseInstances from minimal Blueprints; resolve relative intent with a witness; propagate new assignments unreleased; preserve divergent delivery edits; derive all worker/object scope from locked targets; validate, submit, review, and accept or reject QuestionChangeProposals through the catalog boundary.                                          |
| SD1-F     | API routes/policy, generated TS, browser clients/decoders/components, live-demo scenarios | Keep `/api/course-blueprints` as the sole reusable route family and BP-only decoding; provide one workspace/editor/picker and destination-specific adoption UI; show CourseInstance propagation and explicit release. Provide **Suggest an improvement** as the Instructor-facing QuestionChangeProposal journey.                                                                                                                         |
| SD1-G     | Connected acceptance, visual evidence, technical docs and release closure                 | Prove source/schema/API/browser convergence on the real stack, refresh canonical screenshots/manifest, classify one-time inventories separately, and run final Validation. Human Guidance files remain unchanged.                                                                                                                                                                                                                         |

The package coordinates remain the registered SD1 labels (`SD1-B1` through `SD1-B5`, then
`SD1-C` through `SD1-G`). They do not replace or rename accepted `WP-INST-B1`/`WP-INST-B2`, and
they do not allocate new package IDs for `WP-R0`, `WP-R1`, `WP-R2`, or `WP-PY-L1`.

### Cutover surface register

These are the dependency-ordered source families identified by the Graphify report and direct
inspection. They are implementation navigation, not permanent inventory assertions.

| Order | Current surface                                                                                                                                                                      | Canonical SD1 disposition                                                                                                                                                                                                                                                                                                                                   |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | `crates/question_model/src/reusable_curriculum.rs`, `public_route.rs`, and `curriculum_adoption/contracts/`                                                                          | One BlueprintCourse tree/revision/access/source family, BP-only references, bounded assignment-location projections, and destination-specific adoption commands. Remove Alpha unions and AC locators.                                                                                                                                                       |
| 2     | `crates/learning-data-access/src/contracts/reusable_curriculum.rs`, `src/in_memory/reusable_curriculum.rs`, `src/postgres/reusable_curriculum.rs`, and current curation Store family | One capability, command, decoder, page, validator, and Store method family over a complete ordered tree. Preserve atomic no-op/stale replacement and exact pins. Replace the current favorites migration/API/Store surface with one UserId-owned Star record in the fresh SD1 epoch; expose vetted Instructor Star counts/identities and no favorite alias. |
| 3     | `schemas/migrations/2026081837_blueprint_alpha_curriculum.sql` and accepted `1838`-`1847`                                                                                            | Historical evidence only; do not edit or bridge. SD1-C creates one fresh BlueprintCourse/CourseInstance shape in the status-owned epoch, with exact numbers assigned by the status ledger.                                                                                                                                                                  |
| 4     | `crates/server/src/reusable_curriculum.rs`, `route_policy.rs`, `src/api/`, `generated/api/`                                                                                          | One `/api/course-blueprints` family, strict BP-only decoding, one ETag/no-store contract family, and regenerated direct contracts. No `/api/alpha-courses` route or alias.                                                                                                                                                                                  |
| 5     | `src/features/reusable_curriculum/`, `problem_picker/`, `curriculum_adoption/`, and curation UI                                                                                      | One workspace/list/detail/editor/picker and destination-specific fork/adoption UI. A nested assignment selection never becomes a second source model. Replace favorite controls with one Star control; Watch remains a private in-app subscription, with no alias.                                                                                          |
| 6     | `crates/project-tools/src/e2e_seed/`, `tests/e2e/`, `tests/playwright/`, screenshot manifest                                                                                         | One `blueprint_course` resource with explicit `course_instance` destinations; browser journeys cover publish, fork, instantiate, schedule resolution, unreleased propagation, release, divergence, and stewardship. Refresh visual evidence after behavior is green.                                                                                        |

## Approved Instructor contract

Use one canonical approval predicate from domain through PostgreSQL:

- `approved_instructor(user_id, now)` succeeds for a current manually approved Instructor account.
- `current_course_instructor(user_id, course_id, now)` requires current approval and current direct
  Instructor membership.
- Course creation, question publication, catalog discovery, collections, saved searches, Stars,
  reuse, and improvement use that same predicate.
- Course creation inserts the creator's first ordinary Instructor membership atomically.
- A current course Instructor may invite an approved Instructor as a co-Instructor.
- Invitation acceptance re-evaluates approval, invitation state, and roster revision in one
  transaction.
- Every current co-Instructor uses the same teaching mutation and FERPA-read predicates.
- Revocation serializes with protected reads and writes and takes effect immediately.
- Approval withdrawal closes global Instructor capabilities and course-Instructor FERPA operations
  in the same protected transaction.
- One registered operation matrix proves creator/co-Instructor allow/deny equivalence; actor identity
  differs only in audit evidence.

The current source already contains approval logic in
`crates/domain/src/teaching_authority.rs` and PostgreSQL functions such as
`ple_instructor_approval_eligible`. SD1 consolidates their consumers around one accepted contract.

## Publication contract

`crates/question_model/src/catalog.rs::PublicationScope` and its catalog facet/request consumers are
retired into one shared published state. SD1-B3 owns the plan's exact, closed field inventory through
`InstructorCatalogQuestionV1`, `InstructorCatalogEvidenceV1`, `InstructorCatalogUsageV1`,
`InstructorCatalogLineageV1`, `InstructorCatalogSearchItemV1`, `InstructorCatalogSearchPageV1`, and
`InstructorCatalogDetailV1`. Each direct and nested DTO uses generated `snake_case`,
`deny_unknown_fields`, an enumerated field set, and a new version for any later field addition.
`InstructorCatalogEvidenceV1` releases its available state only when every count and metric satisfies
the formula version's named disclosure threshold; every other result is the field-minimal
insufficient state.
Search facet counts measure published-question metadata only. Usage counts measure assignment-
reference rows only; global usage omits course identity, and named own-course usage is limited by the
current Instructor's course membership. These two count families contain no Student-derived data and
remain separate from the evidence disclosure formula.

Every published lifecycle state remains discoverable and exactly resolvable to approved Instructors;
catalog search and detail visibly label active, deprecated, or archived state and any reason. Active
questions are ordinarily selectable. Deprecated and archived questions remain available for evidence,
provenance, and history but are excluded from ordinary new selection and new references. Drafts remain
private to their workspace until validated publication succeeds.

The projection excludes Student-linked data, accepted responses, grades, cohort-identifying counts,
answer keys, scoring rules, private grader payloads, source packages, provider identifiers and
credentials, object keys, signed URLs, and workspace identifiers. Exact server capabilities
continue to own private grading and execution evidence. Workspace relationships continue to own
unpublished source and drafts. Public presentation-asset delivery remains distinct from source
delivery.

Collection visibility remains a separate concern. One Star concept is UserId-owned; a named personal
collection may add an explicit sharing relationship. Watch is a private in-app subscription. Approved
Instructors may see aggregate Star count and vetted Instructor identities who starred; Students and
anonymous users see neither Star identities nor watch state. Neither curation state creates another
publication visibility state.

## Question stewardship contract

The Question catalog remains installation-wide and is reused by BlueprintCourse and CourseInstance
entries through exact pins. A stable human-facing `QuestionId` names a lineage; immutable
`QuestionVersion` history records each published meaning. A pin names the exact QuestionId/version,
never a mutable latest pointer. The owner makes a moderate edit as a new immutable same-lineage
version. Any approved Instructor may submit a `QuestionChangeProposal` pinned to one exact base
QuestionVersion after validation yields a proposed patch, rationale, semantic impact, grading impact,
and contributor credit. The owner accepts or rejects the proposal; acceptance creates the next
same-lineage version only when the pinned base remains current, and a stale proposal is rebased or
resubmitted. Semantic changes use a closed review classification: presentation, metadata, and other
compatible edits create a new immutable same-lineage version. A validated correction to a wrong key,
calculation, scoring rule, or no-correct-answer defect preserves its QuestionId when the objective,
task, response family, purpose, and answer expectations remain the same; it records explicit impact
and recalculation evidence. A change to the objective, task, response family, purpose, or
substantially different answer expectations is a FullFork with a new QuestionId. Active versions are
ordinarily selectable; deprecated and archived versions remain discoverable/resolvable for evidence
and existing pins but are excluded from ordinary new selection.

Fork lineage is visible in the safe catalog projection. Forks create creator-private drafts, and
publication validates source completeness, answer-free projection, exact pins, licensing/byline, and
change class before shared visibility as a separate lineage. Stars are UserId-owned curation events
and watches are private in-app subscriptions. Approved Instructors may see Star count and vetted
Instructor star identities; Students and anonymous users see neither. Improvement events,
ChangeProposals, and linked replacements are durable, auditable events with no Student identity.
Grading corrections record affected pins and assignment/run impact and require generation-fenced
recalculation or explicit refusal; issued evidence remains immutable.

Attempts, correct counts, and eligible-choice counts are version-specific evidence families. Each is
privacy-thresholded by the existing formula and otherwise returns insufficient evidence, never a
course, Student, response, or small-cohort identity. Ownership stays with existing packages:
`WP-INST-D1` owns discovery, lineage, and thresholded evidence; `WP-INST-D2` owns one UserId-owned
Star concept, private Watch subscriptions, collections, selection, and ChangeProposal submission/
review; `WP-INST-G1` owns correction impact/recalculation; `WP-INST-G2` owns audited learner-work
impact reads; and `WP-INST-G3`/`G4` own linked replacement analysis and improvement events. SD1-B3
defines the contract and SD1-C/D carry it through schema/Store without a new package ID.

### ForcedQuestionCorrection package contract

An emergency security or critical-correctness finding uses a Sysadmin-approved
`ForcedQuestionCorrection`. The validated replacement and closed privacy-safe impact manifest are
prepared first. One authoritative replacement mapping/generation is atomically activated, stopping
new selection and issuance immediately; bounded idempotent generation-fenced workers materialize
affected BlueprintCourse/CourseInstance/assignment/pool/future-issuance references and recalculations
without an unbounded cross-course transaction. The flawed version remains immutable historical and
superseded evidence; issued/graded evidence and grades remain intact. The immutable manifest
deterministically classifies in-progress reissue/excuse and completed-work recalculation with
superseding receipts. CourseInstance operations apply that manifested remediation without a
per-course score choice. Instructors receive audited impact/results and prospective controlled-update
choices; Sysadmin receives only a FERPA-safe projection, and the audit record is append-only.

`WP-INST-D1` owns availability and safe impact projection; `WP-INST-G1` owns compatibility,
grading impact, and generation-fenced recalculation; `WP-INST-G2` owns solution-free audited work
inspection; and `WP-INST-G5` owns action routing. SD1-B3/C/D carry the contract through source,
schema, and Store boundaries. Permanent tests, fresh PostgreSQL/RLS acceptance, and production
browser evidence are required. PostgreSQL proves atomic mapping/generation activation, bounded worker
materialization, no FERPA grant, and append-only receipts; browser evidence proves visible stop state,
audited Instructor impact/results, prospective controlled updates, and safe recovery. The emergency
path is not proven by a screenshot or source inventory.

## Future relationship target

Current course membership supports Student and Instructor. A future
`course_relationship`/`course_capability_grant` contract carries:

- subject `UserId` and exact `CourseId`;
- relationship kind and explicit capability set;
- issuer and issue time;
- active/revoked lifecycle and revision;
- audit identity; and
- required consent/disclosure policy.

The Grader relationship receives bounded grading work. The Course Observer relationship uses an
exact-course audited grant to show named assignment completion, no individual scores, and a separate
privacy-safe anonymous aggregate-grade projection. The Student Observer relationship receives a
consent-backed view of one Student through its own disclosure contract. Each workflow lands as a
complete future package.

`course_relationship` remains distinct from `course_member` and does not satisfy current Student-
owner, Instructor, roster, Gradebook, response, export, artifact, assignment-write, or worker
predicates. Course Observer completion output is a separately typed named completion projection;
its aggregate-grade output uses a separately typed projection with disclosure thresholds and omits
individual scores, responses, grade rows, small-cell, and linkable grade metadata. Student Observer
output binds one Student and one explicit revocable consent/disclosure record.

## Worker target contract

A worker handler derives its course, workspace, catalog, object, and provider target from the locked
current lease and immutable job manifest. Handler family, generation, broker grant, and target type
agree before work begins. Queue payload, retry input, provider response, object reference, and caller
input are evidence rather than authority.

Recurring restricted-login cases offer a valid lease a foreign-course object, foreign job target,
stale generation, wrong handler family, and forged provider completion. Each case closes before read,
write, dispatch, or finalization.

## Schema ownership

SD1-C replaces the active migration corpus with the fresh allocation registered in
[implementation_status.md](../implementation_status.md):

- principals, global accounts, sessions, passkeys, approval, and actor context;
- global immutable catalog and safe discovery evidence;
- private authoring, personal/shared collections, Stars, and saved searches;
- one ordered BlueprintCourse root/module/assignment tree with one aggregate revision, exact
  QuestionId/version pins, draft ACL, published vetted-Instructor projection, and relative schedule
  intent;
- CourseInstances, equal co-Instructors, Students, invitations, schedules, and delivery settings;
- exactly one immutable Blueprint parent/applied revision per CourseInstance; new Blueprint
  assignments are daughter projections marked unreleased until an explicit release;
- assignments, runs, attempts, submissions, artifacts, and retention identity;
- automated grading, Gradebook, analysis, version-specific thresholded evidence, and improvement
  threads;
- typed jobs, exports, objects, and external-tool state; and
- forced RLS, capability brokers, grants, and acceptance helpers.

Each migration owns its local relations, keys, constraints, indexes, functions, policies, grants,
and comments. The status-owned `2026082901`-`2026082932` epoch carries the course/curriculum
BlueprintCourse and CourseInstance slice in `2026082913`-`2026082916` and the broker/RLS/grants/helper
slice in `2026082929`-`2026082932`; SD1-C assigns exact files before implementation. Historical
`2026081837`-`2026081847` remains immutable evidence, not an active schema dependency.

## Validation ownership

### Permanent offline tests

- Domain authorization and type invariants.
- Memory/PostgreSQL Store contract conformance at the deterministic boundary.
- Generated browser contract and strict decoder behavior.
- Grading, idempotency, immutable evidence, revocation, and concealment behavior.
- BlueprintCourse nested ordering, exact pins, one parent/revision binding, minimal-Blueprint blank
  creation, relative schedule resolution, unreleased propagation, explicit release, divergent-edit
  preservation, archived references, and strict Alpha-reference refusal.
- Question stewardship: immutable version history, semantic change classes, visible lineage,
  owner edits, exact-base ChangeProposals with stale-base recovery, creator-private fork drafts,
  validated publication, controlled availability, one Star/private Watch and improvement events,
  correction manifest/remediation, and thresholded per-version evidence.

### PostgreSQL and service acceptance

- Fresh PostgreSQL migration and restricted-login actor/RLS behavior.
- A second co-Instructor performs one representative teaching mutation and one Gradebook/Student-
  work read, then receives immediate denial after revocation.
- The registered authorization matrix proves creator/co-Instructor equivalence, and approval-
  withdrawal cases close discovery, publication, course creation, FERPA reads, and racing writes.
- Student self versus another Student and another-course authorization.
- Audited Sysadmin support and ordinary FERPA refusal.
- Worker lease, object delivery, external adapter, export, retention, and cleanup behavior.
- Fabricated and revoked observer grants fail every current FERPA predicate.
- An exact-course Course Observer grant can read named assignment completion and thresholded
  anonymous aggregate grades, while individual Student scores remain unavailable.
- BlueprintCourse reads never grant CourseInstance or FERPA access; a published source remains
  answer-free and an archived referenced source remains resolvable.
- Source revision races, idempotent apply/retry, rollback, parent/revision uniqueness, unreleased
  propagation, and version-specific evidence thresholding pass on a fresh database.

### Production-browser acceptance

- Shared catalog discovery, collection, Star, saved-search, selection, reuse, and improvement.
- Two equal co-Instructors and connected Student activity through the ordinary Gradebook.
- Visible Instructor and Sysadmin passkey journeys.
- Role-appropriate accessibility and screenshot review.
- One connected BlueprintCourse journey creates/revises/publishes a nested tree, selects one bounded
  assignment projection, forks, instantiates into existing and new CourseInstances, resolves relative
  intent, shows unreleased propagation and explicit release, preserves divergence, and displays
  stewardship lineage/availability, **Suggest an improvement** proposal/review/rebase recovery, and
  correction impact. Visual review separately checks hierarchy, state, focus, contrast, recovery,
  and absence of Alpha labels.

### Visual acceptance

- Fresh Instructor and Sysadmin captures use the canonical 1280 by 800 profile; Student/access
  captures use maintained responsive profiles. Human review checks BlueprintCourse hierarchy,
  private/published authority, CourseInstance destination and release state, schedule provenance,
  archived-reference recovery, stewardship lineage/availability, focus, contrast, and readable
  conflict/error recovery.
- Rendered screenshots prove semantic presentation only. Screenshot bytes, pixel identity, counts,
  and viewport totals are not behavior or authorization gates; no-transport assertions stay in the
  browser lane.

### One-time evidence

- Graphify queries and affected-path reports.
- Broad retired-identifier/source/schema inventories.
- Old-to-new table, key, policy, function, grant, object, and route allocation.
- Clean-volume schema fingerprint and migration-count reconciliation.
- Graphify/source/schema/route/generated/browser-resource inventories, screenshot publication and
  provenance, and exact disposable cleanup receipts. These remain one-time evidence, not permanent
  count or filename tests.

The final material tree runs `source source_me.sh && ./all_test.sh` after every focused and connected
gate is green. The canonical database authorization reference is
[`DATABASE_AUTHORIZATION.md`](../../DATABASE_AUTHORIZATION.md); `DATABASE_TENANCY.md` is retired
migration input and is removed or archived during SD1-A closure.
