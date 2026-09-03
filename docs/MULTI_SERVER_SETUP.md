# Multiple-server setup

This document records the supported local replica topology and the separate
OpenTofu production baseline. It is an operations contract, not evidence that
an AWS deployment has been accepted. The local source of truth is
[containers/compose.yaml](../containers/compose.yaml), with
[tests/e2e/compose.live-demo-browser.yaml](../tests/e2e/compose.live-demo-browser.yaml)
for the fixed browser profile. The lifecycle and recovery commands are in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md).

## Single installation

PLE is one installation with global accounts. An authenticated session resolves
to `AuthenticatedSession { account_id, session_id }`; a browser route, header, queue row,
Object Address, or provider response cannot supply a different Account or scope.
Course selection is the set of exact current memberships returned for that
session. Selecting a course supplies a route reference for the server to check,
not a new authorization claim.

| Record or capability                                            | Exact owner or scope                                        | Authorization                                                   |
| --------------------------------------------------------------- | ----------------------------------------------------------- | --------------------------------------------------------------- |
| Account, email, passkey, session                                | Global `AccountId` and Authenticated Session                | Account/session contract                                        |
| Published question and presentation asset                       | Global immutable `QuestionId` and `QuestionRevisionNumber`  | Every active Instructor                                         |
| Draft Question or private curriculum workspace                  | `WorkspaceId` and Authoring Workspace relationship          | Authoring Workspace Owner or Workspace Collaborator             |
| Draft Blueprint Revision                                        | Exact Blueprint Course and revision                         | Blueprint Course Owner or Blueprint Collaborator                |
| Course, roster, assignment, schedule                            | Exact `CourseId` and child identity                         | Current direct Instructor membership                            |
| Assignment Attempt, Question Attempt, response, grade, artifact | Exact `CourseId` plus Student owner                         | Student self or current course Instructor                       |
| Question Folder, Star, Watch, or Saved Question Search          | Account-owned reference to a Published Question             | Exact Account relationship; Question Folder Shares are explicit |
| Job, export, object, or provider state                          | Typed course, workspace, Question Library, or system target | Locked lease and durable target                                 |

Every current course Instructor, including a Teaching Team Member, has the same
teaching and FERPA-read authority. Course creation creates the first ordinary
Instructor membership; it does not create a privileged owner row. A Student
can read only that Student's records in an enrolled course. A private workspace
is not a course or Question Library. The Question Library exposes only reviewed,
answer-free Question Search Results: every Published Question is discoverable to an
active Instructor, while only `Available` Question Revisions are ordinarily selectable.

Institution names, roster identifiers, display labels, provider identifiers,
renderer IDs, and similar fields are metadata for display, audit, provenance,
or routing. They are never an Account, role, course, Student, workspace,
Question Library, or lease authority. PLE-owned account and session state remains the
authority even when an optional OIDC or SAML identity integration is enabled.

## Scope and status

The supported local topology has one Caddy gateway, one stateless API, one
worker, one PostgreSQL 17 instance, one MinIO instance, and one private
stateless PG renderer. The normal developer/browser owner uses production
authentication and the seeded live-demo workflow. It does not select a caller
project, alternate local credential file, or alternate identity source.

The `replica_restart` profile is the only supported two-API topology. It uses
one gateway, two API replicas, one shared PostgreSQL instance, one shared MinIO
instance, and the same worker contract. It proves durable replay after the API
that issued a question stops. There is no generic random-replica command.

The planned AWS topology is a separate baseline: CloudFront/WAF, an ALB TLS
origin, private Fargate API/worker/publisher services, private multi-AZ RDS
PostgreSQL, four versioned S3 domains, Secrets Manager, KMS, and VPC endpoints.
The OpenTofu files describe this shape, but a disposable-cloud apply, restore,
rollback, and connected browser run are still required before production use.
Local PostgreSQL and MinIO are not highly available and are not production
deployment evidence.

## Local topology

```text
browser -- HTTPS/loopback --> Caddy gateway
                                  |
                         gateway_api network
                         dynamic API discovery
                           /              \\
                      API replica 1    API replica 2
                           |              |
                           +------+-------+
                                  |
                    shared default data network
             +------------+-------+--------+------------+
             |            |                |            |
         PostgreSQL     MinIO           worker       setup jobs
          queue/RLS   object store     lease N        one-shot

API replicas -- renderer_private --> private PG renderer
```

Only the gateway publishes the browser/API port. API replicas have no host
port, so the fixed two-replica profile does not collide on a port. PostgreSQL
and MinIO publish loopback-only convenience ports for operator inspection.
The worker has no HTTP endpoint. The renderer has no host port and is reachable
only from the API and worker private network as configured by Compose.

## Service matrix

| Component          | State and role                                         | Exposure                            | Scale rule                                  |
| ------------------ | ------------------------------------------------------ | ----------------------------------- | ------------------------------------------- |
| `gateway`          | Read-only `dist/`; same-origin API and health proxy    | One loopback host port              | One gateway                                 |
| `api`              | Axum routes, sessions, authorization, attempts, assets | Private `gateway_api`; no host port | One normally; two only in `replica_restart` |
| `worker`           | PostgreSQL queue claim, prepare, and commit            | Default data network; no port       | Add workers for concurrency                 |
| `postgres`         | Accounts, courses, Student records, jobs, audit, RLS   | Loopback `5432` by default          | One local instance                          |
| `minio`            | Four policy-separated S3-compatible buckets            | Loopback `9000` and `9001`          | One local instance                          |
| `createbuckets`    | Idempotent bucket bootstrap                            | No host port                        | One-shot                                    |
| `webwork-renderer` | Bounded PG/PGML render and grade engine                | `renderer_private`; no host port    | One local stateless service                 |

Named volumes `ple_pgdata` and `ple_miniodata` hold local state across a normal
container stop. The PostgreSQL-major guard accepts an empty volume or a
PostgreSQL 17 directory. A major upgrade requires backup, a new volume,
restore, application validation, and retention of the old volume until recovery
is accepted; deleting a populated volume is not an upgrade procedure.

## Gateway and health

Caddy resolves the `api` service through dynamic Compose DNS and round-robins
current API addresses. Its active check calls the API's semantic `/health`
route. The API returns `200 {"status":"ready"}` only after migration state and
checksums match the binary and a real object-store bucket probe succeeds. A
dependency failure returns `503` with safe failing-dependency names. A feature
local failure, such as the private renderer, does not by itself evict a healthy
API replica.

The worker deliberately has no HTTP readiness endpoint. It verifies schema
compatibility at startup, is supervised by the lifecycle, and reports a
bounded capability-bearing readiness receipt through the owner. Do not direct
browser traffic to the worker.

## Stateless API state

API replicas share all correctness state:

- Session tokens are opaque, HttpOnly values. PostgreSQL stores their hashes,
  expiry, and revocation, so a session and its revocation work on every replica.
- Attempts, timing, immutable question references, submissions, idempotency,
  grades, and audit evidence are PostgreSQL records.
- Object identity, checksum, bucket policy, and signed delivery remain
  server-owned in the shared object store and database metadata.
- Grading and provider credentials stay server-side. The browser receives no
  answer key, private payload, or provider secret.

A replica with a different database, bucket set, authentication secret,
WebAuthn relying-party identity, or renderer identity is a split-brain
deployment, not scale-out. Provider and institutional fields remain metadata;
they do not replace PLE account/session or course relationships.

## Typed worker recovery

The worker claims one PostgreSQL job row under a fresh opaque `JobLeaseToken`.
The locked lease and immutable job manifest determine the typed target:
course, workspace, Question Library, object, export, retention, or system. Before any
read, write, provider dispatch, or finalization, all of these must agree:

- Job Kind Registration and declared typed Job Payload;
- current lease and lease token;
- immutable target type and exact target identity;
- exact Job claim-and-lease authority; and
- stale-work generation fence.

Queue payloads, retry input, object references, provider responses, and caller
input are evidence, not authority. A stale token, expired lease, foreign-course
object, foreign job target, stale generation, mismatched Job Kind Registration, or forged
provider completion is rejected before the protected effect. Preparation writes
only replay-safe private output; the committer makes the effect visible and
completes the same claim atomically.

If a worker crashes or loses a dependency, the lease expires and a later worker
may reclaim the job under the bounded retry/backoff policy. A completion after
expiry returns a claim-no-longer-active result and cannot publish a second
effect. External calls with an indeterminate effect remain closed for automatic
relaunch until the owning recovery policy resolves them. Local defaults are
`PLE_WORKER_LEASE_SECONDS=120`,
`PLE_WORKER_PREPARATION_TIMEOUT_SECONDS=90`, and
`PLE_WORKER_POLL_MILLIS=500`; these are bounded controls, not authority.

## Configuration equality

Every API replica uses the same database, object-store endpoint and bucket
names, PLE account/session settings, WebAuthn relying-party settings, and
renderer identity. Worker replicas share the PostgreSQL queue and object store
but use the worker-only database capability. Deployment tasks use separate
API, worker, recovery, fast-path, and publisher secret sets where the owning
OpenTofu task requires them; secret values do not belong in source, browser
state, or OpenTofu variables.

The four local buckets are `public-assets`, `private-content`,
`student-records`, and `temp-processing`. Public assets are immutable and
versioned. Private content is authorized-only. Student records have explicit
expiry/deletion policy. Temporary processing is never served.

## Operate and inspect

Use the fixed lifecycle for the developer/browser stack:

```bash
./run_live_demo.sh
./run_live_demo.sh --headless
./run_live_demo.sh stop
```

The script sources `source_me.sh` through its fixed script-directory path,
installs TypeScript dependencies through `devel/setup_typescript.sh` when needed,
and delegates to `python3 local_stack.py`. Its only mutable browser-session choices
are start and stop, with optional `--headless` on start. It does not accept a
project, environment, identity, SMTP, or skipped-build selector.

Use the controller's read-only commands for the default `containers` project;
raw Compose is for diagnosis after the owner has selected the exact target:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs api worker
source source_me.sh && python3 local_stack.py validate
```

The normal Developer Browser Suite is stopped through its fixed lifecycle, not
a global Podman command. Its fixed owner authenticates the stop over a private
control socket and then proves cleanup. See
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for the default-stack
reset contract and PostgreSQL/MinIO inspection details.

## Future replica evidence

The `replica_restart` profile remains a typed disposable-stack configuration
for the future mounted course-delivery service. Its browser-free oracle returns
when the fresh Store and route contracts can issue and replay Student work.
That successor will start two API replicas against one PostgreSQL and one
MinIO, prove durable replay and idempotent submission through the peer, and
keep observability headers within its dedicated test image.

## Production baseline

`deploy/opentofu/` defines the pre-production AWS shape:

```text
CloudFront/WAF -> ALB TLS origin -> private Fargate API tasks
                                      |
                         private RDS PostgreSQL 17
                         private S3 object domains

private Fargate worker tasks ------> PostgreSQL queue and S3
private Fargate publisher task ----> immutable public-assets promotion
```

The baseline starts with two API tasks, one worker, and one publisher, with
bounded autoscaling for API and worker. RDS is private, TLS-authenticated,
encrypted, multi-AZ, and backup-retained. S3 uses four versioned SSE-KMS
domains; CloudFront reads only tagged immutable public assets. API, worker,
publisher, and their execution roles receive distinct Secrets Manager values
and KMS permissions. No external queue service is declared: the durable queue
remains PostgreSQL-backed.

The external PG renderer is disabled in this baseline. Enable it only after a
separate private-ingress, image-provenance, TLS-identity, no-database-authority,
and fail-closed acceptance. The local Caddy gateway does not supply production
HSTS; the deployed HTTPS edge owns that policy.

OpenTofu formatting, validation, policy checks, disposable apply, migration and
health checks, restore, rollback, drift, and bounded destroy remain required
deployment evidence. This document does not claim those runs occurred.

## Failure behavior

| Failure                     | Closed response                                               | Recovery                                               |
| --------------------------- | ------------------------------------------------------------- | ------------------------------------------------------ |
| API replica stops           | Gateway retries a healthy peer; shared records preserve state | Replace the replica and run the replica oracle         |
| API readiness is `503`      | Gateway removes that replica from rotation                    | Repair database, object store, or schema compatibility |
| PostgreSQL is unavailable   | API is not ready; workers do not drain                        | Restore the database and verify migrations             |
| Object store is unavailable | Object delivery fails closed; relational records remain       | Restore endpoint, bucket, credentials, or network      |
| Worker crashes after claim  | Lease expires; bounded reclaim is possible                    | Inspect redacted worker evidence and queue depth       |
| Renderer fails              | PG-backed work fails closed; PLE records remain               | Recreate and re-attest the renderer                    |
| Gateway fails               | Browser origin is unavailable; API records are unchanged      | Repair or recreate the gateway                         |

## Evidence boundary

`source source_me.sh && python3 local_stack.py acceptance` owns the
canonical connected browser invocation and its disposable stack. The named
renderer and replica commands are browser-free service oracles. The complete
Validation order and the distinction between permanent tests, one-time
Graphify evidence, disposable acceptance, and deployment evidence are owned by
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md). A local pass does not prove
AWS availability, provider compatibility, or production release acceptance.
