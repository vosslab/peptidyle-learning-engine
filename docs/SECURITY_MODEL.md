# Security model

Peptidyle keeps grading authority on the server. The browser may determine
whether a response is structurally ready to submit, but it never receives an
answer key or makes a correctness decision.

## Grading boundary

| Browser-safe surface                           | Server-only surface                |
| ---------------------------------------------- | ---------------------------------- |
| `ResponseDefinition` and `StudentResponse`     | `grading::AnswerKey`               |
| Parameter generation from a supplied seed      | Expected numeric values            |
| Response-format validation                     | Correct choice IDs and ordering    |
| Timer display and pure state transitions       | Accepted text and private rubrics  |
| Correctness and point results after disclosure | Checkers and correctness decisions |

The browser-safe model explains the input shape and public grading policy. For
example, it may reveal a numeric tolerance or that exactly two choices are
required. The expected number and the two correct choice IDs remain in
`crates/grading`.

`crates/grading` is the only answer-bearing crate. Ungraded content has no
`AnswerKey`; it does not use a browser-safe placeholder key. Native H5P remains
ungraded practice because its own evaluation runs in the browser.

## Format validation

`domain::validation::validate_response_format` checks only student-controlled
structure:

- response kind matches the definition;
- numeric input is finite;
- selection count, uniqueness, and IDs are valid;
- short text fits its character limit;
- ordering is an exact permutation of the displayed items; and
- an uploaded response carries a server-issued object reference.

This function has no answer-key parameter and cannot determine correctness.
The browser calls it through `wasm_bridge::validate_response_format`; the
server repeats it before grading because client validation is a convenience,
not an authority. File size, extension, checksum, and ownership are checked by
the server against object metadata rather than trusted from the browser.

## Compile-time closure

The shipped workspace dependency closure of `wasm_bridge` is exactly:

```text
wasm_bridge
+-- domain
|   `-- question_model
`-- question_model
```

It contains no `grading` crate. `tests/test_crate_boundaries.py` resolves the
normal, build, and target-specific local dependency tables conservatively and
fails if any other workspace crate enters this closure. Including build
dependencies matters because a build script could otherwise embed secret data
without becoming a runtime dependency.

Run the closure gate with the repository Python environment:

```bash
source source_me.sh && python3 -m pytest tests/test_crate_boundaries.py
```

## Export allowlist

`tests/test_wasm_export_allowlist.mjs` builds the current bridge, processes it
with the lockfile-matched `wasm-bindgen` tooling, and compares every export name
and kind with a committed allowlist. Its disposable processed module lives
under ignored `generated/wasm-export-check/`.

The reviewed application exports are currently:

- `bridge_version`;
- `timer_verdict`;
- `validate_assignment_config`; and
- `validate_response_format`.

The allowlist also names the exact memory, table, allocator, and lifecycle
exports required by `wasm-bindgen`. A new Rust export fails the gate until a
reviewer determines that it is key-free and deliberately updates the list.
An answer-bearing export is rejected rather than added.

`timer_verdict` is safe in the browser because its inputs are already disclosed
timer policy and server timestamps, and its output cannot reveal an answer.
The server still supplies the authoritative evaluation timestamp and decides
whether to accept a submission; browser time remains display-only.

`validate_assignment_config` receives only question definitions and backend
capability declarations already shown to an instructor. Its violations name a
question version and a missing capability, never an answer or grading key. The
server independently calls the same domain function before publication.

Run the export gate directly:

```bash
node --test tests/test_wasm_export_allowlist.mjs
```

Both boundary gates run from `./check_codebase.sh`.

## Placement rule

Place new code according to the information it needs and the decision it
makes:

- Put response parsing and structural validation in `crates/domain` when the
  result is independent of a correct answer.
- Put expected values, accepted answers, grading rubrics, partial-credit
  weights, and correctness decisions in `crates/grading`.
- Expose a domain function through `crates/wasm` only when all inputs and
  outputs are safe for a student to inspect.
- Return correctness and points through server-controlled feedback policy;
  never return the key or checker state.

When uncertain, ask whether the value would help a student infer the correct
response before submission. If yes, it belongs on the server-only side.

## Other controls

WP-C6 proves the source and WebAssembly boundary. Later work packages add and
verify the remaining controls: authentication, authorization, PostgreSQL
answer-table grants, forced tenant row-level security, sanitized supplied
markup, content security policy, signed object URLs, and browser network-trace
inspection. None of those later controls weakens the crate boundary established
here.
