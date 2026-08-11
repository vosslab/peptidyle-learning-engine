# Pagination product final review

## Verdict

**ACCEPTED after route-lifecycle repair and re-review.** The originally found
P1 stale-course defect and its missing continuation proof were both repaired.
The cursor model, course assignment surface, accessibility/recovery behavior,
production-component fixtures, and MemoryStore scale conformance satisfy this
pagination vertical's product contract.

## Required correction

`src/pages/gradebook_page.tsx` reads `const courseId = params["courseId"]`
once during component construction and calls `loadGradebook()` only from
`onMount`. `loadGradebook`, the cursor loader, and subsequent retry/reload
closures consequently retain that original value. Unlike
`CourseAssignmentsPage`, which has a keyed loaded-page boundary that
reconstructs its append session when the router-provided page changes, the
gradebook has no reactive/keyed course boundary or course-change cleanup.

Make gradebook loading/session ownership depend on the current router course
identifier, discard or supersede an in-flight response from the prior course,
and add a production-component browser test that changes course A to course B
without unmounting the component. The test must prove that the rendered rows
and the cursor continuation request both use B, never the stale A cursor.

## Accepted evidence

- `CursorPageSession` serializes concurrent continuation requests, keeps an
  opaque failed cursor for transport retry, removes duplicate keys from both
  initial and appended pages, and fails closed for repeated or zero-new
  nonterminal pages. The terminal duplicate-only page remains valid.
- Course assignments use a keyed loaded-page boundary. Their production
  fixture mounts `AssignmentList`, navigates by its native fragment link,
  requests page two with Space, focuses the first appended public link, and
  proves transport and protocol recovery controls.
- Gradebook uses the stable `(assignmentId, enrollmentId)` identity, keeps the
  initial request compact, and continues to request run history only after the
  instructor activates that row's public control. No grade/transport detail is
  added to notices beyond the already-visible record count.
- Both surfaces retain a visible native skip link while a continuation or
  recovery action exists. The gradebook's stable focusable fragment container
  supports native fragment navigation followed by one Tab to the actual
  action; the course link targets its actual button. Success focuses the first
  newly appended public control; failures focus the exact retry or reload
  control. Terminal state removes the no-longer-useful skip/action pair.
- The browser fixtures compile and mount the production exported components
  with the production API runtime/client seam. Their `page.evaluate` calls
  inspect or configure only the fixture transport; UI transitions occur by
  native Tab, Enter, or Space.
- MemoryStore conformance creates 51 assignments/enrollments, traverses both
  default and 17-item pages, checks stable order/no duplicates/cursor progress,
  and verifies foreign-tenant concealment. It is correctly described as
  MemoryStore conformance, not false live-PostgreSQL coverage.

## Validation evidence

- `node --import tsx --test tests/test_cursor_page_session.mjs tests/test_gradebook_pagination.mjs` - 9 passed.
- `npx playwright test tests/playwright/course_assignments_pagination.spec.ts tests/playwright/gradebook_pagination.spec.ts tests/playwright/simulator/keyboard_walkthrough.spec.ts --workers=1` - 13 passed.
- `npx tsc --noEmit -p tsconfig.lint.json` - passed.
- Focused ESLint and `npx prettier --check` across changed pagination sources/tests - passed.
- `cargo test -p learning-data-access --test conformance memory_store_conforms` - 1 passed.
- `git diff --check` - passed.

## Re-review after route-lifecycle repair

The product repair itself resolves the original stale-course defect:

- `GradebookPage` now uses a keyed `GradebookCoursePage` per router course,
  so course-owned session and history state is disposed and reconstructed on a
  course change.
- A per-child disposal flag, load generation, session identity check, and
  repeated checks before deferred focus prevent a delayed former-course
  initial/continuation response from rendering or moving focus in the current
  course.
- The production-component fixture now changes the reactive router course from
  A to B while A's initial response is delayed. It proves B's row is rendered
  and the delayed A response cannot replace it.

The re-run succeeded:

- `node --import tsx --test tests/test_cursor_page_session.mjs tests/test_gradebook_pagination.mjs` - 9 passed.
- `npx playwright test tests/playwright/gradebook_pagination.spec.ts --workers=1` - 5 passed.
- `npx tsc --noEmit -p tsconfig.lint.json`, focused ESLint, Prettier, and
  `git diff --check` - passed.

The final A to B fixture now activates B's visible native skip link and
**Load more gradebook records** control with Space while A remains delayed. It
records exactly B's first request and B's opaque continuation cursor, then
releases A and proves both B rows remain with no A row. The focused browser
run passes all five tests. **Final verdict: ACCEPTED.**

## Final native-fragment re-review

**ACCEPTED.** Both pagination skip links are plain anchors with their exact
same-page fragments and `target="_self"`. The installed Solid Router documents
that any `target` value disables interception for an individual anchor, so this
is the deliberate browser-owned fragment path rather than a router transition.
Each link lands on its persistent named `section` (`assignment-pagination` or
`gradebook-pagination`) with `tabindex="-1"`; one native Tab then reaches the
real continuation, retry, or reload button. There is no pagination-link click
handler or custom fragment-focus code in either product component.

Focused browser evidence verifies the URL/landing focus/one-Tab action and the
`target="_self"` attribute for gradebook, and verifies the same named landing
region and `target="_self"` across course transport and protocol recovery.
`npx playwright test tests/playwright/course_assignments_pagination.spec.ts
tests/playwright/gradebook_pagination.spec.ts --workers=1` passed 7 tests;
focused TypeScript, ESLint, Prettier, and `git diff --check` also passed.
