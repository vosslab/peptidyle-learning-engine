# Troubleshooting local stacks

Use this guide when the fixed developer stack is not ready. The Python
controller owns the `ple-live-demo-browser` project, its lease, scoped
diagnostics, and authenticated cleanup. For initial requirements and normal
commands, see [INSTALL.md](INSTALL.md), [USAGE.md](USAGE.md), and
[MACOS_PODMAN.md](MACOS_PODMAN.md).

## Preflight failures

- **`command not found on PATH`:** install the named launcher prerequisite,
  then retry the fixed `start` command. The launcher requires Git, Podman,
  curl, awk, OpenSSL, xxd, and lsof before it changes local state.
- **`neither 'podman compose' nor 'podman-compose' is usable`:** install a
  Compose provider for Podman, then retry the fixed `start` command.
- **`developer browser control state is unavailable`:** inspect the private
  receipt and lease failure, then rerun the fixed command. Do not add a project,
  environment, identity, SMTP, or build selector.

## Read-only diagnostics

Run these commands before changing the stack. They use the controller's
validated Podman and project discovery paths:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py projects
source source_me.sh && python3 local_stack.py status --project ple-live-demo-browser
source source_me.sh && python3 local_stack.py logs --project ple-live-demo-browser --tail 120 gateway api worker
```

`doctor` reports Podman, the Compose provider, the macOS machine, the local
environment file, and labelled projects. `projects` includes retained
data-only projects. `status` reports semantic readiness; a running container
alone is not readiness. `logs` prints a warning because application logs may
contain private local diagnostic data. Use `--follow` only while actively
diagnosing the selected services.

## Podman is unavailable

On macOS, a normal controller `start` attempts to start the Podman machine after configuration validation.
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

- **`host artifact build failed (...)`:** reproduce the production browser
  artifact failure directly, then retry the fixed owner after correcting the
  reported build error:

  ```bash
  ./build.sh --debug
  source source_me.sh && python3 local_stack.py start --headless
  ```

  The lifecycle builds `dist/` before it reconciles the Compose project. Do not
  switch to an alternate browser build or selector.

- **`PostgreSQL did not become ready`:** the launcher leaves its containers running. Inspect the
  service state and recent logs, then correct the reported container failure before retrying.

  Preserve the private owner receipt and correct the reported container or
  image issue before retrying `start`.

- **`the stack did not become ready`:** inspect the gateway, API, and worker logs. The owner
  waits for semantic `/health`, so a running container alone is not a successful start.

  Use the HTTPS origin printed by `start`; do not probe a guessed port or
  substitute a different project. Preserve the private owner receipt if the
  supervisor reports a failed cleanup.

- **The gateway is `unhealthy` while `webwork-renderer` is `starting`:** this is an expected
  transient state during normal startup. The API waits for the renderer's real render-and-grade
  probe before it starts, and the gateway becomes healthy only after the API's semantic health
  check succeeds. Let the launcher reach its configured timeout before treating this state as a
  failure. If it times out, collect the renderer logs below first; the renderer is the upstream
  dependency in this startup sequence.

For `the standalone PG renderer did not pass its render/grade probe`, preserve
the owner receipt and correct the reviewed renderer input before retrying.

## Email sign-in and invitations

- **A new invitation reports `emailDelivery: queued`:** PLE accepted it for processing. This state
  is not proof of provider submission or mailbox delivery. When no external SMTP provider is
  configured, use the Instructor-only one-time copy link through the course's established channel.
  The invitation remains single-use and the learner still completes email authentication before it
  can become course membership.
- **An invitation reports `emailDelivery: sentToProvider`:** the configured provider accepted the
  submission, but this does not confirm mailbox delivery. The copy link remains available as the
  direct course-channel handoff.
- **An invitation reports `emailDelivery: needsAttention`:** delivery needs explicit operator
  attention after an ambiguous result or a failure that remains after retry processing, including
  a permanent failure. Do not treat it as delivered; use a fresh explicit resend when available,
  or cancel it and create a new invitation.
- **An invitation reports `emailDelivery: cancelled`:** its link is fenced and must not be shared.
  Create a new invitation if enrollment is still needed.
- **Email sign-in is unavailable through the developer stack:** do not treat
  that as a local-stack startup failure. The developer entry uses the seeded
  production-auth browser flow; provider and deployment configuration are
  outside this local owner.

## Existing database volumes

`the existing PostgreSQL data volume is not compatible with the pinned PostgreSQL 17 image` means
the launcher found an existing data directory from another PostgreSQL major version. Preserve that
volume and migrate it with an explicit PostgreSQL-major-version procedure; do not delete it merely
to make the local stack start. Once the data is safely migrated, rerun the launcher.

`migration ... was previously applied but is missing in the resolved migrations` means the owner
found an incompatible retained resource. Do not edit `_sqlx_migrations` or use a global cleanup
command. Preserve the private owner receipt and rerun the fixed `start`/`stop` lifecycle after the
underlying image or migration issue is corrected.

## Stop without deleting data

After collecting diagnostics or when finished, stop the active developer session
through its authenticated owner:

```bash
source source_me.sh && python3 local_stack.py stop
```

The owner verifies that its containers, volumes, networks, workspace, and private
receipts are gone. Do not substitute a project selector or raw Compose.

## Acceptance

`source source_me.sh && python3 local_stack.py acceptance` is the no-skip live
acceptance command. Browser selection uses `run_playwright_tests.sh`; the
canonical owner lease serializes developer and browser sessions. Permanent
offline controller tests remain in the normal test gates; Podman/browser
acceptance is explicit evidence. See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md#validation-test-suite).
