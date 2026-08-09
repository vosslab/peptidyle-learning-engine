# Peptidyle Learning Engine project status report

Report date: 2026-08-09  
Plan authority: [implementation_plan.md](implementation_plan.md)  
Owner decisions: [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md)  
Execution handoff: [implementation_status.md](implementation_status.md)

## Purpose and status language

This report is a formal current-state summary. It does not replace the active implementation plan
and is not a claim that the full project objective is complete. A capability is called **verified**
only when current behavior evidence covers its stated boundary. **Implemented, closure pending**
means substantial working code exists but the complete milestone exit criteria have not been
re-audited together. **Planned** means the contract or work package exists without an accepted
implementation.

The baseline Git commit is `b609430` on `main`. The accepted implementation program currently spans
a shared staged and unstaged worktree with more than 300 changed or new paths. That tree is the
authoritative implementation state, but it is not a clean release or commit boundary.

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

The last dependency-ordered QTI package accepted in full is WP-QTI-8: provenance-aware Memory and
PostgreSQL conversion from a reviewed Canvas or Blackboard item into canonical native flat source,
private grading, and a current import origin. The next package is WP-QTI-9 server route
orchestration. WP-QTI-10 through WP-QTI-12 then add the author UI, live end-to-end acceptance, and
independent closeout.

The project is not ready for production deployment. The maintained Compose stack is explicitly a
local-development system and still supplies the PostgreSQL bootstrap account to API and worker
containers. Production runtime roles, startup role attestation, gateway Content Security Policy,
embedded-mode CSRF protection, managed deployment, recovery, and load evidence remain open.

## Status dashboard

| Dimension                       | Status                                        | Current evidence and boundary                                                                                                                                                                                                     |
| ------------------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Architectural invariants        | Verified code-first                           | Crate boundaries, answer-free browser contracts, immutable publication types, tenant context, forced RLS, typed object keys, and conformance tests are present.                                                                   |
| Core learning flow              | Implemented, closure pending                  | Draft, publish, assign, issue, submit, automatic/manual grade, summary, repeat practice, feedback, prefetch, and item analysis exist; the complete M2/M3 exit matrix has not been re-audited as one package.                      |
| Native flat-question authoring  | Verified for v1 single choice                 | Canonical author GET/PUT, ETag/CAS, split public/private compilation, publication, private runtime grading, and a Solid editor are implemented.                                                                                   |
| Required flat-question families | Incomplete                                    | The accepted v1 contract supports static single choice only. MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT remain required by human guidance and need versioned work packages.                                               |
| QTI profile import              | WP-QTI-1 through WP-QTI-8 verified            | Bounded archive/XML parsing, exact Canvas/Blackboard profiles, safe reports, deterministic mapping, canonical native conversion, provenance, Memory/PostgreSQL persistence, and current private grading are accepted.             |
| QTI instructor workflow         | Planned/incomplete                            | WP-QTI-9 routes, WP-QTI-10 UI, WP-QTI-11 live gate, and WP-QTI-12 independent closeout remain. There is no public archive-upload route today.                                                                                     |
| WeBWorK                         | Implemented core, release integration pending | Private renderer client, deterministic cache/runtime boundaries, and question-local outage handling exist. The planned first content release still needs an accepted publication path and reviewed Chapter 1 fixtures.            |
| iMathAS                         | Implemented contracted boundary               | Server-brokered, immutable-source, verified-result flow exists behind explicit configuration; generic hosted execution remains refused.                                                                                           |
| H5P                             | Limited                                       | Import boundary exists for ungraded practice. A scored, server-verified execution path is not implemented.                                                                                                                        |
| PostgreSQL and data access      | Verified code-first                           | Six SQLx baseline migrations, exact ledger verification, forced RLS, broker roles, Memory/PostgreSQL conformance, retry/CAS behavior, and the disposable real-role baseline pass.                                                 |
| Object storage                  | Implemented core, reconciliation incomplete   | Typed three-bucket keys, checksums, access classes, signed-delivery limits, and private-source restrictions exist. M5 orphan quarantine and authoritative inventory reconciliation remain open.                                   |
| Retention and privacy           | Substantially implemented                     | Notify/archive/delete policy, manager API, worker cleanup, write fences, tenant purge, and anonymous-statistics survival are implemented and reviewed. Managed production recovery and full cross-cutting M5 closure remain open. |
| Browser experience              | Substantially implemented                     | Solid routes cover learner, instructor, catalog, authoring, gradebook, and flat-question flows. Course appearance themes/banner are planned but not implemented.                                                                  |
| Exports                         | Implemented core                              | Deterministic DOCX/PDF student and answer-key artifacts exist; PNG decoding is now allocation-bounded and hostile-input tested.                                                                                                   |
| Containers and operations       | Local development only                        | PostgreSQL, MinIO, API replicas, worker, gateway, and private renderer compose locally. Runtime credential separation and deployment acceptance remain open.                                                                      |
| Production deployment           | Not started/blocked                           | M6 AWS, LTI Advantage passback, managed backup/PITR, secrets, observability, scale-out, and FERPA evidence are not complete.                                                                                                      |

## Milestone posture

This table is deliberately conservative. It reports whether the complete milestone exit criteria
are proven, not whether individual modules exist.

| Milestone                   | Posture                      | Remaining proof or implementation                                                                                                                                                                    |
| --------------------------- | ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M0 foundation and toolchain | Concluded                    | Historical evidence is in `m0-results.md`; do not reopen without contradictory current evidence.                                                                                                     |
| M1 contract freeze          | Substantially realized       | Current contracts and consumers compile, but the final project completion audit must still walk every catalog row against the evolved implementation.                                                |
| M2 core lanes               | Implemented, closure pending | Major domain, grading, object, Store, schema, native, API, and client lanes exist. Re-run every M2 exit criterion together before declaring the milestone complete.                                  |
| M3 experience lanes         | Partial                      | Core UI, worker, and export lanes exist. Course appearance is not implemented; the full performance, answer-free network-trace, 31st-run, and all-route appearance exit evidence is incomplete.      |
| M4 adapter lanes            | Partial                      | Major WeBWorK, QTI, and iMathAS boundaries exist. QTI WP-QTI-9 through WP-QTI-12, complete H5P scope, and the combined adapter exit audit remain.                                                    |
| M5 integration hardening    | Partial                      | Retention, statistics, live PostgreSQL gates, replica exercise, and several cross-cutting paths exist. Orphan reconciliation and one combined hostile-input/tenancy/deletion/statistics gate remain. |
| M6 platform and deploy      | Not complete                 | LTI, analytics deployment views, AWS infrastructure, managed recovery, burst scaling, operational secrets, and compliance evidence remain.                                                           |

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

The following gates passed on 2026-08-09 after the security repairs:

- fresh `bash tests/e2e/e2e_database_baseline.sh`: all six migrations, no-op replay, exact verify,
  real-role catalog ownership, QTI provenance, current grading, manual grading, item analysis,
  partition, forced-RLS, and denial oracles;
- combined Rust package tests: QTI adapter 90 unit plus 6 corpus tests and 12 doctests, export 16,
  learning data access 84 unit plus 26 conformance and 3 doctests, server 165 unit plus main and
  doctest;
- strict Clippy for QTI, export, learning data access, and server with all targets/features;
- TypeScript compilation and 43 focused browser-contract/attempt/renderer/client Node tests;
- focused Playwright attempt-storage and route-exit behavior;
- 38 Python security and crate-boundary tests;
- 86 Markdown link tests; and
- `npm audit --omit=dev --audit-level=moderate`: zero production advisories.

An independent review reported PASS for the bounded security slice. `cargo-audit` was unavailable,
so the Rust dependency graph did not receive a current advisory-database scan. The complete
`./check_codebase.sh` gate also passed all 11 stages after generated Cargo artifacts were cleared;
the first attempt stopped only because `target/` exhausted the local disk.

## Release and production blockers

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

The current implementation is a broad shared worktree rather than a clean reviewed release commit.
An integrator must reconcile staged versus unstaged ownership, preserve unrelated user work, run the
full repository and environment gates, and produce a traceable commit series before release.

### 4. Deployment and recovery

There is no accepted production AWS deployment. Managed PostgreSQL point-in-time recovery,
object-store recovery, Secrets Manager rotation, CloudWatch evidence, replica/worker soak, burst
scaling, and the FERPA control checklist remain M6 work.

## Required feature gaps

1. **QTI author workflow:** complete WP-QTI-9 through WP-QTI-12.
2. **Question-agnostic flat format:** add versioned contracts and implementations for MA, FIB,
   MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. The current plan does not yet assign these seven
   required families concrete work packages; this is a plan gap relative to `HUMAN_GUIDANCE.md`.
3. **Course appearance:** implement the accepted 15-theme, three-color, centered-entry-banner plan
   after WP-QTI-12 releases the shared Store/client/route seams.
4. **First teaching-content release:** publish four Chapter 1 questions each for genetics and
   biochemistry: WeBWorK MC, WeBWorK MATCH, flat MC, and flat MATCH. This requires MATCH support and
   reviewed Genetics PGML corrections before it can be accepted.
5. **M5 completion:** implement bounded object inventory/orphan quarantine and run the combined
   cross-cutting security, tenancy, retention, statistics, renderer-outage, and deletion gate.
6. **M6 platform:** implement LTI, deployment, managed recovery, observability, scaling, and
   compliance acceptance.

## Dependency-ordered next work

1. Implement WP-QTI-9 upload/replay/report/convert routes over the accepted WP-QTI-8 Store boundary.
2. Implement WP-QTI-10 author UI against the stable route DTOs.
3. Run WP-QTI-11 full disposable profile-to-flat, grading, RLS, provenance, and cleanup acceptance.
4. Complete WP-QTI-12 independent review and documentation.
5. Add explicit versioned work packages for the seven remaining required flat-question families;
   land MATCH before the first Chapter 1 content release.
6. Implement the dependency-ordered course appearance package.
7. Complete the first genetics and biochemistry assignments with reviewed provenance and grading.
8. Close remaining M5 integration/reconciliation work.
9. Enter M6 deployment only after the complete M5 exit audit passes.

## Decision summary

No architecture decision is currently needed from the owner to start WP-QTI-9. Its dependencies and
acceptance criteria are already frozen. The next planning correction is also clear: after the QTI
route/UI package, the implementation plan needs concrete versioned work packages for the seven
required flat-question families, with MATCH first because it unlocks the requested initial course
content.

## Report maintenance

Update this report only when a package changes the executive assessment, milestone posture, release
blockers, or dependency order. Detailed per-package evidence remains in the focused workstream
documents and `implementation_status.md`; do not duplicate every test transcript here.
