## 2026-08-15

### Fixes and Maintenance

- Accepted WP-PY-L1 direct Python lifecycle cutover. Final material-tree evidence includes 4,881
  pytest passes with zero failed, skipped, or warned; all five `check_codebase.sh` stages and 260 Node
  tests; full `check_rust.sh`; and 202 ordinary Playwright tests. Default typed start/status/read-only
  validate, renderer stop/restart plus full WebWork RPC, schema-v2 J11-J13/J1-J5/J8 walkthrough,
  replica/restart durable replay, and the seven-lane aggregate all passed. The final default state had
  zero containers/networks and exactly `containers_ple_pgdata`, `containers_ple_miniodata`, and
  `containers_ple_identity_runtime` retained.
- `final_python_repository_review.ae3`, `final_podman_security_review.c2`, and
  `walkthrough_acceptance_final_review.ae3` each returned ACCEPT with no P0-P3 finding. M0 remains
  open; the release queue remains open at WP-RC8. Live fixes covered renderer OCI-ID normalization, seed database
  environment fallback, unsupported Compose `rm` removal, restart recovery/readiness, semantic renderer
  probing, Chapter private provenance, replica Question-ID manifest, and browser readiness/foreground.
  Sibling `webwork-pg-renderer` dependency fixes are recorded without any staging or commit claim.

## 2026-08-14

### Additions and New Features

- Accepted WP-R0 catalog discovery evidence: exclusive valid Question ID lookup, normalized lexical
  relevance with typo recovery, deterministic ranked keyset paging, HMAC-bound snapshot cursors,
  immediate lifecycle/RLS tightening, and complete bound facets. PostgreSQL is canonical and the
  Memory backend remains the deterministic conformance model. M0 remains open; WP-R1 is next.

### Fixes and Maintenance

- Historical WP-R1 checkpoint: relocated the local Podman controller from root-level scripts into
  `local_stack_control/` to keep the repository root focused. The public lifecycle and aggregate
  acceptance surface was `source source_me.sh && python3 local_stack.py`; the then-retained
  `launch.sh` and `_restart.sh` were private helpers. This records predecessor evidence, not the
  current WP-PY-L1 lifecycle owner.
- WP-PY-L1 now deletes `local_stack_control/launch.sh`, `local_stack_control/_restart.sh`, and
  `containers/local_identity_bootstrap.sh` in favor of direct focused Python lifecycle, private-state,
  identity, renderer-provenance, Podman, polling, and cleanup ownership. Final offline/live Validation
  is green: default typed lifecycle, renderer recovery/RPC, schema-v2 walkthrough, durable replica replay,
  and all seven aggregate lanes passed; the post-run default state had zero containers and three retained
  volumes. At that 2026-08-14 checkpoint, the three named final independent reviews were still pending,
  so WP-PY-L1 was currently validating rather than accepted.
- Strengthened disposable lifecycle ownership. Every closed owner now uses the one required
  `podman-compose --in-pod false` boundary, preventing unlabelled pods from escaping label-driven
  cleanup, and mutation-time ownership revalidates those exact provider arguments. The private typed
  lifecycle accepts a non-default project only when its runner-held mode-0600 capability matches the
  private environment commitment. The replica owner additionally builds with a nonce-scoped
  application image and removes only that tag and its generated gateway tag after capability-bound
  labelled discovery proves the project empty. Cleanup failures retain private recovery evidence
  and fail the run instead of masking leaked state.
- Repaired the live Chapter 1 browser journey at the actual UI boundaries. Sign-in waits for the
  role-specific course workspace, course opening waits for the mounted assignment surface, and the
  instructor visibly opens the native Add-by-Question-ID disclosure before reaching its textarea.
  The journey still uses the public Crockford Question ID and native keyboard actions; it adds no
  fixture, direct focus shortcut, request bypass, or timing sleep.

### Developer Tests and Notes

- Accepted WP-R2 immutable-question release truth on the final material tree. Every content change
  now publishes a fresh `AAA-BBBB` Question ID and opaque exact publication evidence; explicit,
  revision-checked item replacement changes future runs while issued evidence remains exact. The
  legacy correction-propagation, sequential identity, and version-chain paths are removed. All final
  gates passed: `./check_codebase.sh` completed five steps with 260 Node tests, the fast pytest suite
  passed 4,856 tests, `./check_rust.sh` passed the full Rust suite, and the seven-lane aggregate
  passed ordinary browser, two visual, canonical walkthrough, Chapter One pilot, Chapter One browser
  with four live Question-ID replacements, and WebWork render/grade/outage lanes. Test, UI, and
  architecture reviews returned ACCEPT with no P0/P1 finding. The canonical renderer image was
  rebuilt only for acceptance; cleanup removed all disposable containers, images, and volumes. M0
  remains open. At this 2026-08-14 checkpoint, WP-PY-L1 had passed final offline/live Validation and
  was awaiting its named independent final reviews.
- WP-R0 final evidence is accepted on the final material tree: 91 Memory library tests, 3 server
  catalog tests, 1,173 source-line cases, and a clean PostgreSQL 17 baseline with all 32 migrations,
  idempotence, verification, named Store/continuation/disclosure/plan/broker/RLS/ownership lanes,
  and maintained baseline coverage. The independent final review returned ACCEPT. This is bounded
  catalog evidence, not a full-repository, browser, or M0 Validation claim.
- Accepted WP-R1 on the final material tree. Disclosed statistics rendering and the Python-owned
  Chapter One pilot/browser plus aggregate acceptance lanes now use the typed `local_stack_control`
  boundary. `./check_codebase.sh`, `./check_rust.sh`, and the 4,865-case fast pytest suite passed;
  `source source_me.sh && python3 local_stack.py acceptance` then passed all seven required lanes:
  ordinary browser behavior, two visual verifiers, the canonical J1-J5 walkthrough, Chapter One
  pilot, Chapter One browser, and canonical WebWork render/grade/outage acceptance. The designated
  renderer image name remains the stable selection/rebuild target and each live run records its OCI
  configuration ID for exact runtime provenance. M0 remains open; WP-R2 is next.
- Historical WP-R1 live lifecycle evidence proved a rootless default stop retaining its three named volumes, an exact
  confirmed reset followed by a clean launcher rebuild, stateless renderer recreation, replica
  replacement with durable replay, conflict refusal without mutation, and capability-scoped cleanup
  of every disposable project. The final local state is `containers` stopped with zero containers,
  three retained data volumes, and zero networks.

## 2026-08-13

### Additions and New Features

- Added the predecessor root-level local-stack controller for the rootless local Podman
  Compose stack. It provides read-only diagnostics, project discovery, status, scoped logs, normal
  stop, stateless restart, deliberate reset preview/confirmation, launcher validation delegation,
  and complete Playwright Validation delegation while keeping the existing launcher as the sole
  build, bootstrap, migration, seed, renderer-probe, and readiness owner.
- Added the PLE interface design guide and an active dependency-ordered UI workstream. The durable
  contract now centers dense instructor work at 1280 by 800, keeps student questions responsively
  composed, restores course palettes in standard presentation, defines optional account-backed
  increased contrast, and requires human-facing typed route references instead of UUID navigation.
- Added one non-sequential Crockford Base32 Question ID per current question, including server-only
  HMAC validation, durable secret configuration, a 100,000,000-question cap, canonical input
  normalization, and stable test vectors. Added assignment-level reuse with whole-assignment and
  per-question checklist actions so instructors do not reconstruct question sets from ID ranges.
- Added the implemented page-level interface review with canonical instructor, tablet, phone,
  standard-theme, and increased-contrast evidence. Shared `--ple-*` density tokens now own the shell,
  rhythm, panels, rows, controls, instructor columns, catalogs, tables, course canvas, and compact
  navigation so later distance adjustments remain coherent and low-risk.
- Added a dedicated 13-page instructor visual corpus using one coherent simulated course, assignment,
  roster, gradebook, library, workspace, and account data set. The reproducible 1280 by 800 capture
  rejects visible, announced, or address-bar UUID exposure before atomically refreshing the docs, and
  keeps normal-flow assignment actions non-overlapping across the captured states.
- Added a concise repository TODO and an execution-ready pre-production database roadmap. They keep
  the current 28-file SQLx history unchanged during feature acceptance, require one reviewed
  clean-cluster baseline before the first deployment, keep schema creation separate from demo
  seeding, and begin the durable forward-only migration ledger only after that baseline ships.

### Behavior or Interface Changes

- Defined the plan-scoped Validation test suite as a mandatory goal-completion contract. A goal now
  remains incomplete until every named permanent, integration, live, one-time acceptance, and
  independent-review gate is green on the final material tree; required skips and unrun gates do not
  count as green, and later material changes invalidate the affected evidence.
- Superseded the older universal high-contrast and oversized-target assumptions. Standard
  presentation retains semantic accessibility and a 5.5:1 ordinary-text target without forcing
  theme colors toward near-black-on-white; focus is element-sized, pointer-oriented instructor
  controls may be compact, and UUIDs are prohibited from visible or announced content, address-bar
  routes, and copyable application links while remaining available to protected internal requests.
- Superseded instructor-facing numbered/versioned question identities. Original-owner corrections
  retain the Question ID, archive the replaced snapshot, and advance future assignment definitions;
  existing run snapshots remain immutable, while a substantive fork receives a new Question ID.
- Revisited all 15 course palettes without rewriting their stored three-color anchors. A centralized
  palette recipe now projects the full canvas plus distinct work, grouping, reading-card, text,
  action, boundary, and active-navigation roles; the theme chooser and contact sheet preview that
  applied system instead of an almost-white panel dominated by the same banner fixture. Standard
  ordinary theme text now measures 5.50:1 through 7.92:1 under a 5.5:1 floor and 8.25:1 ceiling.
  Increased contrast remains an optional presentation-only preference and intentionally has no
  upper contrast ceiling.

### Fixes and Maintenance

- Repaired the unaccepted account-presentation preference source in place: account
  contrast now derives exclusively from a live 32-byte account-session hash through
  two narrow `SECURITY DEFINER` functions owned by a membership-free, forced-RLS
  broker. `ple_auth` no longer has direct preference-table access. The disposable
  PostgreSQL baseline invokes a self-contained SQL oracle rather than adding a
  fixture-driven pytest for this live authorization boundary.
- Added forward migration `2026080935_owner_correction_authority.sql` and an explicit
  server-only owner-correction publication authority. Ordinary publication now rejects a revision;
  only a live original-instructor session can publish the successor. The narrow correction broker
  copies institutional grants and repoints future assignment definitions across tenants while
  preserving issued evidence and recording source and affected-assignment audit events.
- Corrected the unaccepted `2026080918_workspace_flat_question_assets.sql`
  descriptor constraint to require the canonical serialized private bucket
  name, `private-content`. This aligns the durable JSONB identity check with
  the typed `Bucket::PrivateContent` model without introducing a compatibility
  migration or changing the private workspace-asset contract.
- Added forward migration `2026080934_catalog_owner_insert_rls.sql`. Catalog visibility remains
  readable by the current tenant, but only a problem's owning tenant can append a tenant grant,
  immutable version payload, or published source artifact. This closes foreign self-grants and
  visibility-based writes without changing owner publication or institution catalog behavior.
- Added forward migration `2026080933_roster_support_actor_repair.sql`, which
  replaces the applied roster-support actor's `pgcrypto` digest call with built-in
  SHA-256, separates a non-action precheck from the audited Sysadmin actor, and
  moves both `SECURITY DEFINER` functions to the narrow RLS-obeying
  `ple_roster_support_broker` role. The audited actor has no boolean audit
  switch; each successful Sysadmin roster operation records one in-transaction
  event, while direct Instructor membership remains unaudited. The broker has
  only `UPDATE (tenant_id)` on `course_member`, which PostgreSQL requires for
  its `FOR KEY SHARE` membership lock.
- Corrected the WebWork live acceptance contract so same-attempt question GETs
  prove exact persisted attempt/receipt snapshot replay with no adapter
  `renderer_call` or `cache_hit` witness. A fresh continued-practice issuance
  proves exactly one required private replay-mapping renderer call. Its random
  seed/key normally misses the safe cache but can rarely collide, so the live
  cache-hit delta may be zero or one; offline adapter tests own the deterministic
  safe-cache-hit contract. Grading, outage isolation, browser evidence, and
  answer-free projection checks remain required. Durable cache and determinism
  contracts now distinguish adapter safe-render cache work from persisted
  snapshot reads.
- Centralized neutral Podman lifecycle contracts in the predecessor `local_stack_control/` package with typed Compose
  targets, env-file authority, label-based container/volume/network discovery, designed one-shot and
  long-running service readiness, and destructive reset confirmation. Added offline behavior tests
  for those contracts without adding a live Podman pytest or fragile source-shape assertion.
- Corrected the opt-in local-stack acceptance path so its launcher and browser runner wait for
  successful one-shot services and every required long-running service before seeding or running
  Playwright. The selected environment file now owns local composition, the worker receives only
  the secrets it needs, and the renderer stores its runtime PID on a writable temporary mount.
  Repaired the strict keyboard journey's natural tab order, the public navigation route's
  fail-closed policy and camelCase representation, and the catalog-search view's Question ID
  projection. The gradebook pagination harness now uses CSS-capable in-memory esbuild output; its
  five affected tests are green.
- Restored every reported Rust and repository-hygiene gate without adding an exemption, fixture, or
  brittle test. The PostgreSQL ownership oracle now clones its consumed Question ID reference; the
  seed convergence test asserts one current question through `question_id` and the internal version;
  the account-presentation store passes its existing UUID directly; and `sha2` again follows the
  workspace's open latest-first dependency policy. The UI review now links durable documentation
  screenshots instead of ignored generated artifacts. Extracted the assignment policy composition
  and shared instructor data-table styles into capability-owned modules, and condensed completed
  plan history into an explicit permanent-test versus one-time-evidence boundary so all authored
  sources remain below the repository limit without an override.
- Reclassified plan validation under the permanent-test checklist. Removed ten fast-suite pytests
  that inspected current shell, Compose, Containerfile, Caddy, SQL, or dated walkthrough artifacts,
  plus their unreferenced baseline fixture; the stable behavior remains covered by its Rust, Node,
  Playwright, or opt-in E2E owner. Loopback adapter and installed-document-reader Rust checks are now
  explicit ignored acceptance tests, while ordinary worker timing uses paused Tokio time and browser
  tests no longer sleep for eventual state. In the intermediate August 13 measurement, the fast suite
  ran 4,433 tests in 2.31 seconds with no test call above 0.01 seconds, and 203 enabled Playwright
  cases passed below two seconds. The final aggregate is recorded below. The active plans and test
  guides now distinguish permanent tests, one-time scratch evidence, opt-in live acceptance, and
  justified serialized fixtures. The PortSwigger review reference is self-contained and no longer
  names untracked local books.
- Repaired four browser regressions without weakening the application boundary. Course-banner
  tests now exercise protected `POST /api/assets/{id}/delivery` authorization before loading the
  returned image URL, and course-creation and gradebook recovery assertions select their own
  visible status instead of colliding with the global screen-reader announcer. Each repaired
  scenario has a two-second Playwright ceiling. Removed a cumulative 50 ms retry from every native
  Tab step in the shared keyboard helper, reducing the three over-budget pagination/matching cases
  below 800 ms even in the fully parallel suite. In that intermediate August 13 run, all 203 enabled
  browser tests completed below two seconds on the development machine.
- Rebuilt the separately owned stateless PG renderer after an authorized complete local Podman reset
  and refreshed PLE's reviewed immutable repository manifest reference. The launcher now rejects
  mutable renderer tags before image inspection, records the distinct resolved image configuration
  identity, and retains the external ownership boundary. Operator guidance now distinguishes those
  two OCI identities and gives one reproducible build, inspect, review, and configure path.

### Developer Tests and Notes

- One-time, opt-in real-stack evidence exercised the corrected disposable Podman path: instructor
  setup and two complete four-question learner chapters passed in two Playwright tests in 9.4
  seconds after every required service reported ready. The renderer's PID-only, secret-free
  configuration passed its real render/grade probe, and cleanup removed the exact generated project
  and gateway image with exit status 0. No new permanent pytest, fixture, or ordinary networked test
  was added.
- The simulated instructor capture completed with its opt-in Playwright case passing in 1.2 seconds.
  All 13 documentation PNGs are 1280 by 800, below the one-megabyte asset budget, and passed a
  combined contact-sheet review. `./check_codebase.sh` passed typecheck, lint, formatting, and all
  273 Node behavior tests. The final one-time validation record is 4,548 fast pytest cases passing;
  the full ordinary Playwright suite has 210 passing cases and 12 intentional
  opt-in skips. It also includes 13 focused assignment-editor and gradebook browser behaviors, a
  clean production build, and the complete `./check_rust.sh` generation/check/Clippy/test/doc-test/
  Wasm gate passing. Live PostgreSQL, MinIO, renderer, and external document-reader cases remain
  explicit acceptance runs rather than regular networked tests.
- A zero-state rebuild removed all prior local Podman containers, pods, images, volumes, custom
  networks, and build cache, then reconstructed PostgreSQL, MinIO, the immutable external renderer,
  API, worker, and gateway without relying on retained state. The exact launcher passed its fresh
  migration, separate seed, renderer render/grade probe, one-shot, health, and six-service readiness
  gates. After readiness, the ordinary Playwright suite again passed 210 cases with 12 intentional
  opt-in skips in 10.0 seconds; the bootstrapped fast suite passed 4,548 cases in 2.61 seconds, and
  the full Rust gate passed again. No container-state probe was added to the permanent pytest suite.

## 2026-08-12

### Fixes and Maintenance

- Closed the human role model to Student, Instructor, and Sysadmin and added
  `docs/USER_ROLES.md` as its canonical reference. Publishing is an Instructor
  or Sysadmin action backed by a dedicated service identity, never a Publisher
  user. Sysadmin status is operator-approved but supplies no ambient access to
  FERPA course records; general access still requires current direct Instructor
  membership, while roster help and retention use separately closed, audited,
  payload-minimizing Sysadmin permissions. Removed the duplicate effective
  `CourseRole` model so one `CourseMembershipRole` relationship scopes Student
  or Instructor to an exact course without defining more human user roles. The
  data-classification, enrollment, security, database, API, frontend, retention,
  and active-plan contracts now treat all course-linked student educational
  records-not merely roster identifiers-as FERPA data under the radioactive
  handling rule. Added an explicit database table map for especially radioactive
  payload-bearing relations and radioactive linkage relations; partitions, query
  results, dumps, WAL, replicas, snapshots, and restores inherit the highest
  classification of the data they contain.
- Added the repository-owned `./check_rust.sh` front door so the complete offline Cargo gate is
  memorable and cannot be removed when the vendored TypeScript `check_codebase.sh` is refreshed.
  It checks Rust-owned TypeScript/fixture generation, formatting, default and all-feature
  compilation, strict Clippy, workspace tests and doctests, and the browser WebAssembly target;
  contributor docs now assign each root gate to its actual owner.
- Applied the permanent-test checklist to the security rebuild. Removed temporary Python and Rust
  scans that froze retired storage strings, exact container UIDs/tmpfs sizes, drain constants,
  migration SQL spelling, and OpenTofu source layout. Passwordless behavior tests now prove bounded
  delivery and denial without freezing tunable quota values. The two-build local credential artifact
  proof moved from the fast Node lane into the explicit non-browser E2E tier and now inspects the
  build dependency graph instead of UI copy or emitted source strings. Timing-driven TCP shutdown
  probes remain one-time implementation evidence instead of scheduler-sensitive unit tests. The Wasm export
  allowlist also moved there because it performs Cargo and bindgen subprocess builds; it now uses a
  temporary output directory instead of maintained generated-tree exclusions. The native OpenTofu
  plan test now inspects planned resource behavior instead of searching HCL source text. One-time
  rebuild probes remain audit evidence rather than permanent suite obligations.
- Applied the six-pass pre-merge audit. Corrected remaining Manager/Administrator role wording to
  the closed Student, Instructor, and Sysadmin model; repaired the documented root and E2E test
  commands; removed milestone tags from durable code comments; and stopped tests from freezing
  signed-URL expiry values. Restored the consumer-owned ESLint exclusion for disposable historical
  `generated/wasm-export-check/` glue so the vendored `check_codebase.sh` remains repeatable without
  excluding maintained `generated/api/` contracts.
- Completed the end-to-end security architecture audit and replaced the dated active audit with a
  current threat model, trust-boundary map, finding ledger, evidence tiers, and explicit live
  activation gates. Clean pre-production corrections now enforce the full passwordless
  session/logout lifecycle, exact Host/Origin and safe-method policy, actor-scoped learner and
  Student/Instructor authority, PostgreSQL login/capability attestation, four isolated storage/KMS domains,
  post-commit immutable public publication through a dedicated publisher, protected POST asset
  delivery, strict image/container validation, and durable external-provider uncertainty fencing.
  The browser/Wasm surface remains answer-free, containers and the declarative AWS topology are
  least-authority by design, and independent re-reviews found no remaining repository-owned P0--P2
  architecture defect. Production activation remains blocked on named disposable AWS/RDS,
  IAM/KMS/S3, edge/parser/cache, backup/restore, provider, email/WebAuthn, and renderer provenance
  evidence; static tests do not claim those live properties.
- Accepted the bounded human-guidance workflow after rerunning its real boundaries rather than
  relying on stale plan checkboxes. The clean-stack no-email teaching loop now covers visible
  four-ID instructor copy/paste, the 15-minute run, keyboard completion, fresh practice, and the
  two-run gradebook; the canonical eleven-image screenshot set was rebuilt and inspected; and the
  exact Genetics-plus-Biochemistry eight-question browser sweep passed separately. The complete
  disposable PostgreSQL baseline applied all 18 tracked migrations and passed its receipt,
  corruption, roster, catalog, timing, RLS, role, QTI, and flat-grading oracles. The seed now treats
  roster-owned student membership as canonical mutable course state while retaining exact
  instructor/course identity, the catalog helper resolves the requested `P-n-v1` instead of
  assuming v2, and the Chapter browser E2E removes only its exact generated gateway image tag.
  Fastmail/email activation and full HOTSPOT author-to-learner object-lifecycle acceptance remain
  explicit future boundaries rather than hidden gates.
- Restored the fast TypeScript codebase gate without weakening its contracts. Prefetched successor
  acceptance now has one typed receipt-binding policy that checks every descriptor available before
  submission, with behavioral mismatch coverage instead of source-text matching. Mock issued
  questions are validated through the production strict presentation decoder rather than a brittle
  exact-key inventory, so the public nonce can evolve as part of its reviewed wire contract while
  answer-bearing additions still fail closed. Walkthrough helpers now model synchronous work as
  synchronous, and the repository's local lint policy explicitly recognizes Playwright's honest
  zero-dependency fixture signature instead of inventing a browser dependency to satisfy lint.
- Reduced Rust development storage after a measured broad-gate workload grew `target/` to 136 GB:
  the shared dev profile now disables incremental compilation and retains line-table debug
  information instead of full debugger data. Cargo tests inherit the same profile, filename/line
  backtraces remain available, and documented environment overrides preserve an explicit full-debug
  escape hatch without making the high-storage mode the repository default. After two live browser
  builds plus workspace all-target check and strict Clippy, the retained cache measured 6.0 GB with
  no incremental state; 20 GB is now the documented local investigation threshold.
- Tightened the shared learner and instructor visual hierarchy so course pages read as compact
  academic workspaces instead of nested padded cards. Course identity is now a narrow color rail on a
  white reading surface; course and assignment entries use ruled scan rows; question, feedback,
  roster, appearance, and authoring sections reserve bounded surfaces for actual controls and
  teaching feedback. Learner response targets are a compact 44 pixels tall, ordinary controls are
  36 pixels tall, and action groups wrap only when their labels no longer fit. The canonical design
  and screenshot canvas is a 1280 by 800 laptop browser; 800 by 1280 covers the representative
  student/tablet path, while one narrow-phone check remains a compatibility guard rather than the
  product's density baseline. A newly issued response now starts with neutral completion guidance
  rather than a red validation error before the learner interacts. The canonical eleven-image live
  walkthrough capture was refreshed from the isolated local stack, including the previously missing
  Genetics assignment overview, a visibly focused unanswered response, repeat-practice completion,
  and the two-run instructor gradebook.
- Removed the synthetic one-question WebWork course from normal and canonical local-stack seeding.
  The launcher now publishes only the reviewed Genetics and Biochemistry Chapter 1 teaching corpus,
  keeping its ordinary catalog, assignment lists, and screenshot evidence representative. The
  renderer acceptance retains the licensed fixture behind its explicit E2E command and stores its
  answer-free manifest only in that test's private temporary directory.
- Expanded the protected flat-question authoring surface across all eight version 2 families while
  keeping accepted answers and feedback out of the learner-equivalent preview. MC, MA, FIB,
  MULTI-FIB, NUM, MATCH, and ORDER now provide family-specific keyboard-first controls; incomplete
  numeric literals block save/review, and MATCH add/remove/reorder operations preserve semantic
  identities. HOTSPOT now starts only from a server-verified image descriptor and provides labeled,
  normalized region controls without exposing storage details. Its immutable publication,
  issue-time asset binding, and real object-lifecycle acceptance remain open.
- Restored the reviewed Cargo dependency-policy boundary: direct registry dependencies may use
  `version = "*"` or an audited open `version = ">=LATEST"` minimum, while caret, exact, tilde,
  and upper-bound requirements need a documented repository-specific exception. The durable rule
  now lives in `HUMAN_GUIDANCE.md`; Rust, PyO3, and Wasm style guides point to it rather than
  independently requiring wildcards. The manifest gate accepts both open forms. Existing wildcard
  manifests and the reviewed `Cargo.lock` resolution are unchanged.

## 2026-08-11

### Additions and New Features

- Added the reviewed source-of-truth corpus for the first Chapter 1 release: Genetics and
  Biochemistry each have one WeBWorK MC, one WeBWorK MATCH, one static PLE flat MC, and one static
  PLE flat MATCH, for eight questions total. The human-readable manifest records titles, source
  paths, upstream revision, license, selected Blackboard item codes, checksums, and explicit
  Biochemistry wording corrections without using UUIDs as author-facing identities. The new
  `cargo tools pilot-content` gate validates the exact four-per-chapter matrix, parses the selected
  MC/MATCH source records, compiles all four flat v2 payloads into answer-free public and private
  grading halves, and proves correct and wrong grading. The host-only Chapter 1 release seed now
  publishes all eight immutable source/catalog records, protected static grading material, two
  four-item assignments, and roster-derived learner enrollments through production PostgreSQL and
  MinIO contracts. Its disposable E2E gate passes an exact rerun, verifies the persisted
  four-native/four-WeBWorK split, and requires distinct `P-...-v1` display identities. Both reviewed
  PGML shapes also pass direct live renderer grading, including matching partial credit. The exact
  built-browser learner gate now completes all eight questions through visible keyboard controls,
  sees feedback after every submission, and reaches fresh practice after each chapter without
  consulting answer keys.
- Added representative release content to the canonical walkthrough without replacing its accepted
  report schema: the instructor catalog step now verifies the visible `P-...-v...` identity and
  backend label with no UUID text, and a separate required phase completes all four Genetics Chapter
  1 questions. The isolated eight-question gate remains the complete Genetics-plus-Biochemistry
  release oracle.
- Expanded the accepted real-stack documentation capture from three images to the complete eleven-stage
  instructor and student story: fake course, two-member fake roster, published problem, assignment
  policies, post-create assignment confirmation, assignment list and overview, timed problem, scored
  retake control, a visibly new Practice run 2, and multi-learner gradebook history. The capture
  still walks visible controls, now uses the unmistakable identities
  `Dr. Fake Instructor`, `Mary Fake Student`, and `Jack Fake Student`, and gives demo course/problem
  titles concise `Fake` labels. Fake-user screenshots are required acceptance evidence; credentials,
  answer material, traces, and raw child output remain excluded.
- Added dedicated instructor and student guides with three fresh real-stack
  Playwright screenshots from the accepted local no-email teaching loop. The
  reproducible capture command follows the visible keyboard journey, records
  assignment overview, repeat-practice, and expanded gradebook states, and
  cleans its private temporary directory and Podman stack. The public
  walkthrough uses AUTO bundle reuse by default; `--build` is its only explicit
  build override.
- Accepted the corrected local no-email teaching-loop pilot. Two independent
  retained-stack `bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build` runs visibly created fresh
  instructor courses, activated the configured local student, and constructed corpus-backed Mastery assignments.
  The student completed and repeated by keyboard; J5 confirmed Best/Latest `100%`, Completed `2`, and two histories.
  Canonical schema-v2 reports retained J11/J12/J13/J1/J2/J3/J4/J5/J8 PASS rows and only the corpus arrangement,
  remained redacted mode 0700/0600 artifacts, and cleanup left no containers or private state. Email/canonical
  onboarding, J6/J7, all-family, multi-learner, and release acceptance remain outside this bounded pilot.
- Added `docs/CONTAINER_PORT_MAPPING.md` for loopback/local private mappings, the `8080`
  gateway default, reserved port ranges, and the explicitly unimplemented AWS ALB/RDS/S3 boundary.
- Added the accepted J2 keyboard-only Mastery retry journey. The forced-build real-stack run records
  only ordered public J1/J2 visible outcomes and retains no browser artifact, answer material, or score;
  the later retained-volume M5 closeout accepts J3-J5/J8 separately.
- Added the typed Python `argparse` walkthrough runner behind its stable shell entrypoint. AUTO
  reuses safe built assets when available, builds when they are absent, and `--build` explicitly
  refreshes them. The accepted M2 run validates only public, mock-free IPv4 `/health` readiness;
  it makes no learner-journey, authentication, enrollment, or content-arrangement claim.
- Added the accepted supported-API retry corpus and Mastery/Exam contrast arrangement in the
  launcher-seeded course. Product assignment creation reconciles the existing learner, whose
  rendered local sign-in flow opens the visible course and current assignment cards. Exact
  Python-runner same-seed replays use IPv4 only and no secrets, SQL, account, or enrollment
  fixtures; this is arrangement evidence, not journey, scoring, or enrollment simulation.
- Added the accepted first keyboard-only learner journey and its visible-outcome report. The
  Python runner uses IPv4-only local access and AUTO build reuse, while the learner uses rendered
  controls without a pointer action or answer reconstruction. The private report remains mode 0600
  in a mode-0700 directory and retains no trace, screenshot, video, or temporary state artifact.

### Fixes and Maintenance

- Removed the pre-production PLE flat-question JSON v1 reader, compiler, family registration,
  private-payload branch, current fixtures, and authoring contract. Native flat content now accepts
  only the closed v2 eight-family source contract, including the v2 single-choice editor and QTI
  profile bridge. QTI profile v1, presentation v1, migrations, and `P-<number>-v<version>` remain
  separate current protocols rather than compatibility shims.
- Replaced the UI walkthrough's inherited hidden Python `PLE_*` run switches
  with documented `argparse` choices and a single explicit, schema-versioned
  private input file for fixed Node and Playwright children. The runner clears
  inherited walkthrough overrides before it starts owned children; private
  input files remain mode 0600 within a mode-0700 runner directory. Focused
  offline boundary checks cover the durable contract. The rebuilt Podman and
  browser walkthrough remains pending one-time acceptance evidence.
- Decoupled the persistent-volume canonical UI walkthrough from the Chapter 1
  all-eight learner sweep. Its default lifecycle now remains J11/J12/J13 then
  J1/J2/J3/J4/J5/J8 against the instructor-created four-question Genetics
  assignment, so reruns preserve student progress rather than resuming it as a
  release sweep. `bash tests/e2e/e2e_chapter_one_browser.sh` remains unchanged
  as the isolated disposable Genetics-plus-Biochemistry release oracle.
- Made the human-readable Chapter 1 manifest the publication seed's source of truth for course and
  assignment display names, question titles, families, point values, and source paths. Validation
  now rejects flat-payload title drift and unsupported point values before the seed publishes, and
  the launcher/FAQ wording now accurately describes the private renderer used by the normal local
  stack.
- Replaced visible assignment-editor UUID tuples with the catalog's copyable human identity and
  backend label, such as `P-1-v1` with `WeBWorK`. Existing assignments now resolve their immutable
  catalog titles and display identities when loaded, while saved requests and walkthrough
  selectors retain the exact internal references without presenting them as problem numbers.
  The editor now makes that identity operational: an instructor can paste one or more comma- or
  newline-separated exact IDs into **Add by question ID**, resolve their immutable published
  versions, and add them atomically. Invalid, unavailable, unauthorized, or duplicate input keeps
  the pasted text and assignment unchanged for correction and retry.
  Focused model, type, production-build, and seven-scenario rendered editor checks pass, including
  reload, conflict recovery, keyboard assignment creation, and assertions that UUIDs are absent
  from selected-question text.
- Extended flat-question storage and publication admission from the legacy single-choice family to
  the complete closed flat v1/v2 family registry. A real author route test and the disposable
  PostgreSQL migration/RLS baseline now publish and grade v2 matching through the protected grading
  boundary.
- Corrected host seeding after the production roster path began creating opaque learner and
  enrollment identities. Seed reruns now resolve and preserve the roster-created enrollment instead
  of treating `UserId` as `StudentId` or assuming a deterministic enrollment UUID; the live Chapter
  1 publication oracle caught and verifies this cross-capability behavior.
- Raised the discarded private-renderer JWT token limit from the obsolete 64 KiB assumption to the
  already enforced 1 MiB response-body boundary. The real Chapter 1 browser path exposed that
  reviewed Genetics PG state legitimately produces larger private answer/session tokens; the tokens
  remain server-only, format-validated, bounded, and absent from browser projections. The same path
  exposed reviewed PGML choice labels containing a narrowly styled color span; the adapter now
  projects that exact reviewed shape to plain text while continuing to refuse arbitrary or hostile
  label markup. Matching projection now accepts the current renderer's exact direct-select,
  mixed-label, and empty compatibility-control shape while retaining strict attribute and hostile
  markup refusals. Both tracked matching PGML sources now emit numeric partial-credit scores rather
  than string-formatted JSON values, so grading stays inside the renderer's numeric score contract.
  Independent audit then bound partial-credit admission to each reviewed path plus immutable source
  SHA-256, moved the flat-v2 grading-function widening into a new forward migration rather than
  changing an applied migration, and expanded the live gate to prove visible instructor and student
  sign-in plus the seeded catalog's human identity.
- Rotated the complete 2026-08-09 changelog day into
  `docs/CHANGELOG-2026-08b.md`, retaining the two newest day blocks here.
- Completed the independently accepted post-pilot WP-E2 runner refactor. The
  documented UI walkthrough command remains stable while reusable Python
  configuration, strict contracts, process values, and lifecycle orchestration
  live in the importable dedicated `tests/walkthrough/walklib/` package; fixed
  subprocess children live beside it and Playwright journeys remain separate.
  Walkthrough tests and helpers now use behavior-based names, with planning
  labels retained only in the serialized report schema. A
  fresh host-bound retained-stack run passed the exact nine-row schema-v2
  no-email teaching loop and independent review confirmed redaction, 0700/0600
  modes, no-volume cleanup, and no residual containers or private state.
- Applied the six-pass pre-merge audit's low-risk repairs: corrected the
  walkthrough runner's Boolean fixture, removed disposable exact-baseline and
  filename-policy pytests, synchronized opt-in test-layout and bulk-E2E docs,
  clarified the WP-O1 compatibility facade, and tightened walkthrough-library
  comments without changing runtime behavior.
- Corrected the walkthrough charter to require visible instructor course/roster/corpus assignment
  setup and student keyboard take/score/repeat. Email and canonical onboarding are outside it.
- Repaired native unlimited-Mastery retry receipt handling and the learner receipt transition, then
  added exact redacted Python runner-stage diagnostics and accepted static harness/baseline gates.
  Visible native retained-page pagination now passes M5 J1/J2/J3/J4/J5/J8; PostgreSQL, M6,
  canonical onboarding, all-family, and release claims remain open.
- Completed the independently accepted live walkthrough orchestration gate: its private redacted
  report and no-volume cleanup leave no containers after a successful run. WP-A1 retry corpus work
  is next.

## 2026-08-10

### Additions and New Features

- Added deterministic named simulator RNG substreams and stable allocation/report ordering with six
  focused tests. This is offline simulator support only: it makes no enrollment, scoring, or browser
  journey claim, and M2 live orchestration remains next.
- Added PLE flat-question JSON v2 as the closed internal source contract for multiple choice,
  multiple answer, fill-in-the-blank, multi-blank, numerical entry, matching, ordering, and image
  hotspot questions while preserving exact v1 single-choice compatibility. The native compiler now
  separates answer-free render definitions from server-only grading keys for all eight families;
  strict browser decoders and Solid response controls cover their exact wire shapes, and the
  no-mouse suite exercises the new compound and hotspot region-list interactions. The design adopts
  the reviewed QTI Package Maker item semantics where available and treats hotspot as a bounded PLE
  extension. Visual family authoring, external QTI-JSONL interchange, all-family PostgreSQL/object
  acceptance, hotspot pointer/media workflows, pilot content, and independent WP-RC4 review remain
  explicitly open.
- Added the durable PLE no-mouse accessibility contract. It makes the platform path primary: Tab and
  Shift+Tab move focus, Space selects choices and activates visible buttons, and the complete
  course-to-mastery journey reaches explicit submission without a widget shortcut. Arrow keys,
  digits 1-9, response-input Enter-to-submit, and Escape now have separately named extension
  scenarios, so failures identify either a platform accessibility regression or a PLE shortcut
  regression. Visible response hints state the platform action first, digit shortcuts are scoped to
  focused choice inputs, and the contract covers focus, recovery, keyboard-safe future families,
  permanent-versus-human evidence, the passed live WebWork extension path, and a permanent axe gate
  for serious or critical student-surface regressions.
- Accepted the implementation-ready secure grading payload decision before WP-RC5. Learner
  submission authority is the authenticated attempt plus idempotency key; each selectable rendered
  object receives a collision-checked, presentation-scoped CRC-16/CCITT-FALSE ID, while a separate
  SHA-256 descriptor detects whole-presentation mismatches. The plan defines minimal wire shapes for
  all eight flat families, server-only partial credit, an atomic persistence/cutover migration,
  one-call normal WeBWorK grading state, timed-prefetch policy, browser recovery, measured latency
  gates, and an evidence-based LibreTexts ADAPT comparison. A fresh source audit distinguishes the
  required render-schema `kind` from the redundant submission `kind`, retires broad learner attempt
  projections, and replaces the current seven-request run-screen assembly with one minimal learner
  screen. Added `docs/ASSESSMENT_PAYLOAD_DESIGN.md` as the durable current-versus-target payload,
  ADAPT comparison, native/WeBWorK grading, consistency, caching, and prefetch guide. Added
  `docs/MULTI_SERVER_SETUP.md` as the current guide to local Caddy/API/worker scaling,
  shared state, health, private WeBWorK isolation, failure behavior, and the explicitly
  planned-not-implemented WP-RC10 production topology.
- Added the decision-complete secure learner file-upload plan while preserving the current
  fail-closed route and widget. The plan replaces browser-supplied object keys with one opaque
  attempt-bound upload ID, streams bytes through an authenticated same-origin API into
  non-deliverable temporary storage, verifies SHA-256 and a closed PDF/text/PNG/JPEG profile,
  requires private malware inspection before promotion, and atomically consumes one ready object
  into the existing manual-grading path. It also owns forced RLS, typed object keys, retention,
  reconciliation, protected attachment delivery, multi-replica recovery, keyboard accessibility,
  `2026080912_secure_learner_uploads.sql`, and six dependency-ordered implementation packages before
  production deployment. No learner upload has been enabled by this planning change.

- Replaced mutable local PostgreSQL, MinIO, and MinIO Client image tags with required immutable
  digest settings. A networkless, read-only pre-start guard now refuses a retained PostgreSQL data
  volume unless its declared major is exactly the supported PostgreSQL 17 baseline. Focused launcher,
  topology, shell, and whitespace checks pass; live Compose configuration remains unclaimed while
  the local Podman machine is unavailable.

- Converted the remaining source-ownership debt into WP-ARCH1 rather than leaving a general
  refactoring note. The package inventories 26 maintained files at 1,000 lines or more, assigns
  persistence, server, adapter/tooling, browser, integration, and independent-review ownership,
  names exact capability destinations and gates, preserves stable facades and behavior, and requires
  a permanent test with no maintained-code exception. It runs after the passed WP-RC3 live gate and
  before final RC3 close-out and WP-RC4
  so later payload and flat-family work extends focused owners.

- Completed WP-ARCH1's implementation and integrated validation without claiming independent
  acceptance. Twenty-six oversized maintained sources now use capability-sized persistence, server,
  adapter/tooling, and browser owners behind stable facades; the untracked-aware inventory reports
  zero maintained-code violations. The permanent size boundary passes 573 cases, the full Python
  suite passes 2,442 tests, the eleven-stage codebase gate passes, and Playwright passes 64 tests
  with two deliberate opt-in skips. One-time symbol, compiler, CLI, and layout probes were removed or
  retained only as evidence rather than becoming permanent tests.

- Refreshed the newcomer and contributor documentation set. README, architecture, installation,
  usage, and file-structure guidance now describe the current local stack and the bounded WeBWorK
  acceptance boundary. Added focused references for development, adapters, database
  tenancy, object storage, input formats, troubleshooting, frequently asked questions, and related
  projects. The release documents remain unchanged because this mixed worktree is not a release.

- Consolidated durable platform decisions and contracts into a navigable documentation set: design
  decisions; assessment lifecycle; API, authorization, identity, data, storage, concurrency, backend,
  cache/prefetch, recovery, mastery, and evidence boundaries. The references distinguish current
  behavior from reserved work, including payload cutover, security mechanisms, and the limited RC3
  WeBWorK acceptance boundary; they do not claim planned features as shipped.

- Added the proposed enrollment design after confirming the current HTTP gap and comparing the
  LibreTexts ADAPT roster workflow. The target gives instructors one course-level roster action while
  retaining separate learner identity, course membership, assignment enrollment, and summary
  records. It uses one global opaque PLE account identity, treats email as the mutable canonical
  sign-in attribute, offers passkeys only as convenience credentials, uses hashed invitations,
  reconciles every current assignment atomically,
  preserves records on access revocation, and keeps course-scoped roster metadata separate from the
  cross-course account.

- Implemented the WP-RC8 passwordless identity and enrollment slice with acceptance still open.
  PLE-owned opaque accounts now support uniform browser-bound email authentication, discoverable
  WebAuthn, multiple passkeys, verified email replacement, and authorized course-context selection.
  Course Instructors can page rosters, enforce exact email domains, send or revoke invitations,
  preview and atomically commit bounded `email,roster_id` CSV files, revoke access, and download a
  synchronous no-store manual grade CSV. Invitation claim and later assignment creation preserve the
  membership-by-assignment enrollment/summary invariant in both stores. Migration 0909 passed the
  fresh/no-op/verify and disposable PostgreSQL/RLS baseline; the all-feature Rust workspace and
  browser suite pass. Off-the-shelf email-authentication/optional-passkey and multi-replica evidence
  plus independent security/HCI closeout remain before WP-RC8 acceptance.

- Removed SMTP as a prerequisite for single-student invitation handoff without weakening email
  authentication. An authorized instructor now receives one no-store, same-origin, fragment-based
  redemption link and can copy it into an existing trusted LMS; configured SMTP through the
  established `lettre` adapter remains an optional delivery channel. The server stores only the
  invitation hash, deterministic idempotent retries reproduce the same link, later roster reads stay
  secret-free, and the Solid roster page keeps the bearer link only in page-session memory with
  keyboard-accessible copy guidance. The local launcher now installs a separate mode-0600 issuer
  secret through a networkless UID-owned volume rather than introducing a PLE mail server. The
  11-stage codebase gate, 3,190-test Python/documentation suite, full browser suite with 76 passes and
  two deliberate opt-in skips, and live local create/revoke check pass; canonical
  email-authentication acceptance still requires the operator-selected SMTP provider.

- Documented the local identity boundary used by enrollment walkthroughs. Local-file sign-in creates
  only a tenant-scoped `ple_session`; invitation redemption requires a persisted PLE account and
  `ple_account_session`, and passkey registration begins after that account exists. The supported
  path therefore uses canonical email authentication before claim. Copy-link delivery bypasses SMTP
  only for invitation handoff, while course and assignment creation remain the two arranged setup
  steps until their instructor UI exists.

- Completed the repository-owned WP-RC8 production account composition without claiming package
  acceptance. `production_router_from_env` now enters the provider-free PLE passwordless/account/
  session graph with an eight-hour `FirstPartyHttps` policy and explicit `ReviewNotRequired`; it
  neither reads local identity settings nor mounts `/api/auth/login`. The local-file launcher stays
  available only through the exact development flag. A live operator-selected external SMTP send,
  optional-passkey and multi-replica browser evidence, and independent security/HCI closeout remain
  required before WP-RC8 acceptance.

- Explored and then rejected a separate course-scoped account-recovery design. Because verified
  email is PLE's canonical registration and sign-in path, passkeys are optional conveniences rather
  than a stronger credential required for ordinary access. A course Instructor may revoke and
  re-invite
  but cannot prove that two accounts belong to the same person strongly enough to transfer
  educational records. Version 1 therefore has no Instructor account merge or record-transfer path;
  object reconciliation, LTI, and secure upload retain the next migration slots 0910 through 0912.

- Removed the unaccepted REC1/REC2 account-transfer foundation before it became a durable schema or
  HTTP contract. Account sessions no longer persist an authentication-method distinction with no
  remaining authorization purpose; email and passkey completion mint the same bounded session.
  Removed the recovery invitation relation, the cross-account learner mapping relaxation, both
  Store transfer implementations, and their temporary tests. The closed email challenge purpose is
  now registration/sign-in or verified email change only. The disposable PostgreSQL baseline passes
  all nine migrations and its passwordless, roster, role-separation, RLS, and existing data-path
  oracles with no recovery relation present.

- Added the dated 2026-08-10 project status report and retained the Aug. 9 report as historical
  context. The new snapshot separates accepted RC3/WP-ARCH1 evidence from WP-RC4's implemented but
  unaccepted flat-family closeout, the partially present secure-payload implementation, planned
  fail-closed learner uploads, and later production packages. Its follow-up records the repaired
  persistence size regression and keeps the remaining current-tree package gates distinct from the
  accepted historical WP-ARCH1 evidence.

- Refreshed newcomer and durable documentation without treating internal work-package labels as a
  public product vocabulary. The README now opens with the mastery teaching model: students work
  through varied problems until they can solve them consistently, then continue with fresh versions
  after completion; architecture, setup, usage, related-project, FAQ, input-format, troubleshooting,
  design, and screenshot guidance now align with that current boundary. The Aug. 10 status and active
  plan made the then-open production-composition prerequisite explicit. This is a
  documentation/status refresh, not a release claim.

### Fixes and Maintenance

- Added the independently accepted, fail-closed walkthrough runner: it requires a deterministic
  explicit seed, writes a private redacted report, and refuses unsafe Podman cleanup ownership. No
  browser or gateway PASS is claimed; WP-O2 live smoke configuration and evidence are next.

- Completed the standalone WeBWorK renderer cutover without making the repository a second
  assignment platform. The normal local stack now consumes the external
  `webwork-pg-renderer` image over its private `/render-api`; it carries no WebWork2 course,
  renderer account, MariaDB service, renderer source mount, or second assignment distribution.
  A live Podman teardown and rebuild preserved PostgreSQL and MinIO volumes, the semantic renderer
  probe proved render plus correct and incorrect grading, and the complete gateway/API/browser
  acceptance passed including cache replay, outage isolation, and keyboard operation. Replaced
  brittle launcher and Compose source-text assertions with four fast permanent trust-boundary tests;
  the retained-volume major guard now runs noninteractively, and the opt-in live browser path uses
  the repository's canonical Playwright wrapper so successful operations stay warning-free.
  Persisted attempts and cache hits are now bound to the configured renderer identity; version drift
  refuses before replay recovery, rendering, or grading. The independent security/runtime re-review
  found no unresolved P0/P1/P2, so WP-RC3R is accepted for its bounded RadioButtons scope.
  The exact runtime identities, timings, volume persistence, and full live behavior remain one-time
  or opt-in integration evidence. Renamed the operator runbook to `LOCAL_STACK_OPERATIONS.md` and
  kept service necessity and state ownership in `LOCAL_STACK_ARCHITECTURE.md`. The eleven-stage
  codebase gate and all 3,190 repository pytest cases pass.

- Clarified the three WeBWorK project boundaries and corrected the release direction. `openwebwork/pg`
  is the PG/PGML render-and-grade engine, `vosslab/webwork-pg-renderer` is PLE's required private
  HTTP integration target, and `openwebwork/webwork2` is the full course/homework application used
  only by the accepted RC3 compatibility path. Recorded WP-RC3R's accepted replacement before
  broader question support, removing the duplicate render course, renderer account, MariaDB, and
  second assignment distribution while retaining RC3's source, projection, grading, cache, outage, and
  browser-secrecy proofs. Durable guidance now states that every `OTHER_REPOS/` copy is read-only
  reference evidence and cannot be a build, import, mount, or runtime dependency.

- Corrected the permanent source-size gate for a preserved dirty index: an indexed path deleted from
  the working tree is no longer opened as though source bytes still exist. The gate continues to
  reject file symlinks, directory-symlink traversal, invalid UTF-8, NUL bytes, and files at the
  exclusive 1,000-line boundary. The focused gate passes 824 cases, the full Python/documentation
  suite passes 3,190 tests, and the eleven-stage codebase gate passes.

- Hardened the in-progress WP-RC8 PostgreSQL identity and roster slice without claiming passwordless
  enrollment complete. Roster writers now acquire one course, roster-state, then invitation/member
  lock order; concurrent learner mapping uses an atomic upsert; only one pending invitation per
  course/email or roster ID is allowed; expired invitation retries fail closed; account-owned
  credential foreign keys have supporting indexes; and retention deletion includes roster PII. A
  new opt-in disposable PostgreSQL oracle proves the invitation capability, tenant-context binding,
  duplicate-invitation refusal, and separation of the auth and educational-record roles. Database
  authentication throttling now uses replica-shared fixed windows keyed by server-HMAC digests of
  normalized email and the gateway-overwritten client address, with a class-safe shared-network
  allowance and a uniform outward response after denial. Discoverable passkey lookup resolves only
  an active credential hash and never accepts browser-supplied user identity. Focused Memory,
  route, strict Clippy, clean PostgreSQL 17 migration, and disposable live-role gates pass. Expiry
  cleanup, tenant-administrator command authority, WebAuthn ceremonies, UI, and complete WP-RC8
  acceptance remain open.

- Completed the offline WeBWorK replay-persistence slice without claiming the secure-payload
  cutover accepted. Issuance now converts private durable choice mappings to presentation-scoped
  rendered IDs before prefetch/attempt persistence; Memory and PostgreSQL reads validate the
  source, version, seed, renderer, digest, and owning attempt before returning grading authority.
  Normal grading reproduces the safe cache without a renderer call and makes one private grade RPC;
  successful submission and terminal instructor action delete replay state atomically. File-upload
  and external-tool attempts remain explicitly outside presentation v1. Focused presentation,
  Store-conformance, server, project-tools, source-size, feature-enabled check, and strict Clippy
  gates pass; compact kind-free HTTP, persisted legacy self-heal, live PostgreSQL/private-renderer
  traces, browser recovery, measurements, and independent WP-P1 through WP-P6 acceptance remain.

- Repaired the two post-WP-ARCH1 persistence size regressions by extracting complete attempt-issuance
  capabilities into paired in-memory and PostgreSQL owners. Presentation binding, prefetch promotion,
  timing creation, predecessor linking, and private WeBWorK replay persistence moved together; Store
  signatures and PostgreSQL SQL, bind order, transaction scope, retry, and RLS behavior are unchanged.
  The permanent source-size gate passes 770 cases, and learning-data-access passes all-feature check,
  133 focused behavior tests with the disposable live fixtures compiled and intentionally ignored,
  and strict Clippy.

- Accepted WP-ARCH1 and WP-RC3 after independent review. The permanent source-size gate reports no
  maintained source at 1,000 lines or more and passes 582 tests; `./check_codebase.sh` passes all 11
  stages; the repository Python suite passes 2,451 tests; the browser suite passes 72 tests with two
  deliberate opt-in skips; and the focused server suite passes 189 tests with three live fixtures
  explicitly ignored. The accepted RC3 boundary is the private, source-pinned RadioButtons path;
  broad OPL compatibility and WeBWorK MATCH remain separately owned by WP-RC5. WP-RC4's PLE flat
  JSON v2 close-out and independent review are now the next dependency; external QTI-JSONL is a
  future adapter concern rather than a native-family prerequisite.

- Ran the source-pinned upstream WeBWorK profile through the complete PLE live acceptance on Podman 6. The real gateway path proved authenticated RadioButtons rendering, one renderer call followed
  by same-attempt cache hits, full and zero server-owned grading, idempotent replay, renderer-outage
  isolation, recovery, keyboard-only browser operation, and absence of source, credentials, hidden
  upstream fields, or answer mappings in browser-visible data. The eleven-stage codebase gate passes;
  WP-ARCH1's implementation and all integrated gates now pass, so RC3 final close-out waits only on
  WP-ARCH1's independent acceptance and the final RC3 review.

- Corrected the local-stack operator contract and stale planning navigation found during the
  documentation audit. The launcher now prints the complete WebWork overlay/profile teardown command,
  the environment template routes first-time startup through the guarded launcher, retained
  PostgreSQL volumes are documented as PostgreSQL 17 and non-destructively checked, and concluded
  database/QTI review records point to the current RC3, WP-ARCH1, RC4, RC5, and RC6 package order
  instead of inviting baseline rewrites or superseded work.

- Corrected the disposable PostgreSQL baseline after the source decomposition revealed a stale
  root-level exact test filter. The runner now executes the complete flat-import provenance
  integration-test binary, so a module move cannot silently produce a zero-test pass. A fresh full
  run executed that test and passed migration replay/checksum refusal, partition and summary plans,
  QTI/flat private grading paths, manual grading, role/RLS denial, and exact disposable cleanup.

- Rotated the 2026-08-06 through 2026-08-08 history into `docs/CHANGELOG-2026-08a.md` with the
  maintained changelog tool. The active changelog retains the two newest date blocks as required by
  repository policy.
