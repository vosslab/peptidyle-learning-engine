# Unresolved or ambiguous items

## Result

Only the following Human Guidance design is explicitly unlocked. It is a positive N/A audit result
for implementation planning, not an unreviewed `[ ]` mismatch. All other open checklist records are
implemented-product gaps and are owned by the topical reports.

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

No genuine product ambiguity was found beyond that explicitly unlocked design. Generated evidence
refreshes, implementation mismatches, and future work are not product ambiguity.

Correction-milestone mappings are pending Milestone G and will be added after all Milestone G runs
are complete.
