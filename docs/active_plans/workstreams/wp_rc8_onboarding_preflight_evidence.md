# WP-RC8 onboarding preflight evidence

## Walkthrough scope addendum

The assignment walkthrough charter follows the visible product workflows in
[LIVE_DEMO_SPEC.md](../../LIVE_DEMO_SPEC.md). This accepted preflight remains historical and reusable
evidence for the separate `WP-RC8` production identity package.

## Result

`tests/playwright/simulator/onboarding_preflight.ts` provides a pure, redacted
operator-readiness check for the canonical account walkthrough. It returns only
`{ outcome, reasonCode }` and keeps SMTP credentials and browser state outside its boundary.

The check requires a production posture (local development auth unset or `0`,
with no local fallback), complete valid external SMTP metadata, an absolute
regular non-symlink mode-0600 SMTP password host-file, an HTTPS public origin,
and two explicit boolean confirmations: an operator-owned test mailbox is
usable and the one-time link is available to the browser operator. `PASS` authorizes WP-W10 to begin;
J9 and J10 retain their own evidence requirements.

## Historical disposition

The default local workspace uses local development authentication and has no
external provider, mailbox confirmation, or delivered one-time link. It is
therefore deterministically `BLOCKED` with `LOCAL_DEVELOPMENT_AUTH`. No
onboarding ceremony was attempted.

## Acceptance

Independent review is [accepted](../audits/wp_rc8_onboarding_preflight_evidence_review.md).
Runner/report integration remains unimplemented. WP-W10 and the J9/J10
onboarding journeys have not run; this preflight result is not their evidence.
