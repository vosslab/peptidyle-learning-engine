# Solid model

This document is the review contract for Solid code in Peptidyle. Components
run once. Reactive reads belong in JSX, memos, resources, and effects; changing
state updates only the subscribers that read it.

## Reactivity map

| State | Primitive | Owner | Update contract |
| --- | --- | --- | --- |
| Session and role | Context over a store | App shell | Read across routes, written only by authentication events |
| Current run and question status | Store with granular paths | Run route | Updating one question does not replace unrelated question state |
| Remaining time | Signal of integer deciseconds | Timer | A tick updates only the displayed time and meaningful announcement intervals |
| Submission phase | Signal of a discriminated union | Response widget | `idle`, `validating`, `submitted`, `graded`, or `failed` |
| Question content | Resource keyed by attempt ID | Question renderer | Pending, success, and failure remain explicit and retryable |
| Prefetched next question | Store keyed by question index | Prefetch controller | Navigation reads warmed content without replacing the current question |
| Catalog page | Resource plus cursor signal | Library route | Loading another page uses a cursor and retains the current page while pending |

Use a signal for one independently changing scalar. Use a store for a nested
tree that receives granular updates. Use a resource for asynchronous data.
Derive values with a plain function or `createMemo`; do not write derived state
from an effect.

## Component and prop rules

- A component body performs setup once. It is not a render loop.
- Read `props.value` at the use site. Do not destructure reactive props.
- Use `mergeProps` for defaults and `splitProps` for safe forwarding.
- Render identity-bearing rows and choices with `<For>` so focus and local DOM
  state follow the item when order changes.
- Use `<Index>` only when a stable position, rather than item identity, owns the
  DOM slot.
- Use `<Show>` or `<Switch>` for reactive branches. Do not hide state changes in
  a one-time function-body conditional.

## Lifecycle contract

One-time DOM setup belongs in `onMount`. Every timer, subscription, listener,
or external resource registers teardown with `onCleanup`. An `onMount`
callback never returns a cleanup function.

The app owns one WebAssembly loader resource at startup. Route components read
the shared facade from context; they do not instantiate modules independently.
The same ownership rule applies to the API runtime and session store.

## Routing and async data

`@solidjs/router` owns navigation. Links use `<A>`; imperative navigation is
reserved for state transitions such as successful login or leaving a completed
run. Route data is wrapped with `query` and read with `createAsync`, keyed by
the relevant run, attempt, or cursor.

Each route has its own error boundary so one screen failure leaves the shell
and navigation usable. The question renderer has a second nested boundary so
bad content does not remove the run controls or timer. Resources show explicit
loading, error with retry, empty, and success states.

## Response-widget state

The reference widget owns only student input and browser-safe format status.
Its phase is a discriminated union:

```text
idle -> validating -> ready -> submitted
                   `-> invalid
           request `-> failed -> validating
```

Selecting a value updates the controlled input and calls the shared WASM
format validator. A monotonically increasing validation request number keeps a
slow older result from replacing a newer result. No client state guesses
correctness.

## Server and browser boundary

| Browser owns | Server owns |
| --- | --- |
| Navigation, display, controlled input, and local buffering | Authentication, authorization, and tenant context |
| Key-free response-format validation | Correctness, points, and feedback disclosure |
| Countdown display reconciled from server data | Deadline and late-submission verdict |
| Mock data during pre-route UI work | Durable attempts, summaries, and idempotency records |

Everything crossing the boundary is JSON-serializable. The generated client
surface derives from `crates/question_model`; answer-bearing `crates/grading`
types never enter it.

## Reactivity verification

The reference slice carries observable tests for these conditions:

- changing one selected choice updates the checked radio and validation live
  region without recreating the widget;
- a number key selects the corresponding choice;
- validation pending, valid, and invalid states are visible and named;
- a route resource exposes loading and successful mock-backed content; and
- the application can dispose without leaving a timer or listener behind.

Later widgets reuse this contract and add focused tests for `<For>` identity,
store-path updates, resource failure and retry, and cleanup counts.
