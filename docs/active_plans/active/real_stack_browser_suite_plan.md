# Plan: Canonical real-stack browser suite

## Primary outcome

`WP-PROF-BS1` gives PLE one production browser artifact and one disposable real-stack browser path.
Playwright, canonical screenshots, direct browser development, and aggregate acceptance all visit the
same production `dist/` bundle through the private HTTPS gateway. That browser uses the real Rust
API, PostgreSQL, MinIO, worker, renderer, authentication, authorization, and seeded live-demo data.
The developer profile uses deployment-gated seeded production-auth entry. Any visitor can select one
of five fixed personas; the server resolves that persona to the ordinary PLE account, account
session, course selection, tenant session, and stored role state. Generic passkey enrollment and
sign-in are exercised inside independent Elena Instructor and Morgan Sysadmin scenarios, so daily
browser work and canonical evidence share one browser application.

The suite owns one exact Compose project, `ple-live-demo-browser`. Each invocation regenerates a fresh
disposable installation from the declared seed baseline under that fixed project: a focused invocation
uses one lifecycle, and a complete invocation shares one lifecycle across its selected scenarios.
Every scenario remains independently runnable: it starts from the regenerated seed baseline, creates a
unique namespace through visible PLE workflows, and observes its own results without a prior scenario.
The live-demo E2E owner becomes the shared browser-suite owner. Narrow decoder, serialization, and
error-mapping tests remain fast unit tests outside the product browser runtime graph.

This package and its frozen-scope successor, `WP-PROF-T3`, were accepted on 2026-08-22. The sole
current professor package is recorded only in [implementation_status.md](../implementation_status.md),
which now names `WP-PROF-D1` after accepted WP-PROF-T5 item-pool delivery. The accepted BS1 closure
remains the historical nine-scenario, 51-artifact record; T3 separately extends that evidence to ten
scenarios and 63 artifacts.

## Evidence and boundaries

The suite proves the user-facing system first. A UI-created effect is normally demonstrated by a
reload, a new authorized session, or a different authorized role observing its product result.
Read-only service receipts supplement that evidence only where the requirement is specifically about
a service boundary:

| Claim                                               | Primary evidence                                                     | Read-only receipt when needed                            |
| --------------------------------------------------- | -------------------------------------------------------------------- | -------------------------------------------------------- |
| Course, assignment, roster, or response persistence | Reload or second authorized session                                  | PostgreSQL only for a database/RLS claim                 |
| Media upload and delivery                           | UI upload followed by an authorized visible load                     | MinIO for object lifecycle or access-policy claim        |
| Render or grade result                              | Visible rendered problem, result, and recovery                       | Renderer or worker for provider/retry claim              |
| Role boundary                                       | Allowed role completes the journey; denied role sees the safe result | Server/network observation for protected-transport claim |
| Screenshot provenance                               | Image metadata records suite origin, scenario, and production bundle | Gateway-origin verifier                                  |

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

| Baseline record          | Fixed identifier or state                                                 | Allowed scenario use                                                |
| ------------------------ | ------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Demo tenant              | `00000000-0000-0000-0000-000000000100`                                    | Tenant context and baseline reads                                   |
| Dr. Elena Rivera         | `00000000-0000-0000-0000-000000000101`, seeded Instructor                 | Ordinary Instructor entry and baseline reads                        |
| Mary Okafor              | `00000000-0000-0000-0000-000000000102`, seeded Student                    | Base-course completed-run observation                               |
| Jack Chen                | `00000000-0000-0000-0000-000000000103`, seeded Student                    | Base-course in-progress-run observation                             |
| Avery Singh              | `00000000-0000-0000-0000-000000000104`, unapproved account                | Sysadmin approval journey and practice-course baseline read         |
| Morgan Reyes             | `00000000-0000-0000-0000-000000000105`, seeded Sysadmin                   | Direct Sysadmin selection, ordinary authorization, and generic passkey login |
| Biochemistry Base Course | `2026-01-01` through `2099-12-31`, `America/Chicago`                      | Baseline course, roster, activity, and assignment reads             |
| Genetics Practice Course | Seeded course with Morgan as Instructor and Avery as Student              | Cross-course authorization and baseline reads                       |
| Peptide bond publication | Published `peptide_bond_geometry` question and one-point assignment       | Visible problem, delivery, and grading baseline reads               |
| WebWork catalog item     | One provenance-validated catalog publication and immutable private source | Visible Library discovery only; all teaching state is UI-created    |

Mary has one completed correctly graded deterministic-seed-17 run; Jack has one open deterministic-
seed-23 attempt. Elena, Mary, and Jack are active Base Course members. The five closed selector
personas are Elena Instructor, Mary Student, Jack Student, Avery Student, and Morgan Sysadmin. The fixed baseline has no
cross-course memberships beyond the stated practice-course membership. The one provenance-validated
WebWork catalog item is recorded in [LIVE_DEMO_SPEC.md](../../LIVE_DEMO_SPEC.md) and is the sole
WebWork publication bootstrap; course, assignment, roster, invitation, run, and submission state are
created through visible UI. Every selector entry establishes an ordinary server session; the server
continues to derive roles, tenant context, memberships, and authorization from stored PLE state.
The installation generation, cryptographic service inputs, and selected infrastructure faults are
harness concerns rather than product-state setup.

The scenario contract uses `schemaVersion: 2`. Each scenario declares its closed personas,
baseline reads, UI-created resources, visible observation, optional service receipt, and screenshot
states. The owner writes only that per-scenario private input and origin receipt. Every scenario
starts independently through direct role entry and owns any generic passkey interaction it needs.
`direct_role_entry` owns Morgan's direct-role and passkey journey; `auth_authorization` owns the
multi-persona authorization journey and Elena's passkey journey.

Each scenario declares:

- its allowed baseline personas and baseline records;
- a scenario ID and collision-resistant namespace used in every UI-created name;
- the visible actions that create and mutate product state;
- the reload, cross-session, or cross-role observation that proves its outcome; and
- any read-only receipt and its service-specific claim.

The complete suite shares one regenerated installation for efficient service startup, while each
scenario creates and owns unique records in its namespace. Focused selection receives its own freshly
regenerated installation under the same fixed project and runs the same scenario contract. The
harness runs selected scenarios serially under the single-flight lease; the default order carries no
behavior dependency, but real browser-suite invocations never overlap. B2 makes these invocations
single-flight, so normal operation never selects a random-suffix compatibility project.

## Behavior-value inventory

The migration matrix classifies each browser case by the user or
contract behavior it protects: real-stack browser scenario, isolated unit test, existing service
oracle, or obsolete implementation detail. It records the successor and evidence location. The
matrix is a behavior-value inventory; legacy filenames, assertion counts, test doubles, and application
shapes do not define replacement scope.

## Small milestones

Each milestone is independently closable by a manager and fresh subagents. A manager dispatches
one owner per work package, runs the stated automated gate, records the result, and moves to the next
ready milestone. Visual review uses the repository `image_evaluator` role against captured artifacts;
it produces an automated structured report for the acceptance record. The plan has no human approval,
interaction, or inspection prerequisite.

| Milestone | Work package     | Outcome                                                                | Depends on              | Automated checkpoint                                                                                                                                           |
| --------- | ---------------- | ---------------------------------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| BS1-1     | `WP-PROF-BS1-H0` | Add a typed owner adapter around the existing lifecycle                | none                    | Caller-configuration rejection before allocation, generated-input validation before Chromium, one visible `live_demo` scenario, and lifecycle/cleanup receipts |
| BS1-2     | `WP-PROF-BS1-H1` | Route `run_playwright_tests.sh` through H0 and build `dist/`           | H0                      | Default and focused command selections reach a freshly regenerated HTTPS origin under the fixed project                                                        |
| BS1-3     | `WP-PROF-BS1-H2` | Freeze seed-baseline and scenario-isolation contracts                  | H0                      | Offline contract tests and plan-backed scenario manifest check                                                                                                 |
| BS1-4     | `WP-PROF-BS1-H3` | Add origin and suite-resource cleanup oracles                          | H0                      | Origin, labelled-inventory, process, and repeat-run checks                                                                                                     |
| BS1-5     | `WP-PROF-BS1-C0` | Expand `all_test.sh` into the aggregate Validation front door          | H1, H3                  | Ordered aggregate receipts and one `local_stack.py acceptance` invocation                                                                                      |
| BS1-6     | `WP-PROF-BS1-B0` | Establish the catalog and multi-scenario owner foundation              | H2, C0                  | Closed registry, exact selection, independent per-scenario input/origin receipts, and direct Sysadmin entry                                                    |
| BS1-7     | `WP-PROF-BS1-B1` | Prove generic passkeys in independent browser journeys                 | B0                      | Visible Elena and Morgan enrollment/sign-in, ordinary authorization, and repeat-run cleanup evidence                                                           |
| BS1-8     | `WP-PROF-BS1-B2` | Make the browser project single-flight with exact reset recovery       | B1                      | Fixed project, owner-labelled reset before regeneration and final cleanup, and sequential real-run proof                                                       |
| BS1-9     | `WP-PROF-BS1-I1` | Migrate one instructor authoring/course family                         | B0, C0                  | Reload and second-session behavior evidence                                                                                                                    |
| BS1-10    | `WP-PROF-BS1-L1` | Migrate one learner delivery/response family                           | B0, C0                  | Reload or second-session learner evidence                                                                                                                      |
| BS1-11    | `WP-PROF-BS1-A1` | Migrate ordinary auth, direct-role entry, and role-boundary families   | B2, B1, B0, C0          | Visible role/session and denial scenarios                                                                                                                      |
| BS1-12    | `WP-PROF-BS1-S1` | Add semantic persistence receipts where claims require them            | I1, L1, A1              | Read-only Store/service receipts tied to named claims                                                                                                          |
| BS1-13    | `WP-PROF-BS1-U1` | Map browser-independent behavior to isolated unit owners               | C0                      | Focused Node/Rust unit tests plus one-time runtime-consumer closure inventory                                                                                  |
| BS1-14    | `WP-PROF-BS1-X1` | Exercise real concurrent-session conflicts                             | I1, B0                  | Two-session UI conflict scenario                                                                                                                               |
| BS1-15    | `WP-PROF-BS1-F1` | Exercise lifecycle-controlled infrastructure recovery                  | C0 plus affected family | Owner-fault scenario and visible recovery                                                                                                                      |
| BS1-16    | `WP-PROF-BS1-V1` | Capture canonical screenshots from accepted real scenarios             | I1, L1, X1, F1          | Origin/provenance verifier and image-evaluator report                                                                                                          |
| BS1-17    | `WP-PROF-BS1-R1` | Retire the alternate browser application and runtime graph             | U1, X1, F1, V1          | One-time build/consumer closure inventory and canonical suite run                                                                                              |
| BS1-18    | `WP-PROF-BS1-D1` | Converge developer entry on the seeded production-auth HTTPS profile   | B2, C0                  | Daily-workflow experiment, one production `dist/` artifact, and seeded-auth lifecycle receipts                                                                 |
| BS1-19    | `WP-PROF-BS1-W1` | Add the catalog-only WebWork baseline and canonical delivery scenario  | B0, I1, L1, S1          | UI-first instructor/learner flow, one renderer-call receipt, and fresh-session persistence                                                                     |
| BS1-20    | `WP-PROF-BS1-Q1` | Add canonical assignment-question replacement behavior                 | B0, I1, L1, X1          | Issued-problem stability, visible replacement, stale-editor reload, and new-run replacement evidence                                                           |
| BS1-21    | `WP-PROF-BS1-R2` | Retire every remaining alternate browser owner and browser tail        | R1, D1, W1, Q1          | One-time static consumer closure inventory, canonical complete suite, and retained non-browser oracle receipts                                                |
| BS1-22    | `WP-PROF-BS1-C1` | Invoke the suite once from aggregate acceptance and complete closure   | B2, C0, R2              | Final Validation and repeat-run cleanup evidence                                                                                                               |

The manager may dispatch H1 and H2 after H0. C0 follows accepted H1 and H3. B0 follows C0 and
supplies the shared catalog, multi-scenario owner, and generic browser helpers. B1 follows B0 and
proves generic passkeys inside independent Elena and Morgan journeys. B2 then closes the fixed-project
single-flight and exact-reset boundary before A1's connected real acceptance. I1 and L1 can proceed
independently after B0; A1 starts its implementation after B0 but its connected real acceptance waits
for B1 and B2. U1 can proceed after C0. Each family creates its own UI state from the H2 baseline. S1, X1, F1,
and V1 use accepted scenario contracts and separate owned files. D1, W1, and Q1 follow their
named foundations and complete before browser-owner integration. R1 and R2 integrate serially;
C1 follows R2 after the B2 gate.

## Work-package contracts

### WP-PROF-BS1-H0: Add the typed owner adapter

- Owner: expert coder.
- Deliverable: a closed, typed browser-suite owner adapter around the existing live-demo lifecycle.
  It validates the selected scenario and caller-supplied configuration before allocation, launches
  the lifecycle, generates the generation-bound scenario private input after installation completes,
  validates that input before Chromium, runs one visible production `dist/` HTTPS
  journey, and records lifecycle and typed-cleanup results.
- Acceptance: invalid selection and caller configuration fail before allocation; generated
  generation-bound input validates before Chromium; success and synthetic Playwright failure each
  produce lifecycle and scoped typed-cleanup receipts.

### WP-PROF-BS1-H1: Consolidate browser entry

- Owner: coder.
- Deliverable: `run_playwright_tests.sh`, Playwright configuration, and production build handoff use
  H0 for default, file, grep, and named-scenario selection.
- Acceptance: each supported selection creates a freshly regenerated installation under
  `ple-live-demo-browser`, and the gateway serves the same `dist/` artifact that direct browser
  development uses.

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
- Deliverable: keep the user-facing `all_test.sh` front door as four ordered gates: `check_rust.sh`,
  `check_codebase.sh`, the repository pytest lane, and one `local_stack.py acceptance` invocation.
  The acceptance lifecycle owns the production build and delegates browser behavior to the fixed
  real-stack suite; the aggregate has no separate build command or direct Playwright invocation.
- Acceptance: the four commands run in that order, the acceptance gate reports the production
  browser build and canonical HTTPS suite, and no duplicate browser lane is introduced.

### WP-PROF-BS1-B0: Establish catalog and multi-scenario owner

- Owner: expert coder.
- Deliverable: the catalog and owner use a closed `schemaVersion: 2` private ABI. Every child gets
  a distinct input and origin receipt containing only scenario ID, namespace, HTTPS origin, closed
  personas, baseline reads, visible observation, and declared optional receipts or faults. The
  registry rejects duplicate IDs, paths, and exclusive seed mutations.
- Current contract: `direct_role_entry` is Morgan's independently runnable Sysadmin scenario and
  `auth_authorization` is the independently runnable multi-persona authorization scenario. Both
  begin from the regenerated baseline, use visible seeded identity and course selection, and retain
  no browser credential or authenticated state from another scenario.
- Acceptance: focused contract and owner tests prove deterministic exact selection, ordered
  independent execution, strict input decoding, origin receipts, and the exact closed field set.
  TypeScript tests prove each spec consumes only its own canonical input and origin path.

### WP-PROF-BS1-B1: Direct generic passkey browser evidence

- Owner: expert coder.
- Current contract: generic passkeys are ordinary account-security behavior. `direct_role_entry`
  enrolls Morgan's passkey through the visible UI, signs out, signs in through the visible passkey
  path, selects Genetics, and proves Sysadmin authorization. `auth_authorization` performs the
  same in-scenario ceremony for Elena before proving Instructor authorization. The browser owner
  owns per-child private inputs, origins, reset, and cleanup; it neither transfers credentials nor
  injects prerequisite state.
- Acceptance: isolated generic-passkey journeys prove session replacement and retained
  server-authorized role behavior. Public receipts contain only bounded public lifecycle and origin
  evidence; private credentials, cookies, and file paths remain within the owner workspace.

### WP-PROF-BS1-B2: Make the browser project single-flight with exact reset

- Owner: expert coder.
- Depends on: B1.
- Deliverable: close the browser-suite lifecycle around one exact Compose project,
  `ple-live-demo-browser`, with one host-held nonblocking lease, one fixed private workspace, and
  an adapter-owned exact destructive reset. Every invocation resets the disposable browser fixture
  before generating its next workspace and resets it again during final cleanup.
- Exact project policy: `DisposableOwnerPolicy(owner="live-demo-browser")` accepts exactly
  `ple-live-demo-browser` through the closed canonical Compose inputs and rejects suffixes, prefixes,
  CLI values, environment overrides, and test-caller substitutions. The rendered services, volumes,
  and networks carry `org.peptidyle.e2e.owner=live-demo-browser`. Within the exact Compose-project-
  labelled inventory, discovery requires the matching owner label, declared browser topology, and
  exact Compose-generated names. A missing, conflicting, or foreign required label on a discovered
  resource fails closed before mutation. Resources without an exact Compose project label are
  outside this fixture's reset authority. The existing per-run capability label may continue to
  establish normal-run integrity, but it is not reset authority.
- Lease boundary: create the mode-0600 non-inheritable lock under the verified mode-0700
  `target/live-demo-browser` root and hold `flock(LOCK_EX | LOCK_NB)` through reset, workspace
  creation, launch, reporting, final reset, and cleanup. Acquire it after pure selection/contract
  validation but before port checks, workspace/provider/build work, lifecycle-adapter invocation,
  Podman, or PostgreSQL work. Contention fails immediately with a bounded `BrowserSuiteError`; it
  never polls, queues, creates workspace contents, checks ports, builds, discovers a provider, or
  touches Podman/PostgreSQL. Release the descriptor last from the outermost `finally`.
- Reset boundary: inventory only the exact labelled project. An empty inventory is already reset.
  For a valid nonempty inventory, after validating the fixed project, fixed owner label, declared
  services, and exact Compose-generated volume and network names, the adapter removes the exact
  discovered container IDs, then the exact volume names, then the exact network names; it
  re-inventories and requires an empty inventory. It may remove only unused browser fixture build
  tags; image pruning remains ordinary lifecycle hygiene. The reset never accepts caller project,
  prefix, manifest, or general Compose arguments.
- Workspace boundary: retain only the held lock in `target/live-demo-browser`; clear and recreate
  one fixed private `workspace` after reset, then generate the strict runtime manifest and
  installation-scoped capabilities needed for the next installation. Final cleanup repeats the exact reset, verifies the
  labelled inventory is empty, removes workspace contents, reports only project/reset/final-empty
  status, and releases the lease.
- Crash and error behavior: a new lease holder repeats the same exact reset before any regeneration,
  regardless of where the prior invocation stopped. Reset is idempotent during reset, launch, browser
  work, and final cleanup. A malformed or foreign-labelled resource discovered in the exact project
  produces a bounded ownership error before mutation; a failed reset leaves only bounded diagnostics
  for the next attempt. The lifecycle has one reset path and no persisted interrupted-run protocol.
- Acceptance: focused offline tests prove exact project/policy and owner/topology labels; immediate
  second-invocation failure before ports, workspace, build, provider, Podman, and PostgreSQL work;
  reset of valid stale resources to an empty inventory; foreign/malformed labels on discovered exact-
  project resources fail before mutation; fixed-workspace regeneration; final reset and cleanup; and
  reset ordering after each controlled fault injection. Two fully automated sequential real-stack invocations prove
  the same exact project, regenerated seed baseline, and empty final labelled container, volume,
  network, private-workspace, and owner-process inventory. The gate requires no human interaction
  or inspection.

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
- Deliverable: real seeded entry for the five fixed personas, ordinary account and course sessions,
  in-scenario generic passkey enrollment and sign-in for Elena Instructor and Morgan Sysadmin, and
  role/cross-course route scenarios. The auth catalog registers `direct_role_entry` and
  `auth_authorization`; Avery's instructor-approval mutation remains owned only by
  `auth_authorization`.
- Acceptance: visible role workflows and direct navigation demonstrate server enforcement; protected
  transport and durable mutation checks use network observation where that exact claim matters. The
  registry remains nonempty and valid throughout the replacement, with each exclusive mutation held
  by exactly one A1 contract. Each scenario starts independently and does not consume another
  scenario's browser state, proof, or credential.

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
  for lifecycle faults, X1 for conflicts, or R1 for browser-graph integration coverage.
- Acceptance: each retained U1 test protects one isolated behavior using literal fixtures or a
  test-local focused fake; no retained U1 test imports `src/api/mock/**`. The one-time allocation
  records every allocated Playwright/runtime behavior and gives each one a real-scenario or focused
  unit owner. U1's focused Node/Rust checks pass as browser-independent tests; the allocation itself
  is one-time migration evidence.

### WP-PROF-BS1-X1: Exercise concurrent-session behavior

- Owner: expert coder.
- Deliverable: two ordinary visible sessions that create a revision conflict through the product's
  real concurrency boundary.
- Acceptance: the initiating user sees the recovery state and the observing session verifies the
  product result after reload; the scenario creates its own namespaced resources.

### WP-PROF-BS1-F1: Exercise real failure behavior

- Owner: expert coder.
- Deliverable: one visible learner saved-response recovery scenario with lifecycle-owner control of
  the fixed stack's real HTTPS gateway.
- Acceptance: visible recovery preserves the learner's selected response, keyboard retry completes,
  a fresh learner session observes the persisted score, and the typed lifecycle receipt proves the
  declared gateway fault was injected, recovered, and completely cleaned up.

### WP-PROF-BS1-V1: Capture canonical visual evidence

- Owner: playwright operator.
- Deliverable: the single JSON corpus authority defines 51 ordered, nested role-and-journey
  artifacts from accepted real-stack scenarios. Each artifact records its scenario, UI-created state,
  role, journey, ordered journey step, privacy checks, production-browser path, and one of the named
  `laptop`, `tablet`, `iphone_pro`, or `square` viewport profiles. The corpus supersedes each of the
  47 prior PNG paths exactly once and retains all canonical evidence under the nested screenshot
  directory structure. The mapping of the 47 superseded PNG paths is one-time migration evidence,
  not a permanent path-count test.
- Acceptance: capture runs use the accepted scenario contracts and the disposable HTTPS gateway
  serving production `dist/`; provenance records the same production origin and bundle for every
  artifact. The publisher atomically publishes the complete nested corpus, the verifier confirms
  manifest, origin, digest, path, coverage, and privacy invariants, and `image_evaluator` returns an
  automated report with resolved findings. Retained evidence contains no private material.

### WP-PROF-BS1-R1: Establish the production browser graph

- Owner: integrator.
- Deliverable: one browser build graph and production client path, with browser-independent behavior
  assigned to the U1 focused-unit owners.
- Acceptance: the one-time build and consumer closure inventory shows one production browser artifact;
  Playwright creates
  product state through visible PLE workflows against real services.

### WP-PROF-BS1-D1: Converge developer browser entry

- Owner: expert coder.
- Depends on: B2, C0.
- Deliverable: immediately make the default developer browser entry use disposable seeded
  production-auth HTTPS with deployment-gated server-resolved persona entry and the ordinary
  Sysadmin passkey ceremony.
  The WebWork and replica service oracles use seeded production authentication with their narrow
  service-observability capabilities. Unit/static tests stay with their owning implementation.
  The bounded developer-profile experiment then proves daily seeded Student and Instructor work, the
  real Sysadmin direct-role/passkey flow, reload and re-entry, real course authoring and learner submission,
  the production `dist/` artifact, normal session/auth traffic, and empty suite-labelled cleanup. This
  package is distinct from R1's production browser-build ownership.
- Acceptance: fresh Student and Instructor browser contexts complete normal daily workflows and the
  seeded Sysadmin completes real direct-role/passkey entry; no request or served production bundle
  exposes `/api/auth/login`, `local-login.txt`, or local-development credentials. Build and lifecycle
  receipts show one production `dist/` artifact and one production-shaped auth graph for developer and
  canonical browser use. The WebWork and replica service oracles complete seeded-production-auth
  migration, and `local_stack.py start`, browser commands, canonical-suite inputs, service oracles,
  and developer documentation contain only the production account/session graph.

### WP-PROF-BS1-W1: Add canonical WebWork delivery

- Owner: expert coder.
- Depends on: B0, I1, L1, S1.
- Deliverable: add one provenance-validated catalog-only WebWork publication to the exact frozen
  baseline and [LIVE_DEMO_SPEC.md](../../LIVE_DEMO_SPEC.md), then add the canonical
  `webwork_delivery` scenario. Provider/source publication is irreducible initial-seed infrastructure:
  the baseline installer validates tracked provenance, publishes one immutable private source/catalog
  record, and exposes no private source, object key, renderer identity, credential, answer, or opaque
  provider ID to Playwright. The scenario then uses visible PLE UI to find the public Question ID and
  creates its namespaced course, assignment, roster, invitation, learner run, and submission state
  through visible UI. It verifies durable completion in a fresh session and records one narrow,
  non-sensitive renderer-call receipt; cache/replay, grading, outage, and privacy service claims
  remain bounded non-browser oracles.
- Acceptance: the exact frozen baseline and `LIVE_DEMO_SPEC.md` identify the one
  provenance-validated catalog-only item. The scenario runs from that fresh baseline under the fixed
  owner, creates all course, assignment, roster, invitation, run, and submission state through visible
  UI, preserves same-origin and redaction boundaries, and records exactly one post-issuance
  `renderer_call` witness. Its fresh authenticated session observes the completed result without
  depending on prior scenario state.

### WP-PROF-BS1-Q1: Preserve issued-question replacement

- Owner: expert coder.
- Depends on: B0, I1, L1, X1.
- Deliverable: add the canonical `assignment_question_replacement` scenario. Elena creates a
  namespaced second question, assignment, and invitation through visible UI; Mary starts a run; a
  second Elena session visibly replaces the assignment question; the stale first editor reloads the
  authoritative revision; Mary reloads and retains her issued original while a new run receives the
  replacement. This is distinct from grade-settings conflict behavior.
- Acceptance: focused and complete invocations start from a fresh stack, create their own UI state,
  demonstrate the issued learner problem remains stable, visibly show the instructor replacement and
  stale-editor reload, and show the replacement only for a newly issued learner run.

### WP-PROF-BS1-R2: Establish the canonical browser owner

- Owner: integrator.
- Depends on: R1, D1, W1, Q1.
- Deliverable: use `playwright.config.ts` as the canonical browser configuration and
  `e2e_browser_suite_owner.py` as the one fixed real-stack browser owner. W1/Q1 own WebWork and QTI
  browser behavior; narrow units own pure parser and generic interaction-helper behavior. Chapter One
  semantic claims remain with the fixed lifecycle seed/manifest and Rust behavior tests. Keep the
  browser-free WebWork and replica service oracles as distinct boundaries; the WebWork script has no
  browser exports or Playwright tail.
- Acceptance: a one-time static closure inventory finds every browser-launch command under the
  canonical owner; all retained browser specs are
  catalog-owned `tests/playwright/e2e/*.spec.ts` children of `playwright.config.ts`. The canonical
  complete suite passes under one fixed stack, and retained WebWork, replica, database/RLS, Rust, and
  renderer evidence makes only its distinct non-browser claims.

### WP-PROF-BS1-C1: Integrate acceptance

- Owner: integrator.
- Depends on: B2, C0, R2.
- Deliverable: `local_stack.py acceptance` invokes exactly one canonical browser-suite command,
  followed serially by the browser-free WebWork and replica service oracles, and updates operational
  documentation.
- Acceptance: final Validation passes, required browser selections use the real suite, canonical
  screenshots carry real-origin provenance, aggregate acceptance contains exactly one browser
  invocation, and repeat-run checks show suite-owned cleanup.

## Acceptance gates

The primary gate is one production browser artifact with one real-stack browser path. The following
subordinate gates prove that outcome:

- Production origin: every Playwright page and screenshot uses the disposable HTTPS gateway serving
  `dist/`; the origin verifier rejects direct service and external origins.
- One browser application: developer entry and the canonical suite use the same production `dist/`
  client and production-shaped session/auth graph. The deployment-gated seeded persona entry stays
  server-resolved; it exposes five fixed personas and preserves ordinary account, course, and
  server-owned authorization behavior.
- UI-created state: scenario actions use visible PLE workflows, with the frozen seed baseline and
  harness-only infrastructure inputs as the declared exceptions.
- Product-visible persistence: scenario results survive reload, a new authorized session, or a
  different authorized role as appropriate to the behavior.
- Generic passkeys: Elena Instructor and Morgan Sysadmin each enroll a passkey through the visible
  account UI, sign out, use the visible passkey sign-in path, choose an authorized course, and prove
  their stored server-authorized capability. Each journey is self-contained; receipts omit
  credentials, cookies, and private state.
- Service-specific receipts: read-only PostgreSQL, MinIO, worker, renderer, and network evidence
  appears only for a requirement about that service boundary.
- Screenshot provenance: the JSON-authoritative 51-artifact nested corpus comes from selected
  production-browser scenarios. Every artifact records the same HTTPS production origin and `dist/`
  provenance, its real UI-created scenario state, role, journey, ordered step, viewport profile, and
  privacy checks; the atomic publisher, verifier, and automated image evaluator confirm the complete
  corpus. Coverage mapping for the 47 superseded PNG paths is retained as one-time migration
  evidence, not as a permanent path-count assertion.
- Focused selectability: named scenario, file, and grep selection use a freshly regenerated
  installation under the fixed project and the same scenario contract as complete runs.
- B2 single-flight and exact reset: the exact project is `ple-live-demo-browser`; a contending
  invocation fails before ports, workspace, build, provider, Podman, or PostgreSQL activity and
  never queues. A valid stale owner-labelled inventory resets to empty before regeneration, and a
  foreign or malformed required label on a discovered exact-project resource fails closed before mutation.
- B2 sequential real proof: two fully automated selected real-stack runs execute serially, reset the
  same exact project before regeneration, expose the regenerated seed baseline, and each finishes
  with an empty labelled resource, fixed-workspace, and owner-process inventory before the next run.
- Cleanup: final exact reset removes the suite's labelled containers, volumes, networks, fixed
  workspace contents, and background processes. Image pruning remains permitted lifecycle hygiene
  after running-container inspection; it is not a browser-architecture acceptance claim.
- Aggregate acceptance: `local_stack.py acceptance` contains exactly one canonical browser invocation
  and only distinct non-browser service oracles with claims outside that suite.

## Validation

`WP-PROF-BS1` closes only after each of these four final-material-tree commands is green twice,
in the listed order, on the final material tree:

```bash
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
source source_me.sh && python3 local_stack.py acceptance
```

Focused development uses the smallest relevant owner test, followed by a selected real-stack
scenario through `./run_playwright_tests.sh --build <selection>`. B2 also runs its deterministic
lease/reset interruption matrix and two selected real scenarios sequentially under the fixed project.
H0 runs a controlled fault injection to prove cleanup; C1 runs the complete four-command suite twice
to prove noninterference. Required live gates run in their declared disposable environment and report
their actual result under
[TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md).

## Migration and cleanup policy

PLE is pre-production, so the implementation improves schemas, contracts, and ownership boundaries
directly when the canonical architecture benefits. A migration allocation is needed only when the
real persistence model changes; testing alone does not create schema work.

All local Podman images and project-named live-stack data volumes are disposable. The lifecycle owner
uses the exact project, fixed owner labels, declared topology, and running-container inspection to
keep reset and cleanup inspectable. It removes the browser fixture's resources before regeneration
and at final cleanup. Image pruning is available after that check as ordinary lifecycle hygiene.

## Risks and recovery

| Risk                                    | Recovery rule                                                                                                                                |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| A scenario relies on another scenario   | Run it focused from a freshly regenerated installation under the fixed project; add its missing UI setup and namespace before closing it     |
| UI setup lacks a product workflow       | Record the missing workflow, keep the concern harness-scoped, and create product state through the visible path when the workflow ships      |
| A live service claim is ambiguous       | Use product-visible evidence first and add the smallest read-only service receipt that states the exact boundary                             |
| A real suite failure leaves resources   | Acquire the lease, run the exact owner-labelled reset, verify an empty inventory, then run a fresh focused scenario before accepting the fix |
| Visual capture changes evidence quality | Re-capture from the accepted real scenario and use the image-evaluator report to resolve findings                                            |

## Closure record

Each work package records its owner, changed boundary, automated command/result, generated artifacts,
and remaining dependencies in the normal changelog and handoff. `WP-PROF-BS1-C1` performs the
requirement-by-requirement closure audit, advances the current-package registry, and archives this
plan after the final Validation suite passes.

`WP-PROF-BS1-C1` accepted the one-artifact/one-stack architecture on 2026-08-22. The nine-scenario
browser catalog, 51-artifact real-origin screenshot publication, image-evaluator re-review, WebWork
service oracle, two-API/one-PostgreSQL restart oracle, and exact cleanup receipts passed. The four
final-material-tree Validation commands passed twice in their required order; the handoff advanced to
`WP-PROF-T3`.
