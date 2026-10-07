# M15 Draft save acknowledgement

## Scope and ownership

The M15/M16 backend source correction owner released the ordinary save seam. This correction owns
`PostgresAuthoringDraftStore::save_authoring_draft` and
`PostgresAuthoringDraftStore::save_authoring_draft_general_feedback_with_question_type` in
[`authoring.rs`](../../../crates/learning-data-access/src/postgres/authoring.rs), plus the focused
[`authoring_draft_source_postgres.rs`](../../../crates/learning-data-access/tests/authoring_draft_source_postgres.rs)
oracle. The ordinary SQL functions are `ple_private.save_authoring_draft`,
`ple_api.save_authoring_draft`, `ple_private.save_authoring_draft_general_feedback`, and
`ple_api.save_authoring_draft_general_feedback` in
[`question_authoring_operations.sql`](../../../schemas/base_schema/50_functions/question_authoring_operations.sql).

The released working tree already captured both save projections before commit. The Rust methods
now document that contract beside each read. No SQL return signature, API response, function, or
grant changed for this correction.

## Capture and preservation

Each method runs the save and `ple_api.load_authoring_draft` query on the same authenticated SQLx
transaction. The ordinary SQL save function locks the Draft row with `FOR UPDATE`, checks the
expected Draft Edit Number, applies the requested source or metadata/support change, and advances
that shared Edit Number once. The adapter decodes the full projection before committing, while the
row lock still serializes later saves. The returned `AuthoringDraft` therefore retains this request's
Edit Number and values after later transactions advance database state.

The projection includes the M07 parent Question Revision Tuple. `decode_draft` continues to decode
it through `decode_parent_revision_tuple`; ordinary source and metadata saves do not write parent
fields. Source save parameters remain separate from metadata. The metadata/support function retains
the explicit optional `questionType` contract: omitted preserves, JSON null clears, and a value sets
the Type. Existing SQL signatures remain 9 source arguments and 15 metadata/support arguments.

Human Guidance says Drafts have no content or metadata requirements until publication. The ordinary
metadata save accepts blank title and description, empty tags, absent license, and absent language.
New-lineage publication retains the nonblank title/description, license, and Type checks; revision
publication retains its Type check. No new model or default was added.

## Regression coverage

The connected Draft source oracle now uses a source record with a distinct checksum and size. It
checks that the source response returns that record and its own next Edit Number while preserving
metadata. A following metadata/support save supplies blank title and description, no license, and
distinct feedback, hint, and worked-solution values; its response must return those values, its own
next Edit Number, and the prior source record and optional WebWork Type. A later metadata save checks
the following Edit Number and source preservation. Reusing the earlier Edit Number for a source save
must still return the stale-CAS error.

This sequence checks both operation projections and the shared Edit Number contract. It does not
force a scheduler interleaving between commit and response; race correctness depends on keeping the
projection read and decode on the CAS transaction before its Draft row lock is released. The ordinary
fixture has no non-null parent tuple, so M07 preservation is confirmed by projection/decoder and SQL
write-boundary inspection rather than by a parented connected fixture.

## Reviews

- Fresh SPEC review: ACCEPT. Confirmed both reads and decodes remain inside the open CAS transaction,
  the stale-CAS and shared Edit Number contract remain intact, and the source, metadata/support,
  optional Type, and M07 projection boundaries are preserved.
- Different fresh QUALITY review: ACCEPT with no scoped findings. Confirmed SQL row locks span the
  in-transaction projection reads, transaction/error ordering is sound, the grants match the
  unchanged 9-argument Draft source-save and 15-argument Draft metadata-save signatures. The
  separate Published Question metadata replacement has 14 arguments. The report distinguishes static parent-field
  evidence from connected runtime evidence.

## Verification

- `rustfmt --edition 2024 --check crates/learning-data-access/src/postgres/authoring.rs crates/learning-data-access/tests/authoring_draft_source_postgres.rs` passed.
- `git diff --check -- crates/learning-data-access/src/postgres/authoring.rs crates/learning-data-access/tests/authoring_draft_source_postgres.rs schemas/base_schema/50_functions/question_authoring_operations.sql` passed.
- `source ./source_me.sh && cargo check -p learning-data-access --features postgres` passed.
- `source ./source_me.sh && cargo test -p learning-data-access --features postgres metadata_question_type_maps_omission_clear_and_value_to_explicit_arguments` passed, 1 test.
- `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test authoring_draft_source_postgres --no-run` passed; compile-only.
- `source ./source_me.sh && python3 tests/test_markdown_links.py` and
  `source ./source_me.sh && ./schema_style/check_schema_style.py` passed.

The ignored connected PostgreSQL oracle was not executed. The root-owned runtime marker requires a
separate start signal and an inspected cleanup receipt for the failed M09 disposable install; the
receipt is not present, so the canonical runtime command remains gated. Connected PostgreSQL/Object
Store, HTTP, renderer, and browser acceptance remain with root. The relevant Rust files are 576 and
529 lines; the shared SQL file is 953 lines.
