# Frontend architecture

Peptidyle is a Solid single-page application backed by one Rust API and one
browser-safe Rust WebAssembly module. The frontend helps a student practice,
recover from ordinary failures, and understand format problems immediately;
it never decides whether an answer is correct.

The browser-interface requirements originate in the
[active implementation plan](active_plans/implementation_plan.md),
[SOLID_MODEL.md](SOLID_MODEL.md),
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md), and
[COLOR_CONTRAST_ACCESSIBILITY.md](COLOR_CONTRAST_ACCESSIBILITY.md). The durable
browser/server, payload, cache, failure, and object-delivery decisions live in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md),
[OBJECT_STORAGE.md](OBJECT_STORAGE.md), and
[SECURITY_MODEL.md](SECURITY_MODEL.md). This document explains how those
contracts meet inside the Solid application; it does not create a competing
wire or security contract.

## Primary flow

The first implemented reference slice follows this path:

```text
course list -> assignment overview -> current run -> select response
                                            |              |
                                            |              +-> local format hint
                                            `-> retry load       |
                                                               submit
```

The primary student goal is to answer the current question and continue
practicing. The response remains visible during validation, submission, and
recoverable failure. Continued practice remains available after completion.

## Route map

| Route                                                          | Surface                                    | Data contract                                                        |
| -------------------------------------------------------------- | ------------------------------------------ | -------------------------------------------------------------------- |
| `/`                                                            | Course list for signed-in role             | Course summaries                                                     |
| `/sign-in`                                                     | Passwordless account entry                 | Uniform email start or usernameless passkey                          |
| `/auth/email/complete`                                         | Canonical email sign-in completion         | One-time browser-bound fragment secret                               |
| `/auth/account/email/complete`                                 | Verified account-email replacement         | Current account session plus one-time secret                         |
| `/course-invitations/redeem`                                   | Learner invitation claim                   | Account session plus one-time invitation                             |
| `/account/security`                                            | Passkey and account-email management       | Account-owned credential projections only                            |
| `/courses/:courseId`                                           | Assignments with progress and run counts   | Cursor-paged assignments and summary rows                            |
| `/courses/:courseId/assignments/:assignmentId`                 | Assignment overview and run history        | Safe Question-ID item summaries and runs                             |
| `/runs/:runId`                                                 | One-question-at-a-time attempt loop        | Run screen query and response widget                                 |
| `/runs/:runId/summary`                                         | Run outcomes and continued-practice entry  | Per-question disclosed feedback and run score                        |
| `/library`                                                     | Shared problem browser                     | Cursor-paged facets and catalog results                              |
| `/curriculum`                                                  | Reusable curriculum workspace              | Revisioned private Blueprints and public Alpha summaries             |
| `/curriculum/:curriculumRef`                                   | Reusable curriculum detail                 | Answer-free definition inspection, editing, reuse, and Alpha fork    |
| `/workspace`                                                   | Instructor drafts                          | Tenant-owned workspace summaries                                     |
| `/workspace/:workspaceId`                                      | Draft editor, validation, and WASM preview | Draft and capability violations                                      |
| `/instructor/courses/:courseId/assignments/:assignmentId/edit` | Assignment policy editor                   | Assignment and capability validation                                 |
| `/instructor/courses/:courseId/gradebook`                      | Summary-row gradebook                      | Student assignment summaries only                                    |
| `/instructor/courses/:courseId/appearance`                     | Course theme and entry banner settings     | Revisioned safe appearance projection                                |
| `/instructor/courses/:courseId/students`                       | Roster, invitations, import, grade export  | Revisioned Instructor-only roster projection                         |
| `/instructor/courses/:courseRef/curriculum`                    | Curriculum adoption and import maintenance | Answer-free previews, typed apply receipts, and recovery projections |

`src/routes.ts` is the executable copy of this table. It also provides a
catch-all not-found route, which is infrastructure rather than a product
surface.

## Component and ownership tree

```text
API runtime provider
`-- WASM boundary and splash state
    `-- Router
        `-- App shell and route error boundary
            `-- Route resource
                `-- Question-renderer error boundary
                    `-- Response widget
```

The API runtime and WASM facade are instantiated once in the composition root.
Routes consume them through narrow context hooks. The shipped browser uses the
production `ApiClient` and same-origin transport; browser-free fixtures belong
only to narrow decoder and serialization tests.

Every browser build is the production build. The disposable HTTPS gateway
serves `dist/` with seeded production authentication, so the browser bundle
contains no local credential form, alternate credential transport, or alternate API
client.

## Client contract

| Concern         | Contract                                                                                                |
| --------------- | ------------------------------------------------------------------------------------------------------- |
| API access      | Every route calls the typed `ApiClient`; transport stays one file deep                                  |
| Queries         | `query` owns cache identity; `createAsync` owns route pending and result state                          |
| Mutations       | Typed methods carry idempotency keys and return explicit success or failure data                        |
| Pagination      | List methods accept or return cursors; no offset appears in the client                                  |
| Test seams      | Browser-free decoder and serialization tests use literal bounded inputs; they are not a browser runtime |
| Generated types | Rust-owned browser-safe types live under ignored `generated/api/`                                       |

`listProblems(cursor)` returns `CatalogProblemSummary` hot metadata rather
than full question payloads. `listTaxonomy(cursor)` uses the same bounded page
shape. `getCatalogProblemDetail(questionId)` is the separate safe exact
Question-ID projection; it excludes answer/response definitions, source
locators, grading material, and internal publication evidence, while browse
does not load those private fields for every row.

`listCourses(cursor)` returns Rust-owned `CourseSummary` values carrying the
signed-in user's effective course role. `listAssignments(courseId, cursor)` is
typed with `CourseId` and returns Rust-owned `AssignmentSummary` values whose
ordered item summaries contain Question IDs and safe display metadata. The
client uses focused add, remove, and revision-checked replacement methods for
item identity changes; it never sends an internal publication pair. The course
API verifies direct course membership before either list is returned. `Sysadmin`
is a platform role, not ambient authority over a course or its FERPA records.

`startRun(assignmentId)` sends `{ assignmentId }` to the run route rather than
encoding the assignment in a tenant-selecting path. `listRuns(enrollmentId,
cursor)` and `listAttempts(runId, cursor)` preserve the cursor-only history
contract. `submitResponse(attemptId, response, idempotencyKey)` sends the key in
the dedicated header; a retry must reuse both the same response and key so the
server can return the first committed result without grading twice.

`assetUrl(assetId)` returns only the stable same-origin
`/api/assets/{assetId}` route. The browser does not receive a bucket key and
does not retain a signed URL. The production route redirects globally public
content to its immutable CDN path. Protected content first requires a no-store
same-origin POST that returns one bounded delivery URL. Browser acceptance
exercises this production delivery route through the HTTPS gateway.

The real transport in `src/api/http_client.ts`
uses the same `ApiClient` interface, sends same-origin credentials with
`no-store`, and decodes every successful JSON body from `unknown` through
field-by-field runtime decoders. It rejects malformed IDs, timestamps, numeric
ranges, discriminants, and nested records before application code sees them.

`getRunScreen(runId)` composes the run, active attempt, enrollment, assignment,
course, and exact immutable question lookup. It verifies their ID and tenant
relationships instead of assuming that independently valid responses belong
together. List traversal remains cursor-only and rejects a repeated cursor.

## Curriculum adoption flow

`src/pages/curriculum_adoption_live_page.tsx` composes the authorized course route with
`src/features/curriculum_adoption/`. `CurriculumAdoptionPage` presents one staged Instructor
workflow for Blueprint/Alpha selection, Blueprint or Alpha instantiation, course rollover,
whole-course term shift, and import inspection. `AlphaForkAction` composes the independent-copy
proposal into the public Alpha detail route. `createCurriculumAdoptionClient` is the single
same-origin client for these operations; each preview is `no-store`, answer-free, and bound to the
server's source, destination, and revisions.

Apply receives only the exact eligible preview and a retry-safe idempotency key. The server owns
calendar-day and local-wall-clock schedule resolution in the target IANA zone, reports DST gaps or
ambiguities for visible correction, and returns typed recovery when issued work or revision drift
blocks an in-place change. Import maintenance can fast-forward an untouched assignment or create a
new source-derived draft when the destination diverged; it never silently overwrites teaching state.
Completed operations return immutable receipt evidence and the page can request receipt-led
reconciliation. Browser state preserves the visible choices needed to correct a failed proposal,
while authority remains in the API and `CurriculumAdoptionStore`.

## Learner attempt boundary

The run page owns one controlled learner-response state machine. It keeps the
editable response and one idempotency key in session storage only for the
current tenant/run/attempt. It removes the buffer after a known success or a
schema-invalid restore, and revalidates a restored response against the exact
issued definition before showing it. It never stores envelopes, feedback
awaiting release, provider state, or answer-bearing material. The machine
calls the response widget only for format-ready responses; it does not infer
correctness, partial credit, or the next question.

`ResponseWidget` is an exhaustive dispatcher over the browser-safe
`ResponseDefinition` vocabulary. Each response-family controller owns native
semantics, local input state, format-status reporting, and its documented
keyboard extension. It does not own a grading rule. The file-upload controller
is intentionally a visible fail-closed unavailable state until the
tenant/learner/attempt-bound upload-capability contract is implemented. The
external-tool controller can request only a same-origin PLE broker path and
treats frame readiness as presentation state, not as a grade.

The current submit request remains attempt-addressed and idempotent. Its typed
`StudentResponse` still carries `kind`; the server revalidates that shape from
the issued attempt, so `kind` is not browser grading authority. The accepted
target contract removes that redundant discriminant from the response wire,
uses an attempt-specific presentation digest plus rendered-item IDs, and lets
the server choose the strict family decoder from the attempt record. The exact
current-versus-target distinction and family payloads are defined in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md); do not duplicate
or quietly widen them in a component.

After a response commits, the browser may retain one answer-free prefetched
successor in memory. It accepts that envelope only when its predecessor, run,
position, immutable version, seed, and rendered hash exactly match the
server's committed successor receipt. A mismatch, decoder failure, network
failure, or route teardown drops the speculation and reloads the ordinary run
screen. Prefetch starts no timer, has no durable browser storage, and warms at
most 12 same-origin image assets. Timed/exam early-disclosure policy remains a
server-route decision. The active implementation plan owns the authoritative
refusal and recovery rules.

The frozen course-appearance boundary is Rust-owned under
`crates/question_model/src/course_appearance.rs`. `CourseAppearance` contains
only one closed `CourseThemeId`, an exact decimal-string
`CourseAppearanceRevision`, and an optional `CourseBannerPresentation` whose
opaque ID resolves through the same-origin asset route. It cannot carry a
bucket, object key, checksum, filename, source bytes, upload metadata, signed
URL, answer, or grading data. `CourseAppearanceUpdate` is the complete atomic
body: it selects a theme and explicitly keeps, removes, or replaces the
banner. Course identity comes only from the authenticated route and the
expected revision comes only from the strong `If-Match` header.

The production transport is one no-store `GET` and compare-and-swap
`PUT` at `/api/courses/{courseId}/appearance`, plus an author-only candidate
upload at `/api/courses/{courseId}/appearance/banner-candidates`. Candidate
upload returns only `CourseBannerCandidateReceipt`; candidate bytes never use
the delivery route. A current `CourseBannerId` is protected media and therefore
uses body-free `POST /api/assets/{id}/delivery`; its persistence owner rechecks
and audits the exact current authorization pointer before signing. Public
question assets retain `GET /api/assets/{id}`, which can only redirect an active
immutable public object. A pending public asset is deliberately unavailable,
not a browser retry or object-key fallback.

`src/features/course_appearance/course_appearance_page.tsx` owns the working instructor form over a
pure draft model and narrow repository. It uses native named theme radios, one raw raster file input,
explicit decorative/informative alt state, exact wide/narrow previews, one atomic save, and a
preserved-draft conflict reload. The selected local file lives only in component memory; its object
URL is revoked through `onCleanup`, and neither browser storage nor the JSON mutation receives the
filename or bytes. `course_entry_identity.tsx` consumes the same already-authorized route context and
renders the text course title plus one optional current banner only on the course-entry page. A null
or removed banner creates no learner image or empty frame.
The server normalizes every accepted upload to one 1200 by 328 pixel WebP. The
course entry and both settings previews preserve that intrinsic aspect and
only scale the same derivative down with CSS; the browser never stretches,
recrops, or selects a different banner rendition.

`src/features/course_appearance/course_theme_scope.tsx` owns the one pre-render course subtree.
Course-ID routes load `CourseRouteData` once; run attempts reuse `RunScreenData.course`; and run
summaries reuse the server-derived safe course projection. The provider does not accept a
browser-supplied course identity for a run, does not issue a post-render theme-only learner fetch,
and renders no course subtree until its projection is available. Route-keyed ownership removes the
old wrapper across course/global navigation. The persistent header, Library, Workspace, authored
scientific content, and semantic success/danger states stay outside theme projection.

`theme_catalog.ts` exhaustively maps the 15 generated IDs to three decorative anchors and measured
derived text/action/link/focus/surface tokens. Grass is the default. Its Roosevelt-inspired anchors
are `#BDDEB1`, `#73C167`, and `#008852`; raw `#008852` remains decorative because it does not meet
the house text threshold, while the derived action and link colors do. An unknown ID throws rather
than substituting another course's appearance.

## WebAssembly facade

`src/wasm/index.ts` is the only browser import boundary for generated
`wasm-bindgen` glue. It loads the same-origin module once, verifies its runtime
shape, initializes it, and converts JSON strings into a typed key-free format
report. No component calls a raw snake-case export. The module is not granted
cookies, tenant identity, object storage, launch state, provider credentials,
answer keys, grading envelopes, or a network authorization role; server
results remain authoritative.

The facade presents five browser-style lower-camel-case operations:

```text
validateResponseFormat(definition, response) -> ResponseFormatReport
timerVerdict(evaluation) -> TimerVerdict
validateAssignmentConfig(config) -> CapabilityViolation[]
previewNativeDraft(request, seed) -> NativeDraftPreviewResult
verifyPresentationDescriptor(envelope, assets, digest) -> match | mismatch | unavailable
```

If the generated module cannot import, has the wrong export shape, or fails to
initialize, the shared facade enters `serverFallback` mode and the root renders
one persistent degraded-mode status. Format validation, timer verdicts, and
assignment-capability validation then call their injected typed API fallbacks:
`/api/validation/response-format`, `/api/validation/timer`, and
`/api/validation/assignment-capabilities`. Native-draft preview has no server
fallback: it returns `{ kind: "unavailable", backend, capability:
"offlinePreview" }` for the requested source backend. Thus a student may
continue with a round trip for the three validation operations, while an
instructor sees preview availability truthfully. There is no local grading
fallback.

The authenticated `/api/validation/response-format`, `/timer`, and
`/assignment-capabilities` routes delegate to the same key-free domain
functions and bound request bodies. These are browser fallback calculations,
not authoritative decisions: publication re-resolves stored question and
backend records, run timing uses server-owned timestamps, and correctness
remains in the server-only grading path.

Presentation verification is deliberately different from local format help.
It hashes only the public envelope, selected public asset bindings, and the
server-issued digest; it neither reads an answer nor authenticates a request.
The browser facade can expose `match`, `mismatch`, or `unavailable` when Wasm
is degraded. Current run acceptance relies on the strict HTTP decoder and
receipt/prefetch descriptor match, while server persistence and reproduction
remain authoritative. The verification operation is therefore a safe browser
diagnostic seam, not permission for a component to accept, reject, or grade a
submission. The planned presentation-digest submission boundary is specified
in [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

Course appearance is data projection and route-scoped styling, not local
computation. It adds no Wasm export and does not change the generated-module
allowlist or the answer-free dependency closure.

## Persistence boundaries

| Browser store    | Allowed contents                                                                                                      | Clear condition                       |
| ---------------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| `localStorage`   | Nonessential preferences only after the applicable consent path permits them                                          | Consent withdrawal or explicit reset  |
| `sessionStorage` | In-progress response only when classified as necessary for explicitly requested run recovery; otherwise consent-gated | Submit success, run exit, or sign-out |
| Neither          | Session tokens, answer keys, grades, or feedback not yet disclosed                                                    | Never stored                          |

Course appearance also belongs in neither browser store. The server-owned
projection is loaded through course route data and cleared with its course
scope; `localStorage` and `sessionStorage` never become an appearance source
of truth.

Session identity is carried by an `HttpOnly` cookie that browser JavaScript
cannot read. It is host-only and has no `Max-Age` or `Expires` attribute; the
backend applies bounded expiration and revocation. See the durable policy in
[`HUMAN_GUIDANCE.md`](HUMAN_GUIDANCE.md#authentication-storage-and-compliance).

## Errors, focus, and recovery

- Loading keeps the shell visible and names what is loading.
- Empty states explain the next available action.
- Route failures preserve navigation and provide a retry control.
- Question-render failures preserve the run shell and timer.
- Submission failure preserves the selected response and offers manual retry
  with the same idempotency key.
- Session renewal returns to the same run and question.
- Feedback moves focus to its heading, then the advance control; the next
  question focuses its first response control.

Error messages name the failed operation and the next action. A generic
"something went wrong" message is not an accepted state.

## Accessibility baseline

The reference widget and every later response widget must meet these testable
conditions on student routes:

- semantic fieldset, legend, labels, and a nonempty accessible name;
- a primary platform path where Tab and Shift+Tab move focus, Space selects or
  activates, and the explicit Submit answer button completes the response;
- separately scoped widget extensions for native radio arrows,
  multiple-answer arrow focus, digits 1-9, response-input Enter-to-submit, and
  Escape; none replaces the visible platform path or native text editing;
- ordering controls that work with Tab plus Space, with separately tested
  Up/Down Arrow movement that preserves focus and announces the new position;
- labeled text fields for every multi-blank slot, native radio groups for each
  matching prompt, and a labeled candidate-region radio/checkbox list as the
  primary no-mouse HOTSPOT path;
- validation changes announced through a polite live region;
- `aria-invalid` paired with visible explanatory text;
- compact primary response targets at least 44 CSS pixels tall;
- visible focus indicator with at least 3:1 non-text contrast;
- text contrast at the repository house target of 5.5:1;
- correct and incorrect states combine text and iconography with color; and
- a genuinely responsive student layout: best at the canonical 1280 by 800 laptop window, fully
  usable at the representative 800 by 1280 portrait target, and compatible with a narrow-phone CSS
  viewport without horizontal overflow or inaccessible controls.

Instructor and Sysadmin routes are desktop-only for visual evidence. Their canonical acceptance
canvas is 1280 by 800 CSS pixels in a 16:10 profile, where information density and the complete
workflow take precedence. Student evidence retains the separate maintained responsive profile policy.

Responsive layout belongs to CSS Grid and Flexbox with complementary media and container queries.
SolidJS owns state only when a responsive interaction, such as an actually-needed disclosure menu,
requires it. Do not add a layout or menu dependency solely to replace these browser capabilities.

The primary action is "Submit answer." It remains visually dominant but calm.
Correctness and teaching feedback are absent until the server response permits
them.

## Security rules

- The client bundle and generated types contain no answer-bearing type.
- State-changing requests use the explicitly typed same-origin method and
  canonical-origin checks. There is no embedded `SameSite=None` session mode.
  LTI is a future separate launch and CSRF design, not a cookie-policy switch.
- Supplied markup is sanitized on the server before becoming a render block.
- The content security policy allows scripts only from the app origin, permits
  WebAssembly instantiation, disables object embedding, and limits frame
  ancestors to configured LMS origins.
- Asset markup carries internal IDs; the API resolves authorized delivery.
- Browser logs contain identifiers and error codes, not names, response text,
  grades, keys, or undisclosed feedback.
- The external-tool view first POSTs to create a server-held launch and then
  displays only its inert same-origin shell. Its sandboxed activity may use
  the narrow `Origin: null` exception only with both ordinary and launch
  cookies; no other browser mutation may rely on that exception.

## Validation gates

The evidence below names what each lane proves. The canonical browser
product-behavior claim comes from production `dist/` on the disposable real
stack. Screenshots use the same owner and carry production-origin provenance.

| Evidence lane                           | Current evidence                                                                                                                  | What it proves                                                                                                                          | Boundary                                                                                     | Status                                      |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ------------------------------------------- |
| Canonical production browser            | `run_playwright_tests.sh` -> `tests/e2e/e2e_browser_suite_owner.py` -> `playwright.config.ts` -> `tests/playwright/e2e/*.spec.ts` | Real visible journeys use the production bundle, HTTPS gateway, authentication, API, database, object store, worker, and renderer.      | One fixed owner runs scenarios serially; a focused selection receives a fresh stack.         | Required browser evidence.                  |
| Visible browser-Wasm proof              | `tests/playwright/e2e/instructor_authoring.spec.ts` through the canonical owner                                                   | The instructor scenario sees `data-runtime-mode="wasm"` and the visible "Response tools are running locally in this browser." status.   | Source and unit checks cannot prove that the production bundle initialized Wasm in Chromium. | Required for browser-Wasm claim.            |
| Browser-free unit and contract evidence | `src/wasm/index.ts`, `src/wasm/context.tsx`, `src/main.tsx`, and `tests/test_frontend_contract.mjs`                               | The facade has one shared loader, typed server fallbacks, unavailable-only preview behavior, and no correctness field in local reports. | It does not replace the production-browser proof.                                            | Complete as narrow implementation evidence. |
| Service-only oracles                    | Browser-free WebWork and replica restart commands                                                                                 | Renderer/cache/outage, durable replay, database/RLS, and lifecycle claims that are not visible UI journeys.                             | These commands do not launch Chromium or import a browser configuration.                     | Retained with explicit boundaries.          |

- Node tests freeze the route map, client behavior, and absence of
  answer-bearing generated names.
- TypeScript compilation checks the client, generated fixture projection, WASM
  facade, and response-widget props without `any` or unchecked casts.
- The focused keyboard evidence and optional future usability evaluation are in
  [`ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md`](ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md).
- The durable student interaction requirements, including future MATCH, FIB,
  MULTI-FIB, and HOTSPOT behavior, are in
  [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md).
- The live renderer and browser acceptance commands, prerequisites, and
  stop conditions are in
  [`active_plans/workstreams/webwork_shipped_integration.md`](active_plans/workstreams/webwork_shipped_integration.md).
- Palette tools record measured contrast for source colors in
  `docs/PALETTE_CONTRAST_AUDIT.md`.
