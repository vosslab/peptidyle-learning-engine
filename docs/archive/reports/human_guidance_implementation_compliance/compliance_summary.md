# Implementation compliance summary

Implementation audit of the running PLE system against Human Guidance on 2026-10-02.
Human Guidance blob: `d0614eba06400c7ae00dfcdb73ca0a9e1ed03efc`.

## Authority

[HUMAN_GUIDANCE.md](../../../HUMAN_GUIDANCE.md) is the product authority. See the
[checklist](../../../active_plans/audits/human_guidance_implementation_checklist.md).
The gap map is context in
[human_guidance_gap_map.md](../../../active_plans/audits/human_guidance_gap_map.md).
The documentation-pass reports under
[human_guidance_compliance](../human_guidance_compliance/COMPLIANCE_SUMMARY.md)
record an earlier docs reconciliation. They are not this audit.

`devel/human_guidance_checklist.py --diff` and `--consistency` both exit 0 against the
working Human Guidance file. Deferred product behavior is excluded from the checklist.
QTI, BBQ upload, AI Bloom assignment, and automated daemon backends were not implemented.

## Counts

| Status | Count |
| --- | --- |
| Verified | 1055 |
| Open | 28 |
| Not applicable | 54 |
| Bullets | 1137 |

| Section | Verified | Open | Not applicable |
| --- | --- | --- | --- |
| Development principles | 20 | 1 | 30 |
| Accounts and roles | 44 | 0 | 8 |
| Interface design | 345 | 24 | 4 |
| Data and history | 159 | 2 | 0 |
| Question specifications | 186 | 0 | 7 |
| Course specifications | 154 | 0 | 4 |
| Assessment specifications | 147 | 1 | 1 |

A verified bullet has locating source or test evidence in the checklist. An open bullet
has `Reason: product decision still unclear` plus a question and both readings, or
`Reason: HG: no locked-in design` for the unlocked Sysadmin Ribbon layout.

## Where the open rows are explained

Every open bullet is recorded once in
[unresolved_or_ambiguous_items.md](unresolved_or_ambiguous_items.md).
Area patterns are in the sibling reports:

- [terminology_and_model_changes.md](terminology_and_model_changes.md)
- [authorization_and_ferpa_changes.md](authorization_and_ferpa_changes.md)
- [question_and_assessment_changes.md](question_and_assessment_changes.md)
- [ui_and_workflow_changes.md](ui_and_workflow_changes.md)
- [architecture_and_implementation_changes.md](architecture_and_implementation_changes.md)
- [product_conflicts.md](product_conflicts.md)

## Generated evidence

The canonical Playwright suite passed for the signed-in top bar and Profile menu.
Breadcrumb ancestor behavior is covered by `tests/test_breadcrumb_ribbon_ancestors.mjs`.
The screenshot corpus and Graphify output were not regenerated in this pass.
