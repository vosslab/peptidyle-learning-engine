# WeBWorK PG renderer contract

This document defines PLE's server-only integration with the external
`webwork-pg-renderer` service. PLE is the only renderer client. A student
browser calls PLE and never contacts the renderer.

## Three different projects

The similar names describe different layers:

| Project                                                               | Responsibility                                                                      | PLE use                                                  |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------------- | -------------------------------------------------------- |
| [WeBWorK PG](https://github.com/openwebwork/pg)                       | PG/PGML language, seeded execution, rendering, answer evaluators, and grading.      | Engine used by the renderer.                             |
| [webwork-pg-renderer](https://github.com/vosslab/webwork-pg-renderer) | Small HTTP service around WeBWorK PG render and grade operations.                   | Required private runtime service.                        |
| [WeBWorK2](https://github.com/openwebwork/webwork2)                   | Complete homework application with users, courses, sets, attempts, and persistence. | Reference and prior art only; not a PLE runtime service. |

PLE owns courses, assignments, enrollment, attempts, feedback, scores, and
retention. The runtime is:

```text
browser -> PLE gateway -> PLE API -> private PG renderer -> WeBWorK PG
```

There is one assignment distribution system: PLE. No WebWork2 course, roster,
assignment, password, session store, or SQL database participates.

`OTHER_REPOS/pg`, `OTHER_REPOS/webwork-pg-renderer`, and
`OTHER_REPOS/webwork2` are read-only comparison snapshots. They are not build
contexts, imports, checked-out dependencies, or runtime sources. PLE consumes a separately built
renderer image through its published API.

## Accepted scope

The accepted live release path is four licensed, user-authored immutable PGML sources: one
`RadioButtons` and one matching question for each of Genetics and Biochemistry Chapter 1. PLE
projects them into answer-free multiple-choice or matching Question Variation Presentations, grades correct, incorrect,
and partial-credit responses as appropriate, and keeps all upstream controls and values private.
Matching partial credit is admitted only for the reviewed path-and-Source Object Checksum pairs.

This is not a claim of broad live PG compatibility. Other PG controls, source revisions, and Open
Problem Library items require their own reviewed source examples and the same PLE E2E and
browser-boundary evidence before they are advertised as supported.

## Deployment boundary

The renderer is a normal service in `containers/compose.yaml`. The default
typed lifecycle requires the declared `PLE_WEBWORK_RENDERER_IMAGE` to exist and starts
it with the rest of PLE.

The service:

- has no host-published port;
- has no persistent volume;
- has no SQL connection or database service;
- joins only `renderer_private` with the API;
- receives no PLE database, object-store, session, or student credential; and
- can be recreated without losing an educational record.

The API base is:

```text
http://webwork-renderer:3000/
```

The adapter joins the fixed relative path `render-api` and sends:

```text
POST http://webwork-renderer:3000/render-api
Content-Type: application/x-www-form-urlencoded
```

It refuses embedded URL credentials, query strings, fragments, redirects,
non-success status, non-JSON content, oversized bodies, malformed JSON,
duplicate members, and unexpected response members.

## Server-owned request

PLE resolves immutable source, version, seed, and renderer identity from the
authenticated attempt. The browser cannot select any of them.

The fixed render form contains:

| Field                                                                              | Server-owned value                                                                            |
| ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `_format`                                                                          | `json`                                                                                        |
| `problemSource`                                                                    | Base64 of the immutable PG/PGML source bytes.                                                 |
| `sourceFilePath`                                                                   | Bounded diagnostic source path. It is not a renderer filesystem path selected by the browser. |
| `problemSeed`                                                                      | Attempt seed.                                                                                 |
| `outputFormat`                                                                     | `default` for the standalone service.                                                         |
| `displayMode`                                                                      | `MathJax`.                                                                                    |
| `isInstructor`                                                                     | `0`.                                                                                          |
| `showSummary`, `showHints`, `showSolutions`                                        | `0`.                                                                                          |
| `hidePreviewButton`, `hideCheckAnswersButton`, `hideAttemptsTable`, `hideMessages` | `1`.                                                                                          |
| `showCorrectAnswersButton`, `showFooter`                                           | `0`.                                                                                          |

For grading, PLE reconstructs the same source and seed and adds only
`submitAnswers=1` plus the server-held upstream field/value that corresponds to
the student's opaque PLE selection. The browser never submits an upstream field
name or value.

## Authentication and response identity

The renderer signs its problem, session, and answer state. Local development
stores the problem and session JWT secrets in ignored mode-0600
`containers/env.local`; deployed environments must provide independent secret
values. These are API-to-renderer credentials, not student credentials.

The expected response is a closed object with these protocol members:

```text
JWT debug flags problem_result problem_state renderedHTML resources
```

The adapter validates token shape and request binding, then discards private
renderer tokens from the Question Presentation. `problem_result.score` is a finite
normalized value between 0 and 1. The bounded all-or-nothing radio contract
accepts 0 or 1 and maps it to the published PLE point value.

Unknown members and protected Question Source, Answer Key, or credential values are a
refusal. This exact closed shape is a security and compatibility boundary, not
an arbitrary collection-size assertion.

## Question Presentation

The adapter selects the rendered `div#problem_body.problem-content`, verifies
the same-origin renderer base/form metadata, and parses only recognized controls.
It converts visible labels to PLE response options and stores the upstream
field/value mapping only in server-side replay state.

The Student Question Presentation may contain:

- typed prompt blocks;
- the PLE Question Type and browser rendering metadata;
- visible choice labels; and
- opaque PLE choice identifiers.

It must not contain:

- PG/PGML source;
- a correct answer or answer hash;
- upstream input names or values;
- renderer JWTs or problem state;
- renderer URLs or credentials; or
- raw renderer HTML.

Unsupported or executable markup, hostile styles and attributes, forms, frames,
unsupported controls, duplicate attributes, and malformed HTML fail closed before typed conversion.

## Cache and replay

The public render cache contains only answer-free typed PLE output bound to
source, version, seed, and renderer identity. Private upstream replay mapping is
stored separately from public cached bytes.

Current cache-hit issuance may make one private same-seed renderer call to
reconstruct and verify replay mapping. The planned one-call replay optimization
must preserve the same binding and secrecy contract; it cannot move private
mapping into the browser cache.

## Startup and failure behavior

The private typed lifecycle:

1. resolves the selected external image name or published digest to its OCI configuration ID;
2. recreates the stateless renderer and verifies its container uses that ID;
3. atomically records the selected image reference and OCI ID in ignored Local Stack State;
4. runs `containers/webwork/probe_render_api.sh` inside the container;
5. proves one deterministic public render plus correct and incorrect grades;
6. seeds the owner-controlled PLE pilot source; and
7. starts the API only after the renderer probe succeeds.

The renderer's own startup diagnostics may report optional PG macro limitations.
PLE acceptance is based on the supported owner-controlled Question Backend behavior, not
on an invented wall-clock threshold or byte-identical container output.

At request time, timeout, outage, malformed output, or identity drift causes a
bounded backend-local refusal. The renderer cannot mutate PLE records directly.

## Verification model

Fast permanent tests cover stable parser and renderer transformation, score, secrecy, and
topology behavior. They use inline recorded data and no real network.

Live acceptance is intentionally separate:

```bash
cargo test -p adapter_webwork --all-targets
cargo clippy -p adapter_webwork --all-targets -- -D warnings
source source_me.sh && python3 local_stack.py validate
./run_playwright_tests.sh --build
```

Exact Compose and lifecycle source inspection was useful during the renderer
cutover but is not retained as pytest. Read-only validation and live E2E
exercise the maintained boundary without freezing configuration text.

The original renderer E2E passed on 2026-08-10 for the licensed PGML `RadioButtons` pilot. The
Chapter 1 release gate subsequently passed all four reviewed PGML sources, including both matching
questions and matching partial credit, through the real renderer, PLE grading, and built-browser
student path on 2026-08-11.

That evidence supports this bounded path. It does not imply every Open Problem
Library item or PG macro is compatible. New Question Types require behavior-focused
adapter tests and a real source-to-browser acceptance path; temporary diagnostic
probes should be removed after they have served that implementation purpose.

See [LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md) for why each local service exists,
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) for the render and
response payload boundary, and
[QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md) for comparison
with PLE Question JSON, iMathAS, H5P, and the QTI Import pathway behind the
shared Question operations.
