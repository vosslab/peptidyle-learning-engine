# M20 Multiple Answer Scoring Helper

## Scope

This report records independent preparation for M20. The pure Rust helper is in
`crates/grading/src/ple_question_json/multiple_answer.rs` and accepts validated
`TP`, `FP`, and `C` counts. It derives `K = TP + FP` so callers cannot pass an
inconsistent selected count. It returns the Native JSON stored credit fraction:

```text
max(0, (TP - FP) / C) * min(K, C) / max(K, C)
```

An empty selection or `C = 0` returns zero. The latter is a defensive helper
boundary; publication validation remains responsible for the real question
constraints. The helper adds no scoring policy for other Question Backends.

## Existing source constraints

The existing Native JSON source compiler requires 2 to 100 choices and a
nonempty, unique correct-choice list containing only available choice IDs.
The Rust grading validator applies the same minimum-two choice shape and
requires the private answer key to name at least one available choice. These
constraints establish `C >= 1` and distinct choice identities. They do not
require an incorrect choice: an all-correct key remains valid, consistent with
the scoring specification's perfect-selection rule when every choice is
correct. This task does not change those validation contracts.

## Evidence

The focused module tests cover the supplied C=2, C=5, and C=8 examples,
including eight correct and two incorrect selections earning 0.6; empty
selection; zero credit when penalties exhaust base credit; score bounds for
all TP/FP combinations across valid ten-choice question counts; the specified
non-increase when adding an incorrect selection; and count stability after
reversing choice order.

Commands run:

- `rustfmt --edition 2024 crates/grading/src/ple_question_json/multiple_answer.rs` - passed.
- `rustc --edition=2021 --test /tmp/m20_multiple_answer_tests.rs -o /tmp/m20_multiple_answer_tests` - passed using a temporary module-path harness because the helper is not yet registered with the grading crate.
- `/tmp/m20_multiple_answer_tests` - 6 passed, 0 failed.

The first temporary harness draft used `include!` and failed because the
helper's inner module documentation is not valid at an `include!` expansion
site. The harness was corrected to use `#[path]`; this did not require a source
change.

## Integration status

The helper is not registered or called by the grader yet. M17's current
Assessment partial-credit control and native evaluation integration remain
pending. No shared Cargo build, SQL work, or runtime acceptance was performed;
the grading crate and downstream behavior still require the root integration
and acceptance gate.
