# Multiple Answer scoring

## Scope and authority

This rule currently applies to Native JSON. Another Question Backend may reuse it by
explicitly adopting this scoring model; each Backend owns its grading rules.

This model calculates the earned credit fraction for Native JSON Multiple Answer Questions. PLE stores
that fraction whether Assessment partial credit is on or off. The Assessment setting controls
awarded points: use the fraction when enabled; otherwise award full credit only for a fraction
of one and zero for all smaller fractions. Assessment Instructors can change that setting for
all Attempts using the stored fractions. Each Question Backend grades its own Questions.
See [QUESTION_BACKEND_SPEC.md](QUESTION_BACKEND_SPEC.md#stored-credit-and-awarded-points).

Neil selected the linear choice-count score as the durable default on October 5, 2026.
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md) owns product intent.
[NATIVE_JSON_SPEC.md](NATIVE_JSON_SPEC.md) owns the source format, and
[QUESTION_BACKEND_SPEC.md](QUESTION_BACKEND_SPEC.md) defines Backend responsibilities.

## Scoring goals

The model rewards identifying correct choices and selecting an appropriate number of choices.
Selecting too few or too many choices reduces credit. Selecting exactly the number of correct
choices receives the full choice-count score.

Each correct selection earns an equal share of the base credit. Each incorrect selection
cancels one correct selection. The final score combines this accuracy with the choice-count score.

## Definitions

| Symbol | Meaning |
| --- | --- |
| `N` | Total number of choices |
| `C` | Number of correct choices, at least one |
| `TP` | Correct choices selected |
| `FP` | Incorrect choices selected |
| `K` | Total choices selected: `TP + FP` |
| `B` | Base score |
| `R` | Choice-count score, also called the cardinality score |
| `S` | Final credit fraction |

Counts refer to distinct choices. Choice order has no effect on the calculation.

## Base score

Each correct selection earns `1 / C`. Each incorrect selection cancels one correct selection:

```text
B = max(0, (TP - FP) / C)
```

## Choice-count score

The choice-count score measures how closely the number selected matches the number correct:

```text
R = min(K, C) / max(K, C)
```

- Selecting exactly `C` choices gives `R = 1`.
- Selecting fewer than `C` choices gives `R < 1`.
- Selecting more than `C` choices gives `R < 1`.

Use this linear ratio directly as the multiplier.

## Final score

For an empty selection, final credit is zero. Otherwise:

```text
S = max(0, (TP - FP) / C) * min(TP + FP, C) / max(TP + FP, C)
```

Equivalent calculation with descriptive names:

```text
selected_count = correct_selected_count + incorrect_selected_count
base_credit = max(0, (correct_selected_count - incorrect_selected_count) / correct_choice_count)
choice_count_score = min(selected_count, correct_choice_count) / max(selected_count, correct_choice_count)
credit = base_credit * choice_count_score
```

Credit stays between zero and one. PLE applies the Assessment partial-credit setting and point value to the stored credit.
Multiply by 100 for a percentage. Calculate credit before display rounding.

## Worked examples

### Two correct of ten

| Student selection | Credit |
| --- | --- |
| 1 correct | 25% |
| 1 correct + 1 incorrect | 0% |
| 2 correct | 100% |
| 2 correct + 1 incorrect | 33.3% |
| All ten choices | 0% |

### Five correct of ten

| Student selection | Credit |
| --- | --- |
| 3 correct + 1 incorrect | 32% |
| 4 correct | 64% |
| 4 correct + 1 incorrect | 60% |
| 4 correct + 2 incorrect | 33.3% |
| 5 correct | 100% |
| 5 correct + 1 incorrect | 66.7% |
| All ten choices | 0% |

### Eight correct of ten

| Student selection | Credit |
| --- | --- |
| 6 correct | 56.25% |
| 6 correct + 1 incorrect | 54.7% |
| 6 correct + 2 incorrect | 50% |
| 7 correct | 76.6% |
| 7 correct + 1 incorrect | 75% |
| 8 correct | 100% |
| 8 correct + 1 incorrect | 77.8% |
| 8 correct + 2 incorrect (all ten) | 60% |

## Scoring principles and rationale

The number of choices selected is part of the response. Different sets of selected choices are
independent response patterns, judged by both accuracy and choice count. Neil explicitly treats
that combination as the intended teaching model.

A Student who selects exactly `C` choices demonstrates knowledge about both which choices are
correct and how many belong in the correct set. Too few or too many selections reduce credit.
For example, one correct choice out of two earns half the base credit and half the choice-count
score, yielding 25%.

Neil chose the linear multiplier as the middle ground among these alternatives:

| Choice-count multiplier | Neil's assessment |
| --- | --- |
| Square root of `R` | Too lenient |
| `R` | Durable default |
| `R` squared | Too punitive |

Selecting everything earns `max(0, (2 * C - N) / N)`. This gives 60% for eight correct out of ten
and 80% for nine correct out of ten. These are consequences of the chosen scoring model.
If every choice is correct, selecting everything is perfect selection and earns full credit.

## Validation and follow-up

Validate the supplied examples, empty selections, full credit for perfect selection, bounds from
zero to one, and independence from choice ordering. Adding an incorrect selection must preserve
or reduce credit. Use the complete formula, including the choice-count score on both sides of
`K = C`.

New Assessments start with partial credit enabled. The formula and applying the current
setting to all Attempts using stored fractions are settled.
Application work belongs in [TODO.md](../TODO.md#question-spec-implementation-follow-up).
The decision history is in
[question_specs_open_questions.md](../active_plans/decisions/question_specs_open_questions.md).
