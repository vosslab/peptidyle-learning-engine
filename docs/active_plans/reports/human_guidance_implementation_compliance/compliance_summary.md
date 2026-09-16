# Human Guidance implementation compliance summary

## Current heading reconciliation

The current untouched Human Guidance snapshot is SHA256
`6f64d0bb877de5a8be3951dd5514fe0eb2fd5c5f3060b21940230b03a2fde481`.
Current status comes from the [implementation checklist](../../audits/human_guidance_implementation_checklist.md):
890 bullets, 448 verified, 396 open (388 owning), and 46 N/A.
Unchanged contextual evidence, including accepted scoring and bounded runtime receipts, is retained.
The finite timing rows now require a calculated default, explicit override, and accommodation
handling. The 250 delivered-Question bound is verified; exact unchanged same-identity SQL resume
is verified. A fresh two-Question NULL-default release resolves 180 seconds, but Student start
fails `Assessment Attempt requires 1 to 250 Questions`, so default/delivery acceptance remains
open. Ratio/UI accommodations and the effective 24-hour cap are unimplemented. Fresh mixed
expiry-worker SQL evidence retains rendered unanswered and actual transport limitations.
Blueprint lifecycle, forks, Change Proposals, and comparison now belong to Course specifications
(Part 08), not Instructor interface (Part 04). Assessment type appearance belongs to Instructor
interface (Part 04). Student/backend/history subheadings route through their existing parts.
Equivalent rewrites were evaluated against their cited evidence; expanded requirements are open
unless current source or accepted receipts establish their whole scope. Themes/light-dark use,
Library Question/Pool metadata, PLE support and disclosure, Pool statistics, Bloom classification,
Change Proposal persistence/acceptance, expanded comparison, selected daughter incorporation, and
expanded Unrelease behavior retain scoped mismatches or named pending proof.
The topical inventories and earlier receipts below retain historical evidence context; old
counts, source-line numbers, and correction IDs are not a current ordered HG projection. Retired
one-way lifecycle, imposed Assessment-matching algorithm, and Blueprint Question-content-change
claims are not current requirements. All nine part gates, identity diff, and consistency pass.

## Authority and method

[Human Guidance](../../../HUMAN_GUIDANCE.md) is the product authority. This report summarizes the current
statuses in the authoritative
[implementation checklist](../../audits/human_guidance_implementation_checklist.md), not a
replacement for it. Docs-pass compliance reports provide context only. The counts below are recomputed
from the checklist by `devel/human_guidance_checklist.py`. Each `[x]` is required by the checklist to
carry repository evidence, while `N/A` records document meta-guidance, human ownership, or explicitly
future/unlocked items rather than skipped work.

This refresh does not independently reconcile the topical-report inventories or correction-milestone
mappings. The topical reports are non-additive narrative views, not sources for checklist totals or
per-bullet status. Later duplicate open bullets with an `Owner:` pointer are excluded from the
owning-open count. The unfinished implementation-compliance product goal remains tracked by the
checklist, gap map, and active plan.

## Checklist status by section

| Human Guidance section | Verified `[x]` | Open `[ ]` | Owning open | N/A | Total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| How to use this guidance | 0 | 0 | 0 | 7 | 7 |
| Development principles | 14 | 4 | 4 | 23 | 41 |
| Product vocabulary | 2 | 10 | 10 | 0 | 12 |
| Accounts and roles | 25 | 18 | 18 | 8 | 51 |
| Interface design | 135 | 98 | 97 | 1 | 234 |
| Data and history | 33 | 38 | 37 | 0 | 71 |
| Question specifications | 77 | 106 | 104 | 2 | 185 |
| Course specifications | 75 | 63 | 62 | 4 | 142 |
| Assessment specifications | 87 | 59 | 56 | 1 | 147 |
| **Total** | **448** | **396** | **388** | **46** | **890** |

The Assessment part closes four exact current-rescore rows. Independent review accepted
private PostgreSQL 17 production-SQL lifecycle proof at
`/private/tmp/ple-current-rescore-proof/proof.sql`; the artifact
`/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` exited 0. It retained `0.5` credit,
used authorized expected-current `ple_api.save_assessment` to change points from `8` to `13` and
advance the edit number, then observed `6.5 / 13`, unanswered `0 / 13`, and `6.5 / 26` through
three replay, history, Student landing, and Instructor Gradebook reads. The evidence hashes for
grading results, Question submissions, Assessment submissions, and automated grading receipts were
unchanged. The no-Backend-interaction row remains open: SQL `already_submitted` has no work fields
and the server bypasses ready-path Backend work, but actual HTTP request counts and Backend
transport were not observed. C510 is closed: the production Gradebook and Student landing share the highest-score
selector, while latest-Attempt progress remains separate; the stale configurable grade-rule enum,
field, SQL columns, and editor choices were removed. Accepted independent PostgreSQL 17 evidence
observed individual scores `12`, `4`, `16`, then an in-progress `NULL` fourth Attempt, with both
projections reporting `12 / 16`, `12 / 16`, then `16 / 16`. The fixture limit was four Attempts,
so this does not establish unlimited-Attempt eligibility; it also does not establish HTTP,
rendering, or actual backend grading. Artifact:
`/private/tmp/ple-highest-score-proof/artifacts.rQxVs8/proof.log` (exit 0).


The Blueprint authoring and lifecycle checklist now reflects the current narrow evidence. Reusable
Blueprints have no delivery dates; their availability is exactly Private, Public, or Archived; new
Blueprint creation is Private; owner-only Private visibility and Public-only adoption are enforced;
and the accepted lifecycle contract covers the directed transition and Archived read boundary.
Private is not adoptable. The selected Blueprint Assessment editor's Questions and Properties tasks
now have accepted compiled-main, actual-loopback-HTTP evidence for one selected Assessment, shared
drafts, all six Student-feedback controls, ordinary Save, and exact reload. The full Guidance
Properties scope remains open because scoring, attempt, and late-work activity controls were not
exercised. Public
Blueprint search is verified only for the accepted C47 name-search workflow; no Course creation,
login/TLS, full accessibility, or broad chooser acceptance follows.
Accepted bounded history proof covers Public and Archived Instructor Revision and recorded-metadata
facts, an exact older read-only Revision, Private-foreign and Student denials, and a read-only data
fingerprint at `/private/tmp/ple-blueprint-owned-pool-artifacts.sEJZUB`; it does not claim browser
pagination or retry, local-draft preservation, login/TLS, or populated Student Work.

The fixed-Question Assessment reference invariant is verified by accepted authenticated HTTP
evidence: Revision 1 remains pinned after fixture head Revision 2, and ID-only/missing-Revision
payloads are rejected without product changes. Pool-copy, initial-member-pin, independent-fork,
and no-Pool-member invariants remain source-audit pending; this does not claim broad runtime
verification.

Accepted current-pair evidence now verifies Public-to-Private fork creation, fresh local Assessment
and Pool identities with exact Question Revision membership, source provenance, independent fork
changes, and explicit owner-selected Apply. The 18-request lineage receipt at
`/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json`
covers visible sibling and transitive pairs, Private concealment, current heads, shared Question-ID
relationships through renamed/reordered/split content, and unrelated-pair denial without product or
Student Work mutation. The 84-request Apply receipt at
`/private/tmp/ple-blueprint-owned-pool-artifacts.vs0NCo/blueprint-local-id-apply-http-proof.json`
proves target-local existing/new destinations, four zero-write denials, and injected-fault rollback.
Compiled-main evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-browser.json`
completes comparison and selected Apply at 1280px and 390px. Separately accepted compiled-main,
actual-loopback-HTTP evidence at
`/private/tmp/ple-blueprint-owned-pool-artifacts.8PN6aH/comparison-browser.json` expands changed
content and verifies DTO-backed titles, instructions, fixed Question IDs, Revisions, and points.
No populated Student Work, login/TLS, full accessibility, or broader C413 workflow claim follows.

The Archived Blueprint read-only row is verified. Owner Save and rename lock the Blueprint and
reject Archived state before replay, CAS, or no-op handling; the existing lifecycle regression
preserves metadata, Revision, content, events, and receipts across denied writes, while restoration
permits Private/Public writes. Accepted actual HTTP proof recorded five `409` denials with unchanged
Blueprint state, `200` owner/nonowner historical reads, `404` nonowner writes, and `200` restored
writes: `/private/tmp/ple-daughter-revision-notice-artifacts.JhV6aj/archived-blueprint-http-proof.json`.
That read-only receipt does not itself verify explicit-include Archived browsing, forking,
adoption, populated daughters, concurrency, or a broader Course milestone.

Two Archived Blueprint discovery rows are verified. The list Store carries an explicit boolean
through `ple_api.list_blueprint_courses`; default and `false` return only Public plus the caller's
Private Blueprint, while `true` also returns Archived Blueprints to both active vetted Instructors.
Independent review accepted the source/runtime evidence. Accepted actual-server HTTP proof covers
owner/nonowner membership, nonowner Archived detail, Private concealment, Student denials, and
invalid-query `400` results. Accepted compiled-main browser proof covers default-off, Include
Archived, a read-only actual Archived detail, and return to off with eight GETs and zero writes.
Artifacts:
`/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-http-proof.json` and
`/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-browser-proof.json`.
The fixture uses privileged Published-Question seed data and fixture sessions; it does not claim
login, TLS, pagination, concurrency, publisher-caller preservation, forking, adoption, or a
broader Course milestone.

Six Course-update rows are newly verified. An authorized Instructor lazily opens the current-parent
Course summary and receives changed, matching, removed-source, Type-mismatch, and automatically-
added adopted-Assessment rows; direct local Assessments are excluded. At 1280 by 900 and 390 by
844, accepted actual-server and compiled-main proof reviewed/reopened the summary, used the
existing detail Apply with exact source Revision 2 and daughter Edit CAS, and refreshed to the
matching row. Cancel made zero POST requests. Student and unrelated-Instructor reads returned
`404 no-store`; a private parent was concealed from another Instructor in the privileged-
availability fixture; Archived review remained available and new adoption was denied. Artifact:
`/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`. The earlier one-Assessment proof
separately covers exact pins, stale/no-op/invalid-Released cases, dates, status, origin, and one
populated Assessment Attempt hash: `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`.
Neither receipt claims direct Assessments or all Student Work. No persisted offer, receipt,
comparison baseline, or new update table is introduced, and no whole-Course lifecycle milestone is
closed. The fresh full
database-baseline gate passed as `database baseline E2E: PASS` in
`/private/tmp/ple-course-summary-baseline.log`.

C15's higher-security Sysadmin row, three automatic-new Blueprint Assessment rows, two
existing-Assessment non-silent-change negative invariants, and two older-Blueprint-Revision
indication rows are newly verified. The revision indication uses the existing authorized Course
load and Course page: an accepted independent actual-server/exact-main browser proof covers empty,
current, newer, and synthetic Private-origin states, with nonenumerating Student/unrelated-Instructor
`404 no-store` denials and no extra Blueprint fetch or write. Its visually inspected newer capture
shows adopted and current Revision values; the proof preserved the original adoption pin, Assessment,
and entries, while Work tables were empty. Artifact:
`/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`. It supplies no update offer, review,
approval, or apply workflow. Accepted independent
SQL and actual-server HTTP receipts require genuine private TOTP
before Sysadmin session creation and deny missing/wrong binding, bad codes, replay, expiry,
counter reuse, and a fresh unused valid counter after five failed attempts. Ordinary Student and
Instructor sessions and limited grants remain intact. Artifacts:
`/private/tmp/ple-sysadmin-session-boundary-artifacts.kSMr1H` and
`/private/tmp/ple-sysadmin-session-boundary-http-artifacts.zexsoO`. Transport was loopback HTTP,
not deployed TLS. Connected Blueprint Save proof preserves exact pins/settings, fresh daughter
Pool IDs, Unreleased state and unset dates across two daughters including an inactive Course,
existing actual Student Work, and the original adoption pin; an unrelated empty Course remained
unchanged, and replay/no-op/stale saves do not duplicate the append. Changing a retained source
Assessment title leaves existing daughter content, entries, and actual Student Work unchanged.
The extended existing permanent lifecycle test passed and supplemental temporary bad-payload rollback was
accepted: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. Existing connected adoption
lifecycle regression passed 1 test with 0 ignored:
`/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. Existing-Assessment update offers,
C410, full Course/Assessment milestones, and offline aggregate acceptance are not claimed.

The final append receipt runs the extended existing permanent
`revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` test, which invokes
the `append.rs` helper `assert_new_assessment_save_preserves_daughter_work`.
No separate seed-sharing test was retained. Fresh Sysadmin SQL and the unchanged permanent security
catalog passed as `ple_migrator` in `/private/tmp/ple-sysadmin-session-boundary-artifacts.kSMr1H`.
Broad baseline/offline aggregate acceptance remains pending, not passed.

Four Assessment-content rows are newly verified. Accepted actual-server/private bundled-main
browser proof saved/reloaded mixed Fixed Question/Pool order and exact pins, moved the Pool across
a Fixed Question, removed both kinds from current content while retaining private retired IDs and
the old Pool Revision, and reimported distinct fork identities without changing source Pool JSON.
Student direct real HTTP returned 404 without current-state change; browser error arrays were empty.
Artifact: `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`. Corrected SQL permits atomic
current-position swaps and retired-position reuse and excludes retired entries from workspace reads.
The existing connected adoption regression passed 1 test with 0 ignored under that SQL; two
supplemental Type projections retained unchanged pins:
`/private/tmp/ple-shared-assessment-adoption-artifacts.UmP416`. The exact **Randomize question order**
label uses the earlier accepted part 04 actual-main/HTTP checkbox receipt, not this mixed-entry
proof. No Student Work history, authentication/TLS, full Live Demo, WeBWorK delivery, C505 filename
rename, or whole-milestone completion is claimed.

Ten bounded Assessment Type purpose/capability rows are newly verified from the real Type
presentation registry, both Instructor creation entry points, and current points/scoring/Properties
capabilities. Neighboring accepted Bonus `8 / 0` and Quiz/Exam one-Attempt receipts remain distinct
runtime evidence. The broad appropriate-defaults row and cross-backend Practice immediate-answer
row remain open; this docs-only receipt does not claim complete settings persistence/enforcement,
automatic collaboration policy, learning-age inference, exam calendars, or Student delivery.

The shared underlying Assessment-model row is also verified. Canonical teaching types and
the ordinary adoption materialization path connect reusable Blueprint content to current Course
Instance Assessments; their distinct storage/lifecycle projections are intentional. Accepted fresh
PostgreSQL 17 connected adoption proof passed 1 test with 0 ignored, preserving mixed ordered
Pool/Fixed entries, nondefault teaching rules, exact Revision pins, independent daughter Pool IDs,
and unset dates. Artifact: `/private/tmp/ple-shared-assessment-adoption-artifacts.IkYuXY`.
Every Type's Student delivery and completion remain outside this architecture receipt.

HG608's distinct algorithmic-Question Pool purpose is verified by accepted fresh actual-server
and private bundled-main HTTP-proxy browser proof: explicit Instructor selection/attestation
created a two-Question reusable Pool and a distinct Assessment-owned fork selecting one Question.
Real WeBWorK response save/resume preserved exact provenance and reproduction facts; whole-Attempt
submit and fresh new-Attempt selection/issued IDs passed. Artifact:
`/private/tmp/ple-algorithmic-pool-artifacts.K2Kk6Z`. This closes only HG608, not full Live Demo,
authentication/TLS, all-backend acceptance, or Correct answer disclosure (configured Never).

C351 closes the manual Draft-deletion row. Source evidence follows owner-scoped SQL with locked
Edit Number CAS through the PostgreSQL Store, authenticated server route, and explicit browser
confirmation. Accepted isolated PostgreSQL 17/MinIO actual-server and focused browser proof
cancelled and confirmed a delete, reloaded the list, denied collaborator, unrelated Instructor,
Student, Sysadmin, and anonymous current-ETag deletes as 404 without changing source/Edit Number,
and returned 428, 400, and 412 for missing, malformed, and stale preconditions. A published-origin
Draft deletion retained parsed Published Question lineage and Revision JSON; repeated DELETE and PUT
returned 404. Artifact: `/private/tmp/ple-draft-delete-artifacts.km9ybM`. This bounded receipt does
not claim S3 erasure, automatic cleanup, authentication acceptance, full Live Demo browser
acceptance, or healthy backend behavior with the renderer disabled.

C831 closes the bounded native-reproduction row. The source model tags native PLE JSON as
`Static`, while renderer-backed Questions retain the paired seeded reproduction facts. Accepted
isolated PostgreSQL 17 evidence issued a shuffled-position-2 native Question with null seed and
hash, retained a numeric seed and 64-character hash only for the real WeBWorK Question, omitted
both fields from all public start/read/save/resume/restored payloads, and restored an identical
issued vector plus saved native response. The database rejected an invalid native seed through
`validate_issued_question_reproduction`. This one-time proof does not claim JavaScript support,
backend parity, or broader randomness behavior. Artifact:
`/private/tmp/ple-native-seed-artifacts.KfY7Op`.

C910 closes three bounded feedback rows. Author-managed General feedback remains immutable Question
Revision metadata; accepted actual Student HTTP and exact-main browser proof rendered it after
submission with all six disclosure timings `Never`, while response, score, correctness, answer, and
explanation remained absent and nonowners received 404. PLE does not reconstruct transient WeBWorK
feedback after the renderer stops. Source and focused domain tests establish that supplied native
Question Feedback is independent of answer disclosure; the runtime WeBWorK fixture supplied no
transient backend feedback. The broader Student workflow and opaque backend-answer gaps remain open.

Bulk Published Question shared metadata now has a bounded C368 behavior closure. The closed
`tags`/`subject`/`topic` DTO, generated 1,000-item bound, canonical-ID validation before Store
access, no-store current read, and one-transaction database command passed fresh PostgreSQL 17
SQL/API proof and independent rerun. Source review establishes once-only native `PLE authoring`/
`Pilot` tags and an empty WebWork start; explicit initial-tag SQL/API proof rejects null elements
before database/publication/object side effects and preserves an empty clear in a successor Revision.
Accepted isolated actual-server HTTP and private exact-main browser proof selected Published
Questions, replaced tags and subject, cleared subject, observed search projection, and returned
all-or-none 412 after an authorized nonowner Instructor advanced one selected Question. The browser
refreshed current metadata without automatically writing again; source, Revision, and availability
were unchanged, and anonymous/Student calls were denied. C366's thousands-Question practical
workflow and the 13k cleanup claim remain open; this three-Question proof is not screenshot-corpus
acceptance. Separately, C58 now has accepted private PostgreSQL 17 and
actual-server HTTP evidence for ordinary words, quoted phrases, minus exclusion, all five PLE
fields, active-vetted-Instructor access, anonymous and Student concealment, and `no-store`; the
bounded 69-Question fixture does not establish expert very-large-library usability or performance.
C60 now has accepted routed-component, actual-server, and private exact-main full-app evidence for
grouped Browse, full-snapshot counts, exact Browse-to-Search transfer, distinct routes, connected
exploration, and shared dense presentation at 1280 by 800. The private asset transport does not
establish deployment-gateway or WASM-runtime behavior. The 13k cleanup remains open as well. C59's
native Search tips disclosure makes the grammar discoverable without obscuring normal controls.
C57 closes the three Search-landing and return-state bullets with accepted one-time compiled-browser
evidence: idle Search did not fetch or show filters/results; entering a query showed results; and
visible return and browser Back restored query, filter, 80 rows, and virtual-list position while a
changed session returned to the empty landing. The temporary harness and screenshots were removed
after acceptance.
C61's independently accepted actual Ribbon/page proof closes the required Assessments labels
and reusable Template-design rows. Follow-up private actual-server and exact-main browser evidence
closes three Due Soon rows with authorized cross-Course results and visible state/Course/Due
emphasis. The first owned Course list retained readable Account-zone Due values in three rows at
1280px and 720px with a different browser zone; a second owned Course showed three mixed
Released/Unreleased rows and distinct Due values matched to its actual 200/`no-store` response.
This closes the bounded Assessment-list scan row. The broader spreadsheet-like collection-density
judgment, empty/error states, release workflow, WASM runtime, deployment gateway, and connected
Template delivery were not claimed.
C62's independently accepted private actual-HTTP and exact-main browser proof closes all five
Assessment-editor-shell rows: the two named editors, Question selection/add/remove/order behavior,
their distinct tasks, grouped responsive Properties, and fixed-Question point-value editing.
Question order, one instructions edit, and point values `2.5` and `1` survived actual-server
reloads. Cancel and Stay/Discard issued no unintended save, and a real stale write retained the
point draft until explicit reload/discard synchronized the latest full Assessment. This does not
claim score recalculation, Released-Assessment editing, deployment-gateway, or WASM-runtime
behavior. Broader spreadsheet-like collection-density judgment stays open under C44.
C64 and C65 close both Assessment-randomization rows. Accepted authenticated Student HTTP evidence
persisted authored and shuffled rules, issued a complete exact fixed-and-Pool vector, and retained
immutable shuffled order on resume after a current-rule edit. Accepted exact-main browser evidence
saved and reloaded the visible checkbox through actual HTTP. The Question authoring/codec/adapter
and presentation-builder source chain owns native choice order, while the closed Assessment rules
have no choice override. This establishes the ownership boundary without claiming a runtime matrix
of every native choice permutation.
Accepted actual-server Pool evidence closes the fork-independence and new-Attempt-selection rows:
an Assessment-owned fork advanced to Revision 2 with reversed exact member pins while its reusable
source remained Revision 1 in original order; Attempt 1 resumed its retained pin and nonce, while
submitted Attempt 2 received a distinct selection ID and nonce. Accepted actual-main Instructor
evidence also created a reusable Pool, imported it, reduced its count from 2 to 1, attested and
reordered its two exact Revision 1 members, and reloaded the Assessment-owned Revision 2 with its
source unchanged. The same accepted receipts establish an ordered Published-Question Pool, an
Instructor interchangeability attestation rather than automatic pedagogy evaluation, and canonical
public `AAAA-ZBBB` IDs for Questions and Pools. They do not establish ID-collision handling,
backend-independence, or broader Pool discovery.
Accepted actual-server sharing evidence also let a second vetted Instructor list and read a root and
child published Pool with exact public pins and no Course facts; it denied nonmember mutation without
change and Student/anonymous Pool reads with no-store 404. This closes only vetted-Instructor Pool
availability.
Accepted actual-main Student evidence then denied three Question Library routes without Library API
calls, while Coursework navigation reached a Released Assessment and the earlier native Attempt proof
delivered its Question content there. This closes the Student Library-access boundary only, without
claiming screenshot or interface-aesthetics acceptance.
Accepted private actual-HTTP and exact-main evidence closes C63's visible-order and direct
Search/Browse rows: numbered order, reorder, save, reload, both direct paths, browser Back, and
Stay/Discard unsaved-state handling were visibly accepted. Exact-revision metadata navigation is
wired, and the authorized private source and checksum now reach the opaque WeBWorK adapter and
hardened iframe. Private PostgreSQL 17/MinIO plus unchanged-renderer HTTP evidence returned 200
with hardened headers and concealed missing Revision, Student, and anonymous requests; exact-main
browser evidence rendered the prompt and five choices, with all five Student Work counts remaining
zero before and after in this isolated preview path. C63 stays open because the renderer JavaScript
dereferences `window.frameElement.id` when the hardened sandbox has no same-origin frame element,
before focus, popover, and parent telemetry. Do not loosen the sandbox or rewrite sibling renderer
HTML; successful hardened-embed behavior and a state-preserving return remain unverified.

This refresh closes the two C207 deadline-cap rows, three C523 due/late default rows, the narrow
C514-C516 Template rows, and the C525 completion and Attempt-limit rows.
Accepted independent PostgreSQL 17 actual-API receipts establish Course-first atomic synchronization
of the current maximum Due date, the immutable six-month Active cap and no-Due fallback, frozen
archived/deleted retention anchors, default Due-based start/save/commit rejection, accommodated
deadlines, valid `accept` and `mark_late` overrides, and expiry finalization that preserves accepted
pre-Due work. The ignored one-time proofs were removed after acceptance. Retention notification,
archive/delete processing, broad release validation, visible unanswered UI, cross-session resume,
and backend-wide behavior remain outside these closures.

This refresh closes the Bonus zero-points-possible/direct-earned row and both occurrences of the
highest-submitted-Attempt score rule. Gradebook and Student landing share the selected score while
latest-Attempt progress remains separate. The fresh C510 lifecycle receipt observed scores `12`,
`4`, `16`, and an in-progress `NULL` fourth Attempt, with both projections retaining `16 / 16`.
The configurable grade-rule model/UI/SQL boundary was removed. The focused PostgreSQL LDA test
compilation used `--features postgres,test-support`; no default-feature or full Rust gate is
claimed. This privileged fixture and simulated backend evidence does not prove HTTP, rendering,
actual backend grading, unlimited-Attempt eligibility, Course totals, or grade calculation.

This refresh closes the narrow C519--C521 release-date and valid-range rows: authorized automated
and interactive validation, the 24-hour and Course Active-limit boundary, date ordering, correction
and rerun, hard-gated release, public-save point bounds, positive-or-null whole-Assessment
Attempt/time limits, and the release-required time limit. The accepted fresh PostgreSQL 17 actual-API
receipt atomically rejected out-of-range and excess-precision point values, then saved and released
the exact maximum. The broader missing, invalid, or unreasonable-values row and the Question-validity
row remain open. The actual Properties component receipt complements source evidence; the main
integrated 7,042-pytest and 329-Node runs passed. Current follow-up evidence also built the
`project-tools` binary, passed its nine focused curriculum-content tests, and passed all 7,155
pytest tests; these offline checks do not establish connected integration behavior.

C838/C839 now close the seven bounded algorithmic-source rows. Fresh PostgreSQL 17/MinIO evidence
published all 42 canonical PGML sources (41 official biologyproblems-website sources plus HLA) as
ordinary available WeBWorK Question lineages at Revision 1 across nine topics, with 42 direct Fixed
entries and zero Pools. Exact replay made no mutation; a same-short-name conflict made no mutation.
Pilot's `validated_question_format` source boundary and four focused tests enforce explicit PG/PGML
format with its matching extension; connected binding proof independently preserved source SHA/size/
path, immutable replay, and stale refusal after an intervening metadata edit. Artifacts:
`/private/tmp/ple-fresh-genetics-artifacts.5ERV83` and
`/private/tmp/ple-pilot-format-binding-artifacts.CKzka1`. Chargaff remains unaccepted; the 76 banks
and 13,434 old generated rows remain unresolved non-published inventory. C840--C841 remain
conditional retained-database work; this receipt does not claim migration, static-row equivalence,
historical rewrite, or Pool retirement.

## Report routes

This refresh closes C512's Bonus Assignment and Quiz icon rows. The genuine bundled Free Solid
sprite uses `star` for Bonus Assignment and `circle-question` for Quiz; all five Assessment Types
now have a fixed bundled glyph and a visible label.

- [Product conflicts](product_conflicts.md) groups cross-cutting incompatibilities and points to their owning inventories.
- [Unresolved or ambiguous items](unresolved_or_ambiguous_items.md) contains only HG-unlocked product design.
- [Terminology and model changes](terminology_and_model_changes.md)
- [Authorization and FERPA changes](authorization_and_ferpa_changes.md)
- [Question and assessment changes](question_and_assessment_changes.md)
- [UI and workflow changes](ui_and_workflow_changes.md)
- [Architecture and implementation changes](architecture_and_implementation_changes.md)
- [Gap map](../../audits/human_guidance_gap_map.md) (implementation planning companion).

## Generated-evidence follow-up

Re-run the checklist consistency check after Human Guidance or checklist status changes. This summary
does not establish that the topical reports cover every owning-open record or that their mappings are
current. Runtime/browser evidence remains a separate acceptance layer and is only used where the
checklist records a valid current receipt.
