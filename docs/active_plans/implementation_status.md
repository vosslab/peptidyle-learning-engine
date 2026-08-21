# Implementation status and handoff

Last updated: 2026-08-21

This file is a durable execution handoff. The architecture, scope, milestone order, security
boundaries, and acceptance criteria remain authoritative in the implementation and active plans.
Durable owner decisions remain authoritative in `../HUMAN_GUIDANCE.md`. This file is authoritative
for the changing global current-package handoff and shared migration allocation registry; other
documents link here instead of copying those mutable values.

The active [professor capability plan](active/professor_capability_architecture_plan.md) supplements
the release plan and does not replace it. This file is the sole global current-package handoff
registry: WP-PROF-T2 and WP-PROF-LD1 are accepted on 2026-08-20; WP-PROF-LD2 is accepted on
2026-08-21; WP-PROF-BS1 is the sole current professor package; and the release queue is PARKED at
still-open WP-RC8.
The professor and release plans own their scope and dependency queues, but defer current-handoff
truth to this registry. WP-PROF-S1, WP-PROF-S2, WP-PROF-S3, WP-PROF-S4, WP-PROF-S5, and WP-PROF-S7
are accepted, as are WP-PROF-S6, WP-PROF-T1, WP-PROF-T2, WP-PROF-LD1, and WP-PROF-LD2. LD1 delivered
the live-demo baseline lifecycle. LD2 accepted its seeded-entry, initial-Sysadmin-claim, and
connected-live-authoring capability. WP-PROF-BS1 is current. WP-PROF-T3 is the planned frozen-scope
successor after accepted BS1, and WP-PROF-T4 follows T3. LD2 uses the
necessary existing WP-RC8 account-session/passkey/origin contracts; its claim, passkey, and
Student/Instructor selector seams remain non-schema. WP-RC8 remains PARKED and open for provider,
mailbox, unrelated passkey, multi-replica, security, HCI, and release gates.
Professor pre-production work may use the shared migration ledger; it does not accept or imply
production email authentication, mailbox delivery, onboarding, deployment, or release acceptance.

WP-PROF-LD1 is accepted. It owns the durable live-demo installation-state and Base Course lifecycle
named by the approved [live-demo specification](../LIVE_DEMO_SPEC.md). Its accepted migration is
`2026081808_live_demo_install_state.sql`. WP-PROF-LD2 is accepted. Its immutable `2026081809`
allocation owns exactly two least-privilege execute-only PostgreSQL brokers: safe normal Sysadmin
approval-candidate discovery and read-only completed live-demo installation-generation lookup used
to bind configured first-ownership proof. Its separately accepted immutable `2026081810` is only the
narrow Student pre-tenant account-course context retention-boundary repair. Selector behavior and
claim, passkey, account, and session data and semantics remain non-schema; the generation-read
broker is the narrow schema authorization seam for that otherwise non-schema ownership flow.
WP-PROF-BS1 is current and is a non-schema browser-architecture package unless implementation
discovers an independent real persistence requirement. It owns the canonical disposable HTTPS
production-browser suite. WP-PROF-T3 remains the planned, non-schema, non-mutating preview plane:
it reuses the forced-RLS `audit_event` and writable repeatable-read snapshot for the one successful
learner-derived-subject audit, while accepted `2026081807_teaching_operations.sql` remains immutable.
The active professor plan owns the frozen LD1, BS1, and T3 scope, privacy boundaries, dependencies,
and acceptance matrices; this registry owns only the current-package handoff and migration allocation truth.

## Shared migration ledger and allocation

The `release integrator` is the single migration-order and ledger owner across the release and
professor roadmaps. This section is the shared allocation registry; plans reference it rather than
creating competing reservation tables. Accepted migration files are immutable and are not amended or
renumbered. Future schema packages receive an allocation before implementation; non-schema packages
do not implicitly receive one.

| Allocation | Package | Capability or disposition |
| --- | --- | --- |
| `2026080801`-`2026080806` | Foundational baseline | Accepted six-file baseline |
| `2026080907` | WP-RC1 | Course appearance; accepted |
| `2026080908` | WP-P2 | Secure question-grading payloads; allocated |
| `2026080909` | WP-RC8 | Passwordless identity and enrollment; allocated, acceptance open |
| `2026080910` | WP-RC7 | Object reconciliation; reserved, acceptance open |
| `2026080911` | WP-RC9 | LTI Advantage; reserved, acceptance open |
| `2026080912` | WP-FU1..WP-FU6 | Secure learner uploads; reserved, acceptance open |
| `2026080914`-`2026080935` | Repository-owned release packages | Existing forward allocations; files are immutable |
| `2026081401` | WP-R0 | Ranked catalog discovery; existing forward allocation |
| `2026081501`-`2026081504` | WP-RC8 repairs | Existing forward allocations; files are immutable |
| `2026081801` | WP-PROF-S2 | Course term and time zone; accepted and immutable |
| `2026081802` | WP-PROF-S7 | Typed references and bylines; accepted and immutable |
| `2026081803` | WP-PROF-S5 | Entitlement, typed group purposes, and materialization; accepted and immutable |
| `2026081804` | WP-PROF-S3 | Effective-policy resolver; accepted and immutable |
| `2026081805` | WP-PROF-S4 | Disclosure policy; accepted and immutable |
| `2026081806` | WP-PROF-S6 | Course grade scheme; accepted and immutable |
| `2026081807` | WP-PROF-T2 | Teaching operations; accepted and immutable |
| `2026081808` | WP-PROF-LD1 | Live-demo installation state; accepted and immutable |
| `2026081809` | WP-PROF-LD2 | Accepted and immutable; exactly two least-privilege execute-only brokers: Sysadmin approval-candidate discovery and read-only completed-installation-generation lookup for configured first-ownership proof |
| `2026081810` | WP-PROF-LD2 | Accepted and immutable; only the narrow Student pre-tenant account-course context retention-boundary repair |

The S3, S4, and S5 allocations were reordered before any of their migration files existed.
Accepted S5 occupies `2026081803`, accepted S3 occupies `2026081804`, accepted S4 occupies
`2026081805`, accepted S6 occupies `2026081806`, and WP-PROF-T2 occupies `2026081807`.
`2026081807_teaching_operations.sql` is accepted and immutable, and the forward migration order
remains contiguous through `2026081807`. `2026081808_live_demo_install_state.sql` is accepted and
immutable. `2026081809` and `2026081810` are accepted and immutable. The 1809 allocation accurately
records its two existing brokers; no additional migration is warranted. No placeholder migration,
absent-file dependency, or out-of-order application
is permitted. Numeric allocation records the forward migration sequence, while package dependency
remains defined by the professor plan.

WP-PROF-LD1 accepted the durable `installing` and `complete` installation state; one advisory lock for
single-writer first-install coordination; deterministic Base Course seeding with generation-bound
storage receipts; and the fresh or mixed PostgreSQL/object-storage lifecycle rules. Retried
`installing` work resumes the same verified generation. Retained `complete` restarts perform no seed
writes, storage inspection, or equality scans. A pre-marker database or mixed database/storage pair
fails closed and directs fresh regeneration of both stores. `learning-data-access` is the sole SQL,
PostgreSQL-lock, durable-state, migration, and Store owner. The focused product crate
`crates/base-course-installation/` (`base_course_installation`) owns the typed recipe and
orchestration; `project-tools` is its direct CLI adapter. The product crate has no HTTP route or
server-start hook. LD1 acceptance includes migration and live lifecycle evidence for interruption
and resume, retained restart, fail-closed mixed state handling, and fresh regeneration. It does not
add account, passkey, session, authentication, origin, or replica schema or behavior: WP-RC8 retains
those security boundaries.

WP-PROF-LD2 was accepted on 2026-08-21 after the authority record accurately classified the two
existing 1809 brokers. The `createbuckets` receipt-create operation previously
exited 137 at 128 MiB; its bounded resource limit is now 256 MiB, and independent review accepted
that narrow repair. The full post-repair runtime tree passed `./check_rust.sh` (terminal `PASS: Rust
workspace checks and tests completed.`); `./check_codebase.sh` (five checks, 322 Node tests);
`source source_me.sh && python3 -m pytest tests/` (6,017 passed, no skips); and `source
source_me.sh && python3 tests/e2e/e2e_live_demo_baseline.py` (PASS: fresh install, retained edit
without storage access, interruption repair, concurrency, pre-marker refusal, and fresh
regeneration with unclaimed seeded Sysadmin). `source source_me.sh && python3 local_stack.py
acceptance` passed all eight lanes, including its terminal connected HTTPS Playwright journey (one
passed) under project `ple-live-demo-browser-d0ff0e97f4ac`; typed cleanup and exact-label checks
left zero containers, volumes, and networks. The baseline E2E project
`ple_live_demo_baseline_124c398f82978266c7370838` likewise left those exact inventories empty.
Both diff checks passed, with no `__pycache__`, `.pyc`, or `.pyo` files. The package acceptance
does not itself prove final-goal completion: final-goal completion additionally requires the complete
final-material-tree Validation after these record edits. This accepts neither public/AWS/operator
deployment nor WP-RC8.

The actual clean-cluster baseline replacement requires both professor WP-PROF-E2 readiness and completion
of all repository-owned release schema packages/RC12, immediately before first production data. WP-PROF-E2
may prepare and review a candidate baseline earlier, but it must not replace the ledger early.

WP-PROF-T2 is accepted on 2026-08-20. It implements many-to-many course groups with five
purpose-specific multiple-membership policies, referenced-group refusal, atomic S5/S3
re-evaluation with sealed receipt history, operator-owned Instructor approval, target-bound 30-day
co-instructor invitations, direct-membership acceptance, final-Instructor protection, server-owned
retention actions, and server-derived entitlement/effective-policy previews. Migration
`2026081807_teaching_operations.sql` is accepted and immutable. Final material-tree Validation
passed `./check_rust.sh`; `./check_codebase.sh` (five checks and 301 Node tests); `source
source_me.sh && python3 -m pytest tests/` (5,481 tests); built Playwright (245/245, zero skips); a
fresh PostgreSQL 17 baseline through all 43 migrations with the T2 live oracles; `source
source_me.sh && python3 local_stack.py acceptance`; T2 visual capture (2/2); and UI corpus
verification (42/42). Both diff checks passed. T3 remains a later dependent package; the current
registry now records WP-PROF-LD1 as accepted and WP-PROF-LD2 as the sole professor handoff;
WP-RC8 remains parked and open. This closeout does not claim provider or mailbox delivery,
production email, passkeys, multi-replica operation, deployment, release activation, or an early
clean-cluster baseline replacement.

WP-PROF-S1 acceptance evidence on 2026-08-18 is recorded on the final material tree:
`source source_me.sh && python3 -m pytest tests/test_markdown_links.py tests/test_ascii_compliance.py`
passed 1,471 tests; `source source_me.sh && python3 -m pytest tests/` passed 5,235 tests in 3.13
seconds; and both `git diff --check` and `git diff --cached --check` passed. Independent acceptance
review returned ACCEPT with no P0/P1/P2 finding.

WP-PROF-S2 is accepted on 2026-08-18. Its final material-tree Validation passed: `./check_rust.sh`;
`./check_codebase.sh` (five checks and 261 Node tests); `source source_me.sh && python3 -m pytest
tests/` (5,235 tests); outside-sandbox `./run_playwright_tests.sh --build` (203 of 203); and the
outside-sandbox `tests/e2e/e2e_database_baseline.sh` PostgreSQL 17 lane (37 migrations, including
the exact course-term constraint, round-trip, and RLS oracle). Both diff checks passed. Independent
database/domain, browser/HCI, and architecture/test final reviews returned ACCEPT with no P0--P3
finding. The release Wasm gzip result is 231,911 bytes, a 353-byte increase. The accepted
test-only repairs replaced the native-date one-Tab assertion with the bounded real-Tab helper and
moved browser term decoding to its focused owner. That S2 closeout did not accept WP-PROF-S7,
WP-RC8, WP-RC12, or production activation.

WP-PROF-S7 is accepted on 2026-08-19. It establishes exact typed `C-`, `A-`, `R-`, `W-`, and `G-`
references, reserves `AC-` for the later Alpha aggregate, and resolves every route reference through
one authorized navigation result. Published versions retain an immutable, validated ordered public
byline that is distinct from private author-account IDs; safe catalog projections and publication
contracts carry the byline without exposing account authority. Migration `2026081802` is accepted
and immutable. Final material-tree Validation passed: `./check_rust.sh`; `./check_codebase.sh`
(five checks and 264 Node tests); `source source_me.sh && python3 -m pytest tests/` (5,235 tests);
outside-sandbox `./run_playwright_tests.sh --build` (203 of 203, zero skips); and outside-sandbox
`tests/e2e/e2e_database_baseline.sh` (fresh PostgreSQL 17 database, 38 migrations, the S7 live
reference/byline oracle, RLS denial matrix, and cleanup). Both diff checks passed. The release Wasm
measurement recorded input 1,122,735 bytes, bindgen raw 1,059,562 bytes, gzip 231,897 bytes, and
SHA-256 `b04c1572d361b10518138e2090a67a33ca78de795f44c175f3cde6b4d7264d15`; versus accepted S2,
the deltas are +373, +405, and -14 bytes. Independent PostgreSQL/RLS, Rust-contract, and
frontend/HCI reviews returned ACCEPT with no final blocking finding. The migration view-dependency
ordering repair remains useful implementation history, not a final failure. This closeout does not
accept WP-RC8, provider or mailbox delivery, passkeys, multi-replica operation, production deployment,
or release activation.

WP-PROF-S5 is accepted on 2026-08-19. One canonical `course_member` episode is now the current
course-membership authority, while `course_roster_profile` retains only subordinate display and
contact evidence. Assignment receipts are derived from typed course-wide or group audiences on the
first entitlement-bearing action, record immutable actor-or-rule provenance and sealed evaluator
basis/scopes, and are never recreated by a roster-by-assignment cross-product. Current learner
actions re-evaluate the same entitlement authority, while historical receipts remain historical.
Migration `2026081803` is accepted and immutable. Final material-tree Validation passed:
`./check_rust.sh`; `./check_codebase.sh` (five checks and 264 Node tests); `source source_me.sh &&
python3 -m pytest tests/` (5,232 tests); outside-sandbox `./run_playwright_tests.sh --build` (203 of
203, zero skips); outside-sandbox `tests/e2e/e2e_database_baseline.sh` (fresh PostgreSQL 17 database,
39 migrations, the exact entitlement/membership/RLS oracle, and cleanup); and outside-sandbox
`source source_me.sh && python3 local_stack.py acceptance` with all seven browser/visual/live lanes
green. Both diff checks passed. Independent domain/Store, PostgreSQL/RLS/security, and API/HCI/test
reviews returned ACCEPT with no P0--P3 finding. The sole professor handoff advances to WP-PROF-S3;
WP-RC8 remains parked and open, and this closeout does not claim provider or mailbox delivery,
passkeys, multi-replica operation, deployment, or release activation.

WP-PROF-S3 is accepted on 2026-08-19. It establishes one pure, ordered effective-assignment-policy
resolver: lifecycle, S5 entitlement, then action authorization deny before modifiers; approved group
schedule offsets and accommodations plus an individual exception resolve per field with provenance.
The resolver consumes the S5 decision and its opaque applicable scopes rather than reading roster,
audience, membership, or enrollment state. Memory and PostgreSQL use that same grant-filtered input
composition for resolution, start, issue, and list paths, so an unrelated group modifier cannot deny
an otherwise entitled learner. Migration `2026081804` is accepted and immutable: normalized base,
group, and individual inputs support append-only sealed per-attempt policy receipts, complete
per-field source rows, and a current pointer only to a sealed receipt. Final material-tree Validation
passed `./check_rust.sh`; `./check_codebase.sh` (five checks and 264 Node tests); `source source_me.sh
&& python3 -m pytest tests/` (5,220 tests); outside-sandbox `./run_playwright_tests.sh --build` (203
of 203, zero skips); a fresh PostgreSQL 17 baseline (40 migrations and the exact normalized S3
oracle, including cleanup); and outside-sandbox `source source_me.sh && python3 local_stack.py
acceptance` (ordinary browser, course-appearance visual, instructor-corpus visual, canonical
walkthrough, Chapter One pilot, live Chapter One 4/4, and live WebWork 4/4). A one-time external
renderer-image rebuild after pruning was an environmental acceptance prerequisite, not a PLE change.
Independent domain/Store, PostgreSQL/RLS, and consumer/test reviews returned ACCEPT with no final
blocking finding; both diff checks passed. The sole professor handoff advances to WP-PROF-S4.
WP-RC8 remains parked and open. This closeout does not accept provider or mailbox delivery, passkeys,
multi-replica operation, production deployment, or release activation.

WP-PROF-S4 is accepted on 2026-08-19. One assignment-owned five-field disclosure policy now governs
score, correctness, feedback text, solution, and class statistics independently. Learner projections
consume current S5 entitlement, the current S3-resolved effective-policy verdict, authoritative
server time, and submission fact; they do not reconstruct those authorities. The direct cutover
removes question-level timing and issuance snapshot columns, while `feedback_release` remains
retention-fenced audit evidence that cannot unlock a projection. Learner assignment and progress
transports omit policy, clock, tenant, and instructor-only inputs; class statistics use an
identity-free closed union with the fixed five-learner privacy floor. A central fail-closed browser
boundary denies every instructor-only route before protected components, course-theme reads, or
transport mount.

Migration `2026081805_assignment_learner_disclosure_policy.sql` is accepted and immutable. The fresh
PostgreSQL 17 baseline applied all 41 migrations and passed migration verification/idempotence, the
selected disclosure-policy/current-S3/RLS oracle, the class-statistics access oracle, and exact
cleanup. Final material-tree Validation passed `./check_rust.sh`; `./check_codebase.sh` (five checks
and 274 Node tests); `source source_me.sh && python3 -m pytest tests/` (5,418 tests and 2 subtests);
outside-sandbox built Playwright (228 of 228, zero skips); and outside-sandbox `source source_me.sh &&
python3 local_stack.py acceptance` with all seven ordered browser, visual, walkthrough, Chapter One,
and isolated disposable WebWork lanes green. Both diff checks passed. The 32-artifact screenshot
corpus includes fresh, inspected allowed-student and instructor-route-denial evidence at 1280 by 800,
800 by 1280, 393 by 852, and 800 by 800; direct route and no-transport tests, not pixels alone, prove
authorization. Independent architecture/security, tests/HCI, docs/legacy, student-access/HCI, and
corpus reviews returned ACCEPT with no unresolved P0--P3 finding. The sole professor handoff advances
to WP-PROF-S6; WP-RC8 remains parked and open. Local-development credentials and invitations were
used because email is unavailable. This closeout does not accept provider or mailbox delivery,
passkeys, multi-replica operation, production deployment, release activation, or an early
clean-cluster baseline replacement.

WP-PROF-S6 is accepted on 2026-08-19. It adds one revisioned course-grade scheme with total points
and weighted categories as the only shipped modes, deterministic drop-lowest behavior, exact point
arithmetic, one final four-decimal half-away-from-zero rounding step, optional letter bands, and
explicit unavailable states. Totals consume maintained assignment summaries rather than rescanning
attempts. The instructor-only browser edits the closed scheme with strong representation ETags,
shows bounded totals, and downloads a synchronous nine-column RFC 4180 export whose durable audit is
PII-free. Completion-based grading remains deferred design work and is absent from runtime,
database, HTTP, and browser contracts.

Migration `2026081806_course_grade_scheme.sql` is accepted and immutable. The final PostgreSQL 17
baseline applied and verified all 42 migrations, passed the course-grade scheme/totals/export/RLS
oracle and the 1805-to-1806 upgrade/retention oracle, completed the constraint, role, and forced-RLS
inventories, and cleaned its disposable project. Final material-tree Validation passed
`./check_rust.sh`; `./check_codebase.sh` (five checks and 278 Node tests); `source source_me.sh &&
python3 -m pytest -q tests` (5,480 tests and 2 subtests); outside-sandbox built Playwright (231 of
231, zero skips); and outside-sandbox `source source_me.sh && python3 local_stack.py acceptance` with
all seven ordered browser, visual, walkthrough, Chapter One, and isolated WebWork lanes green. Both
diff checks and the 36-artifact screenshot ownership/provenance verifier passed. The ordinary
rootless Podman demo is ready on loopback port 8080 with the complete nine-container service suite;
a repeated start replaced all nine prior container IDs, retained its three simulated-data volumes,
and left no owned dangling images. Independent architecture/security, tests/HCI, and
documentation/authority reviews returned ACCEPT with no unresolved P0--P3 finding. The sole
professor handoff advances to WP-PROF-T1; WP-RC8 remains parked and open. This closeout does not ship
completion-based grading or claim provider or mailbox delivery, production email, passkeys,
multi-replica operation, deployment, release activation, or an early clean-cluster baseline
replacement.

WP-PROF-T1 is accepted on 2026-08-19. One revisioned assignment teaching aggregate now owns the
closed Draft, Published, Closed, and terminal Archived lifecycle together with plain-text
instructions, availability, due and close instants, run and attempt limits, late behavior, and
server deadline behavior. Creation is Draft-only; publishing and later changes use the authenticated
revision-CAS settings mutation. Instructor local wall-clock input names the course IANA zone, while
the server alone rejects invalid, ambiguous, nonexistent, out-of-term, or misordered values and
persists absolute instants. Memory and PostgreSQL derive lifecycle gate G1 from stored state and
re-resolve active attempts consistently after a settings change.

Learner list transport stays compact and authority-free, while the separately authorized detail
projects only safe instructions and resolved delivery facts. Current, Recalculating, and Failed are
independent scoring states; every aggregate, run, attempt, submission, and feedback numeric value is
omitted unless scoring is Current. The browser gives instructors a direct assignment-editor action,
keeps learner-only assignment routes and instructor-only routes separated, moves focus into the
replacement task it reveals, and preserves keyboard recovery for field validation and stale
revisions. T1 is a non-schema package: at its 2026-08-19 acceptance, migration `2026081806` was the
last accepted immutable migration and the next schema allocation was unassigned.

Final material-tree Validation passed `./check_rust.sh`; `./check_codebase.sh` (five checks and 279
Node tests); `source source_me.sh && python3 -m pytest tests/ -q` (5,480 tests and 2 subtests); the
fresh PostgreSQL 17 baseline (all 42 migrations, the assignment-teaching lifecycle/policy/receipt/RLS
oracle, the retained S3--S6 oracles, and cleanup); and outside-sandbox `source source_me.sh &&
python3 local_stack.py acceptance`. The uninterrupted acceptance passed 237 of 237 ordinary built
browser tests, both visual-evidence lanes, J1--J5, the Chapter One publication oracle, the 4-of-4
live Chapter One browser journey, and the 4-of-4 live WebWork browser journey. The 36-artifact
screenshot verifier retains the inspected 1280 by 800, 800 by 1280, 393 by 852, and 800 by 800
student/access evidence; direct route and no-transport tests remain the authorization proof. Both
diff checks passed. Independent architecture/security, tests/HCI/browser, and docs/authority
rechecks returned ACCEPT with no P0--P3 finding. The sole professor handoff advances to WP-PROF-T2;
WP-RC8 remains parked and open. This closeout does not claim provider or mailbox delivery,
production email, passkeys,
multi-replica operation, deployment, release activation, or an early clean-cluster baseline
replacement.

## Working rules

- Keep the learning engine question agnostic. Biology examples are fixtures, not product policy.
- Keep answer-bearing content, grading keys, and grading logic server-side.
- Preserve tenant isolation, immutable published content, draft-versus-publication identity, and
  stateless API replicas.
- Preserve the shared dirty worktree. Do not reset, stage, commit, or discard unrelated changes.
- Complete one dependency-ordered slice, run its behavior gates, obtain an independent review, and
  update `../CHANGELOG.md` before advancing.
- Use `source source_me.sh && python3 ...` for repository Python commands.
- Use `python3 local_stack.py` for routine local-stack status, logs, start, restart, normal stop,
  reset preview, and validation. Its typed Python lifecycle is the startup/bootstrap owner.
- Keep maintained source ASCII/ISO-8859-1 compatible. Write non-Latin Unicode in Rust fixtures with
  escapes such as `\u{1F9EC}`.
- `../CLAUDE_HOOK_USAGE_GUIDE.md` is intentionally removed and does not govern Codex work.
- Use `../CODEX_SPARK_SUBAGENTS.md` for bounded Spark delegation.

## Completed and accepted code-first work

The shared tree contains independently reviewed, behavior-tested verticals for:

- immutable catalog publication, search, filters, safe details, and aggregate statistics;
- private workspace ownership/collaboration, revision CAS, safe publication diff, and author
  preview;
- revisioned assignment create/edit with server-owned capability validation;
- generic run, submission, feedback disclosure/release, summary pagination, and next-question
  prefetch;
- server-only native, WeBWorK, iMathAS, and QTI grading/source boundaries;
- private QTI import, publication, published grading, and opt-in production QTI composition;
- assignment export production with four requester-private, prompt-only artifacts;
- identity-free question statistics aggregation, persistence, catalog disclosure, and deletion
  survival;
- retention R1 through R4.4: pure policy, authorized persistence, worker cleanup,
  trusted due dispatch, schedule extension, archive-time assignment disposition, a revisioned
  Instructor/Sysadmin retention API, and truthful archive completion gates; and
- course appearance WP-CA1 through WP-CA7/WP-RC1: closed themes, Grass default, revisioned
  persistence, protected current-only banner objects, safe image normalization, all-seven-route
  Solid scope, keyboard-complete settings, live PostgreSQL/MinIO cleanup, visual evidence, and
  independent acceptance; and
- production-seam closure WP-RC2: implemented H5P/QTI/WeBWorK module names, no native renderer
  declaration, explicit catalog resolve/search Store capabilities, and durable feedback-release
  projection with independent no-P0/P1 review.

These statements describe code-first acceptance. Environment-dependent live PostgreSQL, object
storage, and deployed worker/replica exercises remain one-time deployment gates where documented.

## Accepted task: WP-R0 catalog discovery

WP-R0 is independently accepted on 2026-08-14. It closes the first professor-track M0
catalog-discovery slice; WP-R1 followed in that historical dependency order.

- A valid Question ID uses the exclusive exact-ID branch. Other queries use normalized lexical
  relevance with deliberate trigram typo recovery.
- Search ordering is deterministic: descending rank, descending similarity, then ascending problem
  and version IDs. The HMAC-authenticated cursor binds the query, ranking contract, keyset, and
  publication/first-disclosure snapshot boundary.
- Continuations retain their bound publication/disclosure boundary, immediately reevaluate current
  lifecycle and RLS visibility, and return complete cursor-independent facets for that bound set.
- PostgreSQL owns the canonical behavior. `MemoryStore` is the deterministic conformance model; no
  backend numeric-equivalence claim is made.
- Final evidence: 91 Memory library tests; 3 server catalog tests; 1,173 source-line cases; clean
  PostgreSQL 17 all-32-migration, idempotence, and verification baseline; named Store,
  continuation/disclosure, qualitative plan, broker/RLS/ownership, and maintained baseline lanes;
  and final independent ACCEPT. This is not full-repository, browser, or M0 acceptance.

## Accepted task: WP-R1 local teaching-loop evidence

WP-R1 is independently accepted on 2026-08-14. It closes the next professor-track M0
release-truth slice; WP-R2 followed in that historical dependency order.

- The completed statistics UI discloses the available release-truth evidence.
- Python owns the Chapter One pilot, Chapter One browser journey, and aggregate acceptance lanes over
  the typed `local_stack_control` boundary.
- The designated renderer image name is the stable local selection and rebuild target; each live run
  records its resolved OCI configuration ID as exact runtime provenance.
- Final Validation passed on the final material tree: `./check_codebase.sh`, `./check_rust.sh`, and
  `source source_me.sh && python3 -m pytest tests/` (4,865 passed), followed by
  `source source_me.sh && python3 local_stack.py acceptance` with all seven lanes green: ordinary
  browser, two visual checks, canonical walkthrough, Chapter One pilot, Chapter One browser, and
  canonical WebWork browser acceptance. The final independent review returned ACCEPT with no P0/P1.

## Accepted task: WP-R2 immutable-question release truth

WP-R2 is accepted on the final material tree. WP-PY-L1 is accepted on 2026-08-15 after final offline
and live Validation and its three named independent reviews. These four packages evidence accepted
M0 for the professor roadmap. The current handoff and release-queue state are recorded in the
opening registry entry above.

- Every content change publishes a new immutable `AAA-BBBB` Question ID and fresh hidden
  `(ProblemId, VersionId)` evidence. Optional one-way provenance retains source attribution without
  changing the source or advancing an assignment.
- Assignment creation and focused item replacement select Question IDs. A revision-checked
  replacement changes future runs, while existing assignments, issued runs, and attempts retain their
  exact evidence.
- The final Validation passed: `./check_codebase.sh` completed five steps with 260 Node tests;
  `source source_me.sh && python3 -m pytest tests/` passed 4,856 tests; `./check_rust.sh` passed the
  full Rust suite; and `source source_me.sh && python3 local_stack.py acceptance` passed all seven
  lanes: ordinary browser, two visual verifiers, canonical walkthrough, Chapter One pilot, Chapter
  One browser with four live Question-ID replacements, and WebWork render/grade/outage.
- Test, UI, and architecture reviews returned ACCEPT with no P0/P1 finding. The canonical renderer
  image was rebuilt only for acceptance; cleanup removed all disposable containers, images, and
  volumes while retaining the recorded OCI configuration ID as runtime provenance.

## Previously accepted task: MOD-RETENTION R4.3

The truthful archive boundary is independently accepted. Learner routes now use a central
retention access fence; worker construction is reusable; and archival lifecycle is
truthful only after exact cleanup and idempotent replay.

### Acceptance summary

- Central learner-record archive predicate now fences all learner-facing aliases (courses,
  assignments, enrollments, run/attempt/submission, summary/gradebook, feedback, prefetch,
  external-tool, exports, and StudentRecord-bound assets) via Store access checks and
  PostgreSQL RLS.
- Instructor routes retain direct-course definition read visibility. Sysadmin retains only the
  separately documented payload-free retention lifecycle authority, while Student rows use the
  central course fence and return concealed `404` for archived or deleted records.
- Archive completion is now strictly after exact cleanup, and both Store and server replay are
  exact/idempotent across retry/replay.
- Export, StudentRecord, and external-tool resurrection paths now terminate at the same closed
  archive gate.
- Public catalog asset routes remain unchanged and remain deliverable.
- Real `RetentionJobHandler`, `RetentionJobCommitter`, and `RetentionWorkerComponents` are now
  constructible from production PostgreSQL/object-store dependencies without starting queue
  activation.
- Independent review result: ACCEPT, no P0/P1 findings.
- Current gates are stable: server lib 140, Store lib postgres 44, Store conformance 16,
  Store retention postgres 18, object conformance 2, strict Store/server all-target Clippy,
  rustfmt, ASCII 337, Markdown links 45, PG conformance no-run, global/scoped diff clean.
- `PLE_POSTGRES_TEST_URL` was absent at that handoff; WP-RC7 and WP-RC12 own the live forced-RLS,
  SDF, trigger, and query-plan production-activation evidence and require PASS before release.

## Accepted task: MOD-RETENTION R4.4

R4.4A remains independently accepted. R4.4B had reached an accepted functional handoff, but the
later partial-commit audit correctly reopened its transaction-scale design. The non-destructive
R4.4A package froze the boundary needed before permanent deletion:

- persist the exact typed cleanup-object set before the worker deletes any object;
- replay that immutable manifest for a renewed lease on the same bound job;
- require the persisted manifest before cleanup completion;
- add indexed relational course ownership to student-record audit events without parsing JSON; and
- leave relational learner rows and the `deleted` lifecycle untouched until R4.4B supplies the
  complete purge transaction.

R4.4A evidence:

- Memory and PostgreSQL reject `deleteStudentRecords` preparation before mutation while still
  allowing the durable scheduler to dispatch and terminalize the unavailable work explicitly.
- Archive cleanup stores the exact normalized manifest before delivery revocation, replays it only
  for the same bound job under a renewed lease, and requires the prepared manifest plus current
  lease at commit.
- The manifest contains tenant-owned export and external-transcript objects; it never includes
  shared source objects or derives ownership from JSON payloads.
- Student-record audit events now carry indexed relational course ownership; catalog audit events
  remain course-free.
- Independent R4.4A review result: ACCEPT, no P0/P1 findings.
- Current offline gates are green: Store lib postgres 52, Store retention postgres 26, Store
  conformance 16, PostgreSQL conformance no-run, server lib 140, server retention worker 3, strict
  Store/server all-target Clippy, rustfmt, ASCII 337, and diff checks.
- Live PostgreSQL migration/RLS execution remains an environment-backed deployment gate.

### R4.4 test permanence classification

The repository test policy in `../PYTEST_STYLE.md` applies to the Rust suites as a maintenance
standard. R4.4 keeps only deterministic, offline behavior tests that protect durable authorization,
lease/replay, lifecycle, typed-object, and frozen assignment-disposition behavior. Those tests use
inline inputs or same-module helpers; R4.4 adds no committed `tests/fixtures/` data.

The committed fixture inventory now contains only `tests/fixtures/published_problem/`, which
[../HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md) explicitly approves as reviewed cross-layer
infrastructure. The small QTI ZIP/base64 and WeBWorK PG inputs are inline beside their parser and
backend tests rather than stored as separate fixture files. New committed fixture directories or
files require explicit human approval; temporary database, object-store, migration, and
reconstruction inputs are removed after their one-time evidence is recorded.

The following are one-time implementation or deployment checks, not permanent suite residents:

- rebuilding an empty PostgreSQL database through every migration, including migration replay;
- loading a representative relational purge graph and exercising broker roles, RLS, foreign-tenant
  concealment, malformed cross-course dependencies, lock ordering, and exact object manifests;
- inspecting migration SQL names, text fragments, statement order, or policy-name lengths; and
- live object-storage, multi-replica, worker-soak, query-plan, and deployment exercises.

Scratch SQL and ignored live-database reconstruction tests used while building R4.4B are removed
after their evidence is recorded. The one-time populated PostgreSQL 17 gate passed on 2026-08-08:
it rebuilt and replayed every migration, exercised malformed but FK-valid cross-course prefetch,
successor, and statistics-receipt links in both endpoint directions, purged every identifying link,
and preserved the control course plus the shared anonymous aggregate. The temporary Rust test, SQL
seed, database, and container were removed afterward; this evidence must not be replaced by a
committed fixture file or an implementation-string test.

R4.4B now materializes the exact course-owned relational purge graph while the archived course is
write-fenced, deletes typed objects idempotently, removes learner rows in verified foreign-key
order, applies the frozen assignment disposition, and sets the coarse retention tombstone to
`studentRecordsDeleted` only after every required effect succeeds. At the original R4.4B handoff,
before the later permanent-test pruning, the focused gates were green: Store lib 51, server lib 140,
Store retention 25, server retention worker 3, Memory conformance 16, PostgreSQL conformance
compile, strict Store/server Clippy, rustfmt, ASCII 340, Markdown links 46, and diff checks. The
independent R4.4B security review at that boundary reported no P0/P1 findings; the later partial-
commit audit findings are tracked separately and supersede that readiness claim.

The 2026-08-08 partial-commit audit reopened two permanent-purge design blockers: global
`EXCLUSIVE` table locks blocked unrelated tenants, and whole-course UUID arrays made the transaction
memory-bound. The working tree now replaces both with a course-retention-row writer fence and
private indexed run/attempt/export work sets. A removed one-time PostgreSQL 17 probe applied the
fresh migration chain, purged 50,000 attempts, overlapped a control-course enrollment write that
completed within two seconds, refused a same-course learner insert after prepare, removed every
attempt, and erased the private work sets before the tombstone. Permanent offline gates pass: Store
lib postgres 30, server lib 137, Memory conformance 11, server retention worker 3, and PostgreSQL
conformance no-run. Fresh independent scalability and security/atomicity rereviews both report
ACCEPT with no P0/P1 findings. Current commit readiness and safe partial-commit options are recorded in
[partial_commit_status.md](partial_commit_status.md); do not commit the mixed index before that
index is rebuilt from the accepted working tree.

## Accepted task: WP-QTI-12

The provenance-aware Memory and PostgreSQL conversion boundary is complete and independently
accepted:

- A closed, non-serializable staged profile-evidence value closes H2 while an import is prepared.
  Exact replay is idempotent; divergent evidence and staging after commit refuse without mutation.
- Conversion requires the committed accepted result to bind both its source identifier and exact
  `itemId` to the selected item, then revalidates the profile tuple and every integrity digest.
- Memory and PostgreSQL atomically advance the draft CAS revision and persist the draft, canonical
  source, current private grading, and current origin under the frozen lock order.
- Ordinary saves atomically stage current private grading and preserve imported origin. Publication
  accepts no grading payload from the caller; it promotes only the locked stored value after origin
  promotion.
- PostgreSQL uses the forced-RLS provenance and grading brokers. The Store implementation performs
  no direct reads of private grading, choice-map, or provenance secret tables.
- `Sha256Digest` JSON is exactly lowercase 64-character hexadecimal text and rejects uppercase,
  wrong-width, and non-hex input.
- Shared Memory/PostgreSQL conformance, PostgreSQL feature coverage, the full fresh baseline, and
  independent review passed. The review reported no P0/P1 finding.
- Detailed evidence is in
  `docs/active_plans/workstreams/qti_memory_postgres_implementation.md`.

The WP-QTI-9 server boundary is complete and independently accepted:

- An author upload stores the exact bounded ZIP as a deterministic private workspace object and
  enqueues one deterministic `qtiImport` job. Exact replay is stable; divergent replay refuses.
- The safe report exposes recognized package/item defaults, diagnostics, and acknowledgement digests
  without answer material, source bytes, object keys, or private mappings.
- The profile worker uses strict Canvas and Blackboard detection ahead of generic parsing, commits
  complete accepted-item evidence, and keeps mixed vendor evidence or all-rejected results from
  producing a conversion candidate.
- Conversion requires a current strong draft ETag plus report revision and acknowledgement tokens,
  rereads and reparses the archive, recompiles through the native bridge, and calls the WP-QTI-8
  atomic Store command. Refusals do not mutate a draft.
- Publication copies the source archive to deterministic non-signable `PublishedImportArchive`.
  Memory and PostgreSQL serialize prepared import work with draft deletion, preventing orphaned
  prepared evidence and unsafe identity reuse.
- Every upload, report, and conversion response is answer-free and `Cache-Control: no-store`; denied
  and absent resources have the same external response. Evidence is in
  `docs/active_plans/workstreams/qti_server_routes_implementation.md`.

The WP-QTI-10 author UI is complete and independently accepted:

- A feature-local browser client sends the exact ZIP as opaque bytes to the existing same-origin
  author route, accepts only strict bounded answer-free DTOs, and requires `no-store` responses.
  Browser code does not parse ZIP/XML or persist archives, reports, mappings, or private answers.
- The existing workspace route shows queued and processing states, manual refresh, accepted and
  rejected cards, defaults, warnings, explicit acknowledgement, all-rejected and unsupported
  recovery, and exact retry after an ambiguous upload. It does not add a product route.
- Conversion is guarded by the current displayed clean strong draft revision. It refetches the same
  route and opens the existing flat editor. The old editor is inert across the committed replacement;
  if refetch fails it stays locked behind an explicit repeatable reload action, with no repeat
  conversion or new import. The converted editor unlocks and receives focus only after a successful
  load.
- Permanent offline Node and real-route Playwright tests passed. The Playwright suite has four
  Chromium scenarios covering retry identity, safe mixed reports, all-rejected and unsupported
  recovery, stale/dirty/revision conflicts, refetch recovery, keyboard behavior, and 375 px reflow.
  `./check_codebase.sh` passed all 11 checks with 173 Node and 184 server tests. Independent security
  and HCI re-reviews reported no P0/P1 finding. Detailed evidence is in
  `docs/active_plans/workstreams/qti_author_ui_implementation.md`.

The WP-QTI-11 live gate is complete:

- A fresh isolated PostgreSQL 17 database applied and verified the six-file SQLx baseline and ran
  the real profile upload, worker, conversion, publication, native grading, and cleanup path.
- One minimized Canvas archive produced an accepted item and a visible rejected sibling. The
  accepted item remained editable, published as native flat content, and graded one correct and one
  incorrect response through the isolated PostgreSQL grader.
- Workspace and published archive bytes, canonical source, current and published origins, and their
  checksums agreed. Workspace cleanup removed current private state while immutable published
  provenance remained.
- Application, student, grader, and foreign-tenant probes enforced the RLS and protected-capability
  boundaries. Safe DTO scans found no archive bytes, object keys, correct-choice material, private
  choice maps, grader payloads, or unreleased feedback.
- The full disposable database gate, focused Rust/Node/Playwright checks, strict workspace Clippy,
  workspace tests, all 11 repository checks, 51 built Playwright scenarios, and 1,644 Python tests
  passed. Detailed evidence is in
  `docs/active_plans/workstreams/qti_live_acceptance_implementation.md`.

The WP-QTI-12 independent review and documentation close-out is complete:

- Six separate plan, test, style, documentation, legacy, and comment review passes found no
  production or test defect.
- Documentation review found stale README status plus missing profile-to-native ownership evidence
  in the contracts, code architecture, and file map. The four owner documents were corrected and the
  original reviewer confirmed both findings resolved.
- The contract and architecture maps now name the profile parser, author upload/report route, worker,
  conversion bridge, Solid author workflow, protected grader boundary, and disposable PostgreSQL/RLS
  oracle. That historical closeout assigned future QTI-JSONL ownership externally; the current owner
  decision instead uses PLE flat JSON v2 as the internal all-family source contract.
- Focused Markdown link, ASCII, README first-paragraph, whitespace, and Prettier gates passed. No
  P0/P1 finding remains.

## Recent owner-requested support work

- The 2026-08-11 Human Guidance reconciliation remains active; its completed content and
  instructor-identifier sub-slice does not close the broader guidance audit. The assignment editor
  resolves catalog titles and displays copyable canonical `AAA-BBBB` Crockford Question IDs plus
  backend labels rather than presenting UUID tuples as problem numbers. Server-side checksum
  validation and tenant/actor authorization resolve the exact assigned Question ID; hidden
  immutable snapshots and version identity remain internal for grading and provenance. Its
  **Add by question ID** control accepts one or more comma- or newline-separated exact IDs,
  resolves all of them before changing the draft, and preserves both pasted input and the assignment
  for malformed, unavailable, unauthorized, or duplicate IDs. The Chapter 1 source corpus now contains the owner-specified eight
  reviewed questions: one WeBWorK MC, WeBWorK MATCH, PLE flat MC, and PLE flat MATCH in each of
  Genetics and Biochemistry. `cargo tools pilot-content` proves the source/compiler contract, and
  `bash tests/e2e/e2e_chapter_one_pilot.sh` now passes the real PostgreSQL/MinIO publication path,
  exact idempotent rerun, four-native/four-WeBWorK assignment split, roster-derived enrollments, and
  eight distinct human display IDs. Direct renderer probes pass MC and matching grading, including
  partial credit. The normal launcher now publishes the same two assignments. The exact built PLE
  browser gate completes all eight questions through visible keyboard controls, verifies feedback
  and fresh practice for both chapters, and consults no answer key. That gate caught and repaired
  the adapter's obsolete 64 KiB private-JWT limit: the reviewed PG state is now admitted within the
  already enforced 1 MiB renderer-response boundary. It also exposed reviewed PGML choice labels
  containing a narrowly styled color span; the adapter now projects that exact reviewed shape to
  plain text while continuing to refuse arbitrary or hostile label markup. The matching path now
  projects the current renderer's direct selects, mixed plain/color labels, and exact empty
  compatibility controls without widening arbitrary markup. Both tracked matching sources now
  provide numeric partial-credit thresholds instead of string-formatted JSON scores. The canonical
  walkthrough visibly constructs the Genetics four-question assignment in J13; the isolated release
  gate owns the all-eight learner sweep.
- The current PLE-owned student browser flow and all implemented response families passed a focused
  no-mouse audit. The primary route uses Tab, Shift+Tab, Space, explicit submission, and native link
  activation; Arrow, digit, Enter-to-submit, and Escape extensions have separately classified
  component scenarios. Representative VoiceOver and NVDA sessions remain a fall-pilot human gate.
- Focused private `local_stack_control` Python modules are the maintained build/bootstrap seam behind
  the public controller. They own lifecycle sequencing, default-only private configuration and local
  identity, renderer selection and OCI configuration-ID provenance, Podman subprocess control, polling,
  migration, seeding, grading-role provisioning, semantic readiness, and optional browser opening without
  deleting persistent volumes. The accepted stateless PG renderer remains a required default local-stack
  service using its selected rebuild target, per-run OCI configuration ID, and private `/render-api`
  integration. WP-PY-L1 has passed final offline and live Validation: default typed start/status/validate,
  renderer stop/restart plus full WebWork RPC, schema-v2 canonical walkthrough J11-J13/J1-J5/J8,
  replica/restart durable replay, and all seven aggregate lanes. The final state has zero containers/networks
  and exactly `containers_ple_pgdata`, `containers_ple_miniodata`, and `containers_ple_identity_runtime`
  retained. `final_python_repository_review.ae3`, `final_podman_security_review.c2`, and
  `walkthrough_acceptance_final_review.ae3` each ACCEPT with no P0-P3 finding; WP-PY-L1 is accepted on
  2026-08-15. The professor roadmap records M0 as accepted; the current handoff and release-queue
  state are recorded in the opening registry entry above.

  Live evidence also corrected renderer OCI-ID normalization; database-seed environment fallback;
  unsupported Compose `rm` removal; restart recovery/readiness; semantic renderer probing; Chapter 1
  private provenance output; replica Question-ID manifest handling; and browser readiness/foreground
  handoff. The maintained sibling `webwork-pg-renderer` required its own PG build-context and standalone
  compatibility fixes; those sibling changes are dependency evidence only and are neither staged nor
  committed by this repository status.
- [docs/DATABASE_STRUCTURE.md](../DATABASE_STRUCTURE.md) maps implemented revision, assignment, and
  isolated-score relations. The human owner has superseded the earlier institutional-OIDC-only
  decision: WP-RC8 now owns PLE-managed passwordless accounts, invite-by-email enrollment, passkeys,
  manual roster/grade export, and optional SSO account linking. The document records pilot and
  ten-million-question growth formulas without claiming production email/WebAuthn configuration or
  legal sign-off.

## Accepted task: WP-RC1 course appearance

WP-CA1 through WP-CA7 and WP-RC1 are accepted on 2026-08-09. The production contract, Store/RLS,
object, server, Solid, and learner-entry owners now implement one of 15 measured themes and one
optional exact 1200 by 328 course-entry banner. The instructor settings page is keyboard complete,
preserves local state through validation/network/auth/permission/CAS failures, and supports explicit
reload, replacement, and removal. A bounded request-triggered cleanup executes the real claim,
tenant-owned MinIO deletion, and completion sequence without deleting the exact current object.

Acceptance evidence is recorded in
`docs/active_plans/workstreams/course_appearance_implementation.md`: the disposable PostgreSQL and
MinIO stack passed, including the `ple_app` cross-course pointer refusal and combined lifecycle;
`./check_codebase.sh` passed all 11 checks; the rebuilt browser passed 62 tests with the opt-in
visual generator separately passing 1/1; Python passed 1,743 tests; staged and unstaged diff checks
are clean; and three independent reviewers reported no P0/P1/P2.

## Dependency-ordered remaining work

### Current walkthrough disposition

> **WP-HG1 accepted on 2026-08-12.** The strengthened local instructor-to-student workflow,
> issuance-owned receipt boundary, compact interface, screenshot set, and Chapter 1 content oracles
> now have permanent and live evidence. Earlier M10/M11 runs remain useful historical evidence for
> their narrower baseline; the current acceptance comes from the rebuilt clean-stack teaching loop,
> the separate all-eight learner sweep, and the complete disposable PostgreSQL baseline. This does
> not accept WP-RC5's remaining integrated HOTSPOT lifecycle or the broader release.

- The repository owner corrected the binding walkthrough charter on
  2026-08-11. Overall acceptance requires visible instructor course creation,
  canonical roster membership/enrollment for the fictional learner, and
  corpus-backed assignment creation before the student keyboard
  take/score/repeat loop. Email and canonical onboarding are intentionally
  outside this walkthrough. The local-file identity configuration authenticates
  the fictional actors only; it is not a second roster, membership, or
  enrollment system. The current disposable seed uses the canonical
  `UpsertCourseMember` operation to create the no-contact roster member,
  membership, and enrollment records that the product uses. The rebuilt
  empty-stack teaching-loop evidence passed as part of the current acceptance.
- The earlier M5 learner slice remains accepted evidence: visible native
  keyboard pagination traverses retained assignment and gradebook pages, and
  manager plus independent
  `bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build` runs passed
  J1, J2, J3, J4, J5, and J8 with empty diagnostics. It used an API-arranged
  assignment in a seeded course, so it is not final evidence for the corrected
  instructor setup charter.
- The corrected schema-v2 report contains only the ordered PASS rows J11, J12,
  J13, J1, J2, J3, J4, J5, and J8, with the sole
  `launcher-chapter-one-genetics` arrangement label. J5 visibly proved Best
  `100%`, Latest `100%`, Completed `2`, and two completed run-history rows;
  the two student runs used the keyboard platform path. The private report
  directory/file were mode 0700/0600 and redacted; runner
  no-volume cleanup left no private temporary state or containers. The cursor
  session uses opaque cursors, retry, deduplication, and fail-closed protocol
  handling; native `target="_self"` fragment links enter named
  `tabindex="-1"` regions, then Tab reaches visible actions. The route
  lifecycle guard prevents delayed course A responses from changing course B.
- The default canonical walkthrough now also requires two owner-requested human-guidance checks
  without widening the public schema-v2 report: J13 verifies an operational human-readable
  `AAA-BBBB` Question ID plus backend label rather than UUID text, copies/pastes the four exact
  Genetics Chapter 1 IDs, and visibly observes four selected questions before creation. Server-side
  checksum validation and tenant/actor authorization resolve the exact assigned Question ID;
  immutable snapshots and version identity remain internal for grading and provenance. The complete
  two-chapter eight-question sweep remains the isolated release oracle.
- The current runner refactor replaces inherited hidden Python walkthrough switches with documented
  arguments and one explicit schema-versioned private child-input boundary. Focused offline tests
  cover its validation and environment isolation. The rebuilt Podman/Playwright execution passed
  the strengthened J13 visible copy/paste path and supplied the one-time acceptance evidence.
- Historical WP-HG1.T accepted the former revision-atomic
  `assignmentTiming.timeLimitSeconds` boundary and its 900-second Mastery default. WP-PROF-T1
  directly removes that wire and compatibility API: current whole-run timing is one field of the
  revisioned `AssignmentTeachingSettings` aggregate with lifecycle, instructions, schedule, late,
  and deadline behavior. The earlier timed Podman walkthrough remains historical evidence; it is
  not the current contract or current T1 acceptance evidence.
- The previous corrected-charter evidence in M8-M11 remains useful historical
  evidence for the bounded local pilot, but is superseded as acceptance evidence
  by the strengthened human-reference contract. WP-HG1 now records the rebuilt live J13
  copy/paste run, refreshed screenshots, and independent review. This vertical slice does not
  close RC4--RC12 or substitute the local-file authentication adapter for
  production account onboarding.
- Post-acceptance WP-E2 is accepted. The unchanged compatibility entry points
  delegate to the canonical `tests/walkthrough/` command and importable
  `walklib/` owner. A fresh host-bound retained-stack run passed all nine
  schema-v2 journeys and left only the redacted 0700/0600 public report; Podman,
  private state, and Python bytecode caches were empty. This maintenance slice
  does not reopen or broaden the accepted pilot.
- WP-G1, the schema-v1 WP-G2 baseline, G3 documentation, and the prior final
  review remain accepted historical evidence for the narrower learner slice.
  They are superseded as final acceptance artifacts by schema v2 for this
  corrected instructor-to-student sequence. WP-RC8 email/account acceptance
  remains separate release work and does not block this walkthrough. J6/J7,
  canonical onboarding, all-family, multi-learner, and working-codebase
  release acceptance are likewise outside this pilot and remain unaccepted.

### Human-guidance reconciliation ledger

The 2026-08-12 whole-file review distinguishes completed current-product guidance from later
release activation. A row marked accepted records both implementation and its named evidence; it
does not silently accept a future package.

| Human-guidance area                                               | Current disposition                                           | Evidence or remaining owner                                                                                                                                                                                                                                                                      |
| ----------------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Device priorities and visual density                              | Accepted for the current instructor and learner surfaces      | Canonical 1280 by 800 clean-stack screenshots, 800 by 1280 student/tablet task coverage, one narrow-phone compatibility guard, and the compact-interface browser gates. CSS Grid/Flexbox plus media/container queries remain the default; no responsive-menu dependency was added.               |
| Human-readable instructor identifiers                             | Accepted                                                      | The live instructor path copies and pastes four exact Genetics `AAA-BBBB` Question IDs. Server checksum validation and tenant/actor authorization resolve each exact assigned Question ID, while malformed/unavailable/unauthorized/duplicate recovery remains covered by permanent behavior tests. |
| Teaching loop, timing, repeat practice, and gradebook             | Accepted as WP-HG1                                            | The visible no-email J11/J12/J13/J1--J5/J8 path, 15-minute run, fresh second run, and two-run gradebook passed with redacted private evidence.                                                                                                                                                   |
| Student keyboard interaction and native-family recovery           | Accepted for implemented surfaces                             | Platform Tab/Shift+Tab plus Enter/Space works through the live learner path; all eight native widgets have keyboard controls, answer-free progress, and unsubmitted reset/recovery behavior.                                                                                                     |
| Native flat v2 authoring and runtime                              | Implemented; integrated package acceptance remains WP-RC4/RC5 | All eight closed v2 source families, protected visual editors, server-only grading, immutable HOTSPOT publication, and issue-time asset binding are implemented. Full all-family PostgreSQL/object-store author-to-learner lifecycle and screen-reader closeout remain open.                     |
| First Chapter 1 content                                           | Accepted as the bounded content oracle                        | Deterministic Genetics and Biochemistry four-question assignments pass exact publication rerun, direct renderer grading, and the separate built-browser eight-question sweep.                                                                                                                    |
| Issued presentation, grading authority, and receipt replay        | Accepted as WP-HG1                                            | Memory/HTTP/browser gates plus the live PostgreSQL receipt/corruption oracle prove first grade and replay use issuance-owned state rather than mutable current catalog or backend definitions. The later compact type-free learner wire remains WP-P1--WP-P6.                                    |
| Wasm/browser trust boundary                                       | Accepted for current exports                                  | The native/Node/real-Chromium shared corpus passes; the shipped Wasm dependency closure remains exactly `wasm_bridge`, `domain`, and `question_model`, with no grading crate or answer key.                                                                                                      |
| Course appearance, score precision, retention, and local recovery | Existing accepted or implemented boundaries remain valid      | Course appearance is accepted; Rust/TypeScript midpoint and display tests agree; 30/100/365-day retention is implemented; the local logical restore rehearsal remains local evidence only. Managed backup/PITR, deployment keys, and numerical recovery objectives remain WP-RC10 operator work. |
| Modular ownership, tests, generated artifacts, and local storage  | Accepted for the current tree                                 | Every maintained source is below 1,000 lines; configuration choices are explicit; generated projections regenerate before checks; the broad gates pass; and the measured Rust dev profile retains line-table backtraces without incremental cache growth.                                        |
| Authentication and email                                          | Deliberately non-gating and not activated                     | Local fictional identities own the pilot. No SMTP/email activation is configured; Fastmail is future operator intent. Production email delivery, passkey, multi-replica, and onboarding acceptance remain WP-RC8 and are not claimed here.                                                       |

### Current package order

The complete sequence is authoritative in
`docs/active_plans/active/release_completion_plan.md`:

1. WP-RC1 course appearance is accepted.
2. WP-RC2 production-seam closure is accepted. WP-RC3's pinned upstream WeBWorK `/render_rpc`
   integration is accepted after its live PLE/browser gate and final independent review. It remains
   compatibility evidence, not the final runtime: accepted WP-RC3R replaced the full
   WeBWorK2/MariaDB/render-course stack with the private standalone WebWork PG renderer.
3. WP-ARCH1 is accepted. Its dated 26-file maintained-source acceptance baseline had zero
   maintained-code violations behind stable facades; the permanent size gate (582 tests),
   2,451-test Python suite, eleven-stage codebase gate, and 72-pass browser suite were green. Its
   disposable PostgreSQL
   migration/RLS/conformance baseline also passes through the decomposed owners, and independent
   PostgreSQL, security, provider, TypeScript/HCI, test, size-policy, and architecture reviews found
   no unresolved P0/P1 issue. The later persistence regressions were repaired by moving complete
   attempt-issuance capabilities into paired in-memory and PostgreSQL owners. The current permanent
   size gate passes 824 cases, and the feature-enabled persistence check, test, and strict Clippy
   gates pass.
4. WP-RC3R has removed the parallel WeBWorK2 assignment application, render-course credentials, and
   MariaDB. The normal stack now relies on the external stateless `webwork-pg-renderer` image and
   retains immutable-source, strict projection, grading, cache/replay, outage, and browser-secrecy
   behavior through the private `/render-api`. WP-RC3R is accepted: its focused, complete repository,
   and live Podman/browser gates pass, and independent review found no unresolved P0/P1/P2 after the
   configured renderer identity was bound to cache reuse and grading of persisted attempts.
   `OTHER_REPOS/` is read-only comparison evidence.
5. WP-RC8 has implemented the generic passwordless/account, copy-link/optional-SMTP enrollment,
   course-roster metadata, bulk-import, and manual-grade-export routes with acceptance open.
   Migration `2026080909_passwordless_identity.sql` has passed the disposable PostgreSQL baseline.
   Production now composes the provider-free PLE passwordless/account/session route graph with an
   eight-hour `FirstPartyHttps` policy and explicit `ReviewNotRequired`; it neither reads local
   identity settings nor mounts `/api/auth/login`. The separate local launcher preserves the
   local-file provider only when the exact development flag selects it. The opt-in
   external-provider overlay supports authenticated STARTTLS or implicit TLS and mounts only a
   copied mode-0600 credential file; it adds no mail service. A live provider send,
   optional-passkey and multi-replica evidence, and independent security/HCI closeout remain before
   acceptance; WP-RC4 resumes after that closeout. Email
   authentication is the canonical account path; no manager-assisted account merge or
   educational-record transfer is a version 1 dependency.
6. WP-RC4's PLE flat JSON v2 implementation now covers the eight source/runtime families and awaits
   independent closeout; external QTI-JSONL is no longer a prerequisite.
7. WP-HG1 accepted the issuance-owned presentation, family grading, timing, and immutable receipt
   boundary. WP-P1 through WP-P6 still own the compact public learner-payload cutover before WP-RC5
   acceptance. WP-P2 adds `2026080908_secure_question_grading_payloads.sql`; WP-RC5 then completes
   visual authoring, all-family Memory/PostgreSQL acceptance, and the two exact Chapter 1
   assignments, while WP-RC6 closes QTI export and H5P claims.
   The accepted issued state reproduces persisted presentations, stores WeBWorK replay controls by
   rendered item ID, validates them against the owning attempt, grades with one private RPC, and
   fails closed when required immutable state is missing or mismatched. WP-P1 through WP-P6 remain
   unaccepted until their compact public cutover, browser recovery, measurements, and independent
   reviews pass.
8. After WP-RC8 migration 0909, WP-RC7 adds bounded inventory, object reconciliation,
   `2026080910_object_reconciliation.sql`, and the combined M2-M5 acceptance gate.
9. WP-RC9 implements LTI Advantage with `2026080911_lti_advantage.sql`.
10. WP-FU1 through WP-FU6 implement the server-issued learner file-upload capability in
    `docs/active_plans/active/secure_learner_file_upload_plan.md`, including
    `2026080912_secure_learner_uploads.sql`, after object reconciliation and before production
    deployment.
11. WP-RC10 adds OpenTofu under `deploy/opentofu/`; WP-RC11 adds the measured bot-cost controls.
12. WP-RC12 runs working-codebase release acceptance and documentation closure after WP-ARCH1 and
    the secure upload packages.

The pre-production schema ledger is forward-only; accepted filenames are not renamed or
reordered. SQLx owns the directory-backed ledger, with
`2026080907_course_appearance.sql` as its first forward migration. The active
pre-production migrations `2026080916_submission_receipt_presentations.sql` through
`2026080922_issued_webwork_grading_contracts.sql` implement issued presentation receipts,
successor receipts, private workspace assets, private grading envelopes, HOTSPOT grading
rebinding, and issued flat and WeBWorK grading contracts. The 2026-08-12 disposable PostgreSQL
baseline applied the complete 18-migration ledger and passed the receipt, corruption, RLS, role,
catalog, roster, timing, and current-grading oracles. Full HOTSPOT author-to-learner object-lifecycle
acceptance remains owned by WP-RC5. The fresh pre-production
identity schema already owns the canonical course-roster member model; no separate
local-roster migration or provenance exists.

### Completed package: WP-HG1 issued-receipt and live teaching acceptance

- The issuance-owned presentation and grading contract lets an issued attempt be
  presented, submitted, replayed, and advanced without consulting mutable current catalog or
  private grading state. HOTSPOT publication must retain server-owned asset identity and only
  disclose its issued public presentation binding.
- The complete migration chain through `2026080922` passed in a disposable PostgreSQL environment;
  the corresponding learner path passed with the private Podman `webwork-pg-renderer`; and the
  walkthrough retained only redacted evidence.
- The rebuilt instructor-to-student sequence, required screenshot review, all-eight learner sweep,
  and independent architecture, security, and HCI reviews passed. Remaining HOTSPOT integration is
  explicitly WP-RC5 work rather than an unrecorded WP-HG1 condition.
- WP-RC8's future email/provider and account-composition acceptance is separate. Email is not
  configured and must not gate the local walkthrough; Fastmail remains future operator work.

### Following packages: WP-RC4 closeout and secure learner payload

- WP-RC4 owner: native adapter/runtime owner plus an independent contract/security reviewer.
- Implemented behavior: PLE flat JSON v2 strictly compiles MC, MA, FIB, MULTI-FIB, NUM, MATCH,
  ORDER, and HOTSPOT into answer-free public definitions and bound grader-only keys. Exact browser
  decoders, key-free validation, learner controls, and all-or-nothing server grading cover the
  response shapes. Native content uses the v2 reader only; no current v1 single-choice
  compatibility claim remains.
- Remaining RC4 acceptance: complete invalid-fixture review, secret-free projection scan, and
  independent contract/security verdict.
- Next dependency: accept WP-P1 through WP-P6 before WP-RC5's visual authoring, integrated storage,
  and pilot-content closeout.

## Known operational notes

- The current dated snapshot is
  `docs/active_plans/reports/project_status_report_2026-08-10.md`. The Aug. 9 report and
  `partial_commit_status.md` are historical comparison/handoff records.
- The 2026-08-10 source-size follow-up passes 801 cases with no maintained-source violation. The
  paired attempt-issuance extraction preserves Store behavior and the original PostgreSQL SQL,
  bind order, transaction, and RLS boundaries; all-feature persistence checks are green.

- WP-QTI-11 started from clean `main` at `b297808`; its bounded implementation and later accepted
  work now share a mixed staged/unstaged worktree for owner review. Preserve unrelated user changes
  and do not alter the index.
- Global and scoped `git diff --check` were clean at the R4.4 handoff. Recheck the current shared
  tree before attributing any later formatting failure to this retention slice.
- No credentialed PostgreSQL retention fixture remains in the permanent suite. Fresh role/RLS,
  populated-graph, and migration-replay exercises are recorded as one-time gates and their temporary
  source is removed after execution.
- Cargo artifacts are cleaned as needed during implementation work; source files are never removed.
- Finished agent records cannot be pruned from the current collaboration history. Avoid spawning
  redundant agents; a new conversation is the only way to obtain a short agent-history list.
- Use Spark for simple, bounded independent work under
  [docs/CODEX_SPARK_SUBAGENTS.md](../CODEX_SPARK_SUBAGENTS.md); retain manager ownership
  for architecture, coordination, difficult cross-cutting decisions, and final integration.
