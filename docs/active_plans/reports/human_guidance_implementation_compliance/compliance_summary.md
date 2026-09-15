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
mappings. Later duplicate open bullets with an `Owner:` pointer are excluded from the owning-open count.
The unfinished implementation-compliance product goal remains tracked by the checklist, gap map, and
active plan.

## Checklist status by section

| Human Guidance section | Verified `[x]` | Open `[ ]` | Owning open | N/A | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| How to use this guidance | 0 | 0 | 0 | 5 | 5 |
| Development principles | 14 | 2 | 2 | 23 | 39 |
| Product vocabulary | 2 | 10 | 10 | 0 | 12 |
| Accounts and roles | 24 | 19 | 19 | 8 | 51 |
| Interface design | 78 | 140 | 140 | 1 | 219 |
| Data and history | 28 | 40 | 39 | 0 | 68 |
| Questions | 55 | 84 | 83 | 2 | 141 |
| Courses | 31 | 70 | 64 | 4 | 105 |
| Assessments | 27 | 109 | 104 | 0 | 136 |
| **Total** | **259** | **474** | **461** | **43** | **776** |

The current checklist contains 776 HG bullets: 259 verified, 474 open, and 43 N/A.
Thirteen later duplicate open bullets carry an `Owner:` pointer, leaving 461 owning-open records.

## Report routes

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
