# WP-W3 leave and return

## Status

**ACCEPTED.** J3 passed in the retained-volume M5 walkthrough through the
rendered recovery path. See the [J3 review](../audits/wp_w3_leave_return_review.md),
[resolved blocker audit](../audits/m5_retained_pagination_blocker_review.md), and
[walkthrough integration review](../audits/pagination_walkthrough_integration_review.md).

## Scope

- `tests/playwright/ui_walkthrough_keyboard_j3.spec.ts` signs in through the rendered local form,
  opens the exact current course and Mastery assignment href, and starts the visible run by keyboard.
- The exact Mastery href must exist once and be visible. From the route-focused main landmark, the
  spec reaches that existing rendered link through bounded backward Shift+Tab before native Enter.
  This guards retained assignment lists without selecting a stale duplicate.
- The learner reaches the rendered `Return to assignment (Esc)` control by Tab, confirms focus,
  and activates it with Space. The spec never sends Escape, uses browser history, or navigates
  directly after entering the run.
- J3 gives the rendered route surfaces bounded waits: after visible Return to assignment, the
  overview and focused main landmark must arrive; after each visible Start or resume practice action,
  the `runAttempt` surface must arrive. The J3 fragment appends only after those final assertions.
- `tests/playwright/simulator/student_leave_resume_evidence.ts` defines the narrow future report fragment:
  public course and assignment UUIDs, elapsed time, and only visible start, leave, and return codes.
  It contains no response, answer, feedback, scoring, persistence, or credential material.

## Evidence

- The focused offline Playwright test remains skipped without the explicit real-stack invocation.
- The pure Node test validates the J3 fragment's public-only shape and rejects invalid identifiers
  and elapsed time outside the bounded range.
- M5's shared report integration owns the fixed J1, J2, J3, J4, J5, and J8 state sequence. This
  workstream passes its fragment to that boundary but does not alter the renderer or runner.

## Live acceptance

Visible course pagination reached the exact arranged Mastery link through its
native keyboard control. J3 then used rendered Return, overview, and resume
controls; its report row is PASS with empty diagnostics. The 0700/0600
redacted report and runner-owned no-volume cleanup were independently checked.
