# Container stack

Local development stack for the Peptidyle Learning Engine: the browser gateway,
API server, worker, PostgreSQL, and MinIO. The native stack is the supported
default; the private WeBWorK renderer path is the accepted bounded WP-RC3 profile.
The Compose model is defined in
[containers/compose.yaml](../containers/compose.yaml) and
[containers/Containerfile.api](../containers/Containerfile.api).
Replica scaling, shared-state ownership, failure behavior, and the planned
production topology are in [MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md).

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
| `gateway`          | pinned official Caddy derivative              | browser files plus same-origin API gateway | 127.0.0.1:3000               |
| `api`              | built from `containers/Containerfile.api`     | axum API server                            | none                         |
| `worker`           | built from `containers/Containerfile.api`     | family-filtered durable job draining       | none                         |
| `postgres`         | digest-pinned official PostgreSQL 17          | shared content and tenant-owned records    | 127.0.0.1:5432               |
| `minio`            | digest-pinned official MinIO                  | S3-compatible object storage               | 127.0.0.1:9000, console 9001 |
| `createbuckets`    | digest-pinned official MinIO Client           | one-shot bucket creation, then exits       | none                         |
| `webwork-renderer` | local image built from pinned upstream source | optional private WebWork2/PG profile       | none                         |

Every port binds to `127.0.0.1`, not `0.0.0.0`. The database holds educational
records, so a development container must not be reachable from the local
network.

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
`containers/env.local`, generates independent database/object-store/grader
secrets, generates instructor and student bearer credentials, and mounts only
their hashes into the API. It builds the host artifacts, starts PostgreSQL and
MinIO, applies and verifies the embedded migrations, provisions the restricted
`ple_grading_reader` login, seeds one small course/assignment/native-question
scenario, starts the API/worker/gateway, waits for semantic `/health`, and opens
the browser. Named data volumes remain available for repeated testing. If port
3000 is already occupied during first-run bootstrap, the launcher records the
first available port from 3000 through 3099 in the ignored env file.

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

The root launcher is the maintained startup path because API/worker startup is
deliberately later than migration and grader-role provisioning. Running a bare
`compose up` against an empty database is therefore not equivalent.

## Private WebWork profile

The optional `--with-webwork` profile builds WebWork2 and PG from the exact
official source revisions declared in [containers/env.example](../containers/env.example):
`c7060fe858cb27b17aad5cf77574ff7d1ae3e1fa` and
`726ff42840f968a1d6dfcc270c23c297e1d963f4`. The build's Alpine/git, Node,
Ubuntu, and MariaDB source images are immutable OCI digests with arm64
manifests. The source build verifies each full Git revision before it copies
the source into the final local image. It does not consume a legacy or
operator-selected WebWork image.

The base Compose file contains the private WebWork services behind the
`webwork` profile. The normal native-only stack has no renderer configuration
or secret-copy dependency. `--with-webwork` enables that profile and applies
the narrowly scoped `containers/compose.webwork.yaml` overlay. The overlay
injects the API renderer settings and its read-only secret-runtime volume; API
then uses the internal
`http://webwork-renderer:8080/webwork2/` application base and the renderer
joins no public browser route. It is not a separate browser origin. The browser
continues to call PLE through the loopback gateway only.

The root launcher atomically creates and validates separate ignored mode-0600
files for the direct render-course password and Mojolicious signing secret. It
mounts both files read-only to WebWork. A networkless, capability-minimal
one-shot service copies only the render password to a named volume owned by
the API runtime UID; API mounts that copy read-only. Each `--with-webwork`
start recreates the one-shot service, which refreshes the copy after a password
rotation. Neither secret is an environment value, browser value, source mount,
or object-store value.

WebWork and its MariaDB use named `ple_webwork_courses` and
`ple_webwork_dbdata` volumes together. They join only two internal networks:
`renderer_private` has API and WebWork, and `webwork_db_private` has WebWork
and MariaDB. Neither service publishes a host port or joins PLE PostgreSQL,
MinIO, gateway, worker, or browser networks. WebWork owns no PLE content or
student-record volume.

Start the profile with:

```bash
./launch_local_stack.sh --with-webwork
```

The launcher writes an ignored provenance record with the final local OCI image
ID and both source revisions, then configures that image identity for the API.
This makes an arm64 local build auditable without claiming byte-identical OCI
output across architectures or package mirrors.

`probe_render_rpc.sh` is an authenticated direct service readiness check. It
does not substitute for PLE issue/cache/grade behavior or a browser test.
WP-RC3 live-profile, PLE-integration, and browser-boundary evidence was
accepted on 2026-08-10. Static checks do not supersede that recorded live
evidence when the profile changes. This does not change the native-only stack's
supported-default status.

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
distinction keeps authentication, native questions, and navigation available
when an optional feature-local dependency such as the private WebWork renderer
fails closed.

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:3000/health
curl -s http://localhost:3000/health
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
curl -s http://localhost:3000/health          # {"status":"degraded","failing":["postgres"]}
podman compose -f containers/compose.yaml --env-file containers/env.local start postgres
curl -s http://localhost:3000/health          # {"status":"ready"} once compatibility verifies
```

## Common commands

```bash
./launch_local_stack.sh                                      # build, start, wait, and open
./launch_local_stack.sh --skip-build --no-open               # fast restart without browser open
./launch_local_stack.sh --with-webwork                       # accepted bounded RC3 renderer profile

# Native default: direct Compose commands always load the ignored local env file.
podman compose -f containers/compose.yaml --env-file containers/env.local ps
podman compose -f containers/compose.yaml --env-file containers/env.local logs -f api worker
podman compose -f containers/compose.yaml --env-file containers/env.local build api worker
podman compose -f containers/compose.yaml --env-file containers/env.local \
  up -d --scale api=2 --scale worker=2 api worker gateway
podman compose -f containers/compose.yaml --env-file containers/env.local down

# Accepted bounded WP-RC3 WebWork path: retain both the overlay and the profile.
podman compose -f containers/compose.yaml -f containers/compose.webwork.yaml \
  --env-file containers/env.local --profile webwork \
  up -d --scale api=2 --scale worker=2
podman compose -f containers/compose.yaml -f containers/compose.webwork.yaml \
  --env-file containers/env.local --profile webwork logs -f api worker webwork-renderer
podman compose -f containers/compose.yaml -f containers/compose.webwork.yaml \
  --env-file containers/env.local --profile webwork down
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
