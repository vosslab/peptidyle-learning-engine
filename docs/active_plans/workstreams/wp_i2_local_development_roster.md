# WP-I2 local roster

## Scope

- Add one configured local student to a manager's course without email.
- Keep the route and roster control exclusive to local-development composition.

## Contract

- Local identity records carry a unique lowercase ASCII `learner_alias`.
- The browser posts only `learnerAlias` to the local-only member route.
- The server resolves tenant, user, display name, and roles from local config.
- Activation atomically creates an active roster record, student membership,
  and enrollments for existing assignments.
- Repeating the same activation returns the existing active member unchanged.

## Boundaries

- Production routers omit the local member route and advertise no roster control.
- Invitation, email, passkey, and canonical-account behavior remains unchanged.
- Local members use the explicit `local_development` persistence source with
  null email and roster ID.

## Verification

- Rust formatter, check, clippy, roster tests, and source-length gate.
- TypeScript decoder and alias-only request test.
- A later local-stack walkthrough supplies live PostgreSQL evidence.

## HCI repair

- Member projections carry an explicit local-pilot source label.
- Successful activation visibly identifies and focuses the exact active row.
- Unrecognized aliases retain the entered value and offer non-sensitive correction.

## Browser evidence

- The production HTTP component fixture starts with the mock-preview shell but
  selects the normal same-origin HTTP client before application boot; it stubs
  only bounded server responses.
- The capability-off student fixture shows no local-roster control and makes no
  local-member request.
- The capability-on manager fixture uses native Tab, typing, and Enter to send
  only `learnerAlias`; it proves the pending disabled button, exact active
  Local pilot row, status announcement, focus handoff, and idempotent repeat.
- The unknown-alias fixture returns `404`, retains the typed alias, and shows
  the bounded correction message without exposing a configured identity.
- 2026-08-11: `npx playwright test tests/playwright/frontend_contract.spec.ts
  --grep 'local roster|alias error' --workers=1 --timeout=15000 --reporter=list`
  passed 3 tests in Chromium.
