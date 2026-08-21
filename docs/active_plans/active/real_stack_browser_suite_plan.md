# Plan: Canonical real-stack browser suite

## Primary outcome

`WP-PROF-BS1` gives PLE one production browser artifact and one disposable real-stack browser path.
Playwright, canonical screenshots, direct browser development, and aggregate acceptance all visit the
production `dist/` bundle through the private HTTPS gateway. That browser uses the real Rust API,
PostgreSQL, MinIO, worker, renderer, authentication, authorization, and seeded live-demo data.

The suite owns a fresh stack for a focused invocation and one shared stack for a complete invocation.
Every scenario remains independently runnable: it starts from the declared seed baseline, creates a
unique namespace through visible PLE workflows, and observes its own results without a prior scenario.
The live-demo E2E owner becomes the shared browser-suite owner. Narrow decoder, serialization, and
error-mapping tests remain fast unit tests outside the product browser runtime graph.

This package is current. `WP-PROF-T3` remains a planned frozen-scope successor and resumes after
BS1 acceptance; `WP-PROF-T4` follows T3. The changing handoff is recorded only in
[implementation_status.md](../implementation_status.md).

## Evidence and boundaries

The suite proves the user-facing system first. A UI-created effect is normally demonstrated by a
reload, a new authorized session, or a different authorized role observing its product result.
Read-only service receipts supplement that evidence only where the requirement is specifically about
a service boundary:

| Claim | Primary evidence | Read-only receipt when needed |
| --- | --- | --- |
| Course, assignment, roster, or response persistence | Reload or second authorized session | PostgreSQL only for a database/RLS claim |
| Media upload and delivery | UI upload followed by an authorized visible load | MinIO for object lifecycle or access-policy claim |
| Render or grade result | Visible rendered problem, result, and recovery | Renderer or worker for provider/retry claim |
| Role boundary | Allowed role completes the journey; denied role sees the safe result | Server/network observation for protected-transport claim |
| Screenshot provenance | Image metadata records suite origin, scenario, and production bundle | Gateway-origin verifier |

The browser-suite owner under `tests/e2e/`, backed by `local_stack_control`, owns stack creation,
private inputs, HTTPS origin selection, scenario dispatch, infrastructure faults, read-only receipts,
and cleanup. Playwright owns visible interaction, accessible assertions, product-state creation,
network observation, and screenshots. The production gateway serves `dist/`; the server owns session
validation, authorization, decoding, and persistence.

The architecture keeps the constraints that this boundary directly exercises: closed private-input
shapes and limits; same-origin sensitive requests; server-side session and authorization checks; TLS
at the gateway; least-privilege private input files; and safe browser persistence and caching. Existing
security authorities continue to own broader ASVS baseline coverage.

## Frozen baseline and scenario isolation

The sole allowed non-UI product-state bootstrap is the live-demo baseline defined by
[LIVE_DEMO_SPEC.md](../../LIVE_DEMO_SPEC.md). Regeneration restores this normal PLE data; it does
not create a parallel demo application. The owner freezes these exact baseline records:

| Baseline record | Fixed identifier or state | Allowed scenario use |
| --- | --- | --- |
| Demo tenant | `00000000-0000-0000-0000-000000000100` | Tenant context and baseline reads |
| Dr. Elena Rivera | `00000000-0000-0000-0000-000000000101`, seeded Instructor | Ordinary Instructor entry and baseline reads |
| Mary Okafor | `00000000-0000-0000-0000-000000000102`, seeded Student | Base-course completed-run observation |
| Jack Chen | `00000000-0000-0000-0000-000000000103`, seeded Student | Base-course in-progress-run observation |
| Avery Singh | `00000000-0000-0000-0000-000000000104`, unapproved account | Sysadmin approval journey and practice-course baseline read |
| Morgan Reyes | `00000000-0000-0000-0000-000000000105`, seeded Sysadmin | One visible generation-bound first claim and ordinary passkey login |
| Biochemistry Base Course | `2026-01-01` through `2099-12-31`, `America/Chicago` | Baseline course, roster, activity, and assignment reads |
| Genetics Practice Course | Seeded course with Morgan as Instructor and Avery as Student | Cross-course authorization and baseline reads |
| Peptide bond publication | Published `peptide_bond_geometry` question and one-point assignment | Visible problem, delivery, and grading baseline reads |

Mary has one completed correctly graded deterministic-seed-17 run; Jack has one open deterministic-
seed-23 attempt. Elena, Mary, and Jack are active Base Course members. The fixed baseline has no
cross-course memberships beyond the stated practice-course membership. Student and Instructor
selector entry establishes ordinary server sessions. The installation generation, cryptographic
service inputs, and selected infrastructure faults are harness concerns rather than product-state
setup.

The scenario contract uses `schemaVersion: 2` and declares one closed Sysadmin requirement:
`not_required`, `unclaimed`, or `claimed`. `not_required` begins at the untouched seeded baseline
and has no claim material. The one `unclaimed` first-claim scenario receives the generation-bound
proof and completes Morgan's visible virtual-authenticator claim. A `claimed` scenario begins from
a fresh stack, receives the owner's visible first-claim setup child once, and then uses ordinary
passkey-ready state. I1 and L1 declare `not_required`. B0 initially registers the current
`live_demo` as the sole `unclaimed` transition; A1 atomically replaces it with
`sysadmin_first_claim` and its `claimed` authorization scenario. A complete invocation runs the
first claim only when a selected scenario requires it; focused invocations retain the same declared
transition from a fresh stack.

Each scenario declares:

- its allowed baseline personas and baseline records;
- a scenario ID and collision-resistant namespace used in every UI-created name;
- the visible actions that create and mutate product state;
- the reload, cross-session, or cross-role observation that proves its outcome; and
- any read-only receipt and its service-specific claim.

The complete suite shares a stack for efficient service startup, while each scenario creates and
owns unique records in its namespace. Focused selection receives a new stack and runs the same
scenario contract. The harness can schedule independent scenarios in parallel only after their
namespaces, actors, and mutable product surfaces are disjoint; the default order carries no behavior
dependency.

## Behavior-value inventory

Before retirement, the migration matrix classifies each legacy browser/mock case by the user or
contract behavior it protects: real-stack browser scenario, isolated unit test, existing service
oracle, or obsolete implementation detail. It records the successor and evidence location. The
matrix is a behavior-value inventory; legacy filenames, assertion counts, mocks, and application
shapes do not define replacement scope.

## Small milestones

Each milestone is independently closable by a manager and fresh subagents. A manager dispatches
one owner per work package, runs the stated automated gate, records the result, and moves to the next
ready milestone. Visual review uses the repository `image_evaluator` role against captured artifacts;
it produces an automated structured report for the acceptance record. The plan has no human approval,
interaction, or inspection prerequisite.

| Milestone | Work package | Outcome | Depends on | Automated checkpoint |
| --- | --- | --- | --- | --- |
| BS1-1 | `WP-PROF-BS1-H0` | Add a typed owner adapter around the existing lifecycle | none | Caller-configuration rejection before allocation, generated-input validation before Chromium, one visible `live_demo` scenario, and lifecycle/cleanup receipts |
| BS1-2 | `WP-PROF-BS1-H1` | Route `run_playwright_tests.sh` through H0 and build `dist/` | H0 | Default and focused command selections reach a fresh HTTPS origin |
| BS1-3 | `WP-PROF-BS1-H2` | Freeze seed-baseline and scenario-isolation contracts | H0 | Offline contract tests and plan-backed scenario manifest check |
| BS1-4 | `WP-PROF-BS1-H3` | Add origin and suite-resource cleanup oracles | H0 | Origin, labelled-inventory, process, and repeat-run checks |
| BS1-5 | `WP-PROF-BS1-C0` | Expand `all_test.sh` into the aggregate Validation front door | H1, H3 | Ordered aggregate receipts and one `local_stack.py acceptance` invocation |
| BS1-6 | `WP-PROF-BS1-B0` | Establish the catalog and multi-scenario owner foundation | H2, C0 | Closed registry, exact selection, per-scenario input/origin receipts, and automated Sysadmin transition sequencing |
| BS1-7 | `WP-PROF-BS1-I1` | Migrate one instructor authoring/course family | B0, C0 | Reload and second-session behavior evidence |
| BS1-8 | `WP-PROF-BS1-L1` | Migrate one learner delivery/response family | B0, C0 | Reload or second-session learner evidence |
| BS1-9 | `WP-PROF-BS1-A1` | Migrate ordinary auth, Sysadmin claim, and role-boundary families | B0, C0 | Visible role/session and denial scenarios |
| BS1-10 | `WP-PROF-BS1-S1` | Add semantic persistence receipts where claims require them | I1, L1, A1 | Read-only Store/service receipts tied to named claims |
| BS1-11 | `WP-PROF-BS1-U1` | Move narrow mock-dependent behavior to isolated unit owners | C0 | Focused Node/Rust unit tests and runtime-consumer inventory |
| BS1-12 | `WP-PROF-BS1-X1` | Exercise real concurrent-session conflicts | I1, B0 | Two-session UI conflict scenario |
| BS1-13 | `WP-PROF-BS1-F1` | Exercise lifecycle-controlled infrastructure recovery | C0 plus affected family | Owner-fault scenario and visible recovery |
| BS1-14 | `WP-PROF-BS1-V1` | Capture canonical screenshots from accepted real scenarios | I1, L1, A1, S1 | Origin/provenance verifier and image-evaluator report |
| BS1-15 | `WP-PROF-BS1-R1` | Retire the alternate browser application and mock runtime graph | U1, X1, F1, V1 | Build/consumer inventory and canonical suite run |
| BS1-16 | `WP-PROF-BS1-C1` | Invoke the suite once from aggregate acceptance and complete closure | C0, R1 | Final Validation and repeat-run cleanup evidence |

The manager may dispatch H1 and H2 after H0. C0 follows accepted H1 and H3. B0 follows C0 and
supplies the shared catalog, multi-scenario owner, and generic browser helpers. I1, L1, and A1
become parallel after B0; U1 can proceed after C0. Each family creates its own UI state from the
H2 baseline. S1, X1, F1, and V1 use accepted scenario contracts and separate owned files. R1 and
C1 integrate serially.

## Work-package contracts

### WP-PROF-BS1-H0: Add the typed owner adapter

- Owner: expert coder.
- Deliverable: a closed, typed browser-suite owner adapter around the existing live-demo lifecycle.
  It validates the selected scenario and caller-supplied configuration before allocation, launches
  the lifecycle, generates the generation-bound Sysadmin private input after installation produces
  its claim context, validates that input before Chromium, runs one visible production `dist/` HTTPS
  journey, and records lifecycle and typed-cleanup results.
- Acceptance: invalid selection and caller configuration fail before allocation; generated
  generation-bound input validates before Chromium; success and synthetic Playwright failure each
  produce lifecycle and scoped typed-cleanup receipts.

### WP-PROF-BS1-H1: Consolidate browser entry

- Owner: coder.
- Deliverable: `run_playwright_tests.sh`, Playwright configuration, and production build handoff use
  H0 for default, file, grep, and named-scenario selection.
- Acceptance: each supported selection creates a fresh stack and the gateway serves the same `dist/`
  artifact that direct browser development uses.

### WP-PROF-BS1-H2: Freeze baseline and isolation

- Owner: architect, then expert coder after design acceptance.
- Deliverable: one typed scenario contract defining baseline, namespace, UI actions, user-visible
  observation, optional receipt claim, and failure cleanup.
- Acceptance: every scenario can run from its declared baseline without predecessor state or
  caller-selected product-state APIs.

### WP-PROF-BS1-H3: Verify origin and cleanup

- Owner: tester.
- Deliverable: exact labelled container, volume, network, temporary-artifact, temporary-process,
  origin, and repeat-run inventory oracles around H0/H1 invocations.
- Acceptance: every run reports the production HTTPS origin and leaves zero suite-labelled
  containers, volumes, networks, temporary artifacts, and background processes; a repeat run starts
  cleanly.

### WP-PROF-BS1-C0: Establish aggregate Validation

- Owner: integrator.
- Deliverable: expand the user-facing `all_test.sh` front door to source the repository environment,
  run pytest, record a distinct `build.sh` receipt, run Rust and codebase checks, invoke
  `local_stack.py acceptance` exactly once, and finish with both diff checks. The aggregate delegates
  browser behavior to `local_stack.py acceptance`; it has no direct `run_playwright_tests.sh` call.
  It removes the aggregate's duplicate final compatibility live-demo lane after automated consumer
  proof, retains the wrapper until R1 retirement, updates `TEST_EVIDENCE_MODEL.md` current claims to
  the production real-stack and narrow-unit fake split, and aligns current operational docs while
  preserving historical records.
- Acceptance: ordered aggregate receipts identify environment, pytest, build, Rust, codebase,
  one acceptance invocation, and both diff results. Consumer proof shows the wrapper has no duplicate
  aggregate lane and remains available through its R1 retirement boundary.

### WP-PROF-BS1-B0: Establish catalog and multi-scenario owner

- Owner: expert coder.
- Deliverable: promote the H2 contract to the closed `schemaVersion: 2` private ABI and place its
  generic primitive and registry loader in `tests/e2e/e2e_browser_scenario_contract.py`. Its closed
  `sysadminRequirement` values are `not_required`, `unclaimed`, and `claimed`; the registry rejects
  duplicate scenario IDs, spec paths, and exclusive seed mutations. The flat provider modules
  `tests/e2e/e2e_browser_scenarios_catalog.py`,
  `tests/e2e/e2e_browser_scenarios_legacy_live_demo.py`,
  `tests/e2e/e2e_browser_scenarios_auth.py`,
  `tests/e2e/e2e_browser_scenarios_instructor.py`, and
  `tests/e2e/e2e_browser_scenarios_learner.py` provide explicit deterministic provider order:
  `legacy_live_demo`, `auth`, `instructor`, then `learner`. The legacy-live-demo provider registers
  the current real `live_demo` as the V2 `unclaimed` contract and reserves its exclusive seed mutations, including
  `sysadmin_first_claim` and `avery_instructor_approval`, so the catalog is nonempty without a
  selection fallback. The three family providers begin empty. The suite owner resolves a selection
  to one or many contracts, writes a distinct canonical input and origin-receipt path for each child,
  and records ordered public scenario receipts. It sequences the visible Sysadmin first-claim setup
  only for a declared `unclaimed` target or before a `claimed` target in a fresh stack. Generic
  browser helpers move to `tests/playwright/e2e/real_stack_ui.ts`, and the generic parser moves to
  `browser_suite_live_config.ts`; a temporary re-export remains available through R1.
- Acceptance: offline contract tests prove deterministic provider order, the nonempty legacy
  `live_demo` registration, exact ID/path selection, grep anchoring, and duplicate/exclusive-surface
  rejection before allocation. Owner tests prove a complete selection uses one lifecycle with
  distinct V2 child inputs and origin paths, exact child argument arrays, and ordered receipts.
  Focused tests prove an `unclaimed` target has no extra setup, a `claimed` target receives one
  visible first-claim setup child, and proof appears only in the unclaimed child input/environment.
  TypeScript tests prove each spec receives only its own canonical input and origin path.

  Package ownership is deliberately disjoint after B0: I1 owns
  `e2e_browser_scenarios_instructor.py`, `e2e/instructor_authoring.spec.ts`, and I1-local tests; L1
  owns `e2e_browser_scenarios_learner.py`, `e2e/learner_delivery.spec.ts`, and L1-local tests; A1 owns
  `e2e_browser_scenarios_auth.py`, `e2e/sysadmin_first_claim.spec.ts`,
  `e2e/auth_authorization.spec.ts`, and A1-local tests. Family packages consume B0's generic
  helpers and do not change the owner, catalog, aggregate commands, shared seed data, or another
  family's files.

### WP-PROF-BS1-I1: Migrate instructor behavior

- Owner: expert coder.
- Deliverable: UI-first instructor scenarios for questions, courses, assignments, roster changes,
  teaching operations, and real concurrent edit conflicts that carry behavior value.
- Acceptance: visible actions create namespaced state; reload and a second authorized instructor
  observe the intended result. PostgreSQL or MinIO inspection appears only for a stated service claim.

### WP-PROF-BS1-L1: Migrate learner behavior

- Owner: expert coder.
- Deliverable: UI-first learner scenarios that create their course/assignment prerequisites through
  normal visible instructor and student sessions, then cover issue, response, grade, feedback,
  repeat, leave/return, and summary behavior with value in the inventory.
- Acceptance: a fresh or second authorized learner session observes the expected durable product
  result; browser traffic preserves answer and credential boundaries.

### WP-PROF-BS1-A1: Migrate auth and authorization

- Owner: expert coder.
- Deliverable: real seeded Student/Instructor session entry, generation-bound Sysadmin first claim
  with a virtual authenticator, logout/re-authentication, and role/cross-course route scenarios.
  A1 atomically removes the legacy `live_demo` catalog entry while registering
  `sysadmin_first_claim` and `auth_authorization`, transferring the exclusive claim and Avery
  approval mutations to their owning A1 contracts.
- Acceptance: visible role workflows and direct navigation demonstrate server enforcement; protected
  transport and durable mutation checks use network observation where that exact claim matters. The
  registry remains nonempty and valid throughout the replacement, with each exclusive mutation held
  by exactly one A1 contract.

### WP-PROF-BS1-S1: Add semantic persistence receipts

- Owner: tester.
- Deliverable: read-only semantic Store, object-storage, worker, or renderer verifiers for the
  explicitly named claims left ambiguous by product-visible evidence.
- Acceptance: every receipt identifies the preceding UI action and the service claim it proves;
  verifiers avoid schema-layout and row-count assertions.

### WP-PROF-BS1-U1: Isolate narrow unit behavior

- Owner: tester.
- Deliverable: retain only decoder, serialization, formatting, error-mapping, public-reference,
  checkpoint, and other browser-independent behavior tests whose values are literal or supplied by
  test-local narrow fakes outside the application runtime graph. The U1 behavior-value allocation
  records every remaining mock Playwright or runtime consumer with its meaningful behavior and its
  later owner: I1 for instructor/teaching mutations, L1 for learner delivery and recovery, A1 for
  sign-in and role boundaries, S1 for service-specific claims, V1 for retained visual behavior, F1
  for lifecycle faults, X1 for conflicts, or R1 for fake-application implementation coverage with no
  product behavior to preserve. U1 does not retain fake-server implementation coverage.
- Acceptance: each retained U1 test protects one isolated behavior using literal fixtures or a
  test-local focused fake; no retained U1 test imports `src/api/mock/**`. The allocation is complete
  for every remaining mock Playwright/runtime consumer and gives each one a later real-scenario owner
  or an R1 deletion disposition. U1's focused Node/Rust checks pass without a browser application,
  production stack, or mock-server runtime.

### WP-PROF-BS1-X1: Exercise concurrent-session behavior

- Owner: expert coder.
- Deliverable: two ordinary visible sessions that create a revision conflict through the product's
  real concurrency boundary.
- Acceptance: the initiating user sees the recovery state and the observing session verifies the
  product result after reload; the scenario creates its own namespaced resources.

### WP-PROF-BS1-F1: Exercise real failure behavior

- Owner: expert coder.
- Deliverable: visible concurrent-session conflict scenarios and lifecycle-owner controls for worker,
  renderer, or gateway conditions that have behavior value.
- Acceptance: visible recovery preserves the specified user work; a read-only service receipt is
  included only for the service/retry claim under test.

### WP-PROF-BS1-V1: Capture canonical visual evidence

- Owner: playwright operator.
- Deliverable: selected UI-created screenshot scenarios, canonical viewport capture, and provenance
  records for the suite HTTPS origin and production artifact.
- Acceptance: the manifest contains only captured artifacts from real scenarios; `image_evaluator`
  returns an automated report with resolved findings, and privacy checks keep private material out of
  retained evidence.

### WP-PROF-BS1-R1: Retire the parallel browser application

- Owner: integrator.
- Deliverable: one browser build graph and production client path after removal of the alternate
  artifact, static helper, transport selection, browser login selection, mock globals, and runtime
  mock handlers represented as obsolete in the U1 behavior-value allocation. After the catalog
  inventory proves it unreachable, R1 also removes the retired legacy live-demo spec and source.
- Acceptance: build and consumer inventories show one production browser artifact; Playwright creates
  product state through visible PLE workflows against real services, and no catalog or source consumer
  reaches the retired legacy live-demo path.

### WP-PROF-BS1-C1: Integrate acceptance

- Owner: integrator.
- Deliverable: `local_stack.py acceptance` invokes the canonical browser suite once, preserves
  distinct non-browser service oracles, and updates operational documentation.
- Acceptance: final Validation passes, required browser selections use the real suite, canonical
  screenshots carry real-origin provenance, and repeat-run checks show suite-owned cleanup.

## Acceptance gates

The primary gate is one production browser artifact with one real-stack browser path. The following
subordinate gates prove that outcome:

- Production origin: every Playwright page and screenshot uses the disposable HTTPS gateway serving
  `dist/`; the origin verifier rejects direct service and external origins.
- UI-created state: scenario actions use visible PLE workflows, with the frozen seed baseline and
  harness-only infrastructure inputs as the declared exceptions.
- Product-visible persistence: scenario results survive reload, a new authorized session, or a
  different authorized role as appropriate to the behavior.
- Service-specific receipts: read-only PostgreSQL, MinIO, worker, renderer, and network evidence
  appears only for a requirement about that service boundary.
- Screenshot provenance: canonical artifacts come from selected production-browser scenarios and pass
  metadata, privacy, manifest, and automated image-evaluator checks.
- Focused selectability: named scenario, file, and grep selection use a fresh disposable stack and
  the same scenario contract as complete runs.
- Cleanup: the suite removes its labelled containers, volumes, networks, temporary artifacts, and
  background processes and a repeat run begins without interference. Image pruning remains permitted
  lifecycle hygiene after running-container inspection; it is not a browser-architecture acceptance claim.
- Aggregate acceptance: `local_stack.py acceptance` invokes the browser suite once and retains only
  distinct service oracles with claims outside that suite.

## Validation

`WP-PROF-BS1` closes only after this final-material-tree Validation suite is green:

```bash
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
source source_me.sh && python3 local_stack.py acceptance
git diff --check
git diff --cached --check
```

Focused development uses the smallest relevant owner test, followed by a selected real-stack
scenario through `./run_playwright_tests.sh --build <selection>`. H0 also runs a deliberate child
failure to prove cleanup; C1 runs the complete suite twice to prove noninterference. Required live
gates run in their declared disposable environment and report their actual result under
[TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md).

## Migration and cleanup policy

PLE is pre-production, so the implementation improves schemas, contracts, and ownership boundaries
directly when the canonical architecture benefits. A migration allocation is needed only when the
real persistence model changes; testing alone does not create schema work.

All local Podman images and project-named simulated-data volumes are disposable. The lifecycle owner
uses exact labels, targets, and running-container inspection to keep cleanup inspectable. It removes
the resources created by its invocation. Image pruning is available after that check as ordinary
lifecycle hygiene.

## Risks and recovery

| Risk | Recovery rule |
| --- | --- |
| A scenario relies on another scenario | Run it focused from a fresh stack; add its missing UI setup and namespace before closing it |
| UI setup lacks a product workflow | Record the missing workflow, keep the concern harness-scoped, and create product state through the visible path when the workflow ships |
| A live service claim is ambiguous | Use product-visible evidence first and add the smallest read-only service receipt that states the exact boundary |
| A real suite failure leaves resources | Preserve private diagnostics, run typed labelled cleanup, then run a fresh focused scenario before accepting the fix |
| Visual capture changes evidence quality | Re-capture from the accepted real scenario and use the image-evaluator report to resolve findings |

## Closure record

Each work package records its owner, changed boundary, automated command/result, generated artifacts,
and remaining dependencies in the normal changelog and handoff. `WP-PROF-BS1-C1` performs the
requirement-by-requirement closure audit, advances the current-package registry, and archives this
plan after the final Validation suite passes.
