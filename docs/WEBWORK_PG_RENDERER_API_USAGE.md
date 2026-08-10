# Shipped WeBWorK PG/PGML renderer contract

This is the production-facing contract for PLE's RC3 WeBWorK integration. It
replaces the old, unshipped `/v1/render`, `/v1/grade`, JSON-body, and public
renderer guidance. PLE is the only WebWork client; a student browser never
contacts WebWork directly.

## Scope and status

RC3 ships one bounded, server-rendered question path: a licensed,
user-authored immutable PGML question containing exactly one standard
`RadioButtons` group. PLE renders and grades that question through upstream
WeBWorK, projects it into PLE's multiple-choice envelope, and keeps all
upstream state server-side.

The Rust client, Compose profile, launcher, static topology checks, and
recorded-contract tests are implemented. The source-pinned container build,
authenticated semantic RPC probe, PLE render/grade/cache run, browser boundary
trace, and final independent review passed on 2026-08-10. RC3 is accepted for
this bounded path; it does not claim production deployment or generic PG
control compatibility.

## Private deployment boundary

`./launch_local_stack.sh --with-webwork` uses the `webwork` profile to start a
private WebWork service and a dedicated private MariaDB service. Neither has a
host port. The PLE API may reach WebWork only on `renderer_private`; WebWork
may reach MariaDB only on `webwork_db_private`; the API does not join the
database network. Gateway, browser, worker, PLE PostgreSQL, and object storage
join neither network.

The renderer application base is exactly:

```text
http://webwork-renderer:8080/webwork2/
```

The trailing slash matters. The adapter joins the literal relative path
`render_rpc`, yielding this sole upstream endpoint:

```text
POST http://webwork-renderer:8080/webwork2/render_rpc
```

The client refuses a base URL without an application path and trailing slash,
credentials embedded in a URL, query strings, fragments, redirects, non-2xx
responses, non-JSON responses, response bodies larger than its configured
limit (1 MiB by default), malformed JSON, or duplicate JSON members.

## Server-only authentication and source ownership

The adapter reads these server-owned settings:

| Setting                               | Value or meaning                                                                      |
| ------------------------------------- | ------------------------------------------------------------------------------------- |
| `PLE_WEBWORK_RENDER_COURSE_ID`        | `ple-render` by default                                                               |
| `PLE_WEBWORK_RENDER_USER`             | `ple-renderer` by default                                                             |
| `PLE_WEBWORK_RENDER_PASSWORD_FILE`    | API-readable password-file path; never a browser value or plaintext environment value |
| `PLE_WEBWORK_RENDERER_BASE_URL`       | The application base above                                                            |
| `PLE_WEBWORK_REQUEST_TIMEOUT_SECONDS` | Bounded request deadline (15 seconds by default)                                      |
| `PLE_WEBWORK_MAX_RESPONSE_BYTES`      | Maximum accepted RPC response (1,048,576 by default)                                  |

The launcher creates ignored, mode-`0600` render-password and distinct
Mojolicious-secret files, validates their mode and size, and mounts them only
through their required private owners. It does not print either secret. The
password is direct render-course authentication; PLE never enables
`allow_unsecured_rpc`, disables cookies, or creates a public renderer route.

At publication, PLE copies the exact bytes of
`content/pilot/webwork/which_hydrophobic-simple.pgml` and its
`which_hydrophobic-simple.provenance.json` sidecar into immutable object
storage. The provenance identifies its user-authored license and source
history. An attempt resolves that published source/version and seed from PLE;
the browser cannot select a source URL, filesystem path, seed, renderer
identity, or upstream option.

## Exact upstream form protocol

The upstream protocol is an `application/x-www-form-urlencoded` POST, not a
JSON API. Each render request sends only the following trusted values:

| Form field      | Value                                                             |
| --------------- | ----------------------------------------------------------------- |
| `courseID`      | Configured private render course                                  |
| `user`          | Configured private render user                                    |
| `passwd`        | Password read by PLE from its password file                       |
| `problemSource` | Base64 encoding of immutable PG/PGML source bytes                 |
| `fileName`      | Immutable diagnostic/source path, not an accessible renderer path |
| `problemSeed`   | Attempt's fixed seed                                              |
| `outputformat`  | `json`                                                            |

Render and grade use the same `/render_rpc` route. To grade, PLE first
re-renders the identical immutable source and seed, validates the same strict
projection, translates the submitted opaque PLE choice ID to the newly parsed
upstream radio name/value, and sends one further form request containing that
single radio field and `WWsubmit=1`. The upstream radio name/value, submit
marker, credentials, source bytes, session key, hidden fields, and response
body are never persisted in an attempt, cache, issued envelope, or browser
response.

The adapter refuses empty/oversized sources, invalid version UUIDs, paths that
are absolute, backslash-containing, NUL-containing, or traversal-like, and
unbounded source/path/HTML/control values before a result can be projected.

## Exact default JSON response boundary

The response must be one object with each of these 21 keys exactly once and no
others:

```text
head_part001 head_part010 head_part300 head_part400 head_part999
body_part001 body_part100 body_part300 body_part500 body_part530 body_part550
body_part590 body_part650 body_part700 body_part999
hidden_input_field score real_webwork_SITE_URL real_webwork_FORM_ACTION_URL
internal_problem_lang_and_dir
```

Every part except `hidden_input_field` and `score` must be text. `score` must
be a finite JSON number. PLE verifies and discards
`real_webwork_SITE_URL` as exactly `http://webwork-renderer:8080/` and
`real_webwork_FORM_ACTION_URL` as exactly
`http://webwork-renderer:8080/webwork2/render_rpc`; neither is browser data.
It uses only `body_part550` for the question body and `score` for grading.

`hidden_input_field` must be a JSON object, never HTML. Its exact map is:

```text
sourceFilePath problemSource problemSeed problemUUID psvn pathToProblemFile
courseID user passwd displayMode key outputformat theme language showSummary
showHints showSolutions showPreviewButton showCheckAnswersButton
showCorrectAnswersButton showFooter extraHeaderText
```

PLE verifies request echoes for `problemSource`, `problemSeed`,
`pathToProblemFile`/`fileName`, `courseID`, `user`, `passwd`, `displayMode`,
`outputformat`, and all show flags. It type- and size-checks then discards
server-derived `key`, `sourceFilePath`, `problemUUID`, `psvn`, `theme`,
`language`, and `extraHeaderText`. A nonempty `key` is permitted because
direct-password upstream authentication may produce a server session key; it
is never forwarded or stored. An unknown, missing, duplicate, oversized, or
mismatched protected value is a refusal.

The adapter also refuses top-level errors, protected answer/source/password
material outside the exact hidden map, protected material in the body, unsafe
markup, scripts, styles, event attributes, forms, frames, unsupported inputs,
duplicate attributes, malformed HTML, and off-origin service identity.

## RadioButtons projection and scoring

RC3 accepts exactly one official PG `RadioButtons` group: direct wrapping
`label` elements inside one `div.radio-buttons-container`, with one direct
radio `input` per label. It removes the entire group from the prompt before
projection, so labels never appear twice. It rejects every other form control,
extra group, nested group markup, duplicate field/id/value, malformed control,
or more than 32 choices.

For choice ordinal _n_, PLE calculates an opaque ID from SHA-256 over these
bytes, in order:

1. ASCII `ple:webwork:choice:v1\0`.
2. Raw 16 bytes of the immutable `VersionId` UUID.
3. Attempt seed as an unsigned big-endian 64-bit integer.
4. Radio-group ordinal `0` as an unsigned big-endian 32-bit integer.
5. Choice ordinal as an unsigned big-endian 32-bit integer.

The browser receives `ww-` plus the first 16 digest bytes in lowercase hex.
It never receives the source text, label-derived answer key, upstream field
name, or upstream value.

For RC3, the accepted upstream numeric percent is exactly `0` or `100`.
`0` earns zero points; `100` earns the published positive point value. Strings,
nonfinite numbers, values outside 0-100, and partial values all refuse. PLE
does not infer partial credit from WebWork in this release.

The render cache contains only the sanitized, answer-free PLE envelope and
HTML with trusted source/version/seed/renderer identity. It never records
source bytes, credentials, raw RPC JSON, cookie/session keys, correct answer
data, upstream radio fields/values, or a browser submission. Cache activity is
reported only as structured, non-sensitive `ple.webwork.cache` events such as
`renderer_call` and `cache_hit`.

## Pinned upstream build and verification

The local image is built from exact, unmodified upstream revisions:

| Repository                                    | Immutable revision                         |
| --------------------------------------------- | ------------------------------------------ |
| `https://github.com/openwebwork/webwork2.git` | `c7060fe858cb27b17aad5cf77574ff7d1ae3e1fa` |
| `https://github.com/openwebwork/pg.git`       | `726ff42840f968a1d6dfcc270c23c297e1d963f4` |

The build fetches each full revision, detached-checks it out, verifies
`git rev-parse HEAD`, then records the built local OCI image ID/digest and both
source revisions in ignored local provenance. A source pin is provenance, not
a claim that OCI output is byte-identical across platforms or times.

## Required commands and current evidence state

The following checks are the RC3 acceptance evidence. They were executed
without substituting a public, unauthenticated, or invented protocol.

```bash
cargo test -p adapter_webwork
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
./launch_local_stack.sh --with-webwork --check
source source_me.sh && python3 -m pytest -q tests/test_webwork_renderer_container.py
tests/e2e/e2e_webwork_api_secret_mode.sh
tests/e2e/e2e_webwork_render_rpc.sh
npx playwright test tests/playwright/webwork_run.spec.ts
./check_codebase.sh
source source_me.sh && python3 -m pytest -q tests/
```

Static/recorded and live checks passed. Static checks alone never establish a
container or browser boundary, which is why acceptance also required the live
render/grade profile and Playwright trace.

`tests/e2e/e2e_webwork_api_secret_mode.sh` is a separate strict API
secret-mode E2E. It uses a pinned minimal container to prove that the
API-owned runtime copy is mode `0600`, readable by UID 10001, refreshed after
rotation, and unreadable by an unrelated UID. It does not build or launch
WebWork, and it does not replace the full renderer acceptance.

`tests/e2e/e2e_webwork_render_rpc.sh` is the expensive live full-profile
acceptance. It requires sufficient Podman capacity to build and run the
source-pinned WebWork profile and PLE stack. It must prove the exact source
revisions and local image digest, authenticated semantic render, identical
repeat render, correct and incorrect grading, cache hit, and absence of
private material in the issued payload/cache. Run its accompanying Playwright
browser-boundary acceptance, `npx playwright test
tests/playwright/webwork_run.spec.ts`, against that live profile to prove the
browser contacts PLE only and receives no private renderer material.

A failure of build, authentication, secret handling, semantic rendering,
projection, topology, or browser privacy is a release blocker, not permission
to weaken this contract.

## Explicitly out of scope

| Excluded capability                                                        | RC3 decision                                                                            | Why this version succeeds without it                                                                               |
| -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Generic PG controls and WebWork matching                                   | Refuse them; WP-RC5 owns a separately tested MATCH adapter and browser path.            | One radio MC question proves the private upstream protocol and PLE lifecycle without exposing unprojectable state. |
| Hints, solutions, answer reveal, preview, attempts table, comments, footer | All corresponding upstream display flags are fixed off.                                 | RC3 supplies one safe graded question path; feedback/release policy remains PLE-owned.                             |
| Browser-to-WebWork access, CORS, or public renderer endpoints              | Never shipped.                                                                          | PLE retains authentication, tenancy, grading, FERPA isolation, and retention ownership.                            |
| Broad OPL compatibility                                                    | Accept only the immutable, licensed user-authored PGML fixture plus near-miss refusals. | A mutable OPL checkout is neither needed nor acceptable to prove the bounded v1 path.                              |
| Upstream gradebook/LTI passback                                            | Never configured or called.                                                             | PLE stores and releases its own scores; LTI belongs to WP-RC9.                                                     |
