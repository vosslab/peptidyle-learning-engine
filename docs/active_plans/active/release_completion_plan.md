# Plan: Peptidyle Learning Engine release completion

## Status

**Current prerequisite: WP-SD1-A is pending independent ACCEPT.** PLE is one installation with global accounts, one Instructor-visible Question Library of Published Questions, private drafts, equal active Instructors, equal Teaching Team Members, and exact CourseInstance/Student authorization. Available Question Revisions are ordinarily selectable; Archived Question Revisions remain resolvable for history and evidence. The current SD1 registry owns the pre-production cutover before release work resumes.

The authoritative current-package and migration-allocation state is [implementation_status.md](../implementation_status.md). WP-RC1, WP-RC2, WP-RC3, WP-RC3R, WP-ARCH1, WP-UI1, WP-HG1, WP-R0, WP-R1, WP-R2, and WP-PY-L1 remain accepted where their recorded evidence says so. WP-RC4 through WP-RC12 stay open until their named gates and independent review pass.

`WP-SD1-A-TERM-01-RQB1` is accepted historical evidence. Its prior server-boundary terminology is
superseded by RQB2's exact iMathAS ownership while its least-privilege, PostgreSQL, and aggregate
receipts remain valid evidence.

`WP-SD1-A-TERM-01-RQB2` is accepted after RQB1. It owned one direct pre-production
naming cutover: the remaining generic backend-session names become exact iMathAS Session,
Challenge, Authentication, Grading Context, Result Token, Result Exchange, Result, and Question
Backend Transport ownership; response/control/Student Response marker names become
`ImathasQuestionBackend`/`imathasQuestionBackend`. Question Model owns marker and
generated-boundary renames; LDA and `2026090102` own durable/schema/procedure renames; the iMathAS
adapter owns transport, launch, and result translation; browser copy uses Question Backend only when
backend-agnostic. It preserves RQB1's Session lifecycle, Question Submission marker flow, tables and
relationships, procedures, browser launch flow, security invariants, and focused test categories.
Generated 467 TypeScript declarations; focused adapter, LDA, Node, TypeScript, formatting, and strict
Clippy gates; fresh-apply/no-op/catalog/restricted-login PostgreSQL and both iMathAS PostgreSQL tests;
least-privilege review; and the required aggregate suite all passed on the final tree. This package closes
only the exact terminology cutover; the overall terminology program remains open.

`WP-SD1-A-TERM-01-SRF1` is accepted. This no-schema terminology slice aligns the answer-free Student
Response Format Check and its thirteen exact Student Response Format Issues across domain, Wasm,
strict browser decoding, and visible Response Format Messages. Its direct cutover, focused gates,
independent audit with repaired findings, and complete aggregate acceptance passed. The planned
key-free server format-validation fallback remains a separately allocated future server-boundary
correction; the current server does not mount that route.

`WP-SD1-A-TERM-01-PI2` is accepted as the direct Assignment Question Analysis domain/schema/typed-Job
cutover in fresh migration `2026082923`. `AssignmentQuestionAnalysis` and
`assignment_question_analysis` own the Course Instance-and-Assignment-scoped, Scoring
Generation-bound analysis for one source Assignment Entry and exact Question Revision; its four
Question Outcome Categories remain separate from Unscored Attempt Count. The production repair
constrains the typed Job target, while two fixture/oracle composition repairs complete the existing
durable SD1 evidence. `assignment_analysis_course_assignment_matches` proves the composite Course
Instance-plus-Assignment relationship and rejects a cross-Course Assignment. Focused model/schema
gates, independent inventory, and the live SD1 least-privilege PostgreSQL lane pass. No Store,
route, browser, generated contract, worker, new test file, or fixture suite was added; one compact
reciprocal case was added to the existing durable SD1 oracle. This package precedes but does not
accept the existing `WP-SD1-A` independent architecture/privacy final SD1 gate.

`WP-SD1-A-TERM-01-IAA1` is allocated after the accepted global Account/Authenticated Session baseline
and before `WP-RC8`. It directly replaces the generic executable PostgreSQL account creator with the
exact Sysadmin Create Instructor Account operation and gives the shared private Authentication Email
relation role-qualified integrity rules. It owns no roster route, Store, DTO, invitation claim, or
Instructor email-replacement implementation; `WP-RC8` remains the sole future Course Roster Import
transaction owner for Student Account resolution and creation.

`WP-SD1-A-TERM-01-AEM1` completed removal of the unmounted Assignment Export persistence/Job stub and its
named current-service documentation inventory before any export feature is admitted. That
inventory covers the security, identity, database-authorization, multi-server, object-storage,
data-classification, authorization-contract, component-consumer, implementation-plan, release-
plan, and customer-spec claims recorded in [implementation_status.md](../implementation_status.md). No
current Assignment Export persistence, Job kind or target, Store, route, worker, delivery path, browser
contract, or service remains.
Its residual audit retains ordinary export wording, Course Grade CSV export, QTI interchange, and
the answer-key-free DOCX/PDF renderer, while a future Assignment Export Manifest remains a
complete authorized service prerequisite rather than a preparatory schema or release claim. Fresh
PostgreSQL catalog, print-renderer, documentation, Rust, residual, and independent-review evidence pass.

This is the binding release authority for decisions, objectives, architecture, dependency order, acceptance/evidence, migration policy, risks, rollout, and closeout. The [Current Package Registry](../implementation_status.md) records current package status. Update both documents when a release decision, dependency, status, or acceptance condition changes.

### Evidence classification

Apply [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md) and the permanent-test checklist to every package. Permanent tests prove stable behavior and security boundaries. Disposable service, cloud, browser, screenshot, migration, timing, and reconstruction checks prove their distinct environmental claims. One-time inventories and probes record a decision, then leave the permanent suite. Fixtures exist only for stable serialized contracts; otherwise use inline builders.

## Decisions

### In-scope Decision Register

| Topic                         | Binding decision                                                                                                                                                                                                                                                                                                                                                                                                                                    | Owner                                                                                           |
| ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Installation and roles        | One PLE installation has global accounts. Each account has one immutable Student, Instructor, or Sysadmin role; people needing multiple roles use separate accounts. Course authority is matching exact membership and Student ownership. Sysadmin Create Instructor Account creates one active Account with the fixed Instructor Product Role from a normalized email address and creates no Sysadmin membership; support is explicit and audited. | WP-SD1                                                                                          |
| Reusable courses              | A revisioned `BlueprintCourse` owns reusable ordered structure. Every `CourseInstance` has one immutable Blueprint parent and applied revision; it alone owns Students, deadlines, releases, accommodations, grades, and delivery state.                                                                                                                                                                                                            | WP-SD1-B--G                                                                                     |
| Published questions           | Stable `AAA-BBBB` `QuestionId` identifies a lineage; immutable `QuestionRevision` records hold reviewed revisions. Assignments and evidence pin exact versions and never move automatically.                                                                                                                                                                                                                                                        | WP-R2, WP-SD1                                                                                   |
| Draft questions               | One mutable Draft Question belongs to one Authoring Workspace. Its private UUID is server-only and its positive Draft Question Edit Number is the save/publication concurrency token. Publication creates an immutable `QuestionRevisionReference { question_id, revision_number }`; Draft Question revision history is not retained.                                                                                                               | DQM1 implemented pending independent review; QSRC2 implemented pending joint independent review |
| Question stewardship          | Moderate owner edits, validated exact-base Change Proposals, full private-draft forks, and audited Sysadmin ForcedQuestionCorrections preserve attribution, compatible CC licensing, history, and exact pins. UI label: **Suggest an improvement**.                                                                                                                                                                                                 | WP-R2, WP-SD1                                                                                   |
| PLE questions                 | PLE Question JSON schema version 2 is the PLE Question Source for MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. Public Question Presentations are answer-free; grading remains server-owned.                                                                                                                                                                                                                                              | WP-RC4, WP-RC5                                                                                  |
| Question variation and grades | New Assignment Attempts independently use `reuseSelection` for Question Pool membership and `newVariation` for Question Variations; resumed Issued Questions retain their exact issued variation. The grade default is `highest`.                                                                                                                                                                                                                   | WP-RC0, WP-PROF-T5, WP-PROF-G1--G5                                                              |
| Retention                     | CourseInstance Student records notify at 30 days, archive at 100, delete at 365; course-owned Assignment Content remains. Aggregate publication requires k >= 5.                                                                                                                                                                                                                                                                                    | WP-RC0, WP-SD1                                                                                  |
| Adapters                      | QTI profiles are strict and lossless or refuse. Canvas/Blackboard QTI interchange remains future dedicated work. H5P is ungraded practice unless translated losslessly into the protected native model.                                                                                                                                                                                                                                             | WP-RC6                                                                                          |
| Objects                       | Database records define intended bytes; inventory proves storage. Object Storage Check uses two observations and reference rechecks; Object Storage Repair acts only on that evidence. A dedicated publisher alone activates immutable public copies.                                                                                                                                                                                               | WP-RC7                                                                                          |
| Identity and enrollment       | Email-code sign-in is the primary authentication method; passkeys are optional convenience credentials on the same global account. Invitations create exact course membership and Student records atomically.                                                                                                                                                                                                                                       | WP-RC8                                                                                          |
| LTI                           | LTI 1.3 launch and AGS passback use verified server credentials and summary-derived grades only.                                                                                                                                                                                                                                                                                                                                                    | WP-RC9                                                                                          |
| Deployment and traffic        | OpenTofu owns disposable AWS infrastructure. Anonymous landing traffic terminates at static edge storage; authenticated requests have bounded cost and no client analytics.                                                                                                                                                                                                                                                                         | WP-RC10, WP-RC11                                                                                |

### Out-of-scope decisions

Version 1 excludes content-addressed byte deduplication, a TypeScript API server, scored native H5P, local passwords, mandatory institutional SSO, client analytics, Kubernetes/Redis/Kafka/sharding, unreviewed rich-media QTI mappings, a Rust QTI Package Maker port, actual institutional credentials, and a real 10,000-Student cohort. These future possibilities do not relax release acceptance.

## Objectives and scope

Deliver one coherent automated-grading platform and the required production-stack journey. Grading, answer keys, correctness decisions, object authorization, and course selection remain server-owned. Browser contracts remain answer-free. Issued work and grading evidence are immutable, and Instructor inspection is audited.

The release scope is the dependency-ordered [Current Package Registry](../implementation_status.md): WP-RC1--WP-RC12, WP-ARCH1, and their current-package prerequisites. It includes live delivery convergence, variation, discovery, sharing, reusable curricula, Blueprint updates, automated grading operations, PLE Question implementations, adapters, QTI interchange, DOCX/PDF print rendering, Object Storage Check and Repair, identity/enrollment, LTI, artifacts, deployment, cost controls, and final closure.

No package may turn an unresolved product decision into an implicit compatibility path. If evidence invalidates a decision, update the In-scope Decision Register, every affected package entry, and acceptance evidence in one reviewed planning change before code continues.

### BlueprintCourse and CourseInstance cutover

`BlueprintCourse` is the only reusable course-level aggregate. Its revision holds ordered modules, assignments, relative schedule defaults, and exact published-question pins. A one-assignment reusable unit is a one-module `BlueprintCourseView`, not another type.

Each `CourseInstance` binds to one immutable Blueprint parent and applied source revision. Instantiation copies reusable meaning and resolved defaults, never Student records. Students, deadlines, releases, accommodations, grades, attempts, and delivery settings are private CourseInstance state. A Blueprint revision can make a new assignment available to descendants as unreleased; release requires an explicit Instructor decision and preserves local delivery edits.

`2026082911` owns minimal-Blueprint construction; the direct course-creation capability invokes
it while atomically creating the bound CourseInstance and initial Instructor membership.
`2026082913` owns immutable CourseInstance adoption records and their idempotency key;
`2026082929` owns the only executable Blueprint-operation application capability over those
records, including Assignment Import Repair; `2026082930` owns forced RLS for CourseInstance roots
and dependent private state.
`2026082906` owns the shared Rust
account-transaction installer. The Blueprint operation boundary has exactly six operations and never creates a
blank CourseInstance. An apply receives scope only from session-derived `AuthenticatedSession`; adapters
and protected database operations receive no client-supplied installation scope.

No current product type, route, Store capability, PostgreSQL table/function/policy, generated contract, live-demo resource, or screenshot may use Alpha as a Peptidyle product concept. Historical migrations, changelogs, and ADAPT comparison material remain evidence rather than compatibility contracts. Fresh SD1-C allocations belong only in [implementation_status.md](../implementation_status.md).

### Cutover evidence boundary

| Claim                                                                                                                 | Evidence                                                                      |
| --------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Aggregate normalization, revision races, propagation, and private delivery state                                      | Permanent focused Rust/Memory/Node behavior and contract tests                |
| Global Question Library visibility, CourseInstance isolation, protected database authority, and fresh/no-op migration | Disposable PostgreSQL/RLS acceptance                                          |
| Create, revise, publish, select, instantiate, update, and release workflows                                           | Production HTTPS browser acceptance through `run_playwright_tests.sh --build` |
| Hierarchy, release state, recovery, focus, contrast, and product vocabulary                                           | Rendered screenshots plus independent visual review                           |
| Complete material tree                                                                                                | `source source_me.sh && ./all_test.sh`, with every required lane passing      |

## Architecture and ownership

| Boundary                  | Owner                                                  | Rule                                                                                                                                               |
| ------------------------- | ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Product decisions         | `docs/HUMAN_GUIDANCE.md` and this plan                 | Human Guidance remains terse owner intent; settled engineering interpretation belongs here or in [DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md). |
| Public question contracts | `crates/question_model`                                | Generate answer-free TypeScript contracts.                                                                                                         |
| Source adapters           | `crates/adapters/{ple,qti,webwork,h5p,imathas}`        | One strict versioned adapter per format.                                                                                                           |
| Grading                   | `crates/grading` plus server-only adapter capabilities | No browser, generated TypeScript, or Wasm grading authority.                                                                                       |
| Persistence               | `crates/learning-data-access`, `schemas/migrations`    | Memory/PostgreSQL parity; PostgreSQL is production authority.                                                                                      |
| Objects                   | `crates/objects`                                       | Typed keys, checksums, role-based delivery, inventory, Object Storage Check and Repair.                                                            |
| HTTP and workers          | `crates/server`                                        | Bounded same-origin requests; durable jobs carry explicit least authority.                                                                         |
| Browser                   | `src/`                                                 | Strict decoders, accessible visible workflows, no source archive parsing.                                                                          |
| Local stack               | `local_stack_control/`, `containers/`                  | Python owns complex orchestration; shell entry points are direct facades.                                                                          |
| Deployment                | `deploy/opentofu/`                                     | Declarative, reviewable, disposable before activation.                                                                                             |

## Dependency order

```text
WP-SD1-A independent ACCEPT
  -> WP-SD1-B -> WP-SD1-C -> WP-SD1-D/E -> WP-SD1-F -> WP-SD1-G
  -> WP-RC4 -> WP-P1..WP-P6 -> WP-RC5 -> WP-RC6
  -> WP-P2 -> WP-RC7
  -> WP-SD1-A-TERM-01-IAA1 -> WP-RC8 -> WP-RC9 -> WP-RC10 -> WP-RC11 -> WP-RC12

Accepted foundations: WP-RC1 -> WP-RC2 -> WP-RC3 -> WP-ARCH1 -> WP-RC3R.
Accepted orchestration: WP-R1 -> WP-PY-L1. Accepted stewardship: WP-R2.
```

WP-SD1 is the release prerequisite. `WP-SD1-A-decisions-and-impact-contract` stays pending until its independent architecture/privacy review accepts it. B--G proceed in order without an Alpha bridge or parallel product path. WP-P1 may progress beside RC4 closeout, but all WP-P1--WP-P6 requirements accept before RC5. WP-P2 preserves migration allocation and transitions legacy consumers before RC7 schema work. `WP-SD1-A-TERM-01-IAA1` supplies exact current-baseline Instructor creation and role-qualified private-email integrity before the complete `WP-RC8` roster/import delivery transaction.

## Release acceptance criteria

- Every package has a named owner, actual artifact boundary, behavior, evidence class, and independent review.
- Focused mocks support code work but cannot solely accept a production route, Store, object, identity, worker, or deployment claim.
- Answers, keys, Question Backend credentials, object credentials, and authority selection remain server-owned.
- Student workflows are keyboard complete; authoring has visible focus, status, recovery, reflow, and semantic labels; HOTSPOT has a non-pointer alternative.
- Migrations are forward-only and demonstrate fresh, no-op, checksum, RLS/grant, and live behavior evidence.
- Required skipped, unavailable, stale, or human-only evidence remains open.

## Test and verification strategy

Run the narrowest owner check, then the package gate, then release evidence. The aggregate front door:

```bash
source source_me.sh && ./all_test.sh
```

`all_test.sh` owns `check_rust.sh`, `check_codebase.sh`, repository pytest, and `local_stack.py acceptance`. The current controller runs its declared database/object acceptance lanes. The former `run_playwright_tests.sh --build` and screenshot-corpus browser lanes are unmounted after their owner configuration was retired; browser restoration is required before release acceptance. `tests/e2e/e2e_run_all.sh` is an explicit non-browser bulk E2E owner, never a second aggregate. Development SKIP output names a missing prerequisite; release evidence requires PASS.

| Evidence class          | Owner                              | Claim                                                                                     |
| ----------------------- | ---------------------------------- | ----------------------------------------------------------------------------------------- |
| Permanent offline       | `all_test.sh`                      | Stable Rust, TypeScript, Node, Python, generated-contract, security, and hygiene behavior |
| Real service acceptance | `local_stack.py acceptance`        | Its current declared PostgreSQL and object-service lanes                                  |
| Production browser      | Restoration required               | Built bundle through HTTPS gateway and real UI-created state                              |
| Visual evidence         | Restoration required               | Fresh rendered states and human visual review for UI or viewport changes                  |
| One-time evidence       | Graphify and direct probes         | Narrow decision/migration/config disposition, not permanent tests                         |
| Human acceptance        | Independent review and walkthrough | Teaching workflow, accessibility, visual sense-making, legal/activation decisions         |

The final handoff records command, date, material-tree state, environment, receipt path, and limitations for every required evidence class.

## Migration policy

The shared [Migration Allocation Registry](../implementation_status.md#migration-allocation-registry) is the only allocation registry. New schema packages receive an allocation before implementation; accepted migrations are never inserted or renamed. PLE Question JSON identity stays in its versioned source payload and immutable object/checksum binding; no generic catchall table is added. Current source and disposable test data use PLE Question JSON schema version 2 only. `QuestionPresentationBinding`, QTI profile v1, and `AAA-BBBB` Question IDs are current contracts, not compatibility shims.

## Risk register

| Risk                                                  | Owner                 | Control                                                                                                                    |
| ----------------------------------------------------- | --------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Documentation substitutes for product evidence        | Release integrator    | Package acceptance requires working behavior and evidence.                                                                 |
| Adapter output leaks answers or unsafe markup         | Adapter owner         | Strict translation, sanitization, private network, browser trace.                                                          |
| New protected data boundary exposes an Answer Key     | Boundary owner        | Public/private compilation, DTO scans, server-only grading.                                                                |
| Object Storage Repair deletes valid concurrent bytes  | Object owner          | Two observations, quarantine, reference recheck, idempotency.                                                              |
| Role/membership disagreement selects course authority | Auth owner            | One immutable account/session role, matching Student/Instructor membership, no Sysadmin membership, and origin validation. |
| Published bytes escape before commit                  | Object owner          | Transactional pending registry and dedicated publisher.                                                                    |
| iMathAS Question Backend dispatch outcome is unknown  | iMathAS adapter owner | Durable lease-bound marker and explicit operator resolution.                                                               |
| Deployment exposes secrets or broad destroy           | Deployment owner      | Secret references, unique tags, reviewed plan, bounded destroy.                                                            |
| Bot protection harms legitimate users                 | Edge owner            | Count mode, accessible recovery, versioned legitimate corpus, rollback.                                                    |
| Pilot begins before activation evidence               | Product owner         | Separate signed production-activation checklist.                                                                           |

## Rollout and closeout

Working-codebase release proves reproducible repository-owned artifacts without institutional secrets. Production activation supplies operator credentials, applies deployment, runs named live gates, completes legal review, and enrolls the pilot. Neither milestone substitutes for the other.

WP-RC12 closes only after every package in [implementation_status.md](../implementation_status.md) has required PASS evidence and independent review. It updates release evidence, documentation, implementation status, changelog, and release notes with exact receipts. Source inventories, scratch probes, and temporary diagnostics remain documented one-time evidence rather than fragile permanent tests.

Each package handoff records package ID, owner, changed files, visible/security behavior, focused/package/release checks, evidence paths, governing decisions, and independent findings.
