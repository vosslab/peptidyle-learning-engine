# BPW content import plan

## Outcome

Import every usable collection from the seven public BiologyProblems.org (BPW) collections.
Import the four named subjects (Genetics, Biochemistry, Biotechnology, and Biostatistics) into
public Blueprint Courses. Import the other three collections into the Library only. Keep
Blueprint revision, archive, and adoption behavior intact. Treat BPW source files and metadata as
the authority for content, ordering, and topic names.

## Settled boundaries

- Import all seven usable BPW collections: Genetics, Biochemistry, Biotechnology, and
  Biostatistics are public Blueprint Courses; the other three are Library-only. Omit
  private/publication visibility stages.
- Keep the two import paths explicit. Import PG/PGML directly as WeBWorK Questions; never route
  them through QPM. Convert BBQ files with the configured QPM CLI to ready Native JSON plus assets,
  then send them through ordinary PLE HTTP endpoints.
- Use static variants and Pools for every BBQ set. A PG/PGML set supersedes its matching BBQ set.
  MATCH and WOMC remain distinct assessment modes.
- Preserve BPW topic and assessment boundaries. Use one Module per topic, with each Assessment
  containing fewer than 10 delivered Questions (9 maximum). Split Assessments coherently into
  A/B/C as needed. Map source Hard/Bonus labels to Bonus Assessments with `extraCredit` enabled.
- Use these point values: MC, MA, FIB, NUM, and HOTSPOT are 1 point; MATCH and ORDER are 0.5
  points per choice; MULTIFIB is 1 point per choice. Leave Bloom unset and dates empty.
- Preserve the existing Instructor email-code sign-in. The Python importer keeps its session in
  memory. The installer uses a temporary Instructor session, protected handoff, and revocation.
  Add no API tokens, roles, impersonation, or new authentication policies.
- Keep generic Native JSON HTML, CSS, and image binding in PLE's existing receiver and rendering
  path. Add no importer-specific parser or validation layer.
- Remove the Genetics-specific publisher and its policies. Read subject content directly from BPW build outputs;
  BPW owns the task and build pipeline and its source files.
- Have `local_stack_control` run the same Python importer after readiness, using configured BPW
  and QPM paths. Keep trusted installer people separate. Adopt the full Biochemistry Blueprint
  into the BCHM355 Course Instance while retaining the fictional teaching environment, including
  users, enrollments, selector, and TOTP. Preserve content when starting with `--without-live-demo`.
- Keep the importer single-run and bounded: no history migration, recovery ledger, resume support,
  new idempotency mechanism, automatic retries, SQL/store import, batch endpoint, exports, or new
  server.
- Report progress and error IDs for the installer. Return nonzero for incomplete import and never
  automatically reset existing installations.
- Keep permanent tests limited to stable contracts. Use temporary exhaustive catalog checks and
  browser proof, then clean up temporary evidence.

## Implementation tasks

Complete these tasks in dependency order. The status ledger records current ownership, evidence,
and decision propagation in [bpw_import_status.md](bpw_import_status.md).

1. **Public Blueprints - `finish_blueprints`:** finish public completion and remove token work;
   preserve Blueprint history and lifecycle behavior.
2. **Native content - `native_content`:** receive and render generic Native JSON HTML and media.
3. **Importer - `importer_v2`:** discover source metadata and perform the Python HTTP import.
4. **Installation - `installation_v2`:** invoke the same importer from local stack installation
   with protected temporary-session handoff and revocation.
5. **Plan and ledger - `plan_record`:** keep this plan and its status ledger current.
6. **Fresh specification review:** independently check source coverage, boundaries, and contracts.
7. **Quality and integration review:** validate focused behavior, full checks, and rendered
   representative content; record remaining gaps with owners.

## Acceptance evidence

- A temporary catalog check accounts for all seven usable collections and confirms supersession,
  assessment modes, topic splits, and grading rules.
- The four public Blueprint Courses contain the intended content and retain revision, archive,
  and adoption behavior.
- PG/PGML remains WeBWorK content end to end. BBQ becomes Native JSON through the configured QPM
  CLI, with no pinned-version gate.
- The importer uses ordinary Instructor HTTP operations and reports incomplete items with error
  IDs; an incomplete run exits nonzero.
- A local installation uses the configured source/tool paths, protects and revokes its temporary
  Instructor session, preserves content with `--without-live-demo`, and does not reset existing
  installations.
- Browser evidence covers rendered HTML and bound images in representative imported content.
- Run focused checks, then `source ./source_me.sh && ./launchers/all_test.sh`. Record each result
  separately; do not treat a narrow passing slice as full acceptance.
