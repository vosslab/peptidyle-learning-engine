# Multiple-server setup

This document explains the implemented local multi-replica topology and the
separate OpenTofu production baseline. It is an operations guide, not evidence
that an AWS deployment has been accepted. The source contracts are
[containers/compose.yaml](../containers/compose.yaml),
[containers/Caddyfile](../containers/Caddyfile), and the server composition in
[crates/server/src/composition.rs](../crates/server/src/composition.rs).

## Scope and status

The supported local topology runs one Caddy gateway, one or more stateless API
replicas, one or more durable-worker replicas, one PostgreSQL 17 instance, one
MinIO instance, and one private external stateless PG renderer. The renderer supports the accepted,
deliberately bounded four-source Chapter 1 PGML MC/MATCH profile. It does not imply broad WeBWorK
compatibility or production approval of that externally supplied image.
A local two-API-replica restart test exists and has been used as the behavioral
proof that a learner can continue after the issuing API replica stops.

The target AWS Fargate, ALB, RDS, S3, CloudFront, WAF, KMS, Secrets Manager,
and OpenTofu deployment has a separate deployment acceptance contract. Its
browser-facing HTTPS edge, rather than the API process or this HTTP-only local
gateway, owns HSTS. It is defined in
[active_plans/active/release_completion_plan.md](active_plans/active/release_completion_plan.md),
but a disposable-cloud acceptance run is still distinct from this local
topology evidence.
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

Private PG-renderer path:

API replicas -- renderer_private --> external webwork-pg-renderer
```

Only `gateway` publishes the browser/API port. API replicas have no host port,
so `--scale api=2` never causes a port collision. PostgreSQL and MinIO expose
loopback-only convenience ports for local migrations and inspection; they are
not browser routes. The complete local container policy is in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md).

## Service and network matrix

| Component          | Implemented role                                              | Network or host exposure                                       | Shared durable state                   | Scale rule                                                 |
| ------------------ | ------------------------------------------------------------- | -------------------------------------------------------------- | -------------------------------------- | ---------------------------------------------------------- |
| `gateway`          | Serves read-only `dist/`; proxies `/api`, `/api/*`, `/health` | Host `127.0.0.1:${PLE_GATEWAY_HOST_PORT}`; `gateway_api`       | None                                   | One local gateway; not an API session owner                |
| `api`              | Axum HTTP routes, auth, issue, grade, and asset authorization | `default`, `gateway_api`, and `renderer_private`; no host port | PostgreSQL, S3-compatible object store | `--scale api=N`; each replica uses identical configuration |
| `worker`           | Claims and completes bounded durable jobs                     | `default`; no host port                                        | PostgreSQL queue and object store      | `--scale worker=N`; each process claims one job at a time  |
| `postgres`         | Shared tenant records, sessions, attempts, idempotency, jobs  | `default`; loopback 5432 by default                            | `ple_pgdata`                           | One local instance; no local HA claim                      |
| `minio`            | Shared S3-compatible object store                             | `default`; loopback 9000/9001 by default                       | `ple_miniodata`                        | One local instance; no local HA claim                      |
| `createbuckets`    | Idempotently ensures the four required buckets exist          | `default`; no host port                                        | MinIO buckets                          | One-shot, not scaled                                       |
| `webwork-renderer` | Private standalone PG/PGML render and grade process           | `renderer_private`; no host port                               | None                                   | One normal service; no browser access                      |

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
PLE email-authentication and WebAuthn configuration, grader, and optional
renderer configuration. A different database, different bucket names,
authentication secrets or relying-party settings, or a different renderer
identity is a split-brain deployment, not a scale-out. Optional SMTP delivery
and optional SSO account linking are integrations around the same PLE-owned
account and session model; neither makes institutional identity a required
authority for PLE accounts.

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

| Inputs                                                                           | Owner                                      | All replicas require same value?          | Purpose                                                              |
| -------------------------------------------------------------------------------- | ------------------------------------------ | ----------------------------------------- | -------------------------------------------------------------------- |
| `DATABASE_URL`                                                                   | Compose from PostgreSQL settings           | Yes for API and worker                    | Shared application database                                          |
| `PLE_GRADER_DATABASE_URL`                                                        | Compose/API only                           | Yes for API                               | Restricted answer-bearing reader                                     |
| `PLE_S3_ENDPOINT`, `PLE_S3_REGION`                                               | Compose                                    | Yes for API and worker                    | Shared S3-compatible endpoint                                        |
| `PLE_{PUBLIC_ASSETS,PRIVATE_CONTENT,STUDENT_RECORDS,TEMP_PROCESSING}_BUCKET` | Compose | Yes for API and worker | Fixed policy-separated buckets |
| `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`                                     | Ignored local env; deployment secret store | Yes for API and worker                    | Object-store credentials                                             |
| `PLE_PUBLIC_ASSET_BASE_URL`                                                      | Operator                                   | Yes for API                               | Safe public immutable asset base                                     |
| `PLE_BIND_ADDR`                                                                  | Compose/API                                | Yes except binding address                | API listen address; `0.0.0.0:3000` remains internal                  |
| `PLE_AUTH_PROVIDER`, `PLE_LOCAL_AUTH_FILE`                                       | Local development only                     | Yes for API                               | Development-only tenant-session provider and hash-only identity file |
| `PLE_WEBAUTHN_RP_ID`, `PLE_WEBAUTHN_ORIGIN`, `PLE_WEBAUTHN_RP_NAME`              | Operator                                   | Yes for API                               | PLE passkey relying-party identity                                   |
| `PLE_SMTP_*`, `PLE_PUBLIC_APP_BASE_URL`                                          | Operator, optional SMTP overlay            | Yes for API when delivery is enabled      | External-provider email authentication and invitation delivery       |
| `PLE_WORKER_*`                                                                   | Operator                                   | Yes for workers                           | Bounded worker lease, deadline, and polling controls                 |
| `PLE_GATEWAY_HOST_PORT`                                                          | Local operator                             | Gateway only                              | Loopback browser entry port                                          |
| `PLE_*_IMAGE_SHA256`                                                             | Operator/launcher                          | Applicable services                       | Immutable local image manifests                                      |
| `PLE_WEBWORK_*`                                                                  | Opt-in profile operator                    | Yes for API and renderer where applicable | Private renderer identity, limits, and secrets                       |

The API's `PLE_LOCAL_AUTH_FILE` is a read-only mount of hashes, not the adjacent
local bearer credentials. It is a development-only tenant-session provider and
not PLE's production account path. PLE owns the canonical email-authentication
and account-session flow; WebAuthn passkeys are optional account shortcuts.
The SMTP overlay is optional and connects to an operator-selected external
provider using its credentials. A future SSO integration may link an account,
but PLE does not require an institution as its identity authority.

## Start, scale, inspect, stop

Use the launcher for first startup because it orders configuration bootstrap,
PostgreSQL-major verification, migrations, grader-role provisioning, seed data,
and service readiness. Bare `compose up` against an empty database is not an
equivalent bootstrap path.

```bash
./launch_local_stack.sh --no-open
```

After the normal stack is ready, scale API and worker replicas with the same
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

The renderer stays in the base topology while API and worker replicas scale. The reviewed Chapter 1
matching sources have live single-stack acceptance; broader PG compatibility and multi-replica
matching behavior require separate evidence:

```bash
podman compose -f containers/compose.yaml --env-file containers/env.local ps
podman compose -f containers/compose.yaml --env-file containers/env.local \
  up -d --scale api=2 --scale worker=2
podman compose -f containers/compose.yaml --env-file containers/env.local \
  logs -f api worker webwork-renderer
```

Normal teardown retains volumes:

```bash
podman compose -f containers/compose.yaml --env-file containers/env.local \
  down --remove-orphans
```

The launcher's final output is the authoritative gateway port if it selected a
free port other than the default `8080`. Use that printed port for `curl` and
browser access. See `docs/CONTAINER_PORT_MAPPING.md` for the local and
planned-AWS port boundaries.

## Failure behavior

| Failure                             | Implemented response                                                                  | Operator action                                                                                  |
| ----------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| One API replica stops               | Caddy rediscovers/retries peers; shared session and attempt state permit continuation | Inspect logs, replace the replica, run the replica restart E2E before changing topology          |
| API returns readiness `503`         | Caddy's active `/health` check removes it from rotation                               | Repair PostgreSQL/object-store access or schema compatibility; do not force traffic to it        |
| PostgreSQL unavailable/incompatible | API readiness is `503`; workers refuse schema-incompatible draining                   | Restore database service and expected migration state; do not fabricate migration rows           |
| Object store unavailable            | API readiness is `503`; object operations return a bounded unavailable result         | Restore endpoint, credentials, bucket, or network; preserve object checksum evidence             |
| Worker crashes after claim          | Lease expires; another worker can reclaim according to bounded policy                 | Inspect safe worker logs and queue depth; scale workers only after dependency health is restored |
| Browser retries submission          | Durable idempotency returns the original authorized outcome rather than grading twice | Keep the same request identity and investigate repeated transport failure                        |
| Renderer unavailable                | WeBWorK-backed work fails closed without losing PLE records                           | Recreate the stateless renderer and rerun its semantic probe; keep its port private              |
| Gateway fails                       | Browser origin is unavailable even though API replicas may be healthy                 | Restart/repair gateway; do not publish API ports as an emergency browser bypass                  |

## PG renderer isolation

The API connects only through `renderer_private` to
`http://webwork-renderer:3000/`. The external renderer has no host port, SQL
database, volume, or connection to PLE PostgreSQL, MinIO, gateway, browser, or
worker networks. Browser payloads never carry renderer endpoints, credentials,
source bytes, upstream field names, or keys. Recreating the renderer does not
change any PLE educational record.

## Validation evidence

Run the following in increasing scope. The replica test creates and cleans its
own disposable project; exact Compose-source inspections are one-time review
evidence rather than permanent pytests.

```bash
node tests/e2e/e2e_replica_restart.mjs
./launch_local_stack.sh --check
```

The replica E2E starts two API replicas behind Caddy, logs in, issues a
question, stops the issuing replica, resumes the same attempt through the
surviving replica, and verifies exact envelope replay plus durable idempotent
submission. It uses a test-only attribution header only in its dedicated build;
normal production builds never emit replica identity headers.

Run the three repository-owned offline gates separately:

```bash
./check_codebase.sh
./check_rust.sh
source source_me.sh && python3 -m pytest -q tests/
```

The vendored `check_codebase.sh` owns the browser/TypeScript lane;
`check_rust.sh` owns Cargo and Wasm. Named E2E and live-environment release
gates are additional requirements recorded in
[active_plans/active/release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Production baseline in OpenTofu

`deploy/opentofu/` now defines a pre-production AWS baseline. It is a
configuration and policy-test baseline, not evidence of a live AWS deployment,
successful restore, or real browser traffic. A disposable-account apply and
the probes listed below remain required before production use.

```text
internet -> CloudFront/WAF -> ALB TLS origin -> private API tasks
                                | secret origin header  | HTTPS :3000
                                +-----------------------+
private API/worker/publisher tasks -> RDS PostgreSQL and S3 VPC endpoint
                                  -> distinct Secrets Manager values and KMS keys
CloudFront -> tagged immutable public-assets bucket only
```

Private task subnets have no NAT or public IPs. Their only AWS service paths
are S3 gateway and ECR, Logs, Secrets Manager, KMS, and STS interface
endpoints. Security groups begin with no implicit egress; API-to-iMathAS,
API-to-SMTP, and API-to-renderer egress rules are created only when the
corresponding feature is enabled. The API, worker, and public-asset publisher
are distinct Fargate services with distinct task roles, execution roles,
application-secret values, and database URLs. The publisher is the only task
allowed to promote a verified private source into the immutable public-assets
domain.

The CloudFront viewer path preserves the canonical host to the API. CloudFront
reaches a controlled TLS origin alias and adds a secret origin header; the ALB
denies requests without that header and admits only CloudFront origin-facing
addresses. Edge policy supplies HSTS for browser responses, applies the static
CSP only to static content, and deliberately preserves API CSP. The local HTTP
Caddy stack does not send HSTS.

RDS is private and TLS-authenticated; production login provisioning uses
separate API, worker, publisher, and grader roles rather than an application
superuser. Four versioned, SSE-KMS S3 domains separate public assets, private
content, student records, and temporary processing. CloudFront may read only
tagged immutable public assets. Public promotion is an outbox-backed publisher
operation, never an API write during content authoring.

The WeBWorK renderer is deliberately not deployed by this baseline. Its
integration flag is off by default. Do not enable it until a separately owned
renderer deployment has demonstrated: private API-only ingress, immutable
image provenance, a TLS identity matching its configured origin, no RDS or S3
authority, and a bounded fail-closed request contract.

Before calling this production-ready, run OpenTofu format, validate, and policy
tests, then in a disposable account apply the stack, provision the exact RDS
roles and secret values, verify CloudFront/ALB origin admission and headers,
exercise each enabled integration, migrate and check semantic health, test
publication, restore, rollback, drift detection, and destroy. Those are
live-only probes; this repository does not claim they have happened.
