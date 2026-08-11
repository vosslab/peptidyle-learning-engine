# WP-O1 walkthrough runner independent review

## Verdict

ACCEPTED for WP-O1's runner-only boundary.

The runner accepts its narrow contract, validates all external input before
creating its report directory or starting the launcher, and has no browser or
product behavior. A live PASS remains deliberately unclaimed until WP-O2
provides its smoke spec and validated configuration.

## Findings

No remaining WP-O1 findings.

The final ownership check uses `podman ps --all --quiet`, whose local help
explicitly documents both flags. It queries stopped and running containers by
both exact project labels: the launcher's authoritative
`io.podman.compose.project=containers` label and the Docker-compatible
`com.docker.compose.project=containers` label. `COMPOSE_PROJECT_NAME` is
validated with inherited-over-env-file precedence and refused unless empty or
`containers`, so that fixed-label ownership and the selected Compose cleanup
refer to the same project. The runner repeats that empty-project check after
the launcher's read-only check and before it accepts cleanup ownership.

The port calculation at `tests/e2e/e2e_ui_walkthrough.sh:139-150` matches
`launch_local_stack.sh:547-564`: selected env file, then 3000 default, with a
nonempty inherited `PLE_GATEWAY_HOST_PORT` taking precedence. This holds both
before and after the launcher may rewrite the selected env file.

## Validation

Passed without launching Podman or a browser:

```bash
bash -n tests/e2e/e2e_ui_walkthrough.sh
bash tests/e2e/e2e_ui_walkthrough.sh --help
PLE_UI_WALKTHROUGH_MASTER_SEED='' bash tests/e2e/e2e_ui_walkthrough.sh --skip-build
PLE_UI_WALKTHROUGH_MASTER_SEED=7 PLE_UI_WALKTHROUGH_REPORT_FILE='../escape.json' \
  bash tests/e2e/e2e_ui_walkthrough.sh --skip-build
PLE_UI_WALKTHROUGH_MASTER_SEED=7 PLE_UI_WALKTHROUGH_ENV_FILE=/dev/null \
  bash tests/e2e/e2e_ui_walkthrough.sh --skip-build
PLE_UI_WALKTHROUGH_MASTER_SEED=7 COMPOSE_PROJECT_NAME=not_containers \
  bash tests/e2e/e2e_ui_walkthrough.sh --skip-build
source source_me.sh && python3 tests/check_ascii_compliance.py \
  -i tests/e2e/e2e_ui_walkthrough.sh
source source_me.sh && python3 tests/check_ascii_compliance.py \
  -i docs/active_plans/workstreams/wp_o1_walkthrough_runner.md
git diff --check -- tests/e2e/e2e_ui_walkthrough.sh \
  docs/active_plans/workstreams/wp_o1_walkthrough_runner.md
```

Each of the four invalid-input invocations exited 1 and no
`test-results/ui_walkthrough/` directory was created. The review intentionally
did not run a live stack: the WP-O2 smoke spec/configuration is not present,
so a live PASS would be unsupported.

The direct local `podman ps --help` check documented `--all`, `--quiet`, and
label filters. The Podman machine remains stopped, so this review makes no
claim about current container state and did not launch a stack or browser.
