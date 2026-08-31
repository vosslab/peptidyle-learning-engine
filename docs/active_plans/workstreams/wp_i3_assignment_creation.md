# WP-I3 visible assignment creation

## Scope

- Add the manager-only New assignment action at the course assignment surface.
- Add `/instructor/courses/:courseId/assignments/new` to the executable route contract.
- Reuse the assignment editor with a create mode that starts with the Fall-pilot
  Mastery policy: AllCorrect, Highest, Unlimited, and NewSeeds.
- Search only the public catalog and retain only immutable problem/version tuples
  in browser state and the strict creation payload.

## Acceptance evidence

- The focused Node tests prove the default policy and the repository's exact
  create payload.
- The production-component Playwright tests prove the manager-only visible
  entry, its absence for students, native Tab/Enter navigation into create
  mode, public catalog selection, exact POST payload, and the resulting course
  assignment link.
- Edit-mode regression coverage remains in the same production-component spec.

## Boundary

- This work does not alter course creation, local roster activation, email, the
  live runner, protected journey state, or the walkthrough report.
- It does not claim live-stack acceptance. WP-I4 owns the subsequent visible
  instructor setup journey.
