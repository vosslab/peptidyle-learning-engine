# Bloom integration evidence

## Implementation checks

The API queues successful Question publication without awaiting JEV. It uses BPW rubric version 2
and `jev-1.14.0`, sends both Choice questions together, and validates and maps both answers before
one database save. No publication route requires JEV credentials. Pool creation and membership
edits queue a local mode calculation; forks retain their copied metadata.

Native JSON evidence retains prompt, choices, correct IDs, matching mappings, numeric answers and
tolerances through the existing typed document. Feedback, hints and browser scripts are omitted.
WeBWorK uses the existing exact-source answer-review renderer at seed 0. The text reader retains
MathJax TeX, table cell boundaries, image alternative text and answer-control values, while dropping
executable scripts and styles. Visual-only tasks retain BPW's approximate-text limitation.

## One-time live comparison

On 2026-10-19, PLE's actual Rust JEV client classified the Native pilot
`genetics-disorders-mc.json` as Remember / Factual Knowledge in 262 ms, and
`genetics-disorders-matching.json` as Remember / Factual Knowledge in 222 ms. Both passed response
validation. The temporary probe was removed after use; no credentials or answer-bearing request
logs were saved in the repository.

BPW's existing client and readable evidence extraction classified comparable published questions
from `site_docs/genetics/topic01/bbq-WOMC-genetic_disorders-questions.txt` line 26 and
`bbq-MATCH-genetic_disorders-questions.txt` line 14 as Remember / Factual. These assess the same
phenylketonuria recognition and genetic-disorder matching tasks as PLE's examples; wording and
distractors differ. The matching labels corroborate the adaptation, not a taxonomy accuracy claim.

## Durable checks

Focused tests cover the JEV label/distribution contract and PLE enum mapping, Native task/answer
projection, and WeBWorK-style HTML/math/answer preservation. The disposable PostgreSQL baseline
includes atomic Question pair saves, stale edit rejection, independent Pool modes, NULL exclusion,
deterministic ties, and recalculation after membership edits.

The final `source source_me.sh && ./launchers/all_test.sh` run passed on 2026-10-19,
including Rust and TypeScript gates, 9431 Python tests, and the disposable PostgreSQL authority
and persistence checks. Independent reviews assessed evidence fidelity and integration boundaries.
WeBWorK evidence extraction has focused test coverage; this run did not include a live WeBWorK
renderer classification probe.

## Intentional limits

The queue is in memory. Full queues, service failure, evidence failure or restart may leave Bloom
blank. There is no recurring sweep or retry state. Previously assigned Question classifications
follow existing revision inheritance. Pool instructor corrections remain until a subsequent
membership edit recalculates the modes. No new schema columns or browser API fields are introduced.
