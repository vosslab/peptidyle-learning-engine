# Changelog

## 2026-08-20

### Additions and New Features

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
  is now the sole professor handoff; T3 remains parked. LD2 receives `2026081809` only for safe
  Sysadmin approval-candidate discovery; the selector, ownership claim, and passkey seams remain
  non-schema. Whole live-demo Validation remains incomplete until LD2 and the connected goal close.

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
