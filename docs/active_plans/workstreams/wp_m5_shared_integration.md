# M5 shared walkthrough integration

## Status

**ACCEPTED.** The retained-volume M5 run now passes J1, J2, J3, J4, J5, and
J8 through visible keyboard paths. See the resolved
[blocker audit](../audits/m5_retained_pagination_blocker_review.md), the
[pagination product review](../audits/pagination_product_final_review.md),
[course review](../audits/pagination_course_assignments_review.md),
[gradebook review](../audits/pagination_gradebook_review.md), and
[walkthrough integration review](../audits/pagination_walkthrough_integration_review.md).

**Charter supersession:** this remains accepted evidence for the learner and
gradebook slice. The repository owner's corrected walkthrough charter also
requires visible instructor course creation, active local-student roster
addition, and corpus-backed assignment creation. M5 used a launcher-seeded
course and API-arranged assignments, so it is not final acceptance of that
broader instructor-to-student goal. Email and canonical onboarding are outside
the corrected walkthrough rather than M5 blockers.

## Resolved retained-volume limit

The prior first-page blocker is retained as history. `CursorPageSession` now
keeps opaque cursors, deduplicates appended rows, retries the exact failed
cursor, and fails closed on repeated or zero-new nonterminal pages. Course and
gradebook surfaces expose native same-window `target="_self"` fragment links
to named `tabindex="-1"` pagination sections; one native Tab reaches the real
load, retry, or reload control. Success transfers focus to the first appended
public control. A keyed course boundary prevents late A-to-B route callbacks
from overwriting the current course.

## Accepted retained-volume evidence

- The manager and independent reviewer each ran
  `bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build` against
  retained volumes. Both reports are `PASS`, seed 42, stage `complete`, and
  contain ordered PASS rows for J1, J2, J3, J4, J5, and J8 with empty
  diagnostics.
- The reviewed report directory/file modes are 0700/0600. They retain only
  redacted public evidence; no trace, screenshot, video, private temporary
  root, or selected Podman container remained after runner-owned no-volume
  cleanup. No reset, fresh project, direct route, stale row, or API/browser
  shortcut was used.
- MemoryStore conformance covers the generic 51-record cursor boundary only;
  it is not live PostgreSQL evidence. All-family, secure-payload, and WP-B1
  work remain separate package evidence and do not gate the corrected
  walkthrough.

## Scope limit

This accepts the historical M5 learner slice only. It does not accept the
corrected visible instructor setup, live PostgreSQL pagination, or any release
gate.
