# Peptidyle Learning Engine project status report

Report date: 2026-08-10
Plan authority: [implementation_plan.md](../implementation_plan.md)
Release completion: [release_completion_plan.md](../active/release_completion_plan.md)
Owner decisions: [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md)
Execution handoff: [implementation_status.md](../implementation_status.md)
Previous snapshot: [project_status_report_2026-08-09.md](../project_status_report_2026-08-09.md)

## Status language

This report is the current dated executive snapshot. It does not replace the active implementation
plan and does not claim that the full project or version 1 release is complete.

- **Accepted** means the package passed its named executable gates and an independent reviewer
  reported no unresolved P0/P1 finding.
- **Implemented, acceptance open** means substantial working code exists, but the package's complete
  integrated gate or independent closeout has not passed.
- **Decision accepted, implementation open** means the contract, ownership, migration order, and
  gates are settled, but the complete production path is not accepted.
- **Planned and owned** means a dependency-ordered package names owners, artifacts, behavior, and
  gates but has not yet produced accepted implementation evidence.

Accepted evidence remains valid for its exact snapshot and boundary. A later regression is reported
separately; it does not rewrite history or become acceptable merely because the earlier package
passed.

## Executive assessment

**Overall status: advanced code-first implementation; not production-ready.**

PLE now has accepted course appearance, production-seam cleanup, a bounded live WeBWorK path, and
the first source-ownership decomposition. Native flat JSON v2 implements the eight required question
families. WP-RC8 now implements the first usable institution-independent passwordless account and
course-roster slice, including manual grade export, while its production acceptance remains open.
The secure learner-payload design is
decision-complete, and its descriptor codec and first forward migration are present, but the full
browser/API/WeBWorK cutover remains unaccepted. Learner file upload remains deliberately disabled
behind a separate implementation-ready security plan.

The central architecture remains sound:

- grading keys, answer mappings, and correctness decisions stay server-only;
- authenticated attempts bind learner, course, assignment, immutable version, seed, timing, and
  backend authority;
- drafts remain private workspace state while publication creates immutable shared versions;
- educational records remain tenant-owned under forced PostgreSQL row-level security;
- canonical sources and protected artifacts use typed object identities and checksums;
- API replicas remain stateless over PostgreSQL, object storage, and private backend capabilities;
- the browser receives answer-free render projections and a browser-safe Wasm closure; and
- native-only operation does not depend on the optional private WeBWorK profile.

The post-acceptance persistence size regression is repaired. Complete attempt-issuance capabilities
now live in paired in-memory and PostgreSQL owners, while the public Store facade and the original
SQL text, bind order, transaction scope, retry, and RLS behavior remain unchanged. The permanent
source-size gate passes 824 cases with no maintained-source violation. The feature-enabled
persistence check, Store conformance suite, live database oracle, and strict Clippy gate also pass. This
does not turn the mixed worktree into a release candidate or replace the remaining package gates.

## August 10 changes

### WeBWorK RC3 accepted

The source-pinned upstream WebWork2 and PG profile passed the full local PLE path on Podman 6. The
accepted boundary proves one licensed, immutable, user-authored RadioButtons PGML question through:

- authenticated private `/webwork2/render_rpc` rendering and grading;
- one renderer call followed by same-attempt cache hits;
- full and zero server-owned grading with idempotent replay;
- renderer-outage containment and recovery without gateway failure;
- keyboard-only browser operation through the PLE origin; and
- browser-visible absence of PG source, credentials, upstream hidden fields, session keys, and
  answer mappings.

This is intentionally not broad OPL compatibility. WeBWorK MATCH and other PG controls remain
assigned to WP-RC5 rather than inferred from the private renderer implementation.

### Source ownership accepted

WP-ARCH1 moved the dated 26-file oversized-source baseline into capability-sized Rust, TypeScript,
Python, and test owners behind stable facades. Its acceptance snapshot passed focused package gates,
the eleven-stage codebase check, repository Python and browser suites, the permanent size policy,
and independent PostgreSQL, security, provider, TypeScript/HCI, test, size-policy, and architecture
reviews.

The later persistence size regression was repaired by extracting complete paired issuance owners.
The current permanent source-size gate is green again; that repair does not erase or broaden the
accepted WP-ARCH1 snapshot.

### Flat JSON v2 implemented

PLE flat JSON v2 now strictly compiles and server-grades these eight families while preserving flat
JSON v1 single-choice compatibility:

- multiple choice;
- multiple answer;
- fill in the blank;
- multiple blanks;
- numerical entry;
- matching;
- ordering; and
- image hotspot with a non-pointer response path.

The native compiler separates answer-free public render definitions from server-only grading keys.
Strict TypeScript decoders, Solid response controls, and no-mouse interaction coverage exist for the
new wire shapes. WP-RC4 is **implemented, acceptance open**: its invalid-fixture review,
secret-projection scan, final integrated gate, and independent contract/security closeout remain.
Family-specific visual authoring, integrated all-family PostgreSQL/object acceptance, and Chapter 1
pilot content remain WP-RC5 work.

### Payload design accepted

The secure question-grading payload decision is implementation-ready. It uses the authenticated
`AttemptId` plus the idempotency key as submission authority, removes browser-supplied facts the
server can derive, assigns attempt-presentation-scoped CRC-16/CCITT-FALSE IDs to selectable rendered
objects, and uses a separate SHA-256 presentation descriptor for whole-render consistency.

`PresentationDescriptorV1`, rendered-item generation, the Wasm verification seam, and
`2026080908_secure_question_grading_payloads.sql` are present in the current codebase. The current
offline persistence slice now carries an issued WeBWorK mapping through prefetch/attempt storage,
stores only presentation-scoped rendered IDs, validates the row against its owning attempt, performs
normal grading with one private grade RPC, and deletes replay state on successful or terminal
submission. That does not accept WP-P1 through WP-P6. The native endpoint cutover, minimal
learner-screen response, kind-free submission decoder, persisted missing-row self-heal, disposable
PostgreSQL/private-renderer trace, mismatch recovery, measurements, and independent closure remain
dependency-ordered work before WP-RC5 acceptance.

### Passwordless identity and enrollment implemented

WP-RC8 is **implemented, acceptance open**. The current slice provides PLE-owned opaque global
accounts, uniform short-lived browser-bound email authentication, discoverable WebAuthn,
multiple-passkey management, verified email replacement, and authorized account-to-course context
selection. Course managers can page the roster, set exact allowed-email domains, send or revoke a
single invitation, preview and atomically commit a bounded `email,roster_id` CSV, revoke learner
access, and download a synchronous no-store grade CSV. Invitation claim atomically creates or
reuses the tenant learner identity, course membership, every current assignment enrollment, and
every empty summary; assignment creation applies the same cross-product invariant to current
learners.

Forward migration `2026080909_passwordless_identity.sql` passed fresh migration, no-op replay,
ledger verification, role/grant/forced-RLS checks, and the disposable PostgreSQL enrollment oracle.
The all-feature Rust workspace and 76-test browser suite are green, including a keyboard-only
instructor invitation path. Acceptance remains open for a disposable real-email/optional-passkey
and multi-replica journey plus independent security/HCI closeout. Email authentication is the
canonical account path and passkeys are optional shortcuts. Version 1 has no manager-assisted
account merge or educational-record transfer; email possession authenticates only the account
already bound to that email.

### File upload planned safely

The learner file-upload contract is decision-complete while the product remains fail-closed. The
plan requires one opaque attempt-bound upload ID, authenticated same-origin streaming into
non-deliverable temporary storage, exact SHA-256 verification, a closed PDF/text/PNG/JPEG profile,
private malware inspection, typed promotion, atomic manual-grading consumption, RLS, retention,
reconciliation, protected delivery, multi-replica recovery, and keyboard accessibility.

No learner upload route is accepted or enabled. WP-FU1 through WP-FU6 and reserved migration
`2026080912_secure_learner_uploads.sql` runs after passwordless identity/enrollment, object
reconciliation, and LTI and before
production deployment.

## Status dashboard

| Area | Current status | Boundary |
| --- | --- | --- |
| WP-RC1 course appearance | Accepted | Fifteen measured themes, Grass default, exact entry banner, persistence, cleanup, browser and visual gates |
| WP-RC2 production seams | Accepted | Concrete production module/capability names and no hidden native-renderer placeholder |
| WP-RC3 WeBWorK | Accepted, bounded | One immutable RadioButtons PGML path through live PLE render, cache, grade, outage, recovery, and keyboard evidence |
| WP-ARCH1 source ownership | Accepted | Capability owners and stable facades remain below the permanent 1,000-line boundary |
| WP-RC4 flat JSON v2 | Implemented, acceptance open | Eight native runtime/source families exist; independent contract/security closeout remains |
| WP-P1 through WP-P6 payload | Decision accepted; offline persistence slice implemented | Codec, migration, exact replay binding, and one-call normal WeBWorK grading exist; complete API/browser/live cutover and independent evidence remain |
| WP-RC5 and WP-RC6 content/interchange | Planned and owned | Visual family authoring, integrated storage, Chapter 1 content, QTI export, and honest H5P closeout |
| WP-RC7 data hardening | Planned and owned | Object inventory, twice-observed orphan quarantine, missing-reference alerts, and combined M2-M5 gate |
| WP-RC8 identity/enrollment | Implemented, acceptance open | PLE-managed email accounts with optional passkeys, invitation claim, roster/bulk import, atomic enrollments, and manual no-store grade export; real email/optional-passkey/replica and independent closeout remain |
| WP-RC9 LTI | Planned and owned | LTI 1.3 launch and AGS passback; optional institutional SSO remains a non-blocking future account-linking integration |
| WP-FU1 through WP-FU6 uploads | Planned and fail-closed | Server-issued, inspected, attempt-bound learner upload capability |
| WP-RC10 and WP-RC11 operations | Planned and owned | OpenTofu deployment/recovery/scale, then measured bot-cost controls |
| WP-RC12 release | Not started | Complete local and disposable production evidence plus independent multi-discipline audit |

## Product capabilities

### Learning and grading

- Fresh server-owned seeds support repeated algorithmic practice. An issued attempt preserves its
  seed only for resume, re-render, grading, and audit of that same attempt.
- Completion, grading policy, continued practice, variation, retry, feedback, and timing remain
  orthogonal domain policies. The instructor UI should expose pedagogical assignment types rather
  than require routine assembly from low-level enums.
- Automatic grading and response-bearing manual grading remain server-owned and generation-fenced.
- Student summaries expose only policy-authorized correctness and score information.

### Identity and data

- Durable UUIDs identify long-lived entities; compact rendered-item IDs keep learner submissions
  small without weakening authoritative identity.
- Course and learner records are tenant-owned. Shared published versions are immutable and reusable
  without copying content into every course.
- A PLE account is keyed by one opaque global `UserId`; verified email is the mutable canonical
  sign-in attribute, while course authorization and tenant-scoped `StudentId` mappings isolate
  educational records. Course-scoped roster email and roster ID exist only for instruction and
  manual grade export.
- PostgreSQL startup verifies the migration ledger through a least-privilege projection and never
  applies DDL through the application role.
- The accepted six-file baseline and `2026080907_course_appearance.sql` are frozen. Migrations 0908
  and 0909 exist with package acceptance open; migrations 0910 through 0912 remain
  dependency-ordered reservations.

### Browser and accessibility

- The browser is SolidJS with strict TypeScript decoders and a browser-safe Rust/Wasm facade.
- The platform keyboard contract is primary: Tab and Shift+Tab move focus, Space performs native
  selection, and visible controls provide a complete no-mouse path. Arrows, digits, Enter-to-submit,
  and Escape are separately tested PLE extensions.
- The browser may validate response shape and presentation consistency; it does not authenticate
  itself with a checksum and never grades.
- Learner response recovery uses session-scoped browser state rather than durable client authority.

### Local operations

- `launch_local_stack.sh` remains the maintained build, migration, seed, readiness, and browser
  front door.
- Native PostgreSQL/MinIO/API/worker/gateway operation remains the default.
- The optional WebWork2/MariaDB profile is private and source-pinned, with separate strict secrets.
- Local PostgreSQL is pinned to major 17 and retained volumes are checked non-destructively before
  startup. The local Compose topology is not a production security or deployment configuration.

## Current verification

### Accepted recorded evidence

- WP-RC3: live Podman 6 upstream build, authenticated PLE API render/grade/cache/outage/recovery,
  required Playwright keyboard and privacy trace, and final independent review.
- WP-ARCH1 acceptance snapshot: permanent source-size gate 582 passed, repository Python suite
  2,451 passed, eleven-stage codebase gate passed, browser suite 72 passed with two deliberate
  opt-in skips, focused server suite 189 passed with three live fixtures intentionally ignored, and
  independent discipline reviews found no unresolved P0/P1.
- WP-RC1, WP-RC2, QTI profile-to-native, course appearance, retention, forced-RLS, and object
  boundaries retain their package-specific accepted evidence in the implementation status and
  workstream documents.

### Current report checks

- Working tree: mixed indexed, unindexed, and untracked implementation/documentation work; this is
  not a release or commit boundary.
- Permanent source-size gate: 824 passed with no maintained-source violation. It ignores an indexed
  path deleted from the working tree while retaining its symlink, invalid-UTF-8, NUL, and 1,000-line
  refusal behavior.
- Feature-enabled learning-data-access passes its check, strict Clippy, and 49 Store conformance
  tests. Server core passes 205 library tests with three intentional live ignores plus its binary
  test under the complete gate.
- Focused presentation reproduction, replay ownership/concealment/deletion, and one-private-RPC
  WeBWorK grading tests pass. This is offline implementation evidence, not the WP-P2/P4 live gate.
- The current all-feature Rust workspace test passes; strict all-feature workspace check and Clippy
  pass; the browser suite passes 76 tests with two deliberate opt-in skips.
- The maintained eleven-stage codebase gate passes, including 189 Node tests, generated-contract
  checks, strict TypeScript, Rust format/Clippy/workspace tests, and doctests. The independent
  Python/documentation policy suite passes 3,270 tests.
- The disposable PostgreSQL baseline passes all nine migrations, fresh/no-op/verify behavior,
  passwordless/enrollment RLS and role boundaries, and the existing partition/QTI/flat/manual/item
  analysis oracles. The disposable project and volume were removed after the run.
- These current-tree checks do not replace WP-RC8's still-missing real-email/optional-passkey,
  multi-replica, and independent acceptance evidence or constitute WP-RC12 release acceptance.

## Current gaps

1. Close WP-RC8 acceptance with a disposable real-email/optional-passkey and multi-replica journey
   plus independent security/HCI review. Keep manager-assisted account merge and educational-record
   transfer outside version 1; do not infer record ownership from email possession.
2. Complete WP-RC4's independent flat JSON v2 contract/security closeout.
3. Implement and accept WP-P1 through WP-P6 before WP-RC5 acceptance.
4. Complete WP-RC5 visual family authoring, integrated all-family persistence/object evidence,
   WeBWorK MATCH, and the exact Chapter 1 genetics and biochemistry content.
5. Complete WP-RC6 QTI export and bounded H5P import claims.
6. Complete WP-RC7 object reconciliation and the combined M2-M5 security/data gate.
7. Implement optional SSO, LTI, secure learner uploads, declarative deployment/recovery, and bot-cost
   controls through WP-RC9 to WP-RC11.
8. Run WP-RC12 working-codebase release acceptance and independent review before any production
   readiness claim.

## Dependency order

```text
WP-RC8 passwordless identity/enrollment acceptance closeout
    |
    v
WP-RC4 independent closeout
    |
    v
WP-P1..WP-P6 secure payload ---> WP-RC5 families/content ---> WP-RC6 QTI/H5P
                 |
                 +-------------> WP-RC7 reconciliation/integration

WP-RC8 accepted identity/enrollment ---> WP-RC9 LTI ---> WP-FU1..WP-FU6 uploads ---> WP-RC10 OpenTofu
                                                                    |
                                                                    v
                                                         WP-RC11 bot controls
                                                                    |
                                                                    v
                                                         WP-RC12 release acceptance
```

WP-P1 may overlap the independent RC4 review, and non-schema object inventory work may begin before
WP-P2. Package acceptance and migration order may not be skipped.

## Production boundary

The repository still needs production runtime PostgreSQL identities, startup role attestation,
embedded-mode CSRF and final gateway headers, production passwordless email/WebAuthn configuration
and real-email/optional-passkey acceptance, optional SSO credentials, LTI registration, secure
learner uploads, managed deployment, encrypted backup/restore and point-in-time recovery, aggregate
observability, replica/worker/load evidence, cost controls, and independent release review. Institutional
FERPA/legal/security sign-off and the human fall-pilot accessibility and teaching walkthrough are
external activation evidence, not unfinished source code substitutes.

## Report maintenance

Create a new dated report when package acceptance, the executive assessment, a release blocker, or
dependency order changes. Keep the Aug. 9 report as historical context. Detailed transcripts remain
in package workstreams and [implementation_status.md](../implementation_status.md); this report
records conclusions and current evidence, not every command log.
