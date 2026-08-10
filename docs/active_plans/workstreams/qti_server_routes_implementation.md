# WP-QTI-9 server routes implementation

## Status

Complete and independently accepted on 2026-08-09. This handoff records the server-owned route,
worker, object, and backend-lifecycle boundary. It does not claim author UI, deployment, or live
PostgreSQL/RLS/profile-to-native acceptance.

## Completed behavior

- Author upload accepts only an exact bounded `application/zip` archive after authorization and
  draft access. It persists one deterministic private workspace source object and one deterministic
  `qtiImport` job. Exact replay returns the prior state; divergent replay refuses.
- Safe report reads return only recognized package/item defaults, diagnostics, state, and digest
  acknowledgements. They exclude archive/object identity, raw XML, answer keys, feedback, vendor
  choice maps, canonical source, and grading material.
- The worker detects Canvas and Blackboard profiles before generic parsing, stages all-and-only
  accepted-item evidence, treats all-rejected recognized imports as reportable without conversion,
  and refuses mixed vendor evidence.
- Conversion requires a strong draft ETag plus the report revision and acknowledgement tokens. It
  rereads the retained archive, reparses and remaps the selected item, recompiles through the native
  bridge, and uses the WP-QTI-8 atomic Store command. Refused requests do not mutate the draft.
- Flat publication copies the exact source archive to deterministic non-signable
  `PublishedImportArchive` provenance. Manually authored questions retain no import origin.
- Memory and PostgreSQL serialize draft deletion and prepared profile work. Deletion either prevents
  preparation or removes prepared state before a workspace identity can be reused.
- Responses use `Cache-Control: no-store`; inaccessible and absent resources remain uniformly
  non-enumerable and every route DTO is answer-free.

## Validation evidence

- Adapter: 93 unit, 6 conformance, and 12 documentation tests passed.
- Objects: 17 unit, 3 conformance, and 1 published-archive test passed. One MinIO-dependent test is
  ignored outside a live service.
- Learning-data-access: 93 unit, 39 conformance, and 3 documentation tests passed. One database-only
  unit and 7 live PostgreSQL integration tests remain documented ignores.
- Server: 184 library tests, 1 main test, and 1 documentation test passed.
- Strict Clippy passed for adapter, objects, learning-data-access, and server. Workspace
  all-target/all-feature check, `cargo fmt --check`, and the 5 crate-boundary checks passed.
- A one-time actual 32 MiB plus one-byte chunked-upload probe passed and was removed rather than
  retained as a brittle permanent test.
- Independent route, worker, backend-lifecycle, and publication reviews reported PASS with no P0/P1
  finding.

## Historical successor

WP-QTI-10 was the immediate successor and owned the visible author UI over these stable safe DTOs.
WP-QTI-11 then accepted the disposable live PostgreSQL/RLS/profile-to-native path, grading,
archive/provenance, and cleanup; WP-QTI-12 completed final independent review and documentation
close-out.

WP-QTI-1 through WP-QTI-12 are now accepted history. Current authority is
[release_completion_plan.md](../active/release_completion_plan.md): WP-RC3 shipped upstream WeBWorK
is current, WP-ARCH1 follows it, then WP-RC4 owns the QTI-JSONL contract, WP-RC5 owns families and
Chapter 1 content, and WP-RC6 closes QTI export and H5P claims.

## Scope notes

No browser XML or ZIP parsing was introduced. No direct private grading, choice-map, or provenance
table read was added to the application Store path. No index or staging state was changed.
