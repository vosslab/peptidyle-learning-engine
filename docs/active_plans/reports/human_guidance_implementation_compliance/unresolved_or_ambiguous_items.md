# Unresolved or ambiguous items

## Current heading reconciliation

Current status and retained evidence are recorded in [compliance_summary.md](compliance_summary.md)
and the [implementation checklist](../../audits/human_guidance_implementation_checklist.md).
Blueprint lifecycle/forks/comparison now belong to Course specifications (Part 08); Assessment
type appearance belongs to Instructor interface (Part 04). Earlier topical inventories are
historical context, not current wording, counts, source-line pointers, or ownership.

## Result

The Ribbon design below is explicitly unlocked by Human Guidance. It is a positive N/A audit result
for implementation planning, not an unreviewed `[ ]` mismatch. Current finite timing requirements
remain open where connected or browser acceptance is missing.

| Unlocked design | What Human Guidance settles | What remains open | Source |
| --- | --- | --- | --- |
| Student and Sysadmin Ribbon detail | Student navigation centers Courses and Coursework; Sysadmin navigation centers Accounts, Instructors, Courses, and system settings. | The complete Ribbon task layout for either role. | `docs/HUMAN_GUIDANCE.md` -- Student and Sysadmin interface |

## Plausible later layouts, not requirements

- The Student Ribbon could present `Courses` and `Coursework` as two permanently visible choices, with
  course selection and Coursework details in the page body.
- The Sysadmin Ribbon could present `Accounts`, `Instructors`, `Courses`, and `System settings` as
  permanently visible choices, with a contextual subnavigation row for the selected area.

Human Guidance leaves both behaviors open: it identifies the areas each role's navigation centers, but
does not lock their order, labels, nesting, or whether a contextual subnavigation row exists. Neither
example authorizes implementation before a product decision.

Human Guidance now requires finite duration: a rounded 1.5-minutes-per-Question default, explicit
Instructor override up to 12 hours, and individual 1.5X/2X accommodations after the base duration
with an effective 24-hour cap. Do not infer acceptance from static implementation evidence. A fresh
two-Question NULL-default release resolves 180 seconds but the subsequent Student start fails
`Assessment Attempt requires 1 to 250 Questions`; default/delivery acceptance remains open.
Existing private absolute-second SQL is not ratio/UI or 24-hour-cap acceptance.

Other generated evidence refreshes, implementation mismatches, and future work are not product
ambiguity. Working-speed wording is audited pedagogical purpose (N/A), not a learning-effect gate;
the underlying timing requirements remain binding. Exact unchanged SQL resume is verified, while
actual browser/session resume and rendered expiry-unanswered/Backend-transport proof remain open.

Assessment Pool selection count is not an unresolved product decision. The recorded engineering
choice uses the existing positive `selection_count` on the Assessment-owned Pool fork; the reusable
Pool Revision owns its exact members. C905-C909 still own the missing exact fork Pool ID/Revision
provenance, count-bound validation, delivery chain, and proof.

Correction-milestone mappings are pending Milestone G and will be added after all Milestone G runs
are complete.
