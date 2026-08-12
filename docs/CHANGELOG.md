# Changelog

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
  `Dr. Fake Professor`, `Mary Fake Student`, and `Jack Fake Student`, and gives demo course/problem
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

- Made the human-readable Chapter 1 manifest the publication seed's source of truth for course and
  assignment display names, question titles, families, point values, and source paths. Validation
  now rejects flat-payload title drift and unsupported point values before the seed publishes, and
  the launcher/FAQ wording now accurately describes the private renderer used by the normal local
  stack.
- Replaced visible assignment-editor UUID tuples with the catalog's copyable human identity and
  backend label, such as `P-1-v1` with `WeBWorK`. Existing assignments now resolve their immutable
  catalog titles and display identities when loaded, while saved requests and walkthrough
  selectors retain the exact internal references without presenting them as problem numbers.
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
  Course managers can page rosters, enforce exact email domains, send or revoke invitations,
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
  than a stronger credential required for ordinary access. A course manager may revoke and re-invite
  but cannot prove that two accounts belong to the same person strongly enough to transfer
  educational records. Version 1 therefore has no manager account merge or record-transfer path;
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
