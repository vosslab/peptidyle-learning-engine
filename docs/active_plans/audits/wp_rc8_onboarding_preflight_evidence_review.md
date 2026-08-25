# WP-RC8 onboarding preflight evidence review

## Verdict

ACCEPTED for the bounded, pure `WP-RC8` preflight result. The default local Compose
posture is deterministically `BLOCKED` with `LOCAL_DEVELOPMENT_AUTH`; that is
the accepted B1 outcome, not a canonical onboarding pass. WP-W10 has not run,
and J9/J10 have no browser evidence.

## Evidence

- `onboardingPreflightFromEnvironment` returns only `outcome` and `reasonCode`.
  It neither opens a browser nor has email, invitation, link, token, account,
  or credential side effects.
- Priority is fail-closed and test-covered: exact local development auth first,
  then absent SMTP provider, invalid SMTP metadata, unsafe host password file,
  missing test-mailbox confirmation, and missing delivered-link confirmation.
  Explicit malformed configuration returns `FAIL`; unavailable operator access
  returns `BLOCKED`.
- The local Compose API sets `PLE_AUTH_PROVIDER=local-file`,
  `PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH="1"`, and `PLE_LOCAL_AUTH_FILE`. The
  preflight returns `BLOCKED/LOCAL_DEVELOPMENT_AUTH` before considering SMTP,
  so local login cannot become a canonical-account fallback.
- Production selects its direct PLE passwordless graph when the development
  flag is unset or `0`; it does not read local identity settings or mount the
  legacy local-login route. Any local-provider variables presented to the B1
  production posture fail rather than pass.
- The preflight deliberately checks the host-side
  `PLE_SMTP_PASSWORD_HOST_FILE`, because the SMTP overlay bind-mounts that file
  into a networkless initializer and exposes only the fixed container-side
  `PLE_SMTP_PASSWORD_FILE` path to the API. Inspection uses `lstat` metadata
  only: absolute regular non-symlink file and mode `0600` outside Windows. It
  never reads file contents.
- SMTP validation matches the launcher contract: relay, port, TLS mode,
  username, sender, host password-file path, and HTTPS public origin. The two
  boolean operator confirmations represent only usable mailbox and delivered
  link availability; they do not reveal an address, artifact, link, token, or
  secret.
- The module has no caller outside its focused spec yet. This is appropriate
  for the bounded B1 decision, but report integration and any browser use stay
  pending work; no result here claims a send, mailbox access, invitation claim,
  passkey enrollment, or J9/J10 completion.

## Validation

```text
npx prettier --check tests/playwright/simulator/onboarding_preflight.ts tests/playwright/simulator/onboarding_preflight.spec.ts
npx eslint tests/playwright/simulator/onboarding_preflight.ts tests/playwright/simulator/onboarding_preflight.spec.ts
npx tsc --noEmit
npx playwright test tests/playwright/simulator/onboarding_preflight.spec.ts
source source_me.sh && python3 -m pytest tests/test_markdown_links.py -q
git diff --check
```

All commands passed: the focused spec reports 5 passed; Markdown links report
136 passed. ASCII inspection of the B1 source, focused spec, and workstream
found no non-ASCII characters. `git diff --check` produced no whitespace
errors.

## Follow-on boundary

WP-W10 begins after an operator supplies real provider access, a usable canonical test mailbox, and
a delivered browser-available one-time link so the preflight returns `PASS`. Missing operator access
produces the redacted `BLOCKED` state; J9 and J10 retain their own browser evidence requirements.
