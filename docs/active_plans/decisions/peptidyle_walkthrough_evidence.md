# UI/UX walkthrough planning evidence

## Status

This companion is nonbinding source history and planning evidence for
`docs/active_plans/peptidyle-walkthrough-plan.md`. The plan owns scope,
contracts, dependencies, gates, and execution decisions. This file does not
override it.

The repository owner corrected the binding charter after the earlier snapshot:
the walkthrough now requires visible instructor course creation, active local
student roster addition, corpus-backed assignment construction, and student
keyboard take/score/repeat. Email and canonical onboarding are explicitly
outside the walkthrough.

## Observed repository snapshot

- A live WeBWorK browser gate already uses `launch_local_stack.sh`, the public
  gateway, validated environment inputs, and Playwright. The walkthrough
  reuses that operational shape.
- The default local launcher supplies one seeded learner and local-file
  development account credentials. Those credentials are not canonical global
  account sessions.
- The launcher seeds the native course, membership, learner enrollment, and a
  baseline native assignment, then writes mode-0600 `containers/local-demo.json`.
  Its manifest has `assignmentId` but no `courseId`, so the simulator must
  resolve the course through a supported authenticated course or assignment
  read and reuse the seeded course, membership, and learner relationship. The
  baseline assignment has one permitted attempt and delayed feedback, so it is
  arrangement evidence only, never J1/J2 mastery coverage.
- Passwordless account, invitation, and passkey surfaces remain separate
  production identity work. Their external provider evidence does not gate the
  local walkthrough.
- The instructor roster surface uses
  `/instructor/courses/:courseId/students`.
- The historical learner slice arranged later corpus-backed assignments inside
  the seeded course. The corrected charter requires the instructor to create
  the course, activate the local learner, and construct the assignment through
  visible controls; those actions may no longer be API arrangements.
- The corrected walkthrough arranges only the private retry-corpus publication
  through supported APIs. The instructor visibly creates the one core Mastery
  assignment (`AllCorrect`, `Highest`, `Unlimited`, `NewSeeds`) from that
  published problem. The retry question remains unlimited,
  immediate-full-feedback, and untimed; no answer material may enter browser
  helpers, traces, requests, or reports.
- The native source/runtime list is MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER,
  and HOTSPOT. Its all-family browser claim awaits RC4 acceptance and the
  secure learner payload gate.

## Superseded planning assumptions

The earlier draft contained assumptions that are intentionally absent from the
binding plan:

- Enrollment JSON, checksums, read-back checks, and SQL fixtures are not
  appropriate browser evidence and are removed.
- The schema-v1 learner slice created Mastery and Exam assignments through
  supported APIs. That remains historical evidence only; the corrected core
  uses visible instructor assignment construction and does not require the Exam
  contrast.
- Local-file sessions remain noncanonical, but they are the intended actors for
  this local teaching-loop walkthrough.
- Email authentication, passkeys, SMTP transport, deliverability, provider UI,
  mailbox behavior, and invitation delivery are outside the walkthrough rather
  than entry prerequisites.
- A six-family inventory is stale. The current contract names eight families.
- Undefined WP-E1 and fake-local-identity work have no execution role.

## Evidence sources to refresh before dispatch

Read these current sources before a package changes state:

| Question                              | Source of truth                                                                                        |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Immediate identity/provider readiness | `docs/active_plans/active/release_completion_plan.md` and `docs/active_plans/implementation_status.md` |
| Durable owner decisions               | `docs/HUMAN_GUIDANCE.md`                                                                               |
| Account/enrollment module boundary    | `docs/CONTRACTS.md` and `docs/ENROLLMENT_DESIGN.md`                                                    |
| Keyboard behavior                     | `docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md`                                                              |
| Live-browser operations               | `docs/E2E_TESTS.md`, `launch_local_stack.sh`, and the existing WeBWorK gate                            |
| Route and UI affordance drift         | current `src/pages/` source and a live smoke                                                           |

## Historical notes

The original draft was 1,037 lines and mixed binding requirements with code
snapshots. The binding plan is deliberately kept below the repository source
line limit and this companion holds only the compact snapshot necessary to
understand why corrections were made. Recheck paths and route labels at package
entry because UI and release work may change them.
