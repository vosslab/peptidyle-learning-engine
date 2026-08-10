# Frontend architecture

Peptidyle is a Solid single-page application backed by one Rust API and one
browser-safe Rust WebAssembly module. The frontend helps a student practice,
recover from ordinary failures, and understand format problems immediately;
it never decides whether an answer is correct.

The browser-interface requirements originate in the
[active implementation plan](active_plans/implementation_plan.md),
[docs/PLAYFUL_TRAINING_GAME_STYLE.md](PLAYFUL_TRAINING_GAME_STYLE.md), and
[docs/COLOR_CONTRAST_ACCESSIBILITY.md](COLOR_CONTRAST_ACCESSIBILITY.md).

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

| Route                                                          | Surface                                    | Data contract                                 |
| -------------------------------------------------------------- | ------------------------------------------ | --------------------------------------------- |
| `/`                                                            | Course list for signed-in role             | Course summaries                              |
| `/courses/:courseId`                                           | Assignments with progress and run counts   | Cursor-paged assignments and summary rows     |
| `/courses/:courseId/assignments/:assignmentId`                 | Assignment overview and run history        | Immutable problem/version references and runs |
| `/runs/:runId`                                                 | One-question-at-a-time attempt loop        | Run screen query and response widget          |
| `/runs/:runId/summary`                                         | Run outcomes and continued-practice entry  | Per-question disclosed feedback and run score |
| `/library`                                                     | Shared problem browser                     | Cursor-paged facets and catalog results       |
| `/library/:problemId/versions/:versionId`                      | Problem detail and version lineage         | Immutable published version                   |
| `/workspace`                                                   | Instructor drafts                          | Tenant-owned workspace summaries              |
| `/workspace/:workspaceId`                                      | Draft editor, validation, and WASM preview | Draft and capability violations               |
| `/instructor/courses/:courseId/assignments/:assignmentId/edit` | Assignment policy editor                   | Assignment and capability validation          |
| `/instructor/courses/:courseId/gradebook`                      | Summary-row gradebook                      | Student assignment summaries only             |
| `/instructor/courses/:courseId/appearance`                     | Course theme and entry banner settings     | Revisioned safe appearance projection         |

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
Routes consume them through narrow context hooks. Components do not import mock
fixtures directly; mock data stays behind the same client interface a future
HTTP transport implements.

## Client contract

| Concern         | Contract                                                                         |
| --------------- | -------------------------------------------------------------------------------- |
| API access      | Every route calls the typed `ApiClient`; transport stays one file deep           |
| Queries         | `query` owns cache identity; `createAsync` owns route pending and result state   |
| Mutations       | Typed methods carry idempotency keys and return explicit success or failure data |
| Pagination      | List methods accept or return cursors; no offset appears in the client           |
| Mocking         | `createMockApiClient` satisfies the same interface with no server process        |
| Generated types | Rust-owned browser-safe types live under ignored `generated/api/`                |

`listProblems(cursor)` returns `CatalogProblemSummary` hot metadata rather
than full question payloads. `listTaxonomy(cursor)` uses the same bounded page
shape. `getProblemVersion(problemId, versionId)` is the separate exact payload
lookup, so browse does not load prompts, response definitions, or private
source locators for every row.

`listCourses(cursor)` returns Rust-owned `CourseSummary` values carrying the
signed-in user's effective course role. `listAssignments(courseId, cursor)` is
typed with `CourseId` and returns Rust-owned `AssignmentSummary` values whose
ordered problems are exact immutable ID pairs. The course API verifies direct
membership or tenant-administrator authority before either list is returned.

`startRun(assignmentId)` sends `{ assignmentId }` to the run route rather than
encoding the assignment in a tenant-selecting path. `listRuns(enrollmentId,
cursor)` and `listAttempts(runId, cursor)` preserve the cursor-only history
contract. `submitResponse(attemptId, response, idempotencyKey)` sends the key in
the dedicated header; a retry must reuse both the same response and key so the
server can return the first committed result without grading twice.

`assetUrl(assetId)` returns only the stable same-origin
`/api/assets/{assetId}` route. The browser does not receive a bucket key and
does not retain a signed URL. The production route redirects globally public
content to its immutable CDN path and authorizes protected content before a
short-lived redirect. The mock handler serves fixture bytes directly as the
offline stand-in for that public CDN behavior.

The mock client verifies exact serialized handler responses against the typed
fixture values before returning them. This keeps a server-free UI useful
without an unchecked JSON cast. The real transport in `src/api/http_client.ts`
uses the same `ApiClient` interface, sends same-origin credentials with
`no-store`, and decodes every successful JSON body from `unknown` through
field-by-field runtime decoders. It rejects malformed IDs, timestamps, numeric
ranges, discriminants, and nested records before application code sees them.

`getRunScreen(runId)` composes the run, active attempt, enrollment, assignment,
course, and exact immutable question lookup. It verifies their ID and tenant
relationships instead of assuming that independently valid responses belong
together. List traversal remains cursor-only and rejects a repeated cursor.

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
the delivery route. A current `CourseBannerId` uses the existing
`/api/assets/{id}` route, whose persistence owner rechecks the exact current
course pointer before delivery.

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
report. No component calls a raw snake-case export.

The facade presents four browser-style lower-camel-case operations:

```text
validateResponseFormat(definition, response) -> ResponseFormatReport
timerVerdict(evaluation) -> TimerVerdict
validateAssignmentConfig(config) -> CapabilityViolation[]
previewNativeDraft(request, seed) -> NativeDraftPreviewResult
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
conditions:

- semantic fieldset, legend, labels, and a nonempty accessible name;
- keyboard selection with Tab, native radio arrows, multiple-answer arrow focus,
  Space, and optional number keys; Enter submits a ready response and Escape
  returns to the run overview;
- ordering controls that work with Tab plus Enter and with Up/Down Arrow while
  preserving focus and announcing the new position;
- validation changes announced through a polite live region;
- `aria-invalid` paired with visible explanatory text;
- at least 56 by 56 CSS pixels for primary response targets;
- visible focus indicator with at least 3:1 non-text contrast;
- text contrast at the repository house target of 5.5:1;
- correct and incorrect states combine text and iconography with color; and
- usable responsive layout at 320, 480, 768, and 1920 CSS pixels.

The primary action is "Submit answer." It remains visually dominant but calm.
Correctness and teaching feedback are absent until the server response permits
them.

## Security rules

- The client bundle and generated types contain no answer-bearing type.
- State-changing requests use same-origin JSON transport. Embedded LTI mode
  cannot treat `SameSite=None` as CSRF protection; the future LTI composition
  must add an origin-bound anti-CSRF mechanism before enabling that cookie
  policy in production.
- Supplied markup is sanitized on the server before becoming a render block.
- The content security policy allows scripts only from the app origin, permits
  WebAssembly instantiation, disables object embedding, and limits frame
  ancestors to configured LMS origins.
- Asset markup carries internal IDs; the API resolves authorized delivery.
- Browser logs contain identifiers and error codes, not names, response text,
  grades, keys, or undisclosed feedback.

## Validation gates

The evidence below names what each lane proves. Mock routes and dynamically
mounted fixtures are useful focused evidence, but neither is a live WebWork
acceptance claim.

| Evidence lane                          | Current evidence                                                                                                | What it proves                                                                                                                                                                | Boundary                                                                                                   | Status                                      |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| Built mock route tests                 | `tests/playwright/frontend_contract.spec.ts` and the student keyboard audit                                     | The compiled mock-backed application resolves product routes and completes the reference course-to-answer journey with PLE controls.                                          | It does not exercise the live API, private renderer, or upstream WebWork.                                  | Complete for the audited mock route.        |
| Dynamically mounted component fixtures | `tests/playwright/student_keyboard_accessibility.spec.ts` and `tests/playwright/external_tool_response.spec.ts` | Production Solid response components handle focused keyboard and broker interactions in isolated browser fixtures.                                                            | The fixture bundle is injected into a mock page, not mounted through a complete built route or live stack. | Complete for the named component behaviors. |
| Source and contract evidence           | `src/wasm/index.ts`, `src/wasm/context.tsx`, `src/main.tsx`, and `tests/test_frontend_contract.mjs`             | The four-operation facade has one shared loader, typed fallbacks for three operations, an unavailable-only preview fallback, and no correctness field in mock format reports. | Source and mock-contract checks cannot prove a generated module or a deployed server behaved this way.     | Complete as implementation evidence.        |
| Required live WebWork gate             | `tests/playwright/webwork_run.spec.ts` with `PLE_WEBWORK_LIVE_REQUIRED=1`                                       | A private live stack must prove the browser calls PLE only, remains answer-free, supports keyboard completion, and receives correct/incorrect outcomes through PLE.           | It requires the explicit private stack and credential inputs; the ordinary mock suite skips it.            | Pending required live execution.            |

- Node tests freeze the route map, mock/client behavior, and absence of
  answer-bearing generated names.
- TypeScript compilation checks the client, generated fixture projection, WASM
  facade, and response-widget props without `any` or unchecked casts.
- The focused keyboard evidence and its remaining human evaluation are in
  [`ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md`](ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md).
- The live renderer and browser acceptance commands, prerequisites, and
  stop conditions are in
  [`active_plans/workstreams/webwork_shipped_integration.md`](active_plans/workstreams/webwork_shipped_integration.md).
- Palette tools record measured contrast for source colors in
  `docs/PALETTE_CONTRAST_AUDIT.md`.
