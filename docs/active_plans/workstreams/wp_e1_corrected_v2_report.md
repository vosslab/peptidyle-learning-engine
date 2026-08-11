# WP-E1 corrected schema-v2 report and baseline

## Status

**NOT ACCEPTED LIVE.** This workstream supplies isolated validation and rendering components for
the corrected no-email pilot. It does not replace the historical accepted WP-V2 schema-v1/J1
report, its audit, or its static baseline. The schema-v2 fixture is deliberately prospective until
the complete retained-stack walkthrough has passed and independent review accepts its live reports.

## Scope

- Own the isolated schema-v2 state parser, public report renderer, and renderer child.
- Require exactly J11, J12, J13, J1, J2, J3, J4, J5, and J8, in that order, with closed PASS
  vocabularies and public-ID cross-binding.
- Render only the fixed corpus-publication arrangement label and public milestone evidence.
- Keep the prospective nine-PASS fixture under `tests/fixtures/`; it is not an authoritative
  walked baseline and must not be moved to `docs/active_plans/` before live acceptance.

## Exclusions

- No runner integration, Podman launch, live acceptance claim, or change to schema-v1 artifacts.
- No J6/J7, onboarding, email, mailbox, all-family, multi-learner, score value, learner identity,
  title, response, or run-detail evidence.

## Promotion gate

Promotion requires two same-seed retained-stack runs with fresh instructor-created courses and
assignments, canonical mode-0700/0600 reports, and independent HCI and report-security review.
Only then may a corrected static baseline be published as accepted documentation.
