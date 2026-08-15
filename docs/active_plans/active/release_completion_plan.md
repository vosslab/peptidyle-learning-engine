# Plan: Peptidyle Learning Engine release completion

## Status

Planning state: implementation in progress on 2026-08-10. WP-RC1 course appearance, WP-RC2
production seams, WP-RC3 bounded WeBWorK proof, WP-RC3R standalone renderer replacement, and
WP-ARCH1 source ownership are accepted; this plan owns the remaining version 1 work and supplements
[implementation_plan.md](../implementation_plan.md). WP-UI1 interface composition is accepted.
WP-RC8's generic identity/passwordless/roster routes and provider-free production composition exist,
but a live SMTP send, optional-passkey/multi-replica evidence, and independent acceptance remain;
that acceptance is the next dependency before WP-RC4 resumes. Secure payload closeout and WP-RC5
visual-authoring/all-family integration follow in dependency order.

**Current bounded evidence.** WP-HG1 is accepted through permanent repository gates plus recorded
clean-stack, learner-sweep, and disposable PostgreSQL acceptance. Email activation, Fastmail, and
HOTSPOT lifecycle remain outside that result. WP-RC5 has the Chapter 1 MC/MATCH matrix and browser
sweep; visual authoring, all-family persistence, media, screen-reader, and review remain open.

**2026-08-12 security alignment.** Four storage domains, queued public assets with a dedicated
publisher, actor-scoped learner authority, fenced external tools, protected POST delivery, and the
OpenTofu baseline are code-complete. WP-RC10 needs live AWS/RDS, restore, and renderer evidence.

Completed packages remain accepted. Completion requires working production artifacts, passing
behavior/security gates, current docs, and no P0/P1; stubs, mocks, disabled tests, and TODOs do not.

Two release boundaries are explicit: the **working-codebase release** completes reproducible
repository-owned artifacts without institutional secrets; **production activation** supplies
credentials, applies deployment, runs named live gates, completes legal review, and enrolls the
pilot. External activation cannot hide unfinished repository work.

### Evidence classification for every work package

Apply the [test evidence model](../../TEST_EVIDENCE_MODEL.md) and permanent checklist to every `Validation` item.
Keep small deterministic behavior/contract checks permanent. Source inventories, config inspection, migrations,
screenshots, timings, and reconstruction probes are one-time scratch evidence. Real service checks remain opt-in
E2E/live. Fixtures require a stable serialized contract; otherwise use inline builders.

The active professor-capability roadmap applies this boundary to WP-R2. Offline Memory conformance belongs to
`crates/learning-data-access/tests/conformance/publication.rs` and `assignments.rs`; disposable PostgreSQL/RLS
persistence and role claims to `tests/e2e/e2e_wp_r2_postgres_rls.py`; manufactured manifest convergence to
`crates/project-tools/src/e2e_seed/tests.rs`; and the disposable real host-seed/renderer claim to
`tests/e2e/e2e_wp_r2_host_seed_renderer.py`. `tests/test_assignment_editor_ui.mjs` owns authored mock
decoder/client/editor behavior, while `tests/playwright/assignment_editor.spec.ts` owns its mock-backed visible
replacement behavior. `local_stack_control/acceptance_lanes.py` is the sole live browser replacement route and
`tests/walkthrough/run_ui_walkthrough.py` the sole M6 composition journey. WP-R2 creates no fixture directory,
predicts no assigned live Question ID, count, or hash, and records inventories, screenshots, and timing only in
`_temp_professor_roadmap_20260814/wp_r2_closeout.review.md`. `generated/api/` remains ignored derivative output of
`crates/project-tools/src/tsgen.rs`.

## Decisions

### In-scope decision ledger

| Topic                     | Decision for version 1                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Owning package        |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- |
| Flat family runtime       | PLE flat JSON v2 implements MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT with protected visual authoring; RC5 still owns all-family integrated acceptance, HOTSPOT lifecycle closure, and pilot content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | WP-RC4/RC5            |
| Flat source               | Use closed PLE flat JSON v2 as the only native reader for all eight families, based on reviewed QTI Package Maker item semantics; external QTI-JSONL remains a separate adapter concern                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | WP-RC4                |
| Grade default             | `highest`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | WP-RC0                |
| New-run variation         | `newSeeds`; resuming an issued attempt preserves its seed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | WP-RC0                |
| Retention defaults        | Notify at 30 days, archive at 100 days, delete learner records at 365 days, and publish aggregates only at k >= 5                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | WP-RC0                |
| Course deletion           | Retain assignment definitions by default; delete learner records and student-record objects                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | WP-RC0                |
| Operational payloads      | Keep normalized source/public/private payloads in PostgreSQL only within their existing hard ceilings; refuse an oversized write rather than silently moving a hot-path model                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | WP-RC0                |
| WeBWorK source            | Copy the exact licensed user-authored `content/pilot/webwork/which_hydrophobic-simple.pgml` fixture and provenance sidecar into immutable PLE object storage at publication; attempts never depend on a mutable OPL checkout                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | WP-RC3                |
| WeBWorK protocol          | Preserve RC3's bounded server-only projection and grading proof through the current private external `webwork-pg-renderer` `/render-api`; WebWork2, render-course credentials, and MariaDB stay outside the runtime                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | WP-RC3/RC3R           |
| Course banner timing      | Candidate expires after 60 minutes; a protected course-banner delivery grant lasts at most 60 minutes and rechecks the exact current pointer                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | WP-RC1                |
| QTI profile v1            | Canvas 1.2 and Blackboard 2.1 remain strict static single-choice profiles; unsupported media, feedback, `sub`, and `sup` refuse without loss                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | WP-RC6                |
| QTI profile export        | Canvas and Blackboard export run as background jobs and appear in the author UI only as queued status plus a protected download                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | WP-RC6                |
| H5P                       | Serve native H5P only as ungraded practice and import supported static families into the protected native model for grading                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | WP-RC6                |
| Object lifecycle          | Database records define intended existence; bucket inventory proves bytes; reconciliation quarantines twice-observed orphans and alerts on missing referenced bytes. Publication queues a pending private source transactionally; only the dedicated publisher may write and activate the immutable tagged public copy.                                                                                                                                                                                                                                                                                                                                                                                                         | WP-RC7                |
| Production identity       | PLE-owned global `UserId`; short-lived single-use email authentication is the canonical sign-in path, discoverable passkeys are optional convenience credentials, and optional institutional OIDC/SAML links to an existing account behind `IdentityProvider`                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | WP-RC8                |
| Enrollment                | Invite by email; retain course-scoped roster email, institutional roster ID, and display label for enrollment/manual grade export; enforce optional exact-domain policy; atomically create course membership plus all assignment enrollments/summaries. Every learner-scoped Store read or mutation rechecks active Student membership under the same authority/locking boundary, so revocation takes effect across stale browser identifiers and races.                                                                                                                                                                                                                                                                        | WP-RC8                |
| LTI                       | Implement LTI 1.3 launch plus Assignment and Grade Services passback as a separate verified credential path                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | WP-RC9                |
| Learner file upload       | Use a server-issued attempt-bound upload record, non-deliverable temporary storage, closed inspection worker, SHA-256, atomic manual-submission consumption, and protected student-record delivery                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | WP-FU1..WP-FU6        |
| Infrastructure            | Use OpenTofu in `deploy/opentofu/`; production is AWS Fargate (API, worker, and dedicated public-asset publisher), RDS PostgreSQL, four S3/KMS storage domains, CloudFront, ALB, WAF, Secrets Manager, VPC endpoints, and private networking. The renderer is externally managed and disabled until separately attested.                                                                                                                                                                                                                                                                                                                                                                                                        | WP-RC10               |
| Anonymous traffic         | Ship a static `www` landing origin, same-origin authenticated app/API, aggregate edge metrics, bounded WAF/rate rules, and no client analytics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | WP-RC11               |
| Migration names           | Continue compact ordered names such as `2026080908_secure_question_grading_payloads.sql`; the date and two-digit sequence are the readable ordering contract                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | WP-P2                 |
| Source ownership          | After the WP-RC3 live gate and before WP-RC4, extract every maintained source at 1,000 lines or more into capability modules behind stable facades; add a permanent no-exception size gate                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | WP-ARCH1              |
| Course visual default     | `grass`, with the accepted 15-theme catalog and one 1200 by 328 WebP center crop                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | WP-RC1                |
| Release content           | Genetics and biochemistry Chapter 1 each ship four questions: WeBWorK MC, WeBWorK MATCH, flat MC, and flat MATCH                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | WP-RC5                |
| Human question references | The assigned `AAA-BBBB` Question ID is the sole durable question identity and names one random, non-sequential, immutable published question. No UUID, sequence number, or version number is shown. Every content change receives a new Question ID plus fresh opaque hidden `ProblemId`/`VersionId` evidence with optional immutable provenance; existing assignments and runs remain exact until an Instructor explicitly, revision-checked replaces an item. Legacy `ProblemPublicId`/`P-...`, `ProblemVersionNumber`, predecessor/version-chain, and tenant-derived host-seed question UUIDs are removed by WP-R2. Hidden immutable snapshots remain only for authorized replay, grading, audit, provenance, and transport. | WP-HG1, WP-UI1, WP-R2 |
| Interface composition     | Standard presentation is palette-faithful at a 5.5:1 ordinary-text target; increased contrast is optional and account-backed. Instructor work targets 1280 by 800, student work remains responsive, and typed public route references replace UUID navigation without changing authorization.                                                                                                                                                                                                                                                                                                                                                                                                                                   | WP-UI1                |

`WP-RC0` is a decision-freeze documentation package completed by this plan. It updates the plan,
status, and durable owner guidance; the behavior defaults already present in source remain gates for
later integrated acceptance.

### Out-of-scope decisions

| Excluded from version 1                                                                         | Why version 1 succeeds without it                                                                                                                                                                                                                     |
| ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Content-addressed byte deduplication                                                            | Stable typed keys, checksums, immutable writes, and reconciliation already provide correctness. Deduplication is a storage-cost optimization that can be added behind `ObjectStore` after measured duplication warrants it.                           |
| A TypeScript API server                                                                         | Native `axum` is implemented, tested, and owns the request path. Reopening the runtime would delay release without adding product behavior.                                                                                                           |
| Scored native H5P                                                                               | H5P exposes evaluation to the browser. Server-graded native and imported flat questions supply the secure graded path.                                                                                                                                |
| Local passwords and mandatory institution-controlled SSO                                        | Passwordless PLE accounts avoid password storage and institution coordination. Optional OIDC/SAML account linking must not replace or fork the PLE account.                                                                                           |
| Third-party or client-side analytics                                                            | Edge and server aggregate metrics answer cost and reliability questions without adding student tracking or anonymous API work.                                                                                                                        |
| Kubernetes, Redis, Kafka, sharding, a dedicated search service, and multi-region operation      | The target pilot and 10,000-student scale fit stateless replicas, PostgreSQL, object storage, and a worker queue. Every replacement retains a measured trigger in the architecture plan.                                                              |
| Rich media in the accepted vendor-profile v1 import                                             | The strict importer succeeds by refusing unsupported semantics without data loss. PLE flat JSON v2 HOTSPOT provides the bounded native rich-media path.                                                                                               |
| Vendor feedback, `sub`, or `sup` in profile v1                                                  | No accepted fixture establishes a lossless mapping. Refusal preserves correctness; a new profile version can add one exact fixture-backed mapping later.                                                                                              |
| Broad local-corpus compatibility statistics                                                     | Minimized positive and near-miss fixtures plus live profile acceptance prove the claimed contract. Corpus sampling may guide later profile versions but does not widen v1.                                                                            |
| A Rust port of QTI Package Maker                                                                | QTI Package Maker remains the Python interoperability oracle. PLE ports only the versioned parser/compiler behavior needed at runtime.                                                                                                                |
| Actual institutional credentials, FERPA legal certification, or participation by named students | The repository can prove technical controls with synthetic identities and disposable infrastructure. Credential issuance, legal sign-off, and human participation require the institution and are production-activation evidence, not code artifacts. |
| Ten million real production questions or a live 10,000-student cohort                           | Synthetic partition, query-plan, concurrency, queue, and restore gates prove the design before exposing real records. Real growth is an operational outcome, not a prerequisite for a correct release.                                                |
| Learning trees, discussions, clickers, LMS roster sync, research exports, and generated content | Version 1 succeeds as an assignment, question, attempt, feedback, grade, import/export, and retention platform. These features do not block that learning loop.                                                                                       |

## Objectives

Finish repository behavior and the fall-pilot path without special cases. Preserve server-only grading,
immutable publication, forced-RLS tenancy, replica safety, and answer-free browser contracts. Make
first success one command; close each package with evidence, current docs, and independent review.

## Scope

The version 1 scope is WP-RC1 through WP-RC12 in dependency order. It includes course appearance,
production-seam cleanup, shipped WeBWorK integration, PLE flat JSON v2, all eight flat
families, pilot content, honest QTI/H5P boundaries, object reconciliation and M5 integration,
passwordless identity, invite-by-email enrollment, optional institutional SSO, LTI Advantage,
secure learner file uploads, OpenTofu deployment, bot-cost controls, and final release acceptance.

No package may defer a required file or behavior to a later implementer. If implementation evidence
invalidates a decision, the owner updates this decision ledger, every consumer package, and the
acceptance gate in one reviewed planning patch before code continues.

**2026-08-14 no-drift acceptance.** WP-R2 is accepted after its final Validation. The Question ID
workflow now uses one immutable publication per assigned ID: owner-correction propagation, sequential
public identity, and version-chain resolution are removed from the pre-production schema and code.
Hidden exact internal references remain authorized replay, grading, audit, provenance, and transport
evidence, not a user-supplied locator or latest-resolution feature. Native and WeBWorK host seed
publishers mint fresh opaque question IDs and use protected-manifest or verified-existing-record rerun
recovery; deterministic fixed IDs remain limited to isolated unit fixtures, derived render/cache
identities, and non-question seed records.

**2026-08-14 orchestration dependency.** Python owns stateful subprocess, Podman, private-file,
polling, cleanup, and lane aggregation. WP-R1 is accepted: Chapter One pilot/browser and aggregate
acceptance lanes run in Python over existing typed `local_stack_control`, and the designated renderer
image name selects the rebuild target while each live run records its OCI configuration ID. M0 remains
open; WP-PY-L1 is accepted on 2026-08-15 after final offline/live Validation and three independent
reviews with no P0-P3 finding. It replaces `local_stack_control/launch.sh`,
`_restart.sh`, and `containers/local_identity_bootstrap.sh` all at once before M1; it deletes the shell
implementation instead of shipping a Python wrapper or dual launcher. M1 retains its separate declared
dependency gates. The next current package is WP-RC8 acceptance;
later packages may migrate
remaining E2E, developer, renderer-probe, and destructive-cleanup scripts.

The remaining complex-script order is explicit: after WP-PY-L1, the canonical WeBWorK
renderer/host-seed acceptance owner converts its stateful runner; the release-candidate composition
owner follows. Python owns state, parsing, subprocesses, private inputs, polling, and cleanup in each
conversion, with a direct shell `exec` facade where a shell entry remains.

## Non-goals

The out-of-scope ledger above is exhaustive for known exclusions. An implementer may not create a
new non-goal to avoid an acceptance criterion. New product ideas enter a separate post-v1 plan only
after WP-RC12 closes.

## Architecture and ownership

| Boundary                           | Authoritative owner                                             | Rule                                                                                                                                                                                          |
| ---------------------------------- | --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Product decisions and defaults     | `docs/HUMAN_GUIDANCE.md` plus this ledger                       | Code may expose configuration but must ship the decided defaults                                                                                                                              |
| Public question and response types | `crates/question_model`                                         | Answer-free and generated to TypeScript                                                                                                                                                       |
| Source interpretation              | `crates/adapters/native`, `qti`, `webwork`, `h5p`, `imathas`    | Each format has one strict versioned adapter                                                                                                                                                  |
| Grading                            | `crates/grading` plus injected server-only adapter capabilities | Never in Wasm, generated TS, or browser JSON                                                                                                                                                  |
| Persistence and RLS                | `crates/learning-data-access` and `schemas/migrations`          | Memory/PostgreSQL parity; PostgreSQL is production authority                                                                                                                                  |
| Objects                            | `crates/objects`                                                | Typed keys, checksums, role-based delivery, inventory and reconciliation                                                                                                                      |
| HTTP and workers                   | `crates/server`                                                 | Same-origin, bounded request handling; protected asset grants and external-tool launch are POST-only; durable jobs carry explicit least authority                                             |
| Browser                            | `src/`                                                          | Strict decoders, accessible visible flows, no source archive parsing                                                                                                                          |
| Local stack                        | `local_stack_control/` and `containers/`                        | Python owns complex orchestration. WP-R1 converts Chapter One and aggregate acceptance; WP-PY-L1 was accepted 2026-08-15 after directly replacing the shell lifecycle with focused Python modules, final offline/live Validation, and named final reviews. |
| Production deployment              | `deploy/opentofu/`                                              | Declarative, reviewable, drift-detectable, disposable before activation                                                                                                                       |

## Dependency map

```text
WP-RC1 course appearance
    |
    v
WP-RC2 production-seam cleanup
    |
    v
WP-RC3 accepted compatibility proof ---> WP-ARCH1 accepted source decomposition
                                                   |
                                                   v
                                WP-RC3R accepted standalone PG renderer
                                                   |
                                                   v
                    WP-RC8 production account-provider composition
                                                   |
                                                   v
                                WP-UI1 interface composition
                                                   |
                                                   v
                                        WP-RC4 flat JSON v2
                                                                        |
                                                                        v
             WP-P1..WP-P6 secure learner payload ---> WP-RC5 eight families/pilot
                                                        |
                                                        v
WP-RC6 profiles/H5P
WP-P2 persistent bindings ---> WP-RC7 M5 reconciliation/integration
WP-RC8 accepted identity/enrollment ---> WP-RC9 LTI ---> WP-FU1..WP-FU6 secure uploads ---> WP-RC10 OpenTofu
                                                                        |
                                                                        v
                                                             WP-RC11 bot controls
                                                                        |
                                                                        v
WP-RC12 release acceptance
WP-RC3R accepted catalog publication ---> WP-HG1 operational human references
                                               |
                                               v
                                     WP-UI1 route and visual contract
                                               |
                                               v
                              corrected four-question J13 walkthrough evidence
                                               |
                                               v
                              RC5 Chapter 1 workflow acceptance (not RC5 completion)
```

The human owner reprioritized WP-UI1 interface composition as the immediate repository-owned package, and it is now
accepted. WP-RC8's production composition is complete, while its external provider, browser, and independent
closeout gates remain. WP-RC4 resumes after that acceptance; its internal version 2 implementation no longer waits
on external QTI-JSONL artifacts. WP-P1 may proceed alongside RC4 closeout, but the complete WP-P1 through WP-P6
boundary must be accepted before WP-RC5. WP-RC7's non-schema inventory work may proceed earlier; its schema work
begins only after WP-P2 preserves the reserved migration ordering below.

### WP-UI1: Compose the PLE teaching workspace

- **Status:** accepted on 2026-08-13. The owner rejected the previous geometry and default contrast as component-correct
  but page-level inefficient and visually flattened; the completed review and evidence are recorded in `docs/UI_DESIGN_REVIEW.md`.
- **Contract:** implement `docs/UI_DESIGN_GUIDE.md` and `docs/active_plans/workstreams/ui_design_system_implementation.md`.
  Standard presentation is the default; increased contrast is an account preference. No UUID appears in visible/announced
  UI or application navigation. Questions use the non-sequential Crockford Base32 identity in `docs/QUESTION_ID_SPEC.md`;
  hidden snapshots remain internal. Public references remain locators under existing authorization.
- **Order:** route, Question ID, and gradebook identity; account preference; shared system/theme; instructor pages and
  assignment reuse; student pages; then browser rendering and measured acceptance.
- **Success:** representative pages satisfy the workstream's behavior and visual gates without changing grading, answer
  secrecy, tenancy, or learning semantics.

### WP-HG1: Close the human-guidance operational workflow gaps

- **Status:** accepted on 2026-08-12. This is a cross-cutting acceptance package, not a claim that
  the broad release plan is complete. It replaces the insufficient standard of merely rendering a
  friendly label. Its visible instructor result is a deliberately constructed four-question
  Genetics Chapter 1 assignment; the separate two-chapter eight-question publication and learner
  sweep remains the RC5 release oracle.
- **Depends on:** accepted catalog publication and the visible assignment editor. It must close
  before the corrected instructor walkthrough may again be called accepted and before RC5 claims
  that the Chapter 1 authoring workflow is accepted.
- **Owners:** catalog/domain owner for the cross-boundary reference contract; browser/HCI owner for
  copy/paste recovery; PostgreSQL owner for tenant lookup conformance; walkthrough owner for visible
  evidence; independent architecture, security, and HCI reviewers.
- **Files:** `crates/question_model/src/catalog.rs`, the catalog migration and PostgreSQL resolver,
  generated public contracts and strict decoders, `src/api/`, `src/pages/assignment_editor_*`, mock
  catalog handlers, focused Rust/TypeScript/Playwright tests, `tests/playwright/ui_walkthrough_instructor_setup.spec.ts`,
  `tests/walkthrough/`, this plan, the walkthrough plan, status, and post-acceptance documentation.
- **Behavior:** the editor displays and can copy a human reference, accepts pasted exact Question IDs in one
  obvious add-by-ID control, resolves the one immutable published question named by that ID under the current
  tenant, and changes the assignment only after a whole pasted batch resolves. It preserves pasted
  text and the existing draft for malformed, unavailable, unauthorized, duplicate, race, and network
  cases with labelled recovery. The browser accepts no UUID as a question identifier and exposes no
  UUID-valued DOM helper solely for test extraction. Displayed Question IDs remain selectable and
  copyable in canonical `AAA-BBBB` form.
- **Numeric and endpoint contract:** use one seven-character Crockford Base32 Question ID, displayed
  as `AAA-BBBB`, rather than public number/version pairs. The server validates its HMAC-derived
  checksum before resolving that exact published question through tenant and actor authorization.
  Live and mock resolver semantics agree: malformed or checksum-invalid is 400, unavailable is 404,
  unauthorized is 403, and an accessible exact published question succeeds. A PostgreSQL conformance case
  proves a valid Question ID cannot resolve a foreign-tenant-only question. Hidden immutable snapshots
  and version identity remain internal for authorized replay, grading, audit, provenance, and
  transport; no instructor-facing selector or latest-resolution path accepts them.
- **Canonical walkthrough behavior:** J13 searches the published catalog only to find the displayed
  human references, copies and pastes the four Genetics Chapter 1 `AAA-BBBB` Question IDs, visibly
  observes four selected questions, creates the assignment, and hands only its public
  course/assignment identifiers to later student stages. It must not arrange the assignment through
  an API or extract UUIDs from DOM state. The eight-question Genetics-plus-Biochemistry sweep remains
  a separate release-content oracle; neither test substitutes for the other.
- **Explicit execution boundary:** the Python runner exposes operator choices through documented
  arguments or the selected Compose file, clears inherited `PLE_*` walkthrough overrides from its
  owned children, and hands fixed Node/Playwright stages one schema-versioned mode-0600 private
  input file by explicit argument. This is a narrow process contract, not a growing collection of
  environment flags or manually maintained child configuration.
- **Permanent tests:** parser/domain bounds; Rust/PostgreSQL resolver and RLS conformance; strict
  browser decoder/client/repository recovery; mock/live error-class parity; editor batch atomicity,
  duplicate recovery, and keyboard submit; and a focused visible Playwright copy/paste setup test.
  These test behavior and contracts, not exact fixture counts, CSS strings, or implementation names.
- **One-time acceptance evidence:** a rebuilt current-stack J13/J1--J8 run with clipboard
  permission, redacted report, and refreshed public instructor screenshots; the isolated
  PostgreSQL/MinIO eight-question publication oracle; and independent architecture/security/HCI
  review. These are recorded evidence, not brittle permanent-suite fixtures.
- **Success:** a real instructor can copy a displayed `AAA-BBBB` Question ID, paste it, add that exact
  published question without recognizing a UUID, and construct the required four-question Chapter 1
  assignment; all recovery cases leave their work intact. The evidence distinguishes this completed vertical
  slice from unfinished RC4--RC12 release work.

#### WP-HG1.T: Connect whole-run timing to the assignment editor

- **Status:** accepted on 2026-08-12. This closes the remaining timed-problem gap in the
  human-guidance walkthrough; it does not change the status of the broader release plan.
- **Depends on:** the existing course-owned `AssignmentTimingPolicy`, the visible assignment editor,
  and the shared assignment revision. Implement the Rust/store/editor contract before the Solid form,
  then run focused behavior gates before the current-stack walkthrough.
- **Contract:** whole-run timing remains course-owned `AssignmentTimingPolicy`. The editor alone exposes
  `assignmentTiming: { timeLimitSeconds: positive u32 | null }`; `null` is an intentional untimed
  assignment. A new mastery draft receives the Rust-generated `900`-second default. Create, update, and
  editor GET compose assignment definition and timing atomically under one revision; published question
  versions and their immutable question-level `TimingPolicy` do not change.
- **Behavior:** the instructor can see and save an accessible `Time limit for each practice run` fieldset
  with Timed/Untimed choices and a minutes input. A new mastery assignment visibly starts at 15 minutes.
  The student sees a server-backed countdown for a timed run and `Untimed` only when the saved value is
  null. Invalid input and conflicts preserve the instructor's draft.
- **Permanent tests:** Rust Memory/PostgreSQL editor conformance for default, atomic create/replace, stale
  revision, and active-run deadline handling; strict HTTP/decoder/client tests for the nullable field;
  and keyboard-focused editor/student Playwright tests for the default, toggle, validation, recovery, and
  saved display. These assert behavior, not source strings or fixture counts.
- **One-time acceptance evidence:** run the real Podman PostgreSQL plus `webwork-pg-renderer` walkthrough.
  The instructor creates the Genetics assignment from copied `AAA-BBBB` Question IDs with the 15-minute
  default visible; the student sees the server-backed countdown, completes/retries, and starts a fresh
  timed practice run. Refresh and visually review the public screenshots at the guide boundary. The
  clean-stack walkthrough and screenshot capture supplied this one-time evidence; it remains separate from
  the permanent timing behavior tests.
- **Success:** the canonical instructor-to-student walkthrough truthfully shows the course-owned 15-minute
  whole-run limit without duplicating it into flat or WeBWorK question sources.

#### WP-HG1 accepted evidence boundary

WP-HG1 permanently retains only offline behavior contracts for Question ID parsing and resolution, editor
recovery and atomicity, explicit runner configuration, assignment timing, keyboard use, and responsive task
completion. These checks protect teaching behavior, authorization boundaries, and recoverable work rather
than fixture counts, source strings, CSS tokens, or screenshot pixels.

The rebuilt PostgreSQL, MinIO, and `webwork-pg-renderer` teaching loop; clipboard setup; isolated
release-content sweep; screenshot refresh and visual inspection; and independent architecture, security,
and HCI reviews are recorded one-time acceptance evidence. They are not ordinary pytest or browser-suite
fixtures. This accepted package does not close the broader release plan.

### Pre-production design cleanup

PLE has no users or durable production data. Complete this cleanup in order before the affected package or
walkthrough acceptance. It removes provisional variants from the current design; it does not rewrite
historical audit reports.

- [x] **Unify the roster model.** The code and focused review use the PLE-owned course roster as the single
      source for local and production flows: `2026080913_local_development_roster.sql`, its separate source,
      and duplicate Store path are gone. Local-file auth identifies the fictional actor; its `learner_alias`
      is composition metadata only. The local-only visible roster adapter resolves that alias and invokes the
      canonical `UpsertCourseMember` transaction, which creates the no-contact roster, membership, and
      enrollment records used by the UI. Production omits the adapter and control. Live PostgreSQL
      conformance and the empty-database teaching loop passed.
- [x] **Require the submission receipt snapshot.** Persist the authoritative attempt presentation
      `PresentationEnvelopeV1` plus exact public `AssetBindingV1` snapshot with every first receipt before
      returning feedback or advancing. Require an explicit issued capability: `EnvelopeV1` has the checksummed
      public snapshot and matching server-only grading envelope, while `NotApplicable` has neither. Native flat
      and WeBWorK issuance additionally persist explicit, checksummed, family-owned first-grade contracts;
      required contracts are never inferred from a nullable field. Translate public rendered IDs through that
      immutable private envelope for first grading; never reload a current catalog/grader definition or rerender
      backend state. Derive disclosure from the policy persisted at issuance; make active and submitted question
      `GET` replay the attempt/receipt snapshot rather than catalog, adapter-cache, renderer, or backend state.
      A checksummed `nextIssued` descriptor is immutable; only initial recovery may resolve the sole
      `nextPending` predecessor, and replay never resubmits. Prove missing/mismatched payload refusal and correct
      feedback/advance after refresh, duplicate submission, replica recovery, and unavailable successor data with
      focused permanent Store/server/browser behavior tests and an ignored disposable live PostgreSQL oracle.
      HOTSPOT issuance with real asset metadata remains a separate WP-RC5 dependency and is not claimed by this
      task.
- [x] **Adopt the v2-only native flat reader.** Removed the native flat v1 reader, its compatibility branches,
      and v1-only fixtures after moving current pilot source and inline test inputs to the closed v2 contract.
      Retain exact v2 source validation, answer-free public/private compilation, and all-family grading/learner
      behavior. Use inline or generated inputs under the fixture policy; do not retain duplicate v1 fixtures.
      This task leaves genuine current versioned protocols intact: `PresentationBindingV1`, QTI profile v1, the
      migration ledger, and human-readable `AAA-BBBB` Question IDs.
- [x] **Separate clean-stack screenshots from durability evidence.** Run the canonical eleven-image fake-user
      screenshot capture against a disposable, empty Podman stack and inspect the atomically produced public
      images. Run retained-volume progress, replay, and gradebook durability evidence as a separate live
      acceptance with its own report. The screenshot path proves first-use teaching clarity; the retained-volume
      path proves persistence. Neither result substitutes for the other. The screenshot runner owns a mode-0600
      derived Compose env and project-scoped application image, refuses competing PLE stacks and occupied fixed
      ports, and removes only its generated project volumes. It invokes the lifecycle's explicit
      `--canonical-walkthrough` mode so all fake-user runtime artifacts remain in private runner state rather
      than using env-path equality or mutating ordinary local-stack artifacts.

**Broader-release reminder:** closing WP-HG1 and WP-UI1 does not close the release. The dependency-ordered
follow-on sequence remains WP-RC8 acceptance, WP-RC4, WP-P1 through WP-P6, WP-RC5, WP-RC6, WP-RC7, WP-RC9,
WP-FU1 through WP-FU6, WP-RC10, WP-RC11, and WP-RC12, plus the external operator and institutional sign-offs
recorded in the rollout checklist.

## Milestone plan

| Milestone                          | Packages                       | Exit condition                                                                                               |
| ---------------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| RC-A: complete current experience  | WP-RC1                         | Course appearance works end to end and passes visual/live review                                             |
| RC-B: close runtime truth gaps     | WP-RC2, WP-RC3                 | Production names describe real code and the shipped WeBWorK service renders and grades                       |
| RC-C: finish content breadth       | WP-RC4, WP-RC5, WP-RC6         | Eight families, Chapter 1 content, QTI export, and honest H5P behavior pass                                  |
| RC-D: harden data and integration  | WP-RC7                         | Reconciliation and the combined M2-M5 gate pass; M5 is now accepted via retained-volume visible pagination   |
| RC-E: finish production interfaces | WP-RC8, WP-RC9, WP-FU1..WP-FU6 | Passwordless identity, enrollment, optional SSO, LTI, and inspected uploads use no browser-trusted authority |
| RC-F: deploy and defend            | WP-RC10, WP-RC11               | Disposable AWS deployment, restore, bot-cost, and legitimate-use gates pass                                  |
| RC-G: release                      | WP-RC12                        | Full working-codebase gate and independent release audit pass                                                |

## Work packages

### WP-RC1: Complete course appearance

- **Owner:** `ui-ux-engineer` for WP-CA6, then `integrator`; independent HCI, color, PostgreSQL, and
  security review.
- **Status:** accepted on 2026-08-09. The complete implementation and exact executable/visual review
  evidence are in `docs/active_plans/workstreams/course_appearance_implementation.md`; three
  independent read-only reviews reported no P0/P1/P2.
- **Files:** `src/features/course_appearance/`, `src/routes.ts`, `src/api/client.ts`,
  `src/api/contracts.ts`, `src/api/decoders.ts`, `src/api/http_client.ts`, `src/api/runtime.tsx`,
  `tests/playwright/course_appearance.spec.ts`, `tests/e2e/e2e_course_appearance.sh`,
  `docs/active_plans/workstreams/course_appearance_implementation.md`, and the durable architecture,
  contract, file-map, status, and changelog documents named in the course-appearance plan.
- **Behavior:** provide keyboard-complete theme selection, exact wide/narrow previews, decorative or
  informative alternative text, one save action, conflict reload, replacement/removal, responsive
  layouts, and current-only banner delivery. Candidate rows expire 60 minutes after creation;
  protected delivery grants expire within 60 minutes and do not bypass the current-pointer check.
- **Success:** all 15 themes and the Grass default render across course routes without global bleed;
  one banner appears only at course entry; stale, foreign, student-mutation, hostile-image, missing,
  candidate, and superseded cases refuse; independent review reports no P0/P1.
- **Validation:** focused Node and Playwright tests; Rust Store/server suites; disposable PostgreSQL
  and MinIO acceptance; one-time 15-theme contact sheet and human contrast review;
  `./check_codebase.sh`; built-browser Playwright; `source source_me.sh && python3 -m pytest -q
tests/`; both diff checks. The contact sheet and exact theme inventory are not permanent test
  fixtures.

### WP-RC2: Remove placeholder production seams

- **Owner:** `rust-code-expert`; documentation owner for maps; independent code reviewer.
- **Status:** accepted on 2026-08-09; exact evidence is in
  `docs/active_plans/workstreams/production_seam_closure.md`.
- **Files:** `crates/adapters/h5p/src/import.rs`, `crates/adapters/qti/src/parser.rs` and
  `parser/tests.rs`, `crates/adapters/webwork/src/renderer_contract.rs`, adapter/server importers,
  `crates/learning-data-access/src/lib.rs`, Memory/PostgreSQL/test Store implementations,
  `crates/server/src/run.rs`, and the named contract/map/status/changelog documents.
- **Behavior:** module names describe their concrete responsibility; catalog resolve/search are
  required Store capabilities; named test adapters retain explicit limited behavior; current feedback
  reads durable release state through the sole `project_feedback` policy.
- **Success:** no maintained production file is empty or named `stub`; no production path relies on
  `todo!`, `unimplemented!`, placeholder return data, or a missing catalog-capability default.
  Explicit mocks remain under test/local-development owners.
- **Validation:** focused adapter/Store/server behavior tests, format, strict workspace Clippy,
  workspace tests, `./check_codebase.sh`, pytest, and both diff checks passed. The source-name and
  placeholder closure inventory was one-time human-reviewed evidence and is not a permanent test;
  independent review reported no P0/P1.

### WP-RC3: Integrate upstream WeBWorK as shipped

- **Owner:** `rust-code-expert` for the adapter, `integrator` for containers, independent security
  reviewer.
- **Status:** accepted on 2026-08-10. The live PLE/browser acceptance and final independent review
  passed after WP-ARCH1 accepted the capability extraction and permanent size boundary. The
  decision-complete execution contract is in
  [webwork_shipped_integration.md](../workstreams/webwork_shipped_integration.md).
  The files and commands below preserve that historical compatibility snapshot; WP-RC3R deletes or
  replaces them for the current runtime.
- **Files:** `crates/adapters/webwork/src/http_renderer.rs`, `renderer_contract.rs`,
  `shipped_render_rpc.rs`, and `sanitizer.rs`; `crates/server/src/webwork_backend.rs`;
  `containers/compose.yaml`, `containers/compose.webwork.yaml`, `containers/env.example`,
  `containers/webwork/Containerfile`, `containers/webwork/entrypoint.sh`,
  `containers/webwork/probe_render_rpc.sh`, `containers/webwork/webwork2.mojolicious.yml`,
  `containers/webwork/site.conf`, `containers/webwork/course.conf`, and
  `containers/webwork/init_render_course.sh` for exact-source OCI build, private MariaDB, and
  render-course configuration; focused private local-stack lifecycle modules; the immutable fixture and provenance under
  `content/pilot/webwork/`; `crates/project-tools/src/e2e_seed.rs`;
  `tests/e2e/e2e_webwork_api_secret_mode.sh`, `tests/e2e/e2e_webwork_render_rpc.sh`,
  `tests/playwright/webwork_live_config.ts` and `tests/playwright/webwork_run.spec.ts`; container,
  contract, architecture, status, and changelog docs. Exact Compose/source inspection used during
  implementation was one-time evidence, not a retained pytest.
- **Behavior:** call private upstream `/render_rpc` with server-owned render-course credentials,
  base64 PG source, immutable PG path/version, seed, and bounded form fields. The same upstream route
  renders and grades. RC3 translates exactly one radio-choice PG control to an answer-free PLE
  single-choice envelope, sanitizes browser markup, re-renders to reconstruct the private field/value
  mapping for grade, and translates the server-side score to `GradeOutcome`. Reject answer hashes,
  correct answers, credentials, unexpected protected fields/scripts/resources, redirects, oversized
  bodies, identity/version drift, malformed output, and unsupported controls. One question outage does
  not stop native questions or API health.
- **Success:** `source source_me.sh && python3 local_stack.py start --with-webwork` builds/starts a private WeBWorK plus MariaDB
  profile from exact unmodified upstream revisions, verifies the resulting OCI digest, renders the exact
  immutable licensed `content/pilot/webwork/which_hydrophobic-simple.pgml` RadioButtons fixture twice
  with the same seed and stable result, grades correct and incorrect submissions, proves a cache hit,
  and proves no protected material crosses the browser network trace. This bounded fixture proves the
  shipped upstream protocol and PLE translation with owner-controlled license/provenance; broad OPL
  corpus compatibility remains outside RC3.
- **Validation:** permanent adapter behavior tests use the minimal owned, redacted protocol input;
  private-container render/grade/cache/outage and browser network checks are opt-in live acceptance;
  size/redirect/identity refusals remain focused offline behavior tests; full repository gates pass.

### WP-RC3R: Replace the full homework application with the standalone renderer

- **Owner:** `rust-code-expert` for the adapter, container integrator for the private runtime, and an
  independent security/runtime reviewer.
- **Status:** accepted on 2026-08-10. RC3 remains historical compatibility evidence; this package is
  the current runtime topology. The final correction binds persisted attempts and render-cache hits
  to the configured renderer identity and refuses version drift before any replay lookup, render, or
  grade RPC.
- **Files:** `crates/adapters/webwork/src/http_renderer/`, its protocol owner and tests;
  `crates/server/src/composition/` settings/tests; `containers/compose.yaml`,
  `containers/env.example`, `containers/webwork/probe_render_api.sh`, and
  focused private local-stack lifecycle modules; the WebWork E2E/Playwright/container tests; current contract,
  architecture, operation, status, and changelog docs.
- **Behavior:** consume a declared private image built and maintained by the external
  `webwork-pg-renderer` project. PLE records the selected OCI identity and sends only trusted
  immutable PG/PGML source,
  source metadata, fixed seed, fixed display policy, and the server-resolved submitted answer to the
  standalone render/grade API. Remove WebWork2 course, roster, set, user, password, session,
  assignment, and MariaDB dependencies. Preserve strict bounded response parsing, sanitization,
  answer-free PLE projection, private replay mapping, outage containment, and browser isolation.
  The API is the sole Compose build owner of the shared local Rust application image; the worker
  consumes that image with its own command and runtime configuration, so a constrained local
  Podman machine never performs concurrent duplicate Cargo builds.
  Every `OTHER_REPOS/` path remains reference-only and cannot be a build context, import, mount, or
  runtime source.
- **Success:** `source source_me.sh && python3 local_stack.py start` starts PLE PostgreSQL, MinIO, API, worker,
  gateway, and one private stateless PG renderer. No MariaDB service, WebWork2 application, render
  course, render user, or renderer password exists. The licensed pilot renders twice with a stable
  seed, grades correct and incorrect submissions, proves the cache/replay behavior, survives
  renderer restart without educational-record loss, and exposes no renderer endpoint or protected
  material to the browser.
- **Validation:** fast offline adapter/server behavior tests; a volume-preserving typed lifecycle
  teardown/rebuild and the render/grade/cache/outage path are opt-in E2E; browser network trace;
  strict Rust, documentation, and full repository gates; independent review with no P0/P1. Exact
  source/Compose topology scans were implementation probes and are not retained tests. The live
  E2E proves supported behavior rather than byte-identical images, arbitrary startup timing, or
  unrestricted PG compatibility.
- **Dependency:** complete after accepted RC3/WP-ARCH1 and before WP-RC4 closeout, payload cutover,
  WP-RC5 matching, or any broader WeBWorK compatibility claim.

### WP-RC4: Freeze PLE flat JSON v2

- **Owner:** `rust-code-expert` for the source/compiler boundary, `typescript-engineer` and
  `solid-js-expert` for exact learner contracts, and an independent family/security reviewer.
- **Files:** `crates/adapters/native/src/flat_question.rs` and `flat_question/v2.rs`;
  `crates/question_model/src/response.rs`; `crates/domain/src/validation.rs`;
  `crates/grading/src/{flat_question,key,checker}.rs`; generated contracts and strict decoders;
  `src/components/responses/`; contract, architecture, file-map, and changelog docs.
- **Behavior:** the native reader strictly parses the v2 source for all eight families using the
  reviewed QTI Package Maker item semantics, plus a bounded PLE HOTSPOT extension;
  rejects duplicate/unknown/invalid source; compiles answer-free public and grader-only private
  values; and supplies exact key-free learner response shapes.
- **Implemented evidence:** one source/compiler/grading behavior suite covers correct and incorrect
  responses for all eight families; strict browser decoders refuse answer fields and invalid hotspot
  geometry; keyboard Playwright covers multi-blank, matching, and hotspot alongside existing MC, MA,
  and ORDER controls; affected Rust tests, strict Clippy, TypeScript, ESLint, and source-size gates
  pass.
- **Success still required for package acceptance:** independent contract/security review, complete
  compact inline/generated author-source examples and invalid boundaries, and confirmation that
  version 1 canonical bytes and every browser/Wasm projection remain answer-free. Add a file fixture
  only when its serialized bytes are the compatibility contract. External QTI-JSONL is not an RC4
  prerequisite; a future adapter may translate a separately accepted external contract into these
  compiler outputs.

### WP-RC5: Implement all flat families and Chapter 1 content

- **Owner:** one `rust-code-expert` per family in strict sequence, `solid-js-expert` for author/learner
  widgets, `bptools-writer-expert` for source review, and an integration owner for pilot content.
- **Prerequisite contract:** implement and accept the atomic learner render/response boundary in
  `docs/active_plans/decisions/secure_question_grading_payload_plan.md`
  after WP-RC3 live acceptance and before the first WP-RC5 family. It makes `QuestionAttemptId`,
  authenticated session, and `Idempotency-Key` the only submission authority; it does not delay
  current WP-RC3 live acceptance.
- **Files:** `crates/question_model/src/response.rs` and `envelope.rs`;
  `crates/domain/src/flat_response_validation.rs`; `crates/grading/src/flat_question.rs`;
  `crates/adapters/native/src/flat_question/v2.rs`;
  `crates/learning-data-access/src/flat_question.rs`; `crates/server/src/flat_question_publication.rs`;
  generated contracts; `src/features/flat_question_authoring/`;
  `src/components/responses/{multiple_answer,fill_in,multi_fill_in,numeric,matching,ordered,hotspot}.tsx`;
  `src/components/response_widget.tsx`; behavior tests beside each owner;
  `content/pilot/chapter_1_assignments.yaml` and copied, licensed source
  under `content/pilot/sources/`; `crates/project-tools/src/pilot_content.rs`;
  `tests/e2e/e2e_chapter_one_pilot.py`; `tests/e2e/e2e_chapter_one_browser.py`;
  `tests/playwright/chapter_one_run.spec.ts`; `docs/PILOT_CONTENT.md`.
- **WeBWorK MATCH files:** the bounded Chapter 1 slice extends
  `crates/adapters/webwork/src/http_renderer/`, `renderer_contract.rs`, their contract tests, server
  translation, and `tests/playwright/chapter_one_run.spec.ts` with the reviewed matching render and
  grade path. Additional source shapes remain part of broader WP-RC5 acceptance.
- **Behavior:** protected visual authoring now covers all eight version 2 families with stable
  semantic IDs, family-specific validation, keyboard-first editing, and an answer-free learner
  response preview. HOTSPOT uses server-verified image selection and a labeled region list rather
  than browser-authored storage identities. Complete CAS save, full Memory/PostgreSQL publication,
  issue, feedback, summary, retention, and cleanup acceptance around the implemented eight-family
  source/runtime core. WP-RC5 also extracts and grades a typed WeBWorK MATCH projection through the
  named adapter, contract, server, and browser-live owners before the Chapter 1 matching source is
  accepted. HOTSPOT retains its keyboard/list alternative and scale-independent normalized
  coordinates while adding the secure pointer/media workflow.
- **Pilot inputs:** Genetics uses `genetic_disorders-which_one.pgml`,
  `genetic_disorders-matching.pgml`, `bbq-WOMC-genetic_disorders-questions.txt`, and
  `bbq-MATCH-genetic_disorders-questions.txt`. Biochemistry uses
  `biochemical_functional_groups-which_one.pgml`, `biochemical_functional_groups-matching.pgml`,
  `bbq-WOMC-biochemical_functional_groups-questions.txt`, and
  `bbq-MATCH-biochemical_functional_groups-questions.txt` from the local biology-problems project.
- **Success:** every family passes one complete Memory/PostgreSQL author-to-learner path; the two
  Chapter 1 assignments each publish exactly four reviewed questions with immutable source,
  license/provenance, correct/incorrect grading, and no answer-bearing learner payload.
- **Validation:** permanent family/keyboard/browser behavior gates from the flat-family plan;
  disposable RLS/object and pilot import/publish/grade acceptance; one-time laptop, portrait,
  narrow-phone, zoom, screen-reader, and human visual review evidence; full repository gate and
  independent family/content review. Do not retain screenshot pixels, viewport timing, or exact
  fixture-count assertions.

### WP-RC6: Close QTI export and H5P capability claims

- **Owner:** separate Canvas and Blackboard exporter owners; H5P adapter owner; worker/UI integrator.
- **Files:** `crates/adapters/qti/src/export_profiles/{mod,canvas,blackboard,archive}.rs`;
  `crates/adapters/qti/tests/profile_export.rs`; `crates/server/src/qti_export.rs` and worker tests;
  `src/features/qti_profile_export/`; `crates/adapters/h5p/src/import.rs`; H5P tests;
  `crates/learning-data-access/src/lib.rs`, `in_memory/exports.rs`, and `postgres/exports.rs`;
  `crates/objects/src/lib.rs` and `bucket.rs`; generated contracts; Playwright; contracts,
  architecture, usage, and changelog docs.
- **Behavior:** background profile export snapshots an immutable supported problem version, refuses
  unsupported fields before object creation, emits a protected requester-owned artifact, and exposes
  queued/running/failed/download states without revealing answers. H5P declares ungraded practice
  honestly and imports only the source families it can map losslessly into the protected native
  compiler.
- **Success:** supported Canvas and Blackboard subsets re-import with equal prompt, choice order,
  correct binding, points, and stable PLE identity; every unsupported semantic refuses without an
  artifact. H5P never claims server grading for a browser-evaluated package.
- **Validation:** exporter semantic round trips and refusal matrix; worker replay/concurrency/object
  authorization tests; H5P capability/import tests; visible author export flow; full package gate and
  independent review.

### WP-RC7: Reconcile objects and close M2 through M5

- **M2-M5 note:** the simulator M5 retained-volume gate is accepted after
  visible native course/gradebook pagination reached the current targets in
  manager and independent same-seed `--build` runs. This is not a live
  PostgreSQL, object-reconciliation, or combined WP-RC7 acceptance claim.

- **Owner:** `postgresql-expert` and object-store `rust-code-expert`, followed by integration and
  security reviewers.
- **Files:** `crates/objects/src/inventory.rs`; `crates/learning-data-access/src/object_reconciliation.rs`
  plus Memory/PostgreSQL owners; `schemas/migrations/2026080910_object_reconciliation.sql`;
  `crates/server/src/object_reconciliation_worker.rs`; `tests/e2e/e2e_object_reconciliation.sh`;
  `tests/e2e/e2e_release_integration.sh`; object, retention, security, architecture, status, and
  changelog docs.
- **Behavior:** page through bounded bucket inventory; register every deterministic render/cache
  object; mark first-observed unreferenced bytes; quarantine and delete only after a later inventory
  still finds them beyond injected policy; cancel deletion when a reference appears; alert and
  quarantine delivery when a database record points to missing or mismatched bytes; never delete a
  database record to hide a broken reference; make every pass idempotent and tenant-safe.
- **Success:** memory, PostgreSQL, and MinIO agree on exact/replayed/orphan/missing/mismatch cases;
  concurrent creation cannot be swept; all M2-M5 exit criteria run together, including hostile
  import, answer-key denial, foreign tenant, partition pruning, renderer outage, below-k statistics,
  archive/delete, replica, and worker recovery.
- **Validation:** permanent Store/object behavior conformance; fresh/no-op migration, PostgreSQL/MinIO
  lifecycle, multi-replica/worker soak, and combined system behavior as opt-in disposable E2E; full
  repository gate; independent data-security and milestone reviews. Migration-shape inventories and
  timing observations are one-time evidence.

### WP-RC8: Implement passwordless identity and enrollment

- **Walkthrough boundary:** WP-RC8 remains production identity and enrollment
  release work. Its provider, mailbox, delivered-link, passkey, and
  multi-replica evidence is intentionally not a dependency or acceptance gate
  for the local instructor-to-student assignment walkthrough. That bounded
  local no-email pilot is accepted separately in
  `docs/active_plans/peptidyle-walkthrough-plan.md`; it does not advance or
  satisfy WP-RC8 or working-codebase release acceptance.
- **Status:** generic implementation exists and the repository-owned production-account
  composition task is independently reviewed on 2026-08-10; package acceptance remains open. The account/email/passkey,
  copy-link invitation, roster/policy/bulk import, atomic enrollment, Solid UI, migration, and
  manual no-store grade-export slices exist. `production_router_from_env` now composes the
  provider-free PLE passwordless/account/session route graph with an eight-hour
  `FirstPartyHttps` cookie policy and `ReviewNotRequired`; it neither reads local identity
  environment settings nor mounts `/api/auth/login`. Provider-neutral authenticated
  STARTTLS/implicit-TLS configuration and secret-file Compose plumbing are implemented without a PLE
  mail service. A live send through the operator-selected provider, optional-passkey and
  multi-replica E2E, and independent security/HCI closeout remain; the rollout checklist stays
  unchecked.
- **Owner:** authentication `rust-code-expert`, PostgreSQL owner, enrollment/API owner, UI owner,
  SMTP-provider configuration owner, and independent security/HCI reviewers. PLE does not own mail
  transport or deliverability tooling.
- **Files:** `crates/server/src/auth/{passwordless,email,webauthn,oidc}.rs`;
  `crates/learning-data-access/src/{account_identity,course_roster}.rs` plus Memory/PostgreSQL
  owners; `schemas/migrations/2026080909_passwordless_identity.sql`;
  `src/pages/{sign_in,account_security,course_roster,course_invitation}_page.tsx`;
  `src/auth/session_context.tsx`; passwordless/enrollment E2E and Playwright suites;
  [ENROLLMENT_DESIGN.md](../../ENROLLMENT_DESIGN.md); auth/security/deployment/usage docs and
  changelog.
- **Behavior:** create institution-independent PLE accounts keyed by opaque global `UserId`.
  Use uniform, short-lived, single-use, rate-limited email challenges as the canonical registration
  and sign-in path, with hashed secrets, redacted logs, and browser binding where practical. Verify
  optional discoverable passkeys with an established WebAuthn implementation on the same PLE
  account boundary; permit multiple credentials, normal authenticator user verification, and
  explicit credential revocation without requiring attestation. Email and passkey authentication
  produce the same bounded account session and neither has record-transfer authority. Preserve the
  existing host-only HttpOnly cookie. A global account may join multiple tenants, but every request
  derives its active `TenantContext` from a verified course/tenant relationship rather than browser
  authority.
- **Enrollment behavior:** an instructor creates or bulk-stages an invitation containing verified
  email syntax plus a protected course-scoped institutional roster ID. Optional exact normalized
  email-domain policy catches mistakes and constrains open signup. Single creation returns a no-store
  copy link for delivery through a trusted LMS; configured SMTP through the established Rust adapter
  is optional. Only the learner's authenticated claim creates the `course_member`, tenant-scoped
  `StudentId`, all assignment enrollments, and all
  empty summaries in one Store transaction. Learner-scoped Store entry points derive the actor from
  the authenticated session and atomically recheck an active Student membership before returning or
  changing an enrollment, run, attempt, summary, issued material, feedback, external launch, or
  grading state; deletion/revocation serializes against those checks. Manual grade export uses the roster ID and only the
  destination profile's required course metadata. Pending invites, correction/re-invitation,
  revocation, raw CSV disposal, roster retention, and accessibility follow the enrollment design.
- **Optional SSO behavior:** OIDC Authorization Code with PKCE and SAML may link a verified external
  subject to an existing PLE `UserId`. Validate discovery/metadata, issuer allowlist, state, nonce,
  redirect URI, signature, audience, expiry, and replay. SSO never selects a tenant by email,
  silently creates a parallel account, or becomes required for standalone deployment.
- **Success:** production composition enters the same PLE email-account/session flow as the generic
  passwordless routes; email registration/sign-in, optional passkey login, verified email
  replacement, roster invitation/claim, and course access work across replicas. Account enumeration, token
  replay, credential cloning/mismatch,
  wrong origin/RP ID, cross-course invitation use, domain suffix confusion, roster-ID collision,
  stale bulk commit, revoked-membership/stale-identifier access, revocation races, and cross-tenant disclosure fail safely. An instructor can invite a real
  browser user and export the resulting score without SQL, seeding tools, global `UserId`, passkey
  metadata, or unrelated activity in the export. Logs and browser state contain no authentication
  or invitation secret.
- **Validation:** focused production-composition tests proving the canonical PLE passwordless/account
  graph, rather than local-file development authentication, owns production entry; WebAuthn/email/
  account/roster Store conformance and RLS; deterministic hostile token/domain/CSV tests; copy-link
  browser handoff plus a Fastmail test account, authorized sender, and application credential for
  email authentication and optional-passkey browser E2E; multi-replica
  invitation, login, email replacement, and manual grade-export Playwright; standards-compliant OIDC
  connector E2E only when that optional connector is enabled; security/HCI review; full repository
  gate.

There is no separate account-recovery or Instructor record-transfer workflow in version 1. Possession
of the verified account email is the canonical sign-in path, so losing a passkey does not create a
special state. A signed-in learner may replace the account email only after verifying the new
address. If the learner no longer controls the current email, an instructor may revoke the old
course membership and invite a new address, but PLE does not infer that a new account owns the old
account's educational records. The institutional LMS remains the grade system of record for manual
correction. Any future account merge or educational-record transfer requires a separately approved
identity-proofing and audit design; it is not a version 1 release dependency.

### WP-RC9: Implement LTI Advantage launch and grade passback

- **Owner:** LTI `rust-code-expert`, Store/PostgreSQL owner, LMS integration reviewer.
- **Files:** `crates/server/src/lti.rs`, `lti_launch.rs`, and `lti_ags.rs`;
  `crates/learning-data-access/src/lti.rs` plus backends;
  `schemas/migrations/2026080911_lti_advantage.sql`; `src/pages/lti_launch_page.tsx`;
  `tests/e2e/e2e_lti_advantage.sh`; LTI contract/security/usage/deployment docs and changelog.
- **Behavior:** validate OIDC login initiation and LTI 1.3 launch state/nonce, issuer, deployment,
  client, signature, audience, message type, roles, resource link, and deep-link/assignment binding.
  Store platform credentials server-side; send idempotent AGS scores derived only from
  `student_assignment_summary`; retry with bounded durable jobs; browser messages never become grades.
- **Success:** one LMS sandbox launch maps to the intended tenant/course/assignment and one score
  passback is exact under replay; forged/cross-tenant/stale/mismatched launches and grade URLs refuse;
  provider outage queues bounded retry without blocking native assignment use.
- **Validation:** signed protocol fixtures generated in tests, not captured secrets; Store/RLS tests;
  disposable LMS sandbox or standards harness; replay/cross-tenant/outage tests; full gate and
  independent protocol/security review.

### WP-FU1 through WP-FU6: Secure learner file uploads

WP-FU1 through WP-FU6 run after WP-RC9 and before WP-RC10. Their complete contract, files,
one-owner packages, state machine, security controls, and gates are in
`docs/active_plans/active/secure_learner_file_upload_plan.md`. The packages replace the
raw file-upload object-key placeholder with a server-issued tenant/learner/attempt-bound upload,
stream into non-deliverable temporary storage, inspect and promote exact SHA-256-bound bytes, and
atomically consume one ready upload into the existing manual-grading path. They own
`2026080912_secure_learner_uploads.sql`; the current learner route remains fail-closed until all six
packages are accepted.

Historical note: the former `2026080913_local_development_roster.sql` source was a pre-production
bridge. The completed code/focused-review portion of the cleanup removed it in favor of one
canonical roster model; the remaining live empty-stack evidence does not alter or satisfy WP-RC8
production email identity, invitation, or enrollment acceptance.

### WP-RC10: Add declarative AWS deployment

- **Owner:** M6 deployment architect and `postgresql-expert`; independent operations/security review.
- **Files:** `deploy/opentofu/{versions,providers,variables,locals,network,database,storage,compute,edge,waf,observability,outputs}.tf`;
  `deploy/opentofu/env.example.tfvars`; `deploy/opentofu/tests/policy.tftest.hcl`;
  `devel/deploy_disposable.sh`; `devel/rehearse_restore.sh`; deployment, container, security,
  backup/restore, operations, and cost docs; status and changelog.
- **Behavior:** create private subnets with no default workload egress, encrypted RDS with PITR and
  verified TLS, four separately encrypted S3 domains/lifecycles, ECR/Fargate API, worker, and
  dedicated public-asset publisher, private ALB origin, CloudFront `www` and `app`, WAF, KMS,
  Secrets Manager references, VPC endpoints, least IAM, logs/metrics/alarms, autoscaling ceilings,
  and immutable deployment/rollback manifests. The public bucket is CloudFront-read-only only for
  exactly tagged published assets; API/worker cannot write it; the publisher has only its narrowly
  scoped database and object capability. The external renderer is not deployed or reachable by this
  baseline; the feature remains disabled until an independently reviewed production renderer identity,
  image, protocol, egress, resource-limit, and private-network attestation is supplied. Remote state
  is encrypted/restricted and no secret enters plan, output, image, repository, or browser.
- **Success:** a disposable environment plans from an empty account boundary, deploys, migrates,
  proves deployed application roles cannot bypass RLS and the publisher cannot exercise API/worker
  database authority, passes semantic health and one complete assignment, verifies public-tag/object
  isolation and protected asset delivery, restores from backup, rolls back app/static manifests,
  detects drift, and destroys only resources tagged with its unique deployment ID.
- **Validation:** `tofu fmt -check -recursive`, `tofu init -backend=false`, `tofu validate`, `tofu test`;
  secret/config policy scan; disposable plan/apply/health/RDS-TLS-and-RLS/publication/restore/rollback/
  drift/destroy rehearsal; independent cost, security, database, and operations reviews.

### WP-RC11: Implement bot-cost controls

- **Owner:** the owners assigned in
  [bot_traffic_cost_reduction_plan.md](bot_traffic_cost_reduction_plan.md), with M6 architect
  integration.
- **Files:** `landing/index.html`, `landing/style.css`, `pipeline/build_landing.mjs`,
  `crates/server/src/request_cost.rs`, the existing server auth/router
  owners, `deploy/opentofu/edge.tf`, `waf.tf`, `observability.tf`, and
  `deploy/opentofu/tests/policy.tftest.hcl`; `tests/test_landing_artifact.mjs`;
  `tests/playwright/landing.spec.ts`; `tests/e2e/e2e_bot_cost.sh`;
  `docs/OPERATIONS_BOT_TRAFFIC.md`; and
  `docs/active_plans/workstreams/bot_traffic_cost_acceptance.md`.
- **Behavior:** anonymous landing traffic terminates at static edge storage; missing/malformed session
  input causes zero Store calls; random valid-format unknown tokens cause at most one indexed lookup;
  direct origins remain private; WAF rules start in count mode and preserve class-start/shared-egress,
  VPN/datacenter, international, IPv4/IPv6, keyboard, and screen-reader use; emergency modes expire or
  have one documented recovery action.
- **Success:** cold/warm anonymous load causes no PLE database, object-signing, queue, renderer, or
  grading work; normalized cost and legitimate-failure evidence are recorded; no client analytics or
  educational-record join is introduced; independent reviewers report no P0/P1.
- **Validation:** permanent offline request-cost and browser behavior tests; disposable crawler,
  class-start replay, origin/cache-poison, alarm, and emergency acceptance; one-time before/after cost
  report; and the bot plan's complete gate. Do not retain source-string infrastructure tests or make
  environment-specific cost/timing values permanent assertions.

### WP-ARCH1: Enforce capability-sized source ownership

- **Owner:** four independent expert owners for persistence, server, adapters/tooling, and browser;
  one integration owner; separate PostgreSQL, security, TypeScript/HCI, and architecture reviewers.
- **Status:** accepted on 2026-08-10. The dated 26-file baseline has zero maintained-code violations,
  all focused and integrated gates pass, no maintained code is present in the exact override list,
  and independent PostgreSQL, security, provider, TypeScript/HCI, test, size-policy, and final
  architecture reviews found no unresolved P0/P1 issue.
- **Depends on:** accepted WP-RC3; it completes before WP-RC4 so later payload and family behavior
  lands in focused owners rather than the existing oversized facades.
- **Files:** exact current inventory, module destinations, ownership, dependencies, behavior gates,
  and closure artifacts are in `docs/active_plans/active/source_module_decomposition_plan.md`; the permanent policy is
  `tests/test_source_file_line_limit.py` with exact non-maintained overrides in
  `tests/source_file_line_limit_overrides.txt`.
- **Behavior:** move complete capabilities behind stable public facades; preserve routes, wire,
  schema/SQL behavior, generated contracts, grading/security boundaries, CLI output, and browser
  behavior. Split tests by behavior. Scan maintained source roots with no maintained-code exception;
  immutable migrations and archival/history documents may retain exact manager-approved overrides.
- **Success:** every maintained source file is at most 999 physical lines; the permanent test proves
  999 passes and 1,000 fails; all focused and integrated behavior gates pass; documentation maps the
  new owners; independent reviewers report no P0/P1.
- **Validation:** per-lane Rust/Store/protocol/TypeScript/browser gates; `./check_codebase.sh`; full
  repository pytest; generated-contract checks; the permanent owner-approved source-size boundary;
  one-time final inventory; Markdown/ASCII/diff checks; disposable PostgreSQL baseline; independent
  multi-discipline review.

### WP-RC12: Release acceptance and documentation closure

- **Depends on:** WP-ARCH1, WP-RC1 through WP-RC11, and WP-FU1 through WP-FU6.
- **Owner:** release integrator; separate code, security, database, accessibility, operations, and
  documentation reviewers.
- **Files:** `tests/e2e/e2e_release_candidate.sh`; `docs/RELEASE_EVIDENCE.md`; `README.md`;
  `docs/INSTALL.md`, `USAGE.md`, `LOCAL_STACK_OPERATIONS.md`, `SECURITY_MODEL.md`,
  `DATABASE_STRUCTURE.md`,
  `CODE_ARCHITECTURE.md`, `FILE_STRUCTURE.md`, `CONTRACTS.md`, `RETENTION_POLICY.md`;
  implementation status/report, changelog, and release notes.
- **Behavior:** provide one repeatable local path and one disposable production path from clean clone
  through build, migrate, seed/configure identity, author, publish, assign, issue, answer, grade,
  feedback, export, retain/delete, backup, restore, and rollback. Record every environment-derived
  dependency and every out-of-scope production-activation action plainly.
- **Success:** no empty maintained file, production stub, TODO-only section, fake/disabled acceptance
  test, unresolved scope question, answer leak, cross-tenant path, silent format loss, or undocumented
  operator step remains. All claimed capabilities have current evidence and all reviewers report no
  P0/P1.
- **Validation:** `./check_codebase.sh`; strict Rust/TypeScript/Node/Python gates; built Playwright;
  combined local and disposable-cloud E2E; migration/restore/replica/worker/load/security/a11y gates;
  link/ASCII/diff checks; independent multi-discipline audit.

## Acceptance criteria and gates

- **Binary scope gate:** every known uncertainty is present in one of the two decision ledgers. No
  active plan ends with an open-question or deferred-decision section.
- **Artifact gate:** every in-scope package names owners and real production, test, documentation, and
  evidence files; generated outputs are validated through their owning generator.
- **Behavior gate:** mocks may support focused tests but cannot be the only acceptance evidence for a
  production route, Store, object, identity, worker, or deployment path.
- **Security gate:** answers, keys, provider tokens, object credentials, and tenant selection remain
  server-owned; forced RLS, active-membership Store authority, least grants, non-enumeration, bounded
  input, POST-only authority grants, external-dispatch indeterminacy fencing, and malicious-upload
  gates remain green.
- **Accessibility gate:** all student actions are keyboard complete; author flows have visible focus,
  status, recovery, zoom/reflow, and semantic labels; HOTSPOT has a non-pointer alternative.
- **Data gate:** migrations are forward-only after `2026080907_course_appearance.sql`; each migration
  passes fresh, no-op, checksum, RLS/grant, and live behavior evidence.
- **Review gate:** the implementer does not self-accept. Each package requires an independent no-P0/P1
  review after executable gates pass.

## Test and verification strategy

Run the narrowest owner test first, then the package gate, then the release gate. Permanent tests
assert stable behavior and security boundaries. Disposable checks own cloud credentials, upstream
services, large data, restore timing, cost, and visual artifacts. Scratch probes and deliberate
mutations are removed after their conclusions are recorded; no source-string or exact-count test is
kept merely to memorialize implementation details.

The release gate is:

```bash
./check_rust.sh
npx tsc --noEmit -p tsconfig.json
node --import tsx --test tests/test_*.mjs
./check_codebase.sh
bash run_playwright_tests.sh --build
bash tests/e2e/e2e_release_candidate.sh
source source_me.sh && python3 -m pytest -q tests/
git diff --check
git diff --cached --check
```

Environment-backed commands report `SKIP` with the missing named prerequisite only during ordinary
development. They must pass, not skip, in WP-RC12 release evidence.

## Migration and compatibility policy

- Preserve the accepted six-file baseline and `2026080907_course_appearance.sql`.
- Reserve `2026080908_secure_question_grading_payloads.sql`,
  `2026080909_passwordless_identity.sql`, `2026080910_object_reconciliation.sql`, and
  `2026080911_lti_advantage.sql` in that order. Secure learner uploads then own
  `2026080912_secure_learner_uploads.sql`. The 0908 file remains the schema predecessor even while
  its product cutover acceptance is open. WP-RC7 schema work begins after identity/enrollment 0909,
  while its non-schema object inventory work may run in
  parallel. A package needing another
  migration takes the next two-digit daily sequence; it does not insert or rename an accepted
  version.
- PLE flat JSON source identity lives inside the existing versioned source payload and immutable
  object/checksum binding; no family-shaped table is added.
- Native PLE flat JSON uses its v2 reader only. Current source and disposable test data are rebuilt
  to v2 during pre-production cleanup. `PresentationBindingV1`, QTI profile v1, the forward-only
  migration ledger, and `AAA-BBBB` Question IDs remain current protocol/version markers,
  not compatibility shims. Generic QTI and Canvas/Blackboard profile v1 retain their own named
  external contracts.

## Risk register

| Risk                                                     | Owner                | Control and trigger                                                                                                                           |
| -------------------------------------------------------- | -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Plan completeness becomes documentation-only             | Release integrator   | Package acceptance requires working artifacts and behavior evidence; mocks/docs alone fail                                                    |
| External QTI-JSONL work stalls                           | Interop owner        | PLE flat JSON v2 is authoritative internally; a future external adapter cannot block native families                                          |
| Shipped WeBWorK output leaks keys or unsafe markup       | WeBWorK owner        | Strict result translator, sanitizer, response scan, private network, and real browser trace                                                   |
| New family leaks an answer through generated types       | Family owner         | Public/private compiler split, Wasm dependency gate, DTO scan, server-only grader                                                             |
| Reconciliation deletes a concurrent valid object         | Object owner         | Two observations, quarantine, reference recheck, idempotency, concurrent-creation oracle                                                      |
| Email, passkey, or SSO input selects a tenant            | Auth owner           | Global account identity; course/tenant relationship validation; exact origin/RP ID; no email tenancy                                          |
| Roster convenience becomes permanent cross-course PII    | Enrollment owner     | Course-scoped email/roster ID, bounded imports/exports, direct Instructor authorization, retention lifecycle                                  |
| Revoked learner continues with a stale identifier        | Enrollment owner     | Actor-scoped Store methods recheck and lock active Student membership for every learner read and mutation; revocation-race oracle             |
| Publication makes uncommitted or restricted bytes public | Object owner         | Pending private registry plus durable job; dedicated publisher rechecks exact bytes/checksum and alone writes/activates tagged public objects |
| External provider outcome is unknown after a failed POST | External-tool owner  | Persist a lease-bound dispatch marker before the POST; block retry, grade, new launch, and finalization until explicit operator resolution    |
| LTI browser input becomes a grade                        | LTI owner            | AGS derives from summary rows; signed launch and server credentials; no browser grade authority                                               |
| Cloud plan contains a secret or destructive broad target | Deployment architect | Secret references only, unique deployment tags, reviewed plan, bounded destroy rehearsal                                                      |
| Bot rules block a shared campus or assistive user        | Edge owner           | Count mode, versioned legitimate corpus, accessible recovery, immediate rollback                                                              |
| Real pilot begins before external activation evidence    | Product owner        | Production activation checklist is separate and must be signed before enrollment                                                              |

## Rollout and release checklist

- [x] WP-RC1 course appearance accepted.
- [x] WP-RC2 production seams accepted.
- [x] WP-RC3 shipped WeBWorK accepted.
- [x] WP-RC3R standalone WebWork PG renderer accepted; WebWork2 and MariaDB removed from the PLE
      runtime.
- [x] WP-HG1 operational human references, numeric-domain parity, four-question instructor setup,
      and independent/live evidence accepted.
- [ ] WP-RC4 PLE flat JSON v2 independently accepted.
- [ ] WP-RC5 eight families and Chapter 1 content accepted.
- [ ] WP-RC6 QTI export and H5P claims accepted.
- [ ] WP-RC7 M2-M5 reconciliation/integration accepted.
- [ ] WP-RC8 production PLE email-account composition, passwordless identity, enrollment, and optional
      SSO accepted.
- [ ] WP-RC9 LTI Advantage accepted.
- [ ] WP-FU1 through WP-FU6 secure learner file uploads accepted.
- [ ] WP-RC10 OpenTofu disposable deployment/restore accepted.
  - [x] Repository-owned OpenTofu policy, four-domain storage/IAM, private edge/origin, API/worker/
        publisher task definitions, and static policy tests are code-complete.
  - [ ] A disposable AWS account proves plan/apply, endpoint-only workload paths, deployed RDS TLS and
        non-bypass/forced-RLS roles, publisher-only database/object authority, public-tag isolation,
        protected delivery, migration, semantic health, backup restore, rollback, drift, and bounded
        destroy.
  - [ ] The external renderer's production identity and isolation are independently attested before its
        feature flag is enabled; it is not part of the current deployment claim.
- [ ] WP-RC11 bot-cost controls accepted.
- [ ] WP-RC12 release evidence and independent audits accepted.
- [ ] Production operator supplies email delivery and WebAuthn RP configuration; record optional SSO identities when enabled.
- [ ] Institutional owner completes FERPA/legal/security sign-off.
- [ ] Human fall-pilot accessibility and teaching walkthrough passes before student enrollment.

## Documentation close-out requirements

Every package updates the changelog only after executable acceptance. A package that changes an
owner boundary also updates contracts, code architecture, and file structure. A package that changes
operator or user behavior updates install, usage, container/deployment, and security documentation.
WP-RC12 replaces status prose with exact evidence, archives completed active companion plans using
`git mv`, and leaves this plan open only for external production-activation checklist results.

## Patch plan and reporting format

Land one work package per reviewable patch in WP-RC order. A package may use separately reviewed
subpatches only where its file ownership table permits; the package is not accepted until the
integrated behavior passes. Each handoff reports:

- owner and work-package ID;
- exact files changed and index state;
- user-visible and security behavior completed;
- focused, package, and release commands with results;
- one-time evidence and artifact paths;
- out-of-scope decisions relied upon; and
- independent findings and their disposition.
