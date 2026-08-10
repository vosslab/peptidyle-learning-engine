# Changelog

## 2026-08-10

### Additions and New Features

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

### Fixes and Maintenance

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

## 2026-08-09

### Additions and New Features

- Implemented the static portion of WP-RC3, which is not yet accepted or released. The shipped
  WeBWorK `/render_rpc` adapter now has hardened form/RPC handling, exact RadioButtons projection,
  deterministic 0--100 scoring, response-leak protections, and cache evidence. An optional,
  private arm64 WebWork plus MariaDB stack now has launcher-managed file secrets and pinned-source
  provenance hardening. The package includes an immutable authored PGML seed and the required live
  API, browser, and keyboard acceptance harnesses. Live upstream image build and end-to-end
  acceptance remain pending because the local Podman storage must first have sufficient free disk;
  this entry does not claim a released integration.

- Completed WP-RC2 production-seam closure. The H5P import, QTI parser, and WeBWorK renderer
  contract now use concrete production module names; the empty native renderer declaration is gone;
  `CatalogStore` requires explicit catalog resolve/search behavior; and current feedback projects
  the durable release fact through one server policy function. The human-reviewed closure scan found
  no empty or `stub`-named maintained production file, placeholder return, `todo!`,
  `unimplemented!`, or hidden catalog-capability default. Focused Rust suites, `cargo fmt --check`,
  strict workspace Clippy, workspace tests, all 11 `./check_codebase.sh` stages, 1,733 repository
  Python tests, and both diff checks passed. Independent review reported no P0/P1. Exact ownership,
  decisions, and evidence are in
  `docs/active_plans/workstreams/production_seam_closure.md`; WP-RC3 is next.

- Completed WP-CA6, WP-CA7, and WP-RC1 course appearance. Instructors now have a production-backed,
  keyboard-complete settings route for 15 measured themes, exact wide/narrow banner previews,
  decorative or informative alternative text, one revisioned save, explicit conflict reload, and
  clear replacement/removal recovery. The learner course entry renders the course title as text and
  at most one authorized 1200 by 328 banner; assignment, run, summary, editor, gradebook, settings,
  and global pages never repeat it. A bounded request-triggered cleanup now executes persisted
  claim, tenant-owned object deletion, and completion without making cleanup availability block a
  read. The database rejects current pointers whose delivery kind, tenant, or course does not match.
  A disposable oracle passed real-role PostgreSQL/RLS/CAS, MinIO conformance, combined
  PostgreSQL-to-MinIO idempotent cleanup, current-object preservation, upload, promotion, current
  delivery, and supersession. All seven course surfaces passed built-browser theme traversal; the
  rendered 15-theme contact sheet, 320/480/768/1920/forced-color screenshots, contrast metrics, and
  OKLab differentiation were regenerated and recorded with exact hashes. All 11 repository checks,
  62 rebuilt Playwright cases, the opt-in visual case, 1,743 Python tests, and both diff checks
  passed. Independent HCI/color, persistence/security, and plan/route reviewers reported no
  P0/P1/P2. The complete evidence is in
  `docs/active_plans/workstreams/course_appearance_implementation.md`; WP-RC1 remains accepted.

- Replaced every known implementation-plan deferral with a binary version 1 decision and a
  dispatchable release package. The new release-completion plan owns WP-RC1 through WP-RC12 with
  exact owners, files, behavior, success conditions, validation, migration reservations, and
  working-codebase versus production-activation boundaries. Companion plans now assign QTI Package
  Maker WP-FQ-0 instead of waiting for a QTI-JSONL contract, fix 60-minute course-banner candidate
  and delivery ceilings, require background Canvas/Blackboard export, select institutional OIDC and
  OpenTofu, keep client analytics/passkeys/password/email-code login out of version 1, integrate
  upstream WeBWorK `/render_rpc` as shipped, and give object reconciliation, eight families, Chapter
  1 content, LTI, deployment, bot-cost, and release acceptance complete packages. Status, README,
  flat-format, database, human-guidance, and contract docs now use the same decisions and describe
  executable reference backends rather than production stubs. All 11 repository checks plus focused
  Prettier, Markdown-link, ASCII, plan-closure, and staged/unstaged whitespace gates pass.

- Completed WP-CA5 course-scoped Solid theming and changed the pre-data default to Grass at the
  owner's direction. The Grass anchors are Roosevelt-inspired `#BDDEB1`, `#73C167`, and `#008852`;
  the pale green is explicitly logo-derived rather than claimed as an official brand-guide swatch,
  and raw `#008852` stays decorative while measured action/link projections meet the house contrast
  target. One route-keyed provider now themes course entry, assignment, run, summary, editor,
  gradebook, and appearance routes below the unchanged global shell. Course routes load summary and
  appearance together, attempts reuse `RunScreenData.course`, and summaries receive a safe course
  projection derived from the authorized stored assignment. Unknown theme IDs fail closed, and
  cross-course/global navigation cannot retain the prior variables. Focused Node and Rust checks,
  rendered contrast measurements for all 15 themes, the complete 56-case built Playwright suite,
  and all 11 repository checks passed. WP-CA6/WP-CA7 subsequently completed through WP-RC1.

- Completed a focused no-mouse accessibility pass across every student browser surface. A built
  keyboard-only scenario now uses the skip link, Tab, Enter, native radio arrows, feedback, and the
  single completed-run exit from course selection through submission and back to the assignment.
  Multiple-answer controls add arrow-key focus without changing selection, ordered responses add
  Up/Down Arrow movement with focus preservation and a polite position announcement, and visible
  buttons remain available as the standard fallback. Read-only run summaries no longer add one
  no-op button per response, and every repeated feedback region now owns a unique heading ID. The
  new `docs/ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md` records the task model, WCAG/APG ledger,
  heuristic delta, limitations, and future screen-reader participant gate. Eight response-controller
  tests, all 20 focused built-browser scenarios, all 11 repository checks, and all 1,699
  repository-owned Python tests passed.

- Added `launch_local_stack.sh` and the `npm run launch` alias as the root local-test front door. It
  now bootstraps its ignored local env and separate instructor/student credentials, selects a free
  gateway port on first use, starts backing services before applying and verifying migrations,
  provisions the restricted grader login, seeds one reusable demonstration course, and only then
  starts API, worker, and gateway. The session-recovery screen exchanges a generated local token for
  the existing HttpOnly session without retaining it in browser storage. Caddy serves the host
  `dist/` bundle from a read-only mount while proxying only API-shaped paths, and normal shutdown
  retains database/object volumes. A live macOS Podman run reproduced and repaired the original
  incomplete-Compose failure, handled an unrelated port-3000 owner by selecting port 3001, and
  reached semantic gateway health. The incompatible legacy WeBWorK image is not mislabeled as
  supported: its private profile remains off unless `--with-webwork` receives a reviewed image that
  implements PLE's `/v1/render` and `/v1/grade` contract. The exact full launcher path rebuilt Rust,
  Wasm, generated types, fixtures, and Solid, then reached ready health; generated-instructor login,
  HttpOnly session reuse, seeded-course visibility, read-only preflight, and fail-closed renderer
  opt-in passed against the live stack. The final `--skip-build` run also completed the actual macOS
  browser-opening branch. All 11 repository checks and all 1,699 repository-owned Python tests
  passed.

- Added `docs/DATABASE_STRUCTURE.md`, an evidence-backed map of all implemented database boundaries
  from question revision through assignment delivery and isolated current scoring. It distinguishes
  the existing session table from proposed production principal, institutional OIDC, passkey,
  optional password, and recovery tables; explains why an email code is not two-factor
  authentication; estimates the fall four-course pilot at 5,000-675,000 question attempts; and
  carries the same formulas to 10,000 students and ten million catalog questions. PostgreSQL 17,
  NIST, W3C WebAuthn, FIDO, OAuth/OIDC, PgBouncer, and U.S. Department of Education sources support
  the RLS, partitioning, password, passkey, pooling, FERPA, and recovery guidance. The architecture
  and file maps now route contributors to this document. ASCII, whitespace, Prettier, the 11-stage
  repository gate, and all 1,699 repository-owned Python tests passed with the new artifacts present
  in the tracked-file validation set.

- Completed WP-CA4 atomic course-appearance HTTP and image behavior. The production server now
  exposes no-store current appearance GET, author-only candidate upload, strong-ETag atomic PUT, and
  exact-current banner delivery through the existing same-origin asset route. A server-only,
  feature-limited image boundary accepts bounded JPEG, PNG, or WebP; rejects SVG, animation,
  malformed, oversized, over-pixel, and undersized inputs; applies embedded orientation; strips
  metadata; performs a centered cover crop without upscaling; and emits exactly one 1200 by 328
  lossless WebP. Replacement uses a new narrow Store capability to resolve the hidden future banner
  identity only after session/course/creator/expiry revalidation, verifies candidate bytes, copies
  them first, and then applies revision CAS. Focused tests prove student and outsider refusal,
  strict JSON and media classes, stale preservation, injected object-copy failure, and immediate
  superseded-delivery refusal. All 190 server tests, strict Clippy, the complete seven-migration
  PostgreSQL/RLS/regression gate, and all 11 repository checks passed. WP-CA5 course-scoped Solid
  theming subsequently completed.

- Completed WP-CA3 revisioned course-appearance persistence. The backend-neutral contract now has
  matching Memory and PostgreSQL owners for default creation, persisted manager authorization,
  exact revision CAS, candidate checksum/expiry state, bytes-first idempotent promotion,
  exact-current banner delivery, and bounded two-phase cleanup with a final pointer recheck. The
  first forward migration after the accepted six-file baseline adds tenant-leading appearance and
  candidate tables with forced RLS, a same-transaction course-insert trigger, and a scoped
  security-definer actor resolver without granting `ple_app` direct session-table access. The shared
  conformance scenario and disposable PostgreSQL 17 oracle prove student/outsider refusal,
  membership revocation, stale copied-object ownership, current/superseded delivery, and cleanup
  that preserves current bytes. All 70 data-access unit tests, 40 Memory conformance tests, strict
  PostgreSQL-feature Clippy, the complete seven-migration database/RLS/regression gate, all 11
  repository checks, 1,654 Python tests, and 629 focused documentation checks passed. WP-CA4 atomic
  HTTP/image behavior is next.

- Completed WP-CA2 protected course-banner objects. `objects` now owns tenant/course-bound candidate
  and immutable banner keys with domain-separated derived physical IDs and no caller-supplied path.
  Candidates use temporary storage and refuse signed URLs; current banners are protected
  `CourseContent` and may be signed only at the typed-object layer, leaving exact-current-pointer
  authorization to WP-CA3 before delivery. The shared memory/MinIO-compatible conformance suite now
  covers both paths and preserves existing source refusals and student-record behavior. Focused
  object tests, strict Clippy, S3-feature compilation, the 618-case documentation gate, the 11-stage
  repository gate, and 1,644 Python tests passed. WP-CA3 subsequently completed revisioned
  Memory/PostgreSQL persistence, forced RLS, candidate lifecycle, and current-pointer authorization.

- Completed WP-CA1, the frozen course-appearance contract. `question_model` now owns the closed 15-
  theme ID set with `grass` as its only pre-data default after the owner correction in WP-CA5, exact
  decimal-string appearance revisions,
  browser-safe current/candidate banner route identities, validated decorative or informative alt
  state, a safe current projection, and one strict theme plus keep/remove/replace mutation. The
  existing Rust-to-TypeScript generator now supports Serde `kebab-case`, so `coral-reef`,
  `salt-marsh`, and `sea-floor` are derived rather than copied into a competing union. The
  instructor appearance route is present in the route contract, executable router, honest contract
  surface, route documentation, and source/built-browser tests. The owner image policy now uses one
  exact server-normalized 1200 by 328 pixel WebP, based on the centered YouTube-safe-area
  proportions; browser surfaces only scale that same derivative down. No object key, checksum, source,
  answer, grading value, PostgreSQL schema, storage behavior, HTTP mutation, or Wasm export was
  added. The 11-stage repository gate, 1,644 Python tests, focused 618-case documentation gate, and
  8-case rebuilt Playwright route suite passed. This established the dependency for WP-CA2 protected
  banner object identities and classification.

- Added the QTI-JSONL flat-question integration plan for MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and
  HOTSPOT. At the owner's direction, it replaces the provisional custom version 2 schema with an
  adaptability boundary: the forthcoming QTI Package Maker QTI-JSONL specification supplies the
  lossless source semantics, using `exam_yaml` as a readability guide, and one versioned PLE adapter
  compiles accepted records into answer-free public and grader-only private values. The plan
  preserves exact v1 `singleChoice`, defers media and HOTSPOT source fields to normative owner
  fixtures, keeps MATCH first, and records eight dependency-ordered integration packages. Course
  appearance remains the next implementation package while the owner specification is pending.

- Completed WP-QTI-12 independent review and documentation close-out. Six separate plan, test,
  style, documentation, legacy, and comment passes found no production or test defect. Documentation
  review initially found stale README status and missing profile-to-native ownership evidence in the
  contract register, architecture, and file map. The README now reports the accepted upload,
  answer-free report, conversion, edit/publication, and live PostgreSQL path; the owner documents now
  route contributors through the profile parser, upload route, worker, conversion bridge, author UI,
  protected grader, and disposable PostgreSQL/RLS oracle. Focused Markdown link, ASCII,
  first-paragraph, whitespace, and formatting gates passed, and the original reviewers found no
  remaining P0/P1 issue. Future family source now waits for the owner's QTI-JSONL specification;
  course appearance is next, and MATCH remains first when flat-family implementation begins.

- Completed WP-QTI-11 live PostgreSQL/RLS/profile-to-native acceptance. A fresh isolated PostgreSQL
  17 database applied and verified the six-file SQLx baseline, then exercised the real upload route
  and profile worker with one minimized Canvas archive containing an accepted static-single-choice
  item and a rejected sibling. The accepted item converted through the native flat bridge, remained
  editable, published with immutable archive/provenance checksums, and graded correct and incorrect
  responses through the isolated PostgreSQL grader. Application, student, grader, and foreign-tenant
  probes enforced RLS and protected-capability boundaries; workspace cleanup removed current private
  state while published provenance remained. The full gate also replaced direct Unicode source
  characters with behavior-preserving Rust/PostgreSQL escapes and corrected a feedback Playwright
  fixture that had submitted an external-tool marker to native attempts. The disposable database
  gate, strict workspace checks, all 11 repository stages, 51 built Playwright scenarios, and 1,644
  Python tests passed. Evidence is
  in `docs/active_plans/workstreams/qti_live_acceptance_implementation.md`.

- Completed WP-QTI-10 author UI over the accepted server DTOs. The existing workspace route now
  composes a feature-local, same-origin QTI import panel with the existing flat editor; it adds no
  product route, global browser contract, browser ZIP/XML parser, archive/report persistence, or
  private-answer fields. Authors can upload opaque ZIP bytes, manually refresh queued/processing
  work, review recognized profile/default/warning details and visually distinct accepted/rejected
  items, acknowledge the current report, and convert one accepted item only with the exact displayed
  clean strong draft revision. Exact retry preserves identity only after an ambiguous upload;
  all-rejected and unsupported packages retain clear recovery. After a committed conversion, the
  previous editor stays inert until the replacement refetch succeeds; a failed refetch keeps it
  locked and provides a repeatable reload action without repeating conversion or creating a new
  import. The existing flat editor receives focus only after successful load. Permanent Node tests,
  four real-route Chromium scenarios, and the 11-check full gate passed (173 Node and 184 server
  tests in that gate); independent security and HCI re-reviews found no P0/P1 issue. WP-QTI-11,
  the live PostgreSQL/RLS/profile-to-native gate, is next and unstarted. Evidence is in
  `docs/active_plans/workstreams/qti_author_ui_implementation.md`.

- Completed WP-QTI-9 server ingress, report, conversion, and publication orchestration. An author
  upload retains an exact bounded ZIP in a deterministic private workspace object and enqueues one
  deterministic `qtiImport` job; exact replay is stable and divergent replay refuses. The strict
  profile worker exposes an answer-free report with package/item defaults and acknowledgement
  digests, stages complete accepted-item evidence, and keeps mixed-vendor or all-rejected results
  from creating a conversion candidate. Strong-ETag conversion rereads and reparses the retained
  archive before the atomic WP-QTI-8 Store command, while publication copies the exact archive to a
  deterministic non-signable `PublishedImportArchive`. Memory and PostgreSQL now serialize prepared
  import work with draft deletion. The adapter (93 unit, 6 conformance, 12 documentation), objects
  (17 unit, 3 conformance, 1 published-archive), learning-data-access (93 unit, 39 conformance,
  3 documentation; one database-only unit and 7 live PostgreSQL integration tests ignored as
  documented), and server (184 library,
  1 main, 1 documentation) gates passed, as did strict Clippy, workspace all-target/all-feature
  check, formatting, and 5 crate-boundary checks. A one-time 32 MiB plus one-byte chunked ingress
  probe passed and was removed. Independent P0/P1 route and worker reviews reported PASS. WP-QTI-10
  author UI is next; the WP-QTI-11 live PostgreSQL/RLS/profile-to-native gate remains pending.

- Added an execution-ready bot-traffic cost-containment plan for M6. It separates a tiny, API-free
  public landing host from the authenticated PLE application; keeps app APIs same-origin with
  host-only cookies; requires cheap authorization refusal before Store, object, queue, renderer, or
  provider work; and adds semantic cache separation with tunable TTLs, provider-neutral credential
  refusal, private origins and health, bounded secret rotation, reference-safe manifest cleanup,
  evidence-derived rate and capacity settings, a normalized bot-cost formula, authenticated
  automation concurrency and idempotency bounds, spend controls, and an operator runbook. Production
  is fixed to private S3 origins behind CloudFront. GitHub Pages is limited to a qualifying
  non-commercial project showcase
  because its current terms prohibit using free Pages hosting to operate an online business or
  commercial SaaS. The plan exposes the remaining M6 deployment-tool and production
  `IdentityProvider` decisions and now applies the repository's permanent-test checklist.
  Deterministic offline behavior stays in unit/conformance or local Playwright tests, while cloud,
  network, cost, transfer, visual, mutation, and capacity probes
  are one-time evidence rather than regular tests. Unsupported byte, pixel, timing, percentage,
  retention-count, and rate constants were removed from permanent gates. Independent red-team review
  also removed assumed traffic evidence, noisy negative-cost failure, diagnostic alert boilerplate,
  unsafe secret-materialization claims, and external navigation from the regular Playwright path.
  Durable human guidance now records that discipline explicitly.

- Added a formal project status report and refreshed the README status surfaces. They now separate
  implemented and verified capabilities from accepted plans, identify WP-QTI-9 as the next active
  package, record the seven flat-question families still missing beyond single choice, and make the
  deployment, course-appearance, content-pack, object-reconciliation, and release blockers explicit.
  The complete 11-stage repository check passed after generated Cargo artifacts were cleared.

- Completed a focused frontend/backend security pass. Learner response recovery now uses only
  `sessionStorage` and clears the active buffer on run exit; file-upload answers fail closed until
  a server-issued upload capability exists. Generic QTI rejects active SVG, and PDF export bounds
  PNG dimensions, decoded bytes, and zlib expansion before allocation. Catalog revision and
  lifecycle writes now require tenant-qualified ownership in Memory, SQLx, and forced RLS, while
  API readiness reruns the exact migration/checksum compatibility verifier rather than treating a
  reachable database as compatible. Static SQL review found only bound SQLx values, the npm audit
  reported no production dependency advisories, focused security tests passed, and the fresh
  six-migration PostgreSQL baseline passed every real-role and denial oracle.

- Completed WP-QTI-8 Memory/PostgreSQL provenance-aware conversion. A closed staged profile-
  evidence type closes H2 and binds conversion to the committed accepted result's exact
  `sourceIdentifier`/`itemId`, profile tuple, and digest set. Both backends atomically commit the
  draft CAS revision, canonical source, current private grading, and current origin under the frozen
  lock order. Ordinary saves also stage current grading; publication accepts no caller grading
  payload and promotes only the locked stored value after origin promotion. PostgreSQL uses the
  forced-RLS provenance and grading brokers with no direct Store reads of private grading,
  choice-map, or provenance secret tables. `Sha256Digest` now uses strict lowercase 64-hex JSON
  serialization. Shared conformance, PostgreSQL feature coverage, the full fresh baseline, and
  independent review passed with no P0/P1 finding. WP-QTI-9 server routes are next; the already-
  frozen course appearance plan remains separate.

- Refreshed durable human guidance in the owner's wording: use GPT-5.6 agents,
  favor positive prompting and parallel atomic tasks, support the eight named
  PLE QTI-JSON question families, keep feedback optional, use QTI Package Maker
  as a low-priority conversion reference, and begin with four mixed WeBWorK and
  static questions in each Chapter 1 genetics and biochemistry assignment.

- Recorded the owner's Blackboard-inspired course appearance guidance and added an execution-ready
  M3 plan. The plan keeps 15 measured three-color biome/habitat themes, consolidates only `woodland`
  into `forest`, applies one theme across every course-owned route, and keeps one small centered
  banner on the course entry page. Atomic revision handling, protected object lifecycle, forced RLS,
  fixed server-owned image normalization, accessibility/contrast metrics, visual contact sheets,
  and independent implementation review are required before the feature is complete. Independent
  architecture and HCI reviews of the plan itself reported PASS with no remaining P0/P1 finding;
  the separate QTI sequence remains dependency ordered.

- Refreshed WP-QTI-7 schema/RLS/object-binding implementation evidence after the choice-map checksum
  repair. A dedicated `NOLOGIN`, `NOINHERIT`, `NOBYPASSRLS` provenance broker owns narrow protected
  functions over six forced-RLS current/published origin, private choice-map, and profile/item-
  evidence relations. PostgreSQL recomputes SHA-256 over private choice-map bytes in direct table
  triggers for current and published maps, so divergent digests are refused even for a direct
  provenance-broker write. Current lineage pins the committed import; ordinary draft cleanup
  releases only that current lineage; published lineage and maps remain immutable and retained.
  Origin writes bind the full committed typed `ObjectRecord`, including key classification,
  checksum, size, media type, license, provenance, and creation time. SQL now matches Rust's
  1,024-Unicode-scalar source-item boundary across every linked QTI surface. The final fresh
  baseline pass applied all six migrations, re-applied without change, verified the ledger, and
  exercised capability-negative and direct-broker negative provenance probes with the real-role
  RLS/pin/cleanup oracle. Final independent checksum re-review reported PASS with no P0/P1
  findings; the subsequently completed WP-QTI-8 backend conversion is recorded above.

- Completed Q4/WP-QTI-6 provenance contract and archive-object identity. The adapter now emits a
  versioned private ordered choice-map payload with a fixed binary encoding and checksum; the
  storage boundary owns closed profile/conversion values, current and immutable published origins,
  private payload redaction, fail-closed promotion, and one atomic conversion command. The
  `PublishedImportArchive` key is tenant-bound, deterministic, non-signable, and retained with the
  published version. Focused adapter, data-access, object, bridge, formatting, Clippy, boundary,
  and diff gates passed; independent review reported PASS with no P0/P1 finding. The subsequent
  WP-QTI-7 schema package reconciles the Rust and SQL source-item boundary at 1,024 Unicode scalars;
  WP-QTI-8 backend conversion remains next.

- Added the pure server-only QTI-to-flat bridge. It routes a private mapped
  Canvas or Blackboard v1 item through native validation, canonicalization, and
  existing public/private compilation with fixed imported defaults and a 256
  KiB whole-source cap. Canvas and Blackboard fixtures match equivalent
  hand-authored canonical source exactly; no Store, object, schema, HTTP, UI,
  or Wasm behavior changed. Full focused gates and independent P0/P1 review
  passed.

- Added the bounded Blackboard Original QTI 2.1 static-pool parser. It accepts
  only static single-choice items, keeps the exact inert `SCORE` declaration as
  compatibility provenance rather than scoring, and records the explicit 1.0
  point default as a review-required warning. Real shuffle, scoring variants,
  feedback, media, rich markup, and extensions become per-item safe refusals.
  Correct bindings and vendor-to-PLE maps remain private; full adapter gates,
  strict Clippy, boundary checks, and independent P0/P1 review passed.

- Added the bounded Canvas QTI 1.2 static-single-choice parser. It derives
  profile evidence from the exact bounded archive, manifest/resource graph,
  assessment metadata, and item tree; accepts only declared points and exact
  all-or-nothing scoring; and keeps unsupported feedback, scoring, markup, and
  extensions as per-item safe refusals. Correct bindings and vendor-to-PLE maps
  remain private and the package is neither serializable nor debuggable. Full
  adapter gates, strict Clippy, boundary checks, and independent P0/P1 review
  passed.

- Added the strict shared QTI markup projector. Canvas `mattext` receives one
  bounded HTML-tokenizer layer, while Blackboard ordered XML is projected
  directly and is never reparsed as HTML. Only the documented small text
  allowlist reaches deterministic CommonMark; attributes, recovery-dependent
  HTML, links, rich media, tables, styles, SVG, MathML, and unknown markup
  refuse visibly. Input, token, nesting, and Unicode-output limits are enforced
  before growth. Full adapter gates and independent re-review passed.

- Added the shared QTI mapped-item contract required before either vendor
  parser. Closed instructor-safe diagnostics cannot carry parser-supplied
  answer text; correct choices and ordered vendor-to-PLE maps remain
  non-serializable and non-debuggable. Choice identifiers are deterministic,
  collision-safe, and source-order independent. Mapping digests can now be
  created only by their profile/version-owning mapped item. Full adapter tests,
  strict Clippy, boundary checks, and independent re-review passed.

- Added the ordered XML evidence needed by exact Canvas and Blackboard
  parsers without changing the generic QTI importer. The private XML tree now
  preserves mixed text/child/comment/processing-instruction order, raw CDATA,
  element and attribute prefixes, and inherited namespace bindings. The
  existing aggregate-text behavior and public generic output remain locked by
  regression tests. Full adapter tests, strict Clippy, boundary checks, and an
  independent P0/P1 review passed.

- Completed the first bounded QTI-profile contract package. The adapter now
  has exact persisted Canvas QTI 1.2, Blackboard QTI 2.1, and honest generic
  profile identities; a single corpus-grounded manifest/resource/item matrix;
  strict vendor detection with generic fallback; and canonical safe-report,
  public, private, combined, and warning digests. Private answer mappings
  remain non-serializable and non-debuggable. Focused tests, strict Clippy,
  crate-boundary checks, and independent re-review passed; vendor item parsers
  remain the next package.

- Added the parser-ready QTI profile fixture corpus. Readable minimized Canvas
  QTI 1.2 and Blackboard QTI 2.1 manifests, metadata, items, and one-fact near
  misses now reproduce the retained package structures without test-time
  `OTHER_REPOS` access. Shared test support builds safe logical ZIPs, validates
  balanced single-root XML, and compares member paths and contents rather than
  timestamps or ZIP bytes. Focused/full adapter tests, strict Clippy, boundary
  checks, and independent review passed.

- Extracted the QTI hostile-input foundation before adding vendor parsers. A
  focused archive owner now enforces ZIP, path, link, duplicate, and expansion
  bounds; a separate XML owner enforces UTF-8, DTD/entity refusal, resource
  limits, attributes, nesting, and a single root. The generic parser keeps its
  original narrow entry grammar and fell from 976 to 613 lines. Adapter tests,
  strict Clippy, boundary checks, and independent review passed.

- Added the implementation-ready bounded QTI profile plan. It defines separate
  Canvas QTI 1.2 and Blackboard QTI 2.1 static-single-choice profiles, strict
  refusal instead of lossy grading or markup conversion, deterministic choice
  identity, native flat-question compilation, immutable private archive
  provenance, instructor upload/review/convert behavior, real-role PostgreSQL
  acceptance, and a separately gated later export milestone.

- Completed the instructor PLE flat-question editor. The focused authoring
  modules load and save canonical source only through the authenticated
  author-role, `no-store`, strong-ETag source route; ordinary browser
  contracts, learner preview, Wasm, and public publication DTOs remain
  answer-free. The route falls back to the legacy editor only on the protected
  source route's 404, while other failures remain visible. The editor supports
  create/open/edit/save, local-answer-free preview, visible stale-conflict
  reload, publication review, immutable publish, generation guards, a
  double-save lock, accessible per-choice radios, and 375 px reflow. Focused
  fixtures passed 2 new editor Playwright tests plus 7 generic editor tests;
  the production rebuild and all 11 `check_codebase.sh` stages passed. This
  records mounted component/client/repository evidence, not a deployed
  authentication or browser walkthrough.

- Completed the PLE flat-question persistence, publication, and runtime
  package. A typed compare-and-swap save keeps the draft and private canonical
  workspace source together; publication copies that source to an immutable,
  non-signable `ProblemSource`, persists only the answer-free public model, and
  promotes typed private material through the isolated grader capability. The
  dedicated no-store ETag routes and native runtime keep grading server-only.
  Memory, PostgreSQL/RLS, a real compiled blue-correct/red-incorrect live gate,
  and an independent re-review pass cover the boundary.
- Added the first executable PLE flat-question JSON v1 contract for ordinary
  static single-choice questions. The strict native Rust codec bounds source
  size and text, rejects duplicate and unknown members at every object layer,
  canonicalizes stable semantic choice IDs, and compiles author input into an
  answer-free draft plus checksum-bound grader-only answers and feedback.
- Completed the maintained local whole-system runner. Its three gates call the
  browser-safe Wasm bridge, apply and audit the full six-migration PostgreSQL
  baseline with every live oracle, and exercise a learner through two API
  replicas behind the gateway. The runner stops the issuing replica, requires
  the exact question envelope from the survivor, commits and exactly
  replays one idempotent submission, verifies the scoped persistence rows, and
  removes every generated project resource.
- Added an allowlisted container build context and a derived gateway image.
  API builds receive only Cargo sources, owning Containerfiles, and the SQLx
  migrations embedded by `migrate!`; host targets, generated output, local
  credentials, and unrelated source never enter the context. The gateway starts
  from an explicitly pinned official Caddy digest, removes its unnecessary
  low-port file capability, and runs on port 8080 as UID 1000 with an empty
  runtime capability set.
- Recorded a clean-cluster PostgreSQL 17 backup/restore rehearsal. Encrypted
  role and custom-format database artifacts recreated the six-migration ledger,
  data fingerprint, role attributes without password hashes, function owners,
  grants, forced RLS, and tenant isolation in a separate empty cluster. The
  restored application role persisted a tenant-owned write and invoked a
  restored queue-broker function. Backup and restore each completed in one
  second in this local fixture; managed PITR, production key management, and a
  deployed recovery objective remain separate M6 gates.
- Recorded a one-time production-boundary retention rehearsal against isolated
  PostgreSQL and MinIO services. The real six-family worker deleted a populated
  learner record graph and its typed student-record object while preserving the
  assignment and instructor structure, published catalog/version/source,
  workspace draft, and anonymous global statistics. The reconstruction harness
  was removed after the evidence was recorded, as required by the retention
  verification policy.
- Added a disposable PostgreSQL scale oracle for all four monthly activity
  parents and the production gradebook read shape. It distributes 260,000
  attempts across the fixed 26-month epoch, inspects `EXPLAIN (ANALYZE,
BUFFERS, FORMAT JSON)`, requires each bounded query to scan exactly one
  requested child, and proves an application-role 51-row gradebook lookahead
  page touches only compact current-summary relations with indexed enrollment
  and summary access.
- Activated the production worker as an explicit `--worker` binary mode and
  dedicated Compose service. Its closed registry contains six complete
  handler/committer pairs for scoring, course item analysis, attempt
  auto-submit, retention, assignment export, and QTI import; schema
  compatibility is verified before the first claim, and the process receives
  only PostgreSQL and object-storage configuration.
- Added mandatory family filters to the durable queue broker. Claimable work,
  expired-lease cleanup, and queue depth all use the same nonempty closed
  filter derived from the worker registry. Reserved Render and generic Import
  variants therefore stay untouched until complete implementations exist.
- Added complete per-item QTI import results. Safe packages may now retain
  accepted items while reporting unsupported or missing items as rejected;
  every result carries bounded source identity, disposition, warnings, and an
  answer-free normalized checksum when accepted. Exact and likely duplicates
  within one import batch are reported explicitly, and original archive bytes
  remain available for correction or re-import.
- Added forced-RLS PostgreSQL persistence for QTI item results plus a permanent
  disposable-database oracle. The live path proves preparation invisibility,
  exact commit, provenance, accepted/rejected row shape, warning detail,
  foreign-tenant non-enumeration, and absence of private grading bytes.
- Added a separate, current-only course item-analysis projection instead of
  reusing the identity-free cross-course catalog aggregate. The pure domain
  reducer, Store contract, Memory and PostgreSQL backends, instructor-only API,
  and closed worker handler report graded, unanswered, and pending-manual
  counts; correctness-based difficulty; exact-credit mean and sample standard
  deviation; discrimination; bounded response categories; assignment score;
  and terminal learner-submission time without learner identity, raw response,
  answer key, or grading implementation.
- Added generation-fenced item-analysis staging and publication. A successful
  scoring commit transactionally reserves the corresponding analysis job,
  non-analysis work remains ahead of analysis in both queue backends, and stale
  prepared generations complete as superseded without delaying or rolling back
  the current grade. The complete production registry now drains this family
  while reserved incomplete families remain outside its broker filter.

### Fixes and Maintenance

- Aligned durable delegation guidance in implementation status: Spark handles simple,
  bounded independent work per [CODEX_SPARK_SUBAGENTS.md](CODEX_SPARK_SUBAGENTS.md), while the
  manager retains architecture, coordination on cross-cutting decisions, and final integration
  responsibility.
- Refused direct signed URLs for both workspace and published source objects.
  Source bytes may contain answers or executable grading material even when
  they live in the content bucket; only published assets, render artifacts, and
  authorized student records remain signable through the typed object contract.
- Repaired the live PostgreSQL application boundary found by the replica gate.
  `ple_app` now has only the `SELECT, INSERT` rights its catalog-asset binding
  contract requires. Immutable submission-successor receipts no longer use
  `SELECT FOR UPDATE`, which would have required a misleading table-wide
  `UPDATE` grant; their primary key serializes concurrent insertions and a
  losing finalizer accepts only the exact stored successor.
- Hardened the replica runner around real Podman variation and failure cleanup.
  It falls back from a stale `podman compose` shim to standalone
  `podman-compose`, identifies API containers through exact project/service
  names, emits only bounded allowlisted HTTP errors and redacted deployment
  diagnostics, validates UUIDs before SQL interpolation, and removes exact
  residual containers, networks, and volumes after partial Compose failures.
- Paired every claimable worker handler with its own atomic committer in one
  registry entry, replacing the unsafe combination of a full handler table and
  one unrelated global committer. Production processes one job per bounded
  pass, observes shutdown between claims without dropping active preparation,
  redacts operational error detail, and reports supported-family depth.
- Declared Compose's shared `default` network explicitly. The maintained
  `podman-compose config` check previously rejected the API's existing named
  default attachment as a missing network; PostgreSQL, MinIO, API, and worker
  now share a portable explicit application network.
- Split the QTI adapter into a 293-line data model, 976-line bounded parser,
  and 340-line hostile-archive/import-report test owner. The public adapter
  facade remains stable while archive limits, XML limits, asset validation,
  partial results, and duplicate detection have distinct review surfaces.
- Granted the QTI staging broker only the tenant-context predicate required by
  its security-definer functions. The live import path found the missing
  capability before any production data exists.
- Aligned durable export and import queue payload checks with Rust's established
  snake-case enum-field serialization. The PostgreSQL constraint and atomic
  export/QTI commit predicates now accept the exact closed payload Rust writes;
  a permanent serialization test locks the QTI shape.
- Split the QTI runtime backend into a 394-line production owner, 332-line
  shared private-fixture owner, 263-line direct grading/asset-integrity test
  owner, and 431-line learner run-lifecycle test owner. Issue, reproduction,
  answer-bearing grading, tenant/provenance refusal, asset binding, and replay
  behavior are unchanged; contributors can review each boundary independently.
- Split the QTI publication route's six behavior tests into a focused private
  test owner. The 585-line production module still owns committed-staging
  validation, strong workspace revisions, exact source-byte copying, review
  authorization, and visible-version promotion; the 532-line test module
  preserves every route and race assertion without changing a public path.
- Split authentication sessions and QTI persistence into capability-owned
  contract, in-memory, PostgreSQL, and conformance modules. The stable
  `learning_data_access::Session...` and `learning_data_access::Qti...` paths are unchanged; session SQL
  still assumes only `ple_auth`, opaque credentials remain one-way hashes,
  backend clocks still own expiry, and answer-bearing QTI bytes remain behind
  the separately injected grader handle. Store `lib.rs` drops to 2,319 lines,
  the PostgreSQL parent to 6,630, and the conformance facade to 4,990.
- Added `cargo tools` as the plain-language front door for repository-only
  generation, fixtures, database operations, and E2E seeding while retaining
  `cargo tools` as a compatibility alias. Scripts, generated-file guidance,
  command help, and current plans now lead with the clearer command.
- Split protected asset delivery into a 173-line backend-neutral contract,
  148-line in-memory implementation, and 235-line PostgreSQL implementation,
  paired with its existing focused conformance owner. Existing
  `learning_data_access::Asset...` paths, SQL, tenant checks, immutable catalog binding,
  educational-record non-enumeration, and access-audit behavior are unchanged;
  the crate root drops to 2,520 lines and the PostgreSQL parent to 6,744.
- Moved the complete backend-neutral external-tool Store contract out of the
  crate root into a focused 441-line module. The root remains a compatibility
  facade, so existing `learning_data_access::ExternalTool...` paths and both backend behaviors
  are unchanged while `store/lib.rs` drops from 3,075 to 2,677 lines.
- Kept item-analysis authorization inside the persistence boundary. PostgreSQL
  uses the active stored session and course authority in one database snapshot;
  forced RLS, course-binding and retention-fence triggers, purge order, and
  least-privilege grants protect both current and private staging rows.
- Defined completion time as the interval from run start to the latest terminal
  learner submission in both Store backends. Later manual-grading time cannot
  inflate the learner's completion interval. Difficulty follows the evaluator's
  correctness verdict independently from exact manual credit.
- Removed an unnecessary staging row lock found by the live least-privilege
  oracle. The assignment row lock and exact leased-job validation already
  serialize publication, so the application role does not need an `UPDATE`
  grant on private staging rows.

### Documentation

- Shortened the six pre-data SQLx migration identifiers to the readable
  `YYYYMMDDNN_description.sql` form. The contiguous numeric prefix preserves
  SQLx ordering while avoiding timestamp-like digits that do not carry useful
  information; underscores cannot divide the date because SQLx treats the
  first underscore as the version/description boundary.
- Defined PLE flat-question JSON as the small canonical machine format for
  static authoring and kept QTI as a profile-specific import/export adapter.
  The decision records the JSON-over-YAML reasoning, public/private persistence
  split, immutable source authority, feedback behavior, schema evolution, and
  the still-pending publication and instructor-editor packages.
- Renamed the opaque Rust crate paths atomically: `crates/store` is now
  `crates/learning-data-access`, its backend module is `in_memory`, and
  `crates/xtask` is now `crates/project-tools`. Cargo packages and directories
  use hyphens, Rust imports and modules use underscores, and repository
  automation uses `cargo tools` without the retired `cargo xtask` alias. The
  TypeScript generator recognizes the exact former ownership marker only long
  enough to rewrite generated files under the new package identity.
- Refreshed the architecture guide and added a file-structure guide that map
  each capability to its contract, in-memory implementation, PostgreSQL
  implementation, and focused conformance coverage. The README now introduces
  these plain names before relying on the internal Rust identifiers.
- Recorded the owner's componentization rule: preserve a modular monolith,
  split by capability ownership rather than arbitrary line ranges, keep crate
  roots as facades/composition points, and pair each component's narrow
  contract with its backend implementations and focused behavior tests.
- Replaced the root README's obsolete M0 stub description, frozen three-test
  output, and missing-server/schema/container claims with the current active
  implementation status, adoption blockers, verified repository front doors,
  assignment-to-analysis flow, implemented boundaries, and curated docs map.
  The original project purpose, server-only grading proof, tenant/content
  boundary, licenses, and author context remain intact.

### Developer Tests and Notes

- The embedded migration-status tests and full disposable PostgreSQL baseline
  gate pass with versions `2026080801` through `2026080806`. The gate proves
  six pending migrations, ordered apply, no-op replay, exact compatibility,
  checksum mutation detection, live worker and grading oracles, and real-role
  RLS denial; cleanup leaves only the pre-existing `pg-test` container.
- All five focused flat-question codec tests and both Object Store suites pass
  under strict Clippy. They prove canonical hashing, public/private secrecy and
  binding, correct and incorrect feedback selection, rejection of duplicate or
  extension members, the 256-KiB backstop, source non-signability, and normal
  published-asset delivery.
- `PLE_E2E_GATEWAY_IMAGE_SHA256=<pinned-digest> bash
tests/e2e/e2e_run_all.sh` passes 3/3. The exact cross-replica envelope,
  durable receipt replay, and one-row persistence set all pass.
- The complete disposable database gate passes with exact May-only pruning for
  `question_attempt`, `submission`, `record_access_log`, and `audit_event`.
  The 60,000-row gradebook fixture returns its bounded 51-row page from only
  `assignment`, `enrollment`, and `student_assignment_summary`; every
  disposable project is removed and the pre-existing `pg-test` remains alone.
- Added Memory and disposable-PostgreSQL family-filter oracles. Two concurrent
  PostgreSQL claimants lease distinct supported jobs through `SKIP LOCKED`, an
  older reserved job stays ready, and claiming another family does not
  dead-letter its expired lease. The complete six-migration, checksum, role,
  RLS, QTI, item-analysis, and manual-grading database gate passes with the new
  broker signature.
- All 8 QTI adapter tests pass, including hostile paths, symlinks, expansion
  limits, XML depth/width limits, server-only archive and grading handoffs,
  partial success, exact duplicate warnings, and likely duplicate warnings.
  QTI Store conformance passes in both configurations; the focused server
  import test and the new PostgreSQL live oracle pass. The fresh six-migration
  database gate also passes checksum mutation, real-role RLS, item analysis,
  and mixed manual grading after the queue/grant corrections.
- All five focused QTI runtime tests and the complete 143-test server suite
  pass after the split. The tests still prove no answer material reaches the
  browser, invalid tenant/provenance/asset bindings refuse before private
  grading, and an idempotent submission replay performs no second grader
  lookup.
- All six focused QTI publication tests pass after the ownership move,
  including foreign-tenant denial before object lookup, changed-draft and
  review-time revision races, bytes-first source copying, strict revision
  parsing, and promotion only from committed staging.
- Both `cargo tools database --help` and its `cargo tools` compatibility form
  produce the same project-tool help; all 18 project-tool tests pass. The full
  repository gate passes all 11 stages, including generated projection drift,
  TypeScript checks, 149 Node tests, crate-boundary enforcement, strict Rust
  Clippy, workspace tests, and doctests. Markdown and ASCII checks pass 444
  cases, and the maintained shell scripts pass syntax and warning/error-level
  ShellCheck validation.
- Protected asset conformance passes in both learning-data-access feature
  configurations; all six server asset tests pass. Both complete Store suites,
  strict Clippy in both modes, workspace compilation, and the full 11-stage
  repository gate remain green after the extraction.
- Session replica/revocation and backend-clock expiry conformance passes in
  both learning-data-access feature configurations. QTI staging, publication,
  redaction, and grader-isolation conformance also passes in both modes; all
  15 focused server QTI tests and all 24 focused server authentication tests
  pass after the split.
- External-tool Store conformance passes with and without PostgreSQL features,
  the focused server route tests pass, and both complete Store suites, strict
  Store Clippy modes, workspace compilation, Rust formatting, Markdown/ASCII
  hygiene, and staged/unstaged diff checks remain green after the extraction.
- Added focused reducer, Store conformance, server API/worker, and authorized
  HTTP fixtures. They cover mixed automatic and pending-manual work, correction,
  stale-stage supersession, latest-run selection, force-submit and clear,
  instructor/administrator access, cross-tenant non-enumeration, response
  redaction, and exact completion-time parity.
- Extended the disposable PostgreSQL 17 baseline gate with the production
  item-analysis path. It proves six-file apply/no-op/verify, real-role RLS,
  lower-priority job claiming, mixed manual correction from `0.25` to `0.5`,
  stale analysis rejection, one corrected current report, exact terminal
  submission timing, and cleanup. All disposable projects are removed after
  the run; the developer's existing `pg-test` remains untouched.
