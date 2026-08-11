# WP-G2 walked-journey baseline

## Status

**ACCEPTED HISTORICAL BASELINE; SUPERSEDED FOR FINAL ACCEPTANCE.** This is a
closed, deterministic description of the earlier schema-v1 learner slice, not
a live runner report. It records the manager and independent M5 acceptance
without copying live report identifiers or other ephemeral evidence. The
owner's corrected charter requires a new schema-v2 baseline for visible
instructor course/roster/assignment setup followed by student keyboard
take/score/repeat.

## Record

- [walked_journey_baseline.json](../walked_journey_baseline.json) records master
  seed 42 and only the five arrangement labels; it contains no run-specific IDs,
  titles, scores, identities, paths, secrets, selectors, raw errors, or timing.
- J1, J2, J3, J4, J5, and J8 are `PASS` from manager and independently
  accepted M5 live evidence. The retained-pagination resolution records this
  result without making the baseline a copy of its report. See the
  [retained-pagination blocker review](../audits/m5_retained_pagination_blocker_review.md).
- The J6/J7, J9/J10, all-family, and multi-learner blocker rows preserve the
  old charter's accepted static artifact only. They are not the corrected
  walkthrough's status vocabulary. The schema-v2 replacement will omit them;
  email and canonical onboarding are explicitly outside the walkthrough.

## Guard

`tests/test_walked_journey_baseline.py` keeps the manifest ASCII-only and
enforces its exact ordered keys, arrangement vocabulary, journey order, outcome
vocabulary, and reason-to-dependency mapping. Its hostile in-memory cases
reject false PASS, a mismatched blocker, an added public identifier, and a
live-report record type. The loader also rejects duplicate JSON member names at
every object level before validation, including duplicate top-level record type
or forbidden fields and duplicate journey IDs, outcomes, or reason codes.

## Review boundary

The duplicate-member parser and M5 outcome refresh remain independently
accepted historical evidence. This baseline cannot accept or block the
corrected charter.

## Validation

```bash
source source_me.sh && python3 -m pytest tests/test_walked_journey_baseline.py
```

This fast test validates the static record only. It does not start a stack,
launch a browser, or convert this baseline into live evidence.
