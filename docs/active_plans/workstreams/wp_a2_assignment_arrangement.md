# WP-A2 assignment arrangement

## Scope

- Package: WP-A2, seeded-course Mastery and Exam arrangement.
- Owner: TypeScript/API test engineer.
- Status: independently ACCEPTED through the accepted M3 live integration.
- Files: `tests/playwright/simulator/assignment_arrangement.ts` and its focused spec.

## Contract

- The module accepts an already-authenticated instructor API context, a validated launcher
  manifest summary with only the baseline `assignmentId`, and an answer-free published
  question reference.
- It resolves the seeded course only with `GET /api/assignments/{assignmentId}`, then creates
  exactly two assignments through `POST /api/courses/{courseId}/assignments`, in Mastery then
  Exam order.
- Mastery uses `AllCorrect`, `Highest`, `Unlimited`, and `NewSeeds`. Exam uses `AnswerAll`,
  `First`, `Closed`, and `NewSeeds` as the visible contrast posture.
- Every response must carry valid public identifiers, the resolved course, and its requested
  policy. The returned DTO contains only the arrangement label and public identifiers.
- The module has no authentication, account, course, membership, roster, invitation,
  enrollment, SQL, or cleanup operation. Assignment Arrangement creation uses
  the existing Student Course Membership.

## Evidence

- Focused Playwright tests pin the exact read/create paths, request order and bodies, no extra
  calls after input or response failure, created policy/course validation, caller-input
  immutability, and redacted staged errors for rejected transport or JSON operations.
- [wp_a2_assignment_arrangement_review.md](../audits/wp_a2_assignment_arrangement_review.md)
  independently accepts the offline contract. WP-A1 is also accepted offline.
- The accepted M3 runner created both assignments in the launcher-seeded course and
  the student opened their visible cards through the rendered local sign-in flow.
  See [m3_arrangement_integration.md](m3_arrangement_integration.md) and the
  independent [M3 review](../audits/m3_arrangement_integration_review.md).
