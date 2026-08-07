# Container stack

Local development stack for the Peptidyle Learning Engine: the API server,
PostgreSQL, and MinIO. Defined in
[containers/compose.yaml](../containers/compose.yaml) and
[containers/Containerfile.api](../containers/Containerfile.api).

macOS setup for the Podman virtual machine lives in
[MACOS_PODMAN.md](MACOS_PODMAN.md).

## Services

| Service | Image | Purpose | Local port |
| --- | --- | --- | --- |
| `api` | built from `containers/Containerfile.api` | axum API server | 127.0.0.1:3000 |
| `postgres` | `postgres:17-alpine` | shared content and tenant-owned records | 127.0.0.1:5432 |
| `minio` | `quay.io/minio/minio` | S3-compatible object storage | 127.0.0.1:9000, console 9001 |
| `createbuckets` | `quay.io/minio/mc` | one-shot bucket creation, then exits | none |

Every port binds to `127.0.0.1`, not `0.0.0.0`. The database holds educational
records, so a development container must not be reachable from the local
network.

## Buckets

`createbuckets` creates three buckets, and they are separate because their
rules differ, not for tidiness.

| Bucket | Holds | Serving | Retention |
| --- | --- | --- | --- |
| `content` | source packages, shared assets, cached renders | CDN and immutable URLs for public content, 60-minute authorized URLs for secure content | indefinite, versioned |
| `student-records` | exports, uploaded responses, annotated exams | 5-minute authorized URLs, always logged | explicit expiration and deletion |
| `temp-processing` | extraction and conversion workspaces | never served | lifecycle rule, days |

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

## Verifying health

`/health` returns 200 only after a real `SELECT 1` against PostgreSQL and a
real `HeadBucket` request against the object store. It is not a liveness ping:
a process that is running but cannot reach its database reports 503.

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:3000/health
curl -s http://localhost:3000/health
```

A ready stack prints:

```json
{"status":"ready"}
```

A stack missing a dependency names it, which is the point of the endpoint:

```json
{"status":"degraded","failing":["object-store"]}
```

Prove both directions before trusting the gate. A health check that only ever
returns 200 is indistinguishable from one that is not checking anything:

```bash
podman compose -f containers/compose.yaml stop postgres
curl -s http://localhost:3000/health          # {"status":"degraded","failing":["postgres"]}
podman compose -f containers/compose.yaml start postgres
curl -s http://localhost:3000/health          # {"status":"ready"} once it accepts queries
```

## Common commands

```bash
podman compose -f containers/compose.yaml ps                 # what is running
podman compose -f containers/compose.yaml logs -f api        # follow api logs
podman compose -f containers/compose.yaml build api          # rebuild after Rust changes
podman compose -f containers/compose.yaml down               # stop, keep volumes
```

Named volumes `ple_pgdata` and `ple_miniodata` survive `down`. Removing them
destroys local data, so it is a deliberate step rather than part of the normal
stop command.

## Image shape

`Containerfile.api` is a two-stage build. The first stage compiles the Cargo
workspace with `--locked`, so the image cannot quietly resolve a different
dependency set than `Cargo.lock` records. The second stage carries only the
binary, `ca-certificates`, and `libssl3`, and runs as a non-root user.

Manifests are copied before sources so dependency compilation caches
separately from source edits.

The Rust version in the builder stage is pinned to the same compiler as
[rust-toolchain.toml](../rust-toolchain.toml). Raise both together.

## Health check inside the image

The container `HEALTHCHECK` runs the API binary with `--health-probe`, which
opens an HTTP request to its own `/health` and exits non-zero on anything but
200. Doing it this way keeps the runtime image free of `curl` and `wget`.
