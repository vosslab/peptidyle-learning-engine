# Troubleshooting local stacks

Use this guide when the fixed production-shaped developer stack does not start,
does not become ready, or does not clean up. The root wrapper owns ordinary
start and stop operations. The controller is the direct, scoped diagnostic
interface; it does not authorize a caller-selected live-demo project.

For prerequisites and normal operation, see [INSTALL.md](INSTALL.md),
[USAGE.md](USAGE.md), and [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md).

## Triage order

Use the root front door for mutations. It sources the repository shell environment and invokes
the fixed controller with `python3`, selects the owner-locked `ple-live-demo-browser` project,
and does not accept a caller-selected project, environment, or identity. Start
with read-only inspection, preserve the private owner receipt, and change one
named failure at a time. The command surface is implemented in
[local_stack.py](../local_stack.py#L1-L28) and the wrapper dispatch is in
[run_live_demo.sh](../run_live_demo.sh#L27-L43).

Do not use `podman compose down`, `podman system prune`, or another global
cleanup command. Those commands bypass the label and lease checks that prove
the project scope.

## Preflight failures

- **`Python 3.12 is required`:** install `python3.12`, make it available as `python3`, then rerun
  `./run_live_demo.sh`. The wrapper sources `source_me.sh` and invokes the controller directly.
- **`command not found on PATH`:** install the named prerequisite and retry
  `./run_live_demo.sh`. The wrapper requires Git, Podman, curl, awk, OpenSSL,
  xxd, and lsof.
- **`neither 'podman compose' nor 'podman-compose' is usable`:** install one
  supported Podman Compose provider and retry `./run_live_demo.sh`.
- **`a custom mutating env file must already exist and have mode 0600`:** use
  the repository's first-run path with the default environment. Do not point
  the wrapper at another environment, project, identity, SMTP configuration,
  or build selector.

## Read-only diagnostics

Install the declared dependencies into the selected Python 3.12 environment when an import is
missing:

```bash
source source_me.sh && python3 -m pip install --requirement pip_requirements-dev.txt
```

Run these commands from the repository root before changing the stack:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py projects
source source_me.sh && python3 local_stack.py status --project ple-live-demo-browser
source source_me.sh && python3 local_stack.py logs --project ple-live-demo-browser --tail 120 gateway api worker
source source_me.sh && python3 local_stack.py validate
```

`doctor` checks Podman, its Compose provider, the macOS machine, the selected
environment, and labelled projects. `projects` includes retained data-only
projects. `status` reports semantic readiness; running containers alone are
not sufficient. `logs` prints a private-data warning and accepts `--follow`
only while actively diagnosing. `validate` checks initialized configuration,
the renderer identity, and the available engine without mutating the stack.

## Podman and port failures

- **Podman is unavailable:** inspect the machine and engine, then retry the
  wrapper after correcting the reported state:

  ```bash
  podman machine list
  podman machine start
  podman info
  ./run_live_demo.sh
  ```

  On macOS, use the resource values in [MACOS_PODMAN.md](MACOS_PODMAN.md) if
  the machine is exhausted. These are diagnostics; do not use global Compose
  cleanup.

- **`local port ... is already listening`:** identify the owning process with
  the reported port, stop only that process if you own it, then retry
  `./run_live_demo.sh`. The fixed owner chooses a free loopback gateway port
  on first setup when the default port is occupied.
- **`selected gateway port is occupied`:** correct the selected environment's
  port ownership, then retry `./run_live_demo.sh`; do not substitute a project
  or arbitrary port through the command line.

## Startup failures

- **`host artifact build failed (...)`:** inspect the reported build failure,
  correct the source or dependency, then retry `./run_live_demo.sh`. The owner
  builds the production `dist/` bundle before Compose startup.
- **`PostgreSQL did not become ready`:** inspect retained services and database
  logs, correct the reported container, image, or volume problem, then retry:

  ```bash
  source source_me.sh && python3 local_stack.py status --project ple-live-demo-browser
  source source_me.sh && python3 local_stack.py logs --project ple-live-demo-browser --tail 120 postgres
  ./run_live_demo.sh
  ```

- **`the stack did not become ready`:** inspect `gateway`, `api`, and `worker`
  logs. Readiness is semantic `/health`, not merely a running container.
  Retry the same `./run_live_demo.sh` command after correcting the named
  failure.
- **Gateway `unhealthy` while `webwork-renderer` is `starting`:** this is a
  normal transient dependency state. Wait for the configured timeout. If it
  expires, inspect renderer logs first, correct the renderer image or
  render/grade probe failure, and retry `./run_live_demo.sh`.
- **`running renderer does not match the selected OCI configuration`** or
  **`renderer service is missing or ambiguous`:** preserve the private owner
  receipt, inspect status and renderer logs, and retry the fixed wrapper after
  correcting the labelled renderer. Do not substitute an image or project at
  the command line.

## Future browser and screenshot evidence

The fresh Store-backed browser owner will publish the current browser and
screenshot troubleshooting steps together with its executable route surface.
Current diagnosis uses the typed local-stack controller and its available
database/object acceptance lanes.

## Browser and cleanup permission failures

- **`bootstrap_check_in ... MachPortRendezvousServer ... Permission denied`:**
  macOS denied Chromium before a browser context opened. This is not a PLE or
  `/health` result. Rerun the unchanged browser command from a terminal with
  the required browser permission.
- **`service-oracle final owner-process identity probe permission was denied`:**
  the final cleanup proof could not run its read-only `ps` probe. Rerun the
  unchanged operation with host process-inspection permission. Do not disable
  the probe or claim cleanup without its receipt.
- **Any other `service-oracle final owner-process ...` failure:** preserve the
  private receipt and correct the named probe failure. Resource exhaustion has
  only its bounded retry; missing executables, malformed output, and nonzero
  probe exits remain failures.
- **`developer browser control state is unavailable`:** preserve the private
  receipt and lease error, then retry the same wrapper command. Do not start a
  second project or use raw global Compose cleanup.

## Existing data and migrations

- **`existing PostgreSQL data volume is not compatible with the pinned
PostgreSQL 17 image`:** preserve the volume and migrate it with an explicit
  PostgreSQL-major-version procedure. Do not delete it to make startup pass.
- **`migration ... was previously applied but is missing in the resolved
migrations`:** preserve the retained resource and private owner receipt.
  Correct the image or migration problem, then retry the fixed wrapper; do not
  edit `_sqlx_migrations` or run global cleanup.
- **`2026081866` refuses a nonempty receipt table:** this is the intentional
  clean-volume preflight. Preserve the refusal, migration output, and private
  owner receipt. For disposable demo data, use the fixed owner reset, then
  create a fresh seeded installation:

  The preflight locks both receipt tables and refuses any existing row before
  adding provenance constraints; see
  [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).

  ```bash
  source source_me.sh && python3 local_stack.py reset --confirm-project containers
  ./run_live_demo.sh
  ```

  Retained receipt history follows the separately planned immutable augmentation
  path. Keep the volume for that review; do not edit `_sqlx_migrations`, backfill
  append-only receipts, or weaken the 1866 preflight.

## Stop and acceptance

Stop the active developer owner and remove its fixed disposable browser
containers, volumes, networks, and private workspace:

```bash
./run_live_demo.sh stop
```

This stop is intentionally destructive to the disposable live-demo data. If
the data must be retained for diagnosis, run the read-only commands above and
preserve the owner receipt before stopping. The ordinary `containers` project
has a separate, explicitly confirmed reset path; preview it before any reset:

```bash
source source_me.sh && python3 local_stack.py reset --dry-run
source source_me.sh && python3 local_stack.py reset --confirm-project containers
```

The owner verifies containers, volumes, networks, workspace, and private
receipts. For no-skip connected acceptance, use the fixed interpreter after
the stack is available:

```bash
source source_me.sh && python3 local_stack.py acceptance
```

Browser and Podman acceptance are explicit evidence; permanent offline tests
remain separate. See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md#validation-test-suite).
