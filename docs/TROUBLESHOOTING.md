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
  curl -s http://127.0.0.1:8080/health
  ```

  If bootstrap selected another gateway port, use the recorded `PLE_GATEWAY_HOST_PORT` in
  `containers/env.local` for the health request. See
  `docs/CONTAINER_PORT_MAPPING.md` for the port policy.

- **The gateway is `unhealthy` while `webwork-renderer` is `starting`:** this is an expected
  transient state during normal startup. The API waits for the renderer's real render-and-grade
  probe before it starts, and the gateway becomes healthy only after the API's semantic health
  check succeeds. Let the launcher reach its configured timeout before treating this state as a
  failure. If it times out, collect the renderer logs below first; the renderer is the upstream
  dependency in this startup sequence.

- **`--skip-build requires dist/index.html and dist/main.js`:** build first with `./build.sh`, or
  rerun the launcher without `--skip-build`.

For `the standalone PG renderer did not pass its render/grade probe`, retain the running renderer
for inspection and collect its logs before retrying:

```bash
podman compose -f containers/compose.yaml --env-file containers/env.local \
  logs --tail=80 webwork-renderer
```

## Email sign-in and invitations

- **A new invitation reports `emailDelivery: notSent`:** this is the normal copy-link path when
  no external SMTP provider is configured. Give the manager-only one-time link to the learner
  through the course's established channel. The invitation remains single-use and the learner
  still completes email authentication before it can become course membership.
- **Email sign-in is unavailable through the production process:** this is expected today. The
  canonical account/session route graph is composed, but no SMTP provider or email-activation path
  is configured. Fastmail is the intended future provider. When its operator account, authorized
  sender, and application credential exist, configure it through [the external SMTP provider
  contract](LOCAL_STACK_OPERATIONS.md#external-smtp-provider), then verify live delivery and
  browser sign-in. PLE connects to that provider; it does not run a mail server or need local
  mail-deliverability infrastructure. Copy links avoid SMTP only for invitation delivery; they do
  not replace email authentication. Missing email infrastructure does not block the deliberately
  no-email local teaching walkthrough.

## Existing database volumes

`the existing PostgreSQL data volume is not compatible with the pinned PostgreSQL 17 image` means
the launcher found an existing data directory from another PostgreSQL major version. Preserve that
volume and migrate it with an explicit PostgreSQL-major-version procedure; do not delete it merely
to make the local stack start. Once the data is safely migrated, rerun the launcher.

## UI walkthrough is blocked

Use the opt-in walkthrough only with a fixed seed:

```bash
bash tests/walkthrough/run_ui_walkthrough.sh --master-seed 42
```

It uses IPv4 loopback only. AUTO reuses a safe existing bundle or builds when
missing; use `--build` to force a refresh. Its redacted `PASS` or `FAIL` report is
under `test-results/ui_walkthrough/`, with directory/file modes 0700/0600.

Use documented arguments rather than exporting `PLE_*` values to change this
run. For example, select a nondefault Compose file with `--env-file PATH` or
request documentation capture through the documented screenshot command. The
runner deliberately removes inherited `PLE_*` values before it launches owned
children. If a child reports `walkthrough-inputs`, rerun the shell command and
allow the runner to recreate its private input file; do not create or edit that
file by hand.

The earlier no-email pilot provides historical diagnosis evidence. The current
human-guidance acceptance still requires a fresh real-stack J13 copy/paste run
of the four Genetics visible IDs. Assignment, course, and gradebook surfaces
provide native `target="_self"` fragment routes to named `tabindex="-1"`
pagination regions; Tab reaches visible load, retry, or reload controls. If a
future `playwright_*` stage fails, preserve the redacted report and inspect its
bounded stage rather than deleting volumes or using a direct route. The runner
fails closed for missing/current-target, transport, and protocol issues.
Email/canonical onboarding is deliberately not a diagnostic prerequisite for
this local pilot.

## Stop without deleting data

After collecting diagnostics or when finished, stop the default stack while retaining its named
volumes:

```bash
podman compose -f containers/compose.yaml --env-file containers/env.local \
  down --remove-orphans
```

Use the same environment file that started the stack. Preserve the named PostgreSQL and MinIO
volumes unless their data is intentionally disposable.
