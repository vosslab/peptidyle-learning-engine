# WP-I2 local roster HCI review

## Verdict

ACCEPTED for the WP-I2 browser/client/HCI boundary.

This re-review covers the local-development roster control required by
`docs/active_plans/peptidyle-walkthrough-plan.md`. It does not make production
email, mailbox, invitation claim, passkey, or canonical-account work an
acceptance condition.

## Re-review results

| Requirement | Evidence | Result |
| --- | --- | --- |
| Local-only capability | The local form renders only when the server response advertises `localDevelopmentRoster`; the production-style server route remains unmounted | OK |
| Manager-only action | Server roster and activation routes require manager course access; capability-off Chromium case renders no local control or request | OK |
| Alias-only request | The browser sends exactly `{ learnerAlias }`; the configured server-side directory resolves all identity fields | OK |
| Strict redacted decoder | The member source is a closed browser presentation enum; accepted local responses require `localDevelopment`, active student role, and null email/roster ID | OK |
| Local-pilot distinction | The active row explicitly displays `Local pilot`, rather than mislabelling the record as legacy or exposing email/ID | OK |
| Native keyboard control | Chromium J12 uses Tab, typing, and native Enter through labelled input and submit controls | OK |
| Pending, success, and focus | The submit control disables during the pending request; success visibly renders and focuses the exact active row and announces it | OK |
| Error recovery | An unknown alias preserves entry and gives bounded, actionable, non-sensitive recovery wording | OK |
| Idempotent repeat | A second native keyboard submission repeats the same alias-only request and retains the focused active row | OK |
| J12 no-cheating boundary | The new component journey uses no pointer click, direct focus, route shortcut, API shortcut, cookie, storage, or identity leak | OK |

The prior review findings are resolved. In particular, source is now explicit
instead of inferred from nullable fields, and the focused row is a product
response to successful native submission rather than a browser-test shortcut.

## Checks run

| Check | Result |
| --- | --- |
| `npx tsc --noEmit -p tsconfig.json` | PASS |
| `npx tsc --noEmit -p tsconfig.lint.json` | PASS |
| Focused ESLint for roster/client/test files | PASS |
| Focused Prettier check for roster/client/test files | PASS |
| `node --import tsx --test tests/test_enrollment_client.mjs` | PASS, 6 tests |
| `source source_me.sh && python3 -m pytest -q tests/test_ui_walkthrough_harness_independence.py tests/test_local_development_roster_contract.py` | PASS, 8 tests |
| `./run_playwright_tests.sh --build tests/playwright/frontend_contract.spec.ts` | PASS, 14 tests; includes the 3 local-roster Chromium cases |

## Scope and next gate

WP-I2 is ready for its Store/PostgreSQL and live-integration gates. The later
fixed J12 instructor setup journey must retain this native keyboard contract and
the report must continue to omit credentials, email, roster IDs, tenant IDs, user
IDs, and alias values.
