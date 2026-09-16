# Human Guidance implementation compliance summary

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
| --- | ---: | ---: | ---: | ---: | ---: |
| How to use this guidance | 0 | 0 | 0 | 5 | 5 |
| Development principles | 14 | 4 | 4 | 23 | 41 |
| Product vocabulary | 2 | 10 | 10 | 0 | 12 |
| Accounts and roles | 24 | 19 | 19 | 8 | 51 |
| Interface design | 118 | 103 | 103 | 1 | 222 |
| Data and history | 30 | 38 | 37 | 0 | 68 |
| Questions | 58 | 81 | 80 | 2 | 141 |
| Courses | 31 | 70 | 64 | 4 | 105 |
| Assessments | 81 | 58 | 55 | 0 | 139 |
| **Total** | **358** | **383** | **372** | **43** | **784** |

The current checklist contains 784 HG bullets: 358 verified, 383 open, and 43 N/A.
Eleven later duplicate open bullets carry an `Owner:` pointer, leaving 372 owning-open records.

C910 closes three bounded feedback rows. Author-managed General feedback remains immutable Question
Revision metadata; accepted actual Student HTTP and exact-main browser proof rendered it after
submission with all six disclosure timings `Never`, while response, score, correctness, answer, and
explanation remained absent and nonowners received 404. PLE does not reconstruct transient WeBWorK
feedback after the renderer stops. Source and focused domain tests establish that supplied native
Question Feedback is independent of answer disclosure; the runtime WeBWorK fixture supplied no
transient backend feedback. The broader Student workflow and opaque backend-answer gaps remain open.

Bulk Published Question shared metadata now has accepted bounded source and PostgreSQL evidence,
not a closure. The closed `tags`/`subject`/`topic` DTO, generated 1,000-item bound, canonical-ID
validation before Store access, no-store current read, and one-transaction database command passed
fresh PostgreSQL 17 SQL/API proof and independent rerun. Source review establishes once-only native
`PLE authoring`/`Pilot` tags and an empty WebWork start; explicit initial-tag SQL/API proof rejects
null elements before database/publication/object side effects and preserves an empty clear in a
successor Revision. C366/C368 remain open after accepted temporary compiled Chromium component and
strict-client evidence for the selected replace/clear and stale/ambiguous-refresh workflow; it used
mock/injected transport, not a connected server. Connected HTTP and discovery/search projection
execution remain unverified. Separately, C58 now has accepted private PostgreSQL 17 and
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
C61's independently accepted actual Ribbon/page proof closes only the required Assessments labels
and reusable Template-design rows. Follow-up private actual-server and exact-main browser evidence
closes three Due Soon rows with authorized cross-Course results and visible state/Course/Due
emphasis. The Course list now renders readable Account-zone local Due values in a three-row fixture
at 1280px and 720px, including when the browser uses a different zone. Representative cross-list
scanability, empty/error states, release workflow, WASM runtime, deployment gateway, and connected
Template delivery were not claimed.
C62's independently accepted private actual-HTTP and exact-main browser proof closes all five
Assessment-editor-shell rows: the two named editors, Question selection/add/remove/order behavior,
their distinct tasks, grouped responsive Properties, and fixed-Question point-value editing.
Question order, one instructions edit, and point values `2.5` and `1` survived actual-server
reloads. Cancel and Stay/Discard issued no unintended save, and a real stale write retained the
point draft until explicit reload/discard synchronized the latest full Assessment. This does not
claim score recalculation, Released-Assessment editing, deployment-gateway, or WASM-runtime
behavior. Broad Assessment-list scanability stays open pending representative proof across multiple
Course lists.
C64 and C65 close both Assessment-randomization rows. Accepted authenticated Student HTTP evidence
persisted authored and shuffled rules, issued a complete exact fixed-and-Pool vector, and retained
immutable shuffled order on resume after a current-rule edit. Accepted exact-main browser evidence
saved and reloaded the visible checkbox through actual HTTP. The Question authoring/codec/adapter
and presentation-builder source chain owns native choice order, while the closed Assessment rules
have no choice override. This establishes the ownership boundary without claiming a runtime matrix
of every native choice permutation.
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
highest-submitted-Attempt score rule. Gradebook and Student API evidence independently establish the
selected score while latest-Attempt progress remains separate. Broader Practice extra-credit
authoring and Course-level grade calculation remain outside those closures. The focused PostgreSQL
LDA library check passed; the separate full `server_core` compile remains unverified because of the
existing AWS Smithy dependency incompatibility.

This refresh closes the narrow C519--C521 release-date and valid-range rows: authorized automated
and interactive validation, the 24-hour and Course Active-limit boundary, date ordering, correction
and rerun, hard-gated release, public-save point bounds, positive-or-null whole-Assessment
Attempt/time limits, and the release-required time limit. The accepted fresh PostgreSQL 17 actual-API
receipt atomically rejected out-of-range and excess-precision point values, then saved and released
the exact maximum. The broader missing, invalid, or unreasonable-values row and the Question-validity
row remain open. The actual Properties component receipt complements source evidence; the main
integrated 7,042-pytest and 329-Node runs passed, while the separate full server AWS dependency
integration gate remains blocked.

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
