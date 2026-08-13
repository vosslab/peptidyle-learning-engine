# Local stack architecture

This document answers one operational question: why does each container in the
PLE local stack exist? The authoritative configuration is
[`containers/compose.yaml`](../containers/compose.yaml). The maintained startup
path is [`launch_local_stack.sh`](../launch_local_stack.sh).

The normal stack includes PLE's standalone WeBWorK PG renderer. SMTP is the one
optional overlay because PLE connects to an external mail provider rather than
operating a mail server.

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

## One-shot services

These containers run a bounded initialization task and exit successfully. They
are preferable to giving long-running application containers elevated startup
permissions.

| Service                | Necessary role                                                                                                     | Safety property                                                                                                             |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| `postgres-major-guard` | Reads an existing `PG_VERSION` before PostgreSQL starts.                                                           | Read-only volume, no network, and refusal when the retained volume is not PostgreSQL 17. It never migrates or deletes data. |
| `createbuckets`        | Creates the three required MinIO buckets idempotently.                                                             | It exits after setup; API and worker do not need bucket-administration behavior.                                            |
| `identity-secret-init` | Copies the host-owned invitation issuer secret into an API-readable runtime volume with the API UID and mode 0600. | No network and a minimal capability set; the host path is not exposed to the API.                                           |
| `smtp-secret-init`     | When the SMTP overlay is selected, copies an external provider credential into an API-readable runtime volume.     | No network; PLE never starts a mail-transfer service.                                                                       |

Stopped successful one-shot containers may appear in `podman ps -a`. They are
not failed daemons and consume no running CPU after completion.

## Volumes

| Volume                 | Owner                             | Meaning                                                                                |
| ---------------------- | --------------------------------- | -------------------------------------------------------------------------------------- |
| `ple_pgdata`           | PostgreSQL                        | Durable relational authority. Preserve it across normal `down` and rebuild operations. |
| `ple_miniodata`        | MinIO                             | Durable object bytes and metadata. Preserve it with the relational volume.             |
| `ple_identity_runtime` | Secret initializer and API        | Runtime-only permission-normalized invitation secret copy, not an educational record.  |
| `ple_smtp_runtime`     | Optional SMTP initializer and API | Runtime-only external provider credential copy. Present only with the SMTP overlay.    |

The normal stop command intentionally omits `--volumes`:

```bash
podman compose -f containers/compose.yaml \
  --env-file containers/env.local down --remove-orphans
```

An explicit backup or migration procedure should precede any operation that
removes `ple_pgdata` or `ple_miniodata`.

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
- Restarting PostgreSQL or MinIO reattaches their named volumes.
- Worker failure leaves durable jobs available for a later worker lease.
- Gateway failure removes browser reachability but does not mutate records.

## Verification tiers

Fast permanent tests inspect durable topology and security properties: the
renderer has no database, volume, host port, or browser network; required
services remain in the normal stack; and the launcher exposes no legacy
WebWork option.

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

See [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for operating commands and
[MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md) for replica and production
boundaries.
