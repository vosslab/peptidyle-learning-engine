# Plan: bounded QTI profile mappings

Status: WP-QTI-9 server routes and WP-QTI-10 author UI are complete and independently accepted.
The author UI has real-route Chromium evidence for upload, review, conversion, recovery, keyboard,
and 375 px reflow. WP-QTI-11 is next and remains unstarted: it is the independent live
PostgreSQL/RLS/profile-to-native gate. The frozen course appearance plan remains a separate later
package.

## Context

PLE now has two complementary foundations:

- a hostile-input QTI archive pipeline that preserves the exact ZIP, bounds resource use, records
  unsupported items, and keeps grading material behind a server-only capability;
- a canonical PLE flat-question JSON format that atomically saves, publishes, edits, renders, and
  grades ordinary static single-choice questions without putting answers in learner contracts.

The next package connects those foundations without making vendor XML an internal source format.
Canvas QTI 1.2 and Blackboard QTI 2.1 are materially different dialects. The current generic QTI
parser accepts a bounded QTI 2-style `assessmentItem` shape, but it does not recognize either vendor
profile, construct canonical flat-question source, preserve a vendor-to-PLE choice identity map, or
link a derived flat publication to the archived package. It is also currently labeled
`qti-1.2-subset` even though its accepted root and fixture grammar are QTI 2-style.

The corpus audits behind this plan inspected actual local Canvas and Blackboard packages, not only
standards documents:

- [flat_question_package_implementation.md](../workstreams/flat_question_package_implementation.md)
  records the completed canonical source and runtime boundary.
- [flat_question_editor_implementation.md](../workstreams/flat_question_editor_implementation.md)
  records the completed authoring surface.
- [implementation_plan.md](../implementation_plan.md) requires QTI to remain an adapter and retain
  exact original packages.
- [partial_commit_status.md](../partial_commit_status.md) names bounded Canvas and Blackboard
  mappings as the next package.
- [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) keeps answers server-only and PLE flat-question JSON
  canonical.
- [QTI-JSON_OBJECT_FORMAT.md](../../QTI-JSON_OBJECT_FORMAT.md) defines the closed v1 source contract.

## Objectives

- Recognize two exact, versioned vendor profiles instead of guessing from XML element names.
- Convert only semantically supported static single-choice items into canonical PLE flat-question
  JSON, then use the existing native compiler, persistence, editor, publication, and runtime.
- Preserve the exact original ZIP and an immutable provenance link from a derived publication back
  to its package, item, profile, mapping version, and checksums.
- Give instructors actionable accepted/rejected results and explicit defaults before conversion.
- Keep partial package success: one unsupported item does not erase supported siblings.
- Keep every answer, feedback value, vendor choice map, archive key, and canonical source byte out of
  learner, catalog, ordinary browser, generated API, and Wasm contracts.
- Add import and conversion acceptance first. Add profile export only after import semantics and
  provenance pass independent review.
- Keep new Rust and TypeScript owners small and named by responsibility.

## Design philosophy

This package follows the repository's stated engineering philosophies:

- **Focus on important issues.** Preserve grading meaning, source evidence, answer secrecy, and
  tenant isolation before expanding the number of accepted XML constructs.
- **Fix the design, not the symptom.** Add explicit profiles and a native conversion boundary rather
  than adding Canvas and Blackboard special cases to the generic parser.
- **Scientific method.** Derive each accepted shape from a retained fixture, state the hypothesis,
  and require executable evidence that unsupported semantics refuse without mutation.
- **Long-term and adaptability.** Version profiles, conversion rules, choice mappings, and
  provenance so future parser improvements can re-import the unchanged archive.
- **Positive prompting.** Author-facing results say what was recognized, what can be converted, why
  an item was refused, and what the instructor can do next.
- **Compartmentalization.** Profile parsers, markup conversion, choice identity, flat conversion,
  provenance persistence, HTTP orchestration, and UI each have a focused owner.

## Scope

- Canvas QTI 1.2 static single-choice import profile.
- Blackboard Original QTI 2.1 static single-choice pool import profile.
- Honest relabeling of the existing generic QTI 2-style subset.
- Deterministic vendor-profile detection from manifest evidence.
- Safe bounded XML/HTML-to-Markdown projection for a deliberately small text subset.
- Deterministic collision-safe choice ID mapping.
- Server-only conversion into native flat-question JSON.
- Explicit author review of warnings and PLE defaults before conversion.
- Workspace and immutable published provenance in Memory and PostgreSQL.
- Forced RLS, grants, retention, typed object keys, and non-signable archive copies.
- Author API and focused import/review/convert UI.
- Permanent adapter, Store, server, TypeScript, and Playwright tests.
- One disposable PostgreSQL profile-to-flat oracle in the maintained database gate.
- Later Canvas and Blackboard profile exporters, after the import gate passes.

## Non-goals

- General QTI conformance.
- Automatic repair of unknown or malformed packages.
- Multiple-answer, true/false as a distinct family, matching, ordering, numeric, essay, file,
  hotspot, calculated, or external-tool questions.
- Partial, negative, additive, mapped, branched, or outcome-variable scoring.
- Importing test-level attempts, timers, sequencing, pools, groups, randomization, or feedback policy.
- Images, tables, MathML, SVG, media, embedded objects, external URLs, styles, or scripts in v1.
- Guessing feedback meaning from `itemfeedback`, `modalFeedback`, Canvas comments, or arbitrary
  response processing.
- Silently inventing titles, licenses, language, points, or pedagogy.
- Treating vendor XML or YAML as canonical PLE source.
- Browser-side ZIP or XML parsing.
- Replacing the completed legacy QTI publication/runtime path during this package.
- Claiming byte-identical vendor round trips. Later export promises semantic round trips only.

## Current state summary

### Existing strengths

- `crates/adapters/qti/src/parser_stub.rs` enforces 32 MiB archive, 128 MiB expanded, entry,
  per-file, XML-depth, token, and node limits before conversion.
- The parser rejects unsafe paths, symlinks, DTDs, entities, malformed XML, tables, MathML, and
  unsupported interactions.
- `crates/server/src/qti_import.rs` writes exact source and extracted assets before committing a
  private import registry.
- `crates/learning-data-access/src/qti.rs` separates answer-free import results from the dedicated
  `QtiGradingStore` capability.
- Memory and PostgreSQL persist committed imports, per-item results, warnings, assets, and private
  grader values under tenant scope.
- `crates/server/src/qti_publication.rs` re-reads the exact archive and validates the selected item
  before publishing the existing QTI backend.
- `adapter_native::flat_question::FlatQuestionDocument` already canonicalizes and compiles PLE
  flat source into answer-free and grader-only values.
- The flat-question HTTP and Store paths already provide atomic draft/source/private persistence,
  immutable publication, protected author reload, and native runtime grading.

### Material gaps

- The current parser accepts only a generic QTI 2-style `assessmentItem` shape.
- Real Canvas packages use QTI 1.2 `questestinterop` and `response_lid`.
- Real Blackboard packages use QTI 2.1 package paths and response processing that must be checked,
  not ignored.
- Current package path allowlists reject the observed Canvas and Blackboard item directories.
- No profile ID, profile version, detector, profile-specific diagnostic, or fixture owner exists.
- The current `qti-1.2-subset` label is inaccurate.
- No server-only type carries a complete mapped static question into the native flat compiler.
- No API uploads a QTI ZIP, exposes safe results, or converts one accepted item into a flat draft.
- No current or published relation links flat source to its archived import origin.
- No exporter exists.

## Architecture boundaries and ownership

### Component map

| Component                | Owner                                                     | Responsibility                                                                         |
| ------------------------ | --------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| QTI archive safety       | `adapter_qti::archive`                                    | ZIP limits, paths, media sniffing, XML resource bounds                                 |
| Profile identity         | `adapter_qti::profiles`                                   | Closed profile IDs, versions, detection evidence, diagnostic codes                     |
| Canvas profile           | `adapter_qti::profiles::canvas`                           | Exact Canvas 1.2 recognition and static-item mapping                                   |
| Blackboard profile       | `adapter_qti::profiles::blackboard`                       | Exact Blackboard 2.1 pool recognition and static-item mapping                          |
| Markup projection        | `adapter_qti::profiles::markup`                           | Allowlisted text/XHTML to deterministic Markdown                                       |
| Choice identity          | `adapter_qti::profiles::choice_ids`                       | Stable vendor ID to PLE ID mapping and collision handling                              |
| Native import factory    | `adapter_native::flat_question::imported`                 | Construct and validate canonical flat v1 from trusted mapped fields                    |
| Conversion orchestration | `server_core::qti_profile_conversion`                     | Re-read archive, verify report digest, compile, copy object, call atomic Store command |
| Upload/report API        | `server_core::qti_profile_import`                         | Protected raw ZIP upload, job status, safe result projection, no-store responses       |
| Provenance contract      | `learning_data_access::flat_import_provenance`            | Current origin, publication promotion, and read-free validation types                  |
| Memory backend           | `learning_data_access::in_memory::flat_import_provenance` | Atomic conversion and publication parity                                               |
| PostgreSQL backend       | `learning_data_access::postgres::flat_import_provenance`  | Short transactions, RLS-safe writes, exact promotion validation                        |
| Schema                   | catalog/operations/retention migrations                   | Tables, functions, policies, grants, fences, purge order                               |
| Object boundary          | `objects` typed keys                                      | Non-signable imported and published provenance archives                                |
| Author UI                | `src/features/qti_profile_import/`                        | Upload, status, results, warning review, convert, editor handoff                       |
| Profile export           | `adapter_qti::export_profiles`                            | Later lossless subset export after import acceptance                                   |

No profile module imports a persistence or HTTP crate. `adapter_qti` and `adapter_native` do not
depend on one another. The server depends on both and performs the narrow translation. The browser
never receives profile parser types or answer-bearing mapped items.

### Dependency direction

```text
vendor ZIP
    |
    v
adapter_qti profile parser -----> safe item results and diagnostics
    |                                      |
    | server-only mapped item              v
    +----------------------------> import registry
    |
    v
server conversion -> adapter_native imported factory -> canonical flat JSON
    |                                                |
    v                                                v
immutable import provenance                 existing flat persistence/runtime
```

### Mapping

| Workstream           | Primary components                                  | Produces                                        |
| -------------------- | --------------------------------------------------- | ----------------------------------------------- |
| WS-QTI-1 contracts   | profiles, diagnostics, fixtures                     | Closed profile and refusal contract             |
| WS-QTI-2 Canvas      | canvas, markup, choice IDs                          | Canvas accepted/rejected mapped items           |
| WS-QTI-3 Blackboard  | blackboard, markup, choice IDs                      | Blackboard accepted/rejected mapped items       |
| WS-QTI-4 flat bridge | native imported factory, server conversion          | Canonical source and public/private compilation |
| WS-QTI-5 provenance  | Store contract, Memory, PostgreSQL, schema, objects | Current and immutable origin chain              |
| WS-QTI-6 API/UI      | server routes, TS feature, Playwright               | Visible instructor import and recovery flow     |
| WS-QTI-7 export      | export profiles, archive writer                     | Refusing, versioned semantic export             |
| WS-QTI-8 acceptance  | live oracle, security scan, independent review      | Release evidence                                |

## Resolved decisions

### Profile identities

- Canvas: `canvas-qti-1.2-static-single-choice/v1`.
- Blackboard: `blackboard-qti-2.1-static-single-choice-pool/v1`.
- Existing generic parser: `ple-qti-assessment-item-single-choice/v1`.
- Profile IDs and conversion versions are persisted data. Renaming one requires an explicit reader
  alias and migration test.

### Detection

- Detection reads only bounded manifest evidence after archive safety checks.
- Archive handling separates path traversal/symlink/resource limits from each profile's entry
  allowlist. A profile never widens the generic safe-path rules merely to reach a vendor directory.
- Both profiles require the IMS Content Packaging manifest namespace
  `http://www.imsglobal.org/xsd/imscp_v1p1`.
- Canvas requires resource type `imsqti_xmlv1p2`, a package-relative XML `href` beneath
  `canvas_qti12_questions/`, and item namespace
  `http://www.imsglobal.org/xsd/ims_qtiasiv1p2` with root `questestinterop`.
- Blackboard requires manifest `schema=QTIv2.1`, resource type `imsqti_item_xmlv2p1`, item paths
  beneath `qti21_items/`, and item namespace `http://www.imsglobal.org/xsd/imsqti_v2p1` with root
  `assessmentItem`.
- Ambiguous vendor evidence rejects the profile conversion with an actionable package diagnostic.
- Unknown packages may continue through the existing generic compatibility path, but they cannot be
  converted to flat source under a vendor profile.

### Profile matrix authority

WP-QTI-1 freezes a checked-in profile matrix before either vendor parser lands. Each row names:

- manifest namespace, metadata values, resource type, `href` grammar, and permitted dependency;
- item namespace/root and XPath-like element/attribute cardinality predicates;
- source field for title, prompt, ordered choices, correct response, points, and feedback;
- accepted value normalization and duplicate/missing-value behavior;
- stable rejection code and whether refusal is package-level or per-item;
- one minimized positive fixture and one near-miss fixture.

The initial diagnostic vocabulary is closed to `profile-ambiguous`, `manifest-namespace`,
`manifest-schema`, `resource-type`, `resource-path`, `unexpected-entry`, `item-namespace`,
`item-shape`, `question-type`, `response-cardinality`, `choice-count`, `duplicate-choice-id`,
`correct-response`, `response-processing`, `points`, `feedback`, `markup`, `media`, `shuffle`, and
`policy`. Adding a code or changing a matrix predicate requires a profile version or an explicitly
compatible parser patch with a regression fixture.

| PLE field      | Canvas source                                         | Blackboard source                                |
| -------------- | ----------------------------------------------------- | ------------------------------------------------ |
| title          | `item/@title`                                         | `assessmentItem/@title`                          |
| prompt         | `presentation/material/mattext` before `response_lid` | `itemBody` content excluding `choiceInteraction` |
| choices        | ordered `response_label/material/mattext`             | ordered `choiceInteraction/simpleChoice` content |
| correct choice | sole accepted `varequal` target                       | sole `responseDeclaration/correctResponse/value` |
| points         | required `qtimetadata` `points_possible`              | explicit PLE default `1.0` with warning          |
| feedback       | unsupported in v1                                     | unsupported in v1                                |

### Canvas v1 acceptance

- Exactly one `questestinterop/assessment/section` container with one or more `item` candidates;
  each item succeeds or refuses independently.
- `question_type` exactly `multiple_choice_question`.
- Exactly one `response_lid` with `rcardinality="Single"` and one `render_choice`.
- Between 2 and 100 unique response labels.
- Exactly one `varequal` targeting that response and one `setvar` setting `SCORE` to 100.
- Finite nonnegative `points_possible` is required.
- `original_answer_ids`, when present, must exactly match response-label order. It is consistency
  evidence, never the answer source.
- Any other response processing, feedback construct, media, style, table, or semantic extension
  rejects the item.

### Blackboard v1 acceptance

- Blackboard Original QTI 2.1 pool package with item resources.
- One `assessmentItem` per candidate, with `adaptive=false` and `timeDependent=false`.
- One identifier/single response declaration and exactly one correct value.
- One `choiceInteraction`, `maxChoices=1`, and 2 to 100 unique `simpleChoice` values.
- `shuffle=false` or absent is accepted.
- `shuffle=true` is accepted only when every choice is `fixed=true`, making the order effectively
  static. Any real shuffle rejects.
- Response processing is absent or exactly the observed no-extra-semantics
  `match(variable RESPONSE, correct RESPONSE)` form.
- Any outcome write, mapping, branch, partial score, negative score, custom operator, feedback,
  media, table, style, or extension rejects the item.
- Pool/test structure is retained as package provenance but does not become assignment policy.

### Markup v1

- Normalize entity-decoded text and line endings deterministically.
- Accept plain text plus `p`, `div`, `br`, `strong`, `b`, `em`, `i`, `code`, `ul`, `ol`, and `li`
  with no semantic attributes.
- Convert accepted nodes to a documented CommonMark subset with stable whitespace rules.
- Reject `table`, `style`, `class`, `id`, event attributes, script, iframe, object, SVG, MathML,
  image, audio, video, data URLs, external URLs, and all unknown elements or attributes.
- Defer `sub`, `sup`, and underline until the learner renderer has an exact, separately tested
  round-trip representation. Do not strip them.

### Choice IDs

- Preserve a vendor ID that already satisfies PLE flat v1 choice-ID rules.
- Otherwise derive `qti_` plus a lowercase SHA-256 prefix over profile ID, item identifier, and raw
  vendor ID.
- Extend the digest prefix deterministically when the candidate collides with a preserved or derived
  ID. Refuse only if the 64-byte PLE limit cannot resolve the collision.
- Persist the ordered vendor-to-PLE map only in private import provenance. Never slugify, use display
  labels such as A/B/C, or silently reorder choices.

### Defaults and review

- A nonblank vendor title is required. No filename or identifier title fallback is invented.
- Canvas points must be present and exact.
- Blackboard v1 defaults missing item points to `1.0` and records an explicit warning.
- Imported flat questions default to unlimited attempts, `immediateFull` feedback, untimed,
  `en-US`, `allRightsReserved`, empty tags, empty taxonomy, and no feedback.
- These are PLE authoring defaults, not vendor semantics.
- Every default contributes to a stable warning digest. Conversion requires the instructor to
  acknowledge the exact current digest. A changed report invalidates the acknowledgement.
- The converted source opens in the existing flat editor before publication.

### Import integrity digests

Import commit persists integrity evidence before any conversion is possible:

- `profile_report_sha256` covers canonical safe detector evidence, profile ID/version, mapping
  version, ordered item dispositions, stable diagnostics, defaults, and per-item mapping digests;
- `public_mapping_sha256` covers source location/identifier, title, prompt, ordered PLE IDs and
  choice text, points, PLE defaults, and warnings for one accepted item;
- `private_mapping_sha256` covers the correct source/PLE choice binding, private ordered vendor-ID
  map, and any future profile-mapped feedback without persisting those bytes in a safe DTO;
- `mapping_sha256` binds the public and private mapping digests plus profile and mapping versions;
- `warning_sha256` covers the exact author-visible defaults and warnings.

The worker computes these from one canonical, versioned encoding and stores them with the committed
registry/result. Conversion re-reads the exact archive, re-detects the profile, remaps the selected
item, and requires all stored digests to match. An adapter upgrade that changes semantics therefore
refuses conversion until the package is explicitly re-imported and reviewed; matching warning text
alone is never sufficient.

The browser receives only a safe `reportRevision` derived from the safe report and a `reviewToken`
derived from the visible warnings/defaults. It does not receive the archive digest, canonical source
digest, private mapping digest, choice map, or answer. The Store validates the full server-side
digest set even when the browser acknowledgement is current.

### Feedback

- v1 maps no vendor feedback because the retained fixtures do not establish a single unambiguous
  semantic contract.
- Any feedback construct rejects that item with a specific diagnostic.
- A later profile version may add one exact fixture-backed mapping without changing v1.

### Import versus export

- Import, conversion, provenance, and visible review land first.
- Export is a later milestone, not a release claim for the import package.
- Export refuses any PLE field the target profile cannot preserve; it never drops feedback,
  policies, metadata, markup, points, or semantics silently.
- Exported packages are new artifacts. The original imported archive remains unchanged.

## Milestone plan

### Milestone: Q1 contracts and fixtures

- Depends on: completed flat-question package and QTI import-hardening package.
- Deliverables: exact profile IDs, honest generic label, profile result types, diagnostics,
  minimized readable fixtures, and package detector.
- Workstreams: WS-QTI-1, with independent Canvas and Blackboard fixture preparation.
- Entry criteria: current adapter and Store tests pass.
- Exit criteria: the exact namespace/path/structure/field/rejection matrix is committed; every row
  has a minimized positive and near-miss fixture; detector and report/mapping digests are
  deterministic; the generic parser no longer claims QTI 1.2.
- Parallel-plan ready: yes. Maximum useful doers: 3.

### Milestone: Q2 profile parsers

- Depends on: Q1 frozen profile and diagnostic contracts.
- Deliverables: Canvas and Blackboard parsers, shared markup projector, deterministic choice mapping,
  safe result projection, and private mapped item type.
- Workstreams: WS-QTI-2 and WS-QTI-3 in parallel; shared helpers have one owner.
- Entry criteria: fixtures and diagnostic codes committed.
- Exit criteria: supported fixtures map deterministically; every excluded semantic has a stable code,
  location, and actionable detail; partial package success works.
- Parallel-plan ready: yes. Maximum useful doers: 3.

### Milestone: Q3 pure flat mapping

- Depends on: Q2 and the existing flat compiler contract.
- Q2 is complete: Canvas QTI 1.2 and Blackboard Original QTI 2.1 static-choice
  mappings passed their focused/full gates and independent P0/P1 reviews.
- Q3 is complete: the native imported factory and crate-private server bridge
  passed exact Canvas/Blackboard-to-hand-authored canonical equivalence gates
  without Store or HTTP mutation.
- Deliverables: native imported-single-choice factory, server-only type translation, canonical-byte
  equality tests, and split public/private compile tests. No Store or HTTP mutation lands here.
- Workstreams: WS-QTI-4.
- Entry criteria: mapped item contains all fields needed for a canonical document.
- Exit criteria: the pure bridge produces the exact canonical source and split public/private
  material for a hand-authored equivalent fixture.
- Parallel-plan ready: yes. Maximum useful doers: 2.

### Milestone: Q4 provenance foundation

- Depends on: Q2 digest contract and Q3 canonical mapping.
- Deliverables: import-time report/mapping digests, provenance contract, distinct typed object key,
  workspace/published table shape, publication promotion shape, lock order, RLS/grants, retention,
  and object reconciliation policy.
- Workstreams: WS-QTI-5 contract, object, and schema lanes. Backend mutation waits for these shapes.
- Entry criteria: canonical profile report and mapping encodings are frozen.
- Exit criteria: Store types, object identity, migration shape, transaction sequence, and lifecycle
  are complete enough that no conversion can persist a draft without its current origin.
- Parallel-plan ready: yes. Maximum useful doers: 3 after the contract owner finishes.

### Milestone: Q5 atomic conversion and author workflow

- Depends on: Q3 pure mapping and Q4 durable provenance foundation.
- Deliverables: Memory/PostgreSQL atomic conversion, publication provenance copy, raw ZIP upload,
  safe report, conversion endpoint, focused UI, and editor handoff.
- Workstreams: WS-QTI-5 backend lanes followed by WS-QTI-6 server and TypeScript lanes.
- Entry criteria: Memory route fixture can produce accepted and rejected profile results.
- Exit criteria: an instructor visibly uploads, reviews, acknowledges, converts, edits, and reaches
  the flat publication review; draft, source, private compiler material, current origin, and CAS
  revision commit together; errors preserve input and explain recovery; no learner/public payload
  contains answer or archive material.
- Parallel-plan ready: yes. Maximum useful doers: 4 after backend parity passes.

### Milestone: Q6 integrated acceptance

- Depends on: Q1 through Q5.
- Deliverables: disposable PostgreSQL profile-to-flat oracle, security scan, behavior gates,
  independent review, and implementation handoff.
- Workstreams: WS-QTI-8.
- Entry criteria: focused profile, Store, server, and UI gates pass.
- Exit criteria: real profile fixture reaches native flat runtime grading; rejected sibling remains
  visible; exact source archive and published provenance agree; RLS and secrecy probes pass; no P0 or
  P1 remains.
- Parallel-plan ready: yes. Maximum useful doers: 2, with review independent of implementation.

### Milestone: Q7 profile export

- Depends on: Q6 import acceptance and explicit exporter fixtures.
- Deliverables: Canvas 1.2 and Blackboard 2.1 pool exporters for the exact lossless subset, semantic
  re-import tests, downloadable artifacts, and refusal UI.
- Workstreams: WS-QTI-7 Canvas and Blackboard lanes.
- Entry criteria: supported Markdown and policy subset frozen; vendor import fixtures accepted.
- Exit criteria: PLE to vendor to PLE preserves title, prompt, ordered IDs, correct choice, points,
  and all permitted defaults; unsupported PLE fields refuse before object creation.
- Parallel-plan ready: yes. Maximum useful doers: 3.

## Workstream breakdown

### WS-QTI-1: contracts

- Extract reusable archive/XML safety helpers from the 976-line parser without changing behavior.
- Add closed `QtiProfileId`, profile version, detection evidence, and diagnostic code types.
- Commit the exact namespace/path/structure/field/rejection profile matrix and one positive plus
  near-miss fixture for every row before a vendor parser is accepted.
- Persist `profile_report_sha256`, per-item public/private/combined mapping digests, and warning
  digest in the committed import registry before any conversion work begins.
- Replace inaccurate `qti-1.2-subset` constants with the honest generic profile ID.
- Add reader aliases only if a current fixture or persisted baseline requires them.
- Keep every new production source owner under 1,000 lines, with a target under 600.

### WS-QTI-2: Canvas

- Parse the exact manifest/resource/item hierarchy.
- Validate `original_answer_ids` only as ordered consistency evidence.
- Validate the simple 100-point response-processing shape.
- Require exact points and title.
- Project accepted markup, ordered choices, and correct vendor choice into the private mapped type.
- Record every unsupported feature per item; retain the exact archive.

### WS-QTI-3: Blackboard

- Parse exact QTI 2.1 pool resources and static item declarations.
- Treat all-fixed shuffle as static, while refusing actual shuffle.
- Accept only absent or exact no-op match/correct response processing.
- Default missing points to 1.0 with review-required warning.
- Retain pool metadata as provenance only.
- Reject tables, styles, extensions, scoring variants, feedback, and test policy.

### WS-QTI-4: flat bridge

- Add `adapter_native::flat_question::imported` with a bounded trusted input type.
- Construct `FlatQuestionDocument` through native-owned validation; do not duplicate its parser or
  serializer.
- Canonicalize and compile in a pure test owner with no object, Store, or HTTP mutation.
- Prove equality with the canonical bytes and public/private split of an equivalent hand-authored
  flat source.

### WS-QTI-5: provenance

- Add `workspace_flat_import_origin`, current per workspace, bound to a committed import.
- Persist profile ID/version, source item identifier, archive object/checksum, normalized item
  checksum, report/public/private/combined mapping digests, conversion version, mapped canonical
  checksum, warning digest, acknowledging actor/time, and private ordered choice-map checksum/payload.
- Preserve the origin through later flat editor saves in the same workspace; a different import
  conversion atomically replaces it under compare-and-swap.
- Add tenant-owned `published_flat_import_origin`, immutable per problem/version and hidden from
  other tenants even when the answer-free version is public.
- Add the distinct non-signable object key
  `PublishedImportArchive { tenant, problem, version, import, object }`. Derive its object identity
  from SHA-256 of the fixed v1 domain separator plus tenant, problem, version, import, and archive
  digest, truncated through the repository's deterministic UUID rule.
- Copy the original ZIP to that exact candidate before catalog commit. Exact `AlreadyExists` replay
  re-reads and accepts only identical key, category, media type, size, and digest.
- Extend flat publication promotion so Store copies provenance only from the current trusted
  workspace relation. Browser input cannot manufacture or edit it.
- Current origin holds a restrictive reference that pins its committed workspace import. CAS
  replacement releases the prior pin only after the new origin commits and no current origin refers
  to the old import. Workspace cleanup may then remove the old staging archive under normal policy.
- Published origin and its archive remain with the problem version and survive workspace staging
  cleanup. A failed database publication leaves only a typed candidate orphan, which the existing
  quarantine/reconciliation policy may remove after proving no provenance binding exists.
- Add forced RLS, least grants, exact foreign keys, binding triggers, retention fences, purge order,
  and residual assertions.
- Add no speculative secondary indexes. Primary keys and required foreign-key support indexes land;
  other indexes require measured queries.

### WS-QTI-6: API and UI

- Add `PUT /api/workspaces/{workspace}/qti-imports/{import}` for raw `application/zip` bytes.
- Require author authentication, a UUID import identity, 32 MiB body limit, exact replay, divergent
  replay conflict, no-store responses, and typed workspace-source object storage.
- Enqueue only the complete `qtiImport` worker family.
- Add `GET /api/workspaces/{workspace}/qti-imports/{import}` for a bounded answer-free report.
- Define that report DTO explicitly as `importId`, state, profile ID/label/version, `reportRevision`,
  ordered items with source identifier/title/status, stable code/location/detail warnings, stated PLE
  defaults, and `reviewToken`. It excludes `ObjectRecord`, object IDs/keys, archive and canonical
  checksums, raw XML/ZIP, canonical source, choice maps, correct choice, feedback, and grader bytes.
- Add `POST /api/workspaces/{workspace}/qti-imports/{import}/items/{item}/convert-flat` with strong
  draft ETag when a draft exists plus exact `reportRevision` and `reviewToken` acknowledgement.
- Add `server_core::qti_profile_conversion` only after WS-QTI-5 lands. It re-reads the exact archive,
  repeats detection/mapping, compares every stored report/mapping digest, compiles canonical source,
  copies the workspace source object, and invokes the single atomic provenance-aware Store command.
- Return only the existing answer-free draft DTO plus strong revision ETag.
- Use uniform non-enumerating 404 behavior for inaccessible workspace/import/item relationships.
- Add `src/features/qti_profile_import/` with upload, progress, recognized-profile summary,
  accepted/rejected item cards, warning review, conversion, conflict recovery, and flat-editor handoff.
- Do not parse ZIP/XML, cache archive bytes, or show answer mappings in TypeScript.

### WS-QTI-7: export

- Add one exporter per profile; share only safe archive/XML writers and Markdown conversion.
- Emit profile ID/version and canonical PLE source checksum in package provenance.
- Generate stable vendor-safe response IDs from PLE semantic IDs.
- Refuse non-default policies, unsupported markup, feedback, metadata, or points that cannot be
  represented exactly.
- Preserve choice order and correct binding through semantic re-import tests.
- Store exports as new typed export objects; never overwrite imported archives.

### WS-QTI-8: acceptance

- Add one integrated PostgreSQL live test using a minimized actual-profile fixture.
- Include one accepted and one rejected item where the selected vendor profile permits batching.
- Convert the accepted item, edit or review it, publish as native flat, and grade correct/incorrect
  responses through the isolated flat grader.
- Prove the original archive, current origin, immutable origin, canonical source, and checksums agree.
- Probe `ple_app`, `ple_student`, grader roles, foreign tenant, and direct table/object access.
- Scan every safe DTO and serialized report for correct choice, feedback, archive bytes, object keys,
  grader payload, and private choice maps.
- Run an independent P0/P1 review after executable gates pass.

## Work packages

### WP-QTI-1 profile contract

- Owner: QTI adapter contract owner.
- Depends on: none beyond current green adapter tests.
- Files: adapter profile facade/model, exact profile matrix, generic labels, focused tests.
- Acceptance: closed IDs, deterministic detection, canonical report/mapping digest contract, one
  positive and near-miss fixture per matrix row, and no behavior regression in generic importer.

### WP-QTI-2 fixture corpus

- Owner: fixture/evidence owner.
- Depends on: WP-QTI-1 diagnostic names.
- Files: readable minimized Canvas and Blackboard XML/manifest fixtures plus fixture builder.
- Acceptance: fixtures trace to observed local syntax without depending on `OTHER_REPOS` at test time.

### WP-QTI-3 Canvas parser

- Owner: Canvas profile owner.
- Depends on: WP-QTI-1 and WP-QTI-2.
- Acceptance: exact positive map; table/style/feedback/scoring negatives refuse per item.

### WP-QTI-4 Blackboard parser

- Owner: Blackboard profile owner.
- Depends on: WP-QTI-1 and WP-QTI-2.
- Acceptance: exact positive pool map; all-fixed shuffle accepted; real shuffle and scoring variants
  refuse per item.

### WP-QTI-5 native factory

- Owner: native flat-question owner.
- Depends on: mapped-item contract from WP-QTI-1.
- Acceptance: canonical bytes and public/private split equal a hand-authored equivalent source.
- Status: complete; Q4/WP-QTI-6 provenance contract and object-key freeze is also complete.

### WP-QTI-6 provenance contract and object key

- Owner: data-access and object-contract owners.
- Depends on: WP-QTI-1 digest contract and WP-QTI-5 canonical mapping.
- Acceptance: current/published origin types, distinct deterministic non-signable archive key,
  promotion types, lifecycle, lock order, and one atomic Store command shape are frozen.
- Status: complete. The adapter-owned ordered choice-map bytes, storage-owned current and published
  origin types, server conversion version, fail-closed publication promotion, atomic conversion
  command, lifecycle, and lock order are frozen. `PublishedImportArchive` has a distinct
  deterministic non-signable key and golden identity. No backend or schema mutation is included.
- Evidence: [qti_provenance_contract_implementation.md](../workstreams/qti_provenance_contract_implementation.md).

### WP-QTI-7 schema and objects

- Owner: PostgreSQL/schema owner.
- Depends on: WP-QTI-6 frozen types.
- Acceptance: fresh apply/no-op/verify, tenant-leading RLS/grants/retention, no signed provenance URL,
  pin/release/published-retention behavior, and live role probe.
- Status: complete and independently reviewed on 2026-08-09. The SQL relations and protected
  capabilities bind every accepted origin to the committed import's full typed `ObjectRecord`, not
  a caller-supplied archive summary. Six tenant-owned relations hold current/published origins,
  private choice maps, and committed profile/item evidence; each has forced RLS. A dedicated
  `ple_qti_provenance_broker` is `NOLOGIN`, `NOINHERIT`, and `NOBYPASSRLS`; its narrowly granted
  `SECURITY DEFINER` functions own staging, reading, replacement, promotion, and release.
  Current lineage pins the committed import, ordinary draft cleanup releases current lineage only,
  and published lineage/map rows remain immutable and retained.
- The Rust and SQL boundaries now accept source-item identifiers of up to 1,024 Unicode scalars
  across item, result, grading, evidence, and origin rows. The live disposable oracle proves the
  1,024-scalar round trip, rejects 1,025 scalars with named constraints, exercises real roles/RLS,
  proves pin/current/published lifecycle and child-first cleanup, and runs in the maintained
  baseline gate. Evidence is recorded in
  `docs/active_plans/workstreams/qti_provenance_schema_implementation.md`.

### WP-QTI-8 Memory and PostgreSQL

- Owner: separate backend owners.
- Depends on: WP-QTI-6 and WP-QTI-7.
- Acceptance: one provenance-aware atomic conversion commits CAS revision, draft, canonical source,
  private compiler payload, and current origin together; publication copies immutable origin;
  shared conformance and PostgreSQL feature tests pass with contract-level error parity.
- Status: complete and independently reviewed on 2026-08-09. The H2 staged-profile-evidence gap is
  closed with one non-serializable closed evidence type and exact idempotent staging while an import
  remains prepared. Conversion revalidates the committed archive, accepted result, exact
  `sourceIdentifier`/`itemId` binding, profile tuple, and digest set before mutation.
- Memory and PostgreSQL apply the frozen lock order and commit the draft CAS revision, canonical
  source, current private grading, and current origin atomically. Origin installation precedes
  grading promotion. Ordinary saves replace the current private grading value while preserving
  current origin; publication can promote only the locked stored grading value and cannot accept a
  caller-supplied secret.
- PostgreSQL uses the forced-RLS QTI provenance and grading brokers for protected operations. The
  application Store path performs no direct reads of private grading, choice-map, or provenance
  secret tables. `Sha256Digest` now serializes as and strictly accepts only lowercase 64-character
  hexadecimal text, which keeps JSON evidence typed without widening its accepted form.
- Evidence: `docs/active_plans/workstreams/qti_memory_postgres_implementation.md`.

### WP-QTI-9 server routes

- Owner: server owner.
- Depends on: WP-QTI-3 through WP-QTI-8.
- Acceptance: upload/replay/report/convert behavior, no-store, body bounds, ETag, non-enumeration,
  no mutation on refusal.
- Status: complete and independently accepted on 2026-08-09. The authenticated author route copies
  an exact bounded `application/zip` archive to a deterministic private workspace object, then
  enqueues exactly one deterministic `qtiImport` job. Exact replay returns the prior state while a
  divergent replay refuses. Its answer-free report exposes only recognized package and item defaults,
  safe diagnostics, and digest acknowledgements. The worker detects Canvas and Blackboard profiles
  strictly before the generic path, stages complete accepted-item evidence, and refuses mixed vendor
  evidence without mutation. Conversion requires the current strong draft ETag and report revision
  and acknowledgement tokens, rereads and reparses the archived package, bridges the accepted item
  through native compilation, and invokes the one atomic WP-QTI-8 Store command. Publication copies
  the exact source to the deterministic non-signable `PublishedImportArchive` object. Memory and
  PostgreSQL serialize draft deletion with prepared import work, so deletion either prevents staging
  or removes prepared staging before reuse. Every route keeps `Cache-Control: no-store`, uniform
  inaccessible/not-found behavior, and answer-free response DTOs.
- Evidence: `docs/active_plans/workstreams/qti_server_routes_implementation.md`.

### WP-QTI-10 author UI

- Owner: TypeScript/UI owner.
- Depends on: stable WP-QTI-9 DTOs.
- Acceptance: visible upload-to-editor flow, accessible warnings and recovery, no browser XML parser,
  no sensitive persistence or response fields.
- Status: complete and independently accepted on 2026-08-09. The feature-local same-origin client
  uploads opaque ZIP bytes, decodes only the answer-free report, and retains the selected archive and
  report only in component memory. The visible flow supports queued/processing refresh, mixed or
  all-rejected reports, unsupported-profile recovery, and exact retry only after an ambiguous upload.
  Conversion requires review acknowledgement, an accepted item, and the currently displayed clean
  strong revision; it uses no browser ZIP/XML parsing, archive persistence, or private answer fields.
- Successful conversion refetches the existing workspace route and focuses the existing flat editor.
  During the committed conversion/refetch handoff, the stale editor is inert. A failed refetch keeps
  it locked and offers one repeatable reload action; it neither repeats conversion nor creates a new
  import. The editor unlocks and receives focus only after the converted draft loads successfully.
- Permanent offline Node tests cover strict safe DTOs, no-store transport, acknowledgement invalidation,
  and redacted conflicts. The real-route Playwright suite covers four scenarios, including exact retry,
  all-rejected and unsupported recovery, revision/dirty conflict recovery, committed-refetch recovery,
  keyboard behavior, and 375 px reflow. `./check_codebase.sh` passed 11 of 11 checks, including 173
  Node and 184 server tests; Chromium passed 4 of 4 scenarios. Independent security and HCI reviews
  found no P0/P1 issue.
- Evidence: `docs/active_plans/workstreams/qti_author_ui_implementation.md`.

### WP-QTI-11 live gate

- Owner: independent integration owner.
- Depends on: WP-QTI-3 through WP-QTI-10.
- Acceptance: full disposable profile-to-native-flat path, grading, RLS, archive/provenance, cleanup.

### WP-QTI-12 independent review and docs

- Owner: reviewer and documentation owner, separate from implementers.
- Depends on: WP-QTI-11.
- Acceptance: no P0/P1; active plan/status/changelog/contracts/architecture updated with exact evidence.
- Next package: release the shared Store/client/route/docs seams to the dependency-ordered
  `docs/active_plans/decisions/course_appearance_plan.md`. WP-QTI-13 exporters remain optional and
  follow that course package.

### WP-QTI-13 exporters

- Owner: separate Canvas and Blackboard export owners.
- Depends on: WP-QTI-12 import PASS.
- Acceptance: semantic round trip for accepted subset and refusal before output for every unsupported
  PLE field.

## Acceptance criteria and gates

### Adapter behavior

- The same bytes produce the same profile, result ordering, diagnostics, choice map, canonical flat
  source, and digests.
- Canvas and Blackboard detection never fall through to one another.
- Unknown or ambiguous vendor evidence never receives a vendor compatibility label.
- One unsupported item does not erase accepted siblings.
- Every rejection names profile, item/resource location, stable code, and actionable reason.
- Correct choice is absent from all safe result serialization and `Debug` output.

### Conversion behavior

- Exact archive, committed import, selected item, profile report, public/private/combined mapping
  digests, warning digest, and draft revision are revalidated before mutation.
- The native flat compiler is the only producer of canonical source and private grading material.
- A successful conversion yields the same canonical bytes as the equivalent hand-authored flat
  source.
- Any stale or divergent input leaves draft, source, private material, and provenance unchanged.
- The converted question opens in the existing flat editor and remains answer-free in learner
  preview.

### Security and tenancy

- Import upload, report, conversion, and provenance are author-only and tenant-scoped.
- Inaccessible workspace/import/item relationships are non-enumerating.
- Original and published provenance archives are never signable or directly browser-deliverable.
- RLS is enabled and forced; least-privilege grants are tested using real roles.
- Only the existing flat grader capability can read published answer material.
- Wasm and generated browser contracts do not acquire QTI profile or private flat types.
- Object-store probes prove exact replay, divergent collision refusal, failed-publication orphan
  quarantine, current-origin replacement release, workspace cleanup, and published-version retention.

### Author experience

- The UI names the detected source as Canvas QTI 1.2 or Blackboard QTI 2.1.
- Accepted and rejected items are visually distinct without relying on color alone.
- Defaulted fields are stated in plain language before conversion.
- Conversion stays disabled until the current safe report revision and warning review token are
  acknowledged.
- Errors preserve selected file/report context and give a next action.
- Conversion navigates into the completed flat editor for normal review and publication.
- Keyboard, focus, status, 375 px reflow, and double-action locking have Playwright evidence.

### Export behavior

- Export is absent from product claims until Q7 passes.
- Export refuses unsupported source fields before writing an object.
- Re-import of an exported profile preserves supported semantics and stable PLE choice identity.
- A generated package never masquerades as the exact original vendor archive.

## Test and verification strategy

### Permanent tests

- Adapter unit tests for detection, accepted mapping, every refusal class, deterministic IDs,
  collision extension, safe markup, resource limits, and partial package success.
- Native unit test comparing imported factory output with canonical hand-authored flat source.
- Data-access conformance for atomic conversion, stale CAS, report/mapping/warning digest mismatch,
  origin preservation, publication copy, tenant isolation, and retention.
- Server route tests for upload exact replay/divergence, body/media refusal, worker result, safe GET,
  conversion, no-store, ETag, and non-enumeration.
- TypeScript tests for strict safe DTOs, state recovery, warning acknowledgement, and no archive
  parsing or persistence.
- Playwright for upload, mixed results, warning review, convert, editor handoff, keyboard, and mobile.
- Existing hostile ZIP, legacy QTI, flat publication/runtime, and crate-boundary tests remain gates.

### Disposable PostgreSQL oracle

- Apply the six baseline migrations in an isolated PostgreSQL 17 project.
- Upload and commit one minimized profile archive through the real worker.
- Verify one accepted/rejected report, exact retry, and foreign-tenant non-enumeration.
- Convert accepted content, publish native flat, and grade one correct and one incorrect response.
- Verify current and published provenance rows plus exact archive and canonical checksums.
- Verify direct application/student reads of grading and provenance archive material fail.
- Run retention cleanup for workspace staging without removing published private provenance.
- Remove the exact disposable project; leave pre-existing containers untouched.

### Focused commands

```bash
cargo test -p adapter_qti
cargo test -p adapter_native flat_question
cargo test -p learning-data-access --test conformance qti_
cargo test -p learning-data-access --test conformance flat_import
cargo test -p server_core qti_profile
node --import tsx --test tests/test_qti_profile_import.mjs
bash run_playwright_tests.sh tests/playwright/qti_profile_import.spec.ts
```

### Full package gate

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
npx tsc --noEmit -p tsconfig.json
./check_codebase.sh
bash run_playwright_tests.sh --build
bash tests/e2e/e2e_database_baseline.sh
source source_me.sh && python3 -m pytest -q tests/
git diff --check
git diff --cached --check
```

## Migration and compatibility policy

- The repository is still pre-data, so this package edits the six-file baseline directly rather
  than adding a seventh corrective migration.
- Migration filenames and ledger ordering remain unchanged during this package.
- The generic QTI label correction must update fixtures, worker registry values, runtime adapter
  values, and any persisted schema checks in one atomic patch.
- If compatibility evidence requires reading the old inaccurate label, readers may accept it as a
  legacy alias while every new write uses the honest ID. Do not write both.
- Profile IDs, profile versions, mapping versions, warning digests, normalized item digests, and
  archive digests are immutable evidence.
- Workspace origin is current state; published origin is immutable history attached to the
  published version. No grade or publication event-history table is introduced.
- Published origin retains the publisher tenant as its RLS owner even when the answer-free catalog
  version is institution-visible or public. Other tenants can use the published question but cannot
  enumerate its private vendor archive or import mapping.
- Future profile expansion adds a new version. It does not reinterpret an existing v1 result.

## Risk register

| Risk                                      | Consequence                      | Control                                                                  |
| ----------------------------------------- | -------------------------------- | ------------------------------------------------------------------------ |
| Vendor dialect guessed incorrectly        | Wrong answer or presentation     | Exact manifest/profile contract and fixture-backed parser                |
| Rich HTML flattened                       | Lost scientific meaning          | Small allowlist; tables/media/styles refuse                              |
| Partial/negative scoring ignored          | Wrong grades                     | Reject all nontrivial response processing                                |
| Default policy mistaken for vendor policy | Instructor surprise              | Named PLE defaults, warning digest, explicit review                      |
| Choice IDs silently collide               | Wrong correct answer or feedback | Hash-based stable mapping with collision extension                       |
| Original archive becomes detached         | No forensic recovery             | Current and immutable provenance plus non-signable archive copy          |
| Browser receives answer material          | Academic integrity breach        | Safe DTOs, server conversion, serialization scans, Wasm boundary tests   |
| QTI work grows catch-all files            | Maintainer overload              | Responsibility-named modules and <1,000-line hard owner limit            |
| Export drops unsupported PLE fields       | Lossy round trip                 | Refuse before object creation; import acceptance precedes export         |
| Live tests pass only on mocks             | Production RLS defect            | One real-role PostgreSQL profile-to-flat oracle                          |
| Import worker delays grading              | Learner latency                  | Existing family-filtered worker; operational queue metrics and isolation |
| Broad corpus becomes brittle CI input     | Slow, unstable tests             | Minimized readable fixtures; broad corpus stays discovery-only           |

## Documentation close-out requirements

- Update [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) only for durable profile/default/provenance
  choices that survive implementation.
- Update [QTI-JSON_OBJECT_FORMAT.md](../../QTI-JSON_OBJECT_FORMAT.md) with import provenance and
  explicit profile/default behavior; do not put vendor extensions in the flat schema.
- Update [implementation_plan.md](../implementation_plan.md) with the completed profile boundary and
  exact supported versions.
- Update [partial_commit_status.md](../partial_commit_status.md) with focused/full/live evidence and
  the next unfinished milestone.
- Update `docs/CONTRACTS.md`, `docs/CODE_ARCHITECTURE.md`, and `docs/FILE_STRUCTURE.md` for new
  contracts and responsibility-named modules.
- Add a workstream implementation handoff and independent review report.
- Update `docs/CHANGELOG.md` only after executable acceptance passes.

## Patch plan and reporting format

Patches land in dependency order and remain independently reviewable:

1. Profile contracts, honest generic label, and readable fixtures.
2. Shared markup/choice helpers plus separate Canvas and Blackboard parsers.
3. Native imported factory and pure bridge tests.
4. Provenance contract, typed archive key, schema, and atomic conversion command shape.
5. Memory and PostgreSQL implementations plus conformance.
6. Upload/report/convert server routes and focused HTTP tests.
7. Author UI, Node tests, and Playwright acceptance.
8. Disposable PostgreSQL oracle, independent review, and documentation handoff.
9. Canvas and Blackboard exporters only after import PASS.

Each handoff reports:

- files changed and line counts;
- contract and behavior completed;
- focused and full validation commands with results;
- security/tenancy evidence;
- known limitations and deferred profile versions;
- confirmation that staging/index state was not changed.

## Open questions and decisions needed

No question blocks Q1 through Q6. The following remain intentionally deferred:

- Whether a later flat-source version supports imported images and asset references.
- Which exact fixture-backed vendor feedback forms merit a new profile version.
- Whether `sub` and `sup` gain a tested canonical representation in flat Markdown.
- Whether profile exports are exposed in the first author UI or only through an export job initially.
- Whether broad local-corpus compatibility statistics are worth a one-time report after the strict
  profiles pass; those statistics do not widen the accepted contract.
