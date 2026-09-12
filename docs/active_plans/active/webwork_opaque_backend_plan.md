# Plan: opaque backend-owned WeBWorK interactions

Publish location on approval: `docs/active_plans/active/webwork_opaque_backend_plan.md`.

## Context

PLE parses the renderer's `renderedHTML` with a hand-written html5ever state machine and rewrites
exactly two PG shapes (RadioButtons, two-column matching) into native
`QuestionResponseFormat::MultipleChoice` / `::Matching` with SHA-256 opaque choice IDs, keeps a
private `AnSwErNNNN` replay map per attempt (`ple_private.question_attempt_webwork_replay`), and
on grade reconstructs upstream fields from PLE choice IDs. Any other PG control is refused. About
1,900 Rust lines and ~14 tests exist only for that translation:
`crates/adapters/webwork/src/http_renderer/{html_projection.rs,matching_projection.rs}`,
`client.rs:225-487`, `renderer_contract.rs:15-41`, `lib/issue.rs:362-405`, `lib/source_profile.rs`.

Grading is already delegated to the renderer (`problem_result.score`). The design error is the
presentation/response boundary, not the grader. `docs/HUMAN_GUIDANCE.md:149-151,193` states the
intended rule; `docs/DESIGN_DECISIONS.md:949-966`, `docs/QUESTION_BACKEND_CONTRACTS.md:125-170`,
and `docs/WEBWORK_PG_RENDERER_API_USAGE.md:32-42,123-158` encode the wrong one.

Renderer facts (fork checkout `../webwork-pg-renderer`, the sibling that
`local_stack_control/renderer.py:91-108` builds as `localhost/pg-renderer:reviewed`):
`templates/RPCRenderFormats/default.html.ep` emits `<base href=SITE_URL>`, `form#problemMainForm`
posting to `/render-api`, hidden `sessionJWT`, submit buttons unless `outputFormat=static`, a
footer, and `body onLoad=postMessage('loaded','*')`; assets resolve via `getAssetURL` to
`/webwork2_files/*` and `/pg_files/*` (`lib/RenderApp.pm:232-235`, `Controller/StaticFiles.pm`);
output templates are selected by `outputFormat`, so a PLE embed template is an ordinary addition.
No human step is needed to use fork changes: the local stack builds the sibling working tree.

Question IDs are unaffected: WeBWorK Questions keep ordinary 7-char Crockford IDs; the Published
Question Revision's source selects the backend.

## Objectives

- PLE treats a WeBWorK interaction as opaque backend-owned content and validates only a protocol
  envelope.
- Configured PG content uses the generic backend path without adding a PLE interaction
  implementation.
- WeBWorK render, browser delivery, submission, grading, and partial credit are proven by a small,
  representative one-time behavior set.
- The shared Question Backend boundary is minimal and lets iMathAS and H5P adapters bring their
  own transport later without a core redesign.
- The manager and subagents complete every milestone with automated evidence; no milestone waits
  on a human.

## Design philosophy

- Fix the design, not the symptom: replace the projection layer rather than extend it per control.
- Design for adaptability: backend extensibility is the acceptance criterion.
- Use the scientific method: M2 uses a generic probe with temporary inputs to establish the
  runtime facts that select decisions C and E.
- Be efficient with time: independent lanes run in parallel; file ownership is exclusive per
  work package.
- Long-term over short-term: renderer-side changes in the owned fork keep PG knowledge inside the
  backend.
- Every renderer-fork edit has an upstream merge cost. M1 prefers existing renderer
  configuration, templates, routes, output behavior, and PLE-owned boundary behavior; it adds
  renderer behavior only where an embeddable credential-free document or working generated assets
  cannot reasonably be supplied by PLE.
- PLE presents an opaque backend document with a short, generic baseline stylesheet. The renderer
  keeps its HTML, controls, interaction behavior, and Question-authored CSS; PLE neither parses
  nor rewrites those details. Question-authored CSS deliberately follows the baseline and wins in
  the normal cascade.
- Rejected alternative: renderer emits a typed JSON interaction model. Rejected because it
  reintroduces a PLE interaction vocabulary that grows per PG control.
- Evidence strategy for uncertain methods: fixed requirements plus decisions C and E with rules applied by the manager
  to the findings report produced by the probe tool.

## Scope

- Add a `ple_embed` output format and a two-prefix asset namespace to the renderer fork.
- Add `devel/webwork_render_probe.py`, a permanent tool that renders any PG file through the
  private renderer and writes the envelope, embed document, and grade results.
- Select a small, representative one-time behavior set with minimal connected replay evidence
  (submission fields plus expected scores).
- Define the minimal shared backend-owned presentation/response/state boundary.
- Rewrite the WeBWorK adapter transport and issue/grade lifecycle; state only if decision E requires it.
- Add document delivery and asset proxy routes, submission acceptance, worker forwarding.
- Add the iframe browser component and bridge script.
- Replace the replay table and allow-lists in `schemas/base_schema/`.
- Delete projection code, replay plumbing, coupled tests, and the Perl probe scrape.
- Add a few durable product-boundary tests and run one-time connected E2E evidence.
- Rewrite contract docs, design decision, human guidance, security model, database and API
  references, changelog.

## Non-goals

- Implement iMathAS or H5P adapter, schema, or browser work. This version succeeds without them
  because neither backend is used in Fall; the shared boundary reserves their slot.
- Display PG-native post-submit feedback to Students. This version succeeds without it because
  Fall grading records score and correctness only; the probe tool captures the feedback fields
  so a later task adds one document fetch without changing the render/submit boundary.
- Claim Open Problem Library breadth.
- Change Question ID, Assignment, Attempt, or grade-recording lifecycle.
- Keep the old projection path behind a flag.

## Current state summary

Pipeline today: PG source (S3) -> `protocol::render_fields()` -> `POST /render-api` ->
`parse_render_rpc()` -> `extract_problem_body()` -> radio or matching parser -> typed presentation
plus private replay map -> S3 cache + Postgres replay row -> browser native controls -> opaque
choice IDs -> worker maps IDs back to `AnSwErNNNN` -> `submitAnswers=1` -> score. Frontend
`src/components/question_renderer.tsx` and `question_response_control.tsx` contain no WeBWorK
code. Renderer identity is the configured `QuestionRendererVersion` (`client.rs:83`); the local
stack records the image OCI configuration ID (`local_stack_control/renderer.py:31-48`).

## Architecture boundaries and ownership

1. Shared Question Backend contract (minimal, lifecycle only):
   ```text
   render(question identity, attempt context) -> opaque presentation + opaque state
   submit(opaque response, opaque state)      -> score + feedback + updated opaque state
   ```
   Presentation, response, and state are adapter-owned opaque bytes with an adapter-declared
   kind tag. No "HTML", "form", or "iframe" vocabulary at this layer. Native PLE Question JSON is
   one backend whose presentation is the typed `QuestionVariationPresentation`.
2. WeBWorK adapter transport (implemented fully here): embed document, asset namespace, iframe,
   postMessage form capture, verbatim field forwarding, score mapping, optional state.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component | Review boundary |
| --- | --- | --- |
| M0 | temporary inputs in `tests/_temp/` or `/private/tmp`, selected maintained sources, or temporary authored PG probes | evidence review |
| M1 | maintained sibling `../webwork-pg-renderer`: `Dockerfile`, `templates/RPCRenderFormats/ple_embed.html.ep`, `lib/WeBWorK/FormatRenderedProblem.pm`, `lib/RenderApp.pm`, `docs/RENDERER_API_USAGE.md`, changelog | fork working tree |
| M2 | `devel/webwork_render_probe.py`, `docs/active_plans/reports/webwork_opaque_render_findings.md` | tool + report |
| M3 | `crates/question_model/src/{response.rs,presentation/model.rs}`, `crates/domain`, `src/wasm/` | Rust review |
| M4 | `crates/adapters/webwork/src/{renderer_contract.rs,http_renderer/*}`, `crates/server/src/composition.rs` | Rust review |
| M5 | `crates/adapters/webwork/src/lib/{issue.rs,grade.rs}` | Rust review |
| M6 | `schemas/base_schema/*.sql`, `crates/learning-data-access/src/{webwork_*,postgres/webwork_*}` | SQL + Rust review |
| M7 | `crates/server/src/webwork_asset_proxy.rs` | Rust review |
| M8 | `crates/server/src/{webwork_document_route.rs,assignment_delivery.rs,assignment_delivery/submission.rs,worker.rs,question_library.rs}` | Rust review |
| M9 | `src/public/ple_bridge.js`, `src/styles/ple_embed.css`, `pipeline/build.mjs`, `src/components/question_response_controls/{backend_owned_document.tsx,common.tsx,question_response_control.tsx}`, `src/api/decoders/{assignment_attempt.ts,presentation_delivery.ts,assignment_attempt_navigation.ts}`, `src/pages/assignment_attempt_page.tsx` | TS review |
| M10 | deletions, `containers/webwork/probe_render_api.sh` | Rust + shell review |
| M11 | `crates/adapters/webwork/src/http_renderer/tests.rs`, `lib/tests.rs` | Rust review |
| M12 | temporary connected curl/browser/provisioning inputs; `docs/active_plans/reports/webwork_opaque_e2e_findings.md` | E2E review |
| M13 | `docs/*.md`, `docs/CHANGELOG.md` | docs review |

## Milestone plan

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M0 | Representative one-time behavior evidence | Substantially different PG behaviors with minimal connected replay evidence | Evidence for later gates |
| M1 | Fork embed mode | `ple_embed` template, asset prefixes, baseline stylesheet link, local image build | Renderer emits an embeddable, credential-free document |
| M2 | Probe tool and findings | `devel/webwork_render_probe.py` verifies fixed requirements and decides C, E over supplied inputs | Requirements pass; C and E selected by rule |
| M3 | Shared opaque boundary types | `BackendOwned` presentation/response/state | Model compiles; pairing rules tested |
| M4 | Adapter transport | Envelope validation, embed request, verbatim forwarding | Adapter unit tests green |
| M5 | Adapter issue and grade lifecycle | One render per issuance, no cache; state per E | Lifecycle tests green |
| M6 | Schema and data access | Drop replay; opaque payload; state table only under E2 | Live PostgreSQL tests green |
| M7 | Asset proxy route | Two-prefix GET proxy | Renderer assets served via PLE |
| M8 | Document route, submission, worker | End-to-end server path | curl round trip records grades |
| M9 | Bridge, baseline stylesheet, and browser component | iframe, postMessage, submit reuse, static stylesheet delivery | One-time Playwright evidence submits representative inputs |
| M10 | Delete projection | Remove parsers, replay plumbing, Perl scrape | grep gate clean |
| M11 | Boundary tests | Replace projection tests | Few strong adapter tests |
| M12 | One-time connected E2E evidence | Representative opaque behavior through the generic path | Primary evidence green |
| M13 | Documentation and changelog | Rewrite contracts, decisions, guidance | Docs match code |

Dependency graph (IDs, not order labels):

```text
M0 ----+----> M2 ----> M4 ----> M8 ----> M9 ----> M12
M1 ----+       |        |        ^        |
               +------> M5 -----+        |
M3 ------------+------> M6 -----+         |
M2 (decision C) + M4 -> M7 -----+         |
M4 ------------------> M10 -----> M11 ----+
M2 (facts) + M12 -----> M13
```

Parallel dispatch windows: {M0, M1, M3} at start; {M2} then {M4, M5, M6}; {M7 after M4}; {M8}; {M9, M10,
M11}; {M12, M13 draft}; M13 final.

### Milestone: M0 representative one-time behavior evidence

- Depends on: none for input selection; any behavior that requires the full PG pin waits for
  M1 WP-F3.
- Deliverables: a small one-time set of PG inputs that demonstrate substantially different
  backend-controlled behavior, selected from available instructional sources where useful or
  authored temporarily where that makes the experiment clearer. Inputs and submission evidence
  live only in `tests/_temp/` or `/private/tmp`; no content tree, fixture corpus, prescribed count,
  or permanent inventory is created. Each input has only the correct, incorrect, and, when useful,
  partial payloads needed to establish its score.
- Content policy: maintained instructional Questions remain at their authoritative locations.
  Temporary PG sources are implementation evidence, never production content or fixtures.
- Workstreams: one coder using `/webwork-writer-expert`.
- Entry criteria: none.
- Exit criteria: each selected input renders through the current renderer with
  `flags.error_flag == 0`; after the relevant renderer build, its connected evidence payloads yield
  the expected scores via direct renderer `curl`. Record the result in findings and changelog notes.
- Parallel-plan ready: yes (independent inputs may be checked in parallel; one owner consolidates
  temporary evidence).

### Milestone: M1 fork embed mode

- Depends on: none.
- Deliverables: the minimum additive renderer delta in the maintained sibling
  `../webwork-pg-renderer`: `Dockerfile` pins its full PG checkout to immutable
  `3ca5687eaa28bebde231043a1ea5609c04edd670` and removes the incompatible PG-2.17 single-macro
  overlay; an additive `ple_embed.html.ep`; the smallest `FormatRenderedProblem.pm` format branch
  and two-prefix normalization for renderer-generated body resources; and one
  `GET /webwork2_files/*static` alias in `RenderApp.pm`. The template links the query-free static
  PLE stylesheet `{pleOrigin}/styles/ple_embed.css` after renderer third-party CSS and before PG
  `extra_css_files`; this one product-facing template line is the entire theme mechanism. Existing
  generic request parsing carries
  `pleAssetBase` (`{PLE origin}/api/webwork-assets`) and `pleOrigin`; the existing `/pg_files`
  route remains unchanged. The fork docs and changelog record the short interface contract. The
  vendored `FormatRenderedProblem.pm` is the high-conflict surface, so its patch stays limited to
  format selection and the required `/pg_files/` and `/webwork2_files/` generated-asset URLs,
  emitted as exact query-free `{pleAssetBase}` URLs. Local image rebuilt by `local_stack.py` build
  mode, which builds `Dockerfile`.
- Workstreams: one coder in `../webwork-pg-renderer`.
- Entry criteria: none.
- Exit criteria: temporary M0/M2 connected evidence renders an ordinary PG input with
  `outputFormat=ple_embed` and establishes: no `<base`, no hidden input whose name matches
  `*JWT`, no `submit-buttons-container`, no `id="footer"`, no `onLoad`, and a form without `action`;
  every renderer asset uses `{pleAssetBase}/{webwork2_files|pg_files}/...` with cache/version
  queries stripped; its generic baseline stylesheet URL is `{pleOrigin}/styles/ple_embed.css` in
  the required cascade position; the sole PLE-owned bridge uses `{pleOrigin}/ple_bridge.js`; and a generated PG
  graph/image resource is fetchable after body-markup normalization. A temporary pre/post default
  render-and-grade observation verifies that the new branch leaves existing callers usable. The
  implementation explains why each fork edit is required by the product contract and uses no
  test-only behavior.
- Parallel-plan ready: no (one template, one lane).

### Milestone: M2 probe tool and findings

- Depends on: M0 (representative evidence), M1 (embed format).
- Deliverables: `devel/webwork_render_probe.py` (argparse: `-i/--source` and repeatable
  `--submission` JSON input, `-o/--output` dir; renders with `ple_embed`, writes envelope JSON,
  document HTML, asset URL list, grade results for the supplied connected evidence payloads with
  and without render-issued `sessionJWT`/`problem_state` carried into that one grade request,
  fixed-requirement pass/fail, and the C1/C2 browser check needed to select browser isolation;
  it writes `findings.json` and a findings report. Inputs and probe results are temporary except
  for the concise report of the selected runtime decisions.
- Workstreams: tester writes the tool; reviewer agent applies decision rules C and E and records
  the selection in the report; fixed-requirement failures are filed as defects on their owner
  milestone (M1 fork or M9 bridge) and rerun.
- Entry criteria: M0 and M1 exit.
- Exit criteria: fixed product requirements pass for the representative inputs; C and E each have one
  selected path re-derived by a reviewer agent from `findings.json`.
- Parallel-plan ready: no.

### Milestone: M3 shared opaque boundary types

- Depends on: none (state slot always present; path E only affects storage).
- Deliverables, following the existing `ImathasQuestionBackend {}` marker precedent in
  `crates/question_model/src/response.rs:245-249`: `QuestionResponseFormat::BackendOwned {}`
  marker and `StudentResponse::BackendOwned { payload: Vec<u8> }` (base64 in JSON, 64 KiB
  bound); `QuestionResponseControl::BackendOwned`; `supports_question_type` true; the WeBWorK
  `QuestionVariationPresentation` has empty `prompt` and this marker, the document is served by
  the route; opaque state `Option<Vec<u8>>` on the backend trait in `capability.rs`; domain
  pairing rule: `BackendOwned` response pairs only with `BackendOwned` format; WASM exports
  updated. No new presentation enum: the existing marker pattern already expresses "backend
  owns this" without naming a transport.
- Workstreams: one coder.
- Entry criteria: none.
- Exit criteria: `cargo test -p question_model -p domain`; serde round-trip tests; WASM build.
- Parallel-plan ready: no.

### Milestone: M4 adapter transport

- Depends on: M2 (fixed requirements verified), M3 (types).
- Deliverables: `renderer_contract.rs` without replay types (`RenderedWebworkQuestion {
  document: Vec<u8>, renderer_version, sha256, state: Option<Vec<u8>> }`, `GradeRequest {
  source, seed, payload, state }`); adapter-private `BackendOwned` response bytes are a bounded,
  canonical JSON array of `[name, value]` string pairs, preserving `FormData` order and duplicate
  names; `protocol.rs` sends `outputFormat=ple_embed`,
  `pleAssetBase`, `pleOrigin`; `client.rs` keeps envelope validation (closed member set, JWT
  shape then discard, protected top-level members, size, redirect, content type), stores
  `renderedHTML` verbatim, `grade()` validates this array and rejects reserved server-owned names
  before forwarding its pairs, then overrides server-owned fields
  (`problemSource`, `problemSeed`, `outputFormat`, `_format`, `submitAnswers`, `isInstructor`,
  `showSolutions`, `showCorrectAnswers*`, `problemJWT`, `sessionJWT`) and refuses a payload that
  carries any of them; `composition.rs` derives and validates `pleOrigin` and
  `{pleOrigin}/api/webwork-assets` only from trusted deployment configuration; `QuestionRendererVersion`
  is sourced from Local Stack State OCI ID.
- Workstreams: expert_coder.
- Entry criteria: M2 and M3 exit.
- Exit criteria: `cargo test -p adapter_webwork` for `http_renderer` module; projection modules
  removed from `mod` tree (files deleted in M10).
- Parallel-plan ready: no (M7 route registration waits for M4's trusted origin construction).

### Milestone: M5 adapter issue and grade lifecycle

- Depends on: M2 (decision E), M3.
- Deliverables: `issue.rs` renders once per issuance, returns document bytes and sha256 for the
  server to persist, no `render_replay`, no cache; `lib/cache.rs` and `lib/source_profile.rs`
  deleted; capabilities uniform `AlgorithmicGeneration + ServerGrading + PartialCredit`;
  persisted-document reproduction belongs to the M6/M8 store and document route; under E2, `issue.rs`
  supplies render-issued state and `grade.rs` returns updated state for M6's restricted worker
  commit after one grade request. This lifecycle
  does not re-render a document or submit a second browser response.
- Library cleanup: M5 removes projection-derived educational-type assumptions; M8 consumes the
  revision's persisted `question_type` for Question Library metadata, never a PG/HTML inference or
  `MultipleChoice` default.
- Workstreams: coder (file ownership disjoint from M4: `lib/` only).
- Entry criteria: M2 and M3 exit.
- Exit criteria: `lib/tests.rs` updated and green; exactly one renderer call per issuance and
  one per grade in the recorded-renderer test double.
- Parallel-plan ready: yes (runs beside M4, M6, M7).

### Milestone: M6 schema and data access

- Depends on: M3 (payload type), M2 (decision E).
- Deliverables: drop `question_attempt_webwork_replay` and its capability checks
  (`attempt_presentation.sql:59-86`); add `backend_document text NULL` to the attempt presentation
  table with the `webwork_presentation` capability check requiring `backend_document`; retain the
  existing generic response save and whole-Attempt finalization lifecycle, and delete the dead
  dedicated `WebworkSubmissionStore`, `resolve_webwork_submission`, and
  `accept_webwork_submission` seams. Grading claim returns the opaque payload and, under E2, state;
  E2 stores opaque `bytea` state in a dedicated private attempt-owned state table whose restricted
  worker claim/commit procedure updates only that state under a valid lease. Document, public
  presentation, nonce, and checksum remain immutable. Add required checked author-declared
  `question_type` to Draft Source Binding and immutable Published Question Revision; create/save
  and publication copy it, library/search reads it, and Pilot publication maps its existing
  manifest type through the binding. Rust data access, authoring, Pilot publication, and role
  grants are updated and verified.
- Workstreams: coder.
- Entry criteria: M2 and M3 exit.
- Exit criteria: live PostgreSQL tests; a valid declared type survives Draft binding -> publication
  -> Library entry, unknown type spelling fails, and a real non-`multipleChoice` WeBWorK Library
  summary exposes its declared type without rendering or parsing PG; `cargo tools installation-data
  provision` replays clean on a fresh database.
- Parallel-plan ready: yes.

### Milestone: M7 asset proxy route

- Depends on: M2 (decision C sets proxy security headers and public-scope evidence), M4 (trusted
  origin construction).
- Deliverables: after M2 proves every proxied asset lacks protected or question-answer data,
  `GET /api/webwork-assets/{prefix}/{path}` is intentionally public in both C branches; prefix in
  `{webwork2_files, pg_files}`; raw URI validation rejects queries, redirects, empty or dot
  segments, backslashes, NUL, and percent-encoded separators or traversal before private-upstream
  URL construction; GET only; size bound 8 MiB; `Cache-Control: public, max-age=86400` for
  `webwork2_files`, `no-store` for `pg_files/tmp`; allow-list safe content types and response
  headers only; 404 for anything else; route-specific CORP follows C without changing global CORP;
  route registration follows M4's trusted-origin construction; API contract doc line.
  M7 owns no theme work: `/styles/ple_embed.css` is a normal PLE static file, never a renderer
  asset or asset-proxy response.
- Workstreams: coder.
- Entry criteria: M2 and M4 exits.
- Exit criteria: route tests with a stub upstream; live stack `curl` fetches MathJax, one PG image,
  and a generated graph/image resource from representative evidence.
- Parallel-plan ready: yes.

### Milestone: M8 document route, submission, worker

- Depends on: M4, M5, M6, M7.
- Deliverables: `GET /api/assignment-attempts/{assignment_attempt_reference}/questions/{position}/document`
  using the browser's existing Assignment Attempt reference and issued position (authorized like
  presentation, headers per decision C, `no-store`); selected-C CSP includes `base-uri 'none';
  object-src 'none'; frame-ancestors 'self'; form-action 'none'`; C1 handles its opaque-origin
  sender and route-specific CORP requirements, while C2 uses same-origin checks. `assignment_delivery.rs`
  stores document and `rendered_sha256`; generic submission save/finalize accepts `BackendOwned`
  within bounds; worker forwards payload and state through the existing grading lifecycle;
  `question_library.rs` exposes the immutable revision-declared educational `question_type` without
  PG/HTML inference or a `MultipleChoice` default.
- Workstreams: coder.
- Entry criteria: M4-M7 exit.
- Exit criteria: the M12 one-time curl evidence issues each selected Question, fetches document
  and one asset, posts each connected evidence payload once, waits for the worker, and asserts
  recorded `normalized_credit` equals its expected score.
- Parallel-plan ready: no.

### Milestone: M9 bridge and browser component

- Depends on: M2 (decision C) for the bridge; M8 for live verification.
- Deliverables: `src/public/ple_bridge.js` (loaded only from `{pleOrigin}/ple_bridge.js`, submit
  intercept, `FormData` serialization to the canonical `[name, value]` pair array, `postMessage`
  to `pleOrigin`), `src/styles/ple_embed.css`, and `pipeline/build.mjs` copies the stylesheet via
  its existing `STATIC_STYLESHEETS` list to `dist/styles/ple_embed.css`;
  `backend_owned_document.tsx` (iframe with selected `sandbox`,
  source and origin checks, pending response, keyboard focus into frame, submit reuse);
  decoders; `question_response_control.tsx` switch arm. The stylesheet uses only document-wide or
  ordinary HTML selectors for document background/foreground, readable default typeface, available
  width, inherited form fonts, and a visible focus outline. It has no Question Type, PG macro,
  input-name, or control-specific selector. The bridge remains submission-only.
- Workstreams: coder (bridge and component can be written before M8 exit; verified after).
- Entry criteria: M2 exit (write), M8 exit (verify).
- Exit criteria: `tests/test_webwork_delivery_input.mjs` replaced by a bridge unit test using an
  inline minimal DOM/form contract; the Playwright walkthrough submits each connected evidence
  payload once and its recorded grade matches the expected score.
- Parallel-plan ready: yes (bridge and component are separate files and owners).

### Milestone: M10 delete projection

- Depends on: M4, M5.
- Deliverables: delete `html_projection.rs`, `matching_projection.rs`, and
  `http_renderer/tests/current_matching.rs`; scrub verifies that M5's `cache.rs` and
  `source_profile.rs` deletions remain complete; `probe_render_api.sh`
  keeps envelope check and one known-field submit for `which_hydrophobic-simple.pgml`, drops the
  Perl projection and regex scrape; `local_stack_control/renderer.py` probe expectations updated.
- Workstreams: coder.
- Entry criteria: M4 exit.
- Exit criteria: `git grep -n 'opaque_choice_id\|replay_details\|parse_single_radio\|
  parse_matching_group\|source_profile\|lib/cache' crates/ src/ schemas/ containers/` returns
  nothing; workspace builds.
- Parallel-plan ready: yes.

### Milestone: M11 boundary tests

- Depends on: M4, M5, M10.
- Deliverables: in `http_renderer/tests.rs`: deterministic document bytes for an inline stable
  transport input;
  envelope refusals (unknown member, duplicate key, redirect, oversize, `error`, non-JSON); JWT
  shape then discard; grade forwards fields verbatim; grade refuses browser-supplied
  `problemSource`/`problemSeed`/`isInstructor`/`showSolutions`; score 0, 0.5, 1 mapping; timeout
  and outage refusal; the canonical pair-array payload preserves duplicate names and rejects
  reserved names; if selected, E2 carries render-issued state to one grade request and records
  returned state without a document re-render or second browser submission.
- Workstreams: tester.
- Entry criteria: M10 exit.
- Exit criteria: `cargo test -p adapter_webwork --all-targets`; clippy clean; each test names
  its regression in its doc comment per `docs/PYTEST_STYLE.md` spirit.
- Parallel-plan ready: yes.

### Milestone: M12 one-time connected E2E evidence

- Depends on: M8, M9.
- Deliverables: temporary curl and Playwright inputs under `tests/_temp/` or `/private/tmp` drive
  representative M0 evidence through the Live Demo once; the durable
  `docs/active_plans/reports/webwork_opaque_e2e_findings.md` records results. A fresh temporary PG
  input/configuration may be supplied through the same generic backend path to demonstrate that
  provision, backend-owned document delivery, submission, and recorded grading require no new PLE
  interaction implementation. The one-time browser evidence also records successful
  `/styles/ple_embed.css` delivery, its link after renderer third-party CSS and before a selected
  Question's `extra_css_files`, the ordinary baseline on an uncustomized region, and a selected
  PG-authored CSS override that remains effective.
- Workstreams: tester.
- Entry criteria: M8 and M9 exit.
- Exit criteria: both one-time E2E lanes pass on the built browser; the fresh-input demonstration,
  if used, passes through the generic path; temporary inputs are removed after findings complete.
- Parallel-plan ready: yes (curl and browser lanes; optional fresh-input demonstration).

### Milestone: M13 documentation and changelog

- Depends on: M2 (facts), M12 (evidence).
- Deliverables: see "Documentation close-out requirements".
- Workstreams: planner; docs can draft after M2 and finalize after M12.
- Entry criteria: M2 exit (draft), M12 exit (final).
- Exit criteria: `source source_me.sh && pytest tests/test_markdown_links.py
  tests/test_source_file_line_limit.py`; changelog under 800 lines or rotated.
- Parallel-plan ready: yes (one doc per owner).

## Fixed requirements (no decision; we own the fork and the schema is pre-production)

KISS: complexity earns its place. These are requirements verified by `devel/webwork_render_probe.py`
in M2; a failure is a defect fixed at its owner, never a PLE-side fallback.

- Embed document (owner: fork M1): `ple_embed` output contains no `<base>`, no hidden input whose
  name matches `*JWT`, no submit buttons, no footer, no `onLoad`, no form `action`; each renderer
  asset URL is `{pleAssetBase}/{webwork2_files|pg_files}/...` with cache/version queries stripped;
  its PLE-owned bridge URL is `{pleOrigin}/ple_bridge.js`. PLE stores `renderedHTML` verbatim. No
  PLE HTML filter exists. The renderer rewrites matching body-markup resource URLs under the two
  renderer asset prefixes.
- Generic presentation baseline (owner: fork M1, PLE M9): `ple_embed` links the query-free
  `{pleOrigin}/styles/ple_embed.css` after renderer third-party CSS and before PG
  `extra_css_files`. It supplies only document background/foreground, readable default typeface,
  available width, inherited form fonts, and a visible focus outline through document-wide or
  ordinary HTML selectors. PG-authored CSS remains later and intentionally takes precedence. No
  PLE HTML parsing, control-specific theme rule, Question Type selector, PG macro selector, or
  bridge-driven styling exists.
- Assets (owner: fork M1, PLE M7): every asset lives under `/webwork2_files/` or `/pg_files/`;
  PLE proxies exactly those two prefixes.
- Checkbox prerequisite (owner: fork M1): the complete PG checkout is pinned to
  `3ca5687eaa28bebde231043a1ea5609c04edd670`; no PG-2.17 single-macro overlay remains. M0/M2
  temporary connected evidence covers correct, partial, and wrong repeated native-form and
  JSON-array checkbox submits when that behavior remains representative.
- Submission capture (owner: bridge M9): the bridge serializes the entire form (`new
  FormData(form)`) to the canonical pair array, including legitimate PG hidden answer and control
  fields and preserving order and duplicate names. Renderer credential inputs whose name matches
  `*JWT` are absent from the embed document and therefore cannot be captured. M4 rejects reserved
  server-owned names before grading and forwards entries generically, without comma joins or
  content-specific parsing; grade from bridge-captured fields equals the connected evidence payload.
- Document storage (owner: M5/M6): one document per Question Attempt in a new
  `backend_document text` column on the existing attempt presentation table
  (`schemas/base_schema/attempt_presentation.sql`), written at issue, read by the document route
  and `reproduce()`. No shared render cache: `lib/cache.rs` is deleted. Reason: a render per
  issuance is cheap, and the old cache existed to amortize projection work that no longer exists.
- Renderer identity (owner: M4): `QuestionRendererVersion` is the image OCI configuration ID
  already recorded by `local_stack_control/renderer.py`; composition reads it from Local Stack
  State. No second version concept.
- Credential-free document contract (owner: fork M1): the embed document has no renderer JWT
  inputs. The bridge captures ordinary PG hidden answer/control fields as form data; M4 owns the
  reserved server-owned field boundary. A broad security review follows integration and does not
  block M0 or M1 evidence.

## Decision tables (M2 findings select the path)

Two facts cannot be known without running real PG output; each has a rule and both branches
implemented in full by this plan.

### Decision C: browser isolation

- Experiment: load each embed document in an iframe under C1 `sandbox="allow-scripts
  allow-forms"` with the M7 public safe-scope asset proxy, and C2 `allow-scripts allow-forms
  allow-same-origin` with CSP `default-src 'self'; script-src 'self'; style-src 'self'
  'unsafe-inline'; img-src 'self' data:; base-uri 'none'; object-src 'none'; frame-ancestors
  'self'; form-action 'none'`. Check MathJax typesetting, MathQuill inputs, PG images, keyboard
  navigation, zero console errors, and C1 opaque-origin/CORP behavior.
- Rule: every item passes C1 -> C1. Otherwise -> C2. The selection changes only iframe sandbox,
  document CSP, and route-specific CORP; after M2 safe-scope verification M7 remains intentionally
  public in both branches.
- Implementation: document route sets the selected CSP; component sets the selected `sandbox` and
  validates exact frame source plus the C2 same origin or the C1 opaque sender; M7 sets selected
  route-specific CORP without changing global CORP behavior.

### Decision E: backend state

- Experiment: for each recorded representative submission, including a partial-credit case when
  selected,
  grade once with only `problemSource`, `problemSeed`, fields, fixed flags, and `submitAnswers=1`;
  repeat that one grade request with render-issued `sessionJWT` and `problem_state`. This is not a
  progression test.
- Rule for the implemented render-to-one-grade-request lifecycle: identical `problem_result` and
  `problem_state` for all -> E1 stateless: no state row, the shared contract's state slot is `None`
  for WeBWorK. Any difference -> E2: add a dedicated private attempt-owned opaque `bytea` state
  table; adapter returns state on render, worker carries it to the one grade request, and the
  restricted lease-checked commit records returned state without weakening immutable presentation
  binding.

### Facts captured for later feedback and continuation work

- Fields that drive post-submit re-render, `problem_result.msg`, `problem_state`, and per-answer
  feedback markup. Recorded in the findings report; no implementation in this plan. A stateful
  continuation UI for Scaffold or compound PG progression is an out-of-scope follow-up.

## Work packages

Each package has one owner, exclusive files, one verification command. Reviewer agent on each.
Before broad exploration, implementation agents use Graphify task queries to locate symbols and
relations for their owned paths.

### Work package: WP-C1 select representative one-time evidence

- Owner: coder (`/webwork-writer-expert`).
- Touch points: authoritative source paths when selected, plus temporary evidence inputs only.
- Depends on: none, except that inputs needing the full PG pin wait for WP-F3.
- Acceptance criteria: each representative input renders error-free; temporary connected evidence
  has the correct, incorrect, and, where useful, partial submissions with expected scores verified
  by direct renderer `curl`. Consolidate only the temporary inputs needed by the probe and E2E
  evidence.

### Work package: WP-F1 embed template and format normalization

- Owner: coder (fork).
- Touch points: `templates/RPCRenderFormats/ple_embed.html.ep`,
  `lib/WeBWorK/FormatRenderedProblem.pm`.
- Depends on: none.
- Acceptance criteria: M1 temporary-evidence assertions, including the smallest structural
  normalization of renderer-owned resource URLs in generated problem markup needed for working
  generated assets. The template adds the sole baseline-style link after renderer third-party CSS
  and before `extra_css_files`. Existing generic request parsing supplies the two PLE URL inputs.
  Keep the vendored formatter patch to product-required behavior.

### Work package: WP-F2 asset prefix alias and bridge script tag

- Owner: coder (fork).
- Touch points: `lib/RenderApp.pm`, template script tag.
- Depends on: WP-F1.
- Acceptance criteria: the existing `/pg_files` route remains usable and the added
  `/webwork2_files/*static` alias resolves standard renderer assets; the template includes the
  PLE bridge tag.

### Work package: WP-F3 fork docs and image build

- Owner: maintainer.
- Touch points: fork `Dockerfile`, `docs/RENDERER_API_USAGE.md`, `docs/CHANGELOG.md`; PLE
  `local_stack.py` build mode.
- Depends on: WP-F1, WP-F2.
- Acceptance criteria: `Dockerfile` pins the complete PG checkout at
  `3ca5687eaa28bebde231043a1ea5609c04edd670` and removes the PG-2.17 single-macro overlay;
  concise renderer docs describe `ple_embed`, its two URL inputs, the two asset prefixes, the
  static `{pleOrigin}/styles/ple_embed.css` baseline link and cascade position, and normal PG
  form-field forwarding; `local_stack.py validate` runs the rebuilt image and records its OCI ID.
  M0/M2 supplies temporary checkbox, generated-image, and default-format evidence.

### Work package: WP-P1 probe tool

- Owner: tester.
- Touch points: `devel/webwork_render_probe.py`, `devel/DEVEL_README.md` entry.
- Depends on: WP-C1, WP-F3.
- Acceptance criteria: writes `findings.json` covering product requirements and decisions C, E
  for supplied representative inputs; pyflakes and typing gates pass.

### Work package: WP-P2 findings report and selections

- Owner: reviewer agent.
- Touch points: `docs/active_plans/reports/webwork_opaque_render_findings.md`.
- Depends on: WP-P1.
- Acceptance criteria: each decision row cites `findings.json` paths and states the selected
  path in the rule's words.

### Work package: WP-M1 shared types

- Owner: coder. Touch points and criteria: M3.

### Work package: WP-A1 adapter transport

- Owner: expert_coder. Touch points and criteria: M4.

### Work package: WP-A2 adapter issue and grade lifecycle

- Owner: coder. Touch points and criteria: M5.

### Work package: WP-S1 schema and data access

- Owner: coder. Touch points and criteria: M6.

### Work package: WP-S2 asset proxy

- Owner: coder. Touch points and criteria: M7.

### Work package: WP-S3 document route, submission, worker

- Owner: coder. Touch points and criteria: M8.

### Work package: WP-B1 bridge script

- Owner: coder. Touch points: `src/public/ple_bridge.js`, bridge unit test.
- Depends on: WP-F1 (embed document contract). Acceptance: M9 bridge criteria.

### Work package: WP-B2 browser component

- Owner: coder. Touch points: `src/styles/ple_embed.css`, `pipeline/build.mjs`, component,
  decoders, switch arm.
- Depends on: WP-P2 (decision C), WP-S3 for live check. Acceptance: M9 component criteria.

### Work package: WP-X1 delete projection

- Owner: coder. Touch points and criteria: M10.

### Work package: WP-T1 boundary tests

- Owner: tester. Touch points and criteria: M11.

### Work package: WP-T2 one-time curl evidence, WP-T3 one-time browser evidence

- Owner: tester each. Touch points and criteria: M12.

### Work package: WP-D1..D6 documentation

- Owner: planner per document. Touch points: listed in close-out. Depends on: WP-P2 (draft),
  WP-T2..T3 (final).

## Acceptance criteria and gates

- Per-patch gate: `cargo fmt --check`; `cargo clippy -p <crate> --all-targets -- -D warnings`;
  `cargo test -p <crate>`; `npm test` for TS packages; `source source_me.sh && pytest tests/`.
- Integration gate: `source source_me.sh && python3 local_stack.py validate`; M8 curl lane.
- Browser gate: one-time built-browser walkthrough of representative temporary inputs passes.
- Review gate: reviewer agent per merged work package. A broad security review occurs after the
  integrated implementation and before final changelog entry. Human commit is outside the plan's
  completion condition; the plan completes with a green working tree and changelog.

## Test and verification strategy

- Primary evidence: one-time data-driven E2E through the curl and browser lanes. Temporary inputs
  supply minimal connected replay evidence, so no human types answers.
- Boundary tests: M11 list; few and strong per `docs/PYTEST_STYLE.md`.
- Live PostgreSQL tests: M6.
- Bridge unit test with an inline minimal DOM/form contract (M9), runnable without a renderer.
- One-time visual/DOM evidence (M12) verifies baseline stylesheet delivery and cascade order with
  a representative authored override. It is not a permanent visual compatibility gate.
- The generic probe is a maintainer debug harness for future PG authoring problems; its inputs and
  results remain temporary.
- Permanent tests cover only the opaque boundary: representative document-to-score behavior,
  ordered duplicate pair payloads, reserved internal-field separation, and the generated-image
  route. They do not assert corpus inventory, current PG field names or ordering, or external
  fixture state. One-time behavior evidence is removed at closeout.
- Failure semantics: a gate failure blocks dependent milestones only; independent lanes continue.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Sandbox breaks PG JS | Students cannot answer | C1 console errors | tester | Rule C selects C2 |
| Renderer emits credential inputs in the embed document | Credential crosses the boundary | Embed contract failure | fork coder | Correct the embed template; rerun probe |
| Renderer outage during issuance | Attempt cannot start | Renderer timeout | adapter coder | Bounded refusal; no cache to mask it; local stack probe gates startup |
| Render-issued state affects one grade request | Wrong scores | E mismatch | adapter coder | Rule E selects E2 and records returned state |
| Bridge misses hidden mirrors | Grade differs | Probe grade mismatch | browser coder | Serialize full form |
| Lane file overlap (`issue.rs` vs `client.rs`) | Merge conflict | Two owners edit one file | manager | Exclusive file ownership in M4/M5 |
| Reintroducing control parsing "for one item" | Boundary erodes | New parser in adapter | reviewer | Generic backend path and M10 grep |
| Renderer image drift | Identity mismatch | OCI ID changes | maintainer | G rule; Local Stack State |

## Rollout and release checklist

- [ ] Fork working tree builds to `localhost/pg-renderer:reviewed`; OCI ID recorded.
- [ ] Fresh database provision replays the edited base schema.
- [ ] Live Demo serves representative temporary inputs through ordinary provisioning.
- [ ] One-time curl and browser lanes are green.
- [ ] Old projection symbols absent (M10 grep).
- [ ] Changelog entry written; `tests/_temp/` empty.

## Documentation close-out requirements

- Active plan / progress tracker: `docs/active_plans/active/webwork_opaque_backend_plan.md`
  with a status table per milestone; `git mv` to `docs/archive/` at closure.
- `docs/HUMAN_GUIDANCE.md`: replace lines 149-151 and fold line 193 into the "Question Backend
  ownership" section verbatim (text below); state that Question Type is immutable author-declared
  educational metadata on the Published Question Revision, used by PLE for search, filters, labels,
  and presentation, separate from interaction implementation. External-backend authors select it
  at create/publish; the backend remains responsible for rendering, interaction, interpretation,
  and grading.
- `docs/DESIGN_DECISIONS.md:949-966`: replace with "Question Backends own presentation and
  evaluation": Decision, Why, Consequence (two-layer boundary; WeBWorK transport; iMathAS and
  H5P adopt the boundary with their own transport later), Owner = HUMAN_GUIDANCE section,
  `crates/adapters/webwork`, `WEBWORK_PG_RENDERER_API_USAGE.md`.
- `docs/WEBWORK_PG_RENDERER_API_USAGE.md`: rewrite scope, request (`ple_embed`, `pleAssetBase`,
  `pleOrigin`), document contract, generic baseline stylesheet and cascade order, asset proxy,
  submission, state, per-attempt document, probe tool, verification; remove projection wording.
- `docs/QUESTION_BACKEND_CONTRACTS.md`: table row 57, section 125-170.
- `docs/SECURITY_MODEL.md:633-650`: boundary is envelope validation, CSP, sandbox, server-owned
  field override.
- `docs/DATABASE_STRUCTURE.md`, `docs/API_CONTRACTS.md`, `docs/CODE_ARCHITECTURE.md`,
  `docs/FILE_STRUCTURE.md`, `devel/DEVEL_README.md`: routes, tables, deleted modules, generic probe tool.
- docs/CHANGELOG.md entry: Additions (embed mode, routes, probe tool); Behavior changes
  (opaque boundary); Removals (projection, replay table, allow-lists); Decisions and Failures
  (projection approach set aside and why; C and E selections); Developer Tests and
  Notes (one-time representative evidence).
- Archive / closure notes: findings report stays in `reports/`; `tests/_temp/` emptied.

## Patch plan and reporting format

- Patch 1: M0 (WP-C1), M1 (WP-F1..F3), M3 (WP-M1) in parallel.
- Patch 2: M2 (WP-P1, WP-P2).
- Patch 3: M4, M5, M6, M7 in parallel.
- Patch 4: M8.
- Patch 5: M9, M10, M11 in parallel.
- Patch 6: M12, M13 draft in parallel.
- Patch N: M13 final, changelog rotation if needed, line-limit and link gates.
- Report per patch: work package IDs, gate outputs, reviewer verdict, decision rows relied on.

## Runtime decisions and follow-ups

- Runtime decisions: C and E are selected by the manager applying their written rules to
  `findings.json`; a reviewer agent re-derives each selection.
- Non-blocking follow-ups: post-submit feedback display; stateful continuation UI for Scaffold or
  compound PG progression; iMathAS and H5P adapters adopting the boundary; OPL breadth.

## HUMAN_GUIDANCE.md text (verbatim from user)

```text
## Question Backend ownership

Treat each Question Backend as the authority for the presentation and behavior of its Questions. A Question Backend owns presentation, interaction semantics, response interpretation, grading, partial credit, feedback, and backend-specific state. PLE owns authorization, immutable Question identity, assignment and attempt lifecycle, persistence, and recorded outcomes.

Apply this boundary consistently across Question Backends:

- WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
- H5P owns its runtime, interactions, state, and scoring.
- iMathAS owns its rendering, response model, answer evaluation, and scoring.
- PLE owns these responsibilities directly for PLE-native Questions because PLE is their Question Backend.

Use a backend-agnostic interface in PLE. Exchange backend-owned presentation, responses, and state as opaque values while standardizing the lifecycle and result information PLE needs. Conceptually:

render(question identity, attempt context)
    -> backend-owned presentation + opaque state
submit(opaque response, opaque state)
    -> score + feedback + updated opaque state

Keep backend-specific interaction knowledge within each Question Backend adapter. Model PLE-native response formats within the PLE-native backend. Design shared PLE abstractions around the common lifecycle and outcome contract rather than the interaction models of individual backends.

Use backend extensibility as an architectural acceptance criterion. Adding a new interaction already supported by an existing Question Backend should normally require only Question content or backend configuration, with no corresponding PLE question-type implementation. This allows rich backends such as WeBWorK, H5P, and iMathAS to retain their native capabilities while PLE provides a consistent assignment, attempt, and outcome lifecycle.
```
