# M08 Blueprint fork parent records

Status: SQL implementation and focused PostgreSQL test source are ready. Independent specification
and quality reviews pass. Fresh database behavior and focused test execution remain pending; M08 is
not accepted.

## Implementation

Blueprint parentage now lives on the ordinary `ple_data.blueprint_course` row as the nullable
`parent_blueprint_course_id` and `parent_blueprint_revision_number` pair. A paired-null check
represents root Blueprints, a self-parent check rejects invalid identity, a deferred composite
foreign key pins the exact immediate parent Revision, and an update trigger keeps parentage
immutable while allowing ordinary metadata and Revision changes.

Blueprint forks create an ordinary child with its own identity, Private availability, and Revision 1.
Subsequent Saves use the regular Blueprint Revision sequence. A fork of a fork records the selected
immediate source tuple. The separate fork-parent and fork-receipt tables and their policies, grants,
indexes, and trigger are removed. Known-fork, ancestry, selective-update, and normal Blueprint reads
use the parent columns. Fork request replay uses `blueprint_course_create_receipt`, the ordinary
Blueprint creation receipt, and verifies that its child row has the requested source tuple.

Pool-copy behavior and the API fork action remain unchanged in this milestone.

## Files

- `schemas/base_schema/20_tables/blueprint_course.sql`
- `schemas/base_schema/30_constraints.sql`
- `schemas/base_schema/40_indexes.sql`
- `schemas/base_schema/50_functions/blueprints.sql`
- `schemas/base_schema/50_functions/blueprint_operations.sql`
- `schemas/base_schema/50_functions/blueprint_lineage.sql`
- `schemas/base_schema/60_policies/blueprints.sql`
- `schemas/base_schema/70_grants/blueprints.sql`
- `schemas/base_schema/70_grants/blueprint_lineage.sql`
- `crates/learning-data-access/tests/blueprint_course_postgres.rs`
- `crates/learning-data-access/tests/blueprint_course_postgres/lineage_fork.rs`

## Current evidence

- `graphify --context` completed and showed the existing Blueprint Forking area. `graphify explain
  'schemas/base_schema/50_functions/blueprint_lineage.sql'` resolved the five lineage functions;
  `graphify explain 'ple_api.fork_blueprint_course()'` located the fork operation in that file.
- `git diff --check` passed for the M08 Rust test registration and test module.
- `rustfmt --edition 2024` formatted only `blueprint_course_postgres.rs` and
  `blueprint_course_postgres/lineage_fork.rs`.
- The focused PostgreSQL test source is registered but has not been compiled or run. Its selector is
  `cargo test -p learning-data-access --features postgres --test blueprint_course_postgres blueprint_course_postgres_lineage_fork::blueprint_forks_are_ordinary_lineages_with_immediate_parent_and_normal_revisions -- --ignored --exact`.
  It checks root NULL parentage, a new child ID and Revision 1, exact immediate parentage, ordinary
  child Revision 2 Save, unchanged source history, fork-of-fork parentage, same-request replay, and
  rejection when a replay checksum names a different source Revision Tuple.
- The coordinated fresh-database gate installed and replayed the schema, then stopped at an M26
  fixture failure involving authorized construction under RLS. The manager reported no M08 product
  fault from that run. The gate owner will run the M08 selector before the M03 metadata selector;
  its live result is pending.
- Independent specification review and quality review pass for the scoped SQL design and test
  source. The specification follow-up confirmed the corrected mismatch-replay case covers the exact
  source tuple retry invariant; neither review ran tests.

M08 remains pending its focused database behavior run. M01 runtime acceptance also remains pending
in the approved implementation plan.
