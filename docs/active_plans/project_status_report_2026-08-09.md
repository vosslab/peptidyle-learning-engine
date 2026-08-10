# Peptidyle Learning Engine project status report

Report date: 2026-08-09; evidence refreshed 2026-08-10
Plan authority: [implementation_plan.md](implementation_plan.md)
Release completion: [release_completion_plan.md](active/release_completion_plan.md)
Owner decisions: [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md)
Execution handoff: [implementation_status.md](implementation_status.md)

> Historical snapshot. The current report is
> `reports/project_status_report_2026-08-10.md`. This file remains unchanged as the Aug. 9
> comparison point except for this navigation note and its status-language correction.

## Purpose and status language

This report is a formal historical snapshot. It does not replace the active implementation plan
and is not a claim that the full project objective is complete. A capability is called **verified**
only when current behavior evidence covers its stated boundary. **Implemented, acceptance assigned**
means substantial working code exists and a named WP-RC package owns the integrated exit gate.
**Planned and owned** means exact artifacts, behavior, success conditions, and validation exist but
the implementation has not yet passed.

The baseline Git commit is `b297808` on `main`. WP-QTI-11 started from a clean worktree. Its bounded
implementation and later accepted work now share a mixed staged/unstaged worktree for owner review;
this is not yet a release or commit boundary.

## Executive assessment

**Overall status: advanced code-first implementation; not production-ready.**

PLE already demonstrates its central architecture:

- grading, answer keys, and correctness decisions remain server-only;
- drafts have workspace identity while publication creates immutable reusable problem versions;
- educational records are tenant-owned and protected by forced PostgreSQL row-level security;
- shared published content is not copied into each course;
- canonical source and binary artifacts use typed object keys and checksums;
- API replicas are stateless over PostgreSQL, object storage, and private capabilities; and
- the browser uses a narrow WebAssembly closure that excludes the grading crate.

WP-QTI-9 server orchestration, WP-QTI-10 author UI, and WP-QTI-11 live PostgreSQL/RLS/profile-to-
native acceptance are now complete. The real disposable path processes a mixed accepted/rejected
Canvas archive, converts and publishes the accepted item as native flat content, grades correct and
incorrect responses, verifies RLS and immutable provenance, and cleans the exact disposable project.
WP-QTI-12 independent review and documentation close-out also passed with no remaining P0/P1 issue.
The 2026-08-10 owner decision uses PLE flat JSON v2 as the internal all-family source contract,
based on the reviewed QTI Package Maker item semantics. External QTI-JSONL is no longer a native
family prerequisite. Course appearance
WP-CA1 through WP-CA7 and WP-RC1 are also accepted: the safe Rust/generated-TypeScript contract, executable
instructor route, protected typed banner objects, revisioned Memory/PostgreSQL persistence, bounded
JPEG/PNG/WebP normalization, atomic no-store HTTP operations, exact-current delivery, and cleanup
are complete. Grass is now the default, and the 15-theme Solid scope passes fail-closed decoding,
cross-course/global cleanup, run-data reuse, and rendered contrast. The keyboard-complete instructor
settings workflow, exact course-entry-only banner, all-seven-route browser traversal, database
current-pointer guard, combined PostgreSQL/MinIO cleanup lifecycle, responsive/forced-color visual
evidence, and three independent no-P0/P1/P2 reviews passed. WP-RC2 is also accepted: concrete H5P,
QTI, and WeBWorK module names now replace production seam labels; catalog resolve/search are explicit
Store capabilities; and the durable feedback-release state is the sole current projection input.
Focused/package-wide gates and independent review passed. WP-RC3's bounded shipped-upstream
implementation now also passed the required live OCI build, PLE gateway, cache, grading,
outage-isolation, recovery, and keyboard-browser evidence on 2026-08-10. WP-ARCH1 then closed its
dated 26-file maintained-source baseline with zero maintained-code violations behind stable facades.
Its permanent size gate (582 tests), 2,451-test Python suite, eleven-stage codebase gate, and
72-pass browser suite are green. Independent PostgreSQL, security, provider, TypeScript/HCI, test,
size-policy, and final architecture reviews found no unresolved P0/P1 issue; the final RC3 review
accepted the bounded WeBWorK package.

The project is not ready for production deployment. The maintained Compose stack is explicitly a
local-development system and still supplies the PostgreSQL bootstrap account to API and worker
containers. Production runtime roles, startup role attestation, gateway Content Security Policy,
embedded-mode CSRF protection, managed deployment, recovery, and load evidence are assigned to
WP-RC8 through WP-RC12.

## Status dashboard

| Dimension                       | Status                                      | Current evidence and boundary                                                                                                                                                                                                                                                                                 |
| ------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Architectural invariants        | Verified code-first                         | Crate boundaries, answer-free browser contracts, immutable publication types, tenant context, forced RLS, typed object keys, and conformance tests are present.                                                                                                                                               |
| Core learning flow              | Implemented, acceptance assigned            | Draft, publish, assign, issue, submit, automatic/manual grade, summary, repeat practice, feedback, prefetch, and item analysis exist; WP-RC7 owns the combined M2-M5 exit matrix.                                                                                                                             |
| Native flat-question authoring  | v1 visual editor; v2 source/runtime implemented | Canonical author GET/PUT, ETag/CAS, split public/private compilation, publication, and private runtime grading exist. The Solid author editor remains v1 single choice.                                                                                                                                    |
| Required flat-question families | Runtime core implemented; acceptance open  | PLE flat JSON v2 strictly compiles, renders, validates, and server-grades MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. WP-RC5 retains visual authoring, all-family integrated storage/object acceptance, and pilot content.                                                                     |
| QTI profile import              | WP-QTI-1 through WP-QTI-12 verified         | Bounded parsing, exact Canvas/Blackboard profiles, safe reports, mapping, native conversion, provenance, Memory/PostgreSQL persistence, routes, author UI, live grading, RLS, cleanup, and independent close-out are accepted.                                                                                |
| QTI instructor workflow         | Implemented and independently accepted      | Upload/report/convert routes and the existing-route author UI passed the disposable profile-to-native PostgreSQL gate and six-pass independent review.                                                                                                                                                        |
| WeBWorK                         | Bounded RC3 path accepted                   | WP-RC3 replaces the invented `/v1` dialect with the pinned upstream authenticated `/render_rpc` form protocol, server-only radio projection, private MariaDB profile, and launcher path. The live PLE E2E, keyboard browser proof, WP-ARCH1 boundary, and final review pass. Broad OPL compatibility and MATCH remain separate scope. |
| iMathAS                         | Implemented contracted boundary             | Server-brokered, immutable-source, verified-result flow exists behind explicit configuration; generic hosted execution remains refused.                                                                                                                                                                       |
| H5P                             | Honest limited capability                   | Native H5P is ungraded practice; WP-RC6 closes lossless import into protected native families. Scored native H5P is explicitly out of scope because browser evaluation cannot satisfy server-only grading.                                                                                                    |
| PostgreSQL and data access      | Verified code-first                         | Six SQLx baseline migrations plus the course-appearance forward migration, exact ledger verification, forced RLS, broker roles, Memory/PostgreSQL conformance, retry/CAS behavior, exact current-pointer ownership, and the disposable real-role pass.                                                        |
| Object storage                  | Implemented core, WP-RC7 assigned           | Typed three-bucket keys, checksums, access classes, signed-delivery limits, and private-source restrictions exist. WP-RC7 owns inventory, twice-observed orphan quarantine, broken-reference alerts, and the combined M5 gate.                                                                                |
| Retention and privacy           | Substantially implemented                   | Notify/archive/delete policy, manager API, worker cleanup, write fences, tenant purge, and anonymous-statistics survival are implemented and reviewed. WP-RC7 owns M5 closure; WP-RC10/WP-RC12 own managed recovery evidence.                                                                                 |
| Browser experience              | Substantially implemented                   | Current student routes and response families passed a focused no-mouse audit. All seven course-owned routes use the measured 15-theme scope with Grass as default; instructor banner/theme settings and entry-only learner identity passed WP-RC1.                                                            |
| Exports                         | Implemented core                            | Deterministic DOCX/PDF student and answer-key artifacts exist; PNG decoding is now allocation-bounded and hostile-input tested.                                                                                                                                                                               |
| Containers and operations       | Local development; RC3 live path passed     | The root launcher bootstraps credentials, migrates, seeds, starts, health-checks, and opens the browser. Its optional private WeBWorK profile now passes the full local Podman 6 build and PLE/browser acceptance. WP-RC10 owns production credential separation/deployment; WP-RC12 owns release acceptance. |
| Production deployment           | Planned and owned                           | WP-RC8 owns OIDC, WP-RC9 LTI, WP-RC10 OpenTofu/AWS/backup/PITR/secrets/scale, WP-RC11 edge cost controls, and WP-RC12 release evidence. Institutional credentials and legal sign-off are external activation gates.                                                                                           |

## Milestone posture

This table is deliberately conservative. It reports whether the complete milestone exit criteria
are proven, not whether individual modules exist.

| Milestone                   | Posture                      | Remaining proof or implementation                                                                                                                                                                     |
| --------------------------- | ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M0 foundation and toolchain | Concluded                    | Historical evidence is in `m0-results.md`; do not reopen without contradictory current evidence.                                                                                                      |
| M1 contract freeze          | Substantially realized       | Current contracts and consumers compile, but the final project completion audit must still walk every catalog row against the evolved implementation.                                                 |
| M2 core lanes               | Implemented, WP-RC7 assigned | Major domain, grading, object, Store, schema, native, API, and client lanes exist. WP-RC7 runs every M2-M5 criterion together.                                                                        |
| M3 experience lanes         | Partial, appearance accepted | Core UI, worker, export, and course appearance lanes exist. WP-CA1 through WP-CA7/WP-RC1 are accepted; the remaining seven flat-family author/learner surfaces and pilot content are owned by WP-RC5. |
| M4 adapter lanes            | Partial                      | Major WeBWorK, QTI, and iMathAS boundaries exist. Complete H5P scope and the combined adapter exit audit remain.                                                                                      |
| M5 integration hardening    | Partial, WP-RC7 assigned     | Retention, statistics, live PostgreSQL gates, replica exercise, and several cross-cutting paths exist. WP-RC7 owns reconciliation and the combined hostile-input/tenancy/deletion/statistics gate.    |
| M6 platform and deploy      | Planned and owned            | WP-RC8 through WP-RC12 own OIDC, LTI, OpenTofu, aggregate observability, managed recovery, burst scaling, operational secrets, bot-cost controls, and release evidence.                               |

## Verified architecture and product capabilities

### Learning and grading

- Fresh server-owned seeds support repeated algorithmic practice; an existing attempt preserves its
  seed only for resume, replay, and audit.
- The browser validates response shape but receives no answer key or grading implementation.
- Automatic and response-bearing manual grading publish through generation fences.
- Completion, score policy, continued practice, variation, and feedback disclosure remain separate
  controls.
- Current course item analysis is instructor-only, tenant-owned, aggregate, and identity-free.

### Identity, tenancy, and publication

- A private draft has a workspace identity and no catalog identity.
- Successful publication creates an immutable `ProblemId`/`VersionId`; revisions retain the problem
  and create a new version.
- Shared published versions can be reused across tenants without copying content.
- Course and learner records remain tenant-owned. The 2026-08-09 security pass closed a
  same-`UserId` cross-tenant catalog lifecycle edge by requiring tenant-qualified problem ownership
  in Memory, SQLx, and PostgreSQL RLS.

### Source, objects, and provenance

- Native flat source is canonical, checksummed, non-signable, and compiled into separate public and
  private grading values.
- QTI source archives and imported item provenance are retained through typed, non-signable object
  identities.
- Published assets and protected student records use separate delivery policies and lifetimes.
- Archive parsing rejects traversal, absolute paths, backslashes, NULs, dot components, symlinks,
  duplicate entries, expansion excess, DTDs, and entities.
- Generic QTI now rejects active SVG. PDF export validates PNG structure, dimensions, CRCs, and
  bounded decompression before allocation.

### Frontend and API security

- Authentication uses an opaque host-only HttpOnly cookie; the browser does not store a bearer
  token.
- Learner response recovery uses `sessionStorage`, not `localStorage`, and clears the active buffer
  on run exit.
- File-upload submissions fail closed until a server-issued, tenant/learner/attempt-bound upload
  capability exists.
- API health reruns exact migration/checksum compatibility rather than treating database reachability
  as readiness.
- Request-driven SQL uses static SQLx statements with bound values; the focused security pass found
  no classic SQL-injection path.

## Current verification evidence

The following WP-QTI-11 gates passed on 2026-08-09:

- fresh `bash tests/e2e/e2e_database_baseline.sh`: all six migrations, no-op replay, exact verify,
  mixed accepted/rejected profile import, native conversion/publication, correct and incorrect
  grading, archive/provenance agreement, real-role RLS denial, retention, and exact-project cleanup;
- focused QTI adapter, native-flat, Store conformance, server-profile, and Node tests, plus all four
  QTI author Playwright scenarios;
- strict workspace Clippy, formatting, TypeScript compilation, and `cargo test --workspace`;
- `./check_codebase.sh`: all 11 stages passed;
- rebuilt full Playwright suite: 51 passed; and
- repository Python suite: 1,644 passed.

The full gate found and closed two test-source defects without widening production behavior. Unicode
boundary fixtures now use ASCII-only Rust and PostgreSQL escapes, and the feedback-flow browser
fixture submits a native multiple-choice response instead of violating the strict external-tool
boundary. Six WP-QTI-12 review passes then corrected stale README and ownership-map documentation;
focused documentation gates and original-reviewer rechecks passed with no remaining P0/P1 issue.
Detailed evidence is in
`docs/active_plans/workstreams/qti_live_acceptance_implementation.md`.

WP-CA3 then passed its Memory conformance, strict PostgreSQL-feature Clippy, and complete disposable
PostgreSQL 17 gate. The seventh migration applied and replayed cleanly; the live oracle proved
default creation, manager/student/foreign authorization, stale and successful CAS, bytes-first
promotion tracking, exact-current delivery, membership revocation, and cleanup that preserves the
current banner. All pre-existing database oracles and role matrices remained green, as did all 11
repository checks, 1,654 Python tests, and 629 focused documentation checks.

WP-CA4 then passed 190 server tests, strict server and PostgreSQL-feature Clippy, the complete
disposable PostgreSQL/RLS gate, and all 11 repository checks. The server now normalizes only bounded
JPEG/PNG/WebP input to one metadata-free 1200 by 328 WebP, keeps the hidden future banner identity
out of the browser, applies appearance with strong revision CAS, and refuses candidate,
superseded, outsider, or foreign delivery while preserving the old appearance on every tested
failure.

WP-CA5 then passed its focused Rust/Node checks, complete 56-case built Playwright suite, and all 11
repository checks. The browser maps every closed theme ID, defaults new and migrated courses to
Grass, scopes CSS variables below the persistent shell, reuses attempt and summary route data, and
clears the prior course on cross-course or global navigation. Rendered measurements prove the house
5.5:1 text/action target and 3:1 focus/boundary target for every theme; raw Grass `#008852` remains a
decorative anchor while accessible action and link colors are derived.

The later owner-requested usability and local-testing package also passed its focused and full
gates. Current PLE-owned student routes and response families have a keyboard-only path using Tab,
arrow keys, Space, Enter, and Escape as appropriate; representative screen-reader sessions remain a
fall-pilot human gate. `launch_local_stack.sh` is the maintained build/start/health/browser front
door; it now bootstraps ignored local credentials, migrates before API/worker startup, provisions
the grader login, seeds a demonstration course, and leaves the unconfigured WeBWorK profile off.
[docs/DATABASE_STRUCTURE.md](../DATABASE_STRUCTURE.md) separates implemented relations from the
WP-RC8 OIDC identity mapping and maps the fall pilot through ten-million-question growth. Passkeys,
local passwords, and email-code login are out of version 1; institutional credentials and FERPA
legal sign-off remain production-activation evidence.

### WP-RC3 shipped WeBWorK: bounded path accepted

WP-RC3 has a concrete, bounded implementation owned by the Rust adapter and server owners, the
container/launcher integrator, and an independent security reviewer. The adapter files
`crates/adapters/webwork/src/{http_renderer,renderer_contract,shipped_render_rpc}.rs` replace the
invented renderer endpoints with one authenticated upstream `/render_rpc` form route for both render
and grade. They accept only the documented default JSON shape, discard protected fields, sanitize
the projected body, derive answer-free opaque choice IDs from immutable version/seed bytes, and
re-render server-side to reconstruct the radio field/value only for grading. The server composition
reads the render password from a file rather than a browser-visible or environment-value secret.

The integrator owns `containers/compose.webwork.yaml`, `containers/webwork/`,
`launch_local_stack.sh`, and the RC3 tests. The optional profile fetches exact detached WebWork2 and
PG revisions, provides a dedicated private MariaDB and render course, has no host ports for either
service, and keeps the renderer/database off PLE's PostgreSQL, MinIO, gateway, and browser
networks. The immutable licensed RadioButtons fixture and provenance sidecar live under
`content/pilot/webwork/`; `tests/e2e/e2e_webwork_render_rpc.sh` and
`tests/playwright/webwork_run.spec.ts` are the required PLE-level acceptance artifacts.

The live gate now proves what the earlier static evidence could not. On Podman 6, the launcher built
the exact pinned WebWork2 and PG sources, initialized the private render course, authenticated the
semantic `/render_rpc` probe, seeded the immutable PGML source, and served it through the PLE gateway.
`tests/e2e/e2e_webwork_render_rpc.sh` proved one renderer call followed by same-attempt cache hits,
full and zero grading, idempotent replay, renderer-outage containment while gateway health stayed
available, recovery, and absence of protected source, credential, hidden-field, or answer-mapping
data. Its required Playwright invocation passed three tests, including keyboard-only operation and a
PLE-origin-only structural trace.

The focused adapter suite passes 25 tests, server core passes 189 with three explicitly disposable
live fixtures ignored, project tools passes 29, and the focused container/launcher/topology/shebang
set passes 657. WP-ARCH1 keeps those behavior boundaries while moving complete capabilities behind
stable facades. Formatting, strict Clippy, TypeScript compilation, all eleven
`./check_codebase.sh` stages, 2,451 Python tests, 184 Node tests, and 72 Playwright tests pass; the two
deliberately opt-in browser cases skip. The permanent source-size gate passes 582 tracked-file cases,
and the untracked-aware inventory reports zero maintained-code violations. Its exact exception file
contains only frozen migrations and documentation/history artifacts. The disposable PostgreSQL
baseline also passes migration replay/checksum refusal, serialization retry, concurrent claims,
260,000-attempt partition pruning, bounded summaries, private grading, QTI conversion/provenance,
manual-grading fences, and four-role RLS denial through the decomposed owners. Independent WP-ARCH1
and final RC3 reviews found no unresolved P0/P1 issue, so both packages are accepted.

RC3 deliberately excludes broad OPL compatibility, arbitrary PG control types, browser-to-WebWork
calls, CORS/public renderer routes, upstream gradebook/LTI passback, insecure RPC modes, and mutable
registry-tag adoption. The single immutable, user-authored RadioButtons fixture is sufficient to
prove this first shipped MC path without claiming those broader capabilities. WP-RC5 owns typed
WeBWorK MATCH support and the Chapter 1 content release; WP-RC9 owns LTI; no excluded item is a
hidden dependency of the RC3 acceptance sequence.

## Assigned release and production packages

### 1. Runtime PostgreSQL identity

The local Compose file gives API and worker containers the bootstrap PostgreSQL credential.
Production needs separate migration and runtime URLs, non-superuser/non-`BYPASSRLS` API and worker
logins, narrow role memberships, and eager startup attestation. A compromised runtime container
must not inherit migration or RLS-bypass authority.

### 2. Browser security headers and embedded CSRF

The production gateway must compose a restrictive Content Security Policy and related security
headers. Before `SameSite=None` embedded/LTI mode is enabled, state-changing requests need an
origin-bound anti-CSRF mechanism in addition to launch state, nonce, and replay validation.

### 3. Integrated release boundary

WP-QTI-11 started from clean `main`, and WP-QTI-12 independently accepted its bounded changes, but
the working tree is not yet an owner-approved traceable commit or release boundary.

### 4. Deployment and recovery

WP-RC10 owns the OpenTofu AWS deployment, managed PostgreSQL point-in-time recovery, object-store
recovery, Secrets Manager references/rotation, aggregate observability, replica/worker soak, and
burst scaling. WP-RC12 owns the technical FERPA control evidence; institutional legal sign-off is a
named production-activation action.

## Required feature gaps

1. **Flat-family acceptance:** WP-RC4 closes independent review of the implemented PLE flat JSON v2
   source/runtime contract. WP-RC5 adds visual authoring, all-family PostgreSQL/object acceptance,
   and pilot content around MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT.
2. **First teaching-content release:** publish four Chapter 1 questions each for genetics and
   biochemistry: WeBWorK MC, WeBWorK MATCH, flat MC, and flat MATCH. This requires MATCH support and
   the exact reviewed genetics and biochemistry sources named in WP-RC5.
3. **M5 completion:** implement bounded object inventory/orphan quarantine and run the combined
   cross-cutting security, tenancy, retention, statistics, renderer-outage, and deletion gate.
4. **M6 platform:** WP-RC8 through WP-RC12 implement OIDC, LTI, OpenTofu deployment, managed
   recovery, aggregate observability, scaling, edge cost controls, and release evidence.

## Dependency-ordered next work

1. Complete WP-RC4's independent flat JSON v2 contract/security closeout.
2. Close WP-RC4 review and accept WP-P1 through WP-P6 before WP-RC5 acceptance. Execute WP-RC5
   visual authoring, all-family integrated acceptance, Chapter 1 content, then WP-RC6 QTI export and
   H5P close-out.
3. After WP-P2, execute WP-RC7 object reconciliation and the combined M2-M5 gate.
4. Execute WP-RC8 through WP-RC12 for OIDC, LTI, OpenTofu, bot-cost controls, and release acceptance.

## Decision summary

The decision-complete scope is in
`docs/active_plans/active/release_completion_plan.md`. PLE flat v1 remains closed and byte
compatible; PLE flat v2 is the internal all-family source contract; one compiler isolates source
evolution from runtime consumers; media and HOTSPOT retain assigned object/interaction work. A
future QTI-JSONL format is an external adapter concern, and a wholesale Rust port of QTI Package
Maker remains out of scope.

## Report maintenance

Update this report only when a package changes the executive assessment, milestone posture, release
blockers, or dependency order. Detailed per-package evidence remains in the focused workstream
documents and `implementation_status.md`; do not duplicate every test transcript here.
