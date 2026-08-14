# WP-O1 walkthrough runner

## Current disposition

The 2026-08-11 runner acceptance is retained as historical evidence for the
earlier gateway/browser smoke. It predates the strengthened human-reference
workflow and the explicit-input refactor, so it does not accept the current
walkthrough. [WP-HG1 in the active release plan](../active/release_completion_plan.md#wp-hg1-close-the-human-guidance-operational-workflow-gaps)
owns the rebuilt live acceptance.

The canonical entrypoint is now:

```bash
bash tests/walkthrough/run_ui_walkthrough.sh --master-seed 42 --build
```

`tests/e2e/e2e_ui_walkthrough.sh` remains a directly runnable compatibility
facade. Its earlier command and PASS report document the old smoke boundary;
they do not prove the current four-reference instructor construction or
student journey.

## Current contract

- The thin shell entrypoint sources `source_me.sh` and invokes the typed Python
  argparse CLI. Its public choices are `--master-seed UINT32`, `--env-file
PATH`, `--report-file BASENAME`, `--screenshot-directory PATH`, `--keep`,
  `--build`, `--instructor-setup-only`, and `--student-repeat-only`.
- `--env-file` is the selected Compose configuration. The runner derives the
  loopback gateway port only from that selected file, defaulting to `8080`; an
  inherited `PLE_GATEWAY_HOST_PORT` cannot change the selected run.

- Before every runner-owned launcher, Compose, Node, and Playwright child, the
  runner clears inherited `PLE_*` variables. The runner rejects an inherited or
  selected-file `COMPOSE_PROJECT_NAME`, then generates one bounded disposable
  project name. It passes that standard Compose variable only to its launcher
  and cleanup children; Node and Playwright never receive it.
- Before launch and again immediately before start, the runner fails closed on
  any container, volume, or network bearing either Podman Compose or Docker
  Compose project label for its generated name. This makes volume cleanup safe
  even after an interrupted disposable run.
- The fixed-port clean-stack acceptance is exclusive: it refuses any existing
  default `containers` stack, another `ple-ui-walkthrough-*` stack, or a
  listener on PostgreSQL, MinIO API/console, or the selected gateway port. It
  does not treat preserved inactive volumes as a conflict.
- The runner copies the selected env file into its mode-0700 private state as
  a mode-0600 `compose.env`, appends its generated `PLE_APPLICATION_IMAGE`, and
  passes that file to launcher check/start and cleanup. The application image
  is interpolated in Compose, so API and worker share one project-scoped build
  rather than mutating `containers/env.local` or reusing another stack's tag.
- The runner alone passes `--canonical-walkthrough` to the launcher. That
  explicit launcher mode puts local identities, the fake-user login, invitation
  secret, renderer provenance, and Chapter One publication manifests beside the
  private env file. Ordinary launcher use does not enable that mode; env-path
  equality is not a walkthrough behavior switch.
- The runner creates one mode-0700 private state directory and atomically
  writes a mode-0600, ASCII, schema-versioned `walkthrough-inputs.json` for
  each fixed child stage. Node children receive its path through the explicit
  `--inputs PATH` argument. No child receives walkthrough choices through
  private environment variables.
- The runner writes a private, generated Playwright config that imports
  `ui_walkthrough_config_factory.ts`. It invokes the ordinary Playwright
  `--config PATH` argument, so Playwright reads the same validated private
  input boundary before Chromium starts.
- The fixed stages are arrangement, visible instructor setup, learner journey,
  and redacted report rendering. They exchange only bounded public references,
  local credentials by private path, and runner-owned checkpoint paths; answer
  material and raw child output never enter the report.
- AUTO reuses only safe `dist/index.html` and `dist/main.js`; `--build` forces
  a build. Cleanup runs only for a stack started by this runner, uses its same
  generated Compose project and selected Compose file, and removes that
  project's disposable volumes with `down --volumes --remove-orphans`. The
  normal `containers` project and its volumes are never selected. `--keep`
  retains that runner-owned stack and volumes for diagnosis, then prints the
  generated project name and an exact read-only Compose `ps` command.
- After a successful non-keep `down --volumes`, cleanup removes exactly the
  generated application tag and Podman Compose's generated gateway tag. It
  never uses an image prune or broad image removal. It first checks each exact
  tag, so a partial launcher build that never produced one is already clean;
  an inspection error or removal failure makes the report and exit status fail.

## Current validation and acceptance

Permanent offline tests cover argument validation, selected-env-file port
selection, inherited-`PLE_*` isolation, private input metadata and schema
validation, report safety, and cleanup ownership. These protect stable
behavior; they are not a claim that the live teaching workflow has passed.

One-time WP-HG1 acceptance remains required: rebuild the stack, run the
canonical J13 then J1--J8 sequence with the local Podman WebWork renderer,
use visible assignment reuse or clipboard copy/paste for the Genetics Chapter 1 `AAA-BBBB` Question IDs
references, preserve a redacted report, refresh the instructor screenshots,
and obtain the independent review specified by the active release plan.

## Historical evidence

The earlier elevated command below completed with exit 0, opened the public
IPv4 `/health` origin, required HTTP 200 and exact `{"status":"ready"}`, and
left no runner-owned containers after cleanup:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42
```

It is evidence for the prior WP-O1 gateway smoke only. The historical reviews
remain available as [WP-O1 review](../audits/wp_o1_python_runner_review.md)
and [WP-O2 review](../audits/wp_o2_live_playwright_review.md).
