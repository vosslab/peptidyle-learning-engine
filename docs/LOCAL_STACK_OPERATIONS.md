# Local stack operations

Developer stack for the Peptidyle Learning Engine: the browser gateway, API
server, worker, PostgreSQL, MinIO, and private standalone WeBWorK PG renderer.
The normal Compose topology is
`containers/compose.yaml` with optional `containers/compose.smtp.yaml`.
The public developer entry uses the owner-locked production-auth composition
and TLS gateway described by
`tests/e2e/compose.live-demo-browser.yaml`; it does not select a local-file
authentication overlay or accept a caller-selected project. The image model is in
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
its low-port file capability before running on the owner-selected HTTPS port as UID 1000 with an
empty runtime capability set.

The gateway also mounts the ignored `dist/` browser artifact read-only. It
serves browser navigation while proxying `/api`, `/api/*`, and `/health` to the
API, so the browser and its HttpOnly session use one origin.

The fixed developer lifecycle builds the production `dist/` bundle and serves
it from the owner-locked HTTPS gateway. It uses seeded production authentication
and the ordinary visible account/passkey flow; no local credential form or
local-auth build switch participates.

macOS setup for the Podman virtual machine lives in
[MACOS_PODMAN.md](MACOS_PODMAN.md).

## Services

| Service                | Image                                       | Purpose                                           | Local port                   |
| ---------------------- | ------------------------------------------- | ------------------------------------------------- | ---------------------------- |
| `gateway`              | pinned official Caddy derivative            | browser files plus same-origin API gateway        | owner-selected loopback port |
| `api`                  | shared locally built Rust application image | axum API server                                   | none                         |
| `worker`               | API-owned shared Rust application image     | family-filtered durable job draining              | none                         |
| `postgres`             | digest-pinned official PostgreSQL 17        | shared content and tenant-owned records           | 127.0.0.1:5432               |
| `minio`                | digest-pinned official MinIO                | S3-compatible object storage                      | 127.0.0.1:9000, console 9001 |
| `createbuckets`        | digest-pinned official MinIO Client         | one-shot bucket creation, then exits              | none                         |
| `identity-secret-init` | pinned official Alpine                      | one-shot invitation-issuer and Question ID capability setup | none                         |
| `webwork-renderer`     | external `webwork-pg-renderer` image        | private stateless PG/PGML render and grade engine | none                         |

Every published port binds to `127.0.0.1`, not `0.0.0.0`. The database holds
educational records, so a development container must not be reachable from the
local network. See `docs/CONTAINER_PORT_MAPPING.md` for the complete mapping,
port ranges, and the distinction between container-local and host-published
ports.

## Stateful runtime containment

PostgreSQL, MinIO, and `createbuckets` run under fixed non-root UIDs with an
immutable container root, an empty capability set, `no-new-privileges`, bounded
CPU, memory, and PID budgets, and a bounded non-executable `/tmp`. PostgreSQL
can write only `ple_pgdata`; MinIO can write only `ple_miniodata`; and the
one-shot bucket creator has no durable mount. PostgreSQL receives an additional
ephemeral Unix-socket directory under `/var/run/postgresql`.

`local-data-volume-permissions` is the one networkless preflight that runs as
root *inside the rootless Podman user namespace*. It has only `CAP_CHOWN`. It
assigns the PostgreSQL volume root to UID 999 and the complete MinIO tree to UID
10001 before their daemons start. PostgreSQL already owns its protected
mode-0700 descendants; the MinIO traversal repairs retained local objects
created by an older root-running image. The helper does not create or alter
database/object content and does not retain a running process. This is
necessary because a fresh rootless named volume is engine-owned, while
PostgreSQL's official entrypoint requires its data directory to be writable by
the selected runtime user. MinIO explicitly supports an arbitrary regular user
when `/data` is writable; its transient home is the bounded `/tmp` tmpfs.

This is a disposable developer containment boundary, not an authorization boundary
or a production deployment claim. The service ports remain loopback-only, and
operator access to the host account or its Podman socket can still read the
named volumes. Production uses the separate AWS RDS, S3, IAM, and KMS design.
If an image upgrade changes either documented UID or requires an additional
writable path, the Compose policy test and a disposable Podman start must be
updated together; do not silently remove the fixed-user or read-only settings.

The API stops accepting new work and drains admitted requests for up to 30
seconds. Compose provides a 45-second stop grace period, so a normal container
stop cannot preempt that documented application drain. Workers have their own
longer bounded stop allowance because a claimed durable job must finish or
release safely. This timing is configuration evidence; verify it during a
disposable live shutdown drill before relying on it operationally.

## Buckets

`createbuckets` creates four buckets, and they are separate because their
rules differ, not for tidiness.

| Bucket              | Holds                                        | Serving                                      | Retention                        |
| ------------------- | -------------------------------------------- | -------------------------------------------- | -------------------------------- |
| `public-assets`     | immutable learner-facing problem assets      | public immutable URLs                        | indefinite, versioned            |
| `private-content`  | sources, archives, renders, course banners   | authorized delivery only, never public paths | indefinite, versioned            |
| `student-records`  | exports, uploaded responses, annotated exams | 5-minute authorized URLs, always logged      | explicit expiration and deletion |
| `temp-processing`  | extraction and conversion workspaces         | never served                                 | lifecycle rule, days             |

A course deletion removes `student-records` artifacts and leaves shared
content domains
untouched. Separate buckets make that a policy rather than a filter over a
shared prefix.

## First run

`source source_me.sh && python3 local_stack.py start [--no-open]` is the normal
developer lifecycle. It resolves the fixed `ple-live-demo-browser` project and
calls the canonical production-browser owner. The owner holds one lease through
build, bootstrap, migration, seed, renderer provenance, readiness, and cleanup.

```bash
source source_me.sh && python3 local_stack.py start
source source_me.sh && python3 local_stack.py start --no-open
```

`start` always builds production `dist/`, regenerates the disposable stack, and
waits for the HTTPS origin. Without `--no-open` it opens that origin; with
`--no-open` it prints the origin for a headless or manually opened browser.
Use the visible seeded production-auth UI to choose one of the five fixed
personas and then an authorized course. The server resolves the persona to the
ordinary account, account session, tenant session, and stored role state. The
start boundary accepts no project, environment, identity, SMTP, or skipped-build
option.

The lifecycle returns success only after `postgres`, `minio`,
`webwork-renderer`, `api`, `worker`, and `gateway` are running, every declared
health check is healthy, and the required one-shot services have exited with
status zero. Developer and browser tests serialize through the same owner lease.

For startup failures, preserve the private owner receipt and follow
[TROUBLESHOOTING.md](TROUBLESHOOTING.md). The owner does not authorize a
caller-selected developer project.

The API receives only the owner-generated runtime capabilities required by the
production-auth seed. The browser never receives those capabilities. SMTP and
deployment-provider setup are outside this fixed local developer session.

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

The owner creates renderer JWT secrets in private disposable state. They
authenticate API-to-renderer requests and responses; they never enter browser
data. The lifecycle records the selected OCI configuration ID and runs the
renderer probe before the API starts.

The renderer is stateless. Recreating it loses no PLE record. PostgreSQL and
MinIO retain records in named volumes outside their writable container layers;
normal `down` and rebuild operations preserve those volumes.

The startup probe is not a substitute for PLE integration or browser testing.
The browser-free renderer oracle and the canonical browser selection are the
supported evidence paths; see [USAGE.md](USAGE.md#build-and-validation-commands).

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
fails closed. The normal lifecycle nevertheless requires the renderer to pass
its semantic startup probe before it starts the API.

Use the HTTPS origin printed by `start` when probing `/health`; the owner may
select a free loopback port for the disposable run.

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

Use the controller for ordinary inspection. Raw Compose remains a diagnosis or
recovery tool for the owner, not the normal lifecycle interface.

## Common commands

```bash
source source_me.sh && python3 local_stack.py start          # build, start, wait, and open
source source_me.sh && python3 local_stack.py start --no-open
source source_me.sh && python3 local_stack.py stop           # authenticated cleanup
```

`start` and `stop` are the only developer-session mutations. They do not accept
project, environment, identity, SMTP, or build selectors. Cleanup is exact and
owner-scoped; it does not retain a caller-selected data project.

The fixed owner performs its own exact cleanup. Use
`source source_me.sh && python3 local_stack.py stop` after diagnostics or when
finished; do not use a project selector, confirmation target, or global Podman
cleanup.

The developer and browser runners use one private manifest, capability, and
lease through the canonical owner. On cleanup failure, retain its private
evidence and inspect the owner receipt before retrying; do not broaden cleanup.

Every disposable owner uses `podman-compose --in-pod false`. Disposable pods are intentionally
forbidden because Podman Compose does not attach the resource labels needed for the controller's
capability-bound discovery and cleanup proof to a provider-created pod.

The disposable capability coordinates cooperative processes running as the
same local user; mode 0600 prevents accidental disclosure but does not isolate
against a malicious same-UID process. Before any mutation, the adapter checks
the runner-held capability digest on every labelled resource. Once discovery
proves that no labelled resource remains, it may remove only the owner's exact
project-derived image tags (never an image ID, default tag, or shared image).

The database-baseline owner holds its mode-0700 runtime workspace while it
generates and validates the private manifest and companion files. Browser and
visitor processes have no path into that workspace. Immediately before Compose
starts PostgreSQL, the owner revalidates the bound administrative password;
the resulting administrative database connection provides the post-start
behavioral attestation. This boundary trusts the local stack owner: a
same-UID Podman administrator already controls the engine and its mounts, so
the runtime contract focuses on preventing accidental disclosure and confused
cross-process configuration rather than treating that administrator as a
separate tenant.

## Whole-system verification

The maintained non-browser E2E runner builds on the ordinary repository
artifacts and uses disposable, loopback-only Compose projects. Its
`replica_restart` profile is the only two-API-replica service oracle; PostgreSQL
remains singular:

```bash
bash tests/e2e/e2e_run_all.sh
```

It is designed to exercise the Wasm bridge, PostgreSQL migration/RLS/live
oracle suite, and a real learner submission across two API replicas after
stopping the replica that issued the question. The gateway image is derived from the pinned
official digest and strips Caddy's unnecessary low-port file capability before
running as UID 1000 with `cap_drop: ALL`. The replica lane builds the current
checkout into exact nonce-scoped application and gateway tags, then removes
only those tags after label discovery proves the project is empty. Cleanup is
attempted on both success and failure; an unproved cleanup fails nonzero and
retains its private recovery evidence. The runner never targets a long-lived
development project.

Permanent fast tests protect controller parsing, ownership, topology, and
other deterministic contracts. Real Podman, PostgreSQL, MinIO, renderer,
restart, and browser evidence belongs to an explicit disposable or live
acceptance command; it is not a regular pytest dependency. The complete
required command set for a goal is the active plan's Validation test suite;
see [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md#validation-test-suite).

Named volumes `ple_pgdata` and `ple_miniodata` survive `down`. The lifecycle
runs the read-only `postgres-major-guard` before it starts PostgreSQL and
accepts only a missing data directory or a populated PostgreSQL 17 directory.
PostgreSQL data directories are not compatible across major versions. Upgrade
through a documented, non-destructive migration: back up and verify the old
cluster, create a new PostgreSQL-major volume, restore into it, validate the
migration ledger and application behavior, then retain the old volume until
recovery is accepted. Removing either volume destroys local data, so it is a
deliberate step rather than part of the normal stop command.

## Image shape

`Containerfile.api` is a two-stage build. The `api` Compose service is the
single build owner of `localhost/peptidyle-learning-engine:local`; `worker`
uses that exact image with its own command and runtime settings. The lifecycle
builds the application image once, then builds the gateway, and only then
starts API, worker, and gateway. This prevents duplicate concurrent Cargo
builds from exhausting a constrained Podman machine. The first stage compiles
the Cargo workspace with `--locked`, so the image cannot quietly resolve a
different dependency set than `Cargo.lock` records. The second stage carries
only the binary and `ca-certificates`, and runs as a non-root user.

Manifests are copied before sources so dependency compilation caches
separately from source edits.

The builder follows the current stable Rust channel declared by
[rust-toolchain.toml](../rust-toolchain.toml).

## Health check inside the image

The container `HEALTHCHECK` runs the API binary with `--health-probe`, which
opens an HTTP request to its own `/health` and exits non-zero on anything but 200. Doing it this way keeps the runtime image free of `curl` and `wget`.
