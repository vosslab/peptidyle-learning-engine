# Local stack operations

This guide operates the local Podman stack for the Peptidyle Learning Engine.
The canonical developer/browser entry point is [launchers/run_live_demo.sh](../launchers/run_live_demo.sh);
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
workspace ID, Question ID, Object Address, queue payload, or client-supplied integration field
is only a lookup/input value; it cannot establish authority.

| Data                          | Exact owner                                                            | Local enforcement                         |
| ----------------------------- | ---------------------------------------------------------------------- | ----------------------------------------- |
| Account, session, and passkey | Global `AccountId`                                                     | Server session and PostgreSQL             |
| Published question            | Stable `QuestionId` lineage plus immutable `QuestionRevisionReference` | Approved-Instructor Question Library      |
| Draft authoring               | `WorkspaceId` plus owner/collaborators                                 | Workspace relationship                    |
| Reusable curriculum           | Blueprint Course Reference plus exact Blueprint Revision                | Blueprint Course Owner lifecycle or Active Instructor read access |
| Course and assignment         | `CourseId` and child records                                           | Current Instructor Course Membership      |
| Student work and grades       | Exact course plus Student owner                                        | Student self or current course Instructor |
| Jobs and objects              | Typed target from the locked lease                                     | Store/PostgreSQL capability boundary      |

Current Teaching Team Members are equal. Course creation inserts the creator's first
ordinary Instructor membership and does not create an elevated owner. Students
see only their own work in enrolled courses. Published questions remain in one
shared Instructor Question Library after publication. Published Question
lineage availability is `Available` or `Archived`; only an `Available` Published
Question is eligible for ordinary new selection. Exact archived Question
Revisions remain resolvable. Draft Question Source Bindings and Answer Keys remain private.

Institution names, roster IDs, display labels, provider IDs, renderer IDs, and
similar fields are metadata for presentation, audit, provenance, or routing.
They are never an Account, role, membership, course, Student, workspace,
Question Library, or lease authority. The PLE account/session and exact relationship are
always authoritative.

## Services

| Service                | Purpose                                                                | Local exposure                     |
| ---------------------- | ---------------------------------------------------------------------- | ---------------------------------- |
| `gateway`              | Serves read-only `dist/` and forwards same-origin `/api` and `/health` | One loopback host port             |
| `api`                  | Auth, course operations, attempts, grading, and delivery               | Private network; no host port      |
| `postgres`             | Relational authority, queue, RLS, audit, and records                   | Loopback `5432`                    |
| `minio`                | S3-compatible object storage                                           | Loopback `9000` and console `9001` |
| `createbuckets`        | Idempotently creates four storage buckets                              | One-shot, no host port             |
| `identity-secret-init` | Copies two host capabilities into an API-only volume                   | Networkless, one-shot              |
| `database-migrator`   | Applies schema and checks the API database login before startup         | Profile-only, no host port         |
| `webwork-renderer`     | Private stateless PG/PGML render and grade engine                      | No host port                       |

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
networkless `local-data-volume-permissions` helper runs with only `CAP_CHOWN`,
fixes retained volume ownership, and exits before daemons start. It does not
change database or object content. The Live Demo accepts the active local Podman
connection whether it is rootless or rootful.

Compose execution is capability-based rather than installation-path-based. The
controller prefers `podman compose`, then the selected Python
`podman_compose` module, then a standalone `podman-compose` executable. A
disposable owner keeps the selected adapter for its lifecycle and still passes
its explicit no-pod option. It does not retry a mutating
Compose operation through another adapter after that operation begins.

The controller runs its short-lived base-schema/forward-migration coordinator
and application-schema verifier inside the Compose data network. The local
PostgreSQL loopback port remains an operator diagnostic endpoint; lifecycle
correctness does not depend on its host port-forwarding behavior.

`ple_pgdata` and `ple_miniodata` are named volumes. A normal container stop or
rebuild retains them. The read-only `postgres-major-guard` accepts an empty
volume or PostgreSQL 17 data; it never performs a major upgrade. Upgrade by
verified backup, new target-major volume, restore, validation, and recovery
acceptance. Removing a populated volume is destructive.

The local hardening limits accidental exposure and confused operations. A
person controlling the host account or Podman socket can still inspect disposable
data. Production uses managed RDS, S3, IAM, and KMS controls.

## First run

Use the launcher from the repository root:

```bash
./launchers/run_live_demo.sh
./launchers/run_live_demo.sh open
./launchers/run_live_demo.sh start --open
```

The launcher resolves the checkout from its own filesystem location, sources
[source_me.sh](../source_me.sh), runs
[devel/setup_typescript.sh](../devel/setup_typescript.sh), and then runs
`python3 local_stack.py start`. Start always builds the production `dist/`
bundle, creates a fresh fixed target, waits for the HTTPS gateway, and prints its URL for an operator
to open. `open` opens the exact URL of an already-running fixed target without restarting it;
`--open` is its compatible shorthand. `start --open` creates a fresh target and opens it. `--headless`
remains an accepted explicit spelling of the default non-opening behavior.

The fixed target is always:

```text
owner:   live-demo-browser
project: ple-live-demo-browser
profile: browser
```

The owner holds one lease through build, private capability generation,
PostgreSQL bootstrap, canonical database initialization, installation data,
Question Renderer Version, readiness, and cleanup. It accepts no project,
environment, identity, SMTP, or skipped-build selector. Bare `podman compose
up` against an empty database is not an equivalent bootstrap path.

If you request `./launchers/run_live_demo.sh stop` while that owner is still starting, the
command waits for its authenticated stop endpoint rather than attempting a second cleanup. It then
stops the same fixed owner normally.

## Demo accounts and baseline

The production-auth overlay seeds five ordinary PLE personas: Elena (Instructor),
Mary, Jack, and Avery (Students), and Morgan (Sysadmin). Seeded entry replaces
only the normal identity-verification ceremony with a known persona key. The
server resolves the global Account and issues an ordinary session; it does not
accept a browser role claim.

The default local installation runs `cargo tools installation-data provision`
after the API and supporting services are ready. It creates the database-owned
Pilot publication, ordinary Accounts, Blueprint Draft and Revision, Course,
roster, and released Assignment, then uses ordinary authenticated routes for
the cross-system Student Work and grading effects. Mary and Jack receive real
Assignment Attempts; Avery remains enrolled without one. The database-owned
manifest is convergent; the full provision command is the canonical owner for
the complete known-good Live Demo.

`--without-live-demo` skips this optional product data before provisioning. It
does not select a different schema, role model, or lifecycle.

## Startup order

The lifecycle validates image digests, private files, Compose topology, and
Question Renderer Version before mutating the selected target. It then:

1. removes the prior fixed project while retaining no stale owner resources;
2. runs `postgres-major-guard`, starts PostgreSQL, and waits for readiness;
3. initializes the modular base on an empty database (or applies recognized
   forward migrations), verifies the result, and sets up bounded runtime
   service logins;
4. starts MinIO and idempotently creates its declared buckets;
5. starts, probes, and attests the private renderer;
6. starts API initializers, builds API and gateway images, and starts API and
   gateway;
7. waits for API semantic health; and
8. runs complete Live Demo provisioning unless `--without-live-demo` was
   selected.

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
configuration, Answer Keys, Question Grading Input, Question Source bytes, and
upstream identifiers remain server-side. Provider fields in records are metadata/provenance and do not
authorize a course, Student, or object.

## Health and inspection

`/health` is readiness, not liveness. In the disposable Live Demo topology, it
returns 200 only after the API can acquire its database pool, head the declared
public-assets bucket, connect to the private renderer, and connect to the
worker's private non-HTTP readiness socket. A failed probe returns 503 with
only `database`, `object-store`, `renderer`, or `worker`; credentials,
connection strings, and provider details stay server-side. The gateway forwards
that safe response and maps a missing API process to the same bounded 503 form
with only `api`. Migration verification remains a lifecycle startup gate, not a
claim made by this runtime route.

Inspect through the controller:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs api gateway
source source_me.sh && python3 local_stack.py validate
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
./launchers/run_live_demo.sh stop
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
source source_me.sh && python3 local_stack.py reset \
  --confirm-project containers
```

Use it only when removal of `ple_pgdata`, `ple_miniodata`, and installation
manifests is intended. Normal fixed live-demo stop and normal Compose stop do
not imply this reset.

## Validation commands

The connected acceptance owner is:

```bash
source source_me.sh && python3 local_stack.py acceptance
```

It refuses an existing default or fixed live-demo stack, runs exactly two
browser-free real-service lanes, and cleans each disposable owner exactly: the
disposable PostgreSQL schema, authority, and persistence oracle; and the Course
Appearance PostgreSQL and MinIO coherence oracle. The future two-API profile
returns with the fresh implemented course-delivery Store contract. Renderer,
database, object-store, and worker checks are service evidence, not substitutes for browser
journeys. The separate serial production-browser owner, `./devel/run_playwright_tests.sh --build`,
accepted M19 on 2026-09-07; it remains outside this two-lane service command. See
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for evidence classes and gate boundaries.

## Production boundary

The local lifecycle does not deploy AWS resources. The planned production
baseline in [deploy/opentofu](../deploy/opentofu) uses private Fargate API,
worker, and publisher services, RDS PostgreSQL, versioned SSE-KMS S3 domains,
CloudFront/WAF/ALB, VPC endpoints, and role-separated Secrets Manager values.
The external renderer is disabled there by default. OpenTofu validation,
disposable apply, migration/health, restore, rollback, drift, and bounded
destroy remain deployment gates. A successful local stack or browser journey
does not establish production readiness or release acceptance.
