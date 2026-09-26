# How to refresh screenshots

Operational steps for rebuilding the canonical screenshot corpus under
`docs/screenshots/`. Policy (ownership, privacy, what counts as evidence) lives in
[SCREENSHOT_CONTRACT.md](SCREENSHOT_CONTRACT.md). The generated review surface is
[SCREENSHOT_ATLAS.md](SCREENSHOT_ATLAS.md).

## Prerequisites

- Podman machine running (`podman machine list` shows `Currently running`). See
  [MACOS_PODMAN.md](MACOS_PODMAN.md).
- Playwright browsers installed once: `./devel/setup_playwright.sh`. The capture wrapper
  runs this itself.
- Clean working tree so the publish diff is readable.

The normal path **reuses a running Live Demo**. Start one with
`./launchers/run_live_demo.sh --headless` if none is up. Use `--fresh` only when you want
the wrapper to stop, start, and afterwards stop an owned stack.

## Rebuild the corpus

```bash
./devel/capture_screenshots.sh
```

On a **warm** stack the driver decides what to rebuild from file timestamps, runs at most
one build, replaces application containers only when server crates changed, then captures
and publishes the full corpus. TypeScript, CSS, assets, and `crates/wasm` edits do not
rebuild images, replace containers, or reseed the database: Caddy bind-mounts host `dist/`.

| Newer than its output                                                    | Build                                                                        | Containers                                   |
| ------------------------------------------------------------------------ | ---------------------------------------------------------------------------- | -------------------------------------------- |
| `src/`, `assets/` newer than `dist/index.html`                           | `node pipeline/build.mjs --skip-wasm`                                        | none                                         |
| `crates/wasm/` newer than `dist/wasm/ple_bridge_bg.wasm`                 | `pipeline/build_wasm.sh --debug`, then `node pipeline/build.mjs --skip-wasm` | none                                         |
| other `crates/`, `Cargo.toml`, `Cargo.lock` newer than the api image     | `./build.sh --debug`                                                         | `python3 local_stack.py rebuild-application` |
| `schemas/`, `containers/`, `compose*.yaml` newer than the launch receipt | done by the restart                                                          | full stack restart                           |
| nothing newer                                                            | none                                                                         | none                                         |

`rebuild-application` rebuilds the shared api image and recreates api, worker, and
public-asset-publisher. PostgreSQL, MinIO, the renderer, and the gateway keep running.

A **cold** run (no stack, or `--fresh`) starts the Live Demo through
`./launchers/run_live_demo.sh --headless`, which already builds once, then captures.

After capture the driver hashes only `.png` files under the four role folders. Changed
images are listed (and copied under `test-results/screenshot-corpus/review/`); otherwise
it prints `no visual change` and exits 0.

`--verify`, `--headed`, `--fresh`, `--only=IDS`, and `--help` keep their current meanings.
`--build none|client|wasm_client|full` overrides the timestamp decision.

Expect a warm TypeScript loop to spend most of its time in Playwright, not in the
bundle. A cold start still takes on the order of ten to twenty minutes.

## Verify without changing the corpus

```bash
./devel/capture_screenshots.sh --verify
```

Static checks first (manifest, registry, PNG set, receipt digest, dimensions, atlas, folder
galleries, and README links), then a clean stack replays the complete corpus with the same
assertions. Replay images land under
`test-results/screenshot-corpus/verify/` (gitignored). Byte differences against the tracked
PNGs are reported for human review; they are not a pass/fail gate.

## Watch a capture run

```bash
./devel/capture_screenshots.sh --headed
```

Opens Chromium while the local CLI authenticator still completes Sysadmin MFA. For manual MFA
entry, see the screenshot section of [DEVELOPMENT.md](DEVELOPMENT.md).

## Add or change a capture

One hand-authored edit in one scenario file:

1. In `tests/playwright/screenshot_corpus/scenarios_<role>.ts`, add (or delete) a
   `captures` declaration and the matching `runtime.captureCheckpoint(session, checkpoint)`
   call after the page has reached the state through visible navigation. Capture id and
   path are `<role>_<checkpoint>` and `<role>/<checkpoint>.png`. The reached route is
   observed, not declared.
2. Rebuild with `./devel/capture_screenshots.sh`. Publish writes
   `docs/screenshots/current_capture_manifest.json`, the receipt, the atlas, and coverage
   ledgers, plus the eight generated pages under `docs/screenshot_galleries/`.
   `docs/screenshots/coverage_exceptions.json` is the only hand-kept coverage list;
   generation fails closed on an uncovered unlisted surface.

Capture IDs remain `<role>_<checkpoint>`. Screenshots reached through a Tier 1 and Tier 2
Ribbon task use `<tier1>-<tier2-alias>-<details>.png`: Tier 1 comes from the Ribbon catalog,
and the concise Tier 2 aliases are declared in `tests/playwright/screenshot_corpus/filenames.ts`.
Direct, non-tiered screens keep their checkpoint filename. Course theme comparisons use the
purpose-only `theme_sample-<theme>.png` pattern because those captures compare themes rather than
Ribbon destinations.

Removing a capture is the reverse: delete the declaration and call, then rebuild. The
stale PNG is pruned on publish.

Viewports are fixed: laptop 1280x800, tablet 800x1280, phone 393x852, square 800x800. Add a
non-laptop variant only when the responsive composition changes materially.

## When a capture fails

- The runner names the scenario and checkpoint. Diagnostics are under
  `test-results/screenshot-corpus/`.
- Reproduce on the live stack: `./launchers/run_live_demo.sh --open`, sign in as the seeded
  role, and walk the same workflow by hand.
- Fix the application boundary or the scenario navigation. Do not add screenshot-only routes,
  mocked responses, or fabricated state, and do not remove the manifest record to make the
  run pass.
- A failed publish rolls back to the previous corpus. If a publish was interrupted, the
  recovery backup is left in place and the next run refuses to overwrite it until a human
  inspects it.
- `./launchers/run_live_demo.sh stop` cleans up an owned stack the wrapper could not stop.
