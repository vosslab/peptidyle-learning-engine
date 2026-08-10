# Troubleshooting local stacks

Use this guide when the maintained local-stack launcher stops before the browser application is
ready. It covers observed launcher checks and safe recovery actions. For initial requirements and
normal commands, see [INSTALL.md](INSTALL.md), [USAGE.md](USAGE.md), and
[MACOS_PODMAN.md](MACOS_PODMAN.md).

## Preflight failures

- **`command not found on PATH`:** install the named launcher prerequisite, then rerun
  `./launch_local_stack.sh --check`. The launcher requires Git, Podman, curl, awk, OpenSSL, xxd,
  and lsof before it changes local state.
- **`neither 'podman compose' nor 'podman-compose' is usable`:** install a Compose provider for
  Podman, then confirm it is available with `podman compose version` before rerunning the check.
- **`containers/env.local is missing or unreadable`:** run
  `./launch_local_stack.sh --no-open` once. The normal default launcher creates its ignored local
  configuration and credentials with restrictive permissions; `--check` deliberately does not.
- **`--check cannot validate this pre-image-pin env.local`:** run
  `./launch_local_stack.sh --no-open` once to add the required immutable local image settings, then
  rerun `./launch_local_stack.sh --check`.
- **`Compose configuration is incomplete`:** keep the error text, correct the named value in the
  selected environment file, then rerun the preflight. A custom `--env-file` is not rewritten by
  the launcher and must provide every required setting itself.

## Podman is unavailable

On macOS, a normal launcher run attempts to start the Podman machine after configuration validation.
When that fails, inspect and start the machine explicitly:

```bash
podman machine list
podman machine start
podman info
```

If the machine is already running but a container build is exhausted or killed, stop it, increase
its resources, and start it again using the documented values in [MACOS_PODMAN.md](MACOS_PODMAN.md).
Do not treat `--check` as a machine-start command: it is intentionally read-only.

## Startup does not finish

- **`PostgreSQL did not become ready`:** the launcher leaves its containers running. Inspect the
  service state and recent logs, then correct the reported container failure before retrying.

  ```bash
  podman compose -f containers/compose.yaml --env-file containers/env.local ps
  podman compose -f containers/compose.yaml --env-file containers/env.local logs --tail=80 postgres minio
  ```

- **`the stack did not become ready`:** inspect the gateway, API, and worker logs. The launcher
  waits for semantic `/health`, so a running container alone is not a successful start.

  ```bash
  podman compose -f containers/compose.yaml --env-file containers/env.local logs --tail=80 gateway api worker
  curl -s http://127.0.0.1:3000/health
  ```

  If first-run bootstrap selected another gateway port, use the recorded `PLE_GATEWAY_HOST_PORT`
  in `containers/env.local` for the health request.

- **`--skip-build requires dist/index.html and dist/main.js`:** build first with `./build.sh`, or
  rerun the launcher without `--skip-build`.

For an optional renderer failure, `WeBWorK did not pass its authenticated render_rpc probe`, retain
the running renderer for inspection and collect its logs before retrying:

```bash
podman compose -f containers/compose.yaml -f containers/compose.webwork.yaml \
  --env-file containers/env.local --profile webwork logs --tail=80 webwork-db webwork-renderer
```

## Existing database volumes

`the existing PostgreSQL data volume is not compatible with the pinned PostgreSQL 17 image` means
the launcher found an existing data directory from another PostgreSQL major version. Preserve that
volume and migrate it with an explicit PostgreSQL-major-version procedure; do not delete it merely
to make the local stack start. Once the data is safely migrated, rerun the launcher.

## Stop without deleting data

After collecting diagnostics or when finished, stop the default stack while retaining its named
volumes:

```bash
podman compose -f containers/compose.yaml --env-file containers/env.local down
```

Use the same Compose files, profile, and environment file that started an optional WeBWorK stack.
Avoid volume removal and image pruning unless the affected local data and build cache are
intentionally disposable.
