# Plan: Single-installation authorization and shared teaching model

## Context

PLE is one installation. It has global accounts, one shared published-question catalog, private
Instructor authoring workspaces, and course-scoped educational records. The product has no
institution boundary or institution selector.

Historical pre-SD1 implementation carries an obsolete global-installation identity, context,
keys, RLS state, and fields through the domain, Store contracts, PostgreSQL, workers, objects,
browser contracts, local-stack seed, and documentation. Graphify identifies that identity/context
pair as among the repository's highest-connectivity nodes. Direct source inspection found the model
across hundreds of Rust files and most active migrations.

PLE is pre-production, and its project-named live-stack data is disposable. This package therefore
establishes the intended model directly and creates a fresh migration epoch. It preserves the
product's strongest existing properties: deterministic server-owned grading, answer-free browser
contracts, immutable publication evidence, forced RLS, exact membership revocation, audited
Sysadmin support, shared problem discovery and reuse, and the canonical real-stack acceptance path.
The [single_installation_scope_register.md](single_installation_scope_register.md) assigns affected
repository owners to the atomic packages below.

The SD1 rebase also closes a pre-existing reusable-course split. The current one-assignment
`Blueprint` and multi-module `Alpha` implementations are two shapes of one product concept. The
canonical source is an ordered, answer-free, revisioned `BlueprintCourse`; the canonical delivery
aggregate is a separate exact-`CourseId` `CourseInstance`. The supplied BlueprintCourse/Alpha
consolidation map is one-time design evidence; current source and the contracts remain authoritative.
The registered `WP-R0`, `WP-R1`, `WP-R2`, and `WP-PY-L1` package identities and acceptance status
remain unchanged; SD1 introduces no replacement labels for them.

## Objectives

1. Give the installation one global account and session model keyed by `UserId`.
2. Make the immutable published-question catalog installation-wide and easy for every Instructor to
   discover, collect, share, reuse, and improve.
3. Give every approved Instructor the same product capabilities.
4. Support multiple equal co-Instructors in one course; course creation establishes the first
   Instructor membership rather than a privileged owner.
5. Authorize FERPA records through current exact course membership and Student ownership in the same
   database transaction as each protected operation.
6. Give private drafts and reusable curricula explicit user/workspace ownership and collaboration.
7. Give workers, objects, adapters, exports, retention, and integrations the smallest real scope
   required by their durable target.
8. Keep course authorization adaptable for future bounded Grader, Course Observer, and Student
   Observer relationships.
9. Rebuild the live demo on the resulting real stack and prove that Student activity becomes visible
   to every current co-Instructor through the ordinary grading path.
10. Rebase reusable curriculum and teaching delivery on one `BlueprintCourse` source and one
    `CourseInstance` destination, with immutable parent/revision binding, relative schedule
    resolution, unreleased upstream additions, and no Alpha compatibility surface.

## Design philosophy

- Model the product that exists: one PLE installation, global accounts, shared questions, private
  authoring, and exact course relationships.
- Express authority through real domain relationships and immutable references.
- Re-evaluate current membership and ownership at the protected Store/PostgreSQL boundary.
- Give equal co-Instructors the same course capability set.
- Represent future course relationships as explicit capability sets with complete privacy contracts.
- Keep published questions immutable and globally reusable; keep Student records course-scoped.
- Use one shared catalog visibility: every published assignment question remains discoverable and
  exactly resolvable to every approved Instructor across its lifecycle, with visible lifecycle state.
  Published items are ordinarily selectable through one `is_eligible_for_ordinary_new_selection`
  predicate; deprecated and archived items remain discoverable/resolvable
  for evidence and history but are excluded from ordinary new selection. Drafts remain private
  authoring material.
- Use forced, fail-closed PostgreSQL RLS and narrowly granted broker functions.
- Split migrations and source by durable capability so each file has one clear owner.
- Use permanent tests for stable behavior and one-time evidence for rebuild inventories.
- Prefer the clean pre-production model over compatibility machinery for disposable state.
- Keep reusable meaning and teaching state separate: BlueprintCourse owns reviewed reusable structure;
  CourseInstance owns live delivery and FERPA records. Fix the aggregate boundary rather than
  preserving duplicate types or adapters.
- Treat source, schema, API, and browser cutover as one dependency-ordered change. A branch is not
  accepted while it leaves a second Alpha authority or silently translates Alpha data.

## Scope

- Domain identifiers and authenticated actor context.
- Account, session, passkey, and seeded live-demo identity contracts.
- Course membership, co-Instructor authority, Student ownership, invitations, and revocation.
- Private authoring, curricula, discovery, collections, Stars, saved searches, and shared problem
  selection.
- PostgreSQL schema, keys, indexes, partitions, RLS, roles, grants, and capability brokers.
- Memory/PostgreSQL Store contracts and implementations.
- Runs, attempts, submissions, automated grading, Gradebook, analytics, exports, and retention.
- Worker leases, jobs, object metadata/delivery, adapters, and external-tool launch state.
- Browser DTOs, strict decoders, API routes, live-demo seed, local-stack orchestration, and docs.
- Migration and acceptance evidence required by `docs/TEST_EVIDENCE_MODEL.md`.

## Non-goals

- This plan keeps the current visible personas Student, Instructor, and Sysadmin.
- A later product package will deliver the complete Grader, Course Observer, or Student Observer
  workflow, including consent, aggregation, revocation, audit, and accessible interface behavior.
- This plan preserves useful domain normalization work for its owning roadmap unless the
  single-installation correction requires a key or ownership change for correctness.
- Production data conversion is outside this pre-production rebuild; fresh disposable state is the
  supported transition.

## Product and authority model

### Canonical reusable source and delivery model

`BlueprintCourse` is the only reusable course-level model. It is owned by a `WorkspaceId` and
`UserId`, has one aggregate revision, and contains an ordered tree of modules and assignments. Each
assignment carries reusable meaning, relative schedule defaults, and fixed/pool entries that resolve
to exact immutable published `ProblemVersionRef` pins. A draft is private to its owner and current
workspace collaborators. A deliberately published, answer-free projection is visible and reusable
by every vetted Instructor; it is never a Student or FERPA record.

`CourseInstance` is the ordinary teaching/delivery aggregate for one exact `CourseId`. Creation
always records exactly one immutable Blueprint parent and applied Blueprint revision. It copies
reusable assignment meaning and reviewed relative defaults, then owns Students, releases, live
deadlines, accommodations, grades, runs, and delivery settings. It has no live source tether. Blank
course creation uses a minimal Blueprint rather than a second reusable type. Blueprint visibility
does not grant CourseInstance or FERPA access.

Relative schedule intent is resolved only against the destination CourseInstance term and IANA zone.
The preview reports nonexistent or ambiguous local times and applies only its revision-checked
witness; the resulting deadline is CourseInstance-owned and may be edited there. A new upstream
Blueprint assignment propagates to daughter instances as unreleased. An explicit release decision
is required, and divergent delivery edits are preserved. Archived referenced Blueprints remain
resolvable for provenance and history.

The source and adoption vocabulary is operation-based: `ForkBlueprintCourse` creates an independent
BlueprintCourse, `InstantiateBlueprintAssignment` targets an existing CourseInstance, and
`InstantiateBlueprintCourse` creates a new CourseInstance. Rollover, term shift, fast-forward, and
selected copy remain destination-specific CourseInstance operations. `BlueprintReference` (`BP-*`)
is the only reusable-course locator. Alpha types, routes, schema branches, Store capabilities,
generated aliases, and browser resource kinds are removed in the fresh epoch; no compatibility alias
or silent Alpha translation is permitted.

### Human relationships

- **Fixed account role:** every account has exactly one immutable current Student, Instructor, or
  Sysadmin role. A person needing multiple roles uses separate accounts.
- **Student:** a Student account with current Student membership in an exact course. Student
  access is limited to that Student's own active educational records.
- **Instructor:** a manually approved Instructor account. Every approved Instructor has the same global product
  capabilities. One canonical `approved_instructor` predicate authorizes course creation,
  publication, catalog discovery, collections, reuse, and improvement. Current Instructor
  membership grants the complete teaching capability set for an exact course.
- **Co-Instructor:** another current Instructor member of the same course. All co-Instructors have
  equal course authority. Membership invitation, acceptance, and revocation are audited.
- **Sysadmin:** a Sysadmin account with platform lifecycle capabilities. A Sysadmin may create a
  CourseInstance on behalf of an explicitly assigned approved Instructor; the operation establishes
  that Instructor's first ordinary membership and gives the Sysadmin neither teaching nor FERPA
  authority. Publication, catalog discovery, and ordinary teaching authority use the same current
  `approved_instructor` predicate as any other account. FERPA access uses narrow, audited support
  operations. A person who needs teaching authority uses an approved Instructor account.
- **Future relationships:** Grader, Course Observer, and Student Observer use explicit bounded
  capability sets. A Course Observer has an exact-course audited relationship that can show named
  assignment completion, never individual scores, alongside privacy-safe anonymous aggregate grades.

Instructor approval remains an operator-owned account decision. Co-Instructor invitation and
acceptance verify the target account through `approved_instructor`; acceptance creates an ordinary
course membership only when its role is Instructor and leaves account approval unchanged.

An approved Instructor may create a CourseInstance directly. A Sysadmin may create one for an
explicitly assigned approved Instructor; the created course begins with that assigned Instructor as
its first ordinary co-Instructor. The creating Sysadmin acquires no course membership, teaching
capability, or FERPA access from this operation.

`current_course_instructor` requires both current `approved_instructor(user_id, now)` and current
direct Instructor membership. Approval withdrawal immediately closes every global Instructor
capability and course-Instructor FERPA operation in the same protected transaction. Existing course
records and membership history remain available to other authorized actors and explicit recovery
operations.

Course creation establishes an ordinary membership and no separate creator or owner authority. For every
course-Instructor operation, the creator and a subsequently accepted co-Instructor receive the same allow or
deny result for equivalent current state; only actor-attributed audit fields differ.

Future relationships use `course_relationship` with an explicit `course_capability_grant`: subject `UserId`,
`CourseId`, relationship kind, bounded capability set, issuer, lifecycle/revocation state, audit identity, and
required consent or disclosure policy. They remain distinct from current `course_member` rows and current
Student-owner and Instructor predicates. An exact-course, audited Course Observer can read named assignment
completion and anonymous aggregate-grade projections, never individual scores, responses, roster detail,
small-cell, or linkable grade metadata. A Student Observer binds one Student, explicit revocable consent, and
a distinct projection. Their operation-specific predicates remain future owning-package work.

### Ownership matrix

| Record or capability                                                                                              | Durable scope                                                     | Authorization source                                      |
| ----------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- | --------------------------------------------------------- |
| Account, email, passkey, session                                                                                  | `UserId` and server-issued session                                | Authenticated account/session                             |
| Published Instructor-safe question projection, public assets, provenance, and aggregate-safe improvement evidence | Global `ProblemId` and `VersionId`                                | Every approved Instructor                                 |
| Answer keys, private grading payloads, provider credentials, and unpublished source                               | Exact server capability or private workspace                      | Narrow server/workspace authority                         |
| Discovery, collections, Stars, saved searches                                                                     | `UserId`, plus global published references                        | Authenticated account and explicit sharing                |
| Private draft or curriculum workspace                                                                             | `WorkspaceId` plus owner/collaborator `UserId`                    | Current workspace relationship                            |
| Course, appearance, schedule, assignment                                                                          | `CourseId` and child identity                                     | Current direct Instructor membership for writes           |
| Course membership and invitation                                                                                  | `CourseId`, membership/invitation identity                        | Instructor membership or narrow audited support           |
| Student run, attempt, response, grade, artifact                                                                   | `CourseId` plus Student membership/owner and exact child identity | Student self or current course Instructor                 |
| Grading operation and Gradebook evidence                                                                          | `CourseId`/`AssignmentId` plus operation identity                 | Current Instructor membership or leased worker capability |
| Background job                                                                                                    | Typed course, workspace, catalog, or system scope                 | Lease row and exact durable target                        |
| Object metadata and delivery                                                                                      | One typed catalog, workspace, or course-record scope              | Database scope and current relationship                   |
| LTI launch and passback                                                                                           | Exact course, assignment, account, and launch identity            | Verified platform registration and launch binding         |

### Database authorization

PostgreSQL actor installation, ACL, broker, and RLS enforcement implement these product rules in
[single_installation_database_authorization_plan.md](single_installation_database_authorization_plan.md).

### Shared problem model

Published questions form one shared catalog. SD1-B3 owns these closed, Serde-generated `snake_case`
browser DTOs. Each DTO uses `deny_unknown_fields`, has no flattened extension map, and advances to a
new version when its field set changes:

The shared catalog is available to the authenticated approved-Instructor set. Student question delivery
continues through exact assignment entitlement, and anonymous web clients receive no catalog
authority.

- `InstructorCatalogQuestionV1`: `question_id`, `title`, `backend`, `response_family`,
  `capabilities`, `tags`, `taxonomy` entries containing exactly `scheme`, `code`, and `label`,
  `license` containing its closed `kind` plus `spdx` only for `other`, `byline` containing only
  `names`, `lifecycle` containing closed `state` plus `reason` for deprecated/archived content, and
  `published_at`.
- `InstructorCatalogEvidenceV1`: closed `state`; the available state contains exactly
  `formula_version`, `observed_course_count`, `independent_student_observation_count`,
  `difficulty_index`, `attempts_mean`,
  `time_median_seconds_estimate`, optional `discrimination_index`, and `evidence_at`; insufficient
  evidence contains only `state`. The available state is emitted only when every released count and
  metric satisfies the named disclosure threshold; otherwise the response is the sole insufficient
  state. `formula_version` binds the aggregation formula and disclosure-threshold policy.
- `InstructorCatalogUsageV1`: `global_course_count`, `global_assignment_count`, `own_course_count`,
  `own_assignment_count`, `own_courses` entries containing exactly `course_reference`, `title`, and
  `assignment_count`, and `own_courses_truncated`.
- `InstructorCatalogLineageV1`: closed `relationship`, optional `source_question_id`, optional
  `improvement_thread_reference`, and bounded `published_successor_question_ids`; it carries no actor
  or course identity.
- `InstructorCatalogSearchItemV1`: exactly `question` and `evidence`.
- `InstructorCatalogSearchPageV1`: exactly `items`, `next_cursor`, and `facets`; the closed facets are
  `bylines` entries with `byline` and `count`, `backends` entries with `backend` and `count`, `tags`
  entries with `tag` and `count`, `response_families` entries with `response_family` and `count`,
  `taxonomy` entries with `scheme`, `code`, `label`, and `count`, `capabilities` entries with
  `capability` and `count`, `licenses` entries with `license` and `count`, and
  `evidence_availability` entries with `availability` and `count`.
- `InstructorCatalogDetailV1`: exactly `question`, `prompt`, `evidence`, `usage`, and `lineage`.
  `prompt` has closed `kind` (`static` or `generated_example`) and `blocks`. A text block has exactly
  `kind` and `markdown`; math has `kind`, `latex`, and `description`; image has `kind`, public
  `asset_reference`, `checksum`, and `description`; code has `kind`, `language`, and visible listing
  `source`; table has `kind`, `headers`, `rows`, and `description`.

The count fields have distinct, documented provenance. `InstructorCatalogEvidenceV1` is derived from
Student performance and applies the formula-versioned disclosure threshold above. Search facet
counts are counts of globally published question metadata in the current query snapshot and contain
no course or Student contribution. Usage counts are counts of assignment-to-question references;
global values contain no course identity, while `own_*` and `own_courses` are authorized exact course
records already visible through the current Instructor's memberships. Usage and facet counts never
derive from Student responses, scores, enrollments, or identities.

These DTOs exclude Student-linked data, accepted responses, grades, cohort-identifying counts below
the disclosure threshold, answer keys, scoring rules, private grader payloads, source packages,
provider identifiers/credentials, object keys, signed URLs, workspace identifiers, and arbitrary
metadata. Public presentation-asset delivery is a distinct capability from draft/source delivery.

Every approved Instructor may search the safe projection, save searches, create collections, create
a Star, select shared questions for assignments, and start an improvement thread that publishes a
new immutable question. Existing assignments retain their exact references until an Instructor
applies an explicit revision-checked replacement.

Every published question stays discoverable and exactly resolvable to every approved Instructor,
including when its lifecycle state is deprecated or archived. Catalog search and detail visibly label
that state and its reason. Active questions are ordinarily selectable; deprecated and archived
questions are excluded from ordinary new selection and new references while remaining available for
evidence, provenance, and history. Drafts remain private to their owner/collaborator workspace and
have no shared-catalog identity until validated publication succeeds.

Private drafts remain in an owner/collaborator workspace. Publication crosses that boundary only
after validation succeeds. Publication creates one shared Instructor-visible catalog state rather than
another publication-scope branch. Collections and Stars remain personal or
explicitly shared; their visibility does not change the visibility of the published questions they
reference. Shared problem use never carries Student identity into catalog records or aggregate
improvement evidence.

### Question stewardship binding

The reusable-course cutover preserves the owner's stewardship contract for the shared question
catalog. A stable human-facing `QuestionId` identifies a question lineage; its immutable
`QuestionVersion` history records each published meaning. Assignments and BlueprintCourse entries
pin the exact published QuestionId/version (and hidden immutable evidence needed for replay), never a
mutable latest pointer. Availability is explicit: only Published versions are ordinarily selectable,
through one `is_eligible_for_ordinary_new_selection` predicate, while
deprecated or archived versions remain discoverable and resolvable for evidence, history, and existing
pins but are excluded from ordinary new selection. Semantic change classes are closed and reviewed:
presentation-only, metadata, and other compatible edits create a new immutable same-lineage version.
A validated correction to a wrong key, calculation, scoring rule, or no-correct-answer defect that
preserves the question's objective, task, response family, purpose, and answer expectations creates a
new immutable `QuestionVersion` in the same lineage with explicit impact and recalculation evidence.
A change to the objective, task, response family, purpose, or substantially different answer
expectations is a FullFork that creates a new `QuestionId`. The publication decision records the
class, affected exact pins, and required impact/recalculation evidence.

Published Question stewardship has four change paths. The question owner makes a moderate edit that
creates a new immutable version in the same lineage while preserving original authorship and
compatible CC licensing. Any approved Instructor may submit a `QuestionChangeProposal` against one
exact base QuestionVersion. A proposal contains a validated proposed patch, rationale, semantic and
grading-impact result, contributor credit, and lifecycle state. The owner accepts or rejects it;
acceptance creates the next immutable version in that same lineage with contributor credit. When the
lineage advances first, the proposal becomes stale and its author rebases or resubmits it against the
new exact base version.

Fork lineage is visible in the safe catalog projection. A fork creates a creator-private draft in
the fork owner's workspace, with source attribution and compatible CC licensing; publication creates
a separate lineage only after complete source, answer-free projection, exact-pin, licensing/byline,
and semantic-change validation. Stars and watches are UserId-owned curation events; improvement
events, Change Proposals, and linked replacements are durable, auditable catalog events with no
Student identity. A correction that changes grading semantics records affected exact pins,
assignment/run impact, and the required generation-fenced recalculation or explicit refusal before
any replacement becomes available. It never mutates issued evidence.

Per-version attempts, correct counts, and eligible-choice counts are separate evidence families.
They are disclosed only through the existing version-specific privacy-threshold formula, with an
explicit insufficient-evidence state and no course, Student, response, or small-cohort identity.
Question stewardship belongs across existing package owners: `WP-INST-D1` owns discovery, lineage,
and thresholded evidence; `WP-INST-D2` owns one UserId-owned Star concept, private watches,
collections, and selection;
`WP-INST-G1` owns grading-correction impact and generation-fenced recalculation; `WP-INST-G2` owns
audited learner-work impact inspection; `WP-INST-G3`/`G4` own linked replacement analysis and
improvement events. SD1-B3 carries the closed contract and SD1-C/D preserve it in schema and Store
ownership; no new top-level package ID is introduced.

### ForcedQuestionCorrection package contract

An emergency security or critical-correctness finding uses a Sysadmin-approved
`ForcedQuestionCorrection`. The validated replacement is prepared first, then a closed, privacy-safe
impact manifest names the affected QuestionVersion, assignments, pools, BlueprintCourses,
CourseInstances, and future issuance pins. One transaction atomically updates active pins and stops
new selection and issuance of the flawed version by activating one authoritative replacement mapping
and generation. Bounded idempotent generation-fenced workers materialize affected references and
recalculations from that immutable manifest; the product is atomic without an unbounded cross-course
transaction. The flawed version remains immutable historical and superseded evidence; existing exact
pins, issued/graded work, and grades are preserved.

The immutable manifest determines deterministic remediation: it classifies in-progress work for
reissue or excuse and completed work for recalculation, then writes superseding receipts without
rewriting evidence. Every affected CourseInstance applies that manifested remediation; score
selection is not a per-course decision. Instructors receive audited impact and result views plus
prospective controlled-update choices for future delivery. Sysadmin receives only the FERPA-safe
impact projection. Replacement publication still passes source, answer-free, exact-pin,
licensing/byline, and semantic-change validation.

`WP-INST-D1` owns version availability, validated replacement, safe impact manifest, and lineage;
`WP-INST-G1` owns compatibility, reissue/excuse, grading impact, and generation-fenced
recalculation; `WP-INST-G2` owns audited solution-free Instructor work inspection; and
`WP-INST-G5` owns action routing. SD1-B3 defines the contract and SD1-C/D enforce it in schema/Store.
Permanent tests cover deterministic manifest classification and immutable/superseding receipts;
PostgreSQL acceptance covers Sysadmin-only grants, forced RLS, no FERPA data, atomic pin stop/update,
and append-only audit; production-browser acceptance covers visible stop state, audited Instructor
impact/results, prospective controlled updates, recovery, and safe impact messaging. These are
existing package responsibilities and add no new top-level package ID.

## Security requirements

- Document function-, data-, and field-level access in the ownership matrix and each Store/API
  contract (ASVS 8.1.1, 8.1.2).
- Enforce explicit function, object, and field permissions at the trusted server and PostgreSQL
  boundaries (ASVS 8.2.1, 8.2.2, 8.2.3, 8.3.1).
- Re-evaluate Instructor approval, membership, Student ownership, and relationship revocation in the
  protected transaction so changes take effect immediately (ASVS 8.3.2).
- Keep creator and co-Instructor authorization equivalent through one registered operation matrix;
  attach actor identity only to audit evidence (ASVS 8.1.1, 8.2.1, 8.2.2).
- Apply membership invitation, acceptance, roster revision, and revocation as atomic transactions
  with row locking and exact replay behavior (ASVS 2.3.1, 2.3.3, 2.3.4).
- Keep email, passkey, seeded-demo, and later external identity pathways documented together with
  consistent account/session controls (ASVS 6.1.3, 7.1.1, 7.1.2).
- Maintain the repository's sensitive-data classification and apply minimum response fields,
  `no-store` delivery where required, and scheduled retention to FERPA records (ASVS 14.1.1,
  14.1.2, 14.2.2, 14.2.6, 14.2.7).
- Log authentication events, authorization failures, approval/membership/revocation changes, and
  sensitive-access audit metadata while keeping Student responses, grades, credentials, and private
  grading material out of security logs (ASVS 16.1.1, 16.3.1, 16.3.2, 16.3.3).
- Treat contextual risk controls for privileged production interfaces as production-activation
  evidence under their owning release package (ASVS 8.4.2, Level 3).

## Migration strategy

The current `schemas/migrations/` and its sole compile-time `MIGRATOR` remain the executable epoch
until one reviewed promotion. SD1 staging is non-runtime: it cannot change ordinary status,
local-stack control, or live-demo execution. The preassigned fresh-epoch allocations are solely in
[implementation_status.md](../implementation_status.md); the staged shape has one
BlueprintCourse/CourseInstance model and no Alpha bridge. Historical `2026081881` and `2026081882`
remain immutable evidence/input.

### Principal and ACL contract

The status-allocated principal, ACL, and promotion contracts are in
[single_installation_database_authorization_plan.md](single_installation_database_authorization_plan.md).

### Actor and RLS contract

The status-allocated actor installation, broker, RLS, and Store enforcement contracts are in
[single_installation_database_authorization_plan.md](single_installation_database_authorization_plan.md).

### One reviewed promotion

The companion owns one reviewed promotion and connected acceptance; this plan retains the product
constraint that no partial epoch, obsolete-scope compatibility SQL, or Alpha bridge becomes active.

## Dependency order

```text
SD1-A decisions and exact inventory
  -> SD1-B domain and authorization contracts
     -> SD1-C fresh PostgreSQL epoch
        -> SD1-D Store implementations
           -> SD1-E services, workers, objects, and adapters
              -> SD1-F browser and live-demo contracts
                 -> SD1-G real-stack and release-plan closure
```

Within each package, independent ownership slices may proceed in parallel after their shared contract
is accepted. Every implementation slice receives one owner, one exact file boundary, one outcome,
and one narrow verification command. Independent review follows implementation.

## Milestone SD1-A: Decisions and impact contract

**Dependencies:** Owner guidance and current Graphify/source evidence.

**Deliverables:**

- Make `HUMAN_GUIDANCE.md`, `DESIGN_DECISIONS.md`, `USER_ROLES.md`, this plan, and
  `implementation_status.md` agree on the single-installation model.
- Record the fixed-role account clarification: account/session storage has one immutable role;
  Student/Instructor membership matches that role; Sysadmin provisioning assigns an approved
  Instructor account without creating Sysadmin membership; and support remains explicit and audited.
- Publish the ownership-complete
  [single_installation_scope_register.md](single_installation_scope_register.md) for domain,
  PostgreSQL, browser, object, seed, and documentation boundaries.
- Classify every record with the ownership matrix above.
- Allocate the fresh migration epoch and identify the accepted behaviors each new migration must
  preserve.
- Reconcile the unaccepted `2026081881` and `2026081882` work into the new allocation.
- Replace the active authority sections in `SECURITY_MODEL.md`, `AUTHORIZATION_CONTRACTS.md`,
  `DATABASE_AUTHORIZATION.md`, and `IDENTITY_CONTRACTS.md` with the actor, exact-domain ownership,
  approved-Instructor, shared-publication, and typed-lease contracts before SD1-B begins.
- Retire `DATABASE_TENANCY.md` as the former migration-input document after its consumers are
  redirected; maintain `DATABASE_AUTHORIZATION.md` as the sole canonical database authorization
  reference for the fresh SD1 epoch.
- Record the BlueprintCourse cutover inventory: Alpha/Blueprint domain symbols, adoption/source
  unions, Store methods/capabilities, SQL relations/functions, route-policy branches, generated
  contracts, browser clients/components, live-demo resource kinds, and screenshot names. Mark
  each as remove, replace, or historical evidence; do not preserve an Alpha alias.
- Freeze the Question stewardship handoff to existing discovery, curation, grading, audited-work,
  analysis, and improvement packages, including stable QuestionId/version history, semantic change
  classes, controlled availability, lineage, and version-specific privacy evidence.

**Workstreams:**

- `SD1-A1` decisions and fixed-role account/capability vocabulary.
- `SD1-A2` Graphify-assisted Rust/API/worker/object impact register.
- `SD1-A3` PostgreSQL table/key/policy/grant/broker register.
- `SD1-A4` browser, local-stack, live-demo, and binding authority-document replacement.
- `SD1-A5` independent architecture and privacy review.

**Verification:** Markdown and link/style gates; direct source inventories and Graphify reports
recorded as one-time evidence; the BlueprintCourse/Question stewardship ownership matrix has no
unassigned consumer; independent review returns `ACCEPT`.

**Entry:** Owner decisions are recorded in Human Guidance.

**Exit:** Every affected owner has one successor package and the status registry selects `SD1-B1`.

**Parallel-plan ready:** Yes. A2, A3, and A4 are independent after A1 fixes the vocabulary.

## Milestone SD1-B: Domain and authorization contracts

**Dependencies:** Accepted SD1-A register.

**Deliverables:**

- Define `ActorContext` only in `crates/learning-data-access/src/session.rs`; `SD1-B1-P0` owns its
  server-only, non-forgeable type boundary, while `SD1-B1-F` derives it from a resolved session
  record after exact-scope storage and service support exists. `rls.rs` is transaction-adapter-only:
  it adapts that context to transaction-local PostgreSQL state and defines no `ActorContext` type;
  exact domain scopes replace the retired global identity.
- Make course creation and multiple equal Instructor memberships first-class domain behavior.
- Define the canonical `approved_instructor` predicate for every global Instructor capability and
  verify it when a co-Instructor accepts a course relationship.
- Define `current_course_instructor` as the conjunction of current approval and current direct
  Instructor membership, with transaction-time approval withdrawal.
- Bind Student records to exact course membership/owner and child identity.
- Define workspace owner/collaborator, shared-catalog, typed-job, and typed-object scopes.
- Define the future `course_relationship` and `course_capability_grant` target contract while
  exposing only current live behaviors.
- Define the approved-Instructor catalog projection as the explicit safe field allowlist above.
- Define one `BlueprintCourse` source contract with ordered modules/assignments, one aggregate
  revision, exact published question-version pins, relative schedule intent, draft ACL, and vetted-
  Instructor published projection. Define `CourseInstance` as the separate exact-course destination
  with exactly one immutable Blueprint parent/applied revision and no learner-state copy.
- Define destination-specific fork, assignment adoption, whole-course instantiation, rollover, term
  shift, fast-forward, selected copy, and explicit release operations. Define blank-course creation
  from a minimal Blueprint and new-upstream-assignment propagation as unreleased.
- Define the Question stewardship contract: stable QuestionId lineage with immutable versions,
  owner moderate edits, exact-version `QuestionChangeProposal`s, visible fork lineage,
  creator-private drafts, validated publication, controlled availability, one UserId-owned Star,
  private Watch subscriptions, improvement events, correction impact/recalculation, and
  version-specific thresholded evidence.
- Regenerate direct browser contracts from the accepted Rust Serde owners.

**Workstreams:**

- `SD1-B1-P0` server-only account/session and actor contract declaration; `SD1-B1-F` final
  resolved-record session/auth integration and obsolete-route retirement.
- `SD1-B2` approved-Instructor, course membership, equal co-Instructor, Student ownership, and future
  relationship target contracts.
- `SD1-B3` catalog, authoring workspace, collection, `QuestionChangeProposal`, and improvement
  contracts, including child `WP-SD1-B3-B6` ordinary-new-selection semantics.
- `SD1-B4` job, object, external-tool, and integration scope contracts.
- `SD1-B5` generated TypeScript and strict decoder contract roots.

`SD1-B3` owns the BlueprintCourse source projection and Question stewardship contract, including the
exact-base proposal state machine and contributor-credit receipt; `SD1-B5` owns the single BP-only
route/reference and generated browser contract roots. Adoption commands are destination-specific and
remain a separate B2 boundary.

`SD1-B2` through `SD1-B4` define typed exact-scope contract roots. They make no claim that current
routes resolve an actor. SD1-C/D own the fresh schema, Store/RLS, and direct protected-service
implementation that makes those contracts executable. `SD1-B1-F` then assembles the resolved-record
authentication seam, migrates affected consumers to their exact scopes, and retires the prior
obsolete global-scope session route. This coordination has one final session model and no
installation-wide fallback.

**Verification:** Focused domain tests, Store trait compilation, generated-contract tests, strict
Clippy, TypeScript compilation, and independent architecture/security review. The contract gate
must prove ordered nested positions, exact QuestionId/version pins, relative intent (not deadlines),
strict AC-reference refusal, immutable parent/revision binding, and no Alpha source variant.

**Entry:** The scope register assigns every affected public type and consumer.

**Exit:** Every domain family has a typed exact-scope contract root, and no SD1-B contract treats
the retired global identity as an authorization grant. PostgreSQL implementation and connected acceptance
remain SD1-C/D work.

**Parallel-plan ready:** Yes. After accepted B1-P0, B2-B4 contract roots may proceed in parallel.
Their service/route integration follows accepted C/D support; B1-F then closes the shared auth
cutover, and B5 follows the final server boundary.

## Milestone SD1-C: Fresh PostgreSQL epoch

**Dependencies:** Accepted SD1-B1-P0, accepted SD1-B2--B4 exact-scope contract roots, and the
status-registry allocation.

**Outcome:** A complete fresh staged epoch enforces this plan's global-account, shared-catalog,
private-authoring, exact-course, equal co-Instructor, and Student-privacy contracts without an
Alpha bridge.

**Handoff and exit:** [single_installation_database_authorization_plan.md](single_installation_database_authorization_plan.md)
owns SD1-C1--SD1-C8, staging, ACL/RLS enforcement, and PostgreSQL gates. SD1-D begins only after
its accepted database boundary.

## Milestone SD1-D: Store implementations

**Dependencies:** Accepted SD1-C database boundary and accepted SD1-B2--B4 exact-scope contracts.

**Outcome:** Memory and PostgreSQL Stores enforce the same main-plan ownership and concealment
contracts, then make the protected service boundary available to SD1-E and SD1-B1-F.

**Handoff and exit:** [single_installation_database_authorization_plan.md](single_installation_database_authorization_plan.md)
owns SD1-D1--SD1-D6, Store/RLS gates, and connected protected-service acceptance. SD1-E begins only
after that accepted boundary.

## Milestone SD1-E: Services, workers, objects, and adapters

**Dependencies:** Accepted SD1-D Store capabilities, SD1-B1-F authentication integration, and
SD1-B5 browser-safe contract root.

**Deliverables:**

- Derive actor identity from authenticated sessions and course authority from current membership.
- Route all course Instructor members through the same service capabilities.
- Claim typed jobs globally and derive their target scope from the leased durable row.
- Derive every handler target from the locked current lease and immutable job manifest. Handler
  family, generation, broker grant, and target type must agree.
- Bind object metadata/delivery to one exact catalog, workspace, or course-record scope.
- Bind external-tool launch, provider cache, export, retention, and future LTI state to their exact
  domain identities.
- Preserve server-only provider credentials, answer material, deterministic grading, and audit
  evidence.
- Route BlueprintCourse fork, assignment adoption, and whole-course instantiation through explicit
  destination authorization. Blank CourseInstances use the minimal-Blueprint path; every instance
  retains one immutable parent/revision receipt. Resolve relative schedule intent before apply,
  preserve a corrected witness, and require explicit release of propagated assignments.
- Make Question stewardship effects explicit: owner edits, ChangeProposal validation/submission/
  review, fork lineage, and publication validation stay in the catalog boundary; Star remains
  UserId-owned and Watch remains private; grading-semantic corrections apply their immutable manifest
  and generation-fenced recalculation; no worker or provider input can widen a pin, alter manifested
  remediation, or mutate issued evidence.

**Verification:** Focused server/worker/adapter/object tests; retry/fencing/revocation behavior;
restricted PostgreSQL cases offering a valid lease a foreign-course object, foreign job target,
stale generation, wrong handler family, and forged provider completion; object-store E2E lanes; and
independent security review. Each adversarial case fails before read, write, dispatch, or
finalization. Service/API tests additionally prove source revision binding, destination-specific
authorization, controlled update/release, and correction impact/recalculation behavior.

**Entry:** Store implementations provide complete typed capability boundaries.

**Exit:** Server and worker composition contains only real domain scopes and passes its focused live
service lanes.

**Parallel-plan ready:** Yes. Auth/routes, workers, objects, adapters, and exports may proceed as
separate owners after the shared Store interfaces accept.

## Milestone SD1-F: Browser and live-demo workflow

**Dependencies:** Accepted SD1-E routes and generated contracts.

**Deliverables:**

- Present one global account/session experience and course selection derived from memberships.
- Show equal co-Instructors in the course roster and provide accessible invitation/revocation flows.
- Preserve stable Instructor desktop course navigation.
- Deliver live shared problem search, evidence, collections, Stars, saved searches, selection,
  reuse, and improvement-thread workflows, including **Suggest an improvement** for a validated
  exact-version ChangeProposal.
- Seed one real course with at least two equal co-Instructors and connected Students.
- Carry a Student submission through deterministic grading to the Gradebook visible to both
  co-Instructors.
- Preserve visible passkey testing for Instructor and Sysadmin demo personas.
- Provide one BlueprintCourse workspace/list/detail/editor and one nested module/assignment picker;
  a one-assignment reuse is a bounded location in the same tree. Keep `/api/course-blueprints` as
  the sole reusable route family and accept BP references only. Fork and adoption UI chooses the
  destination operation, including existing-versus-new CourseInstance.
- Show one immutable parent/applied revision per CourseInstance, resolved relative intent and its
  provenance, propagated assignments as unreleased, explicit release, and preserved divergent
  delivery edits. Remove Alpha route calls, labels, resource kinds, generated aliases, and screenshot
  names; do not retain compatibility parsing.
- Preserve the stewardship journey: owner moderate edit, **Suggest an improvement** submission and
  reviewed acceptance/rejection, visible fork lineage, creator-private fork draft, validated
  publication, controlled active/deprecated/archived availability, one Star/private Watch and
  improvement events, and version-specific thresholded counts without Student identity. Vetted
  Instructors may see Star count and Star identities; Students and anonymous users see neither.

**Verification:** Browser contract tests for stable decoders; canonical production-stack Playwright
journeys in which the second co-Instructor performs one teaching mutation and one Gradebook/Student-
work read, followed by immediate revocation denial; accessible keyboard/name/state checks; fresh
Instructor/Sysadmin 1280 by 800 screenshots; Student viewport distribution from Human Guidance;
response inspection proving excluded catalog fields stay server-side; one BlueprintCourse to
CourseInstance creation/update/release journey; archived-reference and AC-reference refusal;
stewardship/lineage, ChangeProposal stale-base recovery, and correction-impact outcomes; independent
HCI review. Browser and visual evidence are separate acceptance lanes.

**Entry:** Real service endpoints expose the intended contracts.

**Exit:** The live demo demonstrates connected users, shared problems, equal co-Instructors, and
ordinary Student-to-Gradebook convergence.

**Parallel-plan ready:** Yes. Identity/course roster, discovery/reuse, Student delivery, and visual
evidence may proceed in parallel against accepted route contracts.

## Milestone SD1-G: Repository and release closure

**Dependencies:** Accepted SD1-F live behavior.

**Deliverables:**

- Reconcile architecture, security, API, data, object, local-stack, install, usage, and active-plan
  documentation with the implemented single-installation model.
- Update Graphify and use it as navigation evidence for the final impact review.
- Record one-time retired-model inventories separately from permanent gates.
- Run the complete final-tree validation and production-stack screenshot corpus.
- Obtain independent architecture, security/privacy, code, and HCI acceptance.
- Reconcile all technical docs, generated contract records, seed/scenario resource kinds, and
  screenshot manifests to one BlueprintCourse/CourseInstance model and one Question stewardship
  vocabulary. Keep old Alpha names only where they are immutable historical evidence or external
  comparison text.

**Verification:** Documentation gates, focused live-stack lanes, PostgreSQL/RLS acceptance,
production-browser behavior, separate visual review, and final `source source_me.sh && ./all_test.sh`
with every required lane run. One-time Graphify/source/schema/generated/screenshot inventories and
cleanup receipts remain implementation evidence, not permanent tests.

**Entry:** All product behavior and focused gates are green.

**Exit:** The repository, schema, browser, live demo, and documentation express one coherent
single-installation platform; all required acceptance evidence is recorded.

Release-wide dependency, acceptance, migration, risk, rollout, and closeout decisions remain binding
in [release_completion_plan.md](release_completion_plan.md).

**Parallel-plan ready:** Yes. Documentation, one-time inventories, visual review, and independent
audits may proceed together before the sequential final aggregate.

## Validation lanes

### Permanent offline tests

- Domain and Store conformance for actor, membership, owner, revocation, concealment, immutable
  publication, idempotency, and grading evidence.
- One data-driven course-Instructor authorization matrix proving creator/co-Instructor equivalence in
  Memory and Store conformance.
- Generated-contract and strict-decoder assertions for the approved-Instructor catalog field
  allowlist.
- BlueprintCourse domain/Store invariants: ordered nested modules/assignments, exact QuestionId/
  version pins, one immutable parent/revision per CourseInstance, minimal-Blueprint blank creation,
  relative-intent resolution, unreleased upstream propagation, explicit release, divergent-edit
  preservation, archived-reference resolution, and strict refusal of Alpha variants.
- Question stewardship behavior: immutable version history under stable lineage, semantic change
  classes, owner edits, exact-base ChangeProposals and stale-base recovery, visible fork lineage,
  creator-private drafts, validated publication, controlled availability, UserId-owned Star/private
  Watch and improvement events, correction manifest/remediation, and version-specific privacy-
  thresholded evidence.
- Final repository aggregate owned by `all_test.sh`.

### PostgreSQL and service acceptance

The connected PostgreSQL, Store/RLS, protected-service, role/ACL, broker, and promotion gates are
owned by [single_installation_database_authorization_plan.md](single_installation_database_authorization_plan.md).
This plan retains browser-visible equal co-Instructor, Student privacy, catalog, CourseInstance, and
no-Alpha outcomes.

### Production-browser acceptance

- Canonical Playwright journeys for shared-question discovery/reuse, equal co-Instructor behavior,
  lifecycle-visible catalog discovery/reuse, equal co-Instructor behavior, immediate revocation, and
  Student submission-to-Gradebook convergence.
- Browser response inspection for the safe catalog field allowlist.
- Accessibility interaction evidence and role-appropriate rendered screenshot review.
- Production HTTPS BlueprintCourse workflow: create/revise/publish one nested tree, select one
  assignment projection, fork, instantiate into existing and new CourseInstances, resolve schedule
  intent, observe unreleased propagation, explicitly release, preserve divergence, and inspect
  stewardship lineage/availability, **Suggest an improvement** proposal/review/rebase recovery, and
  correction impact. Visual review separately checks hierarchy, state, focus, recovery, and no Alpha
  vocabulary.

### Visual acceptance

- Fresh Instructor and Sysadmin captures use the canonical 1280 by 800 profile; Student/access
  captures use the maintained responsive profiles. Review BlueprintCourse hierarchy, private versus
  published authority, CourseInstance destination and release state, relative-intent provenance,
  archived-reference recovery, stewardship lineage/availability, focus, contrast, and readable
  error/conflict recovery.
- Visual review is semantic and human-owned. Screenshot bytes, pixel identity, artifact counts, and
  viewport totals are not behavior gates; rendered images do not prove authorization or no transport.

### One-time implementation evidence

- Graphify impact maps and broad source inventories.
- Old-to-new schema/table/key/policy/grant allocation.
- Direct source search confirming retired global-scope contracts are absent after the rebuild.
- Migration-epoch comparison, clean-volume schema fingerprint, and migration-count reconciliation.
- Temporary diagnostics used to understand dependency ordering or query plans.
- Graphify impact maps, source/schema/route/generated/browser-resource inventories, migration
  registration and cleanup receipts, and screenshot publication/provenance are one-time evidence.

One-time evidence is recorded in the changelog or acceptance receipt and removed from the permanent
test tree when it does not satisfy [PYTEST_STYLE.md](../../PYTEST_STYLE.md).

## Risks and controls

| Risk                                                                     | Control                                                                                                                                                               |
| ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Course membership becomes an ambient read grant                          | Use operation-specific predicates; Student records require Instructor membership or exact Student ownership.                                                          |
| Equal co-Instructor behavior accidentally preserves a privileged creator | Represent course creation as the first ordinary Instructor membership and exercise another co-Instructor through the same mutations and Gradebook reads.              |
| Future observer design weakens current FERPA boundaries                  | Use explicit exact-course audited relationships, named completion-only projections, and privacy-safe anonymous aggregate-grade projections with no individual scores. |
| Schema rebase or a partial epoch loses product behavior                  | [single_installation_database_authorization_plan.md](single_installation_database_authorization_plan.md) owns the staged-epoch and promotion controls.                |
| Worker or object access uses caller-supplied scope                       | Derive typed scope from locked database metadata and server-issued opaque identities.                                                                                 |
| Shared catalog accumulates Student data                                  | Keep catalog evidence immutable, content-focused, aggregate-safe, and detached from Student identifiers.                                                              |
| Broad rebuild encourages fragile inventory tests                         | Keep inventories as one-time evidence and retain permanent behavioral gates only.                                                                                     |
| Blueprint and delivery state become one mutable aggregate                | Keep one immutable Blueprint parent/revision receipt per CourseInstance; resolve and edit live deadlines only in the instance.                                        |
| Upstream Blueprint changes silently alter teaching                       | Propagate new assignments as unreleased, compare source/import/delivery revisions, and require explicit release or selected-copy decisions.                           |
| Question correction mutates issued work or leaks small cohorts           | Pin exact versions, classify semantic changes, preserve immutable evidence, generation-fence recalculation, and disclose version counts only above threshold.         |
| Alpha compatibility survives as a second authority                       | Use one BP route/Store/schema/decoder family and a one-time retired-symbol inventory; fail closed on AC references and remove aliases.                                |

## Success criteria

- The active domain, schema, Store, server, browser, jobs, objects, local stack, and documentation use
  exact user/workspace/course/Student ownership and contain no institution-boundary contract.
- Every approved Instructor has the same global product capabilities.
- Multiple equal co-Instructors can manage one course and see the same current calculated Gradebook.
- Students see only their own course work; another-course and another-Student access fails closed.
- Shared problem discovery, collections, Stars, saved searches, reuse, and improvement remain
  visible live workflows backed by immutable catalog evidence; every published lifecycle state remains
  discoverable/resolvable to approved Instructors, while only Published items are ordinarily selectable.
- Sysadmin passkey testing remains visible while general FERPA access stays outside ambient
  Sysadmin authority.
- BlueprintCourse is the sole reusable course source, CourseInstance is the sole teaching delivery
  aggregate, every instance has one immutable parent/applied revision, and upstream additions remain
  unreleased until an explicit decision.
- Question stewardship preserves immutable version history, exact pins, owner edits, validated
  ChangeProposals, visible lineage, creator-private forks, UserId-owned Star/private Watch and
  improvement events, and version-specific privacy-thresholded evidence without mutating issued work.
- No current SD1 source, schema, Store, route, generated contract, browser resource, or acceptance
  surface exposes an Alpha compatibility alias.
- A clean `./run_live_demo.sh` launch demonstrates the connected Student-to-Instructor grading path.
- The required final `source source_me.sh && ./all_test.sh` run passes with no required skipped lane.
- Independent architecture, security/privacy, code, and HCI reviews accept the final boundary.
