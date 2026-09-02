# WP-W6 J4 visible Mastery and Exam policy contrast

## Status

**ACCEPTED.** The paired J4 policy contrast passed in the retained-volume M5
walkthrough. See the [J4 review](../audits/wp_w6_policy_contrast_review.md),
[course pagination review](../audits/pagination_course_assignments_review.md), and
[resolved blocker audit](../audits/m5_retained_pagination_blocker_review.md).

## Scope

- `ui_walkthrough_keyboard_j4.spec.ts` enters through the rendered local student sign-in form and
  follows the exact arranged course, Mastery, and Exam `href` values using Tab, Enter, and Space.
- The Mastery path handles an already completed state only through the rendered summary and its
  `Start another practice Assignment Attempt` control, then takes the first and second visible radio controls
  through their visible submission and continuation states. It requires exactly two unchecked
  rendered native radios; the second is reached only through bounded backward Shift+Tab.
- The closed Exam path completes one visible response, requires `This Assignment Attempt is complete`, and requires
  that no fresh-practice action or Mastery heading exists. A resumed closed Exam remains visibly
  closed; a resumed fresh-practice state fails rather than being activated. Both completion summaries
  retain `Back to assignment`.
- The journey observes labels, focus, route surfaces, and completion controls only. It does not read
  feedback body content, correctness, score, answer material, browser storage, private state, API
  responses, direct post-login routes, or pointer actions.
- `student_completion_policy_evidence.ts` accepts only fixed visible codes, public UUIDs, and bounded
  elapsed time. Report integration is intentionally deferred to the shared report owner.
- `student_completion_terminal_surface.ts` treats neutral completion, Feedback, and pending rendering as bounded
  transients after Continue. Only the paired Mastery heading/action or exact closed Exam heading is
  accepted as the final visible policy surface.

## Offline evidence

- PASS: `node --import tsx --test tests/test_student_completion_policy_evidence.mjs` reported three
  passing public-boundary and source-guard tests.
- PASS: `npx playwright test tests/playwright/ui_walkthrough_keyboard_j4.spec.ts --list` found the
  one J4 test; the focused offline invocation skipped it as required outside the explicit live mode.
- PASS: `npx tsc --noEmit`, ESLint, and Prettier completed with no diagnostics after formatting.
- PASS: `source source_me.sh && python3 -m pytest tests/test_ascii_compliance.py
tests/test_markdown_links.py -q` reported 958 passed; `git diff --check` exited cleanly.

## Live acceptance

Visible course pagination reached the exact current Mastery and Exam links.
The keyboard-only paired journey passed and appended its public J4 report row
with empty diagnostics. The retained-volume report was mode 0700/0600 and the
runner's no-volume cleanup left no private temporary state or containers.
