# Changelog

## 2026-08-09

### Additions and New Features

- Completed WP-QTI-10 author UI over the accepted server DTOs. The existing workspace route now
  composes a feature-local, same-origin QTI import panel with the existing flat editor; it adds no
  product route, global browser contract, browser ZIP/XML parser, archive/report persistence, or
  private-answer fields. Authors can upload opaque ZIP bytes, manually refresh queued/processing
  work, review recognized profile/default/warning details and visually distinct accepted/rejected
  items, acknowledge the current report, and convert one accepted item only with the exact displayed
  clean strong draft revision. Exact retry preserves identity only after an ambiguous upload;
  all-rejected and unsupported packages retain clear recovery. After a committed conversion, the
  previous editor stays inert until the replacement refetch succeeds; a failed refetch keeps it
  locked and provides a repeatable reload action without repeating conversion or creating a new
  import. The existing flat editor receives focus only after successful load. Permanent Node tests,
  four real-route Chromium scenarios, and the 11-check full gate passed (173 Node and 184 server
  tests in that gate); independent security and HCI re-reviews found no P0/P1 issue. WP-QTI-11,
  the live PostgreSQL/RLS/profile-to-native gate, is next and unstarted. Evidence is in
  `docs/active_plans/workstreams/qti_author_ui_implementation.md`.

- Completed WP-QTI-9 server ingress, report, conversion, and publication orchestration. An author
  upload retains an exact bounded ZIP in a deterministic private workspace object and enqueues one
  deterministic `qtiImport` job; exact replay is stable and divergent replay refuses. The strict
  profile worker exposes an answer-free report with package/item defaults and acknowledgement
  digests, stages complete accepted-item evidence, and keeps mixed-vendor or all-rejected results
  from creating a conversion candidate. Strong-ETag conversion rereads and reparses the retained
  archive before the atomic WP-QTI-8 Store command, while publication copies the exact archive to a
  deterministic non-signable `PublishedImportArchive`. Memory and PostgreSQL now serialize prepared
  import work with draft deletion. The adapter (93 unit, 6 conformance, 12 documentation), objects
  (17 unit, 3 conformance, 1 published-archive), learning-data-access (93 unit, 39 conformance,
  3 documentation; one database-only unit and 7 live PostgreSQL integration tests ignored as
  documented), and server (184 library,
  1 main, 1 documentation) gates passed, as did strict Clippy, workspace all-target/all-feature
  check, formatting, and 5 crate-boundary checks. A one-time 32 MiB plus one-byte chunked ingress
  probe passed and was removed. Independent P0/P1 route and worker reviews reported PASS. WP-QTI-10
  author UI is next; the WP-QTI-11 live PostgreSQL/RLS/profile-to-native gate remains pending.

- Added an execution-ready bot-traffic cost-containment plan for M6. It separates a tiny, API-free
  public landing host from the authenticated PLE application; keeps app APIs same-origin with
  host-only cookies; requires cheap authorization refusal before Store, object, queue, renderer, or
  provider work; and adds semantic cache separation with tunable TTLs, provider-neutral credential
  refusal, private origins and health, bounded secret rotation, reference-safe manifest cleanup,
  evidence-derived rate and capacity settings, a normalized bot-cost formula, authenticated
  automation concurrency and idempotency bounds, spend controls, and an operator runbook. Production
  is fixed to private S3 origins behind CloudFront. GitHub Pages is limited to a qualifying
  non-commercial project showcase
  because its current terms prohibit using free Pages hosting to operate an online business or
  commercial SaaS. The plan exposes the remaining M6 deployment-tool and production
  `IdentityProvider` decisions and now applies the repository's permanent-test checklist.
  Deterministic offline behavior stays in unit/conformance or local Playwright tests, while cloud,
  network, cost, transfer, visual, mutation, and capacity probes
  are one-time evidence rather than regular tests. Unsupported byte, pixel, timing, percentage,
  retention-count, and rate constants were removed from permanent gates. Independent red-team review
  also removed assumed traffic evidence, noisy negative-cost failure, diagnostic alert boilerplate,
  unsafe secret-materialization claims, and external navigation from the regular Playwright path.
  Durable human guidance now records that discipline explicitly.

- Added a formal project status report and refreshed the README status surfaces. They now separate
  implemented and verified capabilities from accepted plans, identify WP-QTI-9 as the next active
  package, record the seven flat-question families still missing beyond single choice, and make the
  deployment, course-appearance, content-pack, object-reconciliation, and release blockers explicit.
  The complete 11-stage repository check passed after generated Cargo artifacts were cleared.

- Completed a focused frontend/backend security pass. Learner response recovery now uses only
  `sessionStorage` and clears the active buffer on run exit; file-upload answers fail closed until
  a server-issued upload capability exists. Generic QTI rejects active SVG, and PDF export bounds
  PNG dimensions, decoded bytes, and zlib expansion before allocation. Catalog revision and
  lifecycle writes now require tenant-qualified ownership in Memory, SQLx, and forced RLS, while
  API readiness reruns the exact migration/checksum compatibility verifier rather than treating a
  reachable database as compatible. Static SQL review found only bound SQLx values, the npm audit
  reported no production dependency advisories, focused security tests passed, and the fresh
  six-migration PostgreSQL baseline passed every real-role and denial oracle.

- Completed WP-QTI-8 Memory/PostgreSQL provenance-aware conversion. A closed staged profile-
  evidence type closes H2 and binds conversion to the committed accepted result's exact
  `sourceIdentifier`/`itemId`, profile tuple, and digest set. Both backends atomically commit the
  draft CAS revision, canonical source, current private grading, and current origin under the frozen
  lock order. Ordinary saves also stage current grading; publication accepts no caller grading
  payload and promotes only the locked stored value after origin promotion. PostgreSQL uses the
  forced-RLS provenance and grading brokers with no direct Store reads of private grading,
  choice-map, or provenance secret tables. `Sha256Digest` now uses strict lowercase 64-hex JSON
  serialization. Shared conformance, PostgreSQL feature coverage, the full fresh baseline, and
  independent review passed with no P0/P1 finding. WP-QTI-9 server routes are next; the already-
  frozen course appearance plan remains separate.

- Refreshed durable human guidance in the owner's wording: use GPT-5.6 agents,
  favor positive prompting and parallel atomic tasks, support the eight named
  PLE QTI-JSON question families, keep feedback optional, use QTI Package Maker
  as a low-priority conversion reference, and begin with four mixed WeBWorK and
  static questions in each Chapter 1 genetics and biochemistry assignment.

- Recorded the owner's Blackboard-inspired course appearance guidance and added an execution-ready
  M3 plan. The plan keeps 15 measured three-color biome/habitat themes, consolidates only `woodland`
  into `forest`, applies one theme across every course-owned route, and keeps one small centered
  banner on the course entry page. Atomic revision handling, protected object lifecycle, forced RLS,
  fixed server-owned image normalization, accessibility/contrast metrics, visual contact sheets,
  and independent implementation review are required before the feature is complete. Independent
  architecture and HCI reviews of the plan itself reported PASS with no remaining P0/P1 finding;
  the separate QTI sequence remains dependency ordered.

- Refreshed WP-QTI-7 schema/RLS/object-binding implementation evidence after the choice-map checksum
  repair. A dedicated `NOLOGIN`, `NOINHERIT`, `NOBYPASSRLS` provenance broker owns narrow protected
  functions over six forced-RLS current/published origin, private choice-map, and profile/item-
  evidence relations. PostgreSQL recomputes SHA-256 over private choice-map bytes in direct table
  triggers for current and published maps, so divergent digests are refused even for a direct
  provenance-broker write. Current lineage pins the committed import; ordinary draft cleanup
  releases only that current lineage; published lineage and maps remain immutable and retained.
  Origin writes bind the full committed typed `ObjectRecord`, including key classification,
  checksum, size, media type, license, provenance, and creation time. SQL now matches Rust's
  1,024-Unicode-scalar source-item boundary across every linked QTI surface. The final fresh
  baseline pass applied all six migrations, re-applied without change, verified the ledger, and
  exercised capability-negative and direct-broker negative provenance probes with the real-role
  RLS/pin/cleanup oracle. Final independent checksum re-review reported PASS with no P0/P1
  findings; the subsequently completed WP-QTI-8 backend conversion is recorded above.

- Completed Q4/WP-QTI-6 provenance contract and archive-object identity. The adapter now emits a
  versioned private ordered choice-map payload with a fixed binary encoding and checksum; the
  storage boundary owns closed profile/conversion values, current and immutable published origins,
  private payload redaction, fail-closed promotion, and one atomic conversion command. The
  `PublishedImportArchive` key is tenant-bound, deterministic, non-signable, and retained with the
  published version. Focused adapter, data-access, object, bridge, formatting, Clippy, boundary,
  and diff gates passed; independent review reported PASS with no P0/P1 finding. The subsequent
  WP-QTI-7 schema package reconciles the Rust and SQL source-item boundary at 1,024 Unicode scalars;
  WP-QTI-8 backend conversion remains next.

- Added the pure server-only QTI-to-flat bridge. It routes a private mapped
  Canvas or Blackboard v1 item through native validation, canonicalization, and
  existing public/private compilation with fixed imported defaults and a 256
  KiB whole-source cap. Canvas and Blackboard fixtures match equivalent
  hand-authored canonical source exactly; no Store, object, schema, HTTP, UI,
  or Wasm behavior changed. Full focused gates and independent P0/P1 review
  passed.

- Added the bounded Blackboard Original QTI 2.1 static-pool parser. It accepts
  only static single-choice items, keeps the exact inert `SCORE` declaration as
  compatibility provenance rather than scoring, and records the explicit 1.0
  point default as a review-required warning. Real shuffle, scoring variants,
  feedback, media, rich markup, and extensions become per-item safe refusals.
  Correct bindings and vendor-to-PLE maps remain private; full adapter gates,
  strict Clippy, boundary checks, and independent P0/P1 review passed.

- Added the bounded Canvas QTI 1.2 static-single-choice parser. It derives
  profile evidence from the exact bounded archive, manifest/resource graph,
  assessment metadata, and item tree; accepts only declared points and exact
  all-or-nothing scoring; and keeps unsupported feedback, scoring, markup, and
  extensions as per-item safe refusals. Correct bindings and vendor-to-PLE maps
  remain private and the package is neither serializable nor debuggable. Full
  adapter gates, strict Clippy, boundary checks, and independent P0/P1 review
  passed.

- Added the strict shared QTI markup projector. Canvas `mattext` receives one
  bounded HTML-tokenizer layer, while Blackboard ordered XML is projected
  directly and is never reparsed as HTML. Only the documented small text
  allowlist reaches deterministic CommonMark; attributes, recovery-dependent
  HTML, links, rich media, tables, styles, SVG, MathML, and unknown markup
  refuse visibly. Input, token, nesting, and Unicode-output limits are enforced
  before growth. Full adapter gates and independent re-review passed.

- Added the shared QTI mapped-item contract required before either vendor
  parser. Closed instructor-safe diagnostics cannot carry parser-supplied
  answer text; correct choices and ordered vendor-to-PLE maps remain
  non-serializable and non-debuggable. Choice identifiers are deterministic,
  collision-safe, and source-order independent. Mapping digests can now be
  created only by their profile/version-owning mapped item. Full adapter tests,
  strict Clippy, boundary checks, and independent re-review passed.

- Added the ordered XML evidence needed by exact Canvas and Blackboard
  parsers without changing the generic QTI importer. The private XML tree now
  preserves mixed text/child/comment/processing-instruction order, raw CDATA,
  element and attribute prefixes, and inherited namespace bindings. The
  existing aggregate-text behavior and public generic output remain locked by
  regression tests. Full adapter tests, strict Clippy, boundary checks, and an
  independent P0/P1 review passed.

- Completed the first bounded QTI-profile contract package. The adapter now
  has exact persisted Canvas QTI 1.2, Blackboard QTI 2.1, and honest generic
  profile identities; a single corpus-grounded manifest/resource/item matrix;
  strict vendor detection with generic fallback; and canonical safe-report,
  public, private, combined, and warning digests. Private answer mappings
  remain non-serializable and non-debuggable. Focused tests, strict Clippy,
  crate-boundary checks, and independent re-review passed; vendor item parsers
  remain the next package.

- Added the parser-ready QTI profile fixture corpus. Readable minimized Canvas
  QTI 1.2 and Blackboard QTI 2.1 manifests, metadata, items, and one-fact near
  misses now reproduce the retained package structures without test-time
  `OTHER_REPOS` access. Shared test support builds safe logical ZIPs, validates
  balanced single-root XML, and compares member paths and contents rather than
  timestamps or ZIP bytes. Focused/full adapter tests, strict Clippy, boundary
  checks, and independent review passed.

- Extracted the QTI hostile-input foundation before adding vendor parsers. A
  focused archive owner now enforces ZIP, path, link, duplicate, and expansion
  bounds; a separate XML owner enforces UTF-8, DTD/entity refusal, resource
  limits, attributes, nesting, and a single root. The generic parser keeps its
  original narrow entry grammar and fell from 976 to 613 lines. Adapter tests,
  strict Clippy, boundary checks, and independent review passed.

- Added the implementation-ready bounded QTI profile plan. It defines separate
  Canvas QTI 1.2 and Blackboard QTI 2.1 static-single-choice profiles, strict
  refusal instead of lossy grading or markup conversion, deterministic choice
  identity, native flat-question compilation, immutable private archive
  provenance, instructor upload/review/convert behavior, real-role PostgreSQL
  acceptance, and a separately gated later export milestone.

- Completed the instructor PLE flat-question editor. The focused authoring
  modules load and save canonical source only through the authenticated
  author-role, `no-store`, strong-ETag source route; ordinary browser
  contracts, learner preview, Wasm, and public publication DTOs remain
  answer-free. The route falls back to the legacy editor only on the protected
  source route's 404, while other failures remain visible. The editor supports
  create/open/edit/save, local-answer-free preview, visible stale-conflict
  reload, publication review, immutable publish, generation guards, a
  double-save lock, accessible per-choice radios, and 375 px reflow. Focused
  fixtures passed 2 new editor Playwright tests plus 7 generic editor tests;
  the production rebuild and all 11 `check_codebase.sh` stages passed. This
  records mounted component/client/repository evidence, not a deployed
  authentication or browser walkthrough.

- Completed the PLE flat-question persistence, publication, and runtime
  package. A typed compare-and-swap save keeps the draft and private canonical
  workspace source together; publication copies that source to an immutable,
  non-signable `ProblemSource`, persists only the answer-free public model, and
  promotes typed private material through the isolated grader capability. The
  dedicated no-store ETag routes and native runtime keep grading server-only.
  Memory, PostgreSQL/RLS, a real compiled blue-correct/red-incorrect live gate,
  and an independent re-review pass cover the boundary.
- Added the first executable PLE flat-question JSON v1 contract for ordinary
  static single-choice questions. The strict native Rust codec bounds source
  size and text, rejects duplicate and unknown members at every object layer,
  canonicalizes stable semantic choice IDs, and compiles author input into an
  answer-free draft plus checksum-bound grader-only answers and feedback.
- Completed the maintained local whole-system runner. Its three gates call the
  browser-safe Wasm bridge, apply and audit the full six-migration PostgreSQL
  baseline with every live oracle, and exercise a learner through two API
  replicas behind the gateway. The runner stops the issuing replica, requires
  the exact question envelope from the survivor, commits and exactly
  replays one idempotent submission, verifies the scoped persistence rows, and
  removes every generated project resource.
- Added an allowlisted container build context and a derived gateway image.
  API builds receive only Cargo sources, owning Containerfiles, and the SQLx
  migrations embedded by `migrate!`; host targets, generated output, local
  credentials, and unrelated source never enter the context. The gateway starts
  from an explicitly pinned official Caddy digest, removes its unnecessary
  low-port file capability, and runs on port 8080 as UID 1000 with an empty
  runtime capability set.
- Recorded a clean-cluster PostgreSQL 17 backup/restore rehearsal. Encrypted
  role and custom-format database artifacts recreated the six-migration ledger,
  data fingerprint, role attributes without password hashes, function owners,
  grants, forced RLS, and tenant isolation in a separate empty cluster. The
  restored application role persisted a tenant-owned write and invoked a
  restored queue-broker function. Backup and restore each completed in one
  second in this local fixture; managed PITR, production key management, and a
  deployed recovery objective remain separate M6 gates.
- Recorded a one-time production-boundary retention rehearsal against isolated
  PostgreSQL and MinIO services. The real six-family worker deleted a populated
  learner record graph and its typed student-record object while preserving the
  assignment and instructor structure, published catalog/version/source,
  workspace draft, and anonymous global statistics. The reconstruction harness
  was removed after the evidence was recorded, as required by the retention
  verification policy.
- Added a disposable PostgreSQL scale oracle for all four monthly activity
  parents and the production gradebook read shape. It distributes 260,000
  attempts across the fixed 26-month epoch, inspects `EXPLAIN (ANALYZE,
BUFFERS, FORMAT JSON)`, requires each bounded query to scan exactly one
  requested child, and proves an application-role 51-row gradebook lookahead
  page touches only compact current-summary relations with indexed enrollment
  and summary access.
- Activated the production worker as an explicit `--worker` binary mode and
  dedicated Compose service. Its closed registry contains six complete
  handler/committer pairs for scoring, course item analysis, attempt
  auto-submit, retention, assignment export, and QTI import; schema
  compatibility is verified before the first claim, and the process receives
  only PostgreSQL and object-storage configuration.
- Added mandatory family filters to the durable queue broker. Claimable work,
  expired-lease cleanup, and queue depth all use the same nonempty closed
  filter derived from the worker registry. Reserved Render and generic Import
  variants therefore stay untouched until complete implementations exist.
- Added complete per-item QTI import results. Safe packages may now retain
  accepted items while reporting unsupported or missing items as rejected;
  every result carries bounded source identity, disposition, warnings, and an
  answer-free normalized checksum when accepted. Exact and likely duplicates
  within one import batch are reported explicitly, and original archive bytes
  remain available for correction or re-import.
- Added forced-RLS PostgreSQL persistence for QTI item results plus a permanent
  disposable-database oracle. The live path proves preparation invisibility,
  exact commit, provenance, accepted/rejected row shape, warning detail,
  foreign-tenant non-enumeration, and absence of private grading bytes.
- Added a separate, current-only course item-analysis projection instead of
  reusing the identity-free cross-course catalog aggregate. The pure domain
  reducer, Store contract, Memory and PostgreSQL backends, instructor-only API,
  and closed worker handler report graded, unanswered, and pending-manual
  counts; correctness-based difficulty; exact-credit mean and sample standard
  deviation; discrimination; bounded response categories; assignment score;
  and terminal learner-submission time without learner identity, raw response,
  answer key, or grading implementation.
- Added generation-fenced item-analysis staging and publication. A successful
  scoring commit transactionally reserves the corresponding analysis job,
  non-analysis work remains ahead of analysis in both queue backends, and stale
  prepared generations complete as superseded without delaying or rolling back
  the current grade. The complete production registry now drains this family
  while reserved incomplete families remain outside its broker filter.

### Fixes and Maintenance

- Refused direct signed URLs for both workspace and published source objects.
  Source bytes may contain answers or executable grading material even when
  they live in the content bucket; only published assets, render artifacts, and
  authorized student records remain signable through the typed object contract.
- Repaired the live PostgreSQL application boundary found by the replica gate.
  `ple_app` now has only the `SELECT, INSERT` rights its catalog-asset binding
  contract requires. Immutable submission-successor receipts no longer use
  `SELECT FOR UPDATE`, which would have required a misleading table-wide
  `UPDATE` grant; their primary key serializes concurrent insertions and a
  losing finalizer accepts only the exact stored successor.
- Hardened the replica runner around real Podman variation and failure cleanup.
  It falls back from a stale `podman compose` shim to standalone
  `podman-compose`, identifies API containers through exact project/service
  names, emits only bounded allowlisted HTTP errors and redacted deployment
  diagnostics, validates UUIDs before SQL interpolation, and removes exact
  residual containers, networks, and volumes after partial Compose failures.
- Paired every claimable worker handler with its own atomic committer in one
  registry entry, replacing the unsafe combination of a full handler table and
  one unrelated global committer. Production processes one job per bounded
  pass, observes shutdown between claims without dropping active preparation,
  redacts operational error detail, and reports supported-family depth.
- Declared Compose's shared `default` network explicitly. The maintained
  `podman-compose config` check previously rejected the API's existing named
  default attachment as a missing network; PostgreSQL, MinIO, API, and worker
  now share a portable explicit application network.
- Split the QTI adapter into a 293-line data model, 976-line bounded parser,
  and 340-line hostile-archive/import-report test owner. The public adapter
  facade remains stable while archive limits, XML limits, asset validation,
  partial results, and duplicate detection have distinct review surfaces.
- Granted the QTI staging broker only the tenant-context predicate required by
  its security-definer functions. The live import path found the missing
  capability before any production data exists.
- Aligned durable export and import queue payload checks with Rust's established
  snake-case enum-field serialization. The PostgreSQL constraint and atomic
  export/QTI commit predicates now accept the exact closed payload Rust writes;
  a permanent serialization test locks the QTI shape.
- Split the QTI runtime backend into a 394-line production owner, 332-line
  shared private-fixture owner, 263-line direct grading/asset-integrity test
  owner, and 431-line learner run-lifecycle test owner. Issue, reproduction,
  answer-bearing grading, tenant/provenance refusal, asset binding, and replay
  behavior are unchanged; contributors can review each boundary independently.
- Split the QTI publication route's six behavior tests into a focused private
  test owner. The 585-line production module still owns committed-staging
  validation, strong workspace revisions, exact source-byte copying, review
  authorization, and visible-version promotion; the 532-line test module
  preserves every route and race assertion without changing a public path.
- Split authentication sessions and QTI persistence into capability-owned
  contract, in-memory, PostgreSQL, and conformance modules. The stable
  `learning_data_access::Session...` and `learning_data_access::Qti...` paths are unchanged; session SQL
  still assumes only `ple_auth`, opaque credentials remain one-way hashes,
  backend clocks still own expiry, and answer-bearing QTI bytes remain behind
  the separately injected grader handle. Store `lib.rs` drops to 2,319 lines,
  the PostgreSQL parent to 6,630, and the conformance facade to 4,990.
- Added `cargo tools` as the plain-language front door for repository-only
  generation, fixtures, database operations, and E2E seeding while retaining
  `cargo tools` as a compatibility alias. Scripts, generated-file guidance,
  command help, and current plans now lead with the clearer command.
- Split protected asset delivery into a 173-line backend-neutral contract,
  148-line in-memory implementation, and 235-line PostgreSQL implementation,
  paired with its existing focused conformance owner. Existing
  `learning_data_access::Asset...` paths, SQL, tenant checks, immutable catalog binding,
  educational-record non-enumeration, and access-audit behavior are unchanged;
  the crate root drops to 2,520 lines and the PostgreSQL parent to 6,744.
- Moved the complete backend-neutral external-tool Store contract out of the
  crate root into a focused 441-line module. The root remains a compatibility
  facade, so existing `learning_data_access::ExternalTool...` paths and both backend behaviors
  are unchanged while `store/lib.rs` drops from 3,075 to 2,677 lines.
- Kept item-analysis authorization inside the persistence boundary. PostgreSQL
  uses the active stored session and course authority in one database snapshot;
  forced RLS, course-binding and retention-fence triggers, purge order, and
  least-privilege grants protect both current and private staging rows.
- Defined completion time as the interval from run start to the latest terminal
  learner submission in both Store backends. Later manual-grading time cannot
  inflate the learner's completion interval. Difficulty follows the evaluator's
  correctness verdict independently from exact manual credit.
- Removed an unnecessary staging row lock found by the live least-privilege
  oracle. The assignment row lock and exact leased-job validation already
  serialize publication, so the application role does not need an `UPDATE`
  grant on private staging rows.

### Documentation

- Shortened the six pre-data SQLx migration identifiers to the readable
  `YYYYMMDDNN_description.sql` form. The contiguous numeric prefix preserves
  SQLx ordering while avoiding timestamp-like digits that do not carry useful
  information; underscores cannot divide the date because SQLx treats the
  first underscore as the version/description boundary.
- Defined PLE flat-question JSON as the small canonical machine format for
  static authoring and kept QTI as a profile-specific import/export adapter.
  The decision records the JSON-over-YAML reasoning, public/private persistence
  split, immutable source authority, feedback behavior, schema evolution, and
  the still-pending publication and instructor-editor packages.
- Renamed the opaque Rust crate paths atomically: `crates/store` is now
  `crates/learning-data-access`, its backend module is `in_memory`, and
  `crates/xtask` is now `crates/project-tools`. Cargo packages and directories
  use hyphens, Rust imports and modules use underscores, and repository
  automation uses `cargo tools` without the retired `cargo xtask` alias. The
  TypeScript generator recognizes the exact former ownership marker only long
  enough to rewrite generated files under the new package identity.
- Refreshed the architecture guide and added a file-structure guide that map
  each capability to its contract, in-memory implementation, PostgreSQL
  implementation, and focused conformance coverage. The README now introduces
  these plain names before relying on the internal Rust identifiers.
- Recorded the owner's componentization rule: preserve a modular monolith,
  split by capability ownership rather than arbitrary line ranges, keep crate
  roots as facades/composition points, and pair each component's narrow
  contract with its backend implementations and focused behavior tests.
- Replaced the root README's obsolete M0 stub description, frozen three-test
  output, and missing-server/schema/container claims with the current active
  implementation status, adoption blockers, verified repository front doors,
  assignment-to-analysis flow, implemented boundaries, and curated docs map.
  The original project purpose, server-only grading proof, tenant/content
  boundary, licenses, and author context remain intact.

### Developer Tests and Notes

- The embedded migration-status tests and full disposable PostgreSQL baseline
  gate pass with versions `2026080801` through `2026080806`. The gate proves
  six pending migrations, ordered apply, no-op replay, exact compatibility,
  checksum mutation detection, live worker and grading oracles, and real-role
  RLS denial; cleanup leaves only the pre-existing `pg-test` container.
- All five focused flat-question codec tests and both Object Store suites pass
  under strict Clippy. They prove canonical hashing, public/private secrecy and
  binding, correct and incorrect feedback selection, rejection of duplicate or
  extension members, the 256-KiB backstop, source non-signability, and normal
  published-asset delivery.
- `PLE_E2E_GATEWAY_IMAGE_SHA256=<pinned-digest> bash
tests/e2e/e2e_run_all.sh` passes 3/3. The exact cross-replica envelope,
  durable receipt replay, and one-row persistence set all pass.
- The complete disposable database gate passes with exact May-only pruning for
  `question_attempt`, `submission`, `record_access_log`, and `audit_event`.
  The 60,000-row gradebook fixture returns its bounded 51-row page from only
  `assignment`, `enrollment`, and `student_assignment_summary`; every
  disposable project is removed and the pre-existing `pg-test` remains alone.
- Added Memory and disposable-PostgreSQL family-filter oracles. Two concurrent
  PostgreSQL claimants lease distinct supported jobs through `SKIP LOCKED`, an
  older reserved job stays ready, and claiming another family does not
  dead-letter its expired lease. The complete six-migration, checksum, role,
  RLS, QTI, item-analysis, and manual-grading database gate passes with the new
  broker signature.
- All 8 QTI adapter tests pass, including hostile paths, symlinks, expansion
  limits, XML depth/width limits, server-only archive and grading handoffs,
  partial success, exact duplicate warnings, and likely duplicate warnings.
  QTI Store conformance passes in both configurations; the focused server
  import test and the new PostgreSQL live oracle pass. The fresh six-migration
  database gate also passes checksum mutation, real-role RLS, item analysis,
  and mixed manual grading after the queue/grant corrections.
- All five focused QTI runtime tests and the complete 143-test server suite
  pass after the split. The tests still prove no answer material reaches the
  browser, invalid tenant/provenance/asset bindings refuse before private
  grading, and an idempotent submission replay performs no second grader
  lookup.
- All six focused QTI publication tests pass after the ownership move,
  including foreign-tenant denial before object lookup, changed-draft and
  review-time revision races, bytes-first source copying, strict revision
  parsing, and promotion only from committed staging.
- Both `cargo tools database --help` and its `cargo tools` compatibility form
  produce the same project-tool help; all 18 project-tool tests pass. The full
  repository gate passes all 11 stages, including generated projection drift,
  TypeScript checks, 149 Node tests, crate-boundary enforcement, strict Rust
  Clippy, workspace tests, and doctests. Markdown and ASCII checks pass 444
  cases, and the maintained shell scripts pass syntax and warning/error-level
  ShellCheck validation.
- Protected asset conformance passes in both learning-data-access feature
  configurations; all six server asset tests pass. Both complete Store suites,
  strict Clippy in both modes, workspace compilation, and the full 11-stage
  repository gate remain green after the extraction.
- Session replica/revocation and backend-clock expiry conformance passes in
  both learning-data-access feature configurations. QTI staging, publication,
  redaction, and grader-isolation conformance also passes in both modes; all
  15 focused server QTI tests and all 24 focused server authentication tests
  pass after the split.
- External-tool Store conformance passes with and without PostgreSQL features,
  the focused server route tests pass, and both complete Store suites, strict
  Store Clippy modes, workspace compilation, Rust formatting, Markdown/ASCII
  hygiene, and staged/unstaged diff checks remain green after the extraction.
- Added focused reducer, Store conformance, server API/worker, and authorized
  HTTP fixtures. They cover mixed automatic and pending-manual work, correction,
  stale-stage supersession, latest-run selection, force-submit and clear,
  instructor/administrator access, cross-tenant non-enumeration, response
  redaction, and exact completion-time parity.
- Extended the disposable PostgreSQL 17 baseline gate with the production
  item-analysis path. It proves six-file apply/no-op/verify, real-role RLS,
  lower-priority job claiming, mixed manual correction from `0.25` to `0.5`,
  stale analysis rejection, one corrected current report, exact terminal
  submission timing, and cleanup. All disposable projects are removed after
  the run; the developer's existing `pg-test` remains untouched.

## 2026-08-08

### Additions and New Features

- Consolidated the pre-data PostgreSQL schema into the six domain-owned SQLx
  baseline migrations specified by the database evolution plan. The new
  `cargo tools database status`, `migrate`, and `verify` commands use SQLx's
  ledger and checksum compatibility checks, while application startup remains
  verify-only. The baseline creates explicit least-privilege principals,
  forced tenant RLS, monthly/default partition families, immutable published
  content, normalized assignment and activity records, operational queues,
  analytics, and retention boundaries without retaining the historical repair
  chain.
- Added human-readable catalog IDs, stable fixed assignment items, pinned
  random-selection candidates, immutable delivered run order, exact decimal
  point values, explicit attempt states, and generation-fenced current scoring.
  Recalculation stages every affected attempt, run, enrollment, and summary,
  atomically publishes only the newest complete generation, and restages when
  a submission arrives during calculation. Normal, zero-point, full-credit,
  extra-credit, exclusion, negative-credit, and selected-group behavior now
  share the same scoring derivation.
- Added the tenant-scoped, revision-checked Delete and Regrade Store command.
  It blocks an affected in-progress attempt, retires and excludes the stable
  item, omits it from future runs and ordinary student outcomes, retains raw
  submitted evidence for instructors and retention, and enqueues current-score
  recalculation without assignment or grade-history tables. The committed
  cross-layer fixture and TypeScript decoders now carry normalized assignment
  items, human catalog IDs, and explicit attempt status.
- Added direct-instructor force-submit and clear operations for question
  attempts. Force-submit closes active work as needing manual grading without
  inventing a response or score; clear removes an evaluation from current
  scoring while retaining protected instructor evidence. Stable action IDs,
  minimal partitioned audit records, transaction-scoped retry serialization,
  and generation-fenced recalculation make concurrent exact retries harmless.
- Added mutable assignment visibility, availability, due/close boundaries,
  late-submission policy, time limits, and attempt limits. Active timing edits
  update the server-owned effective deadline atomically: extensions reschedule
  the same durable job, shortening past elapsed time immediately auto-submits,
  and a leased stale generation must re-resolve before it can commit. Natural
  expiry records no learner response or evaluation, browser connectivity is
  irrelevant, ordinary/support completion cancels pending timing work, and
  retention removes the closed auto-submit jobs with their attempts.
- Added revisioned direct-student and course-group assignment-policy exceptions.
  Each timing dimension resolves independently to the most permissive applicable
  value; issued attempts record the effective policy and contributing targets.
  Exception, group, and course-membership changes re-resolve active attempts in
  the same transaction, including immediate auto-submit when a removed extension
  exposes an elapsed deadline. Retention fences and purges learner group
  membership and direct-student exceptions without turning policy history into
  an append-only record.
- Added the MOD-RETENTION R1-R4.1 foundation: configurable 30/100/365-day
  course-end snapshots behind stored-session, broker-only authority; an
  authoritative scheduler that binds each due current-generation stage to one
  closed queue job; a due/lease/generation-fenced, key-free worker; and exact
  `StudentRecord` cleanup that prevents queued exports from resurrecting
  delivery. Administrators may extend only future unstarted stages, while an
  instructor or administrator may choose the archive-time assignment
  disposition before archive work begins. The durable in-app
  archive/delete/extend intent contains no learner data, and lifecycle remains
  Active until later R4 slices perform access gating and whole-record purge.
- Added the revisioned MOD-RETENTION R4.2 management API. Authorized course
  instructors can end retention tracking and request archive or deletion,
  while tenant administrators can also extend the schedule. Strong ETags,
  Store-owned compare-and-swap, durable replay receipts, strict request bodies,
  fixed notification copy, non-enumerating authorization, and `no-store`
  responses keep retries safe without exposing learner, job, object, or lease
  state. Course lifecycle deliberately remains Active until the later access
  and purge slices complete their real transitions.
- Corrected the MOD-RETENTION permanent-purge transaction to serialize only the
  affected course. Every learner-record producer now shares the exact course
  retention-row fence, while delete preparation freezes private indexed
  run/attempt/export work sets instead of taking shared-table `EXCLUSIVE` locks
  or materializing whole-course UUID arrays. Successful deletion erases those
  private work sets before recording the coarse tombstone; unrelated courses
  remain writable during a large purge.
- Added instructor-created frozen assignment export snapshots. The worker
  atomically leases, produces, and commits four prompt-only DOCX, PDF, and
  accessibility artifacts; requester-only protected downloads and RLS keep
  each export tenant-owned.
- Added opt-in production QTI runtime registration. It requires the exact
  `PLE_QTI_RUNTIME_ENABLED=1` and separate `PLE_GRADER_DATABASE_URL` pair,
  constructs a dedicated `ple_grading_reader` pool before routing, and dispatches
  only persisted QTI sources through the immutable-archive backend. Partial
  configuration fails closed; disabled, foreign, and non-QTI dispatches cannot
  reach private grading material.
- Added a private, bytes-first QTI staging pipeline. A closed queue payload
  binds tenant, workspace, import, and source-object IDs; the worker parses
  only that durable ZIP, stores deterministic-key extracted assets, and keeps
  answer bindings in the dedicated grader boundary. PostgreSQL exposes the
  registry and grading material only through an exact active-lease atomic
  promotion capability, leaving failed preparation as reconciliable object
  orphans rather than visible educational records. The dedicated authenticated
  QTI publication route validates committed staging before minting identities,
  copies source and selected-asset bytes to candidate published keys, and
  leaves generic catalog publication QTI-closed.
- Added the production-independent published QTI run backend. It reparses the
  exact checksum-pinned archive and revalidates every referenced asset before
  issuing, replaying, or grading through normal server run semantics; answer
  bindings are read only through the separately injected dedicated grader
  handle. Browser receipts remain key-free, and production composition/
  registration is the next QTI task.
- Added tenant-owned, server-private teaching feedback beside the first
  submission receipt. Immediate-full and immediate-correctness responses are
  projected through one disclosure policy, while deferred and on-release
  feedback remain absent until the server authorizes disclosure; browser
  contracts never contain private feedback, answer keys, or grading material.
- Added immutable receipt-time run and summary snapshots for PostgreSQL
  idempotency replay. A deferred first submission now replays byte-for-byte
  unchanged after a later question completes the run, matching the MemoryStore
  behavior instead of recalculating disclosure from newer state.
- Added bounded catalog search and immutable version detail routes with
  server-supplied taxonomy, capability, license, and statistics-availability
  facets. Memory and PostgreSQL share query-bound keyset behavior, tenant
  visibility, safe hot-metadata projections, and explicit unavailable
  statistics until the aggregate subsystem lands.
- Replaced the library placeholder with the live, cursor-paged catalog route
  and exact immutable detail/lineage page. The virtualized browser keeps a
  bounded DOM, preserves server facets across later-page recovery, discards
  stale searches, and never fetches the whole catalog to derive counts.
- Added durable private workspace ownership and optimistic revisions for
  authoring drafts. Owners may publish or delete; explicitly bound
  collaborators may read and revise; other instructors in the same tenant see
  no workspace. Strong ETag/If-Match checks prevent stale tabs from silently
  overwriting or deleting newer edits, and legacy drafts with no trustworthy
  owner remain inaccessible until an authorized migration resolves them.
  Server validation and semantic diff bind the saved draft revision; publish
  requires that exact reviewed strong `If-Match` value and rechecks it
  atomically. The editor saves before review, shows the exact proposed title,
  and requires a fresh review after a conflict.
- Added an explicit protected instructor author-preview for saved workspace
  drafts. It binds owner/collaborator access and the exact saved ETag before
  returning reviewed display-ready correct-response and rationale blocks for
  supported native families. Unsupported and external sources are explicitly
  unavailable; the ordinary browser/WASM preview remains key-free.
- Added the authoritative assignment editor and revisioned course-assignment
  API. Instructors select ordered immutable catalog versions and edit the four
  assignment-level run policies; question timing and attempt policies remain
  immutable version properties. The server re-resolves catalog visibility,
  lifecycle, and persisted capabilities before an atomic `If-Match` update,
  returns every safe capability violation, and preserves edits through stale,
  validation, and network recovery.
- Added durable next-question prefetch without starting question N+1 early.
  The Store reserves a key-free server-owned variation, submission atomically
  promotes it into the real attempt and timer, and the immutable first receipt
  carries only a minimal successor descriptor. Replica recovery heals only an
  exact pending predecessor. The browser keeps the envelope in memory, warms
  at most 12 same-origin logical assets, and uses it only after an exact receipt
  match; mismatch, outage, and route teardown fall back without losing feedback.
- Added immutable, content-free instructor feedback-release records for
  `onRelease` questions. Both stores derive direct course-instructor authority,
  retain the original POST receipt unchanged, and expose current release state
  only inside the tenant boundary; tenant-administrator release remains a
  later authenticated server-composition task rather than a caller assertion.
- Added the current-state run summary route and learner page. The server alone
  applies all four feedback policies and release records, while the browser
  consumes a recursively strict, cursor-paged projection with no question
  source, provenance, private feedback, or grading key. The completed-run view
  survives page and practice-start failures and can start a server-authorized
  31st practice run without altering an earlier submission receipt.
- Added an adapter-owned contracted iMathAS scored-embed provider boundary.
  It revalidates the immutable source, signs an exact per-launch nonce and
  attempt binding, retrieves bounded results through a protected transport,
  and refuses generic hosted MyOpenMath or unbound provider/source pairs.
- Added the learner external-tool response surface. It fetches a protected
  same-origin launch only after activation, accepts only an exact
  attempt-bound readiness message from the current frame, and submits the
  empty external-tool marker; browser messages cannot supply a score,
  provider identity, token, answer, or correctness decision.
- Added the protected iMathAS launch route. Provider session state stays
  encrypted and server-held behind a fixed HttpOnly cookie; every activity
  request rechecks tenant, owner, attempt, immutable source, and provenance.
  Only a successful nonce-protected activity document can emit the exact
  readiness handshake, while expired, revoked, copied, tampered, and outage
  responses remain opaque and cannot report a false ready state.
- Completed the MOD-RETENTION R4.3 truthful archive slice: learner archive
  boundaries now use one central, access-fenced predicate across all learner
  aliases and Store/RLS, while manager retention-definition reads remain.
  Archive completion occurs only after exact object cleanup, and replay is
  exact/idempotent across all archive completion paths. Public catalog assets
  stay deliverable, StudentRecord/export/external-tool resurrection is fully
  closed after pre-cleanup fencing, and production `RetentionJobHandler`/
  `RetentionJobCommitter`/`RetentionWorkerComponents` now assemble from
  real PostgreSQL + object-store dependencies without worker activation.
- Added the non-destructive MOD-RETENTION R4.4A cleanup foundation. Archive
  preparation now persists an exact normalized set of tenant-owned export and
  external-transcript object IDs before delivery revocation or object deletion,
  and renewed leases replay that immutable manifest. Student-record audit
  events carry indexed relational course ownership rather than requiring JSON
  inference. Permanent deletion remains explicitly fail-closed until R4.4B can
  remove the complete relational record graph and commit the truthful tombstone
  in one generation- and lease-fenced transaction.
- Completed MOD-RETENTION R4.4 permanent student-record deletion. The worker
  now archives access before persisting and deleting an exact typed-object
  manifest, then removes the complete course-owned learner graph in a single
  lease-, generation-, and stage-fenced PostgreSQL transaction. It preserves
  immutable published content, instructor drafts, catalog metadata, and
  anonymous question statistics; applies the frozen retain-or-delete choice to
  assignment definitions; and records `studentRecordsDeleted` only after every
  object and relational effect succeeds. A disposable PostgreSQL 17 rebuild
  exercised malformed cross-course links in both endpoint directions; that
  environment-backed reconstruction remains one-time evidence rather than a
  committed fixture or ignored permanent test.
- Added optional production registration for an explicitly contracted,
  self-hosted iMathAS provider. Startup validates all provider settings and
  constructs only private redirect-free clients; absent configuration leaves
  iMathAS routes unregistered, while provider outages remain question-local.
- Added the tenant-safe course gradebook summary boundary: a cursor-paged
  `GradebookSummaryRow` joins only assignments, enrollments, and maintained
  `StudentAssignmentSummary` records. The API, strict browser decoder, and
  mock return no attempt history, responses, feedback, grading material, or
  question content. Course instructors and tenant administrators may read it;
  students receive 403 and unrelated courses remain absent.
- Gradebook cursors now encode the native assignment/enrollment UUID tuple.
  PostgreSQL compares and orders that tuple directly, keeping the cursor path
  aligned with its assignment and enrollment page indexes instead of sorting
  text-concatenated identifiers.

### Fixes and Maintenance

- Hardened the consolidated pre-data PostgreSQL baseline after independent
  review. A disposable live gate now proves clean apply, no-op replay, checksum
  drift detection, real-role RLS denial, deterministic empty default
  partitions, and a genuine serialization failure that commits after a bounded
  whole-transaction retry. `problem` and `answer_key` now have forced RLS,
  baseline constraints apply validated, required foreign-key access paths are
  indexed, migration credentials are separate from the application role, and
  E2E seeding must explicitly opt in before changing schema.
- Made both SQLx pools explicit about acquisition, idle, and maximum-lifetime
  bounds; classified serialization and deadlock SQLSTATEs separately; removed
  raw PostgreSQL constraint text from portable errors; and kept mid-verification
  connection failures in degraded-health handling. PostgreSQL migration logic
  and Memory retention behavior/tests now live in focused child modules instead
  of the two largest Store backend files.
- Split PostgreSQL assignment-timing, assignment-export, catalog, external-tool,
  jobs, QTI, retention, connection, migration, and manual-grading behavior; Memory
  external-tool, queue, export, catalog, session, retention, and manual-grading
  behavior; Store activity/scoring and publication validation, feedback, and
  policy contracts; Store conformance domains; Memory catalog/statistics tests;
  and the server external-tool routes into focused child modules. Public Store
  and server paths remain compatible while the largest parent files are
  smaller.
- Centralized four-decimal, midpoint-away-from-zero score rounding at the Rust
  persistence boundary, including recalculation workers. Gradebook percentages
  and learner feedback now share a two-decimal browser formatter that trims
  trailing zeroes, canonicalizes negative zero, and displays `8 / 10` instead of
  a binary floating-point artifact.
- Closed the instructor manual-grading HTTP contract with a real
  response-bearing pending submission, direct-course-instructor
  non-enumeration, revision and idempotency checks, strict decimal input, and a
  minimal grade receipt. The complete private run route group now adds
  `Cache-Control: no-store` even when typed paths, JSON extraction, or the body
  limit reject a request before its handler runs.
- Completed the current-state manual-grading package with a disposable live
  PostgreSQL mixed-run gate. One automatic item plus one response-bearing
  manual item now proves exact `NUMERIC` credit, correction-safe minimal
  receipts, stale-generation supersession, final `0.75` summary publication,
  first-completion and grade-run pointers, and tenant non-enumeration. The gate
  also replaced a successor-link row lock that required an unintended `UPDATE`
  grant with immutable primary-key insertion and made first-completion
  publication part of the scoring-worker fence in both Store backends.
- Added the root `OTHER_REPOS/` reference checkout directory to
  `.prettierignore`, keeping the codebase check, write alias, and direct
  Prettier commands out of vendored or upstream code.

### Developer Tests and Notes

- The isolated PostgreSQL 17 gate passes with six exact migrations, zero
  unvalidated constraints, only the documented migration-ledger and global
  statistics tables outside RLS, fixed 2026-08 through 2028-09 activity
  partitions, and a concurrent SSI fixture that completes in three attempts
  after PostgreSQL aborts one of two transactions. Store validation passes with
  47 PostgreSQL-feature unit tests, 14 conformance tests, and two opt-in live
  tests for bounded serialization retry and mixed automatic/manual scoring.
- Mounted gradebook and learner-feedback acceptance tests pass with the shared
  artifact-free score formatter, including responsive layout, reading order,
  announcement, focus, and keyboard behavior.
- The manual-grading HTTP behavior test covers student submission and exact
  replay, pending evidence, student and outsider read/write denial, strict body
  rejection, instructor grade and correction conflicts, receipt secrecy, and
  `no-store` on malformed and oversized requests. The server suite passes 138
  unit tests plus its doctest.
- Replayed the six-file baseline on a fresh PostgreSQL 17 database, verified a
  second migration run was a no-op, confirmed all six ledger checksums, and ran
  live scoring, timing, direct/group exception, membership-removal, and cleanup
  behavior through the production PostgreSQL Store. Direct inspection found one
  exception-driven auto-submit with a timestamp and no submission, evaluation,
  or score. The full 11-stage codebase gate passed with 149 Node tests, 137
  server tests, 47 PostgreSQL-feature Store unit tests, and 14 Store conformance
  tests.
- A six-pass pre-commit audit found and corrected accommodation recomputation on
  course roster changes, retention coverage for the new learner-linked rows,
  cross-course group identity drift in MemoryStore, missing live PostgreSQL
  exception coverage, and undocumented lock ordering. Production queue draining
  remains a later MOD-WORKER composition task: activating the current unfiltered
  generic claim with an incomplete handler registry could consume another job
  family's work.

- Verified a real `OTHER_REPOS/` TypeScript file is ignored with both the
  checker's explicit `.prettierignore` path and Prettier's default discovery.
  Shell syntax, `git diff --check`, and all 1,181 pytest cases pass. The full
  codebase gate reaches Prettier after passing generation, type checking, and
  ESLint, then stops on the pre-existing formatting warning in
  `crates/question_model/bindings/Capability.ts`.
- Applied the repository fixture policy to the Rust integration suites. The
  explicitly approved published-problem corpus remains shared infrastructure;
  small QTI archive and WeBWorK PG inputs now live inline beside their behavior
  tests, and temporary PostgreSQL reconstruction inputs remain one-time evidence
  that is removed after use.
- Replaced the handwritten PostgreSQL migration registry and custom ledger loop
  with SQLx's directory-backed, checksummed migrator. This also prevents new
  migration files from being silently omitted from Rust registration; the Store
  build script invalidates Cargo output when that directory changes. Removed
  migration/source-string tests that pinned implementation spelling and the
  ignored credentialed database mega-test that mixed many mutable scenarios.
  Store behavior and PostgreSQL compilation remain permanent gates; fresh
  database replay, roles/RLS, and migration-race checks remain disposable
  one-time evidence for the upcoming six-file baseline.
- The partial-commit audit removed private MemoryStore wiring tests and
  compile-only composition checks, corrected retention-policy links and backup
  wording, and reopened R4.4B acceptance: its global table locks and whole-course
  UUID arrays must be replaced before the permanent purge is commit-ready.
- Added all-four-policy native submission and browser feedback matrices,
  recursive strict receipt decoding, raw-JSON secrecy checks, foreign-tenant
  refusal, and a two-question deferred replay test that remains identical
  after run completion. Live PostgreSQL execution remains an opt-in gate.
- Added a 51-attempt Store paging gate and a mounted 31-outcome run-summary
  gate. They prove tenant/run-bound cursor integrity, exact continuation with
  no duplicates, retained rows across a failed later page, and recovery from a
  failed fresh-practice request without client-side disclosure inference.
- Added a 10,000-problem catalog behavior gate covering bounded pages, stable
  opaque cursors, tamper and query-mismatch refusal, aggregate facets, no
  duplicates, and tenant isolation. Static PostgreSQL checks prohibit
  `OFFSET`; a representative live `EXPLAIN` remains an environment-dependent
  release-readiness check.
- Added mounted external-tool behavior tests for lazy launch, copied and
  foreign-frame messages, unknown fields, outage/retry isolation, and exact
  marker-only submission through its launch-scoped child route. The mock
  boundary enforces the same strict marker and idempotency grammar without
  fabricating a provider grade.
- Added memory and PostgreSQL-feature scale gates for the summary-only query,
  including tenant/course isolation, stable cursors, empty summaries, and a
  static SQL guard against history scans or aggregates. The representative
  PostgreSQL EXPLAIN/index inspection remains a one-time check pending a
  configured `PLE_POSTGRES_TEST_URL`.
- Added assignment-editor conformance and mounted browser gates covering
  revisioned create/read/update, wrong-course and foreign non-enumeration,
  published/deprecated versus archived lifecycle handling, strict nested JSON,
  ordered immutable references, all capability violations, stale-write
  recovery, keyboard operation, and the 420-pixel layout.
- Added Store, HTTP, WebWork, mock, and mounted-browser prefetch gates. They
  cover concurrent idempotent reservation, exact predecessor promotion,
  immutable replay after later progress, process-crash healing, foreign and RLS
  isolation, backend-owned rendered hashes, 204 exhaustion, strict body and JSON
  boundaries, cache-hit zero-fanout advance, online retry, bounded asset warming,
  storage secrecy, and late-abort isolation across the 31st run.

## 2026-08-07

### Additions and New Features

- Added one shared, non-mutating learner-title policy for published question
  metadata and issued envelopes: titles require non-whitespace content and at
  most 512 Unicode scalar values. Draft persistence, publication, adapters,
  render caches, and the strict browser decoder now enforce the same boundary.
- Completed MOD-CLIENT with a strict same-origin HTTP implementation of the
  browser `ApiClient`. Successful response bodies enter as `unknown` and pass
  exhaustive field-by-field decoders; malformed UUIDs, timestamps, numeric
  ranges, nested records, and discriminants fail before reaching application
  state without `any` or unchecked assertions.
- Added authenticated, bounded HTTP fallbacks for key-free response-format,
  timer, and assignment-capability evaluation. They delegate to the same pure
  Rust domain functions as WebAssembly while trusted publication, database-time
  enforcement, and correctness decisions remain on their authoritative server
  paths.
- Added exact course lookup and guarded run-screen composition across the run,
  active attempt, enrollment, assignment, course, and immutable question
  resources. The client rejects inconsistent identifiers, tenants, or repeated
  attempt cursors instead of combining independently valid but unrelated data.
- Restricted newly issued API attempt seeds to JavaScript's exact 53-bit
  integer range while retaining operating-system randomness and the internal
  generator's full `u64` contract.
- Completed MOD-API-ASSET with an immutable database registry that maps one
  route identifier to an exact typed `ObjectRecord`. Catalog assets reuse
  their logical `AssetId`; tenant-owned student-record artifacts reuse their
  `ObjectId`, preserving both model identities behind one stable route.
- Added public-CDN and protected asset delivery in
  `crates/server/src/asset.rs`. Public catalog requests bypass authentication
  and object-store signing, while protected content resolves the HttpOnly
  session, authorizes in the session-derived tenant, appends a URL-free audit
  event, and redirects to the exact stored key.
- Added the forced-RLS `asset_delivery` migration plus shared MemoryStore and
  PostgreSQL `AssetStore` implementations. Registration accepts only matching
  published `ProblemAsset` records or tenant-matching `StudentRecord` exports;
  source packages, render caches, and temporary-processing objects are not
  deliverable through the route.
- Completed MOD-API-RUN with authenticated start-or-resume, run history,
  question-attempt history, idempotent submission, enrollment, and grading
  summary routes. Students mutate only their own enrollment; course
  instructors and tenant administrators receive scoped read access.
- Added one-question-at-a-time issue and resume behavior. Each new attempt gets
  a fresh operating-system-random seed, the unresolved attempt retains its
  seed, repeated problem references remain distinct by assignment position,
  and the store lock prevents concurrent requests from starting two timers.
- Added atomic MemoryStore and PostgreSQL submission paths. Database-owned
  timestamps drive timer verdicts and completion; response, grade event, run,
  enrollment, and compact summary changes commit together, while exact retries
  return the immutable first result without grading or counting twice.
- Added the cursor-paged `listRuns` and `listAttempts` client contracts and
  aligned the run mock with the real `{ assignmentId }` start body and
  enrollment-owned route group.
- Completed MOD-GRD with the server-only
  `grading::grade(question, response, key)` contract. The generic checker
  supports all-or-nothing numeric, multiple-choice, short-text, and ordering
  questions after repeating shared response-format validation.
- Added explicit grader outcomes and errors for ungraded practice, missing or
  mismatched keys, invalid public parameters, backend-owned partial credit,
  and file uploads requiring manual review. The public result contains only
  correctness and points, never answer-bearing material.
- Completed MOD-API-COURSE with authenticated cursor-paged course and
  assignment reads plus server-owned course and assignment creation. Course
  access uses explicit course-local membership; tenant administrators use a
  separate derived authority path rather than a persisted membership value.
- Added Rust-owned `CourseId`, `CourseMembershipRole`, `CourseRole`,
  `CourseSummary`, and `AssignmentSummary` contracts. The generated TypeScript
  client now uses `CourseId` instead of an untyped string, and fixture schema 3
  records the signed-in student's course role.
- Added normalized `course` and `course_member` PostgreSQL tables, forced RLS,
  membership and assignment paging indexes, and hot `course_id` and title
  columns on assignments. Memory and PostgreSQL stores now share course-local
  visibility, assignment scoping, and validation behavior.
- Completed MOD-API-CAT with authenticated catalog browse, exact version,
  taxonomy, publication, deprecation, and archival routes. Every route derives
  its tenant from the shared session store and every list uses a bounded
  stable-key cursor.
- Expanded the initial identity contract to the plan's five one-way lifecycle
  states: draft, validated, published, deprecated, and archived. Added
  institution and public scopes, trusted backend summaries, nonempty author
  ownership, linear revision lineage, attributed forks, and database-owned
  publication timestamps.
- Added a dedicated `CatalogStore` contract with memory and PostgreSQL
  implementations. Publication locks and compares the exact validated draft,
  installs any institution visibility grant, writes hot metadata and immutable
  payload, and removes the draft in one transaction.
- Added the server-owned `BackendRegistry` and configurable
  `PublicReviewGate`. The publish route returns every capability violation,
  permits institution publishing for instructors, and restricts public
  publishing to publishers or administrators plus any configured review.
- Added Rust-owned `CatalogProblemSummary` browser types and fixture schema 2.
  Catalog and taxonomy mocks now match the cursor-paged route shapes while
  browse results omit prompts, responses, and private backend locators.
- Implemented the provider-neutral MOD-API-AUTH route group for login, current
  session, and logout. A typed `IdentityProvider` boundary permits later OIDC,
  institutional SSO, LTI, or explicit local-development integration without
  coupling credential verification to session mechanics.
- Added distinct Rust-owned `UserId` and multi-role `UserRole` contracts, with
  lower-camel serialized role names and regenerated browser types. Authenticated
  users are no longer incorrectly represented by assignment `StudentId`.
- Added a separate replica-safe `SessionStore` contract with memory and
  PostgreSQL implementations. Sessions use a 256-bit OS-random opaque cookie,
  persist only its SHA-256 hash, use backend-authoritative expiration and
  revocation, and derive tenant context only after resolving the session row.
- Added the forced-RLS `auth_session` migration and narrowly scoped `ple_auth`
  role. Route tests prove login on one replica, session use on another, and
  immediate cross-replica revocation; the PostgreSQL conformance gate reuses
  the same behavior when a dedicated test database is configured.
- Recorded permanent session-storage compliance guidance in
  `docs/HUMAN_GUIDANCE.md` and kept authentication state in an HttpOnly cookie
  rather than `localStorage`. HTTPS defaults to `Secure; SameSite=Lax`, LTI
  embedding has an explicit `SameSite=None; Secure` mode, and plain HTTP
  requires the explicit local development policy. The cookie now omits
  `Max-Age` and `Expires` so ordinary authentication ends with the browser
  session; persistent `remember me` behavior requires a separate consent and
  legal review.
- Completed MOD-SCHEMA and MOD-STO with an embedded, checksummed PostgreSQL
  migration runner and a full `PostgresStore` implementation of the frozen
  backend-neutral persistence contract.
- Added shared catalog tables, forced-RLS tenant tables, a 16-way hash split
  for problem-version payloads, and monthly partitions for question attempts,
  submissions, grade events, and audit events.
- Added an opt-in PostgreSQL execution of the reusable store conformance suite.
  It also proves an unfiltered foreign-tenant query returns zero rows and the
  student role receives PostgreSQL permission error `42501` on `answer_key`.
- Recorded adaptability as an owner design priority in
  `docs/HUMAN_GUIDANCE.md`: the system must evolve with changing requirements
  and insights while remaining functional and relevant.
- Completed MOD-OBJ with one replica-safe `S3ObjectStore` implementation shared
  by AWS S3 and MinIO, explicit three-bucket name mapping, and no SDK types in
  the backend-neutral trait.
- Added an opt-in MinIO execution of the reusable object-store conformance
  suite. The normal offline gate continues to run the same behavior against
  `MemoryObjectStore` without requiring services or credentials.
- Completed MOD-CAP with `domain::policy::validate_assignment_config`, returning
  every missing immutable-question/backend capability pair in deterministic
  question and capability order.
- Added `crates/domain/tests/capability_violation_cases.json`, a reviewed
  behavior table covering all eight capabilities, plus a key-free WebAssembly
  export, typed browser facade, API fallback contract, and server-free mock.
- Completed MOD-SCORE with `domain::scoring::score`, a pure completed-run grade
  selector, and moved the existing incremental summary projection into its
  canonical scoring owner. The frozen `domain::run` compatibility path remains
  available.
- Added typed completed-run and grade-selection values plus explicit errors for
  invalid scores, zero or duplicate run numbers, duplicate run IDs, and invalid
  instructor selections.
- Completed MOD-TIME with the clock-free `domain::timing::timer_verdict`
  contract. It evaluates untimed, open, grace-period, submitted-on-time,
  submitted-within-grace, and timed-out states from server-owned inputs.
- Added typed timer evaluation to the reviewed WebAssembly boundary, browser
  facade, API fallback contract, and server-free mock. The browser wire uses
  lower camel case while Rust items retain idiomatic snake case and upper camel
  case.
- Completed MOD-STATE with `domain::attempt::apply`, one pure state machine for
  not-started, active, submitted, correct, incorrect, retry-available,
  exhausted, timed-out, and abandoned question states.
- Implemented `crates/domain/src/completion.rs` as the owner of within-run
  completion derivation. Empty runs remain in progress; answer-all,
  all-correct, and inclusive score-threshold behavior are explicit and invalid
  fractions remain errors.
- Completed WP-C8 with `docs/CONTRACTS.md`, covering all 36 modules in the
  active-plan catalog. Every row names its contract source, one owning role,
  every direct consumer, and the allowed dependency stub.
- Classified contract sources as frozen, stubbed, or reserved so the register
  distinguishes callable interfaces from compiling boundaries and future
  source ownership without presenting later milestone behavior as complete.
- Completed WP-C9, freezing the frontend architecture in
  `docs/FRONTEND_ARCHITECTURE.md` and the Solid reactivity contract in
  `docs/SOLID_MODEL.md`. The executable contract defines all 11 product routes
  plus a separate not-found route.
- Added a browser-safe `ApiClient`, router-owned cached query runtime, and
  server-free typed mock client. The course, assignment, and active-run screens
  all use that injected boundary rather than importing fixtures directly.
- Added the single browser WebAssembly facade and a fully controlled
  multiple-choice reference widget. Number keys select and move focus, Enter
  submits, Escape returns to the assignment, and a live region announces only
  key-free response-format status.
- Added a persistent app shell with route and question error boundaries,
  explicit pending and empty states, responsive 56-pixel response targets, and
  an honest contract surface for each later M3 route.
- Added `docs/PALETTE_CONTRAST_AUDIT.md`, generated from source by the
  color-accessibility tooling. All seven source palette colors pass the 5.5:1
  repository target against white, from 6.56:1 to 16.27:1.
- Completed WP-C7 with a Rust-owned fixture generator, a reviewed published
  problem corpus, and a dependency-free mock API. The corpus contains two
  checksummed SVG assets, one draft, one published problem version, one
  assignment reference, one enrollment, three completed runs, and one
  in-progress run.
- Added full attempt reproducibility records to all four fixture runs, including
  adapter, generator, source artifact, asset object, grading implementation,
  parameter hash, rendered-question hash, and a distinct fresh seed for every
  issued instance.
- Added mock handlers for all five planned API route groups: authentication,
  catalog, course and assignment, run and grading, and authorized assets. The
  fetch replacement has no network fallback, so missing mocks fail locally.
- Completed WP-C6, establishing the browser/server grading boundary.
  `grading::AnswerKey` now holds every answer-bearing value, while
  `domain::validation::validate_response_format` provides key-free structural
  checks callable through the WebAssembly bridge.
- Added an exact processed-WebAssembly export allowlist and a conservative
  workspace dependency-closure test. The shipped `wasm_bridge` closure is
  exactly `wasm_bridge`, `domain`, and `question_model`; it contains no
  `grading` crate.
- Added `docs/SECURITY_MODEL.md`, documenting code placement, the key-free
  browser surface, the answer-bearing server surface, and the evidence required
  before a new WebAssembly export is accepted.
- Completed WP-C5, implementing the deterministic seeded parameter generator
  and a shared native/browser parity harness. The committed corpus covers all
  current integer, decimal, choice, and fixed branches with 65 seeds for
  `parameter-map@1`, including the full `u64` seed endpoint.
- Added `GeneratorReference`, which keeps a generator's stable ID and additive
  version together in published randomization definitions, generated variants,
  and attempt provenance.
- Added `docs/DETERMINISM_CONTRACT.md`, a maintained seed-vector regeneration
  example, a version-matched local `wasm-bindgen-test` setup script, and a real
  headless-Chromium parity runner under `tests/playwright/`.
- Completed WP-C4, freezing the backend-neutral `Store` and `ObjectStore`
  contracts. `Store` covers drafts, immutable published versions, assignments,
  enrollments, runs, question attempts, and compact summaries; every list
  operation uses a bounded cursor request.
- Added reusable store and object-store conformance suites, initially run
  against `MemoryStore` and `MemoryObjectStore`. Later PostgreSQL, MinIO, and S3
  implementations must pass these same suites.
- Added typed object buckets and semantic keys derived only from problem,
  version, asset, tenant, seed, and object identities. Object writes compute
  SHA-256 metadata, reads verify the bytes, and signed URLs apply the documented
  60-minute content, 5-minute student-record, and never-served temporary rules.
- Added `docs/HUMAN_GUIDANCE.md` and linked it from `AGENTS.md`, preserving owner
  decisions about plan status, Codex versus Claude guidance, local Podman use,
  repeated practice, tenant/content boundaries, server-only grading, and
  measured Rust/WebAssembly performance choices.
- Completed WP-C3, the enrollment, run, question-attempt, policy, and compact
  summary contract (MOD-RUN). Every educational-record type carries a direct
  `TenantId`, enrollment completion derives from its first-completion record,
  and assignment runs deliberately carry no stored completion boolean.
- Added `crates/domain/src/run.rs` with pure within-run completion,
  continued-practice eligibility, and transactional summary projection rules.
  Added `crates/domain/tests/run_31.rs`, which drives 31 completed runs and
  compares the final projection with a hand-computed 31-run, 93-attempt value.
- Added 19 Rust-owned activity definitions to `generated/api/`, including
  `AssignmentEnrollment`, `AssignmentRun`, `QuestionAttempt`, and
  `StudentAssignmentSummary`. `AttemptProvenance` captures the full plan-defined
  implementation, object, and checksum record needed to reproduce an attempt.
- Added `docs/ACTIVITY_MODEL.md`, documenting the three-level record model,
  tenant ownership, independent policies, derived completion, summary
  projection, and WP-C3 gates.

### Behavior or Interface Changes

- Public asset redirects now carry an immutable one-year cache policy and the
  object's SHA-256 ETag. Protected redirects are `no-store`, suppress the
  referrer, and are independently rejected if the object backend exceeds the
  60-minute `content` or 5-minute `student-records` lifetime.
- Missing, cross-tenant, and unauthorized protected assets share the same
  not-found response. Signed URLs remain response headers only; audit payloads,
  browser storage, persisted markup, and JSON responses never contain them.
- Enrollment authorization now uses an explicit `UserId` distinct from the
  institution's `StudentId`. `QuestionAttempt` now carries a zero-based
  `assignmentPosition`, allowing repeated problem/version references and their
  retries to remain separate on the lower-camel browser wire.
- Run submission responses reveal correctness and points only when the
  question's feedback policy permits them. Deferred results remain hidden
  until run completion and release-gated results remain hidden; answer keys,
  expected values, private rubrics, and checker state remain server-only.
- The run API issues every never-attempted assignment position once before
  selecting an allowed retry. At most one unresolved attempt exists in a run,
  and a correct position cannot be retried.
- The grading contract now takes the exact published question definition
  rather than the compact persisted attempt. The run service resolves that
  definition from the attempt's immutable problem/version reference before
  grading, so tolerance and points are not duplicated into activity rows.
- Generic grading deliberately refuses partial credit without a capable
  backend or explicit private rubric, and refuses to fabricate a score for a
  file upload requiring review. Ungraded practice produces an explicit
  ungraded outcome rather than a false correctness result.
- Assignments now belong to exactly one `CourseId`, require a nonempty bounded
  title and at least one exact published version, and are listed only through
  their parent course. Assignment writes reject missing courses and catalog
  versions that are hidden or not assignable; they never copy a question
  payload into a tenant record.
- Coarse instructor status no longer implies access to every tenant course.
  Direct students may read their course assignments, direct instructors may
  manage them, tenant administrators may access every tenant course, and
  nonmembers receive the same not-found response as an absent course.
- Removed the generic direct-publication store method. All publication writes
  now pass through the atomic draft-to-catalog transition, so callers cannot
  bypass draft comparison, visibility grants, ownership, or lineage rules.
- Context-free catalog reads now expose public content only. Institution
  versions require the session-derived `TenantContext` and an exact
  tenant/problem/version grant under forced PostgreSQL row-level security.
- Deprecation now follows the active plan's soft-withdrawal contract: the
  version disappears from browse but remains assignable by an exact reference.
  Archival additionally blocks new assignments; both states remain exactly
  resolvable for historical records.
- The PostgreSQL application role can insert catalog records and update only
  lifecycle fields. It cannot update or delete immutable problem identity,
  payload, scope, backend, capabilities, metadata, authorship, or lineage.
- Taxonomy paging now uses collision-free hex-encoded scheme/code cursor keys,
  so slashes inside either controlled-vocabulary field cannot merge distinct
  terms.
- PostgreSQL store operations now assume the non-superuser, non-bypass
  `ple_app` role inside each transaction. Tenant operations set
  `ple.tenant_id` with transaction-local scope so pooled connections cannot
  retain another request's context.
- Activity writes lock their run, enrollment, and summary records as needed and
  commit the immutable activity plus compact grade projection atomically.
  Stable-key cursor queries use no positional offset.
- Complete persistence records now serialize as lower-camel JSONB with a
  SHA-256 checksum while normalized identity, relationship, timestamp, and
  paging columns retain database constraints and indexes.
- S3-compatible object writes now use `If-None-Match: *`, so concurrent or
  repeated puts cannot overwrite immutable content. Reads reconstruct the full
  object record from encoded metadata and reject semantic-key, bucket,
  category, version, media-type, size, or SHA-256 mismatches.
- S3-compatible signed URLs now confirm that the exact object record exists and
  use the server-supplied timestamp as the signing start time. Content remains
  valid for 60 minutes, student records for 5 minutes, and temporary objects
  remain unsignable.
- Seeded generation, graded work, partial credit, immediate hint feedback, and
  per-question timing now imply their backend requirements directly from each
  question definition. Client rendering, print export, and offline preview are
  explicit assignment-wide requirements applied to every selected question.
- Duplicate requirements collapse to one violation. The assignment editor and
  future publish route receive the same lower-camel question/capability result
  from one Rust implementation.
- First and latest grading now select by one-based run number rather than input
  order. Highest-score ties keep the earlier run, and instructor-selected
  grading remains empty until an instructor names a completed run.
- `store` now consumes the MOD-SCORE projection owner directly; WP-C3 callers
  may continue using the compatibility re-export from `domain::run`.
- Timed deadlines and grace boundaries are inclusive. Authorized pause time is
  reconstructed from server audit events and extends the base deadline before
  grace is applied; browser time remains display-only and cannot change the
  verdict.
- Malformed timer records now fail explicitly for policy/deadline mismatch,
  negative pause duration, timestamp ordering errors, and arithmetic overflow.
- Grading may no longer skip the submitted state, retry policy must resolve an
  incorrect response to retry-available or exhausted, and terminal attempt
  states refuse later events. Starting a retry authorizes a new attempt with a
  fresh seed and never mutates the earlier attempt record.
- Within-run completion now lives at
  `domain::completion::derive_within_run_completion`; `domain::run` retains a
  compatibility re-export for existing WP-C3 consumers.
- Frozen contract changes must now update the register, owning source, every
  named consumer and stub, generated projections, affected evidence, and the
  changelog atomically. Producer-first changes with later consumer repair are a
  blocking contract-gate failure.
- The built mock-backed client now supports the complete first-success path:
  course list, assignment list, assignment overview, resume the active run,
  validate one response in Rust/WebAssembly, and submit through the typed API
  boundary without a running backend.
- `src/wasm/index.ts` explicitly initializes the generated module from
  `ple_bridge_bg.wasm`. If initialization fails, the same facade uses the typed
  server-format fallback and displays one persistent degraded-mode notice; it
  never falls back to browser grading.
- Browser-facing generated data remains lower camel case while Rust modules,
  fields, and raw wasm-bindgen exports remain snake case behind their adapters.
  Components call only the lower-camel `validateResponseFormat` facade.
- JavaScript and CSS now receive independent content hashes in the built HTML,
  so a stylesheet-only change cannot reuse a stale JavaScript-derived cache
  key.
- `./build.sh` and `./check_codebase.sh` now verify the tracked fixture bytes
  against Rust types before regenerating their ignored TypeScript projection.
  The projection uses `satisfies MockFixtureCorpus`, so MOD-QM wire changes fail
  TypeScript compilation instead of silently drifting into mock data.
- Prettier validation now explicitly uses `.prettierignore`, keeping ignored
  `generated/api/` and `generated/fixtures/` inside the formatting gate while
  excluding only disposable `generated/wasm-export-check/` glue.
- Response-format violations now cross the Rust/JavaScript boundary in lower
  camel case. An exact serialization test covers both the tagged variant
  (`textTooLong`) and its data fields (`maxLength` and `actualLength`), and the
  generated Node bridge exercises the same JSON contract end to end.
- Moved executable response-shape validation out of `question_model` and into
  the key-free `domain` crate. The question model remains declarative;
  correctness checks and private rubrics remain server-only in `grading`.
- Seeded generation now uses `rand_chacha::ChaCha20Rng`, expands each `u64`
  seed through a domain-separated SHA-256, iterates output-bearing maps in
  `BTreeMap` order, samples inclusive ranges without modulo bias, and formats
  decimals as exact fixed-precision strings.
- Fresh practice takes priority over replay: every newly issued parameterized
  question instance receives a fresh server-owned seed, while resume and
  re-render of that same attempt retain its recorded seed.
- Moved reproducible TypeScript API definitions out of tracked `src/` and into
  ignored root `generated/api/`. Both `./build.sh` and `./check_codebase.sh`
  regenerate them from the tracked Rust model before consumption; TypeScript,
  ESLint, and Prettier still validate the resulting files. The generator
  removes stale files only when its ownership header is present.
- Added an explicit, non-defaultable `TenantContext` to every tenant-owned
  persistence operation. The memory backend keys each educational record by
  tenant and returns no rows when queried through another tenant's context.
- Activity writes and their summary projection are atomic in the store
  contract. Completing a run now updates its enrollment's first-completion,
  current-grade, and best-grade pointers in the same operation; subsequent
  policy-permitted practice runs remain startable.
- Object metadata, store errors, and object-store errors expose no PostgreSQL,
  SQLx, AWS SDK, or S3 implementation types.
- Aligned the run-policy wire contract with the active plan. Continued practice
  is unlimited, capped, or closed; grade policy is first, latest, highest, or
  instructor-selected; and variation uses new seeds, selected problem variants,
  or full regeneration.
- Aligned feedback disclosure with the four server-evaluated modes in the plan:
  immediate full teaching feedback, immediate correctness without an answer,
  deferred feedback, and instructor release.
- `StudentAssignmentSummary` now maintains current, best, and latest scores,
  completed-run count, total question attempts, and last activity without a
  synchronous scan of historical attempts. Out-of-order activity cannot move
  the last-activity timestamp backward.

### Fixes and Maintenance

- Repeated key-free response-format validation at the run server boundary
  before invoking an adapter, and added store-side rejection of nonfinite,
  negative, over-awarded, or zero-possible backend point results.
- Restricted the submission-idempotency table to application `SELECT` and
  `INSERT` privileges. The first receipt is immutable, changed keys or
  responses conflict, and unauthorized attempt owners receive a nonrevealing
  not-found result.
- Removed the vulnerable `rustls-webpki` 0.101.7 dependency path reported by
  three GitHub security alerts. The S3 dependency now selects only the modern
  HTTP 1.x TLS client, and the all-features graph resolves `rustls-webpki`
  0.103.13 with no legacy Rustls 0.21 client.
- Refreshed direct Rust and TypeScript dependencies to current stable releases
  with open minimum-version requirements. Major migrations include axum 0.8,
  rand_core/rand_chacha 0.10, SHA-2 0.11, syn 3, and SQLx 0.9; TypeScript uses
  the compatible open range `>=6.0.3 <7`.
- Moved the Rust toolchain to the floating stable channel and made the
  wasm-bindgen test setup derive its matching runner version from Cargo rather
  than carrying a static version.
- Synchronized all Cargo workspace packages to the repository's `26.08`
  CalVer release, represented as SemVer `26.8.0` in Cargo and npm metadata.
- Updated current container configuration to floating Rust, Debian, and
  PostgreSQL images. The PostgreSQL volume now mounts `/var/lib/postgresql`,
  the path required by PostgreSQL 18 and later official images, and SQLx uses
  its current Rustls/AWS-LC TLS backend.
- Added a lockfile boundary test requiring every resolved `rustls-webpki`
  release to include all three reported fixes.
- Moved the pure route table into `src/route_contract.ts` so Node contract tests
  do not import Solid Router's client-only implementation. `src/routes.ts`
  derives the executable browser definitions from that single table.
- Changed mock submission acknowledgement to return the in-progress attempt
  with no result, avoiding disclosure of an unrelated historical correctness
  result in the reference flow.
- Fixed number-key navigation to move focus with selection, prevented the
  narrow header from clipping its third navigation item, distinguished neutral
  idle validation from success, and applied the focus outline to the whole
  choice target.
- Excluded disposable `generated/wasm-export-check/` bindgen glue from ESLint
  and Prettier while retaining typecheck, lint, and formatting coverage for the
  regenerated `generated/api/` contract.
- Fixed the TypeScript generator's union wrapping to account for the actual
  declared type name and Prettier's intermediate line-break form. The longer
  `FeedbackDisclosure` union now passes Prettier unchanged, with a focused
  generator regression test.

### Decisions and Failures

- Recorded the pre-data database evolution plan in
  `docs/active_plans/decisions/database_schema_evolution_plan.md`.
  Published problems retain immutable versions, assignments retain one
  optimistic-concurrency-protected current state, and issued runs snapshot the
  exact configuration they execute. The working migrations will become a
  clean initial epoch before the first non-disposable database freezes
  forward-only history.
- Schema migration and application access remain separate privilege domains.
  The migration connection creates roles and schema objects; a production
  runtime login may assume `ple_app` but must not be superuser or bypass RLS.
- The PostgreSQL conformance test requires an explicit
  `PLE_POSTGRES_TEST_URL` pointing to a dedicated empty database. It is compiled
  in the normal all-features gate but remains ignored until the later local
  service pass; no `psql` client or port 5432 listener was available in this
  work session, and deployment debugging remains out of this lane.
- SQLx 0.9's public meta-crate records optional MySQL and SQLite macro-driver
  packages in `Cargo.lock` even with default features disabled. The compiled
  all-features workspace graph contains only `sqlx-core` and `sqlx-postgres`;
  using semver-exempt internal SQLx crates would require the exact pins the
  owner has prohibited.
- The frozen `ObjectStore::get` contract returns the full `ObjectRecord`, so
  the S3-compatible backend stores an encoded record with the object instead of
  keeping replica-local metadata. Base64 0.23.1 is optional with the `s3`
  feature and uses an open `>=` manifest requirement.
- A bounded live MinIO attempt stopped after `podman info` succeeded but the
  compose provider could not resolve its selected machine connection. No
  container deployment debugging followed; the compiled opt-in conformance
  test remains the exact live gate for the later deployment pass.
- npm initially selected TypeScript 7 because it is the registry's latest
  release, but the current frontend toolchain requires TypeScript 6. The
  manifest now permits every compatible 6.x release instead of pinning 6.0.3.
- SQLx 0.9 removed the combined `runtime-tokio-native-tls` feature; selecting
  its separate Tokio and Rustls/AWS-LC features restored the intended runtime
  and removed the old native-TLS dependency path.
- rand_core 0.10 renamed the generator's imported trait to `Rng`, and SHA-2
  0.11 removed hexadecimal formatting from its output array. Focused API
  migrations preserved the existing seed vectors and exact lowercase fixture
  checksums.
- MOD-DEPLOY is an implicit whole-system consumer in the register rather than
  being repeated in every consumer cell. Reserved paths for schema, worker,
  statistics, retention, LTI, and production deployment are explicit ownership
  commitments, not claims that those later modules are implemented.
- The first frontend Node test imported `src/routes.ts` and correctly failed
  because Solid Router called a client-only API without a DOM. Separating pure
  route data from browser assembly restored a real Node boundary instead of
  emulating a browser in unit tests.
- The first Chromium response-flow run exposed a degraded fallback: the current
  wasm-bindgen loader does not infer its `.wasm` URL when called without an
  argument. Passing the same-origin URL explicitly made Chromium request both
  bridge files and restored local Rust validation.
- A later Chromium launch was denied by the macOS command sandbox before any
  page opened. The identical repository runner passed with browser permission;
  this was an environment denial, not an application correction.
- Independent desktop and 320-pixel visual review used ignored artifacts under
  `generated/ui/`, keeping derivative screenshots out of Git while preserving
  local evidence.
- The committed JSON and SVG fixture corpus is intentional, reviewed work
  evidence. Its TypeScript projection is fully derivative and remains ignored
  under root `generated/`, matching the repository owner's artifact rule.
- Focused WP-C7 validation caught three issues before the complete gate: SVG
  colors required a wider Rust raw-string delimiter; a `satisfies` expression
  made the asset-body map too narrow for URL lookup; and ESLint rejected an
  `async` mock function with no `await`. Each was corrected at its owning
  boundary without weakening strict checks.
- Prettier's default Git-ignore behavior showed that ignored generated API
  files were not actually being formatted despite the earlier gate comment.
  Selecting `.prettierignore` explicitly made the stated policy executable.
- The first WP-C6 complete gate reached ESLint after the export test generated
  bindgen inspection glue, then correctly failed on two unused variables in
  that derivative file. Narrowly excluding only the disposable export-check
  directory made the focused lint check and the otherwise unchanged full gate
  pass; generated API definitions remain validated.
- The seed table remains Git-tracked as a deliberately reviewed compatibility
  baseline and work record; ordinary reproducible build output remains ignored
  under root `generated/`.
- The first Chromium launch was denied by the macOS command sandbox before any
  test ran; the identical local runner passed with browser permission. The
  first full pytest run then identified its Playwright import under
  `tests/e2e/`; moving it to the required `tests/playwright/` tier made the
  targeted and complete gates pass.
- The first complete gate run stopped in `cargo:clippy` with `E0463` while
  loading `aws-sdk-s3` dependencies. The manifest, lockfile, and a clean
  temporary build were correct, isolating stale metadata in `target/`; a
  package-scoped rebuild of `aws-sdk-s3`, `arc-swap`, and `rustversion` repaired
  the cache, and the unmodified gate then passed.

### Developer Tests and Notes

- MOD-CLIENT passes five focused HTTP behavior tests, strict TypeScript
  compilation, two authenticated validation-route tests, the JSON-safe fresh
  seed test, focused warning-free Rust Clippy, and diff hygiene. The complete
  `./check_codebase.sh` gate passes all 11 stages with 20 Node tests and 20
  server tests; live PostgreSQL and MinIO checks remain deferred to the later
  deployment pass.
- `cargo test -p learning-data-access --all-features --test conformance` passes five memory
  tests with the opt-in PostgreSQL case ignored. The new reusable asset suite
  covers public, institution, student-record, foreign-tenant, duplicate, and
  forbidden temporary deliveries, and the memory audit proves only authorized
  protected requests append events.
- `cargo test -p server_core asset --all-features` passes three asset-route
  tests covering CDN bypass, session authorization, nonrevealing denial,
  auditing, cache controls, and exact bucket lifetimes.
- `./check_codebase.sh` passes all 11 stages after MOD-API-ASSET, including
  TypeScript generation and checking, Node contracts, crate boundaries, Rust
  formatting, warning-free all-target Clippy, workspace tests, and doctests.
  All 812 repository pytest checks and the five-stage debug build pass as the
  complete task gate; live PostgreSQL and MinIO remain explicit opt-in service
  checks for the later deployment pass.
- MOD-API-RUN passes 14 server behavior tests, the reusable memory/PostgreSQL-
  compiled store conformance suite, 14 typed client/mock tests, focused
  all-feature Clippy with warnings denied, and generated fixture/type checks.
  The route tests cover authorization, format refusal before grading, exact
  replay, changed-request conflicts, fresh seeds, one-active-question advance,
  feedback redaction, and cursor-paged run history.
- The complete MOD-API-RUN gate passes all 11 `check_codebase.sh` stages, all
  812 repository pytest checks, and the five-stage debug build including native
  Rust, WebAssembly, generated types, fixture verification, and the Solid
  client bundle. Live PostgreSQL execution remains the explicit opt-in service
  gate for the later deployment pass.
- MOD-GRD passes six focused behavior tests covering all four numeric
  tolerance modes, choice-set comparison, text normalization, ordering,
  malformed response/key combinations, invalid public parameters, ungraded
  practice, backend-owned partial credit, and manual review. Package-scoped
  all-target, all-feature Clippy passes with warnings denied.
- The complete MOD-GRD per-patch gate passes all 11 `check_codebase.sh`
  stages, all 812 repository pytest checks, and the five-stage debug build.
  The compiled browser closure and processed WebAssembly allowlist remain
  answer-key-free.
- MOD-API-COURSE passes the memory/PostgreSQL-compiled store conformance suite,
  12 server library tests, 32 question-model tests plus two doc tests, eight
  generator tests, eight focused TypeScript/mock behavior tests, and focused
  all-feature, all-target Clippy with warnings denied. The live PostgreSQL
  execution remains an explicit opt-in service gate.
- The complete MOD-API-COURSE per-patch gate passes all 11
  `check_codebase.sh` stages, all 812 repository pytest checks, and the
  five-stage debug build including native Rust, WebAssembly, generated types,
  fixture verification, and the Solid client bundle.
- MOD-API-CAT focused validation passes 31 question-model unit tests and two
  doc tests, the memory catalog conformance suite, all 11 server library tests,
  eight generator tests, 11 TypeScript/mock behavior tests, and 251 Markdown
  link and ASCII checks. Focused all-feature Clippy passes with warnings denied;
  the PostgreSQL execution remains an explicit opt-in service gate.
- The complete MOD-API-CAT per-patch gate passes all 11
  `check_codebase.sh` stages, all 812 repository pytest checks, and the
  five-stage debug build including native Rust, WebAssembly, generated types,
  fixture verification, and the Solid client bundle.
- MOD-STO passes the reusable memory conformance suite and compiles its ignored
  PostgreSQL conformance, forced-RLS, and answer-key grant gate. Focused
  all-feature, all-target Clippy passes with warnings denied.
- The combined MOD-OBJ/MOD-SCHEMA/MOD-STO/MOD-API-AUTH offline gate passes all
  11 `check_codebase.sh` stages, all 812 repository pytest checks, focused auth
  and session tests, and the five-stage build. The live PostgreSQL and MinIO
  gates remain intentionally deferred to the local-service pass.
- MOD-OBJ passes `cargo test -p objects --all-features` with four unit tests,
  memory conformance, and the compiled opt-in MinIO test; focused all-target
  Clippy also passes with warnings denied.
- The MOD-OBJ complete offline gate passes all 11 `check_codebase.sh` stages,
  all 804 pytest checks, and the five-stage build. The live MinIO conformance
  test is intentionally pending the later container-connection pass.
- The dependency refresh passes all 11 `check_codebase.sh` stages, including
  TypeScript checks, ESLint, Prettier, Node tests, WebAssembly dependency
  closure, rustfmt, Clippy with warnings denied, all workspace tests, and doc
  tests. `npm audit` reports zero vulnerabilities, and static compose expansion
  confirms the current PostgreSQL volume path and required run-time secrets.
- MOD-CAP's committed table covers all eight capability variants, no-requirement
  behavior, multiple simultaneous gaps, deterministic ordering, and duplicate
  suppression. The published fixture returns exactly `algorithmicGeneration`
  and `serverGrading` through the built Node WebAssembly bridge.
- The MOD-CAP complete gate passed all 11 `check_codebase.sh` checks, all 798
  pytest checks, all 33 domain unit tests, the processed WebAssembly allowlist,
  TypeScript strict checking, ESLint, and the mock fallback behavior test.
- MOD-SCORE's hand-computed 0.4, 0.9, 0.7 fixture selects 0.4 for first, 0.7
  for latest, and 0.9 for highest through both batch reconciliation and
  incremental projection. Tie, instructor-selection, malformed-input, and
  monotonic-activity cases also pass.
- The MOD-SCORE complete gate passed all 11 `check_codebase.sh` checks, all 798
  pytest checks, and all 30 domain unit tests plus both domain integration
  tests.
- MOD-TIME table-driven tests cover untimed, open, grace, late, pause-adjusted,
  malformed, overflow, and inclusive-boundary cases. Native Rust, the mock
  fallback, and the built Node WebAssembly bridge all returned the same
  lower-camel verdicts; the reviewed export allowlist includes `timer_verdict`.
- The MOD-TIME complete gate passed all 11 `check_codebase.sh` checks, all 715
  pytest checks, the 314-check documentation gate, and the five-stage build.
- `cargo test -p domain`: 26 unit tests plus the 31-run and seed-vector
  integration tests pass. The state table covers all 10 legal transitions,
  illegal grading, terminal-state refusal, and exact lower-camel serde values.
- The contract register and active-plan catalog each contain exactly 36 module
  rows. The focused ASCII, Markdown-link, and whitespace gate passes 314 tests.
- `node --import tsx --test tests/test_frontend_contract.mjs`: 5 contract tests
  pass for the exact route table, server-free run screen, key-free format and
  timer fallbacks, and answer-bearing-type exclusion.
- `./run_playwright_tests.sh tests/playwright/frontend_contract.spec.ts`: 3
  Chromium tests pass for all 11 product routes, course-to-submission keyboard
  flow through the real WebAssembly module, focus movement, 56-pixel targets,
  and the 320-pixel no-overflow baseline.
- `./build.sh`: the complete five-stage build passes in 4.01 seconds observed;
  the built document independently fingerprints `main.js` and `style.css`.
- The color-accessibility audit measures all seven source colors as PASS at the
  5.5:1 target against white; the narrowest ratio is success green at 6.56:1.
- `cargo test -p domain --test test_determinism -- --nocapture`: all 65
  committed vectors pass natively, with failures designed to name the first
  divergent seed.
- `node tests/playwright/e2e_wasm_determinism.mjs`: the same 65 vectors pass in
  headless Chromium through `wasm-bindgen-test` (1 passed, 0 failed).
- The `wasm32-unknown-unknown` no-run gate compiles the browser determinism test.
- Package-scoped Clippy passes for the question model, domain, WASM bridge, and
  store with all targets, all features, and warnings denied.
- `cargo test -p objects -p learning-data-access`: both memory conformance suites, checksum
  tamper detection, and bounded-pagination validation pass.
- `cargo clippy -p objects -p learning-data-access --all-targets --all-features -- -D warnings`:
  passes.
- `cargo test -p question_model`: 30 unit tests and 2 doc tests pass after
  moving executable format behavior to `domain`.
- `cargo test -p domain`: 13 unit tests plus the dedicated 31-run and seed-vector
  integration tests pass.
- `./pipeline/build_wasm.sh && node tests/e2e/e2e_wasm_bridge.mjs`: the Node
  bridge returns the Rust-owned version and the expected camel-case validation
  report.
- `node --test tests/test_wasm_export_allowlist.mjs`: the processed module's
  complete export list matches the reviewed allowlist.
- `source source_me.sh && python3 -m pytest -q tests/test_crate_boundaries.py`:
  the exact browser workspace closure passes and excludes `grading`.
- `cargo test -p project-tools`: 8 generator tests pass, including fixture-contract
  shape, answer-key exclusion, and safe stale-output cleanup.
- `node --import tsx --test tests/test_mock_handlers.mjs`: 4 mock behavior tests
  pass with no API server running.
- `./build.sh`: the five-stage debug build passes, including fixture checking
  and projection generation; total observed time was 6.25 seconds.
- `./check_codebase.sh`: all 11 gate steps pass, including fixture drift, mock
  behavior, WebAssembly export allowlist, and crate-closure boundaries.
- `source source_me.sh && python3 -m pytest tests/`: 715 passed in 0.94 seconds.

## 2026-08-06

### Additions and New Features

- Brought the M0 foundation to a compiling, gated state across both toolchains:
  the 13-crate Cargo workspace, the Solid browser client, the WebAssembly build
  path, the container stack, and the extended check gate.
- Added `crates/server/src/health.rs`: readiness is a pure function over probe
  results, plus `probe_over_http`, which lets the API binary health-check
  itself so the runtime image needs no `curl` or `wget`.
- Added real dependency probes behind `/health`: `learning_data_access::postgres::ping` issues
  `SELECT 1`, and `objects::minio::probe_bucket` issues `HeadBucket`. The
  endpoint returns 200 only when both answer and 503 naming the failing
  dependency otherwise.
- Added `containers/Containerfile.api` (two-stage build, `--locked`, non-root
  runtime user) and `containers/compose.yaml` (PostgreSQL 17, MinIO, named
  volumes, one-shot creation of the `content`, `student-records`, and
  `temp-processing` buckets), with `containers/env.example` as the credential
  template.
- Added `pipeline/build_wasm.sh`, which compiles `crates/wasm` for
  `wasm32-unknown-unknown` and emits both `wasm-bindgen` flavors into the
  gitignored `dist_wasm/` staging directory: `web` for the browser client and
  `node` for the WP-F2 gate.
- Added `pipeline/build.mjs`, the esbuild JavaScript-API build for the Solid
  client, with content-hash cachebusting on the script and stylesheet URLs.
- Added `rust-toolchain.toml` pinning the compiler to 1.96.0 with the
  `wasm32-unknown-unknown` target, and a repo-root `Brewfile` for `podman`.
- Added `crates/project-tools`, the workspace-local `wasm-bindgen` glue generator. It
  is build tooling, outside the product boundary, and no shipped crate depends
  on it.
- Added `tests/e2e/e2e_run_all.sh` as the non-browser E2E runner. It carries
  the `e2e_` prefix because `tests/test_test_naming_conventions.py` requires it
  for shell files under `tests/e2e/`, which takes precedence over the
  `run_all.sh` name suggested in `docs/E2E_TESTS.md`.
- Added `src/log.ts` as the single logging surface for `src/`, `src/app.tsx` as
  the app shell, and base styling in `src/style.css`.
- Added `tests/e2e/e2e_wasm_bridge.mjs`, which loads the generated bridge in
  Node and asserts a value that crossed the Rust boundary.
- Added `docs/CODE_ARCHITECTURE.md`, `docs/CONTAINER.md`, and
  `docs/MACOS_PODMAN.md`.
- Added `docs/active_plans/m0-results.md`, the evidence record for M0: which
  plan assumptions held, which were disproved, what only running the steps
  revealed, the first build and artifact measurements, and the claims that
  remain untested.
- Added `build.sh`, the single master build. It runs rust, wasm, tsgen, and
  client in dependency order and reports per-stage timing, because the useful
  question is where the whole build spends its time and cargo only sees its own
  stage.
- Added `crates/project-tools/src/tsgen.rs`, the repo's own TypeScript generator. It
  parses the question model with `syn` and emits Prettier-shaped TypeScript
  into `src/api/generated/`.
- Completed WP-C1, the question model and taxonomy (MOD-QM). `question_model`
  now carries identity (`WorkspaceId`, `ProblemId`, `VersionId`, `AssetId` as
  distinct newtypes), `Capability` and `BackendCapabilities`, answer shapes
  (tolerance, text matching, selection cardinality), `ResponseDefinition` and
  `StudentResponse`, `ContentBlock` and `QuestionEnvelope`, generation specs,
  the question and run policies, and `QuestionDefinition` itself. 34 types, 23
  unit tests, 2 doc tests.
- Completed WP-C2, identity and lifecycle (MOD-ID). `Lifecycle` has three
  states (draft, published, withdrawn), and every change passes through one
  fallible `apply` function. Publishing is the transition that assigns a
  `ProblemId`; republishing a published version is refused, because changing
  published content means publishing a new version. Withdrawal keeps existing
  assignments working and stops new ones.
- Added `docs/PROBLEM_IDENTITY.md`, the WP-C2 document: the four identifiers,
  the draft rule, the transition table, and why immutability pays for itself.
- Added `docs/QUESTION_MODEL.md`, the WP-C1 contract document: what belongs in
  the model versus in `grading`, the wire format, and the Rust-to-TypeScript
  mapping table.
- Generated TypeScript for all 34 boundary types into `src/api/generated/`,
  passing `tsc --noEmit`, ESLint, and `prettier --check` unchanged.
- Added `.cargo/config.toml` with a `cargo tsgen` alias. Committed, not
  ignored: it is project configuration, and cargo's caches live in `~/.cargo`.

### Behavior or Interface Changes

- `check_codebase.sh` now runs three Rust steps after the TypeScript steps:
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test --workspace`. A missing `cargo` emits a loud SKIP, matching
  the script's existing honesty convention.
- `build_github_pages.sh` now delegates to `node pipeline/build.mjs`, because
  the esbuild CLI cannot load `esbuild-plugin-solid`.
- The workspace moved to edition 2024 (`rust-version` 1.85 already allowed it)
  and every member now inherits `version`, `edition`, and `rust-version` from
  `[workspace.package]`.
- `crates/domain` declares `chrono` with `default-features = false`, dropping
  the `clock` feature. The plan's load-bearing property is that `domain` has no
  clock; with default features it had one.
- `sqlx` moved from `crates/server` to `crates/learning-data-access`, and `aws-sdk-s3` is
  reached only through `crates/objects`. Both are feature-gated, and
  `crates/server` names neither: it uses the `learning_data_access::postgres::Pool` and
  `objects::minio::S3Client` aliases instead.
- `package.json` placeholders are filled: name `peptidyle-learning-engine` and
  version `26.8.0`. `clean` now points at `devel/clean_build.sh` and a new
  `clean:dist` points at `devel/dist_clean.sh`.
- `solid-js` and `@solidjs/router` moved from `devDependencies` to
  `dependencies` with `>=` pins, matching the policy in
  `docs/TYPESCRIPT_STYLE.md`.
- `tsconfig.json` gained `"jsx": "preserve"`, `"jsxImportSource": "solid-js"`,
  an `exclude` list covering `OTHER_REPOS`, `target`, `dist`, and `dist_wasm`,
  and an `include` list that covers `.tsx`.
- `src/index.html` loads `main.js` and `style.css` by relative path. A leading
  slash resolves to the user-site root on a GitHub Pages project site and 404s.

### Fixes and Maintenance

- Fixed five defects that stopped the workspace from compiling at all: a single
  `/` used as a comment in `crates/objects/src/lib.rs`, Python `#` comments in
  `crates/learning-data-access/src/lib.rs`, unmarked prose after `pub mod docx;` in
  `crates/export/src/lib.rs`, a literal `\n` escape pasted into
  `crates/server/src/main.rs`, and a call to `anyhow::Result` with `anyhow`
  undeclared.
- Fixed `src/main.tsx`, which called `createRoot(element, <App />)`.
  `solid-js/web` exports `render`, and it takes a component function; the old
  call would have rendered a static snapshot outside a reactive root.
- Renamed `src/App.jsx` to `src/app.tsx`. As a `.jsx` file it matched neither
  the ESLint nor the Prettier glob, so the only component in the app sat
  outside every gate.
- Fixed `tsconfig.lint.json`, whose `include` list matched no files and made
  `tsc` exit 2 with TS18003, which in turn broke typed linting of
  `playwright.config.ts`.
- Stripped trailing whitespace from the six `.toml` files that carried it, and
  reformatted every `.rs` file with rustfmt.
- Added `containers/env.local` and `.env` to `.gitignore`.

### Removals and Deprecations

- Removed three temporary files from the index that were never meant to be
  tracked: `crates/adapters/webwork/CARGOFILEtmp.txt`,
  `crates/adapters/webwork/Cargo.tmp`, and `pipeline/build.mjs.tmp`.
- Reconciled ten paths that were staged as added while deleted from the working
  tree, so a commit could not resurrect files that had been removed.
- Removed 16 empty directories accidentally created at the repository root
  (`dy/`, `e/`, `ea/`, `g-e/`, `gi/`, `i/`, `in/`, `LE/`, `le-l/`, `MS/`, `n/`,
  `ne/`, `pt/`, `rc/`, `rn/`, `ROB/`). Git cannot see empty directories, so
  they would never have appeared in `git status`.
- Removed the untracked root symlink `implementation_plan.md`, which duplicated
  `docs/active_plans/implementation_plan.md`.

### Decisions and Failures

- The prior M0 state was reported as complete but had never passed a gate: the
  Rust workspace did not compile, the Solid entry point called an API that does
  not exist, and `pipeline/build.mjs` was an empty file. Recorded here because
  the lesson is procedural rather than technical: run the gate before reporting
  the milestone.
- `npm run clean` points at `devel/clean_build.sh`, not `devel/dist_clean.sh`
  as WP-F3 specifies. `dist_clean.sh` is the deep reset that also removes
  `node_modules` and `target/`, and its own header documents `clean_build.sh`
  as the everyday cleaner wired to `npm run clean`. The plan's underlying
  defect (the script path did not exist) is fixed, and `clean:dist` exposes the
  deep reset under its own name.
- `package.json` carries version `26.8.0` rather than the `26.08` in `VERSION`.
  npm rejects `26.08` as invalid semver, so the CalVer value is expressed in
  the nearest valid form.
- Build timings reported by `build.sh` are information: they show where time
  went so a stage that grows becomes visible. Correctness lives in
  `./check_codebase.sh`, and the timings are most useful read as a trend across
  runs on one machine.
- `ts-rs` and `xml-rs` were both dropped in favor of our own implementations,
  at the owner's direction. Evidence that prompted the first: ts-rs output is
  not `prettier --check` clean, and M1 requires generated TypeScript to pass
  Prettier unchanged, which a third-party generator gives no way to control.
  The QTI parser is now ours to write, which raises the stakes of the M4
  hostile-corpus gate; that is the accepted cost.
- `uuid` refuses to compile for `wasm32-unknown-unknown` without an explicit
  randomness source. Rather than granting the browser an RNG, identifier
  generation moved behind a `question_model/generate` feature that the server
  enables and the WASM bridge does not. The browser never mints identifiers: a
  `ProblemId` is constructed on the publish transition. A compile error became
  a boundary.
- `build_github_pages.sh` was removed along with a stray `build_rust.sh`, and
  `./build.sh` replaces both. This repository ships a server platform, so the
  GitHub Pages framing was template inertia; `dist/` is a client bundle the API
  serves, not a static site. `deploy-pages.yml` was removed for the same
  reason.
- The `wasm-bindgen` glue is generated by a workspace crate (`crates/project-tools`,
  depending on `wasm-bindgen-cli-support`) rather than an externally installed
  `wasm-bindgen` binary. The generator and the `wasm-bindgen` crate compiled
  into the module must be the same version; taking the generator from the
  workspace makes `Cargo.lock` pin both, so they cannot drift. The first
  attempt used a Homebrew CLI plus a version comparison in the build script,
  which was a check for a problem the structure can prevent outright.
- The generated Node flavor failed to load: it is CommonJS, and the repo-root
  `package.json` sets `"type": "module"`, so Node parsed the `exports.`
  assignments as ESM. `pipeline/build_wasm.sh` now writes
  `{"type":"commonjs"}` into `dist_wasm/node/`, which scopes the module system
  to that directory instead of renaming generated files after the fact.
- The container health check is declared in `containers/compose.yaml` as well
  as in the Containerfile: podman compose did not carry the image-level
  `HEALTHCHECK` onto the running container, leaving `State.Health` unset.
- Two Swift build scripts for an unrelated application, `build_debug.sh` and
  `build_release.sh`, are staged at the repository root. They reference
  `swift build` and an `AppCloser` binary and appear to have arrived by
  accident. Left in place pending the owner's decision.

### Developer Tests and Notes

- WP-C1 tests caught two defects that would have shipped a wrong wire format.
  First, serde's `rename_all` renames enum _variants_ and leaves the fields
  inside them alone, so `graceSeconds` was serializing as `grace_seconds` while
  sibling structs used camelCase; every tagged enum now also declares
  `rename_all_fields`. Second, the generator's serde-attribute reader stopped
  at the first entry it did not recognize, so a `rename_all` written after
  `tag` was invisible to it and the TypeScript disagreed with the JSON.
- A third defect came from a brittle assertion rather than the code: a test
  searched the serialized JSON for `mass` and matched inside `molar_mass`. The
  assertion now searches for the quoted key.
- Identifier types were originally declared inside a `macro_rules!` body, which
  made them invisible both to the TypeScript generator, which reads source, and
  to a reader skimming for the contract. The four structs are now written out,
  with `impl_identifier!` supplying the shared behavior.

- `./check_codebase.sh`: 8 checks pass, 1 SKIP (`test:node`, no
  `tests/test_*.mjs` present yet). The Rust steps are `cargo:fmt`,
  `cargo:clippy`, and `cargo:test`.
- `cargo test --workspace`: 3 unit tests and 3 doc tests pass, 0 failures.
- `pytest tests/`: 645 passed in 1.09s.
- `podman compose -f containers/compose.yaml --env-file containers/env.local up -d`
  brings up `api`, `postgres`, `minio`, and the one-shot `createbuckets`.
  `mc ls local` lists `content/`, `student-records/`, and `temp-processing/`.
- The WP-F4 health gate was verified in both directions, which is the part that
  matters: `/health` returns 200 `{"status":"ready"}` with both services up;
  503 `{"failing":["postgres"],...}` with PostgreSQL stopped; 503
  `{"failing":["postgres","object-store"],...}` with both stopped; and 200
  again after both restart.
- The built bundle was checked for Solid rather than React output: zero
  occurrences of `react` or `jsx-runtime` in `dist/main.js`, with Solid's
  `insert` calls present.
- Tested the esbuild CLI claim rather than repeating it. `npx esbuild
src/main.tsx --bundle --jsx=automatic` fails with three errors of the form
  `No matching export in "node_modules/solid-js/dist/solid.js" for import
"jsx"`. The CLI path is unusable, as the plan says, but the failure mode is a
  hard build error, not the silent React-flavored bundle described in the first
  draft of these comments. The build script, `pipeline/build.mjs`, and
  `docs/CODE_ARCHITECTURE.md` were corrected to state the measured behavior.
- Learned during WP-F2 that `wasm32-unknown-unknown` was not installed, and
  that listing it under `targets` in `rust-toolchain.toml` makes rustup fetch
  it automatically on the next cargo invocation; no manual `rustup target add`
  was needed.
- `./pipeline/build_wasm.sh` produces `dist_wasm/web/` (19 KB `.wasm` plus 5 KB
  of glue) and `dist_wasm/node/`.
- `bash tests/e2e/e2e_run_all.sh`: 1 passed, 0 failed. The bridge returns
  `0.1.0` to Node, read from the Rust side's `CARGO_PKG_VERSION`.
- `./build_github_pages.sh` produces `dist/` with `index.html`, `main.js`
  (11.5 KB), `main.js.map`, `style.css`, `wasm/`, and `.nojekyll`, and the
  script URL carries the bundle content hash (`main.js?v=352b012a`).
