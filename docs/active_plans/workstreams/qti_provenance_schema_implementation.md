# QTI provenance schema implementation

## Status

WP-QTI-7 implementation evidence is refreshed after the 2026-08-09 choice-map checksum repair.
The final independent checksum re-review reported PASS with no P0/P1 findings. This package
implements only schema, RLS, retention, and protected SQL capabilities; the backend atomic
conversion and object-copy orchestration remain WP-QTI-8.

## Persisted boundary

- `ple_qti_provenance_broker` is a dedicated `NOLOGIN`, `NOINHERIT`, `NOBYPASSRLS` role.
- Six tenant-owned provenance relations store current/published origins, private choice maps, and
  committed profile/item evidence. RLS is enabled and forced on each relation.
- Protected `SECURITY DEFINER` capabilities use the explicit `pg_catalog, public, pg_temp` search
  path, revoke `PUBLIC` execution, and grant only the named roles needed for staging, reading,
  replacement, promotion, and release.
- Current origin writes validate the committed import registry's complete typed workspace-source
  `ObjectRecord`: key shape/classification, no public version, checksum, size, media type, license,
  provenance, and creation time. They do not trust a caller-supplied archive summary.
- Before an import commits, profile/report/public/private/combined/warning/choice-map evidence is
  staged and bound to each accepted item and normalized digest. Provenance reads require that
  committed evidence.
- PostgreSQL recomputes SHA-256 over private choice-map bytes at the table boundary. The digest
  trigger fences both current and published choice maps, including direct writes by the provenance
  broker, rather than trusting a capability caller's supplied digest.

## Lifecycle and bounds

- Current lineage pins its committed import. A pinned import cannot regress, delete, or be cleaned
  until current provenance is released.
- Ordinary workspace-draft cleanup releases current lineage only. Published origins and choice maps
  are immutable, tenant-owned retained evidence; cleanup proceeds child-first after release.
- Current and published archive provenance remains non-signable by the typed object contract; no
  provenance relation contains a browser-deliverable URL.
- SQL matches the Rust 1,024-Unicode-scalar bound for import item, result, grading, published
  grading, profile evidence, and origin identifiers. The registry-level package identifier remains
  independently bounded at 512 bytes.

## Validation evidence

- `cargo test -p learning-data-access publication_validation --no-default-features`: 3 passed.
- `bash -n tests/e2e/e2e_database_baseline.sh`: passed.
- `git diff --check`: passed.
- `bash tests/e2e/e2e_database_baseline.sh`: passed fresh after the checksum repair. The disposable
  gate applies all six migrations to an empty database, verifies a no-op reapply and final verify,
  then runs the real-role provenance/RLS/pin/cleanup oracle. It rejects bad choice-map digests
  through both the protected capability and direct provenance-broker inserts for current and
  published maps. The oracle also round-trips 1,024 multibyte Unicode scalars and rejects 1,025
  with named constraints.
- Final independent checksum re-review: PASS with no P0/P1 findings.

## Next package

WP-QTI-8 implements the backend-owned atomic conversion in Memory and PostgreSQL. It must commit
the CAS revision, draft, canonical source, private compiler payload, and current origin together;
it must then promote only the locked current origin while copying the immutable archive. Its
conformance and PostgreSQL feature tests must keep the accepted schema capabilities narrow.
