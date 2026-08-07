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

| Route | Surface | Data contract |
| --- | --- | --- |
| `/` | Course list for signed-in role | Course summaries |
| `/courses/:courseId` | Assignments with progress and run counts | Cursor-paged assignments and summary rows |
| `/courses/:courseId/assignments/:assignmentId` | Assignment overview and run history | Immutable problem/version references and runs |
| `/runs/:runId` | One-question-at-a-time attempt loop | Run screen query and response widget |
| `/runs/:runId/summary` | Run outcomes and continued-practice entry | Per-question disclosed feedback and run score |
| `/library` | Shared problem browser | Cursor-paged facets and catalog results |
| `/library/:problemId/versions/:versionId` | Problem detail and version lineage | Immutable published version |
| `/workspace` | Instructor drafts | Tenant-owned workspace summaries |
| `/workspace/:workspaceId` | Draft editor, validation, and WASM preview | Draft and capability violations |
| `/instructor/courses/:courseId/assignments/:assignmentId/edit` | Assignment policy editor | Assignment and capability validation |
| `/instructor/courses/:courseId/gradebook` | Summary-row gradebook | Student assignment summaries only |

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

| Concern | Contract |
| --- | --- |
| API access | Every route calls the typed `ApiClient`; transport stays one file deep |
| Queries | `query` owns cache identity; `createAsync` owns route pending and result state |
| Mutations | Typed methods carry idempotency keys and return explicit success or failure data |
| Pagination | List methods accept or return cursors; no offset appears in the client |
| Mocking | `createMockApiClient` satisfies the same interface with no server process |
| Generated types | Rust-owned browser-safe types live under ignored `generated/api/` |

The mock client verifies exact serialized handler responses against the typed
fixture values before returning them. This keeps a server-free UI useful
without an unchecked JSON cast. The real HTTP client later replaces that
mock-specific oracle with generated runtime decoders.

## WebAssembly facade

`src/wasm/index.ts` is the only browser import boundary for generated
`wasm-bindgen` glue. It loads the same-origin module once, verifies its runtime
shape, initializes it, and converts JSON strings into a typed key-free format
report. No component calls a raw snake-case export.

The facade presents browser-style lower camel case:

```text
validateResponseFormat(definition, response) -> ResponseFormatReport
```

If WebAssembly initialization fails, the facade delegates validation to the
typed server-format endpoint and reports one persistent degraded-mode status.
The student may continue with a round trip per validation. There is no local
grading fallback.

## Persistence boundaries

| Browser store | Allowed contents | Clear condition |
| --- | --- | --- |
| `localStorage` | Theme, sound, and reduced-motion preferences | Explicit reset |
| `sessionStorage` | In-progress response keyed by attempt ID | Submit success, run exit, or sign-out |
| Neither | Session tokens, answer keys, grades, or feedback not yet disclosed | Never stored |

Session identity is carried by an `HttpOnly` cookie that browser JavaScript
cannot read.

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
- keyboard selection with Tab and number keys, Enter to submit, and Escape to
  return to the run overview;
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
- Supplied markup is sanitized on the server before becoming a render block.
- The content security policy allows scripts only from the app origin, permits
  WebAssembly instantiation, disables object embedding, and limits frame
  ancestors to configured LMS origins.
- Asset markup carries internal IDs; the API resolves authorized delivery.
- Browser logs contain identifiers and error codes, not names, response text,
  grades, keys, or undisclosed feedback.

## Validation gates

- Node tests freeze the route map, mock/client behavior, and absence of
  answer-bearing generated names.
- TypeScript compilation checks the client, generated fixture projection, WASM
  facade, and response-widget props without `any` or unchecked casts.
- Playwright proves the mock-backed run screen, mouse and number-key selection,
  live format feedback, submission, route resolution, focus, and responsive
  layout over the built artifact.
- Palette tools record measured contrast for source colors in
  `docs/PALETTE_CONTRAST_AUDIT.md`.
