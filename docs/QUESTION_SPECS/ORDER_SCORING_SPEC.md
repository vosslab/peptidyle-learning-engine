# ORDER partial credit

## Scope and authority

This rule currently applies to Native JSON. Another Question Backend may reuse it by
explicitly adopting this scoring model; each Backend owns its grading rules.

This model calculates the earned credit fraction for Native JSON ORDER Questions. PLE stores
that fraction whether Assessment partial credit is on or off. The Assessment setting controls
awarded points: use the fraction when enabled; otherwise award full credit only for a fraction
of one and zero for all smaller fractions. Assessment Instructors can change that setting for
all Attempts using the stored fractions. Each Question Backend grades its own Questions.
See [QUESTION_BACKEND_SPEC.md](QUESTION_BACKEND_SPEC.md#stored-credit-and-awarded-points).

Neil supplied this scoring decision on October 5, 2026.
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md) owns product intent.
[NATIVE_JSON_SPEC.md](NATIVE_JSON_SPEC.md) owns the source and response formats, including stable
item IDs and a submitted order containing every item exactly once.

## Scoring goals

ORDER scoring rewards two kinds of knowledge equally:

1. Placing items in their correct absolute positions.
2. Preserving the correct relative order among items.

Absolute-position scoring alone is too harsh for shifted but mostly correct sequences.
Relative-order scoring alone gives random permutations an expected score of 50%.
The average of the two gives credit for both kinds of knowledge.

## Definitions and formula

| Symbol | Meaning |
| --- | --- |
| `n` | Number of items |
| `A` | Absolute-position score |
| `R` | Relative-order score |
| `S` | Final credit fraction |

The absolute-position score is the fraction of items in their exact keyed positions:

```text
A = correct_position_count / n
```

Every distinct pair of items contributes to the relative-order score. A pair is correct when
its two items appear in the same relative order as in the answer key:

```text
pair_count = n * (n - 1) / 2
R = correctly_ordered_pair_count / pair_count
```

The two scores receive equal weight:

```text
S = (A + R) / 2
```

The result stays between zero and one. PLE applies the Assessment partial-credit setting and
point value to this stored credit. Multiply by 100 for display as a percentage; round only for display.

## Worked example

For the answer key `ABCD` and response `DABC`, no item has its correct absolute position, so
`A = 0 / 4 = 0`. The six distinct pairs are `AB`, `AC`, `AD`, `BC`, `BD`, and `CD`:

| Pair | Correct order | Student order | Correct? |
| --- | --- | --- | --- |
| A, B | A before B | A before B | Yes |
| A, C | A before C | A before C | Yes |
| A, D | A before D | D before A | No |
| B, C | B before C | B before C | Yes |
| B, D | B before D | D before B | No |
| C, D | C before D | D before C | No |

Therefore:

```text
R = 3 / 6 = 0.5
S = (0 + 0.5) / 2 = 0.25 = 25%
```

The response earns credit because A, B, and C remain in the correct relative order. Every item
being in the wrong absolute position reduces the final score.

## Counting correctly ordered pairs

Map each stable item ID to its position in the submitted order. For every pair of items where
the first precedes the second in the answer key, test:

```text
student_position[first_item_id] < student_position[second_item_id]
```

Count each true comparison as one correctly ordered pair. Equivalently:

```text
correctly_ordered_pair_count = pair_count - inversion_count
```

An inversion is a pair whose relative order is reversed from the answer key. `DABC` has three
inversions against `ABCD`, so it has `6 - 3 = 3` correctly ordered pairs. Compare stable item IDs
against the answer key; displayed labels and the initial shuffled order do not define correctness.

## Behavior across Question sizes

One-time exact-arithmetic checks evaluated all 5,910 permutations for three through seven items
and reproduced the supplied table. These are checks of the specified formula, not runtime tests.
The range is the supplied validation example, not an approved minimum or maximum item count.
Native JSON's current three-item minimum is implementation evidence in
[NATIVE_JSON_SPEC.md](NATIVE_JSON_SPEC.md#current-implementation-limits).
The wrong-permutation columns exclude the perfect answer. The single-pair-swap mean covers every
possible swap of two items from the perfect answer, including adjacent and distant swaps.

| Items | Wrong permutations | Mean wrong score | Mean single-pair swap | Wrong answers <50% | Wrong answers <=50% |
| --- | --- | --- | --- | --- | --- |
| 3 | 5 | 30.0% | 38.9% | 60.0% | 100% |
| 4 | 23 | 34.8% | 55.6% | 78.3% | 87.0% |
| 5 | 119 | 34.5% | 65.0% | 78.2% | 87.4% |
| 6 | 719 | 33.2% | 71.1% | 85.5% | 89.7% |
| 7 | 5039 | 32.1% | 75.4% | 89.5% | 92.6% |

Across these sizes, average wrong-answer credit stays near one-third, and most wrong answers
score below 50%. For uniformly random permutations including the perfect answer, the expected
score is `1 / (2 * n) + 1 / 4`; this approaches 25% as the number of items grows.

## Single-pair swaps

An adjacent swap disrupts two absolute positions and one relative-order pair:

| Items | Adjacent-swap score |
| --- | --- |
| 3 | 50.0% |
| 4 | 66.7% |
| 5 | 75.0% |
| 6 | 80.0% |
| 7 | 83.3% |

As the Question grows, a neighboring swap changes a smaller share of the total ordering
information. More distant swaps disrupt more relative-order relationships. For seven items:

| Swap distance | Score |
| --- | --- |
| 1 position | 83.3% |
| 2 positions | 78.6% |
| 3 positions | 73.8% |
| 4 positions | 69.0% |
| 5 positions | 64.3% |
| 6 positions (first and last) | 59.5% |

For a swap at distance `d` from a perfect answer, `n - 2` positions stay correct and `2 * d - 1`
pairs become inverted. The score follows directly from the same formula.

## Required properties

- Perfect order earns 100%; every other valid permutation earns less.
- Credit stays between zero and one.
- Correct absolute positions and correctly ordered pairs each contribute half of the score.
- Correct relative order earns credit even when absolute positions are wrong.
- Starting from the perfect answer, a local swap earns more than a more distant swap.
- A single adjacent swap earns proportionally more credit as the item count grows.
- Scoring depends on the submitted order and answer key, using stable item IDs.
- Renaming labels or changing initial presentation order preserves the score for the same response.

A fully reversed order has zero correctly ordered pairs. With an even item count it earns zero.
With an odd item count, its middle item stays in the correct position and earns `1 / (2 * n)`:
16.7% for three items, 10% for five, and 7.1% for seven.

## Rationale and implementation follow-up

Absolute position measures whether each item is exactly where it belongs. Relative order measures
whether the Student understands which items precede or follow others. Equal weighting rewards
small ordering errors while reducing credit for broadly incorrect sequences, using the same rule
across Question sizes.

The current Native JSON grader compares the complete order and returns only zero or one.
Implementing this settled partial-credit rule belongs in
[TODO.md](../TODO.md#question-spec-implementation-follow-up). Verify the example, permutation and
swap tables, required properties, and the awarded credit through saved and displayed scores.

New Assessments start with partial credit enabled. Later changes apply to all Attempts
using the stored fractions. The decision and its reason are recorded as Q20 in
[question_specs_open_questions.md](../active_plans/decisions/question_specs_open_questions.md).
