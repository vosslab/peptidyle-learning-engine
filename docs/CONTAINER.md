# Container stack

Local development stack for the Peptidyle Learning Engine: the API server,
worker, PostgreSQL, and MinIO. Defined in
[containers/compose.yaml](../containers/compose.yaml) and
[containers/Containerfile.api](../containers/Containerfile.api).

The root `.containerignore` is an allowlist for this
image build. Only the Cargo manifests, Rust crates, embedded SQLx migrations,
and owning Containerfiles enter the context; host `target/`, generated artifacts,
local credentials, and unrelated source trees never reach the builder. The
gateway image derives from the configured official Caddy digest and removes
its low-port file capability before running on port 8080 as UID 1000 with an
empty runtime capability set.

macOS setup for the Podman virtual machine lives in
[MACOS_PODMAN.md](MACOS_PODMAN.md).

## Services

| Service         | Image                                     | Purpose                                 | Local port                   |
| --------------- | ----------------------------------------- | --------------------------------------- | ---------------------------- |
| `api`           | built from `containers/Containerfile.api` | axum API server                         | 127.0.0.1:3000               |
| `worker`        | built from `containers/Containerfile.api` | family-filtered durable job draining    | none                         |
| `postgres`      | `postgres:latest`                         | shared content and tenant-owned records | 127.0.0.1:5432               |
| `minio`         | `quay.io/minio/minio`                     | S3-compatible object storage            | 127.0.0.1:9000, console 9001 |
| `createbuckets` | `quay.io/minio/mc`                        | one-shot bucket creation, then exits    | none                         |

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

Credentials arrive at run time from the environment. Nothing in the compose
file has a default password, so the stack refuses to start until you provide
one.

```bash
cp containers/env.example containers/env.local
# edit containers/env.local and set real values
podman compose -f containers/compose.yaml --env-file containers/env.local up -d
```

`containers/env.local` is gitignored.

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
podman compose -f containers/compose.yaml stop postgres
curl -s http://localhost:3000/health          # {"status":"degraded","failing":["postgres"]}
podman compose -f containers/compose.yaml start postgres
curl -s http://localhost:3000/health          # {"status":"ready"} once compatibility verifies
```

## Common commands

```bash
podman compose -f containers/compose.yaml ps                 # what is running
podman compose -f containers/compose.yaml logs -f api        # follow api logs
podman compose -f containers/compose.yaml logs -f worker     # follow bounded worker passes
podman compose -f containers/compose.yaml build api worker   # rebuild after Rust changes
podman compose -f containers/compose.yaml down               # stop, keep volumes
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

Named volumes `ple_pgdata` and `ple_miniodata` survive `down`. PostgreSQL 18
and later store versioned data below the mounted `/var/lib/postgresql` path.
Removing either volume destroys local data, so it is a deliberate step rather
than part of the normal stop command.

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
