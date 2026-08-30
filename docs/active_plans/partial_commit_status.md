# Partial commit status

> Historical handoff snapshot. The current dated codebase status is
> [project_status_report_2026-08-09.md](project_status_report_2026-08-09.md), and the authoritative
> remaining dependency order is [active/release_completion_plan.md](active/release_completion_plan.md).

Status recorded 2026-08-09 after the audited database-evolution checkpoint, independent PostgreSQL
review remediation, the manual-grading and course-item-analysis packages, Store/server module
extraction, the score precision/display package, the refreshed README, QTI import hardening,
family-filtered production worker activation, and the strengthened live PostgreSQL acceptance
gate with measured partition-pruning, current-summary gradebook evidence, and a one-time
production-worker Course/Student-record purge validation, followed by a clean-cluster encrypted logical restore and
the maintained three-part local whole-system gate, the first executable PLE flat-question JSON
contract, and its completed persistence/publication/runtime package.

The status also includes accepted WP-QTI-7 schema/RLS/object-binding evidence, the completed,
independently reviewed WP-QTI-8 Memory/PostgreSQL conversion boundary, accepted WP-QTI-9 server
routes, accepted WP-QTI-10 author UI, and accepted WP-QTI-11 live
PostgreSQL/RLS/profile-to-native acceptance. WP-QTI-12 independent close-out is also accepted with
no remaining P0/P1 finding.

The seven missing flat-question families now have an integration plan in
`docs/active_plans/active/flat_question_family_evolution_plan.md`. It
preserves exact v1 single choice and assigns the QTI Package Maker QTI-JSONL specification, reference
engine, examples, and tests to WP-FQ-0 instead of freezing a separate PLE schema. One versioned
adapter/compiler protects the runtime from source-format change; MATCH is the first family vertical.

The owner's Blackboard-inspired course appearance guidance is accepted through the M3 package in
`docs/active_plans/decisions/course_appearance_plan.md`: 15 measured three-color biome/habitat
themes (`woodland` consolidated into `forest`), one revisioned centered course-entry banner, and
course-root theming with security, accessibility, object-lifecycle, and visual gates. WP-CA1 is
accepted: the Rust/generated browser contract and executable instructor route passed focused, full,
and built-browser gates without widening Wasm. WP-CA2 is also accepted: course-bound banner
candidate/current object identities passed classification, signing/refusal, memory conformance,
S3-feature compilation, and full repository gates. WP-CA3 is accepted as well: one forward migration
and the Memory/PostgreSQL owners now enforce revision CAS, persisted session authority, bytes-first
promotion, exact-current delivery, and bounded two-phase cleanup. HTTP image normalization,
production appearance routes, and current-only protected delivery are now accepted through WP-CA4.
WP-CA5 is also accepted: Grass is the default, all 15 fail-closed theme projections are scoped below
the global shell, run data is reused without a theme-only learner fetch, and cross-course/global
navigation clears prior variables. WP-CA6/WP-CA7 and WP-RC1 are also accepted: the real
keyboard-complete settings page, exact entry-only banner, all-seven-route scope, combined
PostgreSQL/MinIO idempotent cleanup, current-pointer database guard, visual evidence, durable docs,
and three independent no-P0/P1/P2 reviews passed. WP-RC2 is also accepted: production adapter names
now match their implementation, catalog resolve/search are explicit Store requirements, the empty
native renderer declaration is removed, and durable feedback release has one projection. The next
dependency is WP-RC3 shipped upstream WeBWorK while independently owned WP-FQ-0 contract work may
proceed.

Later owner-requested support work is also present in the shared tree: the focused student no-mouse
pass, the all-in-one local Podman launcher, and the implemented/proposed database structure and
growth map. These do not make the incomplete course UI, production identity, FERPA deployment, or
managed operations complete.

## Commit boundary

WP-QTI-11 started from clean `main` at `b297808`. Its bounded implementation and later accepted work
now share a mixed staged/unstaged worktree for owner review. The historical database checkpoint
below records the coherent cross-layer transition from the disposable 34-migration history to the
accepted six-file pre-data SQLx baseline; it is not a claim that those paths form one current commit.

## Completed work

The dependency-ordered database-plan packages now complete:

- exactly six domain-owned migrations for principals, catalog/authoring, courses/assignments,
  activity/feedback, operations/analytics, and retention;
- explicit `cargo tools database status`, `migrate`, and `verify` operations, with verify-only
  application startup and exact SQLx ledger/checksum compatibility;
- human-readable catalog problem IDs and versions while UUID identity remains authoritative;
- normalized stable assignment items, pinned selection candidates, immutable delivered run order,
  exact decimal point values, and explicit attempt states;
- generation-fenced current scoring with private staging, atomic newest-generation publication,
  concurrent-submission restaging, and no scoring-history tables;
- revision-checked Delete and Regrade, future-run omission, protected submitted evidence, and
  recalculation;
- direct-instructor force-submit and clear with stable action IDs, minimal audit evidence,
  retry serialization, and no fabricated student response or grade;
- mutable visibility, availability, due/close boundaries, late policy, time limits, attempt limits,
  and generation-fenced durable auto-submit; and
- revisioned direct-student and course-group policy exceptions. Each dimension chooses the most
  permissive applicable value, issued attempts record the resolved policy and contributors, and
  exception/group/course-membership changes atomically re-resolve active work.

When a removed accommodation exposes an elapsed deadline, the active attempt auto-submits in the
same transaction. It records an authoritative submission time but creates no response, evaluation,
or score. Course roster replacement removes invalid group membership in both Store backends;
stable group identities cannot move between courses. Retention fences and purges group membership
and direct-student exception records.

## Subsequent database hardening

The independent Rust/SQLx/PostgreSQL review has been converted into executable remediation rather
than left as a static report:

- the isolated PostgreSQL gate applies the six migrations twice, verifies exact checksums,
  detects a copied migration mutation, and leaves the developer's `pg-test` instance untouched;
- `problem` and `answer_key` now use forced RLS, all baseline constraints apply validated, activity
  partitions use the fixed 2026-08 through 2028-09 epoch, default partitions must remain empty, and
  the reviewed foreign-key access paths have supporting indexes;
- migration commands require `PLE_MIGRATION_DATABASE_URL`, refuse the application principal for
  DDL, and E2E seeding requires the explicit `--apply-migrations` flag;
- both SQLx pools have explicit acquisition, idle, and lifetime limits; constraint failures no
  longer forward raw database text; connection loss during schema verification degrades health;
  and `40001`/`40P01` aborts receive at most three whole-operation attempts; and
- PostgreSQL assignment-timing, assignment-export, asset-delivery, catalog, external-tool, jobs,
  QTI, retention, sessions, connection, migration, and manual-grading behavior; Memory
  asset-delivery, external-tool, queue, exports, catalog, QTI, sessions, retention, and
  manual-grading behavior; Store activity/scoring, asset-delivery, external-tool, QTI,
  publication-validation, feedback, and policy contracts; conformance domains; Memory
  catalog/statistics tests; server external-tool routes; and QTI publication/runtime-backend tests
  now live in focused child modules.
  Public paths remain compatible. The PostgreSQL parent is now 6,630 lines, down from the
  11,329-line starting point; Memory is 4,126 lines, down from 9,856; and Store `lib.rs` is 2,319
  lines, down from 4,840. Authentication sessions now have a 251-line contract, 71-line Memory
  implementation, 127-line PostgreSQL implementation, and 84-line conformance owner. QTI has a
  241-line contract, 172-line Memory implementation, 310-line PostgreSQL implementation, and
  553-line conformance owner. The QTI adapter has a 16-line stable facade, 293-line model,
  976-line bounded parser, and 340-line test owner. The QTI publication route is a 585-line
  production owner paired with a 532-line private behavior-test owner. The QTI runtime backend is a
  394-line production owner with a 332-line shared fixture owner, 263-line direct private-grading
  owner, and 431-line run-lifecycle owner. Further bounded capability extraction remains useful.

The live retry fixture forms a real serializable read/write dependency cycle. PostgreSQL aborts one
transaction, the Store starts a fresh transaction, and both operations commit in exactly three
total attempts. This is a permanent part of the disposable database gate.

## Contributor component naming

Contributor documentation and physical paths now use learning data access,
in-memory data access, PostgreSQL data access, and project tools. The atomic
rename changed `crates/store` to `crates/learning-data-access`, its `memory`
backend module to `in_memory`, and `crates/xtask` to `crates/project-tools`.
Cargo package and directory names use hyphens, Rust import and module names use
underscores, and `cargo tools` is the sole repository-automation command.

## Manual grading checkpoint

The current-state manual grading design is implemented through shared types, MemoryStore,
PostgreSQL, schema/RLS/retention integration, and an independently reviewed server route. It keeps
one mutable current evaluation plus a minimal idempotency receipt, persists exact
instructor-entered credit as `NUMERIC`, and leaves summary publication behind the
scoring-generation fence. The focused HTTP gate now covers the real pending-submission dispatch,
non-enumerating instructor authority, strict decimal and body handling, revision conflicts, exact
replay, receipt secrecy, and `no-store` extractor failures. A permanent opt-in PostgreSQL fixture
now completes the package through the production Store: one normal automatic submission and one
response-bearing manual submission remain unpublished while manual review is pending; a grade is
corrected from `0.25` to exact `NUMERIC` `0.5`; the prepared old generation is superseded; and only
the corrected generation publishes the `0.75` mixed score, original first-completion timestamp,
and current/best run pointers. Student, unrelated-student, and foreign-course probes cannot
enumerate the evaluation. The disposable baseline runner invokes this fixture on every live run.

That live path found and closed two production-only defects. Immutable successor-link creation no
longer uses `SELECT FOR UPDATE`, which PostgreSQL correctly rejected under the least-privilege
insert-only application grant; primary-key `INSERT ... ON CONFLICT DO NOTHING` now serializes the
link without broadening privileges. The generation-fenced projection now carries the earliest
eligible completed-run timestamp into the enrollment in both Store backends instead of publishing
only the summary and grade-run pointers. Course-local analysis remains a separate projection and
consumes only the current published scoring state, as described below.

## Course item-analysis checkpoint

Course-local item analysis is now a separate Course-owned, current-only projection rather than an
extension of the identity-free global catalog statistics. The pure reducer and both Store backends
select each enrollment's newest run, suppress prior completed work while that newest run is active,
and use the newest current attempt per immutable assignment item and problem version. Graded items
contribute correctness-based difficulty, exact-credit mean and sample standard deviation,
discrimination, fixed response categories, and assignment score. Pending manual work and
unanswered terminal work remain explicit instead of becoming zeroes; cleared and exempt work is
omitted.

Scoring publication transactionally reserves one lower-priority analysis job. Its worker prepares
outside the publication transaction, then verifies the assignment, scoring generation, job lease,
and private staged checksum before atomically replacing the current report. A correction that
overtakes prepared analysis completes the old job as superseded. Analysis failure cannot delay or
roll back a current learner-visible grade. The production registry now claims this family only
because its handler and atomic committer are both present.

The instructor-only route derives direct-course-instructor or Sysadmin authority from
the persisted active session inside the Store boundary. Its DTO contains aggregate metrics only:
no installation-scope or course identifiers, learner or attempt identity, raw response, object key, answer key,
feedback, or grading implementation. Current and staging tables use forced RLS, course binding,
retention fences, purge ordering, and least-privilege grants.

The permanent Memory conformance matrix covers mixed automatic and manual grading, correction,
force-submit and clear, latest-active-run suppression, stale-stage supersession, authorization, and
serialized privacy. The disposable PostgreSQL path independently proves the production query,
generation fence, one corrected current report, real-session authority, RLS, retention integration,
and exact completion time from run start to terminal learner submission. Manual-grading delay never
inflates learner completion time. Independent re-review reports no remaining P0/P1 finding.

## QTI import-hardening checkpoint

WP-QTI-1 is complete and independently reviewed. The adapter now owns exact persisted Canvas,
Blackboard, and honest generic profile identities; one corpus-grounded vendor matrix; normalized
manifest/resource/item detection; and canonical safe-report/public/private/combined/warning digest
contracts. Retained Canvas and Blackboard packages supplied the exact nested schema metadata,
vendor-local `assessment_meta` paths, and reciprocal dependency graphs. Malformed or mixed vendor
evidence refuses the vendor claim, while unrelated IMS Content Packages retain the generic
compatibility path. Private choice mappings cannot be serialized or formatted for logs. The
implementation evidence is recorded in
`docs/active_plans/workstreams/qti_profile_contract_implementation.md`. WP-QTI-2 parser-ready
fixtures are next; no vendor item parser, flat-source conversion, schema, or UI behavior landed in
this contract package.

WP-QTI-2 is also complete and independently reviewed. The local adapter corpus now contains
parser-ready Canvas and Blackboard manifests, assessment metadata, static single-choice items, and
one-fact near misses grounded in retained package syntax. A reusable test support module constructs
safe logical ZIPs, reads exact members, and validates structurally balanced single-root XML; no test
depends on `OTHER_REPOS` or compares brittle ZIP timestamps/bytes. The implementation evidence is
recorded in
`docs/active_plans/workstreams/qti_profile_fixture_corpus_implementation.md`. Exact vendor parsers
are next; this fixture package changed no runtime parser, Store, schema, route, or UI behavior.

The planned shared hostile-input extraction is complete as well. The generic parser now delegates
to a 211-line bounded archive owner and a focused XML owner while retaining its exact narrow entry
grammar and public behavior; its parent fell from 976 to 613 lines. Opaque crate-private validated
entries prevent profile parsers from bypassing path/link/size checks, and both vendor parsers can
reuse the same DTD/entity/nesting/resource-limit implementation. Independent review found no
P0/P1 behavior or security regression. Evidence is recorded in
`docs/active_plans/workstreams/qti_shared_safety_extraction.md`.

The additive ordered-XML foundation is complete and independently reviewed. The same private XML
tree now records mixed content, raw CDATA, comments, processing instructions, prefixes, attributes,
and inherited namespace bindings in source order while preserving the generic parser's previous
aggregate-text behavior exactly. Undeclared prefixes remain visible for profile-specific refusal
rather than being assigned a fabricated namespace. The XML owner remains below the 600-line target,
and no public adapter, Store, schema, server, or UI surface changed. Evidence is recorded in
`docs/active_plans/workstreams/qti_ordered_xml_foundation.md`.

The shared Q2 mapped-item contract is complete and independently reviewed. Deterministic choice-ID
mapping preserves valid PLE IDs, reserves them before deriving `qti_` SHA-256 identifiers, and
extends digest prefixes on collision without changing a choice's mapping when source order changes.
Accepted and rejected instructor reports serialize only closed code/location/template diagnostics;
correct choices, raw vendor IDs, private maps, object/archive identities, and all mapping digests
remain outside Debug and serialization. Canvas points must be declared; Blackboard points must use
the explicit reviewed 1.0 default. Only the owning mapped item can create its accepted disposition
or profile/version-bound integrity digests. Focused owners remain compartmentalized at 393 production
lines or fewer, with mapped-item tests in a separate 279-line owner. Evidence is recorded in
`docs/active_plans/workstreams/qti_mapped_item_contract_implementation.md`.

The shared strict-markup package is complete and independently reviewed. Canvas XML text/CDATA is
bounded before concatenation and receives exactly one `html5ever` tokenizer pass; the forgiving DOM
builder is not used. Blackboard content follows the ordered XML tree directly with exact item/XHTML
namespace checks and is never reparsed as HTML. The common projector accepts only the frozen text
allowlist and emits deterministic escaped CommonMark. Attributes, comments, processing instructions,
HTML recovery, links, tables, styles, media, SVG, MathML, and unknown elements refuse with a closed
safe diagnostic. Token storage and rendering remain bounded before allocation growth, including
entity expansion, code fences, lists, and block separators. Evidence is recorded in
`docs/active_plans/workstreams/qti_markup_implementation.md`.

WP-QTI-3, the bounded Canvas QTI 1.2 parser, is complete and independently reviewed. It accepts
only the exact bounded Canvas archive grammar, manifest/resource/dependency evidence, assessment
metadata, and static single-choice tree. A candidate must declare finite nonnegative points, use
one single-cardinality response with two through 100 ordered labels, and use the exact single
correct `varequal` plus `SCORE=100` scoring shape. Strict markup and deterministic choice mapping
are reused. Unsupported scoring, feedback, media, table/style markup, and structural extensions
refuse only their item, while invalid manifest/resource graphs, duplicate item identifiers, and
unexpected archive entries reject the package. Safe reports remain opaque to answers, raw vendor
identifiers, archive bytes, and mapping digests; the private result has neither `Debug` nor
serialization. The exact IMSMD LOM element remains inert provenance evidence. The focused
production/test owners are 553, 234, 135, and 214 lines. Adapter validation passed 60 unit tests,
6 fixture integration tests, and 7 compile-fail doctests, plus strict Clippy, formatting,
crate-boundary, whitespace, and diff checks. Independent review reported PASS with no P0/P1
finding. Evidence is recorded in
`docs/active_plans/workstreams/qti_canvas_parser_implementation.md`. WP-QTI-4, the exact
Blackboard QTI 2.1 parser, is next.

WP-QTI-4, the bounded Blackboard Original QTI 2.1 static-pool parser, is complete and independently
reviewed. It accepts only the exact bounded pool archive, manifest/resource/dependency evidence,
assessment-test references, and static single-choice item subset. An item needs one correct single
response, two through 100 ordered choices, and absent/exact no-op response processing. Missing or
false shuffle is static; `shuffle="true"` is accepted only when every choice is fixed. The observed
inert `SCORE`/single/float outcome declaration is allowed as compatibility provenance only; it does
not add scoring. Every accepted item uses the explicit safe, review-required 1.0-point default.
Real shuffle, alternate scoring, feedback, media, tables/styles, extensions, policy, and unsupported
markup refuse only that item. Safe reports exclude answers, raw vendor IDs, maps, archive bytes, and
digests; private mapped items are neither serializable nor debuggable. Root XSI schema hints and
IMSMD LOM stay inert provenance only. The focused owners are 371, 411, 151, and 275 lines. Adapter
validation passed 79 unit tests, 6 fixture integration tests, and 9 compile-fail doctests, plus
strict Clippy, formatting, crate-boundary, whitespace, and diff checks. Independent review reported
PASS with no P0/P1 finding. Evidence is recorded in
`docs/active_plans/workstreams/qti_blackboard_parser_implementation.md`. Q3 pure native flat
mapping is next.

Q3/WP-QTI-5, the pure native factory and server-only QTI flat bridge, is complete and independently
reviewed. The native factory consumes only trusted ordered mapped fields, applies fixed imported
flat v1 defaults, validates canonical finite nonnegative points, enforces the 256 KiB canonical
source cap through the native owner, canonicalizes and reparses, then invokes the existing split
public/private compiler. The crate-private server bridge accepts only owner-bound Canvas or
Blackboard v1 mappings, retains its private server parts for the later provenance command, and
produces no Store, object, schema, HTTP, UI, or Wasm behavior. Real Canvas and Blackboard fixtures
equal the canonical source, public draft, and private binding from equivalent hand-authored flat
source; Blackboard retains its exact defaulted 1.0 points. Inputs, factory products, bridge results,
and mapping parts remain outside `Debug` and serialization. The owners are 422 and 288 lines.
Validation passed native 31 unit tests and 6 doctests, QTI 79 unit tests, 6 fixture integration
tests, and 9 doctests, plus 3 focused server bridge tests and the full 162-unit/1-doctest server
gate. Strict Clippy, formatting, crate-boundary, whitespace, and diff checks passed. Independent
review reported PASS with no P0/P1 finding. Evidence is recorded in
`docs/active_plans/workstreams/qti_native_flat_bridge_implementation.md`. Q4/WP-QTI-6, the
provenance contract and object key, is complete; no backend mutation has begun.

Q4/WP-QTI-6 is complete and independently reviewed. The adapter now emits a versioned opaque
ordered choice-map payload with a fixed binary encoding and checksum. Storage owns closed Canvas
and Blackboard profile tuples, the server conversion version, current and immutable published
origin types, opaque private payload handling, fail-closed promotion, and one atomic conversion
command. The contract preserves current origin through ordinary editor saves, replaces it only
through a provenance-aware conversion, and fixes the workspace-draft/import/origin/source/
publication lock order. `PublishedImportArchive` is a distinct published-record-bound, non-signable object
key with a deterministic SHA-256-derived identity and published-retention semantics. Focused
adapter, data-access, and object tests passed; strict formatting, Clippy, crate-boundary,
whitespace, and diff checks passed. Independent review reported PASS with no P0/P1 finding.

Q5/WP-QTI-7, the QTI provenance schema/RLS/object-binding gate, has refreshed implementation
evidence after the choice-map checksum repair; final independent checksum re-review reported PASS
with no P0/P1 findings. A
dedicated `NOLOGIN`, `NOINHERIT`, `NOBYPASSRLS` provenance broker owns narrow protected
capabilities over six forced-RLS current/published origin, private choice-map, and committed
profile/item-evidence relations. Every origin verifies the committed import's full typed
`ObjectRecord`, including typed workspace-source key, checksum, size, media type, license,
provenance, and creation time. The SQL boundary now aligns the Rust 1,024-Unicode-scalar identifier
limit across import item/result/grading, published grading, evidence, and origin rows. Current
lineage pins a committed import; ordinary draft cleanup releases current lineage only; published
lineage and choice maps are immutable and retained. PostgreSQL recomputes SHA-256 over private
choice-map bytes in a direct table trigger for both current and published maps, so even a direct
provenance-broker write cannot supply a divergent digest. The final fresh disposable baseline pass
applied all six migrations to an empty database, re-applied without change, verified the ledger, and
ran capability-negative plus direct-broker negative probes alongside real-role RLS, Unicode,
evidence, pin/release, published-retention, and child-first cleanup checks. Evidence is recorded in
`docs/active_plans/workstreams/qti_provenance_schema_implementation.md`. WP-QTI-8 Memory and
PostgreSQL atomic conversion is complete.

WP-QTI-8 is complete and independently reviewed. One closed, non-serializable staged profile-
evidence value closes H2 while the QTI import is prepared; conversion requires the committed
accepted result's exact source-identifier and `itemId` binding, profile tuple, and complete digest
set. Memory and PostgreSQL follow the frozen draft/import/origin/source/publication lock order and
atomically commit the CAS revision, draft, canonical source, current private grading, and current
origin. Ordinary save and conversion stage current grading; publication accepts no caller grading
payload and promotes only the locked stored value after origin promotion. PostgreSQL uses the
forced-RLS provenance and grading brokers and performs no direct Store read of private grading,
choice-map, or provenance secret tables. `Sha256Digest` now has strict lowercase 64-hex JSON serde.
Shared conformance, PostgreSQL feature coverage, the full fresh database baseline, and independent
review passed; review reported no P0/P1 finding.

WP-QTI-9 is complete and independently accepted. Author upload stores the exact bounded ZIP in a
deterministic private workspace object and creates one deterministic `qtiImport` job; exact replay
is stable and divergent replay refuses. Safe reports contain package/item defaults, diagnostics, and
digest acknowledgements only. The strict profile worker stages complete accepted-item evidence;
strong-ETag conversion rereads and reparses the retained archive before the atomic WP-QTI-8 Store
command; flat publication copies the source to deterministic non-signable `PublishedImportArchive`.
Memory and PostgreSQL both serialize prepared import work with draft deletion, preventing orphaned
prepared evidence. Focused and full offline checks, one-time oversized/chunked ingress evidence,
and independent route/worker reviews passed with no P0/P1 finding. WP-QTI-10 through WP-QTI-12 are
now also accepted.
Evidence is recorded in
`docs/active_plans/workstreams/qti_server_routes_implementation.md`.

WP-QTI-10 is complete and independently accepted. The existing workspace author route now composes
one feature-local QTI review panel above the existing flat editor; it does not create a new product
route or widen global browser contracts. The client sends an opaque ZIP to the existing same-origin
route, decodes only bounded answer-free reports, keeps selected files and report context in component
memory, and requires `no-store`. The UI names the recognized profile, distinguishes accepted and
rejected items without color alone, shows defaults and warnings, requires explicit acknowledgement,
and supports queued/processing refresh, all-rejected and unsupported recovery, and exact retry after
an ambiguous upload. Conversion requires an accepted item and the displayed clean strong revision.
After a committed conversion, the stale editor becomes inert while the same workspace route refetches;
failed refetch keeps it locked behind a repeatable reload action, with no second conversion or new
import. Node contract tests and four real-route Chromium scenarios passed, including keyboard and
375 px reflow. `./check_codebase.sh` passed 11 of 11 checks with 173 Node and 184 server tests;
independent security and HCI reviews reported no P0/P1 findings. Evidence is in
`docs/active_plans/workstreams/qti_author_ui_implementation.md`.

WP-QTI-11 is complete. A fresh isolated PostgreSQL 17 database applied and verified the six-file
baseline, processed a minimized mixed accepted/rejected Canvas archive through the real upload and
worker path, converted and published the accepted item as native flat content, and graded correct
and incorrect responses through the isolated PostgreSQL grader. Real application, student, grader,
and foreign-account probes enforced RLS and protected-capability boundaries. Current and published
archive/provenance checksums agreed; workspace cleanup removed current private state while immutable
published provenance remained. The complete disposable database gate, all 11 repository checks, 51
built Playwright scenarios, and 1,644 Python tests passed. WP-QTI-12 then ran six separate review
passes, corrected stale README and profile-to-native owner-map documentation, and passed re-review
with no remaining P0/P1 finding.

The accepted
`docs/active_plans/decisions/qti_profile_mapping_plan.md` defines the completed
QTI-profile conversion sequence. It keeps the generic hostile-archive importer intact, adds
separate exact Canvas QTI 1.2 and Blackboard QTI 2.1 static-single-choice
profiles, maps accepted items through canonical PLE flat-question JSON, and
preserves an immutable private provenance link to the unchanged archive.
Unsupported scoring, feedback, rich markup, media, and policy remain visible
per-item refusals rather than lossy conversion. Import and instructor review
precede an optional, separately gated export milestone.

QTI import now distinguishes unsafe archives from unsupported content. Path
escapes, symlinks, unreferenced executable-like files, expansion limits, and
malformed or over-complex XML reject the archive before any Store mutation.
For a structurally safe package, supported items continue while missing or
unsupported items receive durable rejected results with source identifiers and
actionable warning detail. Accepted items receive answer-free normalized
checksums; exact normalized matches and matching presentations with different
grading are reported as exact and likely duplicate warnings within the import
batch. The original immutable package remains the re-import/correction source.

The data-access contract validates a complete bounded per-item report and
persists it identically in Memory and PostgreSQL. PostgreSQL uses a normalized
forced-RLS result table while the committed safe registry remains the read
projection; private grading bytes stay in the separately injected grader
capability. The server carries manifest identity, source format, importer,
warning detail, status, and normalized checksums into that registry without
putting answers or archive bytes into a browser DTO.

The permanent hostile-package and partial-success adapter tests, Store
conformance, server worker test, and disposable PostgreSQL oracle cover this
boundary. The live test found and closed two older production-only gaps: the
QTI staging broker could not call the actor-context predicate, and queue SQL
expected camel-case export/import fields even though the durable Rust enum
intentionally serializes its fields in snake case. The corrected live path
proves hidden preparation, exact lease-bound commit, accepted/rejected rows,
provenance, warning persistence, answer secrecy, and foreign-account
non-enumeration.

## PLE flat-question JSON checkpoint

PLE now has a deliberately narrow internal source format for ordinary static
single-choice questions. The native adapter owns a bounded, strict, versioned
JSON parser and compiler rather than adopting Canvas or Blackboard QTI XML as
the internal model. It uses stable semantic choice IDs, per-choice feedback,
correct/incorrect outcome feedback, the shared score and policy types, and
canonical SHA-256 bytes. Duplicate members, extension fields, invalid IDs,
unbounded text, and oversized sources refuse before publication.

Compilation produces the existing answer-free draft question plus separately
serializable grader-only key and feedback material. The private half is bound
to the exact public content and refuses substitution. Typed object delivery now
also refuses signed URLs for both workspace and published sources, closing the
answer-bearing source boundary for this format and the existing adapters.

The completed persistence/publication/runtime package now atomically saves the
typed workspace draft and its private canonical source, promotes exact source
bytes to an immutable non-signable `ProblemSource`, and writes the answer-free
public payload separately from typed grader-only key/feedback material. The
original save and publication responses never return private source or grading
bytes.
`PostgresGraderStore` reaches the security-definer/RLS boundary through its own
login, and the native runtime receives only that isolated capability. A real
compiled blue-correct/right and red-wrong PostgreSQL live gate, followed by an
independent re-review, passed.

The completed instructor editor adds the only intentional answer-bearing
browser path: an authenticated author-role instructor's own canonical-source
`GET`/`PUT`, with `Cache-Control: no-store` and a strong ETag. It neither
returns a signed source URL nor widens ordinary browser contracts. Learner
preview, Wasm, and public publication DTOs remain answer-free. The focused
feature modules provide create/open/edit/save/reload-conflict/review/publish
behavior with request-generation guards and a double-save lock. The route uses
the legacy generic editor only when that protected source route returns 404;
other protected-load failures remain visible. The accessible choice radios have
per-choice names, and the author surface reflows at a 375 px viewport. Focused
fixtures cover the mounted component/client/repository boundary; this is not a
claim that production authentication or a deployed browser walkthrough has
been exercised. Bounded QTI-profile mappings are next. YAML remains an
optional later human-editing input that compiles into canonical JSON rather
than becoming a second source of truth.

## Production worker checkpoint

The queue boundary now requires every claimant to provide a nonempty, duplicate-free family
filter. Memory and PostgreSQL apply that filter to ready claims, expired-lease cleanup, and
operational depth, so a partial worker cannot consume or dead-letter another family's job.
PostgreSQL retains its short `FOR UPDATE SKIP LOCKED` broker transaction and the existing
non-analysis priority; the filter is an explicit closed list of durable payload discriminants.

The server registry pairs each claimable family with both its cancellable handler and atomic
committer, then derives the broker filter from those same entries. Production contains six complete
families: current assignment scoring, course item analysis, attempt auto-submit, retention,
assignment export, and QTI import. Reserved Render and generic Import variants have no production
producer/committer pair and remain unclaimed instead of being routed to a placeholder.

The binary now has an explicit `--worker` mode. It verifies the database schema before any claim,
processes one job per bounded pass, observes `SIGTERM`/interrupt between claims without dropping an
active preparation future, redacts operational errors to classifications, and reports only outcome
counters plus supported-family depth. Compose runs this mode as a separate service with database
and object-store access only; it receives no identity, grader, public-URL, or renderer settings.
The live PostgreSQL oracle proves two concurrent filtered workers claim distinct supported jobs,
an older reserved job remains ready, and another family's expired-lease cleanup leaves it
untouched. The complete six-migration/RLS gate remains green.

## README checkpoint

The root README no longer describes an M0 stub. It now states the active, non-production-ready
status before onboarding; names the implemented Rust, browser, Wasm, PostgreSQL, adapter, export,
manual-grading, item-analysis, and container boundaries; preserves the server-only grading and
Course/content guarantees; explains one assignment-to-analysis flow; uses unfrozen success output;
and routes readers to current evidence. Remaining database operations gates stay visible as
adoption blockers; the worker row now records the active six-family production registry and its two
reserved variants.

## Score precision and display

Scoring continues to use `f64`; it does not impose fixed-point arithmetic on Rust, WebAssembly, or
browser calculation. One Rust helper now rounds computed points and completed-run ratios to four
decimal places before current-state persistence, including both recalculation workers. Exact
midpoints round away from zero and negative zero is canonicalized.

The browser uses one reusable formatter for gradebook percentages and learner feedback. It shows at
most two decimal places, trims trailing zeroes, and renders values such as `8 / 10`, `8.5 / 10`, and
`8.33 / 10` without binary artifacts. Matching positive, negative, midpoint, and artifact cases are
permanent Rust and TypeScript tests.

## Independent audit

Six fresh reviewers independently audited plan conformance, tests, style, documentation,
legacy/dead code, and comments. The integrated fixes were:

- course membership removal/demotion now owns accommodation recomputation and active timing updates;
- direct-student exceptions and group memberships are covered by retention fences, broker policies,
  purge order, residual assertions, and MemoryStore cleanup;
- MemoryStore now matches PostgreSQL by rejecting movement of an existing group ID between courses;
- the PostgreSQL acceptance fixture now exercises combined student/group resolution, recorded
  attempt policy, membership-triggered immediate auto-submit, and exception cleanup; and
- the assignment advisory-lock and deterministic multi-assignment lock order are documented.

No fragile tests, additional dead code, or stale execution-workstream comments were found.

The earlier audit finding about an inactive, unfiltered production worker is closed by the
family-filtered complete registry, dedicated process/container mode, bounded shutdown, and live
concurrency oracle described above. Reserved families remain explicit rather than receiving fake
handlers.

## Migration safety evidence

Live PostgreSQL acceptance was rerun after the final schema and Store changes on a newly created,
empty disposable database:

1. pre-migration status reported an absent ledger and exactly six pending migrations;
2. all six migrations applied successfully;
3. a second migration run completed without applying another migration;
4. status reported all six exact versions/checksums applied and compatible;
5. verify reported the application compatible;
6. two filtered queue claimants atomically leased distinct supported jobs while reserved work and
   another family's expired lease remained untouched; 260,000 synthetic attempts and the other
   three activity families each pruned to exactly one requested monthly child, while a 60,000-row
   application-role gradebook fixture read a bounded page from current summaries only; the
   production PostgreSQL Store then completed catalog, assignment, scoring-generation,
   QTI partial-import persistence,
   concurrent-submission, Delete and Regrade, force-submit, clear, base timing,
   stale-generation rescheduling, student/group exception, membership-removal, cleanup, and mixed
   automatic/manual generation-fenced scoring behavior, then published only the corrected current
   course item analysis after rejecting a stale prepared generation; and
7. final verification remained compatible after live behavior.

Direct inspection of the exception case found one auto-submitted attempt with a submission
timestamp, zero submission rows, zero evaluation rows, zero score rows, only the direct-student
resolution after group membership disappeared, and zero remaining exception or group-member rows.

This is still a pre-data baseline. Once any environment accepts real durable data on this epoch,
these applied migrations must never be edited in place; every later change must be a new forward
migration. The disposable acceptance container/database was removed after final validation.

## Permanent validation evidence

- `./check_codebase.sh`: all 11 stages passed.
- Browser/Node contract suite: 167 passed.
- Store unit suite with PostgreSQL features: 51 passed, with the live serialization test ignored
  outside its disposable database; the four dedicated worker-filter/QTI/grading/analysis tests are
  also ignored outside the disposable runner.
- Store conformance suite: 20 passed.
- Server unit suite: 147 library and 1 binary-mode test passed.
- Repository Python hygiene suite: 1,342 passed; the prior two pending
  flat-question Markdown links now resolve, and the naming-migration boundary
  and scale tests passed 8 of 8.
- Rust formatting, strict workspace Clippy, workspace tests, and doctests: passed.
- TypeScript generation, fixtures, type checking, linting, formatting, and Node tests: passed.
- Full debug build produced the API workspace, WebAssembly web/node bridges, generated TypeScript
  contracts, and Solid bundle under the documented artifact directories.
- Mounted gradebook and learner-feedback Playwright acceptance: 6 passed.
- Mounted flat-question editor Playwright acceptance: 2 new tests passed;
  the 7 existing generic editor tests also passed (9 editor acceptance tests
  total). The focused flat editor fixture covers authoring, protected
  save/reload conflict recovery, review/publish, keyboard controls, and 375 px
  reflow without asserting a deployed authenticated browser session.
- Fresh migration, second no-op, status, verify, checksum mutation, real-role RLS, deterministic
  partitions, exact one-month pruning, bounded current-summary gradebook planning, serialization
  retry, concurrent family-filtered queue claims, QTI partial import, mixed automatic/manual
  scoring, and course item analysis: passed.
- One-time isolated Course/Student-record purge validation through the production worker and typed object store:
  passed. The populated learner graph and student-record object were absent; the assignment,
  instructor membership, published catalog/version/source, workspace draft, and anonymous global
  statistics remained. Its temporary reconstruction harness was removed after evidence capture.
- One-time encrypted logical backup/restore into a separate empty PostgreSQL 17 cluster: passed.
  The restored six-migration ledger, logical fingerprint, role attributes without password hashes,
  function owners, grants, forced RLS, exact-resource isolation, application write, and broker call all
  matched the source contract. Backup and restore each took one second for the small fixture; this
  does not claim deployed managed PITR, object-store recovery, or a production recovery objective.
- Maintained local whole-system runner: 3 passed, 0 failed. The Wasm bridge, complete disposable
  PostgreSQL acceptance suite, and two-replica learner path all passed. The live path stopped the API
  container that issued the question, reproduced the exact envelope on the surviving replica,
  committed and replayed one idempotent submission, and found exactly one scoped attempt,
  submission, idempotency, evaluation, and current-score row. Exact project containers, networks,
  volumes, and temporary local identities were removed; the pre-existing `pg-test` remained alone.

## Remaining implementation order

The exact authoritative sequence is WP-RC1 through WP-RC12 in
`docs/active_plans/active/release_completion_plan.md`:

1. WP-RC1 course appearance is accepted.
2. WP-RC2 production-seam closure is accepted; WP-RC3 next integrates pinned upstream WeBWorK
   `/render_rpc`; WP-FQ-0 proceeds in QTI Package Maker.
3. WP-RC4 through WP-RC7 deliver eight families, two Chapter 1 assignments, QTI/H5P close-out,
   object reconciliation, and the combined M2-M5 gate.
4. WP-RC8 through WP-RC12 deliver OIDC, LTI, OpenTofu, bot-cost controls, managed recovery, and
   working-codebase release acceptance.
