# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was
> changed and believed at the time. They are not product authority. Current
> intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old
> Assignment, Blueprint, lifecycle, grading, role, retention, and UI models.

## 2026-09-16

> September 15 entries are archived in [CHANGELOG-2026-09k.md](CHANGELOG-2026-09k.md).

### Behavior or Interface Changes

- Recorded the shared Library classification model plus bounded Question and Pool backend evidence
  without closing a Human Guidance row. Optional UUID hierarchy identity and the cross-Discipline
  Subject flag are preserved through strict chain/cursor contracts; Question matching uses current
  lineage classification, and Pool matching uses Pool-owned metadata before ordering, `LIMIT`,
  lookahead, and continuation. Root generated 370 TypeScript types; five model tests (session
  44587), Cargo check, 13 Question tests, and two Pool tests (session 75888) passed. A fresh
  network-none PostgreSQL 17 actual-role proof passed and its labelled disposable container was
  removed. Root `cargo check -p server_core -p project-tools --tests` later passed in 3.87 seconds
  with no warnings; five model, 13 Question Rust, two Pool Rust, 24 focused Node, and strict
  TypeScript gates passed. Pytest session 48126 passed 7,532 tests in 5.58 seconds after
  invalid-URL recovery and Pool wiring. Question classification and malformed-URL recovery have
  accepted isolated actual-component/router/fake-client evidence. Pool discovery has accepted
  isolated actual `LibraryPage` fixture-browser evidence. There is no HTTP, deployed, or connected acceptance;
  the connected Library remains name-only pending current-source acceptance. Pool text/Tags filtering
  remains unavailable, and distinct Question/Pool modes require neither a mixed engine nor a shared
  cursor. Inventory remains 999 occurrences: 450 verified, 504 open, and 45 N/A. Receipts:
  `/private/tmp/ple-library-search-backend-evidence-receipt-20260916.md` and
  `/private/tmp/ple-library-ui-batch-evidence-receipt-20260916.md`.

- Clarified the native response terminology: a response action saves or changes that response
  (`Save response`, `Restore initial response`), while Assessment submission is the whole-Attempt
  action. This is terminology-only; no code, Human Guidance, or runtime behavior changed. Receipt:
  `/private/tmp/ple-response-terminology-positive-20260916.md`.

- Recorded the implemented, Blueprint-only shared-classification search slice without closing its
  broad Human Guidance rows. Public Blueprint Search now combines ordinary text with optional
  Discipline -> Subject -> Topic -> Subtopic identities and an explicit cross-Discipline Subject
  option; hierarchy state, applied-search retry/pagination, and cursor binding are covered by
  source, isolated actual-role SQL, and actual-component evidence. Root Cargo session 31251,
  focused Rust session 56442, strict TypeScript plus 21 Node tests session 19414, and full pytest
  session 39220 (7,462 passed) passed. Live `8147` predates this source: connected HTTP/browser
  integration, real vocabulary-parent validation, and authorization remain pending. Library
  search remains name-based; Tags, sorting, result metadata, and return-state work are untouched.
  Receipt: `/private/tmp/ple-classification-search-pool-receipt-20260916.md`.

- Fixed Question Pool review metadata controls to use their available review-fieldset width while
  retaining visible required labels. Isolated actual-component plus parent-host evidence at 1280px
  retained Title/Description through picker return, showed the focused Pool task, and found no
  horizontal overflow; it is not an authenticated full `LibraryPage` mount or an I09 closure.
  Receipt: `/private/tmp/ple-classification-search-pool-receipt-20260916.md`.

- Recorded locally implemented Blueprint Promoted evidence without closing either Human Guidance
  row. Source adds searchable lineage metadata, cursor-bound promoted-only discovery, Sysadmin-only
  metadata-ETag/CAS mutation, and the Public Blueprint Search filter. Canonical PostgreSQL 17
  bootstrap/install as `ple_migrator` and isolated actual-role SQL proof passed (`BEGIN`, `PASS`,
  `DO`, `ROLLBACK`, exit 0); its exact labelled disposable container was removed. Root Cargo
  session 60804 passed in 7.74 seconds, stricter TypeScript plus 11 Blueprint-client Node tests
  session 65918 passed, and pytest session 36484 passed 7,462 tests in 5.55 seconds. Live `8147`
  predates the source, so deployed HTTP/browser integration and connected current-source evidence
  remain unverified. Generated inventory remains 996 occurrences: 450 verified, 503 open, and 43
  N/A. Receipt: `/private/tmp/ple-blueprint-promotion-receipt-20260916.md`.

- Corrected `docs/BLOOM_TAXONOMY_GUIDE.md` to match Human Guidance: AI assigns both Bloom
  dimensions before a Question or Pool Revision enters the Library, and a Pool is classified as a
  whole. Root's stricter TypeScript gate, 16 focused history/disclosure/navigation Node tests, and
  full pytest session 60790 passed (7,462 tests, 5.42 seconds) after the temporary review-density
  harness moved outside `tests/`. This changes no Bloom implementation and does not establish
  deployed demo `8147`, connected Student proof, or global Human Guidance closure. Receipt:
  `/private/tmp/ple-parallel-batch-receipt-20260916.md`.

- Compacted submitted Student Assessment history into clearly separated Question units with
  wrapping submission/result/points metadata and a labeled recorded response. Accepted isolated
  actual-component browser proof at 1280 and 390 preserves permitted result information,
  withholds protected grade and teaching fields, contains long prose and code, retains Unanswered,
  and preserves keyboard Return access without document overflow. The source review confirms that
  server-projected disclosure conditions are unchanged. This is not deployed demo `8147`,
  connected authorization or Coursework-settings acceptance, or global Human Guidance closure.
  Receipts: `/private/tmp/ple-parallel-review-density-20260916.md` and
  `/private/tmp/ple-parallel-summary-density-review-20260916.md`.

- Excluded the Human Guidance How-to-use and Product vocabulary/glossary metadata subtrees from
  the implementation-checklist inventory while retaining their interpretation authority. Current
  parser-derived totals are 996 occurrences: 450 verified, 503 open, and 43 N/A. This bookkeeping
  change leaves all remaining checklist evidence and ownership rows unchanged; it is not
  implementation progress or a Human Guidance closure. Receipt:
  `/private/tmp/ple-checklist-meta-exclusion-20260916.md`.

- Refined Student Question navigation keyboard focus to use PLE's dedicated focus token, distinct
  from the current-Question accent. Existing compact first/current-range/last pagination remains
  unchanged. Temporary actual-component 250-Question proof passed at 390/1280px with width
  adaptation, saved/current cues, all Questions reachable by keyboard, and no document overflow;
  the three focused navigation tests, scoped ESLint, and diff whitespace gate passed. Broader
  Coursework journey acceptance remains separate. Receipt:
  `/private/tmp/ple-parallel-nav-finish-20260916.md`.

- Corrected the shared native-response reset label to `Restore initial response`: every default
  reset control restores the response captured when its component mounted, rather than deleting a
  saved response. Ordering retains its explicit `Reset order` label. This changes no response
  handler, validation, autosave, store/API, or submission behavior; a separate clear/delete
  contract does not exist. Temporary isolated component evidence is recorded in
  `/private/tmp/ple-response-restore-label.md`. The broader Student Coursework action-label row
  remains open.

- Recorded the reviewed raster-first HOTSPOT implementation without closing either HOTSPOT Human
  Guidance row. Draft owner/CAS upload, source binding, publication preparation, numeric regions,
  and the PLE-owned Student image pointer/keyboard control have focused source, Rust, TypeScript,
  PostgreSQL, and actual-role receipts. Root Cargo session 32160, pytest session 36235 (7,462
  passed), and TypeScript session 35906 passed. Current demo `8147` is pre-HOTSPOT; normal
  author/save/publish, worker Ready activation, Student grading, rendered pointer/keyboard proof,
  and screenshots remain unproved, SVG remains unimplemented, and no browser authorization or PNG
  publication was granted. Receipt: `/private/tmp/ple-hotspot-implementation-evidence-receipt.md`.

- Corrected the Question Pool review task identity at source level: confirmation now focuses a
  primary Pool heading, suppresses the competing Library chrome, and presents ordered Questions,
  Title/Description, the interchangeability explanation, attestation, and next action compactly.
  The existing picker remains the search/filter path; ordered selection and metadata survive a
  retry. The screenshot scenario now expects the review's primary heading. TypeScript, formatting,
  and focused request tests passed. Browser/rendered verification remains pending against frozen
  demo `8147`; this does not close I09 or any Human Guidance requirement. Receipt:
  `/private/tmp/ple-pool-review-compact-task-identity.md`.

- Recorded root-supplied canonical build 51649 (`Ready`, `8147`) and connected Pool metadata
  HTTP/browser acceptance, plus no-workaround Course/Blueprint selector proof (85118 exit 0).
  Combined independently reviewed source and accepted actual-role SQL/concurrency with these named
  connected receipts to close only nine owning requirements: four independent Pool metadata and
  five bounded Course classification rows. Generated coverage is 1,038 occurrences: 450 verified,
  538 open, 50 N/A; first owners are 441 verified, 524 open, 50 N/A. Full shared classification,
  Pool append/publication metadata/Bloom/discovery/delivery, and global compliance remain open.
  Preserved pre-HOTSPOT pytest snapshot, failed unpublished Student capture, and pending new live
  support TLS acceptance boundaries. All nine part gates, identity diff, and consistency passed;
  focused Markdown/ASCII/whitespace checks passed 2,589 tests and diff whitespace passed.
  Receipt: `/private/tmp/ple-connected-pool-classification-receipt.md`.

- Reconciled current Human Guidance checklist evidence without closing requirements. Replaced
  obsolete Pool-metadata absence claims with bounded actual-role SQL/source receipts and named
  rebuilt HTTP/browser proof still pending; retained separate attribution/support gaps. Recorded
  frozen-source support E2E session 38897 PASS and the exact pre-build image-prune receipt without
  broad support, runtime-rebuild, or global compliance claims. Restored verbatim generated link
  targets; changed only HG's BIOME target to the equivalent `/docs/BIOME_THEME_PALETTES.md` so
  copied bullets remain navigable without generator normalization. Identity diff/consistency
  pass for 1,038 occurrences; focused Markdown/ASCII/whitespace checks passed 2,589 tests.

- Recorded the bounded connected-progress receipt. Canonical Live Demo `2914` reached `Ready` and
  the old Course-projection failure is fixed. Actual-role Course browser proof passed
  create/history, exact metadata, unchanged Revision, name drafts, and Course-Instance source
  independence with an explicit Biology reselection workaround. Tags-only async selection remains
  defective in old `8075`; the source correction records four blank selections before and four
  correct Tags-only selections after, with save, parent-clear, and blank validation passing. Pool
  metadata source work spans SQL, Rust, API, and browser metadata but is not in running `8075`;
  HTTP/browser acceptance remains pending. SQL, final Rust/API, and browser re-reviews passed.
  Fresh PostgreSQL 17 install, role/rollback, and two-client wait/commit proofs passed. `ple_app`
  accepted 65 Tags, and the arbitrary 64-Tag cap was removed while per-Tag bounds remain. Root
  Cargo check passed in 9.61 seconds, `cargo tsgen` generated 370 types, and 348 Node tests passed.
  After three narrower test-lane repairs, full `python3 -m pytest tests/ -q` session 22889 terminally
  exited 0 with 7,462 passed in 6.69 seconds; live TLS acceptance and canonical rebuild remain
  pending. The 75-entry screenshot manifest publishes no PNG; HOTSPOT authoring
  ingest/picker/publication remains an implementation gap, and support E2E remains unaccepted
  while Morgan MFA helper wiring is repaired. No Human Guidance status, checklist count, or
  SQL-lockdown claim follows. Receipt: `/private/tmp/ple-pool-connected-progress-receipt.md`.

- Recorded the pre-build runtime-image cleanup boundary. API, gateway, database-migrator, and
  renderer builds share a checkout-directory lease with cleanup. Cleanup removes only untagged,
  undigested, project-source-labeled images that no running or stopped container references, using
  exact non-forcing `podman image rm --no-prune <id>` calls. It retains named inputs and outputs,
  builder cache, unknown ownership, and container-referenced images; broad post-ready pruning is
  removed. Actual cleanup removed 86 images with status 0, reducing image inventory from 169 to 83
  and reported image storage from 34.16 GB to 33.03 GB; 49 containers and 39 volumes were
  unchanged. No four-boundary rebuild has started, so this does not claim a ready runtime. Root
  `pytest` passed 7,448 tests in 5.47 seconds, `git diff --check` passed, and `cargo check -p
  project-tools --tests` passed in 3.32 seconds after the strict Course DTO consumer repair.

- Recorded the interim runtime and full-corpus evidence boundary. The independently accepted
  Draft classification-name component proof is at
  `/private/tmp/ple-classification-preserve-names.md`. The Public owner-editability correction has
  source/SQL/HG alignment, nine focused passing tests, and accepted independent review. The
  installation Live Demo/oracle malformed `DO $` repair and extracted `ple_migrator` provisioned-
  vocabulary block passed. The earlier canonical demo stopped before `Ready`; retry root process
  23160 terminally failed Course projection and is not accepted. The strict Course DTO consumer
  repair and independent review passed, but its next rebuild has not started. Root Node tests
  (345/345) and `npx tsc --noEmit` passed. The 7,443-test pytest receipt predates the latest
  manifest preparation and is not a current aggregate claim. The Public Blueprint Search scenario
  is present in the 59-entry
  manifest/atlas source but has no PNG; static verification correctly fails for that absence, so
  the current corpus is not refreshed. The disposable PostgreSQL proof container was removed and
  only logs remain. No Human Guidance-status or SQL-lockdown claim follows. Receipt:
  `/private/tmp/ple-runtime-full-capture-receipt.md`.

- Recorded bounded current-progress evidence: root `CARGO_INCREMENTAL=0 cargo check -p
  server_core -p project-tools --tests` passed in 11.37 seconds, `cargo tsgen` generated 369
  types, and a repeated fresh PostgreSQL 17 canonical install through `ple_migrator` passed. These
  do not establish all SQL or whole-feature acceptance. Self-contained actual-role
  Course-classification proof passed create/authorization, ETag no-op/stale/history, no-Revision,
  exact-pin, 65-hierarchy-tag, and fork/Instance-independence checks. The corrected actual-role
  support proof passed constraints and rollback; independent review accepted source and proof.
  This is Student-roster-only support evidence: Course/content remain open, and durable HTTP
  support E2E did not run while helpers are fixed, so no browser or deployed acceptance follows.
  Independent source/render review accepted compact Student S04/S05 details at 1280 and 390 pixels
  (`/private/tmp/ple-student-rules-disclosure-review.md`); this component receipt does not establish
  full shell, theme, zoom, or all-Student-interface acceptance.

- Split requested user-owned files without behavior changes: `question_model` Blueprint Course
  code now separates `assessment_content.rs` (291 lines) from `blueprint_course.rs` (738 lines),
  and the TypeScript workspace public surface now separates list (265 lines), detail (807 lines),
  and types (12 lines). The root source-line-limit gate passed at 1,350. Root Rust and TypeScript
  checks remain in progress. A separate classification-name Draft repair is underway after the
  split and is not part of this refactor.

- Reconciled the full [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) against current
  [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), explicitly preserving Human Guidance precedence
  and distinguishing product intent from accepted runtime support as PLE prepares for launch.
  Corrected adoption scope, Bonus/Quiz icons, immutable Question Type, Draft-first Question forks,
  public Stars/private Watches, and Student-facing action vocabulary. Added shared classification,
  Library Object/Bloom/support-content distinctions, Pool and Blueprint fork/comparison rules,
  Promoted authority, finite Attempt limits, and complete Student Work/retention definitions.
  Removed route and JSON-key assertions from the vocabulary authority. Later-adoption handling
  of existing Course content and adoption counts remains unspecified rather than invented.
  Three focused contract link/encoding/whitespace checks passed; the contract is below 1,000
  lines and scoped diff checks passed. Human Guidance, application behavior, and checklist
  acceptance were not changed by this documentation edit; pre-existing local edits were preserved.

- Reconciled the current Human Guidance checklist to SHA256
  `9e92c864a019d89ef9952cfb6e3b9c05a4f45d6055b6c3f49520c400f4dc7055`.
  All nine generator part gates, diff, and consistency passed: 1,038 occurrences (441 verified,
  547 open, 50 N/A) and 1,015 first-owner identities (432 verified, 533 open, 50 N/A). Sixty-nine
  new or changed requirements remain pending independent audit; no new Human Guidance row closed.
  The prior 998-bullet accounting remains historical only. The focused required-classification
  fixture repair in `tests/test_ple_question_json_authoring.mjs` passed 26 tests; this is neither
  an all-tests result nor Course-feature completion. Product work remains in progress on the
  Course-classification/Store HTTP foundation, exact Course-authorization issuer preservation, and
  compact Student details UI as independent bounded slices.

- Corrected the bounded Draft Questions list into compact aligned records with separate full title,
  description, state, and accessible Edit/Delete actions, retaining the existing safe Delete
  confirmation. Independent review accepted observed component-browser checks at 1280/320 pixels
  and enlarged text: no horizontal overflow, plus keyboard Delete opening confirmation focused on
  Keep draft. This does not establish connected-route/CAS/error behavior, successful deletion, or
  actual dark-theme acceptance. Independent review also accepts I05's bounded connected desktop
  publication receipt: Question published focuses its heading, retains neutral Question-authoring
  navigation, removes Save draft, shows the exact title/public ID/Revision, and links directly to
  that Question. This does not establish mobile, theme, broad accessibility, failure, concurrency,
  or all-authoring behavior.

- Recorded accepted bounded corrections: finalized `QuestionResponse` parent/Attempt/time
  identity, editable native response-control saves, receipt-gated anonymous-statistics retention
  through Unrelease, and the narrow support-repair lock/reference fix. Fresh PostgreSQL 17.11
  parent, Unrelease, support-role, and mandatory-NULL shape proofs passed. Root-observed Cargo,
  TypeScript, and focused response-control checks passed. This does not claim connected-browser or
  deployed-app acceptance, whole-Course retention, public-statistics disclosure, resource-specific
  support authority, or global Human Guidance closure; the current reconciliation establishes only
  inventory fidelity.
  A separate fresh PostgreSQL 17.11 proof retained exactly three valid unique indexes with no
  duplicate access paths; independent review accepted the three redundant-index removals.

- Recorded a bounded isolated HTTP/SQL receipt for the shared classification hierarchy. A vetted
  active Instructor and MFA-attested Sysadmin read all four hierarchy selectors; anonymous,
  Student, and inactive-Instructor access remained identically concealed. One native Question was
  published with Biology/Genetics metadata, then had only its Subject changed to Biochemistry;
  exact Question Revision/source bindings and stale-`412` metadata behavior were preserved. The
  fresh disposable PostgreSQL/MinIO proof used the current host binary and made no Live Demo
  change. This is not browser HTTPS, provisioned full-Course, rendered selector-workflow,
  deployment, or global Human Guidance acceptance.

- Made Student Coursework action labels reflect the available next step: Resume in-progress work,
  Review completed work, or Open otherwise. The accepted component-browser evidence at 1280 and
  390 pixels keyboard-activates each label and retains existing axe/page-error checks; root
  `node --import tsx tests/playwright/student_course_entry_m6_evidence.mjs` exited 0. This does
  not claim live perfect-score behavior, complete Student UI acceptance, full HTTP/browser
  coverage, or global UI closure.

- Added the shared classification-hierarchy installation fixture with two Disciplines, six
  Subjects, and six preserved Subject-Discipline links. Fresh-install and replay proof passed
  without creating an Account or session; independent review accepted the fixture. `cargo check -p
  project-tools` passed in 1.22 seconds and `cargo check -p project-tools --tests` passed in 8.06
  seconds. This does not claim full HTTP/browser behavior, deployment, or project-tools test
  execution.

- Put the Question prompt before compact supporting metadata and stewardship controls on the
  1280 by 800 inspection view, while retaining the backend-owned sandboxed WeBWorK preview. The
  bounded I04 receipt accepts native keyboard reachability for Star and Watch without activation,
  and reports no writes, protected-answer requests, or page errors. It does not establish native
  response-control rendering beyond the existing stem-only capture, every Question Type, a
  submission workflow, broader accessibility, or all UI closure.

- Put Current roster before compact, initially closed native Roster tools; keyboard navigation
  reaches roster actions and the disclosure controls. Import/export/revocation feedback now has
  distinct success and error presentation, while actual successful and partial-success outcomes
  remain unproven. Student narrow headers now keep compact identity, upper-right Profile, and
  full-width navigation with full keyboard-reachable breadcrumbs. Focused gates and the bounded
  browser receipt pass; the Profile breadcrumb reservation, awkward 200% overview wrapping, and
  global responsive/Human Guidance closure remain open.

- Recorded the independently accepted PostgreSQL 17 content-classification command prerequisite:
  role-aware vocabulary creation and selectors, trim-before-validation, global Subject uniqueness,
  Sysadmin-only nonempty association replacement, and two-session association serialization.
  This bounded SQL receipt does not claim HTTP/editor integration, content attachments, search,
  lifecycle, deployed state, Human Guidance closure, or permanent tests.

- Recorded the bounded PostgreSQL 17 Published Question classification receipt. New Question
  lineages and metadata bulk replacement require Discipline and Subject, with optional Topic and
  Subtopic UUIDs that satisfy the stored hierarchy; exact Question Revision and source bindings
  remain unchanged. Fresh-install proof passed valid publication, atomic invalid/stale refusal,
  Student denial, and retained/referenced-association behavior; `cargo check -p server_core` also
  passed. This does not establish HTTP, browser, producer/tool, or complete classification closure;
  Projecttools input contracts and TypeScript generation/browser consumers remain pending.

- Recorded source-level integration of shared classification-hierarchy selectors, Published Question
  consumers, and Pilot/curriculum publishers. Selector consumers submit provisioned UUID
  identities without defaults, automatic associations, or inferred classifications. All 42
  curriculum sources and two Pilot chapters now provide explicit authored classification; the
  supporting restriction-enzyme Genetics exercise uses Biology/Genetics. Publisher recovery
  returns existing immutable publications without re-resolving mutable classification metadata.
  `cargo check -p project-tools` passed after that recovery correction in 7.51 seconds; shared
  `npx tsc --noEmit` passed earlier and `cargo tsgen` refreshed 367 types. Runtime HTTP/browser
  integration, deployment, actual imports, new count-audit machinery, and global Human Guidance
  checklist closure are not claimed.

- Corrected the Human Guidance classification evidence for the current Subject-Discipline
  association schema. The bounded PostgreSQL 17 receipt proves multi-Discipline association,
  association integrity, one-parent Topic/Subtopic relationships, and owner-only access. Commands,
  content attachments, normalization, and the future atomic at-least-one association writer remain
  open; no Human Guidance, production, or permanent-test changes were made.

- Reconciled the current 998-bullet Human Guidance inventory and existing generator heading
  mappings: 449 verified, 499 open, and 50 audited N/A. Preserved unchanged accepted evidence
  and moved duplicate ownership to current headings. New glossary, Profile-avatar, and global
  classification requirements remain open; latest Subject multi-Discipline associations supersede
  the older single-parent foundation assumption. All nine part gates, identity diff, and consistency
  pass. Focused formatting/link checks report 285 passes and four known missing/copied-link failures;
  verbatim source trailing whitespace remains visible rather than silently changing Human Guidance.

- Added the current-plan receipt for the installed global classification-vocabulary prerequisite:
  four UUID vocabulary levels with mandatory parent references, owner-only forced RLS, no runtime
  grants, and fresh PostgreSQL 17 proof. The 1,412 focused pytest gates cover tracked files; the
  new SQL file separately passed source-size and ASCII checks. No live database changed and the
  temporary proof containers were cleaned up. ProductRole-aware access, Discipline lifecycle,
  attachments, editors, search, and global feature closure remain open.

- Corrected native Question response-control copy so a saved individual response is described as
  saved rather than submitted; the whole Assessment Attempt remains the Student submission
  boundary. Local reset no longer implies successful persistence, and preview acceptance does not
  promise feedback. MULTI-FIB instructions explain field navigation without a false per-Question
  "Submit answer" action. Save behavior, validation, and backend boundaries are unchanged.

- Added one atomic Account-creation trigger that randomly selects and persists a currently
  selectable PLE gallery avatar for every Product Role. An empty selectable gallery rejects
  Account creation; existing self-selection and Profile-image authorization are unchanged.
  Shell syntax, diff checks, and the fresh PostgreSQL 17 persistence, explicit-selection,
  replay, and empty-gallery rollback proof pass; independent review found no correctness
  or security issue. The owned disposable container was cleaned up. No HTTP or deployed
  acceptance is claimed, and no live database, migration, or backfill was changed.

- Made Student "Before you start" compact with aligned labels and values, grouped Questions/points
  and timing/Attempt rules, one timing-zone label, and clear "No closing time" / "Unlimited Attempts"
  wording. Canonical component selectors now apply; the nearby Start control is content-sized.
  Actual Avery overview receipts at 1280/768/600/390 pixels show no horizontal overflow and a
  keyboard-reachable Start control without creating an Attempt. The laptop Start moved from about
  y720 to y443, with previous Attempts visible at y494. Previous-Attempt score projection and the
  broader Coursework/Ribbon findings remain separate open work.

- Closed the narrowly evidenced Student terminal-Attempt findings: submitted and expired Attempts
  hide active navigation rather than showing false current/saved progress, and history visibly labels
  closed no-response records **Unanswered**. Accepted 1280/390 Avery R-4 history retains Q2's four
  MATCH pairs and `1 / 1`, three unanswered `0 / 1` records, and total `1 / 4`; the prior SQL
  evidence remains the no-submission/no-grading proof. Before-start density, narrow Ribbon,
  all-types presentation, and previous-Attempt score projection remain open.

- Student Assessment Attempt review now labels Question records closed without a saved response
  as **Unanswered**. The unavailable-response message remains reserved for submitted Questions
  whose saved response is not released.

- Renamed the Profile Avatar Store/Shop surface to **Avatar Gallery** throughout the direct
  data-access and PostgreSQL adapter APIs and the Profile picker. Routes, schema, stored data,
  and generic `StoreError` behavior are unchanged; no compatibility aliases were added.

- Hide active Question navigation and its loading state after an Assessment Attempt is submitted
  or expires. The terminal accepted/automatic-submission messages remain unchanged, so closed
  Question positions no longer present as current/saved navigation.

- Reconciled the two latest Student Coursework wording changes at HG `ab1bced1` (943 bullets).
  Visual current/saved/focus distinction and response-effect labels remain open across native
  response controls; accepted navigation styling receipts remain partial evidence. Current totals
  are 451 verified, 444 open (436 owning), and 48 audited N/A. Part 05 gate/splice, diff and
  consistency pass; no source, runtime, permanent-test or Human Guidance changes were made.

- Extended the accepted 940-bullet reconciliation to current Human Guidance's 943 bullets
  (`00a3b948` SHA256 prefix): Sysadmin work priority is audited N/A; searchable Blueprint Promoted
  and exclusive Sysadmin control remain binding gaps. Preserved earlier evidence and reopened only
  current/progress visibility for observed misleading submitted-Attempt summary; active navigation
  closures remain accepted. Actual expired R-4 history retains correct MATCH `1 / 1` and total
  `1 / 4`, but unanswered Questions say Closed/response unavailable rather than Unanswered. Current
  counts are 452 verified, 443 open (435 owning), 48 N/A; no HG or source correction was made.

- Reconciled the evidence-bearing Human Guidance checklist to the current 940-bullet snapshot
  (`c12b5d45` SHA256 prefix), preserving unchanged evidence and current first-owner pointers.
  Five compact Attempt-navigation rows have bounded acceptance; broader UI/MATCH and the new
  Discipline classification/name-validation contract remain open. Existing Public Genetics and
  installation-source receipts do not close fresh-install acceptance. Current totals are 453
  verified, 440 open (432 owning), and 47 audited N/A. All nine part gates, diff, and consistency
  pass; no Human Guidance, production, permanent-test, runtime, build, or Git state was changed.

- Made Student Attempt navigation a compact numbered row with current/saved cues, width-adaptive
  first/last pagination, ellipses, and Previous/Next; corrected its stylesheet selectors, flattened
  the Attempt header, and removed duplicate progress text. Strict frontend checks and all three
  existing navigation regressions pass. Supplied desktop/phone evidence preserves exact saved
  responses; a temporary 250-Question harness proves keyboard reachability, direct first/last
  jumps, no phone document overflow, and reachable local scrolling at 200% enlargement.
  Independent review accepts this bounded change; broader Human Guidance closure remains open.

- Added ordinary owner publication of only the installation-owned retained Genetics example;
  already-Public replay skips mutation and generic curriculum imports remain Private. Corrected
  the opt-out oracle to current Public-only discovery and ownership checks. Narrow Cargo check,
  formatting, lint, six controller tests, and temporary shell-generation checks pass; fresh-install,
  replay, connected oracle, and independent review remain pending. No runtime change was made.

- Quieted the existing Course theme reading surfaces and flattened Course Teaching Team and action
  presentation without changing durable theme IDs. Supplied Live Demo Course captures at 1280 and
  390 pixels show the bounded presentation; the narrow document remains 390 pixels wide.

- Replaced repeated matching choices with one bank and prompt slots, supporting keyboard and
  click/tap assignment, replacement, Clear, optional same-bank drag, and Native choice-reuse rules.
  Initial, edit, submit, and Reset serialization now omit unanswered slot values, preserving strict
  validation of actual unknown and duplicate choice IDs. Independent temporary native/presentation
  partial-response and reset checks pass. Supplied live evidence proves keyboard assignment, normal
  incomplete feedback after Reset, and click assignment plus Save/reload of all four exact choice
  texts; real mouse drag restores a cleared slot's exact prior choice and Save succeeds. Assessment
  submission, full accessibility, and global Human Guidance closure remain unestablished.

- Changed frontend build cleanup to preserve the `dist` output root for existing directory bind
  mounts. Independent source/probe review accepts the inode-preserving cleanup; the runtime owner
  reports HTTP 200 after rebuilding without restart and a passing post-Reset frontend build
  (`dd97c93a`). Narrow Ribbon density remains a separate presentation gap.

- Confirmed the current Live Demo Genetics example Blueprint is Public with nine Assessments and
  42 distinct Questions through ordinary owner publication, without a restart. Fresh-install setup
  still needs to publish only the installation-owned retained example; generic imports stay Private.
  The non-executed `assert-live-demo-absent` Genetics oracle remains pending correction because its
  no-argument Blueprint list call and `available` assertion are obsolete.

- Reconciled finite Attempt-timing documentation to the new Question-Pool selection-count guidance.
  The checklist now has 891 bullets: 448 verified, 397 open (389 owning), and 46 N/A. Accepted
  independent SQL evidence at `/private/tmp/ple-finite-duration-role-artifacts.TABP74` reproduces
  the former role-order failure and proves fixed-entry arithmetic, empty/251 refusal, the 12-hour
  constraint, and ordinary Instructor-release/Student-start/resume. It does not establish browser,
  HTTP, renderer, real Instructor override-save, accommodation, or multi-selection Pool timing
  acceptance. No authority, production, runtime, generated-code, permanent-test, Cargo, Podman,
  or browser change was made.

- Clarified Human Guidance: Question Pools count by the number selected for the Assessment Question
  limit and default time calculation; selecting 3 of 199 counts as 3.

- Reconciled current finite Attempt-timing guidance without editing the authority. The accepted
  PostgreSQL Pool-import proof closes the at-most-250 delivered-Question row: import to 250 passes,
  251 returns `23514`, and the rejected import rolls back its child Pool, Revision, Entry,
  association, and parent Edit Number. A fresh two-Question NULL-default release resolves 180
  seconds, but Student start fails `Assessment Attempt requires 1 to 250 Questions`; no
  default/delivery, override, browser, accommodation-ratio, or effective-24-hour-cap acceptance is
  claimed. Exact unchanged same-identity SQL resume remains verified; mixed expiry-worker SQL
  evidence does not establish rendered unanswered state or actual Backend transport exclusion.
  Accepted authenticated HTTP evidence also closes fixed Question ID-plus-Revision authoring
  persistence and four atomic ID-only/missing-Revision rejections, not publication, rendering,
  browser, or Student Work. Current inventory is 890 bullets: 448 verified, 396 open (388 owning),
  and 46 N/A. Part 09 gate, splice, identity diff, consistency, and diff-check pass. This
  reconciliation makes no authority, production, permanent-test, Cargo, Podman, or browser change.

- Reconciled the evidence-bearing Human Guidance audit to the untouched current heading and semantic
  snapshot (`4e0b9778` SHA256 prefix), preserving current scoring closures and accepted bounded runtime
  receipts. The fixed nine-part generator changes only five root names. Blueprint lifecycle, forks,
  and comparison now belong to Course specifications; Assessment type appearance belongs to
  Instructor interface. Contextual reconciliation retained 741 exact occurrences and evaluated 46
  equivalent/consolidated occurrences; 95 changed/new occurrences include 89 open, four narrow
  source-verified metadata/theme-ID claims, and two N/A heading-maintenance rules. Expanded support,
  Pool metadata/statistics, Bloom, Change Proposals, selected daughter incorporation, comparison,
  and Unrelease remain scoped gaps, not inherited whole-behavior acceptance. Current inventory is
  882 bullets: 445 verified, 392 open (384 owning), and 45 N/A. All nine part gates, identity diff,
  consistency, and diff-check pass; focused existing Markdown/whitespace/Pyflakes checks pass 857
  tests. Broader links/style selection passes 1119 tests but has two unrelated missing-screenshot
  link failures in README and the prior docs-pass summary. No HG, product, permanent-test, browser,
  Cargo, or Podman changes were made by this audit task.

- Closed four exact Assessment scoring rows from accepted current-rescore evidence: retained credit
  as the grading outcome, current-credit/current-points score calculation, current-point
  recalculation, and immutable grading outcomes. Independent review accepted private PostgreSQL 17
  production-SQL lifecycle proof at `/private/tmp/ple-current-rescore-proof/proof.sql`; the artifact
  `/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` exited 0. The fixture retained `0.5`
  credit, used authorized expected-current `ple_api.save_assessment` to change points from `8` to
  `13` and advance the edit number, observed `6.5 / 13`, unanswered `0 / 13`, and `6.5 / 26` on
  three replay/history/Student-landing/Instructor-Gradebook reads, and matched evidence JSON hashes
  before and after. The no-Backend-interaction row remains open: SQL `already_submitted` has no
  work fields and the server bypasses ready-path Backend work, but actual HTTP request counts and
  Backend transport are unobserved. This temporary SQL-only fixture simulates initial synchronous
  Backend credit, is not a permanent test, and does not establish HTTP, rendering, authentication,
  or full product acceptance. Part 09 gate, splice, and consistency pass at 828 bullets: 468
  verified, 317 open (310 owning), and 43 N/A.

- Closed C510's highest-submitted-Attempt score row after removing the stale configurable
  grade-rule enum, field, SQL columns, and editor choices. Accepted independent PostgreSQL 17
  lifecycle evidence recorded scores `12`, `4`, `16`, and an in-progress `NULL` fourth Attempt;
  both Instructor Gradebook and Student landing projected `12 / 16`, `12 / 16`, then `16 / 16`,
  while latest-Attempt state remained separate. The proof passed at
  `/private/tmp/ple-highest-score-proof/artifacts.rQxVs8/proof.log`. The fixture permits four
  Attempts only, so it does not claim unlimited-Attempt eligibility; privileged-fixture and
  simulated-backend evidence also does not claim HTTP, rendering, or actual backend grading.
  Focused LDA test compilation used `--features postgres,test-support`, not a default-feature or
  full Rust gate; a separate `cargo check -p learning-data-access --no-default-features` pass
  confirms the optional PostgreSQL adapter boundary. Part 09 gate, splice, and consistency pass at
  828 bullets: 465 verified, 320 open (312 owning), and 43 N/A.

- Closed only the two unanswered-zero Assessment rows from a disposable root-only PostgreSQL 17
  proof. `/private/tmp/ple-unanswered-connected-proof/run.sh --isolated` ran the non-versioned
  temporary fixture `/private/tmp/ple-unanswered-connected-proof/proof.sql`, which invoked ordinary
  `ple_api.prepare_student_assessment_attempt_finalization` and
  `ple_api.commit_student_assessment_attempt_finalization`, then history and Gradebook readers. It
  distinguishes unanswered null credit from evaluated zero credit: Full Credit was `0 / 8` versus
  `8 / 8`, then `0 / 13` versus `13 / 13` after current-point recalculation, with Gradebook
  `13 / 26`. The disposable fixture is not a permanent test and did not invoke an expiry finalizer,
  so the expired-Attempt row remains open. The five unrelated source-only scoring rows remain open.
  The broad highest-Attempt-score row is also open because the reusable editor still offers first,
  latest, highest, and Instructor-selected grade rules. No HTTP, authentication, renderer,
  backend-transport, timed-expiry, or full product acceptance is claimed. Part 09 gate, splice,
  and consistency pass at 828 bullets: 464 verified, 321 open (314 owning), and 43 N/A.

- Reconciled only three accepted Blueprint browser behaviors into the Human Guidance checklist:
  opening one selected Blueprint Assessment, showing only its Questions, and expanded canonical
  comparison detail. The compiled-main, actual-loopback-HTTP Properties receipt proves shared
  drafts, six Student-feedback controls, one ordinary Save, and exact reload, but not the full
  scoring, attempt, or late-work activity-control scope; that Properties row remains open. The
  independently accepted expanded-comparison receipt proves DTO-backed titles, instructions, fixed
  Question IDs, Revisions, and points at desktop and narrow widths. No populated Student Work,
  login/TLS, full accessibility, or whole-C413 workflow claim is made. Part 04, splice, and
  consistency are recorded with the updated 828-bullet inventory: 462 verified, 323 open
  (315 owning), and 43 N/A.

- Reconciled the C47 Public Blueprint search and Part 09 Assessment locator audit into the global
  Human Guidance checklist. Accepted actual HTTP and compiled-main evidence closes only submitted
  Public-name search, literal filtering, query-bound paging, empty reset, detail, and adoption
  preselection; Properties, comparison, Course creation, login/TLS, and full accessibility remain
  open. Part 09 reopens seven target scoring rows and the adjacent unanswered duplicate because
  the current `full_credit` path can award points for missing work; the active correction remains
  open. Part 04 and 09 gates, identity diff, and consistency pass at 828 bullets: 459 verified,
  326 open (318 owning), and 43 N/A.

- Closed the bounded Blueprint fork, current-pair comparison, and selected Apply checklist rows from
  accepted source, actual-HTTP, and compiled-main browser evidence. The receipts establish visible
  sibling/transitive pairs, Question-ID matching across renamed/reordered/split content, fresh local
  Assessment/Pool identities, explicit target selection, denied stale/authorization/Archived cases,
  and rollback. The expanded changed-content detail remains open because its available browser
  capture is collapsed; no populated Student Work, login/TLS, full accessibility, or C413 claim is
  made. Generator-derived inventory: 466 verified, 321 open (313 owning), and 43 N/A across 828.

- Reconciled the verbatim Human Guidance checklist and audit parts to the current untouched
  828-bullet snapshot, including the reordered Interface headings and expanded density wording.
  Added only the four existing-generator manifest headings required to cover those source sections;
  preserved unchanged evidence and accepted Blueprint closures, with new density rows open.
  Identity diff and consistency pass; seven part gates pass, while the existing Instructor comparator
  locator and eight Assessment locators remain unresolved. Recomputed counts are 443 verified,
  342 open (335 owning), and 43 N/A; earlier inventory counts below are historical.

- Reconciled the Blueprint authoring and lifecycle checklist against current source and accepted
  runtime evidence without changing Human Guidance. Closed the bounded no-date, three-state,
  owner-Private visibility, Public-only adoption, lifecycle, Archived/history, recorded-metadata,
  and older-Revision rows; retained Public Blueprint search, fork creation, comparison/newer,
  owner Apply, and full Blueprint Assessment Properties/browser acceptance as open. The current
  820-row inventory is 441 verified, 336 open (330 owning), and 43 N/A. The global checklist splice
  remains blocked by pre-existing audit-part/global-order drift and an unrelated invalid comparator
  evidence locator; no unrelated audit repair was made.

- Added a bounded exact-reference/Assessment-owned-Pool contributor receipt without changing Human
  Guidance, its checklist/counts, or any closure. Actual-server and compiled-main receipts at
  `/private/tmp/ple-blueprint-owned-pool-artifacts.nbKrXt` verify exact fixed Question ID/Revision
  retention through reorder, explicit latest-revision re-add after removal, one ordinary Save, and
  unchanged sibling/source Pools and Student Work fingerprint. The separate 84-request target-local
  Apply receipt at `/private/tmp/ple-blueprint-owned-pool-artifacts.vs0NCo` verifies fresh Pool
  clones, explicit existing/new targets, four CAS denials, atomic injected-fault rollback, and
  Archived-owner `409`. `question_model` passed 135 focused tests; Store no-run compilation and
  root TypeScript passed. Publication now uses full Question-ID-and-Revision equality, but the
  unused exported Blueprint Picker remains ID-only deduplicated. No populated Student Work,
  current-pair comparison, newer indication, browser Apply, or whole-plan claim is made, and no
  permanent test was added.

- Renamed all 32 Instructor screenshot PNGs to remove the redundant `_laptop` filename suffix.
  Updated the current capture manifest and receipt, generated atlas, README, and compliance-report
  links without recapturing images; capture IDs and laptop viewport semantics are unchanged.

- Synchronized the three current Human Guidance Revision/fork rows into the implementation
  checklist from source evidence only. Published Question successor publication keeps its Question
  ID; Question Pool revision append keeps its Pool ID; both fork paths establish a fresh public
  identity and Revision 1, with the Pool path copying exact ordered Question ID-and-Revision
  membership. No runtime proof is claimed. The broader Assessment authoring exact-reference row
  remains open because its exported picker adapter still deduplicates by Question ID. The
  reconciled inventory is 820 bullets: 431 verified, 346 open (340 owning), and 43 N/A.

- Synchronized the current Human Guidance Assessment-content and Question-Pool membership wording
  with the implementation checklist. The exact Published Question ID-and-Revision reference
  invariant remains open pending authoring-input cutover; Pool fork and no-nesting additions remain
  source-audit pending. The recomputed inventory is 817 bullets: 428 verified, 346 open (340
  owning), and 43 N/A. The Assessment audit gate still reports eight pre-existing invalid evidence
  locators in unrelated Attempt/scoring rows.

- Added bounded Blueprint Assessment-owned Pool/fork contributor evidence without changing Human
  Guidance, its checklist/counts, or any closure. Independent review accepted the actual-server
  43-HTTP-request pass at `/private/tmp/ple-blueprint-owned-pool-artifacts.ySNfXX` and focused
  build log `/private/tmp/ple-blueprint-owned-pool-build-4.log`. Repeated imports minted fresh
  Pool IDs; retained edits created immutable Pool Revisions with exact ordered Question IDs and
  revision pins; retry, stale, and foreign/error paths were covered; and a whole Blueprint fork
  minted fresh Assessment and Pool IDs with the same exact ordered Question revision membership.
  The internal `question_pool_revision.created_in_transaction xid8 DEFAULT pg_current_xact_id()`
  marker replaces a timestamp ownership heuristic and is never public. Independent review also
  accepted the compiled-main browser receipt at
  `/private/tmp/ple-blueprint-owned-pool-artifacts.ytU6GT`: its browser/state JSON, desktop and
  narrow screenshots, and matching compiled hash show lazy exact-member reads, local Cancel,
  reorder, remove/add, and closed-panel missing-attestation Save blocking. One ordinary Save made
  exactly one `PUT` and one Revision while source and sibling Pools remained unchanged. Root pytest
  passed 7,280 tests. This evidence claims no populated Student Work (only an empty
  `student_record`), new Selective Apply contract, current-pair full workflow, login, TLS, full
  accessibility, or full rollback; all relevant Human Guidance closures remain pending. Removed
  three obsolete numeric identifier references from an archived plan.

- Synced the new Blueprint-fork Assessment/Question Pool wording and no-Assessment-history
  comparison row as open. Independent Pool forks are not implemented, and historical
  pin-preserving proof does not close them. The inventory has 811 bullets: 427 verified,
  341 open (335 owning), and 43 N/A.

- Reconciled Blueprint audit/planning records to the latest Human Guidance comparison and history
  authority without changing runtime code or HG. Retained valid bounded read-only evidence;
  reopened visible same-lineage pair coverage, shared-Question-ID Assessment matching, robust
  shared/added/removed correspondence, history/newer indications and connected Apply gaps.
  Recorded origin remains provenance, not a required comparison baseline. The user's fresh local
  Assessment-ID/no-cross-Blueprint-Assessment-lineage clarification requires fork/Apply audit;
  prior internal-ID contributor proof does not close it. Checklist gates pass with 808 bullets:
  427 verified, 338 open (332 owning), 43 N/A.

- Closed only the six bounded Blueprint fork-review HG rows: known-fork owner discovery,
  source-row opening, current-head comparison, recorded-origin baseline, visible source-only and
  fork-only distinction, and requested canonical-JSON calculation. Accepted C881/C882 receipts are
  `/private/tmp/ple-fork-reader-artifacts.nRikDO` and
  `/private/tmp/ple-fork-review-http-artifacts.LTUgsF`; root visual review accepted the compiled
  UI at 1280 by 800 and initially at 390px. The lazy, retryable review is GET-only, `no-store`,
  and leaves all `ple_data` unchanged. The direct fork page has no Compare entry; review begins
  from the source known-fork row. Root's latest pytest run passed 7,232 tests; accepted source
  build, TypeScript, and Node-21 evidence is limited to this scope. C883 selective apply and C413
  whole-workflow closure remain open.

- Added accepted source-only C880 Blueprint fork-comparison contributor evidence without closing
  C881-C884 or C413, changing Human Guidance, or adding runtime workflow behavior. The Question
  Model projection compares canonical Question JSON exports rather than PostgreSQL layout; it keeps
  stable identities and order across unchanged, source-only, fork-only, and overlapping changes,
  and duplicate IDs are constructor errors so map construction cannot silently lose an item. The
  temporary consumer also covered module-parent/current-name changes, pins, and defaults. Root
  `cargo build -p question_model`, `cargo test -p question_model`, and `cargo clippy -p
  question_model --lib -- -D warnings` passed; logs are in
  `/private/tmp/ple-blueprint-fork-comparison/`. No persistence, authorization, server read/apply,
  UI, public comparison-state enum, or permanent test was added.
  The documentation also records the user's clarification that ordinary Blueprint visibility governs
  viewing/comparison and fork ownership governs apply mutation; it changes no lifecycle state or
  product decision.

- Added accepted C881 contributor evidence for the corrected-current-head Blueprint fork read
  boundary. Typed ordinary-visibility reads now supply the recorded origin, source current Revision,
  fork current Revision, and current names; source head may equal origin. The corrected-current-head
  build and connected proof passed at `/private/tmp/ple-fork-reader-artifacts.nRikDO` and
  `/private/tmp/ple-fork-reader-connected.log`; independent evidence review accepted. This records
  the expanded Guidance model's read boundary only: C882-C884 and C413, including server comparison,
  selection/apply, and UI, remain open. No Human Guidance or checklist closure, persistence, public
  comparison-state vocabulary, or permanent test is claimed.

- Added accepted C882 contributor evidence for the on-request Blueprint fork review endpoint.
  Actual HTTP proof at `/private/tmp/ple-fork-review-http-artifacts.LTUgsF` covered exact current
  heads, names/ETags, ordinary visibility, `no-store`, and zero `ple_data` mutation. Root build,
  TypeScript generation/typechecking, and 21 Node tests passed; independent source and evidence
  review accepted. This receipt covers Fixed Questions only; Pool semantics remain with the earlier
  C880 source. C883/C884/C413 full workflow, including selection/apply and UI closure, remain open;
  no Human Guidance or checklist closure and no permanent test are claimed.

- Published the canonical 58-capture screenshot corpus with normal Morgan MFA through the reviewed
  separate CLI, without a bypass. The refresh and independent live replay verification passed at
  `/private/tmp/ple-screenshot-refresh-20260916.log` and
  `/private/tmp/ple-screenshot-refresh-verify-20260916.log`. It includes four new-content captures,
  refreshed Sysadmin captures, and the reviewed atlas-generator alt-text fix. Twenty-nine
  byte-different replay images remain in `test-results/screenshot-corpus/verify`; semantic checks
  passed and no byte-equivalence gate is claimed. Root visually inspected the refreshed Sysadmin
  home and earlier review covered the four new-content captures. Static Markdown links (285) and
  five screenshot tests passed. No Human Guidance closure is implied.

- Closed only the two Archived Blueprint discovery HG rows. The normal Blueprint list defaults to
  excluding Archived records; an explicit strict `includeArchived=true` shows them to every active
  vetted Instructor while Private records remain owner-only. Accepted actual-server HTTP evidence
  covers owner/nonowner default, false, and true membership; nonowner Archived `200`, Private
  `404`, Student `404`, and invalid query `400` results. Accepted compiled-main browser evidence
  covers default off, explicit include, actual read-only Archived detail, return to off, eight GETs,
  and zero writes. Artifacts:
  `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-http-proof.json` and
  `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-browser-proof.json`.
  The fixture has privileged Published-Question seed data, ordinary vetted account APIs, fixture
  sessions, and an accepted Sysadmin MFA fixture helper; it does not claim login, TLS, pagination,
  concurrency, publisher-caller preservation, forking, adoption, or a broader Course workflow.
  Root `cargo build -p server_core --features local-disposable-storage -p project-tools`, the full
  canonical database baseline (`/private/tmp/ple-archived-discovery-baseline.log`), pytest (7,151),
  21 Blueprint client/UI/model Node tests, and TypeScript typechecking passed. Checklist splice,
  diff, consistency, and the Course gate passed: 423 verified, 318 open, 312 owning-open, and 43
  N/A across 784 bullets. No aggregate `all_test.sh` claim is made.

- Closed only the Archived Blueprint read-only HG row. Owner Save and rename now reject Archived
  Blueprints before replay, CAS, or no-op handling; the extended existing lifecycle regression
  preserves metadata, Revision, content, events, and receipts across denied writes, while restored
  Private/Public writes succeed. Accepted actual HTTP proof recorded five `409` denials with
  unchanged Blueprint state, `200` owner/nonowner historical reads, `404` nonowner writes, and
  `200` restored writes:
  `/private/tmp/ple-daughter-revision-notice-artifacts.JhV6aj/archived-blueprint-http-proof.json`.
  Root `cargo test --workspace --no-run`, the full canonical database baseline, and pytest (7,151)
  passed. Explicit-include Archived browsing, forking, adoption, populated daughters, concurrency,
  and broader Course completion remain open; no aggregate `all_test.sh` claim is made. Checklist
  splice, diff/consistency, and the Course gate passed: 421 verified, 320 open, 314 owning-open,
  and 43 N/A across 784 bullets.

- Added focused selected-source CLI evidence without changing C838/C839 status or any Human
  Guidance count. The ordinary-Instructor publication command selected the fresh 42-source
  canonical manifest entry `topic05-degrees-of-dominance-which-one`, publishing one available
  WeBWorK PGML Question Revision 1 with exact bytes, checksum, and source provenance; it created
  no Pool or Blueprint. Replay made no additional publication, and unknown source, hash mismatch,
  and missing-path inputs were rejected before writes. The private fixture's vetted Instructor and
  audit seed are explicitly privileged; the publication session is ordinary Instructor. The
  isolated proof is `/private/tmp/ple-canonical-family-artifacts.TOOlBJ`, run by
  `bash /private/tmp/ple-canonical-family-publication-proof.sh --isolated`. Separately,
  `cargo build -p project-tools` and `cargo test -p project-tools --bin project-tools
  curriculum_content::` passed, including four existing curriculum tests. It does not claim
  Sysadmin authentication, rendering,
  browser acceptance, or a retained catalog. Questions checklist splice, diff/consistency, and
  gate passed unchanged at 420 verified, 321 open, 315 owning-open, and 43 N/A across 784 bullets.

- Closed only six Course Blueprint-update HG rows: the three owning Course-summary workflow rows
  and their three duplicate occurrences. An authorized Instructor can lazily review the current
  parent Revision for adopted Assessments only, seeing changed, matching, removed-source,
  Type-mismatch, and automatically-added rows; direct local Assessments are excluded. Accepted
  actual-server and compiled-main proof at 1280 by 900 and 390 by 844 covered lazy open/reopen,
  per-read five-row coherence, Course-to-detail review, zero POST on Cancel, exact source Revision
  2 plus daughter Edit CAS on Apply, and refresh to a matching row. Student and unrelated-
  Instructor reads were nonenumerating `404 no-store`; a private parent was concealed from another
  Instructor in the privileged-availability fixture; Archived review remained available and new
  adoption was denied. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  The earlier detail-only receipt remains the separate proof of exact pins, stale/no-op/invalid-
  Released cases, dates, status, origin, and one populated Assessment Attempt hash:
  `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`. Neither receipt claims direct
  Assessments or all Student Work. No persisted offer, receipt, comparison baseline, or new update
  table is introduced, and no whole-Course lifecycle completion is claimed. Root build, TypeScript,
  LDA clippy, pytest (7,151),
  Node (341), and fresh full database baseline passed; the baseline receipt is
  `/private/tmp/ple-course-summary-baseline.log`. No aggregate `all_test.sh` claim is made.

- Added one accepted retained-Assessment Blueprint-update contributor without closing C410/C411 or
  changing any Human Guidance count. An authorized Instructor can `GET` a derived current-parent
  review and explicitly `POST` Apply for one adopted Assessment using both expected parent Revision
  and daughter Assessment Edit Number. It has no offers, approval receipts, comparison baselines,
  or update tables. Accepted actual-server and compiled-UI proof reviewed complete current/proposed
  Fixed Question and one-member Pool Revision-1 facts, made Cancel issue zero POST requests, then
  applied while preserving dates, status, origin, and one populated Assessment Attempt hash. It also
  covered stale parent/daughter CAS, authorization, no-op, and invalid Released rollback; visual
  review was desktop-only at 1280 by 900. Artifact:
  `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`. Whole-Course discovery/review,
  all correspondences, narrow viewport, dirty/stale UI, missing/type/Archived parent, lock-wait,
  and multi-Revision cases remain open. This proof does not establish normal Student start, future
  Attempts, responses, submissions, grades, or all Student Work tables. Static server/build,
  TypeScript, scoped PostgreSQL LDA clippy, pytest (7,151), and Node (341) lanes passed; the
  independent baseline-fixture reviewer accepted the focused PostgreSQL 17 sequence at
  `/private/tmp/ple-expiry-unrelease-focused-artifacts.iOpO6r`. The full canonical database-baseline
  gate also passed as `database baseline E2E: PASS`; no aggregate `all_test.sh` claim is made.

- Captured four one-time Instructor preview screenshots of new Genetics WeBWorK content from the
  production-shaped HTTPS Live Demo: DNA structure, meiosis prophase, chi-square, and chromosome
  shapes. The ordinary-Instructor runner passed route, Ribbon, privacy, page-error, and origin
  checks for all four 1280x800 images; visual review found the prompts and controls present.
  These 320 KiB files remain untracked review evidence in
  `test-results/screenshot-corpus/new-content/instructor/`, separate from permanent tests and the
  unchanged 54-capture canonical atlas. Canonical publication was not attempted because it requires
  an ordinary Sysadmin MFA session; static canonical verification still passed. No Human Guidance
  closure or product change is claimed.

- Closed only the two HG rows that require an older Blueprint Revision to be obvious on a daughter
  Course Instance. The existing authorized Course load and Course page now show adopted/current
  Revision values and the newer-state notice. Accepted independent actual-server/exact-main proof
  covered empty, current, newer, and synthetic Private-origin states; the newer PNG was visually
  inspected, denials were nonenumerating `404 no-store`, and no extra Blueprint fetch/write or
  browser errors occurred. Original adoption pin, Assessment, and entries remained unchanged;
  Work tables were empty, so no populated-Student-Work claim is made. Artifact:
  `/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`. This is not Blueprint update
  offer/review/approval/apply work. The independently accepted focused PostgreSQL 17 access-seed,
  security, expiry, grading, and Assessment Unrelease sequence, including its causal row lock,
  passed at `/private/tmp/ple-expiry-unrelease-focused-artifacts.5Kb3MC`; it is a fixture cutover
  receipt, not a further HG-row closure. Checklist splice, diff, consistency, and the Course gate passed:
  414 verified, 327 open, 318 owning-open, and 43 N/A across 784 bullets. The final full pytest
  rerun passed 7,151 tests. The broad database-baseline gate is not green: after its earlier
  stages passed, it stopped in final Blueprint lifecycle support at `support.rs:82` on a
  permission-denied `ple_data` relational-identity query. No repair is claimed.

- Corrected stale Quiz/Exam disclosure evidence without closing either HG row. The current
  `history_decision` calls `gate_quiz_exam_answers_for_current_cohort`, and
  `project_released_content` applies that gate. Accepted independent PostgreSQL 17 installed-
  predicate proof with administrator-inserted synthetic fixtures covers never-started blocking,
  pending-invitation exclusion, joined-current-membership blocking, Account-deactivation membership
  preservation, Course-end noncompletion, ended-episode exit/new-episode rejoin, and retained
  submissions:
  `/private/tmp/ple-assessment-cohort-transition-artifacts.nWdHzT`. This is not public membership
  API, whole-submit, or HTTP answer-withholding acceptance. Opaque WeBWorK answer display remains
  unimplemented and HTTP verification is pending, so both product rows remain open.

  Baseline fixtures now use an explicit role-scoped provenance read on one `PgConnection`,
  isolated second-test IDs, and the forwarded manifest; no production permission is weakened and no
  permanent test is added. The full 7,151-test pytest run passed. The broad
  database-baseline gate failed because an existing negative fixture used the pre-public-reference
  return column. Its focused Course connected correction passed both existing tests sequentially
  against fresh PostgreSQL 17 with 0 ignored:
  `/private/tmp/ple-course-lifecycle-proof-artifacts.l8ilq0/connected-test.log`. A later broad
  rerun passed the Course tests and security catalog, then failed because
  `tests/e2e/attempt_expiry_connected_oracle.sql` still calls removed
  `ple_api.start_assignment_attempt`; direct caller cutover and rerun remain pending.

- Closed only six HG rows: C15's higher-security Sysadmin row, three automatic-new Blueprint
  Assessment rows, and two existing-Assessment non-silent-change negative invariants. Accepted
  independent SQL/actual-server loopback HTTP proof requires genuine private TOTP
  before Sysadmin session issuance, preserves ordinary Student/Instructor sessions and limited
  grants, and denies binding failures, bad codes, replay/expiry/counter reuse, and a fresh unused
  valid counter after five failed attempts. Artifacts:
  `/private/tmp/ple-sysadmin-session-boundary-artifacts.kSMr1H` and
  `/private/tmp/ple-sysadmin-session-boundary-http-artifacts.zexsoO`. Connected normal Blueprint
  Save proof preserves exact settings/pins, fresh daughter Pool IDs, Unreleased/null-date copies,
  an inactive daughter, unrelated empty-Course nonmutation, existing actual Student Work,
  original adoption pin, retained source-title change without existing daughter mutation, and replay/no-op/
  stale safety; supplemental bad-payload rollback passed:
  `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. The existing stable product-contract lifecycle
  test was extended with the append helper; temporary SQL/HTTP proof remains outside Git. Existing connected adoption
  lifecycle regression passed 1 test with 0 ignored:
  `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. Checklist splice, diff/consistency,
  and three narrow gates passed: 412 verified, 329 open, 320 owning-open, 43 N/A across 784 bullets.
  No Human Guidance wording, deployed TLS/full Live Demo, C410 existing-Assessment update offers,
  whole Course/Assessment milestones, or offline aggregate acceptance is claimed.

- Closed only four Assessment-content HG rows. Accepted actual-server/private bundled-main
  browser proof saved/reloaded mixed Fixed Question/Pool order and exact pins, reordered and
  removed both kinds, retained private retired IDs/old Pool Revision, and reimported distinct fork
  identities with source Pool JSON unchanged. Student direct real HTTP returned 404 without
  current-state change; browser error arrays were empty. Artifact:
  `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`. SQL now supports atomic current-
  position swaps/retired-position reuse and excludes retired entries from workspace reads.
  Supplemental current-schema proof (`/private/tmp/ple-assessment-mixed-entries-artifacts.vdrKfr`)
  compared old/new join predicates with four retained/two active entries and rejected duplicate
  current positions at commit (`23505`) without semantic-state change; no old-index or performance claim.
  Existing connected adoption regression passed 1 test with 0 ignored under the new SQL; supplemental Type
  projections retained unchanged pins: `/private/tmp/ple-shared-assessment-adoption-artifacts.UmP416`.
  The exact Randomize question order label uses the earlier accepted part 04 actual-main/HTTP
  checkbox receipt, not this mixed-entry proof. Assessment splice, diff/consistency, gate, and
  `git diff --check` passed: 406 verified, 335 open, 324 owning-open, 43 N/A across 784 bullets.
  No permanent tests were added. Student Work history, authentication/TLS, full Live Demo, WeBWorK
  delivery, C505 filename rename, and whole-milestone completion are not claimed.

- Closed ten bounded Assessment Type purpose/capability HG rows in documentation only. The real
  Type registry renders Instructor-selected purposes in Course and Blueprint creation; current
  points/extra-credit scoring and Properties controls establish the named capabilities. Existing
  accepted Bonus `8 / 0` and Quiz/Exam one-Attempt receipts remain separate runtime evidence.
  The broad appropriate-defaults and cross-backend Practice immediate-answer rows stay open;
  this does not claim all-settings persistence/enforcement, formal collaboration policy,
  learning-age inference, exam calendars, or complete Student delivery. Assessment splice,
  diff/consistency, gate, and `git diff --check` passed: 402 verified, 339 open, 328 owning-open,
  43 N/A across 784 bullets. No product code or Human Guidance changed.

- Closed only the shared underlying Assessment-model HG row. Canonical teaching types and
  ordinary adoption connect reusable Blueprint content to current Course Instance Assessments;
  distinct storage/lifecycle projections are intentional. Fresh PostgreSQL 17 connected adoption
  proof passed 1 test with 0 ignored, preserving Type, mixed ordered Pool/Fixed entries, nondefault
  teaching rules, exact Revision pins, independent daughter Pool IDs, and unset dates. Artifact:
  `/private/tmp/ple-shared-assessment-adoption-artifacts.IkYuXY`. No production rewrite was needed;
  existing stable adoption-test fixtures now use opaque public References and explicitly close
  their shared application pool before the race, without raising connection limits. Supplemental
  checks stayed outside Git. Assessment splice, diff/consistency, gate, and `git diff --check`
  passed: 392 verified, 349 open, 338 owning-open, 43 N/A across 784 bullets. Every Type's Student
  delivery/completion remains outside this architecture receipt.

- Closed only HG608's distinct algorithmic-Question Pool purpose. Accepted fresh PostgreSQL
  17/MinIO actual-server and private bundled-main HTTP-proxy browser proof selected two distinct
  canonical PGML Questions with Instructor interchangeability attestation, created a reusable
  Pool and distinct Assessment-owned fork selecting one Question, preserved exact provenance,
  reproduction facts, and radio response on resume, and submitted before a fresh new Attempt.
  Artifact: `/private/tmp/ple-algorithmic-pool-artifacts.K2Kk6Z`. Correct answer Never and a
  3600-second time limit bounded the release; answer disclosure, full Live Demo/authentication/TLS,
  all-backend acceptance, and other HG rows remain outside this receipt. Questions splice,
  diff/consistency, gate, and `git diff --check` passed; totals are 391 verified, 350 open,
  339 owning-open, and 43 N/A across 784 bullets.

- Closed the seven bounded algorithmic-source Human Guidance rows. Accepted fresh PostgreSQL
  17/MinIO evidence published the 42 canonical Genetics PGML sources as ordinary WeBWorK
  Revision-1 Questions in nine topics with 42 Fixed entries and zero Pools; exact replay and a
  same-short-name conflict made no mutation. Pilot source/tests enforce explicit PG/PGML format and
  matching extension; connected binding proof preserved source SHA/size/path, immutable replay, and
  stale refusal after an intervening metadata edit. Artifacts: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`
  and `/private/tmp/ple-pilot-format-binding-artifacts.CKzka1`. Chargaff, the non-published
  76-bank/13,434-row inventory, and conditional retained-catalog C840--C841 work remain open.
  The earlier Pilot PGML mismatch statement is superseded by this receipt.

- Closed the exact C351 manual Draft-deletion checklist row. Accepted isolated PostgreSQL 17/MinIO
  actual-server and focused browser proof covered owner cancel/confirm and list-reload persistence,
  owner-only current-ETag deletion, denial without source/Edit Number change for collaborator,
  unrelated Instructor, Student, Sysadmin, and anonymous callers, precondition failures, preserved
  published lineage/Revision JSON, and 404 repeated mutations. The artifact is
  `/private/tmp/ple-draft-delete-artifacts.km9ybM`; this narrow receipt does not claim S3 erasure,
  automatic cleanup, authentication acceptance, full Live Demo browser acceptance, or healthy
  backend behavior with the renderer disabled.

- A fresh Live Demo run passed all 51 screenshot captures and canonical corpus promotion; `SCREENSHOT_ATLAS.md` was
  regenerated, the root static verifier passed, and the three new published PNGs (Template, Pool review, and canonical
  Genetics PGML) received visual inspection. The two real repairs are the native Draft codec and Student View's stale
  `assessment_attempt_limit` manifest key, now `attempt_limit` in its audited reader projection. Current capture
  selectors and the released-feedback privacy profile are corrected. Thirty-one focused Node tests, 7,154 Python tests,
  and PostgreSQL `cargo check` passed. Standard `--headed` operator recapture is documented; the external authenticator
  was temporary proof-only and adds no permanent credential plumbing. This does not claim broader Human Guidance
  completion. Pilot PGML source mislabeled PG by `pilot_content/publication.rs` remains a separate open mismatch.

- Repaired Live Demo screenshot startup: removed the stale browser-compose identity-initializer command and API
  volume overrides, inheriting the canonical TOTP seed, wrapping-key initialization, and read-only API mount.
  Local topology now owns the legitimate `ple_sysadmin_totp_runtime`; focused 61-test ownership/startup checks
  passed, and `./launchers/run_live_demo.sh stop` reports `Developer browser stopped: ple-live-demo-browser`. Startup
  now succeeds; 13 installer and 11 diagnostic tests passed. Broader C351 runtime behavior and final runtime
  acceptance are not claimed.

- A fresh Live Demo replay promoted all 54 canonical screenshot captures and regenerated the atlas and receipt.
  The final three published 1280x800 PNGs were visually inspected: HLA offspring, monohybrid matching, and X-linked
  offspring counts; each was readable unanswered biology. The capture selector now recognizes renderer CSS tables.
  Static verification, 5 Node tests, and 7,151 pytest tests passed. The temporary private external-authenticator
  runner added no permanent credential plumbing, application code, or tests; this narrow evidence does not claim
  broader Human Guidance closure.
