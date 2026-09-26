# Podman on macOS

macOS cannot run Linux containers directly, so Podman runs them inside a Linux
virtual machine it manages for you. Everything in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) assumes that machine is running.

## Install

Install Podman, then create the machine with enough resources for the first Rust
build. On Apple Silicon, this explicit rootful configuration uses Apple's native
hypervisor and makes `/Users` available inside the VM:

```bash
brew install podman
podman machine init \
  --provider applehv \
  --rootful \
  --cpus 8 \
  --memory 16384 \
  --disk-size 60 \
  -v /Users:/Users
podman machine start
```

`podman machine init` is a one-time step. Omit `--rootful` when the host should
use Podman's rootless default. The PLE controller accepts either mode and uses
the active default Podman connection; it does not select or change that
operator-owned setting. `podman machine start` is needed after a reboot.

AppleHV is the virtualization provider. The Fedora-based Podman Machine OS is
the Linux guest running inside AppleHV, not a competing provider. Seeing Fedora
in `podman info` therefore does not mean that Podman ignored `--provider applehv`.
Podman's normal macOS configuration already shares `/Users`, but the
explicit mount above makes this checkout requirement visible at machine
creation.

## Check the machine

```bash
podman machine list
podman machine inspect
podman info --format '{{.Host.Arch}}'
```

A machine that is not `Currently running` is the cause of most
"cannot connect to Podman" errors.

## Existing machines

The first container build compiles the whole dependency tree. Prefer setting
CPU, memory, disk, provider, mount, and root mode during the one-time
initialization above. Do not recreate a machine merely to change root mode.
Podman can change the preferred connection on an existing stopped machine:

```bash
podman machine stop
podman machine set --rootful=true   # or --rootful=false
podman machine start
```

Rootful and rootless Podman storage are separate. Select the intended mode
before starting PLE; changing modes does not delete either store. PLE neither
switches modes nor searches another store automatically because doing so could
operate on the wrong project.

## Architecture notes

Apple Silicon runs `arm64` Linux images natively. The selected PostgreSQL,
Alpine secret-initializer and gateway base-image digests
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
so `curl --insecure https://localhost:8080/health` reaches the canonical Live
Demo HTTPS gateway from the host exactly as it would on Linux. The controller-
managed browser path trusts only that disposable gateway's internal CA; the
diagnostic command above does not change the host trust store. Inside the
compose network, services address each other by service
name (`postgres`, `minio`), not `localhost`. See
`docs/CONTAINER_PORT_MAPPING.md` for port selection.

## Registry prefixes

Podman does not assume Docker Hub. Image references in
[containers/compose.yaml](../containers/compose.yaml) are fully qualified and use
floating tags, including `docker.io/library/postgres:latest` and
`docker.io/library/alpine:latest`. The gateway builds from `docker.io/library/caddy:latest`.
The external renderer image is owned and built by the separate `webwork-pg-renderer`
project; PLE records its resolved image configuration ID as runtime evidence.

These references are not all runtime images. `postgres` and `identity-secret-init` run their specified external
images. `minio` and `createbuckets` share `localhost/ple-object-storage:reviewed`,
built from official server and client sources selected with `@latest` in
`containers/Containerfile.object_storage`. The upstream community distribution is
[source-only](https://github.com/minio/minio), and its former registry images
are unavailable. The first build downloads Go dependencies and compiles both
executables with two Go workers; subsequent launches reuse the build cache.
Startup preserves that cache rather than pruning it before each build. `api` builds the shared local application image from
`containers/Containerfile.api`, and `worker` consumes that exact image;
`gateway` is built locally from `containers/Containerfile.gateway` using the
current official Caddy image. The `webwork-renderer` is an external-project image. Build-mode PLE
startup reuses the tracked `localhost/pg-renderer:reviewed` selection or rebuilds
it from the maintained sibling checkout when pruning removed it. PLE resolves that selected name to its OCI
configuration ID, confirms the container runs that ID, and records both as
Question Renderer Version. A published deployment can select a pullable
`repository@sha256:<64-lowercase-hex>` value through the same configuration
key.

## PostgreSQL retained volumes

Before PostgreSQL starts, the private typed lifecycle runs the maintenance-profile `postgres-major-guard`.
The guard mounts `ple_pgdata` read-only and checks its `PG_VERSION` file. A
populated volume whose major differs from the selected image's `PG_MAJOR` is
refused before the database service starts.

This guard never migrates, rewrites, or deletes the volume. If it refuses an
older or newer volume, stop and choose a deliberate PostgreSQL migration or
restore procedure with a backup; do not remove the volume merely to bypass the
check.

## Cleaning up

```bash
./launchers/run_live_demo.sh stop                  # stop and remove live-demo data
podman machine stop                                # stop the virtual machine
```

`./launchers/run_live_demo.sh stop` performs the authenticated owner cleanup for the
fixed live-demo project. It removes only that project's labelled containers,
volumes, networks, and private workspace, so the next launch starts with a
fresh demo dataset. Review status and logs before stopping when diagnostic
evidence needs to be retained. Image pruning remains a separate manual
operation; this command does not affect the retained `containers` project or
other Podman images.
