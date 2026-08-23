# Podman on macOS

macOS cannot run Linux containers directly, so Podman runs them inside a Linux
virtual machine it manages for you. Everything in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) assumes that machine is running.

## Install

```bash
brew install podman
podman machine init
podman machine start
```

`podman machine init` is a one-time step. `podman machine start` is needed
after a reboot.

## Check the machine

```bash
podman machine list
podman info --format '{{.Host.Arch}}'
```

A machine that is not `Currently running` is the cause of most
"cannot connect to Podman" errors.

## Resource sizing

The default machine is small for a Rust build. The first container build
compiles the whole dependency tree, and a cramped machine turns that into a
long wait or an out-of-memory failure.

```bash
podman machine stop
podman machine set --cpus 8 --memory 16384 --disk-size 60
podman machine start
```

## Architecture notes

Apple Silicon runs `arm64` Linux images natively. The selected PostgreSQL,
MinIO, MinIO Client, Alpine secret-initializer, and gateway base-image digests
are multi-architecture manifests in
[containers/env.example](../containers/env.example). Podman selects their
`linux/arm64` variant on Apple Silicon, so the normal local stack does not need
emulation. Keep the manifest digests unchanged unless the selected replacement
has an `arm64` variant too.

If you need to reproduce an `amd64` deployment locally, pass the platform
explicitly and expect it to run slowly under emulation:

```bash
podman build --platform linux/amd64 -f containers/Containerfile.api -t peptidyle-api .
```

## Ports and localhost

Podman forwards published ports from the virtual machine to macOS `localhost`,
so `curl http://localhost:8080/health` works from the host exactly as it would
on Linux. Inside the compose network, services address each other by service
name (`postgres`, `minio`), not `localhost`. See
`docs/CONTAINER_PORT_MAPPING.md` for port selection.

## Registry prefixes

Podman does not assume Docker Hub. Image references in
[containers/compose.yaml](../containers/compose.yaml) are fully qualified and
digest-pinned. The tracked contract takes this form:

```text
docker.io/library/postgres@sha256:${PLE_POSTGRES_IMAGE_SHA256}
quay.io/minio/minio@sha256:${PLE_MINIO_IMAGE_SHA256}
quay.io/minio/mc@sha256:${PLE_MINIO_MC_IMAGE_SHA256}
docker.io/library/alpine@sha256:${PLE_SECRET_INIT_IMAGE_SHA256}
```

`containers/env.example` supplies the selected 64-character digest values.
Copy it to ignored `containers/env.local` and retain the pins; do not replace
them with a tag such as `latest`. The gateway takes a fully qualified
digest-pinned Caddy value. The external renderer image is owned and built by the
separate `webwork-pg-renderer` project; PLE records the resolved image
configuration ID together with the selected image name.

These references are not all runtime images. `postgres`, `minio`,
`createbuckets`, and `identity-secret-init` run their specified external
images. `api` builds the shared local application image from
`containers/Containerfile.api`, and `worker` consumes that exact image;
`gateway` is built locally from `containers/Containerfile.gateway` using the
pinned Caddy build argument. The `webwork-renderer` is an external-project image. Build-mode PLE
startup reuses the tracked `localhost/pg-renderer:reviewed` selection or rebuilds
it from the maintained sibling checkout when pruning removed it. PLE resolves that selected name to its OCI
configuration ID, confirms the container runs that ID, and records both as
renderer-version provenance. A published deployment can select a pullable
`repository@sha256:<64-lowercase-hex>` value through the same configuration
key.

## PostgreSQL retained volumes

The local stack is pinned to PostgreSQL 17. Before it starts PostgreSQL,
The private typed lifecycle runs the maintenance-profile `postgres-major-guard`.
The guard mounts `ple_pgdata` read-only and checks its `PG_VERSION` file. A
populated volume declaring a major other than `17` is refused before the
database service starts.

This guard never migrates, rewrites, or deletes the volume. If it refuses an
older or newer volume, stop and choose a deliberate PostgreSQL migration or
restore procedure with a backup; do not remove the volume merely to bypass the
check.

## Cleaning up

```bash
source source_me.sh && python3 local_stack.py stop # stop the stack, keep data
podman machine stop                                # stop the virtual machine
```

Destructive cleanup (removing volumes, pruning images) is deliberately not
scripted here. Run those by hand when you mean them.
