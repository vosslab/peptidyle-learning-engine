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
| Interface design | 88 | 132 | 132 | 1 | 221 |
| Data and history | 30 | 38 | 37 | 0 | 68 |
| Questions | 55 | 84 | 83 | 2 | 141 |
| Courses | 31 | 70 | 64 | 4 | 105 |
| Assessments | 81 | 58 | 55 | 0 | 139 |
| **Total** | **325** | **415** | **404** | **43** | **783** |

The current checklist contains 783 HG bullets: 325 verified, 415 open, and 43 N/A.
Eleven later duplicate open bullets carry an `Owner:` pointer, leaving 404 owning-open records.

Bulk Published Question shared metadata now has accepted bounded source and PostgreSQL evidence,
not a closure. The closed `tags`/`subject`/`topic` DTO, generated 1,000-item bound, canonical-ID
validation before Store access, no-store current read, and one-transaction database command passed
fresh PostgreSQL 17 SQL/API proof and independent rerun. Source review establishes once-only native
`PLE authoring`/`Pilot` tags and an empty WebWork start; explicit initial-tag SQL/API proof rejects
null elements before database/publication/object side effects and preserves an empty clear in a
successor Revision. C366/C368 remain open after accepted temporary compiled Chromium component and
strict-client evidence for the selected replace/clear and stale/ambiguous-refresh workflow; it used
mock/injected transport, not a connected server. Connected HTTP and discovery/search projection
execution remain blocked by the known AWS Smithy server-build incompatibility. C58 now has accepted
actual-source parser evidence for ordinary words, quotes, minus, PLE fields, literal unknown tokens,
empty fields matching nothing, and exact-ID-plus-filter behavior, but its connected HTTP/search projection remains open;
the 13k cleanup remains open as well. C59's native Search tips disclosure now makes the grammar
discoverable without obscuring the normal controls; expert large-library narrowing remains open.
C61's independently accepted actual Ribbon/page proof closes only the required Assessments labels
and reusable Template-design rows. Due Soon state rows and connected Template delivery remain open.

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
