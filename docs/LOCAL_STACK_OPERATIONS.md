# Local stack operations

This guide operates the local Podman stack for the Peptidyle Learning Engine.
The canonical developer/browser entry point is [run_live_demo.sh](../run_live_demo.sh);
it delegates to `local_stack.py` and the private `local_stack_control` owner.
The topology is defined by [containers/compose.yaml](../containers/compose.yaml)
and the fixed production-auth overlay at
[tests/e2e/compose.live-demo-browser.yaml](../tests/e2e/compose.live-demo-browser.yaml).
Replica behavior and the planned AWS shape are in
[MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md).

The local stack is a disposable development environment. It is not a
production security boundary, a highly available database, or deployment
acceptance evidence.

The **Local Stack Controller** is the tracked `local_stack_control/` package.
It creates and owns **Local Stack State** under the ignored repository-root
`local_stack_state/` directory. The browser suite keeps its controller-owned
lease, private control receipts, and resettable workspace below that state
directory. Controller code is reviewable repository content; Local Stack State
is disposable host state and is never an application contract or source of
authority.

## Authority model

PLE is one installation with global accounts. The authenticated server session
derives `AuthenticatedSession { account_id, session_id }`. It then authorizes the exact
course selected by the user from current membership rows. A route course ID,
workspace ID, catalog ID, object key, queue payload, or external-provider field
is only a lookup/input value; it cannot establish authority.

| Data | Exact owner | Local enforcement |
| --- | --- | --- |
| Account, session, and passkey | Global `AccountId` | Server session and PostgreSQL |
| Published question | Global immutable `QuestionId`/`QuestionVersionNumber` | Approved-Instructor catalog |
| Draft or curriculum | `WorkspaceId` plus owner/collaborators | Workspace relationship |
| Course and assignment | `CourseId` and child records | Current direct Instructor membership |
| Student work and grades | Exact course plus Student owner | Student self or current course Instructor |
| Jobs and objects | Typed target from the locked lease | Store/PostgreSQL capability boundary |

Current Teaching Team Members are equal. Course creation inserts the creator's first
ordinary Instructor membership and does not create an elevated owner. Students
see only their own work in enrolled courses. Published questions remain in one
shared Instructor catalog across `active`, `deprecated`, and `archived`
lifecycle states; only `active` questions are eligible for ordinary new
selection. Draft source and answer-bearing material remain private.

Institution names, roster IDs, display labels, provider IDs, renderer IDs, and
similar fields are metadata for presentation, audit, provenance, or routing.
They are never an Account, role, membership, course, Student, workspace,
catalog, or lease authority. The PLE account/session and exact relationship are
always authoritative.

## Services

| Service | Purpose | Local exposure |
| --- | --- | --- |
| `gateway` | Serves read-only `dist/` and forwards same-origin `/api` and `/health` | One loopback host port |
| `api` | Auth, course operations, attempts, grading, and delivery | Private network; no host port |
| `postgres` | Relational authority, queue, RLS, audit, and records | Loopback `5432` |
| `minio` | S3-compatible object storage | Loopback `9000` and console `9001` |
| `createbuckets` | Idempotently creates four storage buckets | One-shot, no host port |
| `identity-secret-init` | Copies two host capabilities into an API-only volume | Networkless, one-shot |
| `webwork-renderer` | Private stateless PG/PGML render and grade engine | No host port |

All published ports bind to `127.0.0.1`. The API is the sole PLE application
process in the supported local topology. The renderer has no SQL database,
course, roster, volume, or browser path; PLE remains the educational-record
authority.

The four buckets are `public-assets`, `private-content`, `student-records`,
and `temp-processing`. Their distinct policies are part of the contract:
public presentation assets are immutable and versioned, private content is
authorized-only, Student records have explicit expiry/deletion handling, and
temporary processing is never served.

## Containment

PostgreSQL, MinIO, API, gateway, and renderer use read-only container roots,
dropped capabilities, `no-new-privileges`, bounded resources, and non-executable
temporary filesystems. PostgreSQL runs as UID 999 and MinIO as UID 10001. The
networkless `local-data-volume-permissions` helper runs as root inside the
rootless Podman user namespace with only `CAP_CHOWN`, fixes retained volume
ownership, and exits before daemons start. It does not change database or object
content.

`ple_pgdata` and `ple_miniodata` are named volumes. A normal container stop or
rebuild retains them. The read-only `postgres-major-guard` accepts an empty
volume or PostgreSQL 17 data; it never performs a major upgrade. Upgrade by
verified backup, new target-major volume, restore, validation, and recovery
acceptance. Removing a populated volume is destructive.

The local hardening limits accidental exposure and confused operations. A
person controlling the host account or rootless Podman socket can still inspect
disposable data. Production uses managed RDS, S3, IAM, and KMS controls.

## First run

Use the root script from the repository root:

```bash
./run_live_demo.sh
./run_live_demo.sh --headless
```

The script sources [source_me.sh](../source_me.sh), runs
[devel/setup_python.sh](../devel/setup_python.sh) for the repo-local Python
3.12 environment, installs Node dependencies with
[devel/setup_typescript.sh](../devel/setup_typescript.sh) when `node_modules`
is absent, and runs `.venv/bin/python local_stack.py start`. Start always
builds the production `dist/` bundle, creates a fresh fixed target, and waits
for the HTTPS gateway. Without `--headless` it opens the URL; with it, it
prints the URL for an operator to open.

The fixed target is always:

```text
owner:   live-demo-browser
project: ple-live-demo-browser
profile: browser
```

The owner holds one lease through build, private capability generation,
PostgreSQL bootstrap, migration, seed, renderer provenance, readiness, and
cleanup. It accepts no project, environment, identity, SMTP, or skipped-build
selector. Bare `podman compose up` against an empty database is not an
equivalent bootstrap path.

## Demo accounts and courses

The production-auth overlay seeds five ordinary PLE personas: Elena (Instructor),
Mary and Jack (Students), Avery (Student approval candidate), and Morgan
(Sysadmin). The public selector chooses only a known seeded persona key. The
server resolves the global account and issues an ordinary session; it does not
accept a browser role claim.

The seeded course memberships are exact:

- `Biochemistry: Protein Structure and Function`: Elena is the Instructor;
  Mary and Jack are Students.
- `Genetics Practice Course`: Morgan is the Instructor member and Avery is the
  Student member.

The visible course list is therefore membership-derived. Choosing a course
does not grant access to another course, another Student, or another
workspace. New browser scenarios may create additional course records through
the ordinary Instructor workflow; those records remain scoped to their exact
`CourseId`.

## Startup order

The lifecycle validates image digests, private files, Compose topology, and
renderer provenance before mutating the selected target. It then:

1. removes the prior fixed project while retaining no stale owner resources;
2. runs `postgres-major-guard`, starts PostgreSQL, and waits for readiness;
3. applies the migration set and provisions bounded runtime logins;
4. starts MinIO and idempotently creates its declared buckets;
5. starts, probes, and attests the private renderer;
6. starts API initializers, builds API and gateway images, and starts API and
   gateway; and
7. waits for API semantic health before reporting the HTTPS origin.

The API receives its one bounded runtime database URL. Migration children receive
administrator authority only for their bounded startup calls. Raw passwords and
Question ID capabilities remain in mode-0600 private files or runtime volumes;
the browser never receives them.

## Renderer boundary

`PLE_WEBWORK_RENDERER_IMAGE` names the external standalone
`webwork-pg-renderer` image. PLE records its selected OCI configuration ID and
probes the private `/render-api` contract. The renderer has no host port and
joins only `renderer_private`; it cannot reach PostgreSQL, MinIO, gateway, or
the browser. Its failure closes PG-backed work without losing PLE records.

This is a bounded provider integration, not broad WeBWorK compatibility. The
local stack does not run WebWork2 or MariaDB. Provider credentials, renderer
configuration, answer material, source bytes, and upstream identifiers remain
server-side. Provider fields in records are metadata/provenance and do not
authorize a course, Student, or object.

## Health and inspection

`/health` is readiness, not liveness. It returns 200 only when the API verifies
expected migration versions/checksums and successfully probes the content
bucket. A failing dependency returns 503 with safe names. The gateway polls
this route; it does not evict a replica for every feature-local application
failure.

Inspect through the controller:

```bash
source source_me.sh && .venv/bin/python local_stack.py doctor
source source_me.sh && .venv/bin/python local_stack.py status
source source_me.sh && .venv/bin/python local_stack.py logs api gateway
source source_me.sh && .venv/bin/python local_stack.py validate
```

Raw Compose inspection is diagnostic only and must use the exact env file:

```bash
podman compose -f containers/compose.yaml \
  --env-file containers/env.local ps
```

Do not direct browser traffic to an API container or publish the private
renderer. The fixed HTTPS origin printed by the lifecycle is the only browser
entry point.

## Stop and cleanup

Stop the canonical live-demo session with:

```bash
./run_live_demo.sh stop
```

The stop request authenticates to the private owner control socket. The owner
then runs the exact fixed Compose cleanup (`down --volumes --remove-orphans`),
rechecks all project-labelled containers, volumes, and networks, removes its
private workspace artifacts, and only then removes project-derived image tags.
It never runs a global Podman prune or accepts a caller-selected project. A
cleanup failure retains private recovery evidence and returns nonzero.

If the control socket is unavailable, the next invocation first reacquires the
same exclusive lease, proves no owner is live, and purges only the fixed
`ple-live-demo-browser` resources. It does not infer ownership from a process
name or remove an unrelated project.

The default `containers` project has a separate explicit destructive reset for
retained local data:

```bash
source source_me.sh && .venv/bin/python local_stack.py reset \
  --confirm-project containers
```

Use it only when removal of `ple_pgdata`, `ple_miniodata`, and installation
manifests is intended. Normal fixed live-demo stop and normal Compose stop do
not imply this reset.

## Validation commands

The connected acceptance owner is:

```bash
source source_me.sh && .venv/bin/python local_stack.py acceptance
```

It refuses an existing default or fixed live-demo stack, runs the canonical
real-stack browser lane and current database/object service oracles, and cleans
each disposable owner exactly. The future two-API profile returns with the
fresh mounted course-delivery Store contract. Renderer, database, object-store,
and worker checks are service evidence, not substitute browser journeys.
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) defines the required evidence
classes and final gate order.

## Production boundary

The local lifecycle does not deploy AWS resources. The planned production
baseline in [deploy/opentofu](../deploy/opentofu) uses private Fargate API,
worker, and publisher services, RDS PostgreSQL, versioned SSE-KMS S3 domains,
CloudFront/WAF/ALB, VPC endpoints, and role-separated Secrets Manager values.
The external renderer is disabled there by default. OpenTofu validation,
disposable apply, migration/health, restore, rollback, drift, and bounded
destroy remain deployment gates. A successful local stack or browser journey
does not establish production readiness or release acceptance.
