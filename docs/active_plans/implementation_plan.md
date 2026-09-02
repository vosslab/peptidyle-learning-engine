# Plan: Peptidyle Learning Engine platform build

## Status

**Binding single-installation architecture (2026-08-29).** PLE operates as one installation with
global accounts, an Instructor-visible Question Library for every Published Question,
private drafts, equal active Instructors, multiple equal Teaching Team Members per course, and exact
course/Student authorization for educational records. The active SD1 registry owns the domain,
schema, Store, service, browser, live-demo, and documentation correction before the remaining
package sequence resumes. Its fresh pre-production migration epoch is the authority for database
ownership and authorization.

`WP-SD1-A-TERM-01-RQB1` is accepted historical evidence. `WP-SD1-A-TERM-01-RQB2` replaces
its prior server-boundary terminology with exact iMathAS Session, Result Exchange, Result, and
Transport terms while preserving its security receipts.

`WP-SD1-A-TERM-01-RQB2` is accepted after RQB1. It directly replaces the remaining generic
backend-session names with exact iMathAS Session, Challenge, Authentication, Grading Context,
Result Token, Result Exchange, Result, Question Backend Transport, and
`ImathasQuestionBackend` response/control/Student Response marker terms, and amends fresh
migration `2026090102` without aliases or compatibility support. It preserves the accepted
lifecycle, relationships, procedures, browser launch flow, security invariants, and tests.

`WP-SD1-A-TERM-01-SRF1` is accepted. This no-schema Student Response Format terminology correction
uses one domain-owned answer-free Student Response Format Check with thirteen exact Student Response
Format Issues and one strict shared browser decoder for Wasm JSON and the declared HTTP response.
The direct cutover retires the response-format report/violation wire names, `missingUploadReference`,
and the `violations` shape. Focused evidence, an independent audit with repaired findings, and complete
aggregate acceptance passed. Mounting the planned key-free server fallback route remains a separate
future server-boundary allocation.

[customer-spec.md](customer-spec.md) describes a
backend-agnostic assignment platform built around repeated attempts, algorithmic questions, and
question-level timing. The foundational M0 and M1 platform slices and the main M2 through M4
learning paths are implemented, while current release-track acceptance remains governed by the active
release plan. The Instructor roadmap's evidenced M0 release-truth exit is accepted; the sole global
current-package handoff is recorded in [implementation_status.md](implementation_status.md). Retention R4.4, QTI
profile import WP-QTI-12 is accepted. Historical course-appearance receipts WP-CA1 through
WP-CA7/WP-RC1 and WP-RC2 are retained as evidence, but the durable Course Appearance Store,
current-pointer/schema, PostgreSQL, route, authorization, upload-promotion/cleanup, and editor
feature is deferred and incomplete after the pre-production schema reset. The separate current
`WN1-QM-PRESENTATION-COURSE-APPEARANCE-VIEW` package is only the existing reader-boundary
terminology closure. The authoritative
remaining package sequence, binary scope ledger, owners, files, behavior, success conditions, and
validation are in
[release_completion_plan.md](active/release_completion_plan.md). The release plan owns release scope
and dependency order, while [implementation_status.md](implementation_status.md) owns the sole global
current-package handoff; this foundation plan does not duplicate that changing handoff.

**Historical 2026-08-14 WP-R2 acceptance.** WP-R2 recorded the original no-drift publication
cutover, exact hidden snapshot evidence, and explicit assignment replacement behavior. That receipt
remains evidence of the work completed at the time. The current stable-lineage and immutable-version
authority below supersedes its earlier one-Question-ID-per-content-snapshot interpretation while
preserving all exact Assignment, Issued Question, grading, audit, provenance, and transport pins. Final
material-tree evidence is recorded in [implementation_status.md](implementation_status.md) and the
changelog. WP-PY-L1 is accepted on 2026-08-15. The Instructor roadmap records M0 as accepted from
the four evidenced release-truth packages; its subsequent work follows the Instructor dependency
queue.

Current browser evidence is tracked through the sole current-package handoff in
[implementation_status.md](implementation_status.md). Production email and canonical onboarding
remain separate release work.

**Current live-demo capability.** [WP-INST-LD3](active/live_delivery_convergence_plan.md) established
ordinary live assignments, Assignment Attempts, deterministic server-owned grading, immutable issued
evidence, receipt replay, and audited Instructor inspection as the canonical product path.
WP-INST-T5 is accepted after extending that model with visible item-pool authoring, policy-correct
automatic variation, executable no-store preview, and ordinary Student delivery. WP-INST-D1 and
WP-INST-D2, WP-INST-B1, and WP-INST-B2 are accepted. Reusable curricula now advance into ordinary
teaching courses through explicit adoption, rollover, term shifting, provenance, and controlled
updates. WP-INST-T6 is accepted: each assignment has a linked home, separate Questions and Policies
pages, and a live answer-free Student view. WP-INST-G1 persists one immutable accepted student
input before grading and adds assignment-local exception recovery,
bounded retry, generation-fenced recalculation, and immutable receipts. `WP-INST-WN1` is the current
repository-wide corrective prerequisite under the [wire naming contract migration plan](active/wire_naming_contract_migration_plan.md).
Current pre-WN1 transport remains lower camel where source still does so; the approved direct
Serde-to-TypeScript snake data-object boundary lands through WN1-A/B/C1-C6/QM/WA/D/F before G2 resumes.
`WP-INST-G2` is implemented and acceptance-open behind WN1 and its remaining visual/documentation close-out.
It establishes a roster-first, server-calculated Gradebook and an explicit atomic-audit inspection
read that carries the Student Response with permitted correctness and score, and is no-store. The shared status registry owns the
current handoff and migration allocation; the Instructor capability plan retains the accepted
capability contracts and evidence boundaries.

**BlueprintCourse/CourseInstance binding (2026-08-29).** The accepted historical B1/B2 behavior is
re-based for the SD1 cutover as one canonical reusable source model. `BlueprintCourse` is one
answer-free, ordered, revisioned course-level aggregate: a vetted Instructor-visible published
projection contains ordered modules and assignments, and every entry pins an exact published
`QuestionRevisionReference`. Draft source remains owner/workspace-collaborator scoped. `CourseInstance` is
the separate exact-`CourseId` teaching aggregate created from that source; it owns copied assignment
definitions, Students, releases, live deadlines, accommodations, grades, and delivery settings.
It has exactly one immutable Blueprint parent and applied source revision. A blank CourseInstance is
created from a minimal Blueprint, not from a second course type. Relative calendar-day and local-wall-
clock intent is resolved against the destination term and IANA zone; it never becomes a live
Blueprint deadline. New upstream assignments propagate to daughters as unreleased, while explicit
release and local delivery edits remain CourseInstance decisions. Archived referenced Blueprints stay
resolvable for history and evidence.

`CourseStore::create_course_impl`, composed as `Store::create_course`, owns the canonical
Course Creation operation. Its SD1 protected database operation atomically obtains or creates the
normal minimal BlueprintCourse revision,
creates the non-null bound CourseInstance, and adds the first Instructor membership. This capability
remains outside the closed six-operation Blueprint operation boundary: Fork Blueprint Course, Create
Course from Blueprint, Copy Assignment from Blueprint, Apply Blueprint Update, Copy Course for New
Term, and Shift Course Dates.

This is a clean pre-production cutover, not a compatibility layer. The SD1 sequence removes Alpha
types, routes, schema branches, Store capabilities, generated aliases, and browser resource kinds;
`BlueprintCourseReference` (`BP-*`) is the only reusable-course locator. Existing `WP-INST-B1` and
`WP-INST-B2` acceptance records remain historical evidence; the current handoff and migration
allocation remain solely in [implementation_status.md](implementation_status.md). `WP-R0`, `WP-R1`,
`WP-R2`, and `WP-PY-L1` retain their registered package identities and acceptance status.

**Current authority for the SD1 cutover.** A resolved authenticated session establishes one global
account and its immutable role. Each protected operation receives the exact system, Question Library,
workspace, course, course-membership, Student-ownership, or short-lived capability identity it
authorizes. Published QuestionIds are
stable lineages with immutable QuestionRevisions: moderate steward edits preserve original authorship
and license in the lineage, while full forks give their author a private draft and, after validation,
a separately attributed and source-compatible licensed lineage. `QuestionChangeProposal` is the
lightweight middle path: it pins an exact base version, carries a proposed patch and rationale,
passes automated validation, and lets the lineage steward accept or reject it. An accepted proposal
creates an immutable same-lineage version with contributor credit; a stale base is rebased and
resubmitted. Proposals reuse improvement threads through a focused accept/reject workflow.
Assignments and evidence pin exact versions and receive only explicit controlled
updates. A Sysadmin-approved `ForcedQuestionCorrection` is the separate critical-flaw path with a
validated replacement, deterministic remediation, privacy-safe impact, and immutable superseding
receipts. Automated grading is the sole score authority; exception handling, inspection, and
recalculation are audited operations rather than manual grading. This authority governs SD1 work
without advancing the current package status.

**SD1-B1 compile-coordinated session cutover (2026-08-29).** The global-session end state is one
server-owned `SessionId` in a resolved `SessionRecord`, and one `SessionStore`. The resolved record
establishes an account and its immutable role after the store verifies the opaque credential; it
does not grant course, workspace, Student, or capability access. Exact domain relationships
authorize protected work. Store contracts carry the exact `AccountId`, `CourseMembership`, `Student`,
workspace relation, or worker lease that their operation needs, with `SessionId` retained only for
session and audit facts.

This conversion uses compile-coordinated, atomic package outcomes rather than a fabricated
installation-wide scope or a lasting dual session API:

1. `SD1-B1-P0` owns the `learning-data-access::session` public type boundary. `SessionId` is
   server-only and belongs to a resolved `SessionRecord`; it introduces no grant. The accepted P0
   receipt proves only this type boundary; it does not claim B1 acceptance or route integration.
   Its narrow evidence is
   `cargo check -p learning-data-access --no-default-features`.
2. `WP-SD1-B1-P1`, after `SD1-C2` and before `SD1-D1`, persists `SessionId` on `SessionRecord` and
   exposes the account and session facts required by a Store operation. It enables exact-identity
   transactions without converting routes or removing legacy session models. `SD1-D1` depends on
   `SD1-C7` and P1. `SD1-B1-F` follows accepted D1--D6 and performs the single route/model cutover:
   it removes `SessionSubject`, `AccountSession*`, and obsolete global-scope auth/session seams. `SD1-B5`
   follows B1-F and regenerates browser-safe contracts.
3. `SD1-B2` through `SD1-B4` own course/Student, Question Library/Question authoring workspace, and job/object/iMathAS Question Backend
   exact-scope contract definitions respectively. Each names the exact account and durable resource
   input it requires, but does not claim existing routes have switched authentication. C/D provide the
   matching Store/RLS/service implementation before final route integration. Their package receipts
   record the owning focused contract lane.
4. `SD1-C` and `SD1-D` own the fresh PostgreSQL schema, Store/RLS, and direct protected-service
   implementation for those scopes. They prove their own migration, restricted-role, and service
   behavior against the disposable stack before the authentication integration. No global scope
   replacement is introduced.
5. `SD1-B1-F` owns session record/store, account-identity session types, Memory/PostgreSQL adapters,
   server authentication, direct composition callers, and retirement of the prior global-scope auth
   route seam. `SessionRecord` owns `SessionId` plus global `AccountId`; `SessionSubject` and every
   `AccountSession*` durable-session type/trait are removed; the server passes the resolved record's
   account/session facts only where required; and all affected consumers use their B2-B4 exact-scope
   path. This is the B1
   acceptance boundary. Evidence combines focused learning-data-access/server checks with the
   relevant C/D protected-route, Store, and RLS lanes.
6. `SD1-B5` owns generated auth/browser contracts and direct decoders. The browser-safe account
   projection reflects the completed auth boundary and contains no session credential, generic identity context, or
   obsolete global-scope compatibility field. Its evidence is the B5 generated-contract and TypeScript
   compilation lanes.

`SD1-B1-P0` is accepted preparatory progress, not a partial replacement for the singular session
model. `SD1-B2` through `SD1-B4` establish the typed scope inputs; C/D implement and prove those
inputs; `SD1-B1-F` then removes the old subject and duplicate account-session model in one
compile-coordinated integration. B5 follows the final server boundary, and later E/F/G work builds
on it. This order preserves a coherent end state without a global-scope fallback or a compatibility
alias.

## Grounded evidence rule for open packages

Open-package acceptance starts with a binding contract that names owned modules, routes, migrations
when persistence changes, dependency order, and an evidence-class table. Permanent tests retain
deterministic, offline behavior that could plausibly regress. Disposable PostgreSQL, browser,
multi-replica, and controlled-clock exercises retain connected behavior. Source inventories,
rendered captures, capacity measurements, and query-plan inspections are one-time implementation or
release evidence when they support a documented decision. Final Validation is the `./all_test.sh`
gate on the final material tree after each package's local lanes are green.

For the BlueprintCourse cutover, evidence has five non-substitutable lanes: permanent offline
behavior tests; recurring PostgreSQL/RLS and service acceptance; production-browser behavior against
the real stack; human semantic visual acceptance; and one-time Graphify/source/schema/generated
inventories, rendered-capture publication, cleanup, and migration receipts. A green browser mock,
source inventory, screenshot, or visual review cannot substitute for another lane. Required unrun or
skipped lanes keep the package incomplete under [TEST_EVIDENCE_MODEL.md](../TEST_EVIDENCE_MODEL.md).

Role-specific visual coverage follows Human Guidance: Instructor and Sysadmin evidence uses only the
canonical 1280 by 800 desktop profile; Student evidence may use the maintained laptop, portrait,
narrow-phone, and square profiles. Each applicable profile is reviewed for semantic usability,
focus, contrast, recovery, and readable hierarchy; exact pixels, viewport shares, screenshot hashes,
and source_object_reference totals have no acceptance authority.
Server response tests cover declared success, validation, conflict, denial, and pagination categories
rather than enumerating individual response instances. Teaching-policy tests choose representative
precedence partitions, including deny, grant, override, conflict, and no-op paths.

Capacity, retention, signed-link expiry, and theme support use documented configuration models: a
deployment declares workload shape, retained-record horizon, object and payload limits, expiry
duration, supported appearance variants, and scaling policy. Tests exercise controlled-clock expiry
and configured boundary behavior; one-time load, route-inventory, source-inventory, and rendered
variant reviews record the model and observations without becoming permanent count gates.

The completed local-stack lifecycle controller foundation is recorded in
`docs/active_plans/workstreams/local_stack_controller_implementation.md`.
It adds one typed Python lifecycle layer around the Compose stack for developers,
Codex, aggregate browser acceptance, and canonical disposable walkthrough ownership. The shared layer
centralizes provider/env/project resolution, label discovery, preflight/status, and scoped cleanup;
focused Python modules are the current build, bootstrap, migration, seed, renderer, and readiness owner.
That direct Python lifecycle conversion is accepted: WP-PY-L1 replaces the
launcher, `_restart.sh`, and `local_identity_bootstrap.sh` together after WP-R2 and before M1. No Python
wrapper or dual launcher is an accepted intermediate state. Final offline and live Validation are green,
as are the three named independent final reviews. The Instructor roadmap's M0 evidence is accepted;
its subsequent work follows the Instructor dependency queue, while the release plan retains its
later acceptance gates.

**WP-R2 test and live-evidence boundary.** Offline Memory publication, replacement, and replay behavior
requires a current registered conformance test target. The former disconnected
`crates/learning-data-access/tests/conformance/` corpus is retired; server
Question-ID request and replacement behavior belongs in `crates/server/src/catalog/tests/publication.rs`
and `crates/server/src/course/tests/assignment_revision.rs`. The disposable PostgreSQL/RLS driver is
`tests/e2e/e2e_wp_r2_postgres_rls.py`; `crates/project-tools/src/e2e_seed/tests.rs` owns manufactured
manifest convergence. The canonical `webwork_delivery` and
`assignment_question_replacement` scenarios carry the retained browser behavior; the browser-free
WebWork and replica restart commands retain their distinct service claims; fixed seed/manifest and
Rust tests retain Chapter One publication semantics. `tests/test_assignment_editor_ui.mjs` owns
narrow decoder/client/model behavior. The canonical `assignment_question_replacement` and
`instructor_authoring` scenarios own visible assignment behavior.
`tests/e2e/e2e_browser_suite_owner.py`, dispatched by `local_stack_control/acceptance_lanes.py`, owns
the one fixed live browser route and canonical composition scenarios. WP-R2 uses inline builders and
adds no fixture directory. Generated `generated/api/` output is
ignored derivative output from `crates/project-tools/src/tsgen.rs`; authored consumers own their
decoders and behavior. Durable M0 package evidence is recorded in
[implementation_status.md](implementation_status.md) and [CHANGELOG.md](../CHANGELOG.md); one-time
migration/schema/source/route/generated inventories, screenshots, and timing observations are
historical evidence only and are not referenced through an ignored scratch source_object_reference.

**Local Question Renderer Version.** WP-R1 is accepted on 2026-08-14. Its completed Chapter One
pilot/browser and aggregate-acceptance Python work uses one designated configured renderer image name
as the stable local selection and rebuild target. Each live run records the inspected immutable OCI
image configuration ID as exact runtime provenance. Image pruning may remove the selected local bytes,
after which the configured target is rebuilt before use. WP-R2 is accepted; the Instructor roadmap's
M0 evidence is accepted, and WP-PY-L1 is accepted on 2026-08-15 after final offline/live Validation
and its named independent final reviews. The release plan retains its later acceptance gates.

ADAPT (`OTHER_REPOS/adapt/`) is the surface model and the source of the sharpest lessons, because its
weaknesses are visible in its own schema. Three review passes (`reviewer_commments.md`,
`reviewer_commments_2.md`, `reviewer_commments_3.md`) plus the owner's operating experience moved this
design. Six requirements shape it:

- **Answer-bearing content is a separate security class.** Answers, keys, and grading logic stay on
  the server, so grading is a server round trip.
- **Object storage is a core subsystem**, not an afterthought.
- **Identity must separate drafts from published Questions.** ADAPT mints a durable official ID for
  every saved problem, so the owner's sandbox holds abandoned experiments carrying permanent Question Library
  numbers.
- **The sharing boundary is educational content versus educational records.** Assignments are course
  artifacts, not shareable content.
- **Demand is met by adding replicas**, making in-process state and process clocks design errors.
- **Completion is not the end of activity.** The owner reports students voluntarily running a
  finished assignment 30 or more times to learn through algorithmic variation. This is the single
  largest change: Peptidyle is a high-volume attempt-event system rather than an assignment
  submission system.

The intended outcome is ADAPT's surface and its best feature -- one published Question reusable by
thousands of instructors without copying -- without its three structural weaknesses: unbounded
payloads in operational tables, no content integrity, and identity granted before publication.

This plan records the platform architecture and accepted milestone history. Remaining implementation
uses dependency-ordered production packages with explicit owners and behavior gates; scaffolding or
mock-only wiring is never completion evidence.

## Decisions

The architecture decisions below remain authoritative. The in-scope and out-of-scope ledgers in
`docs/active_plans/active/release_completion_plan.md` record the release boundary. No implementer is
expected to
choose a product default, storage rule, source format, authentication method, deployment tool, or
release boundary while writing code.

## Objectives

- Deliver a mastery loop whose perceived latency is dominated by local work: answer-format validation
  and the next question are already in the browser, and grading is a server round trip whose
  server-side processing time is measured and recorded as a baseline.
- Support unlimited post-completion practice Assignment Attempts as a first-class product behavior, with completion,
  grading, and variation as three independent policies.
- Guarantee no Answer Key or Question Grader code is reachable from the browser, enforced by the
  crate dependency graph rather than reviewer discipline.
- Make every historical attempt reproducible from seed, generator version, and problem version, at a
  per-row cost small enough to survive hundreds of millions of rows.
- Separate draft identity from published identity so an abandoned experiment never occupies a durable
  Question Library numbers.
- Keep published content shared and immutable while every educational record carries exact course,
  Student, workspace, or AccountId ownership and is protected by database-enforced row-level security.
- Delete Student records on a privacy-by-default schedule with the configured course lifecycle policy,
  while anonymous question statistics survive so the library keeps improving.
- Keep binary and archival content out of PostgreSQL, with every Source Object Reference carrying its Source Object Checksum, size,
  media type, Question License, and exact owning relationship.
- Keep API containers stateless so demand is met by adding replicas.
- Freeze module contracts early enough that at least six lanes proceed in parallel without
  coordinating mid-flight.
- Land first-party algorithmic, WeBWorK, and iMathAS adapters, plus DOCX and PDF exam export.
- Demonstrate the local teaching loop end to end: instructor course/roster/assignment setup followed
  by student keyboard take, scoring, repeat practice, and instructor gradebook confirmation.

## Scope

- Preserve and close the accepted M0 through M4 architecture and behavior.
- Complete WP-RC1 through WP-RC12 from the release-completion plan in dependency order.
- Complete M8 through M11 from the corrected instructor-to-student walkthrough plan without an email
  or canonical-onboarding dependency.
- Treat all repository-owned Rust, TypeScript, SQL, container, OpenTofu, test, and documentation
  artifacts as in scope for the working-codebase release.
- Treat externally supplied institutional credentials, legal certification, and named human-pilot participation as
  production-activation evidence with explicit external owners, not unfinished code.
- Require a real implementation, behavior gate, and independent review for every claimed capability.

## Design philosophy

Three organizing trade-offs.

**Secrecy over local speed.** Answers stay on the server, so responsiveness comes from moving
_non-secret_ work to the browser and hiding the round trip behind prefetch. Native H5P shows why this
matters: it ships answer evaluation to the browser, so any H5P question is inspectable by any student.
That is a property of the format, and it sets the adapter's honest capability declaration.

**Contracts before code.** Freezing interfaces in one milestone costs a serial stage and buys wide
parallelism afterward. It only works if the contracts are complete, so the contract-freeze milestone
ships executable reference implementations and conformance suites, not just type definitions.

**Single-installation account scoping.** The product boundary is one installation with global accounts,
shared published content, private authoring workspaces, and exact course/Student educational records.
Its implementation is one PostgreSQL cluster with server-derived `AuthenticatedSession { account_id, session_id }`,
operation-specific ownership predicates, and forced row-level security. A missing authenticated Account, foreign course,
another AccountId, revoked membership, or absent workspace relationship returns no protected rows. One
cluster means one connection pool, one migration run, and one backup policy; typed course, workspace,
Question Library, and system scopes preserve future adaptability without a multi-institution boundary.

Cited from [REPO_STYLE.md](../REPO_STYLE.md):

- **Fix the design, not the symptom.** Grading lives in a crate the WASM build cannot depend on, so
  shipping a key to the browser is a compile error. Account, membership, and ownership isolation is a
  database policy, not a code-review habit.
- **Design for adaptability.** Every engine enters through one adapter trait publishing capabilities.
  Physical storage hides behind an object service. Question Library search hides behind a repository so a
  dedicated search service can replace PostgreSQL full-text without touching callers.
- **Atomic task decomposition.** The module catalog gives every module one owner, one contract, one
  independent verification.
- **Long-term over short-term.** Hidden immutable publication snapshots, random checked Question
  IDs, exact course/Student/workspace ownership, and cursor pagination are foundational because all
  four are painful to retrofit. The snapshots preserve grading; instructors still work with one
  current Question rather than a Question Revision list.
- **Perfect is the enemy of good.** No Kubernetes, Redis, Kafka, sharding, dedicated search index, or
  microservice fleet. M0 through M4 run on `podman compose` with MinIO.

Evidence strategy for uncertain methods:

- Cross-target determinism is settled by measurement in WP-C4: a committed seed table, hashed
  outputs, the same assertions under `cargo test` and `wasm-bindgen-test`. If parity fails the
  primitive is replaced before any dependent lane starts.
- The secret-free WASM claim is settled by WP-C5: an export allowlist plus a dependency-graph
  assertion. "We were careful" is not evidence.
- Authorization isolation is settled by tests for a missing authenticated session, foreign course, another
  AccountId, and revoked membership returning zero rows, run in `tests/e2e/` on every gate.
- Performance gates assert correctness absolutely and speed relatively. A first run establishes a
  recorded baseline; later gates compare against that baseline rather than against a number chosen in
  advance. Grading latency is split into server-side processing time, which this project controls and
  can hold to a regression budget, and network round trip, which varies with the student's connection
  and is reported for context rather than gated.
- Exactness is required in one place and one place only: seeded generation must produce identical
  output on both targets, because the render cache and the reproducibility record are keyed on that
  equality. Everywhere else, tolerances and baselines are the right instrument.

## Detailed platform scope

- Create the Cargo workspace, Solid toolchain, container set, and object storage subsystem.
- Freeze every module contract with executable in-memory reference implementations and conformance
  suites before production backends consume it.
- Implement the question model, identity and lifecycle, attempt state machine, timing, scoring,
  capability validation, and audit events in Rust.
- Implement the Student Record, Assignment Attempt, and Question Attempt model with independent completion, grading, and
  variation policies, plus transactionally maintained summary rows.
- Implement the Question Library, private Question authoring workspaces, and exact course/Student records in one
  cluster with forced RLS.
- Implement the object store with immutable keys, checksums, and three-bucket separation.
- Implement the first-party algorithmic adapter, then WeBWorK, QTI, H5P, and iMathAS.
- Build the Solid student assignment interface and instructor assignment editor.
- Implement DOCX and PDF export, and the worker pool that produces them and drains render work.
- Implement LTI Advantage grade passback.
- Document architecture, contracts, question model, identity, storage, database authorization, and
  determinism.

## Non-goals

Phrased as the behavior to follow, per the **prompt positively** principle in
[REPO_STYLE.md](../REPO_STYLE.md). Each bullet
names what this plan does instead of the excluded alternative, so a subagent reading it acts on the
instruction directly.

- Serve native H5P as ungraded practice with `serverGrading: false`, and import supported types into
  the server-graded internal representation when grading is required.
- Keep the infrastructure to containers, one PostgreSQL cluster, object storage, and a worker pool.
  Kubernetes, an in-memory cache tier, a streaming bus, sharding, a dedicated search index, and
  multi-region deployment each have a documented threshold in the scale evaluation and arrive when
  measurement calls for them.
- Focus the product on assignments, problems, attempts, and grades. Discussions, clickers, LMS roster
  sync, external research exports, and generated question content stay outside this plan.
- Schedule Adaptive Question Support as the first post-M6 candidate.
- Consume the WeBWorK renderer over HTTP as a separate service, using it as shipped.
- Store binary and archival content in object storage at every size.
- Derive every storage key from stable IDs and versions.
- Serve every asset through a stored object record.
- Read grades from `student_assignment_summary`.
- Paginate every list endpoint with a cursor.
- Model assignments as exact-course artifacts that select explicitly chosen immutable published
  Question IDs while retaining the exact hidden publication snapshot needed for grading reproducibility.
- Use executable in-memory reference backends for fast tests and PostgreSQL for every environment
  holding durable student records.

## Current state summary

- The Rust workspace, Solid browser application, generated contract pipeline, PostgreSQL schema,
  object store, API, worker, local container stack, and repository gates are implemented.
- M0/M1 contracts, core author/publish/assign/Assignment Attempt/grade/feedback/export/statistics paths, retention
  R4.4, QTI profile import WP-QTI-12, the database baseline, keyboard pass, local typed lifecycle, and course
  appearance WP-CA1 through WP-CA7/WP-RC1 have accepted evidence.
- Focused private `local_stack_control` Python modules now build, migrate, seed, start, wait for semantic
  health, and open the local application. Its default profile includes the private standalone
  `webwork-pg-renderer`, but does not run WebWork2 or MariaDB. WP-PY-L1 is accepted on 2026-08-15 after
  final offline/live Validation and its named independent final reviews.
- The first forward migration after the accepted six-file baseline is
  `schemas/migrations/2026080907_course_appearance.sql`; later filenames and owners are reserved in
  the release-completion plan.
- The remaining work is not an architecture discovery exercise. WP-RC2 through WP-RC12 name every
  production source_object_reference, behavior, validation command, and release boundary.

### What ADAPT actually does with content

Established from migrations and controllers, because `reviewer_commments_2.md` asked for evidence
rather than characterization. The answer is **type-based hybrid storage with no size threshold**:

| Question asked                        | Finding                                                                      | Evidence                                                                                                 |
| ------------------------------------- | ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Binary in MySQL?                      | No. Zero `blob`, `binary`, or `mediumBlob` columns in any migration          | `OTHER_REPOS/adapt/database/migrations/`                                                                 |
| Large XML or JSON moved out of MySQL? | No. QTI XML is a `text` column; parsed `qti_json` stays on the questions row | `2022_05_06_150939_create_qti_imports.php:21`, `2023_02_03_115902_update_qti_json_type_to_questions.php` |
| Small images also in S3?              | Yes. All media goes to S3 by type; no threshold exists                       | `2024_06_03_173537_create_question_media_uploads_table.php`                                              |
| Configured size threshold anywhere?   | None found                                                                   | upload and import controllers                                                                            |
| Original imported package preserved?  | No evidence. Only parsed XML plus `directory` and `filename`                 | `qti_imports` schema                                                                                     |
| H5P packages stored?                  | No. Referenced by remote `technology_id`                                     | `OTHER_REPOS/adapt/app/Question.php`                                                                     |

Three weaknesses neither review named, each becoming a requirement here:

- **No checksum column** on `question_media_uploads` (`id`, `question_id`, `original_filename`,
  `size`, `s3_key`, `transcript`, `status`). No checksum means no deduplication, no corruption
  detection, and no way to prove a historical attempt saw a given image.
- **Keys are random, not content-addressed, and filenames participate in identity.**
  `$s3_key = md5(uniqid('', true)) . '.html'`
  (`OTHER_REPOS/adapt/app/Http/Controllers/QuestionMediaController.php:242`), and `qti_imports`
  uniquely indexes `(account_id, directory, filename)`.
- **Signed URLs live seven days.** `temporaryUrl(..., Carbon::now()->addDays(7))`
  (`QuestionMediaController.php:279`). A leaked URL grants a week of access, which for a
  Student Record Object is a FERPA-relevant exposure. This plan uses minutes.

## Resolved decisions

| Decision                     | Choice                                                                                                                                          | Reason                                                                                                                                                                                                                  |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Server runtime               | Native Rust `axum`; shared crates also built for `wasm32`                                                                                       | Owner-selected. Native is fastest and keeps direct database access                                                                                                                                                      |
| Web server                   | No Apache, nginx, or lighttpd in the request path                                                                                               | See the LAMP mapping below; all reviews agree the load balancer replaces Apache                                                                                                                                         |
| Database                     | **PostgreSQL on RDS**, one cluster                                                                                                              | Owner-selected. JSONB with indexing, forced row-level security, mature `FOR UPDATE SKIP LOCKED`                                                                                                                         |
| Authorization boundary       | **One installation: global AccountId accounts, AuthenticatedSession, exact course/Student/workspace ownership, forced RLS**                     | SD1 owner decision. Shared published content, private drafts, and course records use their actual relationships; a missing authenticated session, foreign course, another AccountId, and revoked membership fail closed |
| Grading location             | **Server only**                                                                                                                                 | Owner-selected. No answer, key, or grading code reaches the browser                                                                                                                                                     |
| H5P grading                  | Native H5P is ungraded practice; `serverGrading: false`                                                                                         | Owner's observation: H5P ships answer evaluation to the browser                                                                                                                                                         |
| WASM contents                | Parameter generation, answer-format validation, timer display, state transitions                                                                | Non-secret work only, enforced by the dependency graph                                                                                                                                                                  |
| Sharing boundary             | **Shared published content; private workspaces and exact course/Student records**                                                               | SD1 owner decision. Assignments may be reused as teaching structures while authorization and educational records remain bound to their exact course and Student relationships                                           |
| Student Work Records model   | `student_record` / `assignment_attempt` / `issued_question` / `question_attempt`                                                                | Owner-reported repeated-practice observation: completion is not terminal and practice continues through new Assignment Attempts with frozen Issued Questions                                                            |
| Grade computation            | Transactionally maintained summary rows; never scan attempt history                                                                             | The declared capacity model keeps grade pages on summaries as workload grows; one-time query review records the observed plan                                                                                           |
| Question identity            | One random checked `AAA-BBBB` Question ID; hidden UUIDs and snapshots remain internal                                                           | The Question ID is the only human-facing identity. It is non-sequential, copiable, names one immutable published question, and never carries a version suffix                                                           |
| Partitioning                 | Monthly range partitions on the four highest-volume append-only tables only                                                                     | Capacity-model candidate for the declared planning workload; a one-time workload/query review validates it and other tables remain unpartitioned until observed need                                                    |
| Pagination                   | Cursor only; `OFFSET` banned by lint and review                                                                                                 | Large `OFFSET` scans are unusable at Question Library and history scale                                                                                                                                                 |
| Content storage              | Split by role with a size backstop (below)                                                                                                      | Answers the owner's direct question                                                                                                                                                                                     |
| PLE Question JSON source     | Versioned PLE Question JSON, compiled into separate public and grader-only values                                                               | Keeps ordinary static authoring small and deterministic; QTI remains an import/export adapter instead of defining the internal model                                                                                    |
| Question Library table split | `question_revision` metadata separate from hash-partitioned `question_revision_payload`                                                         | Planning sizing observation favors a hot metadata projection and cold payload store; configured budgets and one-time query review decide when a partition or index change is needed                                     |
| Object storage               | S3 with four physical security domains; MinIO locally                                                                                           | `public-assets`, `private-content`, `student-records`, and `temp-processing` have distinct IAM/KMS, retention, and delivery policies                                                                                    |
| Asset delivery               | CloudFront immutable URLs for activated public Question Library assets; authorized POST-minted short-lived URLs for private/student records     | CDN handles non-record public bytes; a protected navigation cannot mint an access grant                                                                                                                                 |
| Rendered output              | Cached by `(question_id, revision_number, seed)` in `private-content`; no public renderer feature until externally managed renderer attestation | Rendering remains deterministic, but the renderer is outside the production baseline until its identity and isolation are independently accepted                                                                        |
| Session storage              | Opaque session ID cookie, session row in the database                                                                                           | Works across replicas and stays revocable                                                                                                                                                                               |
| Timer clock                  | Timestamps from PostgreSQL, never a process clock                                                                                               | Replica clock skew would otherwise change verdicts                                                                                                                                                                      |
| Background work              | `worker` container pool on a jobs table with `FOR UPDATE SKIP LOCKED`                                                                           | Import, export, and renderer work leave the request path                                                                                                                                                                |
| Autoscaling                  | Fargate target tracking: request count for `api`, queue depth for `worker`                                                                      | A class-start burst is a request spike; renderer load is a queue-depth signal                                                                                                                                           |
| Execution shape              | Contract freeze, then parallel module lanes                                                                                                     | Owner-requested modularization; see the module catalog                                                                                                                                                                  |
| Repo layout                  | Reduced monorepo: `src/`, `crates/`, `pipeline/`, `containers/`, `schemas/`                                                                     | No `apps/` or `packages/` split until a second app exists                                                                                                                                                               |

### Recorded disagreement with reviewer 1

`reviewer_commments.md` recommends a TypeScript API server with Rust as a called library. The owner
chose the native Rust server with the trade-offs visible. The reviewer's velocity argument is real;
the counter-arguments are that two languages in the request path means serialization at every domain
call, and that `customer-spec.md` itself requires the TypeScript layer not to contain grading rules.
Recorded rather than reopened; raise it if velocity becomes the binding constraint.

### Answering the storage question directly

**A split, but a narrow and principled one. Not "just metadata."** The rule is by _role_, because role
is stable, with size only as a backstop:

| Category                    | Home                                        | Contents                                                                                                                        |
| --------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Operational content         | PostgreSQL                                  | The compact normalized question model the renderer and grader execute; grading rules; policies; all metadata and references     |
| Answer-bearing content      | PostgreSQL, separate tables, separate grant | Answer keys and checker configuration, readable only by the grading role, never joined into a student-facing query              |
| Archival and binary content | Object storage, always, at any size         | Original QTI ZIP, images, audio, video, H5P packages, DOCX and PDF exports, large source bundles                                |
| Derived artifacts           | Object storage, separate prefix             | Rendered output, sanitized HTML, extracted resources, thumbnails, student-specific exports. Regenerable, so different retention |
| Temporary                   | Container disk, then discarded              | Archive extraction, conversion, scanning                                                                                        |

Backstop: normalized operational payloads remain subject to their accepted schema and contract
ceilings, including the 256 KiB private grading ceiling where defined. An oversized source/private
write refuses before mutation; archival source and binary bytes use typed object storage. Version 1
does not silently replace a hot-path normalized model with an object reference, and the shipped
ceilings are not an unresolved profiling decision.

Why not metadata-only: a normalized question model is kilobytes and is read on **every attempt**.
Pushing it to object storage adds a network hop to the hottest path for no benefit, the exception
`reviewer_commments_2.md` names. Why not ADAPT's approach: unbounded payloads in operational tables
with no threshold and no checksum, which is the bloat the owner is right to worry about.

The rule that makes the split safe: **every Source Object Reference carries identity metadata regardless of which
side it lands on** -- `object_id`, `sha256`, `size_bytes`, `media_type`, `question_id`, `revision_number`,
and its Question License. Text in PostgreSQL is checksummed exactly like a ZIP in S3.

### The modern LAMP equivalent

| LAMP letter | 1999                                                    | This project                                    |
| ----------- | ------------------------------------------------------- | ----------------------------------------------- |
| **L**inux   | Host OS, hand-configured                                | Container image, immutable                      |
| **A**pache  | HTTP server, mod_php process manager, static files, TLS | **Nothing.** The Rust binary is the HTTP server |
| **M**ySQL   | Same box as Apache                                      | PostgreSQL on RDS, plus S3 for files            |
| **P**HP     | Templates rendering HTML per request                    | TypeScript in the browser, Rust on the server   |

Apache is not obsolete, it is unemployed here. Its four historical jobs are gone or reassigned: there
is no CGI, so `axum` on `tokio` handles concurrency in-process with no interpreter pool to supervise;
`ServeDir` and CloudFront serve static files; the load balancer terminates TLS and rotates
certificates; routing is typed application code rather than a config file no test covers. Adding
Apache in front would buy a network hop, a second config surface, and a second thing to patch.

On the alternatives asked about: nginx is right when a reverse proxy is genuinely needed and the
operator knows it, but the ALB covers that role. lighttpd offers no 2026 advantage and a smaller
community. A Python server is wrong twice: `http.server` is not production, and gunicorn or uvicorn
would mean a Python backend, contradicting the Rust decision.

## Architecture boundaries and ownership

### Contributor-facing component names

Use plain capability names first. Cargo package and crate-directory names use
hyphens; Rust import and module names use underscores:

| Plain component name  | Physical path                                | Rust name              | Responsibility                                                                                                                |
| --------------------- | -------------------------------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Learning data access  | `crates/learning-data-access`                | `learning_data_access` | Typed persistence contracts, authorization-aware queries, PostgreSQL, migrations, and RLS                                     |
| In-memory data access | `crates/learning-data-access/src/in_memory*` | `in_memory`            | Database-free implementation used by conformance and server behavior tests                                                    |
| Project tools         | `crates/project-tools`                       | Cargo-only binary      | Repository-only TypeScript generation, fixtures, database operations, and pilot-content validation invoked with `cargo tools` |

```text
browser                     ALB          stateless replicas
+------------------------+           +----------------------------+
| Solid SPA (src/)       |  +-----+  | api x N (crates/server)    |
|  student/ instructor/  |->| ALB |->|   axum, native Rust        |
|  +------------------+  |  +-----+  |  +----------------------+  |
|  | domain.wasm      |  |           |  | domain + grading     |  |
|  | params, format   |  |           |  | authoritative        |  |
|  | validate, timer  |  |           |  +----------------------+  |
|  | NO answers       |  |           +----------------------------+
|  +------------------+  |             |            |         |
+------------------------+             v            v         v
        ^                       PostgreSQL      jobs queue   S3: four domains
        | immutable Question Library one cluster:     |      public-assets
   CloudFront (tag-gated)        shared content      v      private-content
        |                        + exact course  worker x N  student-records
   POST /api/assets/{id}         + forced RLS    exports,    temp-processing
   for protected delivery                        imports
                                                     |
                                                     +--> dedicated public-asset publisher
                                                     |    (only public-object writer)
                                                     v
                                             externally managed renderer
                                             (disabled until attested)
```

### The sharing boundary

The boundary is **educational content versus educational records**, which is also the FERPA line and
the reuse line. Those turn out to be the same line, which is why the design works. Both live in one
cluster; the distinction is ownership and policy, not physical separation.

| Shared installation content                      | Exact relationship-owned records (RLS enforced)                  |
| ------------------------------------------------ | ---------------------------------------------------------------- |
| Published Question Library                       | Courses and sections                                             |
| Immutable problem versions                       | Assignments                                                      |
| QTI, H5P, WeBWorK, and iMathAS source references | Instructor workspaces                                            |
| Shared media assets                              | Draft problems                                                   |
| Tags, Question Classifications, licensing        | Enrollments, Assignment Attempts, Question Attempts, submissions |
| Backend capability definitions                   | Grades, summaries, timers                                        |
| Anonymous question statistics                    | Per-student analytics and audit logs                             |
| Public and community libraries                   | Student-record artifacts                                         |

An assignment is **not** shareable content. It is a course source_object_reference referencing published Questions,
which is what lets one published Question serve thousands of instructors without copying:

```text
Published Question Revision (shared, immutable)
        |                         |
        v                         v
Assignment (Course A)     Assignment (Course B)
        |                         |
        v                         v
Assignment Attempts (A)   Assignment Attempts (B)
```

RLS is enforced, not advisory: every protected table declares `FORCE ROW LEVEL SECURITY`, the
application connects as a non-superuser role that cannot bypass it, and transaction-local authenticated Account
context comes from the authenticated session -- never from a client-supplied parameter. Operation
predicates enforce exact course membership, Student ownership, workspace relationship, or leased
capability. Tests in `tests/e2e/` cover a missing authenticated session, foreign course, another AccountId, and revoked
membership denial.

### Reusable source and teaching delivery boundary

The reusable-course model has one canonical source aggregate and one delivery aggregate:

```text
BlueprintCourse (workspace-owned, revisioned, answer-free)
  -> ordered BlueprintModule -> ordered BlueprintAssignment
  -> exact published QuestionRevisionReference pins and relative schedule intent

CourseInstance (exact CourseId, private teaching aggregate)
  -> copied definitions, resolved deadlines, releases, accommodations,
     Students, Assignment Attempts, grades, and delivery settings
```

Every CourseInstance has exactly one immutable Blueprint parent and applied source revision. Blank
course creation uses a minimal Blueprint. A BlueprintCourse revision is the complete ordered tree;
published revisions are visible and reusable by every vetted Instructor, while a Draft Blueprint
Revision remains scoped to its Blueprint Course Owner and exact Blueprint Collaborators. A BlueprintCourse has no Students, enrollments, releases, live
deadlines, accommodations, grades, or activity. Relative schedule intent is resolved against the
destination CourseInstance term and IANA zone and becomes CourseInstance-owned state. New upstream
assignments arrive in daughter instances as unreleased; release and divergent delivery edits require
an explicit CourseInstance action. Archived referenced Blueprints remain resolvable for provenance
and history.

The SD1 cutover is source-, schema-, API-, and browser-wide. It retains only `BlueprintCourseReference`
(`BP-*`) and one exact Store/route/decoder/editor boundary; it removes Alpha types, route surfaces, schema
branches, capabilities, aliases, and browser resource kinds. One-assignment reuse is a bounded
module/assignment projection of the same BlueprintCourse. Fork Blueprint Course, Copy Assignment from Blueprint, and
Create Course from Blueprint are distinct operations selected by destination, with no live source tether.

Question stewardship remains a shared dependency of both aggregates. Stable human-facing
`QuestionId` lineage carries immutable `QuestionRevision` history; every BlueprintCourse and
CourseInstance entry pins an exact version. Presentation/metadata, student-content, and grading-
semantic changes use the closed semantic classes: the last class is a major change and mints a new
QuestionId. Forks show lineage and remain creator-private drafts until complete publication
validation. Available Question Revisions are ordinarily selectable; Archived Question Revisions remain
discoverable and resolvable for evidence and existing pins. One AccountId-owned Star provides a public
bookmark and endorsement; vetted Instructors may see aggregate Star count and Star identities, while Students and
anonymous users see neither identities nor private Watch state. Durable improvement events remain
separate from publication authority. Grading corrections record affected
pins and Assignment Attempt impact, then use generation-fenced recalculation without mutating issued
evidence. Attempts, correct counts, and eligible-choice counts are version-specific and disclosed
only under the existing privacy-threshold formula. Existing `WP-INST-D1`, `D2`, `G1`, `G2`, `G3`, and
`G4` packages own these behaviors; SD1 introduces no package ID.

The emergency QuestionRevision path is a Sysadmin-approved `ForcedQuestionCorrection`: a validated
replacement and closed, FERPA-safe impact manifest precede one atomic authoritative replacement
mapping and generation. New selection and issuance resolve to the replacement immediately. Bounded
idempotent generation-fenced workers materialize affected BlueprintCourse/CourseInstance/assignment/
pool/future-issuance references and deterministic remediation without an unbounded cross-course
transaction. The flawed version and issued/graded evidence remain immutable; in-progress items are
reissued or excused and completed work receives the manifest's deterministic remediation, with
superseding recalculation receipts. Course Instructors receive audited impact and actionable results;
Sysadmin receives privacy-safe aggregate impact only. D1 owns availability/impact, G1
compatibility/recalculation, G2 audited inspection, and G5 action routing. Permanent, PostgreSQL/RLS,
and production-browser lanes prove the behavior independently.

Object-storage domains are physically separated so IAM, encryption keys, retention, and delivery
policy are enforceable rather than conventions:

| Domain            | Contents                                                          | Delivery                                                               | Authority and retention                                                        |
| ----------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `public-assets`   | Published Question Library assets only                            | CloudFront only, immutable keys, exact public tag required             | Dedicated publisher is the only writer; published tags/bytes cannot be mutated |
| `private-content` | Source packages, restricted course assets, cached/private renders | Authorized application path only; no CDN origin                        | API/workers use least privilege; immutable published source/version records    |
| `student-records` | Student-specific exports, uploads, annotated exams                | Authorized `POST /api/assets/{id}` then short-lived URL, always logged | Explicit expiration and deletion; five-minute signed URLs                      |
| `temp-processing` | Extraction, conversion, and inspection workspaces                 | Never served or signable                                               | Isolated worker-only lifecycle in days                                         |

Publication first stores candidate bytes and a pending registry in `private-content` in the same
Question Library transaction that enqueues `PublishPublicAssets`. A dedicated publisher database login and
service re-resolve the pending record, verify exact source bytes and checksum, write the final tagged
immutable object to `public-assets`, then lease-conditionally activate the registry. A crash before
activation leaves the asset unavailable rather than public or partially committed.

Crates and their forbidden dependencies:

This table records the current designed dependencies rather than an exhaustive permanent allowlist;
composition boundaries are permanent only where the architecture requires them, with the exact
`crates/wasm` dependency closure retained as a security gate that keeps answer-bearing grading outside browser builds.

| Crate                                   | Owns                                                                                                                          | Depends only on                                  |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| `crates/question_model`                 | Question types, capabilities, identity, Question Classifications, `ts-rs` derives                                             | External crates                                  |
| `crates/domain`                         | Assignment Attempt state machine and completion rules, timing verdict, seeded generation, capability validation, audit events | `question_model`                                 |
| `crates/grading`                        | **Answer keys, checkers, correctness decisions**                                                                              | `question_model`, `domain`                       |
| `crates/objects`                        | `ObjectStore` trait, S3 and MinIO backends, key construction, checksums                                                       | `question_model`                                 |
| `crates/learning-data-access`           | `Store` trait, PostgreSQL backends, migrations, RLS context management                                                        | `question_model`, `domain`, `objects`            |
| `crates/adapters/{ple,webwork,qti,h5p}` | Per-engine load, generate, grade delegation, capability declaration                                                           | `question_model`, `domain`, `grading`, `objects` |
| `crates/export`                         | Print model, DOCX and PDF writers                                                                                             | `question_model`, `objects`                      |
| `crates/wasm`                           | `wasm-bindgen` bridge, delegating every call to `domain`                                                                      | `question_model`, `domain`                       |
| `crates/server`                         | axum routes, auth, worker mode, composition root                                                                              | Every crate above                                |

`learning-data-access` remains the sole owner of Base Course SQL, PostgreSQL locking, durable
install-state transitions, migrations, and Store implementation. `project-tools` calls the focused
product crate directly as a CLI adapter; neither it nor the installer crate belongs in the server
composition root.

Two load-bearing properties follow from that table. `crates/domain` reaches only
`question_model`, so it has no clock and no database, which lets it run in a browser and makes the
seed-parity test meaningful; time and storage arrive as parameters. `crates/wasm` reaches only
`question_model` and `domain`, so the answer-bearing surface in `crates/grading` sits outside its
dependency closure and shipping an answer to the browser becomes a compile-time impossibility rather
than a code-review question.

## Student Work Records

The owner's observation that Students voluntarily make 30 or more Assignment Attempts on a completed
Assignment for learning is the largest single change to this plan. Completion is not terminal, and the
earlier model -- Assignment to Question Attempts -- could not express it.

Five ownership levels, per reviewer 3:

| Entity                | Holds                                                                                                                            | Cardinality                                                  |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `assignment_attempt`  | Direct Student Record and Assignment parents, attempt number, started and completed times, score, activity, and variation policy | Many per Student Record and Assignment; 30 or more is normal |
| `issued_question`     | Source Assignment Entry, exact Question Revision, issued order, scoring treatment, and selection evidence                        | One per delivered Question within an Assignment Attempt      |
| `question_attempt`    | Issued Question ID, seed, parameter hash, timing, operational state, and exact Question Attempt Reproduction Details             | Several per Issued Question                                  |
| `question_submission` | One accepted Student Response for its exact Question Attempt                                                                     | At most one per Question Attempt                             |
| `grading_result`      | One authoritative evaluation for its exact Question Submission and automated grading operation                                   | At most one per Question Submission                          |

Four policies, deliberately independent so an instructor can combine them freely:

| Policy                   | Options                                                       |
| ------------------------ | ------------------------------------------------------------- |
| Completion requirement   | First time all required questions are correct                 |
| Grade policy             | First, latest, highest, or instructor-defined                 |
| Continued practice       | Unlimited new Assignment Attempts after completion, or capped |
| Question Pool Reuse Rule | Reuse Selection or Select Again                               |
| Question Variation Rule  | Reuse Variation or New Variation                              |

Two derivations that look contradictory and are not, so the distinction is stated once here:
**Completion within an Assignment Attempt is derived** from Question states, never stored as a boolean, which keeps the
state machine honest. **Grade state across Assignment Attempts is a maintained summary row**, updated transactionally
when an Assignment Attempt changes, because scanning hundreds of millions of attempts to render a course page is not
an option. Different scopes, different mechanisms.

`student_assignment_summary` holds `best_score`, `latest_score`, `completed_assignment_attempt_count`,
`total_question_attempts`, and `last_activity_at`. Ordinary course and gradebook pages read only the
summary. Historical Assignment Attempts stay available for learning analysis through explicitly asynchronous
analytics, never through a synchronous page query.

## Data retention and deletion

ADAPT's retention practice -- notify the instructor 30 days after a course ends, automatically reset
at 100 days -- is privacy by default rather than privacy by policy, and it is worth adopting rather
than reinventing. It also validates the content-versus-records boundary: deleting every student record
in a course destroys no reusable content, because assignments reference shared problem versions
instead of owning copies. The boundary was drawn for sharing and turns out to be the same boundary
retention needs.

Lifecycle, with each stage a scheduled worker job:

```text
course ends
   |
   +30 days   notify instructor: archive, delete, or extend
   |
   +100 days  automatic archive of student records (configurable default)
   |
   +1 year    permanent deletion of student records (configurable)
```

What each stage touches:

| Deleted with student records                        | Retained indefinitely                                    |
| --------------------------------------------------- | -------------------------------------------------------- |
| Enrollments                                         | Published Questions and immutable Question Revisions     |
| Assignment Attempts, Question Attempts, submissions | Question Library, Question Classifications, licensing    |
| Grades and summary rows                             | Instructor question drafts and workspaces                |
| Timer events and render traces                      | Assignment Content (Instructor's choice at archive time) |
| Per-student analytics                               | Backend capability metadata                              |
| Student-record bucket artifacts                     | Anonymous question statistics (below)                    |

Wording matters in the notification, because "reset" sounds like breakage. The instructor-facing copy
is: _This course ended 30 days ago. Student records are still available. If they are no longer needed,
archive or delete the course now. Student records will be automatically removed after 100 days unless
the course is archived or the retention period is extended by a sysadmin._

Retention follows the configured course lifecycle policy, with the privacy-preserving default applying
when no course-specific policy is present. Any broader operational retention capability is an explicit,
audited Sysadmin operation rather than ambient course or Question Library authority.

### How backups interact with deletion

Stated plainly because external institutions may ask, and because the honest answer is a constraint rather
than a feature. Deletion removes student records from live systems immediately and irreversibly.
Encrypted backups and point-in-time recovery snapshots taken before the deletion still contain those
records until they age out under the backup window's own retention.

Selective purge from a point-in-time recovery window is not possible with managed snapshots, so the
documented guarantee is: _deleted student records are immediately unrecoverable through the
application, and expire from encrypted backups within the configured backup window._
`docs/RETENTION_POLICY.md` states the current window numerically. An external institutional policy
requiring a shorter total exposure must shorten its backup window, which is a deliberate
durability-versus-privacy trade-off for that institution to make rather than one this platform makes.

### Anonymous question statistics survive deletion

The feature that makes deletion sustainable: the question library should keep improving after the
records that taught it are gone. Aggregate statistics live in **shared content**, carry no course or
Student identifiers, and survive record deletion:

```text
Question 123 (Question Revision Number ...)
  attempts_mean 2.7
  time_median_s 58
  difficulty_index 0.71
  discrimination_index 0.43
  cohort_size 214
```

Three design consequences, because this cannot be bolted on afterward:

- Statistics are **aggregated incrementally or by scheduled rollup while records still exist**, never
  computed on demand from attempt history. Deleting the attempts must not delete the knowledge.
- Discrimination index needs per-student total scores, so it is computed before deletion and stored as
  an aggregate. A post-deletion recomputation is impossible by design, which is the point.
- Aggregates are suppressed below a minimum cohort size. With one student, "average attempts: 3" _is_
  that student's record, so a k-anonymity threshold (default 5) gates publication. The reviewer did
  not raise this; it is the difference between an anonymous statistic and a re-identifiable one.

## Question identity and lifecycle

One human identity sits above three implementation identities with distinct jobs:

| ID                       | Scope                                     | Mutability                      | Visibility                              |
| ------------------------ | ----------------------------------------- | ------------------------------- | --------------------------------------- |
| Question ID              | One published-question lineage            | Stable for its lineage          | Human-facing, discoverable, and citable |
| `workspace_id`           | One Instructor's private authoring item   | Freely editable and deletable   | Private implementation identity         |
| Question Revision Number | One immutable published Question Revision | Never changes after publication | Exact version evidence                  |

Lifecycle: `draft -> validated -> published`; published Question Revisions then have Available or Archived Question Revision Availability.

- A draft gets an internal UUID immediately so it can be referenced and collaborated on, but that
  UUID is never presented as a problem number.
- The first publish transition mints the stable `QuestionId` and immutable positive Question Revision
  Number `1` only after validation passes. A later same-lineage publication advances that immutable
  version number after the same validation. Retired sequential display identifiers are not
  published-question identities.
- Editing never mutates a historical QuestionRevision. A moderate lineage-steward edit publishes a
  validated immutable successor under the same `QuestionId`, preserves original authorship and the
  existing Creative Commons license, and records its semantic class and impact. A full fork by any
  vetted Instructor creates a private draft with fork-author authorship plus source attribution and
  a source-compatible Creative Commons license; validation publishes it as a new `QuestionId`
  lineage.
- `QuestionChangeProposal` is the lightweight contribution path. It pins one exact base version and
  carries a patch and rationale through automated validation; the lineage steward accepts or rejects
  it. Acceptance creates an immutable same-lineage version with contributor credit and preserved
  original authorship/license. A stale base is rebased and resubmitted. Proposals reuse improvement
  threads through a focused accept/reject workflow.
- Replacing an image creates a new asset object with a new checksum and key; the old object stays so
  historical attempts remain reproducible.
- Assignments and Issued Questions retain hidden exact `(question_id, revision_number)` snapshot evidence. No
  ordinary successor advances an Assignment, Issued Question, or grading evidence; an Instructor applies a
  Blueprint Update under the Assignment's strong revision contract. A
  `ForcedQuestionCorrection` follows its separately authorized, deterministic replacement and
  remediation contract.
- Every Published Question remains discoverable and exactly resolvable to every active Instructor
  after publication, with its Question Revision Availability and any Archive reason
  visible in Question Library results and Question Details. `Available` Question Revisions are ordinarily selectable for
  new assignments. `Archived` Question Revisions remain discoverable and resolvable for
  evidence and history, but are excluded from ordinary new selection and new references.
  An Archive reason records why an Instructor no longer offers that Question Revision for new selection.

### Publication governance

Who may publish into the Question Library is a product decision that shapes the data model, so it is
settled here rather than discovered during M4.

Publication has one shared Question Library entry event. Drafts remain private to their Question authoring workspace and
explicit collaborators until validation succeeds. A validated publication is discoverable and
resolvable by every active Instructor, regardless of course, with visible Question Revision Availability. Available
Question Revisions are ordinarily selectable; Archived Question Revisions remain available for
evidence/history resolution but are excluded from ordinary new selection. Publication authority uses
the approved-Instructor predicate; Sysadmin status alone is not a publication or course-creation
authority.

Conflicting changes from multiple authors are prevented structurally rather than merged. A published
QuestionId has a lineage steward or steward set; validated moderate edits use that authority and
preserve original authorship and license. Every vetted Instructor may instead create a private fork
draft with its own authorship, source attribution, and source-compatible license; validation gives
that fork its own published QuestionId lineage. An optional immutable Question Fork Source records
the source Question Revision without making version management an Instructor task.

An Instructor who wants to contribute a focused improvement without creating a fork submits a
`QuestionChangeProposal` against an exact published version. Automated validation runs before the
lineage steward's accept/reject decision. Acceptance records the contributor, preserves original
authorship and license, creates an immutable successor in the existing lineage, and adds the result
to the existing improvement thread. A proposal whose base is stale is rebased and resubmitted.

### Replacing questions and teaching sets

A validated moderate edit publishes a new immutable version in the same QuestionId lineage; a full
fork that becomes a distinct teaching task has a new QuestionId. Existing Assignments and Issued Questions
remain pinned. An Instructor may explicitly apply a controlled assignment update after revision
checking; publication itself leaves teaching records unchanged. Assignment import/copy and selectable
checklists from existing assignments are the normal way to reuse a teaching set; direct Question ID
entry is a bounded lookup tool, not a range or batch-selection language.

## Object storage and content identity

Keys are immutable and derived from IDs and versions, never from user-supplied filenames:

```text
questions/{question_id}/versions/{revision_number}/source/{object_id}
questions/{question_id}/versions/{revision_number}/assets/{asset_id}/{object_id}
questions/{question_id}/versions/{revision_number}/restricted-assets/{asset_id}/{object_id}
student-records/exports/{course_id}/{export_id}/exam.pdf
temp-processing/imports/{import_id}/
```

Every Object Record carries its typed Object Address, address-derived Object Storage Area and Object
Data Class, Object ID, SHA-256, size, media type, and creation time. The owning Question Revision,
import, render, export, or Student Record relationship retains its own source and legal evidence.

Requests resolve assets from a known object record and read pre-parsed models, so bucket listings and
archive parsing stay in the worker at import time. Public Question Library assets are served from CloudFront by
an immutable URL only after the publisher activates a precisely tagged `public-assets` record.
Restricted content and student records require `POST /api/assets/{id}`; the server authenticates and
authorizes the authenticated Account, logs the grant, and returns a bounded short-lived signed URL. There is no
protected GET route whose browser navigation, history, or speculative fetch can mint authority.

The `renders/{seed}` prefix is what makes the WeBWorK renderer affordable: rendering is deterministic
given `(question_id, revision_number, seed)`, so the first render fills the cache and every later Student with that seed
gets a CDN hit instead of a Perl fork.

Authoritative-versus-derived roles, settled per backend:

| Backend         | Authoritative source_object_reference                      | Derived                                                  |
| --------------- | ---------------------------------------------------------- | -------------------------------------------------------- |
| PLE algorithmic | Generator id and version; parameters derived from the seed | Rendered output                                          |
| PLE static      | Canonical versioned PLE Question JSON source               | Public Question model and private Question Grading Input |
| WeBWorK         | PG source reference and version                            | Rendered HTML, images, cached renders                    |
| QTI             | Original ZIP in object storage                             | Parsed model in shared content, extracted assets         |
| H5P             | Remote package reference                                   | Any imported internal representation                     |

QTI import runs in the worker, never a request: store the original ZIP unchanged; validate structure;
reject unsafe paths, symlinks, and unexpected entries; enforce maximum archive size, maximum expanded
size, and file-count limits; extract into an isolated `temp-processing` workspace; parse the manifest;
store each referenced asset as its own checksummed object; rewrite content references to internal
asset IDs; convert supported content into the internal model; record unsupported features explicitly
so they survive as data; preserve the original package so a later parser improvement can re-import.
Determine every media type by sniffing the stored bytes, treating any supplied type as a hint to
verify.

PLE Question JSON follows the same source-preservation principle without
copying QTI's interchange model. The bounded answer-bearing JSON is private in
the workspace, is canonicalized and checksummed by the PLE Question Backend, and is
promoted to an immutable non-signable Question Source object at publication.
The compiler writes an answer-free operational JSONB projection and separate
grader-only key/feedback JSONB. Student rendering and grading read those compact
database projections rather than fetching or reparsing the source object.

The PLE Question JSON package implements that transition: a typed
compare-and-swap save atomically advances the workspace draft and source
metadata; publication binds the copied canonical source, answer-free model, and
typed private payload in one Question Library transition; and PLE runtime grading
uses an independently injected grader-only capability. The completed instructor
editor uses the protected author-only canonical-source route as its narrow
answer-bearing browser exception; student and public contracts remain
answer-free. The bounded Canvas and Blackboard QTI profile mappings, Q3 pure
PLE Question JSON bridge, Q4 provenance contract, Q5/WP-QTI-7 schema/RLS/object-
binding implementation, WP-QTI-8 Memory/PostgreSQL conversion boundary, and WP-QTI-9 server routes
are complete. WP-QTI-8
closes staged profile evidence, revalidates exact accepted-result `itemId` binding, and atomically
commits the CAS revision, draft, canonical source, current private grading, and current origin under
the frozen lock order. Ordinary saves stage current grading, publication promotes only the stored
grading value after origin promotion, and PostgreSQL reaches private provenance and grading only
through forced-RLS protected database operations. Strict lowercase 64-hex `Sha256Checksum` serialization keeps
the evidence boundary exact. WP-QTI-9 adds deterministic private archive/job ingress, strict worker
evidence, answer-free report review, strong-ETag atomic conversion, deterministic published archive
copy, and a prepared-import draft-deletion fence with Memory/PostgreSQL parity. The route, worker,
and independent P0/P1 evidence passed. WP-QTI-10 author UI is also complete: it uses a feature-local
answer-free QTI client and the existing workspace route/editor, keeps the selected ZIP and safe report
only in component memory, requires an acknowledged report plus the displayed clean strong revision,
and locks the stale editor through conversion/refetch recovery. Real-route Chromium and offline
evidence passed. WP-QTI-11 live PostgreSQL/RLS/profile-to-PLE acceptance is complete: a fresh
PostgreSQL 17 database exercised the real upload worker, mixed accepted/rejected report, native
conversion and publication, correct/incorrect grading, role denials, provenance, and exact cleanup.
WP-QTI-12 independent review and documentation close-out are also complete: six separate passes
reported no remaining P0/P1 issue after stale README and ownership-map findings were corrected and
re-reviewed. PLE Question JSON schema version 2 now implements MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT
source/runtime semantics based on the reviewed QTI Package Maker item models. Version 1 source is
refused; version 2 is the sole PLE reader. Remaining acceptance is recorded in
`docs/active_plans/active/ple_question_json_schema_evolution_plan.md`; external QTI-JSONL is a separate
future adapter concern. Historical course-appearance WP-CA1 through WP-CA7/WP-RC1 and WP-RC2
receipts do not establish current product acceptance: the durable Store/current-pointer, PostgreSQL,
route, authorization, upload-promotion/cleanup, and editor feature is deferred and incomplete after
the pre-production schema reset. `WN1-QM-PRESENTATION-COURSE-APPEARANCE-VIEW` remains a separate
reader-only terminology closure. The shipped upstream WeBWorK implementation is historical WP-RC3 evidence. The dependency order and exact
profiles, refusal semantics, provenance
boundary, author workflow, and acceptance gates are tracked in the current status registry.

Deduplication is designed for but not built: the logical `asset_id` is stable and the physical key is
chosen inside MOD-OBJ, so a later move to `objects/sha256/ab/cd/...` changes no caller.

## Reproducibility record

Every Question Attempt persists its exact `QuestionRevisionReference { question_id, revision_number }`, source Object Reference `object_id` and `sha256`,
Question Backend Version, Question Renderer Version where one applies, Question Generator,
seed, **parameter hash**, asset `object_id` list, Question Grader Version, and rendered-question hash.

The record stores a parameter _hash_ rather than the parameters, because parameters are reproducible
from seed plus generator version by construction and WP-C4 proves regeneration is exact. At 300 M
attempts per term, storing both would add hundreds of gigabytes to restate what the seed already
determines. The hash still detects a regeneration mismatch.

This is the requirement that forces immutability upstream: a mutable question row or an overwritten
image makes the record a lie.

## Source-of-truth and compatibility policies

Four ownership questions that look like implementation details and are actually architecture. Each is
settled here because discovering the answer during recovery or a decade-later migration is expensive.

### Which source_object_reference defines a PLE Question

The other backends have obvious authoritative artifacts; the PLE Question Backend
needed separate rulings for generated and static questions.

For an algorithmic question, **the pinned generator identifier, generator
version, and parameter specification are authoritative.** The normalized
Question Revision public model is a derived, cached projection for
rendering and search, regenerable from the pinned generator at any time.

The consequence that matters: **a generator evolving leaves every historical publication snapshot
intact.** Generator version remains part of hidden snapshot identity. A validated moderate generator
or content edit publishes a new immutable QuestionRevision in the same QuestionId lineage; a full fork
publishes a new QuestionId with an explicit Question Fork Source. Existing assignments and completed attempts keep
resolving to their exact evidence until an Instructor explicitly applies a controlled assignment
update. Generator implementations are therefore additive-only while referenced by historical grading
evidence.

For a static Question, **the canonical, versioned PLE Question JSON
source is authoritative.** Publication preserves it as a private immutable
source object and compiles it into an answer-free public model plus separately
granted grader-only material. The two compiled values carry checksums and a
public/private binding, but neither replaces the preserved source for recovery
or future re-import. QTI remains an adapter into and out of supported internal
semantics; Canvas or Blackboard XML never becomes the PLE source contract.

| Backend         | Authoritative                                           | Derived and regenerable                                  |
| --------------- | ------------------------------------------------------- | -------------------------------------------------------- |
| PLE algorithmic | Pinned generator id, generator version, parameter spec  | Normalized model, rendered output                        |
| PLE static      | Canonical versioned PLE Question JSON source            | Public Question model and private Question Grading Input |
| WeBWorK         | PG source reference and version                         | Normalized model, rendered HTML, cached renders          |
| QTI             | Original ZIP in object storage                          | Parsed model, extracted assets                           |
| H5P             | Remote package reference                                | Any imported internal representation                     |
| iMathAS         | Checksum-pinned source snapshot and integration profile | Safe render envelope and deterministic render cache      |

### Reading a version 1 payload with version 5 software

Immutability creates a long-lived compatibility obligation. Every stored payload carries a
`model_schema_version`, and readers **upcast on read** into the current in-memory model, leaving the
immutable row as written.

The mechanism that keeps this honest is a committed compatibility fixture set holding one payload per historical schema
version, with a test asserting every one still loads into the current model. A schema change that
cannot upcast an existing fixture-set entry is rejected at the gate. Dropping support for a historical
schema version is an explicit breaking change requiring a documented batch re-publication path, never
a silent read failure.

### Database or object store: who owns existence

The reconciliation job implies these can diverge, so the asymmetry is stated rather than discovered:

- **The database is authoritative for object existence.** An object record with no corresponding
  bucket object is a _broken reference_: a defect, alerted on, never auto-repaired by deleting the
  record.
- **The object store is authoritative for bytes.** A bucket object with no record is an _orphan_:
  garbage, collectable after a quarantine window.

Write ordering follows from that: **bytes first, record second.** A crash between the two leaves an
orphan, which is harmless and collectable. The reverse ordering would leave a broken reference, which
is harmful. Checksums are verified on read so a silently corrupted object surfaces as an error rather
than as wrong content shown to a student.

### Grading version compatibility and regrading

**Question Grader Versions are additive and permanently executable.** A Question Grader Version is never
removed while any attempt references it, because a grade is a record and being unable to explain how it
was produced is not acceptable.

Regrading is supported and explicit: it creates a **new grade event** referencing the new Question
Grader Version, never overwriting the old one. The history therefore shows both results and the reason for
the change, which is what makes "why did my grade change" answerable. Durable replay behavior tests invoke
representative retained Question Grader Versions; a one-time compatibility review confirms that every
release referenced by accepted historical records remains callable.

## Scale evaluation

Planning scenarios: 10,000,000 problems, 1,000 instructors, and 50,000 students; starting
observation: 500 problems, 2 instructors, and 100 students. These values are sizing assumptions,
not acceptance thresholds. The application model stays stable while a configured deployment model
responds to observed workload; the one-time capacity review records the evidence for each decision.

### Activity volume is the dominant concern

Planning assumption based on the owner's reported practice behavior, not a permanent acceptance
target. The values give the capacity review a declared workload shape; observed workload and the
configured storage/query budgets decide whether the deployment model needs to change.

| Planning quantity                                                 | Sizing observation    |
| ----------------------------------------------------------------- | --------------------- |
| Students x assignments x questions x complete Assignment Attempts | 50,000 x 10 x 20 x 30 |
| Question instances per term                                       | ~300 M                |
| Plus incorrect Question Attempts within Assignment Attempts       | 500 M+ rows over time |
| Peak submission rate (due-date evening)                           | ~300-500 / s          |
| Database writes per submission                                    | ~4                    |
| Peak write rate                                                   | ~2,000 writes / s     |

These figures are one-time sizing observations. A class-start or release review records the actual
workload shape, observed queue/error/latency distribution, and configured capacity budgets; no value
in this table is a permanent pass/fail threshold. The planning scenario suggests the following
design responses:

- The four highest-volume tables (`question_attempt`, `submission`, `grade_event`, `audit_event`) use
  monthly range partitions from the first migration. A one-time workload and query-plan review
  validates that choice against the configured retention and storage model; other tables remain
  unpartitioned until observed workload justifies a change.
- Grades come from `student_assignment_summary`, never from scanning attempts.
- Attempt rows stay compact: seed plus parameter hash, not parameters.
- Verbose render traces and temporary artifacts get retention rules, not indefinite storage.

### Planning sizing observation: 10 million problems

The 10-million-problem scenario is a planning assumption rather than a release gate. The one-time
capacity review records the observed payload distribution and configured hot/cold storage budgets.
The illustrative arithmetic below explains the proposed split without granting permanent authority
to row, byte, or index counts:

| Table                       | Contents                                                                    | Size at 10 M              | Access                       |
| --------------------------- | --------------------------------------------------------------------------- | ------------------------- | ---------------------------- |
| `question_revision`         | Identity, lifecycle, capability and Question Classification refs, checksums | ~2 GB                     | Hot; browse, search, resolve |
| `question_revision_payload` | Normalized Question Revision public model                                   | ~100 GB, hash-partitioned | Cold; read on attempt issue  |

Browse and search run against the configured metadata projection. A one-time query-plan and workload
review decides whether the current PostgreSQL faceting remains suitable; immutable versions make a
later search-index replacement safe when observed query volume or configured capacity calls for it.

### Planning sizing observation: 1,000 instructors

This scenario tests the single-installation capacity choice. One cluster with forced RLS remains the
configured starting model; observed workload, regulatory requirements, and the deployment capacity
model decide whether a later storage or service split is warranted. A one-time operations review
records that decision and its evidence.

Simultaneous students are evaluated as observed concurrency against the configured replica and
database budgets, with stateless replicas added when the measured workload requires them.

### WeBWorK workload observation

Rendering and grading are CPU-heavy Perl in the observed provider configuration. A named load review
records queue depth, CPU, timeout, and user-visible readiness behavior against configured budgets.
Deterministic render caching by `(question_id, revision_number, seed)`, Question prefetch, and worker scaling remain
the first responses when the observations show pressure. Submitted answers are still graded
server-side regardless of caching.

### Capacity-model replacement decisions

| Component                          | Replacement signal                                      | Decision                                                                                              |
| ---------------------------------- | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `FOR UPDATE SKIP LOCKED` job queue | Observed queue pressure exceeds its configured budget   | Evaluate a durable external queue with the same job contract                                          |
| PostgreSQL faceted search          | Observed query workload exceeds configured index budget | Add a search index fed from immutable versions                                                        |
| Single writer                      | Observed sustained writes exceed configured DB budget   | Add Question Library/reporting replicas, then evaluate typed scope partitioning                       |
| One installation, one cluster      | Regulatory or contractual separation need               | Evaluate a separately approved installation boundary; preserve exact resource ownership in this model |

These are decision procedures, not permanent numeric gates. Each replacement requires a dated
workload observation, the active configuration, and an architect-approved receipt.

### Deployment at each end

|            | Start                          | Target                                 |
| ---------- | ------------------------------ | -------------------------------------- |
| `api`      | 1-2 Fargate tasks              | Autoscaled on request count            |
| `worker`   | 1 task                         | Autoscaled on queue depth              |
| `renderer` | 1 task                         | Autoscaled pool                        |
| Database   | One modest RDS instance        | Larger primary plus read replicas      |
| Objects    | One MinIO, then one bucket set | CDN-backed, lifecycle rules            |
| Search     | PostgreSQL full-text           | Dedicated index if faceting demands it |

## Browser interface design

The student-facing surface is where the platform is judged, so it gets the same treatment as the
domain. Two repo documents already govern it and are treated as requirements rather than suggestions:
[PLAYFUL_TRAINING_GAME_STYLE.md](../PLAYFUL_TRAINING_GAME_STYLE.md)
targets students aged 15-30 building a real skill, which is exactly this audience, and
[COLOR_CONTRAST_ACCESSIBILITY.md](../COLOR_CONTRAST_ACCESSIBILITY.md)
governs palette contrast.

### Route map

Human-facing route parameters are typed public references. The server resolves them inside the
authenticated `AuthenticatedSession` and exact course/membership boundary before loading the existing internal
UUID model. Public
references are locators, never authorization. Internal UUIDs may remain in background API and asset
requests, but they do not appear in the address bar or user-copyable navigation links.

| Route                                                                    | Surface                                                             | Notes                                                            |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `/`                                                                      | Course list for the signed-in role                                  | Student and instructor views diverge below this                  |
| `/courses/:courseRef`                                                    | Assignment list with progress and Assignment Attempt counts         | `C-n`; reads summary rows                                        |
| `/courses/:courseRef/assignments/:assignmentRef`                         | Assignment overview, Assignment Attempt history, start or resume    | `C-n` and `A-n`; entry point for an Assignment Attempt           |
| `/assignment-attempts/:assignmentAttemptRef`                             | The Assignment Attempt loop, one Question at a time                 | `R-n`; the screen that must feel instant                         |
| `/assignment-attempts/:assignmentAttemptRef/summary`                     | Assignment Attempt result, per-Question outcomes, practice re-entry | `R-n`; where practice re-entry lives                             |
| `/library`                                                               | Question Library browser                                            | Virtualized, faceted, cursor-paged                               |
| `/library/:questionId`                                                   | Exact published-question detail and statistics                      | `AAA-BBBB`; hidden snapshot lineage is not UI state              |
| `/workspace`                                                             | Instructor drafts                                                   | Private, pre-publication                                         |
| `/workspace/:workspaceRef`                                               | Draft editor with validation and preview                            | `W-n`; preview renders through WASM generation                   |
| `/instructor/courses/:courseRef/assignments/:assignmentRef`              | Instructor assignment home and local navigation                     | `C-n` and `A-n`; summary and next actions                        |
| `/instructor/courses/:courseRef/assignments/:assignmentRef/questions`    | Assignment questions, pools, and selection                          | Focused revision-checked content mutation                        |
| `/instructor/courses/:courseRef/assignments/:assignmentRef/policies`     | Assignment delivery and lifecycle policies                          | Focused revision-checked policy mutation                         |
| `/instructor/courses/:courseRef/assignments/:assignmentRef/student-view` | Current answer-free student landing                                 | Instructor identity stays active; graded work uses Student entry |
| `/instructor/courses/:courseRef/gradebook`                               | Gradebook                                                           | `C-n`; reads summary rows only                                   |

### Reactivity contract

`docs/SOLID_MODEL.md` records this and is the file a reviewer checks a component against. Solid's
fine-grained model is the reason a timer ticking four times a second costs one text-node update rather
than a component re-render.

| State                                              | Primitive                                | Owner                    | Rationale                                                                                        |
| -------------------------------------------------- | ---------------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------ |
| Session and role                                   | Context over a store                     | App shell                | Read everywhere, written rarely                                                                  |
| Current Assignment Attempt and per-question status | Store with granular reads                | Assignment Attempt route | Nested and partially updated; a store avoids replacing the whole object on one question's change |
| Remaining time                                     | Signal holding an integer of deciseconds | Timer component          | Scalar, high-frequency, one subscriber                                                           |
| Submission in flight                               | Signal holding a discriminated union     | Attempt component        | `idle`, `validating`, `submitted`, `graded`, `failed`                                            |
| Question content                                   | Resource keyed on question attempt id    | Question component       | Async, suspendable, cache-friendly                                                               |
| Prefetched next question                           | Store keyed by question index            | Prefetch controller      | Written by prefetch, read by navigation                                                          |
| Question Library browse results                    | Resource plus cursor signal              | Question Library route   | Cursor pagination, never an offset                                                               |

Conventions the review checklist enforces positively: read props at the use site so reactivity is
preserved; place teardown in `onCleanup`; render dynamic lists with `<For>` when identity matters and
`<Index>` when position matters; derive values with `createMemo` rather than writing them from an
effect.

### Question rendering and sanitization

Rendering a backend-neutral question is the most security-sensitive part of the frontend, because two
adapters return markup produced elsewhere.

The pipeline: the API returns a **render envelope** holding prompt blocks, a Question Response Format, and
asset references. The renderer maps each block to a component, and each Question Response Format to a
question response control. Two block kinds carry supplied markup -- WeBWorK rendered HTML and QTI converted
content -- and both pass through a **server-side allowlist sanitizer** before ever reaching the
envelope. Sanitization happens on the server, in the worker at render time, so the sanitized form is
what gets cached and what every client receives; the browser trusts the envelope because the server
already validated it.

The allowlist covers structural markup, math, tables, and images whose `src` resolves to an internal
asset ID. Script, style, event-handler attributes, iframes, and external URLs are dropped at
sanitization time and the drop is recorded on the render record, so an adapter producing unexpected
markup is visible rather than silent.

Question Response Controls, one per response type in `question_model`, are the reusable core of the student UI:
numeric entry with unit display, formula entry with live format validation, single and multiple
selection, ordering, matching, and short text. Each widget calls the WASM
format-validator on input and shows a local, immediate hint when the shape is wrong -- the one place
the browser gives a real-time answer-adjacent response, and it is safe because format validity carries
no information about correctness.

### Student disclosure is assignment-owned

`docs/PLAYFUL_TRAINING_GAME_STYLE.md` makes the wrong-answer screen the highest-value screen in the
product and requires three parts in order: what the student chose, the correct answer, and one sentence
of why. That is pedagogically right for mastery and practice, and wrong for a quiz or exam where
revealing the answer defeats the assessment.

Each assignment owns five independently timed student fields: score, per-item correctness, feedback
text, solution, and class statistics. Each field uses one closed timing: during attempt, after
submit, after due, after close, or never. The server evaluates the current assignment policy only
after current S5 entitlement, using the current S3-resolved effective-policy verdict/decision,
authoritative time, and the submission fact. When a due or close boundary is absent, the
corresponding timing withholds the field; withheld fields are omitted from the response envelope.

The browser receives neither policy nor clock inputs and therefore cannot infer a future disclosure.
`feedback_release` is an immutable audit receipt of an instructor action, not a student-result
transition or alternate authority. A client asking for more receives no more, which keeps the
answer-secrecy guarantee independent of UI correctness.

### Timer design

The browser timer is display; the server owns the verdict. A signal decrements from the server-supplied
expiry, and the component reconciles against the server's remaining-time value on every response so
drift self-corrects rather than accumulating. At expiry the client submits whatever exists and the
server rules on whether it arrived in time. A clock moved forward on the client shortens only that
student's own display, and the server's verdict is unaffected -- verified by a test.

Presentation follows the training-game guidance: the timer is legible at a glance, calm rather than
alarming, and it never becomes the loudest element on screen. A student who runs out of time sees a
teaching screen, not a failure screen.

### Prefetch and perceived latency

Perceived speed comes from three mechanisms, in order of contribution:

1. **Next-question prefetch.** While a student works on question N, the client requests question N+1's
   envelope and warms its assets. Navigation after a graded answer is then a store read.
2. **Local format validation.** Malformed input is caught in WASM with no request.
3. **Explicit pending state.** The submit button enters a `submitted` state immediately with the
   student's answer echoed back, so the round trip is visible progress rather than a frozen UI. No
   correctness is implied or guessed before the server answers.

Next-question prefetch uses a durable, server-only reservation rather than creating an early attempt.
Its browser projection is answer-free, but the reservation retains the issued private grading
authority needed to avoid later Question Library or renderer reconstruction. It binds the current unresolved
attempt, the first unattempted assignment position, the server-owned seed, parameter hash, and
complete Question Attempt Reproduction Details. Submitting question N promotes that reservation into the one real N+1
attempt and timer, then records either an immutable
`nextIssued` descriptor or durable `nextPending` state in N's idempotent receipt. Initial recovery
can heal a committed-but-unlinked successor from the sole pending receipt, but replay never scans
newer Assignment Attempt state to rewrite a receipt.

The browser keeps the prefetched envelope only in memory. It advances without another Assignment Attempt screen fetch
only when the receipt's minimal `nextIssued` descriptor exactly matches predecessor, Assignment Attempt, position,
version, seed, and backend-owned rendered hash. Mismatch, outage, teardown, or a late response clears
the speculative value and uses the ordinary authoritative screen path. Asset warming is limited to 12
deduplicated same-origin logical asset routes.

### Failure states

An assessment tool is judged on what happens when the network drops mid-question, so these are
designed rather than discovered:

| Situation                              | Behavior                                                                                                                            |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Submission request fails               | Retry with the same idempotency key, backing off; the answer stays visible and editable; the timer keeps its server-anchored expiry |
| Repeated failure                       | A persistent banner states the answer is saved locally and will be submitted, with a manual retry control                           |
| Session expires mid-Assignment Attempt | Re-authentication returns the Student to the same question with the Assignment Attempt intact                                       |
| Question content fails to load         | The question shows a retry affordance and the Assignment Attempt remains resumable                                                  |
| Renderer unavailable                   | Only WeBWorK-sourced questions show a degraded state; the rest of the Assignment Attempt proceeds                                   |

### Accessibility

An assessment platform carries institutional accessibility obligations, so this is a gate rather than
a polish pass. `docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md` owns the complete student interaction
contract. Every question response control has a visible platform path: Tab and Shift+Tab move focus, Space
selects choices and activates buttons, and native links retain Enter activation. The primary
acceptance journey reaches the explicit Submit answer button without requiring an Arrow key, digit,
response-input Enter, or Escape. Enter-to-submit, Arrows, visible-choice digits 1-9, and Escape are
widget extensions with separate scenarios so their failures do not hide a platform-path regression.
Every widget carries a programmatic label and
announces its validation state through a live region, so a screen-reader user learns that an entry is
malformed at the same moment a sighted user sees it. Timers announce at meaningful intervals rather
than on every tick. Student primary response controls provide comfortable 44 CSS-pixel targets,
while pointer-oriented instructor controls may use a compact 36 CSS-pixel control height with
adequate separation. Contrast is verified against `docs/PALETTE_CONTRAST_AUDIT.md` with measured
values in both standard and increased-contrast presentation, and color never carries meaning alone:
correct and incorrect states pair their color with an icon and text. MATCH, FIB,
MULTI-FIB, and HOTSPOT must pass both their platform path and separately scoped extension behavior
before WP-RC5 acceptance.

### Client architecture

| Concern               | Choice                                                                                                                                                                    |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Framework shape       | Solid SPA, static bundle, one Rust backend                                                                                                                                |
| Router                | `@solidjs/router`                                                                                                                                                         |
| Query and cache layer | `query` plus `createAsync` from `@solidjs/router`, already present with the router; keyed on Question Attempt, Assignment Attempt, and cursor so revalidation is explicit |
| API access            | The generated typed client only; every call goes through it so the boundary stays one file deep                                                                           |
| WASM loading          | One module instantiated once at app start, awaited behind a splash state, shared by every consumer                                                                        |
| WASM fallback         | When instantiation fails, format validation falls back to a server call and the app continues with a round trip per validation, reporting the degradation once            |
| Server authority      | The server owns grading, timer verdicts, completion, and grade state; the client owns navigation, display, and input buffering                                            |

Browser persistence is deliberately narrow, since anything stored is data at rest on a shared machine:

| Store            | Contents                                                                            | Cleared                                                     |
| ---------------- | ----------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| Account settings | Presentation contrast preference; default standard, optional increased contrast     | On user change or explicit reset                            |
| `localStorage`   | Device-local sound and reduced-motion preferences only                              | On explicit reset                                           |
| `sessionStorage` | In-progress response text keyed by Question Attempt, for crash and refresh recovery | On successful submit, Assignment Attempt exit, and sign-out |
| Memory only      | One exact key-free next-question envelope and its receipt-binding descriptor        | On advance, mismatch, Assignment Attempt exit, or unmount   |
| Nothing          | Session tokens, keys, grades, and any answer-bearing value                          | n/a                                                         |

Session identity lives in an `HttpOnly` cookie the page cannot read, which is what keeps it out of the
table above.

### Frontend security rules

- **Answer-bearing types stay out of the generated client.** Type generation runs over
  `crates/question_model` only, and `crates/grading` is never a generation input. A test asserts the
  generated surface contains no answer-key type, mirroring the WASM export allowlist so both halves of
  the secrecy boundary are checked the same way.
- **Supplied markup is sanitized server-side** before it enters a render envelope, so the sanitized
  form is what gets cached and delivered.
- **Content Security Policy** ships with the app: script sources limited to the bundle's own origin,
  `wasm-unsafe-eval` present because WebAssembly instantiation requires it, `object-src` empty, and
  frame ancestors limited to the LMS origins configured for LTI launch. The esbuild bundle contains no
  inline script, so no inline allowance is needed.
- **Asset URLs** are internal asset IDs resolved through the client, so a bucket URL never appears in
  markup.
- **Logging** carries identifiers and error codes; response text, grades, and student names stay out of
  the browser console and any telemetry payload.

### Forms, errors, and focus

- Question Response Controls are controlled inputs with validation state as data, so a widget renders its own
  error text and an Assignment Attempt-level summary can list every outstanding issue from the same source.
- An error boundary wraps each route and, separately, the question renderer, so a failure in one
  question's content leaves the Assignment Attempt shell and timer intact and offers a retry.
- Focus moves deliberately between attempt phases: to the feedback panel when a result arrives, so a
  screen-reader user hears the teaching content immediately, then to the advance control once feedback
  has been announced. Focus returns to the first question response control on the next question.
- Every asset carries required accessibility text on its object record, and the renderer surfaces it as
  alt text or an extended description. Math renders as MathML with a text alternative; structure and
  sequence figures require a description before a problem version may be published, which is a
  validation rule rather than an author's good intention.

### Frontend validation strategy

| Layer                    | Covers                                                                                                                                                                                |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Node component tests     | Question Response Control behavior, validation state transitions, envelope-to-component mapping                                                                                       |
| Playwright functional    | Mastery loop, post-completion practice Assignment Attempt, give-up flow, resume after refresh, retry after a failed submission, timer expiry, publish refusal                         |
| Playwright accessibility | Keyboard-only completion of a full Assignment Attempt, focus order across attempt phases, live-region announcements, contrast measured against `docs/COLOR_CONTRAST_ACCESSIBILITY.md` |
| Playwright network       | Offline submit and recovery, slow renderer, expired session mid-Assignment Attempt, WASM instantiation failure falling back to server validation                                      |
| Playwright visual        | Feedback panel states and timer states, where a rendering regression is easier to see than to assert                                                                                  |
| Interaction latency      | Recorded during named release or usability investigations; not a permanent browser assertion until PLE has a reproducible benchmark environment and an approved user-facing SLO       |

### Instructor surfaces

The instructor side is the larger build and its hard problem is scale, not styling.

**Problem browser over ten million rows.** A virtualized list backed by cursor-paged queries, with
facets over Question Classification, capability, license, and statistics. Facet counts come from the Question Library's own
aggregates so the UI never triggers a full scan. Search is a single input over full-text and trigram
matching, and the component boundary keeps the query behind a repository call so a dedicated search
service can replace it without a UI change.

**Assignment editor.** Question selection uses one exact published Question ID from the Question Library; a
workspace draft must be published before it can be selected. Internal references retain the exact
snapshot needed for deterministic grading without exposing a version choice. The editor exposes the four
Assignment Activity Rule controls with their current values visible. Timing and attempt
policies remain properties of each published question rather than assignment overrides. The browser
submits only the ordered resolved references and policy choices. The server resolves every Question
ID through the Question Library after the approved-Instructor and exact-course checks, uses the persisted
capability declaration, and returns the
complete deterministic
`validate_assignment_config` violation list. Capability failures render beside the affected selections so
the instructor sees every violation at once rather than one per submission.

In the maintained desktop profile, assignment organization uses the useful page width instead of
confining the Question Library to an aside. Selected questions form compact scan rows with drag ordering,
small directional controls, and direct position selection; these mechanisms share one ordered-list
mutation contract. Four questions, the policy summary, and the save action fit in the initial
workspace. Permanent explanatory copy becomes contextual help where it does not affect the current
decision.

**Draft editor and preview.** A draft renders through the same question components as the student view,
generating parameters in WASM so an author sees a real variant per seed without a server round trip.
The preview shows the student view and the answer-key view side by side, since an author needs both.

**Publish flow.** Validation results identify whether an edit is a moderate same-lineage successor or
a full fork, show the resulting authorship and license, and offer a content diff against the selected
source. A moderate steward edit publishes a new immutable QuestionRevision under the same QuestionId;
a validated full fork publishes a new QuestionId lineage with source attribution. The interface shows
the stable QuestionId and version history without exposing a hidden snapshot ID, and records
controlled-update impact separately from publication.

**Course appearance (deferred target architecture).** One focused Instructor surface will select a
closed, measured three-color biome or habitat theme and optionally upload one small centered
banner. The theme will be authoritative course-scoped state and apply through one route scope to
every student and Instructor page inside that course; it will never recolor global PLE pages,
authored scientific content, or semantic success/danger/correctness states. The banner will appear
only on the course entry page. Theme and banner changes will share one strong appearance revision
so a stale instructor tab preserves its local choices rather than overwriting another instructor.
This target is deferred: no mounted Instructor UI, durable Course Appearance Store or record,
current-pointer relation, PostgreSQL schema or migration, server route, authorization oracle,
Course Banner upload-promotion-cleanup lifecycle, or mounted editor exists. The separately scoped
`WN1-QM-PRESENTATION-COURSE-APPEARANCE-VIEW` package is reader-only: it provides no persistence,
current-pointer, route, authorization, upload-lifecycle, or editor implementation. The eventual
feature requires an approved dedicated plan that freezes its palette, object, RLS, image, API,
accessibility, and atomic work-package contracts before implementation.

**Gradebook.** Reads summary rows only, showing best and latest scores, completed Assignment Attempt count, and
last activity. A Student's Assignment Attempt history is a drill-down that loads on demand, so the default view stays
a summary query regardless of how many practice Assignment Attempts a class has accumulated. Rows show the
student's display name and never render a student UUID.

## Module catalog

The unit of owned work. Every module has one owner, one contract, the contracts it consumes, an
executable reference/test implementation where isolation is useful, and one independent
verification. A reference implementation is behavior-bearing test infrastructure, not release
substitution for a required production path.

| ID                         | Module                                                                   | Exposes                                                                                                                               | Consumes                                                                                | Reference/test implementation       | Independent verification                                                                                                                                                                                                                    |
| -------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MOD-QM                     | `question_model`                                                         | Types, capabilities, identity, Question Classifications                                                                               | none                                                                                    | n/a (root contract)                 | `cargo test`; `ts-rs` output compiles                                                                                                                                                                                                       |
| MOD-ID                     | Identity and lifecycle                                                   | Draft workspace identity, published `QuestionId`/`QuestionRevisionNumber`, lifecycle                                                  | MOD-QM                                                                                  | n/a                                 | Lifecycle tests; no published identity construction outside publish                                                                                                                                                                         |
| MOD-ACTIVITY               | Assignment Activity Rules                                                | Assignment Attempt lifecycle and independent policy rules                                                                             | MOD-QM                                                                                  | n/a                                 | Representative repeat-practice history preserves issued Assignment Attempts and summary behavior across policy combinations                                                                                                                 |
| MOD-STATE                  | Attempt state machine                                                    | `apply(state, event)`, completion within an Assignment Attempt                                                                        | MOD-QM, MOD-ACTIVITY                                                                    | n/a                                 | Every legal transition plus a rejected illegal one                                                                                                                                                                                          |
| MOD-TIME                   | Timing rules                                                             | `timer_verdict(...)` pure fn                                                                                                          | MOD-QM                                                                                  | n/a                                 | Table-driven grace and pause cases                                                                                                                                                                                                          |
| MOD-SCORE                  | Scoring and grade policies                                               | `score(...)`, summary projection                                                                                                      | MOD-QM, MOD-ACTIVITY                                                                    | n/a                                 | First/latest/highest agree with a hand-computed fixture                                                                                                                                                                                     |
| MOD-CAP                    | Capability validation                                                    | `validate_assignment_config -> Vec<Violation>`                                                                                        | MOD-QM                                                                                  | n/a                                 | Committed violation table                                                                                                                                                                                                                   |
| MOD-GEN                    | Question Variation generation                                            | `generate(question_seed, definition)`                                                                                                 | MOD-QM                                                                                  | n/a                                 | Question Seed parity (WP-C4)                                                                                                                                                                                                                |
| MOD-GRD                    | Grading (server-only)                                                    | `grade(question, response, key)` and typed PLE Question JSON private integrity                                                        | MOD-QM, MOD-STATE                                                                       | n/a                                 | Checker behavior tests; MOD-STO's opaque typed integrity use is server-only; absent from the `wasm32` closure (WP-C5)                                                                                                                       |
| MOD-OBJ                    | Object store                                                             | `ObjectStore` trait                                                                                                                   | MOD-ID                                                                                  | `MemoryObjectStore`                 | Conformance suite on memory, MinIO, S3                                                                                                                                                                                                      |
| MOD-STO                    | Persistence and RLS context                                              | `Store` trait                                                                                                                         | MOD-QM, MOD-ID, MOD-ACTIVITY, MOD-GRD (opaque PLE Question JSON private integrity only) | `MemoryStore`                       | Conformance suite on memory and PostgreSQL; cursor pagination only; no private material enters Wasm                                                                                                                                         |
| MOD-SCHEMA                 | Migrations, RLS policies, partitions                                     | Shared schema with exact relationship predicates                                                                                      | MOD-ID, MOD-ACTIVITY                                                                    | n/a                                 | Fresh apply; a missing authenticated session, foreign course, another AccountId, and revoked membership return zero rows                                                                                                                    |
| MOD-ADP-PLE                | PLE Question Backend                                                     | Algorithmic Question Types and strict PLE Question JSON compiler                                                                      | MOD-QM, MOD-GEN, MOD-GRD                                                                | n/a                                 | End-to-end generated Question Type; PLE Question JSON public/private split and reproducible hash                                                                                                                                            |
| MOD-ADP-WW                 | WeBWorK adapter                                                          | Adapter impl, renderer client, render cache                                                                                           | MOD-QM, MOD-OBJ                                                                         | Recorded renderer fixtures          | Approved immutable authored `which_hydrophobic-simple.pgml` RadioButtons fixture renders and grades; repeat seed cache hit; private topology, timeout, PLE API, and browser gates pass; broad OPL fixture-set compatibility is out of scope |
| MOD-ADP-QTI                | QTI adapter                                                              | Import pipeline, export                                                                                                               | MOD-QM, MOD-OBJ                                                                         | `MemoryObjectStore`                 | Hostile-ZIP fixture set rejected; unsupported features recorded                                                                                                                                                                             |
| MOD-ADP-H5P                | H5P adapter                                                              | Adapter impl, `serverGrading: false`                                                                                                  | MOD-QM                                                                                  | n/a                                 | Capability honesty test; import path to internal model                                                                                                                                                                                      |
| MOD-ADP-IMATHAS            | iMathAS adapter                                                          | Immutable Question Source snapshot, iMathAS Question Backend Session, iMathAS Result verification, iMathAS Render Cache, capabilities | MOD-QM, MOD-OBJ, MOD-STO, MOD-API-ASSIGNMENT-ATTEMPT                                    | Recorded, redacted iMathAS fixtures | Pinned seeded item renders and grades; replay, cache, outage, disclosure, and isolation gates                                                                                                                                               |
| MOD-EXPORT                 | Print model and writers                                                  | DOCX and PDF                                                                                                                          | MOD-QM                                                                                  | Fixture version                     | Each supported export path produces a valid document from one representative input; unexportable content is flagged before build                                                                                                            |
| MOD-WASM                   | WASM bridge                                                              | Typed exports                                                                                                                         | MOD-QM, MOD-STATE, MOD-TIME, MOD-GEN, MOD-CAP                                           | n/a                                 | Export allowlist; no `grading` in closure                                                                                                                                                                                                   |
| MOD-API-AUTH               | Auth and sessions                                                        | `/auth`                                                                                                                               | MOD-STO                                                                                 | `MemoryStore`                       | Login on one replica, proceed on another                                                                                                                                                                                                    |
| MOD-API-CAT                | Question Library routes                                                  | `/questions`, Question Classifications, publication                                                                                   | MOD-STO, MOD-ID, MOD-CAP                                                                | `MemoryStore`                       | Publish refuses on violations; drafts hold no Question ID; cursor paging                                                                                                                                                                    |
| MOD-API-COURSE             | Course routes                                                            | `/courses`, `/assignments`                                                                                                            | MOD-STO                                                                                 | `MemoryStore`                       | Assignments store exact `(question_id, revision_number)` pins                                                                                                                                                                               |
| MOD-API-ASSIGNMENT-ATTEMPT | Assignment Attempt, Question Attempt, submission, and grading routes     | `/assignment-attempts`, `/question-attempts`, `/submissions`, `/grading`                                                              | MOD-STO, MOD-ACTIVITY, MOD-STATE, MOD-TIME, MOD-GRD                                     | `MemoryStore`                       | DB timestamps; idempotent replay; summary updated transactionally; no key in any response                                                                                                                                                   |
| MOD-API-ASSET              | Asset delivery                                                           | `POST /api/assets/{id}`                                                                                                               | MOD-OBJ, MOD-STO                                                                        | `MemoryObjectStore`                 | Authorizes and logs before a bounded signed URL; only activated public Question Library assets bypass to CDN                                                                                                                                |
| MOD-WORKER                 | Jobs queue and worker pool                                               | Enqueue and drain                                                                                                                     | MOD-STO                                                                                 | `MemoryStore`                       | Two workers never claim one job; scales on queue depth                                                                                                                                                                                      |
| MOD-STATS                  | Anonymous question statistics                                            | Incremental aggregation, k-anonymity gate                                                                                             | MOD-ACTIVITY, MOD-STO                                                                   | `MemoryStore`                       | Aggregates match a hand-computed fixture; below-threshold cohorts suppressed; aggregates survive record deletion                                                                                                                            |
| MOD-RETENTION              | Retention lifecycle                                                      | Scheduled notify, archive, delete; configured course policy                                                                           | MOD-STO, MOD-OBJ, MOD-STATS                                                             | `MemoryStore`                       | Controlled-clock checks exercise configured notification, archive, and deletion stages; deletion removes records and bucket artifacts while Question Library content and statistics retain their declared state                             |
| MOD-CLIENT                 | Typed API client                                                         | TS client from generated types                                                                                                        | Generated types                                                                         | Mock handler set                    | Type tests; no `any`, no unchecked `as`                                                                                                                                                                                                     |
| MOD-UI-SHELL               | App shell, routing, session context, error boundaries, focus conventions | Route tree, boundaries, layout                                                                                                        | MOD-CLIENT, WP-C9                                                                       | Mock handlers                       | Representative registered routes resolve; a thrown render error leaves the shell usable. Route registration review is a one-time receipt.                                                                                                   |
| MOD-UI-COURSE              | Course shell and appearance settings                                     | Course-scoped three-color theme, entry banner, instructor appearance workflow                                                         | MOD-UI-SHELL, MOD-CLIENT, MOD-API-COURSE, MOD-OBJ                                       | Appearance mock repository          | Theme follows all course routes without global bleed; keyboard save/conflict flow; contrast and visual source_object_reference gates                                                                                                        |
| MOD-UI-WIDGETS             | Question Response Control set                                            | One component per response type, with local format validation                                                                         | MOD-WASM, WP-C9                                                                         | Reference widget                    | Each widget satisfies `docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md`, is label-announced, and flags invalid shape without issuing a request                                                                                                      |
| MOD-UI-RENDER              | Question renderer                                                        | Envelope-to-component mapping, asset resolution, math and figure alternatives                                                         | MOD-UI-WIDGETS                                                                          | Fixture envelopes                   | Representative supported block kinds render; sanitized markup renders without script execution; missing accessibility text surfaces as an authoring error. The supported-kind review is one-time evidence.                                  |
| MOD-UI-ATTEMPT             | Assignment Attempt loop                                                  | Submit, pending state, current student-disclosure display, timer, prefetch, retry                                                     | MOD-UI-RENDER, MOD-CLIENT                                                               | Mock handlers                       | Full mastery Assignment Attempt; long-history practice remains available; timer expiry; offline submit recovers; server-projected disclosure respected                                                                                      |
| MOD-UI-BROWSE              | Question Library browser                                                 | Virtualized cursor-paged list, facets, Question Details                                                                               | MOD-CLIENT                                                                              | Mock handlers                       | Cursor navigation requests only the next bounded page while scrolling; facet counts come from aggregates and recover after an empty or stale page                                                                                           |
| MOD-UI-EDITOR              | Draft and assignment editors                                             | Draft editing, WASM preview, policy controls, capability gating, publish flow                                                         | MOD-UI-RENDER, MOD-WASM                                                                 | Mock handlers                       | Preview generates a real variant per seed offline; a policy a backend cannot support marks the question and names the capability; publish shows the version diff                                                                            |
| MOD-UI-GRADEBOOK           | Gradebook                                                                | Summary-row views, Assignment Attempt-history drill-down                                                                              | MOD-CLIENT                                                                              | Mock handlers                       | Default view issues one summary query regardless of Assignment Attempt count                                                                                                                                                                |
| MOD-LTI                    | LTI Advantage                                                            | Launch and grade passback                                                                                                             | MOD-STO, MOD-API-AUTH                                                                   | Sandbox fixtures                    | Passback verified against an LMS sandbox                                                                                                                                                                                                    |
| MOD-DEPLOY                 | Containers and AWS                                                       | Compose, images, Fargate, RDS, buckets, CDN                                                                                           | all                                                                                     | n/a                                 | A declared workload-model exercise demonstrates independent replicas and records observed capacity, failures, and scaling decisions; numeric projections have no permanent gate authority                                                   |

Shared artifacts with exactly one owning module, so lanes never contend:

| Artifact                                         | Owner      |
| ------------------------------------------------ | ---------- |
| `crates/domain/tests/seed_vectors.json`          | MOD-GEN    |
| `tests/fixtures/published_question/` fixture set | MOD-QM     |
| `schemas/migrations/**`                          | MOD-SCHEMA |
| WASM export allowlist                            | MOD-WASM   |
| Mock API handler set                             | MOD-CLIENT |
| `containers/compose.yaml`                        | MOD-DEPLOY |

## Milestone plan

| M   | Title                    | Summary                                                                                    | Goal                                                                         |
| --- | ------------------------ | ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| M0  | Foundation and toolchain | Workspace, Solid build, containers, gates                                                  | Both toolchains green on a hello-path                                        |
| M1  | Contract freeze          | Every contract, reference backend, and conformance suite                                   | Six or more lanes start without coordinating                                 |
| M2  | Core lanes               | Domain, Assignment Attempts, grading boundary, storage, objects, PLE Question Backend, API | Parity, secrecy, and cross-course/cross-user authorization gates green       |
| M3  | Experience lanes         | Student and instructor UIs, worker pool, export                                            | Long Assignment Attempt history remains usable and correctly summarized      |
| M4  | Adapter lanes            | WeBWorK with render cache, QTI, H5P                                                        | Adapter behavior is green and a one-time source-boundary receipt is accepted |
| M5  | Integration hardening    | Cross-cutting E2E, isolation, hostile inputs, retention                                    | Every gate green together, not just per lane                                 |
| M6  | Platform and deploy      | LTI, analytics, AWS, autoscaling                                                           | Passback verified; capacity-model workload demonstrates independent replicas |

### Milestone: M0 foundation and toolchain

- Depends on: none.
- Deliverables: Cargo workspace with every crate present and compiling; Solid app rendering one route;
  `pipeline/build.mjs` producing `dist/main.js` plus the `.wasm` asset; compose bringing up `api`,
  `postgres`, and `minio`; template defects fixed; repository-owned `check_rust.sh` provides the Rust
  gate without modifying the vendored `check_codebase.sh`.
- Entry criteria: none.
- Exit criteria: `./check_codebase.sh`, `./check_rust.sh`, and `pytest tests/` green; the separate
  one-time local-stack acceptance yields `/health` 200 backed by a real `SELECT 1` and a MinIO bucket
  probe.
- Parallel-plan ready: no. Bootstrapping is serial; the workspace must compile before any lane starts.

### Milestone: M1 contract freeze

- Depends on: M0.
- Deliverables: MOD-QM types with generated TypeScript; MOD-ID identity; MOD-ACTIVITY Assignment Activity and policy model;
  trait signatures for MOD-OBJ, MOD-STO, and the adapter boundary; `MemoryStore` and
  `MemoryObjectStore`; both conformance suites; the approved serialization fixture set; narrow
  test-local fakes; the WASM
  export allowlist; the frontend architecture contract with `docs/SOLID_MODEL.md`,
  `docs/FRONTEND_ARCHITECTURE.md`, and one reference question response control; `docs/CONTRACTS.md`.
- Entry criteria: M0 exit criteria met.
- Exit criteria: each declared Question Library contract has compiling consumers and its executable reference
  backend where required; conformance suites pass against in-memory backends; generated TypeScript passes
  `tsc --noEmit`, ESLint, and Prettier unchanged and contains no answer-bearing type; a UI lane builds a
  screen against the WASM facade and generated client with no backend running; isolated protocol tests
  use only narrow test-local fakes; a
  reviewer examines the changed contract surfaces and their declared consumers for a contract gap.
- Parallel-plan ready: partly. MOD-QM is serial and first; MOD-ID, MOD-ACTIVITY, the reference-backend
  packages, and the frontend contract then run as lanes.

The frontend contract lands here rather than in M3 for the same reason the backend contracts do: the
two UI lanes are only independent once the route map, state model, facade signatures, and one reference
widget exist to build against.

### Milestone: M2 core lanes

- Depends on: M1.
- Deliverables: domain modules, Assignment Attempt and scoring model, grading boundary with both gates, real object
  storage, PostgreSQL store with RLS and partitions, PLE Question Backend including the PLE PLE Question JSON
  compiler and split persistence path, API route groups.
- Lanes: (1) MOD-STATE, MOD-TIME, MOD-SCORE, MOD-CAP; (2) MOD-GEN, MOD-ADP-PLE; (3) MOD-GRD,
  MOD-WASM; (4) MOD-OBJ; (5) MOD-SCHEMA, MOD-STO; (6) the five API modules; (7) MOD-CLIENT.
- Entry criteria: M1 exit criteria met.
- Exit criteria: seed parity green on both targets; WASM allowlist and dependency assertion green;
  conformance suites green against PostgreSQL and MinIO; a missing authenticated session, foreign course, another
  AccountId, and revoked membership return zero rows;
  the student-facing role cannot read any answer-key table; an in-progress Assignment Attempt resumes across restart
  and across replicas; a replayed submission returns the first result; every list endpoint uses a
  cursor; PLE Question JSON publication preserves the non-signable canonical source and binds grader-only
  material to the exact answer-free public model.
- Parallel-plan ready: yes. Seven lanes.

### Milestone: M3 experience lanes

- Depends on: M2 for live behavior; browser behavior uses the real stack, while browser-independent
  unit work may start after M1 with narrow test-local fakes.
- Deliverables: app shell and routing; the question response control set; the question renderer; the attempt loop
  with prefetch, timer, and server-projected student disclosure; Question Library browser; draft and assignment editors;
  gradebook; course appearance theme/banner capability and instructor settings; worker pool and jobs
  queue; print model with DOCX and PDF writers.
- Lanes: (1) MOD-UI-SHELL, then MOD-UI-WIDGETS and MOD-UI-RENDER; (2) MOD-UI-ATTEMPT;
  (3) MOD-UI-BROWSE, MOD-UI-EDITOR, MOD-UI-GRADEBOOK; (4) MOD-UI-COURSE after its closed
  object/Store/API contracts; (5) MOD-WORKER; (6) MOD-EXPORT.
- Entry criteria: M1 exit, including the frontend architecture contract, for browser-independent
  unit work;
  M2 exit for live integration.
- Exit criteria: a documented, configurable workload model drives a one-time server-side grading
  measurement, with its environment, workload shape, and observed distribution recorded for release
  capacity planning; end-to-end round trip is recorded alongside it for context; a browser network trace confirmed free of
  any answer or key; answer-format validation confirmed to resolve locally with no request issued; a
  student completes an assignment and starts another practice Assignment Attempt with fresh variants and a correct
  summary row after a varied retained history;
  publish refusal names the question and capability; a draft carries no Question Library number; a
  multi-worker exercise shows independent job claims; supported export variants render from one
  fixture; one instructor changes
  a course theme/banner under CAS and a student sees the theme across every course route without a
  banner outside the entry page or palette bleed into global pages.
- Parallel-plan ready: yes. Six lanes. The course appearance package supplies its own two-owner
  parallel limits and one-owner integration seams.

### Milestone: M4 adapter lanes

- Depends on: M1 for the adapter contract; M2 for MOD-OBJ and MOD-STO; the atomic
  draft-identity contract amendment below before its lane starts.
- Deliverables: WeBWorK adapter, renderer container, and deterministic render cache; QTI import
  pipeline and export; H5P adapter with honest capabilities and an import path; iMathAS adapter
  (with an iMathAS-compatible deployment) using immutable source snapshots, the
  `imathas_remote_grading_v1` profile, and a verified server-to-server iMathAS Result.
- Lanes: (1) MOD-ADP-WW; (2) MOD-ADP-QTI; (3) MOD-ADP-H5P; (4) MOD-ADP-IMATHAS after the
  draft-identity contract amendment. These lanes do not modify `crates/domain`.
- Entry criteria: M2 exit criteria met.
- Exit criteria: the immutable licensed authored
  `content/pilot/webwork/which_hydrophobic-simple.pgml` RadioButtons fixture renders and grades
  through the shared model; a repeat `(question_id, revision_number, seed)` is served from cache without touching the
  renderer; the renderer has no public endpoint, no PLE database access, enforced CPU, memory, and
  request-time limits, no SQL database or persistent renderer volume; its timeout degrades only
  WeBWorK questions; PLE
  API and browser-network gates prove no protected material crosses the boundary. Broad OPL fixture-set
  compatibility is outside this bounded fixture acceptance. The hostile-ZIP fixture set is fully rejected
  with actionable errors; unsupported QTI features are recorded rather than dropped; the original
  package is re-importable; H5P declares `serverGrading: false`; an iMathAS sandbox preview remains
  unversioned and private, while publication archives a checksum-pinned snapshot and profile before
  minting a durable version; iMathAS grades only through an authenticated, idempotent
  server-to-server exchange; a browser message is presentation/readiness only; and an iMathAS
  outage affects only that attempt. Adapter conformance tests protect the behavior boundary; a
  one-time source-diff receipt records that adapters leave `crates/domain` unchanged.
- Parallel-plan ready: yes. Three independent lanes immediately; iMathAS begins after its atomic
  draft-identity contract amendment and then proceeds independently.

#### Draft identity prerequisite and iMathAS lane

The persisted backend wire value is `imathas`; the product label is **iMathAS**. An iMathAS-compatible
deployment is not a second Question Backend. Before MOD-ADP-IMATHAS begins, land the following atomic
draft-identity refactor. It is a prerequisite for every adapter, not an iMathAS-lane repair, and no
adapter may consume an intermediate form:

- MOD-QM defines a private workspace-owned `DraftQuestionRevision` with workspace-only identity and no
  `QuestionId` or `QuestionRevisionNumber`; published-only definitions and references require both IDs.
- MOD-ID makes the lifecycle transition from validated draft to published content mint the full
  `QuestionRevisionReference`, stable QuestionId lineage, and immutable QuestionRevision only after all
  publication validation succeeds. A validated moderate steward edit mints a successor version in
  its existing lineage; a validated full fork mints a new lineage with fork authorship, source
  attribution, and a source-compatible license. A failed publication mints neither published
  reference nor version.
- MOD-STO and MOD-SCHEMA update the memory and PostgreSQL stores, migration and JSON payload
  boundaries so drafts store only their workspace identity and published rows store the immutable
  reference. Question Library and API publication paths use that transition rather than a draft-held version.
- MOD-CLIENT updates generated TypeScript and all direct browser/API consumers. MOD-QM regenerates
  the generated clients and published/draft fixtures through their owners, and conformance tests
  prove a sandbox draft is private and unversioned until successful publication.

This same patch must update the lifecycle, store conformance, Question Library/API fixtures, generated clients,
and Published Question Library records; it follows the frozen-contract change rule and blocks
MOD-ADP-IMATHAS until complete. Adapter behavior remains covered by its owning conformance tests; a
one-time source-boundary receipt records that the adapter adds no edits inside `crates/domain`.

After that prerequisite:

- Add `ImathasQuestionBackendBinding` and `QuestionBackend::Imathas`. A draft sandbox preview may retain
  only an unversioned, private iMathAS Item Reference. Publishing first fetches and validates the
  `imathas_remote_grading_v1` profile, then stores immutable source bytes with an `ObjectId` and SHA-256 and pins the
  integration profile. The prerequisite publication transition mints the applicable immutable
  version only after that work succeeds; a moderate steward update remains in its QuestionId lineage
  and a full fork creates a new lineage with hidden exact evidence.
- Keep iMathAS deployment endpoint, credentials, accepted origin, and egress policy in deployment configuration
  keyed by iMathAS Deployment Reference. Never persist or serialize an arbitrary iframe URL, launch JWT,
  answer, solution, remote session state, or credential as question source.
- Keep `ImathasQuestionBackendLaunch` separate from serializable question source. Its `launchUrl` is a
  non-secret, same-origin, session-authenticated attempt route or handle, never an opaque iMathAS
  launch capability. It contains no iMathAS bearer, token, credential, source locator, or
  correctness material, and none may enter URLs, browser history, logs, traces, or serializable
  source. The server keeps the iMathAS launch and correlation server-held or in a server-only
  HttpOnly session, and rechecks enrollment and attempt ownership on every route use. Browser
  messages may communicate validated presentation/readiness state only; they never grade.
- The server alone launches and grades: it checks attempt/enrollment ownership, holds correlation and
  idempotency keys, verifies iMathAS authentication, expiry, nonce, and attempt/version/seed
  correlation, then persists the first verified grade. A browser score or callback is never a
  fallback. Unsupported verification or deterministic seeding makes the requested graded feature
  unavailable rather than pretending capability.
- Source snapshots are answer-bearing objects, not CDN assets. Student launches, responses, and
  iMathAS transcripts are exact course/Student/attempt records under RLS and retention; immutable
  shared content stays outside course records. Cache keys include immutable version and seed, never
  Student or course data.
- iMathAS failures show a question-local retry/degraded state with saved editable response. They do
  not become incorrect grades, block adjacent questions, or weaken disclosure policy. Solutions are
  requested, sanitized, and returned only after server policy permits them.

Permanent behavior gates use recorded, redacted protocol fixtures and a test-local iMathAS protocol fake:
sandbox
publication refusal without a snapshot or durable IDs; immutable replay after iMathAS mutation;
deterministic render cache; forged or stale browser/iMathAS messages leaving attempts ungraded;
idempotent grade replay and cross-course isolation; copied launch URLs and disclosure traces free of source,
iMathAS launch material, secrets, answer, and unauthorized score; outage isolation; and capability
refusal for unsupported profiles. A dedicated
non-production iMathAS/MyOpenMath compatibility probe is a one-time release-readiness check, never a
permanent credentialed or network test.

### Milestone: M5 integration hardening

- Depends on: M3, M4.
- Deliverables: cross-cutting `tests/e2e/` suite; orphaned-object Storage Checks; MOD-STATS
  incremental aggregation with the k-anonymity gate; MOD-RETENTION lifecycle with configured course
  policy; asynchronous analytics; `docs/SECURITY_MODEL.md`, `docs/RETENTION_POLICY.md`.
- Lanes: (1) MOD-STATS; (2) MOD-RETENTION; (3) cross-cutting E2E, owned by `integrator`.
- Entry criteria: M3 and M4 exit criteria met.
- Exit criteria: package-local lanes are green before final Validation; controlled-clock expiry,
  cross-course/cross-user authorization, answer-key grants, object round trip, partition pruning under the documented
  workload model, and
  renderer-outage degradation all proven together; a course deletion test proves student records and
  `student-records` bucket artifacts are gone while Question Library content, Instructor drafts, and anonymous
  statistics remain; a below-threshold cohort's statistics are proven suppressed.
- Parallel-plan ready: no. This milestone exists to find interactions that per-lane green results
  hide.

### Milestone: M6 platform and deploy

- Depends on: M5.
- Deliverables: LTI Advantage passback; server-side aggregate views reading summaries and anonymous
  statistics with no client analytics; OpenTofu AWS deployment (Fargate API, worker, and dedicated
  public-asset publisher; RDS PostgreSQL; four storage domains; ALB, CloudFront, Secrets Manager,
  KMS, CloudWatch, and WAF); backup and retention policy; burst load test; FERPA control checklist
  with evidence. The renderer remains externally managed and its feature stays disabled until its
  production identity, network isolation, and security attestation are accepted.
- Lanes: (1) MOD-LTI; (2) aggregate observability; (3) MOD-DEPLOY.
- Entry criteria: M5 exit criteria met.
- Exit criteria: passback verified against an LMS sandbox; encryption at rest and in transit
  demonstrated; restore-from-backup is exercised with the recovery evidence recorded; a controlled
  class-start workload uses the documented capacity model to demonstrate multi-replica independence
  and records observations for deployment sizing; the connected-term journey below demonstrates
  the named semantic transitions on the same live product model.
- Parallel-plan ready: yes. Three lanes.

#### M6 connected-term journey

M6 composes the already-gated capabilities in one smallest-useful live narrative. The journey uses
Elena Instructor and the seeded Mary Student record for student delivery and inspection. Elena
and Morgan passkey enrollment, sign-out, and sign-in remain independent suite-owned scenarios; this
journey starts from their ordinary authenticated sessions. Assisted tagging participates only when
`WP-INST-D3` has shipped; the core journey remains complete with human Question Classification and Question Folder actions.

1. **Discover.** Elena searches the Question Library by concept, filters to safe evidence, and
   opens Question Details.
2. **Organize.** Elena Stars the selected Questions and places them in a named Question Folder. The
   same live selection is available to the assignment picker. If `WP-INST-D3` is accepted, she may
   review and confirm a proposed tag with recorded classification evidence; otherwise human Question Classification is the
   demonstrated path.
3. **Reuse.** Elena creates or revises a Blueprint Course with one fixed question and one pool
   definition. The fixed member remains selected; the pool records its draw rule and delivery order
   without becoming a student assignment or grade.
4. **Instantiate.** Elena instantiates that reusable definition into an ordinary Fall teaching term
   with its start date and IANA time zone. The destination receives teaching-owned definitions and no
   student records.
5. **Preview and accommodate.** Elena previews resolved schedule dates, then grants the enrolled
   Mary an accommodation. The preview shows the effective window and its source before she saves
   the assignment.
6. **Deliver.** Mary enters the released Assignment through the ordinary student workflow,
   receives fixed and policy-selected pool items bound to the issued Assignment Attempt, submits, and receives the
   deterministic grade, immutable receipt, and permitted disclosure. Elena inspects the same audited
   student work through the Instructor surface.
7. **Recover and recalculate.** A deterministic grader exception for one issued item routes to
   Elena's operation view. After the bounded correction, she requests the generation-fenced
   recalculation and observes the refreshed course total without changing the original receipt.
8. **Analyze and improve.** Elena opens course-local item analysis, inspects the affected student
   evidence and usage context, and publishes either a linked immutable successor or a validated fork
   according to the change class. She records the controlled-update decision; future teaching can use
   the selected improvement while the issued Assignment Attempt remains pinned to its original evidence.
9. **Rollover.** Elena previews and creates the next-term rollover. The manifest carries reusable
   teaching definitions and improvement notes while excluding roster membership, accommodations,
   student work, attempts, grades, and retention state.
10. **Shift and improve.** Elena previews and applies the next term's date shift, resolves any
    daylight-saving correction, and reviews the linked replacement's improvement evidence alongside
    the source question. The receipt records the shifted schedule and the decision for later review.

Each step asserts the semantic transition and its visible result; it does not require a fixed
collection size, source_object_reference total, screenshot match, or timing target. The named student state is the
smallest live state that demonstrates the transition, while aggregate/item-analysis evidence uses
the configured privacy threshold and existing seeded contributions where required.

## Work packages

M0 and M1 remain below as accepted bootstrap history. The complete remaining M2-M6 closure packages,
including owners, files, behavior, success conditions, and validation, are WP-RC1 through WP-RC12 in
`docs/active_plans/active/release_completion_plan.md`; no milestone-entry expansion
or implementer-authored specification is required.

### M0 packages

| ID    | Title                                 | Owner          | Depends on   | Acceptance                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ----- | ------------------------------------- | -------------- | ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| WP-F1 | Create the Cargo workspace            | `expert_coder` | none         | Every crate in the boundary table exists and compiles empty; current edition; `Cargo.lock` committed; the forbidden-dependency column encoded as real absences, not comments                                                                                                                                                                                                                                                                                                                               |
| WP-F2 | Add the WASM build path               | `expert_coder` | WP-F1        | `wasm-bindgen` output into a gitignored staging dir; a trivial export callable from Node; current stable toolchain and version-matched runner                                                                                                                                                                                                                                                                                                                                                              |
| WP-F3 | Stand up Solid and the build pipeline | `expert_coder` | WP-F2        | `build_github_pages.sh` delegates to `node pipeline/build.mjs` with `esbuild-plugin-solid` and copies the `.wasm`; `tsconfig.json` gains `"jsx": "preserve"`, `"jsxImportSource": "solid-js"`, and an `exclude` for `OTHER_REPOS` and `target`; `src/log.ts` exists so no `console` appears in `src/`; placeholders filled matching `VERSION`; `clean` points at `devel/dist_clean.sh`                                                                                                                     |
| WP-F4 | Containerize api, postgres, minio     | `expert_coder` | WP-F1        | `containers/Containerfile.api` builds a multi-stage slim image under `podman build`; `containers/compose.yaml` brings up current stable PostgreSQL and MinIO with named volumes and creates the four named storage domains; `/health` returns 200 only after exact migration/checksum compatibility verification and a bucket probe; credentials arrive at run time from the environment; `docs/LOCAL_STACK_OPERATIONS.md` records the commands and `docs/MACOS_PODMAN.md` records the macOS machine setup |
| WP-F5 | Add the repository-owned Rust gate    | `coder`        | WP-F1        | Root `check_rust.sh` owns contract generation, fixture verification, format, check, strict Clippy, native/all-feature tests and doctests, and the Wasm target check; the vendored `check_codebase.sh` remains untouched and Cargo absence fails with an actionable message                                                                                                                                                                                                                                 |
| WP-F6 | Foundation documentation              | `coder`        | WP-F3, WP-F4 | README first paragraph pure prose under 250 chars passing `tests/test_readme_first_paragraph.py`; `docs/CODE_ARCHITECTURE.md` carries the container, boundary, bucket, and crate tables; `pytest tests/` green                                                                                                                                                                                                                                                                                             |

### M1 contract-freeze packages

#### Work package: WP-C1 define the Question Model and Question Classification

- Owner: `architect`. Module: MOD-QM. Depends on: WP-F1.
- Touch points: `crates/question_model/src/`, `docs/QUESTION_MODEL.md`.
- Acceptance criteria: covers the spec's `QuestionRevision` fields; `QuestionBackendCapabilities` carries all
  eight flags; response and grading shapes are enums whose invalid combinations do not compile; tags,
  Question Classification, and licensing types included as shared-content data; **no answer-bearing type defined
  here**; every public item documented per `docs/RUST_STYLE.md` section 13; `ts-rs` derives on every
  boundary type; the public Question ID follows `docs/QUESTION_ID_SPEC.md`, is random rather than
  sequential, and internal UUIDs never become browser-facing question identities.
- Evidence or review: `reviewer` confirms no capability is a bare `bool` that two call sites must
  re-check, per `docs/RUST_STYLE.md` section 9.
- Next dependency: WP-C2 and WP-C3 consume this accepted package.

#### Work package: WP-C2 define identity and lifecycle

- Owner: `architect`. Module: MOD-ID. Depends on: WP-C1.
- Touch points: `crates/question_model/src/identity.rs`,
  `crates/question_model/src/question_library.rs`, `docs/QUESTION_ID_SPEC.md`,
  `docs/IDENTITY_CONTRACTS.md`, and `docs/QUESTION_MODEL.md`.
- Acceptance criteria: `WorkspaceId`, `QuestionId`, and `QuestionRevisionNumber` are distinct branded
  types that cannot substitute for one another; publication creates a stable Question ID and Version
  Number 1; each accepted same-lineage change advances the positive monotonic Version Number; a fork
  creates a new Question ID; and `QuestionRevisionReference` carries the exact pair. Question
  Publication Requirements, Question Publication Validation, Question Publication Issues, Question
  Publication Event, and Question Revision Availability remain separate contracts.
- Next dependency: WP-C3 and WP-C4 consume this accepted package.

#### Work package: WP-C3 define the Assignment Activity, policy, and summary model

- Owner: `architect`. Module: MOD-ACTIVITY. Depends on: WP-C2.
- Touch points: `crates/question_model/src/student_work.rs`, `crates/domain/src/assignment_activity.rs`,
  `docs/STUDENT_WORK_RECORDS.md`.
- Acceptance criteria: Student Record, Assignment Attempt, and Question Attempt as distinct types; the four
  policies (completion requirement, grade policy, continued practice, variation policy) are independent enums
  that compose freely; completion within an Assignment Attempt is a derivation with no stored boolean; the summary
  projection is a pure function of an Assignment Activity transition so the Store can apply it transactionally;
  compact behavior tests cover varied completion and grade-policy histories against hand-computed outcomes
  without making an arbitrary Assignment Attempt count part of the contract.
- Evidence or review: the transition examples are the source_object_reference a reviewer reads because they encode
  the owner's observed student behavior as requirements.
- Next dependency: WP-C4 and WP-C5 consume this accepted package.

#### Work package: WP-C4 freeze the store and object contracts with reference backends

- Owner: `expert_coder`. Modules: MOD-STO, MOD-OBJ (contract portion). Depends on: WP-C3.
- Touch points: `crates/learning-data-access/src/{lib,in_memory}.rs`, `crates/objects/src/{lib,in_memory}.rs`, both
  conformance suites.
- Acceptance criteria: `Store` covers every entity, exposes cursor pagination only with no `OFFSET`
  parameter anywhere in the trait, and carries explicit `AuthenticatedSession` plus typed target scopes that
  cannot be defaulted; `ObjectStore` exposes `put`, `get`, `delete`, `signed_url` with keys built only from IDs
  and versions and no caller-supplied key; checksums computed on write and verified on read; both
  memory backends pass conformance suites the PostgreSQL and S3 backends will later run unchanged; no
  SQL or AWS type leaks through either trait.
- Evidence or review: the conformance suites are the deliverable, because they are the contract every
  later lane is held to.
- Next dependency: WP-C5 and M2 lanes 4 and 5 consume this accepted package.

#### Work package: WP-C5 commit the seed vector table and parity harness

- Owner: `tester`. Module: MOD-GEN (verification). Depends on: WP-C1.
- Touch points: `crates/domain/tests/seed_vectors.json`, `crates/domain/tests/test_determinism.rs`,
  `crates/wasm/tests/test_determinism_wasm.rs`, `docs/DETERMINISM_CONTRACT.md`.
- Acceptance criteria: a compact vector table covers every generator and materially distinct branch
  of its parameter space; each entry records its expected output hash, but no test asserts a fixture set
  length; the same assertions run under `cargo test` natively and
  `wasm-bindgen-test` in headless Chromium; a failure names the first divergent seed; the contract
  states that `rand_chacha::ChaCha20Rng` is used because its algorithm carries a stability guarantee
  that `StdRng` does not, that `BTreeMap` is used wherever iteration order reaches output, and that
  exact equality is the requirement here because the render cache and reproducibility record are keyed
  on it.
- Evidence or review: both command outputs in the tracker. This gate blocks every generation-dependent
  lane and underwrites both the parameter-hash storage decision and the render cache.
- Next dependency: WP-C6 consumes this accepted package.

#### Work package: WP-C6 establish and prove the grading boundary

- Owner: `expert_coder`. Modules: MOD-GRD, MOD-WASM (boundary). Depends on: WP-C1.
- Touch points: `crates/grading/src/lib.rs`, `crates/wasm/src/lib.rs`,
  `tests/e2e/e2e_wasm_export_allowlist.mjs`, `tests/test_crate_boundaries.py`,
  `docs/SECURITY_MODEL.md`.
- Acceptance criteria: `crates/grading` holds the answer-bearing surface; answer _format_ validation,
  needing no key, stays in `crates/domain` so the browser can call it; the `.wasm` export list is
  compared against a committed allowlist so any new export fails the gate until deliberately added; a
  second check asserts `crates/grading` is absent from the `wasm32` dependency closure. The dependency
  boundary runs in the fast architecture lane; the compiled-export inspection remains the explicit
  non-browser E2E source_object_reference gate. The document states which side new code belongs on.
- Evidence or review: the allowlist diff is what a reviewer reads. This makes "answers never reach the
  browser" checkable rather than aspirational.
- Next dependency: M2 lane 3 consumes this accepted package.

#### Work package: WP-C7 build approved serialization fixtures and narrow test-local fakes

- Owner: `coder`. Modules: MOD-QM (fixtures), MOD-CLIENT (serialization tests). Depends on: WP-C3.
- Touch points: `tests/fixtures/published_question/` and test-local fixture builders.
- Acceptance criteria: keep only the explicitly approved published-Question cross-layer fixture set whose
  production serialization is itself under test. Other examples are inline or generated by typed
  builders. Narrow local fakes may supply isolated protocol responses; there is no mock browser
  application or mock transport in the shipped runtime graph. Fixture counts and complete route-name
  inventories are one-time implementation evidence, not permanent assertions.
- Evidence or review: browser-independent decoder, serialization, and failure-mapping tests pass with
  literal fixtures or narrow local fakes; the real browser suite owns visible product behavior.
- Next dependency: M3 lanes 1 and 2 consume this accepted package.

#### Work package: WP-C9 freeze the frontend architecture contract

- Owner: `architect`, with `solid-js-expert` and `ui-ux-engineer` guidance. Depends on: WP-C1, WP-C7.
- Touch points: `docs/SOLID_MODEL.md`, `docs/FRONTEND_ARCHITECTURE.md`, `src/routes.ts`,
  `src/wasm/index.ts` facade signature, `src/api/` client shape, one reference question response control.
- Acceptance criteria: the route map, reactivity contract, client architecture table, persistence
  boundaries, security rules, focus and error conventions, and the validation strategy from the browser
  interface design section are recorded as the frontend's frozen contract; the WASM facade and generated
  client signatures exist so both UI lanes compile against them; one question response control is implemented end
  to end as the pattern the remaining widgets follow; the accessibility baseline is stated as testable
  conditions rather than aspirations; a test asserts the generated client surface contains no
  answer-bearing type.
- Evidence or review: narrow browser-independent tests can exercise the facade and generated client
  with no backend running. The real browser suite owns connected UI behavior; this document keeps the
  client boundary independently verifiable.
- Next dependency: M3 UI lanes consume this accepted package.

#### Work package: WP-C8 write the contract register

- Owner: `architect`. Depends on: WP-C1 through WP-C7, WP-C9.
- Touch points: `docs/CONTRACTS.md`.
- Acceptance criteria: one row per catalog module naming its contract file, owner, consumers, and
  executable reference/test implementation;
  a stated rule that changing a frozen contract requires updating this file and every consumer lane in
  the same patch.
- Next dependency: M2 dispatch consumes the accepted contract register across seven lanes.

### M3 course appearance package

#### Work package: WP-M3-COURSE-APPEARANCE add course themes and banners

- Owner: historical owner record only. The `WP-CA1` through `WP-CA7/WP-RC1` and `WP-RC2` receipts
  are retained, but the pre-production SD1 schema reset removed their durable product substrate.
- Current state: the Course Appearance Store and retained record, revision/current-pointer schema
  relation, PostgreSQL migration, server route, authorization oracle, Course Banner Upload
  promotion/cleanup, and mounted Instructor editor are deferred and incomplete. No historical
  receipt authorizes an acceptance claim for those absent capabilities.
- Separate terminology scope: `WN1-QM-PRESENTATION-COURSE-APPEARANCE-VIEW` owns only the already
  existing reader projection and its Question Model/public facade, generated declaration, strict
  browser decoder, reader/client consumers, focused fixtures/tests, and affected documentation. It
  neither restores nor accepts the deferred durable feature.
- Acceptance criteria: retain the documented supported theme variants, including the `woodland` to
  `forest` consolidation; store only a closed theme ID and one revisioned current banner relation; apply exactly canvas, secondary, and
  accent theme colors through one course scope; show the centered banner only on the course entry
  page; preserve global and semantic status colors; prove CAS recovery, forced RLS, current-pointer
  authorization, object lifecycle, 5.5:1 text contrast, keyboard behavior, responsive rendering, and
  absence of grading/answer/object-key data in browser contracts.
- Evidence or review: focused Rust/Node/Playwright gates, disposable PostgreSQL and MinIO oracles,
  semantic review of representative supported variants with palette metrics, and independent PASS
  with no P0/P1 finding.
- Next dependency: a future durable Course Appearance package must re-establish the deferred
  persistence, authorization, promotion/cleanup, and mounted-browser boundaries before it can run
  the acceptance criteria or evidence gates below. The current reader package remains separate.

### M3 PLE Question JSON type evolution package

#### Work package: WP-M3-PLE-QUESTION-JSON-TYPES complete all PLE Question JSON Types

- Owner: `architect` coordinates the Question Type closeout in
  `docs/active_plans/active/ple_question_json_schema_evolution_plan.md`.
  Depends on: accepted WP-M3-COURSE-APPEARANCE, the secure student-payload package, and
  the existing PLE Question JSON, grading, object, Store, schema, server, client, and frontend contracts.
- Touch points: closed PLE Question JSON schema version 2 source/compiler; public/private
  compilation; Question Type-specific response/checker types; source-to-object bindings; persistence, author
  editors, student widgets, live evidence, and durable documentation.
- Current implementation: the v2-only source/runtime core covers MC, MA, FIB, MULTI-FIB, NUM,
  MATCH, ORDER, and HOTSPOT.
- Acceptance criteria: keep answers and optional feedback protected; complete Question Type-specific visual
  authoring and the Memory/PostgreSQL/object-store paths; prove accessible author/student flows,
  immutable publication, forced RLS, asset lifecycle, correct/incorrect grading, cleanup, and no
  browser/Wasm answer association.
- Evidence or review: focused Rust/Node/Playwright gates, disposable PostgreSQL/object-store oracles,
  the full repository gate, and independent PASS with no remaining P0/P1 finding.
- Next dependency: WP-RC5 publishes the exact Chapter 1 content after MATCH and completes the other
  Question Type work packages in their frozen dependency order.

## Acceptance criteria and gates

- Per-patch gate: `./check_codebase.sh` green for its vendored TypeScript/Node lane;
  `./check_rust.sh` green for Rust/Cargo/Wasm; `pytest tests/` green;
  `docs/CHANGELOG.md` updated in the same patch.
- Contract gate: changing a frozen contract requires the same patch to update `docs/CONTRACTS.md`,
  every production consumer, and every executable reference/test implementation. A contract change
  landing without its consumers is a blocking finding.
- Determinism gate: WP-C5 parity green on both targets. Blocks every generation-dependent lane.
- Secrecy gate: WP-C6 allowlist and dependency assertions green, plus the M3 network trace. A red
  secrecy gate is a release blocker with no workaround.
- Authorization isolation gate: from M2, a missing authenticated session, a foreign course, another AccountId, or
  revoked membership returns zero rows and the Student-facing role cannot read any answer-key table.
- Scale gate: typed bounded-page Store contracts and behavior tests remain green. A one-time source and
  query-plan review verifies that new query paths use stable cursor predicates; a repository-wide
  lexical SQL scanner is not retained as a permanent pytest.
- Integration gate: each behavior-owned `tests/e2e/` and `./run_playwright_tests.sh` lane is green
  with its declared disposable stack. The final material-tree Validation runs the complete required
  suite after those connected lanes pass.
- Course appearance gate: the frozen WP-M3-COURSE-APPEARANCE contract, real-role RLS/current-pointer
  oracle, built-browser workflow, computed contrast metrics, and semantic supported-variant review
  are green before the appearance capability may be called complete.
- Performance evidence: record server-side grading, issue, and browse measurements during a named
  release or load investigation. Treat the result as one-time environment evidence until an actual
  user-facing SLO and reproducible benchmark environment exist; do not make a percentage of a local
  baseline a permanent test.
- Independent review gate: each lane reviewed by a `reviewer` that did not write it, using
  `audit-code-reviewer` before milestone exit.

## Test and verification strategy

- `cargo test --workspace`: domain rules, transitions, Assignment Attempt and policy combinations, timing and
  scoring behavior, identity lifecycle, and conformance suites against in-memory backends. Fast, no
  container.
- `pytest tests/`: repo hygiene and durable architecture boundaries only. It performs no real CLI
  round trips, network work, source-fragment inventories, repository-wide query parsing, or assertions
  on collection sizes, dates, tunable constants, and file layout.
- `node --import tsx --test tests/test_*.mjs`: generated-type freshness, API
  client serialization/decoder behavior, and strict transport shapes.
- `tests/playwright/`: mastery loop, a post-completion practice Assignment Attempt, timer behavior, publish refusal,
  and the network trace proving no answer crosses the wire. Timing measurements are diagnostic output,
  not pass/fail assertions.
- `tests/e2e/`: container-dependent checks -- restart durability, replica independence, clock-skew
  invariance, submission replay, migration application, RLS cross-course/cross-user isolation, answer-key
  grants, object round trip, partition selection under the declared workload model, render cache hit,
  hostile-ZIP handling, worker queue concurrency, renderer-outage degradation. Excluded from pytest by
  the existing `collect_ignore`.

Failure semantics: a red per-patch gate blocks the patch. A red determinism, secrecy, isolation,
contract, or scale gate blocks the milestone and triggers design review rather than a workaround.

## Risk register

| Risk                                                           | Impact                                                                          | Trigger                                                                                                                                      | Owner            | Mitigation                                                                                                                                                                                                                                                                                                                                                                                           |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| An answer or key reaches the browser                           | Assessment integrity lost, silent until exploited                               | A new WASM export, or grading code moved into `domain`                                                                                       | `expert_coder`   | `grading` absent from the `wasm32` closure; export allowlist gate; M3 network trace (WP-C6)                                                                                                                                                                                                                                                                                                          |
| RLS is bypassed, unset, or outlives membership                 | Cross-course, cross-account, or revoked-Student exposure of educational records | Application connects as a bypassing role, authenticated Account context comes from client input, or a revoked student uses stale identifiers | `expert_coder`   | `FORCE ROW LEVEL SECURITY`; non-superuser role; context from authenticated session only; account-and-relationship-scoped Store reads/mutations lock and recheck active Student membership; foreign-course, another-AccountId, and revocation-race tests on every gate                                                                                                                                |
| A frozen contract turns out incomplete                         | Parallel lanes stall or diverge                                                 | A lane finds a missing trait method mid-flight                                                                                               | `architect`      | Conformance suites ship with contracts in M1; the contract gate updates declared consumers in the same patch; one-time architecture review examines each changed contract surface                                                                                                                                                                                                                    |
| Native and wasm32 generation diverge                           | Historical attempts not reproducible; render cache serves wrong content         | Parity mismatch                                                                                                                              | `tester`         | Ban known causes up front; measure before dependent lanes start; replace the primitive rather than special-case the platform (WP-C5)                                                                                                                                                                                                                                                                 |
| Attempt tables outgrow the design                              | Slow gradebook, painful migrations                                              | Observed workload approaches a configured storage or query budget                                                                            | `expert_coder`   | The documented capacity model sets partition, summary, and retention parameters; grade reads use summaries; one-time query-plan and workload review validates the chosen configuration                                                                                                                                                                                                               |
| Grade computed by scanning history                             | Course pages time out at scale                                                  | A convenient aggregate query in a page path                                                                                                  | `expert_coder`   | Summary row is the only grade source; review rejects any aggregate over `question_attempt` in a request path                                                                                                                                                                                                                                                                                         |
| Database bloat from payloads in operational tables             | Slow backups, restores, replication                                             | A payload exceeds its documented operational-storage budget                                                                                  | `expert_coder`   | Role-based split; configured payload ceilings refuse oversized writes; archival source and binary data use typed object storage; hot and cold records remain separate                                                                                                                                                                                                                                |
| WeBWorK renderer saturates                                     | Timed questions fail to load under burst                                        | Many students on WeBWorK questions at once                                                                                                   | `expert_coder`   | Deterministic render cache; prefetch; worker pool autoscaled on queue depth, latency, CPU, and timeout rate                                                                                                                                                                                                                                                                                          |
| iMathAS Question Backend callback or retry accepted as a grade | Assessment integrity or cross-course isolation lost                             | Browser message, stale launch, an unverifiable backend response, or an ambiguous failed launch POST reaches grading                          | `expert_coder`   | iMathAS browser messages are presentation-only; the same-origin launch is POST-only; a lease-bound dispatch marker is committed before backend contact and blocks retry, grading, new launch, and finalization after an indeterminate outcome; server-held correlation/idempotency and authenticated server-to-server verification have forged-message, cross-course, expiry, and crash-window gates |
| Malicious archive during QTI import                            | Remote code execution or disk exhaustion                                        | A crafted ZIP uploaded                                                                                                                       | `expert_coder`   | Import in the worker; size, expanded-size, and file-count limits; path and symlink rejection; media sniffing; never serve from an extracted path; hostile fixture-set test                                                                                                                                                                                                                           |
| Course banner exhausts image processing                        | Availability failure or active-content exposure                                 | Oversized decoded raster, SVG, animation, malformed codec input                                                                              | `expert_coder`   | Pre-read byte cap; decoded-pixel cap; JPEG/PNG/WebP raster allowlist; metadata-stripping normalization; hostile-image tests                                                                                                                                                                                                                                                                          |
| Course theme bleeds across route scope                         | Wrong course identity or unreadable global/status UI                            | Prior course variables remain after navigation                                                                                               | `ui-ux-engineer` | One course-subtree provider; cross-course/global cleanup tests; computed rendered-pair contrast and contact-sheet review                                                                                                                                                                                                                                                                             |
| Orphaned objects accumulate                                    | Storage cost and retention drift                                                | Deleted records leaving objects behind                                                                                                       | `expert_coder`   | Reconciliation job comparing object records to bucket inventory; lifecycle rules; M5 deliverable                                                                                                                                                                                                                                                                                                     |
| Small-cohort statistics re-identify a student                  | Privacy failure disguised as an anonymous aggregate                             | A question attempted by one or two students publishes its statistics                                                                         | `architect`      | k-anonymity threshold (default 5) gates publication; suppression test in M5 exit                                                                                                                                                                                                                                                                                                                     |
| Statistics lost when records are deleted                       | The library stops learning, and deletion becomes something instructors avoid    | Statistics computed on demand from attempt history                                                                                           | `expert_coder`   | Incremental or scheduled aggregation while records exist; discrimination index computed before deletion; MOD-STATS ordered before MOD-RETENTION                                                                                                                                                                                                                                                      |
| Retention deletes reusable content                             | Instructors lose authored work and stop trusting the system                     | A deletion path following assignment references into shared content                                                                          | `expert_coder`   | Deletion is scoped to exact course/Student records by construction; the M5 deletion test asserts Question Library content and private drafts survive                                                                                                                                                                                                                                                 |
| Signed URL leakage                                             | Educational records exposed                                                     | A URL is shared, logged, or used after its configured expiry                                                                                 | `expert_coder`   | The signed-link configuration supplies a short-lived expiry appropriate to its storage domain; controlled-clock tests prove issue, valid use, expiry refusal, and logged access                                                                                                                                                                                                                      |
| Draft problems leak into shared content                        | The exact ADAPT failure this design exists to avoid                             | A code path minting `QuestionId` outside publish                                                                                             | `architect`      | Keep the `QuestionId` constructor private to the typed publish transition; durable behavior tests cover draft, publish, replay, and replacement outcomes, while a one-time source review receipts every construction path and confirms no alternate public boundary                                                                                                                                  |
| Parallel lanes collide on a shared source_object_reference     | Merge conflicts and lost work                                                   | Two lanes editing migrations or the seed table                                                                                               | `integrator`     | One owning module per shared source_object_reference, tabulated in the catalog                                                                                                                                                                                                                                                                                                                       |
| Scope creep toward ADAPT parity                                | Version 1 never ships                                                           | Requests for rubrics, Adaptive Question Support, discussions                                                                                 | `architect`      | Binary out-of-scope ledger in the release-completion plan                                                                                                                                                                                                                                                                                                                                            |
| Plan drifts from implementation                                | Reviews check the wrong thing                                                   | Package work outpacing the tracker                                                                                                           | `architect`      | Release-completion tracker updated at every WP-RC exit                                                                                                                                                                                                                                                                                                                                               |

## Rollout and release checklist

- [ ] M0 through M4 run only on `podman compose` with MinIO; no cloud resources provisioned.
- [x] The maintained Compose E2E exercises multi-replica request independence, so replica assumptions
      are evidenced before deployment sizing.
- [ ] RDS PostgreSQL with KMS encryption at rest, automated backups, and point-in-time recovery.
- [ ] PostgreSQL in private subnets; TLS with certificate verification on every hop.
- [ ] Application role is non-superuser and cannot bypass RLS; verified in the deployed environment,
      not only in tests.
- [ ] Four named storage domains, each encrypted with its own KMS policy and lifecycle: `public-assets`,
      `private-content`, `student-records`, and `temp-processing`. `public-assets` is readable only
      through the tag-gated CDN origin and writable only by the dedicated publisher; the other three
      have no public access.
- [ ] Secrets in Secrets Manager; no credential in any image layer or in git.
- [ ] Fargate autoscaling: `api` on request count with minimum two tasks; `worker` and the dedicated
      public-asset publisher use queue/lease work. The externally managed renderer has no activation
      path until its attested private-network, CPU/memory/request-limit, image-identity, and protocol
      evidence is accepted.
- [ ] Class-start workload review run from the documented capacity model; deployment-sizing observations
      recorded.
- [ ] Restore-from-backup exercised; the recovery evidence and environment are recorded.
- [ ] FERPA control checklist completed with evidence per control; retention and deletion implemented
      for `student-records` and render traces.
- [ ] Retention default configured to the privacy-preserving value, with the configured course-policy
      override documented and one non-default course policy exercised.
- [ ] A real course deletion exercised end to end: records and bucket artifacts gone, Question Library content
      and anonymous statistics intact, and the result recorded.
- [ ] Course appearance acceptance completed: real-role RLS/current-pointer oracle, centered
      entry-banner lifecycle, all-route theme scope, semantic supported-variant review, and measured contrast.
- [ ] `devel/make_release.py` run for the first tagged release after WP-RC12 is green.

## Documentation close-out requirements

- Active plan and tracker: update
  `docs/active_plans/active/release_completion_plan.md` at every WP-RC exit and move completed
  companion plans to `docs/archive/` with `git mv` only after their acceptance gates pass.
- `docs/CHANGELOG.md`: one entry per patch under the canonical section headings, recording key
  implementation choices and failures so the log stays a learning record.
- New durable docs, each owned by the work package creating it: `docs/CONTRACTS.md`,
  `docs/CODE_ARCHITECTURE.md`, `docs/FILE_STRUCTURE.md`, `docs/INSTALL.md`, `docs/USAGE.md`,
  `docs/LOCAL_STACK_OPERATIONS.md`, `docs/MACOS_PODMAN.md`, `docs/QUESTION_MODEL.md`,
  `docs/STUDENT_WORK_RECORDS.md`,
  `docs/QUESTION_ID_SPEC.md`, `docs/IDENTITY_CONTRACTS.md`, `docs/OBJECT_STORAGE.md`,
  `docs/DATABASE_AUTHORIZATION.md`,
  `docs/DETERMINISM_CONTRACT.md`, `docs/ADAPTER_DEVELOPMENT.md`, `docs/SECURITY_MODEL.md`,
  `docs/RETENTION_POLICY.md`, `docs/SOLID_MODEL.md`, `docs/FRONTEND_ARCHITECTURE.md`,
  `docs/DEVELOPMENT.md`.
- Closure notes: record measured latency, determinism parity evidence, the WASM export allowlist, and
  the partition-pruning result in the tracker before archiving, so the architecture's central claims
  stay auditable.

## Patch plan and reporting format

- Patch 1: WP-F1, WP-F2 (workspace and WASM path).
- Patch 2: WP-F3 (Solid app, build pipeline, template defect fixes).
- Patch 3: WP-F4, WP-F5 (containers and extended gate).
- Patch 4: WP-F6 (foundation documentation).
- Patch 5: WP-C1 (Question Model and Question Classification).
- Patch 6: WP-C2 (identity and lifecycle).
- Patch 7: WP-C3 (Assignment Attempt, policy, and summary model with compact policy-history behavior tests).
- Patch 8: WP-C4 (store and object contracts with reference backends and conformance suites).
- Patch 9: WP-C5 (seed vectors and parity harness) -- its own patch because it is a gate.
- Patch 10: WP-C6 (grading boundary) -- its own patch because it is a gate.
- Patch 11: WP-C7 (approved serialization fixtures and narrow test-local fakes).
- Patch 12: WP-C9 (frontend architecture contract, reactivity model, reference widget) -- its own patch
  because the UI lanes' independence rests on it.
- Patch 13: WP-C8 (contract register).
- Patches 14 onward are accepted implementation history. Remaining patches follow WP-RC2 through
  WP-RC12 in `docs/active_plans/active/release_completion_plan.md`, one integrated
  package at a time with only the explicitly owned subpatch splits.

Report each patch as: module ID and work package ID, files touched, gate commands with their exact
output lines, and any skipped check with a one-line scope note.

## Decision completeness

The current implementation and scope decisions are expanded into dispatchable packages in
`docs/active_plans/active/release_completion_plan.md`:

- The protected visual author editor now supports all eight version 2 Question Types. MC, MA, FIB,
  MULTI-FIB, NUM, MATCH, and ORDER provide their complete keyboard-first form controls. HOTSPOT
  provides verified-image selection, immutable version-scoped publication, exact issue-time asset
  binding, and the primary keyboard region-list workflow. Its integrated author-to-student
  object-lifecycle acceptance remains open in the PLE Question JSON type plan.
- New assignments default to `highest`; new practice Assignment Attempts use `newSeeds` while resumed attempts keep
  their issued seed.
- Retention defaults are notify at 30 days, archive at 100 days, student-record deletion at 365 days,
  and aggregate publication at k >= 5.
- Course deletion retains Assignment Content by default.
- Existing normalized-payload hard ceilings remain strict refusal boundaries; oversized archival and
  binary source uses typed object storage.
- Content-addressed deduplication is out of scope because it is an optimization, not an integrity or
  lifecycle requirement.
- Published WeBWorK PG Question Source is an immutable PLE object with its Question License, Source Object Reference, and Source Object Checksum, and the adapter
  calls the private standalone `/render-api` Question Backend service directly.
- Native Rust `axum` is the server runtime.
