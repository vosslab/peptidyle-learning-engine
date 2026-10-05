# Shared search speed evidence

## M12 mixed Library replay

The rebuilt M12 Library passed its real API and browser journeys on port 8314. The same four
read-only HTTP scenarios used 3 warmups and 15 measurements against the same 49 Questions,
one verification Pool, and one member. Raw output: `/tmp/ple_shared_search_speed_m12.json`.

| Case | M12 median ms | M12 p90 ms | Original baseline p90 ms |
| --- | ---: | ---: | ---: |
| Both, Questions in no Pool | 23.983 | 25.187 | 1339.4 |
| Text `genetics` | 17.745 | 21.915 | 1478.6 |
| Biology Discipline | 19.255 | 19.769 | 1132.6 |
| Questions only, All Questions, 50 rows | 18.538 | 19.193 | 1074.1 |

Every median remains below the original baseline p90. No further query or indexing change was
indicated. These are local small-fixture measurements, not production-scale throughput claims.

## M10 regression and correction

The first rebuilt M10 Live Demo contained 49 Questions and one verification Pool with one
member. The mixed default therefore returns 48 Questions plus the Pool. The regenerated Biology
UUID differs from the baseline, while its named Discipline and seeded content remain equivalent.
The probe now explicitly requests `kind=both&membership=noPool` for the first three scenarios
and `kind=questions&membership=all` for All Questions. It retains 3 warmups and 15 timed requests.

| Case | First M10 median ms | Baseline p90 ms |
| --- | ---: | ---: |
| Default | 1734.8 | 1339.4 |
| Text word | 1609.1 | 1478.6 |
| One Discipline | 1593.0 | 1132.6 |
| All Questions, 50 rows | 1605.3 | 1074.1 |

Raw HTTP output is `/tmp/ple_shared_search_speed_m10.json`. The required regression investigation
captured `/tmp/ple_shared_search_explain_m10/default-wrapper.txt` and `default-body.txt`.
The wrapper took 1506.8 ms; the nested query took 1490.1 ms, of which 1464.7 ms compiled 329 JIT
functions. A read-only transaction with `SET LOCAL jit = off` reduced the same wrapper to
22.0 ms with comparable buffer activity (6956 versus 6962 shared hits); its plan is
`default-no-jit.txt` in that directory.

The private mixed-search function now sets `jit = off` locally. PostgreSQL restores a function's
SET parameters on exit; this avoids compilation cost for the interactive search and its nested
reads without changing cluster or caller settings. See PostgreSQL's
[JIT decision guidance](https://www.postgresql.org/docs/18/jit-decision.html) and
[function SET clause](https://www.postgresql.org/docs/18/sql-createfunction.html).
Independent review approved the scoped correction. The same complete connected matrix passed
before and after it (81.39 seconds versus 3.39 seconds). The final current-source Live Demo
rebuild, API and browser smoke checks, and HTTP replay pass. The verification Pool was recreated
through its public API after the rebuild, preserving the same 49 Questions/one Pool/one member
dataset. Final raw output is `/tmp/ple_shared_search_speed_m10_final.json`.

| Case | Final M10 median ms | Final M10 p90 ms | Baseline p90 ms |
| --- | ---: | ---: | ---: |
| Default | 17.0 | 20.4 | 1339.4 |
| Text word | 15.6 | 16.0 | 1478.6 |
| One Discipline | 17.4 | 18.8 | 1132.6 |
| All Questions, 50 rows | 17.6 | 18.1 | 1074.1 |

All final medians pass the plan's regression threshold. The final installed function's EXPLAIN
returns 49 rows in 22.3 ms with 6956 shared hits, using normal caller settings:
`/tmp/ple_shared_search_explain_m10/default-final-function.txt`. These are local pilot-sized
measurements, not a claim about large production datasets. M12 requires another replay.

Measured on 2026-10-04 against the ready `ple-live-demo-browser` stack, before M10 changes
the Library server query. The default `containers` project was absent. This is a read-only
baseline: it created only an in-memory Live Demo Instructor session.

Run it again with:

```sh
source ./source_me.sh && python3 tests/_temp/shared_search_speed_baseline.py
```

The probe selects the first active Discipline by case-insensitive name so it remains usable as
the Live Demo vocabulary changes. This run selected `Biology`
(`84799742-4241-4515-ba09-596442424017`). It uses the exact word `genetics` for the text case.

Each case made three unmeasured warmups, then 15 timed `GET` requests as `elenaInstructor`.
The session cookie lived only in the Python process. A timed value starts immediately before
the direct IPv4 HTTPS request to the fixed gateway and ends after the complete JSON response
body is read and decoded. It therefore includes loopback connection/TLS and gateway/API/database
work, but does not include browser rendering, session setup, classification lookup, Podman CLI,
or subprocess startup.

| Case | Exact endpoint path | Median ms | Mean ms | p90 ms | Min-max ms |
| --- | --- | ---: | ---: | ---: | ---: |
| Default | `/api/questions/search?page_size=50` | 1060.9 | 1065.2 | 1089.1 | 1048.0-1091.9 |
| Text word | `/api/questions/search?page_size=50&text=genetics` | 1115.0 | 1123.3 | 1179.6 | 1050.5-1293.6 |
| One Discipline | `/api/questions/search?page_size=50&discipline_uuid=84799742-4241-4515-ba09-596442424017` | 1253.5 | 1295.3 | 1410.9 | 1089.5-2058.0 |
| All Questions, 50 rows | `/api/questions/search?page_size=50` | 1061.8 | 1066.9 | 1151.4 | 1007.5-1177.5 |

## Baseline semantics

Before M10, the endpoint returns Questions only and has no result-kind or Pool-membership
parameter. Therefore the requested pre-M10 **All Questions at 50 rows** case deliberately uses
the same current endpoint path and semantics as the default case. It is retained as its own
measurement so the post-M10 `membership=all` behavior can be compared with this baseline instead
of being mistaken for an already-existing server filter.

The direct gateway timing is suitable for detecting a meaningful post-M10 endpoint slowdown.
Its common roughly one-second floor is current end-to-end endpoint behavior, not `podman exec`
overhead.

## M10 implementation handoff (planned)

This section is a source map for the M10 owner. It describes planned changes only; it does not
change the measured baseline above.

- **One query boundary:** extend the private/public
  `search_question_library_entries` pair in
  `schemas/base_schema/50_functions/question_library_operations.sql`, then its typed binding in
  `crates/learning-data-access/src/postgres/question_library/search.rs`. Build Question and Pool
  rows with `UNION ALL`, apply kind-specific predicates before the union, and sort/page the union
  once. Do not combine the separately paged Pool discovery endpoint with Question results.
- **NULL-kind facets sentinel:** the current SQL returns one NULL `published_question_id` row to
  carry facets when a page is empty, and Rust discards it by testing that ID. A mixed result needs
  an explicit nullable row kind/sentinel so a real Pool row is never discarded.
- **Global cursor:** widen the request/page/cursor types in
  `crates/learning-data-access/src/question_library.rs`. Continue binding every filter into the
  opaque cursor, including kind and membership. Sort by title then public ID, or newest timestamp
  then public ID; the shared public-ID reservation makes that final key complete. Pool newest is
  `created_at`.
- **HTTP and decoding:** retain parser normalization in
  `crates/server/src/question_library/query.rs` and route wiring in
  `crates/server/src/question_library.rs`; extend the client allowlist in
  `src/api/question_search_query.ts`. Replace the question-only generated page/result contract
  with a strict tagged mixed-row decoder in `src/api/decoders/question_library.ts` and mapping in
  `src/api/question_library_repository.ts`. Question source rendering/statistics must remain on
  the Question branch only; Pool rows do not pass through `entries_to_summaries`.
- **Predicates:** Question membership is an `EXISTS` check over every Pool member, including
  Assessment forks. Author, used-in-my-courses, capability, and authored-by-current-account
  apply only to Questions. Pool filters use only Pool-owned metadata; a member text or Tag match
  returns that Question under All Questions and never makes its Pool match.
- **Tests:** the connected PostgreSQL oracle is
  `crates/learning-data-access/tests/blueprint_course_postgres/question_library.rs`; transport,
  cursor, and kind lookup tests belong in `crates/server/src/question_library/tests.rs`; strict
  client/fixture coverage belongs in `tests/test_library_classification_search.mjs`. Add the M10
  mixed-kind, membership, owner, filter, and tied-page cases there, then compare the rebuilt Live
  Demo with this report's speed probe.

## Manager baseline replay

The manager reran the full command before M10 with the same queries, 3 warmups, and 15 timed
requests per case. These are the baseline thresholds to use for the plan's post-M10 comparison;
the earlier independent run above also records the variation on this shared development host.
Raw replay output: `/tmp/ple_shared_search_speed_manager.json`.

| Case | Median ms | p90 ms |
| --- | ---: | ---: |
| Default | 1256.6 | 1339.4 |
| Text word | 1248.9 | 1478.6 |
| One Discipline | 1081.7 | 1132.6 |
| All Questions, 50 rows | 1051.4 | 1074.1 |

This replay used the pre-M10 Live Demo. Concurrent local builds and browser checks
may contribute to timing variation; investigate a threshold regression with query-plan evidence
as specified by the plan rather than attributing it to SQL from wall-clock time alone.

## Pre-M10 PostgreSQL plan capture

This is the required before-change `EXPLAIN (ANALYZE, BUFFERS, VERBOSE, SETTINGS)` evidence for
the current `ple_api.search_question_library_entries` function. It was captured read-only on
2026-10-05 from the ready `ple-live-demo-browser` PostgreSQL 18.6 (Debian 18.6-1.pgdg13+2)
container. Each call installed a fresh `elenaInstructor` session in one transaction, switched to
the ordinary `ple_app` role, ran the function, and rolled the transaction back. The opaque
session token/hash remained in the capture process and psql stdin; raw plans contain no
credentials.

| Case | Function output rows | Shared buffers hit | Execution ms |
| --- | ---: | ---: | ---: |
| Default, 50 rows | 49 | 6701 | 1047.045 |
| Text `genetics`, 50 rows | 34 | 6692 | 1076.563 |
| Biology Discipline, 50 rows | 46 | 6701 | 105.409 |

The planned pre-M10 **All Questions** case has the identical current function arguments and SQL
shape as Default, so it reuses the Default capture. Raw plan files are retained at
`/tmp/ple_shared_search_explain_pre_m10/{default,text_genetics,discipline}.txt`.

The Live Demo database contained 49 Published Question lineages, 49 accepted revisions, zero
Pools, zero Pool members, and was 25 MB. This is a small Question-only baseline, appropriate for
the before/after function-cost comparison but not evidence of mixed-Pool selectivity.

The top-level PostgreSQL plan is necessarily a `Function Scan`: the existing search body is a
PL/pgSQL function, so safe caller-side EXPLAIN does not reveal its nested CTE/index nodes. The
capture still records actual returned rows, total execution time, and all shared-buffer activity
for the exact authorized production function call. After M10, rerun the same command and compare
these values; if a rewrite needs index decisions, capture an additional isolated plan for the
new static union body rather than inferring nested nodes from the function scan.
