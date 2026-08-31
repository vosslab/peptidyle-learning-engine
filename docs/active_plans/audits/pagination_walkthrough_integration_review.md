# Walkthrough visible-pagination integration review

## Verdict

**ACCEPTED.** The earlier synthetic-focus finding is repaired. Positive
pagination traversal now runs the real production `AssignmentList`; the pure
helper fixture is restricted to fail-closed branches and does not mutate a
fragment, URL, or focus target.

## Verified behavior

- The production-component browser proof starts with 50 assignments, reaches
  the visible native **Skip to load more assignments** link with Tab, activates
  its real fragment target with Enter, loads two pages through Space/Tab, and
  reaches the one exact 101st assignment. It confirms the actual product focus
  transfer to the first appended **Review assignment** link, then the 101st
  target after the second append.
- `tabTo` remains closed at 40 steps. The 80-step forward traversal is private
  to pagination and is used only after the first appended public control has
  focus. The initial absent-target path first uses the visible native skip
  link; it never directs focus to pagination.
- The assignment fragment targets the real action. Gradebook targets its
  stable, labeled `#gradebook-pagination` container with `tabindex="-1"`, then
  uses exactly one native Tab to reach the action. The helper's gradebook
  protocol branch proves that same route reaches the visible Reload control.
- Final native-fragment re-review: both product links are plain same-window
  anchors (`href="#assignment-pagination"` / `href="#gradebook-pagination"`,
  `target="_self"`). They land on named `tabindex="-1"` pagination sections,
  then take exactly one native Tab to the load, retry, or reload button. No
  skip-anchor JavaScript focus, router handler, history mutation, or pointer
  activation mediates that transition.
- The remaining fixture cases fail closed for protocol error, transport error,
  terminal state, and duplicate exact targets. The helper also rejects a
  non-increasing rendered count and an excessive number of visible loads.
- J1-J5 keyboard journeys use root entry, rendered controls, and the shared
  helper. They contain no direct route, storage, private API, answer, pointer,
  or direct-focus shortcut. The separate arranged confirmation test is
  intentionally pointer-driven and is not a keyboard journey.
- Each post-helper J1-J4 assignment locator is now explicitly required to be
  exactly one visible rendered link; J5 likewise requires exactly one visible
  gradebook row before it reaches that row's history control. These guards
  restore the source-level uniqueness/visibility contract without changing the
  keyboard path.

## Resolved finding

The former `window.location.hash` assignment that manufactured appended-link
focus is gone. The production component's focus behavior is now the positive
test oracle, so the helper no longer relies on a fixture-only URL transition.

## Validation evidence

- `npx prettier --check` for the helper/spec, production assignment proof, and
  arranged/J1-J5 call sites - passed.
- `npx tsc --noEmit -p tsconfig.lint.json` - passed.
- Focused `npx eslint` for the same files - passed.
- `source source_me.sh && python3 -m pytest
tests/test_ui_walkthrough_harness_independence.py -q` - 7 passed.
- `PW_PORT=4177 npx playwright test
tests/playwright/simulator/keyboard_walkthrough.spec.ts
tests/playwright/course_assignments_pagination.spec.ts --workers=1` - 6
  passed on IPv4 loopback. The default preview port was occupied, so this used
  an unused loopback port.
- Final fragment verification: `node --import tsx --test tests/test_*.mjs` -
  234 passed; focused Prettier, TypeScript, ESLint, G1 scanner, and diff checks
  passed. `PW_PORT=4178 npx playwright test`
  `course_assignments_pagination.spec.ts gradebook_pagination.spec.ts`
  `simulator/keyboard_walkthrough.spec.ts --workers=1` - 11 passed on IPv4
  loopback.
- `git diff --check` - passed.

No product implementation files were edited by this review.
