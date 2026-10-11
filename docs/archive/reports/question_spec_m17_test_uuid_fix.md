# M17 PostgreSQL test UUID compile correction

The M17 highest-Attempt regression used `Uuid::new_v4()` for fixture identities,
but the active `uuid` features do not expose that constructor. Replaced those
calls with a small `fixture_uuid` helper backed by the already enabled
`Uuid::from_u128` constructor, matching neighboring grading lifecycle fixtures.
The fixed IDs use a distinct `f570` through `f575` range and retain separate
identities for the Assessment Entries, both Attempts, all four Question
Attempts, the accommodation, and each issued Question.

## Verification

- `source ./source_me.sh && rustfmt --edition 2024 crates/learning-data-access/tests/grading_lifecycle_postgres/partial_credit_policy.rs` - passed.
- `git diff --check` - passed.
- `source ./source_me.sh && cargo test -p learning-data-access --features postgres --test grading_lifecycle_postgres --no-run` - passed; compiled `tests/grading_lifecycle_postgres.rs`.

No connected PostgreSQL test or broader build was run. This correction establishes
compile readiness only; fresh specification and quality review and connected
database acceptance remain pending.
