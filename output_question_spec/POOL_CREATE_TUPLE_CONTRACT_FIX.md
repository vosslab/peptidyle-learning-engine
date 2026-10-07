# Pool creation tuple contract fix

`POST /api/question-pools` now deserializes each `members` entry as the shared
`PublishedQuestionRevisionTuple`. This accepts the browser's
`{ publishedQuestionId, revisionNumber }` shape and retains the tuple's typed
Published Question ID, positive Revision Number, and closed-field validation.
The handler continues to enforce the bounded body and member count, Instructor
session, server-issued Pool ID and collision retry, Store validation, and
answer-free no-store creation receipt.

The focused server tests accept and round-trip a canonical tuple, and reject the
legacy `questionId` shape, a malformed Published Question ID, Revision Number
zero, and an unknown tuple field. The Pool creation row in
[`API_CONTRACTS.md`](../docs/API_CONTRACTS.md) now documents the same member
shape. The browser request implementation and TypeScript contract were not
changed.

## Review

- Fresh SPEC review approved the direct shared-tuple boundary before editing.
- A distinct fresh QUALITY review accepted the browser/server composition and
  found no in-scope defects.

## Focused verification

```sh
source ./source_me.sh && rustfmt --edition 2024 --check crates/server/src/question_pool_creation.rs
source ./source_me.sh && git diff --check -- crates/server/src/question_pool_creation.rs docs/API_CONTRACTS.md
source ./source_me.sh && cargo test -p server_core question_pool_creation::tests --lib
source ./source_me.sh && cargo check -p server_core
source ./source_me.sh && node --import tsx --test tests/test_question_pool_creation_client.mjs
```

Rust formatting and scoped whitespace checks passed. The focused server tests
passed 3/3, server compilation passed, and the existing browser-client tests
passed 2/2. Cargo reported three warnings in existing authoring and Pool
library code. A plain `node --test` invocation did not resolve extensionless
TypeScript imports before test startup; rerunning with the repository's `tsx`
loader passed.

No TypeScript files were edited, so TypeScript compiler checks were not run.
No database, container, or full M29 runtime checks were run; full runtime
acceptance remains pending.
