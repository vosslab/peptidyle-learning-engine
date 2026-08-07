# Podman on macOS

macOS cannot run Linux containers directly, so Podman runs them inside a Linux
virtual machine it manages for you. Everything in
[CONTAINER.md](CONTAINER.md) assumes that machine is running.

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

Apple Silicon runs `arm64` images natively. The images this stack uses
(`postgres:17-alpine`, `quay.io/minio/minio`, `rust:1.96-bookworm`,
`debian:bookworm-slim`) all publish `arm64` variants, so no emulation is
involved.

If you need to reproduce an `amd64` deployment locally, pass the platform
explicitly and expect it to run slowly under emulation:

```bash
podman build --platform linux/amd64 -f containers/Containerfile.api -t peptidyle-api .
```

## Ports and localhost

Podman forwards published ports from the virtual machine to macOS `localhost`,
so `curl http://localhost:3000/health` works from the host exactly as it would
on Linux. Inside the compose network, services address each other by service
name (`postgres`, `minio`), not `localhost`.

## Registry prefixes

Podman does not assume Docker Hub. Image references in
[containers/compose.yaml](../containers/compose.yaml) are fully qualified
(`docker.io/library/postgres:17-alpine`, `quay.io/minio/minio:latest`) so a
pull never depends on local registry search order.

## Cleaning up

```bash
podman compose -f containers/compose.yaml down    # stop the stack, keep data
podman machine stop                                # stop the virtual machine
```

Destructive cleanup (removing volumes, pruning images) is deliberately not
scripted here. Run those by hand when you mean them.
