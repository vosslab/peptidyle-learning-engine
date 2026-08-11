# WP-W4 instructor gradebook journey

## Status

**ACCEPTED.** J5 and derived J8 passed in the retained-volume M5 walkthrough.
See [wp_w4_instructor_gradebook_review.md](../audits/wp_w4_instructor_gradebook_review.md),
[gradebook pagination review](../audits/pagination_gradebook_review.md), and
[resolved blocker audit](../audits/m5_retained_pagination_blocker_review.md).

## Scope

- `ui_walkthrough_keyboard_j5.spec.ts` opens a new isolated browser context, signs in through the
  rendered instructor local-login form, opens the exact arranged course with Tab and Enter, then
  reaches only `a[href="/instructor/courses/<arranged-course-id>/gradebook"]` and its visible View
  run history control by keyboard. The spec asserts one matching link before Tab, focus, and native
  Enter activation.
- The browser assertion observes only the gradebook surface, the button's `aria-expanded=true`,
  and a named run-history region. It neither reads score, date, or learner-identity text nor uses
  API contexts, session injection, saved browser state, cookie operations, or database access.
- `instructor_gradebook_j5.ts` owns a narrow public-only J5 fragment with exact course and
  assignment identifiers and the two visible outcome codes. The M5 report-integration owner must
  add it to the closed shared renderer and runner state contract.
- The local instructor credential reader validates the same private regular mode-0600 file and
  reads only its one `instructor=` value at the visible form boundary.

## Shared integration

The fixed Python runner now appends only the closed J1, J2, J3, J4, J5, J8
sequence and renders the final public-only result. J5 still makes a fragment
only after one exact visible gradebook row and its scoped run-history control
are reached.

## Offline evidence

Focused formatter, linter, strict TypeScript, credential-reader, J5 fragment,
and shared-integration checks are recorded by the linked reviews. The visible
gradebook native pagination path reaches the exact current row, then its
rendered run-history control. J5 and J8 report PASS with empty diagnostics;
the reviewed 0700/0600 redacted report and no-volume cleanup retain no private
temporary state or containers.
