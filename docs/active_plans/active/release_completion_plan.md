# Plan: Peptidyle Learning Engine release completion

## Status

**Current prerequisite: WP-SD1-A is pending independent ACCEPT.** PLE is one installation with global accounts, one Instructor-visible Question Library of Published Questions, private drafts, equal approved Instructors, equal Teaching Team Members, and exact CourseInstance/Student authorization. Active Questions are ordinarily selectable; inactive and archived Questions remain resolvable for history and evidence. The current SD1 registry owns the pre-production cutover before release work resumes.

The authoritative current-package and migration-allocation state is [implementation_status.md](../implementation_status.md). WP-RC1, WP-RC2, WP-RC3, WP-RC3R, WP-ARCH1, WP-UI1, WP-HG1, WP-R0, WP-R1, WP-R2, and WP-PY-L1 remain accepted where their recorded evidence says so. WP-RC4 through WP-RC12 and WP-FU1 through WP-FU6 stay open until their named gates and independent review pass.

This is the binding release authority for decisions, objectives, architecture, dependency order, acceptance/evidence, migration policy, risks, rollout, and closeout. The current package ledger is [implementation_status.md](../implementation_status.md). Update both documents when a release decision, dependency, status, or acceptance condition changes.

### Evidence classification

Apply [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md) and the permanent-test checklist to every package. Permanent tests prove stable behavior and security boundaries. Disposable service, cloud, browser, screenshot, migration, timing, and reconstruction checks prove their distinct environmental claims. One-time inventories and probes record a decision, then leave the permanent suite. Fixtures exist only for stable serialized contracts; otherwise use inline builders.

## Decisions

### In-scope decision ledger

| Topic                   | Binding decision                                                                                                                                                                                                                                                                                                                                                            | Owner                              |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| Installation and roles  | One PLE installation has global accounts. Each account has one immutable Student, Instructor, or Sysadmin role; people needing multiple roles use separate accounts. Course authority is matching exact membership and Student ownership. Sysadmin Account Creation assigns an approved Instructor Account and creates no Sysadmin membership; support is explicit and audited. | WP-SD1                             |
| Reusable courses        | A revisioned `BlueprintCourse` owns reusable ordered structure. Every `CourseInstance` has one immutable Blueprint parent and applied revision; it alone owns Students, deadlines, releases, accommodations, grades, and delivery state.                                                                                                                                    | WP-SD1-B--G                        |
| Published questions     | Stable `AAA-BBBB` `QuestionId` identifies a lineage; immutable `QuestionVersion` records hold reviewed revisions. Assignments and evidence pin exact versions and never move automatically.                                                                                                                                                                                 | WP-R2, WP-SD1                      |
| Question stewardship    | Moderate owner edits, validated exact-base Change Proposals, full private-draft forks, and audited Sysadmin ForcedQuestionCorrections preserve attribution, compatible CC licensing, history, and exact pins. UI label: **Suggest an improvement**.                                                                                                                         | WP-R2, WP-SD1                      |
| Native questions        | Closed PLE flat JSON v2 is the native source for MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. Public projections are answer-free; grading remains server-owned.                                                                                                                                                                                                  | WP-RC4, WP-RC5                     |
| Variation and grades    | New runs use `newSeeds`; resumed issued attempts retain their seed. The grade default is `highest`.                                                                                                                                                                                                                                                                         | WP-RC0, WP-PROF-T5, WP-PROF-G1--G5 |
| Retention               | CourseInstance Student records notify at 30 days, archive at 100, delete at 365; course-owned assignment definitions remain. Aggregate publication requires k >= 5.                                                                                                                                                                                                         | WP-RC0, WP-SD1                     |
| Adapters                | QTI profiles are strict and lossless or refuse. Canvas/Blackboard export is background work. H5P is ungraded practice unless translated losslessly into the protected native model.                                                                                                                                                                                         | WP-RC6                             |
| Objects                 | Database records define intended bytes; inventory proves storage. Reconciliation uses two observations and reference rechecks. A dedicated publisher alone activates immutable public copies.                                                                                                                                                                               | WP-RC7                             |
| Identity and enrollment | Email code is canonical; passkeys are optional convenience credentials on the same global account. Invitations create exact course membership and Student records atomically.                                                                                                                                                                                               | WP-RC8                             |
| LTI                     | LTI 1.3 launch and AGS passback use verified server credentials and summary-derived grades only.                                                                                                                                                                                                                                                                            | WP-RC9                             |
| Deployment and traffic  | OpenTofu owns disposable AWS infrastructure. Anonymous landing traffic terminates at static edge storage; authenticated requests have bounded cost and no client analytics.                                                                                                                                                                                                 | WP-RC10, WP-RC11                   |

### Out-of-scope decisions

Version 1 excludes content-addressed byte deduplication, a TypeScript API server, scored native H5P, local passwords, mandatory institutional SSO, client analytics, Kubernetes/Redis/Kafka/sharding, unreviewed rich-media QTI mappings, a Rust QTI Package Maker port, actual institutional credentials, and a real 10,000-Student cohort. These future possibilities do not relax release acceptance.

## Objectives and scope

Deliver one coherent automated-grading platform and canonical live production-stack journey. Grading, answer keys, correctness decisions, object authorization, and course selection remain server-owned. Browser contracts remain answer-free. Issued work and grading evidence are immutable, and Instructor inspection is audited.

The release scope is the dependency-ordered package ledger in [implementation_status.md](../implementation_status.md): WP-RC1--WP-RC12, WP-FU1--WP-FU6, WP-ARCH1, and their current-package prerequisites. It includes live delivery convergence, variation, discovery, sharing, reusable curricula, controlled updates, automated grading operations, native families, adapters/export, reconciliation, identity/enrollment, LTI, artifacts, deployment, cost controls, and final closure.

No package may turn an unresolved product decision into an implicit compatibility path. If evidence invalidates a decision, update this ledger, every affected package entry, and acceptance evidence in one reviewed planning change before code continues.

### BlueprintCourse and CourseInstance cutover

`BlueprintCourse` is the only reusable course-level aggregate. Its revision holds ordered modules, assignments, relative schedule defaults, and exact published-question pins. A one-assignment reusable unit is a one-module projection, not another type.

Each `CourseInstance` binds to one immutable Blueprint parent and applied source revision. Instantiation copies reusable meaning and resolved defaults, never Student records. Students, deadlines, releases, accommodations, grades, attempts, and delivery settings are private CourseInstance state. A Blueprint revision can make a new assignment available to descendants as unreleased; release requires an explicit Instructor decision and preserves local delivery edits.

`2026082911` owns minimal-Blueprint construction; the canonical course-creation capability invokes
it while atomically creating the bound CourseInstance and initial Instructor membership.
`2026082913` owns immutable CourseInstance adoption records and their idempotency key;
`2026082929` owns the only executable curriculum-adoption apply/reconciliation capability over those
records; `2026082930` owns forced RLS for CourseInstance roots and dependent private state.
`2026082906` owns the shared Rust
account-transaction installer. Curriculum adoption has exactly seven operations and never creates a
blank CourseInstance. An apply receives scope only from session-derived `AuthenticatedSession`; adapters
and brokers receive no client-supplied installation scope.

No current product type, route, Store capability, PostgreSQL table/function/policy, generated contract, live-demo resource, or screenshot may use Alpha as a Peptidyle product concept. Historical migrations, changelogs, and ADAPT comparison material remain evidence rather than compatibility contracts. Fresh SD1-C allocations belong only in [implementation_status.md](../implementation_status.md).

### Cutover evidence boundary

| Claim                                                                                                      | Evidence                                                                      |
| ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Aggregate normalization, revision races, propagation, and private delivery state                           | Permanent focused Rust/Memory/Node behavior and contract tests                |
| Global Question Library visibility, CourseInstance isolation, broker authority, and fresh/no-op migration | Disposable PostgreSQL/RLS acceptance                                          |
| Create, revise, publish, select, instantiate, update, and release workflows                                | Production HTTPS browser acceptance through `run_playwright_tests.sh --build` |
| Hierarchy, release state, recovery, focus, contrast, and product vocabulary                                | Rendered screenshots plus independent visual review                           |
| Complete material tree                                                                                     | `source source_me.sh && ./all_test.sh`, with every required lane passing      |

## Architecture and ownership

| Boundary                  | Owner                                                  | Rule                                                                                                                                               |
| ------------------------- | ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Product decisions         | `docs/HUMAN_GUIDANCE.md` and this plan                 | Human Guidance remains terse owner intent; settled engineering interpretation belongs here or in [DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md). |
| Public question contracts | `crates/question_model`                                | Generate answer-free TypeScript contracts.                                                                                                         |
| Source adapters           | `crates/adapters/{native,qti,webwork,h5p,imathas}`     | One strict versioned adapter per format.                                                                                                           |
| Grading                   | `crates/grading` plus server-only adapter capabilities | No browser, generated TypeScript, or Wasm grading authority.                                                                                       |
| Persistence               | `crates/learning-data-access`, `schemas/migrations`    | Memory/PostgreSQL parity; PostgreSQL is production authority.                                                                                      |
| Objects                   | `crates/objects`                                       | Typed keys, checksums, role-based delivery, inventory, reconciliation.                                                                             |
| HTTP and workers          | `crates/server`                                        | Bounded same-origin requests; durable jobs carry explicit least authority.                                                                         |
| Browser                   | `src/`                                                 | Strict decoders, accessible visible workflows, no source archive parsing.                                                                          |
| Local stack               | `local_stack_control/`, `containers/`                  | Python owns complex orchestration; shell entry points are direct facades.                                                                          |
| Deployment                | `deploy/opentofu/`                                     | Declarative, reviewable, disposable before activation.                                                                                             |

## Dependency order

```text
WP-SD1-A independent ACCEPT
  -> WP-SD1-B -> WP-SD1-C -> WP-SD1-D/E -> WP-SD1-F -> WP-SD1-G
  -> WP-RC4 -> WP-P1..WP-P6 -> WP-RC5 -> WP-RC6
  -> WP-P2 -> WP-RC7
  -> WP-RC8 -> WP-RC9 -> WP-FU1..WP-FU6 -> WP-RC10 -> WP-RC11 -> WP-RC12

Accepted foundations: WP-RC1 -> WP-RC2 -> WP-RC3 -> WP-ARCH1 -> WP-RC3R.
Accepted orchestration: WP-R1 -> WP-PY-L1. Accepted stewardship: WP-R2.
```

WP-SD1 is the release prerequisite. `WP-SD1-A-decisions-and-impact-contract` stays pending until its independent architecture/privacy review accepts it. B--G proceed in order without an Alpha bridge or parallel product path. WP-P1 may progress beside RC4 closeout, but all WP-P1--WP-P6 requirements accept before RC5. WP-P2 preserves migration allocation and transitions legacy consumers before RC7 schema work.

## Release acceptance criteria

- Every package has a named owner, actual artifact boundary, behavior, evidence class, and independent review.
- Focused mocks support code work but cannot solely accept a production route, Store, object, identity, worker, or deployment claim.
- Answers, keys, provider credentials, object credentials, and authority selection remain server-owned.
- Student workflows are keyboard complete; authoring has visible focus, status, recovery, reflow, and semantic labels; HOTSPOT has a non-pointer alternative.
- Migrations are forward-only and demonstrate fresh, no-op, checksum, RLS/grant, and live behavior evidence.
- Required skipped, unavailable, stale, or human-only evidence remains open.

## Test and verification strategy

Run the narrowest owner check, then the package gate, then release evidence. The aggregate front door:

```bash
source source_me.sh && ./all_test.sh
```

`all_test.sh` owns `check_rust.sh`, `check_codebase.sh`, repository pytest, and `local_stack.py acceptance`. The controller runs one `run_playwright_tests.sh --build` browser lane plus distinct database/RLS, course-appearance, WebWork renderer, and replica-restart service oracles. `tests/e2e/e2e_run_all.sh` is an explicit non-browser bulk E2E owner, never a second aggregate. Development SKIP output names a missing prerequisite; release evidence requires PASS.

| Evidence class          | Owner                              | Claim                                                                                     |
| ----------------------- | ---------------------------------- | ----------------------------------------------------------------------------------------- |
| Permanent offline       | `all_test.sh`                      | Stable Rust, TypeScript, Node, Python, generated-contract, security, and hygiene behavior |
| Real service acceptance | `local_stack.py acceptance`        | PostgreSQL/RLS, objects, renderer, replica, and real-browser boundaries                   |
| Production browser      | `run_playwright_tests.sh --build`  | Built bundle through HTTPS gateway and real UI-created state                              |
| Screenshot publication  | `capture_screenshots.sh`           | Rendered production-origin states when UI, Question Library, or viewport contracts change |
| One-time evidence       | Graphify and direct probes         | Narrow decision/migration/config disposition, not permanent tests                         |
| Human acceptance        | Independent review and walkthrough | Teaching workflow, accessibility, visual sense-making, legal/activation decisions         |

The final handoff records command, date, material-tree state, environment, receipt path, and limitations for every required evidence class.

## Migration policy

The shared migration ledger in [implementation_status.md](../implementation_status.md) is the only allocation registry. New schema packages receive an allocation before implementation; accepted migrations are never inserted or renamed. PLE flat JSON identity stays in its versioned source payload and immutable object/checksum binding; no family-shaped table is added. Current source and disposable test data use native v2 only. `PresentationBindingV1`, QTI profile v1, and `AAA-BBBB` Question IDs are current contracts, not compatibility shims.

## Risk register

| Risk                                                  | Owner               | Control                                                                                                                    |
| ----------------------------------------------------- | ------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Documentation substitutes for product evidence        | Release integrator  | Package acceptance requires working behavior and evidence.                                                                 |
| Adapter output leaks answers or unsafe markup         | Adapter owner       | Strict translation, sanitization, private network, browser trace.                                                          |
| New family exposes answer material                    | Family owner        | Public/private compilation, DTO scans, server-only grading.                                                                |
| Reconciliation deletes valid concurrent bytes         | Object owner        | Two observations, quarantine, reference recheck, idempotency.                                                              |
| Role/membership disagreement selects course authority | Auth owner          | One immutable account/session role, matching Student/Instructor membership, no Sysadmin membership, and origin validation. |
| Published bytes escape before commit                  | Object owner        | Transactional pending registry and dedicated publisher.                                                                    |
| External dispatch outcome is unknown                  | External-tool owner | Durable lease-bound marker and explicit operator resolution.                                                               |
| Deployment exposes secrets or broad destroy           | Deployment owner    | Secret references, unique tags, reviewed plan, bounded destroy.                                                            |
| Bot protection harms legitimate users                 | Edge owner          | Count mode, accessible recovery, versioned legitimate corpus, rollback.                                                    |
| Pilot begins before activation evidence               | Product owner       | Separate signed production-activation checklist.                                                                           |

## Rollout and closeout

Working-codebase release proves reproducible repository-owned artifacts without institutional secrets. Production activation supplies operator credentials, applies deployment, runs named live gates, completes legal review, and enrolls the pilot. Neither milestone substitutes for the other.

WP-RC12 closes only after every package in [implementation_status.md](../implementation_status.md) has required PASS evidence and independent review. It updates release evidence, documentation, implementation status, changelog, and release notes with exact receipts. Source inventories, scratch probes, and temporary diagnostics remain documented one-time evidence rather than fragile permanent tests.

Each package handoff records package ID, owner, changed files, visible/security behavior, focused/package/release checks, evidence paths, governing decisions, and independent findings.
