# WP-I4 instructor setup

## Scope

This workstream adds the fixed local instructor J11/J12/J13 browser child. It preserves schema-v1
student modules and does not retarget J1-J5 or J8.

## Implemented boundary

- J11 signs in through the visible local form, creates a unique course, and opens its exact card.
- J12 opens Students, reads the configured alias only at the visible local-only form action, and
  observes one active local-pilot row.
- J13 returns through the rendered Back to course link, creates a Mastery assignment from the
  published catalog, and observes its exact course link.
- The runner's `--instructor-setup-only` branch arranges only retry-corpus publication and runs one
  fixed child. It retains no child stdout, stderr, credential, alias, or private state artifact.
- The schema-v2 state commits the exact J11/J12/J13 prefix atomically only after all visible J13
  assertions pass. It contains only public course, assignment, problem, and version IDs.

## Offline evidence

- Both root TypeScript configurations, ESLint, Prettier, `git diff --check`, and focused scanner pass.
- Focused Python runner/scanner tests pass, including one-child failure/redaction/no-volume cleanup.
- Focused Playwright state/config/arrangement tests pass; the live-only J11/J12/J13 test skips outside
  the explicit runner invocation.

## Accepted integration evidence

The two retained-stack seed-42 `--build` walkthroughs used this fixed child to
create and open a fresh course, activate the local student, and construct the
corpus-backed Mastery assignment before student work began. Its public-ID
handoff then bound the remaining journeys to that visible setup. The reports
were redacted mode-0700/0600 artifacts and cleanup left no containers or
private state. This accepts only the local no-email pilot setup; canonical
email onboarding and production enrollment remain outside this workstream.
