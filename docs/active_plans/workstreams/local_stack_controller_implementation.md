# Local stack controller workstream

> **Historical and superseded browser record (2026-08-22).** This workstream preserves the dated
> WP-PY-L1 controller evidence and acceptance. Its browser ledger, including the `202` ordinary
> mock-preview tests and seven legacy lanes, is historical evidence only and does not define the
> supported browser architecture, catalog, commands, or lane ownership. For current truth, follow
> the [real-stack browser suite plan](../active/real_stack_browser_suite_plan.md),
> [implementation status](../implementation_status.md), and
> [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md).

## Status

The controller foundation is complete as of 2026-08-14. WP-PY-L1 was accepted on 2026-08-15: focused
Python modules directly replace its shell lifecycle owner, final material-tree Validation is green, and
the required independent reviews ACCEPT with no P0-P3 finding. M0 remains open; WP-RC8 acceptance is
the next package in the active release plan.

## Objective and completion boundary

Build the one supported, Python-based Podman lifecycle layer for PLE's local Compose environments.
It gives developers, Codex agents, the aggregate browser-acceptance entry point, and the canonical UI
walkthrough the same typed provider selection, environment ownership, project discovery, readiness,
and cleanup rules. It is a controller for explicitly declared Compose targets, not a second launcher
or a general-purpose wrapper around the user's Podman store.

The supported operator entry point is:

```bash
source source_me.sh && python3 local_stack.py <command>
```

The workstream is complete only when that controller is the normal local operator front door, its
shared library is used by the aggregate acceptance and canonical-walkthrough lifecycle paths, its
full Validation test suite is green on the final tree, and the independent reviews below have no
unresolved blocking finding. The work does not require compatibility behavior for a pre-production
legacy controller or retained local data.

## Stable lifecycle contract

### Ownership and boundaries

- The system remains rootless-first and Compose-owned. It does not introduce rootful Podman,
  `sudo`, a Podman pod, Quadlet, host systemd, direct volume mounting, global prune, or a broad
  resource-removal command.
- Focused Python modules now own build order, local-secret bootstrap, migrations, grading-role
  provisioning, seed publication, renderer identity/probe, polling, and semantic service readiness.
  WP-PY-L1 replaces `launch.sh`, `_restart.sh`, and `containers/local_identity_bootstrap.sh` together;
  no Python wrapper or dual launcher is permitted. WP-PY-L1 final material-tree Validation and its
  independent reviews are accepted.
- `containers/env.local`, or an explicit `--env-file`, owns Compose interpolation values. The
  controller reads names and safe metadata only, strips inherited colliding names from child
  environments, and never prints credentials, bearer values, capability material, or a private
  environment file's contents.
- The default mutating target is exactly the pre-production `containers` project. Read-only label
  discovery may inspect another project. Mutation of another target is possible only through a
  typed disposable-owner contract created by its owning runner, never through a user-supplied
  project name or a broad `--force` switch.
- Resource discovery uses Compose project labels, including
  `io.podman.compose.project` and `com.docker.compose.project`, for containers, volumes, and
  networks. Generated container names are display-only evidence, never authority.

### Public command surface

| Command | Scope and effect |
| --- | --- |
| `doctor` | Read-only engine, Compose-provider, selected env-file metadata, macOS machine, and declared-versus-observed port diagnostics. |
| `projects` | Read-only labelled Compose-project inventory for humans and automation. |
| `status [--project NAME]` | Read-only typed project snapshot and semantic status: required long-running health, one-shot completion, and published ports. Nonzero means the requested stack is not ready. |
| `logs [SERVICE...]` | Read-only scoped Compose logs. It warns that logs may contain local diagnostics and does not claim redaction. |
| `validate` | Runs typed read-only validation, then reports current label-derived status when a stack exists. It does not create an env file, start a machine, build, or mutate containers. |
| `start` | Calls the selected normal target's typed lifecycle directly, including `--skip-build`, `--no-open`, `--with-smtp`, and `--env-file`. |
| `stop` | Resolves and prints the default target, then runs scoped `compose down --remove-orphans`; named volumes remain. |
| `restart SERVICE` | Recreates only the declared stateless service set (`api`, `worker`, `gateway`, `webwork-renderer`) after target resolution. It rejects PostgreSQL and MinIO. |
| `reset --confirm-project containers` | Prints the exact labelled resources and scoped `compose down --volumes --remove-orphans` command, then removes only the default project's Compose data. `--dry-run` is inspection only. It preserves ignored host configuration and tells the operator to use `start` for reinitialization. |
| `acceptance` | Runs Python-owned aggregate acceptance lanes after the shared preflight establishes that no conflicting PLE target is active. The lane runner uses the typed boundary directly and owns no separate provider, label, or cleanup implementation. |

No command accepts arbitrary destructive project, volume, network, image, `all`, `prune`, or force
semantics. Each mutation displays its resolved non-secret argv and target before it starts. Reset
requires the visible project acknowledgement; stopping and restarting retain data but still prove
the target by labels first.

### Typed reusable layer

The package command stays intentionally thin. `local_stack_control/` is the neutral, importable owner
of the controller's durable concepts:

- `ComposeProvider`, `ComposeTarget`, `DisposableComposeTarget`, `ProjectSnapshot`, and typed
  container, volume, network, port, and readiness records;
- provider selection, Compose argv creation, sanitized child environments, and selected env-file
  validation;
- label-driven resource discovery and conflict/preflight decisions;
- status and readiness evaluation that distinguishes required one-shots from long-running services;
- a narrow cleanup planner that can form a normal default-project stop/reset plan or a
  `DisposableComposeTarget` plan. The latter requires the runner-provided project name, private
  environment file, explicit compose files, private runner-held cleanup capability, and closed
  owner-policy validation before it can issue cleanup. One neutral typed policy registry owns every
  disposable owner's namespace, full project grammar, and exact ordered Compose files; consumers
  may add application assertions but cannot restate or widen target shape. Every disposable target
  uses the controller-wide `podman-compose --in-pod false` provider contract, so an unlabelled pod
  cannot escape label-driven cleanup. The controller cannot infer
  authority from a name pattern or a self-asserted token alone.

The implementation uses the Python standard library only: `argparse`, `dataclasses`, `json`,
`pathlib`, `platform`, `subprocess`, and `collections.abc` as needed. It does not add a runtime
package or a test-only framework dependency. Subprocess calls use argument arrays and a typed
runner boundary so pure decisions remain testable offline.

## Required centralization and retained specialised ownership

This work intentionally removes repeated lifecycle policy, rather than only adding a new command.
The following migration is part of this workstream and is not deferred:

| Consumer | Moves to `local_stack_control` | Retains local ownership |
| --- | --- | --- |
| `python3 local_stack.py` and operator docs | Provider selection, default-target/environment handling, labels, preflight, status, logs, stop, restart, reset, and typed lifecycle calls. | Human-facing command wording and normal `containers` target. |
| `python3 local_stack.py acceptance` | Its PLE-stack conflict discovery/preflight and Python aggregate lane handoff use the shared project snapshot and typed preflight result. | Ordered browser lanes and their reports. It never deletes a stack it did not create. |
| The former `tests/walkthrough/walklib/runner.py`, `podman_preflight.py`, `podman_ownership.py`, and `stack_environment.py` | Compose-provider choice, sanitized private child environment, label discovery, fixed-port preflight, generated-project target construction, exact resource preview, and cleanup execution use the shared typed layer. | Random project identity, private credentials, user-visible walkthrough actions, report redaction, owned images, and failure receipt. The runner retains the cleanup capability and remains the sole authority to request its cleanup. |
| [../../../tests/e2e/e2e_webwork_render_rpc.sh](../../../tests/e2e/e2e_webwork_render_rpc.sh) | Normal-stack provider/target resolution and the stateless renderer restart route through the controller or a thin explicit adapter around its public library. | WebWork render/grade assertions and its application-specific evidence. |
| Other disposable E2E owners | Use the shared provider, environment, labelled snapshot, and disposable-target APIs when their own ownership contract is migrated in this workstream; each migration has an explicit target and focused acceptance. | Random ports, isolated seed/input, secrets, test semantics, and only the resources their runner created. |

The first required disposable migration is the canonical walkthrough. WP-R1 immediately follows by
moving Chapter One pilot/browser and aggregate acceptance lanes to Python over this typed boundary.
The WebWork restart follows after the normal `restart webwork-renderer` live proof. Course-appearance,
database-baseline, and replica runners migrate later where their mechanics match the shared contract.
A retained shell wrapper is direct `exec` or `source` only and must not retain provider selection,
target sanitization, label discovery, generic cleanup, private files, polling, or lane aggregation.

## Work packages, dependencies, and owned files

These packages are a dependency order, not optional follow-up work. A later package is not complete
if it merely wraps an earlier implementation while retaining a second lifecycle policy in a shell
script or test runner.

1. **Contract, placement, and source-of-truth update.** Record the owner decisions here, in
   [../../HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md), and in the foundation plan. Confirm the
   root command/package placement against [../../REPO_STYLE.md](../../REPO_STYLE.md),
   [../../PYTHON_STYLE.md](../../PYTHON_STYLE.md),
   [../../PYTEST_STYLE.md](../../PYTEST_STYLE.md), [../../../tests/TESTS_README.md](../../../tests/TESTS_README.md),
   and [../../../devel/DEVEL_README.md](../../../devel/DEVEL_README.md). This package lands before
   code or permanent tests.
2. **Typed controller foundation.** Own `local_stack_control/` with focused modules for models,
   process execution, environment parsing, Compose target and disposable-owner construction,
   resource discovery, readiness/status, cleanup planning, command behavior, and CLI parsing. Keep
   every authored source below the repository line limit. This package depends only on package 1
   and the standard library; it must not depend on a test runner.
3. **Developer lifecycle and operator guidance.** Implement and document the public command surface in
   [../../LOCAL_STACK_OPERATIONS.md](../../LOCAL_STACK_OPERATIONS.md), [../../USAGE.md](../../USAGE.md),
   [../../DEVELOPMENT.md](../../DEVELOPMENT.md), [../../TROUBLESHOOTING.md](../../TROUBLESHOOTING.md),
   [../../CODE_ARCHITECTURE.md](../../CODE_ARCHITECTURE.md), and
   [../../FILE_STRUCTURE.md](../../FILE_STRUCTURE.md). Direct Compose commands remain narrowly
   documented recovery evidence, not a competing normal path. This package depends on package 2.
  A non-default lifecycle invocation additionally requires the runner-held capability matching the
   private environment commitment; an asserted project name or environment path is not authority.
4. **Aggregate acceptance ownership.** Move provider selection, environment sanitization, project
   conflict preflight, status gating, lifecycle handoff, and failure-safe cleanup into the Python
   controller. The public Python acceptance command invokes the internal ordered browser lanes and
   preserves their report locations and exit propagation. It must not implement a second Podman
   lifecycle or delegate without checking the typed result. This package depends on packages 2 and 3; browser
   behavior remains a distinct consumer of the ready stack.
5. **Disposable-owner migrations.** Migrate the former `tests/walkthrough/walklib/runner.py`,
   `podman_preflight.py`, `podman_ownership.py`, and `stack_environment.py` to the shared typed
   provider, environment, label, target, preview, and cleanup APIs. The generated walkthrough
   project, private credentials, random ports, seed/input, visible-action checks, report redaction,
   runner-held cleanup capability and cleanup authority remain with the walkthrough runner. Then
   migrate the WebWork,
   Chapter 1, course-appearance, database-baseline, and replica owners only where their mechanics
   match the shared contract; their application assertions remain distinct. Update
   former `tests/walkthrough/README.md` and the
   `tests/e2e/` documentation when ownership changes. This
   package depends on packages 2 and 4 and is not deferred to a later cleanup project.
6. **Durable proof, live evidence, and release handoff.** Add only permanent pure behavior tests to
   [../../../tests/test_local_stack_control.py](../../../tests/test_local_stack_control.py). Run the
   real Podman stop-retain, confirmed reset-rebuild, stateless restart, disposable walkthrough, and
   full no-skip acceptance checks as one-time/opt-in evidence, not regular networked pytest. Record
   evidence and unresolved findings here, complete independent safety and repository-rule reviews,
   and update [../../CHANGELOG.md](../../CHANGELOG.md). This package depends on all prior packages
   and is the completion gate.

## Test classification

Permanent fast tests are limited to behavior that is both durable and offline. The meaningful
controller modules remain separate because each owns a distinct contract:

- `tests/test_local_stack_control.py` covers normal target, environment, typed discovery, readiness,
  rootless-engine proof, and cleanup authority;
- `tests/test_local_stack_service.py` covers the narrowly allowed default-renderer outage;
- `tests/test_local_stack_consumer.py` covers the closed generic disposable adapter and capability
  binding;
- `tests/test_local_stack_chapter_one_consumer.py` covers the Chapter One browser owner's project
  and image boundary; and
- `tests/test_local_stack_replica_consumer.py` covers replica-specific stop and diagnostic-redaction
  constraints.

They inject a runner and inline all data. They do not run Podman, Compose, the lifecycle, a browser,
a network service, a clock, or a real shell script; they do not assert current container counts,
port numbers, service lists, command transcript text, source lines, fixture files, or tuned
timeouts.

Opt-in E2E/live acceptance proves the real engine, macOS machine boundary where applicable, Compose
labels, persistent-volume behavior, lifecycle reinitialization, and live browser ownership. It stays
outside `pytest tests/`. One-time implementation probes may inspect a retained or newly created
local stack while refining the controller, but are removed when their question is answered and are
recorded here only when the conclusion remains useful. The plan creates no fixture directory and no
regular test with a network or subprocess dependency.

## Exact Validation test suite

The final material tree must pass every applicable command below. A required skip, unavailable
service, or later material change leaves this workstream incomplete under
[../../TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md#validation-test-suite).

### Permanent and repository gates

```bash
source source_me.sh && python3 -m pytest \
  tests/test_local_stack_control.py \
  tests/test_local_stack_service.py \
  tests/test_local_stack_consumer.py \
  tests/test_local_stack_chapter_one_consumer.py \
  tests/test_local_stack_replica_consumer.py
source source_me.sh && python3 -m pytest tests/
./check_codebase.sh
./check_rust.sh
./run_playwright_tests.sh --build
git diff --check
git diff --cached --check
```

### Controller and live lifecycle acceptance

```bash
source source_me.sh && python3 local_stack.py --help
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py projects
source source_me.sh && python3 local_stack.py validate
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py reset --dry-run
```

On the explicitly resolved `containers` project, inspect the displayed target, then prove:

1. `stop` stops only the labelled default stack and its named data volumes remain observable.
2. A confirmed `reset --confirm-project containers` removes only the previewed default-project
   Compose resources, never a foreign/disposable target or image store.
3. `start --no-open` rebuilds only through the typed lifecycle and reaches its one-shot,
   health, PostgreSQL, MinIO, renderer, seed, and gateway readiness evidence.
4. `restart webwork-renderer` recreates that stateless service and the maintained WebWork live
   acceptance verifies a render/grade round trip.
5. The canonical walkthrough runs against its typed disposable target, cleans only that target,
   and leaves the developer default target untouched; the aggregate acceptance preflight rejects a
   conflicting labelled target without mutating it.
6. `source source_me.sh && python3 local_stack.py acceptance` completes every required no-skip
   lane after the lifecycle handoffs are centralized.

These are opt-in pre-production data-loss operations. Before each confirmed reset or owned
disposable cleanup, show the exact project/resources/argv and obtain the operator confirmation
required by the controller and Podman operating guidance.

## Validation evidence ledger

Record each command only after it runs against the final material tree. `PENDING` is not passing
evidence and does not satisfy this workstream's completion boundary.

| Gate | Command or observation | Status | Final-tree evidence / finding |
| --- | --- | --- | --- |
| Focused offline evidence | Injected controller/lifecycle/private-state/consumer/walkthrough tests plus typing, ASCII, source-line, and Markdown gates | PASS | Final material tree: focused offline evidence passed; no Podman, service, browser, or network action. |
| Full pytest | `python3 -m pytest tests/` | PASS | 4,881 collected and passed; 0 failed, skipped, or warned. |
| Repository build/lint | `./check_codebase.sh` and `./check_rust.sh` | PASS | Five Node/TypeScript checks and 260 Node tests passed; Rust format, check, strict Clippy, tests/doctests, fixtures, contracts, and Wasm check passed. |
| Browser suite | `./run_playwright_tests.sh --build` with no required skip | PASS | 202 ordinary mock-preview tests passed; 0 failed or skipped. Dedicated live lanes remain explicit aggregate inputs. |
| Diff hygiene | `git diff --check` and `git diff --cached --check` | PASS | Both passed on the final material tree; no Python bytecode cache or compiled bytecode was found. |
| Read-only controller | help, doctor, projects, validate, status, reset dry run | PASS | Default typed commands completed without creating labelled resources; `validate` remained read-only and reset dry-run refused an empty target rather than mutating it. |
| Retain/reset/rebuild | default start/status/validate, renderer stop/restart, full WebWork RPC | PASS | Typed default lifecycle reached semantic readiness; restart recovers an exited selected renderer while refusing unrelated unhealthy state, reproves OCI provenance, and reruns the semantic render/grade probe. |
| Renderer outage | `restart webwork-renderer` and real render/grade evidence | PASS | The selected renderer name remains the rebuild target; bare Podman IDs are normalized to the recorded OCI configuration ID, and the real render/grade RPC path passed after restart. |
| Disposable ownership | canonical walkthrough and replica/restart | PASS | Schema-v2 walkthrough completed visible J11-J13/J1-J5/J8 with private report ownership. Replica/restart preserved durable replay through its exact envelope and Question-ID manifest, with owner-scoped cleanup. |
| Aggregate acceptance | public controller completes no-skip lanes | PASS | All seven lanes passed: 202 ordinary, course visual, instructor visual, built walkthrough, Chapter publication, Chapter browser with four tests and Question-ID replacement, and WebWork RPC. Final default state: zero containers/networks and exactly `containers_ple_pgdata`, `containers_ple_miniodata`, and `containers_ple_identity_runtime`. |
| Independent reviews | Python/rules, Podman, walkthrough/acceptance | PASS | `final_python_repository_review.ae3`, `final_podman_security_review.c2`, and `walkthrough_acceptance_final_review.ae3` each returned ACCEPT on 2026-08-15 with no P0-P3 finding. |

### Required independent reviews

- A Python/repository-rules review confirms the package has typed boundaries, no hidden
  environment-control policy, no raw shell interpolation, no secret disclosure, and no test-only
  ownership logic in the neutral module.
- A Podman review confirms actual rootless engine/machine state, effective Compose provider,
  labels, ports, health, writable volumes, normal-stop retention, reset scope, and post-reset
  rebuild. It records the exact observed project/resources rather than relying on intended names.
- A walkthrough/acceptance review confirms shared centralization did not weaken the canonical
  runner's fail-closed conflict detection, private input boundary, visible-action checks, report
  redaction, or exact disposable cleanup contract.

## Acceptance criteria

- The documented controller is usable by a developer or Codex agent without remembering a private
  collection of `podman compose` forms, and ordinary inspection makes no state change.
- All default-stack mutation is label-resolved and project-scoped; reset is visibly acknowledged and
  never broadens into global Podman cleanup.
- The typed lifecycle remains the single init/migrate/seed/renderer/readiness owner, so one command cannot
  produce a subtly different teaching stack from another.
- Aggregate acceptance and the canonical walkthrough consume the same typed lifecycle machinery as
  the operator command while preserving each runner's distinct credentials, ports, report, and
  resource-ownership boundaries.
- Status/preflight is meaningful enough to gate live Playwright on every required long-running
  service being active and healthy and every one-shot service completing successfully.
- Documentation tells operators what a command changes, what data remains or is removed, how to
  inspect before change, and how to recover through the normal controller.
