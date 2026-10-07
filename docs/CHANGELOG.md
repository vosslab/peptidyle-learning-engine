# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> September 29 entries are archived in [CHANGELOG-2026-09r.md](CHANGELOG-2026-09r.md).
> September 28 entries are archived in [CHANGELOG-2026-09s.md](CHANGELOG-2026-09s.md).

## 2026-10-07

### Fixes and Maintenance

- Corrected stale Native JSON import status to point to the accepted M01-M29 ledger and separated
  unfinished converter/HTML integration. Clarified that QTI `externalResources` remains source
  metadata without implying a fetching or rendering plan. Recorded the six-pass documentation
  audit; no product behavior or runtime acceptance changed.
- Settled initial-release Question license scope (CC0, CC BY, CC BY-SA; NC/ND excluded for later
  reconsideration), clarified expired-Draft cleanup wording without setting an expiry policy, and
  recorded that PLE accepts Native JSON directly from qti-package-maker-rs. Documented existing image
  storage and per-Pool statistics accumulation across member-set changes. Converter and Draft
  integrations remain separate future work; accepted Question-spec implementation is unchanged.
- Added a concrete qti-package-maker-rs Native JSON writer handoff grounded in the existing decoder,
  Human Guidance, and image tuple. Clarified HTML display and multi-image binding gaps, kept
  HOTSPOT's pre-binding reference as a shared implementation question, and corrected stale Native
  JSON M29 status statements without extending M29 acceptance to the newer HTML requirement.
- Narrowed Question-spec references to deferred evaluation: Assessment point-value and partial-credit
  changes recalculate awarded points from stored Backend credit without another Backend evaluation;
  the deferred case is reevaluating previously submitted Native JSON responses after an answer-key
  or grading-rule correction. It stays outside the current grill unless release-blocking.
- Settled that calculated Pool metadata is stored for search and kept current when the Pool is
  created or saved. Search reads the stored values; periodic backend cron recalculation is deferred.
  Existing field ownership, first-member Discipline/Subject, and license calculation rules remain in
  force.
- Settled invalid numeric Draft editor input as an implementation detail: typed fields retain their
  types, invalid text stays local until valid, and autosave preserves the valid representable source
  state. Save does not run Question Publication Validation; no product question remains. Updated the
  uncertainty log and implementation ledger.
- Withdrew the proposed Native JSON image-reference question after tracing the existing
  `QuestionImageAssetTuple` and logical-versus-physical identity paths. Content-relative import
  references locate supplied bytes; PLE owns the resulting asset IDs, checksums, and storage identity.
  Server integration remains future work. No schema or transport contract was added.
- Recorded that Native JSON display content uses HTML with inline CSS. PLE resolves imported
  content-relative image references and owns Question image IDs, checksums, and storage; converter
  output does not depend on PLE asset IDs. Current escaping remains implementation drift tracked in
  TODO, with sanitization detail and final stored HTML `src` syntax deferred.
- Identified an implementation gap: inline HTML image references in imported prompts and choices
  still need to resolve and display through existing image tuples, storage, and renderers. Visual
  behavior remains pending.
- Closed the approved 29-milestone Question-spec implementation plan. Final `all_test` retry 8228
  passed with 11,178 pytest tests, all 19 fresh PostgreSQL selector groups, installation replay,
  and Course Appearance acceptance; the permitted production-browser suite and 257-image screenshot
  corpus also passed. Fresh final SPEC and distinct QUALITY reviews passed, along with independent
  review of representative canonical and all five focused captures. All 35 scoped TODO behaviors
  are accepted. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Earlier checkpoint: corrected the then-current question-spec checkpoint: final capture mapping and current 12/12 corpus
  checks pass fresh SPEC/QUALITY review; independent visual review passed across representative
  Library, picker, Pool, Native, Student, Sysadmin, and focused PG/PGML/theme captures. M15/M16
  connected Draft workflows and final images pass. M26 direct-request and other-role authorization
  evidence is accounted for. M09's unused transaction-created schema marker and guard were removed after a no-consumer check;
  this changes no Pool behavior. Fresh SPEC/QUALITY reviews and generator/style checks pass;
  M04/M05/M06 specific connected criteria pass. Fresh database runtime and M29 final integration
  QUALITY remain pending. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Earlier checkpoint: retained the passing one-time connected M04/M05/M06 proof as ignored inert source/config and its
  run log under `output_question_spec/final_proofs/`. It covers Native metadata detail/search and
  supplied-language publication, nullable Pool Bloom edit/readback, and non-owner read/use/fork
  with owner-only editing. This closes those specific criteria; M29 fresh-database runtime and
  final integration QUALITY remain pending. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Updated the existing Student Question Type screenshot helper for current Draft controls and
  All Questions searches, including searches that retain the previous filter. The isolated capture
  passed across four viewports. Corrected the staff capture navigation and own-score privacy check;
  the full fresh corpus now passes with 257 captures and its atlas. That capture-checkpoint note
  predates the final mapping review below; see the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Aligned the checklist tool's stale Sysadmin evidence strings with HG's current administrative
  access, confirmation, and audit rules. Checklist diff and consistency checks pass with 1,242
  matching bullets. HG itself is unchanged.
- Completed the temporary question-spec proof cleanup: removed `tests/_temp/` proofs, moved twelve
  misplaced logs into root `output_question_spec/`, and retained indexed ignored audit evidence in
  `output_question_spec/final_proofs/`. Fresh SPEC and distinct QUALITY cleanup reviews passed.
  At that cleanup checkpoint, the database retry was active; no milestone or TODO mapping was
  promoted. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Added the missing score to the Student previous-attempt access projection using the canonical
  scoring rules. Fresh SPEC and distinct QUALITY reviews passed; isolated-database confirmation
  passed. Disposable-stack browser confirmation remains pending. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Rotated the October 5 day block to [CHANGELOG-2026-10d.md](CHANGELOG-2026-10d.md), preserving
  the complete block and keeping the two newest day blocks active.

- Latest canonical checkpoint: `all_test` retry 33479 exited 0, including all 19 database selector
  groups and installation/appearance acceptance. The exact bounded Question Library filter/page
  selector ran its mixed-search matrix; focused Library browser proof passed Student Scores and
  search discard/reopen. The score projection evidence covers Student Scores/overview/History, not
  Instructor Gradebook. Broader browser and final integration remain pending. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md); earlier
  October 7 entries retain their status as recorded at those checkpoints.
- Final UI checkpoint: canonical production browser acceptance and the five-format real-grading to
  History/display probe passed, with separate fresh SPEC/QUALITY reviews and final client build.
  The probe establishes projected points and matching numeric ratios; it does not establish raw
  fraction persistence. Pool wording tests pass SPEC, with QUALITY pending. The corrected
  canonical screenshot retry is still running after the first attempt encountered obsolete
  controls. M29 remains active pending refreshed captures, visual and final integration review,
  temporary proof cleanup, and reconciliation of all 35 TODO mappings. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).

## 2026-10-06

### Behavior or Interface Changes

- Aligned Watch event number names across SQL, typed Rust, and JSON: Question events use
  `question_revision_number`/`questionRevisionNumber`, and Pool events use
  `question_pool_edit_number`/`questionPoolEditNumber`; the nullable JSON pair is selected by the
  existing target/event kind. No notification behavior changed. The PostgreSQL-feature integration
  target compiles; review and runtime acceptance remain pending.

- Simplified Citation to one optional plain-text value across authoring, publication, forks, Library,
  API, and UI; removed URL/format/content gates. Schema, generated contracts, Cargo, TypeScript, and
  11 citation-sensitive Node tests passed; implementation and bind reviews passed. M12 then exposed
  stale Blueprint/grant/visibility fixtures and Student-role fairness setup; corrections passed review.
  Retry 14 passed saved-response, citation, Pool, Blueprint, privacy, Watch, fairness, and owner/
  Sysadmin checks, then stopped on an undefined psql variable. Authority marker 11 and full acceptance
  remain pending. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Sysadmins can archive or restore any Published Question; Instructors remain limited to Questions
  they own, and Students remain denied. The existing availability Edit Number, exact-title archive
  confirmation, state checks, and actor audit remain in force. The registered connected oracle
  covers the role boundary and affordance; execution remains pending the coordinated runtime.
  See [Sysadmin Archive and Restore alignment](active_plans/reports/QUESTION_SPEC_SYSADMIN_ARCHIVE_ALIGNMENT.md).
- Fixed Sysadmin same-ID correction publication after retry 21 exposed the instructor-only Draft
  image actor predicate; Instructor and Sysadmin are now permitted with workspace-owner enforcement.
  Retries 23-25 exposed a test Course discipline, SQL column, and UUID/text bind mismatch; scoped
  corrections passed formatting, PostgreSQL-feature compile-only, diff, and distinct fresh SPEC/
  QUALITY reviews. Retry 26 (session 6882) passed its first 18 selectors and Course publication
  Pool ID/count assertions, then exposed provenance readback blocked by forced RLS. The test-only
  read now uses transaction-local `ple_api_owner`; focused checks and fresh SPEC/QUALITY passed.
  Retry 27 passed selectors/oracle; provision then failed HTTP 503, corrected by forwarding `question_title`.
  Saved-state logic distinguishes Draft-only `isSavedDraft` from Published-aware `isSaved`; publisher retains
  Draft predicate. Source reviews, focused lint/TypeScript/format/diff, and build 36264 (bundle `aa8c718d`) passed.
  Literal sync waits for `loaded`/`reloadSucceeded`; build 48634 (`60f3e9e1`) and checks passed.
  Reviews found no new unapproved feature; the proposed 1024 cap duplicates existing Pool cap (Class 2).
  Roster label: `Course access removed`; Pool permission cleanup passed SPEC/QUALITY without behavior changes.
  HOTSPOT HTTP 200 then app error in runs 44768/58958/76517; worker 7277 is diagnosing preview.
  Run 79670 Draft API passed; exact-ID search missed despite title hit; investigation open, no fix.
  Run 98770 release/Attempt checks passed; nested Question fixture failed; parser fix reviewed, rerun pending.
  Five scenarios scoped; docs 58517: 2,589 passed, 10 existing warnings; M28 partial; full acceptance pending.
- Corrected Draft autosave guards after drift review found numeric/hotspot/upload checks blocked
  unrelated metadata and support edits. Ordinary snapshots now save while invalid numeric input
  remains unsaved and navigation-guarded, with existing unsaved/navigation/publication checks intact.
  The ignored proof now confirms metadata and source readback after correcting its stale-status
  assertion and request-listener race. Temporary TypeScript and format checks passed; fresh SPEC
  `draft_autosave_readback_spec` and distinct QUALITY `draft_autosave_gate_quality` passed. A sampled
  source review of metadata, citation, Bloom, statistics, scoring, and import paths found no
  actionable new drift, without establishing whole-plan clearance. Browser runtime remains pending. No permanent test was added;
  raw unfinished numeric-literal persistence remains separately deferred.

- Registered the isolated `m23_available_statistics` browser scenario. It creates
  a new Course, publishes one Native Matching Revision and originating Pool, then
  records six ordinary Student deliveries with five graded stored-credit outcomes.
  The exact Revision and Pool assertions expect 6 received, 5 graded, 50% average,
  and 40% full/zero credit; a half-credit response still awards 0 of 1 Assessment
  points with partial-credit points disabled. Focused source checks and fresh SPEC
  and QUALITY reviews pass. Browser/database runtime remains unrun; see the
  [M23 browser preparation report](active_plans/reports/output_question_spec/M23_AVAILABLE_STATISTICS_BROWSER.md).

- Added the registered `library_scoring_evidence` M29 browser scenario for
  Student-visible partial-credit score and highest-Attempt changes, exact
  Question evidence withholding after real submissions, the combined Assessment
  Question/Pool picker, and confirmed Library search discard/reopen behavior.
  Focused Prettier, ESLint, and direct TypeScript checks pass; the broader lint-project
  typecheck reports diagnostics outside this scenario. Browser runtime remains pending.
  Underlying normalized-credit retention remains an M17 connected-oracle assertion,
  with connected execution still pending. The Question and Pool evidence displays
  assert the privacy-withheld state after real submissions. Numeric shared
  measures remain pending because the seeded Course has three Students, below
  the five-contributor privacy floor.

- Earlier M28 preparation described a synthetic CLI that was withdrawn before runtime. A later
  temporary readback passed for 8 generated Pilot IDs, owner/metadata/source bytes, and 4 ordered
  Native Blueprint Tuples. This is partial evidence: dependencies and asset/Pool/Theme readback
  remain pending, as does whole-milestone acceptance. See the [M28 status report](active_plans/reports/QUESTION_SPEC_M28_IMPLEMENTATION.md)
  and the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).

- Completed six fresh focused midplan reviews of the Question-spec implementation diff. Corrected
  stale FIB regex and PG/PGML authoring TODO wording to distinguish source implementation from
  connected acceptance; see the [midplan review](active_plans/reports/QUESTION_SPEC_MIDPLAN_CODE_REVIEW_2026_10_06.md).

- Registered the existing ignored raw Draft source PostgreSQL oracle in the
  canonical baseline before M03 Revision metadata. The source-only runtime gate
  handoff preserves existing selectors and records the missing cleanup receipt
  from the failed pre-oracle M09 install. No runtime acceptance is claimed; see
  [runtime gate preparation](active_plans/reports/QUESTION_SPEC_RUNTIME_GATE_PREPARATION.md).

- M16 adds saved-Draft preview and instructor testing for Native JSON, WebWork PG, and PGML. Each
  uses its stored backend/format/path binding; testing sends only the Draft Edit Number and transient
  response, with no Student Work or Published Revision identity. Owner/Sysadmin correction opens an
  ordinary Draft with the current exact source, support, and Native HOTSPOT image, then calls the
  existing `/publish-revision` operation. Focused Rust, TypeScript, and client checks pass. Connected
  renderer/database/browser acceptance remains pending; the existing revision handler still validates
  Native source only, so PG/PGML correction publication needs the M15/root handler handoff.

- Raw Draft source saves now retain unfinished Native JSON and PG/PGML bytes under the registered
  binding, including a valid Native HOTSPOT Type with an unresolved image reference. WebWork Type
  preserve, clear, and set now use explicit SQL parameters within one Draft Edit Number CAS. Both
  store saves now capture the full Draft projection and Edit Number inside that same transaction
  before releasing its row lock. Focused Rust tests, formatting, schema style, and diff checks pass;
  both PostgreSQL source/acknowledgement targets compile, while connected runtime acceptance remains
  pending. See the [M15 save acknowledgement report](active_plans/reports/QUESTION_SPEC_M15_SAVE_ACK.md).

- M29 browser acceptance source now covers current Draft creation and autosave/reopen, valid Native,
  PG, and PGML preview/testing, nullable Bloom metadata, Archive/Fork/search, ordinary Pool member
  editing and local sort invariance, Blueprint Theme inheritance, and confirmed Sysadmin data access.
  The Live Demo authoring journey no longer uses obsolete manual Save controls. Focused parsing,
  scenario-contract, formatting, lint, and TypeScript checks pass. Browser/runtime and screenshot
  acceptance remain unrun; the M15 exact-save acknowledgement and connected M09 Pool Save gates remain
  pending. See the [M29 browser acceptance preparation](active_plans/workstreams/question_spec_m29_browser_acceptance_prep.md).

- Registered the M29 valid Draft execution scenario for Native, PG, and PGML, including the shared
  renderer fixture, after-Type Edit Number checks, exact publication Revision Tuple readbacks, and
  post-refinement testing. Focused esbuild parsing/transpilation, provider Python compile, and six
  scenario-contract pytest checks pass. Browser/renderer runtime acceptance remains pending the
  parent-owned M15 exact-save acknowledgement correction and a coherent stack.

- M12 now checks Pool validity and requested item counts independently for each Assessment entry,
  accepts positive requested counts above current membership, and reports specific Pool release
  issues. The Assessment editor count maximum was removed in the M12-owned input handler. Final
  fresh static SPEC and different fresh static QUALITY reviews passed with no confirmed defects.
  Schema-style, Rust no-run, TypeScript, and focused Node checks pass (10/10); focused whitespace
  checks are clean. `M11_POOL_REFERENCE_READY` records source readiness only. M03 readiness and
  connected database/browser/integrated acceptance remain pending; see the
  [M12 release validation handoff](active_plans/reports/QUESTION_SPEC_M12_RELEASE_VALIDATION.md).

- Existing exact Pool Revision Tuples remain eligible when their Published Question is Archived;
  unavailable exact metadata is excluded from the valid-member count and remains a release issue.
  Assessment metadata corrections save normally and can make an affected Pool invalid. The M12
  lifecycle oracle covers tuple and Pool Edit Number preservation and excludes whole-Pool earned
  and possible points across Attempts. Root should rerun
  `source ./source_me.sh && ./tests/e2e/e2e_assessment_saved_response.sh` after M03 readiness and
  canonical runtime coordination; no connected execution or overall M12 acceptance is claimed.

- Question forks now use ordinary Draft and Published Question parent ID/Revision fields. Draft
  creation copies the exact source Revision's license, authors, metadata (including nullable Bloom),
  and content, reserves the new public Question ID, and keeps the Instructor who forked as the new
  Question owner. Publication uses locked Draft values and starts the new Question at Revision 1
  with that reserved ID. Forks of forks retain the immediate parent. Successful publication consumes
  the ordinary creation receipt; the Watch event is emitted once on Published Question creation.
  Parent API readers, focused model/decoder and publisher tests, and a registered PostgreSQL lifecycle
  oracle are in place. Focused Rust, TypeScript, Node, and
  schema-style checks pass. The oracle was not run; connected-test compilation and the server crate
  are blocked by concurrent Pool/Blueprint and WebWork contract errors. See the [M07 Question parent
  report](active_plans/reports/QUESTION_SPEC_M07_PARENTS.md).

- Archived Question detail now offers the ordinary exact-Revision Fork action to any Instructor.
  SQL accepts only exact `available` or `archived` source tuples under a lineage lock. Fork stays
  under the Instructor gate outside the available-only action callback; Pool creation remains
  availability-gated. At that checkpoint Restore remained owner-only; see the Oct 6 correction
  above. Focused UI evidence uses archive SSR plus a structural detail-action assertion. The
  registered PostgreSQL oracle compiles but is not run;
  connected PostgreSQL, browser, and runtime acceptance remain pending. M07 remains M15/M16 source
  readiness only, and M24 shared discovery confirmation remains pending.

- Ordinary Question Pool detail now displays exact members in the shared
  spreadsheet-style table with local sorting by title, Question ID, Revision,
  and license. Sorting copies display rows and leaves Pool membership and its
  Edit Number untouched; see the [M09 Pool display sorting report](active_plans/reports/QUESTION_SPEC_M09_DISPLAY_SORT.md).

- Ordinary Pool detail now renders the shared Statistics panel from the Pool's existing
  privacy-gated evidence. It explains committed deliveries and graded responses, shows average
  stored credit as a percentage, and uses graded responses for full-credit and zero-credit rates.
  Question detail keeps the same exact-Revision evidence and per-Revision breakdown. Focused
  TypeScript, Prettier, ESLint, and
  diff checks pass. M29 browser assertion updates and numeric proof with five distinct Student
  contributors remain pending; see the [M22 Pool statistics display fix](active_plans/reports/output_question_spec/M22_POOL_STATISTICS_DISPLAY_FIX.md).

- Added the Pool Questions draft editor for Pool Owners and Sysadmins. It adds eligible exact
  Revisions, saves the full unordered tuple set with its Edit Number, and retains drafts after save
  failure. Local sorting leaves membership/token unchanged; reload guards preserve edits. Focused
  Node/model/browser and Cargo checks pass, but broader authoring has one unrelated failure and the
  stubbed Save is not connected evidence. Fresh SPEC passed; QUALITY, connected DB/API/browser, and
  milestone acceptance remain pending. See the [M09 editor boundary](active_plans/reports/output_question_spec/M09_ORDINARY_POOL_EDITOR_BOUNDARY.md).

- M09 Pool Save permits the owning Instructor and any Sysadmin while retaining tuple, license,
  classification, concurrency, and no-op checks. The registered oracle checks locked ownership and
  the app-granted `ple_api` SECURITY DEFINER path. SPEC/QUALITY passed; database setup failed before
  the oracle, so connected acceptance remains open. See the
  [M09 permission report](active_plans/reports/QUESTION_SPEC_M09_POOL_SAVE_PERMISSION.md).

- Mixed Library search now uses `/api/library-objects/search`, preserving typed Question/Pool fields
  and applying Type, Backend, Author, and no-Pool filters across the correct result kinds. The
  Assessment picker shares search; Pool contents stays Question-only. Node (30), TypeScript, model,
  SPEC, and QUALITY checks passed. Server compilation is blocked by concurrent model mismatches;
  connected acceptance is queued. See the [M24 report](active_plans/reports/QUESTION_SPEC_M24_LIBRARY_RESULTS.md).

- Assessments now reference existing Pools directly and own their positive selection counts; removed
  exclusive associations, fork-only routes, and automatic fork-on-add. Assessment edits leave Pool
  ownership/membership unchanged. Focused fixtures cover shared Pools and retained Attempt tuples;
  connected PostgreSQL/browser acceptance remains pending. See the
  [M10 report](active_plans/reports/QUESTION_SPEC_M10_POOL_REFERENCES.md).

- Blueprint and Course operations now preserve ordinary Pool IDs and Assessment-local counts without
  pinning Pool Edit Number; release validates adequacy. Removed automatic Pool materialization and
  Blueprint member editing; explicit Pool forks copy current metadata, exact members, ownership, and
  lineage. Focused Node/schema checks passed; PostgreSQL and integration builds remain pending due to
  concurrent workspace errors. See the [M11 report](active_plans/reports/QUESTION_SPEC_M11_POOL_OPERATIONS.md).

- Removed manual Question/Pool impact notices across schema, SQL, permissions, Rust/API, UI, and tests.
  Watch retains events for Revisions, forks, and Pool edits; Change Proposals stay separate. Registered
  connected tests replace notice-only coverage. Canonical retry 19 reached the disposable PostgreSQL
  boundary but produced no result; fresh Watch SPEC/QUALITY reviews passed, and connected acceptance
  remains pending. See the
  [M14 report](active_plans/reports/QUESTION_SPEC_M14_NOTICES.md).

- Pools now store unordered exact Revision tuples, unique by Published Question. Saves advance Edit
  Number only when membership changes; Attempt positions remain local. Removed persisted ordering across
  layers while keeping sampler sorting and display-only table sorting. Focused model, Node, Rust,
  browser, TypeScript, schema, formatting, and diff checks passed. SPEC passed; QUALITY and connected
  Save acceptance remain pending. See [M09 order correction](active_plans/reports/QUESTION_SPEC_M09_ORDER_CORRECTION.md) and [storage report](active_plans/reports/QUESTION_SPEC_M09_POOL_STORAGE.md).

- Question Type now lives in each Revision's ordinary metadata snapshot and shares its metadata Edit
  Number. Native Type must match the immutable source interaction; WeBWorK Type can be corrected
  without a new Revision. Owner/Sysadmin metadata writes retain the same archive and CAS checks.
  Focused Rust compilation and schema style checks pass; connected PostgreSQL and generated-contract
  acceptance remain pending. See the [M06 permissions report](active_plans/reports/QUESTION_SPEC_M06_PERMISSIONS.md).

- Blueprints now store Theme as ordinary metadata under the Blueprint Edit Number, and Blueprint
  creation/forking carries the selected/source Theme. Adopting a Blueprint copies its persisted Theme
  once into the Course; later Course Theme changes remain independent. A focused client validator
  test and PostgreSQL lifecycle oracle are in place. Generated-contract, connected database, and live
  browser checks remain pending; see the [M27 Theme report](active_plans/reports/QUESTION_SPEC_M27_THEME.md).

- Question usage now counts committed presentations once, then records each stored Backend credit
  once at finalization. Statistics stay per exact Revision and use the Pool captured on the delivery
  receipt; Pool outcomes no longer follow current membership. The Library panel shows received,
  graded, mean credit, full-credit %, and zero-credit %. Decoder checks pass. The connected oracle
  now exercises production presentation and finalization for direct and Pool-selected Questions,
  including retry, fractional credit, disabled partial-credit points, and Pool member replacement
  after delivery. The focused Question model suite passes 4/4. The fixture has not run. Fresh
  PostgreSQL and browser acceptance remain pending. See
  [statistics implementation report](active_plans/reports/QUESTION_SPEC_M22_M23_STATISTICS.md).

- Native FIB and MULTI-FIB now support explicit regular-expression matching while preserving
  accepted-answer lists and literal modes. MULTI-FIB awards equal credit per authored blank;
  omitted, wrong, and blank answers earn zero. Focused Rust and browser checks pass; generated
  TypeScript and connected storage/points/display acceptance remain pending. See the
  [M19 FIB report](active_plans/reports/QUESTION_SPEC_M19_FIB.md). Also removed
  lingering Draft revision-history and HTTP-import claims, and restored HG's archive confirmation.
  All 457 documentation checks pass; HG and its checklist agree on 1,204 bullets.

- Corrected the M17 Blueprint Course creation test to read Assessment defaults from the declared
  nested input shape while retaining the explicit `partialCreditEnabled: false` expectation. The
  focused `question_model` test and crate formatting check pass; see the
  [M17 Blueprint fixture report](active_plans/reports/QUESTION_SPEC_M17_BLUEPRINT_FIXTURE.md).


- Native PLE JSON now carries display and grading content without Question record metadata.
  Preview and issuance receive the exact Question Revision title from their authorized database
  projection, and trusted-import inputs contain only mapped source content. The focused adapter
  suite and learning-data-access type check pass; database projection acceptance remains pending
  the coordinated fresh-database run. See the [M04 native adapter report](active_plans/reports/question_spec_m04_native_adapter.md).

- Pilot and parameterized curriculum producers now place supplied metadata on ordinary Draft
  records while Native JSON stays content-only. Pilot tags and citations moved into its manifest,
  unknown parameterized language stays absent, and the in-repository QTI source conversion now
  calls the content-only import constructor. Focused producer checks pass; see the
  [M04 producer report](active_plans/reports/question_spec_m04_producers.md).

- The ordinary Question metadata editor now reads Title, Description, Tags, classification, and
  the metadata Edit Number from one current snapshot. The shared read supports Instructors and
  Sysadmins, and its SQL role gate retains Student denial. The single-object save and bulk update
  role checks are unchanged. Focused browser checks pass; connected PostgreSQL execution remains
  pending the shared fresh-database gate. See the [M03 read correction report](active_plans/reports/question_spec_m03_metadata_read_fix.md).

- Bloom now appears in ordinary Question and Pool metadata editors with independently nullable
  dimensions. Assessment sorting keeps incomplete classifications last in stable order, and Pool
  metadata uses its existing metadata Edit Number separately from tuple-set edits. The dedicated
  Bloom correction client and editor are removed. Focused Node checks (59) and the Owner/Sysadmin
  component browser harness pass; generated-contract TypeScript and connected Pool API checks remain
  pending. See the [M05 browser report](active_plans/reports/QUESTION_SPEC_M05_BROWSER.md).

- Ordinary Draft creation now presents Native JSON, WebWork PG, and WebWork PGML, with explicit
  backend/format/path binding and blank raw PG/PGML source. The raw source workspace reports the
  acknowledged Edit Number and offers retry or discard-and-reload after a failed save. Focused
  source/creation Node checks (11), scoped lint/format, and the application TypeScript check pass.
  Connected PG/PGML creation remains pending registration of the shared ordinary create route; its
  current handler still accepts Native JSON only.

### Fixes and Maintenance

- Added `POST /api/question-pools/{source_question_pool_id}/fork` as the explicit Instructor path
  for forking an ordinary reusable Question Pool. Scoped source and formatting checks pass; runtime
  acceptance remains pending.
- Aligned the shared Question JSON fixture media type and changed Watch source-classification setup to use the loaded fork Draft instead of a denied `ple_data` SELECT. Retry 16 exited 1 at that SELECT; the correction passed focused format, diff, PostgreSQL-feature compile-only, fresh SPEC, and distinct fresh QUALITY checks. A Sysadmin fixture-count read now runs under transaction-local `ple_private_owner`; format, diff, PostgreSQL-feature compile-only, fresh SPEC, and distinct fresh QUALITY checks passed, with no grant or payload changes. Canonical retry 17 (session 45432) exited 1 after Question Library Watch passed and Pool member Watch failed on `SET LOCAL ROLE ple_auth` over the authenticated migration connection. The ordinary Store fixture correction and explicit Pool-fork application path passed scoped checks/reviews. Canonical retry 18 (session 35850) exited 1 after eight selectors passed, then failed compiling Blueprint Revision acceptance: `stewardship.rs:367` moves `forked_pool_id` and `source_pool_id`, reused at lines 370 and 383. The PostgreSQL-feature compile-only command reproduced this; an earlier check omitted that feature. Approved Watch field names and the owned test-clone fix are implemented; retry 19 later passed all three Watch selectors before failing at Sysadmin fixture inspection. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Corrected the Watch fixture for creating a Question fork after canonical retry 14 showed its source bytes
  did not match the bound object metadata. The fixture now copies the source checksum, size, and
  media type from that exact-bound object; generated-ID readback is unchanged. Formatting,
  PostgreSQL-feature compile-only, and diff checks passed. Reused SPEC `m03_metadata_api` and distinct
  reused independent QUALITY `fork_reservation_test_quality` both passed because fresh reviewer slots
  were unavailable. Connected runtime acceptance remains pending; see the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Removed the Watch fixture's direct SQL `CHECK`-shape probe. The existing PostgreSQL acceptance
  actions cover Human Guidance Watch delivery; the helper duplicated schema implementation detail
  and failed under the migration role's table permissions. The fork publication fixture now uses
  the Question ID reserved by its Draft instead of inventing a second identity.
- Corrected two PostgreSQL Sysadmin fixtures: the Question metadata test now uses a distinct
  UUID-derived token after retry 12 collided with the Course fixture's `0xc4` hash, and the
  correction publication test uses one transaction timestamp for session creation and expiry.
- Reworked the Sysadmin same-ID correction fixture lookup to reuse its admin pool transaction with
  transaction-local `ple_data_owner`; focused reviews and feature-enabled compile passed. Integrated
  retry 20 is still running, so runtime acceptance remains pending.
- Used one transaction timestamp for metadata-test Sysadmin session creation and expiry; separate
  `clock_timestamp()` calls could violate `updated_at >= created_at`.
- Aligned the Human Guidance implementation checklist and its development audit part with the
  current Human Guidance wording. The two new configuration principles remain pending an
  implementation audit; existing assessed statuses are preserved.
- Removed `FOR KEY SHARE` from immutable Question-fork license and citation reads: the API role
  can read them, but PostgreSQL locking reads require UPDATE privilege. Other locks remain. The
  diagnostic reproduced the permission error; schema style, scoped diff, SPEC, and QUALITY checks
  passed. Retry 9 then exposed ambiguous `created_at`, fixed below; runtime acceptance is pending.
  See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Renamed the fork function's local `created_at` to `fork_created_at` after retry 9 exposed
  ambiguity with authorship data. Schema style and fresh SPEC/QUALITY reviews passed; retry 10,
  browser checks, and broader M29 acceptance remain pending. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Kept the reserved-ID assertion under the fixture-owner role after retry 10 showed the migration
  pool cannot read `ple_private`. The test now reads the reservation in a transaction after
  `SET LOCAL ROLE ple_private_owner`; runtime privileges are unchanged and retry 10 awaits rerun.
  See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Expanded agent guidance to define robust behavior as context-sensitive graceful recovery and to
  keep configuration options, modes, and extension points grounded in demonstrated needs.
- Split the optional reserved-Draft UUID guard from the private lookup, preventing an unset UUID
  from reaching an unauthorized table read. Retry 4's reproduced setup defect is fixed; runtime
  rerun remains pending. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Corrected mixed Library fixture setup to insert initial Pool Bloom metadata directly, preserving
  NULL for empty Pools; retry 5 exposed an UPDATE that bypassed the metadata edit counter. Retry 6
  passed earlier cases, then found a missing Blueprint-fork SQL signature. Rust now sends a nullable
  text ID for SQL's existing public-ID trigger. Scoped formatting/schema/diff checks, focused
  SPEC/QUALITY reviews, and Cargo checks passed. The ignored image proof is not runtime evidence.
  Retry 7 and milestone/browser/whole-plan acceptance remain pending. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Corrected the fork fixture to use its stored Revision `published_at`; retry 7 exposed that
  independent timestamps violated the existing equality constraint. Formatting and diff checks
  passed; runtime rerun remains pending. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Fresh source-only SPEC and distinct QUALITY reviews passed for the fork timestamp fix. Retry 7
  remains failed; retry 8 is running. Focused documentation pytest (551) and diff check passed,
  and cleanup found no containers. Runtime, browser, lane 2/3, milestone, and whole-plan acceptance
  remain pending. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Aligned Rust row lookups with SQL's `draft_question_id` and `authoring_workspace_id` labels
  after retry 8 failed at Question-fork acceptance (SQLSTATE 42501); Rust domain names remain.
  This fixes the decode mismatch only. Error capture, runtime rerun, and scoped root review remain
  pending. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Pool Save proof 09 now asserts expected SQLSTATE categories and unchanged state, not exact error
  wording. Fresh SPEC/QUALITY reviews and focused M12 wrapper retry 25 passed; canonical retry 4
  stopped in Library setup and awaits the trigger fix. Full M12, M29, and browser proofs remain
  incomplete. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Removed unused `CourseInstancePoolIdIssuer` wiring, retaining `QuestionPoolIdIssuer`, and
  corrected the Blueprint import comment for direct Pool references. SPEC/QUALITY, Cargo checks,
  and PostgreSQL test compilation passed without warnings. The co-Instructor same-ID/owner runtime
  oracle remains pending; cleanup alone is not runtime acceptance. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Simplified four ignored browser proofs to assert required behavior without unsupported defaults,
  immediate autosave, exact response shape, or redundant Theme text. SPEC/QUALITY, Prettier, and 551
  documentation tests passed; ESLint ignored these files. No permanent tests were added. Browser
  runtime and canonical retry remain pending. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Fixed the Course creation receipt losing a Blueprint's Theme: the database already persisted the
  adopted Theme, but its creation function omitted Theme from the returned row and Rust hardcoded
  Grass in `CreatedCourseInstance`. Creation now returns the saved Theme and Rust decodes it with
  the existing Course Theme parser; Empty Courses retain the SQL `grass` default. Canonical retry 2
  exposed the failure (`Forest` persisted, `Grass` returned); fresh source-only SPEC
  `course_theme_response_spec` and distinct QUALITY `course_theme_response_quality` passed with no
  further defect found. Canonical retry 3 (session 37923) passed the Theme checks and independent
  Course edit, then exited 1 on the obsolete co-Instructor Pool-fork expectation; the corrected
  oracle rerun remains pending. See the [M27 report](active_plans/reports/QUESTION_SPEC_M27_THEME.md)
  and [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Corrected retry 3's obsolete co-Instructor oracle to expect the same adopted Pool ID and owner,
  consistent with direct Pool references. Theme and Course edit checks passed; rerun and M11 source
  reviews remain pending. Cleanup found no containers. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Aligned Blueprint Assessment Pool validation/persistence with Rust's `questionPoolId` and removed
  obsolete `sourceQuestionPoolId` normalization. The canonical run exposed the stale SQL key after
  five finalization cases passed; retry 2 then exposed the Course Theme response defect fixed above.
  Focused reviews passed, M12 retry 23 passed, and retry 3 plus later lanes remain pending. Removed
  stale SQL fork locals/comments without behavior or privilege changes. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Updated saved-response wrappers for the current direct-Pool-reference notice. Retry 22 passed SQL
  proofs through fairness, then failed on the stale notice name; focused metadata, release, and
  fairness proofs passed separately. Wrapper correction awaits fresh rerun; full acceptance did not
  start. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Corrected citation authoring bind counts to match the 17- and 14-argument SQL functions. Review
  found the mismatch despite prior Cargo checks. Retry 11 exposed an old grant signature; its grant
  correction preserved privileges and passed SPEC/QUALITY. Retry 12 found stale Blueprint visibility;
  retry 13 found a fairness save running as Student, now corrected. Retry 14 passed saved-response,
  citation, Pool, Blueprint, privacy, Watch, fairness, and owner/Sysadmin checks, then stopped on an
  undefined psql variable in authority marker 11. Canonical acceptance did not start. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Repaired the deferred Pool bulk metadata procedure under `ple_private_owner`, restoring its grants
  and filtering to the active Instructor's Pools before updates. Added an ignored nonowner-denial
  assertion. Instructor owner editing remains; Sysadmin/broader bulk editing stays deferred. Schema,
  diff, SPEC, and QUALITY checks passed; runtime remains pending. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Fixed retry 7's stale `pool_metadata` test variable by reading current public metadata before the
  Tags assertion and setting up checksum helpers as `ple_api_owner`. The failure was in the oracle,
  not evidence of volatile or lost writes. Review and runtime acceptance remain pending; deferred
  bulk/Sysadmin scope is unchanged. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Added the missing citation read/write permissions for existing private-owner readers and
  publication writers. Retry 9 confirmed citation access and metadata notice, then exposed that Pool
  reads return one row per Question while the Assessment oracle counts Entries. Assertions now count
  distinct `assessment_entry_id` values without changing SQL. SPEC/QUALITY and retry 10 remain
  pending; authority proof was not reached. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Changed the Pool metadata API to accept Bloom as text and convert it inside the `SECURITY DEFINER`
  wrapper, removing `ple_app`'s private-schema access need. Rust/public SQL pass nullable text; the
  private enum function retains validation and updates. Schema, formatting, and diff checks passed;
  fresh SPEC/QUALITY and runtime reviews remain pending. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Pool-support SPEC/QUALITY and TypeScript, Rust, schema, and diff checks passed. Retry 5 then failed
  because `ple_app` could not cast Bloom through `ple_data`; a read-only diagnosis is ongoing, so the
  cause is unconfirmed. Authority proof was not reached; M12, M29, and canonical acceptance remain
  incomplete. Direct Sysadmin HTTP-route coverage is still a nonblocking gap. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Clarified Question Pool membership as an unordered set of Question Revision Tuples, with one
  Question ID per Pool and Pool properties defined separately from Published Question properties.
- Focused Unrelease retry 8 passed its connected oracle, post-release content, and schedule-boundary
  checks. M12 retries then exposed stale lineage, license, permission, and authority-proof fixtures;
  the agreed Pool boundary is shared reads for Instructors/Sysadmins and owner/Sysadmin writes. SQL,
  server, UI, and oracle corrections are reported complete, but fresh review/runtime proof and full
  canonical acceptance remain pending. Documentation checks passed (1,239 HG bullets matched); no
  generated schema output is claimed. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Removed the unused private `instructor` helper from the question-pool library routes. Active
  `library_reader` authentication and pool ownership checks remain in place; see the
  detailed local ignored report at `output_question_spec/POOL_ROUTE_DEAD_HELPER_CLEANUP.md`.
- Corrected grading lifecycle fixtures that used an unknown Native JSON field or violated source
  binding; assertions are unchanged. Test compilation and fresh setup SPEC/QUALITY reviews passed.
  Full acceptance retry is active; runtime outcome remains pending.
- M12 retry 15 passed the Pool-support authority proof, then found an undefined `shared_pool_id`
  psql variable. The audited correction uses `created_pool_id`; review is pending. Wrapper exit status
  is unknown because `tee` lacks `pipefail`; cleanup found no containers and acceptance remains
  incomplete. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- M12 retry 16 passed authority and release-size checks, then hit a fabricated partial-metadata
  fixture not required by Human Guidance. Removed that block and dependent notices, retaining ordinary
  release/count and metadata-correction checks; remaining psql variables were resolved. True exit 3;
  trimmed proof awaits review and acceptance remains incomplete. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- M12 retry 17 passed current markers and oversized-request blocking, then failed Archive preservation.
  The proof had not established a clean issue baseline; cause remained under diagnosis with no
  workaround. Exit 3 was captured and cleanup completed; M12/canonical acceptance remain incomplete.
  See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- A read-only probe traced retry 17's Archive failure to an earlier fork oracle that reduced the
  shared Pool to one member. Setup now restores both tuples with owner-authorized Save. Review passed;
  retry 19 verified the restored Pool and Archive/restore preservation, then found an unresolved Bloom
  cast permission failure. No workaround was applied; M12/canonical acceptance remain incomplete.
  See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- M12 retry 20 passed ordinary metadata Save after the text-cast fix, then failed a nested public
  Pool-issue assertion whose differing JSON field is unconfirmed. The ignored oracle now checks Pool
  ID and each specific issue while retaining the blocking-release assertion; rerun is pending. A
  separate permanent oracle cleanup passed review and focused runtime proof. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- M12 retry 21 passed release validation, metadata correction, release, and the omission case, then
  failed the fairness proof because a private read ran as `ple_data_owner`. The ignored proof now uses
  the authorized roles for public Entry, private Student Work, and Student history reads. No production
  policy or permanent test changed; rerun remains pending. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Current status: M01 and M02 are accepted on their literal criteria. Initial publication and exact-ID search passed in `output_question_spec/authoring_exact_id_verified_20261007.log`; successor publication returned HTTP 422, and `output_question_spec/authoring_successor_diagnostic_20261007.log` identifies the Native create `type: None` bug. The create fix has 7 focused tests and fresh SPEC/QUALITY passes. The image build is running and not deployed; rebuild and restart the current source before proving create inference. The latest Native run passed licensed initial publication and detail assertions, then failed the existing published-image GET with 404 (run 26741; direct runtime result). Sysadmin access and Invitation Export dry run passed in `output_question_spec/support_access_verified_20261007.log` and `output_question_spec/invitation_export_verified_20261007.log`; Course seed exited 0 with no output. Rust full gate passed before the create fix in `output_question_spec/rust_check_retry_20261007.log`; Node passed (594) in `output_question_spec/codebase_check_20261007.log`; the full pytest run recorded 11,156 passes and one target-budget failure in `output_question_spec/pytest_20261007.log`, followed by a separate focused target-budget pass after cleanup. Schema generation/style and the 1,242-bullet Human Guidance checklist passed; target size is 9.55 GiB. Pool, Theme, and Statistics fixture corrections are under validation, and runtime reruns are incomplete. M01 and M02 remain accepted; overall TODO/evidence reconciliation is pending and does not reopen either acceptance. The latest drift review rejected unnecessary autosave gates, unrelated test-strengthening/coverage gates, and generic diagnostic redaction; the earlier autosave correction remains a separate implementation correction under existing Human Guidance. No new product features are authorized. Later milestones and overall plan acceptance remain pending. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Removed the unapproved published-Question fixture and moved its 12 Node consumers to local inputs;
  deleted obsolete build stages. Node, TypeScript, Cargo, formatting, native-response and full codebase
  checks passed, as did fixture cleanup reviews and the Markdown link check (549 tests). Browser WASM
  remains pending.
- Fixed five data-owner helpers after statistics INSERT denial in `presentation_private`; SPEC and
  QUALITY passed. Retry 31886 cleared schema/security, presentation, and idempotent resume, then a
  second presentation failed the `question_attempt` check. Cause remains under investigation; no
  integrated acceptance is claimed. See the linked retry log.
- Retry 94027 passed schema/security, presentation, and idempotent resume but did not reach grading.
  Baseline session 1249 passed five direct-grading cases; its next removal step was correctly blocked
  because an available Assessment still used the Question. Oracle ordering was corrected and passed
  SPEC; QUALITY/runtime and overall acceptance remain pending.
- Focused retry 3 passed schema/security and presentation/resume, then found a `ple_app` read of a
  private timestamp table. The oracle now captures timestamps as the private owner; SPEC/QUALITY
  passed and retry 4 started. M05 documentation review passed, but no build/runtime acceptance is
  claimed. See the linked retry log.
- Focused retry 4 passed schema/security, presentation/resume, both finalizations, partial-credit, and
  statistics, then found that released-Assessment saves wrongly reapplied full-content validation
  during sole-Pool retirement. The source fix was assigned; focused M03/M16/M17 reviews and a separate
  five-case grading check passed. Combined M12/M17/M29 acceptance remained pending. See the linked
  retry log.
- The final documentation review for M05 passed on current APIs, generated DTOs, and nullable Bloom
  schema. This is a documentation result; M05 connected Pool API/runtime behavior remains pending.
  At that checkpoint M17 SPEC had passed and its QUALITY review was active.
- Focused retry 5 passed schema/security, presentation/resume, grading, statistics, and sole-Pool
  retirement, then failed Pool replacement on immediate constraint timing. A deferred-transaction
  correction was assigned. Schedule and Sysadmin browser extensions were source-reviewed but not run;
  combined M12 and overall runtime/browser acceptance remain pending. See the linked retry log.
- Retry 6 passed base, post-retirement persistence, rollback, and zero-point checks; its final case
  lacked a Pool entry and failed `questions_required`. Test and production fixes for Pool references,
  kind guard, and fixture Type passed SPEC/QUALITY; combined retry 8 and later acceptance remain
  pending. See the linked retry log.
- Canonical focused retry 7 passed schema/security, the base oracle, lock race, and post-Unrelease
  Student-start rejection; cleanup was clear. This accepts only focused Unrelease behavior. Browser
  and integrated acceptance remain pending. See the linked retry log.
- Combined M12 proof passed installation, saved-response, and Blueprint visibility, then failed setup
  because the fixture inserted absent `question_type`. Corrected fixture passed SPEC/QUALITY; rerun is
  queued and M12 is not accepted. See the linked retry log.
- Corrected M12 retry 23 passed proofs 01-11 and the permanent wrapper after replacing a stale notice
  marker; focused reviews and cleanup passed. This is focused M12 evidence only; browser dependencies
  and full-plan acceptance remain pending. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Saved-response retry 24 passed SQL checks and wrapper. Runners now use `ON_ERROR_STOP` and oracle
  exit status instead of grepping success text. SPEC/QUALITY and shell syntax passed; cleanup found
  no containers. Broader M12/browser and full-plan acceptance remain pending. See the
  [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).
- Canonical retry 8 passed earlier cases and fork timestamp setup, then returned `StoreError::Forbidden`;
  the underlying SQLSTATE was not confirmed. Rust decoding now uses SQL's result labels; SPEC/QUALITY
  and Cargo checks passed. Error capture and a new three-lane run remain pending; product permissions
  are unchanged. See the [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).

- Corrected the M06 WeBWorK Type tag gate and editor: all eight defined labels, including
  `hotspot`, now pass as ordinary metadata while Native Type stays source-bound. The focused
  Owner/Sysadmin browser regression and metadata client test pass, and fresh SPEC/QUALITY reviews
  found no in-scope blockers. Connected PostgreSQL remains pending the root batch; test-target
  compilation, workspace TypeScript, and schema-style checks encounter unrelated in-progress
  `selection_rule`, `QuestionSummary`, and Draft-table findings.
  See the [M06 correction report](active_plans/reports/QUESTION_SPEC_M06_PERMISSIONS.md).

- M16 same-ID correction publication now carries the saved manual WebWork Draft Type into successor
  Question metadata while retaining Native source-derived Type. A focused PostgreSQL regression
  covers unchanged and deliberately changed Draft Types, the exact parent Revision, and stable
  Question ownership. Fresh SPEC and QUALITY reviews accept the scoped change. Rust compile-only,
  formatting, and schema-style checks pass. Connected execution awaits the disposable PostgreSQL
  runtime. See the
  [M16 manual Type correction report](active_plans/reports/QUESTION_SPEC_M16_MANUAL_TYPE_CORRECTION.md).

- Corrected M05 browser decoding against the generated 416-contract snapshot: Pool metadata now
  validates both independently nullable Bloom dimensions, Pool detail decodes `canEditMetadata`,
  and Assessment sorting keeps incomplete pairs last in stable order. The focused 59-test slice has
  56 passes and three unrelated M24 search failures; TypeScript checks retain only M24 search
  diagnostics. See the [M05 browser type fix report](active_plans/reports/QUESTION_SPEC_M05_BROWSER_TYPE_FIX.md).

- Rotated the complete October 4 changelog block to
  [CHANGELOG-2026-10c.md](CHANGELOG-2026-10c.md), keeping October 5 and 6 active.

- Corrected the root M12 backend diagnosis records: missing metadata leaves the Revision backend
  known, while a missing exact Revision is invalid under the member foreign key. The current helper
  remains correct and the proposed root helper edit is closed. Oracle 08 already asserts missing-
  metadata behavior; it does not separately assert backend-mismatch absence. The helper ACL and
  checksum-owner evidence remain recorded. Fresh read-only SPEC and separate QUALITY reviews agree;
  no source, schema, oracle, test, or runtime changes were made. See [M12 release validation handoff](active_plans/reports/QUESTION_SPEC_M12_RELEASE_VALIDATION.md)
  and [root schema-owner and M12 helper report](active_plans/reports/output_question_spec/schema_owner_and_m12_helper_fix_cli.md).

### Removals and Deprecations

- Withdrew the one-time M28 synthetic readback helper from project tooling before connected
  execution, and removed its block from the installation-data E2E. Ordinary Pilot and curriculum
  import paths plus Live Demo provision, replay, and Course/Assessment seed readback remain. The
  unrun Question source, metadata/ownership, Pool, and Blueprint readbacks remain outside current
  installation acceptance; M28 stays unaccepted.

- M25 removes persistent search prompt/result storage and retains temporary search-state behavior.
  The six model search tests and `text_match_modes` serialization test pass in the single
  `question_model --lib` run (165 executed, 164 passed); the package run remains red on the separate
  issued-Question identity test, whose supposed-valid ID fails canonical Crockford Base32 validation.
  Fresh-database and runtime pagination acceptance remain pending. See the
  [M25 search storage report](active_plans/reports/QUESTION_SPEC_M25_SEARCH_STORAGE.md).

### Developer Tests and Notes

- Prepared the M28 supported-import boundary with canonical Pilot/curriculum/demo caller paths,
  authenticated readback routes, and a focused connected acceptance insertion point. Fresh SPEC
  and QUALITY reviews pass; no tests or runtime were run. M07/M11 readiness markers are absent,
  while M08 database execution, M16 shared Draft/preview testing, and M27 connected Theme adoption
  and browser evidence remain pending. M28 is unaccepted. See the [M28 boundary](active_plans/reports/output_question_spec/M28_BOUNDARY.md)
  and [implementation ledger](active_plans/reports/question_spec_implementation_ledger.md).

- Added a connected M28 synthetic acceptance helper and an installation-data lane hook. The helper
  uses ordinary authenticated Draft/source-binding/Question/Pool/Blueprint/Course operations, reads
  immutable source and image bytes through ObjectStore, and emits generated identities for same-cookie
  API readbacks. Shell assertions cover Question ownership and metadata, exact Pool/Blueprint references,
  image delivery, and inherited Ocean appearance. Formatting passed for the new Rust helper and shell
  syntax passed; focused Cargo checks were blocked by shared `learning-data-access` compile errors. The
  connected lane and dependency/runtime acceptance remain pending. See the
  [M28 implementation report](active_plans/reports/QUESTION_SPEC_M28_IMPLEMENTATION.md).

- Registered a focused ignored PostgreSQL acceptance test for ordinary Course-to-Blueprint
  publication. It verifies that persisted Blueprint content retains the Assessment's direct Pool
  ID and requested selection count, without changing source ownership or creating a source-linked
  Pool child. The focused target compiles; the exact selector stops before database connection because
  the acceptance manifest and database URL environment variables are unset. See the
  [M11 Course publication acceptance report](active_plans/reports/output_question_spec/M11_COURSE_BLUEPRINT_ACCEPTANCE.md).

- Prepared the unregistered M15 common Draft creation route and focused request tests for Native JSON,
  WebWork PG, and PGML. `rustfmt`, scoped diff checks, and the existing persistence media/path binding
  test pass; module compilation awaits route registration, and connected PostgreSQL, Object Store,
  HTTP, and browser acceptance remain pending. See the
  [M15_M16_BOUNDARY.md](active_plans/reports/output_question_spec/M15_M16_BOUNDARY.md).
