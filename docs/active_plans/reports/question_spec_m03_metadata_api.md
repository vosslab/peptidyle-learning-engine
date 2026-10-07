# M03 ordinary Question metadata API

Status: Rust, browser contract, client, detail editor, and focused local checks are complete. Fresh
database execution remains pending with the M03 runtime gate.

## Behavior

An ordinary metadata save replaces Title, Description, Tags, Discipline, Subject, Topic, and
Subtopic on the exact current Published Question Revision. The request carries
`PublishedQuestionRevisionTuple` and `expectedMetadataEditNumber`. The trusted SQL procedure checks
both values, ownership or Sysadmin authority, and current availability in one transaction. It
preserves the Revision Number, source, and language. A stale tuple or Edit Number maps to a
precondition failure.

The detail editor reads the current metadata counter and classification through the existing
current-metadata read. It separately reads the current lineage and shows the editor only when the
server reports `viewerMayEditMetadata` and its tuple matches the displayed Revision. The receipt is
bound back to that tuple before the page reloads the current Revision.

The edit affordance is false for every viewer while a Question is archived, including Sysadmins.
The focused PostgreSQL oracle checks the Sysadmin affordance before and after archival as well as
the existing archived-save rejection. Fresh database execution remains pending with the M03 runtime
gate.

## Files

- `crates/question_model/src/question_metadata.rs` defines the closed request, replacement, and
  receipt contracts.
- `crates/learning-data-access/src/question_metadata.rs` and
  `crates/learning-data-access/src/postgres/question_metadata.rs` validate the bounded replacement
  and call the trusted SQL wrapper. `NULL` language means preserve the stored value under lock.
- `crates/server/src/question_metadata.rs` registers the authenticated owner/Sysadmin route.
- `crates/question_model/src/question_library.rs`,
  `crates/learning-data-access/src/question_library.rs`, and
  `crates/server/src/question_library.rs` carry the separate, server-computed metadata-edit
  affordance.
- `src/api/question_metadata.ts`, `src/api/http_client/question_metadata.ts`, and
  `src/api/decoders/question_metadata.ts` compose and strictly decode the browser save operation.
- `src/features/question_metadata/question_metadata_editor.tsx` provides the bounded detail editor;
  `src/pages/question_detail_page.tsx` wires it into exact Revision loading and refresh.
- `generated/api/` contains the generated request, replacement, receipt, and updated lineage view.

## Evidence

Passed:

- `source ./source_me.sh && cargo tsgen` - generated contracts are current.
- `source ./source_me.sh && cargo check -p learning-data-access --features postgres`.
- `source ./source_me.sh && cargo check -p server_core`.
- `source ./source_me.sh && cargo fmt --all --check`.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.json`.
- `source ./source_me.sh && npx tsc --noEmit -p tsconfig.lint.json`.
- Scoped ESLint and Prettier checks for touched browser/client files.
- `node --import tsx --test tests/test_question_metadata_client.mjs` - 2 passed.
- Related Question availability, authoring, and archive tests - 34 passed.
- Focused Question model detail-wire test - 1 passed.
- Focused server Question Library test - 1 passed.

Pending: run `question_revision_metadata` PostgreSQL behavior coverage and fresh schema/function
verification against the disposable database. These checks establish SQL function compilation and
transaction behavior that the Rust and browser checks do not cover. The new Sysadmin affordance
assertions compile with the test target but have not yet run against PostgreSQL.
