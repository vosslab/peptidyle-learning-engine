# UI walkthrough tests

This directory owns the opt-in, real-stack instructor-to-student walkthrough.
It is independent of the generic E2E runners under `tests/e2e/`.

Run the canonical entry point from the repository root:

```bash
bash tests/walkthrough/run_ui_walkthrough.sh --master-seed 42
```

`walklib/` contains importable Python orchestration. `children/` contains fixed
TypeScript subprocesses used for corpus arrangement, cross-actor binding, and
redacted report rendering. Browser journeys remain under `tests/playwright/`
because the repository requires every Playwright import to live there.

The walkthrough owns its random project, private inputs, fixed-port checks,
visible actions, and redacted report. It delegates Compose-provider choice,
sanitized child environment, label discovery, target construction, and exact
cleanup to `local_stack_control/`. Its private state, environment file, and
runner-held cleanup capability are required to form a disposable target;
neither a project-like name nor a caller-supplied cleanup flag grants
authority. A cleanup failure is a failed walkthrough and preserves its private
receipt/evidence for the owner to inspect rather than hiding the target.

The old `tests/e2e/e2e_ui_walkthrough.sh` and Python path are compatibility
entry points and delegate here. They do not own implementation.

## Documentation screenshots

Refresh the complete instructor and student stage set through the same
real-stack walkthrough:

```bash
node tests/playwright/capture_docs_screenshots.mjs
```

The default is AUTO: reuse safe existing `dist/` outputs and build only when
they are missing. Pass `--build` to force a fresh bundle. The capture owns and
removes its private temporary directory, requires the full cleanup-enabled
walkthrough, and atomically installs only its approved stage PNG files under
`docs/screenshots/`.

These fake-user screenshots are required walkthrough evidence, not a privacy exception. The
capture still excludes login credentials, answer material, traces, and raw child output.

## Test value boundary

Permanent tests protect user-visible browser behavior, strict parsers,
redaction, filesystem safety, cleanup ownership, and failure containment.
They remain offline and deterministic unless they are explicit Playwright or
walkthrough E2E tests.

Implementation checks are disposable. Exact source text, child paths, argv
lists, help snapshots, timeout constants, collection sizes, and migration-only
wiring probes may be run while changing the runner, but do not remain in the
regular suite. The checklist in [PYTEST_STYLE.md](../../docs/PYTEST_STYLE.md)
decides which checks are permanent; when a check is doubtful, remove it.

Do not use `python3 -m py_compile` as a walkthrough gate. It explicitly writes
`.pyc` files even when `PYTHONDONTWRITEBYTECODE=1` is exported. Focused imports,
pytest, and Pyflakes provide the retained Python checks without cache artifacts.
