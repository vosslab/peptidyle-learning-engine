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
| Interface design | 85 | 135 | 135 | 1 | 221 |
| Data and history | 30 | 38 | 37 | 0 | 68 |
| Questions | 55 | 84 | 83 | 2 | 141 |
| Courses | 31 | 70 | 64 | 4 | 105 |
| Assessments | 66 | 73 | 69 | 0 | 139 |
| **Total** | **307** | **433** | **421** | **43** | **783** |

The current checklist contains 783 HG bullets: 307 verified, 433 open, and 43 N/A.
Twelve later duplicate open bullets carry an `Owner:` pointer, leaving 421 owning-open records.

This refresh closes only the two C207 deadline-cap rows and the three C523 due/late default rows.
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

This refresh closes the narrow C519--C521 release-date validation rows: authorized automated and
interactive validation, the 24-hour and Course Active-limit boundary, date ordering, correction and
rerun, and hard-gated release. The accepted five actionable date messages are partial evidence only;
the broader missing, invalid, or unreasonable-values row remains open with the valid-range and
Question-validity rows. The accepted fresh PostgreSQL 17 actual-API receipt and actual Properties
component receipt complement the source evidence; the main integrated 7,042-pytest and 329-Node
runs passed, while the separate full server AWS dependency integration gate remains blocked.

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
