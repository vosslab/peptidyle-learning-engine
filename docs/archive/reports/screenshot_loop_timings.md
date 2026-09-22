# Screenshot loop timings

Measured 2026-09-20 on this checkout. Commands are the ones this machine ran.

## Component commands

| Item | Command | Seconds | Notes |
| --- | --- | --- | --- |
| Client bundle, missing wasm | `node pipeline/build.mjs --skip-wasm` | 2.8 | fail: `dist_wasm/web` was absent |
| Client bundle, warm | `node pipeline/build.mjs --skip-wasm` | 2.7 | after wasm existed; no container work |
| Client bundle after `touch src/style.css` | `node pipeline/build.mjs --skip-wasm` | 2.7 | content-unchanged mtime bump |
| WASM debug bridge, first compile | `pipeline/build_wasm.sh --debug` | 10.2 | crates/wasm and its crates |
| WASM debug bridge, incremental | `pipeline/build_wasm.sh --debug` | 3.6 | already built |
| Client after incremental WASM | `node pipeline/build.mjs --skip-wasm` | 2.6 | copies debug bridge into `dist/wasm/` |
| WASM then client, incremental | `pipeline/build_wasm.sh --debug` then skip-wasm | 6.2 | 3.6 + 2.6 |
| Full host build, incremental | `./build.sh --debug` | 9.8 | includes workspace Cargo, tsgen, fixtures |
| Full host build, missing catalog | `./build.sh --debug` | 0.2 | fail: `provided_avatar_catalog.sql` absent |
| Live Demo cold start, missing catalog | `./launchers/run_live_demo.sh --headless` | 6.3 | fail: host artifact build |
| Live Demo cold start, SQL duplicate | `./launchers/run_live_demo.sh --headless` | 369.0 | fail: `course_instance_id` declared twice |
| Live Demo cold start after SQL fix | `./launchers/run_live_demo.sh --headless` | 514.4 | fail: gateway 503; worker `question_id` missing |
| Live Demo warm reuse | `./launchers/run_live_demo.sh --headless` | unmeasured | no ready stack |
| Full capture-and-publish | `./devel/capture_screenshots.sh --build none` | unmeasured | needs a running Live Demo |

`node pipeline/build.mjs` without `--skip-wasm` rebuilt the WASM bridge in release (11.8s cargo plus bindgen) even after `pipeline/build_wasm.sh --debug`. The shipped `wasm_client` row therefore runs `--skip-wasm` on the client step so the debug bridge is copied, not replaced.

Three end-to-end warm loops to a finished screenshot were not timed: Live Demo did not become ready in this environment. Component timings and the written decision below still apply.

## Decision

- **Keep `wasm_client`.** Incremental `pipeline/build_wasm.sh --debug` plus
  `node pipeline/build.mjs --skip-wasm` measured 6.2s versus incremental
  `./build.sh --debug` at 9.8s. The wasm row does not compile the api crate
  and does not rebuild the application image. Folding it into `full` would
  force an application image rebuild for a bind-mounted browser artifact.
- **Do not stop before the driver.** The warm TypeScript bundle is 2.7s.
  Capture was not timed here because Live Demo never became ready. Capture
  is still expected to dominate that 2.7s, but the expensive step the user
  reported is bringing the stack up (observed 369s and 514s failed cold
  starts that included image work). The warm driver removes image builds,
  container replacement, and database reseed for TypeScript, CSS, and WASM
  edits. That saving is independent of Playwright duration.
- Later milestones follow this table: `client` / `wasm_client` replace no
  containers; other crate edits use `./build.sh --debug` plus
  `rebuild-application`; schema/Compose edits restart the stack.

## Component timing log

Rows come from `tests/_temp/measure_screenshot_loop.sh` and follow-up
passes after `python3 devel/generate_avatar_catalog.py` wrote the missing
`schemas/base_schema/provided_avatar_catalog.sql` and after removing the
duplicate `course_instance_id` output column from
`lock_archived_course_for_recovery`.
