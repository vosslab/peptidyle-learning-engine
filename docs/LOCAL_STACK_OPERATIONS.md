# Local stack operations

Local development stack for the Peptidyle Learning Engine: the browser gateway,
API server, worker, PostgreSQL, MinIO, and private standalone WeBWorK PG
renderer. These are the normal stack; SMTP is the only optional overlay.
The Compose model is defined in
[containers/compose.yaml](../containers/compose.yaml) and
[containers/Containerfile.api](../containers/Containerfile.api).
Replica scaling, shared-state ownership, failure behavior, and the planned
production topology are in [MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md).
The necessity, persistence, and network boundary of every service are in
[LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md).
`docs/CONTAINER_PORT_MAPPING.md` records which service ports are host-published
and which remain private.

The root `.containerignore` is an allowlist for this
image build. Only the Cargo manifests, Rust crates, embedded SQLx migrations,
and owning Containerfiles enter the context; host `target/`, generated artifacts,
local credentials, and unrelated source trees never reach the builder. The
gateway image derives from the configured official Caddy digest and removes
its low-port file capability before running on port 8080 as UID 1000 with an
empty runtime capability set.

The gateway also mounts the ignored `dist/` browser artifact read-only. It
serves browser navigation while proxying `/api`, `/api/*`, and `/health` to the
API, so the browser and its HttpOnly session use one origin.

macOS setup for the Podman virtual machine lives in
[MACOS_PODMAN.md](MACOS_PODMAN.md).

## Services

| Service            | Image                                         | Purpose                                    | Local port                   |
| ------------------ | --------------------------------------------- | ------------------------------------------ | ---------------------------- |
| `gateway`          | pinned official Caddy derivative              | browser files plus same-origin API gateway | 127.0.0.1:8080               |
| `api`              | built from `containers/Containerfile.api`     | axum API server                            | none                         |
| `worker`           | built from `containers/Containerfile.api`     | family-filtered durable job draining       | none                         |
| `postgres`         | digest-pinned official PostgreSQL 17          | shared content and tenant-owned records    | 127.0.0.1:5432               |
| `minio`            | digest-pinned official MinIO                  | S3-compatible object storage               | 127.0.0.1:9000, console 9001 |
| `createbuckets`    | digest-pinned official MinIO Client           | one-shot bucket creation, then exits       | none                         |
| `identity-secret-init` | pinned official Alpine | one-shot invitation-secret permission setup | none |
| `webwork-renderer` | external `webwork-pg-renderer` image | private stateless PG/PGML render and grade engine | none |

Every published port binds to `127.0.0.1`, not `0.0.0.0`. The database holds
educational records, so a development container must not be reachable from the
local network. See `docs/CONTAINER_PORT_MAPPING.md` for the complete mapping,
port ranges, and the distinction between container-local and host-published
ports.

## Buckets

`createbuckets` creates three buckets, and they are separate because their
rules differ, not for tidiness.

| Bucket            | Holds                                          | Serving                                                                                 | Retention                        |
| ----------------- | ---------------------------------------------- | --------------------------------------------------------------------------------------- | -------------------------------- |
| `content`         | source packages, shared assets, cached renders | CDN and immutable URLs for public content, 60-minute authorized URLs for secure content | indefinite, versioned            |
| `student-records` | exports, uploaded responses, annotated exams   | 5-minute authorized URLs, always logged                                                 | explicit expiration and deletion |
| `temp-processing` | extraction and conversion workspaces           | never served                                                                            | lifecycle rule, days             |

A course deletion removes `student-records` artifacts and leaves `content`
untouched. Separate buckets make that a policy rather than a filter over a
shared prefix.

## First run

The normal root command owns the local-only bootstrap:

```bash
./launch_local_stack.sh
```

On its first default run, the launcher creates an ignored mode-0600
`containers/env.local`, generates independent database/object-store/grader and
invitation-issuer secrets, generates instructor and student bearer credentials,
and mounts only their hashes into the API. It builds the host artifacts, starts
PostgreSQL and MinIO, applies and verifies the embedded migrations, provisions the restricted
`ple_grading_reader` login, seeds one small course/assignment/native-question
scenario, verifies the external PG renderer, starts the API/worker/gateway,
waits for semantic `/health`, and opens the browser. Named data volumes remain
available for repeated testing. The default gateway port is `8080`. If its
selected port is occupied during first-run bootstrap, the launcher records the
first available port from 8000 through 8099 in the ignored env file. An existing
explicit `PLE_GATEWAY_HOST_PORT` remains an operator choice until it is changed.

The recovery screen accepts either generated value from the ignored
`containers/local-login.txt`. The browser sends it once to the same-origin
local login endpoint; the established credential is the existing HttpOnly
session cookie, not local or session storage. Do not copy these local files to
a deployed environment.

Use `./launch_local_stack.sh --check` for a read-only configuration preflight,
`--no-open` on a headless machine, or `--skip-build` when the existing `dist/`
bundle is intentionally current. A custom `--env-file` is never rewritten or
seeded and must provide every required secret itself. `npm run launch` is an
optional alias.

The API reads the mode-0600 invitation issuer from a read-only named volume
populated by a networkless one-shot initializer running under the pinned Alpine
image. This makes manager copy-link invitations available without SMTP and
keeps the raw issuer out of environment variables. PLE uses its established
Rust SMTP adapter only when an operator supplies provider settings; the local
stack does not run or maintain a mail server.

## External SMTP provider

SMTP is an opt-in connection to an operator-selected service, not another PLE
container. Keep the normal stack unchanged until a provider account exists.
When it does, copy `containers/env.example` to an operator-owned environment
file and set:

- `PLE_SMTP_RELAY` to the provider hostname, without `smtp://` or `smtps://`;
- `PLE_SMTP_PORT` and `PLE_SMTP_TLS_MODE` to either mandatory `starttls`
  submission or `implicit-tls` submission, as specified by the provider;
- `PLE_SMTP_USERNAME` and `PLE_SMTP_FROM` to provider-authorized values;
- `PLE_SMTP_PASSWORD_HOST_FILE` to an absolute, non-symlink, mode-0600 file
  containing only the provider-issued SMTP password or token; and
- `PLE_PUBLIC_APP_BASE_URL` to the deployed public HTTPS PLE origin.

Preflight and start that configuration explicitly:

```bash
./launch_local_stack.sh --env-file path/to/env.local --with-smtp --check
./launch_local_stack.sh --env-file path/to/env.local --with-smtp --no-open
podman compose -f containers/compose.yaml -f containers/compose.smtp.yaml \
  --env-file path/to/env.local down
```

The SMTP overlay copies the credential through a networkless, capability-minimal
one-shot container into an API-readable, read-only named volume. The API never
receives the host path or credential text in its environment. Omitting
`--with-smtp` passes no SMTP configuration to the API; copy-link invitations
continue to work, while email sign-in remains unavailable until the external
provider is configured. PLE does not manage sender reputation, DNS mail policy,
bounces, or provider accounts.

The root launcher is the maintained startup path because API/worker startup is
deliberately later than migration and grader-role provisioning. Running a bare
`compose up` against an empty database is therefore not equivalent.

## Private standalone PG renderer

WeBWorK-backed questions are part of the normal stack. PLE relies on the
external `webwork-pg-renderer` image named by `PLE_WEBWORK_RENDERER_IMAGE`; this
repository neither rebuilds that service nor runs the full WebWork2 homework
application. The renderer wraps upstream WeBWorK PG/PGML execution behind one
private `/render-api` form endpoint.

The renderer has no volume, SQL database, course, roster, assignment, user, or
host-published port. It joins only `renderer_private` with the API. The gateway,
browser, worker, PostgreSQL, and MinIO cannot reach that network. PLE remains the
sole assignment distributor and educational-record authority.

The default local bootstrap creates renderer JWT secrets in the ignored
mode-0600 `containers/env.local`. They authenticate API-to-renderer requests and
responses; they never enter browser data. A custom environment supplies its own
values. The launcher records the selected OCI image ID in an ignored provenance
file and runs `containers/webwork/probe_render_api.sh` inside the container to
exercise both rendering and grading before the API starts.

The renderer is stateless. Recreating it loses no PLE record. PostgreSQL and
MinIO retain records in named volumes outside their writable container layers;
normal `down` and rebuild operations preserve those volumes.

The startup probe is not a substitute for PLE integration or browser testing.
The explicit live E2E accepted on 2026-08-10 proves the bounded licensed PGML
`RadioButtons` path through PLE, including correct/incorrect grading, cache
behavior, renderer outage recovery, keyboard use, and protected-material
non-disclosure. Matching and broader PG compatibility require their own source
and live evidence.

The worker handles one job per bounded pass and concurrency comes from scaling
the service. It claims only current scoring, course item analysis, attempt
auto-submit, retention, assignment export, and QTI import. Reserved Render and
generic Import rows remain ready until both preparation and atomic commit
implementations exist. `PLE_WORKER_LEASE_SECONDS`,
`PLE_WORKER_PREPARATION_TIMEOUT_SECONDS`, and `PLE_WORKER_POLL_MILLIS` are
bounded in-process controls with documented defaults in `env.example`.

## Verifying health

`/health` returns 200 only after the bounded PostgreSQL compatibility verifier
confirms the exact migration versions, states, and checksums expected by the
running binary, plus a real `HeadBucket` request against the object store. It
is not a liveness ping: a process that is running but cannot reach its database
or reaches an incompatible schema reports 503.

The gateway actively checks this dedicated route every five seconds. It does
not treat an arbitrary application 503 as replica unhealthiness. That
distinction keeps stored records diagnosable when a feature-local dependency
fails closed. The normal launcher nevertheless requires the renderer to pass
its semantic startup probe before it starts the API.

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8080/health
curl -s http://localhost:8080/health
```

A ready stack prints:

```json
{ "status": "ready" }
```

A stack missing a dependency names it, which is the point of the endpoint:

```json
{ "status": "degraded", "failing": ["object-store"] }
```

The worker exposes no HTTP endpoint, so Compose explicitly disables the API
image health check for that service. Its liveness is the supervised process;
its useful operational signal is the supported-family queue depth emitted with
non-empty pass counters. Worker startup verifies schema compatibility and
refuses to drain when verification is unavailable or incompatible.

Prove both directions before trusting the gate. A health check that only ever
returns 200 is indistinguishable from one that is not checking anything:

```bash
podman compose -f containers/compose.yaml --env-file containers/env.local stop postgres
curl -s http://localhost:8080/health          # {"status":"degraded","failing":["postgres"]}
podman compose -f containers/compose.yaml --env-file containers/env.local start postgres
curl -s http://localhost:8080/health          # {"status":"ready"} once compatibility verifies
```

## Common commands

```bash
./launch_local_stack.sh                                      # build, start, wait, and open
./launch_local_stack.sh --skip-build --no-open               # fast restart without browser open

# Direct Compose commands always load the ignored local env file.
podman compose -f containers/compose.yaml --env-file containers/env.local ps
podman compose -f containers/compose.yaml --env-file containers/env.local \
  logs -f api worker webwork-renderer
podman compose -f containers/compose.yaml --env-file containers/env.local build api worker
podman compose -f containers/compose.yaml --env-file containers/env.local \
  up -d --scale api=2 --scale worker=2 api worker gateway
podman compose -f containers/compose.yaml --env-file containers/env.local \
  down --remove-orphans
```

## Whole-system verification

The maintained non-browser E2E runner builds on the ordinary repository
artifacts and uses disposable, loopback-only Compose projects:

```bash
PLE_E2E_GATEWAY_IMAGE_SHA256=<64-hex-official-Caddy-digest> \
  bash tests/e2e/e2e_run_all.sh
```

It proves the Wasm bridge, the complete PostgreSQL migration/RLS/live-oracle
suite, and a real learner submission across two API replicas after stopping the
replica that issued the question. The gateway image is derived from that pinned
official digest and strips Caddy's unnecessary low-port file capability before
running as UID 1000 with `cap_drop: ALL`. Generated projects and volumes are
removed on both success and failure; the runner never targets a long-lived
development project.

Named volumes `ple_pgdata` and `ple_miniodata` survive `down`. The launcher
runs the read-only `postgres-major-guard` before it starts PostgreSQL and
accepts only a missing data directory or a populated PostgreSQL 17 directory.
PostgreSQL data directories are not compatible across major versions. Upgrade
through a documented, non-destructive migration: back up and verify the old
cluster, create a new PostgreSQL-major volume, restore into it, validate the
migration ledger and application behavior, then retain the old volume until
recovery is accepted. Removing either volume destroys local data, so it is a
deliberate step rather than part of the normal stop command.

## Image shape

`Containerfile.api` is a two-stage build. The first stage compiles the Cargo
workspace with `--locked`, so the image cannot quietly resolve a different
dependency set than `Cargo.lock` records. The second stage carries only the
binary and `ca-certificates`, and runs as a non-root user.

Manifests are copied before sources so dependency compilation caches
separately from source edits.

The builder follows the current stable Rust channel declared by
[rust-toolchain.toml](../rust-toolchain.toml).

## Health check inside the image

The container `HEALTHCHECK` runs the API binary with `--health-probe`, which
opens an HTTP request to its own `/health` and exits non-zero on anything but 200. Doing it this way keeps the runtime image free of `curl` and `wget`.
