# M15/M16 authoring handoff

**Current status:** The initial checkpoint below is historical. Its statements that Draft source
saves are parser-gated and creation is Native-only are superseded by the later M15 raw-source and
common-creation contracts. Current Draft saves persist unfinished UTF-8 source under a registered
binding; current creation accepts Native JSON, PG, or PGML. Do not use those first-checkpoint
descriptions as active requirements.

This file records current source boundaries for the Draft workflow. The approved outcome remains
the M15/M16 text in `docs/active_plans/active/question_spec_implementation_plan.md` and
`docs/HUMAN_GUIDANCE.md`.

## Current source and overlap

- Native editor and request client: `src/features/ple_question_json_authoring/question_json_editor_page.tsx`,
  `question_json_client.ts`, `question_json_repository.ts`, and `question_json_source.ts`.
  The current page has explicit Save and rejects source validation errors before sending a typed
  `PleQuestionJsonDocument`.
- Draft source API: `crates/server/src/authoring.rs` registers and handles `/source`; GET returns
  the source with an ETag, while PUT calls `validated_source(&body)`, then saves the source-derived
  Type through the Draft store. Keep this endpoint's publication and image security checks in scope.
- Draft store and binding: `crates/learning-data-access/src/authoring.rs`,
  `crates/learning-data-access/src/postgres/authoring.rs`,
  `schemas/base_schema/20_tables/question_authoring.sql`, and
  `schemas/base_schema/50_functions/question_authoring_operations.sql`.
  Draft creation records explicit `question_format` and optional WebWork PG path; `AuthoringDraft`
  and the current source read response do not return those binding fields.
- M07 overlap: `question_authoring_operations.sql` and
  `question_publication_operations.sql` own existing Draft fork-parent persistence and the
  publication handoff that reads it. Hold Draft SQL and source-repository edits until
  `output_question_spec/M07_DRAFT_SQL_READY` exists. Coordinate against that marker before changing
  shared queries or persistence structs.
- M16 Native evaluation already has `crates/domain/src/draft_preview.rs` and the WASM bridge in
  `src/wasm/index.ts`; the browser Native editor currently uses its local PLE preview path.
- M16 WebWork execution has `crates/server/src/webwork_document_route.rs`,
  `crates/adapters/imathas/src/imathas_question_backend/`, and the opaque browser frame in
  `src/components/opaque_webwork_preview_frame.tsx`. The preview frame bridge currently reports
  resize; testing needs an explicit, origin-checked response path. Keep Draft render/test results
  transient and do not create Student Work or Published Question tuples.
- Existing correction publication operation: `/api/authoring/drafts/{draft_question_id}/publish-revision`
  in `crates/server/src/authoring.rs`; it accepts the same Owner/Sysadmin authorization boundary
  described in `docs/active_plans/reports/QUESTION_SPEC_M06_PERMISSIONS.md`. M15/M16 own the
  owner/Sysadmin content-correction affordance wired to that operation.

## Coordination

**Root coordination decision (2026-10-06):** M15 may edit the ordinary `/source` handlers and
ordinary Draft load/save paths now. In `question_authoring_operations.sql`, the independent ordinary
functions are `ple_private.load_authoring_draft`, `ple_private.save_authoring_draft`, and their
`ple_api` wrappers. M07 retains the Draft table declaration, source-parent/fork creation functions
and wrappers, and publication-parent handoff functions in `question_publication_operations.sql`.
Use `output_question_spec/M07_DRAFT_SQL_READY` to coordinate the remaining schema/parentage merge;
it is not a blanket hold on ordinary save/read routes or functions.

The browser, ordinary source route, and ordinary source-store owners may proceed within their
separate function boundaries. Keep the explicit source Backend/format binding through save, read,
preview, test, and publication. Draft saves accept unfinished content; publication validates source
and required metadata. CSP response headers and local work preservation on errors remain required.

The current M15 route work must verify malformed Native bytes in addition to empty Native and broken
PG/PGML content. Limit source support to Native JSON, PG, and PGML; do not add Imathas support here.
The raw-source persistence oracle must also exercise a valid-shaped Native HOTSPOT source with an
unresolved/unregistered image tuple, save and reopen its exact bytes, and confirm no trusted image
binding is fabricated.

The ordinary `/source` handler and ordinary Draft load/save functions are explicitly authorized
independent work and may proceed before the M07 marker. M07's marker gates only parent/fork/schema
functions listed above. Source validity/type/image consistency remains a preview/publication concern;
the ordinary persistence gate should enforce technical safety bounds and registered binding, while
storing unfinished source bytes intact.

`crates/server/src/authoring.rs` was 990 lines during M15 integration. Keep it below the repository
limit by moving cohesive handlers/helpers into a focused module if additional routes would exceed it.

## M16 implementation boundary - 2026-10-06

### Owned paths

- Add the isolated Draft preview/test HTTP module at `crates/server/src/draft_preview.rs` and its
  focused route tests under `crates/server/src/draft_preview/`. Register it additively from
  `crates/server/src/lib.rs` and `crates/server/src/composition.rs`.
- Use the current `AuthoringDraft` source binding, `authoring_source::load_verified_source`, the
  configured PLE and WeBWorK backend paths, and the same saved Draft Edit Number. Read source from
  the stored Object Record on every preview/test request. Keep M16 requests transient: no
  Published Question Revision Tuple, Attempt, response row, or Student Work write.
- Where needed to send an unversioned Draft through the existing WeBWorK renderer, own only the
  narrow adapter boundary in `crates/adapters/webwork/src/renderer_contract.rs`,
  `crates/adapters/webwork/src/lib/issue.rs`, and
  `crates/adapters/webwork/src/http_renderer/{protocol.rs,client.rs}`. Preserve the current
  published-source methods and renderer CSP response policy.
- Add a small browser client and preview/test surface in
  `src/features/question_draft_preview/draft_preview_client.ts` and
  `src/features/question_draft_preview/draft_preview_panel.tsx`. Connect it to the existing
  Draft route in `src/pages/question_draft_editor_page.tsx` only after the source binding and M15
  autosave state are available.
- M15 still owes an editable raw-source workspace for registered PG and PGML bindings on the
  existing Draft route. The current route loads every source through the Native JSON client and
  leaves other backends read-only; retain the same serialized debounced autosave contract and saved
  Edit Number for PG/PGML text. Keep it isolated from the Native structured editor.
  On failed save, preserve the newest local text and expose an explicit retry/reload choice; a
  subsequent keystroke must not be the only recovery path.
- Mount Native preview/test in `src/features/ple_question_json_authoring/question_json_editor_workspace.tsx`
  using its saved-state and response-format control; wire the existing revision publication client
  in `question_json_client.ts`, `question_json_repository.ts`, and `question_json_editor_page.tsx`.
- Extend only the existing opaque no-write preview bridge in
  `src/components/opaque_webwork_preview_frame.tsx` and `src/public/ple_bridge.js` to capture the
  instructor's WeBWorK form response through a closed `postMessage` record. Keep the existing
  `allow-scripts` sandbox, `form-action 'none'`, source/origin checks, and no direct frame network
  submission.
- Add the Owner/Sysadmin correction action in `src/pages/question_detail_page.tsx`. It may create
  or open a Draft through the existing authoring/fork command, then must use the existing
  `/api/authoring/drafts/{id}/publish-revision` client operation. No separate publication route.
- Keep focused checks in new `crates/server/src/draft_preview/` tests,
  `crates/adapters/webwork/src/lib/draft_preview_tests.rs`, and
  `tests/test_draft_preview_client.mjs`; write this handoff to
  `output_question_spec/m16_draft_preview_worker_note.md`.

### Shared files and overlap

- `crates/server/src/authoring.rs` is M15-owned and explicitly out of scope. M16 adds routes only
  through the new module and the composition root. Do not change the M15 raw `/source` request or
  response contract.
- `crates/server/src/composition.rs` and `crates/server/src/lib.rs` already contain concurrent
  M01-M27 edits. M16 owns one module declaration and one router merge; preserve every existing hunk.
- `src/pages/question_draft_editor_page.tsx` contains M15 editor work. M16 owns only additive
  preview/test composition after saved-source binding is known; keep autosave and General Feedback
  behavior intact.
- `src/features/ple_question_json_authoring/question_json_editor_workspace.tsx`,
  `question_json_editor_page.tsx`, and `question_json_client.ts` are Native authoring surfaces with
  concurrent M15 work. M16 owns a saved-only preview/test panel and the existing revision publish
  call; retain autosave, response editing, and first-publication behavior.
- `src/pages/question_detail_page.tsx` contains concurrent M26 metadata/archive work. M16 owns only
  an additive correction affordance gated by the server-projected owner/Sysadmin permission; retain
  the current M26 edits.
- `src/components/opaque_webwork_preview_frame.tsx` is shared with Published Question previews.
  M16 owns an opt-in response-capture prop that is absent from every existing caller, so those
  previews remain no-write and report size only.
- `crates/learning-data-access/src/authoring.rs`,
  `crates/learning-data-access/src/postgres/authoring.rs`,
  `crates/learning-data-access/src/question_source.rs`, and
  `schemas/base_schema/50_functions/question_publication_operations.sql` remain outside M16
  ownership. M07 owns parent/fork/publication persistence. Use the bound Draft state already
  returned by the M15 store path; escalate only if that exact binding is not available at runtime.
- Generated contracts, `question_authoring_operations.sql`, all source-generation outputs, and Git
  index state are outside this implementation.

### Assumptions to verify during implementation

- The browser waits for the M15 save acknowledgement before previewing/testing; the server then
  reloads one current Draft and its exact registered backend, format, path, and source Object Record.
- Native preview/testing uses the existing PLE Question JSON parser, presentation, and grader. PG and
  PGML remain distinct stored formats while both use the configured WeBWorK renderer/grader and
  registered PG path.
- Native preview must reuse the established presentation and response renderer for content the
  current Native source format represents. Its closed prompt/response schema does not add a new
  requirement for table, equation, image, or geometry source blocks. Keep Native preview and
  response testing within that existing source contract.
- The renderer can accept an unversioned source request without fabricating a Published tuple. If
  the installed renderer protocol requires one, keep preview visible as unsupported until the
  renderer contract can carry Draft identity safely; never synthesize a Published tuple.

### M15 initial-create request contract

The normal `POST /api/authoring/drafts` request uses one common registered-source shape:
`{ metadata, questionBackend, questionFormat, webworkPgPath, source }`. `source` is raw UTF-8 text
for all three formats (Native JSON text, PG, or PGML); empty and incomplete text is permitted. The
Backend, Format, and optional PG path select the stored binding and media type; source bytes never
select or change them. Generic Question metadata stays in its own member. Initial Draft Type remains
unset until valid Native source is inspected or publication requires it. This contract supports the
normal New Draft flow for Native JSON, PG, and PGML without an import-only path.

### M16 correction-Draft boundary amendment - 2026-10-06

- M16 also owns `crates/server/src/question_correction_draft.rs`, registered additively from
  `crates/server/src/composition.rs`, and its focused route tests. It creates a current-content
  correction Draft only after the exact latest published tuple projects `viewer_may_edit_metadata`
  for the authenticated Owner or Sysadmin.
- A correction Draft is an ordinary unparented Draft. The existing `/fork` command creates a Draft
  reserved for a new lineage, and the existing `/publish-revision` SQL expressly rejects that
  fork Draft shape. M16 must not route correction through `/fork` or change M07 SQL.
- Preserve the exact source bytes, backend/format/PG path binding, Native HOTSPOT image binding, and
  separate Hint, Question Feedback, and Worked Solution values. The exact-revision read projection
  in `schemas/base_schema/50_functions/question_library_operations.sql` and
  `crates/learning-data-access/src/postgres/question_library.rs` may gain server-only support fields;
  they remain absent from browser DTOs and generated contracts. Publication continues through the
  existing `/publish-revision` command.
- The browser correction affordance is gated by the existing server-projected permission, creates
  the ordinary correction Draft with the exact current tuple, and navigates to that Draft with the
  parent tuple/reason context required by `/publish-revision`. Sysadmin correction uses the same
  route and store path.

## M15 browser source and ordinary creation contract - 2026-10-06

### Owned browser paths

- The raw source workspace is `src/features/question_draft_preview/webwork_draft_source_editor.tsx`.
  It uses `createQuestionDraftAutosave` from
  `src/features/ple_question_json_authoring/question_draft_autosave.ts` and the M16-owned
  `DraftQuestionPreviewClient` interface. Keep source text separate from the Native structured
  editor; use this workspace for registered PG/PGML and malformed Native JSON recovery.
- The existing source client is `src/features/question_draft_preview/draft_preview_client.ts`.
  `loadSource(draftQuestion)` returns exact decoded UTF-8 text, the GET ETag as
  `draftQuestionEditNumber`, and the immutable Backend/format/media/path binding from the response
  headers. `saveSource(draftQuestion, binding, source, expectedEditNumber)` sends the text body
  unchanged with the registered media type and strong `If-Match`; success requires 204 plus the
  newest ETag. The raw editor advances its shared Edit Number only from that acknowledgement.
- The ordinary creation surface is `src/pages/question_drafts_page.tsx`, its request client is
  `src/api/question_draft_creation.ts`, and its local layout is
  `src/pages/question_drafts_page.css`. Its normal form presents Native JSON, WebWork PG, and
  WebWork PGML; it creates through `POST /api/authoring/drafts` and navigates to the returned Draft.
- Focused browser checks live in `tests/test_draft_source_client.mjs` and
  `tests/test_question_draft_creation.mjs`. Existing serialized-save coverage stays in
  `tests/test_question_draft_autosave.mjs`.

### Browser/server request contract

- `POST /api/authoring/drafts` receives one JSON envelope with `metadata`, `source`,
  `questionBackend`, `questionFormat`, and `webworkPgPath`. Native JSON uses backend `ple`, format
  `pleQuestionJson`, a null path, and the default Native document. PG/PGML use backend `webwork`,
  their distinct format, the Instructor-supplied allowed relative PG path, and empty raw source.
  Type is not supplied at creation; publication requires the author to provide it.
- The stored PG path uses the same relative-path bounds as
  `crates/learning-data-access/src/authoring.rs::valid_webwork_pg_path`: nonempty, at most 1,024
  UTF-8 bytes, no leading slash, backslash, NUL, empty segment, `.` segment, or `..` segment. The
  source GET response remains the authority for the saved binding after creation.
- `GET /api/authoring/drafts/{id}/source` uses `ETag`, `Content-Type`,
  `X-PLE-Question-Backend`, `X-PLE-Question-Format`, and encoded
  `X-PLE-WebWork-PG-Path`. `PUT` uses that returned binding as-is. Native source is
  `application/vnd.peptidyle.question+json`; PG and PGML are distinct formats sharing
  `text/x-wework-pg`. The path and backend never come from source bytes.
- The current `crates/server/src/authoring.rs::CreateDraftRequest` still accepts only `metadata` and
  Native `source`, and its POST handler hardcodes Native format. The browser creation client now
  targets the explicit envelope above; the M15 server owner must align that handler before connected
  creation acceptance. This browser task does not edit `authoring.rs`.

### Route handoff

- The current `src/pages/question_draft_editor_page.tsx` already routes PG/PGML and malformed Native
  JSON through `WebworkDraftSourceEditor`; valid Native JSON stays in
  `PleQuestionJsonEditorPage`. Preserve that route ownership. Its source workspace receives the
  source GET result, shared Edit Number accessor/setter, source client, and response validator. Its
  additive `onSavedStateChange(saved)` callback reports whether the newest source generation has an
  acknowledged Edit Number for correction publication gating. M16 keeps ownership of
  `DraftPreviewPanel` and route composition until its handoff.
- Save failure leaves the newest textarea value in local state and exposes Retry plus an explicit
  discard-and-reload action. Reload replaces the local value only after a successful GET with the
  same immutable binding. Preview/test remains enabled only for the latest acknowledged Edit Number.

## Real-browser PG/PGML preview and response-test handoff - 2026-10-06

- Known repository PGML content includes `content/pilot/webwork/which_hydrophobic-simple.pgml`
  (a complete randomized multiple-choice problem with `RadioButtons`) and
  `content/pilot/sources/genetics/genetic_disorders-which_one.pgml`. There are no `.pg` files under
  `content/`; the renderer service's accepted PG source is therefore the source of truth for a
  PG fixture. Preserve each registered relative PG path as its binding during the Draft journey.
- The Draft route's preview section is `Saved Draft preview and test` in
  `src/features/question_draft_preview/draft_preview_panel.tsx`. Its WebWork iframe title is
  `PG Draft preview` or `PGML Draft preview`; the outside button label is `Test this response`.
  The frame must load with `draftTest=true` to enable the closed capture bridge.
- The bridge contract is implemented by `src/components/opaque_webwork_preview_frame.tsx` and
  `src/public/ple_bridge.js`: parent sends `{kind:"ple.webwork.draft-test.capture", version:1,
  captureId}` to the exact frame; opaque frame responds with
  `{kind:"ple.webwork.draft-test.response", version:1, captureId, pairs:[[name,value],...]}`.
  Parent accepts only opaque origin, exact `contentWindow`, matching 16-hex capture ID, and bounded
  closed fields. Clicking the button then sends the response through Draft `POST /test` with the
  exact latest saved Edit Number; the parent reports correct, partially correct, incorrect, or
  ungraded result. No frame form submission or Student Work write occurs.
- Real acceptance should use a valid PG and a valid PGML response-producing fixture; verify visible
  renderer content, change a response in-frame, click `Test this response`, observe the returned
  grade/status, refine source, wait for `Saved`, and publish through the normal `/publish` operation.
  An incomplete-source error proves preservation behavior only; it does not satisfy M16 preview/test.

## Publication-client ownership amendment - 2026-10-06

The exact UI publication owner is split: the active UI worker owns metadata/editor/control files;
a separate fresh worker owns only `src/features/ple_question_json_authoring/question_json_client.ts`,
`question_json_repository.ts`, and focused client tests. The client worker generalizes existing
`/publish` and `/publish-revision` decoding to accept either supported Backend summary through the
ordinary Question Library decoder, while retaining Native behavior. Reuse the established client
and request types; do not introduce a duplicate publisher or a separate publication object model.
Fresh SPEC and then different fresh QUALITY reviewers cover the client owner and UI owner together
after both hand off.

## Save acknowledgement correction - pending ownership handoff

Final M15 SPEC found a real concurrency defect: `PostgresAuthoringStore::save_authoring_draft`
and `save_authoring_draft_general_feedback` commit their compare-and-save operation, then reload
the Draft. A later writer can advance the Edit Number before that reload, causing the first request
to acknowledge the later writer's token as its own. Capture and return each operation's exact
post-update row/Edit Number within the CAS transaction, then decode that captured row. Preserve M07
parent fields and the M15/M16 optional Type tri-state. A fresh correction owner takes this exact
ordinary save seam after the active publication server owner releases it; until then this defect
remains blocking and M15/M16 source-ready is withheld.
- Question Type `Hotspot` on WebWork is a manual tag and carries no Native image binding. Only a
  Native source that actually compiles as HOTSPOT requires verifying/copying its Draft-owned image.
  Do not reject WebWork correction Drafts based on their manually assigned `Hotspot` Type.
- Verify existing `/publish-revision` inherits or carries the exact Revision's discipline/subject/
  topic/subtopic, Bloom fields, authorship, license, citation, and language while applying the
  corrected source plus current ordinary metadata/support values.

## M15 common creation module handoff - 2026-10-06

Integrated `crates/server/src/authoring/create.rs` as a child module through
`get(list_drafts).merge(create::route())`. Its interface is
`pub(super) fn route() -> MethodRouter<AuthoringRouteState>`; the method router owns the 2 MiB
request-body limit. As a child of `authoring`, it reuses
`AuthoringRouteState`, `CreatedDraftResponse`, `authoring_session_hash`, `now`, `private_error`, and
`private_store_error`, plus `authoring_source::matches_source_content_type` and
`authoring_source::put_workspace_source`. Its POST handler uses the session-authorized ordinary Draft
store path. It requires the five common top-level members,
selects media only for `(ple, pleQuestionJson, null)`, `(webwork, webworkPg, safe path)`, and
`(webwork, webworkPgml, safe path)`, preserves `source` as exact UTF-8 bytes, keeps `metadata`
separate, and sets initial Type to `None`. The existing `CreateAuthoringDraftInput` and ordinary
creation transaction represent the request; no shared persistence or SQL edits are needed.

The module and its focused tests are registered. `source ./source_me.sh && cargo test -p
server_core --lib authoring::create::tests` passed 5/5, covering exact request members, all three
registered bindings, empty/broken raw source, metadata separation, and unset initial Type. Scoped
Cargo check, formatting, and diff checks are also recorded in
`output_question_spec/m15_create_route_registration_cli.md`. The earlier report's E0382 test failure
was repaired; this current focused test is green. Connected PostgreSQL/Object Store persistence and
readback plus New Draft HTTP/browser acceptance remain owed. The existing M07 marker and parent/
fork/publication SQL fields and functions are unchanged.

## M15/M16 backend-agnostic publication metadata handoff - 2026-10-06

- Keep the raw source request unchanged: source text remains exact UTF-8 and separate from record
  metadata. Use the existing `GET/PUT /api/authoring/drafts/{id}/metadata` route and shared Draft
  Edit Number to edit normal Question record metadata while PG/PGML or malformed Native source is
  open.
- Add nullable `questionType` to the metadata read response. Add optional nullable `questionType`
  to the strict metadata save request: omission preserves the saved Type, explicit `null` clears it,
  and a value saves the ordinary Draft record Type. Native Type stays intrinsic to compiled source;
  reject a manual Type write for Native. WebWork Type is manual record metadata and remains unset
  until selected in the publication review.
- Saving metadata, support text, and an optional WebWork Type advances the same Draft Edit Number.
  The UI reports Saved only after both raw source and metadata/support/Type saves acknowledge the
  current snapshot. It disables preview and publication when any local field is unsaved.
- New PG/PGML publication requires a saved WebWork Type. It validates the exact saved backend,
  format, PG path, and source through the configured WebWork renderer, then invokes the existing
  publication service and SQL path. Native publication retains source-derived Type. Same-ID
  correction preserves its ordinary Draft Type and uses the existing `/publish-revision` operation.
- WebWork Type `Hotspot` is a manual tag. Correction image lookup/copy occurs only for Native
  source that compiles as HOTSPOT. Do not route ordinary edits through a new publication command or
  persist preview certification.
- UI integration is now owned after the M16 editor handoff: `GeneralFeedbackOnlyPage` and
  `WebworkDraftSourceEditor` will edit/save ordinary metadata and optional Type on the shared Edit
  Number, and the existing publication dialogs/commands will be reused for first publication and
  same-ID correction.

## Sysadmin correction-Draft publication handoff - 2026-10-06

Root authority requires Sysadmins to correct any eligible Published Question through the same
ordinary correction Draft and existing `/publish-revision` operation, while the Published Question
continues to keep its owning Instructor. Source inspection found that workspace creation,
ordinary Draft create/save, and `ple_private.publish_question_revision` accept Instructor or
Sysadmin, and a Sysadmin-created workspace is owned by that account. One source-read helper remains
Instructor-only: `ple_private.load_draft_question_publication_source` in
`schemas/base_schema/50_functions/question_publication_operations.sql`. A fresh narrow owner may add
Sysadmin to that function's existing role predicate and add a focused connected regression only;
preserve all M07 parent/lineage/publication persistence logic and other concurrent SQL changes.
Final source review must verify correction route permission, Sysadmin workspace access, ordinary
Draft write path, source-read helper, and same-ID publication authorization together. Root owns the
full connected `/publish-revision` and browser journey acceptance.

Root decision (2026-10-06): same-ID correction publication writes the correction Draft's saved
manual WebWork Type into the successor complete Question record. A correction Draft starts with
the exact current parent's Type; if its ordinary Draft Type is deliberately changed, preserve that
change at publication. Native Type remains compiled source-derived. Preserve all existing owner,
parent-revision, license, attribution, citation, classification, and Bloom inheritance rules.
The existing `ple_private.publish_question_revision` currently copies the parent's WebWork Type
unconditionally, so a fresh narrow SQL owner must update only that `CASE` expression and add a
meaningful copied-versus-changed Type successor regression after the active exact Sysadmin helper
owner releases `question_publication_operations.sql`. **Release resolved:** the Sysadmin helper
worker completed its exact predicate and focused compile-only regression and exited successfully;
its report is `docs/active_plans/reports/QUESTION_SPEC_M16_SYSADMIN_CORRECTION.md`. The distinct
`publish_question_revision` Type projection and test may proceed.

## Ordinary Draft metadata save permissiveness - 2026-10-06

Root decision: Drafts may remain unfinished; publication is the completeness boundary. The final
source SPEC checks ordinary metadata autosave only against the Draft model's technical bounds and
must verify that temporarily blank/incomplete fields accepted by that model are savable, rather
than importing publication requirements into Save. Do not add new nullable models, defaults, or
unrequested metadata behavior. Preserve required technical identity and registered source binding.

## Explicit manual WebWork Type transport - 2026-10-06

Root decision: persist the optional WebWork `questionType` tri-state through explicit ordinary SQL
function parameters and the exact required grant/caller hunks. Do not transport request metadata via
transaction-local GUCs: that creates hidden coupling in a preproduction contract. A grant/signature
update is an ordinary coordinated source change; it is not a reason to retain or disguise a
compatibility constraint. The active backend publication owner must poll this decision, remove any
GUC-based Type transport, and release the exact authoring SQL/store/grant files before a fresh
save-ack owner starts. The latest Native HOTSPOT test failure also remains open: preserve the
published Native intrinsic-Type/image guard, allow raw unresolved HOTSPOT source save, and verify
meaningful Draft storage/reopen without admitting or exposing foreign images. WebWork manual
HOTSPOT has no Native image binding.

Root clarified that unfinished Draft persistence accepts unresolved Native HOTSPOT image references.
Audit both the ordinary Save function and the Draft binding/storage constraints so no equivalent
registered-image gate survives elsewhere. Registered exact Draft-owned image validation belongs to
the existing preview/publication execution paths. Published Native HOTSPOT still requires a verified
image; this rule does not apply to a manual WebWork `hotspot` tag.

The focused Native HOTSPOT publication unit fixture must follow M04's full content-only source
contract: remove every record-metadata member (including `language`, `questionTitle`, and
`questionDescription`), then re-run the parser/test while preserving the missing-authoring-context
assertion.

The backend source correction owner released the ordinary save seams at 2026-10-06 10:58 UTC:
the two Postgres save methods and the ordinary source/metadata private/API SQL functions. A fresh
M15 acknowledgement owner is now capturing each saved Draft projection on the same open CAS
transaction/connection before commit. Preserve the released Type parameters/grants and HOTSPOT raw
storage behavior while making that narrow store-method correction.
