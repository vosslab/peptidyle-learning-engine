# WP-I3 assignment creation review

## Scope and verdict

Independently reviewed WP-I3 against the corrected instructor-to-student
walkthrough plan. This review covered only the browser assignment-creation
surface, its public client boundary, and focused test evidence. It did not run
the live stack, inspect email work, or change implementation.

**CHANGES REQUIRED.** The create route, create/edit separation, Mastery
defaults, exact immutable catalog tuples, strict payload boundary, recovery
copy, success link, and create-editor keyboard evidence are sound. One
production-component test is still needed to prove the instructor can reach
the route from the manager-only visible New assignment entry and that a learner
cannot see that entry.

## Confirmed behavior

- `CourseAssignmentsPage` renders `New assignment` only when the session has a
  global instructor/administrator role and the course projection says the
  actor is that course's instructor or administrator. The route gate repeats
  the course-role check before constructing the editor.
- `/instructor/courses/:courseId/assignments/new` is an executable route,
  distinct from the existing revisioned edit route. Create mode never loads an
  assignment ID, starts from an empty draft, and calls `createAssignment`;
  edit mode retains load plus compare-and-swap save behavior.
- Create defaults are exactly AllCorrect, Highest, Unlimited, and NewSeeds.
- Catalog state retains only public title plus immutable `problem`/`version`
  tuples. The outgoing body is exactly `title`, `problems`, and `policies`.
  The request decoder rejects any extra field rather than silently dropping it.
- Validation, conflict, 403, 409, and ordinary transport failure leave the
  current draft in place and provide labelled recovery text. A successful
  create renders a real course-assignment anchor using the returned public IDs.
- The focused production-component test exercises native labels, keyboard
  Enter activation for adding the published version and creating the
  assignment, checks all four default policies, captures the exact POST body,
  and observes the visible success link.

## Finding

### Medium - manager-only entry evidence starts beyond the visible entry point

- **Location:** `tests/playwright/assignment_editor.spec.ts:330-424` and
  `src/pages/course_assignments_page.tsx:236-244`.
- **Evidence:** The create-mode Playwright test changes browser history directly
  to `/instructor/courses/:courseId/assignments/new`. It proves the gated route
  and editor, but it does not render the course assignment surface to assert
  that a course manager sees the `New assignment` link, that a learner does
  not, or that keyboard Enter on that real link reaches the create screen.
  Source inspection shows the intended condition, but this is a pilot-facing
  visible-action contract and should have production-component evidence.
- **Required repair:** Add a focused production-component Playwright test that
  starts on the course assignment surface in both manager and learner course
  roles. Assert the exact New assignment link is present only for the manager,
  use Tab/Enter to follow it, and then retain the current create-flow assertions.
  This test must not use a direct route as the proof of the visible entry.

## Validation

- `node --import tsx --test tests/test_assignment_editor_ui.mjs tests/test_assignment_client.mjs`
  - passed: 9 tests.
- `npx tsc --noEmit`
  - passed with no diagnostic output.
- `npx prettier --check` on the reviewed TypeScript and Playwright files
  - passed: all matched files use Prettier code style.
- `npx eslint --max-warnings 0` on the reviewed TypeScript and Playwright files
  - passed with no diagnostic output.
- `npx playwright test tests/playwright/assignment_editor.spec.ts`
  - passed: 5 tests.
- `git diff --check`
  - passed.

## External concurrent changes

The working tree contains concurrent course, roster, pagination, runner, and
other walkthrough edits. This review did not attribute or assess those changes;
the only reviewed implementation files were the WP-I3 assignment route, editor,
repository, client contract, and their focused tests.

## Re-review verdict - 2026-08-11

**ACCEPTED.** The repair closes the sole finding. The production-component
Playwright coverage now opens the visible course card, proves that only the
course manager sees the exact New assignment link, reaches and activates that
link with native Tab and Enter, confirms the create screen, and proves a course
learner sees no link and issues no mutation. The prior create-mode test still
proves the strict public payload, Mastery defaults, immutable catalog selection,
success link, and keyboard creation path.

The owner-reported focused suite passed 7 Playwright tests. My local rerun was
blocked before any test body by macOS Chromium Mach-port permission failures in
the shared agent environment; this is external execution contention, not a
product assertion failure.
