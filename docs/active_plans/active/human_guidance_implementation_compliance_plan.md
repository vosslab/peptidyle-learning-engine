# Plan: Human Guidance implementation compliance

## Current bounded SQL corrections

Accepted bounded corrections establish these narrow contracts: every finalized
`QuestionResponse` has an exact parent Assessment submission, owning Attempt, and matching
finalization time; native response controls use editable `save` and `formatOnly` modes; private
receipt-gated anonymous statistics increment atomically and remain after Unrelease; and the
support repair command no longer takes a redundant Account lock or has ambiguous use/revoke
references. Fresh PostgreSQL 17.11 parent proof passed all four rejection/retention cases; the
statistics/Unrelease oracle and actual-role support proof exited 0. The installed mandatory-NULL
shape matrix passed 25 LIKE cases. Root-observed checks include `cargo tsgen` (367 types),
`cargo check -p server_core -p project-tools --tests`, `cargo check -p wasm_bridge --tests`, and
the wasm32 target-feature check; response-control TypeScript and 19 focused tests passed. These
are not connected-browser, deployed-app, or global-closure demonstrations.
Fresh PostgreSQL 17.11 installation through `ple_migrator` also retained exactly the three valid
unique indexes with no duplicate access paths; independent review accepted the three removals.

Residual audit work remains for Course/Pool classification, Bloom, Change Proposals, recovery,
and resource-specific support authority. The current reconciliation receipt below supersedes the
former generator-drift statement; it changes inventory accounting only, not product acceptance.

## Current connected acceptance

Current local Blueprint classification-search source is implemented but not connected acceptance.
Public Blueprint Search accepts ordinary text plus optional Discipline, Subject, Topic, Subtopic,
and a Subject-across-Disciplines flag. The server validates hierarchy identities and associations,
binds the complete submitted tuple into its version-3 cursor, and retains existing visibility and
Public-only rules. Root Cargo session 31251, focused Rust session 56442, strict TypeScript plus 21
Node tests session 19414, and full pytest session 39220 (7,462 passed) passed; canonical
PostgreSQL 17 bootstrap/install and an isolated actual-role proof also passed with rollback. The
actual-component selector proof covers progressive hierarchy, keyboard input, cascades, applied
retry/pagination state, Clear, stale suppression, and 390px overflow only. Live `8147` predates
this source, so connected HTTP/browser, actual vocabulary-parent, and authorization acceptance
remain pending. This does not close the broad Blueprint search rows, alter the 450/503/43
inventory, or extend to Question Library search, which remains name-based. Tags, sorting, result
metadata, and return-state work remain separate. Receipt:
`/private/tmp/ple-classification-search-pool-receipt-20260916.md`.

Root's canonical build session 51649 terminally exited 0 and reached `Ready` at
`https://localhost:8147/sign-in`, including Pool metadata and the classification-selector fix.
Root-supplied `/private/tmp/ple-pool-metadata-connected-report.md` records ordinary Elena login,
required Title/Description denials, mixed-Subject `422` with unchanged public list, exact ordered
pins and independent list/current metadata, Student/anonymous concealed `404`, and actual Library
UI denied-create draft preservation, picker reselection, and successful creation. Browser errors
were empty. Current Revision 1 is proved; no historical Pool HTTP endpoint or journey is claimed.
Accepted actual-role SQL/concurrency and independently reviewed source remain complementary proof.

Root's no-workaround script `/private/tmp/ple-course-classification-no-workaround-20260916.mjs`
(session 85118, exit 0) uses normal Elena login and ordinary Course/Blueprint creation/editors:
saved Biology hydrates without reselection, Tags-only saves pass, both unsaved Blueprint names
survive, and browser errors are empty. Earlier broader Course browser proof remains historical
and explicitly used a reselection workaround; its independent-metadata/adoption evidence is not
represented as a newly rerun journey.

Nine narrow owning rows now close: four independent Pool metadata/Title/Description/first-member/
nonduplicated-metadata obligations and five Course-wide/exactly-one-Discipline/optional-hierarchy/
Tags/Instance-independence obligations. Current generated inventory has 996 occurrences: 450
verified, 503 open, and 43 N/A. The excluded How-to-use and Product vocabulary/glossary metadata
subtrees remain Human Guidance interpretation authority, not implementation-checklist items. Broad
shared classification, discovery, additional-member connected append, attribution, support-content,
Bloom, and Student delivery obligations remain open. No blanket Pool/Course closure.

Current local Blueprint Promoted implementation adds the searchable lineage flag, promoted-only
cursor-bound discovery, Sysadmin-only ETag/CAS mutation boundary, and a Public Blueprint Search
filter. Canonical PostgreSQL 17 bootstrap/install as `ple_migrator` and the isolated actual-role
script passed (`BEGIN`, `PASS`, `DO`, `ROLLBACK`, exit 0); its exact labelled disposable container
was removed. Root Cargo session 60804 passed `cargo check -p server_core -p project-tools --tests`
in 7.74 seconds, stricter TypeScript plus 11 Blueprint-client Node tests session 65918 passed, and
pytest session 36484 passed 7,462 tests in 5.55 seconds. These source, isolated SQL, and offline
gates do not close either Promoted row: deployed HTTP/browser integration remains unverified, and
live `8147` predates this source. No connected-current-source claim follows.

Current parallel-batch receipt (2026-09-16): root's stricter `npx tsc --noEmit -p
tsconfig.lint.json` gate and 16 focused history/disclosure/navigation Node tests passed. After the
temporary review-density harness was moved out of `tests/`, root's full
`source source_me.sh && python3 -m pytest tests/ -q` session 60790 terminally exited 0 with
7,462 passed in 5.42 seconds. The current Bloom guide was also corrected to match Human Guidance:
AI assigns both dimensions before a Question or Pool Revision enters the Library, and a Pool is
classified as a whole. This preserves the current 450 verified, 503 open, and 43 N/A inventory;
it does not establish deployed demo `8147`, connected Student proof, or global Human Guidance
closure. Receipt: `/private/tmp/ple-parallel-batch-receipt-20260916.md`.

HOTSPOT now has a reviewed raster-first implementation: owner/CAS Draft upload and source binding,
publication preparation, numeric region authoring, and a PLE-owned Student image pointer/keyboard
control. Root `cargo check -p server_core -p project-tools --tests` session 32160 passed in 2.77
seconds; root full pytest session 36235 passed 7,462 tests in 5.62 seconds; and `npx tsc --noEmit`
session 35906 passed. The 353-test Node receipt predates the final geometry/readiness fixes, though
their focused checks passed independently. Fresh canonical PostgreSQL 17 install and the isolated
actual-role `ple_auth`/`ple_app` proof passed after the helper correction, with genuine changed
successor source and exact `PQR01` stale handling. The isolated actual-component dot proof then
passed at 1280 and 390 pixels: intrinsic geometry, pointer and keyboard selection, clear and
fresh-mount restoration, saving lock, and image-replacement readiness/error reset. This is
rendered component evidence, not connected acceptance: `8147` remains pre-HOTSPOT; normal
author/save/publish, worker Pending-to-Ready activation, Student grading, connected delivery, and
published screenshot-corpus proof remain unproved. SVG remains unimplemented; its repository
fixtures are not SVG ingestion support. Receipt: `/private/tmp/ple-hotspot-isolated-dot-render-proof.md`.
The corrected screenshot scenario heading did not authorize author/publish/release activity, so no
browser rerun or PNG was published. New live support TLS acceptance also remains pending.

## Historical heading reconciliation

Pre-build cleanup receipt: root executed exactly `podman image prune -a -f`; image usage fell
from 36.84 GB to 2.712 GB, with no containers or volumes removed. Source now invokes that exact
command once before each complete managed image-build cycle, under its checkout lease, and
blocks the cycle on failure. `/private/tmp/ple-simple-prebuild-prune.md` records 78 focused
passing checks. Canonical runtime rebuild acceptance remains pending.

Historical reconciliation snapshot before the metadata exclusion used Human Guidance SHA256
`950d2712abc154e9d7591983aeaed7bb696a0c7b60a49b8d28518d59bdfad191`.
All nine existing generator part gates, identity diff, and consistency passed. The generator and
checklist then agreed on 1,038 occurrences: 441 verified, 547 open, and 50 N/A. The first-owner
inventory is 432 verified, 533 open, and 50 N/A across 1,015 distinct identities; 23 are later
duplicates. Sixty-nine new or changed requirements remain explicitly pending independent audit.
This historical receipt does not close any Human Guidance row, alter a product status, or establish
global compliance.

The prior 998-bullet snapshot (449 verified, 499 open, 490 owning-open, and 50 N/A at
`e81d5bb0a63cfb7d3ca5f4f34287e5155dc9d20b0b91cb856fdbefa1ef6fa82b`) is historical evidence,
not the current inventory. Earlier scoring, timing, Blueprint, terminal-Attempt, and bounded MATCH
receipts remain limited to their stated evidence boundaries.

Product work is in progress, not complete: the Course-classification foundation and Store HTTP
slice use mandatory Discipline with optional hierarchy and tags, independent Course Instances;
support work must preserve the exact issuer of Course authorization; and the compact Student
details UI is an independent surface. The classification foundation is owned by its storage and
Store/HTTP boundary, issuer preservation by Course authorization, and Student details by its UI
boundary. Their dependency is limited to shared authorized Course identity where a surface needs
it; none implies classification inheritance, Instance synchronization, or completion of another
slice.

The only completed test repair in this update is the required-classification fixture repair in
`tests/test_ple_question_json_authoring.mjs`: 26 tests passed. Its report is
`/private/tmp/ple-authoring-fixture-classification-repair.md`. This is not an all-tests result or
Course-feature completion.

Current bounded progress receipt: root `CARGO_INCREMENTAL=0 cargo check -p server_core -p
project-tools --tests` passed in 11.37 seconds, and `cargo tsgen` generated 369 types. A fresh
PostgreSQL 17 canonical installation through `ple_migrator` was repeated and passed. These are compiler,
generation, and canonical-install receipts only, not an all-SQL or whole-feature acceptance.
The self-contained actual-role Course-classification proof passed at
`/private/tmp/ple-course-classification-actual-role-result.log`: create/authorization, ETag
no-op/stale/history behavior, no Revision change, exact pins, 65 hierarchy tags, and fork/Instance
independence were exercised. The corrected actual-role support proof passed its constraints and
rollback at `/private/tmp/ple-support-exact-authority-result.log`; independent review accepted its
source and corrected proof. Neither SQL receipt is browser, deployed, or whole-feature acceptance.
The later frozen-source full support `--issue` run (session 38897) terminally exited 0; accepted
bounded exact-Student-scope authority, unsupported-class denial, concealment, and revocation
evidence is recorded at `/private/tmp/ple-course-support-e2e-prerequisite.md`. Current Course
fixtures and ordinary Morgan MFA were repaired without weakening authority expectations. This
prerequisite does not establish all-resource support, rebuilt Pool, or global HG acceptance.

Current connected-progress checkpoint (2026-09-16): canonical Live Demo `2914` reached `Ready`
and the old projection failure is fixed. Actual-role Course browser proof passed create/history,
exact metadata, unchanged Revision, name drafts, and Course-Instance source independence, with an
explicit Biology reselection workaround. Tags-only asynchronous selection remains defective in the
old `8075` bundle. `/private/tmp/ple-classification-async-selection-fix.md` records four blank
selections before and four correct Tags-only selections after the source correction; Tags-only
save, parent-clear, and blank validation passed. Shared-selector independent review passed with
TypeScript check, lint, and format. The next canonical rebuild remains pending.

Pool metadata source work is complete through SQL, Rust, API, and browser metadata boundaries,
but is not in running `8075`; HTTP/browser acceptance remains pending. SQL review, final Rust/API
review after the trim fix, and browser re-review after the trim/hierarchy fix passed. Fresh
PostgreSQL 17 `ple_migrator` install, `/private/tmp/ple-pool-metadata-proof.sql`, and
`/private/tmp/ple-pool-concurrency-proof.py` passed their stated install, role/rollback,
concurrent-wait, and dual-commit cases. Actual `ple_app` metadata accepted 65 Tags; the arbitrary
64-Tag cap was removed from SQL and Rust while per-Tag bounds remain. The exact disposable proof
container was stopped with `--rm` after label verification; its fixture database is gone and its
logs/tool receipts remain. Root Cargo check passed in 9.61 seconds, `cargo tsgen` generated 370
types and 348 Node tests passed. After correcting three narrower test-lane failures, root's full
`python3 -m pytest tests/ -q` run (session 22889) terminally exited 0: 7,462 passed in 6.69 seconds.
Live TLS acceptance and canonical runtime rebuild remain pending. No Pool
creation, selection, delivery, HTTP, browser, deployed, or global Human Guidance acceptance follows.

The screenshot manifest has 75 entries (the old 59 plus 16 native/WeBWorK laptop/phone scenarios)
and publishes no PNG. HOTSPOT raster asset ingest, authoring, publication preparation, and Student
control source now exist; isolated actual-component rendering passes at 1280 and 390 pixels, but
connected authoring/publication/Ready delivery, Student grading, and screenshot-corpus proof remain
absent. SVG remains an implementation gap, not a product-decision blocker; fixture SVGs do not
establish SVG ingestion support. Support E2E and new live TLS acceptance remain pending.

Student S04/S05 compact-details work has accepted independent source/render review at
`/private/tmp/ple-student-rules-disclosure-review.md`; root
`node /private/tmp/ple-student-rules-proof.mjs` exited 0 at 1280 and 390 pixels. This component
receipt does not establish the full shell, theme, zoom, or all-Student-interface acceptance.

Historical runtime/capture context (2026-09-16): the Draft classification-name component proof
and Public owner-editability source/SQL/HG review remain accepted bounded evidence. The malformed
`DO $` Live Demo/oracle repair and extracted provisioned-vocabulary `ple_migrator` block also
passed. The earlier pre-`Ready` canonical attempt, retry process 23160 projection failure, and
345-Node/7,443-pytest receipts are superseded by the current checkpoint above. The Public Blueprint
Search PNG remains absent. The 75-entry manifest contains no *new* published PNG; no stale or
retired image is represented as current evidence. No Human Guidance status or SQL-lockdown claim
follows. Historical receipt: `/private/tmp/ple-runtime-full-capture-receipt.md`.

Pre-build checkpoint (2026-09-16): scoped cleanup removed 86 untagged, undigested,
project-source-labeled images not referenced by containers (169 to 83 images; 34.16 GB to
33.03 GB), with all exact non-forcing removals returning status 0. Builder cache, named inputs,
unknown-ownership images, 49 containers, and 39 volumes were retained. The four build boundaries
share a checkout-directory lease; broad post-ready pruning is removed. Root `pytest` passed 7,448
tests in 5.47 seconds, `git diff --check` passed, and `cargo check -p project-tools --tests` passed
in 3.32 seconds. No new build has started, so this establishes no runtime Ready state, Human
Guidance status, checklist closure, or SQL-lockdown claim.

Accepted R-4 desktop/phone terminal receipts hide active navigation and visibly label three
no-response records Unanswered, incorrect `0 / 1`; the four exact MATCH pairs remain correct
`1 / 1`, total `1 / 4`. Native diagnostic `AZA01TD` / `R-5` follow-up saved/reloaded FIB, MA,
MULTI-FIB, NUM, and ORDER, then submitted the whole Attempt: four correct `1 / 1` responses,
deliberately partial-reordered ORDER incorrect `0 / 1`, total `4 / 5`. Practice-default permitted
correct answers are displayed separately from retained responses. The supplied ledgers and
`submitted-review-1280.png` / `submitted-review-390.png` under
`/private/tmp/ple-student-types-proof/` are bounded receipts, not all-eight-type, complete keyboard,
touch, or contrast acceptance. HOTSPOT and WeBWorK coverage remain open. Earlier topical
inventories/correction IDs and superseded contradictions below are historical provenance.

## Current classification prerequisite

The shared global vocabulary and its authenticated commands are installed in
[`schemas/base_schema/content_classification.sql`](../../../schemas/base_schema/content_classification.sql)
and
[`schemas/base_schema/content_classification_operations.sql`](../../../schemas/base_schema/content_classification_operations.sql).
Four UUID vocabulary tables retain mandatory Topic/Subtopic parents; the global case-insensitive
Subject index rejects case-only duplicates. Commands trim before validation, reject invalid or
overlong normalized names, create the hierarchy with role-aware authority, find a global Subject,
and list only immediate children of the supplied parent. Vetted active Instructors may create
Subjects, Topics, and Subtopics and explicitly add an existing Subject to a Discipline; only active
Sysadmins create Disciplines or replace a Subject's nonempty association set. The composite key,
foreign keys, and Subject-row lock preserve real associations. `FORCE ROW LEVEL SECURITY`, no
direct runtime DML grants, and command authorization keep the vocabulary boundary closed.

The independent fresh PostgreSQL 17 command proof was accepted. The isolated runner exited 0;
`/private/tmp/ple-classification-commands-artifacts.fZuHxB/{install,proof,concurrency-result}.log`
records installation, role-aware creation and denial, normalization and global-uniqueness checks,
parent-filtered selection, explicit existing-Subject acceptance, and Sysadmin-only replacement
with rollback on invalid input. A real two-session test shows an addition waits on replacement and
both associations persist. No hash attestation was recorded for these logs, and no live database
was modified.

This accepted SQL prerequisite is not global feature closure or deployed-state evidence. It does
not establish HTTP routes, editors, Course or Library Object content attachments, search, lifecycle,
or deployment. Existing Question free text is unchanged. No inheritance or synchronization behavior
is implied.

The current source-level next slice is implemented: authenticated selector transport/decoding,
Published Question Publish Review and bulk-metadata consumers, and Pilot/curriculum publishers use
the shared UUID classification hierarchy. Those consumers start without defaults and require
existing provisioned hierarchy identities plus authorized parent associations; they do not create
vocabulary or associations, infer classification, or fall back to a default. Publish Review and
bulk editing retain required Discipline/Subject and optional Topic/Subtopic semantics. All 42
curriculum sources and two Pilot chapters have explicit classifications. The supporting
restriction-enzyme Genetics exercise carries Biology/Genetics, so authored source input no longer
blocks a full batch; actual publication remains unverified.

`cargo check -p project-tools` passed after the publisher-recovery correction in 7.51 seconds;
shared `npx tsc --noEmit` passed earlier and `cargo tsgen` refreshed 367 types. The recorded SQL
behavioral proof remains separate. Installed-session HTTP acceptance, browser interaction,
deployment, actual imports, and global classification closure remain pending. This plan update
does not add count-audit machinery or change checklist counts.

An additional isolated HTTP/SQL receipt now establishes a narrow installed-session boundary for
the shared classification hierarchy. A fresh disposable PostgreSQL 17 and MinIO installation
running the current host binary exited 0: a vetted active Instructor and an MFA-attested Sysadmin
each read the Discipline, parent-filtered Subject, parent-filtered Topic, and parent-filtered
Subtopic selectors. Anonymous, Student, and inactive-Instructor requests received identical
concealed `404` responses. The same receipt published one native Question with explicit
Biology/Genetics metadata, then changed only its Subject to Biochemistry through bulk metadata
editing; its exact Question Revision and source binding remained unchanged. A stale metadata
request returned `412` and left the metadata unchanged. The temporary runner and artifacts are
evidence only, not permanent documentation links, and the proof made no Live Demo change.

This receipt is not browser HTTPS acceptance, a provisioned full-Course acceptance, or a rendered
selector-workflow acceptance. Those boundaries, along with deployment and global classification
closure, remain open; it adds no checklist-count or Human Guidance closure claim.

## Context

The 2026-09-14 docs pass reconciled all 295 `docs/` files against `docs/HUMAN_GUIDANCE.md` (HG).
HG is now the product authority. That pass changed documentation; code is the next step. The compliance reports
(`docs/active_plans/reports/human_guidance_compliance/`) already show the running system carries
legacy concepts: `/assignments` routes and DTOs, "Policies" editor naming, old Ribbon labels,
6:1/5:2 banner renditions, missing Student/Sysadmin Profile routes, stale generated screenshots and
Ribbon ledger. Nobody knows yet whether the code gap is 20 items or 200.

Outcome: a factual observation baseline (verbatim HG checklist, each product-behavior bullet
verified against code) first; then gap-driven correction in many small milestones sized from the
actual gaps; then re-audit of the same checklist and closeout. Audit and repair are separate
activities per HG section so the baseline survives, and repair of an audited section begins as
soon as that section lands.

## Goal

Bring the implemented PLE system into compliance with `docs/HUMAN_GUIDANCE.md`. HG defines the
destination. The manager and subagents complete the plan on their own: every gate is a command
exit code or a mechanically checked subagent report; verification uses repository evidence,
captured fixtures, synthetic records and dates, debug probes, and automated browser checks.
Prefer more, smaller milestones. Each milestone has one narrow outcome that is implemented,
verified, and closed independently.

Adopted from the external reviewer: verbatim HG copy (headings, wording, intentional duplicates);
three statuses `[x]`, `[ ]`, `N/A`; an open `[ ]` uses `Mismatch:` for missing or incorrect
behavior, or `Verification pending:` for implemented behavior awaiting named proof; audit before repair; audit split into small parallel
milestones; correction milestones generated from the gap map; completion contract per milestone;
evidence kinds (source, runtime, test); ownership convention for duplicate bullets; positive
phrasing; incremental audit-to-repair handoff; deterministic closeout; checklist reconciliation
as the acceptance mechanism.

Own conclusions: the checklist generator keeps verbatim fidelity and later HG drift visible;
audit ownership follows HG sections so independent repairs can proceed without shared-file
conflicts. The checklist is a record of product evidence, not a second product system.

## Authority order

1. `docs/HUMAN_GUIDANCE.md` wins.
2. Durable contracts in `docs/` where they agree with HG.
3. This plan coordinates work only. Current code, routes, SQL, DTOs, tests, and generated views
   are implementation evidence.

## Decision rules for findings

- When HG settles the intended behavior, implement it.
- When HG leaves design freedom, choose the simplest implementation that satisfies the stated
  behavior.
- When implementation details are missing, derive them from repository evidence and existing
  architecture; the product boundary is HG.
- When a finding appears to conflict with HG, first read the current Human Guidance compliance
  reports (`docs/active_plans/reports/human_guidance_compliance/`, especially
  `PRODUCT_CONFLICTS.md`, `COMPLIANCE_SUMMARY.md`, and `UNRESOLVED_OR_AMBIGUOUS_ITEMS.md`) for
  how the docs reconciliation interpreted that area. The reports are supporting context;
  `docs/HUMAN_GUIDANCE.md` remains authoritative.
- When two HG bullets appear to conflict, read the surrounding sections, related HG bullets, the
  docs-pass reports, and the implementation evidence before concluding that a real contradiction
  exists.
- When one reading is clearly supported by HG and the surrounding product model, record it as
  `Decision:` under the bullet, implement it, and continue.
- When two materially different product behaviors remain plausible after that review, leave the
  item `[ ]` with `Reason: product decision still unclear` and a one-line `Question:`; record
  both readings in the fresh `unresolved_or_ambiguous_items.md` (Milestone B); unrelated work
  continues.
- Record implementation behavior that conflicts with clear HG in the fresh `product_conflicts.md`.
- When HG explicitly says a design is not locked in (currently: complete Student and Sysadmin
  Ribbon layouts), verify the behavior HG does specify and leave only the unlocked design detail
  `[ ]` with `Reason: HG: no locked-in design`.
- Terminology compliance covers the surfaces people and integrations see: visible UI text,
  browser API routes and DTO field names, exported formats (canonical Blueprint JSON, CSV/TSV,
  QTI), and durable documentation. Internal identifiers (Rust modules, SQL tables and columns,
  CSS classes, test names) are implementation evidence; rename one when the correction milestone
  already rewrites that module, or when the identifier encodes a retired product model at a
  boundary the milestone changes.

## Artifacts

| Artifact | Path |
| --- | --- |
| This plan (copied in at M1) | `docs/active_plans/active/human_guidance_implementation_compliance_plan.md` |
| Checklist | `docs/active_plans/audits/human_guidance_implementation_checklist.md` |
| Audit part files (temporary) | `docs/active_plans/audits/hg_checklist_parts/NN_<section>.md` |
| Generator and checks | `devel/human_guidance_checklist.py` |
| Gap map | `docs/active_plans/audits/human_guidance_gap_map.md` |
| Fresh implementation compliance reports (eight) | `docs/active_plans/reports/human_guidance_implementation_compliance/*.md` |
| Correction milestones | appended to the plan under "Correction milestones" as the gap map grows |

Markdown under `docs/active_plans/` is exempt from the 1000-line source limit, so the checklist
may exceed HG's 1013 lines once evidence lines are added. `devel/human_guidance_checklist.py`
follows `docs/PYTHON_STYLE.md` (tabs, type hints, argparse `-b/--build`, `-d/--diff`,
`-c/--consistency`, `-g/--gate PART`).

## Checklist format

Header:

```markdown
# Human Guidance implementation compliance checklist

Source: `docs/HUMAN_GUIDANCE.md`. Human Guidance remains authoritative. This file records
implementation status only.

- [x] Verified: implemented behavior matches the bullet. Evidence follows.
- [ ] Unverified: `Mismatch:` identifies missing or incorrect behavior; `Verification pending:`
  identifies implemented behavior awaiting named proof.
- N/A: a positive audit result: the bullet makes no claim about implemented PLE behavior.
  A short reason follows, either on the bullet or as the inherited reason for its section.
```

Body: HG headings and bullets are copied exactly; product-behavior bullets initially become
`- [ ] <text>`; nested HG sub-bullets stay nested checkboxes; duplicate bullets stay duplicated.
Bullets in `## How to use this guidance` use the section-level `Implementation status: N/A` and
`Reason:` shown below, rather than invented per-bullet implementation evidence.

Status lines, indented under the bullet. Evidence carries its kind so a later reader knows what
was actually established:

```markdown
- [x] Every Account has exactly one Product Role: **Student**, **Instructor**, or **Sysadmin**.
  - Evidence (source): `schemas/base_schema/accounts.sql` `accounts.product_role` CHECK constraint.
  - Evidence (test): `tests/e2e/database_baseline_security_catalog.sql` product_role assertion.
- [x] Attempt time continues while the **Student** is disconnected or the browser is closed.
  - Evidence (runtime): probe `tests/_temp/hg_audit_09_expiry.sql` set `expires_at` in the past;
    reload returned a submitted Attempt. Oracle `tests/e2e/attempt_expiry_connected_oracle.sql`.
- [ ] **Assessment** is the PLE object for organizing Questions into a graded or practice activity.
  - Mismatch: routes, DTOs, SQL, and pages use `assignment` as the generic object
    (`src/api/contracts.ts`, `schemas/base_schema/assignments.sql`, `src/pages/assignment_*`).
- [ ] Students may use multiple passkeys across their devices.
  - Mismatch: `schemas/base_schema/authentication.sql` holds email-code ceremonies; a passkey
    credential table and WebAuthn ceremony are missing.
  - Owner: Accounts and roles > Student role (duplicate of Account rules bullet).
## How to use this guidance

Implementation status: N/A
Reason: This section gives rules for writing and maintaining Human Guidance. It does not specify
PLE product or code behavior.

- N/A Guidance bullets should start with the subject when practical, making them easier to scan.
```

Evidence kinds and locators. The checklist outlives this plan, so each locator names a backticked
path plus a stable symbol (table, column, function, type, route, test name, CSS class, heading);
a `:line` suffix is optional convenience.

- `source`: schema, Rust, or TypeScript that defines the behavior.
- `test`: a permanent test or E2E oracle that exercises it.
- `runtime`: an observation on the disposable stack or a synthetic-record probe, naming the probe
  file and what was observed.

Behavior that requires `runtime` or `test` evidence for `[x]`: grading outcomes and scoring,
Attempt expiry and resumption, retention transitions, authorization denials, backend rendering
and grading, responsive Student layouts, keyboard-only operation, banner scaling. When the probe
is still pending, the auditor marks `[ ]` with `Verification pending: needs runtime evidence` so a
later pass runs it.

Ownership of duplicate bullets: the first HG occurrence owns the finding. Later occurrences carry
the same status and a one-line `Owner: <section>` pointer. `--consistency` reports duplicates
whose statuses differ. Repair milestones are keyed to the owning bullet; when it flips, its
duplicates flip with it.

N/A rules, fixed now so all auditors agree:

- Audit every HG bullet. Mark it N/A when it governs Human Guidance itself, records human
  ownership or approval, describes an explicitly future possibility, or otherwise makes no claim
  about the implemented PLE system. N/A means "audited and not an implementation requirement," not
  "skipped." Give a short reason that makes that conclusion clear.
- `## How to use this guidance` is one N/A section: use its one `Implementation status: N/A` and
  one inherited `Reason:` for its five verbatim N/A bullets. The gate accepts that inherited
  reason; it does not require repetitive per-bullet reasons.
- N/A also includes `### Agent working principles`, ownership or approval statements
  (Mac-Studio-36G, Neil's separate logins), "may eventually" / "potential future" role bullets,
  and bullets that state the design is still open.
- Checkable: `### Codebase development rules` (line limit, snake_case, latest deps, placeholder
  tables/states/APIs/workers absent), `### PLE development rules` (default Live Demo, Genetics
  ships, global installation, launcher entry point), and everything from `## Product vocabulary`
  down.
- A vocabulary bullet is `[x]` when visible UI text, browser API routes and DTO fields, and
  exported formats use that term for that concept (terminology rule above).
- PLE development rules about installation (default Live Demo, Genetics ships, global
  installation, launcher entry point) are audited against `schemas/installation_data/`,
  `local_stack.py`, `local_stack_control/`, `launchers/`, and `containers/`; a mismatch there is
  in scope for correction.
- Stale generated evidence (screenshots, Graphify, capture manifest, Ribbon ledger) is derived
  output. When the code complies, mark `[x]` and add `Generated evidence stale:` naming the
  artifact; Milestone R1 regenerates it.

## Milestone 1: Generate the checklist

Owner: manager. Scope: the generator script, the checklist file, and the plan copy.

1. Write `devel/human_guidance_checklist.py`:
   - `--build`: read HG, drop the vendored header block, emit the checklist header, copy every
     heading verbatim, convert product-behavior `- ` bullets at any depth to `- [ ] `, and emit
     the five `## How to use this guidance` bullets as N/A beneath that section's inherited
     status and reason. Write the checklist when the path is free; when a checklist already
     exists, print its path and exit 1 so recorded evidence is preserved.
   - `--diff`: strip status markers and status lines, compare bullet text and heading sequence to
     current HG, print added/removed/changed bullets, exit 1 on drift.
   - `--consistency`: list identical bullet texts carrying different statuses; exit 1 if any.
   - `--splice PART`: validate the allowlisted audited part, then regenerate that exact checklist
     subtree from the part input. Do not hand-edit the generated checklist output.
   - `--gate PART`: require the exact named part, HG section order, headings, and verbatim
     bullets; verify every bullet has exactly one of `[x]`, `[ ]`, `N/A`; every `[x]` has an
     `Evidence (kind):` line with a real in-repository backticked path and stable symbol; every
     exact runtime-required identity named in this plan has at least one `runtime` or `test`
  evidence line; every `[ ]` has nonempty `Mismatch:` or `Verification pending:`; every N/A has `Reason:`, except the five
     `## How to use this guidance` bullets which inherit the section reason; and duplicate
     bullets have consistent status and ownership. Exit 1 on any miss.
2. Run `--build`, then `--diff` (clean). The script prints HG and checklist bullet counts; equal.
3. Copy this plan to `docs/active_plans/active/human_guidance_implementation_compliance_plan.md`.
4. Compliance report freshness: confirm the eight reports under
   `docs/active_plans/reports/human_guidance_compliance/` are the committed versions
   (`git status --short` on that folder is empty) and that `COMPLIANCE_SUMMARY.md` and
   `UNRESOLVED_OR_AMBIGUOUS_ITEMS.md` still name only the complete Student and Sysadmin Ribbon
   layouts as unlocked. When HG has changed since the reports (`--diff` against the checklist
   detects later drift; for M1 compare HG's blob hash with the reports' commit), record the
   differing bullets in the plan copy so auditors read those bullets from HG directly.
5. Runtime isolation check: inspect the launcher and its fixed Developer Browser Suite owner and
   lease before any mutating audit. Do not attempt a second launch while that suite is active.
   Mutating audits (A2, A6, A9) run serially on that one suite, and every synthetic record they
   create carries the prefix `hg_audit_<part>_`.

M1 execution record (2026-09-14): all eight docs-pass reports were clean and committed at
`9e8b992f04693e4cc8af95a67eb9dd904e852cbd`. That commit's HG blob is
`e0513fdd2af5d2943a728f7dccbc3b67e7db3164`. The current working HG blob is
`bb453652d3ed4d0ce58c85cf59f9aaab7f3399e7`: after the report comparison, exactly two Markdown destinations changed mechanically
to root-relative `/docs/NAMING_CONVENTIONS.md` and `/docs/LIVE_DEMO_SPEC.md`. No HG product bullet
wording or behavior changed. The reports leave only complete Student and Sysadmin Ribbon layouts
unlocked. The launcher has one fixed Developer Browser Suite owner/lease; its first invocation
replaced the already active fixed suite. Two suites could not coexist, and no second launch was
attempted. Therefore A2, A6, and A9 must run serially on one suite with `hg_audit_<part>_`
synthetic-record prefixes.

Gate: `--diff` exits 0; the generated checklist has exact HG part sections, order, and verbatim
bullets; every evidence locator names a real in-repository path and stable symbol; all exact
runtime-required identities have runtime or test evidence; duplicate statuses/owners are
consistent; and N/A reasons, including the inherited `How to use this guidance` reason, are
valid. Then run `pytest tests/test_pyflakes_code_lint.py tests/test_function_typing.py
tests/test_shebangs.py tests/test_markdown_links.py`. Changelog entry.

## Milestones A1-A9: Section audits (parallel observation)

Nine fresh reviewer-type subagents launched together. Each receives: its HG section text as
generated, the N/A rules, the status-line format, the evidence-kind rule, its evidence map, and
this instruction: "Audit every bullet. Mark N/A for a non-implementation requirement under the
plan's N/A rule; otherwise, mark `[x]` for behavior you verified and cite the evidence, or `[ ]`
with a plain-language mismatch. Your deliverable is your part file; the checklist records
observations and the gap map that follows records repair work."

Each auditor also receives the eight compliance report paths as context for how the docs pass
interpreted its area (context, with HG authoritative).

### Audit focus

- Audit every Human Guidance bullet.
- Focus review effort on important issues: give the most attention to bullets that affect
  correctness, product behavior, authorization, data retention, grading, persistence,
  maintainability, validation, or delivery.
- Use lightweight evidence (one source locator) for straightforward or low-risk bullets when that
  evidence establishes the behavior.
- Spend deep review time on behavior, and record wording or cosmetic differences with a single
  mismatch line.

Runtime verification: start the disposable stack (`./launchers/run_live_demo.sh`) under the
isolation result from Milestone 1 step 5, drive it with Playwright or `curl` against the seeded
demo, use synthetic records and dates through existing E2E fixtures or a
`tests/_temp/hg_audit_<part>_*` probe. Synthetic records carry the `hg_audit_<part>_` prefix so
evidence from one auditor is distinguishable from another's. Probes are deleted when the part
passes its gate. Existing `tests/`, `tests/e2e/`, `tests/playwright/`, `docs/screenshots/`, and
`docs/ux/RIBBON_DESTINATION_LEDGER.md` are evidence inputs.

| M | Part file | HG sections | Primary evidence locations |
| --- | --- | --- | --- |
| A1 | `01_development` | Development principles, PLE development rules, Product vocabulary | `tests/test_source_file_line_limit.py`, `Cargo.toml`, `package.json`, `pip_requirements*.txt`, `launchers/`, `local_stack.py`, `schemas/installation_data/`, term usage across `src/`, `crates/`, `schemas/` |
| A2 | `02_accounts` | Accounts and roles (Account rules, Instructor, Student, Sysadmin, Future roles) | `schemas/base_schema/accounts.sql`, `authentication.sql`, `authorization.sql`, `course_membership.sql`, `course_roster.sql`; `crates/server/src/auth/`, `instructor_account.rs`, `course_roster.rs`, `support_capability.rs`; `crates/learning-data-access/src/authentication_*.rs`, `session.rs`; `crates/domain/src/teaching_authority.rs`; runtime: authorization denials via `curl` |
| A3 | `03_shell` | General interface design, Ribbon and page layout, User top bar, Breadcrumbs | `src/ribbon/`, `src/application_shell.tsx`, `src/navigation/`, `src/styles/`, `src/assets/`, `src/style.css`, `docs/screenshots/`; runtime: Playwright geometry checks for Ribbon and breadcrumb stability |
| A4 | `04_instructor_ui` | Instructor interface: Courses, Blueprint Courses, Course Instances, Questions, Search, Browse, Assessments, High-consequence actions | `src/ribbon/ribbon_catalog.ts`, `src/routes.ts`, `src/pages/*course*`, `*blueprint*`, `library_*`, `question_*`, `assignment_*`, `src/features/`, `crates/server/src/question_library/` |
| A5 | `05_student_sysadmin_ui` | Student interface, Sysadmin interface | `src/pages/student_*`, `assignment_attempt_*`, `src/features/question_attempt/`, `instructor_accounts_page.tsx`, `support_roster_page.tsx`, `src/ribbon/`; runtime: Playwright at laptop, portrait tablet, narrow phone, square viewports; keyboard-only journey |
| A6 | `06_data` | Data and history, Student and FERPA data, Course retention, Retention processing, Revisions and history, Dates and time zones | `schemas/base_schema/attempt*.sql`, `corrections.sql`; `crates/server/src/worker/`; `crates/learning-data-access/src/attempt_expiry.rs`, `account_time_zone.rs`; `crates/domain/src/timing.rs`, `statistics/`; `src/log.ts`; runtime: synthetic Course dates and `expires_at` |
| A7 | `07_questions` | Questions, Draft Questions, formats and types, Native JSON, JavaScript, Question Backends, Question Pools, Question Library, identity, revisions and forks, stewardship, statistics, behavior | `crates/question_model/`, `crates/adapters/`, `crates/server/src/question_publication/`, `question_library/`, `webwork_*`; `src/question_id.ts`, `src/features/ple_question_json_authoring/`, `question_curation/`, `question_picker/`; `schemas/base_schema/question*.sql`; runtime: render and grade one fixture per backend |
| A8 | `08_courses` | Courses, Blueprint Courses (lifecycle, revisions, stewardship, adoption, forks and Change Proposals, JSON), Course Instances, Course names | `schemas/base_schema/blueprints.sql`, `course_blueprint_adoption.sql`, `course_core.sql`, `course_media.sql`; `crates/server/src/blueprint_course.rs`, `course_instance.rs`, `course_appearance/`; `crates/learning-data-access/src/blueprint_course.rs`, `course_banner.rs`, `course_theme.rs`; `src/features/blueprint_course/`, `blueprint_operations/`, `course_appearance/`; runtime: banner scaling at supported viewports |
| A9 | `09_assessments` | Assessments, content, types, appearance, Blueprint and Course Instance Assessments, Templates, release and defaults, Attempts, responses and submission, timing and expiration, Student Work, scoring | `schemas/base_schema/assignments.sql`, `attempts.sql`, `attempt_*.sql`, `grading.sql`; `crates/domain/src/effective_assignment_policy/`, `scoring.rs`, `student_feedback_release/`, `validation.rs`; `crates/grading/`; `crates/server/src/assignment_delivery/`, `assignment_release.rs`, `worker/`; `crates/export/`; `src/pages/assignment_*`, `src/ribbon/ribbon_icons.ts`; runtime: synthetic Attempts, expiry oracle `tests/e2e/attempt_expiry_connected_oracle.sql`, point-change rescoring |

Gate per part, run by the manager:

1. `devel/human_guidance_checklist.py --gate <part>` exits 0. A failing part goes to a fresh
   replacement subagent with the gate output.
2. Spot-check, risk-based: one fresh reviewer subagent re-verifies, using the cited evidence as
   its sole input, every `[x]` in a runtime-required category plus one in five of the remaining
   `[x]` items chosen at random (at least five). An unsupported `[x]` flips to `[ ]` with a
   mismatch, and the reviewer then re-verifies every other `[x]` in the part that cites the same
   evidence kind and file. A second unsupported `[x]` found that way marks the audit reasoning
   systemic: the part is re-audited by a fresh subagent with both findings attached.
3. Manager splices the part into the checklist (`_temp.py`), runs `--diff` and `--consistency`,
   and resolves duplicate conflicts by re-reading the cited evidence (stricter reading wins).
4. The part's probes under `tests/_temp/hg_audit_*` are removed. Changelog entry per part.

A part that passed steps 1-3 is baseline for its section and can feed Milestone G immediately.

## Milestone B: Fresh implementation compliance reports

The existing reports under `docs/active_plans/reports/human_guidance_compliance/` record the
completed documentation pass and remain evidence from that pass. The implementation audit writes
its own set from what it finds, so the two audits stay distinct.

Location: `docs/active_plans/reports/human_guidance_implementation_compliance/` (snake_case
filenames per `docs/REPO_STYLE.md` active-plans rule), mirroring the docs-pass set:

- `compliance_summary.md`: method, evidence kinds, per-section counts, links to the checklist
  and gap map, generated-evidence follow-ups.
- `product_conflicts.md`: implementation behavior that conflicts with clear HG, one row per
  pattern with representative owning bullets and source locations.
- `unresolved_or_ambiguous_items.md`: apparent HG contradictions and materially ambiguous
  product readings (`Reason: product decision still unclear`) plus HG-unlocked design, each with
  the two plausible behaviors and the evidence that keeps both plausible.
- `terminology_and_model_changes.md`, `authorization_and_ferpa_changes.md`,
  `question_and_assessment_changes.md`, `ui_and_workflow_changes.md`,
  `architecture_and_implementation_changes.md`: patterns found across bullets in that area, with
  the correction milestones that own them.

The checklist stays the exhaustive bullet-by-bullet record; the reports collect and explain the
important patterns across bullets. Build them from the implementation findings; the docs-pass
reports help locate known legacy areas and are cited only as context.

Timing: a fresh writer subagent produces the first set once all nine parts have landed (in
parallel with the last Milestone G runs), and a fresh subagent refreshes the set at R2 from the
re-audited checklist. A fresh reviewer subagent checks each set against the checklist.

Gate: the reports accurately summarize material unresolved product questions and implementation
patterns found in the checklist; `compliance_summary.md` states the current checklist summary;
`pytest tests/test_markdown_links.py tests/test_ascii_compliance.py` pass; changelog entry.

## Milestone G: Gap map and correction milestones (incremental)

A fresh planner subagent runs once per landed part (nine runs, each fresh) and appends to
`docs/active_plans/audits/human_guidance_gap_map.md`. Input: the part's `[ ]` items with owner
pointers resolved. For every owning `[ ]` item record: quoted bullet, current evidence, concrete
mismatch, owning source area, dependencies on other gaps (including gaps from parts not yet
audited, marked `pending`), verification method (focused test, E2E lane, synthetic fixture,
Playwright check).

From the gap map the planner writes correction milestones appended to the plan. Sizing: one code
boundary per milestone (one crate, or the frontend, or one schema file family), 1 to 8 owning
bullets, one focused gate. A gap that cuts across boundaries becomes a short dependency chain of
milestones, foundation first: exactly one milestone in the chain is the closure owner that flips
the bullet, and prerequisite milestones list the gap under `Contributes to:`. Each correction
milestone carries a completion contract:

- HG bullets closed (quoted, owning occurrences) and bullets contributed to.
- Expected behavior in one or two sentences derived from those bullets.
- Modules, tables, routes, and components touched. Each rename or identifier removal names the
  HG behavior, public contract, or misleading identifier at a changed boundary that motivates it.
- Focused gate commands and the E2E lane(s) kept green.
- Dependencies on other correction milestones and the code boundary it owns while running.

Expected areas, used as a dependency ordering hint for the planner, in order: foundational
models and terminology (Assessment as the object, identity, Revisions); authorization and
retention; Attempt behavior; grading and scoring; Questions and Pools; Question Backend
boundaries; Blueprint behavior; Course and roster; shared and Instructor interfaces; Student
interfaces; Sysadmin interfaces. The audit decides whether an area has zero, one, or many
milestones.

A fresh reviewer subagent confirms each new milestone is completable by a coder subagent with
the plan and repository alone, touches one code boundary, and stays within HG behavior; the
planner revises until the reviewer confirms every milestone.

Gate per run: every `[ ]` product item of the part has a practical implementation owner or
carries `Reason: HG: no locked-in design` or `Reason: product decision still unclear`; reviewer
confirmation clean; changelog entry.

## Correction milestones (C1, C2, ... written by Milestone G)

### C1: Split the shared frontend stylesheet

- HG bullets closed: "Every source file should stay below 1000 lines. Split complete capabilities
  into focused modules."
- Expected behavior: shared frontend styles are divided by complete capability without changing
  the rendered interface, and no tracked source file exceeds 1000 lines.
- Owned boundary: frontend shared styling, `src/style.css` and new focused `src/*.css` modules.
- Modules and components: move coherent style groups with their existing imports; preserve the
  application stylesheet entry point and every referenced CSS class.
- Dependencies: none.
- Focused gates: `source source_me.sh && python3 -m pytest tests/test_source_file_line_limit.py`;
  `node tests/e2e/e2e_ribbon_production_styles.mjs`;
  `source source_me.sh && ./launchers/run_fast_checks.sh`.

### C2: Add a focused development-conformance inventory

- HG bullets closed: "Use readable `snake_case` whenever possible; see
  [NAMING_CONVENTIONS.md](/docs/NAMING_CONVENTIONS.md) for details."; "Do not create or leave
  placeholder database tables, states, APIs, workers, or compatibility scaffolding before the
  feature has an approved product design."
- Expected behavior: a focused repository inventory identifies source names that are not readable
  `snake_case` unless an external contract requires them, and identifies product placeholders
  unless an approved design records their purpose. The milestone creates
  `devel/development_conformance_audit.py` as the one blocking audit command.
- Owned boundary: repository development-conformance tooling in `devel/`.
- Modules, tables, routes, and components: add `devel/development_conformance_audit.py` and its
  documented allowlist/approved-design input; do not rename public routes, DTOs, SQL tables, or
  external contracts in this milestone.
- Dependencies: none.
- Focused gates: `source source_me.sh && python3 devel/development_conformance_audit.py --check`;
  `source source_me.sh && ./launchers/run_fast_checks.sh`. These are executable after this
  milestone creates the named audit command. The coder uses a disposable failing fixture under
  `tests/_temp/` before running the gate, then removes it at closeout.
- Failure and recovery: the C2 coder investigates every reported path. Correct a source name or
  remove unsupported scaffolding when it is not contract-bound; otherwise add the smallest
  documented allowlist/approved-design record naming the external contract or design owner. Rerun
  `source source_me.sh && python3 devel/development_conformance_audit.py --check` until clean;
  the C2 reviewer rejects broad or unexplained allowlists.

### C3: Establish dependency freshness across manifest families

- HG bullets closed: "Cargo, Node, and PyPI dependencies should use the latest versions to include
  security fixes."
- Expected behavior: dependency freshness is checked consistently for Cargo, Node, and PyPI
  declarations using a recorded registry snapshot, and supported security releases are adopted.
  The milestone creates `devel/dependency_freshness_audit.py` as the one blocking freshness
  command.
- Owned boundary: dependency freshness tooling in `devel/`.
- Modules, tables, routes, and components: `Cargo.toml`, `package.json`, `pip_requirements*.txt`,
  and `devel/dependency_freshness_audit.py`; preserve package-manager lockfile and install
  contracts.
- Dependencies: none.
- Focused gates: `source source_me.sh && python3 devel/dependency_freshness_audit.py --check`;
  `source source_me.sh && python3 -m pytest tests/test_crate_boundaries.py -k latest_first`;
  `source source_me.sh && ./launchers/run_fast_checks.sh`. These are executable after this
  milestone creates the named audit command. The coder uses a disposable stale-manifest fixture
  under `tests/_temp/` to prove failure, then removes it at closeout.
- Failure and recovery: the C3 coder investigates each stale dependency against the recorded
  registry snapshot, upgrades the applicable manifest and lockfile, and reruns that ecosystem's
  package check. If an upgrade is blocked by an incompatible supported dependency, record the
  exact blocker and supported security release in the audit input for reviewer approval; do not
  silently suppress it. Rerun `source source_me.sh && python3 devel/dependency_freshness_audit.py --check` until clean; the C3 reviewer verifies every
  exception is narrow and dated.

### C4: Classify non-product conditional development guidance correctly

- HG bullets closed: "Adaptability should be a focus so the software can evolve as requirements
  and insights change."; "If an interface is measured as too slow, consider moving the slow code
  to Rust/WebAssembly."
- Expected behavior: these bullets are recorded as audited `N/A` because they are not separately
  closable product behaviors. They remain binding review constraints: Course, Assessment,
  Question Backend, retention, and authorization work must favor adaptable boundaries and simple
  domain concepts over speculative machinery. A later measured performance issue remains eligible
  for a focused implementation plan.
- Owned boundary: Human Guidance checklist audit classification in
  `docs/active_plans/audits/hg_checklist_parts/01_development.md` and the generated checklist.
- Modules, tables, routes, and components: no production module, table, route, or component is
  changed. Add concise inherited or per-bullet N/A reasons only.
- Dependencies: none.
- Focused gates: `source source_me.sh && python3 devel/human_guidance_checklist.py --consistency`;
  `source source_me.sh && python3 devel/human_guidance_checklist.py --gate 01_development.md`.

### C5: Classify Live Demo priority as human-owned guidance

- HG bullets closed: "The polished PLE Live Demo is the top priority; see
  [LIVE_DEMO_SPEC.md](/docs/LIVE_DEMO_SPEC.md)."
- Expected behavior: the checklist records this project-priority statement as `N/A`, not as an
  unverifiable runtime requirement; Live Demo behavior remains audited through its actual HG
  product bullets.
- Owned boundary: Human Guidance checklist audit classification in
  `docs/active_plans/audits/hg_checklist_parts/01_development.md` and the generated checklist.
- Modules, tables, routes, and components: no production module, table, route, or component is
  changed.
- Dependencies: none.
- Focused gates: `source source_me.sh && python3 devel/human_guidance_checklist.py --consistency`;
  `source source_me.sh && python3 devel/human_guidance_checklist.py --gate 01_development.md`.

### C6: Enforce reusable Blueprint Course boundaries

- HG bullets closed: "**Blueprint Course**: A reusable course used to create **Course Instances**.
  It has no enrolled **Students** or deadlines."
- Contributes to: C7 Course Instance creation from a Blueprint.
- Expected behavior: Blueprint Course data is reusable content only and cannot own roster or
  deadline data; adoption creates a separate Course Instance without mutating reusable content.
- Owned boundary: Blueprint schema and adoption operations,
  `schemas/base_schema/blueprints.sql` and `schemas/base_schema/course_blueprint_adoption.sql`.
- Modules, tables, routes, and components: Blueprint tables, constraints, and adoption procedures;
  no frontend rename is included.
- Dependencies: A8 Blueprint findings may add prerequisites before this milestone starts.
- Focused gates: `source source_me.sh && cargo test -p server_core blueprint_course`;
  `bash tests/e2e/e2e_live_demo_blueprint_course.sh --service`;
  `source source_me.sh && ./launchers/run_fast_checks.sh`.

### C7: Complete Course Instance teaching lifecycle storage

- HG bullets closed: "**Course Instance**: A course used for teaching. It has **Students**,
  deadlines, releases, and other course settings. It may be created from a Blueprint Course or
  started empty."
- Expected behavior: C7 may now implement the explicit `Empty | Adopted` Course root and its
  independent teaching state (memberships, deadlines, releases, and Course settings). The C503
  tagged direct-versus-adopted Assessment origin is now in `schemas/base_schema/assessments.sql`,
  and `ple_data.create_assessment` creates direct-origin rows with no Blueprint source. Accepted
  private actual-main/HTTP proof created an Unreleased direct Practice Assessment in a newly
  browser-created Empty Course. Follow-up accepted actual-main/HTTP proof added and saved one
  available Published Question's exact Revision pin, set a valid Due/time limit through Properties,
  passed release readiness, and released that direct Assessment. Accepted isolated actual-server
  and exact-main Student proof imported and claimed a real roster invitation, showed that Released
  direct Assessment and its title/Type/count/points/one-hour limit/zero-prior-Attempts overview to
  the member, omitted an Unreleased sibling, and denied an outsider. A further isolated actual-
  server/native run published the checked-in PKU Question through Draft authoring, saved its
  correct opaque-choice response, reopened it, whole-submitted a graded 1/1 Attempt, and started a
  distinct second unlimited Practice Attempt. This proves one direct fixed native Question path;
  Pool content, full Student browser Attempt interaction, other backend rendering/grading, and the
  full started-empty teaching lifecycle remain unverified.
- Current bounded contribution: the Rust/browser creation wire now uses strict `Empty | Adopted`,
  and accepted private actual HTTP plus exact-main browser proof created and persisted an Empty
  Instructor Course with zero Blueprint-list requests and Student denial. A separate Public
  Blueprint exact-Revision creation and adoption then produced a daughter Course with one
  Unreleased Practice Assessment, fixed Question Revision, finite Attempt limit, and unset dates.
  The two exact creation alternatives and the creation-only composite are verified. Direct
  started-empty Assessment creation and one fixed-Question add/save/release are additionally proven
  through the actual form/HTTP boundary. Roster import/claim and bounded Released Student
  visibility/pre-start facts are additionally proven. One real native fixed Question response/
  grade and unlimited-after-perfect retry are additionally proven by actual HTTP. Pool copying,
  complete Student browser Attempt interaction, other backend delivery/grading, and the complete
  teaching lifecycle are not. C503's remaining content work remains a dependency for full C7
  closure.
- Owned boundary: Course schema family, `schemas/base_schema/course_core.sql`,
  `schemas/base_schema/course_membership.sql`, `schemas/base_schema/course_roster.sql`, and
  `schemas/base_schema/course_operations.sql`.
- Modules, tables, routes, and components: Course Instance tables, constraints, and procedures;
  adoption reads the C6 boundary without rewriting Blueprint data.
- Dependencies: C6 contributes the reusable Blueprint boundary. C49/C72 make only Public
  Blueprints eligible for the Adopted branch; they do not block Empty-root/state work. C503 -> C7
  is required before C7 closes the complete started-empty authoring/delivery behavior. A8 course
  findings may add prerequisites before this milestone starts.
- Focused gates: `source source_me.sh && cargo test -p server_core course_instance`;
  `bash tests/e2e/e2e_live_demo_course_instance.sh --authority`;
  `source source_me.sh && ./launchers/run_fast_checks.sh`.

### C8: Complete the validated Question Library contract

- HG bullets closed: "**Published Question**: A validated question in the global **Question
  Library**, available to vetted **Instructors**."; "**Question Library**: The global collection
  of Published Questions and published Question Pools available to vetted **Instructors**."
- Expected behavior: valid published Questions and published Question Pools appear in one global
  library only for vetted Instructors; invalid, Draft, and unpublished records are excluded.
- Owned boundary: Question authoring schema contract,
  `schemas/base_schema/question_authoring_state.sql`,
  `schemas/base_schema/question_authoring_operations.sql`, and related
  `schemas/base_schema/question_*.sql` files.
- Modules, tables, routes, and components: publication validation facts, Question/Pool library
  entries, and SQL authorization boundary; frontend presentation stays unchanged.
- Dependencies: Accounts-and-roles vetting corrections from A2 are pending.
- Current receipt: independent review and a fresh root PostgreSQL 17 rerun accepted Course-fixed
  exact Pool members, stable IDs, authorization, and retired/inactive handling. Connected HTTP
  and Cargo execution remain pending the AWS Smithy dependency cutover.
- Focused gates: `source source_me.sh && cargo test -p server_core question_library`;
  `bash tests/e2e/e2e_live_demo_question_library.sh --api`;
  `source source_me.sh && ./launchers/run_fast_checks.sh`.

### C9: Enforce Draft Question privacy and publication validation

- HG bullets closed: "**Draft Question**: A private question being developed by an **Instructor**.
  It must pass validation before publication."
- Expected behavior: only the authorized Instructor can access a Draft, and the publication
  service rejects invalid Draft content before it reaches the global library.
- Owned boundary: server Question publication service,
  `crates/server/src/question_publication.rs` and its focused tests.
- Modules, tables, routes, and components: Draft authorization and publication-validation service
  methods; SQL library validation facts are consumed from C8 rather than duplicated.
- Dependencies: C8; Accounts-and-roles authorization corrections from A2 are pending.
- Focused gates: `source source_me.sh && cargo test -p server_core question_publication`;
  `bash tests/e2e/e2e_live_demo_question_library.sh --api`;
  `source source_me.sh && ./launchers/run_fast_checks.sh`.

### C10: Complete Sysadmin role capabilities

- HG bullets closed: "**Sysadmin**: A PLE administrator who manages the system, approves
  **Instructors**, creates accounts, and helps manage courses."
- Expected behavior: Sysadmin authorization covers account creation, Instructor approval, and
  defined system/course-management actions, while other roles are denied those actions.
- Owned boundary: Accounts authorization schema and API, `schemas/base_schema/accounts.sql`.
- Modules, tables, routes, and components: role constraints and `ple_api` account operations; C7
  supplies Course Instance operations rather than duplicating them.
- Dependencies: A2 Accounts and roles corrections are pending; C7 contributes the course action.
- Focused gates: `source source_me.sh && cargo test -p server_core instructor_account`;
  `bash tests/e2e/e2e_live_demo_instructor_accounts.sh --service`;
  `source source_me.sh && ./launchers/run_fast_checks.sh`.

### C11: Complete vetted Instructor Question workflows

- HG bullets closed: "**Instructor**: An approved user who teaches courses and can browse, reuse,
  create, fork, and publish Questions."
- Expected behavior: a vetted Instructor can perform each stated Question workflow through the
  browser, while an unapproved account is denied at the API boundary.
- Owned boundary: frontend Question workflow feature, `src/pages/question_*.tsx` and
  `src/api/question_*.ts`.
- Modules, tables, routes, and components: Question Library browse, Draft creation, reuse/fork,
  and publish UI/API clients; retain the C8/C9 server contracts.
- Dependencies: C8, C9, and Accounts-and-roles approval corrections from A2.
- Focused gates: `node --test tests/test_question_picker.mjs tests/test_question_availability_client.mjs`;
  `bash tests/e2e/e2e_live_demo_question_library.sh --browser`;
  `source source_me.sh && ./launchers/run_fast_checks.sh`.

### C12: Rename the public Assessment editor boundary

- HG bullets closed: "**Student**: A user enrolled in a **Course Instance** who completes
  Assessments and other course activities."; "**Assessment Question Editor**: The **Instructor**
  editor for selecting, adding, removing, and ordering Questions in an Assessment.";
  "**Assessment Properties Editor**: The **Instructor** editor for settings that apply to the
  whole Assessment, such as dates, scoring, attempts, late work, and what **Students** can see."
- Expected behavior: visible UI text, browser routes, and DTO fields use Assessment rather than
  Assignment; Students complete Assessments and Instructors use the named Question and Properties
  editors without losing question ordering or policy behavior.
- Owned boundary: frontend Assessment public boundary, `src/pages/assignment_workspace/`,
  `src/api/`, and its browser routes and DTO decoders.
- Modules, tables, routes, and components: rename each changed public route/DTO/UI identifier only
  where it encodes the retired product model; migrate Question Editor and Properties Editor pages,
  paths, models, and browser clients together.
- Dependencies: C7 for enrollment behavior; A9 Assessment behavior corrections are pending.
- Focused gates: `node --test tests/test_assignment_workspace_questions.mjs tests/test_assignment_workspace_policy_model.mjs`;
  `bash tests/e2e/e2e_live_demo_course_instance.sh --browser`;
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
  Student View remains unavailable and is verified by C45/C74 rather than by a deleted test.

### C13: Make Live Demo entry authentication explicit

- HG bullets closed: "Email is not configured for the Live Demo yet; use the visible seeded-role
  entry for demo access."
- Expected behavior: the existing Live Demo exposes the visible seeded-role entry and does not
  deliver email-code authentication, despite seeded demonstration email data.
- Owned boundary: verification only. Inspect but do not modify `crates/server/src/auth/live_demo.rs`,
  `src/pages/live_demo_auth_model.ts`, or `src/pages/live_demo_auth.css` for this milestone.
- Dependencies: none.
- Focused gates: `node --import tsx --test tests/test_live_demo_auth_ui.mjs`; then ignored
  `bash tests/_temp/hg_c13_live_demo_auth.sh --service`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain the disposable probe only if its seeded inputs
  and visible entry contract are deterministic. If it fails, report the mismatch to the owning
  authentication/UI milestone; never add an email flow merely to make the proof pass.

### C14: Require institutional email for roster identity

- HG bullets closed: "**Students** are required to use their university or institutional (`.edu`
  in the USA) email accounts."; "Roster import uses institutional email to find an existing
  Student Account or create one when needed."
- Expected behavior: a US `.edu` address is accepted only when its canonical domain ends in
  `.edu` (not a lookalike suffix), and its import resolves an existing global Student Account or
  creates exactly one. Rejected input creates no Account, Student Record, membership, invitation,
  or audit-side effect. The non-US institutional-domain rule remains the product question stated
  in C800-C862 below.
- Owned boundary: C801 owns `crates/learning-data-access/src/course_roster.rs` validation; C802
  owns the existing lookup-or-create transaction and its named seed/oracle/E2E/browser/capture
  consumers. No new Account type.
- Dependencies: C801 -> C802.
- Focused gates: C801's exact inventory command in C800-C862; then
  `bash tests/e2e/e2e_live_demo_roster.sh --import`; `bash tests/e2e/e2e_live_demo_roster.sh --browser`;
  `bash tests/e2e/e2e_live_demo_support_capability.sh --issue`; and
  `bash tests/e2e/e2e_live_demo_support_capability.sh --browser`. Capture only under the existing
  Browser Suite lease.
- Permanent-gate decision and failure plan: the lookup-or-create/no-side-effect transaction is a
  permanent E2E candidate because it protects stable identity and data integrity; browser and
  screenshot migration proof remains temporary unless it independently satisfies
  `docs/PYTEST_STYLE.md`. On failure, preserve transaction evidence and repair validation or the
  lookup/create transaction without weakening the no-side-effect assertion.

### C15: Require stronger Sysadmin authentication

- HG bullets closed: "**Sysadmin** accounts should require higher security than other accounts,
  like TOTP authentication"
- Expected behavior: the recorded TOTP decision is implemented vertically: C803 PostgreSQL keeps
  the wrapped encrypted seed and atomically consumes one unused short-lived Account/browser-bound
  attestation while issuing a session; C804 creates pending MFA, verifies the 30-second counter,
  replay protection, and rate limit before the final database gate; C805-C807 make the Live Demo
  use the same Morgan Delgado pending-MFA ceremony without a fixed/shared secret. Student and
  Instructor authentication remains unchanged.
- Owned boundary: C803-C807 in C800-C862 below. The Store owns typed calls only; it does not
  decide role or session policy.
- Dependencies: recorded durable decision -> C803 -> C804 -> C805 -> C806 -> C807.
- Focused gates: C803 `source source_me.sh && cargo test -p learning-data-access --features postgres --lib`;
  C807 `npx playwright test tests/playwright/e2e/auth_authorization.spec.ts` after local controller
  provisioning; then `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain only stable final-gate/one-use authorization
  and role-outcome tests. Real seeds, codes, controller diagnostics, and local artifacts stay
  ignored. A failure repairs seed encryption, pending state, verification/replay/rate limiting, or
  the database gate; it never weakens the factor requirement.

### C16: Prove Instructor deactivation preserves records

- HG bullets closed: "Instructor Accounts may be deactivated without deleting their authored
  content, Course relationships, or historical records."
- Expected behavior: deactivation blocks new access while retaining all named authored,
  relationship, and history data.
- Owned boundary: account lifecycle schema, `schemas/base_schema/accounts.sql`.
- Modules, tables, routes, and components: account state transition and its database preservation
  guarantees; do not change Course or Question lifecycle behavior.
- Dependencies: none.
- Focused gates: `source source_me.sh && cargo test -p server_core instructor_account`; then
  `bash tests/e2e/e2e_live_demo_instructor_accounts.sh --service`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: extend the existing Instructor-account service E2E
  with authored-content, relationship, and history preservation assertions. This protects an
  irreversible data-loss boundary. If it fails, inspect the deactivation transaction and foreign
  key/delete effects, repair the data behavior, and retain the test; use a temporary synthetic
  fixture only while isolating the failure.

### C17: Record completed Instructor identity vetting

- HG bullets contributed to: "A **Sysadmin** vets an Instructor's real identity before creating
  the Instructor Account."; "Sysadmins vet **Instructors** and create Instructor Accounts."
- Expected behavior: a completed, immutable, auditable vetting decision can be referenced by the
  later account-creation boundary.
- Owned boundary: account identity schema, `schemas/base_schema/accounts.sql`.
- Modules, tables, routes, and components: vetting decision data, constraints, and audit facts;
  no account-creation HTTP behavior in this prerequisite.
- Dependencies: none.
- Focused gates: `source source_me.sh && cargo test -p server_core instructor_account`; then
  `bash tests/e2e/e2e_live_demo_instructor_accounts.sh --service`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: extend the existing Instructor-account service E2E
  with vetting-decision immutability and invalid-decision denial. The audit trail is a stable
  account-authorization contract. If it fails, repair the schema constraint or service mapping
  that admits an invalid decision; retain the assertion and keep any SQL reduction in
  `tests/_temp/`.

### C18: Gate Instructor creation on completed vetting

- HG bullets closed: "A **Sysadmin** vets an Instructor's real identity before creating the
  Instructor Account."; "Sysadmins vet **Instructors** and create Instructor Accounts.";
  "Sysadmins approve Instructors before they receive Instructor capabilities."
- Expected behavior: only a Sysadmin presenting a completed identity-vetting decision can create
  an Instructor Account; an unapproved Instructor has no Instructor capability; and the resulting
  action is auditable.
- Owned boundary: Instructor-account creation service, `crates/server/src/instructor_account.rs`.
- Modules, tables, routes, and components: creation request, validation, and audit receipt; consume
  C17's schema fact rather than duplicating it.
- Dependencies: C17.
- Focused gates: `source source_me.sh && cargo test -p server_core instructor_account`; then
  `bash tests/e2e/e2e_live_demo_instructor_accounts.sh --service`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: add missing, invalid, completed-vetting, and
  pre-approval capability-denial cases to the existing Instructor-account E2E. These are stable
  authorization-denial behavior (ASVS 2.2.1, 2.3.1, and 5.3.2). If one fails, repair request
  validation or the authorization handoff rather than loosening the test; remove any temporary
  endpoint probe from `tests/_temp/` at closeout.

### C19: Browse Public and Archived Blueprint content

- HG bullets closed: "**Instructors** can browse the content of Public and Archived **Blueprint
  Courses**."
- Expected behavior: a vetted Instructor can browse Public and Archived Blueprint content without
  becoming an owner or gaining adoption/edit authority; Archived content remains non-adoptable.
- Owned boundary: Blueprint Course server routes, `crates/server/src/blueprint_course.rs`.
- Modules, tables, routes, and components: browse route and its authorization policy only; do not
  alter Blueprint persistence or lifecycle definitions.
- Dependencies: C6. C6 is the reusable Blueprint boundary; C7 follows C6 for Course Instance
  creation and is not a prerequisite for this read-only browse route.
- Focused gates: `source source_me.sh && cargo test -p server_core blueprint_course`; then
  `bash tests/e2e/e2e_live_demo_blueprint_course.sh --service`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: extend the existing Blueprint Course service E2E with
  Public/Archived non-owner browse and Archived non-adoption cases. This is a stable lifecycle and
  authorization contract. If it fails, repair route policy or lifecycle enforcement and retain the
  case; use `tests/_temp/` only for a short-lived ownership matrix reproduction.

### C20: Reset and restore Student Course access

- HG bullets closed: "An **Instructor** can reset Student login access and send a new signup code
  when needed."; "An **Instructor** can restore the Student's Course access later."
- Expected behavior: an authorized Instructor can invalidate prior Student signup access, send one
  fresh signup code through the delivery seam, and later restore the original Course access without
  creating an Account or erasing records.
- Owned boundary: course-roster server routes, `crates/server/src/course_roster.rs`.
- Modules, tables, routes, and components: reset and restore handlers and their existing delivery
  seam; do not modify retention storage.
- Dependencies: C14, C23.
- Focused gates: `source source_me.sh && cargo test -p server_core course_roster`; then
  `bash tests/e2e/e2e_live_demo_roster.sh --import`; then
  `bash tests/e2e/e2e_invitation_mailer.sh`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: extend the existing roster E2E with reset/restore
  cases and retain the existing invitation-mailer oracle for delivery. The behavior affects
  durable access recovery and must not regress. On failure, identify whether invitation invalidation,
  delivery export, or membership restoration is wrong, repair that boundary, and retain the case;
  discard temporary delivery experiments from `tests/_temp/`.

### C21: Store the Course Instance retention schedule

- HG bullets contributed to: "Student Work, Attempts, submissions, and grades follow Course
  retention independently of the Student Account."; "Removing a **Student** from a Course revokes
  future Course access but does not immediately delete the Student's Course records or Student
  Work."; "Student Work and grades remain subject to the normal Course retention policy after
  enrollment ends."; "Deactivating Course access does not delete the Student Account or Student
  Work."
- Expected behavior: the existing Course-core facts remain the single authority from which later
  Course-governed retention processing can decide independently of the global Student Account.
- Owned boundary: C808 audits existing `schemas/base_schema/course_core.sql`,
  `ple_data.course_instance`, `created_at`, `active_until_at`, `latest_assessment_due_at`,
  `retention_starts_at`, lifecycle fields, and their current triggers/procedures once. It adds or
  moves no retention state and closes no HG bullet. C206 consumes these facts for the new policy
  schedule/due actions; C809/C207 own Assessment synchronization and deadline enforcement.
- Dependencies: C808 supplies audited facts to C206; C21 closure remains its retained-data
  behavior and is not implied by C808 or C809.
- Focused gates: read-only schema/call-site inventory plus ignored fresh-schema creation-anchor
  probe; then `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: the audit/probe is temporary. If it finds the current
  Course-core invariant false, repair only that authoritative invariant; never add a parallel
  retention schedule.

### C22: Process retained Student Course data

- HG bullets closed: "Student Work, Attempts, submissions, and grades follow Course retention
  independently of the Student Account."; "Student Work and grades remain subject to the normal
  Course retention policy after enrollment ends."
- Contributes to: the predictable-purge portion of "**Student** data should be collected
  reluctantly, used deliberately, and purged predictably." The active
  `/root/student_data_minimization` owner applies the same principle to collection and use through
  the simplest existing category-and-operation boundaries; it does not wait for a field-level
  allowlist design decision.
- Expected behavior: idempotent retention processing purges the defined Student Course data on the
  Course policy schedule without deleting the global Student Account or unrelated Course metadata.
- Owned boundary: server retention worker, `crates/server/src/worker.rs`.
- Modules, tables, routes, and components: one Course-retention sweep iteration alongside the
  existing Attempt-expiry sweep, its narrowly scoped store capability, and its scheduler entry
  point; consume C21 retention facts without redefining them.
- Dependencies: C21.
- Focused gates: `source source_me.sh && cargo test -p server_core worker`; then
  `bash tests/e2e/e2e_live_demo_worker_topology.sh --service`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: add controlled-clock, late-run, and repeat-run Course
  retention cases to the existing worker test module and worker topology E2E. The idempotent
  deletion schedule is a durable privacy behavior. If a case fails, repair the worker/store
  transaction or idempotence guard and retain it; any one-off database timing reproduction stays
  in `tests/_temp/` and is removed at closeout.

### C23: Preserve Course records through roster revocation

- HG bullets closed: "Removing a **Student** from a Course revokes future Course access but does
  not immediately delete the Student's Course records or Student Work."; "Deactivating Course
  access does not delete the Student Account or Student Work."
- Contributes to: C20 Student Course-access restoration.
- Expected behavior: revoking a roster relationship denies future Course access but retains the
  global Student Account and Course records through the retention boundary.
- Owned boundary: roster relationship operations, `schemas/base_schema/course_operations.sql`.
- Modules, tables, routes, and components: revoke/restore-compatible data operation and its access
  predicate; do not implement retention processing.
- Dependencies: C21.
- Focused gates: `bash tests/e2e/e2e_live_demo_roster.sh --import`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: extend the existing roster E2E with revocation
  access-denial plus Account/Student-Work preservation assertions. This guards against irreversible
  data loss. If it fails, repair the revoke transaction or deletion guard and retain the case;
  keep any direct SQL diagnosis only in `tests/_temp/`.

### C24: Separate Sysadmin administration from Course-record access

- HG bullets closed: "A **Sysadmin** has full administrative authority over PLE."
- Contributes to: C26 FERPA-safe support and C25 scoped repair capabilities.
- Expected behavior: Sysadmin platform-administration authority is durable and distinct from
  Course membership and FERPA record-read authority.
- Owned boundary: Sysadmin authorization schema, `schemas/base_schema/authorization.sql`.
- Modules, tables, routes, and components: platform-administration authorization predicates only;
  no support-capability route changes.
- Dependencies: none.
- Focused gates: `source source_me.sh && cargo test -p learning-data-access --features postgres --lib`;
  then `bash tests/e2e/e2e_live_demo_support_capability.sh --issue`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: extend the existing authorization contract/E2E with
  platform-administration allowance and absent-Course-read denial. This is a stable least-privilege
  boundary. If it fails, repair the authorization predicate or capability role and retain the
  assertion; a temporary matrix probe belongs in `tests/_temp/`.

### C25: Issue scoped Sysadmin repair capabilities

- HG requirement: "Sysadmins can help Instructors repair Courses, Students, and content."
- Contributes to: C26 FERPA-safe support and no implied Instructor/Course membership.
- Current implementation scope: Student roster support only. Course and content repair remain open
  implementation requirements, not Human-Guidance-approved exclusions.
- Expected behavior: explicitly requested, purpose-limited, time-scoped capabilities authorize
  the implemented Student-roster repair and record its use; later Course and content scopes need
  their own authorized resource boundaries before they can be accepted.
- Owned boundary: support-capability server routes, `crates/server/src/support_capability.rs`.
- Modules, tables, routes, and components: capability request, resource-class validation,
  revocation, and audit receipt; consume C24 authorization predicates.
- Dependencies: C24.
- Focused gates: `source source_me.sh && cargo test -p server_core support_capability`; then
  `bash tests/e2e/e2e_live_demo_support_capability.sh --issue`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Current proof state: the corrected actual-role proof passed constraints and rollback at
  `/private/tmp/ple-support-exact-authority-result.log`; independent review accepted the source
  and corrected proof. Each resource class still needs its own success gate: the implemented,
  evidenced slice is Student roster only, while Course and content remain open. Durable HTTP
  support E2E did not run and helper issues are still being fixed; no browser or deployed
  acceptance follows.
- Permanent-gate decision and failure plan: extend the existing support-capability E2E with each
  resource class, purpose, expiry, revocation, and audit receipt. These are durable scoped-support
  guarantees. If it fails, repair capability validation or audit recording and retain the case;
  remove any exploratory support request from `tests/_temp/`.

### C26: Enforce FERPA-safe Sysadmin support boundaries

- HG requirements: "Student Course data falls under FERPA; treat it as radioactive.";
  "**Sysadmins** have full platform-administration capability but do not automatically have access
  to FERPA Course records."; "Sysadmin support does not make the Sysadmin an **Instructor** or
  Course member."
- Current implementation scope: the Student-roster capability boundary only. Course and content
  support remain open implementation requirements, not Human-Guidance-approved exclusions.
- Expected behavior: ordinary Sysadmin administration cannot read FERPA Course records; an audited,
  task-scoped Student-roster capability permits only its authorized records and never changes role
  or Course membership. Later Course/content support requires separate least-privilege proof.
- Owned boundary: support authorization enforcement,
  `schemas/base_schema/course_operations.sql`.
- Modules, tables, routes, and components: support-capability checks, audit facts, and
  role/membership non-escalation constraints; no broader administration route work.
- Dependencies: C24, C25.
- Focused gates: `source source_me.sh && cargo test -p server_core support_capability`; then
  `bash tests/e2e/e2e_live_demo_support_capability.sh --issue`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Current proof state: the corrected actual-role proof passed constraints and rollback at
  `/private/tmp/ple-support-exact-authority-result.log`; independent review accepted the source
  and corrected proof. It evidences Student-roster scope only. Durable HTTP support E2E did not
  run and helper issues are still being fixed; no browser or deployed acceptance follows.
- Permanent-gate decision and failure plan: extend the existing support E2E with absent-capability
  FERPA denial and no-role/no-membership-escalation assertions. The boundary prevents material
  privacy and authorization regression. If it fails, repair the SQL authorization check or role
  transition and retain the test; temporary capability probes remain in `tests/_temp/` only.

### C27: Prove Sysadmin desktop workflows

- HG bullets closed: "Instructor and **Sysadmin** workflows should work well in a 1280 by 800 desktop browser viewport."
- Expected behavior: Sysadmin completes Instructor Accounts and Scoped Support Roster workflows at 1280 by 800 without horizontal overflow or inaccessible required controls.
- Owned boundary: Ribbon browser evidence, `tests/playwright/ui_corpus_manifest.ts` and `tests/playwright/ribbon_m9_responsive_evidence.mjs`. Dependencies: none.
- Focused gates: `node tests/playwright/ribbon_m9_responsive_evidence.mjs`; temporary self-starting `bash tests/_temp/hg_a3_c27_sysadmin_desktop_workflows.sh`; `bash tests/e2e/e2e_live_demo_instructor_accounts.sh`; `bash tests/e2e/e2e_live_demo_support_capability.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain only stable role/viewport behavior; remove the temporary stack probe. Repair geometry or the unreachable named control and rerun both live lanes.

### C28: Complete keyboard reordering

- HG bullets closed: "Reordering must also have a precise keyboard-accessible method."
- Contributes to: A3-16 drag-and-drop decision.
- Expected behavior: every reorderer in Blueprint entries, Assignment Workspace entries, and native JSON editors has a labelled keyboard operation with the same saved result as pointer controls.
- Owned boundary: frontend reorder controls in `src/features/blueprint_course/blueprint_assignment_content_editor.tsx`, `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx`, and `src/features/ple_question_json_authoring/`. Dependencies: none.
- Focused gates: temporary `node tests/_temp/hg_a3_c28_keyboard_reorder_inventory.mjs`; `node --import tsx --test tests/test_blueprint_course_model.mjs tests/test_ple_question_json_editor_model.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain stable keyboard behavior tests only; delete the inventory probe. Repair the omitted control mapping before implementing any selected drag behavior.

### C29: Present Grassland by its habitat name

- HG bullets closed: "Themes should use biome and habitat names."
- Expected behavior: stored `grass` stays compatible while visible labels say Grassland.
- Owned boundary: `src/features/course_appearance/course_theme_registry.ts`. Dependencies: none.
- Focused gates: `node --import tsx --test tests/test_course_theme_scope.mjs`; `node tests/playwright/ribbon_m9b_density_evidence.mjs`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain the stable visible-label test; repair presentation labels only if it fails.

### C30: Keep UUIDs out of presentation seams

- HG bullets closed: "UUIDs should never appear in visible content, navigation URLs, or copyable links."
- Expected behavior: visible text, hrefs, and copy/share/export values use canonical public references, never internal UUIDs.
- Owned boundary: `src/navigation/public_route.ts`, `src/navigation/resolved_route.ts`, `src/route_contract.ts`, and inventory-discovered consumers. Dependencies: A4/A7/A8/A9 presentation inventories.
- Focused gates: temporary `node tests/_temp/hg_a3_c30_uuid_presentation_inventory.mjs` over checked-in consumers `src/route_contract.ts`, `src/navigation/public_route.ts`, `src/navigation/resolved_route.ts`, `src/ribbon/route_scope_controller.ts`, `src/api/question_search_query.ts`, and every current `.tsx` `href`/copy producer; `node --import tsx --test tests/test_public_navigation.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain the public-navigation contract, delete the inventory probe; trace and replace any failing API-to-presentation UUID seam.

### C31: Bundle Atkinson Hyperlegible Mono

- HG bullets closed: "Use Atkinson Hyperlegible Mono for code and other monospace text."
- Expected behavior: official local Mono assets and `@font-face` declarations supply the monospace token.
- Owned boundary: `src/styles/browser_fonts.css`, `src/style.css`, and local font assets. Dependencies: none.
- Focused gates: temporary `node tests/_temp/hg_a3_c31_mono_font_computed_style.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: promote only a stable font-source contract; delete exploration and repair asset/token wiring if it fails.

### C32: Deny Student upload requests at the API boundary

- HG bullets contributed to: "Students should have no upload capabilities. Instructor-created content should use text boxes."
- Expected behavior: every existing A2-inventoried educational-content upload endpoint denies Student authority independently of the UI; this milestone does not create avatar routes.
- Owned boundary: existing server upload authorization routes identified by A2. Dependencies: A2 upload-endpoint inventory.
- Focused gates: `source source_me.sh && cargo test -p server_core upload_authorization`; temporary `bash tests/_temp/hg_a3_c32_existing_upload_denial.sh`; `./check_rust.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain denial tests as a durable authorization boundary; delete exploratory requests. C33 is the closure owner.

### C33: Use text controls for Instructor-created content

- HG bullets closed: "Students should have no upload capabilities. Instructor-created content should use text boxes."
- Expected behavior: A2-approved educational-content surfaces use text controls, while C32 denies Student uploads.
- Owned boundary: frontend authoring controls identified by the inventory. Dependencies: C32 and A2 inventory.
- Focused gates: temporary `node tests/_temp/hg_a3_c33_authoring_text_control_inventory.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: delete the inventory unless it becomes a stable public contract; repair the specific authoring surface, not profile/media settings.

### C34: Give every role a home dashboard

- HG bullets closed: "Each Product Role has its own home dashboard and navigation."
- Expected behavior: Student, Instructor, and Sysadmin have explicit home routes and matching Ribbon models.
- Owned boundary: `src/route_contract.ts`, `src/routes.ts`, `src/ribbon/ribbon_schema.ts`, and `src/ribbon/ribbon_contract.ts`. Dependencies: A2 role facts.
- Focused gates: temporary `node --import tsx tests/_temp/hg_a3_c34_role_home_routes.mjs`; `node tests/playwright/ribbon_m9_responsive_evidence.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain the stable role-home route contract, delete the temporary matrix, and repair role/Ribbon derivation on failure.

### C35: Render the shared generic Profile affordance

- HG bullets closed: "Profile appears at the far right as an icon-only avatar."; "The Profile avatar uses a generic user avatar until the user selects another avatar."
- Expected behavior: each signed-in role has one accessible, icon-only far-right Profile control with generic fallback.
- Owned boundary: `src/ribbon/app_ribbon.tsx`, `src/ribbon/app_ribbon.css`, and `src/application_shell.tsx`. Dependencies: C34.
- Focused gates: temporary `node tests/_temp/hg_a3_c35_three_role_profile_affordance.mjs`; `node tests/playwright/ribbon_m9_responsive_evidence.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain the shared accessible-control contract only; delete matrix exploration and repair common Ribbon composition on failure.

### C36: Move account actions into the Profile menu

- HG bullets closed: "Clicking the Profile avatar opens the Profile menu."; "Sign Out belongs in the
  Profile menu rather than the main top bar."
- Contributes to, but does not close: "The Profile menu contains Profile settings, account
  settings, and Sign Out." and the account-action portion of "Avoid scattering related actions
  across page headers, menus, navigation, and content areas."
- Expected behavior: Profile opens an accessible menu and Sign Out is not standalone in the main
  top bar. C819-C823 must supply real Profile Settings and recorded-scope Account Settings
  destinations before the broader menu-content/no-scattering bullets can close.
- Owned boundary: `src/ribbon/app_ribbon.tsx`, `src/ribbon/app_ribbon.css`, and existing shell action seam. Dependencies: C35.
- Focused gates: `node --test tests/test_ribbon_contract.mjs`; `node tests/playwright/ribbon_m10_shell_evidence.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain stable pointer/keyboard/focus and Sign-Out
  relocation behavior; delete temporary visual exploration and repair focus/action dispatch on
  failure. Never add disabled substitutes.

### C37: Store approved role-neutral avatars

- HG bullets contributed to: A3-29 through A3-32.
- Expected behavior: `docs/DESIGN_DECISIONS.md` "Account avatars are one role-neutral aggregate" requires `account_avatar` in `schemas/base_schema/profile_media.sql` to store closed `ProvidedAvatarId` or self-owned image through one discriminated shape.
- Owned boundary: profile-media schema family, `schemas/base_schema/profile_media.sql`. Dependencies: the durable design decision named above.
- Focused gates: ignored `bash tests/_temp/hg_c37_profile_avatar_schema.sh --postgres17`; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: the PostgreSQL 17 fresh-container fixture (including
  administrator `dblink` setup and trap cleanup) is temporary. It proves Student provided-avatar
  select/replay/upload denial, staff self-owned finalization, other-account concealment,
  replacement cleanup, and a two-session finalization race. Repair declarative/transaction
  invariants before C38-C42 begin; do not borrow the fixed Browser Suite database.

### C38: Expose typed account-avatar persistence

- HG bullets contributed to: A3-29 through A3-32.
- Expected behavior: typed stores preserve C37's discriminated avatar shape without cross-account access.
- Owned boundary: `crates/learning-data-access` profile store modules, after C812's sole typed
  `ProfileImage` object-address bridge removes the live Instructor-only ProfileThumbnail path.
  Dependencies: C37, C812, C833. C833 supplies generated catalog facts; this row does not copy
  catalog literals into the store.
- Focused gates: ignored C812 bridge probe; `source source_me.sh && cargo test -p learning-data-access --features postgres --lib`; `./check_rust.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain exactly the typed private/signed/serde/path
  test because it prevents exposure or a wrong physical address, and the legacy `profileThumbnail`
  JSON-rejection test because it prevents a second current-image path. Remove the temporary bridge
  proof after C38 acceptance. On failure restore ProfileImage-only privacy, address, and signing
  behavior and repair the owning migration; never re-enable legacy deserialization.

### C39: Authorize self avatar APIs

- HG bullets contributed to: A3-29 through A3-32.
- Expected behavior: self-derived `/api/profile/avatar*` routes deny Student image upload and
  keep Instructor/Sysadmin image delivery self-only. Generated provided-avatar catalog conformance
  belongs to C834 and its sole route/UI consumer C836, not this self-image authorization boundary.
- Owned boundary: `crates/server` profile routes. Dependencies: C38.
- Focused gates: `source source_me.sh && cargo test -p server_core profile_avatar`; `bash tests/e2e/e2e_live_demo_profile_avatar.sh --authorization`; `./check_rust.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain the behavior-level role/IDOR E2E because the
  self-only public-avatar authorization boundary is intentionally stable, important, and plausibly
  regressive. Parser and normalization permutations remain temporary and are removed. A failure
  means public-avatar authorization regressed: repair route/store policy before HG closure.

### C40: Deliver the Student avatar picker

- HG bullets closed: "**Students** select avatars from a PLE-provided collection and cannot upload Profile images."; "Student avatar selection should be visual and playful, similar to choosing a LEGO avatar."
- Expected behavior: after C836 integrates the reusable picker on real `/profile`, Student selects
  only a provided visual avatar and sees no image-upload control. C40 remains the sole closure
  owner for both Student bullets; C832-C836 are contributors and do not flip checklist status.
- Owned boundary: Student profile acceptance and its real route integration. Dependencies: C836.
- Focused gates: temporary `node tests/_temp/hg_a3_c40_student_avatar_picker.mjs`; `bash tests/e2e/e2e_live_demo_profile_avatar.sh --student-upload-denial`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain stable selection behavior only; remove picker exploration.

### C41: Deliver Instructor and Sysadmin avatar settings

- HG bullets closed: "**Instructors** and **Sysadmins** may select a provided avatar or add their own Profile image."
- Expected behavior: after C836 integrates the reusable picker on real `/profile`, both roles
  select a provided avatar or their authorized self image through C39. C41 remains the sole
  closure owner; C832-C836 are contributors and do not flip checklist status.
- Owned boundary: Instructor/Sysadmin profile acceptance and its real route integration. Dependencies: C836.
- Focused gates: temporary `node tests/_temp/hg_a3_c41_staff_avatar_settings.mjs`; `bash tests/e2e/e2e_live_demo_profile_avatar.sh --self-delivery`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain stable role-capability tests; delete exploration.

### C42: Project the current avatar everywhere

- HG bullets closed: "The current avatar appears consistently anywhere PLE represents that user."
- Expected behavior: C837 first projects generic/provided static assets across every representation
  surface. A private staff Profile image remains C39 self-only. C42 cannot close until the C837
  terminal product question resolves what cross-account delivery, if any, is authorized.
- Owned boundary: shared frontend identity-avatar projection acceptance. Dependencies: C837 and
  the recorded C837 privacy decision; provided static assets may appear cross-account before that
  decision, but private Profile images may not.
- Focused gates: temporary `node tests/_temp/hg_a3_c42_avatar_projection_matrix.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain stable projection behavior only; repair the omitted surface and remove temporary inventory.

### C43: Make breadcrumbs permanent and link-complete

- HG bullets closed: "All signed-in users have a permanent breadcrumb row below the top Ribbon."; "The breadcrumb row remains in the same location and keeps the same space as users navigate."; "Breadcrumbs show the path from the user's home dashboard to the current page."; "Each breadcrumb level links back to its corresponding page."
- Expected behavior: signed-in routes reserve a role-home-rooted breadcrumb row and every level, including current, links to its route with current `aria-current="page"`.
- Owned boundary: `src/application_shell.tsx`, `src/ribbon/ribbon_contract.ts`, `src/ribbon/route_scope_controller.ts`, and `src/route_contract.ts`. Dependencies: C34 and A4/A5/A8/A9 route inventories.
- Focused gates: temporary `node tests/_temp/hg_a3_c43_breadcrumb_route_matrix.mjs`; `node tests/playwright/ribbon_m10_shell_evidence.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain stable breadcrumb route behavior; delete matrix probes and repair route model/link projection on failure.

### C800-C903: Accepted cross-boundary corrections

These rows supersede the narrower/contradictory details in C13-C15, C21, C24, C28, C36-C38,
C54, C203, C206, and C207. Each has one owner and one gate. Temporary work stays ignored under
`tests/_temp/` unless it independently earns permanent status under `docs/PYTEST_STYLE.md`.

| ID | Owner and outcome | Boundary / dependency | Gate and lifetime |
| --- | --- | --- | --- |
| C800 | C13 verification only: prove existing seeded-role entry and no email-code delivery; no production/UI edit. | `tests/test_live_demo_auth_ui.mjs`; inspect `auth/live_demo.rs` and live-demo UI only. | `node --import tsx --test tests/test_live_demo_auth_ui.mjs`; ignored `hg_c13_live_demo_auth.sh --service`. |
| C801 | C14 US `.edu` validator contributor: canonical `.edu`, no lookalike suffix, no Account creation. | `crates/learning-data-access/src/course_roster.rs`; C802 consumes it. | Exact inventory: `rg -n -i 'live-demo\.invalid|m17\.support|screenshot\.support|@[^[:space:],;]+\.edu|Email, roster ID' schemas/installation_data/live_demo.sql schemas/installation_data/live_demo_oracle.sql schemas/installation_data/prepublication_context.sql crates/learning-data-access/src/course_roster.rs crates/learning-data-access/src/postgres/course_roster.rs crates/server/src/course_roster.rs src/api/course_roster.ts src/api/decoders/course_roster.ts src/pages/course_roster_page.tsx src/pages/roster_import_template.ts tests/e2e/e2e_live_demo_roster.sh tests/e2e/e2e_live_demo_support_capability.sh tests/playwright/e2e_live_demo_roster_browser.mjs tests/playwright/e2e_live_demo_support_capability_browser.mjs tests/playwright/screenshot_corpus/scenarios_instructor.ts tests/playwright/screenshot_corpus/scenarios_student.ts tests/playwright/screenshot_corpus/scenarios_sysadmin.ts tests/playwright/capture_live_demo_screenshots.mjs tests/playwright/screenshot_corpus/cli.ts docs/screenshots/current_capture_manifest.json`; temporary malformed-address probe first. |
| C802 | C14 lookup-or-create owner: migrate `schemas/installation_data/live_demo.sql`, `live_demo_oracle.sql`, `prepublication_context.sql`, roster/support E2E and browser expectations (including `m17.support`), and Instructor/Student/Sysadmin screenshot scenarios (including `screenshot.support`); prove reuse, exactly-one create, and rejected-input no side effect. | `course_roster` data/server/API/pages; exact capture chain `tests/playwright/capture_live_demo_screenshots.mjs -> tests/playwright/screenshot_corpus/cli.ts -> scenarios_instructor.ts/scenarios_student.ts/scenarios_sysadmin.ts -> docs/screenshots/current_capture_manifest.json`; depends C801. | `bash tests/e2e/e2e_live_demo_roster.sh --import`; `bash tests/e2e/e2e_live_demo_roster.sh --browser`; `bash tests/e2e/e2e_live_demo_support_capability.sh --issue`; `bash tests/e2e/e2e_live_demo_support_capability.sh --browser`; lease-gated capture. Transaction E2E is permanent candidate; browser/capture proof temporary. |
| C803 | C15 database owner: encrypted-at-rest wrapped TOTP seed, role-derived atomic one-use Account/browser-bound attestation/session gate; Store typed calls only. | `authentication.sql`, `authentication_ceremony.rs`, PostgreSQL adapter; recorded TOTP decision. | Fresh-schema encryption/attestation/session fixture; `cargo test -p learning-data-access --features postgres --lib`. Stable final-gate/one-use test only. |
| C804 | C15 server owner: primary authentication -> pending MFA; verify 30-second counter, replay, rate limit; invoke C803 gate. | `auth.rs`, `auth/browser_boundary.rs`, `auth/session_cookie.rs`; depends C803. | Existing auth E2E plus temporary controlled-secret probe; retain only role outcome. |
| C805 | C15 Live Demo owner: Morgan selection creates pending MFA/no session through C804's same ceremony, never a separate demo-only mode. | `auth/live_demo.rs`, `live_demo_auth_model.ts`, `live_demo_auth.css`; C800/C804. | `node --import tsx --test tests/test_live_demo_auth_ui.mjs`; lease-gated browser evidence. |
| C806 | C15 local-controller owner: OS-CSPRNG seed, encrypted persistence, ignored mode-0600 path-only artifact, separate authenticator. | local stack/controller fixture boundary; C803-C805. | Ignored provisioning probe; never permanent and never logs/fixtures seed or code. |
| C807 | C15 connected-integration owner: pending/no-session, wrong-code denial/rate limit, genuine-code session, replay denial, ordinary roles unchanged. | `tests/playwright/e2e/auth_authorization.spec.ts`; C803-C806. | `npx playwright test tests/playwright/e2e/auth_authorization.spec.ts`; retain only deterministic outcome proof. |
| C808 | C21 evidence-only owner: audit existing `course_core.sql` creation anchor/current facts once; add or move no retention state and close no bullet. | `course_instance.created_at`, `active_until_at`, `latest_assessment_due_at`, `retention_starts_at`, lifecycle fields and call sites. | Read-only inventory and ignored fresh-schema probe. C206 consumes facts. |
| C809 | A6 C207 contributor: atomic Assessment save/release synchronizes the current latest Due date and rejects a Due date past the Active limit. | `course_core.sql`, `assessments.sql`, private `assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline`, `student_assessment_landing.sql`, `grading_access.sql`, `install.sql`, and canonical Live Demo seed/oracle; C206/C207. | Expanded ignored PostgreSQL 17 proof covers all three save APIs, release, stale CAS, wrong authorization, cap rollback, deterministic concurrency, archive race/freeze, helper ACL, and seeded current MAX; remove after acceptance. |
| C810 | C24 command correction only. | `learning-data-access` PostgreSQL authorization contract. | `cargo test -p learning-data-access --features postgres --lib`. |
| C811 | C37 PostgreSQL proof owner: fresh labelled PostgreSQL 17 baseline, install, administrator `CREATE EXTENSION dblink`, fixture, trap cleanup; never fixed Browser Suite DB. | `e2e_database_baseline.sh`, baseline compose, install/object/profile schema, `hg_c37_profile_avatar_schema.sql`. | Ignored `hg_c37_profile_avatar_schema.sh --postgres17`: Student select/replay/upload denial; staff self image; other-account concealment; cleanup; two-session race. |
| C812 | C38 prerequisite: one typed ProfileImage object bridge and no second live ProfileThumbnail path. | new model `ProfileImageReference`; `ObjectAddress::ProfileImage`, ProfileImage class, private content, `profiles/images/{image}/{object_id}`; remove old thumbnail address/store/router. | Retain exactly typed private/signed/serde/path coverage against exposure/wrong physical address and legacy `profileThumbnail` JSON rejection against a second current-image path. Remove the temporary bridge proof after C38 acceptance; failure restores ProfileImage-only privacy/address/signing and repairs the owning migration, never legacy deserialization. |
| C813 | C54 frontend contributor: small centered responsive 5:1 banner, no source validation/rendition choice. | course-appearance frontend/API consumers; depends C815. | Temporary 1280-by-800/narrow viewport proof. |
| C814 | C54 inventory owner: inventory model, server, data access, objects, `course_media.sql`, frontend, E2E/Playwright consumers before dimensions change. | Exact `rg -n 'CourseBannerRendition|CourseBanner|course_banner|normalized_course_banner'` over those families. | HG requires 5:1 and recommends 1280x256; exact rendition/minimum is unlocked pending decision. |
| C815 | C54 server owner: accept valid still 5:1 within safety bounds, including 1280x256/higher and smaller valid 5:1; refuse non-5:1; no crop. | course appearance server/image validation/store/schema consumers; C814. | Temporary small/1280/2560/non-5:1 fixtures; course-appearance E2E and Playwright. 5:1/no-crop is durable candidate. |
| C816 | C28 command correction only. | keyboard reorder tests. | `node --import tsx --test tests/test_blueprint_course_model.mjs tests/test_ple_question_json_editor_model.mjs`. |
| C817 | C203 evidence reconciliation: permanent real-session BOLA/FERPA outcome oracle replaces removed temporary probe. | `authorization.sql`; `crates/learning-data-access/tests/assignment_access_postgres.rs`. | Existing `bash tests/e2e/e2e_database_baseline.sh`; failure retains D05 and repairs session/predicate/ownership boundary. |
| C818 | C36 narrow interaction owner: avatar opens menu and Sign Out is relocated; contributes, not closes, contents/no-scattering. | Ribbon component/CSS/contract; C35. | Ribbon contract plus `ribbon_m10_shell_evidence.mjs`; retain pointer/keyboard/focus/relocation behavior. |
| C819 | Role-neutral Profile Settings authorization: self-derived time zone and C39's self-only avatar capability only; no catalog selection or avatar closure. | account time-zone store and account-profile server seam; C39. | PostgreSQL/lib and server self/other-denial probe; remove unless a stable authorization outcome earns promotion. |
| C820 | Role-neutral `/profile` page/route, migrating rather than duplicating Instructor Profile; generic identity/time-zone only until C836. | route contract/routes/new profile page/API; C819. | Ribbon-route and ribbon-contract tests; temporary role matrix. |
| C821 | Recorded Account Settings scope decision: all roles receive only self-derived time-zone read/write at `/account-settings`. Account Settings exposes no credential lifecycle. HG-required Student/Instructor passwordless authentication and multiple Student passkeys remain Accounts-and-roles milestone-owned. Only self-service credential enumeration/revocation/re-authentication, identity-proofed recovery/notification, and session termination remain unresolved pending a separate decision. | `docs/DESIGN_DECISIONS.md` "Account Settings is one self-only time-zone preference"; no credential-lifecycle implementation. | Decision is satisfied; no implementation evidence or HG closure is implied. |
| C822 | All-role `/account-settings` page/route consumes C821's recorded self-only time-zone behavior; invitations remain separate. | route contract/routes/new account-settings page/API; C821. | Closed-shape self-only behavior plus Ribbon route/contract tests; no fake/disabled controls. |
| C823 | Final Profile menu composition: real `/profile`, `/account-settings`, Sign Out together; sole closure owner for menu contents and account-action no-scattering. | Ribbon/route contract; C818, C820, C822. | Ribbon route/contract and M10 shell tests; lease-gated capture. |
| C824 | C311 evidence owner: inventory every biologyproblems.org WeBWorK problem family, record its provenance/license, canonical algorithmic author source, and closed `source_format` (`pg` or `pgml`), and map each family to its generated static QTI/PG variants. `pgml` is allowed only for fully PGML-compliant source and uses `.pgml`; traditional or mixed source is `pg` and uses `.pg`. This is import metadata, not a runtime parser or separate backend. The accepted inventory now records 42 canonical PGML sources (41 official biologyproblems-website sources plus HLA); it creates no Question lineage, Pool, Blueprint, archive, or historical rewrite. | `docs/TODO.md`; `content/genetics/manifest.yaml`, `content/genetics/sources/`, `content/genetics/pg/`; hands C838. | A temporary independent validator accepted all 42 manifest registrations, including source-format/path, local and upstream SHA, license, and legacy-bank mapping; its 42-source renderer/lint/whitelist and representative seed/grading evidence passed and was removed. A missing canonical source is an implementation gap to repair, not authority to retain static copies. |
| C825 | C300 model contributor: tagged `Static` versus `Seeded { question_seed, generated_parameter_sha256 }` reproduction evidence; native presentation omits seed and descriptor checksum is v4, while nonce remains only for PLE-controlled response/choice order. | `docs/CONTRACTS.md` "Student Work and assessment evidence"; `crates/question_model`; recorded "Native PLE JSON attempt reproduction is seed-free" decision; C300. | Ignored model/presentation fixture; retain only a stable public-representation contract. |
| C826 | C300 adapter contributor: `adapter_ple::issue_question_json(&source)` has no seed or parameter hash. | `crates/adapters/ple`; C825. WeBWorK/iMathAS retain seeded interfaces. | Focused adapter test and ignored migration probe; no call-order test. |
| C827 | C300 schema contributor: nullable seed/hash physical pair with CHECK; PLE both NULL and WeBWorK/iMathAS both populated; triggers/APIs enforce. | attempts/operations/interaction/presentation/finalization schema; C825. Preproduction direct base correction/reinitialize; no compatibility views. | Ignored fresh PostgreSQL 17 pair/trigger/API fixture; always temporary. |
| C828 | C300 LDA contributor: tagged types/codecs/read/write/resume preserve C827's backend distinction. | LDA attempt reproduction codecs; C827. | Focused codec/read-resume test; retain only stable externally meaningful resume behavior. |
| C829 | C300 server contributor: mint seed only for `Seeded` source backend; native delivery/finalization has no seed and accepts no browser seed. | `crates/server` assignment delivery/finalization; C826,C828. | Ignored server delivery/finalization proof; no sentinel. |
| C830 | C300 browser contributor: browser API and TypeScript decoders reject legacy native seed fields and expose no public seed. | browser API contract and TypeScript decoders; C829. | Closed-shape decoder/API probe; retain stable public rejection only. |
| C831 | C300 sole closure: connected fresh-PG17/server proof that native rows/payload/resume have no seed/hash, seeded backends retain server-only seed, and invalid PLE seed insertion fails. | C830; hands C304,C323,C331,C332,C309. | Ignored connected fixture first; promote only one small deterministic native no-seed outcome contract if `PYTEST_STYLE.md` approves. |
| C832 | Avatar-catalog contributor: add canonical provided-avatar assets as original safe SVGs, `avatar_catalog` manifest, provenance, and one deterministic safe generator deriving Rust, TypeScript, and SQL registry artifacts. | Asset source, manifest/provenance, generator and generated registry outputs; no profile route or selection behavior. | Ignored source/provenance/generator determinism and safe-SVG probe; render every catalog asset with the SVG rendering skill at smallest, typical, and largest picker tiles, recording dimensions and accessible name/decorative status. Repair source, generator, or metadata and remove the probe. |
| C833 | Avatar-catalog schema contributor: seed generated catalog facts into `profile_media.sql` with `is_selectable`; a retired asset stays renderable for existing `account_avatar` rows but is not selectable for new rows. | C37 profile-media shape and C832 generated SQL; fresh PostgreSQL 17 only. | Ignored fresh-PG17 fixture proves seed id/checksum correspondence, unknown-ID refusal, selectable-only write, and retired-existing render/no-new-select behavior; repair declarative/transaction invariants, then remove. |
| C834 | Avatar-catalog server/domain contributor: consume only the generated registry, enforce catalog conformance, and deny unknown or retired selection while allowing a previously selected retired asset to render. | generated Rust/SQL registry from C832; C833; C836 is its sole route/UI consumer. | Ignored domain/server contract probe for unknown, retired, selected, and render-only cases; retain no generated-catalog snapshot or call-order test. |
| C835 | Avatar UI contributor: provide reusable `ProvidedAvatarPicker` and `AvatarVisual` components from C832's generated TypeScript registry data, with keyboard selection, visible name/text alternative, and playful visual grid. The pure picker accepts only `currentAvatarId` and `onSelect(id)` props; it makes no API call, has no page or route, consumes no C834 server shape, and closes no HG bullet. | generated TypeScript registry from C832 only; no duplicated catalog literals. | Ignored component accessibility/render probe and SVG-skill three-tile render evidence; remove after handoff. Promote only a stable user-visible keyboard/accessible-name contract if it independently passes every `PYTEST_STYLE.md` question. |
| C836 | Avatar UI integration contributor: integrate C835 on the real all-role `/profile` route only after C819/C820; Student gets provided-avatar selection with no upload control, while Instructor/Sysadmin get provided selection plus their C39 self-image capability. It contributes the actual UI to C40/C41; C40/C41 retain their respective HG closure attribution and acceptance gates. | C39, C819, C820, C834, C835. No alternate profile route or fake control. | Ignored three-role route/browser probe plus Student upload-denial and staff self-delivery E2E; repair role gates/route wiring, remove probes, then run C40/C41 closure evidence. |
| C837 | C42 contributor and terminal privacy question: project generic/provided static avatars through `AvatarVisual` on every representation surface; private Instructor/Sysadmin Profile images remain self-only under C39 and must not be projected cross-account. | C35, C39, C835, C836; C42 remains the sole A3-32 closure owner after the question is resolved. | Ignored representation-surface inventory and self/provided rendering matrix; remove after handoff. **Question:** May a private Instructor or Sysadmin Profile image be delivered to another authenticated PLE user when PLE represents that account, and if so to which roles/surfaces and under what authorization? HG does not resolve this; until a decision, only provided static assets may render cross-account. |
| C838 | C311 shipped-catalog input and publication owner: define the small canonical Genetics manifest from the 42 C839-accepted canonical PGML sources (41 biologyproblems-website sources plus HLA), their provenance/license, nine nonempty topics, and direct Fixed entries. It creates ordinary available WeBWorK Question lineages only; it creates no implicit variant Pool. This is no parser and no separate backend. | Canonical `content/genetics/pg/topicNN/` sources and minimal Genetics manifest; ordinary publication only: `schemas/base_schema/question_authoring_operations.sql`, `crates/server/src/question_publication.rs`; C824. | The focused fresh-install proof validates the manifest inputs and ordinary publication, then reads the created canonical Blueprint. It has no catalog-migration, static-row equivalence, historical ID/revision/pin rewrite, or Pool-retirement claim. |
| C839 | C311 canonical-source acceptance owner: the shipped fresh Genetics catalog contains exactly 42 accepted canonical PGML sources: 41 official biologyproblems-website sources plus HLA. Acceptance records source/provenance and the deterministic render/grading contract for those sources; it is not row-for-row equivalence to static expansions. The 43rd Chargaff source is unaccepted. | C838's manifest inputs; hands fresh install and, only for a retained database lineage, C840. | The 76 banks and 13,434 old generated rows are an explicit non-published import inventory, not the active install manifest. They are unresolved and neither imported nor complete. Do not restore static expansions or claim their publication, migration, or retirement. |
| C840 | C311 retained-catalog migration owner, not a fresh-baseline requirement: only when an actual existing database retains Genetics lineages, Blueprint pins, or Student Work, reconcile through the current ordinary Revision and availability operations with expected-current CAS. Preserve every historical pin and Student Work record; a stale CAS makes no publication mutation. | Existing retained database records; current Question, Blueprint Revision, availability, and CAS operations; C839. | The disposable fresh baseline has no existing catalog to reconcile and needs no Pool-retirement subsystem. Do not infer or create implicit Pools. A retained-lineage migration must use ordinary forward Revisions/availability, never deletion or historical rewrite. |
| C841 | C311 retained-catalog recovery owner, not a fresh-baseline requirement: after a successful C840 CAS operation, recover any retained static lineage only through ordinary availability and a later Revision while preserving Blueprint and Student Work pins. | Existing retained database records and C840; no fresh-install manifest change. | No persistent database reset is authorized by this plan edit. Any reset requires verified disposable state, confirmation that no durable data exists, and the existing operator command. Never silently drop pins or present a retained-catalog migration as completed without its actual-database proof. |

**C838 status (2026-09-16): ACCEPTED.** The Rust direct-Fixed fresh publisher and static review
are accepted. Four focused Cargo curriculum tests and full-manifest validation pass. The accepted
isolated PostgreSQL 17/MinIO proof in `/private/tmp/ple-fresh-genetics-artifacts.m8Jn51`, driven
by `/private/tmp/ple-fresh-genetics-proof.sh`, published 42 Questions and 42 Question Revisions
across nine Assessments, with 42 Fixed entries and zero Pools. An exact canonical replay made no
mutations. A conflicting same-short-name catalog was rejected before mutation. Long-name handling
remains static-review-only; the accepted fresh-install proof does not claim it.

This accepts the fresh shipped catalog only. C839 accepts the 42 canonical PGML sources (41
official biologyproblems-website sources plus HLA); Chargaff remains unaccepted. The 76 banks and
13,434 old generated rows remain unresolved non-published import inventory. This does not close
the whole C311 cohort, retained-catalog migration/recovery, static-row equivalence, historical
ID/revision/pin rewrite, or Pool retirement. Live Demo installation/provisioning remains separate;
no screenshot evidence is claimed here.

#### C311 current Genetics install boundary

The active fresh-install scope is exactly the 42 C839-accepted canonical PGML sources,
their nine nonempty topics, and direct Fixed entries. It creates the canonical Genetics
Blueprint when absent and makes an exact canonical replay idempotent. A same-name conflict
or static catalog fails before any publication mutation. The 43rd Chargaff source and the
76 banks/13,434 old generated rows are unresolved non-published import inventory, not active
install inputs. C840-C841 apply only if an actual retained database has lineages, Blueprint
pins, or Student Work. The fresh baseline has none, so it requires neither migration nor a
Pool-retirement subsystem. No lineage, Pool, Blueprint pin, or Student Work may be silently
dropped.
| C842 | Question-ID foundation contributor: make `crates/question_model/src/question_library.rs` the one Rust contract input and add `devel/generate_question_id_contract.py`, which generates the browser-safe constants/normalization contract at `generated/api/QuestionIdSyntaxContract.ts`. Wire its check command, `source source_me.sh && python3 devel/generate_question_id_contract.py --check`, into the existing generation front door. `src/question_id.ts` consumes that output; no hand-maintained second alphabet, length, grouping, alias map, or regex remains. Identity is the seven compact characters excluding compact index 4; no browser secret or HMAC implementation. | DD "Public Question and Pool IDs"; `docs/QUESTION_ID_SPEC.md`; hands C843,C369,C844. This owns model input, generator, generated output, and browser-safe syntax contract only: it does not allocate IDs or resolve them on the server. | Ignored model/generator/output conformance probe covers canonical compact/display shape and alias normalization; run the generation check; remove after handoff. No generated snapshot becomes permanent. |
| C843 | Question-ID schema/live-demo contributor: make the fresh base schema and Live Demo fixtures use only canonical compact IDs and add `devel/preflight_question_id_cutover.py --migration-database`. Before any reinitialization, that deployment-prep command inspects the preexisting `ple_data.published_question` relation through the migration connection: a missing relation or zero rows permits `cargo tools database initialize`; any nonzero count exits nonzero, prints only the stop/escalate result, and never begins rebuild. | C842; fresh preproduction schema only; hands C369,C845. | Ignored fresh-PG17/deployment-prep fixture proves both an empty/missing relation permits the subsequent initialize command and a nonzero existing `published_question` row halts before it; remove after handoff. No legacy conversion or rewrite path. |
| C844 | Question-ID browser-codec contributor: use C842's generated syntax contract at browser entry/copy/display seams and transmit only canonical display IDs. | C842,C369; hands C845,C846. This excludes the active/accepted C358 `crates/server/src/question_library.rs` boundary and its tests. | Ignored browser codec matrix proves documented aliases/case normalize and malformed input fails before request; remove after handoff. Retain no implementation-coupled decoder snapshot. |
| C845 | Question-ID fixture-sweep contributor: replace stale Question-ID literals only in named executable fixtures, seed data, docs examples used as executable inputs, and server/browser contract fixtures with canonical values. | C843,C844,C358; hands C846. It waits for and excludes C358's `crates/server/src/question_library.rs` boundary and its tests; it neither edits nor inventories that active/accepted workstream. | Ignored bounded fixture inventory plus focused fixture load proves the owned executable inputs are canonical; remove. A textual absence check is never permanent. |
| C846 | Question-ID integrated proof contributor: on a fresh schema, prove mint, compact storage, canonical display/serde, documented normalization, server HMAC-before-lookup, and malformed/nonmatching denial. | C369,C844,C845; hands C319,C342,C343,C354,C885. | Ignored fresh-PG17/server/browser proof; repair the named failed boundary and rerun. Consider only one stable public canonical-ID contract under every `PYTEST_STYLE.md` question; otherwise remove. |
| C847 | C209a retention-notification schema contributor: add late `course_retention_notifications.sql` receipt/lease tables and the dedicated NOLOGIN notifier capability. The unique notification identity is exactly `(course_id, action_kind, due_at, recipient_account_id)`, with `action_kind` exactly `warn_inactive` or `notify_archive`; archive/delete never notice. A receipt starts DB-owned `next_attempt_at = due_at`. At one evaluated timestamp, claim requires action still due, `next_attempt_at <= evaluated_at`, no provider acceptance, and absent/expired lease; order claims by `(due_at, id)`, increment attempt count, and set next attempt to lease expiry. Every claim derives the current eligible recipient and current verified Instructor destination from the deduplicated assigned-Instructor plus active-Instructor-membership union; the receipt stores no address snapshot and the capability exposes no generic lookup. Store the idempotency key before provider invocation and provider acceptance. | C206's due actions and current Course membership/assignment facts; hands C848,C849. No invitation export/Mail.app table or generic email queue. | One temporary fresh-PG17 proof covers action-kind refusal, current eligibility/address, recipient deduplication, one identity/receipt, evaluated-at predicate and `(due_at,id)` order, `SKIP LOCKED`, lease expiry, durable idempotency-key reuse after crash, terminal acceptance/no resend, failure nonblocking, and the exact three-function notifier capability; remove. No new permanent test. |
| C848 | C209b typed LDA notification Store contributor: expose only typed claim, record-provider-acceptance, and unaccepted-attempt failure operations through C847's NOLOGIN notifier capability. Provider acceptance is terminal for sending. A successful recorded failure clears the lease and deterministically sets `next_attempt_at = failed_at + min(3600 seconds, 60 seconds * 2^(attempt_count - 1))`; an action no longer due cannot retry, and accepted receipts never resend. | C847; hands C850,C851. It has no invitation, Account search, Student Work, session, object, renderer API, delivery callback, or inbox-delivery state. | The C847 temporary proof exercises the typed Store and capability denial; remove. |
| C849 | C209c server delivery contributor: define provider-neutral `CourseRetentionNotificationDelivery` and disabled `NotConfigured` adapter. It accepts only a fixed redacted sign-in-only message with no Course identifier/title, raw ID, FERPA data, capability, or recovery link; it records provider acceptance only. `NotConfigured` records non-send failure, sends nothing, and never returns fake success. | C847; hands C850,C851. Provider credentials are operational configuration; Live Demo `NotConfigured` is not delivery evidence. | The C847 temporary proof covers message redaction, disabled recorded failure/no-send, and terminal provider acceptance. The boundary has no provider callback, inbox-delivery guarantee, or provider exactly-once promise; remove. No provider mock call-order test is permanent. |
| C850 | C209d process-isolation contributor: provision one isolated retention process with exactly two independently attested, non-inheriting database profiles/pools: C215 retention executor and C848 notifier. It has no third database authority and no API listener, S3/object-store, session, or renderer credential/configuration. | C847,C848,C849; hands C851. | Ignored failed-access/two-pool attestation proof; remove. Static configuration/file inventories are never permanent. |
| C851 | C209e orchestration contributor: after C215 plus C847-C850, a late run attempts required earlier notices in C847 `(due_at,id)` order through the typed boundary, then executes archive/delete after a successfully recorded pre-acceptance failure. It stops the notice lane only for an unknown typed Store state and always continues retention transitions; it never calculates dates or policy. A crash before send leaves the receipt leased until expiry; a crash after provider invocation reuses C847's durable idempotency key on an eligible reclaim. | C215,C848,C849,C850; hands C209. | The C847 temporary proof covers one evaluated-at claim, due order, lease/crash reclaim, acceptance no-resend, deterministic failure backoff through cap, action-no-longer-due exclusion, repeated-run idempotency, recorded-failure nonblocking archive/delete, and unknown Store state notice-lane stop with continued retention transitions; remove. No new permanent test. |
| C852 | C371 verified-name foundation contributor: store one bounded server-controlled Verified Instructor Display Name only in C17/C18's real identity-vetting/Account-creation transaction. It is neither self-editable nor a Profile/directory/general Account field. | C17,C18; hands C853. | Ignored vetting/creation fixture proves the only write path and rejects Profile/self-edit/directory projections; remove. A stable write-authorization test is a candidate only after `PYTEST_STYLE.md` review. |
| C853 | C371 Star projection contributor: Star SQL, typed LDA, and server projection return exact Verified Instructor Display Names only to an active Instructor viewing a Published Question's Star list. Return no email, UUID, Account reference, avatar, Course, or substitute identifier. | C370,C852; hands C854. | Retain one small real-session authorization/privacy outcome test only if it protects the stable disclosure contract: active Instructor gets exact vetted names; Student, anonymous, inactive, and non-Published/non-Star-list paths do not. Failure restores the projection predicate/fields. Use ignored multi-identity fixtures for all additional cases, then remove them. |
| C854 | C371 frontend contributor: render the exact server-projected Verified Instructor Display Names in the Published Question Star list, with no client-side name lookup, Profile link, or substitute identity. | C853; hands C855. | Ignored rendered-name fixture and accessibility probe; remove. Do not promote a component snapshot. |
| C855 | C371 integrated proof contributor: prove C17/C18 -> C852 -> C370/C853 -> C854 exact-name delivery on the real Published Question Star list. A self-Star/count-only route is interim evidence and closes no C371 occurrence until this whole chain passes. | C17,C18,C370,C852,C853,C854; hands C371. | Ignored connected PostgreSQL/server/browser fixture plus the C853 retained privacy test decision; remove all temporary fixtures after review. |
| C856 | Blueprint Star identity-projection closure owner: after C408/C409/C852, Star SQL, typed LDA, and server projection return exact Verified Instructor Display Names only to an active vetted Instructor viewing a Public or Archived Blueprint Star list. Return no email, UUID, Account reference, avatar, Course, substitute identifier, or any Watch identity/state; the client performs no name lookup and provides no Profile link. | C408,C409,C852; hands C423. C423 supplies final browser closure. | Retain one small real-session authorization/privacy outcome test only if it passes every `PYTEST_STYLE.md` criterion: the authorized active vetted Instructor gets exact names and every other role, visibility, or Watch path does not. Use ignored multi-identity/browser fixtures for the full matrix, then remove them. Failure restores the projection predicate/fields, not a fixture snapshot. |
| C857 | Author-content descriptor/persistence contributor: after C302, carry answer-free `AuthorContentPresentation` through adapter, model, persistence, issuance, reproduction, and checksum. It contains only validated author-script source and closed reviewed library IDs; it excludes Answer Key, feedback correctness, grading input/output, seed/generated-parameter hash, response bindings, session/capability, Account/Course/Attempt metadata, and arbitrary URL. Raw source never enters a generic browser DTO. | C302; hands C858. | Ignored adapter/model/persistence/issuance/reproduction/checksum matrix proves permitted and excluded fields; remove. Do not retain an implementation-coupled serialization snapshot. |
| C858 | Author-content document contributor: after C857 and C901, provide one authenticated no-store HTML document route for exact Student/position authorization and current reviewed runtime reproduction. Safely encode source; no raw-source browser API/save/submit/grading/object-store/general API. The closed `libraries: []` branch has no runtime tags, `connect-src 'none'`, nonce-only `script-src`, and no `'unsafe-eval'`. Only `libraries: ["rdkit"]` has `Content-Security-Policy: sandbox allow-scripts; default-src 'none'; base-uri 'none'; object-src 'none'; connect-src` restricted only to C901's exact current RDKit WASM path; `img-src 'none'; media-src 'none'; font-src 'none'; frame-src 'none'; worker-src 'none'; form-action 'none'; frame-ancestors 'self'; script-src` restricted only to the current registry JS SRI hash, server bootstrap nonce, and `'unsafe-eval'` required by the current official RDKit loader in this opaque document. The main PLE CSP receives no such allowance; no author URL or broad `'self'` source is used. It stores no package version, asset digest, cache key, or historical runtime identity. | C857,C901; hands C859. | Ignored route/header/authorization matrix proves the document, current SRI/`locateFile` assets, and every header/CSP exclusion; remove. Retain a small real-session access/isolation outcome only if it independently passes every `PYTEST_STYLE.md` question. |
| C859 | AuthorContentFrame sole closure owner: render C858 only through typed optional frame reference/availability. Exact iframe is `sandbox="allow-scripts" referrerpolicy="no-referrer" allow=""`; no same-origin/forms/popups/downloads/modals/top-nav/pointer-lock/storage-access/permissions. No parent init/message; optionally accept only source-identity-checked, finite-integer, clamped versioned `author-content.resize`, never answer/URL/HTML/navigation/storage/API commands. | C858; hands C860. | Ignored security browser matrix proves sandbox/CSP, no privileged parent channel, denied form/navigation/network/API paths, and optional resize predicate; remove. Retain only a stable real-session isolation contract if every `PYTEST_STYLE.md` criterion approves it. |
| C860 | Author-content connected-proof contributor: prove C859 rendering/interaction, isolation, grading independence, and C901's current reviewed local RDKit runtime on the authorized Student position before C304 proceeds. | C859,C901; hands C903. | Ignored connected server/browser/RDKit-manifest fixture; remove after review. No permanent renderer orchestration or mock call-order test. |
| C861 | Draft-cleanup removal owner: Human Guidance's optional cleanup bullet is N/A because it specifies no clock or duration. Delete prohibited baseline placeholder warning/recovery table, APIs, grants, Store, worker, generated seams, and test seams; preserve manual C351 deletion/publication and do not implement cleanup. | Independent of C351. Unresolved question: Should PLE automate cleanup of abandoned Draft Questions? If yes, what event starts inactivity; how long until warning; how long is the recovery period after a successfully delivered warning; which save/edit/publication/ownership events reset or cancel it; and what is the outcome when warning delivery fails? | Ignored bounded removal inventory proves those placeholders are absent while manual deletion/publication remain; remove. No textual absence test becomes permanent. |
| C862 | Blueprint lifecycle browser-codec/client direct-cutover contributor: generated `BlueprintAvailability` is exactly `Private|Public|Archived`; update only `src/api/decoders/blueprint_course.ts`, its client fixtures, and executable generated consumer to reject legacy `available|archived` aliases. It does not expand C50 workspace ownership. | C49,C72; hands C50,C19 browser gates. | Ignored codec matrix proves canonical decode and legacy-alias rejection; remove. Retain only a stable public lifecycle-decoder contract if every `PYTEST_STYLE.md` criterion supports it. |
| C863-C869 | **Blocked, no dispatch until this exact H5P product question is answered:** **Which H5P content type(s) are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?** C863 binds immutable Static `.h5p` content and records its declared library/version metadata and SHA256 as reproducible content evidence, not a dependency pin; C864 supplies the rootless Node.js Lumi runtime/container and its current GPL/license/provenance/supply-chain record without dependency pinning; C865 private one-use ticket/gateway/auth; C866 terminal xAPI normalizer and opaque bounded state; C867 atomic fraction/evidence persistence; C868 separate-origin browser frame; C869 Podman connected acceptance. | decision -> `{C863,C864}`; `{C863,C864}->C865`; `C864->C866`; `{C865,C866}->C867`; `C865->C868`; `{C867,C868}->C869`. Runtime has no PLE credentials/egress; failures fail closed. No current row claims H5P delivery or completion. | All proof is ignored first; retain only stable external authorization/outcome contracts after `PYTEST_STYLE.md`. No mock orchestration test. |
| C870 | Immediate H5P placeholder-removal owner: delete draft/revision H5P binding tables/functions/grants/policies; h5p revision backend and secondary attempt branches/DTOs/APIs/tests; adapter_h5p delivery registration/lifecycle/importer and unused crate/workspace member; workspace-import h5p enum/state. Keep generic secondary table only if a delivered non-H5P backend consumes it. No compatibility. | Independent of blocked C863-C869; preserves delivered non-H5P backends. | Ignored bounded removal inventory; prove no dormant H5P seam and no loss of delivered backend behavior; remove. This closes HG's no-placeholder rule, not H5P delivery. |
| C876 | Question-fork lineage contributor: after C319's install-order audit, `question_lineages.sql` exposes only a server-consumable exact immutable Published Question Revision read/pin for fork attribution. It creates no Draft, authoring operation, client-selected ID, or source-attribution table because this install phase precedes authoring tables. | C211,C846,C319; hands C877. | Ignored exact-source/revision read probe; remove. Do not retain a source-table or call-order test. |
| C877 | Question-fork authoring-schema contributor: later `question_authoring_operations.sql` creates one atomic active-Instructor operation and immutable fork-source attribution for a private Draft. Its server-only inputs are the allocated canonical Question ID, resolved source Revision, and actor-bound opaque idempotency key; `(actor, idempotency key)` returns the same Draft only for that exact source and otherwise refuses. It stores no client-supplied authorship, source facts, or Draft content. | C9,C876; hands C878. | Ignored fresh-schema/RLS/concurrent-repeat matrix proves private ownership, exact immutable pin, key/source refusal, and one-Draft result; remove. Retain only a narrow stable authorization or idempotency outcome if it meets every `PYTEST_STYLE.md` criterion. |
| C878 | Question-fork typed command contributor: `QuestionForkStore` and server command resolve the exact authorized Published Revision from the canonical request path, obtain the new ID only from C369's server HMAC allocator, and invoke C877 once. The request body contains no Question ID, source, attribution, authorship, or Draft payload. | C369,C877; hands C879. | Ignored Store/server authorization, malformed-path, allocator-collision, retry, and concurrent-request matrix; remove. No mock call-order test is permanent. |
| C879 | Question-fork closure owner: an active Instructor can invoke the C878 command from a Published Question and reach only the returned distinct private Draft; it carries own authorship, exact immutable source Revision attribution, and no library admission before C9 publication validation. | C9,C878. | Ignored connected PostgreSQL/server/browser proof covers two Instructors, source pin/attribution, private cross-account denial, retry/concurrency, distinct HMAC-issued ID, and attempted prevalidation publication denial. Retain only a small real-session active-Instructor authorization/privacy or idempotency outcome if every `PYTEST_STYLE.md` criterion passes; otherwise remove. |
| C885 | Pool schema/create-operation contributor: `question_pools.sql` stores a unique compact canonical Pool ID, immutable sequential Pool Revisions, and each Revision's ordered exact Published Question Revision members. Its trusted create/append operations accept a server-issued typed ID only at Revision 1, never a browser/client grant, and are neither allocator nor unused coordinator. | C312,C313,C846,C354; hands C886,C342,C905. | Ignored uniqueness/revision/member-pin/RLS matrix proves no direct client path; remove. No schema call-shape test is permanent. |
| C886 | Pool creation command contributor: typed `QuestionPoolCreationStore` and an active-Instructor server route mint every Pool ID only through C369's HMAC allocator and atomically create Revision 1 from a bounded ordered nonempty distinct list of exact Published Question Revision references plus the Instructor's interchangeability attestation. The browser supplies only that content and attestation, never an ID, owner, stored revision number, or backend behavior; a uniqueness collision obtains a new ID and retries the one creation transaction. | C369,C885; hands C887. | Ignored Store/server authorization, member validation, client-ID refusal, backend-mix, forced collision/retry, and atomic-create matrix; remove. No mock allocator/call-order test is permanent. |
| C887 | Pool creation closure owner: the authorized Instructor Pool workflow creates a reusable Published Pool with a new canonical `AAAA-ZBBB` ID, Revision 1, ordered pinned Published Question members, and interchangeability attestation through C886. C342 projects that ID; C905 owns Assessment-owned fork provenance and selection evidence. | C886; hands C342,C355,C905. | Ignored connected PostgreSQL/server/browser proof covers active-Instructor creation, denied non-Instructor/client-selected ID, collision retry, unique canonical ID, Revision 1, exact member pins, and C342 handoff. Retain only a narrow real-session authorization or issuance outcome if every `PYTEST_STYLE.md` criterion passes. |
| C893 | Bulk-metadata server contributor: after C365/C367 and C338's accepted canonical distinct nonempty selection, an active vetted-Instructor route accepts only the typed closed metadata patch and bounded selection, applies C365's one-transaction all-or-none CAS command, and returns only whole outcomes. It has no idempotency key, request digest, replay receipt, generic coordinator, queued job, partial result API, or arbitrary JSON patch. | C365,C367,C338; hands C366,C368. | Accepted isolated actual-server HTTP and private exact-main browser proof covered no-store current read, typed replace/clear, search projection, authorized nonowner Public Published access, anonymous/Student denial, stale two-Question all-or-none 412, and browser refresh without automatic retry. This bounded three-Question proof closes C368's shared-metadata behavior, not C366's thousands-Question practical workflow. |
| C899 | Author-dependency declaration correction: remove the author-supplied `externalDependencies[].{id,cdnUrl,localPath}` shape and validators. The closed author-script `libraries` enum remains the sole request, so neither an author URL nor an author-proposed local path can become runtime authority. Reclassify existing C305 evidence as a contributor only. | C301,C302; hands C900. | Ignored source-shape migration matrix proves legacy declarations are rejected while closed `rdkit` remains accepted; remove. No textual absence test or serializer snapshot is permanent. |
| C900 | Reviewed current-RDKit supply-chain owner: record official npm/rdkit-js provenance, BSD-3-Clause license, npm integrity, and exactly the current reviewed `RDKit_minimal.js`/`.wasm` SHA-256 entries. Add one deterministic vendoring/check command that resolves the current official release, verifies integrity, license, paths, bytes, and generated registry, and rejects extra runtime files, a CDN, or author URL. It creates no historical version catalog. | C899; hands C901,C902. | Ignored current-release clean-cache/tamper/extra-file/generation-reproducibility matrix; remove. A manifest snapshot is not permanent. |
| C901 | Server-owned local-asset/registry owner: consume only C900's current generated registry and expose its current JS/WASM pair through the exact unversioned GET/HEAD routes `/api/author-content-dependencies/rdkit/RDKit_minimal.{js,wasm}`. These two API-origin routes avoid a broad static-asset/proxy surface; they permit no caller-selected path, redirect, directory list, object-store URL, or cookie-dependent response. Send exact MIME, `nosniff`, `Cross-Origin-Resource-Policy: cross-origin`, `Cache-Control: no-cache`, and the sole required CORS exception `Access-Control-Allow-Origin: *` with no credential grant, so RDKit can fetch its fixed public WASM from an opaque `allow-scripts` frame. Derive the current JS SRI hash and exact WASM `locateFile` route that C858 needs. No package version/digest is retained in a presentation or served from a historical catalog. | C900; hands C858,C860,C902,C903. | Ignored route/header/method/path traversal/current-registry-mismatch, exact anonymous CORS, and opaque-origin frame matrix; remove. Retain only a narrow stable immutable-asset authorization/byte-integrity outcome if it independently satisfies every `PYTEST_STYLE.md` criterion. |
| C902 | Current dependency-refresh workflow owner: document and enforce one reviewed command that refreshes the current official RDKit release under HG's latest-dependency policy after provenance/license/integrity/file-hash/browser-compatibility and isolated-frame review. It replaces the pre-production current local runtime and generated registry directly; it creates no version catalog, retirement workflow, or per-presentation package/digest persistence. | C900; hands C903. | Ignored current-release refresh/reject-malformed/browser-proof matrix; remove. No release-script call-order or manifest inventory test is permanent. |
| C903 | External-dependency closure owner: on an authorized Student position, prove the closed `rdkit` declaration yields only C901 current local reviewed JS/WASM, successful isolated chemistry rendering, no network/CDN/API fallback, exact current CSP/SRI/`locateFile` confinement, and unchanged server grading. This closes the three external-dependency occurrences only after C858-C860 also pass. | C858,C859,C860,C900,C901,C902; hands C304. | Ignored connected server/browser/offline-network-denial proof; remove after review. Retain a permanent test only if one small, stable user-visible local-runtime or security outcome independently passes every `PYTEST_STYLE.md` question. |
| C910 | General-feedback closure owner for the three feedback behaviors: author-managed general feedback is immutable Revision metadata and is projected at the submitted-history boundary independently of backend-provided interaction feedback. PLE preserves recorded native feedback, treats WeBWorK feedback as transient, and does not extract or reconstruct it from PGML or renderer output. Question Feedback has no Assessment delayed-release state; the six remaining timing fields independently gate score, correctness, submitted response, answer, explanation, and class statistics. | C307; independent of C331,C362. | Accepted fresh-PG17 publication proof preserved two immutable general-feedback values with identical source provenance. Accepted actual Student HTTP and exact-main browser proof started a WeBWorK Attempt, stopped the renderer, submitted it, rendered exact-Revision General feedback with all six timings `Never`, withheld response/score/correctness/answer/explanation, returned a completion-only two-field submission acknowledgement, and concealed history from nonowners and staff. Source and focused domain tests establish provided native feedback independently of answer disclosure; the runtime WeBWorK fixture supplied no transient backend feedback. |
| C904 | engineering decision; 0; use the existing positive `selection_count` on the Assessment-owned Pool entry/fork. The reusable immutable Pool Revision owns exact members; no Pool default or Assessment override mechanism is added. This is the simplest existing architecture consistent with HG, not a claim that HG mandates field placement. | `crates/question_model/src/assignment.rs`; `schemas/base_schema/assessments.sql`; imported Pool fork belongs to its Assessment. Hands C905-C909. | Source audit records existing positive per-entry count; no new code or permanent test. |
| C905 | Pool-entry provenance schema contributor: retain the Assessment-owned fork's exact reusable Pool ID and immutable Revision, and validate its positive `selection_count` is no greater than the exact fork member count. It adds no Pool default or override. | C313,C885,C887,C904; hands C906. | Ignored fresh-schema validation/provenance matrix; remove. No permanent inventory test. |
| C906 | Pool-selection typed-LDA contributor: carry the Assessment-owned fork's exact Pool ID/Revision and `selection_count` through typed selection inputs without client-selected revision, backend behavior, or arbitrary selection policy. | C905; hands C353,C907. | Ignored typed authorization/provenance codec matrix; remove. No Store call-order test is permanent. |
| C907 | Pool-selection server contributor: the authorized Assessment operation writes the existing per-entry `selection_count`, resolves its exact owned fork Pool ID/Revision, calls C353's backend-neutral selection, and C315 persists that exact fork Pool ID/Revision with the resulting exact selected Question Revisions. | C353,C315,C906,C887; hands C908. | Ignored server/Assessment authorization, count-bound, fresh-Attempt, resume, and Pool/Question provenance persistence matrix; remove. No orchestration test is permanent. |
| C908 | Pool-selection client contributor: the Instructor Assessment editor and typed browser client submit only the Assessment entry's positive `selection_count`; the server derives the exact owned fork Pool ID/Revision and returns a whole validation outcome. They cannot provide selected Question IDs, Pool-ID issuance, backend behavior, or a default/override. | C907; hands C909. | Ignored connected client/editor boundary proof; remove. No component snapshot is permanent. |
| C909 | Pool-selection closure owner: after C908, connected Instructor/Student proof closes C355's five Pool behaviors: Assessment-owned count configuration, new-Attempt backend-neutral selection from exact fork Pool Revision, resume preservation, and exact Pool/Question Revision evidence. | C355,C907,C908; hands C314,C334. | Ignored fresh schema/server/browser proof; remove. Retain only a narrow stable selection-evidence or authorization outcome if every `PYTEST_STYLE.md` criterion passes. |

**Blueprint exact-reference and owned-Pool contributor (2026-09-16):** Actual-server evidence at
`/private/tmp/ple-blueprint-owned-pool-artifacts.nbKrXt/blueprint-owned-pool-http-proof.json`
passed 45 requests: repeated imports mint distinct owned-Pool IDs; retained reorder preserves the
same fixed Question ID at explicit Revisions 1 and 2; and a member edit creates one immutable Pool
Revision and one Blueprint Revision without changing the source or sibling Pool. The compiled-main
browser/state receipts in that directory passed at desktop and 390px: lazy exact-member reads,
historical reorder/Cancel pins, explicit Library remove then latest-revision re-add, missing-
attestation Save blocking, and one ordinary `PUT`/Revision with unchanged Student Work fingerprint.
The separate actual-HTTP Apply receipt at
`/private/tmp/ple-blueprint-owned-pool-artifacts.vs0NCo/blueprint-local-id-apply-http-proof.json`
passed 84 requests: fresh Pools, explicit existing and new target Assessments, four CAS denials,
atomic injected-fault rollback, and Archived-owner `409` protection. Focused `question_model`
tests passed 135 tests; the Store no-run compile and root TypeScript check passed. Source review
also confirms full Question-ID-and-Revision equality at publication while the unused exported
Blueprint Picker remains ID-only deduplicated. This remains contributor evidence only: it does not
close the current-pair comparison, newer-indicator, populated-Student-Work, browser Apply, login,
TLS, accessibility, or whole-plan checks; no checklist/count change or permanent test is claimed.

Cross-boundary DAG: `C801 -> C802`; recorded TOTP decision -> `C803 -> C804 -> C805 -> C806 -> C807`;
C808 audits existing Course-core facts consumed by C206; `{C206, C809} -> C207`; `C37 -> C811`; C812
and `C832 -> C833 -> C834`; `{C812, C833} -> C38 -> C39 -> C819 -> C820`; `C832 -> C835`;
`{C39, C820, C834, C835} -> C836 -> C40/C41`; `{C35, C39, C835, C836} -> C837 -> C42` after its terminal
privacy decision; `C814 -> C815 -> C813`; `C821 -> C822`,
`C821 -> C822`, and `{C818, C820, C822} -> C823`. C810 and C816 are command-only; C817 is
independent and closes D05 only after its existing baseline gate passes. C838-C839 own the
minimal 42-source fresh manifest/publication/install proof. C840-C841 are a separate branch only
for an actual retained database, using ordinary Revision, availability, and CAS. `C842 -> C843`;
`{C318, C341, C842, C843} -> C369`; `{C842, C369} -> C844`;
`{C843, C844, C358} -> C845`; `{C369, C844, C845} -> C846 -> {C342, C343, C354, C885}`;
`{C211,C846} -> C319; {C9,C319} -> C876 -> C877; {C369,C877} -> C878; {C9,C878} -> C879`;
`{C312,C313,C846} -> C354 -> C885; {C369,C885} -> C886 -> C887 -> C342`;
`{C313,C885,C887,C904} -> C905 -> C906 -> C353 -> C315 -> C907 -> C908 -> C909 -> {C314,C334}`;
`{C313,C887} -> C355 -> C909`.
`C338 -> C365; {C365,C339} -> C367; {C338,C367} -> C893 -> C366 -> C368 -> C337; {C9,C893} -> {C368,C337}`. `C300 ->
C825 -> {C826,C827}; C827 -> C828; {C826,C828} -> C829 -> C830 -> C831`, then C831 supplies
C304, C323, C332, and C309. `C307 -> C910`; C910 is independent of C331/C362. C362 retains its
other declared backend-boundary prerequisites. C306 has no H5P mapping. `C206 -> C847 ->
{C848, C849} -> C850`; `{C215, C848, C849, C850} -> C851 -> C209`.
`{C17, C18} -> C852`; `{C370, C852} -> C853 -> C854 -> C855 -> C371`.
For Blueprint stewardship, `C408 -> C409`; `{C408, C409, C852} -> C856`; and
`{C409, C856, C47, C48} -> C423`, whose browser result is the final closure.
For author JavaScript, C303 and the validation-only C305 are architectural
contributors; `C302 -> C857`; `{C301,C302} -> C899 -> C900 -> C901 -> C858(document) -> C859 -> C860`; and
`C900 -> C902`; then
`{C858,C859,C860,C900,C901,C902} -> C903 -> C304`.
For Blueprint lifecycle browser codecs, `{C49, C72} -> C862 -> {C50, C19}`.

### C871-C875: blocked Question Watch notification completion

C346 is a partial contributor, not a closure: verified source-bound revision and fork Watch events
exist, but neither improvement threads nor impact notices has a product-defined lifecycle. The
Watch-notification bullet stays `[ ]`; it is not N/A.

No work may dispatch until these two questions have product answers:

- **Improvement threads:** Who may create, read, reply to, edit, and resolve a thread; which
  identity is shown; whether attachments are allowed; what Question, Revision, or fork lineage it
  may link to; which events notify whom; and what retention rule applies?
- **Impact notices:** Who may create an impact notice; what condition justifies it; what text,
  category, and severity are required; whether it is manual or derived; what Question, Revision,
  or fork lineage it may link to; which audience receives it; and who may update or cancel it?

After both answers, the atomic no-placeholder chain is `C871` thread schema/LDA, `C872` impact
schema/LDA, `{C871,C872} -> C873` authorized APIs, `{C346,C871,C872} -> C874` source-bound outbox
writers, and `{C373,C873,C874} -> C875` private four-event notification closure. Each starts with
an ignored focused proof and retains only a stable authorization or idempotence contract that earns
permanent status under `docs/PYTEST_STYLE.md`.

### C44 through C68: Complete the A4 Instructor-interface gaps

The following rows are 25 canonical sequential correction-milestone completion contracts. Their
verbatim HG bullet allocation is the A4 gap-map crosswalk; a row marked "Contributes" deliberately
flips no checklist bullet. C50, C65, C67, and C68 are the relevant closure owners.

Every C44-C68 coder begins with an untracked `tests/_temp/hg_a4_<id>.*` reproduction probe.
Promote it only when it protects stable external behavior under `docs/PYTEST_STYLE.md`; otherwise
remove it at handoff. On a failed focused gate, retain the output, repair only the row's owned
boundary (or return a prerequisite to its owner), rerun the gate and
`source source_me.sh && ./launchers/run_fast_checks.sh`. A temporary probe is never promoted
merely to obtain a passing result.

| Contract | Expected behavior | Exact owned boundary | Closure / contribution | Dependencies | Focused gate |
| --- | --- | --- | --- | --- | --- |
| C44 | Dense Instructor ribbon and zero-record collections expose available tasks, first actions, and consistent placement. | `src/ribbon/ribbon_catalog.ts`, `src/pages/course_list_page.tsx`, `src/pages/library_page.tsx` | Closes A4-01 (7). | C12; C46, C56, C61. | `node --test tests/test_ribbon_catalog.mjs tests/test_ribbon_route_contract.mjs`; `node tests/e2e/e2e_ribbon_destination_ledger.mjs` |
| C45 | Student View presents the server-authorized answer-free/no-write projection. | `src/pages/assessment_workspace/assessment_workspace_student_view_page.tsx` | Closes A4-02 (1) only after the unavailable route becomes the required projection. | C12, C74; pending A9 no-write preview API. | Required future verification after implementation: use a seeded Instructor/Student Work fixture to prove the projection contains no answers and creates no Student Work, Attempt, submission, or grade. Retain a permanent external-contract test only if it earns status under `docs/PYTEST_STYLE.md`; browser proof then exercises the implemented route. |
| C46 | Courses offers distinct active/inactive/public-search destinations and teaching activity. | `src/ribbon/ribbon_catalog.ts`, `src/pages/course_list_page.tsx`, `src/pages/course_instance_page.tsx` | Closes A4-03 (5). | C7; C12; C47, C55. | `node --test tests/test_course_instance_summary.mjs tests/test_ribbon_route_contract.mjs`; `bash tests/e2e/e2e_live_demo_course_instance.sh --browser` |
| C47 | Instructor can query and narrow Public Blueprint Courses. | `src/pages/blueprint_course_search_page.tsx`, `src/api/blueprint_course.ts` | Bounded acceptance closes G-A4-04 (2): exact submitted names, Public-only filtering, literal query text, query-bound paging, empty reset, detail, and adoption preselection. | C19 accepted; no broader Course-creation, login/TLS, accessibility, manual-chooser, Properties, or comparison closure. | Accepted actual HTTP and compiled-main browser receipt: `/private/tmp/ple-blueprint-owned-pool-artifacts.C6RwpH/public-search-result.json` and `public-search-browser.json`; `ple_data` remains unchanged. |
| C48 | Blueprint Editor selects one Assessment and supplies separate Question/Properties editors. | `src/features/blueprint_course/blueprint_course_workspace.tsx`, `src/features/blueprint_course/blueprint_assignment_content_editor.tsx` | Closes G-A4-05 (5). | C12; C6; pending A8 Blueprint Assessment API. | `node --test tests/test_blueprint_course_ui.mjs tests/test_blueprint_course_model.mjs`; `bash tests/e2e/e2e_live_demo_blueprint_course.sh --browser` |
| C49 | Blueprint schema permits only Private/Public/Archived and records lifecycle/adoption predicates, including Public-only eligibility for adopted Course creation. | `schemas/base_schema/blueprints.sql` | Contributes to A4-06 (8); flips none. | C6. | `bash tests/e2e/e2e_live_demo_blueprint_course.sh --service` |
| C50 | Controls expose only server-permitted lifecycle transitions and their meanings. | `src/features/blueprint_course/blueprint_course_workspace.tsx` | Closes A4-06 (8). | C19, C49, C72. | `node --test tests/test_blueprint_course_ui.mjs tests/test_blueprint_course_model.mjs`; `bash tests/e2e/e2e_live_demo_blueprint_course.sh --browser` |
| C51 | Forking a Public Blueprint makes a separately editable Private child. | `crates/server/src/blueprint_course.rs` | Closes A4-07 (1). | C49, C72. | `source source_me.sh && cargo test -p server_core blueprint_course`; `bash tests/e2e/e2e_live_demo_blueprint_course.sh --service` |
| C52 | Adoption server projects preserved Assessment content and unreleased/no-date defaults. | `crates/server/src/course_instance.rs` | Closes A4-08 (2). | C6, C7, C12, C48, C49, C72, C73. | `bash tests/e2e/e2e_live_demo_course_instance.sh --authority` |
| C53 | Active Course Instance cannot extend beyond six months from creation. | `crates/question_model/src/course_term.rs` | Closes G-A4-09 (1). | C7. | `source source_me.sh && cargo test -p question_model course_term` |
| C54 | Centered responsive Course Banner accepts valid still 5:1 sources within independent safety bounds, preserves 5:1 delivery without crop, and treats 1280x256 as HG's recommended authoring size rather than a required minimum or exact rendition. | C814 inventories `crates/question_model/src/course_appearance.rs`, `crates/server/src/course_appearance.rs`, `crates/learning-data-access/src/course_banner.rs`, `crates/learning-data-access/src/postgres/course_banner.rs`, `crates/objects/{bucket,image_validation}.rs`, `schemas/base_schema/course_media.sql`, frontend, and test consumers; C815 owns normalization/delivery; C813 owns centered frontend. | Closes G-A4-10 (5). | C814 -> C815 -> C813. Exact fixed rendition dimensions remain an unlocked design requiring a recorded decision. | ignored small-valid-5:1, 1280x256, 2560x512, non-5:1 fixtures; `bash tests/e2e/e2e_live_demo_course_appearance.sh`; `npx playwright test tests/playwright/e2e/course_appearance_propagation.spec.ts` |
| C55 | Course Editor has Assessment names and separate Question/Properties routes. | `src/pages/course_instance_page.tsx`, `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx`, `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` | Closes G-A4-11 (6). | C7, C8, C12. | `node --test tests/test_assignment_workspace_questions.mjs tests/test_assignment_workspace_policy_model.mjs`; `bash tests/e2e/e2e_live_demo_course_instance.sh --browser` |
| C56 | My Questions, Starred, Watched are working zero/nonzero personal lists. | `src/ribbon/ribbon_catalog.ts`, `src/pages/question_drafts_page.tsx`, `src/pages/library_page.tsx` | Closes G-A4-12 (4). | C8, C9, C11. | `node --test tests/test_question_picker.mjs`; `bash tests/e2e/e2e_live_demo_question_library.sh --browser` |
| C57 | Initial Search foregrounds one entry; return restores query, filters, position. | `src/pages/library_page.tsx` `LibraryPage`; `src/pages/library_page_model.ts` `saveQuestionLibraryReturnState`/`takeQuestionLibraryReturnState`. | Closed G-A4-13 (3): accepted one-time compiled-browser evidence confirmed idle no-fetch, query-start transition, visible-return and Back restoration of query/filter/80 rows/scroll, and changed-session isolation. The temporary harness and screenshots were removed after acceptance; this is not connected HTTP evidence. | C8. | `./check_codebase.sh`; `source source_me.sh && python3 -m pytest tests/`; `source source_me.sh && cargo test -p server_core --lib` |
| C58 | One parser supports ordinary words, quotes, minus, PLE field tags. | `crates/server/src/question_library/search_query.rs`, composed by `crates/server/src/question_library.rs`. | Closed G-A4-14 (7): accepted private PostgreSQL 17 and actual-server HTTP evidence exercised ordinary words, quotes, minus, all five PLE fields, active-vetted-Instructor access, anonymous/Student concealment, and `no-store`. | None. | Accepted one-time private actual-server proof; `source source_me.sh && cargo test -p server_core --lib` passed. |
| C59 | Help makes advanced grammar discoverable without burdening ordinary Search. | `src/pages/library_page.tsx` | Closes G-A4-15 (2). | C58. | `node tests/playwright/e2e_live_demo_question_library_browser.mjs` |
| C60 | Browse is grouped/count-bearing distinct path with hierarchy, dense rows, Search handoff. | `src/pages/library_page.tsx`, `src/pages/library_page_model.ts` | Closed all 8 G-A4-16 rows: overview groups, hierarchy, full-snapshot counts, exact Search handoff, distinct paths, connected exploration, and shared dense presentation have accepted evidence. | None. | Accepted isolated routed-component, private actual-server grouping, and private exact-main full-app browser evidence; focused TypeScript, route, Ribbon, screenshot-manifest, and picker checks passed. The browser fixture was test asset transport, not deployment-gateway or WASM-runtime evidence. |
| C61 | Assessments ribbon/Due Soon/Templates use Assessment labels and scannable state rows. | `src/ribbon/ribbon_catalog.ts`, `src/pages/assessments_due_soon_page.tsx`, `src/pages/assessment_templates_page.tsx`, `src/pages/course_instance_page.tsx` | Closed all 6 G-A4-17 rows with accepted Ribbon/Templates, cross-Course Due Soon, and two owned Course-list receipts. Due Soon displayed Course per row; both Course lists displayed release state and readable Account-zone Due, including 720px first-Course evidence with a browser configured to a different zone. | None for G-A4-17; broader spreadsheet-like collection-density judgment remains under C44. | `node --test tests/test_assignments_due_soon_client.mjs`; accepted private PostgreSQL 17, actual-server, and exact-main browser proof with 200/`no-store` Assessment-list responses. Empty/error states, release workflow, WASM runtime, deployment gateway, and connected Template delivery were not claimed. |
| C62 | Assessment workspace has distinct named Question/Properties tasks. | `src/pages/assessment_workspace/assessment_workspace_live_page.tsx`, `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx`, `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` | Closed all 5 G-A4-18 rows with accepted actual-HTTP and exact-main browser evidence for the two editors, Question add/move/remove/order persistence, distinct tasks, grouped responsive Properties, persisted instructions, and fixed-Question point-value editing with explicit conflict recovery. | None. | Focused Question/Properties/Ribbon tests; accepted private PostgreSQL 17, actual-server, exact-main browser, computed-style, CAS-conflict, and independent-review evidence. Test asset transport was not deployment-gateway or WASM-runtime proof; score recalculation and Released-Assessment editing were not exercised. |
| C63 | Question editor shows order, inspection-before-add, Search/Browse paths. | `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` | Closed G-A4-19's visible-order and direct Search/Browse rows with accepted actual-server and browser evidence. Exact-revision inspection now resolves the authorized private source and checksum, then renders through the opaque WeBWorK adapter in a hardened iframe. The real renderer visibly supplies a prompt and five choices, but inspection remains open: its JavaScript dereferences `window.frameElement.id` when the hardened sandbox has no same-origin frame element, so it errors before focus, popover, and parent telemetry. | C60, C62. | Accepted private PostgreSQL 17/MinIO and unchanged-renderer HTTP evidence returned preview 200 with hardened headers and concealed missing Revision, Student, and anonymous requests; exact-main browser evidence showed the prompt and five choices. The isolated preview path left all five Student Work counts at zero before and after: Assessment Attempts, Question Attempts, saved responses, submissions, and grading results. Do not loosen the iframe sandbox or rewrite the sibling renderer HTML. Successful hardened-embed behavior and return without losing Assessment state remain pending. |
| C64 | Persist the selected Assessment Question-order rule with the immutable Attempt, and shuffle the complete fixed-and-Pool issued vector only when that rule is `shuffled`; Question presentation remains Question-owned. | `schemas/base_schema/assessment_attempt_operations.sql`; `crates/learning-data-access/src/postgres/assessment_delivery_start.rs` | Contributes to G-A4-20 (2); flips none. | C62; source, compile, private SQL, and actual Student HTTP start/resume gates pass; C65 browser integration remains pending. | Accepted private PostgreSQL 17 actual-API and authenticated Student HTTP proof covered authored order, shuffled complete-vector persistence, concurrent Instructor-write locking, exact pins, immutable resume, and outsider/anonymous concealment. |
| C65 | Properties UI presents Assessment-order and Question-owned-choice behavior. | `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` | Closed G-A4-20 (2): actual HTTP persisted immutable authored/shuffled Question order, and exact-main browser evidence saved/reloaded the checkbox while preserving explicit Question-owned answer-choice copy. | None. | Accepted C64 Student HTTP start/resume and C65 exact-main browser proof; focused Question source/codec and nonce-reproduction tests establish the closed ownership boundary without claiming every native permutation runtime matrix. |
| C66 | Unrelease server requires confirmed title and deletes Work only after confirmation. | `crates/server/src/assignment_release.rs` | Contributes to G-A4-21 (2); flips none. | pending A9 Work deletion contract. | `source source_me.sh && cargo test -p server_core assignment_release`; `bash tests/e2e/e2e_live_demo_assignment_release.sh --service` |
| C67 | Unrelease UI explains deletion and requires Assessment title. | `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` | Closes G-A4-21 (2). | C12, C62, C66. | `node --test tests/test_assignment_workspace_policy_model.mjs`; `bash tests/e2e/e2e_live_demo_assignment_release.sh --browser` |
| C68 | Shared Danger Zone lists Unrelease plus both archives; every archive explains availability and requires confirmation. | new `src/features/high_consequence_actions/availability_archive_confirmation.tsx`; consumers `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx`, `src/features/blueprint_course/blueprint_course_workspace.tsx`, `src/pages/question_detail_page.tsx` | Closes G-A4-21 combined Danger Zone bullet and G-A4-22 archive bullet (2). | C50, C67, C9; pending A7 Published Question archive action. | temporary `tests/_temp/hg_a4_danger_zone_matrix.mjs`; then `node tests/playwright/e2e_live_demo_assignment_release_browser.mjs` |



### C69: Establish task hierarchy and local actions

- HG bullets closed: "Design around what users need to find and do."; "Important information should stand out from supporting information."; "Related information should be visually grouped and aligned."; "Similar pages should place similar controls in consistent locations."; "Primary actions should be easy to find and appear near the content or workflow they affect."
- Expected behavior: each role route follows `docs/ux/RIBBON_TASK_MODEL.md` and `docs/UI_DESIGN_GUIDE.md`: one current teaching decision, four hierarchy levels, local grouped controls, and a local primary action.
- Owned boundary: shared frontend task hierarchy/page-action primitives and their role-route consumers. Dependencies: C44-C68 task destinations where applicable.
- Focused gates: temporary `bash tests/_temp/hg_a3_c69_hci_walkthrough.sh`; `node tests/playwright/ribbon_m9_responsive_evidence.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: the walkthrough ledger, role screenshots, keyboard traversal, and axe run are one-time HCI evidence, not permanent metric tests. Repair the responsible route/primitives, rerun the ledger, and delete its temporary artifacts after review.

### C70: Make collections scan, search, and compare

- HG bullets closed: "Optimize large collections for scanning, searching, filtering, and comparison."; "Show enough useful information at once to support comparison without excessive scrolling."; "Search and filters should help users quickly narrow large collections."; "Dense pages should remain easy to scan."
- Expected behavior: Question, Course, and Assessment collection routes implement the named task-model scan fields, visible query/filter state, and comparison path at the guide's canonical role viewports.
- Owned boundary: collection frontend presentation/query controls. Dependencies: C44-C68 collection routes and their backed query APIs.
- Focused gates: temporary `bash tests/_temp/hg_a3_c70_collection_walkthrough.sh`; `node tests/playwright/e2e_live_demo_question_library_browser.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: keep only stable query/result contracts; HCI screenshots, axe, and walkthrough are temporary. Repair missing scan fields/filter feedback/keyboard path and rerun the reviewed walkthrough.

### C71: Apply the precision-field-console system

- HG bullets closed: "Use spacing to separate meaningful groups rather than simply making pages spacious."; "Prefer alignment, typography, and dividers over unnecessary cards, boxes, borders, and nested containers."; "Keep the visual design compact, flat, information dense, and consistent across PLE."; "Dream big on the UI. Choose one visual philosophy and carry it through the entire interface."
- Expected behavior: shared styling implements `docs/UI_DESIGN_GUIDE.md`'s precision field console, its four-level hierarchy, proximity-before-containment, and token-owned density across role routes.
- Owned boundary: shared frontend style tokens and layout primitives. Dependencies: C69, C70.
- Focused gates: temporary `bash tests/_temp/hg_a3_c71_precision_field_console_walkthrough.sh`; `node tests/playwright/ribbon_m9b_density_evidence.mjs`; `./check_codebase.sh`; `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Permanent-gate decision and failure plan: retain only stable accessibility/layout contracts. Screenshots, axe, and HCI judgement are one-time evidence; repair the owning style primitive then rerun and remove temporary artifacts.

### C72: Enforce Blueprint lifecycle transitions in the server

- HG bullets contributed to: A4-06 Blueprint Private/Public/Archived lifecycle bullets.
- Expected behavior: the server applies C49's lifecycle constraints to every transition, owner
  read, public discovery, adoption-conditioned return-to-Private, Draft rejection, and adopted
  Course creation (Public only; Private and Archived denied).
- Owned boundary: `crates/server/src/blueprint_course.rs`; do not change schema or frontend.
- Dependencies: C49 and C6.
- Focused gates: `source source_me.sh && cargo test -p server_core blueprint_course`; then
  `bash tests/e2e/e2e_live_demo_blueprint_course.sh --service`; then fast checks.
- Permanent-gate decision and failure plan: retain lifecycle/authorization transition tests because
  they protect durable visibility and adoption rules. Use a temporary ownership matrix only for
  diagnosis; repair the route/service predicate and remove the probe before handoff.

### C73: Store Blueprint adoption defaults

- HG bullets contributed to: A4-08 Blueprint-to-Course-Instance adoption bullets.
- Expected behavior: adoption persistence carries the exact Blueprint Assessment structure,
  Questions, pools, and settings while storing unreleased Course Instance Assessments with dates
  unset for C52 to project.
- Owned boundary: `schemas/base_schema/course_blueprint_adoption.sql`; do not change server
  response or frontend code.
- Dependencies: C6, C7, C48, and C12.
- Focused gates: `bash tests/e2e/e2e_live_demo_course_instance.sh --authority`; then fast checks.
- Permanent-gate decision and failure plan: retain the adoption default assertions as a stable
  data-contract regression test. A direct SQL diagnosis belongs in `tests/_temp/`; repair only
  the adoption procedure before C52 runs.

### C74: Enforce answer-free, no-write Student View delivery

- HG bullets contributed to: A4-02 Student View projection bullet.
- Expected behavior: the preview delivery boundary returns no answers and rejects every mutation
  that could create Student Work, an Attempt, submission, or grade.
- Owned boundary: `crates/server/src/assignment_delivery.rs`; do not alter the frontend preview
  presentation in this contributor.
- Dependencies: pending A9 Attempt/write boundary.
- Priority note: iMathAS is not planned for the pilot and remains a PLE Question Backend. Prioritize Native PLE
  and WeBWorK Student View delivery; finish the already-bounded no-write iMathAS adapter without
  spending substantial additional time, then defer broader iMathAS production wiring and live proof.
  This is neither an N/A classification nor an exemption from Human Guidance completion; its later
  production completion remains open.
- Focused gates: `source source_me.sh && cargo test -p server_core assignment_delivery`; then
  `bash tests/e2e/e2e_live_demo_course_instance.sh --service`; then fast checks.
- Permanent-gate decision and failure plan: retain no-write/answer-redaction authorization cases
  only if they exercise the stable public preview contract. Put synthetic Student Work setup and
  exploratory request matrices under `tests/_temp/`; repair the delivery authorization/redaction
  path on failure and remove those probes.

### C75 through C90: Complete A5 Student and Sysadmin interface gaps

The A5 map crosswalk below is authoritative for 26 raw audited entries: 23 owning open
product-behavior bullets, two HG-unlocked Ribbon details, and one N/A permission classification.
C80, C81, and C83 are temporary discovery or repair handoffs and flip no bullet; C75, C85, and
C88 are server/API contributors. Every temporary check begins in ignored `tests/_temp/`, is removed at handoff
unless it earns permanent status under `docs/PYTEST_STYLE.md`, and never freezes CSS values,
handler calls, or request counts. A failed gate repairs the named owner boundary or returns the
dependency to its owner; it does not broaden a frontend milestone into a server authorization
change.

| Contract | Expected behavior | Owned boundary | Closure / contribution | Dependencies | Focused gates and failure plan |
| --- | --- | --- | --- | --- | --- |
| C75 | The authorized Student Coursework projection supplies Type, icon identity, due date, and delivery/completion state without exposing another Student's data. | `crates/server/src/live_student_course_landing.rs` and its browser API contract. | Contributes to A5-S01, S02, S05, S10, S11, S12. | A9 Assessment Type, icon, and delivery-state contracts; A1 terminology. | `node tests/_temp/hg_a5_c75_student_projection.mjs`; `source source_me.sh && cargo test -p server_core live_student_course_landing`; `source source_me.sh && ./launchers/run_fast_checks.sh`. Failure repairs the server/API projection. |
| C76 | The Student landing says Coursework and renders each item with the specific Type label and icon. | `src/pages/student_course_landing_page.tsx`. | Closes A5-S01, S02, S05; contributes S11. | C75. | `node tests/_temp/hg_a5_c76_student_coursework_browser.mjs`; `source source_me.sh && ./launchers/run_fast_checks.sh`. Consider promotion only after this temporary browser fixture demonstrates an independently plausible learner-facing terminology/Type regression; failure repairs the page. |
| C77 | The Student landing makes upcoming, available, completed, and missed Coursework distinct and scans due date, Type, and completion. | `src/pages/student_course_landing_page.tsx`. | Closes A5-S10, S11. | C75; A9 delivery/late-state contract. | `node tests/_temp/hg_a5_c77_coursework_states.mjs`; `source source_me.sh && ./launchers/run_fast_checks.sh`. Failure repairs the state-to-display mapping. |
| C78 | Before starting, the Student sees title, Type, Question count, points possible, time limit, and previous Attempts. | `src/pages/assignment_overview_page.tsx` and `src/components/student_assignment_presentation.tsx`. | Closes A5-S12. | C75. | `node tests/_temp/hg_a5_c78_pre_start_facts.mjs`; `source source_me.sh && ./launchers/run_fast_checks.sh`. Promote only if the direct pre-start fact contract is not covered by C82; failure repairs pre-start presentation. |
| C79 | Backed Student navigation uses learner language and supplies a Student-only path to active Courses and Coursework, without Instructor authoring/grading or Sysadmin administration tasks. | `src/ribbon/`. | Closes A5-S03, S06. | C76. The complete Student Ribbon task layout remains HG-unlocked. | `node tests/_temp/hg_a5_c79_student_navigation.mjs`; `source source_me.sh && ./launchers/run_fast_checks.sh`. Promote only a stable learner-navigation contract; failure repairs catalog/capability admission, never a control-count proxy. |
| C80 | Discover the four viewport and keyboard Student journey failures before permanent test selection. | `tests/_temp/hg_a5_student_workflow.mjs`. | Contributes to A5-S07, S08; flips none. | C76-C79. | `node tests/_temp/hg_a5_student_workflow.mjs` at `1280x800`, `768x1024`, `320x640`, `768x768`: `/student/courses/:courseRef` -> Coursework -> pre-start -> start/resume -> response -> Question navigation -> summary. Record route, viewport, task, observable failure only. |
| C81 | Convert every C80 finding into a new concrete, page-specific correction milestone before attempting closure. | Manager dispatch record; each new milestone owns one exact named page/path and has its own ID after C90. | Contributes to A5-S07, S08; flips none. | A concrete C80 failure. | C80 lists zero unresolved findings before C82 starts. C81 never owns a recurring placeholder page; each defect gets a new one-page owner, focused rerun, and reviewer. |
| C82 | Retain durable cross-viewport and keyboard journey evidence only after C80 finds no remaining defect and every C81-created concrete page milestone is closed. | `tests/playwright/e2e/student_coursework.spec.ts`. | Closes A5-S07, S08. | C80 and all C81-created milestones. | `npx playwright test tests/playwright/e2e/student_coursework.spec.ts`; fast checks. This is permanent only because HG explicitly requires four viewports and keyboard use; failure creates a new concrete C81 page milestone. |
| C83 | Discover Student API and route authorization separately from frontend admission, including ASVS 5.3.2 authorization denial evidence. | Ignored `tests/_temp/hg_a5_c83_student_endpoint_manifest.json` and `tests/_temp/hg_a5_c83_student_authorization.mjs`. | Contributes to A5-S09; flips none. | C24-C26 and related A2 authorization work. | `node tests/_temp/hg_a5_c83_student_authorization.mjs --manifest tests/_temp/hg_a5_c83_student_endpoint_manifest.json`; the discovery manifest is removed at handoff, so it does not freeze the evolving route/API inventory. A server failure returns to C24-C26/A2. |
| C84 | Browser route admission matches the server-authorized Student surface. | `src/route_contract.ts`, `src/routes.ts`, `src/route_access_boundary.tsx`. | Closes A5-S09 after C83 is clean. | C83; C24-C26 for any server correction. | `node tests/_temp/hg_a5_c84_student_route_admission.mjs --routes-from-contract`; fast checks. Promote only a stable route/access contract without pinning an endpoint inventory; failure repairs admission or the referenced server boundary. |
| C85 | Sysadmin account search/list API exposes only approved fields and provides ASVS 2.2.1 bounded name/email search, filters, and pagination. | `crates/server/src/instructor_account.rs` and browser API contract. | Contributes to A5-Y02, Y03, Y04. | C24, C26 and an A2 FERPA-approved field inventory. | `source source_me.sh && cargo test -p server_core instructor_account`; `bash tests/e2e/e2e_live_demo_instructor_accounts.sh --service`; fast checks. Failure repairs unsafe or incomplete server projection. |
| C86 | Sysadmin account list consumes C85 search/filter/paging without disclosing unapproved fields. | `src/pages/instructor_accounts_page.tsx`. | Closes A5-Y02, Y03; contributes Y04. | C85. | `node tests/_temp/hg_a5_c86_sysadmin_account_list.mjs`; fast checks. Promote only bounded search/filter/pagination behavior because it is an explicit HG contract; failure repairs the page, not request order. |
| C87 | Sysadmin account detail presents approved role/status information, C18-backed approval changes, and high-consequence confirmation. | `src/pages/instructor_accounts_page.tsx`. | Closes A5-Y04, Y06, Y11, Y12. | C85-C86; C17 then C18. | `node tests/_temp/hg_a5_c87_sysadmin_account_detail.mjs`; fast checks. Promote only approval/access and destructive-confirmation behavior because it protects a stable security/workflow boundary; failure repairs clarity/action flow, never colors, CSS classes, or request counts. |
| C88 | Sysadmin Course API establishes ASVS 5.3.2-authorized installation-wide Course search, inspection, status, and allowed management. | new server Course-administration route module. | Contributes to A5-Y07, Y08, Y09. | C24-C26; A8 authoritative Course status/operations. | `node tests/_temp/hg_a5_c88_sysadmin_course_api.mjs`; `source source_me.sh && cargo test -p server_core sysadmin_course`; `bash tests/_temp/hg_a5_c88_sysadmin_course_service.sh --authorized`; `source source_me.sh && ./launchers/run_fast_checks.sh`. Failure repairs absent or over-broad server boundary. |
| C89 | Sysadmin Course routes/pages provide the C88 capabilities. | `src/route_contract.ts`, `src/routes.ts`, new `src/pages/sysadmin_course_*`. | Closes A5-Y07, Y08, Y09. | C88. | `node tests/_temp/hg_a5_c89_sysadmin_courses.mjs`; fast checks. Promote only the stable authorized Course-management behavior; failure repairs unavailable task or wrong-role data. |
| C90 | Sysadmin navigation reaches Accounts/Instructors, Courses, and the separate settings area after A5-Y10 is resolved. | `src/ribbon/`. | Closes A5-Y01. | C87, C89, and A5-Y10 resolution. | `node tests/_temp/hg_a5_c90_sysadmin_navigation.mjs`; fast checks. Promote only the stable administrative-navigation behavior; failure repairs navigation catalog only. |

No correction milestone is allocated for A5-S04: its "may provide filters" language is permission,
not an implemented behavior requirement, so the audit reclassifies it N/A with that reason. A5-Y10
remains `[ ]` pending the recorded product question; do not create an empty settings page.

### C200-C216: Complete A6 Data and history gaps

C91 and later are reserved for the dynamic page-specific A5 dispatches. A6 therefore uses the
disjoint explicit C200-C216 range; C205 is the recorded product question, not a code milestone.
This allocation does not reuse C1-C90 or reserve a dynamic A5 identifier.

The authoritative one-to-one A6 record map is
`docs/active_plans/audits/human_guidance_gap_map.md` under "A6 Data and history - canonicalized
2026-09-14": 37 unchecked A6 records, two named later duplicates, and 35 owning gaps. C200-C216
contains 16 atomic implementation/evidence milestones (C205 is the product question). New proof
starts in ignored `tests/_temp/` and is deleted at plan closeout unless it independently satisfies
every `docs/PYTEST_STYLE.md` permanent-test question. C203 is the accepted permanent
authorization-denial oracle; C208 deletion/preservation and C213 immutable issuance-pin checks
remain candidates. A failure keeps the named bullet `[ ]`, repairs the named boundary, and does
not create a new product policy.

| Contract | HG records closed / expected behavior | Exact boundary and dependencies | Focused gate |
| --- | --- | --- | --- |
| C200 | Contributor to D01-D02. Define the allowlisted Student response contract; no bullet flips until conversion is enforced. | `crates/browser-api-contract/src/student_assignment_decision.rs`; depends on A7 server-owned grading/backend delivery audit evidence. | `source source_me.sh && python3 tests/_temp/hg_a6_c200_contract_probe.py` |
| C214 | D01-D02. Enforce C200's allowlist for native and WeBWorK delivery. | `crates/server/src/assignment_delivery/` conversion only; depends on C200. | `source source_me.sh && python3 tests/_temp/hg_a6_c214_delivery_probe.py` |
| C201 | D04. Browser observability rejects/redacts classified Student Work and has no facade bypass. | `src/log.ts`; depends on C214. It does not create analytics, URL, or browser-storage behavior. | `node --test tests/_temp/hg_a6_c201_log_probe.mjs`; `npx playwright test tests/_temp/hg_a6_c201_browser.spec.ts` |
| C202 | Contributor to D03 for the discovered Question recognition/copy surface; it does not close global D03. | `src/components/copyable_question_id.tsx`; show title plus reference rather than opaque ID alone. | `node --test tests/_temp/hg_a6_c202_labels.mjs`; `npx playwright test tests/_temp/hg_a6_c202_labels.spec.ts` |
| C216 | D03. Run one temporary frontend conformance inventory across the route manifests and candidate consumers, then close only when every recognition/copy/entry surface has human title/reference labels. | Route manifests `src/routes.ts`, `src/route_contract.ts`; exact candidate set `src/components/copyable_question_id.tsx`, `src/pages/assignment_editor_content_list.tsx`, `src/pages/assignment_editor_model.ts`, `src/pages/library_page.tsx`, `src/pages/library_page_model.ts`, `src/pages/question_detail_page.tsx`, regenerated by `rg -l "CopyableQuestionId|reference_number|assignment_title|course_reference_number|question_id" src/routes.ts src/route_contract.ts src/pages src/components`. Depends on C202, C30, C46, C55, C76-C78, C86-C87, C89, and explicit A8-PENDING-title-reference-inventory/A9-PENDING-title-reference-inventory resolution before dispatch. The inventory is ignored temporary evidence, not a permanent DOM/file-list contract; failure creates a focused correction in the owning page milestone. | `node tests/_temp/hg_a6_c216_human_identity_inventory.mjs --routes src/routes.ts --contract src/route_contract.ts` |
| C203 | D05. Permanent behavior-level PostgreSQL oracle proves exact Course membership and Student-record ownership through a real application session: allow owner; deny nonmember, same-Course other Student, and same-Account other-Course record. | `schemas/base_schema/authorization.sql`; `crates/learning-data-access/tests/assignment_access_postgres.rs`; existing baseline owner. This is a stable high-impact BOLA/FERPA boundary and asserts outcomes rather than schema/function structure. | existing `bash tests/e2e/e2e_database_baseline.sh`; on failure retain D05 `[ ]`, preserve baseline evidence, identify session installation, exact-membership predicate, or ownership fault, repair it, and rerun the same gate. |
| C204 | D08-D09. Named Student Work composes distinct Attempt, response, issuance, submission, grading, and receipt identities. | `crates/question_model/src/student_work.rs`; after C213 supplies the Pool Revision field. | `source source_me.sh && python3 tests/_temp/hg_a6_c204_probe.py` |
| C205 | D12 remains open. | Product question: **What shared-statistics release rule, including minimum cohort and intersection behavior, makes a slice not reasonably identifying for PLE?** HG, `DATA_CLASSIFICATION.md`, `RETENTION_POLICY.md`, and `DESIGN_DECISIONS.md` require protection but do not resolve it. Record in fresh unresolved report; no numeric threshold is invented. | Retain `[ ]` with `Reason: product decision still unclear` and exact `Question:`. |
| C206 | D13-D14, D16-D17, D20, D25, D27. Store the new operational policy schedule/due actions consuming the audited existing Course-core facts; clock calculation has no action side effect. | new `schemas/base_schema/course_retention.sql`: policy schedule and due actions `warn_inactive`, `notify_archive`, `archive`, `delete`, archive marker. It consumes, rather than duplicates, current `course_core.sql` facts. No generic recovery state/queue/snapshot service. | `source source_me.sh && python3 tests/_temp/hg_a6_c206_schedule_probe.py` |
| C207 | D15, D19. Assessment release/save synchronizes `latest_assessment_due_at` and rejects `due_at` past existing `active_until_at`, atomically under concurrent Course/Assessment edits. | `schemas/base_schema/course_core.sql`; `schemas/base_schema/assessments.sql`; private `schemas/base_schema/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline`; exact retention readers, installation order, and canonical Live Demo seed/oracle; depends on C206 and C809. | Accepted independent PostgreSQL 17 actual-API proofs covered all three save APIs, release, current MAX/no-Due fallback, immutable Active cap, stale CAS, wrong authorization, cap rollback, deterministic two-Assessment concurrency, archive race/freeze, helper ACL, and seeded current MAX; the ignored proof was removed after acceptance. |
| C208 | D06-D07, D10-D11, D24. Archive protects Course Student records; delete at expiry removes identifiable records while preserving Account, Course metadata, definitions, Questions, settings, and existing aggregates. | `schemas/base_schema/course_retention.sql` transition procedures; depends on C206. Statistics disclosure is C205, not a C208 prerequisite. | `source source_me.sh && python3 tests/_temp/hg_a6_c208_transition_probe.py` |
| C215 | Contributor to C209. Store interface/adapter returns stored due actions and commits transitions under its least-privilege database capability; flips no HG bullet. | `crates/learning-data-access/src/retention.rs`; depends on C206, C208. | `source source_me.sh && python3 tests/_temp/hg_a6_c215_store_probe.py` |
| C209 | D18, D21-D22, D26, D28-D30. Worker consumes C215 stored due actions, invokes only C851's retention-notification boundary, and archives/deletes; late and repeated runs are equivalent. Provider failure never blocks archive/delete. | `crates/server/src/worker.rs`; depends on C215,C851. | `source source_me.sh && python3 tests/_temp/hg_a6_c209_worker_probe.py`; lease-gated `bash tests/_temp/hg_a6_c209_retention_e2e.sh` |
| C210 | D23. Normal reads exclude archived Work while a protected pre-delete read remains available. | `schemas/base_schema/attempt_history.sql` and `schemas/base_schema/student_assignment_landing.sql`; depends on C208. No general recovery workflow/state machine. | `source source_me.sh && python3 tests/_temp/hg_a6_c210_visibility_probe.py`; `npx playwright test tests/_temp/hg_a6_c210_visibility.spec.ts` |
| C211 | D31. Revision creation remains conservative: source/answer/grading/feedback/assets revise; title/description/tags/subject/topic do not. | `schemas/base_schema/question_authoring_operations.sql`; it connects the already implemented A7 Published Question rule and has no unclosed A7 dependency. | `source source_me.sh && python3 tests/_temp/hg_a6_c211_revision_probe.py` |
| C212 | D32-D34. Published Pool has immutable identity and sequential per-Pool Revision Numbers. | `schemas/base_schema/assignments.sql` Pool tables/edit functions; depends on A7 published-Pool/public-ID/revision correction. | `source source_me.sh && python3 tests/_temp/hg_a6_c212_pool_probe.py` |
| C213 | D35. Issuance pins exact Pool Revision with selected Published Question Revision. | `schemas/base_schema/attempts.sql` selection/issuance constraints; depends on C212 and A7 correction that new Attempts select freshly. | `source source_me.sh && python3 tests/_temp/hg_a6_c213_pin_probe.py` |

Dependency DAG: `C200 -> C214 -> C201`; C808 audits existing Course-core facts consumed by C206;
`{C206, C809} -> C207`; `C206 -> C208`; `C208 -> {C215, C210}`;
`C206 -> C847 -> {C848, C849} -> C850`;
`{C215, C848, C849, C850} -> C851 -> C209`; `A7 Pool lineage -> C212 -> C213 -> C204`; `A7 backend delivery -> C200`;
`C202 -> C216`; C30, C46, C55, C76-C78, C86-C87, C89, A8-PENDING-title-reference-inventory,
and A9-PENDING-title-reference-inventory feed C216; C203 is independent. C205/D12 is a product question independent of C208, so privacy
release policy cannot cycle with retention deletion.

### C300-C375: Complete A7 Questions gaps

C300-C375 is the ordinary A7 dispatch range, disjoint from A5's C91+ range and A6's C200-C216
range. C825-C831 is the reserved cross-boundary native-reproduction chain for C300. Use the
current A7 rows in `docs/active_plans/audits/human_guidance_gap_map.md` for the named owner,
affected boundary, prerequisite handoff, and proof. Do not dispatch from superseded drafts.

C904 records the Assessment-owned-fork engineering decision; C905-C909 remain the implementation and proof chain for the five
unowned C909 Pool-selection occurrences. The exact duplicate
**"Privacy-safe aggregate Question statistics remain after the underlying Student records are
deleted."** remains A6 C208; it is not an A7 closure. C321 owns the separate A7 aggregate-retention
behavior. A6 C205 is the only unresolved prerequisite: it blocks C322's threshold-dependent
statistics view, not the other A7 work.

The map labels each C300-C375 row and the C825-C831 native correction either as a closure owner or
a contributor. A contributor has no HG occurrence and is dispatchable only as the declared
predecessor of its named closure row. The
valid atomic closures C331-C334, C336-C337, C340, C342-C343, and C347-C351 remain canonical.
C346 is a partial contributor only: it establishes real revision/fork Watch events but cannot close
the notification bullet without the unresolved thread and impact-notice product decisions. The
remaining multi-boundary predecessors are split only where the canonical gap map names them.
Architect approval is required before cross-cutting closures C303, C355, or C359 begins.

Every dispatch is one coder, one named boundary, one outcome, one fresh reviewer, and one exact
gate in the map. Proof starts ignored in `tests/_temp/hg_a7_*`; it is removed at handoff unless a
review against every `docs/PYTEST_STYLE.md` question approves a stable, important, externally
meaningful regression test. C317's 13,000-Question exercise is permanently temporary load
evidence. A failed gate leaves the named occurrence `[ ]`, records the failure, and repairs only
that boundary; it never makes a new policy or broad refactor.

C338's TypeScript temporary seam uses exactly `node --import tsx
tests/_temp/hg_a7_c338_bulk_api_seam.mjs`; the canonical gap-map row owns the same command.

Dispatch rules:

- A correction milestone starts when its declared dependencies are closed and its owned code
  boundary is free (no running milestone owns the same crate, frontend feature, or schema file
  family). Milestones with distinct boundaries run in parallel while other parts are still being
  audited. The manager applies checklist flips and changelog entries serially from each coder's
  report, so those two shared files never gate parallelism.
- One fresh coder subagent and one fresh reviewer subagent per milestone.
- Change the design directly. PLE is pre-production: edit `schemas/base_schema/*.sql` in place
  and keep one path per behavior. Renames follow the terminology rule: each one is tied in the
  contract to an HG behavior, a public contract, or a misleading identifier at a boundary the
  milestone changes.
- Keep every file under 1000 lines; split modules before an addition crosses it.
- Gate order: focused Rust, Node, or Python tests for the touched area; then
  `source source_me.sh && ./launchers/run_fast_checks.sh`; then the affected `tests/e2e/`
  lane(s); then the coder reports flips and a changelog bullet to the manager.
- The manager flips the bullets named in the contract (owning occurrence plus duplicates) with
  the coder's fresh `Evidence (kind):` lines, runs `--diff --consistency`, and adds the changelog
  entry.
- Temporary proofs live in `tests/_temp/` and are promoted or removed at close. Permanent tests
  protect stable, externally meaningful behavior (`docs/PYTEST_STYLE.md` checklist). The
  checklist is the compliance record.

### C400-C425: Complete A8 Courses gaps

C400-C425 is the ordinary A8 dispatch range, disjoint from C300-C375 (A7) and C800-C862
(cross-boundary work). The current A8 rows in the gap map identify the owner, shared-boundary
handoff, temporary proof, and browser closure where required. C400-C407 are contract-only:
their recipient must verify the behavior before a checklist bullet changes. A8 owns only the new
isolated modules/tables named in the map; work that changes an existing shared C6/C7/C19/C46-C55/
C72/C73/C206/C216 boundary waits for its named owner.

The accepted decisions are locked in the map: in-app Watch projection for Revision/publish/archive/restore; six-month Active-to-Inactive policy with warning/deadline/FERPA-clock rules; publish-new Private Blueprint rather than Instance conversion; import into a new actor-owned Private Blueprint; subset Change-Proposal acceptance with Revision/ETag conflict handling; and C880-C884's explicit, Revision-based Blueprint fork update. Backend evidence is not a closure where the map declares an Instructor browser workflow (C411, C413, C420, C421, C423-C425) necessary.

### C880-C884: Blueprint fork review and selective-save chain

Course Revision indication handoff (2026-09-16): manager-approved implementation reuses the
existing authorized Course load and Course page to show an adopted Revision beside a newer current
Revision. It closes only the two obvious-older-Revision indication rows. It creates no update
subsystem, state, table, or workflow and does not close C410 or C411's offer/review/approval/apply
work.

Course Revision update status (2026-09-16): C410/C411 are closed by the authorized lazy Course
summary and existing detail workflow. The summary derives current-parent adopted-Assessment rows
for changed, matching, removed-source, Type-mismatch, and automatically-added correspondences;
direct local Assessments are excluded. Each detail Apply uses `expectedSourceRevision` plus
daughter Edit CAS under parent -> Course -> Assessment authorization/visibility locks. The accepted
desktop and narrow proof confirms lazy open/reopen, no POST on Cancel, exact Revision-2 Apply, and
a refreshed match. It introduces no persisted offer, receipt, comparison baseline, new update
table, or Store.
Trusted Pool materialization preserves dates, status, origin pins, and Student Work; reusable
semantic equality precedes fresh identities. Removed members and Type mismatches have clear
cannot-apply results, with no implicit deletion or type conversion. This does not close a
whole-Course lifecycle milestone or claim direct Assessments or all Student Work.

The latest Human Guidance is binding. Its Blueprint comparison section requires comparison of any
visible related Blueprint Courses in the same fork lineage, normally using newest source and fork
Revisions. Shared Question IDs provide durable content relationships and match Blueprint
Assessments; show shared, added and removed Assessments and Question IDs and canonical-JSON changes
through renames, order and structural changes. Recorded origin remains fork provenance, not a
required three-way comparison baseline. C880-C883's earlier direct-source/internal-Assessment-ID
proof remains contributor evidence only; C880-C882 require reconciliation to these requirements
and C884/C413 connected workflow proof remains open. History availability and obvious newer-source
and newer-downstream indications also remain open. No new persistence, artificial matching IDs,
public classification vocabulary or merge framework is implied; unresolved matching presentation
and implementation details must be settled from the current authority and source before dispatch.

C880 contributor record (2026-09-16): accepted source-only `question_model` comparison code now
constructed the earlier canonical Revision projection and preserved internal identities and source order across
unchanged, source-only, fork-only, and overlapping changes. It compares canonical Question JSON
exports, not PostgreSQL layout. Constructors reject duplicate IDs so a map cannot silently discard
an item. The accepted temporary consumer also covered module-parent/current-name changes, pins, and
defaults. At C880 acceptance C881-C884 and C413 were open; C881-C883 are now
contributor-verified for their earlier bounded scope, while the new same-lineage and Question-ID
matching requirements remain open alongside C884 and C413. C880 itself adds no read, authorization,
persistence, server, apply, UI, baseline, or public-state work.

Viewing and comparison follow ordinary Blueprint visibility for both fork and source: every vetted
Instructor may view Public and Archived Blueprints; explicit Archived inclusion governs discovery,
not permission to view a known Course. Private Blueprints are owner-only on either side. Ownership
controls the fork's apply mutation. The Instructor selects displayed changes, potentially several
related changes in one request. The server builds and validates one coherent complete fork tree and
saves selected content as one immutable Revision through the ordinary expected-current Revision CAS.
Selected name changes use the ordinary metadata ETag. The user corrected this existing-visibility
record on 2026-09-16; it is not a new product decision. Star, Watch, adoption, Course, and Student
state remain excluded, and no source change applies automatically.

| ID | Atomic owner and outcome | Dependencies | Temporary proof and permanent-test decision |
| --- | --- | --- | --- |
| C880 | VERIFIED except the expanded changed-content detail: current pairs match Assessments by shared Question IDs across renamed, reordered, and split content, with disjoint sets unmatched. | C412,C414. | Accepted 18-request actual HTTP receipt at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json`; no permanent test. The collapsed browser detail remains open. |
| C881 | VERIFIED: ordinary visibility and nonenumeration permit arbitrary visible related sibling/transitive current pairs in either orientation. | C412,C51,C880. | The accepted lineage receipt conceals Private intermediates and denies unrelated pairs; no permanent test. |
| C882 | VERIFIED: the server derives on-request current-pair canonical JSON comparison without persistence. | C880,C881. | The accepted lineage receipt records real GETs with unchanged product and Student Work fingerprints; no permanent test. |
| C883 | VERIFIED: selected Apply uses fresh local Assessment/Pool identities, explicit existing/new targets, CAS/ownership/Archived guards, and atomic rollback. | C882. | Accepted 84-request HTTP receipt at `/private/tmp/ple-blueprint-owned-pool-artifacts.vs0NCo/blueprint-local-id-apply-http-proof.json`; no permanent test and no populated Student Work claim. |
| C884 | VERIFIED for the current-pair Instructor review and selected Apply workflow at desktop and narrow viewports. | C883. | Accepted compiled-main browser receipt at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-browser.json`; it does not close broader C413, login/TLS, or full accessibility. |

Latest-authority reconciliation (2026-09-16): the table above preserves prior contributor outcomes,
not completion of the current comparison contract. C880 is OPEN for shared-Question-ID Assessment
relationships and shared/added/removed content correspondence independent of internal Assessment
IDs. C881/C882 are OPEN for visible related pairs beyond the direct parent/source route and its
connected review. C413 additionally owes Blueprint history and obvious newer-source/downstream
indications; its prior source-row read-only proof does not establish these. C883's backend evidence
must be revalidated against the corrected comparison input before C884/C413 closure. No new
milestone chain or predetermined technical matching algorithm is introduced.

User clarification (2026-09-16): Assessment internal IDs are local to each Blueprint Course;
forks receive fresh Assessment IDs. There is no persistent Assessment lineage/history contract
across Blueprint Courses. Comparison infers relationships only from shared Question IDs; disjoint
Assessment Question sets remain unmatched. Audit fork creation and selective Apply for reused
cross-fork Assessment IDs. C883's previous proof is historical contributor evidence, not closure
of corrected matching/Apply where it relies on cross-fork identity equality. This clarification
does not authorize a new Assessment lineage model or require changes to HG wording in this pass.

`C412`/`C413` known-forks contributor (CONTRIBUTOR-VERIFIED): the ordinary-visibility backend
reader projects each visible child fork's Reference, current name, current Revision, and verified
owning Instructor display name from existing ancestry/index data. It returns neither Private
children nor their owner identities and creates no state. Accepted reader/API and read-only UI
source-entry evidence is `ohrNXz` and `dr4JU2`; it is independent of the C882 review route and
C883 apply. Direct fork discovery and apply UI remain open.

The C413 read-only source-entry/known-forks/current-comparison UI slice has accepted evidence.
Apply controls depend on C883; complete C413 closure still waits `{C47,C50,C884}`. This permits UI
validation without freezing the selective-apply backend; direct fork discovery and apply UI remain
missing.

`C413` remains the only Instructor UI closure for discovery, the known-forks reader, review, and
deliberate selected application. It creates no Private disclosure or source-owner exception.
Question-level hunk selection is optional and does not block C880-C884 or C413.

Every dispatch uses the exact ignored temporary probe and recipient gate from the map, removes that probe at handoff unless an independent docs/PYTEST_STYLE.md review justifies a durable replacement, and records the result before the manager changes a checklist status. The A8 title/reference inventory is an ignored C216 prerequisite, not a permanent DOM/file-list test.

### C500-C536: Complete A9 Assessments gaps

C500-C536 is the A9 implementation range, disjoint from C400-C425 and C800-C862. This plan and
the current A9 gap-map rows give the same practical handoffs; use the one that names the affected
shared file as the writer. C502 contributes to C216 and closes no A9 occurrence. Every row below has one owner, one
boundary, one outcome, and one verification step. Proof starts ignored under `tests/_temp/hg_a9_*`;
the named cleanup removes it after the observation is recorded. "Conditional retain" means retain
one focused permanent replacement only if an independent review answers yes to every
`docs/PYTEST_STYLE.md` question; otherwise remove it. "Remove" means do not promote it.

**DD-A9-01 precedes C500.** Record the approved direct preproduction cutover in
`docs/DESIGN_DECISIONS.md`, `docs/CONTRACTS.md`, and terminology contract: Assessment is the
only generic product object; Assignment is only one of the three display Types. Rename generic
schema/domain/API/Blueprint JSON/routes directly, with no compatibility SQL view, dual DTO, import
path, redirect, or legacy endpoint. Update callers to canonical Assessment APIs and remove the
legacy route/API/test surface in the same cutover. Preserve public A-reference/UUID values and Type
enums. There is no mixed-version deployment: rollback restores both previous code and its matching
resettable base together.

**Cutover strata are dependency-graph atomic.** C500 and C501 were originally described by their
root files, but neither root is independently installable or compilable. Their smallest safe
implementation units are therefore the root plus every consumer made invalid by that root's
rename. This is one direct cutover, not a compatibility layer: no view, alias, dual decoder, or
old-name adapter may be introduced to split it further. C870 first removes only dormant H5P
placeholder seams. `attempt_presentation.sql` and its LDA consumers remain: they are the live
native C857 and WeBWorK issued-presentation/reproduction evidence boundary, and C500 must rename
their generic Assessment vocabulary. C857 first hands off its `question_model` changes. At dispatch time, the owner
regenerates the ignored direct-consumer manifest from the handed-off tree, changes that whole
manifest in one task, and deletes the manifest after its gate. It is diagnostic evidence, not a
permanent source-inventory test. C500 and C501 have disjoint final file ownership and may run in
parallel only in isolated worktrees after both handoffs. This workspace is shared, so the manager
serializes their mutating bundles and runs no install, compile, or integration gate until both are
present, then uses T-A9-1 as the first combined receipt. The consumer-manifest preflight and
independent review may run in parallel without touching production files.

| ID / owner lines | Atomic outcome and exact boundary (current -> canonical) | Exact temporary proof; observation, failure, cleanup, permanence |
| --- | --- | --- |
| C500 / L3,L5,L7,L11,L13 | **One install-safe schema cutover bundle.** Direct-cutover `schemas/base_schema/assignments.sql` and `attempts.sql` plus every post-C870 installed SQL consumer of their generic tables, types, functions, grants, and API names: `assignment_operations.sql`, `attempt_access.sql`, `attempt_finalization.sql`, `attempt_history.sql`, `attempt_interaction.sql`, `attempt_operations.sql`, `attempt_operations_api.sql`, `attempt_presentation.sql`, `blueprint_lineage.sql`, `blueprints.sql`, `corrections.sql`, `course_blueprint_adoption.sql`, `course_operations.sql`, `course_retention_transitions.sql`, `delivery.sql`, `delivery_backends.sql`, `grading.sql`, `grading_access.sql`, `question_asset_operations.sql`, `student_assignment_landing.sql`, `unrelease.sql`, and `install.sql`. `attempt_presentation.sql` is retained and renamed because it is the actual native C857 and WeBWorK reproduction boundary, not an H5P placeholder. The execution manifest also includes any direct consumer newly exposed by H5P removal. The resulting base schema contains only Assessment/Assessment Attempt variants and associations. | Regenerate ignored `tests/_temp/hg_a9_g01a_schema/direct_consumers.txt`; run `source source_me.sh && python3 tests/_temp/hg_a9_g01a_schema/probe.py` and a fresh base-schema install. Prove both variants, reject the generic category, and prove install succeeds. Repair the one schema graph bundle, then remove the manifest and probe; conditional retain only a stable external schema/data contract. |
| C501 / L9,L97,L109,L119,L121,L213,L215 | **One compile-safe Rust cutover bundle.** Direct-cutover `question_model/src/assignment.rs`, `assignment_activity_rules.rs`, and `domain/src/assignment_activity.rs` plus every post-C857/C870 Rust workspace consumer that imports, names, serializes, constructs, or tests their generic Assignment/attempt symbols. The owner moves modules and updates all affected `question_model`, `domain`, `learning-data-access`, `server`, and workspace test consumers in the same change; no `pub use` alias, old module path, or dual type remains. Blueprint rejects delivery/Attempt while Course Instance allows it. | Regenerate ignored `tests/_temp/hg_a9_g01b_domain/direct_consumers.txt`; run `source source_me.sh && python3 tests/_temp/hg_a9_g01b_domain/probe.py` and `source source_me.sh && cargo test --workspace --no-run`. Prove the domain variant boundary and workspace test compilation. Repair the one Rust consumer graph bundle, then remove the manifest and probe; conditional retain only a stable external domain contract. |
| C502 / contributor only | Inventory `src/routes.ts`, `src/route_contract.ts`, and current Assessment editor/overview/Attempt/workspace pages for C216; every recognition/copy/entry surface has a human title/reference or named page owner. | `node tests/_temp/hg_a9_g01c_title_reference_inventory/inventory.mjs --routes src/routes.ts --contract src/route_contract.ts`; failure creates the focused owner correction; remove. |
| C503 / L19,L23,L107,L302 | The current Assessment schema uses tagged direct-versus-adopted origin, and manual `ple_data.create_assessment` creates direct rows with no Blueprint triplet. Accepted private actual-main/HTTP proof created a direct Practice Assessment in an Empty Course, added/saved one available Published Question exact Revision/point pin, set Due/time limit, checked readiness, and released it. A real roster import/claim then showed that Released direct Assessment to its Student member, withheld an Unreleased sibling, and exposed pre-start facts. A separate real native fixed-Question HTTP run saved/restored the PKU choice, whole-submitted a graded 1/1 Attempt, and started a distinct unlimited Practice Attempt afterward. Pool content, full Student browser Attempt interaction, other backend delivery/grading, and complete reusable/direct Assessment behavior remain pending; `domain/scoring.rs` remains the scoring boundary. | Accepted private direct-create, direct-author/release, Student member-visibility, and native fixed-Question Attempt HTTP/browser artifacts plus current `schemas/base_schema/assessments.sql` origin constraint and `schemas/base_schema/assessment_creation.sql` `ple_data.create_assessment`; next focused Pool/browser-delivery gate before broad closure. |
| C504 / L21 | Instructor entry authoring: canonical Assessment content list, Pool editor, and picker controllers support authorized add/remove/reorder of either kind. | `node tests/_temp/hg_a9_g03_authoring/probe.mjs`; repair UI/API authorization or action boundary; remove. |
| C505 / L25 | Rename `assignment_workspace_policies_page.tsx` to `assessment_workspace_properties_page.tsx`; the accessible label is exactly `Randomize question order`. C64 alone owns persisted question-order policy. | `node tests/_temp/hg_a9_g04_label/probe.mjs`; repair label only; remove. |
| C506 / L30,L32,L34,L38,L102,L123 | Fixed five-Type registry in canonical Assessment model, Assessment Properties, and schema. | `source source_me.sh && python3 tests/_temp/hg_a9_g05_types/probe.py`; prove exactly five Types across variants; repair registry, then remove; conditional retain. |
| C507 / L36 | Selection-only canonical Assessment creation command/UI: permitted selected Type, denied arbitrary Type creation. | `source source_me.sh && python3 tests/_temp/hg_a9_g06_selection/probe.py`; repair authorization/route; remove. |
| C508 / L40,L42 | Rename `effective_assignment_policy.rs` to `effective_assessment_properties.rs`; per-Assessment Properties override preserves Type/default identity. | `source source_me.sh && python3 tests/_temp/hg_a9_g07_override/probe.py`; repair properties model; remove. |
| C509 / L44,L46,L48,L50,L61,L65 | Put Type pedagogy copy in canonical create UI/registry metadata. | `node tests/_temp/hg_a9_g08_pedagogy_copy/probe.mjs`; required descriptions render; repair copy, remove. |
| C510 / L52,L57,L59 | Practice/Bonus grade contribution belongs in the production SQL projection boundary: a focused helper in `schemas/base_schema/grading.sql`, consumed only by Gradebook and pre-start SQL projections, plus the C75 live-gradebook adapter/decoder acceptance of earned points above points possible. Preserve raw Assessment Attempt activity scoring everywhere else; the Student's point-based Assessment score must select the highest submitted Attempt, not the latest. Bonus contributes direct earned points with zero points possible. PLE performs no Course-total, percentage, or grade-scheme calculation; Course-level weighting belongs in the home LMS. `crates/domain/src/scoring.rs` has no production grade-contribution caller, so do not add an orphan Rust helper or tests there. | `source source_me.sh && python3 tests/_temp/hg_a9_g09_bonus_practice/probe.py`; prove ordinary raw Attempt scores remain unchanged, a Bonus Assignment contributes earned points above zero points possible through the actual Gradebook/pre-start SQL projections and live adapter decode, and the production Assessment-score projection selects the highest submitted point-based Attempt. Repair the production projection boundary, then remove the probe; retain only contract coverage grounded in the delivered projection. |
| C511 / L63,L67 | Quiz and Exam Assessment Properties remain configurable so an Instructor may choose settings more restrictive than a Regular Assignment. Human Guidance does not require Type-mandated restrictive defaults or arbitrary thresholds. This row makes no collaboration-policy decision. | `source source_me.sh && python3 tests/_temp/hg_a9_g10_restrictive_properties/probe.py`; record the supported Properties controls and prove a chosen stricter configuration round-trips without a Type-forced restriction. Record unavailable `availableAt`/`closesAt` authoring and any undefined collaboration policy as open; remove the probe. |
| C512 / L72,L74,L76,L78,L80,L82,L84,L86,L88,L90,L92 | Five Type tokens in `src/ribbon/ribbon_icons.ts`, `src/style.css`, and presentation: exact icons and visible label without color-only meaning. | `node tests/_temp/hg_a9_g11_type_tokens/probe.mjs`; repair presentation, remove. |
| C513 / L114,L127,L129 | A9-side Assessment propagation after A8 adoption: copy, daughter isolation, and new item unreleased; do not edit A8-owned source. | `source source_me.sh && python3 tests/_temp/hg_a9_g12_propagation/probe.py`; repair propagation, remove. |
| C514 / L111,L134,L136,L138,L142,L150 | Template schema/new model stores Type/settings only, never content or Blueprint. | `source source_me.sh && python3 tests/_temp/hg_a9_g13_template_schema/probe.py`; repair schema; conditional retain after removal decision. |
| C515 / L140 | Template ownership command/UI permits owner and denies nonowner. | `source source_me.sh && python3 tests/_temp/hg_a9_g14_template_owner/probe.py`; repair authorization, remove. |
| C516 / L144,L146,L148 | Template create/copy isolation: later edits do not bleed between copies. | `source source_me.sh && python3 tests/_temp/hg_a9_g15_template_copy/probe.py`; repair copy boundary, remove. |
| C517 / L125 | Course Instance delivery fields/policy only; Blueprint has none. | `source source_me.sh && python3 tests/_temp/hg_a9_g16_delivery_base/probe.py`; repair variant boundary, remove. |
| C518 / L263 | IN PROGRESS: current Human Guidance requires finite Assessment duration, at most 250 delivered Questions, Pool selection count for the limit/default, rounded 1.5-minutes-per-Question default, and explicit Instructor override up to 12 hours. Authored NULL means calculated default; positive seconds mean explicit override. Individual 1.5X/2X accommodations and effective 24-hour cap remain a separate incomplete slice, not ratio/UI acceptance. | The default/override cutover uses one current-content SQL base helper at read/start without a derived persisted cache. Pools count `selection_count`, not whole membership. Accepted independent SQL reproduces the former role-order failure, proves fixed-entry 1/2/3/250 arithmetic, empty/251 refusal, the 12-hour constraint, and ordinary Instructor-release/Student-start/resume; it does not establish browser, HTTP, renderer, real Instructor override save, accommodation, or multi-selection Pool timing acceptance. |
| C519 / L164,L166,L169,L171,L173 | `schemas/base_schema/assessments.sql` owns one shared issue-producing release validator and hard gate; `assessment_operations.sql`, LDA, and the browser release contract project actionable issues. `domain/validation.rs` remains the Student-response parser. | `source source_me.sh && python3 tests/_temp/hg_a9_g18_release_rules/probe.py`; repair validation; conditional retain compact cases. |
| C520 / L158,L160,L162,L177 | The shared Assessment schema defaults new Course Assessments to Unreleased; accepted private direct and Public Blueprint-adopted actual-main/HTTP creation proof verified that initial state. The subsequent direct fixed-Question release passed readiness and release after valid Due/time limit. Complete release-validation coverage and Student delivery remain open. | Accepted private direct/adopted creation and direct-author/release artifacts plus current `schemas/base_schema/assessments.sql` `assessment_status` default; future focused release/Student delivery gates before C520 completion. |
| C521 / L175 | Assessment Properties UI supports invalid -> repair -> rerun -> release. | `node tests/_temp/hg_a9_g20_rerun/probe.mjs`; repair Properties UI; remove. |
| C522 / L179,L181 | Student delivery and Work issuance through server/data-access/access pages. | `source source_me.sh && python3 tests/_temp/hg_a9_g21_delivery/probe.py`; prove allowed and denied delivery/issuance; repair access, remove. |
| C523 / L183,L185,L187 | Due/late defaults in policy/create: due cutoff and late rejection. | `source source_me.sh && python3 tests/_temp/hg_a9_g22_due_defaults/probe.py`; repair defaults, remove. |
| C524 / L54,L189,L191,L193,L195 | Six independently timed disclosure fields in policy/summary/UI; Question Feedback is shown when provided and has no separate delayed-release state. | `node tests/_temp/hg_a9_g23_disclosure/probe.mjs`; repair policy/UI, remove. |
| C525 / L198,L200 | Open implementation work: the release cohort uses current Student Course relationships. Pending invitations are excluded; accepted enrollment joins; ended or withdrawn relationships and deactivated Course access exit; Account deactivation preserves membership; never-started Work without a submission blocks release; and Course end is not completion. Existing `assessment_submission` evidence makes every Assessment Attempt complete on whole Student submission or expiry automatic submission, independently of score or correctness. An accepted actual-server native Practice run whole-submitted a graded 1/1 Attempt, then issued a distinct second Attempt under unlimited policy despite the perfect score. Quiz and Exam still permit exactly one Attempt. | Narrow acceptance evidence covers whole Student submission, expiry automatic submission, and unlimited Attempts after a perfect score at the existing submission and cohort boundaries. The native HTTP receipt contributes only the unlimited Practice/perfect case; cohort transitions/expiry and Quiz/Exam are not claimed by it. Do not add a completion snapshot, latch, new DAG, or bookkeeping machinery. |
| C526 / L217,L273,L275 | Cross-session resume in Attempt data access/delivery/issuance preserves same ID, response, and expiry. | `source source_me.sh && python3 tests/_temp/hg_a9_g25_resume/probe.py`; repair boundary; conditional retain. |
| C527 / L219,L221,L223 | Attempt limits with Regular unlimited behavior. | `source source_me.sh && python3 tests/_temp/hg_a9_g26_repeat/probe.py`; finite denial and unlimited Regular; repair policy; conditional retain. |
| C528 / L237,L250 | Student response/grading/adapters require complete-or-unsaved response. | `source source_me.sh && python3 tests/_temp/hg_a9_g27_response_complete/probe.py`; incomplete never persists; repair boundary, remove. |
| C529 / L245,L258 | Student result UI makes unanswered visible and outcome hidden before submit. | `node tests/_temp/hg_a9_g28_result_visibility/probe.mjs`; repair UI, remove. |
| C530 / L252 | Pre-submit interaction bridge never assigns final grade from interaction. | `source source_me.sh && python3 tests/_temp/hg_a9_g29_interaction/probe.py`; repair bridge, remove. |
| C531 / L265 contributor | A5 pre-start/timer presentation only: time visible before start, timer subtle and outside Question card. | `npx playwright test tests/_temp/hg_a9_g30_timer/timer.spec.ts`; manager Browser Suite lease required; repair A5 UI, remove. |
| C532 / L290 | Pool/Question revision evidence preserves exact revisions across later edit. | `source source_me.sh && python3 tests/_temp/hg_a9_g31_pool_revision/probe.py`; repair evidence store; conditional retain. |
| C533 / L295 | Delivered evidence is isolated: later edit cannot replace delivered evidence. | `source source_me.sh && python3 tests/_temp/hg_a9_g32_evidence_isolation/probe.py`; repair isolation; conditional retain. |
| C534 / L297 | Minimal-retention inventory with A6: each retained field has an interpret/grade purpose. | `source source_me.sh && python3 tests/_temp/hg_a9_g33_retention/probe.py`; repair retention, remove. |
| C535 / L319 | Prohibited-model inventory over schema/scoring/APIs; review candidates but do not claim universal absence. | `bash tests/_temp/hg_a9_g34_absence_inventory/inventory.sh`; investigate a hit; remove. |
| C536 / L322,L324 | **Terminal product question; no code:** Which identity/LMS-matching columns must CSV/TSV contain, and what Student/Course metadata is permitted or forbidden for matching without unnecessary FERPA disclosure? | `source source_me.sh && python3 tests/_temp/hg_a9_g35_export_question/record_question.py`; fresh report records exact question. Failure blocks export route/schema; remove. |

**Per-row dependencies.** An empty entry means the prerequisite is DD-A9-01 or no unclosed
prerequisite. These notes prevent overlapping edits; they are not a separate validation system.

| ID | Required closed handoff(s) |
| --- | --- |
| C500 | DD-A9-01; C870 H5P placeholder removal. `attempt_presentation.sql` remains a required C857/WeBWorK rename consumer. |
| C501 | DD-A9-01; C857 author-content `question_model` handoff; C870 H5P placeholder removal. Existing presentation LDA remains a required rename consumer. |
| C502 | T-A9-1; it contributes only to C216 |
| C503 | T-A9-6 canonical terminology chain; hands tagged direct-versus-adopted Assessment origin to C7 for its final started-empty closure |
| C504 | C503; A2 authorization; A4 Instructor interface |
| C505 | C503; C64 owns persisted question-order policy |
| C506 | T-A9-6 |
| C507 | C506; A2 authorization; A4 Instructor interface |
| C508 | C506 |
| C509 | C506 |
| C510 | C506 |
| C511 | C506 |
| C512 | C506 |
| C513 | A8 adoption; T-A9-1 canonical variants; C503 content |
| C514 | C506 |
| C515 | C514; A2 authorization; A4 Instructor interface |
| C516 | C515 |
| C517 | T-A9-6 |
| C518 | C517 |
| C519 | C503; A7 Question validity; A8 six-month Active limit; C518 |
| C520 | A2 authorization; C517; C519 |
| C521 | C520; A4 repair/rerun interface |
| C522 | A2 authorization; A5 Student interface |
| C523 | C517 |
| C524 | C506; C508; A5 Student display; A7 backend outcome/feedback |
| C525 | existing `assessment_submission` authority and current Student Course cohort |
| C526 | C522 issuance; A7 response contract; C528 complete-response boundary |
| C527 | C508 policy override; C522 issuance |
| C528 | A7 response/adapters |
| C529 | C528; C524 |
| C530 | A7 response/adapters |
| C531 | A5-C78; A5-L48; manager Browser Suite lease |
| C532 | C522 issued Work; A7 Pool selection/revision |
| C533 | C503 content; C532 selected evidence |
| C534 | A6 retention owner; C533 evidence isolation |
| C535 | no unclosed prerequisite |
| C536 | terminal product question; after resolution, A2 authorization, A4 export UI, and security/privacy owner |

**Required terminology chain (before any A9 terminology `[x]`).** C12 is contributor-only frontend
evidence. `{C857,C870} -> {C500,C501} -> T-A9-1 integrated schema/compile barrier -> T-A9-2 LDA/server canonical API -> T-A9-3 TypeScript
decoders/client -> T-A9-4 routes/link emitters -> T-A9-5 direct caller and no-legacy verification -> T-A9-6
fresh-install/E2E`. T-A9-2 has temporary
`source source_me.sh && python3 tests/_temp/hg_a9_t2_server_api/probe.py`; T-A9-3 has
`node tests/_temp/hg_a9_t3_client/probe.mjs`; T-A9-4 has
`node tests/_temp/hg_a9_t4_routes/probe.mjs`; T-A9-5 has
`node tests/_temp/hg_a9_t5_no_legacy_routes/probe.mjs`; T-A9-6 has
`source source_me.sh && python3 tests/_temp/hg_a9_t6_fresh_install/probe.py` then manager-leased
`bash tests/e2e/e2e_live_demo_course_instance.sh --browser`. T-A9-1 is owned by the cutover
integrator: it runs a fresh base-schema install and `source source_me.sh && cargo test --workspace --no-run`
against the combined C500/C501 tree, records the two successful receipts, and removes its ignored
receipt. A failure returns the broken graph to C500 or C501; it never adds compatibility. All
T-A9 probes remove by default; only one stable external canonical API contract may be
considered for retention after the six-question review. A failure repairs that chain stage and
reruns all later stages.

Canonical routes are `/courses/:courseRef/assessments/:assessmentRef`,
`/assessment-attempts/:assessmentAttemptRef`,
`/assessment-attempts/:assessmentAttemptRef/summary`,
`/instructor/courses/:courseRef/assessments/new`, and
`/instructor/courses/:courseRef/assessments/:assessmentRef` with `/questions`, `/properties`,
`/student-view`, and `/delivery-check`. Parameters remain `:assessmentRef` and
`:assessmentAttemptRef`. Canonical JSON spellings are `assessment`, `assessmentReference`,
`assessmentAttempt`, `assessmentEntry`, `assessmentStatus`, `assessments`, and
`blueprint_assessment_reference`; new public JSON never emits generic Assignment/attempt names.
Legacy `/assignments` browser routes, APIs, decoders, and their tests are absent; callers use the
canonical routes directly.

**Acyclic dependencies.** `DD-A9-01+C870->C500; DD-A9-01+C857+C870->C501; {C500,C501}->T-A9-1->T-A9-2->T-A9-3->T-A9-4->T-A9-5->T-A9-6; T-A9-1->C502; T-A9-6->{C503,C506,C517}; C503->{C504,C505,C7}; A2+A4->{C504,C507,C515}; C64+C503->C505; A8+T-A9-1+C503->C513; C506->{C507,C508,C509,C510,C511,C512,C514}; C514->C515->C516; C517->{C518,C523}; C503+A7+A8+C518->C519; A2+C517+C519->C520; C520+A4->C521; A2+A5->C522; C522+A7+C528->C526; C508+C522->C527; C506+C508+A5+A7->C524; C528+C524->C529; A7->{C528,C530,C532}; C522->C532; C503+C532->C533; A6+C533->C534; A5-C78+A5-L48->C531; C536 terminal; resolved C536+A2+A4+security owner unlocks export.`

Close this range only when all 104 owners verify or C536 retains its exact question, every
temporary path is removed or has its documented six-question retention decision, and
`source source_me.sh && python3 devel/human_guidance_checklist.py --gate 09_assessments.md`
exits zero. Then run the existing Browser Suite's applicable serial lane; do not start, stop,
replace, or clean that shared suite.


## Milestone R1: Regenerate derived evidence

After the last correction milestone closes: regenerate screenshots and capture manifest
(`docs/screenshots/`), the Ribbon destination ledger (`tests/e2e/e2e_ribbon_destination_ledger.mjs`
-> `docs/ux/RIBBON_DESTINATION_LEDGER.md`), Graphify output, and `docs/SCREENSHOT_ATLAS.md`.
Remove `Generated evidence stale:` notes. Gate: regeneration commands exit 0; screenshot contract
tests pass; changelog entry.

### Interim full-corpus refresh checkpoint

Before final R1 acceptance, run one fresh Live Demo replay that captures every current manifest
entry for Public, Instructor, Student, and Sysadmin, then regenerate and validate the manifest and
atlas together. The Student portion explicitly includes all eight native Question Types and a
WeBWorK delivery/review scenario. A required scenario without a newly captured PNG keeps the
corpus unrefreshed; a static-only check cannot substitute for replay. Keep stale or retired images
outside the current corpus and record them as such. This checkpoint adds no alternate release
path, status bookkeeping, or dependency graph; R1 remains the final derived-evidence gate.

## Milestone R2: Re-audit the checklist

Nine fresh subagents repeat A1-A9 against the corrected system; the manager repeats the per-part
gate, spot-check, and splice. Any newly exposed `[ ]` item goes through Milestone G and a new
correction milestone before R3. Gate: every product-behavior bullet is `[x]`, or `[ ]` with
`Reason: HG: no locked-in design`, or `[ ]` with `Reason: product decision still unclear` plus
its `Question:` line; every non-implementation bullet is audited N/A with a valid reason
(including the permitted inherited reason for `How to use this guidance`). Bullets with a
recorded `Decision:` reach `[x]` through that implementation; `Decision:` and `Question:` notes
stay in place for Neil's later review. The fresh reports are refreshed from the re-audited
checklist (Milestone B, second run), and R3 lists the still-unclear items in its changelog entry.

## Milestone R3: Final verification and cleanup

- `source source_me.sh && ./launchers/all_test.sh` green.
- Remove legacy tables, routes, DTOs, page files, CSS, tests, and fixtures that the HG model
  replaced.
- Review permanent tests added by this plan; keep those protecting important stable behavior
  that could plausibly regress; delete the rest and every `tests/_temp/hg_*` probe.
- Prepend the final summary block to the checklist: `[x]`/`[ ]`/`N/A` counts per top-level HG
  section, date, `git rev-parse HEAD:docs/HUMAN_GUIDANCE.md`.
- `git mv` the plan to `docs/archive/`; the checklist stays in `docs/active_plans/audits/` as
  the living compliance record; changelog entry with final counts.

## Autonomous completion

- Every gate is a command exit code or a mechanically checked subagent report.
- Live behavior is verified on the disposable stack with Playwright or `curl`.
- Authentication bullets are verified against the seeded-role demo entry,
  `tests/e2e/e2e_invitation_mailer.*`, and code.
- Time-based behavior (expiry, six-month limit, retention) is verified with synthetic timestamps
  and existing SQL oracles.
- Reviewer subagents provide review. Planner-reviewer disagreement is settled by the stricter
  reading of HG; when HG is silent, the decision rules above apply.
- Closeout is deterministic: HG-settled bullets reach `[x]`; the only `[ ]` left at R3 carries
  `Reason: HG: no locked-in design` or `Reason: product decision still unclear`; every decision
  the rules allow is recorded and implemented; the still-unclear list is handed to Neil through
  the fresh compliance reports and the R3 changelog entry.

## Scope boundary

The plan covers every HG bullet: the checklist, the gap map, correction milestones wherever an
in-scope bullet's evidence lives (`src/`, `crates/`, `schemas/`, `tests/`, `launchers/`,
`local_stack.py`, `local_stack_control/`, `containers/`), and generated evidence under `docs/`.
Work with no HG bullet behind it (cloud deployment, backups, the compliance report folder rename)
belongs to other plans.

### Human-facing reference IDs (2026-09-15)

HG now requires short, cryptographically random Crockford Base32 reference IDs for Blueprint
Courses (`BP`), Course Instances (`CI`), Assessments (`A`), and Sysadmin support Accounts (`U`).
The implementation uses six shared random characters; that length is an implementation decision,
not an HG requirement. The prefix is joined directly to the random portion, uniqueness retries
collisions, and internal primary keys remain unchanged. `CI` currently denotes a private Course
Invitation transport reference, so that use is retired or reassigned directly with no legacy alias
before `CI` becomes a Course Instance reference. Account `U` references remain Sysadmin-only, and
course-scoped roster identities stay private. Private `R`, `W`, `D`, and `M` route tokens gain no
human-facing label unless a real workflow needs one. Published Question and Question Pool
`AAAA-ZBBB` identities remain unchanged.

## Verification summary

| Milestone | Gate |
| --- | --- |
| 1 | `--diff` clean; bullet counts equal; lint, typing, shebang, markdown-link tests pass |
| A1-A9 | `--gate` exits 0; risk-based spot-check clean (or part re-audited); spliced; `--diff --consistency` clean |
| G | Every owning `[ ]` mapped once; reviewer confirms each milestone is self-contained, single-boundary, and bounded by HG |
| C* | Focused tests; `run_fast_checks.sh`; affected E2E lane; bullets flipped; `--diff --consistency`; changelog |
| R1 | Regeneration exits 0; screenshot contract tests pass |
| B | Fresh reports cover every `[ ]` once; links test passes; counts match the checklist summary |
| R2 | Re-audit complete; every remaining `[ ]` carries `HG: no locked-in design` or `product decision still unclear`; reports refreshed |
| R3 | `all_test.sh` green; cleanup done; summary block; plan archived |

Latest Student response distinction rewrite: live HG requires visually distinct current Question,
saved-response status and keyboard focus, plus response-effect labels distinguishing Save/Clear/
change from whole Coursework submission. The shared native control now labels its default action
`Restore initial response`; isolated actual-component evidence shows mount-baseline restoration,
not a clear/delete operation or Student workflow persistence. Both rows remain open; earlier
navigation styling receipts are retained as partial proof, not blanket acceptance of native
response controls/actions.
