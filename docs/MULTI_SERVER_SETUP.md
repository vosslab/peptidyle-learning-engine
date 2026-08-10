# Multiple-server setup

This document explains the implemented local multi-replica topology and the
separate, planned production topology. It is an operations guide, not evidence
that AWS deployment has been accepted. The source contracts are
[containers/compose.yaml](../containers/compose.yaml),
[containers/Caddyfile](../containers/Caddyfile), and the server composition in
[crates/server/src/composition.rs](../crates/server/src/composition.rs).

## Scope and status

The supported local topology runs one Caddy gateway, one or more stateless API
replicas, one or more durable-worker replicas, one PostgreSQL 17 instance, and
one MinIO instance. The native stack is the supported default. The optional
WeBWorK renderer and its MariaDB database are private adjuncts that support the
accepted, deliberately bounded WP-RC3 path: one licensed user-authored PGML
RadioButtons fixture. They do not imply broad WeBWorK compatibility.
A local two-API-replica restart test exists and has been used as the behavioral
proof that a learner can continue after the issuing API replica stops.

The target AWS Fargate, ALB, RDS, S3, CloudFront, WAF, KMS, Secrets Manager,
and OpenTofu deployment is a future WP-RC10 work package. It has a complete
acceptance contract in
[active_plans/active/release_completion_plan.md](active_plans/active/release_completion_plan.md),
but its files and disposable-cloud acceptance are not implemented or accepted.
Do not describe local PostgreSQL or MinIO as highly available, and do not use
this local topology as production deployment evidence.

## Local topology

```text
                         loopback host port only
browser ---------------------------------------------------+
  Solid/Wasm bundle and HttpOnly cookie                   |
                                                         v
                                                +----------------+
                                                | gateway / Caddy|
                                                | :8080 in stack |
                                                +--------+-------+
                                                         |
                              dynamic Compose DNS A records, round robin
                              gateway_api internal network
                        +----------------+---------------+----------------+
                        |                |                                |
                        v                v                                v
                  +-----------+    +-----------+                    +-----------+
                  | API replica|   | API replica|        ...         | API replica|
                  +-----+-----+    +-----+-----+                    +-----+-----+
                        |                |                                |
                        +----------------+-------------+------------------+
                                                         |
      +----------------------- default Compose network--+-----------------------+
      |                         |                        |                       |
      v                         v                        v                       v
+------------+           +------------+           +------------+          +------------+
| PostgreSQL |           |   MinIO    |           | worker x N |          | createbuckets|
| shared DB  |           | shared S3  |           | leases/jobs|          | one-shot     |
+------------+           +------------+           +------------+          +------------+

Optional native-to-WebWork private path:

API replicas -- renderer_private --> WebWork renderer -- webwork_db_private --> MariaDB
```

Only `gateway` publishes the browser/API port. API replicas have no host port,
so `--scale api=2` never causes a port collision. PostgreSQL and MinIO expose
loopback-only convenience ports for local migrations and inspection; they are
not browser routes. The complete local container policy is in
[CONTAINER.md](CONTAINER.md).

## Service and network matrix

| Component                 | Implemented role                                              | Network or host exposure                                       | Shared durable state                   | Scale rule                                                 |
| ------------------------- | ------------------------------------------------------------- | -------------------------------------------------------------- | -------------------------------------- | ---------------------------------------------------------- |
| `gateway`                 | Serves read-only `dist/`; proxies `/api`, `/api/*`, `/health` | Host `127.0.0.1:${PLE_GATEWAY_HOST_PORT}`; `gateway_api`       | None                                   | One local gateway; not an API session owner                |
| `api`                     | Axum HTTP routes, auth, issue, grade, and asset authorization | `default`, `gateway_api`, and `renderer_private`; no host port | PostgreSQL, S3-compatible object store | `--scale api=N`; each replica uses identical configuration |
| `worker`                  | Claims and completes bounded durable jobs                     | `default`; no host port                                        | PostgreSQL queue and object store      | `--scale worker=N`; each process claims one job at a time  |
| `postgres`                | Shared tenant records, sessions, attempts, idempotency, jobs  | `default`; loopback 5432 by default                            | `ple_pgdata`                           | One local instance; no local HA claim                      |
| `minio`                   | Shared S3-compatible object store                             | `default`; loopback 9000/9001 by default                       | `ple_miniodata`                        | One local instance; no local HA claim                      |
| `createbuckets`           | Idempotently ensures the three required buckets exist         | `default`; no host port                                        | MinIO buckets                          | One-shot, not scaled                                       |
| `webwork-renderer`        | Optional private upstream render/grade process                | `renderer_private`, `webwork_db_private`; no host port         | Upstream course files only             | Optional profile; no browser access                        |
| `webwork-db`              | Optional upstream MariaDB state                               | `webwork_db_private`; no host port                             | `ple_webwork_dbdata`                   | Paired with renderer; separate from PLE PostgreSQL         |
| `webwork-api-secret-init` | Copies strict render secret into API-private runtime volume   | No network                                                     | `ple_webwork_api_runtime`              | One-shot on each WebWork launch                            |

The named volumes preserve local data across normal `down`. Removing a volume
is destructive and is intentionally outside routine stop commands.

The launcher runs `postgres-major-guard` against `ple_pgdata` before starting
PostgreSQL. The guard accepts only an empty volume or a PostgreSQL 17 data
directory. PostgreSQL data directories are not cross-major compatible: a major
upgrade requires a verified backup, a new volume initialized by the target
major, restore and application validation, and retention of the old volume
until recovery is accepted. Do not bypass the guard or remove a populated
volume as an upgrade procedure.

## Gateway discovery and health

Caddy uses `dynamic a api 3000` rather than resolving one API container name at
gateway startup. It refreshes Compose DNS every two seconds and round-robins
the current A records. Network failures retain the bounded ten-second passive
failure window; a five-second active check calls each replica's semantic
`/health` endpoint and requires HTTP 200. The five-second retry window and
250 ms retry interval give a transient replica failure a bounded recovery
opportunity.

An API's `/health` endpoint is readiness, not a process liveness ping. It
rechecks the expected PostgreSQL migration states and checksummed compatibility
with a two-second bound, then performs a real object-store `HeadBucket` on the
content bucket. It returns `200 {"status":"ready"}` only when both pass;
otherwise it returns `503` with safe failing dependency names. Caddy actively
polls only this route. It deliberately does not classify every application 503
as replica failure: a feature-local dependency such as the private WebWork
renderer may fail one request closed while the same replica remains healthy
for authentication, courses, native questions, and navigation.

The worker intentionally has no HTTP readiness endpoint. It verifies schema
compatibility at startup, its supervisor restarts failed processes, and its
useful signal is its safe queue-depth/pass log. Do not direct browser traffic
to a worker.

## Stateless API contract

API replicas may be added or removed because persistent authority is not held
in process memory:

- The opaque HttpOnly session token is hashed and resolved, expired, and
  revoked through the shared PostgreSQL `SessionStore`. A login on one replica
  works on another, and a revocation is immediately shared.
- The authenticated session establishes tenant context server-side. Tenant IDs
  never arrive as authority-bearing browser parameters. PostgreSQL tenant-owned
  tables use forced row-level security; catalog content remains immutable and
  tenant-free. See [SECURITY_MODEL.md](SECURITY_MODEL.md) and
  [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).
- Attempts, run timing, question version, seed, grading backend, submissions,
  and submission idempotency are durable PostgreSQL records. A second replica
  reconstructs the same authorized attempt instead of trusting a browser copy.
- `S3ObjectStore` is an S3-compatible shared dependency. Its object identity,
  checksum, bucket policy, and signed-delivery decisions remain server-owned.
  No API replica owns a private object cache as correctness authority.
- Grading remains server-side. The API routes receive only an authorized learner
  response and load answer-bearing material through the restricted grader
  boundary; the browser never receives keys or grading logic.

Every API replica needs the same database, object-store, bucket, public-asset,
authentication, grader, and optional renderer configuration. A different
database, different bucket names, different local identity contents, or a
different renderer identity is a split-brain deployment, not a scale-out.

## Worker coordination

Workers are safe to scale because PostgreSQL grants an opaque lease capability
to exactly one claimant. A worker can complete or fail only its current lease;
stale lease tokens are rejected. A crash or dependency outage lets the lease
expire and a later worker reclaim the job with bounded retry/backoff. Completion
uses an atomic durable effect, so a retry cannot make two current exports,
scores, imports, or retention transitions visible.

`PLE_WORKER_LEASE_SECONDS` must stay within the code's bounded range and exceed
the normal preparation time. `PLE_WORKER_PREPARATION_TIMEOUT_SECONDS` bounds
one preparation pass, and `PLE_WORKER_POLL_MILLIS` controls idle polling. The
local defaults are 120, 90, and 500 respectively. Add worker replicas for
concurrency; do not turn one process into an unbounded in-process batch.

The current worker supports only its declared durable job families. A queue row
is not permission to execute an undeclared job family.

## Required configuration

Copy [containers/env.example](../containers/env.example) to the ignored
`containers/env.local`, or let [launch_local_stack.sh](../launch_local_stack.sh)
bootstrap the default local file. The launcher creates generated local secrets
and credentials with mode 0600; never commit or reuse them outside local work.

| Inputs                                                                           | Owner                                      | All replicas require same value?          | Purpose                                              |
| -------------------------------------------------------------------------------- | ------------------------------------------ | ----------------------------------------- | ---------------------------------------------------- |
| `DATABASE_URL`                                                                   | Compose from PostgreSQL settings           | Yes for API and worker                    | Shared application database                          |
| `PLE_GRADER_DATABASE_URL`                                                        | Compose/API only                           | Yes for API                               | Restricted answer-bearing reader                     |
| `PLE_S3_ENDPOINT`, `PLE_S3_REGION`                                               | Compose                                    | Yes for API and worker                    | Shared S3-compatible endpoint                        |
| `PLE_CONTENT_BUCKET`, `PLE_STUDENT_RECORDS_BUCKET`, `PLE_TEMP_PROCESSING_BUCKET` | Compose                                    | Yes for API and worker                    | Fixed policy-separated buckets                       |
| `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`                                     | Ignored local env; deployment secret store | Yes for API and worker                    | Object-store credentials                             |
| `PLE_PUBLIC_ASSET_BASE_URL`                                                      | Operator                                   | Yes for API                               | Safe public immutable asset base                     |
| `PLE_BIND_ADDR`                                                                  | Compose/API                                | Yes except binding address                | API listen address; `0.0.0.0:3000` remains internal  |
| `PLE_AUTH_PROVIDER`, `PLE_LOCAL_AUTH_FILE`                                       | Local development only                     | Yes for API                               | Local identity provider and hash-only identity file  |
| `PLE_WORKER_*`                                                                   | Operator                                   | Yes for workers                           | Bounded worker lease, deadline, and polling controls |
| `PLE_GATEWAY_HOST_PORT`                                                          | Local operator                             | Gateway only                              | Loopback browser entry port                          |
| `PLE_*_IMAGE_SHA256`                                                             | Operator/launcher                          | Applicable services                       | Immutable local image manifests                      |
| `PLE_WEBWORK_*`                                                                  | Opt-in profile operator                    | Yes for API and renderer where applicable | Private renderer identity, limits, and secrets       |

The API's `PLE_LOCAL_AUTH_FILE` is a read-only mount of hashes, not the adjacent
local bearer credentials. In an institutional deployment, the local-file
provider is replaced by the approved identity integration; it is not a
production multi-server authentication design.

## Start, scale, inspect, stop

Use the launcher for first startup because it orders configuration bootstrap,
PostgreSQL-major verification, migrations, grader-role provisioning, seed data,
and service readiness. Bare `compose up` against an empty database is not an
equivalent bootstrap path.

```bash
./launch_local_stack.sh --no-open
./launch_local_stack.sh --with-webwork --no-open
```

After the native stack is ready, scale API and worker replicas with the same
environment file. API replicas stay behind the gateway; workers remain private.

```bash
podman compose -f containers/compose.yaml --env-file containers/env.local \
  up -d --scale api=2 api gateway
podman compose -f containers/compose.yaml --env-file containers/env.local \
  up -d --scale worker=2 worker
podman compose -f containers/compose.yaml --env-file containers/env.local ps
podman compose -f containers/compose.yaml --env-file containers/env.local logs -f api worker
PLE_GATEWAY_HOST_PORT="$(awk -F= '$1 == "PLE_GATEWAY_HOST_PORT" { print $2 }' containers/env.local)"
curl --fail --silent --show-error "http://127.0.0.1:${PLE_GATEWAY_HOST_PORT}/health"
```

For the accepted, bounded WP-RC3 renderer path, the base Compose file supplies
the profile services and `compose.webwork.yaml` injects API renderer
configuration and its secret-runtime volume. Preserve both the overlay and the
profile on all subsequent operator commands. Matching and broader problem
compatibility remain assigned to WP-RC5:

```bash
podman compose -f containers/compose.yaml -f containers/compose.webwork.yaml \
  --env-file containers/env.local --profile webwork ps
podman compose -f containers/compose.yaml -f containers/compose.webwork.yaml \
  --env-file containers/env.local --profile webwork \
  up -d --scale api=2 --scale worker=2
podman compose -f containers/compose.yaml -f containers/compose.webwork.yaml \
  --env-file containers/env.local --profile webwork logs -f api worker webwork-renderer
```

Normal teardown retains volumes:

```bash
podman compose -f containers/compose.yaml --env-file containers/env.local down
podman compose -f containers/compose.yaml -f containers/compose.webwork.yaml \
  --env-file containers/env.local --profile webwork down
```

The launcher's final output is the authoritative gateway port if it selected a
free port other than 3000. Use that printed port for `curl` and browser access.

## Failure behavior

| Failure                             | Implemented response                                                                                                 | Operator action                                                                                  |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| One API replica stops               | Caddy rediscovers/retries peers; shared session and attempt state permit continuation                                | Inspect logs, replace the replica, run the replica restart E2E before changing topology          |
| API returns readiness `503`         | Caddy's active `/health` check removes it from rotation                                                              | Repair PostgreSQL/object-store access or schema compatibility; do not force traffic to it        |
| PostgreSQL unavailable/incompatible | API readiness is `503`; workers refuse schema-incompatible draining                                                  | Restore database service and expected migration state; do not fabricate migration rows           |
| Object store unavailable            | API readiness is `503`; object operations return a bounded unavailable result                                        | Restore endpoint, credentials, bucket, or network; preserve object checksum evidence             |
| Worker crashes after claim          | Lease expires; another worker can reclaim according to bounded policy                                                | Inspect safe worker logs and queue depth; scale workers only after dependency health is restored |
| Browser retries submission          | Durable idempotency returns the original authorized outcome rather than grading twice                                | Keep the same request identity and investigate repeated transport failure                        |
| Renderer/MariaDB unavailable        | Native API/readiness remains independent; WebWork-backed work returns a bounded 503 without evicting the healthy API | Restore only the private WebWork profile; never expose its port to the browser                   |
| Gateway fails                       | Browser origin is unavailable even though API replicas may be healthy                                                | Restart/repair gateway; do not publish API ports as an emergency browser bypass                  |

## Optional WebWork isolation

`./launch_local_stack.sh --with-webwork` enables a source-pinned WebWork2/PG
renderer. API connects only through `renderer_private` to the internal
`http://webwork-renderer:8080/webwork2/` base. The renderer and MariaDB share
only `webwork_db_private`; neither has a host port or joins PLE PostgreSQL,
MinIO, gateway, browser, or worker networks.

The render-course password and Mojolicious signing secret are independent
host-owned mode-0600 files. The renderer mounts them read-only. A networkless,
capability-minimal init service copies only the render password into a named
runtime volume owned by the API UID; API mounts that copy read-only. The
launcher refreshes the copy at every WebWork start. Browser payloads never carry
renderer endpoints, credentials, source bytes, upstream field names, or keys.

The native stack does not receive renderer configuration or renderer-secret
dependencies. This preserves native question availability when the optional
renderer is absent or unhealthy.

## Validation evidence

Run the following in increasing scope. The first two are permanent static
policy checks; the replica test creates and cleans its own disposable project.

```bash
source source_me.sh && python3 -m pytest -q tests/test_replica_compose_topology.py
source source_me.sh && python3 -m pytest -q tests/test_replica_e2e_compose_override.py
node tests/e2e/e2e_replica_restart.mjs
./launch_local_stack.sh --check
```

The replica E2E starts two API replicas behind Caddy, logs in, issues a
question, stops the issuing replica, resumes the same attempt through the
surviving replica, and verifies exact envelope replay plus durable idempotent
submission. It uses a test-only attribution header only in its dedicated build;
normal production builds never emit replica identity headers.

For a complete repository acceptance run, use `./check_codebase.sh`. The
full required release gates and their acceptance ownership remain in
[active_plans/active/release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Planned production topology

WP-RC10 specifies, but does not yet deliver, this production shape:

```text
internet -> CloudFront and WAF -> private ALB -> Fargate API replicas
                                                     |        |         |
                                              encrypted RDS   |   private renderer tasks
                                               PostgreSQL     |
                                                        encrypted S3 buckets

Fargate worker tasks -> shared durable job state in RDS and objects in S3
all tasks            -> scoped Secrets Manager and KMS access in private subnets
```

The planned package adds private networking, least-privilege IAM, encrypted
RDS with PITR, three private encrypted S3 buckets and lifecycle policies, ECR,
Fargate API/worker/renderer tasks, ALB, CloudFront, WAF, KMS, Secrets Manager,
logs, metrics, alarms, autoscaling ceilings, immutable deployment manifests,
restore rehearsal, rollback, drift detection, and tagged teardown. It must
pass OpenTofu policy/format/validation plus a disposable account deployment,
migration, semantic-health, assignment, restore, rollback, drift, and destroy
rehearsal before it can be called deployed.

This version succeeds locally without production infrastructure because the
same stateless API, PostgreSQL session/RLS, object-store, job-lease, health,
and private-renderer contracts are executable and replica-tested. It does not
claim AWS availability, automated failover, managed backup/restore, autoscaling,
CDN, WAF, or multi-region operation before WP-RC10 proves those behaviors.
