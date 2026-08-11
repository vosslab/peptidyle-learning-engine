# WP-G2 walked-journey baseline review

## Verdict

**ACCEPTED.** The static manifest is a closed non-live baseline. Its parser
rejects duplicate JSON members at every object level before exact schema
validation. This refreshed record truthfully incorporates the accepted M5
journeys without importing live-run evidence.

## Resolved P2: duplicate members evade closure

- Location: `tests/test_walked_journey_baseline.py:41-58`.
- Resolution: `object_pairs_hook=reject_duplicate_json_members` rejects each
  repeated name before the parsed record reaches the exact-schema validator.
  This hook is applied by the standard JSON decoder at every object depth.
- Evidence: hostile raw JSON fixtures reject duplicate top-level `recordType`,
  duplicate forbidden `courseId`, and duplicate J3 `id`, `outcome`, and
  `reasonCode` members. Existing hostile object tests still reject false PASS,
  changed blockers, extra fields, and a live-report record type.

## Confirmed boundaries

- The committed record uses seed `42`, the five required arrangement labels,
  and the exact ordered J1 through J10, all-family, and multi-learner rows.
- Only J1, J2, J3, J4, J5, and J8 are `PASS`. J6/J7 use the shared
  `RELEASE_READINESS_PREREQUISITE`; J9/J10 use
  `CANONICAL_ONBOARDING_PREREQUISITE`; all-family uses
  `ALL_FAMILY_AND_SECURE_PAYLOAD_RELEASE_GATES`; multi-learner uses
  `CANONICAL_ONBOARDING_AND_ALL_FAMILY_RELEASE_GATES`.
- The manifest contains no IDs, titles, scores, identities, paths, secrets,
  selectors, raw errors, or timing. The workstream document states that it is
  static and non-live; it does not invoke a runner, stack, or browser.
- Exact key and complete-row equality reject ordinary extra and changed fields;
  duplicate-member rejection closes the parser-level hole before those
  comparisons.

## Validation

```bash
source source_me.sh && python3 -m pytest -q \
  tests/test_walked_journey_baseline.py \
  tests/test_pyflakes_code_lint.py \
  tests/test_ascii_compliance.py \
  tests/test_markdown_links.py \
  tests/test_source_file_line_limit.py
pyflakes tests/test_walked_journey_baseline.py
git diff --check
```

The pytest command passed 1,821 tests. Targeted ASCII checks and `git diff
--check` also passed. No live command was run.
