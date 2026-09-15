# Human Guidance implementation compliance summary

## Authority and method

[Human Guidance](../../../HUMAN_GUIDANCE.md) is the product authority. This report is a fresh
code-and-repository audit generated from the authoritative
[implementation checklist](../../audits/human_guidance_implementation_checklist.md), not a
replacement for it. Docs-pass compliance reports provide context only. Each `[x]` has repository
evidence; each owning `[ ]` is inventoried once in a topical report; and `N/A` records are
audited meta-guidance, human ownership, or explicitly future/unlocked items rather than skipped work.

The inventory count excludes later duplicate records with `Owner:` pointers. A disposable generator
parsed the checklist headings, statuses, and duplicate pointers, assigned each owning `[ ]` to one
topical report, then checked that the union matches the owning-open set exactly.

## Checklist status by section

| Human Guidance section | Verified `[x]` | Open `[ ]` | Owning open | N/A | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| How to use this guidance | 0 | 0 | 0 | 5 | 5 |
| Development principles | 12 | 7 | 7 | 20 | 39 |
| Product vocabulary | 2 | 10 | 10 | 0 | 12 |
| Accounts and roles | 23 | 20 | 20 | 8 | 51 |
| Interface design | 70 | 149 | 149 | 0 | 219 |
| Data and history | 25 | 37 | 35 | 0 | 62 |
| Questions | 54 | 78 | 77 | 0 | 132 |
| Courses | 31 | 70 | 64 | 4 | 105 |
| Assessments | 27 | 109 | 104 | 0 | 136 |
| **Total** | **244** | **480** | **466** | **37** | **761** |

The generated checklist contains 761 HG bullets: 244 verified, 480 open, and 37 N/A.
Fourteen later duplicate open bullets carry an `Owner:` pointer, leaving 466 owning-open records
for the one-to-one implementation inventory.

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

The checklist generator and this report inventory are durable audit artifacts. Re-run their consistency
and inventory checks after Human Guidance or checklist status changes; generated reports must not become
hand-maintained claims. Runtime/browser evidence remains a separate acceptance layer and is only used
where the checklist records a valid current receipt.

Correction-milestone mappings are pending Milestone G and will be added only after every Milestone G
run is complete. These reports intentionally do not infer those mappings early.
