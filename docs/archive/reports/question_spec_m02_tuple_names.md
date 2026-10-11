# M02 Revision Tuple names verification

Status: Accepted on literal criteria; implementation, focused checks, and independent specification
and quality reviews passed. M01 runtime dependencies and overall plan acceptance remain pending.

M02 aligns the tuple type name to `BlueprintCourseRevisionTuple` and its fields to language-native
names across current Rust, TypeScript, API JSON, generated contracts, tests, and active
documentation. The wire shape identifies one Blueprint Course Revision with `blueprintCourseId`
and `revisionNumber`. The contract emits only this canonical shape.

## Evidence

- `cargo test -p browser-api-contract revision_view_keeps_the_exact_immutable_tuple`: passed. The
  contract test round-trips the exact ID and Revision Number and rejects noncanonical tuple names.
- `source ./source_me.sh && node --import tsx --test tests/test_question_revision_tuple_decoder.mjs`:
  5 passed, 0 failed. The decoder accepts the canonical tuple and rejects noncanonical tuple names,
  snake_case fields, and extra members.
- `source ./source_me.sh && npx tsc -p tsconfig.json`: passed.
- `source ./source_me.sh && npx tsc -p tsconfig.lint.json`: passed.
- `git diff --name-only -z -- '*.rs' | xargs -0 rustfmt --check --edition 2024`: passed for the
  modified Rust files.
- Generated TypeScript from the current contract sources matched the current
  `generated/api/` files byte-for-byte across all 411 tsgen outputs. The additional
  `QuestionIdSyntaxContract.ts` file is owned by its separate generator.
- `git diff --check`: passed.
- `source ./source_me.sh && python3 -m pytest -q tests/test_markdown_links.py
  -k 'question_spec_m02_tuple_names'`: passed (1 test).
- The current-code search finds noncanonical tuple names only in deliberate rejection cases in
  `crates/browser-api-contract/src/blueprint_course.rs` and
  `tests/test_question_revision_tuple_decoder.mjs`. Historical changelogs and archived audit
  records retain their original terminology as historical evidence.

The wider Node run recorded during implementation passed 58 suites/cases with no failures; its log
was `/private/tmp/ple_m02_node_tests.log`. That temporary log is supplemental evidence; the focused
five-case rerun above is the durable M02 gate.

## Remaining acceptance

The scoped runtime check passed for `server_core` and `project-tools`, including PostgreSQL and the
project-tools runtime feature:

```sh
source ./source_me.sh && cargo check -p server_core -p project-tools --features project-tools/runtime-tools
```

M02 is accepted on its literal criteria; implementation, focused checks, and both independent
reviews passed. M01 runtime dependencies and the broader question-spec implementation remain pending
independently of this naming milestone.
