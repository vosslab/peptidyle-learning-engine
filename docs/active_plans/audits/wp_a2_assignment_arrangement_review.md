# WP-A2 assignment arrangement review

## Verdict

ACCEPTED for offline contract work. WP-A1 is independently accepted offline, and
WP-A2 now satisfies its arrangement and redacted-failure contract. M3 remains
pending integrated real-stack evidence.

## Contract evidence

- `assignment_arrangement.ts` makes exactly one baseline `GET` to
  `/api/assignments/{assignmentId}`, then two `POST` requests to the resolved
  `/api/courses/{courseId}/assignments`, in Mastery then Exam order.
- Its request bodies exactly match the strict Rust create boundary: `title`,
  `problems`, and `policies`. The server's `strict_assignment_request` rejects
  unknown or noncanonical fields.
- The Mastery request uses `allCorrect`, `highest`, `unlimited`, and `newSeeds`.
  The Exam contrast uses `answerAll`, `first`, `closed`, and `newSeeds`.
- The server returns the browser-safe `AssignmentSummary` projection. The module
  verifies HTTP status, UUID-shaped assignment and course IDs, the resolved
  course, and the requested policies before returning only arrangement labels and
  public IDs.
- No code reads or creates a course, membership, roster, invitation, account, or
  enrollment; it has no SQL, cleanup, environment, manifest-file, or logging
  operation. The caller-owned manifest and question reference remain unchanged.

## Repair verification

`readBaselineAssignment` and `createAssignment` now convert every rejected
transport or JSON promise into an `AssignmentArrangementError` for its current
stage. The focused suite injects a secret-bearing sentinel into rejected baseline
GET, Mastery POST, Exam POST, and each JSON body. Every resulting error is
redacted, names the expected stage, and records only the request prefix that can
have run; later assignment calls never occur.

## Validation

- Prettier and ESLint checks on the module and its spec - passed.
- Strict focused TypeScript check - passed.
- Focused Playwright contract suite - 12 passed.
- ASCII checks, `pytest -q tests/test_markdown_links.py` (136 passed), and
  `git diff --check` - passed.

```bash
npx prettier --check tests/playwright/simulator/assignment_arrangement.ts \
  tests/playwright/simulator/assignment_arrangement.spec.ts
npx eslint tests/playwright/simulator/assignment_arrangement.ts \
  tests/playwright/simulator/assignment_arrangement.spec.ts
npx tsc --ignoreConfig --noEmit --target es2020 --module esnext \
  --moduleResolution bundler --strict --noImplicitAny --noUncheckedIndexedAccess \
  --noImplicitOverride --verbatimModuleSyntax --useUnknownInCatchVariables \
  --noFallthroughCasesInSwitch --noImplicitReturns --noUnusedLocals \
  --noUnusedParameters --isolatedModules --esModuleInterop --skipLibCheck \
  tests/playwright/simulator/assignment_arrangement.ts \
  tests/playwright/simulator/assignment_arrangement.spec.ts
npx playwright test tests/playwright/simulator/assignment_arrangement.spec.ts \
  --reporter=line
```

## Scope boundary

This is an offline review only. M3 live acceptance still requires the existing
seeded student to open both later assignments through the integrated runner.
