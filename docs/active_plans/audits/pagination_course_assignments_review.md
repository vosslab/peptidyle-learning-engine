# Course assignment pagination independent review

## Verdict

**ACCEPTED.** The course list now provides a visible native route to its real
pagination or retry control, keeps opaque-cursor semantics in the shared model,
and has focused keyboard evidence for first-page, second-page, retry, protocol,
and terminal behavior. No P0 or P1 finding remains.

## Resolved evidence repair: Browser fixture is isolated

The earlier fixture race is fixed. The specification now starts at `about:blank`
and creates its own `main#main-content` before mounting the bundled product
component. Its normal path proves the native route skip, native pagination
fragment, Space activation, 51st-card focus handoff, and a second continuation
page. The focused Playwright run passes two tests.

## Resolved P1: Recoverable retry has a top-of-list native shortcut

The skip link now remains visible for `transport` and `protocol` errors and
uses `target="_self"` to land natively in the same browsing context on the
real, labelled `section#assignment-pagination`. Its `tabindex="-1"` permits
native fragment focus, then ordinary Tab reaches the actual load, retry, or
Reload assignments button. No hidden sentinel or JavaScript focus handler
mediates this route. Terminal state removes both the continuation link and its
section because `nextCursor` is null. The focused specification verifies normal,
transport, and protocol fragment focus plus the exact-cursor retry.

## Accepted design points

- `CursorPageSession` retains the exact failed opaque cursor, shares concurrent
  calls, deduplicates initial and appended pages, and makes nonterminal repeated
  cursors or zero-new pages terminal protocol errors.
- `CourseAssignmentsPage` uses a keyed loaded-page boundary, so a changed
  router-owned initial page reconstructs the append session instead of mixing
  courses.
- The load-more control is after the cards and focus moves to the first newly
  appended Review assignment link. That preserves reading order and the
  browser proof reaches the next control through a second visible page.
- The grid is marked busy while a request is pending; terminal, retry, and
  protocol paths keep visible records and provide native recovery controls.

## Checks run

```text
node --import tsx --test tests/test_cursor_page_session.mjs   # 6 passed
npx tsc --noEmit                                              # passed
npx eslint src/pages/cursor_page_session.ts src/pages/course_assignments_page.tsx \
  tests/playwright/course_assignments_pagination.spec.ts       # passed
npx prettier --check src/pages/cursor_page_session.ts src/pages/course_assignments_page.tsx \
  tests/test_cursor_page_session.mjs \
  tests/playwright/course_assignments_pagination.spec.ts       # passed
git diff --check                                              # passed
```

The isolated Playwright check ran with `PW_PORT=4188` outside the macOS
sandbox, because Chromium requires the Mach service denied inside it. It passed
two tests. The model suite passed six tests; TypeScript, ESLint, Prettier, and
diff checks passed. No product or test files were edited by this review.
