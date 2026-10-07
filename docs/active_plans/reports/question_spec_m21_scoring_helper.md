# M21 ORDER scoring helper preparation

## Scope and interface

Added a dependency-free scoring helper in `crates/grading/src/ple_question_json/ordering.rs`.
Its interface is `ordering_credit_fraction(answer_key_ranks: &[usize]) -> f64`. The caller maps
stable response item IDs to zero-based positions in the answer key, then passes the submitted
order as those ranks. This keeps the helper independent of display labels, presentation indices,
database schema, and Assessment partial-credit policy.

The helper assumes its input is a validated permutation of `0..n`. It does not validate item IDs,
duplicate ranks, missing ranks, or size. Current Native JSON source validation requires at least
three Ordering items (`source_compile.rs`); the grading response-format validation also requires
three (`ple_question_json_validate.rs`). Existing domain response validation checks an exact
permutation against the response format. The later grading-orchestrator change should preserve
those validation seams and map IDs to answer-key ranks there. This helper is preparatory until M17's
Assessment scoring setting and the Native JSON evaluation orchestrator are integrated.

## Formula and evidence

For `n` answer-key ranks, the helper averages the fraction of ranks in their matching absolute
positions and the fraction of position pairs whose ranks remain increasing. This implements the
equal-weight position and pair rule in `ORDER_SCORING_SPEC.md`.

Focused cases cover the `DABC` against `ABCD` example (0.25), perfect order, even and odd
reversals, adjacent swaps for three through seven items, and every swap distance for seven items.
An exhaustive property case checks all 5,910 permutations across three through seven items for
bounded scores and confirms only the perfect permutation receives full credit.

I independently recalculated the spec's summary table by enumerating every permutation for each
size. The wrong-permutation counts, mean scores, single-swap means, and below/equal-50% counts all
match the quoted table. The seven-item swap-distance results also match: 83.3%, 78.6%, 73.8%,
69.0%, 64.3%, and 59.5%. No scoring example discrepancy was found.

## Validation and handoff

Standalone command:

```sh
rustc --edition=2021 --test crates/grading/src/ple_question_json/ordering.rs -o /tmp/ple_ordering_tests
/tmp/ple_ordering_tests
```

Result: 4 tests passed, including the exhaustive 5,910-permutation case. No Cargo module
registration or runtime integration was made; those remain with the M21 integration owner after
M17 is ready. The integration handoff should call this helper only after existing complete-order
validation and stable-ID rank mapping, then verify stored fraction and displayed points through the
saved-response path.

## Open question

Should the eventual orchestrator map each submitted stable ID to its answer-key rank and call this
helper, or should the helper accept both validated ID sequences and do the mapping itself? The
current single-slice rank interface is smaller and keeps ID lookup with existing grading
validation, but the integration owner should confirm that it fits the actual composition boundary.
