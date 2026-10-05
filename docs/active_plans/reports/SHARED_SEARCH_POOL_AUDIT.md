# Shared search M8 Pool-membership audit

Date: 2026-10-04

This is the M8 prerequisite from
`docs/active_plans/read-docs-active-plans-active-shared-sea-peaceful-mountain.md`.
It is evidence for the later membership-rule repair, not the repair itself.
The report and helper make no schema or data changes.

## Scope and method

The inspected Live Demo was the already-running `ple-live-demo-browser`
project. `local_stack.py status --project ple-live-demo-browser --json` reported
it ready, with exactly one running PostgreSQL container (`91fc9f685b80` at the
time of inspection). The audit used a PostgreSQL `BEGIN READ ONLY` transaction,
then `SET LOCAL ROLE ple_data_owner`, and ended with `ROLLBACK`. It did not
start, stop, rebuild, migrate, or otherwise take the Developer Browser Suite
lease.

Run the repeatable live audit from the repository root:

```sh
source ./source_me.sh && python3 tests/_temp/audit_shared_search_pool_membership.py
```

The helper asks the existing Local Stack Controller for the active PostgreSQL
container, so it does not assume the transient container ID above. It prints
the exact SQL result and returns nonzero if the named project is absent or not
ready.

## Executed Live Demo evidence

The audit query returned no affected Pool rows. Its final total was:

| Pools | Pool members | Affected Pools |
| ---: | ---: | ---: |
| 0 | 0 | 0 |

Therefore the current Live Demo seed has no affected Pools, no affected
Assessment entries, and no baseline Questions-per-Attempt or total-points rows
to preserve. This is executed database evidence, not an inference from the
seed source. It must be rerun after a Live Demo rebuild because a zero-row
result only describes the inspected database.

The executed query identifies a Pool when it has more than one member
`question_type`, more than one `backend`, or more member rows than distinct
`published_question_id` values. For every affected Pool it returns every
member pin, then each Assessment entry using the Pool with `selection_count`
and `points_per_item`, together with that Assessment's active-entry baseline:

```sql
WITH pool_member AS (
    SELECT member.question_pool_id, member.member_position,
           member.published_question_id, member.question_revision_number,
           revision.question_type, revision.backend
      FROM ple_data.question_pool_member AS member
      JOIN ple_data.question_revision AS revision
        ON revision.published_question_id = member.published_question_id
       AND revision.revision_number = member.question_revision_number
), affected_pool AS (
    SELECT pool.question_pool_id, pool.title,
           count(DISTINCT member.question_type) AS type_count,
           count(DISTINCT member.backend) AS backend_count,
           count(*) - count(DISTINCT member.published_question_id) AS duplicate_question_pins
      FROM ple_data.question_pool AS pool
      JOIN pool_member AS member ON member.question_pool_id = pool.question_pool_id
     GROUP BY pool.question_pool_id, pool.title
    HAVING count(DISTINCT member.question_type) > 1
        OR count(DISTINCT member.backend) > 1
        OR count(*) > count(DISTINCT member.published_question_id)
)
SELECT affected.*, entry.assessment_id, entry.assessment_entry_id,
       pool_entry.selection_count, pool_entry.points_per_item
  FROM affected_pool AS affected
  LEFT JOIN ple_data.assessment_entry_pool AS pool_entry
    ON pool_entry.question_pool_id = affected.question_pool_id
  LEFT JOIN ple_data.assessment_entry AS entry
    ON entry.assessment_entry_id = pool_entry.assessment_entry_id;
```

The full helper includes the complete baseline CTE. For each Assessment it
computes `Questions per Attempt = fixed entries + sum(pool selection_count)`
and `total points = fixed points + sum(pool selection_count * points_per_item)`
over available entries. The M8 worker must save this output before repair and
compare it with the same query afterward for every now-affected Assessment.

## Static fixture candidates (not executed Live Demo evidence)

Only two current fixture owners directly insert `question_pool_member` rows:

| Fixture owner | Pool shape now | M8 action if Pool Type/Backend become required |
| --- | --- | --- |
| `crates/learning-data-access/tests/blueprint_course_postgres/support.rs` | `QUESTION_POOL` receives one `QUESTION`, revision 1. The fixture creates that revision as `ple` / `multipleChoice`. | Populate the new Pool Type and Backend from its first member in the Pool insert. No mixed-member repair is indicated. |
| `crates/learning-data-access/tests/blueprint_course_postgres/discovery.rs` | 250 discovery Pools each receive the same one `QUESTION`, revision 1 from the shared support fixture. | Populate the new Pool Type and Backend from that first member in the bulk Pool insert. No mixed-member repair is indicated. |

These are static source candidates, not a claim that their connected fixture
database was executed during this audit. The current schema keeps the member
pin unique only as `(question_pool_id, published_question_id,
question_revision_number)` in
`schemas/base_schema/20_tables/question_pool.sql`; it still permits two
Revisions of one Question. The two fixture owners have one member each and
therefore do not presently exercise that loophole.

Repeat the source inventory after M8 changes with:

```sh
rg -n "INSERT INTO ple_data\\.question_pool_member|question_pool_member \\(" \
  --glob '*.{sql,rs,ts,tsx,mjs,py,sh}'
```

## Required repair plan when the audit finds a defect

1. For duplicate revisions of one Question, retain the exact revision currently
   delivered by the affected Assessment; remove the other pin. If no Assessment
   uses the Pool, retain the newest valid revision after confirming the Pool's
   learning target.
2. For mixed Type or Backend membership, keep the original Pool focused on its
   existing learning target and move each off-rule member into a new Pool with
   the same Discipline and Subject. Do not silently discard a usable Question.
3. If an affected Assessment entry's `selection_count` exceeds the remaining
   original Pool size, split that entry across the resulting compatible Pools.
   The split selection counts must sum to the original selection count and each
   entry retains the original `points_per_item`. Record the changed selection
   chances explicitly.
4. Capture the baseline query output before mutation and rerun it afterward.
   Each affected Assessment must retain identical Questions per Attempt and
   total points. Review the changed fixture against the behavior it was written
   to test.

M8 will additionally make `(question_pool_id, published_question_id)` unique,
store Pool Type and Backend from the first member, and enforce matching values
on creation, member replacement, ordinary fork, and Course-adoption fork. The
current report deliberately does not claim those pending changes are present.

## Implementation handoff (planned)

This is a file-and-function map for the M8 backend owner. It describes planned
work only; the audit above is the completed evidence.

| Responsibility | Planned location and smallest change |
| --- | --- |
| Persist the Pool invariant | `schemas/base_schema/20_tables/question_pool.sql`: add required `question_type` and `backend` columns to `ple_data.question_pool`; replace the member uniqueness rule with `UNIQUE (question_pool_id, published_question_id)`. Existing `ple_data.question_type` and `ple_data.question_backend` enums are defined in `schemas/base_schema/10_types.sql`. |
| Centralize admission | `schemas/base_schema/50_functions/question_pools.sql`, `ple_data.validate_question_pool_member_insert`: it already resolves the exact Question Revision for every insert. Compare the revision's Type and Backend to the stored Pool pair here, so every writer receives the same rejection. `ple_data.validate_question_pool_lineage_update` should make the pair immutable with the existing Pool identity fields. |
| Create and replace members | In the same file, `ple_data.create_question_pool` stores the first member's Type/Backend; `ple_data.save_question_pool_members` continues to retain that pair while its delete/reinsert path is protected by the centralized trigger. No separate API parameter is needed. |
| Preserve Pool pair on forks | `ple_data.construct_question_pool_fork` copies the source pair. Its ordinary and Course-adoption wrappers (`ple_data.fork_question_pool`, `ple_data.fork_question_pool_for_course_adoption`) then inherit the invariant. Import and append paths in `assessment_pool_forks.sql`, `blueprint_pools.sql`, and `course_blueprint_adoption.sql` already use those functions or `save_question_pool_members`; they should not duplicate validation. |
| Keep read contracts aligned | Thread the pair through the Pool readers in `question_pools.sql`, the model records in `crates/question_model/src/question_pool_library.rs`, LDA records and PostgreSQL decoding in `crates/learning-data-access/src/{question_pool_library.rs,postgres/question_pool_library.rs}`, and server responses in `crates/server/src/question_pool_library.rs`; regenerate the API and strictly decode the added fields in `src/api/decoders/question_pool_library.ts` and `src/api/decoders/assessment_pool_fork.ts`. |

The two direct fixture writers identified by the audit need the same required
pair in their Pool insert: `support.rs` inserts a `ple` / `multipleChoice`
Question revision, and `discovery.rs` creates its 250 one-member Pools from
that revision. Neither has mixed membership or duplicate Question revisions to
repair; populate the pair from the first member rather than inventing fixture
metadata.

Focused preservation checks should prove rejection of a second Revision of the
same Question, a mismatched Type, and a mismatched Backend through Pool create
and member-save paths. They should also prove the first-member pair remains
unchanged and is copied by ordinary, Course-adoption, Blueprint, and Assessment
forks. Use the audit helper's before/after Assessment baseline to confirm
Questions per Attempt and total points remain unchanged for every repaired
live-data Assessment.

## Before-M8 Live Demo refresh (2026-10-05)

Executed the owned read-only probe against the already-running Live Demo at
this milestone. It locates the running PostgreSQL container through the local
stack controller, starts `BEGIN READ ONLY`, assumes `ple_data_owner` only
inside that transaction, and rolls back. It did not rebuild, seed, migrate, or
change the running stack.

```sh
python3 tests/_temp/audit_shared_search_pool_membership.py
```

Observed result:

| Measure | Value |
| --- | ---: |
| Pools | 0 |
| Pool members | 0 |
| Pools with mixed Type, mixed Backend, or two Revisions of one Question | 0 |
| Assessment entries referring to an affected Pool | 0 |
| Affected Assessment baselines to preserve | 0 |

This is executed pre-M8 Live Demo evidence, not a claim about static fixtures
or the M8 source changes now in progress. With zero Pools and zero members,
there is no live Pool content to repair at this point; rerun the same probe
after the M8 worker's rebuild before treating the implementation as complete.

## M8 fixture and oracle review (source-only)

Reviewed the stable M8 fixture and SQL-oracle source without rebuilding or
altering the Live Demo. The two direct fixture writers remain aligned with
their original test purposes:

| Source | Verified finding |
| --- | --- |
| `crates/learning-data-access/tests/blueprint_course_postgres/support.rs` | Its one-member Pool now derives required `question_type` and `backend` from the exact seeded Question revision 1. The fixture still represents one `ple` / `multipleChoice` Question, so it needs no content repair. |
| `crates/learning-data-access/tests/blueprint_course_postgres/discovery.rs` | Each of the 250 discovery Pools derives the same pair from the shared Question revision 1 before adding that sole member. The bulk discovery cardinality and one-member search purpose remain intact. |
| `tests/e2e/assessment_saved_response/03_course_pool_forks.sql` | The new admission fixtures add distinct Questions only to exercise the three M8 rejections: two Revisions of one Question, mismatched Type, and mismatched Backend. It checks these through Pool creation and member replacement, then confirms the Assessment-owned fork copies the source Type/Backend pair while retaining the original course-Pool assertions. |

The review found no fixture Pool with mixed Type, mixed Backend, or duplicate
Question membership that needs a pedagogical content move. The negative
Questions are isolated oracle inputs, not fixture-content repairs. This is a
source review result; the after-rebuild live evidence follows.

## After-M8 rebuilt Live Demo audit (2026-10-05)

After the M8 Live Demo rebuild and Library capture, reviewed the manager's
read-only probe receipt at `/tmp/ple_shared_search_m8_after_audit.log` and
reran the owned probe independently. Both returned the same result as the
before-M8 baseline:

| Measure | Before M8 | After rebuilt Live Demo |
| --- | ---: | ---: |
| Pools | 0 | 0 |
| Pool members | 0 | 0 |
| Affected Pools | 0 | 0 |
| Assessment entries referring to an affected Pool | 0 | 0 |
| Affected Assessment Questions-per-Attempt / total-points baselines | 0 | 0 |

There were no live Pools or Assessment entries before the repair, and there
are none afterward. Therefore no content movement, selection-count split, or
Assessment total preservation comparison was required; the count of affected
Assessment totals is unchanged at zero.

**Final M8 audit verdict:** the rebuilt Live Demo contains no mixed-Type,
mixed-Backend, or duplicate-Question Pool membership. The direct fixture
writers and their admission/fork oracle preserve their stated purposes, and
the required live-data repair set is empty.
