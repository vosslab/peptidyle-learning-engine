# HG email sign-in browser consumer - 2026-10-04

## Implemented boundary

- `/sign-in` now offers ordinary Student and Instructor email-code sign-in alongside the retained
  seeded-role Live Demo entry required by Human Guidance 201.
- The same-origin browser client starts `POST /api/auth/email-code/start` with only `email`, then
  completes `POST /api/auth/email-code/complete/{challenge_id}` with only `code`. Its strict
  decoders accept only `{ emailCodeRequested: true, challengeId }` and `{ authenticated: true }`.
- Accepted start requests show generic delivery language. The browser never indicates whether an
  Account exists. A missing configured route (404) or unavailable mail service (503) shows that
  email sign-in is unavailable and does not claim a code was sent.
- Completion refreshes the shared session bootstrap before moving the signed-in viewer to their
  role home. The server continues to own code validation, one-use consumption, rate limits, and
  the host-only browser binding cookie.

## Security boundary

- ASVS 1.5.2: response DTOs are closed and validate the opaque UUID before it reaches a route.
- ASVS 2.2.1 and 2.2.2: the form uses familiar constrained email and code controls for usability;
  the existing server endpoint remains the trusted input-validation authority.
- ASVS 2.3.1: completion is available only after a server-issued challenge ID, and browser success
  is accepted only after the completion response before session refresh.
- ASVS 4.1.1: the existing shared JSON transport requires JSON responses and does not surface
  response bodies in errors. It sends same-origin, no-store credentialed requests only.

## Validation

- `node --import tsx --test tests/test_email_code_sign_in.mjs`: 3 passed.
- `npx tsc --noEmit`: passed.
- `git diff --check`: passed for the shared in-flight worktree.

## External dependency

Live email delivery remains unavailable until the operator configures the SMTP account. The local
consumer is wired and fails clearly when the normal email-code route is absent or delivery is
temporarily unavailable; no external email was sent during this work.
