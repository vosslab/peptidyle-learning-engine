# M22/M23 Question Statistics

## Current connected evidence

The current Unrelease connected run passed in
`output_question_spec/statistics_current_20261007.log` (exit 0). It executes
`tests/e2e/unrelease_connected_oracle.sql` through
`tests/_temp/e2e_unrelease_focused_install.sh` and verifies delivery retry,
direct-versus-Pool counts, repeated finalization, and replacement-member origin.
This closes the prior SQL-runtime gap for these oracle assertions; M11/M17
dependencies and browser/integrated M22/M23 acceptance remain pending.

## Implementation

The private statistics receipt is created in the transaction that commits a
Question presentation. Its primary key `(course_instance_id,
issued_question_id)` makes retries idempotent. The receipt retains the exact
Question Revision and the originating Pool ID copied from the issued Pool
selection. A later finalization stores the submission and normalized Backend
credit on that same receipt. A NULL credit remains ungraded; it does not
increase the graded count. The Assessment partial-credit setting and point
conversion do not enter this path.

Question Revision aggregates retain deliveries and outcomes by exact Revision.
Pool aggregates retain deliveries and outcomes by the Pool ID recorded when
the Question was delivered. Pool reads no longer derive outcomes from current
Pool membership, and bulk Question reads use the current Revision rather than
summing historical Revisions. Existing privacy contributor floors and
identity-free retention remain in place. Removed the obsolete current-member
statistics table and its trigger/helper.

The Question Statistics panel now shows Received, Graded, Mean credit, Full
credit, and Zero credit, with per-Revision rows on Question detail. Percentages
use graded outcomes as the denominator.

## Focused evidence

- `source ./source_me.sh && devel/generate_schema_tables_doc.py && schema_style/check_schema_style.py` -
  schema docs/catalog regenerated; no findings from the statistics changes.
  One unrelated advisory remains for the M09 Issued Question tuple index.
- `source ./source_me.sh && node --import tsx --test tests/test_question_statistics_decoder.mjs` - 2 passed.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json` - blocked by
  concurrent generated-contract/type drift in Blueprint Theme, Bloom, regex,
  and Pool tuple work; the changed statistics panel produced no reported
  diagnostic.
- `source ./source_me.sh && cargo test -p question_model question_library_statistics --lib` -
  4 passed after the concurrent M09 fixture updates landed. This checks the
  stable projection shape and privacy floor behavior; it is not connected
  database evidence.

The connected oracle source commits both direct and Pool-selected presentations
through the production API, retries one committed presentation, and checks
that each delivery is counted once before grading. It stores one `0.5` Backend
fraction and one full-credit response through the Student finalization API,
repeats finalization, and checks the exact Revision and originating-Pool totals.
The first Assessment has partial credit disabled: its `0.5` remains a partial
Question outcome while awarded points are zero. After the Pool Question is
issued, the fixture replaces the Pool member through the production member
save function, then checks that the outcome remains with the originating Pool
and does not appear on the replacement Question. These assertions passed in the
current connected run identified above; earlier source-only review notes below
describe the prior state.

API projection and browser acceptance also remain open.

## Independent review

The earlier M22/M23 SPEC review passed source alignment and blocked milestone
acceptance on the then-missing connected evidence. Its independent QUALITY
review reached the same split and identified stale Rust comments that
described an all-Revision Question rollup and current-member Pool outcomes.
Those comments are corrected in the model projections and PostgreSQL store.
Both reviews predate this expanded direct-and-Pool oracle source and do not
review it; their result is recorded in
`output_question_spec/m22_m23_spec_cli.md`. Fresh source reviews remain needed.

## Limits

API acceptance and browser rendering remain pending. The two prior independent
source reviews passed conditionally; neither accepts M22 or M23 by itself. The
current connected SQL pass proves the stated oracle assertions, while M11/M17
dependencies and integrated milestone acceptance remain open.
