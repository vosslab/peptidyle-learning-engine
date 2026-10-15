# Solid model

This document is the review contract for Solid code in Peptidyle. Components run once. Reactive
reads belong in JSX, memos, resources, and effects; changing state updates only subscribers that
read it. It records the cross-cutting owners and the implemented reference slices below, not a
complete inventory of every route-local signal.

## Reactivity map

| State                                | Primitive                                    | Owner                                   | Update contract                                                                                                                                                                             |
| ------------------------------------ | -------------------------------------------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Session identity and roles           | Context containing a signal accessor         | `SessionProvider`                       | Browser-visible state contains identity and roles only. Authentication events replace it; server authorization remains authoritative.                                                       |
| Shared key-free Wasm facade          | Resource and context                         | `WasmRuntimeProvider`                   | One startup resource loads the facade before Question Response Controls render. Consumers read context; fallback validation stays behind the same facade.                                   |
| Route scope data                     | Route resource and context                   | `RouteScopeProvider`                    | One route-owned query loads authorized Course or Attempt data for the current route. Descendants consume it without a second transport request.                                              |
| Resolved appearance                  | Memos, resource, and render effect           | `AppearanceOwner`                       | The owner resolves Course, personal Instructor, and default Themes with the independent display mode, then writes the result to `<html>`.                                                    |
| PLE Question JSON Draft Question     | Signals, memos, effects, and `<For>`         | `createQuestionJsonEditorController` in [question_json_editor_controller.ts](../src/features/ple_question_json_authoring/question_json_editor_controller.ts) | The page creates the controller, which coordinates private source, Edit Number, review, status, and locks. Reducers replace the explicit editor state; derived source/errors stay memos. |
| Native author preview                | Saved Draft `DraftPreviewPanel` and private instructor check | `question_json_editor_page.tsx`          | The editor uses the shared server-backed Draft preview through the existing autosave workflow. `PleQuestionJsonInstructorAnswerCheckView` remains a separate private check for the Instructor. |
| iMathAS Question Backend Launch      | Signals, refs, effect, and lifecycle cleanup | `ImathasQuestionBackendResponseControl` | The browser receives a same-origin launch path only after activation. Readiness is presentation state; it cannot provide a score, backend identity, or grading input.                       |
| Route-local screens                  | Signals or router `createAsync` resources    | Owning route                            | Each route owns its pending, ready, error, and retry state. Use a resource for route-backed async data and signals for local interaction state.                                             |

Use a signal for one independently changing scalar or discriminated state. Use a resource for an
async owner whose pending/error state belongs to the route or provider. Use a store only for a
nested tree that needs path-granular updates; the current implemented authoring slices use typed
reducer state and focused signals instead of a catch-all application store. Derive values with a
plain function or `createMemo`; do not write derived state from an effect.

## Component and prop rules

- A component body performs setup once. It is not a render loop.
- Read `props.value` at the use site. Do not destructure reactive props.
- Use `mergeProps` for defaults and `splitProps` for safe forwarding.
- Render identity-bearing rows, choices, and import reports with `<For>` so focus and local DOM
  state follow the item when order changes.
- Use `<Index>` only when a stable position, rather than item identity, owns the DOM slot.
- Use `<Show>` or `<Switch>` for reactive branches. Do not hide state changes in a one-time
  function-body conditional.
- Use `createMemo` for derived editor errors, selected-report counts, conversion eligibility, and
  similarly reused calculations. Effects coordinate an external side effect, such as focus, never
  duplicate derived state.

## Lifecycle contract

One-time DOM setup belongs in `onMount`. Every timer, subscription, listener, object URL, or other
browser resource registers teardown with `onCleanup`. An `onMount` callback never returns a cleanup
function.

The app owns one PLE Wasm loader resource at startup. Route components read the shared facade from
context. The separate QPM conversion package loads only when Import opens. The same ownership rule applies to the API
runtime and session bootstrap. `ImathasQuestionBackendResponseControl` installs the same-origin `message` listener during component initialization and removes it
on disposal; a request counter rejects late launch results after a new attempt or disposal.

## Routing and async data

`@solidjs/router` owns navigation. Links use `<A>`; imperative navigation is reserved for a state
transition such as creating a workspace or entering an Assessment Attempt. Route-backed reads use the router's
`createAsync` queries where a shared route cache is useful, while the private PLE Question JSON workspace editor uses a
keyed `createResource` for its private draft read.

`RouteScopeProvider` loads only the current route's authorized Course or Attempt data. It supplies
that data to page content and to `AppearanceOwner`; it does not apply appearance. `AppearanceOwner`
resolves the Course Theme where the route supplies one, otherwise an Instructor's personal Theme or
the default, and applies the resolved Theme and display mode to `<html>`. This keeps one document
appearance owner while Course route data remains authorized and route-local.

The workspace editor is an implemented bounded Instructor authoring capability. Its route guard
and server boundary resolve the route-selected private Draft UUID under
Authoring Workspace access before returning any draft; the current routes are listed in
[API_CONTRACTS.md](API_CONTRACTS.md). The answer-free preview remains separate, and private author
source does not enter Student components, URLs, browser storage, or diagnostics.

Question import loads vendored QPM WASM when the Instructor opens Import from My Draft Questions.
QPM parses supported files locally and returns Native JSON and conversion warnings. The browser
checks each item's display content for images, skips those items until inline-image binding is
available, and saves the remaining items sequentially through the existing Draft API. The results
list retains created Draft links, server rejections, skipped reasons, and individual request
failures. Each Draft uses the ordinary editor, preview, and publication workflow.

Each route has an error boundary through the app shell so a screen failure leaves navigation usable.
Documented slices must show loading, error/recovery, empty where applicable, and ready states rather
than treating a resource as always resolved.

## Question Response Control state

The ordinary reference Question Response Control owns only Student input, save state, and browser-safe format status. Its phase is a
discriminated union:

```text
idle -> validating -> ready -> saving -> saved
                   `-> invalid       `-> failed -> saving
```

Selecting a value updates the controlled input and calls the shared Wasm format validator. Saving
persists a complete working response without exposing a grading outcome. A
monotonically increasing validation request number keeps a slow older result from replacing a newer
result. No client state guesses correctness.

The iMathAS Question Backend control is also implemented. It follows its own local phase progression:

```text
idle -> loading -> awaitingReady -> ready -> saving -> saved
                 `-> failed
```

Opening the iMathAS Question Backend asks the PLE
API for a protected same-origin launch path. The iframe is sandboxed; its ready message must come
from that origin and that iframe. Backend completion saves the Question
response; the Student still submits the whole Assessment Attempt through the
ordinary PLE flow.

## Server and browser boundary

| Browser owns                                                                     | Server owns                                                                                                                          |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Navigation, display, controlled input, local buffering, and answer-free previews | Authentication, authorization, authenticated Account context, private drafts, and durable revisions                                  |
| Appearance presentation from authorized route data and Account settings           | Course identity, Account appearance persistence, Banner-object authorization, and independent mutation decisions                     |
| Question authoring state and local question-bank selection                         | Complete Question Source persistence, publication review, backend evaluation, and feedback disclosure                                |
| QPM parsing and conversion, warnings, per-question image checks, and import results | Existing Draft authorization and storage; Native JSON acceptance in preview and publication |
| iMathAS iframe presentation and same-origin readiness status                     | iMathAS Question Backend Launch authorization, configuration, correlation, verification, correctness, and grade recording            |
| Countdown display reconciled from server data                                    | Deadline and late-submission verdict                                                                                                 |

The Assessment Question owns points; the Assessment owns Attempt limits,
timing, and disclosure. The Question Backend owns credit, and the whole
Assessment Attempt owns submission.
The same browser/server ownership split applies to every registered Question Backend. QPM emits
Native JSON before the existing Draft Question and publication operations.

Everything crossing the browser boundary is JSON-serializable except browser-local `File`, DOM ref,
and object-URL state, which never crosses it. The generated client surface derives from
`crates/question_model`; answer-bearing `crates/grading` types never enter it. The iMathAS Question Backend
launch DTO is intentionally only a same-origin path, never a provider URL, token, score, or source.
Protected database functions remain narrow and subject to
forced RLS; see `docs/DATABASE_AUTHORIZATION.md#row-level-security` and
`docs/ADAPTER_DEVELOPMENT.md`.

## Reactivity verification

Slice tests must prove observable behavior rather than only construction:

- changing one selected response updates its control and live status without recreating the Question Response Control;
- `<For>` keeps identity for choices, draft rows, and import results;
- a course route loads one authorized `CourseRouteView` and drops it on navigation;
- the native preview remains answer-free until an instructor explicitly opens the protected check;
- imported questions use the ordinary saved Draft preview, while each request failure leaves
  earlier results intact and permits later saves;
- an external iframe accepts readiness only from its own same-origin frame and cleanup removes its
  listener; and
- application disposal leaves no timer, listener, or object URL owned by these slices.

Later slices reuse these contracts and add focused tests for route-resource retry and resource
failure behavior. Do not weaken the server/browser or private-source boundaries merely to simplify
a component fixture.
