## Completed

Removed WebWork `questionType` GUC transport. The Draft metadata save now passes `p_replace_question_type` and nullable `p_question_type` through matching 15-argument private/API functions, grants, and caller. Omission preserves Type, explicit `null` clears it, and a value sets it-all under one Draft Edit Number CAS. Draft source save uses a separate 9-argument signature. These Draft functions are distinct from the 14-argument Published Question metadata replacement in `published_question_metadata_operations.sql`.

Ordinary source saves now retain empty or malformed Native JSON, unresolved Native HOTSPOT references, and broken PG/PGML text under the registered binding. Image admission is deferred to preview/publication; Native HOTSPOT publication validation remains. The stale Native HOTSPOT fixture was corrected.

The queued acknowledgement correction is integrated: both save methods load and decode the saved Draft on the same transaction before commit releases the row lock. This ties each response's Edit Number and values to its own save.

Implementation is in [the source handler](../../../../crates/server/src/authoring.rs#L369), [source inspection](../../../../crates/server/src/authoring_source.rs#L105), [Postgres save adapter](../../../../crates/learning-data-access/src/postgres/authoring.rs#L173), and [Draft SQL functions](../../../../schemas/base_schema/50_functions/question_authoring_operations.sql#L565). The 14-argument [Published Question metadata function](../../../../schemas/base_schema/50_functions/published_question_metadata_operations.sql#L8) is a separate operation. This report records the Draft signatures and checks; see the [save acknowledgement report](../QUESTION_SPEC_M15_SAVE_ACK.md) and [M15 ledger entry](../question_spec_implementation_ledger.md#L40).

## Verification

Focused Rust tests passed: source inspection 7/7, HOTSPOT 5/5, server Type tri-state 1/1, and adapter Type mapping 1/1. Both PostgreSQL test targets compiled with `--no-run`; neither was executed against a database. Rust formatting, schema style, `git diff --check`, and the under-1,000-line checks passed. Server `authoring.rs` is 867 lines; the SQL functions file is 953.

Connected PostgreSQL, Object Store, renderer, HTTP, and browser acceptance remains pending. `DATABASE_URL` and `PLE_ACCEPTANCE_RUNTIME_MANIFEST` are unset, and canonical runtime acceptance still requires the inspected M09 cleanup receipt and separate root start signal. The ledger keeps M15 unaccepted pending connected acceptance and fresh scoped reviews.
