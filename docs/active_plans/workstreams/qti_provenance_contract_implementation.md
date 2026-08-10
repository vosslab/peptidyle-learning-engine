# QTI provenance contract

## Status

Complete. Q4/WP-QTI-6 passed focused validation and independent review on 2026-08-09. This
package freezes the server-to-storage provenance boundary and the typed archive object identity;
it does not add backend or schema mutation.

## Private choice-map bytes

`adapter_qti::profiles::QtiChoiceMapPayload` owns the versioned encoding. The exact byte sequence
is:

```text
ple:qti-choice-map:v1\0
u32 choice count (big endian)
for each ordered choice:
    u32 vendor identifier byte length (big endian), UTF-8 vendor identifier bytes
    u32 PLE identifier byte length (big endian), UTF-8 PLE identifier bytes
```

The source order is retained. The adapter rechecks the 100-choice and identifier bounds, rejects
duplicate vendor or PLE identifiers, and computes SHA-256 over the complete encoded bytes. The
blue/red fixture digest is
`304b5c4bd3bda1952f96be4f3e3bbc1da68636d70aaf47b764ab5e3a9cd2cdb9`. The payload deliberately
implements neither `Debug` nor serialization.

## Storage-owned contract

`learning_data_access::flat_import_provenance` owns closed persistence types, so adapter types do
not cross the Store boundary:

- `PersistedFlatImportProfile` accepts only Canvas QTI 1.2 static single-choice v1 and Blackboard
  QTI 2.1 static single-choice pool v1, with profile and mapping version `v1`.
- `FlatImportConversionVersion` stores the server-owned
  `ple-qti-profile-flat-conversion/v1` value after bounded identifier validation.
- `WorkspaceFlatImportOrigin` retains the committed import reference, workspace archive record,
  source item identifier, profile/conversion identity, normalized/report/public/private/combined
  mapping and warning digests, mapped canonical-source digest, acknowledgement actor/time, and
  opaque private choice-map bytes plus checksum.
- `PublishedFlatImportOrigin` is an immutable tenant-owned copy bound to one problem/version.
  `FlatImportPublicationPromotion` can be made only from the locked current origin and its
  deterministic published archive candidate.
- `QtiProfileFlatConversionCommand` is the one all-or-nothing command for draft, canonical source,
  private grading payload, and current origin. Its constructor revalidates source and private
  binding; ordinary flat-editor saves preserve the current origin.

The private origin types do not implement `Debug` or serialization. Promotion is fail-closed:
the server must supply the expected current-origin identity and exact typed archive record, and
the Store rechecks the committed import, archive metadata, selected item, profile/version contract,
all digests, draft revision, canonical source, and private grading binding before mutation.

Every implementation uses this lock order: workspace draft `FOR UPDATE`, committed QTI import
`FOR KEY SHARE`, current origin, current flat source, then immutable publication rows. Object
copies happen before the database transaction. Cleanup may lock an import and inspect origin pins,
but never acquires the workspace draft afterward, avoiding the conversion/cleanup deadlock.

## Published archive object

`objects::ObjectKey::PublishedImportArchive { tenant, problem, version, import, object }` is a
distinct content/source key with path
`tenants/{tenant}/problems/{problem}/versions/{version}/imports/{import}/archive/{object}`. It
retains the published version, is never eligible for a signed URL, and derives `object` from the
first 16 bytes of SHA-256 over the v1 domain separator, raw tenant/problem/version/import UUIDs,
and the raw 32-byte archive digest. The fixed `u128` 1/2/3/4 plus `archive fixture` golden is
`e6ca5943-2fb2-c3b2-bf14-5c9cc3813aa1`.

## Validation evidence

- `cargo test -p adapter_qti choice_map_payload --lib`: 6 passed, including encoding, order,
  bounds, sensitivity, determinism, and golden digest checks.
- `cargo test -p learning-data-access flat_import_provenance --lib --no-default-features`: 5
  passed; the provenance payload compile-fail doctest also passed.
- `cargo test -p objects published_import_archive --all-features`: 3 focused tests passed; the
  broader object contract gate passed with MinIO-only tests ignored outside a live service.
- The QTI bridge's focused tests and the prior full server gate passed; strict formatting, Clippy,
  crate-boundary, whitespace, and diff checks passed. Independent review reported PASS with no
  P0/P1 finding.

## Historical successor

WP-QTI-7 was the immediate successor. It added the schema, grants, RLS, retention, and object-copy
implementation, reconciled the Rust 1,024-character source-item bound with the SQL-side
512-character constraint, and proved the chosen boundary through validators plus migration and
conformance tests before backend mutation.

WP-QTI-1 through WP-QTI-12 are now accepted history. Current authority is
[release_completion_plan.md](../active/release_completion_plan.md): WP-RC3 shipped upstream WeBWorK
is current, WP-ARCH1 follows it, then WP-RC4 owns the QTI-JSONL contract, WP-RC5 owns families and
Chapter 1 content, and WP-RC6 closes QTI export and H5P claims.
