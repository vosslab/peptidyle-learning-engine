# WP-B1 onboarding preflight

## Walkthrough scope addendum

The repository owner has explicitly removed email and canonical onboarding
from the assignment walkthrough charter. This accepted preflight remains
historical and reusable evidence for the separate WP-RC8 production identity
package; it is no longer a walkthrough dependency, journey, or blocker.

## Result

`tests/playwright/simulator/onboarding_preflight.ts` provides a pure, redacted
operator-readiness check for the canonical account walkthrough. It returns only
`{ outcome, reasonCode }`; it neither reads SMTP password contents nor starts a
browser, sends email, creates an invitation, or consumes a one-time link.

The check requires a production posture (local development auth unset or `0`,
with no local fallback), complete valid external SMTP metadata, an absolute
regular non-symlink mode-0600 SMTP password host-file, an HTTPS public origin,
and two explicit boolean confirmations: an operator-owned test mailbox is
usable and the one-time link is available to the browser operator. `PASS` means
only that WP-W10 may begin; it is not J9 or J10 evidence.

## Current disposition

The default local workspace uses local development authentication and has no
external provider, mailbox confirmation, or delivered one-time link. It is
therefore deterministically `BLOCKED` with `LOCAL_DEVELOPMENT_AUTH`. No
onboarding ceremony was attempted.

## Acceptance

Independent review is [accepted](../audits/wp_b1_onboarding_preflight_review.md).
Runner/report integration remains unimplemented. WP-W10 and the J9/J10
onboarding journeys have not run; this preflight result is not their evidence.
