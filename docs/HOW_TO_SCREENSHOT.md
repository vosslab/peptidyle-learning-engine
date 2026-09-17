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
- No other owned Live Demo running. The wrapper calls `./launchers/run_live_demo.sh stop`
  before it starts, so a stale demo is not a blocker.

## Rebuild the corpus

```bash
./devel/capture_screenshots.sh
```

What it does, in order:

1. Stops any owned Live Demo, then starts a fresh seeded one headlessly through
   `./launchers/run_live_demo.sh --headless`.
2. Resolves the local Sysadmin MFA authenticator path for the owned stack (path only; the
   seed never reaches Chromium or the terminal).
3. Runs `tests/playwright/capture_live_demo_screenshots.mjs --publish` against the reported
   `Live demo entry:` URL. Every scenario in
   `tests/playwright/screenshot_corpus/scenario_registry.ts` walks its normal visible
   workflow and captures each declared checkpoint.
4. Validates route, semantic-state, privacy, page-error, origin, and dimension invariants,
   then publishes: rewrites the four role folders, prunes PNGs the manifest no longer
   declares, regenerates `docs/screenshots/current_capture_receipt.json` and
   `docs/SCREENSHOT_ATLAS.md`.
5. Stops the stack and prints `Screenshot corpus complete`.

Expect roughly ten to twenty minutes. Run it in the background and tail the log:

```bash
./devel/capture_screenshots.sh > /tmp/capture_publish.log 2>&1 &
tail -f /tmp/capture_publish.log
```

Afterwards, `git status` should list only PNGs under `docs/screenshots/`, the receipt, and
the atlas. Open a sample of PNGs (one per role folder plus any new capture) and check for
blank pages, error banners, or overlapping chrome before recording the refresh in
`docs/CHANGELOG.md`.

## Verify without changing the corpus

```bash
./devel/capture_screenshots.sh --verify
```

Static checks first (manifest, registry, PNG set, receipt digest, dimensions, atlas), then a
clean stack replays the complete corpus with the same assertions. Replay images land under
`test-results/screenshot-corpus/verify/` (gitignored). Byte differences against the tracked
PNGs are reported for human review; they are not a pass/fail gate.

## Watch a capture run

```bash
./devel/capture_screenshots.sh --headed
```

Opens Chromium while the local CLI authenticator still completes Sysadmin MFA. For manual MFA
entry, see the screenshot section of [DEVELOPMENT.md](DEVELOPMENT.md).

## Add or change a capture

A capture needs three coordinated edits:

1. `docs/screenshots/current_capture_manifest.json`: add a record with `id`, `path`, `role`,
   `routeId`, `area`, `workflow`, `state`, `scenario`, `checkpoint`, `viewport`,
   `privacyProfile`, and `gallery`. Update the route and Ribbon coverage ledger rows that
   the capture now satisfies. Records hold identity and presentation only, never selectors.
2. `tests/playwright/screenshot_corpus/scenarios_<role>.ts`: inside the named scenario, reach
   the state through visible navigation, assert the state that proves it, then call
   `captureCheckpoint(runtime, scenario, "<checkpoint>", session)`. Add the checkpoint name
   to that scenario's `checkpoints` list.
3. Rebuild with `./devel/capture_screenshots.sh`. Validation refuses a manifest capture with
   no registered scenario checkpoint, and a registered checkpoint with no manifest capture.
   Publication prunes PNGs the manifest does not declare.

Removing a capture is the reverse: delete the manifest record, drop the checkpoint from the
scenario, and rebuild. The stale PNG is pruned on publish.

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
