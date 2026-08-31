# Solid model

This document is the review contract for Solid code in Peptidyle. Components run once. Reactive
reads belong in JSX, memos, resources, and effects; changing state updates only subscribers that
read it. It records the cross-cutting owners and the implemented reference slices below, not a
complete inventory of every route-local signal.

## Reactivity map

| State                                | Primitive                                     | Owner                    | Update contract                                                                                                                                                                                       |
| ------------------------------------ | --------------------------------------------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Session identity and roles           | Context containing a signal accessor          | `SessionProvider`        | Browser-visible state contains identity and roles only. Authentication events replace it; server authorization remains authoritative.                                                                 |
| Shared key-free Wasm facade          | Resource and context                          | `WasmRuntimeProvider`    | One startup resource loads the facade before widgets mount. Consumers read context; fallback validation stays behind the same facade.                                                                 |
| Course route identity and appearance | Route `createAsync` resource and context      | `CourseThemeScope`       | One route-owned query loads the authorized course projection for course, run, and summary paths. Descendants consume it without a second transport request.                                           |
| Course-theme CSS variables           | JSX-derived token functions                   | `CourseThemeScope`       | The scope applies tokens only below a course-owned route and disposes on pathname change. Global routes receive no course variables.                                                                  |
| Appearance settings                  | Independent signals plus pure model functions | `CourseAppearancePage`   | `current`, editable `draft`, selected `File`, preview URL, candidate receipt, phase, errors, and messages remain separate so a failure preserves local work.                                          |
| Native flat-question draft           | Signals, memos, effects, and `<For>`          | `FlatQuestionEditorPage` | The author editor keeps private source, revision, review, status, and locks local. Reducers replace the explicit editor state; derived source/errors stay memos.                                      |
| Student-equivalent author preview    | Signal and answer-free projection             | `FlatQuestionPreview`    | Choice selection is local only. The normal preview has no correct answer, feedback, grading, request, URL, or storage write; an explicit author-only panel may display the protected check.           |
| QTI profile review and conversion    | Signals, memos, effects, and `<For>`          | `QtiProfileImportPage`   | The selected archive remains component memory. The UI displays only the server's answer-free report, preserves it across refresh failure, and locks the visible draft during replacement and refetch. |
| External-tool launch                 | Signals, refs, effect, and lifecycle cleanup  | `ExternalToolResponse`   | The browser receives a same-origin broker path only after activation. Readiness is presentation state; it cannot provide a score, provider identity, or grading input.                                |
| Route-local screens                  | Signals or router `createAsync` resources     | Owning route             | Each route owns its pending, ready, error, and retry state. Use a resource for route-backed async data and signals for local interaction state.                                                       |

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

The app owns one Wasm loader resource at startup. Route components read the shared facade from
context; they do not instantiate modules independently. The same ownership rule applies to the API
runtime and session bootstrap. `CourseAppearancePage` owns its local preview URL and revokes it on
cleanup. `ExternalToolResponse` installs the same-origin `message` listener at mount and removes it
on disposal; a request counter rejects late launch results after a new attempt or disposal.

## Routing and async data

`@solidjs/router` owns navigation. Links use `<A>`; imperative navigation is reserved for a state
transition such as creating a workspace or entering an Assignment Attempt. Route-backed reads use the router's
`createAsync` queries where a shared route cache is useful, while the native workspace editor uses a
keyed `createResource` for its private draft read.

`CourseThemeScope` classifies only course-owned routes. It loads `courseScope(courseId)` for course
and instructor-course routes, `assignmentAttemptScreen(assignmentAttemptId)` for an attempt, and `assignmentAttemptSummary(assignmentAttemptId)` for a
summary. The context exposes the authorized course projection to the course entry identity, theme,
and appearance settings; the scope is below the persistent shell and therefore cannot leak a prior
course's CSS variables onto a global route.

The appearance form starts from that route projection but keeps its own editable draft and upload
candidate. Save revalidates the route-owned course query, allowing the outer scope to reflect the
new server revision without creating another appearance owner. A stale revision or request failure
does not discard the local file, theme, alternative text, or recovery message.

The workspace route is an implemented authoring slice, not a hypothetical preview. It authorizes
the instructor role, loads exactly the route-selected private draft, and mounts the native
flat-question editor. The student-equivalent preview uses the answer-free projection; the separate
instructor check is deliberately named and contained in the author surface. Private author source
does not enter student components, URLs, browser storage, or diagnostics.

QTI profile import is composed on the same workspace route above the native editor. A selected ZIP
is uploaded for server-side review, then the browser receives queued, processing, failed,
unsupported-profile, or answer-free ready-report state. The supported conversion profiles are
bounded; the browser does not parse ZIP/XML, choose a conversion result, or persist provenance.
Conversion requires a reviewed accepted item and a clean visible draft, then the route temporarily
makes the editor inert while the server replaces and the browser refetches that private draft.

Each route has an error boundary through the app shell so a screen failure leaves navigation usable.
Reference slices must show loading, error/recovery, empty where applicable, and ready states rather
than treating a resource as always resolved.

## Response-widget state

The ordinary reference widget owns only student input and browser-safe format status. Its phase is a
discriminated union:

```text
idle -> validating -> ready -> submitted
                   `-> invalid
           request `-> failed -> validating
```

Selecting a value updates the controlled input and calls the shared Wasm format validator. A
monotonically increasing validation request number keeps a slow older result from replacing a newer
result. No client state guesses correctness.

The external-tool variant is also implemented. It follows its own local phase progression:

```text
idle -> loading -> awaitingReady -> ready -> submitting -> submitted
                 `-> failed
```

Opening the tool first persists only the ordinary external-tool response marker, then asks the PLE
API for a protected same-origin launch path. The iframe is sandboxed; its ready message must come
from that origin and that iframe. The final submission remains the ordinary PLE response flow.

## Server and browser boundary

| Browser owns                                                                     | Server owns                                                                                                                |
| -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Navigation, display, controlled input, local buffering, and answer-free previews | Authentication, authorization, authenticated Account context, private drafts, and durable revisions |
| Course-theme presentation from an authorized route projection                    | Course identity, appearance revision, banner-object authorization, and conflict decisions                                  |
| Native author editing state and local QTI archive selection                      | Flat-source persistence, publication review, correctness, points, and feedback disclosure                                  |
| QTI report display, item selection, acknowledgement, and refetch handoff         | ZIP/XML parsing, bounded profile recognition, accepted-item evidence, conversion, provenance, and atomic draft replacement |
| External iframe presentation and same-origin readiness status                    | Broker launch authorization, provider configuration, correlation, verification, correctness, and grade recording           |
| Countdown display reconciled from server data                                    | Deadline and late-submission verdict                                                                                       |

Everything crossing the browser boundary is JSON-serializable except browser-local `File`, DOM ref,
and object-URL state, which never crosses it. The generated client surface derives from
`crates/question_model`; answer-bearing `crates/grading` types never enter it. The external-tool
launch DTO is intentionally only a same-origin path, never a provider URL, token, score, source,
or provenance record. The durable broker roles remain narrow and subject to forced RLS; see
`docs/DATABASE_AUTHORIZATION.md#row-level-security` and `docs/ADAPTER_DEVELOPMENT.md`.

## Reactivity verification

Reference-slice tests must prove observable behavior rather than only construction:

- changing one selected response updates its control and live status without recreating the widget;
- `<For>` keeps identity for choices, draft rows, and QTI report items;
- a course route loads one authorized theme projection and drops it on navigation;
- a course-appearance failure preserves the editable draft and revokes its preview URL on disposal;
- the native preview remains answer-free until an instructor explicitly opens the protected check;
- QTI conversion rejects a dirty or unavailable draft and leaves the editor locked only during the
  replacement/refetch handoff;
- an external iframe accepts readiness only from its own same-origin frame and cleanup removes its
  listener; and
- application disposal leaves no timer, listener, or object URL owned by these slices.

Later slices reuse these contracts and add focused tests for route-resource retry and resource
failure behavior. Do not weaken the server/browser or private-source boundaries merely to simplify
a component fixture.
