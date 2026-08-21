# Local stack operations

Local development stack for the Peptidyle Learning Engine: the browser gateway,
API server, worker, PostgreSQL, MinIO, and private standalone WeBWorK PG
renderer. The normal Compose model layers
`containers/compose.local-development.yaml`
after [containers/compose.yaml](../containers/compose.yaml); the local overlay owns
local-file authentication and local worker commands. SMTP is the optional third
overlay. The live-demo owner deliberately uses the base plus its TLS overlay
without the local-development file. The image model is in
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

The typed lifecycle builds this local bundle with
`PLE_BROWSER_LOCAL_DEVELOPMENT_AUTH=1`, which includes the local credential
form only alongside the server's explicit local login route. An ordinary
`./build.sh` leaves that build capability disabled for production artifacts.

The loopback gateway is deliberately HTTP-only and does not set HSTS. It is a
local development origin, not a production TLS edge; production HSTS is owned
by CloudFront. Do not treat a local browser check as evidence of edge-header
behavior.

macOS setup for the Podman virtual machine lives in
[MACOS_PODMAN.md](MACOS_PODMAN.md).

## Services

| Service                | Image                                       | Purpose                                           | Local port                   |
| ---------------------- | ------------------------------------------- | ------------------------------------------------- | ---------------------------- |
| `gateway`              | pinned official Caddy derivative            | browser files plus same-origin API gateway        | 127.0.0.1:8080               |
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

This is a local-development containment boundary, not an authorization boundary
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

`source source_me.sh && python3 local_stack.py` is the normal operator-facing
lifecycle. It resolves the explicit `containers` project and selected environment,
reports labelled resources, and calls focused typed Python lifecycle modules directly.
Those modules own build, bootstrap, migration, seed, renderer provenance, polling,
and bounded stateless restart.

```bash
source source_me.sh && python3 local_stack.py start
source source_me.sh && python3 local_stack.py start --skip-build --no-open
```

On its first default run, the typed lifecycle creates an ignored mode-0600
`containers/env.local`, generates independent database/object-store/grader,
invitation-issuer, and Question ID capability secrets, generates instructor
and student bearer credentials,
and mounts only their hashes into the API. It builds the host artifacts, starts
PostgreSQL and MinIO, applies and verifies the embedded migrations, provisions the restricted
`ple_grading_reader` login, publishes the two Chapter 1 assignments with four native and four
WeBWorK questions, verifies the external PG renderer, starts the API/worker/gateway,
waits for semantic `/health`, and opens the browser. Named data volumes remain
available for repeated testing. The default gateway port is `8080`. If its
selected port is occupied during first-run bootstrap, the lifecycle records the
first available port from 8000 through 8099 in the ignored env file. An existing
explicit `PLE_GATEWAY_HOST_PORT` remains an operator choice until it is changed.

The explicitly selected environment file is authoritative for Compose
interpolation and host-side migration connections; inherited shell variables
with the same names do not create split credentials.

The lifecycle returns success only after `postgres`, `minio`,
`webwork-renderer`, `api`, `worker`, and `gateway` are running, every declared
health check is healthy, the required one-shot services have exited with status
zero, a project-wide pre-start reconciliation has replaced every prior project
container and removed Compose orphans without deleting named volumes, and
every image not used by a current container has been pruned. The active full
suite protects its current images; obsolete application, renderer, gateway,
base, and intermediate builds do not accumulate. Live/full-stack Playwright
starts only after that success. The local demo-preview browser suite is a
separate behavior lane and does not claim Podman-stack acceptance.

The recovery screen accepts either generated value from the ignored
`containers/local-login.txt`. The browser sends it once to the same-origin
local login endpoint; the established credential is the existing HttpOnly
session cookie, not local or session storage. Do not copy these local files to
a deployed environment.

Start with read-only inspection when diagnosing a local stack:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs
source source_me.sh && python3 local_stack.py validate
source source_me.sh && python3 local_stack.py status --project containers
```

`doctor`, `projects`, `status`, `logs`, and `validate` are read-only. `status`
and `logs` may name another project with `--project`, but lifecycle mutations
always target the explicit default `containers` project. `validate` performs
the typed lifecycle configuration check and then reports the observed runtime
state; it does not bootstrap, start, repair, or otherwise mutate containers.

Use `python3 local_stack.py validate` for a read-only configuration preflight,
`--no-open` on a headless machine, or `--skip-build` when the existing `dist/`
bundle is intentionally current. A custom `--env-file` is never rewritten or
seeded and must provide every required secret itself. `npm run launch` remains
an optional alias for the normal public `start` command.

The API reads the mode-0600 invitation issuer and Question ID capability from a
read-only named volume populated by a networkless one-shot initializer running
under the pinned Alpine image. The API alone mounts that runtime copy; the
worker does not receive it. This makes Instructor copy-link invitations
available without SMTP and keeps the raw capabilities out of environment
variables. PLE uses its established Rust SMTP adapter only when an operator
supplies provider settings; the local stack does not run or maintain a mail
server.

## External SMTP provider

SMTP is an opt-in connection to an operator-selected service, not another PLE
container. No provider is configured today. Fastmail is the intended future
provider, but keep the normal stack and no-email teaching walkthrough unchanged
until its account, authorized sender, and application credential exist. When it
does, copy `containers/env.example` to an operator-owned environment file and
set:

- `PLE_SMTP_RELAY` to the provider hostname, without `smtp://` or `smtps://`;
- `PLE_SMTP_PORT` and `PLE_SMTP_TLS_MODE` to either mandatory `starttls`
  submission or `implicit-tls` submission, as specified by the provider;
- `PLE_SMTP_USERNAME` and `PLE_SMTP_FROM` to provider-authorized values;
- `PLE_SMTP_PASSWORD_HOST_FILE` to an absolute, non-symlink, mode-0600 file
  containing only the provider-issued SMTP password or token; and
- `PLE_PUBLIC_APP_BASE_URL` to the deployed public HTTPS PLE origin.

Preflight and start that configuration explicitly:

```bash
source source_me.sh && python3 local_stack.py validate \
  --env-file path/to/env.local --with-smtp
source source_me.sh && python3 local_stack.py start \
  --env-file path/to/env.local --with-smtp --no-open
source source_me.sh && python3 local_stack.py stop \
  --env-file path/to/env.local --with-smtp
```

The SMTP overlay copies the credential through a networkless, capability-minimal
one-shot container into an API-readable, read-only named volume. The API never
receives the host path or credential text in its environment. Omitting
`--with-smtp` passes no SMTP configuration to the API; copy-link invitations
continue to work, while email sign-in remains unavailable until the external
provider is configured. PLE does not manage sender reputation, DNS mail policy,
bounces, or provider accounts.

Configured course-invitation email is a leased, durable worker outbox, not an
API-request send. A failed worker attempt records one operator-only
`smtp_delivery_failed` event in its server-minted opaque `delivery_id` span.
It contains only one outcome, `known_rejected` for an explicit SMTP rejection
or `ambiguous` when SMTP might have accepted the message before the transport
failed, plus one category: `dns_or_connectivity`, `tls_handshake`,
`authentication`, or `provider_rejection`. It contains no recipient, link,
token, username, password, or provider response.

The browser receives only the invitation's durable coarse state: `queued`,
`sentToProvider`, `needsAttention`, or `cancelled`. `sentToProvider` is not
mailbox-delivery confirmation, and an ambiguous attempt is never retried
automatically. Passwordless sign-in and email-change messages are synchronous
API delivery attempts; their browser response remains the generic unavailable
outcome while the same redacted event may inherit the server-minted request
correlation span.

### Credential rotation and revocation

`ple_smtp_runtime` is a retained named volume. A normal `stop` preserves its
copied credential so the next normal start can use the selected SMTP overlay;
stopping the stack does not revoke or erase the runtime copy.

For a bounded credential rotation, first create the replacement provider
credential and update the mode-0600 host credential file named by
`PLE_SMTP_PASSWORD_HOST_FILE`. Then stop and restart the selected overlay so
the networkless initializer overwrites the API-readable runtime copy:

```bash
source source_me.sh && python3 local_stack.py stop \
  --env-file path/to/env.local --with-smtp
source source_me.sh && python3 local_stack.py start \
  --env-file path/to/env.local --with-smtp --no-open
```

After the replacement start is healthy, revoke the old provider credential.
If a credential must be revoked without replacement, stop the overlay, revoke
it at the provider, and deliberately remove the retained runtime copy with the
default-project reset. This reset removes all default-project named data, not
only the SMTP credential volume, so preview the exact target and back up any
needed local PostgreSQL or MinIO data first:

```bash
source source_me.sh && python3 local_stack.py reset --dry-run --with-smtp
source source_me.sh && python3 local_stack.py reset \
  --confirm-project containers --with-smtp
```

The reset retains the host credential file. Remove or replace that mode-0600
host file through the operator's secret-management procedure before any later
`--with-smtp` start. These local controls do not prove acceptance by an SMTP
provider; live provider delivery remains a separate pre-production gate.

The private typed lifecycle is the maintained startup path because API/worker startup is deliberately
later than migration and grader-role provisioning. Running a bare
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
values. The lifecycle records the selected OCI configuration ID in an ignored provenance
file and runs `containers/webwork/probe_render_api.sh` inside the container to
exercise both rendering and grading before the API starts.

The renderer is stateless. Recreating it loses no PLE record. PostgreSQL and
MinIO retain records in named volumes outside their writable container layers;
normal `down` and rebuild operations preserve those volumes.

### Deliberate renderer outage

`service stop webwork-renderer` is not a routine lifecycle command. It exists
only to prove or diagnose the narrowly scoped WebWork outage: it requires one
running, label-resolved renderer in the default `containers` project, prints
the exact Compose command, stops that one service, and proves that labelled
volumes and networks did not change.

```bash
source source_me.sh && python3 local_stack.py service stop webwork-renderer
source source_me.sh && python3 local_stack.py restart webwork-renderer
```

Use `restart` to restore the service. The command cannot stop PostgreSQL,
MinIO, API, worker, gateway, an arbitrary container, or a disposable project.

The startup probe is not a substitute for PLE integration or browser testing.
The repository contains explicit E2E gates for the bounded licensed PGML
`RadioButtons` path and four reviewed Chapter 1 sources, covering render,
grading, cache, renderer outage recovery, keyboard use, and protected-material
non-disclosure. Run them in a disposable stack before treating that behavior as
current live evidence. Broader PG compatibility always requires its own
reviewed source and live evidence.

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

Use the controller for ordinary inspection. Raw Compose remains a diagnosis or
recovery tool when its exact command is necessary, not the normal lifecycle
interface. Exercise a deliberate database-outage rehearsal only through its
own disposable E2E runner, rather than interrupting teaching data in the
default project.

The private lifecycle accepts a non-default project only when its closed runner
supplies the mode-0600 cleanup capability whose SHA-256 commitment is recorded
in that project's private environment. A project name and environment path
alone do not authorize a disposable launch.

## Common commands

```bash
source source_me.sh && python3 local_stack.py start          # build, start, wait, and open
source source_me.sh && python3 local_stack.py start --skip-build --no-open
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs gateway api worker
source source_me.sh && python3 local_stack.py logs --follow api worker
source source_me.sh && python3 local_stack.py restart api
source source_me.sh && python3 local_stack.py stop           # retains named volumes
```

`restart` is intentionally limited to the stateless `api`, `worker`,
`gateway`, and `webwork-renderer` services. It delegates to the typed lifecycle so
the restarted service still receives the appropriate readiness and dependency
checks. Restarting PostgreSQL or MinIO individually is not a controller
operation; preserve their data and use the supported start path or a named
disposable E2E/recovery procedure.

## Read-only status

`status` reports semantic readiness, rather than merely listing containers.
`ready` means every required long-running service is running and healthy (the
worker deliberately has no HTTP health check) and every required one-shot
service exited with status zero. `starting`, `partially-active`, `failed`,
`stopped-with-data`, and `absent` distinguish incomplete topology, missing
services, failed or duplicate services, retained data with no active stack,
and no labelled resources. A one-shot container that exited zero is successful
and does not consume CPU; it is not a stopped daemon.

The selected `--with-smtp` topology requires `smtp-secret-init`. Conversely,
status infers the SMTP overlay if its labelled initializer or runtime volume is
present, so a persisted overlay is not misread as the normal no-email topology.
Use `status --json` for a structured non-secret report. `projects` lists every
labelled Compose project, including a project that currently has only retained
volumes.

`logs` defaults to `gateway`, `api`, and `worker`; it accepts only services in
the selected topology and warns that application diagnostics may contain
private local data. Prefer this scoped command over an unfiltered engine log
dump.

To intentionally discard disposable pre-production data, preview the exact target first and then
confirm the visible project name:

```bash
source source_me.sh && python3 local_stack.py reset --dry-run
source source_me.sh && python3 local_stack.py reset --confirm-project containers
source source_me.sh && python3 local_stack.py start --no-open
```

`reset` removes only the default project's labelled Compose containers,
networks, and named volumes through `down --volumes --remove-orphans`. The dry
run prints the exact resource snapshot, database-bound host manifest
`containers/local-chapter-one-pilot.json`, and command. The mutating form
requires the literal confirmation `--confirm-project containers`. After label
discovery proves the Compose resources and volumes are gone, it removes that
private Chapter 1 manifest so the next lifecycle run publishes a fresh
database-bound corpus. It retains host credentials such as
`containers/env.local`, `containers/local-login.txt`, and mode-0600 capability
files. The reset itself does not mutate the global image store. The next
successful ordinary start prunes every image not used by a current container.

Disposable E2E runners use their own private manifest plus a runner-held
cleanup capability through private `local_stack_control/_consumer_cli.py`; that adapter is not a
general operator cleanup command. On cleanup failure, the runner exits
nonzero and retains its private evidence directory or manifest path for
inspection. Do not delete that evidence before the label-resolved target is
inspected and its exact cleanup is retried. `PLE_E2E_KEEP=1` intentionally
retains the owner-created target and its evidence for diagnosis.

Every disposable owner uses `podman-compose --in-pod false`. Disposable pods are intentionally
forbidden because Podman Compose does not attach the resource labels needed for the controller's
capability-bound discovery and cleanup proof to a provider-created pod.

The disposable capability coordinates cooperative processes running as the
same local user; mode 0600 prevents accidental disclosure but does not isolate
against a malicious same-UID process. Before any mutation, the adapter checks
the runner-held capability digest on every labelled resource. Once discovery
proves that no labelled resource remains, it may remove only the owner's exact
project-derived image tags (never an image ID, default tag, or shared image).

## Whole-system verification

The maintained non-browser E2E runner builds on the ordinary repository
artifacts and uses disposable, loopback-only Compose projects:

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
