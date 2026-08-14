# Local stack architecture

This document answers one operational question: why does each container in the
PLE local stack exist? The authoritative configuration is
[`containers/compose.yaml`](../containers/compose.yaml). The normal operator
path is `local_stack.py`, which delegates bootstrap and startup to
`launch_local_stack.sh`. This keeps
Compose lifecycle discovery, scoped logs, confirmation, and acceptance
preflight in one Python controller while the launcher remains the only owner
of build, migration, seed, renderer, and readiness sequencing.

The normal stack includes PLE's standalone WeBWorK PG renderer. SMTP is the one
optional overlay because PLE connects to an external mail provider rather than
operating a mail server.

This is intentionally an HTTP, loopback-only development topology. Caddy does
not emit HSTS locally: a browser must not be instructed to require HTTPS for a
development origin that intentionally has no local TLS endpoint. HSTS belongs
to the production CloudFront edge.

Before the API starts, the host launcher uses the production PostgreSQL and MinIO contracts to
publish the reviewed Genetics and Biochemistry Chapter 1 assignments. This host-only bootstrap
does not add a content-management service or expose source bytes to the browser.

## Long-running services

| Service            | Necessary role                                                                                                                                                 | Durable state                                                              | Network boundary                                                                 |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `gateway`          | Serves the built browser client and forwards same-origin `/api` and `/health` requests to the API. It is the only PLE browser entry point.                     | None. The built `dist/` directory is mounted read-only.                    | Publishes one loopback port; joins only `gateway_api`.                           |
| `api`              | Authenticates sessions, authorizes course actions, coordinates attempts, and translates private backend results into browser-safe PLE responses.               | None in the container. Authoritative records live in PostgreSQL and MinIO. | Joins the data network, `gateway_api`, and `renderer_private`.                   |
| `worker`           | Claims durable background jobs for retention, exports, imports, score maintenance, and item analysis.                                                          | None in the container. Its leases and job state live in PostgreSQL.        | Joins only the data network.                                                     |
| `postgres`         | Stores relational platform authority: identities, courses, memberships, assignments, attempts, submissions, scores, jobs, and audit records.                   | `ple_pgdata`, a named volume mounted at PostgreSQL's data directory.       | Publishes a loopback development port and joins the data network.                |
| `minio`            | Stores typed objects too large or inappropriate for relational rows: content packages, protected learner artifacts, exports, and temporary processing objects. | `ple_miniodata`, a named volume mounted at `/data`.                        | Publishes loopback development API and console ports and joins the data network. |
| `webwork-renderer` | Runs the external `webwork-pg-renderer` image to execute PG/PGML render and grade requests. It is an engine, not a second assignment platform.                 | None. It has no volume and no SQL database.                                | Joins only `renderer_private`; it has no host-published port.                    |

PostgreSQL is replaceable as a container, but the database service is not
semantically stateless. Its data is correctly outside the writable container
layer in the named `ple_pgdata` volume. Removing and recreating the container
keeps that volume unless the operator explicitly requests volume deletion.

The external renderer is genuinely stateless from PLE's perspective. PLE owns
immutable question source, attempt state, replay mapping, and grades. A
renderer restart therefore cannot lose an educational record.

Every long-running local service is non-root where its upstream image permits,
has a read-only root filesystem, drops all Linux capabilities, sets
`no-new-privileges`, and has bounded CPU, memory, process, and writable-tmpfs
budgets. The exception is not a broad privilege grant: the networkless,
one-shot `local-data-volume-permissions` helper starts as root inside the
rootless Podman user namespace with only `CAP_CHOWN`, then exits after setting
the PostgreSQL volume root and retained MinIO tree owners. These controls
contain accidental service escape and resource exhaustion; they do not make a host or Podman-socket
administrator unable to read local development data.

## One-shot services

These containers run a bounded initialization task and exit successfully. They
are preferable to giving long-running application containers elevated startup
permissions.

| Service                | Necessary role                                                                                                     | Safety property                                                                                                             |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| `local-data-volume-permissions` | Assigns the PostgreSQL volume root and retained MinIO tree to the fixed daemon UIDs. | Rootless, networkless, one-shot, read-only-root task with only `CAP_CHOWN`; it changes ownership metadata and does not remain running. |
| `postgres-major-guard` | Reads an existing `PG_VERSION` before PostgreSQL starts.                                                           | Read-only volume, no network, and refusal when the retained volume is not PostgreSQL 17. It never migrates or deletes data. |
| `createbuckets`        | Creates the four required MinIO buckets idempotently.                                                              | It exits after setup; API and worker do not need bucket-administration behavior.                                            |
| `identity-secret-init` | Copies the host-owned invitation issuer and Question ID capabilities into an API-only runtime volume with the fixed API UID and mode 0600. | Networkless with a minimal capability set; raw host paths are not mounted into the API, and the worker does not receive this volume. |
| `smtp-secret-init`     | When the SMTP overlay is selected, copies an external provider credential into an API-readable runtime volume.     | No network; PLE never starts a mail-transfer service.                                                                       |

Stopped successful one-shot containers may appear in `podman ps -a`. They are
not failed daemons and consume no running CPU after completion.

`local_stack.py status` makes that distinction explicit. A required one-shot
is complete only when it exited zero; a required long-running service is ready
only when it is running and healthy, except that the worker is ready when its
supervised process runs because it has no HTTP health check. The controller
reports duplicate labelled service instances as a failure rather than choosing
one. Selecting `--with-smtp` requires `smtp-secret-init`, and status also
infers that overlay from its labelled initializer or runtime volume when the
operator omitted the flag.

## Volumes

| Volume                 | Owner                             | Meaning                                                                                |
| ---------------------- | --------------------------------- | -------------------------------------------------------------------------------------- |
| `ple_pgdata`           | PostgreSQL                        | Durable relational authority. Preserve it across normal `down` and rebuild operations. |
| `ple_miniodata`        | MinIO                             | Durable object bytes and metadata. Preserve it with the relational volume.             |
| `ple_identity_runtime` | Secret initializer and API        | Runtime-only permission-normalized invitation issuer and Question ID capability copies, mounted only by the API; not educational records. |
| `ple_smtp_runtime`     | Optional SMTP initializer and API | Runtime-only external provider credential copy. Present only with the SMTP overlay.    |

PostgreSQL, MinIO, and `createbuckets` use fixed non-root identities, immutable
container roots, empty capability sets, `no-new-privileges`, bounded resources,
and bounded non-executable temporary filesystems. PostgreSQL writes only its
data volume plus an ephemeral Unix-socket directory; MinIO writes only its data
volume; `createbuckets` writes only temporary client configuration. The retained
data volumes are deliberately local-development state, not a host-compromise
barrier: a user who controls the rootless Podman socket or host account can read
them. Production protection is separately owned by managed database/object
storage, IAM, and KMS controls.

The normal stop command intentionally omits `--volumes`:

```bash
source source_me.sh && python3 local_stack.py stop
```

The controller requires `reset --confirm-project containers` before removing default-stack named
volumes. An explicit backup or migration procedure should precede any operation that removes
`ple_pgdata` or `ple_miniodata`.

## Networks

| Network              | Members                                          | Purpose                                                                                              |
| -------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| default data network | `postgres`, `minio`, `api`, `worker`, setup jobs | Relational and object-storage communication.                                                         |
| `gateway_api`        | `gateway`, `api`                                 | Same-origin browser delivery without publishing the API directly.                                    |
| `renderer_private`   | `api`, `webwork-renderer`                        | Private PG render/grade traffic. The browser, gateway, worker, PostgreSQL, and MinIO do not join it. |

There is no `webwork_db_private` network because PLE does not run WeBWorK2 or
MariaDB. WebWork2 remains reference material for application behavior; the
runtime integration is the smaller external PG renderer.

## External components

The image named by `PLE_WEBWORK_RENDERER_IMAGE` is built or obtained from the
separate `webwork-pg-renderer` project. PLE verifies that the image exists,
records its OCI identity, probes real render and grade behavior, and consumes
its documented API. PLE does not copy the renderer implementation into this
repository.

`OTHER_REPOS/pg`, `OTHER_REPOS/webwork2`, and
`OTHER_REPOS/webwork-pg-renderer` are comparison snapshots only. They are not
Compose build contexts, mounts, imports, or runtime dependencies.

An unrelated container such as a manually started `pg-test` is not a PLE
service. Compose project labels, rather than a name resemblance, determine
whether the PLE launcher may manage a container.

## Failure behavior

- If PostgreSQL or MinIO is unavailable, API readiness reports degradation
  because authoritative state cannot be accessed safely.
- If the renderer is unavailable while the API is running, WeBWorK-backed
  questions fail closed. Native questions and stored records remain intact.
- Restarting or recreating the renderer requires no data recovery.
- A supported full start reattaches PostgreSQL and MinIO to their named volumes.
- The controller limits individual restart to the stateless API, worker,
  gateway, and renderer services; it does not independently restart stateful
  storage services.
- Worker failure leaves durable jobs available for a later worker lease.
- Gateway failure removes browser reachability but does not mutate records.

## Verification tiers

Fast permanent tests inspect durable topology and security properties: the
renderer has no database, volume, host port, or browser network; required
services remain in the normal stack; and controller parsing, project ownership,
and cleanup confirmation preserve their bounded contracts.

Live container and browser behavior belongs in the explicit E2E lane:

```bash
tests/e2e/e2e_webwork_render_rpc.sh
bash tests/e2e/e2e_chapter_one_pilot.sh
bash tests/e2e/e2e_chapter_one_browser.sh
```

The renderer acceptance script creates its licensed one-question WebWork fixture explicitly after
the canonical launcher is ready. Its answer-free manifest is private temporary test state; the
normal launcher and canonical teaching walkthrough publish only the reviewed Chapter 1 corpus.

The renderer gate exercises render, grade, cache, outage recovery, and browser non-disclosure. The
Chapter 1 publication gate publishes the exact two-by-four release matrix into isolated PostgreSQL
and MinIO, then proves an exact rerun. The Chapter 1 browser gate completes those eight questions
through visible keyboard controls in a complete isolated PLE stack. They are explicit E2E evidence,
not permanent pytest cases or a claim that every PG problem is compatible.

The aggregate live browser command is `source source_me.sh && python3
local_stack.py acceptance`. It runs a read-only conflict preflight first and
refuses default or walkthrough projects with retained containers, so it cannot
silently reuse, stop, or delete another local run. The active plan names the
full Validation test suite; [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md#validation-test-suite)
defines why permanent offline checks and opt-in live acceptance remain separate.

See [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for operating commands and
[MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md) for replica and production
boundaries.

The local renderer has a pinned image reference and private network, but that
evidence does not approve an AWS renderer. Production keeps that feature
disabled until the external service is separately attested for private ingress,
image provenance, TLS identity, no database/object-store authority, and
fail-closed behavior.
