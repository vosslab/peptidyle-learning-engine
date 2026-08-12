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
  runner clears inherited `PLE_*` variables. `COMPOSE_PROJECT_NAME` remains an
  ecosystem ownership check: it must be unset, empty, or `containers`.
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
  a build. Cleanup runs only for a stack started by this runner, uses the same
  selected Compose file, and never removes volumes. `--keep` retains that
  runner-owned stack for diagnosis.

## Current validation and acceptance

Permanent offline tests cover argument validation, selected-env-file port
selection, inherited-`PLE_*` isolation, private input metadata and schema
validation, report safety, and cleanup ownership. These protect stable
behavior; they are not a claim that the live teaching workflow has passed.

One-time WP-HG1 acceptance remains required: rebuild the stack, run the
canonical J13 then J1--J8 sequence with the local Podman WebWork renderer,
use visible clipboard copy/paste for all four Genetics Chapter 1 `P-n-vn`
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
