# Changelog

## 2026-08-24

### Fixes and Maintenance

- Accepted WP-PROF-T5 item pools on the canonical live-demo product path. Elena creates an ordered
  mixed fixed/pool assignment using public Question IDs, previews a server-generated no-store draw,
  and the normal Student path issues, grades, resumes, and exposes immutable evidence for the
  selected items. Pre-issue structural edits serialize against first run; issued work stays
  immutable and later structural editing presents a visible new-assignment recovery path.
  Production HTTPS acceptance, refreshed screenshot provenance/privacy publication, independent
  visual review, and complete final Validation passed; the professor handoff advances to
  WP-PROF-D1 discovery.
- Hardened WP-PROF-T5 assignment authoring: create and update now establish the session,
  Instructor course authority, and update-route assignment binding before bounded JSON decoding;
  every refusal remains `no-store`. Shared Rust/TypeScript cardinality limits now cap ordered
  entries at 1,024, candidates per pool at 1,024, and total candidates at 8,192 before any catalog
  resolution; accessible authoring recovery feedback names the applicable correction.
- Accepted `WP-PROF-LD3` on the canonical live product path and advanced the professor queue to
  `WP-PROF-T5` item pools. The production HTTPS browser completed all ten visible role, passkey,
  authoring, preview, replacement, grading, learner-delivery, recovery, WebWork, and QTI-import
  scenarios; the isolated WebWork and replica-restart oracles also passed with exact durable replay
  and empty final resource, process, private-artifact, and workspace inventories. The 63-artifact
  production screenshot corpus passed privacy checks and independent visual review. Reserved
  `2026081826` for T5's one complete revision-checked pre-issue assignment-definition replacement,
  closed v1 pool-draw authority, and database serialization against first run creation.
- Initialized the browser owner's ordered execution contracts for ordinary and screenshot selections
  after optional screenshot replacement, preventing ordinary acceptance from raising an unbound-local
  failure; added a focused behavioral regression for the ordinary child-execution path.
- Selected the live demo as PLE's single product and acceptance path and retired the unaccepted
  `WP-PROF-T4` parallel execution sidecar before production. New current package `WP-PROF-LD3`
  converges ordinary assignment mutation, learner delivery, deterministic server-owned grading,
  immutable issued evidence and receipt replay, course grade control, and audited Instructor
  inspection. Removed the sidecar domain, Store, server composition, routes, generated contracts,
  dedicated tests, E2E hook, and migrations 1811, 1813, 1815, 1821, and 1822; recomposed the
  surviving 1812, 1814, and 1816--1820/1823 authority chain around live course state. The active
  source and migration inventories contain no retired execution references; focused Rust suites and
  the fresh PostgreSQL assignment-authority oracle are green.
- Restored whole-course grade-settings mutation after assignment writes moved behind their dedicated
  broker. PostgreSQL now performs the exact assignment-set validation and serialization inside the
  grade-control capability; the application adapter validates only the closed request shape. Hand-
  raised T4 expected-revision and immutable-witness conflicts no longer masquerade as retryable
  database serialization failures, so stale grade drafts reach the browser as the intended preserved
  412 workflow. The settings page also keeps a successful save acknowledged when its subsequent
  totals refresh fails and names the visible recovery action.
- Bound browser HTTP requests and Solid Router query entries to one abortable session generation.
  Confirmed logout and subsequent session establishment now retire pending work before another
  account can load, while the production-browser course picker waits for visible course identity
  instead of treating the persistent application shell as route readiness.
- Kept learner assignment discovery and explicit entitlement inspection on the read-only S5 path.
  These projections no longer request mutation locks that the application role intentionally lacks;
  later learner writes continue to reauthorize through their server-owned transactional capability.
- Split learner activity projection hydration from mutation locking. Assignment-overview progress,
  enrollments, runs, attempts, summaries, and instructor history now use plain snapshot reads and the
  read-only entitlement evaluator; brokered activity transitions remain the sole owners of write
  serialization. This restores the live learner assignment entry without broadening application-role
  privileges.
- Kept post-start run items, attempt lists, prefetched-question lookups, and pending-submission
  projections on that same read boundary. Server-owned attempt issuance continues through the 1817
  learner-work broker witness, while ordinary run navigation no longer tries to lock teaching source
  rows through the application role.
- Made public course, assignment, run, and workspace reference resolution uniformly projection-only.
  Learner run-reference authorization now evaluates current entitlement without mutation locks, so a
  newly issued run can open through its opaque public route under the least-privilege application role.
- Separated future-content replacement from active-attempt timing-policy mutation. A revision-checked
  fixed-item replacement now updates the assignment definition for future runs while leaving issued
  question evidence and effective-policy receipts untouched, matching the immutable issued-work
  contract and avoiding a false assignment conflict when learner work is active.
- Completed the focused assignment-mutation boundary around that replacement. The consolidated
  broker trigger now admits only its transaction-scoped operation marker, Rust verifies the exact
  revision advance returned by every focused mutation, wrong course/tenant routes are concealed
  before broker entry and revalidated under its lock, and repeated ordered positions may reference
  the same immutable publication without weakening reference validation.
- Bound successor-receipt reads and finalization to the explicit learner course/assignment route.
  Receipt replay, issued-presentation validation, and completed-run summaries are lock-free
  projections; successor mutation remains owned by the learner-work preparation broker. Completed
  public receipts now always suppress successor delivery state, including idempotent replay, while
  wrong routes remain concealed and cannot alter immutable receipt evidence.
- Kept assignment delivery preview source reads on the same least-privilege projection boundary.
  Derived previews now bind their internal assignment and membership evidence inside one consistent
  repeatable-read snapshot and append the private audit as the final statement, without acquiring
  teaching-data mutation locks through the application role.
- Restored teaching-operations projections under the same least-privilege rule: instructor checks
  and course-group policy reads now remain plain application-role reads, while the dedicated
  server-owned mutation brokers retain locking and current-authority revalidation. The live browser
  journey now waits for the visible group editor before attempting its first mutation.
- Closed co-instructor invitation mutation authority with session-derived database capabilities for
  create, revoke, and decline, completed the existing acceptance broker's lock privileges, and
  revoked direct application-role invitation writes and sequence access. Memory and PostgreSQL now
  reject an actor that disagrees with the presented active session.

## 2026-08-23

### Fixes and Maintenance

- Migrated server course-creation test fixtures to deterministic, same-tenant
  Sysadmin session-bound authority and added a shared validated test helper.

## 2026-08-22

### Fixes and Maintenance

- Split the canonical browser-suite owner into explicit launch, visible-scenario execution,
  cleanup, receipt, and screenshot-transfer responsibilities. Shared Playwright helpers now own
  timeout setup, relative dates, and origin receipts instead of duplicating those contracts across
  production-stack scenarios.
- Made build-mode local lifecycle startup self-sufficient after image pruning: it reconstructs the
  canonical reviewed renderer from the maintained sibling checkout, or pulls a published renderer
  only by immutable digest. Production-auth topology inspection remains read-only and renderer OCI
  identity is still proven before any renderer container starts.
- Applied the six-pass post-acceptance audit cleanup: removed unused controller parameters, made
  permanent comments and developer callback contracts describe current behavior, documented each
  production Playwright selector contract, pruned replica pytest assertions tied to SQL layout, and
  routed the course-appearance audit through the canonical browser and screenshot front doors.
- Made the external-SMTP overlay the sole Compose owner of the invitation-delivery worker. The
  canonical non-SMTP browser stack now reconciles only services it can launch, while SMTP-selected
  stacks retain the worker's complete runtime, security, network, and cleanup contract.
- Started the persistent developer live-demo supervisor in its own session so the fixed
  `ple-live-demo-browser` owner continues after the short-lived root start command exits.
- Made the developer live-demo parent reap an early-exited process-like supervisor immediately
  when it has not published its authenticated readiness receipt, then run the established exact
  fixed-owner recovery sequence instead of consuming the full start budget.
- Derived the live-demo developer start and supervisor-termination budgets from the canonical
  240-second lifecycle launch budget plus a bounded handoff/cleanup margin, while retaining the
  20-second authenticated stop budget so a clean production build can finish before recovery owns
  its fixed disposable stack.
- Added root `run_live_demo.sh` shortcut for the canonical production build,
  readiness, HTTPS open, headless start, and owner-scoped stop commands.
- Pinned the live-demo service-owner child environments to unbuffered Python with bytecode writing
  disabled, preserving the closed runtime allowlist while preventing aggregate service oracles from
  recreating repository `__pycache__` artifacts.
- Accepted `WP-PROF-T3`: the preview plane now authorizes the direct Instructor before decoding an
  identity-free learner-derived subject, evaluates through the accepted S5 -> S3 -> S4 chain, and
  atomically appends the sole successful PII-minimal audit event. Memory and PostgreSQL prove that
  audit/no-mutation boundary; the fixed serial `DATABASE_BASELINE` profile under
  `ple-live-demo-browser` applied and verified 46 migrations. The focused `preview_plane` scenario
  exercised the production browser at the real HTTPS origin. The 63-image canonical corpus includes
  12 nested T3 images from that same origin with full origin and empty-cleanup receipts; architecture,
  security, HCI, and documentation reviews found no unresolved P0-P3 issue. Screenshot publication is
  the explicit `capture_screenshots.sh` gate; `all_test.sh` validates the same system without
  rewriting published artifacts. Final Validation passed: Rust, codebase (5/5), pytest (6,417), ten
  production-browser scenarios at one HTTPS origin, WebWork, replica restart, and every lane's exact
  cleanup receipt are green.
- Audited permanent-test admission against the repository test-style rules. The evidence model now
  separates stable callable behavior checks from one-time source, migration, and screenshot-coverage
  closure evidence; the active plan and test guidance describe the same boundary, and obsolete
  mock-browser inventory language is no longer a permanent-test requirement.
- Completed the post-acceptance audit: preview UI state is namespaced, the obsolete wrapper, synthetic
  source, and QTI snapshot are retired, and current documentation routes validation and browser
  evidence to their canonical plans. The exact Wasm secrecy gate remains preserved while the Base
  Course boundary stays adaptable.
- Accepted `WP-PROF-BS1`: Playwright, canonical screenshot capture, direct browser development, and
  aggregate acceptance now share one production `dist/` browser and the fixed disposable
  `ple-live-demo-browser` HTTPS stack. The real Rust API, PostgreSQL, MinIO, worker, renderer,
  production account/session authentication, authorization, and seeded live-demo baseline are the
  sole connected browser path. Focused scenarios receive a fresh installation; the complete catalog
  shares one stack and creates namespaced product state through visible PLE workflows.
- Retired the alternate test-only browser artifact, mock transport/runtime graph, local-file
  authentication and local-teaching activation, random browser/service owners, and superseded
  walkthrough/Chapter One browser lanes. `run_playwright_tests.sh`, `capture_screenshots.sh`,
  `local_stack.py acceptance`, and the four-gate `all_test.sh` front door now converge on the fixed
  owner. Browser-free WebWork and two-API/one-PostgreSQL restart oracles remain only for their distinct
  service claims and run serially after the one browser invocation.
- Corrected the learner assignment-summary boundary discovered by the real UI journey. An entitled
  learner with no materialized receipt now receives a key-free, read-only `noActivity` projection;
  current entitlement and disclosure policy are still evaluated, denied and unknown assignments stay
  concealed, and summary reads create no enrollment. A materialized enrollment without its required
  summary remains a service failure rather than a valid empty state.
- Published the JSON-authoritative 51-image screenshot corpus into nested role and journey folders
  from the production HTTPS origin. The atomic publisher records production-bundle identity, scenario,
  viewport, privacy checks, and SHA-256 for every artifact. Automated image review found one saved-
  appearance timing defect; recapture now visibly includes `Course appearance saved.` and the selected
  Forest theme, and the re-review closed the finding.
- Hardened fixed-target Compose inspection around a closed `SafeLoader` subclass that retains only the
  required `!reset` tag and rejects Python object constructors. The instructor guide now references the
  manifest-owned active-roster capture, and the migrated-browser inventory checks meaningful visible
  feedback sections without freezing their incidental order.
- The complete nine-scenario browser catalog passed with same-origin receipts and exact cleanup. The
  aggregate connected gate then passed the browser catalog, WebWork render/grade/cache/outage/redaction
  oracle, and replica restart with two API replicas sharing one PostgreSQL instance. Final validation
  completed the required four-command sequence twice on the recorded tree.

## 2026-08-21

### Fixes and Maintenance

- Retired the obsolete mock UI-walkthrough/simulator browser graph and its aggregate acceptance
  lane. The three proven narrow utilities now live under `tests/support/` with Node ownership;
  simulator-only evidence grammars, private walkthrough configuration, orchestration, and exact
  wrappers/tests were removed.
- Accepted `WP-PROF-BS1-H0`: a typed shared `live_demo` browser-suite owner and wrapper now validate
  closed selection/configuration before allocation, create and validate generation-bound input before
  Chromium, sanitize the Playwright environment, and record truthful lifecycle plus typed-cleanup
  receipts with aggregated failures and private diagnostic retention.

- Accepted `WP-PROF-BS1-H1`: `run_playwright_tests.sh` now owns default, `--build`, named
  `live_demo`, approved-file, and literal-grep selections through a fresh disposable production
  `dist/` HTTPS stack. The lifecycle owns build and cleanup; WebWork/walkthrough callers retain
  closed owner-fixed transitional commands until their later consolidation, and TLS bypass applies
  only to the live-demo path.

- Accepted `WP-PROF-BS1-H2`: Python `ScenarioContract` is the sole policy owner with strict generic
  `BrowserScenarioInputV1` and Rust Base Course receipt authority. A public-safe run namespace
  replaces `Date.now()`/`parallelIndex`; product mutations use visible UI, route-abort fault behavior
  moves to F1, and Avery's visible sign-out/sign-in proves teaching-team persistence.

- Accepted `WP-PROF-BS1-H3`: typed browser-suite oracles now prove the exact disposable HTTPS
  gateway origin, lifecycle-derived `podman-compose --in-pod false` provider policy, labelled
  resources, private artifacts, and owner processes. Receipts expose safe metadata only; successful
  cleanup leaves each final ownership inventory empty.

- Accepted `WP-PROF-BS1-I1`: the real production-browser instructor-authoring journey uses visible
  PLE controls to create and publish a namespaced question, course, assignment, and roster
  invitation. A roster reload plus a fresh Elena browser session visibly confirm the persisted
  course, published assignment, and pending invitation. Real-stack validation corrected the
  canonical-viewport interaction by focusing the accessible assignment-edit link and activating it
  with Enter. The successful `ple-live-demo-browser-d12ad2600259` invocation verified the exact
  disposable HTTPS gateway origin and removed its labelled containers, volumes, networks, generated
  gateway image, and private run artifacts. Offline checks passed; correctness accepted the
  `not_required` Sysadmin semantics as genuine scenario independence, and the focused security
  review passed.

- Accepted `WP-PROF-BS1-C0`: `all_test.sh` is the fail-fast aggregate front door, ordered as
  environment, pytest, build, Rust, codebase, one local-stack acceptance handoff, and both diff
  checks. Aggregate evidence has one canonical real-stack production-browser lane and two labelled
  transitional visual-fixture lanes pending V1; the duplicate compatibility-wrapper lane is gone.
  The evidence model and operational documentation now state that boundary and reserve fakes for
  narrow isolated contracts.

- Accepted `WP-PROF-BS1-B0`: the V2 browser-scenario contract now has a closed
  `sysadminRequirement`, a deterministic flat catalog with an explicit legacy-live-demo bridge, and
  a multi-scenario owner that uses one disposable stack for a complete selection and a fresh stack
  for a focused selection. Each child receives only its own canonical input and origin-receipt path;
  ordered public success and failure receipts record each child outcome without exposing private
  claim material.

- Accepted `WP-PROF-BS1-U1`: retained browser-independent Node evidence now uses literal decoder
  payloads, direct narrow dependencies, and a test-local recording fetch. It restores
  issued-question secrecy checks without a fake server, retains no `src/api/mock/**` imports, and
  adds a deterministic consumer/allocation scanner that assigns every remaining mock-runtime marker
  to its later real-stack migration owner (`I1`, `L1`, `A1`, `S1`, `V1`, `F1`, or `R1`).

- Accepted `WP-PROF-BS1-L1`: the real production-browser learner journey creates its namespaced
  instructor prerequisites through visible PLE controls, then lets Mary visibly claim the invitation,
  submit the UI-authored known-correct response, receive `Correct` feedback, and observe completion
  plus `Current`, `Latest`, `Best`, and `This run` 100% score evidence. Mary starts a second practice
  run; a fresh empty-storage learner session then signs in through the UI and confirms the completed
  score state persists. The focused connected run used exact owner origin project
  `ple-live-demo-browser-5596d9484f43` and completed owned-resource and private-state cleanup.
  Real-stack corrections use the compact namespaced roster identifier and active second-run UI state.
  Offline boundary gates passed; independent correctness accepted and security passed with no P0-P3
  findings.

- Accepted `WP-PROF-BS1-B1`: the unclaimed visible Sysadmin setup child now exports one strictly
  validated, mode-0600 WebAuthn continuation; a fresh claimed child imports it into a new virtual
  authenticator, signs in through the ordinary visible passkey path, and writes an owner-validated
  acknowledgement. Public evidence records only truthful consumption state. The fixed-project
  `auth_authorization` receipt proved the producer-to-consumer transition and reported
  `webAuthnContinuationConsumed: true`; independent correctness and security accepted the boundary.

- Accepted `WP-PROF-BS1-B2`: every browser invocation now owns the exact
  `ple-live-demo-browser` project through one nonblocking checkout lease, one fixed private
  workspace, and one exact owner-labelled reset before regeneration and during final cleanup. The
  production entry acquires the lease before ports, provider, build, Podman, or PostgreSQL work;
  reset authority requires that held lease; public reporting follows the final reset and fresh
  process observation; and receipts expose no private locator or session material. Two serial real
  runs regenerated the same seeded fixture and each left zero containers, volumes, networks,
  workspace artifacts, or owner processes. Correctness and security reviews accepted the design.

- Accepted `WP-PROF-BS1-A1`: real production-browser journeys visibly complete generation-bound
  Sysadmin first claim and passkey reauthentication; Instructor and Student session entry/reentry;
  Morgan's Genetics access and Avery approval; Elena's invitation; Avery's acceptance and fresh
  teaching session; and Mary's safe cross-course and instructor-role denials. Product state changes
  use visible PLE controls, the claimed scenario receives no first-claim proof, and no mock route or
  backend product-state setup participates. Independent correctness/HCI and security reviews found
  no P0-P3 issue.

- Accepted `WP-PROF-BS1-X1`: two independent Elena sessions now create and mutate one namespaced
  course through visible PLE controls, produce a real grade-settings revision conflict, preserve the
  stale local draft, retry it by keyboard, and reload the authoritative result in the observing
  session. The connected fixed-stack receipt proves each BrowserContext independently used the same
  production HTTPS gateway; final reset left no project containers, volumes, networks, private
  artifacts, or owner processes. Real-stack validation also corrected the scenario's status-region
  locator and bounded its generated letter-band labels to the product's visible 32-character rule.
  Independent correctness/HCI and security review accepted the final design.

- Accepted `WP-PROF-BS1-S1`: reload, fresh authorized sessions, role observation, and protected
  transport already provide the semantic persistence evidence required by the accepted instructor,
  learner, and authorization journeys, so no schema-coupled Store or service receipt was added. The
  three remaining S1 mock-browser behaviors now have machine-checked successors and explicit R1
  retirement dispositions. External-tool launch and submission use one exact per-attempt route
  contract with narrow client/widget and Rust service evidence; renderer safety remains under narrow
  TypeScript and Rust projection tests plus the accepted real learner journey; and Canvas QTI import
  now runs through Elena's visible production UI, real PostgreSQL/MinIO worker path, conversion, and
  a fresh empty-storage Elena session. Its connected receipt proved separate exact HTTPS origins and
  complete fixed-stack cleanup. Independent correctness/HCI and security review accepted the result.

- Accepted `WP-PROF-BS1-F1`: the canonical production-browser owner now drives one typed gateway
  outage at Mary's visible saved-response boundary. Elena and Mary create all product state through
  the PLE UI; the learner retains her selected response, sees the real network-recovery state,
  keyboard-activates the retry after owner-proven gateway readiness, and a fresh learner session
  confirms the persisted score. The closed owner/Playwright-worker handshake uses one lease-owned
  fixed AF_UNIX channel resource with authenticated bounded messages, mode-0600 visible-state
  markers, process-group cleanup, and autonomous stale-channel recovery. The connected fixed-stack
  receipt proved the declared fault was injected and recovered at one HTTPS origin, then left no
  containers, volumes, networks, workspace artifacts, owner processes, or fault-channel directory.
  Independent correctness/HCI and security review accepted the final design with no P0-P3 findings.

- Converged the retained WebWork and replica/restart service oracles on the fixed
  `ple-live-demo-browser` owner and seeded production authentication. Profile policy now owns the
  exact two-API replica cardinality for both launch and restart while PostgreSQL remains singular.
  The connected replica gate authenticated Mary, restarted across two distinct API instances,
  verified exact-envelope durable replay, and completed an empty cleanup receipt for 10 containers,
  3 volumes, 4 networks, both project images, the private workspace, and owner processes. That run
  also exposed a shared Rust identifier defect: random product IDs had been stored as arbitrary
  128-bit UUID-shaped values. `learning-data-access` now owns one cryptographically random UUID-v4
  contract for identifiers, with exact byte-normalization tests, while three private lease/fencing
  capabilities retain all 128 random bits through an explicit storage envelope. Focused Rust tests,
  PostgreSQL-feature checks, strict Clippy, and independent re-review passed.

- Restored the executable mode for `tests/e2e/e2e_live_demo_browser.py`, bringing its shebang
  into alignment with the repository executable-script contract.

- Moved the live-demo selector identities and generation-bound Sysadmin claim
  context out of the common Compose topology and into the HTTPS live-demo
  browser overlay. Ordinary local, Chapter One, WebWork, and walkthrough
  owners now launch without demo-only inputs; the retired default-only proof
  command no longer suggests that ordinary local state owns the live-demo
  capability.

- Increased the one-shot MinIO Client service memory envelope from 128 MiB to
  256 MiB after the real `mc pipe` receipt-create operation again exited 137.
  The narrow resource repair retained the service hardening and received
  independent acceptance.
- Repaired the disposable UI walkthrough environment to bootstrap and validate
  its private ownership/passkey setup context under its declared owner policy
  without accepting a source or ambient proof path; the browser walkthrough
  itself does not exercise a Sysadmin passkey journey. Its interim live-demo
  claim-context plumbing was superseded and removed when demo auth was
  isolated to the explicit HTTPS live-demo overlay.
- Repaired offline UI walkthrough temporary repositories so they include both canonical Compose
  files required by the `ui-walkthrough` disposable-owner policy.
- Kept the database-and-storage live-demo baseline owner independent of the
  browser-only Sysadmin claim context. The interim ordinary-local claim-context
  plumbing was superseded and removed when demo auth was isolated to the
  explicit HTTPS live-demo overlay, whose TLS browser owner alone creates the
  private, generation-bound passkey-ownership proof.

### Decisions and Failures

- Reprioritized `WP-PROF-BS1` as the sole current professor package after accepted live-demo
  delivery, ahead of planned frozen-scope `WP-PROF-T3` and `WP-PROF-T4`. It establishes one
  disposable production-browser HTTPS path for Playwright, screenshots, and acceptance, then
  retires the parallel mock application as real scenarios and narrow unit tests take ownership of
  meaningful behavior. The plan starts with a typed live-demo owner adapter, uses UI-created
  namespaced state and product-visible persistence evidence, adds service receipts only for their
  named claims, and records visual evidence through automated image evaluation. Pre-production
  implementation may strengthen foundational boundaries directly; suite-labelled cleanup and
  repeat-run noninterference are acceptance evidence, while image pruning is permitted lifecycle
  hygiene.

### Developer Tests and Notes

- BS1-H0 Validation passed 3,251 affected tests. Its real production HTTPS `live_demo` scenario
  passed under `ple-live-demo-browser-46aa4964966b`; successful cleanup left zero matching Podman
  resources and no `run-*` private state. Final correctness review returned ACCEPT. Final security
  rereview closed `BS1-H0-SEC-001` with no P0-P3 finding.

- BS1-H1 passed 21 initial focused Python, 7 Node config, and 43 corrected focused pytest tests,
  TypeScript checks, shell/pyflakes/diff checks, and real default `--build`
  `ple-live-demo-browser-2898787a80e3` plus focused-grep
  `ple-live-demo-browser-3410dc98a716` runs. Each left zero matching resources and no `run-*`
  state. Final correctness and security reviews returned ACCEPT with no P0-P3 finding.

- BS1-H2 passed corrected 32 Python and 4 Node tests, TypeScript, Prettier, and diff checks; the
  canonical real `live_demo` passed and cleaned its owned stack. Final correctness returned ACCEPT
  with no P0-P3 finding; final security closed PASS with 34 pytest, 8 Node, TypeScript, and diff
  evidence.

- BS1-H3 passed 37 focused pytest checks, 4 Node checks, TypeScript, pyflakes, Prettier, and diff
  validation. Two sequential fresh production HTTPS runs, `ple-live-demo-browser-ba258fc76aeb` and
  `ple-live-demo-browser-b0984e1ae0ea`, each recorded the exact gateway origin and an empty final
  labelled-resource, private-artifact, and owner-process inventory. Independent correctness
  returned ACCEPT and security returned PASS with no P0-P3 finding.

- BS1-C0 passed one aggregate `local_stack.py acceptance` run with successful labelled-resource
  cleanup, the full 6,232-test pytest suite, shell syntax, and working-tree plus cached-diff gates.
  The aggregate receipt names the canonical production-browser behavior lane separately from two
  transitional visual-fixture lanes. Independent correctness returned ACCEPT and security returned
  PASS with no P0-P3 finding.

- BS1-B0 passed 42 focused Python tests, 5 Node parser tests, TypeScript compilation, Prettier,
  pyflakes, and both diff checks. Sequential real default and focused production-browser runs used
  distinct disposable HTTPS origins and removed their private state and all owned resources.
  Independent correctness returned ACCEPT and security returned PASS with no P0-P3 finding.

- BS1-B1/B2/A1 connected acceptance ran `sysadmin_first_claim` and `auth_authorization` serially
  through `./run_playwright_tests.sh --build --scenario ...`. Both used the exact fixed project;
  the first origin was `https://localhost:55092/`, the regenerated second origin was
  `https://localhost:55206/`, and all observed page/request origins matched their production HTTPS
  gateway. The second receipt recorded the visible unclaimed producer and claimed consumer, a
  validated WebAuthn acknowledgement, and the complete auth/authorization UI journey. All six
  initial/final reset and final-empty facts were true for both runs; independent inventories were
  empty between and after them, so no two PostgreSQL stacks overlapped. `target/` remained 12G.
  The corrected material tree passed all 6,542 pytest cases plus two subtests, including the
  repository function-typing gate; B2 focused correctness/security, B1/B2 runtime, and A1
  correctness/HCI/security reviews all returned ACCEPT.

- BS1-U1 passed the retained focused Node evidence: 52 named tests (53 file-level reported passes
  including its support module), ESLint, TypeScript, Prettier, and both diff checks. The current
  integrated material tree passed `./check_codebase.sh` with 278 Node tests and
  `source source_me.sh && pytest tests/` with 6,396 tests. Independent correctness returned ACCEPT
  and security returned PASS with no P0-P3 finding.

- Final validation for the executable-mode repair passed Rust workspace checks, five TypeScript
  checks with 322 Node tests, 6,089 pytest cases, 248 built Playwright cases, and the complete
  disposable local-stack acceptance suite with no required skips.

- Applied authority ruling A to the accepted immutable WP-PROF-LD2 allocation: migration
  `2026081809` owns exactly two narrow least-privilege execute-only brokers, safe normal Sysadmin
  approval-candidate discovery and read-only completed-installation-generation lookup used to bind
  configured first-ownership proof. `2026081810` is accepted and immutable only for the Student
  pre-tenant account-course retention repair. Selector behavior and claim/passkey/account/session
  data and semantics remain non-schema; the generation-read broker is the narrow schema
  authorization seam for that otherwise non-schema ownership flow. This supersedes the temporary
  P2 reopening record below, restores LD2 acceptance on 2026-08-21, advances T3 as sole current,
  and leaves WP-RC8 parked and open. This documentation closeout does not itself prove final-goal
  completion: final-goal completion additionally requires the complete final-material-tree Validation
  after these record edits.
- Reopened WP-PROF-LD2 after the independent final material review rejected a P2
  documentation-and-migration scope contradiction. Migration `2026081809` contains both the
  Sysadmin approval-candidate discovery broker and the restricted completed-installation-generation
  read broker used by the ownership route. Its exact allocation scope is under authority correction;
  `2026081810` remains allocated for the narrow Student pre-tenant account-course context
  retention-boundary repair. Neither allocation is accepted or immutable. The full
  final-material-tree Validation and a fresh independent review are required after correction; this
  entry does not choose between widening 1809's accepted scope and splitting a new migration.
- Historical post-repair runtime evidence for the open WP-PROF-LD2 passed
  `./check_rust.sh`; `./check_codebase.sh` (five checks and 322 Node tests);
  pytest (6,017 tests, no skips); the baseline E2E under
  `ple_live_demo_baseline_124c398f82978266c7370838`; and all eight aggregate
  acceptance lanes. The terminal owner-locked local HTTPS Playwright journey
  passed once under `ple-live-demo-browser-d0ff0e97f4ac`. Typed cleanup and
  exact-label checks left zero containers, volumes, and networks after both
  connected runs; both diff checks passed and no Python bytecode artifacts
  remained. This documentation closeout does not itself prove final-goal
  completion; final-goal completion additionally requires the complete
  final-material-tree Validation after the scope correction. This does not claim
  public, AWS, operator, or production deployment activation and does not
  close WP-RC8.
- Corrected the live-demo regeneration checklist to the KISS product contract:
  fresh database and object-storage regeneration creates the seeded Sysadmin
  in its original unclaimed baseline state without a credential-replay
  requirement. Clarified that the owner-locked TLS overlay is disposable
  connected-E2E validation, not public or production deployment activation.

## 2026-08-20

### Additions and New Features

- Split the common Compose topology from the ordinary local-development overlay. The default
  lifecycle and local disposable owners now use base plus local development (then SMTP when
  selected), while the live-demo browser owner remains base plus its TLS overlay without a local
  identity bind. Live bootstrap still creates its seed, invitation, Question ID, and claim inputs
  but no longer creates local-file credentials or an authentication host-file setting.
- Accepted WP-PROF-T2 with `2026081807_teaching_operations.sql` immutable. Course groups,
  purpose-specific allow/warn policies, referenced-group refusal, atomic S5/S3 re-evaluation with
  sealed receipt history, operator-owned Instructor approval, target-bound 30-day co-instructor
  invitations, direct-membership acceptance, final-Instructor protection, server-owned retention,
  and server-derived preview boundaries are complete. Final Validation passed `check_rust`,
  `check_codebase` (5 checks, 301 Node tests), pytest (5,481), built Playwright (245/245, zero
  skips), the fresh 43-migration PostgreSQL baseline with all T2 live oracles, `local_stack`
  acceptance, T2 visual capture (2/2), UI corpus verification (42/42), both diff checks, and
  independent final reviews with no unresolved P0--P3 finding.
- Repaired the local-stack lifecycle so an ordinary start replaces the project-owned containers
  instead of accumulating them. Startup now removes orphans, recreates dependencies before
  force-recreating application services, and waits for semantic readiness. Scoped image cleanup
  removes inactive project aliases and prunes unused images, while named data volumes remain under
  the existing explicit reset contract. Permanent lifecycle/image-cleanup tests pass, and two
  consecutive starts replaced all nine containers without increasing container, image, or volume
  counts.
- Continued WP-PROF-T2 through the typed domain, Store, Memory, PostgreSQL, and migration
  boundaries. Course groups now retain five purpose-specific multiple-membership policies, reject
  referenced deletion, and atomically re-resolve active S5/S3 work after audience, membership,
  purpose, or M2--M4 changes while preserving sealed receipt generations. Global Instructor
  approval remains non-authorizing operator eligibility; target-bound 30-day co-instructor
  invitations create ordinary direct Instructor membership only after approval recheck and refuse
  removal of the final active Instructor.
- Added the in-progress `2026081807_teaching_operations.sql` migration and its disposable PostgreSQL
  upgrade, group-policy, and teaching-authority oracles. The final database-baseline rerun passed all
  43 tracked migrations, exact group receipt transitions, four approval/invitation/concurrency
  cases, forced-RLS and global-table inventory checks, and representative role denial. This is an
  intermediate database vertical receipt; WP-PROF-T2 server, browser, retention-page, full
  Validation, and final independent-review gates remain open.
- Added accepted typed `U-`, `M-`, and `CI-` public locators for accounts, course-membership
  episodes, and co-instructor invitations. Authorized Memory and PostgreSQL projections now keep
  account lookup session-bound, membership lookup exact-course Instructor-bound, and pending
  invitations target-bound; bounded snapshot lists return display labels without email or UUIDs.
  PostgreSQL identity sequences remain least-privilege (`ple_auth` for accounts and `ple_app` for
  memberships and invitations), and fresh independent review found no remaining P0--P3 issue.
- Accepted the strict WP-PROF-T2 teaching-operations wire contract before route implementation.
  Group, modifier, preview, co-instructor, and retention DTOs use only typed human references and
  bounded values; strong revisions remain in `If-Match`, denied previews cannot carry resolved S3
  fields, and retention shapes match the existing server-owned lifecycle endpoints. Temporary
  TypeScript generation emitted 272 concrete types with no unresolved generic or private identity.

### Decisions and Failures

- Accepted WP-PROF-LD1 on 2026-08-20: `base_course_installation` now owns the typed two-phase Base
  Course lifecycle, while learning-data-access owns its SQL, advisory lock, durable state, migration,
  and Store boundary and project-tools remains the direct CLI adapter. The final PostgreSQL 17 plus
  MinIO lifecycle proved five ordinary accounts across two courses, interruption/resume, retained
  restarts, concurrent serialization, fail-closed mixed state, and fresh regeneration. WP-PROF-LD2
  made WP-PROF-LD2 the next professor handoff; T3 remains parked. LD2's `2026081809` and
  `2026081810` allocations were later found to need 1809 scope reconciliation: the current 1809
  file contains approval-candidate discovery and a completed-installation-generation read broker.
  The selector and passkey ceremony seams remain non-schema; the generation-read broker awaits an
  authority ruling. Whole live-demo Validation remains incomplete until LD2 and the connected goal
  close.

- Recorded the intermediate WP-PROF-T2 contract/allocation gate before implementation. The shared
  migration ledger reserves `2026081807` for future
  `2026081807_teaching_operations.sql`; the professor plan and durable guidance now define
  many-to-many group warning policy, referenced-group refusal, atomic S5/S3 re-evaluation,
  operator-owned non-authorizing Instructor approval, target-bound 30-day co-instructor invitations,
  direct-membership acceptance, final-Instructor protection, and server-owned preview/retention
  boundaries. This documentation gate makes no code, schema, test, or acceptance claim.

## 2026-08-19

### Additions and New Features

- Began WP-PROF-T1 with the serial assignment teaching contract. A closed
  `draft | published | closed | archived` lifecycle, validated plain-text instructions, and the
  existing S3 base policy now form one typed teaching-settings value. The domain maps only
  `published` through lifecycle gate G1 and keeps archived assignments terminal, while learner
  progress carries an independent `current | recalculating | failed` scoring status and omits
  numeric totals whenever scoring is not current.
- Continued WP-PROF-T1 through the Store, Memory, PostgreSQL, and server boundaries. One
  instructor-authorized, revision-checked teaching-settings mutation now owns lifecycle,
  instructions, availability, due and close instants, run and attempt limits, late behavior, and
  the existing server deadline behavior without a second timing writer. Memory and PostgreSQL both
  derive G1 from stored lifecycle, re-resolve active attempts through S3 after a settings change,
  preserve sealed receipt generations, and carry atomic scoring status with learner summaries,
  run summaries, and gradebook rows.
- Added the in-progress course-local schedule transport for WP-PROF-T1. Instructor inputs use exact
  local wall-clock values plus the course's IANA zone; the server alone converts them to absolute
  policy instants and refuses zone mismatch, out-of-term values, invalid ordering, and ambiguous or
  nonexistent daylight-saving times. Paginated learner assignment rows remain compact, while the
  separately authorized detail projects text-safe instructions and resolved learner-specific
  timing and late status without exposing base policy, provenance, tenant, or clock authority.
- Continued WP-PROF-T1 through the browser and permanent learner evidence. The instructor editor now
  keeps Teaching operations separate from content saves, supports keyboard save, exact field focus
  for schedule and lifecycle failures, and explicit stale-revision adoption without discarding either
  draft. The learner overview renders plain-text instructions and server-resolved course-zone delivery
  facts, while recalculating or failed scoring omits every numeric score. The refreshed student/access
  corpus proves the 1280 by 800, 800 by 1280, 393 by 852, and 800 by 800 compositions and the central
  no-instructor-transport route boundary; final live and aggregate acceptance remain pending.
- Repaired WP-PROF-T1 review findings before acceptance. Store creation now enforces Draft even for
  direct callers; Memory and PostgreSQL terminalize active attempts consistently after a teaching
  policy change; instructor editor reads pair durable lifecycle intent with a server-derived
  scheduled, open, or clock-closed state; and noncurrent scoring removes per-attempt results and
  feedback point values in addition to aggregate scores. Permanent cross-route tests cover both
  Recalculating and Failed, while live PostgreSQL and full demo-environment acceptance remain pending.
- Accepted WP-PROF-T1 with one revisioned assignment-teaching aggregate. Draft-only creation and an
  instructor-authorized revision-CAS mutation now own lifecycle, plain-text instructions, course-zone
  scheduling, run and attempt limits, late behavior, and deadline behavior. Memory and PostgreSQL
  share stored-lifecycle gating and active-attempt re-resolution. Learner list transport remains
  compact, resolved detail omits policy and clock authority, and Current, Recalculating, or Failed
  scoring status consistently controls every aggregate and per-attempt numeric projection.
- Accepted WP-PROF-S6 with a closed course-grade contract for total points and weighted categories.
  The shared evaluator consumes maintained assignment summaries, uses exact point values and category
  weights, applies drop-lowest rules deterministically, rounds once to four decimal places, and keeps
  recalculating, failed, empty, and zero-possible states distinct from a numeric zero.
- Added the accepted, immutable `2026081806_course_grade_scheme.sql` migration for normalized course
  schemes, ordered categories and membership, optional letter bands, and PII-free course-export audit metadata.
  It also aligns compact summary score constraints with the existing negative- and extra-credit range.
- Added the course-grade Store, Memory, PostgreSQL, and instructor HTTP boundaries. Scheme
  reads carry current assignment titles while revision-checked writes remain title-free; course totals
  use one atomic scheme snapshot and bounded compact-summary reads; and the separate synchronous CSV
  export records only durable PII-free audit metadata.
- Completed the S6 instructor browser vertical for total-points and weighted-category
  grading. The keyboard-complete settings page edits assignment membership, category weights,
  deterministic drop-lowest counts, and optional letter bands; preserves local work across a stale
  revision; displays current totals; and downloads the server-generated audited CSV without exposing
  its audit identifier. Four fresh 1280 by 800, 800 by 1280, 393 by 852, and 800 by 800 screenshots
  are manifest-owned under `docs/screenshots/instructor/course_grade/`.
- Accepted WP-PROF-S4 with one assignment-owned learner-disclosure policy. Score, correctness,
  feedback text, solution, and class statistics each use an independent closed timing, while current
  S5 entitlement, the current S3-resolved verdict, authoritative server time, and submission fact
  remain the only projection inputs.
- Added accepted immutable migration `2026081805_assignment_learner_disclosure_policy.sql`. It adds
  the five required assignment fields, removes their temporary backfill defaults, and directly drops
  the retired assignment, issued-attempt, and submission-snapshot disclosure columns without a JSON
  shadow or compatibility reader.
- Added the learner-safe class-statistics projection and four-viewport student access corpus. The
  identity-free union omits withheld data, reports metric-free insufficient evidence, and exposes
  only cohort size plus normalized average at the fixed five-learner floor. Eight allowed/denied
  access images at 1280 by 800, 800 by 1280, 393 by 852, and 800 by 800 now live under role-based
  screenshot subfolders within a 32-artifact manifest-owned corpus.
- Accepted WP-PROF-S3 with one pure effective-assignment-policy resolver. Ordered lifecycle,
  entitlement, and authorization gates deny before grant-filtered group modifiers or an individual
  exception resolve per-field policy and provenance. The resolver consumes S5 authority rather than
  reconstructing roster, audience, group membership, or enrollment state.
- Added accepted immutable migration `2026081804_effective_policy_resolver.sql`. It normalizes base
  policy and modifier inputs, then preserves resolved attempt policy in append-only sealed receipts
  with complete per-field source rows and a current pointer only to a sealed generation.
- Accepted WP-PROF-S5 with one typed entitlement authority. Closed assignment audiences, canonical
  course-membership episodes, purpose-capable course groups, evaluator-issued applicable-policy
  scopes, and immutable materialization provenance now share one Rust domain and Store contract.
- Added accepted immutable migration `2026081803_entitlement_membership.sql`. It normalizes current
  membership and profile evidence, assignment audiences, materialized enrollment receipts, grant
  basis, applicable scopes, and assignment-summary scoring state without a JSON shadow model or a
  generic polymorphic target.

### Behavior or Interface Changes

- Learner assignment, enrollment, run, attempt, submission, feedback, and progress responses now
  omit instructor policy, tenant, clock, and raw-storage authority inputs. Neutral score states never
  turn withheld values into zero or promise a later release, and `feedback_release` remains audit-only
  evidence that cannot change disclosure.
- The browser now evaluates one central route-role contract before instructor components, course
  theme reads, or transport mount. A student may use learner assignment, run, and account pages but
  receives an accessible denial for every instructor-only deep link, including roster and gradebook.
  Direct navigation/reload and no-transport tests provide the authorization proof; screenshots alone
  do not.
- Active roster membership no longer eagerly creates the roster-by-assignment enrollment
  cross-product. The first entitlement-bearing learner or instructor action evaluates current
  membership and audience under the action transaction, materializes exactly one receipt, and
  preserves its original actor-or-rule provenance on replay.
- Learner list, detail, run, attempt, submission, feedback, summary, prefetch, and public-route
  resolution re-evaluate current entitlement. Revocation or audience narrowing therefore removes
  current access without rewriting historical receipts; reinvitation creates a fresh membership
  episode while preserving the course-local learner identity and prior evidence.
- Course-grade GET and PUT now use one strong representation revision. Creating an assignment or
  changing its title advances that revision in Memory and PostgreSQL, so an ETag cannot continue to
  name changed assignment membership or titles and a stale write conflicts before body validation.
  Course-grade exports use a generic filename and the browser reports completion without displaying
  the internal audit identifier.
- Instructor assignment cards now open the instructor editor directly instead of traversing the
  separately authorized learner detail route. Activating Replace moves keyboard focus into the
  revealed Replacement Question ID task; students retain the Start assignment action and never
  receive the instructor link.

### Fixes and Maintenance

- Removed the retired question-level feedback timing type from Rust, TypeScript, authoring, imports,
  fixtures, mocks, and maintained documentation. Browser mocks now consume static server projections
  instead of synthesizing disclosure from legacy immediate/deferred/release labels.
- Organized the screenshot corpus into instructor, student, student/access, and shared ownership
  subfolders. Recursive provenance now binds each owner refresh to one generation plus per-file
  digest and exact PNG dimensions, so mixed partial refreshes, changed bytes, wrong dimensions,
  symlinks, and undeclared artifacts fail verification.
- Made the required WebWork browser acceptance own a private disposable full stack instead of
  reusing the retained default `containers` project. Its capability permits only structured launch,
  exact renderer outage/restart, one bounded redacted API-evidence log read, and label-proven cleanup;
  arbitrary Compose commands are rejected.
- Made Memory and PostgreSQL compose resolver inputs from the same evaluator-approved scopes on
  resolve, start, issue, and list paths. An unrelated group modifier therefore cannot suppress a
  learner who is currently entitled to the assignment.
- Sealed each PostgreSQL receipt set only after its one grant basis and complete applicable-scope
  set are present. Direct application writes, late scope insertion, reversible membership episodes,
  cross-tenant reads, and unauthorized instructor provenance are rejected at the database/Store
  boundary, with Memory and PostgreSQL sharing the same closed authority matrix.
- Replaced duplicate membership authority and payload-backed enrollment summaries with canonical
  relational owners. PostgreSQL learner pagination now filters through the entitlement evaluator
  before it exposes an opaque cursor, matching Memory without leaking inaccessible assignments.
- Repaired the in-progress S6 migration and live oracles from clean PostgreSQL evidence. The course
  default is a deferred constraint trigger, grade mappings follow assignment retention deletion,
  policy exceptions grant the retention broker only their required tenant-scoped cleanup authority,
  and direct score/category/mode/role probes remain rollback-isolated. The upgrade oracle proves the
  1805-to-1806 backfill and retention wrapper, while the course-grade oracle proves current totals,
  weighted/drop-lowest behavior, CAS, RLS, audit immutability, bounds, and the 501-learner refusal.
- Split course-management navigation and course-grade settings presentation into component-owned CSS
  modules after the integrated source-size gate found the shared stylesheet at 1,070 lines. The same
  built-browser rules now leave `src/style.css` at 959 lines without changing the visual contract.
- Added an idempotent `devel/reset_podman.sh` front door for the fixed ordinary local project. The
  normal lifecycle now performs a project-wide container and orphan reconciliation while preserving
  its named volumes and recreates the complete designed suite. Only after semantic readiness it
  removes exact inactive image tags belonging to declared disposable projects, then runs
  `podman image prune --all --force`. This closes Podman's shared-image alias case, where an aborted
  walkthrough tag can survive because the same image ID remains active under the ordinary gateway
  tag. Existing containers protect their exact references; every superseded tag or otherwise unused
  local image is removed so repeated project builds cannot accumulate an image backlog. A repeated
  real start passed after this repair and left the complete nine-container demo ready at loopback
  port 8080 with only the seven images used by those containers. The generated local-login record
  remains mode 0600.
- Rotated the complete 2026-08-10 through 2026-08-15 day blocks into
  `docs/CHANGELOG-2026-08c.md` with the maintained changelog tool. The active changelog retains the
  two newest date blocks as required by repository policy.

### Decisions and Failures

- WP-PROF-T1 is accepted as a non-schema package. Accepted migration `2026081804` already removed the redundant
  assignment `visible` column and retained the normalized base-policy row together with the existing
  instructions and four-state lifecycle columns, so no `2026081807` allocation or compatibility
  model is warranted for T1. At T1 acceptance on 2026-08-19, the sole professor handoff advanced to
  WP-PROF-T2 and the next schema allocation was unassigned; the 2026-08-20 T2 contract gate now
  reserves `2026081807`. WP-RC8 remains parked and open.
- WP-PROF-T1 final material-tree Validation passed `./check_rust.sh`; `./check_codebase.sh` (five
  checks and 279 Node tests); `source source_me.sh && python3 -m pytest tests/ -q` (5,480 tests and 2
  subtests); the fresh PostgreSQL 17 baseline with all 42 migrations and the assignment-teaching plus
  retained S3--S6 oracles; both diff checks; and the 36-artifact screenshot verifier. The final
  uninterrupted `source source_me.sh && python3 local_stack.py acceptance` passed 237 of 237 ordinary
  built-browser tests, both visual lanes, J1--J5, Chapter One publication, live Chapter One 4 of 4,
  and live WebWork 4 of 4. Independent architecture/security, tests/HCI/browser, and docs/authority
  rechecks returned ACCEPT with no P0--P3 finding. Local fictional users route around unavailable
  email. This does not claim provider or mailbox delivery, production email, passkeys, multi-replica
  operation, deployment, release activation, or that screenshots alone prove authorization.
- Worked the three completion-mode examples required by the professor plan. A mixed practice-plus-exam
  course needs a hybrid rule not present in the current table, and adding assignments mid-term needs a
  membership or required-count rule. Completion-based grading therefore leaves S6 for a later package;
  no runtime, database, API, or browser consumer may assume that third mode exists. S6 closes with
  only total points and weighted categories, and the sole professor handoff advances to WP-PROF-T1.
- Defined the requested test-drive environment as the ordinary local, networked Podman stack: the
  complete designed service suite communicates through its normal topology and is seeded with sample
  data and fictional local users. It is not an offline bundle or a specialized acceptance stack. The
  stale simulated volumes and generated Chapter 1 pilot receipt were reset. A fresh full-stack start
  and all seven acceptance lanes passed, and the ordinary demo is now ready on loopback port 8080.
- Accepted WP-PROF-S4 after independent architecture/security, tests/HCI, docs/legacy,
  student-access/HCI, and screenshot-corpus reviews returned ACCEPT with no unresolved P0--P3
  finding. The sole professor handoff advances to WP-PROF-S6; WP-RC8 remains parked and open.
- Final material-tree Validation passed `./check_rust.sh`; `./check_codebase.sh` (five checks and 274
  Node tests); `source source_me.sh && python3 -m pytest tests/` (5,418 tests and 2 subtests);
  outside-sandbox built Playwright (228 of 228, zero skips); the fresh PostgreSQL 17 baseline (all 41
  migrations, the S4 disclosure/current-policy/RLS and class-statistics oracles, and cleanup); all
  seven aggregate browser, visual, walkthrough, Chapter One, and isolated disposable WebWork lanes;
  the 32-artifact screenshot verifier; and both diff checks. Local-development credentials and
  invitations route around unavailable email. This does not claim provider or mailbox delivery,
  passkeys, multi-replica operation, deployment, release activation, or that screenshots alone prove
  authorization.
- Accepted WP-PROF-S3 after independent domain/Store, PostgreSQL/RLS, and consumer/test reviews
  returned ACCEPT with no final blocking finding. The sole professor handoff advances to WP-PROF-S4;
  WP-RC8 remains parked and open.
- Final material-tree Validation passed `./check_rust.sh`, `./check_codebase.sh` (five checks and 264
  Node tests), `source source_me.sh && python3 -m pytest tests/` (5,220 tests), outside-sandbox
  `./run_playwright_tests.sh --build` (203 of 203, zero skips), the fresh PostgreSQL 17 baseline (40
  migrations and the normalized S3 oracle with cleanup), and all seven local-stack acceptance lanes.
  An external renderer image rebuild after pruning was one-time environmental evidence, not a PLE
  implementation change. Both diff checks passed. This acceptance does not claim provider or mailbox
  delivery, passkeys, multi-replica operation, deployment, or release activation.
- Accepted WP-PROF-S5 after final independent domain/Store, PostgreSQL/RLS/security, and API/HCI/test
  reviews returned ACCEPT with no P0--P3 finding. The sole professor handoff advances to
  WP-PROF-S3, which consumes S5's decision and applicable scopes instead of reconstructing roster or
  group authority. WP-RC8 remains parked and open.
- Final material-tree Validation passed `./check_rust.sh`, `./check_codebase.sh` (five checks and 264
  Node tests), `source source_me.sh && python3 -m pytest tests/` (5,232 tests), outside-sandbox
  `./run_playwright_tests.sh --build` (203 of 203, zero skips), the fresh PostgreSQL 17 baseline (39
  migrations and the entitlement/membership/RLS oracle), and all seven aggregate browser, visual,
  walkthrough, Chapter One, and WebWork lanes. Both diff checks passed. This acceptance does not
  claim provider or mailbox delivery, passkeys, multi-replica operation, deployment, or release
  activation.

## 2026-08-18

### Additions and New Features

- Accepted WP-PROF-S7: one full-string typed public-reference grammar now names courses,
  assignments, runs, workspaces, and course groups as `C-`, `A-`, `R-`, `W-`, and `G-`; `AC-` stays
  reserved for the later Alpha aggregate. Published versions now carry one immutable, validated,
  ordered public byline that is deliberately distinct from private author-account identities.
- Added accepted WP-PROF-S2 support for one mandatory teaching-course term. Shared Rust
  values now own exact calendar dates, inclusive ordering, and case-sensitive IANA membership;
  `CourseRecord`, `CourseSummary`, the existing Store and course routes, generated TypeScript, and
  the course form all carry that same required value without a default or compatibility reader.
- Added one authority for the committed screenshot corpus. `tests/playwright/ui_corpus_manifest.ts`
  declares all 24 artifacts with their surface, route, role, owning pipeline, live-capture reason,
  and evidence purpose, and both capture runners now read it instead of holding separate name lists.
  `tests/playwright/ui_corpus_provenance.mjs` records the capture commit per artifact, and
  `tests/playwright/verify_ui_corpus.mjs` reports ownership gaps and staleness, so "is this visual
  evidence current?" is answerable without re-running a capture pipeline.

### Behavior or Interface Changes

- Course creation now requires `{title, term: {startDate, endDate, timeZone}}`. The instructor form
  exposes all four inputs without deriving a browser zone; a bounded field-specific term refusal
  preserves the entered values, announces the correction, focuses its field, and supports retry.
- Recorded the owner's device correction across `docs/HUMAN_GUIDANCE.md` and
  `docs/UI_DESIGN_GUIDE.md`: 1280 by 800 is the canonical laptop viewport for both instructors and
  students, the 800 by 1280 portrait tablet is a high-priority student design target rather than a
  secondary tier, and the narrow phone remains a compatibility guard for occasional use such as
  working while commuting.
- Closed SEC-1 so catalog browse/search/detail routes are now Instructor/Sysadmin-only on the server,
  and made the global route contract own route-role policy for Library and Workspace. Added
  `catalog_read_routes_reject_student_access` to prevent regressions on student catalog reads.
- Added learner-facing assignment outcomes on the overview page from `/api/assignments/{assignment}/summary`:
  students now see current, latest, and best score, completed runs, total attempts, and last activity
  before they start practice.
- Added a compact progress line to student assignment cards and made course, assignment, and
  gradebook pagination announce count-based completion states such as `Loaded N ...` instead of the
  old `All N ... are shown.` wording. The recovery text now describes the already visible items with
  singular/plural grammar.
- Student assignment cards now keep both current and latest scores in the compact progress line
  when both are available, alongside best score and completed runs.

### Fixes and Maintenance

- Accepted immutable migration `2026081802_public_references_byline.sql`. It adds the course-group
  public scalar and normalized public-byline projection, recreates the dependent security-invoker
  catalog view in dependency order, and keeps public attribution separate from private authorship
  authority. The view-dependency ordering repair is retained as implementation history, not a final
  failure.
- Added accepted immutable migration `2026081801_course_term.sql`, keeping native start date, end
  date, and time-zone text on the existing course row with non-null, bounds, order, and shape
  constraints. PostgreSQL reads rebuild the shared value and fail unavailable on corrupt stored
  terms; no second table, database IANA enum, backfill, default, index, or legacy path was added.
- Completed the pre-production Question ID cutover for flat publication and the retry-corpus
  simulator. Browser consumers now accept only native, published catalog summaries with the
  requested scope, resolve browser-safe public detail by Question ID, and reject mismatched or
  answer-bearing responses without restoring internal problem/version identifiers.
- Made the Library detail route share the Instructor/Sysadmin boundary with catalog search, and
  prevent its data-owning component from mounting for a student deep link. The route shell and
  navigation now read the same centralized role contract.
- Repaired PostgreSQL roster support semantics so every successful Sysadmin replay or no-op remains
  audited, expired invitation replays materialize the terminal state and cancel pending delivery,
  and enrollment live tests retain their exact runner-visible names while living in cohesive files.
  Forward migrations also qualify output-shadowed columns in invitation-delivery claiming and email
  replacement session revocation.
- Consolidated repository-local private E2E state under the Python lifecycle owner. Chapter One,
  walkthrough, host-seed, and replica runners now use descriptor-anchored, identity-checked cleanup;
  the duplicate Node owner and implementation-shaped E2E-import pytest modules were removed.
- Reclassified reconstruction probes using the permanent-test checklist. Exact fixture layouts,
  environment inventories, timing defaults, private source positions, and duplicate consumer-level
  cleanup attacks were removed, while public behavior, authorization, decoder, and shared-owner
  security assertions remain.
- Corrected two live acceptance defects found only by the complete service gates: extracted roster
  SQL no longer sends literal backslashes to PostgreSQL, and course-appearance delivery tests treat
  signed URLs as the opaque object-store capabilities their contract declares instead of requiring
  the in-memory adapter's query format.

### Decisions and Failures

- Repaired the professor M1 dependency graph before implementation: WP-PROF-S5 is now the sole
  current package and owns typed `EntitlementDecision` reasons, applicable group-purpose scopes,
  derived authority, and the materialization seam. WP-PROF-S3 waits for accepted S5 output and then
  consumes it for policy composition; it does not reconstruct entitlement. The three unimplemented
  migration reservations were reordered as S5 `2026081803`, S3 `2026081804`, and S4 `2026081805`,
  preserving the forward dependency sequence without a placeholder or out-of-order file. None of
  those packages is accepted by this planning repair.
- Reconciled the professor roadmap with the release track: evidenced M0 is accepted for the professor
  track, the sole global current-package handoff is recorded in
  [implementation_status.md](active_plans/implementation_status.md), and the release queue is parked
  at still-open WP-RC8. WP-RC8, WP-RC12, and production activation remain open.
- Accepted WP-PROF-S1 on 2026-08-18 after recording the four product decisions in
  [docs/HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md): teaching-course term and time zone, identity-free
  rehearsal, an actionability-gated cross-course attention surface, and anonymous catalog evidence
  with `insufficient evidence` below disclosure strength. Independent acceptance review returned
  ACCEPT with no P0/P1/P2 finding.
- Accepted WP-PROF-S2 on 2026-08-18. The normalized course-owned term has no Alpha flag: active
  teaching courses carry the mandatory term, while the later Alpha curriculum aggregate has its own
  identity and instantiates a term-bearing teaching course. The sole current-package handoff now
  names dependency-ready WP-PROF-S7; the release queue remains parked at open WP-RC8.
- Accepted WP-PROF-S7 on 2026-08-19 after independent PostgreSQL/RLS, Rust-contract, and
  frontend/HCI reviews each returned ACCEPT. Its completed serial-core boundary advances the sole
  professor handoff to WP-PROF-S3, the effective-policy resolver. WP-RC8 remains parked and open;
  this does not claim provider or mailbox delivery, passkeys, multi-replica operation, deployment,
  or release activation.
- Recorded planning weights for professor and student viewport work. The weights guide design
  planning only; they are not test quotas or telemetry targets. The email-unconfigured route-around
  uses fictional local identities, direct local roster membership, and copyable links without
  accepting production authentication or onboarding.
- Registered the six named M1 schema-package reservations (`2026081801` through `2026081806`) in the
  shared migration ledger owned by the release integrator. No placeholder SQL or amendment of
  accepted files is allowed. WP-PROF-E2 may prepare a candidate baseline earlier, but actual replacement
  requires professor WP-PROF-E2 readiness plus all repository-owned release schema packages/RC12,
  immediately before first production data.
- Repair iteration and acceptance closeout: centralized the changing current-package handoff and migration allocation in
  `implementation_status.md`; plans now own scope and dependency order and Human Guidance records
  only the durable authority rule. The professor allocation rule is schema-only, the database
  reference is a physical migration inventory, and the open provider/mailbox/passkey,
  multi-replica, security, and HCI gates remain owned by WP-RC8. WP-PROF-S1 is accepted, and
  WP-PROF-S2 is the next dependency-ready package.
- Repaired the global package-identity collision by reserving `WP-PROF-*` for the active professor
  roadmap. The status registry now records accepted WP-PROF-S1 and names WP-PROF-S2 as the sole
  current package, the six M1
  reservations use WP-PROF-S2/S7/S3/S4/S5/S6, and the baseline condition waits for WP-PROF-E2; legacy
  walkthrough package IDs remain in their historical scope.

### Developer Tests and Notes

- WP-PROF-S6 final material-tree evidence is green for `./check_rust.sh`;
  `./check_codebase.sh` (five checks and 278 Node tests); the full Python suite
  (`source source_me.sh && python3 -m pytest -q tests`) passed 5,480 tests and 2 subtests;
  outside-sandbox `./run_playwright_tests.sh --build` passed 231 of 231 with zero skips; the
  36-artifact screenshot ownership/provenance verifier and both diff checks also passed.
  A fresh PostgreSQL 17 baseline applied all 42 migrations, passed the selected course-grade
  scheme/totals/export/RLS and 1805-to-1806 upgrade/retention oracles, completed the role and schema
  inventories, and cleaned its disposable project. All seven aggregate acceptance lanes passed;
  independent architecture/security, tests/HCI, and documentation/authority reviews returned ACCEPT
  with no unresolved P0--P3 finding.
- WP-PROF-S7 final material-tree Validation passed: `./check_rust.sh`; `./check_codebase.sh` (five
  checks and 264 Node tests); `source source_me.sh && python3 -m pytest tests/` (5,235 tests); and
  outside-sandbox `./run_playwright_tests.sh --build` (203 of 203, zero skips). The fresh
  PostgreSQL 17 `tests/e2e/e2e_database_baseline.sh` run applied 38 migrations and passed the S7
  live reference/byline oracle, RLS denial matrix, and cleanup. Both diff checks passed. Release
  Wasm input was 1,122,735 bytes, bindgen raw 1,059,562 bytes, gzip 231,897 bytes, SHA-256
  `b04c1572d361b10518138e2090a67a33ca78de795f44c175f3cde6b4d7264d15`; versus accepted S2, those
  deltas are +373, +405, and -14 bytes. The live baseline is one-time database evidence; no
  networked regular test or new fixture was added.
- WP-PROF-S2 acceptance evidence: `./check_rust.sh`; `./check_codebase.sh` (five checks and 261
  Node tests); `source source_me.sh && python3 -m pytest tests/` (5,235 tests); outside-sandbox
  `./run_playwright_tests.sh --build` (203 of 203); and outside-sandbox
  `tests/e2e/e2e_database_baseline.sh` (37 PostgreSQL 17 migrations, exact course-term constraint,
  round-trip, and RLS oracle) passed on the final material tree. Both diff checks passed, and the
  database/domain, browser/HCI, and architecture/test final reviews returned ACCEPT with no P0--P3
  finding. The release Wasm gzip result was 231,911 bytes (+353). Test-only repairs use the bounded
  real-Tab helper for native date controls and give browser term decoding its focused owner; no new
  fixture or networked regular test was added.
- WP-PROF-S1 acceptance validation evidence: `source source_me.sh && python3 -m pytest
  tests/test_markdown_links.py tests/test_ascii_compliance.py` passed 1,471 tests; `source
  source_me.sh && python3 -m pytest tests/` passed 5,235 tests in 3.13 seconds; and both `git
  diff --check` and `git diff --cached --check` passed. Independent acceptance review returned
  ACCEPT with no P0/P1/P2 finding, so WP-PROF-S1 is accepted.
- Final repository-owned validation passed on the material tree: `./check_rust.sh`;
  `./check_codebase.sh` with 261 Node tests; `source source_me.sh && python3 -m pytest tests/` with
  5,235 pytest tests; and `./run_playwright_tests.sh --build` with 203 built-browser tests. The
  outside-sandbox `source source_me.sh && python3 local_stack.py acceptance` also passed all built,
  visual, walkthrough, Chapter One, and live WebWork browser lanes with no required skip.
- Disposable live validation passed the aggregate five-lane non-browser runner, PostgreSQL RLS,
  WP-RC8 migration/outbox/account/roster authority, WP-R2 host-seed renderer, and combined
  PostgreSQL/MinIO course-appearance gates. Ignored live adapter suites passed 3 iMathAS, 7 WebWork,
  and 4 export tests. These local gates do not satisfy the still-open operator-provider send,
  optional-passkey, account-flow multi-replica, or independent security/HCI acceptance required by
  WP-RC8.

- Completed a read-only codebase and human-interaction review covering the Rust workspace, the
  SolidJS browser, the documented contracts, all 25 committed screenshots, and `OTHER_REPOS/adapt`
  as comparison evidence. Findings and recommendations are in
  `docs/active_plans/audits/codebase_and_interaction_review.md` with the full register in
  `codebase_and_interaction_review_evidence.md`. The review accepts no work package.
- Measured evidence staleness rather than assuming it. A spike compared `src`, `src/style.css`,
  `src/pages`, `src/components`, and `src/features` and found they share one last-change commit
  because this repository lands large batched commits, so narrowing the owning path adds no
  discrimination. Staleness is therefore reported as a commit count rather than enforced. The
  measurement retired the earlier reading that the mock-captured screenshots were current: all 24
  artifacts predate the current browser sources, the 13 mock images by one commit and the 11 live
  images by three.
- Corpus reconciliation found `docs/screenshots/peptide_bond_mastery_overview.png` committed with no
  producing pipeline and no citing document, and no 800 by 1280 artifact for any of the six student
  surfaces, although the design guide already named student pages at that viewport as canonical
  evidence.
- Recorded that `npx tsc --noEmit -p tsconfig.lint.json` fails at HEAD on
  `tests/playwright/roster_ui_accessibility.spec.ts(137,31)`, so `check_codebase.sh` step 2 is red on
  the committed tree independently of this review. Left unchanged as out-of-scope project context.
