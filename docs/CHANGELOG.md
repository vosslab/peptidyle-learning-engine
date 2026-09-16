# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was
> changed and believed at the time. They are not product authority. Current
> intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old
> Assignment, Blueprint, lifecycle, grading, role, retention, and UI models.

## 2026-09-15

### Behavior or Interface Changes

- Corrected `./launchers/run_live_demo.sh --open` to prepare and start a fresh Live Demo before
  opening it, matching `start --open`. The explicit `open` command continues to open only an
  already-running fixed target, while the default and `--headless` continue to print the origin.

### Additions and New Features

- Removed the invented seventh delayed-release setting for Question Feedback. Assessment and
  Template contracts now carry six independent timing fields; provided native Question Feedback
  and exact-Revision General feedback are shown at the existing submitted-history boundary, while
  response, score, correctness, answer, explanation, and class statistics retain their own gates.
  History now projects required author content through its exact private source row and fails
  closed on Store read/decode errors instead of returning a successful response with missing
  feedback. The completion acknowledgement exposes only Attempt ID and submission state, not a
  transient score. Accepted actual Student HTTP and exact-main browser proof rendered General
  feedback with all six timings `Never`, withheld the protected fields, returned 404 for another
  Student, an Instructor, a Sysadmin, and an anonymous caller. The renderer was stopped after
  issuance and before submission and history. Source and focused domain tests establish provided native feedback independently of
  answer disclosure; the WeBWorK runtime fixture supplied no transient backend feedback. Fresh
  gates passed 7,149 Python tests, 337 Node tests, `npx tsc --noEmit`, both Rust binaries, 14 domain
  feedback tests, 11 assessment-delivery tests, and canonical generation of 351
  TypeScript types. This closes only C910's three bounded feedback rows, not the broader C524 or
  Student-workflow gaps.
- Repaired the trusted curriculum publisher for the current reusable Question Pool contract.
  Unreplaced static banks now publish real immutable Pools with ordered exact Question Revision
  members, and Blueprint Pool entries carry the Pool public ID and selection policy instead of a
  second inline member list. Retained-Blueprint replay loads the exact stored Pool Revision and
  revalidates its ordered Question provenance; accepted canonical replacements remain Fixed
  Questions. The canonical-source validator now enforces the intended exact
  `pg/<topic>/...` and `genetics/<topic>/...` ownership paths instead of rejecting every real
  `pg/topicNN/...` `PathBuf`. Focused publication tests, project-tools check/build, the full Python
  suite, and an isolated PostgreSQL 17/MinIO run passed. That run published three Questions, one
  ordered two-member Pool, and one `[pool, fixed]` Blueprint, then proved an unchanged exact rerun.
  It is publication/pin plumbing evidence, not PGML rendering or algorithmic-source acceptance.
  Bundled Genetics publication remains open: 76 unreplaced banks still reference 13,434 generated
  PG files removed by the user, so those families require canonical-source migration rather than
  restoration of redundant expansions. Repository maintenance also removed 5,958 recoverable
  loose Rust code-generation objects (about 2.2 GiB) only after matching archived libraries were
  verified; libraries, metadata, binaries, and fingerprints were preserved.
- Implemented the bounded C64 Assessment Question-order runtime without closing its Human Guidance
  row yet. The authorized Student start path now holds one transaction and Assessment lock through
  preflight, fixed-and-Pool selection, unbiased whole-vector shuffle, and immutable Attempt
  persistence; immediate resume returns the retained Attempt before source reads or randomness.
  Two connected Pool defects were also corrected: full Assessment saves resolve the canonical
  public Pool ID to the exact Assessment-owned fork, and workspace decoding returns one Pool Entry
  while retaining all member Question facts. Focused Rust compilation, a fresh PostgreSQL 17 SQL
  gate, and independent review passed. Accepted authenticated Student HTTP evidence then covered
  authored order, a complete shuffled fixed-and-Pool vector, a real concurrent Instructor save
  blocked behind the Student-start lock, immutable same-Attempt resume after a current-rule edit,
  exact Question Revision pins, and outsider/anonymous concealment. Accepted exact-main browser
  evidence then saved and reloaded **Randomize question order** through actual HTTP while retaining
  explicit Question-owned answer-choice copy. The PLE Question authoring/codec/adapter/presentation
  chain owns native choice randomization, while the closed Assessment rules expose no choice-order
  override. This closes the two bounded Assessment-randomization rows without claiming a runtime
  matrix of every native choice permutation. The same browser evidence confirmed readable Chicago
  Account-zone Due values in a three-row Course list at desktop and 720px while the browser used Los
  Angeles time; broad representative cross-list scanability remains open.
- Recorded C63's implemented WeBWorK exact-Revision preview path without closing its inspection
  row. The authorized private source and checksum reach the existing opaque adapter and hardened
  iframe, and the page reuses `QuestionAvailabilityClient.getQuestionRevision` rather than adding a
  weaker duplicate client. Focused client tests and the current TypeScript pass. Private PostgreSQL
  17/MinIO plus unchanged-renderer HTTP evidence returned preview 200 with hardened headers and
  concealed missing Revision, Student, and anonymous requests. Exact-main browser evidence visibly
  rendered the prompt and five choices. The renderer JavaScript then
  dereferenced `window.frameElement.id` where the hardened sandbox supplies no same-origin frame
  element, before focus, popover, and parent telemetry. The isolated preview path left all five
  Student Work counts at zero before and after: Assessment Attempts, Question Attempts, saved
  responses, submissions, and grading results. C63 remains open; do not loosen the sandbox or
  rewrite sibling renderer HTML while successful hardened-embed behavior remains unverified.
- Projected each exact Question Revision's optional registered WeBWorK PG path through the
  Instructor-authorized private Question Library query and decoded it into the server-only Store
  entry. A fresh PostgreSQL 17 installation proved the exact path through both list and
  exact-Revision reads, the focused PostgreSQL LDA compile passed, and independent review accepted
  the private DTO boundary. This prerequisite does not implement or verify rendered WeBWorK
  Question preview.
- Closed all five C62 Assessment editor-shell rows plus C63's visible Question-order and direct
  Search/Browse rows. The browser
  contract now matches the server's current equal-co-Instructor Course view and creation receipts
  without obsolete authority booleans;
  Question and Properties routes, Ribbon tasks, headings, and breadcrumbs use their canonical
  names; and the owning editor CSS targets the current Assessment selectors. Accepted private
  PostgreSQL 17, actual-server, and exact-main browser evidence added, moved, removed, re-added,
  saved, and reloaded two Questions, persisted one Properties instructions edit, saved and reloaded
  fixed-Question point values `2.5` and `1`, and verified the grouped two-column desktop and
  one-column 720px Properties layout. Cancel, Stay/Discard, and a real stale-write refusal with
  explicit reload/discard recovery passed. Search and Browse links,
  browser Back, and Stay/Discard unsaved-state handling passed the same actual-server path.
  Broad Assessment-list scanability remains open because the Course list renders a raw local ISO
  due value in a one-row fixture. This does not claim score recalculation, Released-Assessment
  point editing, the meaningful C63 inspection behavior, release workflow, deployment-gateway
  behavior, or WASM runtime. The canonical checklist now contains 784 bullets: 353 verified, 388
  open, and 43 N/A.
- Closed three more C61 Assessments Due Soon rows. The SQL projection now returns the canonical
  Course public reference expected by the Rust model instead of an internal numeric reference, and
  the page stylesheet now targets the current Assessment class names. Accepted private PostgreSQL
  17, actual-server, and exact-main browser evidence covered two owned Courses, outsider-Course
  exclusion, anonymous/Student concealment, `no-store`, and Course/Due values formatted from actual
  HTTP instants in the Account zone. The broad Assessment-list scanability row remains open pending
  a second production list. This does not claim empty/error states, release workflow, WASM runtime,
  deployment gateway, or connected Template delivery. The canonical checklist now contains 784
  bullets: 346 verified, 395 open, and 43 N/A.
- Closed 13 bounded Question Library Human Guidance rows across C58 and C60. A private PostgreSQL
  17 and actual-server HTTP receipt exercised ordinary words, quoted phrases, minus exclusion, all
  five PLE field tags, exact subject/topic filters, full-authorized-snapshot group counts despite
  `page_size=1`, explicit 64-value truncation flags, active-vetted-Instructor access, anonymous and
  Student concealment, and `no-store`. Accepted routed-component evidence separately covered the
  distinct Browse route, subject-to-topic navigation, exact Browse-to-Search handoff, retained
  selected filters, and shared result presentation. A follow-up private exact-main browser run
  authenticated against the actual server, traversed overview through Biology and Enzymes into
  focused Search, and produced accepted production-styled 1280 by 800 screenshots. This closes all
  eight C60 Browse rows without claiming deployment-gateway or WASM-runtime evidence. Expert
  very-large-library behavior remains open. After the concurrent Human Guidance theme split, the
  canonical checklist contains 784 bullets: 343 verified, 398 open, and 43 N/A.
- Documented the demonstrated RDKit/Prettier scope conflict for later starter-repository review.
  The proposal records the explicit negated-glob workaround and the existing protected formatting
  write command, rejects rewriting reviewed dependency bytes, and leaves any propagation change
  unapproved pending inspection and user decision.
- Rebalanced the preliminary biome theme specification around demonstrated visual territory rather
  than a fixed theme count. Five crowded palettes were replaced by Wildflower Meadow, Autumn
  Woodland, Tropical Lagoon, Volcanic Field, and Red Rock Canyon; Glacier was added as a distinct
  ice-cyan identity; and Taiga, Savanna, Heathland, and Kelp Forest were strengthened. All 20
  changed or added light/dark Accent colors pass against their own Canvas and Surface at the
  preferred 5.5:1 comfort target. The specification now requires role-specific contrast: 4.5:1 for
  normal text, 3:1 for large text and meaningful non-text UI, and tested foreground tokens over
  decorative Secondary colors. Lavender Field, Sunflower Meadow, and Cypress Swamp remain
  render-gated candidates.
- Documented the implementation contract needed to turn the preliminary biome palettes into PLE
  Course Themes. The specification now states the current 15-ID, three-anchor, light-only runtime
  gap; requires durable IDs and complete semantic tokens; separates Course Theme from display mode;
  identifies the unresolved legacy-ID/default migration; lists the atomic Rust, PostgreSQL,
  generated-TypeScript, browser, route, and seed cutover; and defines rendered accessibility and
  visual acceptance evidence. This is documentation only and does not claim runtime theme support.
- Established the target Course Theme palette model: every theme provides fixed Canvas, Surface,
  Secondary, and Accent colors in both light and dark mode. Course data continues to store only the
  stable theme ID; display mode selects one four-color set, and one shared tested projection derives
  the remaining semantic tokens with only measured, named exceptions.
- Recast the biome palette document as a normative specification sheet. It now separates scope,
  compatibility, fixed data, runtime records, display mode, migration, rollout, acceptance, and
  unresolved decisions; assigns explicit status to the 25-theme registry and deferred themes; and
  removes exploratory language that could be mistaken for implementation authority.
- Cut over server object-storage composition to the explicit disposable-local
  MinIO topology, removing the direct `aws-config`/`aws.rs` fallback. API/worker
  and publisher credentials remain separate, and the retained S3-compatible SDK
  graph adds no dependency pin. Fresh `server_core` library evidence passed 99
  tests; `objects` with `s3` passed 38 library tests, three conformance tests,
  and one archive test. One isolated loopback-MinIO conformance run passed.
  Separately, the owned container composition passed health with a read-only
  root, data tmpfs, zero persistent volumes, and pull policy `never`; ordinary
  exact cleanup left no owned container or temporary secret and did not touch the
  shared stack. This does not claim PLE HTTP, full-cloud, or deployment evidence.
- Closed C57's three bounded Question Library Search rows. The initial page foregrounds the one
  Search entry and performs no request until input; the first query reveals filters and results.
  A one-time compiled-browser receipt restored query, selected filter, 80 loaded rows, and
  virtual-list position through visible detail return and browser Back, while a changed session
  returned to the empty landing. The temporary harness/screenshots were removed after acceptance;
  this is not connected HTTP evidence. Integrated codebase, pytest, and `server_core` library
  checks passed.
- Replaced the misleading `coral-crossing` provided-avatar artwork with an original decorative Coral reef tile: branching coral, a small fish, and a contained ocean wave. The stable catalog ID and asset path remain unchanged; the canonical manifest and generated Rust, TypeScript, and SQL registry facts now use the name "Coral reef" and its accurate description.
- Accepted C8's source boundary after independent review and a fresh root PostgreSQL 17 rerun:
  Course-fixed exact Pool members, stable IDs, authorization, and retired/inactive handling are
  correct. Connected HTTP and Cargo execution remain pending the AWS Smithy dependency cutover.
- Added `Verification pending:` as the open-checklist qualifier for implemented behavior awaiting
  named proof; `Mismatch:` remains reserved for missing or incorrect behavior. The checklist
  remains 325 verified, 415 open, 43 N/A, and 783 total.
- Added bounded Question Library editing for `tags`, `subject`, and `topic`, with Keep/Replace/Clear
  and refresh-before-resubmit recovery. No-store current reads and atomic CAS updates validate
  canonical IDs; there are no replay receipts or automatic second writes. Native publication seeds
  tags once, successor Revisions preserve edits/clears, and null tag elements fail validation.
  Current metadata now feeds search. PostgreSQL SQL/API, LDA, tsgen, frontend, and injected
  Chromium replace/clear/stale-refresh proof passed. C58's parser receipt covers words, quotes,
  minus, PLE fields, literal unknown tokens, empty fields matching nothing, and exact IDs; HTTP search remains AWS-blocked. C59/C61 close discoverability, two labels, and Template design only.
- Refreshed the accepted Part 09 Assessment definition evidence. The checklist now records the
  implemented Blueprint and Course Instance Assessment variants, reusable Blueprint content,
  Course ownership, editable adopted Course Assessments, Course-owned Assessment Attempts, and
  the absence of Blueprint Assessment Attempts. The C503 direct/adopted PostgreSQL and production-
  mapper receipt supports only the adopted-copy editability row; its standalone proof did not
  rerun the publisher-backed installation-data seed. Fresh actual-Store evidence now covers
  Instructor Exam policy saves that retain Type. The current generator-recorded checklist snapshot
  is 325 verified, 415 open, 43 N/A, and 783 total, with 11 duplicate-owner rows and 404 owning-open
  records.
- Accepted C514-C516 Template and C525 Attempt-completion evidence. Fresh independent PostgreSQL
  17 actual-Store coverage passed Template create/save/read/by-value copy, unlimited retries after
  perfect and nonperfect submissions, Quiz submission/second-Attempt denial, generic deadline
  finalization, and the two-current-Student cohort transition. It did not run Quiz/Exam worker
  finalization directly; that accepted completion row composes the type-independent submission
  authority with Quiz submission, generic expiry, and expired-pending Exam denial. Correct-answer
  release from the cohort fact, connected HTTP delivery, and opaque WeBWorK answers remain open.
- Added the independently accepted C503 Assessment-origin cutover. Manual five-argument creation
  now derives immutable direct origin with no Blueprint fields or source-choice API; Blueprint
  adoption alone records the exact nonnull Blueprint Course, Revision, and Assessment triplet, and
  the Live Demo remains adopted. A fresh PostgreSQL 17 public-API proof passed direct fixed/Pool
  order and exact points, adopted save/load provenance, malformed shapes, and immutability. An
  attested `PostgresLiveAssessmentStore` proof also passed direct create/load/save and adopted
  load/save through the production mapper. The standalone proof did not rerun the full
  publisher-backed installation-data seed, and this receipt closes no checklist row.
- Replaced the unavailable Bonus Assignment and Quiz icon names with the approved Free Solid
  `star` and `circle-question` glyphs. All five Assessment Types now have one guaranteed bundled
  Ribbon glyph and retain visible Type labels in the Student Course landing and Assessment overview.
  The regenerated same-origin sprite, strict TypeScript check, Human Guidance checklist gates, and
  independent review passed. A temporary Chromium proof served the generated sprite and visibly
  confirmed the new glyphs beside their labels. Human Guidance now also defines the one-Attempt
  Quiz/Exam rule, Quiz/Exam submission-or-expiry completion, score- and correctness-independent
  Assessment Attempt completion, and the KISS constraints; their implementation rows remain open.
- Added the independently accepted C514 Assessment Template domain model. The private-UUID
  aggregate owns a validated name, one Assessment Type, a positive CAS Edit Number, and only the
  reusable settings copied into a future Assessment. Its strict payload reuses the canonical
  instructions, non-date policy defaults, nine activity rules, and six feedback-release timings;
  focused compilation, existing rule tests, Clippy, and one-time serialization proof passed. This
  makes the model ready for owner-scoped persistence but does not close the schema, API, UI, copy
  integration, or full Assessment Template workflow.
- Corrected C514 Assessment Template deserialization to reject attempt limits above PostgreSQL
  `INTEGER` range while preserving the exact closed settings payload. Both raw limit fields now
  share the canonical Assessment bounds; maximum and `null` remain valid. A focused ignored serde
  proof and independent SQL parity review passed; no broader Template workflow is claimed.
- Added the independently accepted C515 owner-only Assessment Template persistence and private API
  subset. Active Instructors can list, create, read, and replace only their own strict settings
  aggregates; creation uses server UUIDs and Type-derived canonical defaults, while full saves use
  strong Edit Number ETags without resetting supplied settings after a Type change. A fresh
  PostgreSQL 17 actual-Store proof passed owner/nonowner and inactive-account authorization, stale
  CAS, strict row decoding, and settings round-trip behavior. Focused PostgreSQL-feature LDA
  compilation and strict Clippy passed; full server compilation remains blocked by the existing AWS
  Smithy dependency incompatibility. This does not add the Template UI, copy integration, sharing,
  publishing, history, or full workflow closure.
- Added C519/C520's shared Assessment Release Validation authority and hard gate. Actionable
  readiness now requires a Due date at least 24 hours ahead, no later than the immutable Course
  Active cutoff, with Available no later than Due and Due no later than Closes. Unreleased drafts
  remain correctable; Released saves enforce the same issues while unrelated edits preserve an
  unchanged near or past Due date. C519's valid-range row is also supported by public-save point
  checks, positive-or-null whole-Assessment Attempt/time limits, and the release-required time
  limit: fresh PostgreSQL 17 actual-API proof atomically rejected `1000000001` and
  `1000000000.99999`, then saved and released exact `1000000000.9999`. Fresh PostgreSQL 17 and
  actual-component proofs passed exact date boundaries, refusal/correction/release, all three save
  paths, authorization, and least privilege.
- Aligned Assessment Entry persistence with the existing typed point-value domain: zero through
  `1000000000.9999`, with at most four decimal places. Both complete-save Entry variants reject
  excess precision or range before writing, while table constraints protect alternate writers.
  An independently rerun fresh PostgreSQL 17 proof passed exact maximum, `0.0001`, zero, rollback,
  archived Question-pin preservation, and zero-point release; its temporary helper was removed.
- Added the independently accepted C207 Course deadline synchronization. Assessment release and
  all three authorized save paths now serialize Course-first, reject a Due date after the immutable
  Active cutoff atomically, and maintain the current maximum Unreleased/Released Assessment Due
  fact. Active Course retention follows that maximum or falls back to the cutoff; archive and delete
  freeze the retention anchor while the current maximum fact remains accurate. Canonical Live Demo
  installation now synchronizes its released Assessment and uses the canonical public Blueprint
  lifecycle status. An ignored PostgreSQL 17 proof passed the actual APIs, stale CAS, authorization,
  cap rollback, deterministic two-Assessment concurrency, archive races and freeze, schedules,
  helper ACL denial, and the full seeded Live Demo maximum; independent unchanged rerun accepted it.
  The 2,791 focused source-style tests and scoped diff check also passed, and the temporary proof was
  removed after review.
- Added C511's accepted bounded availability controls to Assessment Properties. Instructors can
  save or clear Available and Closes local date/time pairs in their IANA zone; incomplete pairs
  remain visible and unsaved through recovery. No restrictive Type default is mandated, and the
  undefined collaboration policy remains open. Focused TypeScript, browser, and review gates pass.
- Added C524's accepted bounded Assessment-disclosure defaults and post-submit path. Practice
  defaults correct answers to after submission; Regular and Bonus default them to never; submitted
  responses, correctness, answers, and explanations remain independently timed; Question Feedback
  is shown when provided and has no separate delayed-release state.
  Actual-component dialog and summary proofs passed, and fresh PostgreSQL 17 API proofs established
  no pre-submit response source, native PLE post-submit answer disclosure, and fail-closed Quiz/Exam
  release. `./check_codebase.sh` passed 328 Node tests and the latest manager pytest run passed 7042;
  the source-only publisher filter collected zero runtime-only tests and is not claimed as a pass.
  Universal Practice disclosure remains open for opaque WeBWorK, and C525 remains fully open for
  Quiz/Exam cohort and eventual-release semantics.
- Added the accepted Bonus and entry-level extra-credit Gradebook contribution boundary. Course
  Gradebook and pre-start/current Assessment worth preserve earned points while contributing zero
  points possible for Bonus, Extra Credit, and Excluded entries; raw Question and Assessment
  Attempt performance remains unchanged. A fresh PostgreSQL 17 install and API proof passed, plus
  the focused Rust and browser decoder gates. Configured Attempt selection and Course-total grade
  calculation remain open, so this does not close C510 globally.
- Corrected the production Gradebook to select each Student's highest grading-complete submitted
  Assessment Attempt by earned points calculated from immutable credit and current Question point
  values. Later lower, unsubmitted, and grading-pending Attempts no longer erase an established
  score; the latest Attempt still supplies progress when no score exists. Fresh PostgreSQL 17 API
  proof also preserved the Bonus zero denominator without adding score persistence or Course totals.
- Cut over the Student Course landing to the same highest submitted Assessment-score selection
  while preserving the latest Attempt as the progress, completion, and resume authority. The
  selected Attempt's copied feedback policy controls disclosure, and the direct `assessmentScore`
  contribution accepts Bonus points over a zero denominator without weakening raw Attempt scores.
  Fresh PostgreSQL 17 API proof passed earlier-high/later-low/newest-resumable and inverse
  disclosure cases; strict decoder and compiled Solid browser proofs passed, including Bonus
  `8 / 0`. The focused `learning-data-access` PostgreSQL library check passed in 5.72 seconds
  without warnings, verifying the new model, decoder, and export. Full `server_core` compilation
  remains blocked by the existing incompatible AWS Smithy dependency pair.
- Added the canonical no-write Instructor Student View server boundary. The manifest projects
  current answer-free Assessment policy and transient exact Pool selections with a strong saved
  Edit Number ETag; separate reauthorized reads deliver native PLE presentations and sandboxed
  pre-submission WeBWorK documents without creating Student Work, Attempts, submissions, grades,
  stored objects, or renderer cache entries. The prior metadata-only Assessment `/preview` API is
  no longer registered. Production iMathAS Student View rendering remains explicitly unavailable
  until its existing configured backend is composed into this server path.
- Added the Published Question detail-page Archive action for the current Question Owner. The
  confirmation names the current server-projected title, explains the shared discovery and new-
  selection impact, and preserves exact Revisions and Student Work; stale title, ETag, lifecycle,
  and ownership changes refresh or close the affordance without exposing owner identity. A fresh
  PostgreSQL 17 schema proved that the current non-Author Owner receives the capability while a
  different active Instructor who is an Author does not. Actual-component Chromium proof passed
  confirmation, cancellation, success, failure, stale recovery, permission loss, and narrow layout
  against mock transport. The connected server/browser journey remains unverified because the
  unchanged AWS Smithy dependency incompatibility prevents the full server build; this receipt does
  not claim the broader shared C68 availability-confirmation work.
- Added locally vendored IBM Plex Sans Condensed only for long Citation URL input values, including
  slashed-zero numerals, same-origin production delivery, and a one-time Chromium font receipt.
  Atkinson Hyperlegible Next remains the main PLE font; this adds no broad anchor styling.
- Added active-vetted-Instructor published Question Pool reads: bounded opaque-cursor global
  discovery, server-HMAC-validated current Revision detail, and Course-Instructor-owned exact
  Assessment fork detail reuse the answer-free exact Question Revision projection. Every published
  Pool, including child forks, is reusable; global responses disclose no source provenance,
  Course/Assessment association, or Student facts. A fresh PostgreSQL 17 proof passed global
  paging, current ordered pins, child-of-child reuse, and Student denial. A separate fresh
  PostgreSQL 17 Course-adoption proof confirmed that a Blueprint may pin a child Pool and the
  adopted Assessment fork records that immediate child Revision as its source without flattening
  provenance to the root Pool.
- Cut over the retained Student Assessment Attempt Node contracts to canonical Assessment test
  filenames, source modules, API fields, routes, and opaque Course Instance and Assessment
  references. The 34 focused tests continue to protect disclosure, response persistence,
  request/acknowledgement matching, BackendOwned capture, answer-free navigation, supplied-time-zone
  rendering, and withheld-score behavior. This is a bounded test integration repair; the broader
  Assessment cutover remains open.
- Updated retained transport, decoder, and connected-E2E contracts to use canonical opaque
  `CI`, `A`, and `BP` references and canonical Assessment routes. Removed the retired
  Assignment-access helper test and an internal bundle-export test; their durable behaviors are
  covered by the current policy and route-scope tests. The focused Node contracts, gateway pytest,
  shell syntax checks, formatting, and independent review passed. Broader Assessment test cutover
  remains open.
- Simplified the live browser-suite origin receipt to one direct envelope.
  Aggregate-only journeys write `contexts: null`; journeys that observe each
  BrowserContext write the named context evidence they captured. The oracle
  rejects the retired two-field receipt rather than treating its missing
  distinction as compatible. Existing focused origin-security tests passed;
  the one-time retired-shape probe was removed.
- Registered the accepted canonical PGML Chargaff source as a mapped migration input for the
  Genetics static `chargaff-dna-percent-5-choices` bank. Its record pins the flat bundled path,
  immutable upstream generator, and separate content/source-code licenses. This is one mapped
  family, not catalog publication or completion of the remaining Genetics migration.
- Tightened TypeScript generator output ownership to its current exact header.
  Retired historical generator headers now remain protected as unowned files
  rather than being silently replaced; current generated contracts already use
  the canonical marker.
- Repaired the retained Pool-selection unit fixtures to use canonical 4-4
  Question IDs. Removed the transient-entropy replay test: it constrained the
  random-selection implementation rather than a durable product contract.
  The retained tests protect available pinned-item selection/order and refusal
  when too few available items remain; C353 reusable Pool provenance remains
  open.
- Reclassified Pool selection count from an artificial product question to an
  engineering decision: the existing positive Assessment-entry count belongs to
  the Assessment-owned Pool fork, while the reusable immutable Pool Revision
  owns its members. C905-C909 remain open for exact fork provenance,
  count-bound validation, delivery, and proof; this does not claim those
  behaviors are implemented.
- Blueprint Course adoption now resolves each reusable Pool to one exact root
  Pool Revision, creates an Assessment-owned child Pool lineage at Revision 1,
  and retains original source provenance and member pins atomically with fresh
  Assessments. The Course-side HMAC issuer supplies fork public IDs; a scoped,
  non-public Sysadmin adoption capability preserves the source attestation
  without exposing a standalone Sysadmin Pool operation.
- Made `cargo tsgen` a source-only `project-tools` binary. The regular
  `cargo tools` host retains its runtime commands and dependencies, while the
  TypeScript contract generator now builds without the database, object-store,
  server, or AWS dependency graph. Its optional output-directory argument and
  generated-contract semantics are unchanged.
- Consolidated Human Guidance's durable algorithmic-Question, Question-Pool,
  backend-feedback, human-reference-ID, and WeBWorK PG/PGML source-format
  rules. BiologyProblems.org import and migration requirements apply per
  relevant family, not to one example. This records requirements and checklist
  audit scope, not product closure or a completed catalog migration.
- Added Human Guidance requirements for opaque human-facing references: `BP`, `CI`, `A`, and
  Sysadmin-only `U` prefixes use one common random Crockford Base32 format without a separator;
  public Question and Question Pool `AAAA-ZBBB` IDs remain separate. This records required
  behavior only. Current sequential references remain noncompliant pending implementation.
- Simplified Human Guidance compliance coordination. The checklist remains the
  verbatim, evidence-backed audit record, while the plan and gap map now use
  short owner and handoff notes instead of dependency-parser, count, and
  ownership-ledger requirements. Adaptability remains a binding review
  constraint despite its N/A classification. Student-data minimization now has
  an active owner applying the simplest category-and-operation boundary review,
  rather than waiting for an invented field-allowlist decision. The checklist
  splice operation now replaces one complete manifest part through its next
  part boundary, preventing repeated splices from duplicating a generated
  section.
- Corrected C3's dependency-freshness gate to follow the latest-first policy.
  PyPI requirements now audit as one `>=` floor rather than exact pins, and the
  snapshot no longer permits an AWS downgrade exception. The recorded current
  `aws-sdk-s3` 1.147.0 resolution is audited without freezing the dependency.
  The audit does not duplicate the separate workspace build gate; its upstream
  Smithy failure remains an open build report rather than an excuse to weaken
  the latest-first declaration policy.
- Completed C365's atomic Bulk Published Question metadata database boundary.
  An active vetted Instructor can replace only `tags`, `subject`, and `topic`
  for a bounded distinct selection carrying every current metadata Edit Number.
  The security-definer command locks canonical Question-ID order, validates all
  targets before its first write, advances each metadata Edit Number together,
  and stores an actor-bound opaque idempotency receipt keyed to a canonical
  request digest. Stale, invalid, unavailable, unauthorized, duplicate, or
  oversized requests make no partial change and expose no per-target outcome.
  The ignored fresh PostgreSQL 17 proof covered unvetted denial, replay,
  same-key mismatch, stale/unknown/invalid rollback, and final zero-write
  state; it passed independent review and was removed. C367/C893 own the
  typed Store and HTTP outcomes.
- Completed C877's atomic Question-fork authoring boundary. An active
  Instructor can create one distinct private Draft from an exact Available
  Published Question Revision, with immutable source attribution and an
  actor-scoped opaque idempotency receipt. Concurrent same-key requests return
  the same Draft; a reused key for another source is refused. The stored
  server-allocated compact Question ID is required if that Draft is later
  published. The ignored PostgreSQL 17/RLS proof covered private ownership,
  source pinning, actor scope, mismatch refusal, concurrent retry, and
  fork-versus-archive ordering; it passed independent review and was removed.
  C878-C879 still own the typed server command and Instructor workflow.
- C331's isolated opaque WeBWorK render/pair/grade proof passed. The adapter
  boundary is intact, but required WeBWorK feedback is discarded after the
  renderer score; the Human Guidance bullet remains open pending
  cross-boundary outcome, persistence, and disclosure work.
- Corrected the earlier C900 entry in this section. Its frozen 2025 RDKit
  tarball and versioned immutable-route design conflicts with Human Guidance's
  latest-dependency rule and is historical attempted work, not current intent.
  The current correction uses `@rdkit/rdkit >=2026.3.6` through the npm lock,
  one reviewed local JS/WASM pair, unversioned revalidating routes, and no
  package-version/digest persistence or historical catalog. The current
  generator check (`node devel/sync_author_content_dependency.mjs --check`),
  Rust formatting, and diff check passed. A temporary Caddy-routed Chromium
  probe loaded the exact SRI JS/WASM pair in the opaque sandbox and denied
  parent, storage, cookie, and private-fetch access; it was removed after use.
  The server crate's focused test is blocked by an unrelated incompatible AWS
  Smithy resolution, and the generated checklist currently has an unrelated
  duplicate-status inconsistency. Independent review accepted the two exact
  API-origin CloudFront routes: they forward only canonical `Host`, never
  cookies, query strings, authorization, or other viewer input, while retaining
  the two-file anonymous CORS/CORP exception. This correction does not claim
  C901 or C903 complete.
- Corrected the Human Guidance plan and contracts from a code audit.
  `attempt_presentation.sql` remains the live native and WeBWorK
  issued-presentation/reproduction boundary and is now explicitly a C500
  Assessment-rename consumer; C870 removes only dormant H5P seams. The backend
  contract records H5P as blocked, not supported. The corrected RDKit chain
  uses a current local runtime only; it has no immutable-identity handoff,
  version catalog, or retirement workflow. These are scope and dependency
  corrections, not completed implementation claims.
- The WebWork audit found that the renderer path currently normalizes only
  `problem_result.score`, while Student history supplies default feedback.
  This historical audit made C910 the atomic contributor for typed,
  policy-gated backend feedback. It is superseded by the current C910 entry:
  Question Feedback has no separate delayed-release state, while the six
  disclosure timings remain independently gated. The original dependency
  correction did not itself establish feedback completion.
- Completed C371's Published Question Star closure. A Star is now a visible
  favorite/endorsement, and an active vetted Instructor can see its count and
  the exact vetted display names of its endorsers. The permanent isolated
  PostgreSQL/server test passed with the closed name-only response and
  concealment for anonymous, Student, inactive-Instructor, and non-Published
  requests. A one-time compiled Chromium check confirmed the accessible plain
  text name list has no profile link, control, or avatar; it was removed after
  review. C347 may now use C371 as its completed Star prerequisite.
- Historical C900 record, superseded by the latest-first correction above:
  the former frozen `@rdkit/rdkit@2025.3.4-1.0.0` manifest and versioned
  runtime catalog were removed because they conflict with Human Guidance.
  Current C900/C902 use the npm lockfile's reviewed current release and one
  unversioned local `RDKit_minimal.js`/`.wasm` pair. The generator still
  rejects tampered, extra, or nonregular runtime files and generated-registry
  drift; it never permits an npm/CDN/author URL at runtime. The earlier
  clean-cache/reproducibility matrix remains historical evidence, not a
  claim that the frozen artifact is current authority.
- Added C414's canonical model-layer Blueprint export projection. It serializes only reusable
  short/long names, authored module and Blueprint Assessment order, reusable Assessment settings,
  and exact Published Question Revision pins and published Question Pools. It carries no owner,
  source or revision identity, visibility, Star/Watch, Course, Student, delivery, or private
  operational state; relational storage remains primary. C415 still owns import, comparison-store,
  and exchange workflow behavior. The ignored behavioral fixture proved deterministic bytes,
  authored order, complete fixed/pool settings, and identity omission, then was removed after
  independent review. No permanent test was warranted under `docs/PYTEST_STYLE.md`.
- Completed C876's Question-fork source-pin boundary. An active Instructor's
  server command can resolve only one exact immutable Revision of an Available
  Published Question; missing, archived, wrong-revision, Student, and
  unauthenticated requests receive no source fact. This schema phase creates no
  Draft, attribution, authoring operation, or client-facing identity path. The
  ignored fresh PostgreSQL 17 proof passed and remains temporary under the
  plan's test-liability policy.
- Completed C856's Blueprint Star verified-name projection. Only an active
  vetted Instructor viewing a Public or Archived Blueprint can receive active
  vetted endorsers' exact immutable display names; the separately closed,
  no-store response contains no email, UUID, Account/Profile link, avatar,
  Course, substitute identifier, Star aggregate, or Watch fact. An independent
  fresh-PostgreSQL multi-identity review passed. Its ignored disposable-stack
  matrix remains temporary because it does not earn a permanent test.
- Completed C862's remaining lifecycle-consumer cutover. Blueprint Course
  adoption and reusable-assignment source selection now accept only Public
  Blueprints; Private courses remain owner-only and Archived courses remain
  browseable history rather than new selection sources. The TypeScript check,
  lifecycle-model contract, lint, formatting, and narrow diff checks pass. The
  existing decoder-client test is blocked before execution by a concurrent
  missing generated API constant, so it supplies no result for this change.
- Completed C885's trusted Pool schema boundary. Its create and append
  procedures accept a canonical compact ID only at Revision 1, preserve the
  database collision authority, and retain each Revision's nonempty ordered
  distinct exact Published Question Revision pins plus its active Instructor
  interchangeability attestation. Appends require and replace a metadata ETag
  atomically; member sets cannot be changed after commit. Membership remains
  backend-neutral and stores no selected-count setting. There is no
  `ple_app`/browser mutation grant or public coordinator; C886-C887 own the
  allocator, route, retry, and workflow. The ignored PostgreSQL 17
  uniqueness/revision/member-pin/RLS matrix passed and was removed after
  review. No permanent test was warranted.
- Implemented C342's narrow schema projection for public content references.
  An active Instructor can receive only canonical `AAAA-ZBBB` Question and
  Pool references with their current Revision Numbers; it exposes no UUID,
  Pool contents, selection rule, UI, or lineage detail. The fresh PostgreSQL
  17 proof passed and was temporary-only. C342 remains dependent on C887's
  actual Pool-creation closure and this entry does not claim that workflow.
- Completed C343's bounded Question-ID collision retry. New-lineage
  publication now receives a typed identity-collision result only when the
  PostgreSQL adapter confirms SQLSTATE `23505` on `published_question_pkey`.
  It deletes and retries only that unregistered object; every other Store or
  object outcome retains potentially committed evidence. The permanent tests
  protect retry cleanup, capped exhaustion, fail-closed cleanup, and ambiguous
  outcome retention; the ignored one-time probe was removed after review.
- Completed C854's Published Question Star display-name surface. The Question
  detail page renders only the exact verified Instructor display names in the
  server's closed Star projection, as plain accessible list text. It performs
  no identity lookup and introduces no Profile link, avatar, email, UUID,
  Account reference, Course, substitute identity, or Watch disclosure. An
  ignored SSR rendered-name/accessibility probe passed and remains temporary;
  it is not a permanent component snapshot.
- Completed C354's Question Pool public-identity and immutable-revision
  persistence seam. Pools now retain a server-issued compact Crockford public
  ID with database collision authority, begin at Revision 1, and append only
  the expected next immutable Revision under a lineage lock. Direct table
  mutation remains unavailable to the application role; active Instructor
  capability is required for the narrow creation and CAS append procedures.
  A disposable PostgreSQL 17 proof passed shape/collision, authorization,
  immutability, stale-CAS, and two-session race checks. It remains ignored
  temporary evidence because its container-backed implementation matrix does
  not earn a permanent test.
- Completed C853's Question Star verified-name projection. Only an active
  Instructor viewing a Published Question can receive its active vetted
  endorsers' exact immutable display names; the closed response has no Account
  identity, email, avatar, Course, substitute identifier, or Watch state. An
  ignored fresh-PostgreSQL multi-identity proof passed and remains temporary
  because its setup-heavy matrix does not earn a permanent test. Invalid,
  revoked, expired, or deactivated sessions and HMAC-valid non-Published
  Questions now take the same concealed 404 disclosure path; genuine session
  storage failures remain unavailable errors.
- Completed C361's WebWork test-liability cleanup. The durable tests retain
  opaque issued-presentation and grading outcomes, stateless lifecycle
  rejection, and the boundary that rejects native PLE responses before the
  renderer. They no longer constrain renderer call counts or call order; the
  native-response test now fails immediately if the renderer is reached.
- Completed C57's bounded Question Library return path. Opening a Question now
  retains the active search, selected filters, loaded server-validated browse
  pages, and Library scroll position for one immediate in-document return;
  unrelated Library visits still begin with Search. The implementation proof is
  ignored temporary verification, not a new permanent test.
- Completed C880's Blueprint fork-sync schema foundation. Each C412 fork now
  receives private, append-only immutable baselines for its short name, long
  name, every stable-reference whole Blueprint Assessment, and its ordered
  Assessment list. The schema has no application-facing source read path; it
  preserves origin-linked source snapshots for the later authorized C881-C884
  comparison and selected-application chain. A disposable PostgreSQL 17 probe
  verified source linkage, private nonenumeration, immutability, and append-only
  unit history; the ignored temporary proof was removed after acceptance.
- Completed C881's canonical Blueprint fork comparator. It classifies only
  short name, long name, each stable-reference whole Blueprint Assessment, and
  the ordered Assessment list as safe, already applied, conflict, or not
  applicable from immutable base/source/fork values. Canonical Assessment
  equality includes reusable settings, Published Question Revision pins, and
  Question Pool content; ownership, visibility, Star, Watch, adoption, Course,
  Student, and other operational state cannot enter the comparison type. The
  ignored deterministic matrix covered every status, Assessment addition and
  removal, and exact-once stable ordering, then was removed after independent
  review. No permanent fixture inventory was warranted.
- Completed C83's one-time Student endpoint authorization discovery. The
  ignored proof traced 25 endpoints across Student-only, shared-self,
  authenticated-membership, authenticated-asset, public-renderer-asset, and
  Student-denied classes through their server-side session and Store/SQL
  authority boundaries, independently of frontend route admission. It found no
  server authorization flaw and deliberately creates no permanent
  endpoint-inventory test.
- Implemented C202's bounded Question-recognition improvement: every existing
  shared Question-reference control now receives and shows its already-present
  Question title alongside the canonical copyable reference. The planned
  disposable-stack browser proof remains pending its suite owner's input, and
  this contributor does not claim the wider C216 recognition/copy-surface
  inventory.
- Completed the C6 reusable Blueprint-content schema boundary. The closed
  content model now carries the bounded immutable facts required by Human
  Guidance; the service and browser vertical remain pending and are not
  claimed by this record.
- Completed C17's immutable, auditable Instructor-vetting store foundation.
  It records the durable vetting decision without claiming the separate C18
  Account-creation workflow.
- Completed C803's Sysadmin TOTP foundation: encrypted and zeroized
  database-bound seed handling plus account/browser-bound attestations,
  replay protection, and rate limiting. Session completion and the remaining
  C804-C807 ceremony are still pending.
- Completed the C37/C38/C812 Profile-image storage foundation. Role-neutral
  exact private `ProfileImage` schema, Object Address, and Learning Data
  Access support replace the legacy thumbnail representation; C39 and the
  browser-facing Profile work remain pending.
- Completed C815's Course Banner rendition pipeline foundation. It produces
  one complete, oriented, exact-5:1 no-crop rendition. Focused acceptance
  passed; the lease-owned end-to-end browser lane remains pending.
- Completed C824's local evidence mapping for 12 candidate parameterized
  Genetics sources, including hashes, provenance, and license pins. It is a
  source-selection contributor only; C838-C841 still own publication,
  equivalence, archive, historical-preservation, and recovery behavior.
- Completed C832/C833/C835's Profile-image catalog foundation: generated
  original safe SVG assets, catalog schema, and picker contribution. C834,
  C836, and the browser route work remain pending, so this does not claim
  selectable Profile-image delivery.
- Completed C338's Question Library bulk-selection request seam. It accepts
  only a nonempty, duplicate-free list of canonical Published Question IDs and
  carries no operation, metadata, route, or simulated mutation. C365 owns the
  real bulk request boundary and C366 its UI. The ignored temporary proof was
  run with the repository's TypeScript loader and is not a permanent test.

### Fixes and Maintenance

- Archived the complete 2026-09-14 day block verbatim in `CHANGELOG-2026-09j.md`; the active
  changelog now retains the current 2026-09-15 block.
- Reopened C880-C884: C880/C881's completed foundation could not support the full Blueprint fork-
  update workflow. Removed its unused four-state Rust comparator and private append-only per-unit
  JSON baselines, which duplicated C412's immutable source origin/Revisions, omitted module-label
  changes, and could not atomically apply content with placement. C412 retains its independent Private fork, exact source Revision, immutable Course/Revision ancestry, and authorization/nonenumeration.
  Comparison, review, and selective application remain open through an ordinary-CAS full-tree save
  from existing origin, source, and fork Revisions that may select related changes. Fresh C412 PostgreSQL 17 proof, focused `question_model` checks/tests/Clippy, hygiene, and review passed; no replacement baseline, merge framework, public status family, or permanent test was added, and C880/C881 completions are superseded historical evidence.
- Corrected C847-C851/C209 retention-notification privacy/KISS and absolute-retention boundaries.
  Student-data deletion uses its configured scheduled archive deadline, not late observed archive time; an overdue Course archives then deletes in one pass.
  Observed archive time remains immutable evidence; ordinary recovery and Assessment definitions remain.
  Claims recheck the current eligible active Instructor and verified address; the private table no longer snapshots `verified_destination`. Removed `delivered_at`, delivery-record API/store/server callback, fourth notifier capability/grant/attestation, unused transport fields/outcome enum; claims advance `next_attempt_at` to lease expiry.
  Provider acceptance promises neither inbox delivery nor provider exactly-once, and delivery remains unimplemented. Preserved unique Course/action/due/recipient dedup/concealment, lease/SKIP LOCKED/order, stable pre-provider key, terminal no-resend, failure backoff, redaction, NotConfigured failure, and absolute nonblocking archive/delete.
  Fresh PostgreSQL 17 proof passed recipient/address changes, distinct concurrent claims, dedup, lease/key reuse, accepted no-resend, failure then archive, absent snapshot/delivery columns, and exact three-function notifier authority; temporary proof removed. Notification-delivery Human Guidance remains OPEN because no provider exists.
- Verified the direct preproduction Assessment cutover has no legacy Assignment browser redirect or
  410 compatibility route/caller; authorities expose canonical Assessment paths. Its one-release
  redirect allowance is superseded historical evidence, not current policy.
- Corrected C523's server-owned Assessment Attempt cutoff. New Attempts now expire at the
  earliest effective time limit, Closes, or Due when late work is rejected; `accept` and
  `mark_late` continue past Due, and accommodated Due values remain pinned in the Attempt.
  Response saves now share the Assessment -> Assessment Attempt -> Question Attempt lock order
  with finalization so a valid pre-Due response cannot be stranded by a worker race. Fixed-shape
  finalization rows also retain the declared generated-parameter checksum placeholder for empty
  or otherwise source-free outcomes. A fresh PostgreSQL 17 canonical-schema proof passed actual
  Student and expiry-worker APIs, including post-Due refusal, zero-saved submission with no
  Question Backend grading, deterministic stale-snapshot rejection, reprepare, and preservation
  of both pre-Due saved responses. The ignored proof was removed after independent acceptance;
  this does not claim broader C519 release-validation closure.
- Repaired the Student Assessment Attempt response-source history reader to reuse the existing
  exact Student-record ownership capability instead of requiring direct Student-record and Course-
  membership table reads. The ordinary retention fence remains intact and no table authority was
  widened. A fresh PostgreSQL 17 actual-API proof returned one row only to the owning active
  Student, zero to another Student or an Instructor, zero after archive, and false ownership after
  membership ended.
- Corrected the Human Guidance assessment-compliance evidence: the live Gradebook query selects the
  latest Assessment Attempt by start time, while the prior two highest-score closures cited an
  unconnected domain rule. Both rows are reopened; the duplicate row retains its owning pointer.
- Cut over the retained Student entry, Course appearance, and deferred Ribbon browser harnesses to
  current Assessment contracts, opaque public references, and route-local initialization. Focused
  compiled-component browser evidence passed the Student navigation, Course theme, accessibility,
  scope-recovery, and deferred-content behaviors; this is test integration proof, not live-server
  acceptance.
- Made the Student Course landing a Coursework list with distinct upcoming, available, in-progress,
  completed, and missed states derived from server-calculated resumability. Each row now scans the
  canonical Assessment Type label and available exact icon, due time and display zone, and
  completion. Focused Node tests and a reviewed temporary Chromium fixture passed at 320px and
  1280px with keyboard order and overflow checks. The two unresolved exact Type icons remain a
  shared-registry dependency, so this does not claim full C75 or C77 closure.
- Repaired the Assessment Workspace CSS selector cutover so current policy, responsive, and
  Danger Zone rules apply while shared editor classes remain intact; canonical Assessment labels
  now identify the two editor surfaces. A temporary Chromium fixture verified the distinct
  destructive panel and readable confirmation action at 1280px and 600px after review.
- Repaired the Question Asset Publication claim to use the canonical Job attempt fields and
  unambiguous Job-qualified CAS predicates. A fresh PostgreSQL 17 proof passed exact claim,
  single-attempt increment, active-lease exclusion, and atomic Pending-to-Ready activation.
- Repaired the deferred Question Asset Publication-to-Job invariant. The trigger now reads the
  referenced current Job row and null-safely checks its exact kind, target, worker, Question, and
  Revision instead of reading nonexistent publication fields; a fresh PostgreSQL 17 proof passed
  valid Pending/Ready commits and rejected wrong bindings without disabling triggers or RLS.
- Split accepted publication receipt and Question Library test owners into focused modules, and
  modularized Question-authoring and Blueprint SQL without changing their statements or behavior.
  Focused formatting and line-limit gates plus a fresh PostgreSQL schema install passed; the full
  Cargo compile remains blocked by the unrelated AWS Smithy dependency incompatibility.
- Set the local combined compiled-artifact budget to under 10 GB across workspace and temporary
  build targets. The development guide now preserves useful compatible caches, requires exact
  target, owner, and active-process inspection before an explicit cleanup decision, and does not
  treat the budget as an entire-checkout limit or recurring test threshold.
- Repaired durable contract and archived release-readiness links after the Assessment source
  cutover, and restored the required rationale fields for three existing design decisions.
- Synchronized shared style guides, tests, and repository support files from the starter template.
- Synchronized shared style guides, tests, and repository support files from the starter template.

### Decisions and Failures

- Recorded blocked future H5P C863-C869 architecture and C870 immediate dormant
  placeholder removal. H5P has no delivered runtime until its exact content,
  library, terminal xAPI, and scoreless-activity question is answered; no
  compatibility path is authorized. This is planning, not completion.
- Recorded C862's Blueprint lifecycle browser-codec cutover. Generated
  `BlueprintAvailability` accepts only `Private|Public|Archived`; the decoder,
  client fixtures, and executable generated consumer reject legacy aliases.
  C49/C72 hand this bounded client work to C50/C19 without expanding C50's
  workspace ownership. This is approved planning, not completion.
- Reclassified the optional abandoned-Draft-cleanup sentence as an audited N/A:
  it supplies no clock or durations. C326/C352/C374/C375 are removed from
  dispatch; C861 will remove prohibited placeholder cleanup seams while
  preserving manual Draft deletion and publication. The unanswered cleanup
  policy question remains explicit. This is approved planning, not completion.
- Recorded the C303/C857-C860 author-JavaScript correction. C303 is now an
  architectural handoff, while the answer-free descriptor, authenticated
  no-store isolated document, locked-down frame, and connected proof own the
  implementation. C859 alone owns the five isolation behaviors; C304 waits for
  C860. The document permits only reviewed local libraries and a bootstrap
  nonce, never author URLs or PLE authority. This is approved planning, not
  completion.
- Recorded the C856 Blueprint Star identity split. C409 owns Star/unstar/count,
  private self Watch state, and lifecycle Watch fan-out without names; C856 may
  disclose exact vetted names only to an active vetted Instructor on a
  Public/Archived Blueprint Star list, never Watch identities/state or other
  account/Course identifiers. C423 is the final browser closure. This is
  approved planning, not completion.
- Recorded the C371 Verified Instructor Display Name boundary. The bounded
  server-controlled name is created only by C17/C18 vetting/Account creation,
  never self-edited or surfaced through Profile/directory projections. C852-C855
  carry it through authorized Published-Question Star SQL/LDA/server/frontend
  delivery; exact names, not a self/count-only route, are required before C371
  closes. The retained-test candidate is the stable real-session disclosure
  authorization/privacy outcome. This is approved planning, not completion.
- Recorded the C209 retention-notification delivery boundary. C847-C851 use
  only `warn_inactive`/`notify_archive` receipts, one verified Instructor
  destination per claim, terminal provider acceptance, callback updates to the
  same idempotency key, a disabled recorded-failure adapter, two isolated
  non-inheriting worker database pools, and due-time-order nonblocking
  transitions. Invitation export, Mail.app, fake success, and Live Demo
  delivery evidence are excluded. This is approved implementation planning,
  not a completion claim.
- Recorded the architect-approved direct preproduction Question-ID cutover.
  Question storage is compact `AAAAZBBB`; display and serde are `AAAA-ZBBB`;
  the middle compact character is the HMAC high-five-bit Crockford check over
  the other seven identity characters. The server validates before lookup, the
  existing specification owns accepted normalization, and a nonempty published
  Question table stops the fresh-schema cutover for escalation. C842-C846 now
  provide its atomic foundation, schema/fixture, browser, sweep, and integrated
  proof handoffs before C319/C342/C343/C354. No dual parser, legacy rewrite, or
  compatibility reader is authorized. This is an implementation plan decision,
  not a completion claim.
- Accepted the C832-C837 Avatar planning boundary: Profile images will use a
  PLE-provided, first-party generated-SVG catalog with stable selectable and
  retired IDs, SVG provenance, and a safe grammar. There is no avatar-list
  API. C40 waits for C819/C820, and cross-Account staff Profile-image delivery
  remains an explicit product question. This is accepted atomic planning only;
  it does not claim implementation.
- Corrected the Genetics parameterized-replacement planning boundary from 11
  to 12 sources. C824 supplies the evidence, and C838-C841 cover 12 new
  lineages, new Blueprint CAS, per-bank all-199 equivalence, conditional
  archive, historical preservation, and forward recovery. This is accepted
  atomic lifecycle planning only; it does not claim implementation.
- Accepted the A9 Human Guidance Milestone G plan and checklist map. Canonical
  C500-C536 is a 37-row atomic authoritative ledger with 104 owning behaviors,
  five duplicate pointers, and an acyclic dependency graph; C502 is
  contributor-only. C525 (cohort/completion) and C536 (export
  identity/privacy) remain terminal Human-Guidance-unresolved product
  questions. The plan includes DD-A9-01 and C12's complete Assessment
  terminology-cutover chain. Its temporary-proof, remove-by-default test
  policy is accepted. This is planning and audit evidence only; it does not
  claim product implementation.
- Accepted the A8 Human Guidance Milestone G plan and checklist map. Canonical
  C400-C425 accounts for 64 owning behaviors through 22 closure and four
  contributor milestones, with six duplicate pointers and an acyclic
  dependency graph. Its temporary-proof-first policy and independent planning
  review are accepted. This is planning and audit evidence only; it does not
  claim product implementation.
- Recorded DD-A9-01, the durable Assessment terminology decision. Assessment
  is the only generic object; Assignment remains only one of the three Type
  display names. Preproduction uses a direct cutover to the exact canonical
  routes and JSON shape, with no legacy API or runtime-import compatibility.
  One announced release may provide only a finite, safe browser GET/HEAD
  redirect before it is removed. This records the approved implementation
  boundary, not completion of that cutover.
- Recorded the Course Banner rendition decision. A banner is one complete,
  oriented, exact-5:1, no-crop rendition; 1280 by 256 is guidance and the
  generated output, not an input minimum. Preproduction uses a direct cutover.
  This records the approved implementation boundary, not completion of it.
- Recorded the architect-approved `Static`/`Seeded` attempt-reproduction
  decision. Native `pleQuestionJson` is `Static` and has neither a
  `QuestionSeed` nor generated-parameter hash; a PLE presentation nonce only
  binds response-item and authored choice ordering. Descriptor evidence
  checksum v2 binds the tag and its applicable facts. WeBWorK and seed-using
  iMathAS delivery remain backend-owned `Seeded` cases; C306 must make an H5P
  binding choose a tag before H5P delivery. The preproduction cutover replaces
  the generic seed shape directly, reinitializes a fresh database, and forbids
  sentinels, null ambiguity, and compatibility shims. One public no-seed
  contract test is justified; its vertical database proof is temporary and is
  removed after use. This is a durable design and contract decision, not a
  claim that the current implementation has completed the cutover.
- Recorded the approved self-only Account Settings boundary. Every signed-in
  Product Role uses `/account-settings` and `GET` / `PUT /api/account/settings`
  for the one closed exact-IANA time-zone preference; callers select no Account,
  Course, or role, and PostgreSQL derives the active Account in the atomic
  update. The change affects display and later Instructor wall-clock entry, not
  stored instants. Profile Settings retains avatar work, while Instructor
  Profile displays and links to the preference. Account Settings does not
  expose credential controls; the required Student and Instructor passwordless
  behavior and multiple Student passkeys remain Accounts-and-roles work. Only
  self-service credential enumeration, revocation, re-authentication,
  identity-proofed recovery, notification, and session-termination semantics
  await a separate decision. C15's Sysadmin TOTP session boundary is unchanged.
- Corrected the pending H5P C863-C869 planning language to remove dependency and
  runtime pinning. Immutable `.h5p` declared library/version metadata and SHA256
  remain reproducible content evidence; the rootless Node.js Lumi runtime has no
  prescribed release. Content-type and terminal outcome decisions remain blocked,
  and no H5P delivery or completion status changed.
