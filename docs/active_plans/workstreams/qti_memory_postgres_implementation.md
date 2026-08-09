# QTI Memory/PostgreSQL implementation

## Status

WP-QTI-8 is complete and independently reviewed. Shared backend conformance, PostgreSQL feature
coverage, and the full fresh database baseline reported PASS. Independent review reported PASS with
no P0/P1 findings. WP-QTI-9 server routes are next.

This handoff implements only the backend persistence boundary frozen by
[qti_profile_mapping_plan.md](../decisions/qti_profile_mapping_plan.md). It does not add archive-copy
or HTTP orchestration. The separately frozen
[course_appearance_plan.md](../decisions/course_appearance_plan.md) remains a later package.

## Closed evidence

- One closed `QtiProfileImportEvidence` value stages profile, mapping-version, item, and digest
  evidence only while the referenced QTI import remains prepared. It implements neither `Debug` nor
  serialization.
- This closes the H2 staged-profile-evidence gap. Exact replay is idempotent. Divergent replay,
  foreign tenancy, a non-prepared import, or mismatched accepted-result evidence refuses without
  changing the stored row.
- The accepted result must bind both `sourceIdentifier` and `itemId` to the selected source item and
  must carry the exact normalized digest. Conversion revalidates that H2 evidence after the import
  commits.
- `Sha256Digest` serializes as lowercase 64-character hexadecimal text. Deserialization rejects
  uppercase, wrong-width, and non-hex input rather than accepting multiple evidence forms.

## Atomic backend behavior

- Memory and PostgreSQL implement the same provenance-aware conversion contract and error behavior.
- One successful conversion advances the workspace draft CAS revision and commits the draft,
  canonical source binding, current private grading payload, and current import origin atomically.
  Every refusal leaves the complete prior state unchanged.
- Both implementations preserve the frozen order: workspace draft, committed import, current
  origin, current source, then immutable publication rows. PostgreSQL uses the corresponding
  `FOR UPDATE`/`FOR KEY SHARE` locks. Object copying remains before the database transaction.
- Origin replacement installs the new import pin before any old pin is released. Origin promotion
  also precedes promotion of private grading, so a grading publication cannot outrun its immutable
  lineage.
- Ordinary flat saves atomically replace the current private grading payload with their draft and
  source while preserving any current import origin.
- Publication no longer accepts private grading from the caller. It promotes only the grading value
  already stored for the exact locked current source, which keeps grading behind the dedicated
  capability.

## PostgreSQL boundary

- Profile-evidence staging and origin reads/writes use the narrow WP-QTI-7 provenance broker over
  forced-RLS relations. Private grading staging and promotion use the grader broker.
- The application Store implementation does not directly read private grading, private choice-map,
  current-origin, or profile/item-evidence secret relations. Broker outputs are bounded to the exact
  answer-free evidence needed for validation.
- Conversion still validates the committed import's complete typed workspace archive record and the
  accepted result before persisting derived flat state.
- Existing RLS, broker roles, pin/release behavior, immutable published lineage, and non-signable
  archive rules remain unchanged.

## Validation evidence

- Shared Memory and PostgreSQL conformance: PASS for exact staging replay, atomic conversion, stale
  CAS, digest/result refusal without mutation, ordinary save origin preservation, stored-only
  grading publication, immutable origin promotion, and tenant isolation.
- PostgreSQL feature coverage: PASS for prepared-only evidence staging, broker/RLS enforcement,
  exact result `itemId` binding, frozen lock/promotion order, and current/published provenance.
- Strict `Sha256Digest` JSON round-trip and malformed-input tests: PASS.
- Full fresh `bash tests/e2e/e2e_database_baseline.sh`: PASS, including six-migration apply,
  no-op reapply, ledger verification, real-role RLS/broker probes, and the WP-QTI-8 live path.
- Independent implementation review: PASS with no P0/P1 findings.

## Next package

WP-QTI-9 owns archive-copy orchestration and the upload/replay/report/convert server routes. It must
use the accepted Store commands and broker boundaries, preserve no-store/ETag/non-enumeration
behavior, and add no direct private-table or secret read.
