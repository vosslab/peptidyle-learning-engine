# Release work-package ledger

## Authority and status

This ledger holds detailed work-package delivery information for
[release_completion_plan.md](release_completion_plan.md). The release plan remains binding for release
decisions, objectives, architecture, dependency order, acceptance/evidence, migration policy, risk,
rollout, and closeout. [implementation_status.md](../implementation_status.md) remains the sole
current-package and migration-allocation registry.

`WP-SD1-A-decisions-and-impact-contract` is the current package and remains pending independent
architecture/privacy ACCEPT. No later entry advances it or claims implementation completion.

## Dependency-ordered package registry

| Order | Package | Status | Depends on | Outcome |
| --- | --- | --- | --- | --- |
| 0 | WP-R0 | accepted | none | Ranked-catalog foundation |
| 1 | WP-R1 | accepted | none | Python-owned acceptance orchestration |
| 2 | WP-PY-L1 | accepted | WP-R1 | Legacy shell lifecycle replacement |
| 3 | WP-R2 | accepted | WP-R1 | Question lineage/catalog foundation |
| 4 | WP-SD1-A | **pending independent ACCEPT** | WP-R0--R2, WP-PY-L1 | Single-installation decision and impact contract |
| 5 | WP-SD1-B | pending | WP-SD1-A | Session/actor/browser authority |
| 6 | WP-SD1-C | pending | WP-SD1-B | Fresh BlueprintCourse/CourseInstance schema epoch |
| 7 | WP-SD1-D/E | pending | WP-SD1-C | Store/PostgreSQL/adoption/propagation |
| 8 | WP-SD1-F | pending | WP-SD1-D/E | Server/API/browser/live-demo convergence |
| 9 | WP-SD1-G | pending | WP-SD1-F | Real-stack cutover closure |
| 10 | WP-RC1 | accepted | none | Course appearance |
| 11 | WP-RC2 | accepted | WP-RC1 | Production seam cleanup |
| 12 | WP-RC3 | accepted | WP-RC2 | Bounded WeBWorK compatibility proof |
| 13 | WP-ARCH1 | accepted | WP-RC3 | Capability-sized source ownership |
| 14 | WP-RC3R | accepted | WP-RC3, WP-ARCH1 | Standalone renderer replacement |
| 15 | WP-UI1 | accepted | WP-RC3R | Teaching workspace composition |
| 16 | WP-HG1 | accepted | WP-RC3R, WP-UI1 | Instructor workflow evidence |
| 17 | WP-RC4 | open | WP-SD1-G, WP-RC3R | Flat JSON v2 closure |
| 18 | WP-P1--WP-P6 | open | WP-RC3R | Secure Student payload contract |
| 19 | WP-RC5 | open | WP-RC4, WP-P1--WP-P6 | Eight families and Chapter One |
| 20 | WP-RC6 | open | WP-RC5 | QTI export and H5P boundaries |
| 21 | WP-P2 | open | WP-R2 | Persistent bindings/migration handoff |
| 22 | WP-RC7 | open | WP-P2, WP-RC6 | Object reconciliation and M2--M5 |
| 23 | WP-RC8 | open | WP-SD1-G | Production identity and enrollment |
| 24 | WP-RC9 | open | WP-RC8 | LTI Advantage passback |
| 25 | WP-FU1--WP-FU6 | open | WP-RC9 | Secure Student artifact uploads |
| 26 | WP-RC10 | open | WP-FU1--WP-FU6 | Declarative deployment/restore |
| 27 | WP-RC11 | open | WP-RC10 | Bot-cost controls |
| 28 | WP-RC12 | open | all above | Release acceptance and docs closure |

## Accepted foundations

### WP-RC1: Complete course appearance

- **Owner/status:** UI/UX owner then integrator; accepted 2026-08-09. Detailed evidence is in
  [course_appearance_implementation.md](../workstreams/course_appearance_implementation.md).
- **Files:** `src/features/course_appearance/`, route/API/decoder contracts, course-appearance
  Playwright/E2E, and its architecture/contract/status/changelog consumers.
- **Behavior:** keyboard-complete theme selection, responsive previews, save/conflict recovery,
  banner replacement/removal, and current-pointer-only delivery. Candidate and delivery grants expire
  within 60 minutes.
- **Evidence:** focused Node/Playwright/Rust Store/server checks, disposable PostgreSQL/MinIO, and
  independent HCI/color/security review. Visual contact sheet was one-time evidence.

### WP-RC2: Remove placeholder production seams

- **Owner/status:** Rust and documentation owners; accepted 2026-08-09. Evidence is in
  [production_seam_closure.md](../workstreams/production_seam_closure.md).
- **Files:** H5P/QTI/WeBWorK adapters, Store backends, server run handling, and contract/map docs.
- **Behavior:** maintained modules name concrete responsibilities; catalog resolve/search is required;
  current feedback follows current assignment disclosure; explicit limited adapters stay test/local
  development owned.
- **Evidence:** focused behavior, format/Clippy/workspace/codebase/repository gates and independent
  review. Placeholder inventory was one-time evidence.

### WP-RC3: Integrate upstream WeBWorK as shipped

- **Owner/status:** adapter, container integrator, security reviewer; accepted 2026-08-10. Historical
  evidence: [webwork_shipped_integration.md](../workstreams/webwork_shipped_integration.md).
- **Files:** WebWork renderer adapter/server backend; historical private Compose topology; immutable
  licensed pilot fixture/provenance; seed, E2E, Playwright, and contract docs.
- **Behavior:** strict private render/grade translation reconstructs only server-side maps, sanitizes
  markup, emits answer-free envelopes, and rejects protected data, malformed output, identity drift,
  redirects, and unsupported controls.
- **Evidence:** owned redacted protocol tests and opt-in container/browser trace prove render, grade,
  cache, outage containment, and browser secrecy. Broad OPL compatibility is outside the claim.

### WP-ARCH1: Enforce capability-sized source ownership

- **Owner/status:** persistence, server, adapter/tooling, browser owners plus integration/review;
  accepted 2026-08-10. Closure is in
  [source_module_decomposition_plan.md](source_module_decomposition_plan.md).
- **Behavior:** complete capabilities live behind stable facades. Maintained source is at most 999
  physical lines; immutable migrations/history have only named approved exceptions.
- **Evidence:** permanent `tests/test_source_file_line_limit.py`, focused/integrated behavior,
  generated-contract/repository gates, disposable PostgreSQL baseline, and independent review.

### WP-RC3R: Standalone renderer replacement

- **Owner/status:** Rust adapter, container integrator, runtime/security reviewer; accepted
  2026-08-10. RC3 is historical; RC3R is the runtime topology.
- **Files:** WebWork HTTP renderer capability modules, server composition, Compose/env/probe/lifecycle
  modules, render/grade/cache/outage tests, and operation/status docs.
- **Behavior:** PLE uses a declared private stateless `webwork-pg-renderer` image. It sends trusted
  immutable source/metadata/seed/display policy and server-resolved response. WebWork2, MariaDB,
  course/roster/user/password/session dependencies are absent. Attempts/cache hits bind renderer
  identity and refuse drift.
- **Evidence:** offline adapter/server, volume-preserving live lifecycle, browser secrecy, repository
  gates, and independent review. Source/Compose scans were one-time probes.

### WP-UI1 and WP-HG1

- **WP-UI1:** accepted 2026-08-13. Shared teaching workspace, route/Question ID/gradebook identity,
  account preference, page composition, and visual acceptance; evidence in `docs/UI_DESIGN_REVIEW.md`.
- **WP-HG1:** accepted 2026-08-12. An Instructor can find `AAA-BBBB`, add it, and build the four-
  question Genetics Chapter One assignment through recovery. Details:
  [peptidyle-walkthrough-plan.md](../peptidyle-walkthrough-plan.md#wp-hg1-contract).

## Open release packages

### WP-RC4: Freeze PLE flat JSON v2

- **Owner/files:** Rust source/compiler, TypeScript/Solid contracts, independent family/security
  review; native flat reader/compiler, response/envelope models, validation/grading, generated
  contracts/decoders, response components, and architecture/contract docs.
- **Behavior:** strictly parse all eight families, reject duplicate/unknown/invalid source, compile
  answer-free public and server-only private values, and expose key-free learner response shapes.
- **Acceptance:** compact author examples and invalid boundaries, independent contract/security review,
  and evidence that canonical bytes and browser/Wasm projections remain answer-free.

### WP-P1 through WP-P6: Secure Student payload contract

- **Owner/details:** question-model, server, Store, browser, and security owners in
  [secure_question_grading_payload_plan.md](../decisions/secure_question_grading_payload_plan.md).
- **Behavior:** `QuestionAttemptId`, authenticated session, and idempotency key are sole submission
  authority. Issuance stores immutable presentation/asset and private grading snapshots; replay uses
  that evidence rather than current catalog/renderer state.
- **Acceptance:** focused Store/server/browser and disposable PostgreSQL prove mismatch refusal,
  refresh, duplicate, replica recovery, and unavailable-successor behavior.

### WP-RC5: All flat families and Chapter One content

- **Owner/files:** family Rust owners, Solid author/learner owner, content review/integration owner;
  response/validation/grading/native/Store/server, authoring/widgets, reviewed pilot source/provenance,
  seed/manifest, replacement scenario, and `docs/PILOT_CONTENT.md`.
- **Behavior:** protected visual authoring for eight families, keyboard-first editing, answer-free
  learner preview, server-verified HOTSPOT media with list alternative, complete lifecycle, and typed
  WebWork MATCH delivery.
- **Acceptance:** every family completes a Memory/PostgreSQL author-to-Student path. Genetics and
  Biochemistry Chapter One each publish four reviewed questions (WeBWorK MC/MATCH and flat MC/MATCH)
  with immutable provenance and correct/incorrect server grading. Visual review is evidence, not
  fixed-pixel/count tests.

### WP-RC6: QTI export and H5P capability claims

- **Owner/files:** Canvas/Blackboard exporters, H5P adapter, worker/UI integrator; QTI profiles/tests,
  export Store/object delivery/worker, author UI/Playwright, H5P importer/tests, generated contracts,
  and capability/usage docs.
- **Behavior:** background export snapshots one immutable supported version, refuses unsupported
  semantics before object creation, and provides requester-owned status/download. H5P remains ungraded
  practice unless it translates losslessly into protected native v2.
- **Acceptance:** supported round trips preserve prompt/choice order/correct binding/points/identity;
  unsupported semantics create no artifact; worker/object authorization and independent review pass.

### WP-P2: Persistent bindings and migration handoff

- **Owner/outcome:** persistence ownership preserves migration order and removes legacy consumers
  before RC7 schema work. Allocation belongs only in implementation status.
- **Acceptance:** the migration allocation review, legacy-consumer transition, and named Store
  boundaries pass before WP-RC7 accepts any schema change.

### WP-RC7: Reconcile objects and close M2--M5

- **Owner/files:** PostgreSQL and object-store owners; inventory/reconciliation Store modules, the
  allocated migration, worker, object/
  retention/security docs, and named integration E2E.
- **Behavior:** bounded inventory records deterministic bytes, marks first observed orphans,
  quarantines/deletes only twice-observed unreferenced bytes, cancels deletion on reference, and alerts
  on missing/mismatched referenced bytes. Database records are never deleted to hide breakage.
- **Acceptance:** Memory/PostgreSQL/MinIO agree on exact/replay/orphan/missing/mismatch/concurrent
  creation. Combined M2--M5 includes hostile import, answer/cross-user denial, partitions, renderer,
  statistics, archive/delete, replica, and worker recovery.

### WP-RC8: Production identity and enrollment

- **Owner/files:** authentication, PostgreSQL, enrollment/API, UI, SMTP configuration, independent
  security/HCI; passwordless/email/WebAuthn/OIDC, account/roster Stores, allocated migrations,
  sign-in/account/roster/invitation UI, E2E/Playwright, and identity/enrollment docs.
- **Behavior:** global `UserId`, rate-limited single-use email challenges, optional passkeys on the
  same account, host-only HttpOnly session, exact course/Student authority, atomic invitations and
  enrollment, optional OIDC/SAML linking only to existing accounts, and protected grade export.
- **Acceptance:** real email, optional passkey, replacement, invitation claim, access, multi-replica,
  hostile token/domain/CSV refusal, browser handoff, and independent security/HCI. Demo role entry is
  not production identity evidence.

### WP-RC9: LTI Advantage launch and grade passback

- **Owner/files:** LTI, Store/PostgreSQL, LMS review; LTI server/store, allocated migration, launch UI,
  E2E, and security/usage/deployment docs.
- **Behavior:** validate OIDC and LTI 1.3 launch binding, retain credentials server-side, derive
  replay-safe AGS scores from `student_assignment_summary`, and use bounded durable retry.
- **Acceptance:** LMS sandbox/standards harness proves mapping/passback; forged, cross-course/user,
  stale launches and grade URLs refuse; outage queues safely; protocol/security review passes.

### WP-FU1 through WP-FU6: Secure Student artifact uploads

- **Owner/details:** [secure_student_file_upload_plan.md](secure_student_file_upload_plan.md).
- **Behavior:** server-issued exact CourseInstance/Student/attempt upload authorization writes to
  non-deliverable temporary storage, performs closed inspection, promotes SHA-256-bound bytes, and
  atomically consumes one ready upload in the automated artifact workflow. The Student route is
  fail-closed until all six packages accept.
- **Acceptance:** the state-machine, authorization, hostile-file, quarantine/promotion/consumption,
  Store/PostgreSQL/browser, object delivery, and independent security gates named in its plan pass.

### WP-RC10: Declarative AWS deployment

- **Owner/files:** deployment architect/PostgreSQL; `deploy/opentofu/`, policy tests, disposable
  deploy/restore scripts, and deployment/security/operations/cost docs.
- **Behavior:** private endpoints, encrypted RDS/PITR/TLS, four encrypted S3 domains, ECR/Fargate
  API/worker/publisher, private ALB, CloudFront, WAF, KMS, Secrets Manager references, least IAM,
  alarms, ceilings, and immutable rollback manifests. Renderer remains disabled pending separate
  production attestation.
- **Acceptance:** disposable plan/apply/migrate/live journey, RLS role separation, publisher-only
  public authority, public-tag isolation, protected delivery, restore, rollback, drift, bounded destroy,
  and operations/security review.

### WP-RC11: Bot-cost controls

- **Owner/details:** [bot_traffic_cost_reduction_plan.md](bot_traffic_cost_reduction_plan.md) with
  deployment integration; landing, request-cost/router, edge/WAF/observability, offline/browser/E2E,
  and bot operations/acceptance docs.
- **Behavior:** anonymous traffic ends at static edge storage; malformed sessions make zero Store
  calls; valid-format unknown tokens make at most one indexed lookup; origins stay private; WAF starts
  in count mode with accessible recovery.
- **Acceptance:** crawler/class-start/origin/cache-poison/alarm/emergency evidence establishes bounded
  cost without client analytics or educational-record joins. Timing/source-string probes stay one-time.

### WP-RC12: Release acceptance and documentation closure

- **Owner/files:** release integrator; aggregate/local-stack/E2E/browser/screenshot owners, release
  evidence, core operational/architecture/security docs, implementation status, changelog, and notes.
- **Behavior:** one reproducible live journey from clean clone through build, migration, identity,
  teaching, deterministic grading, export, retention, backup, restore, and rollback. Environment and
  external activation dependencies are explicit.
- **Acceptance:** `source source_me.sh && ./all_test.sh`, screenshots where required, disposable
  cloud/service/fault/accessibility exercises, link/ASCII/whitespace checks, and independent
  code/security/database/accessibility/operations/documentation reviews. Required lanes pass, not skip.

## Handoff requirement

Every package handoff states package ID, owner, changed files, behavior/security boundary, focused/
package/release checks, evidence class and receipt, governing decision, and independent findings.
Update this ledger and the binding release plan together; update implementation status only when its
registry facts change.
