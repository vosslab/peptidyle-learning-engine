# Install

For a developer checkout, installation means starting the real, disposable PLE live demo. The
primary command prepares missing JavaScript dependencies, builds the production browser artifact,
and starts the production-shaped HTTPS stack with real PostgreSQL, MinIO, API, worker, gateway,
and private WebWork renderer services.

## Requirements

- Git and Bash.
- Current Node.js and npm. The first launch installs the dependencies locked in `package-lock.json`.
- Current stable Rust through `rustup`; [rust-toolchain.toml](../rust-toolchain.toml) selects
  `rustfmt`, Clippy, and `wasm32-unknown-unknown`.
- Python 3.12 available as `python3`. The live-demo wrapper sources the repository shell
  environment and invokes the fixed local-stack controller with that interpreter.
- Podman and a usable Compose provider for the local stack. On macOS, also start a Podman machine;
  see [MACOS_PODMAN.md](MACOS_PODMAN.md).
- `curl`, `awk`, `openssl`, `xxd`, and `lsof`, which the typed stack lifecycle uses.

## Fresh-clone success

Clone the repository and run its one supported developer front door:

```bash
git clone https://github.com/vosslab/peptidyle-learning-engine.git
cd peptidyle-learning-engine
./run_live_demo.sh
```

`./run_live_demo.sh` is the supported fresh-clone front door. It sources the repository shell
environment through its fixed `source_me.sh` path and invokes `python3 local_stack.py`. When
`node_modules` is absent, it also visibly runs `devel/setup_typescript.sh`; it then builds the
production `dist/` bundle, creates the disposable
`ple-live-demo-browser` HTTPS session, waits for readiness, and opens the printed origin.
Select a seeded role in the visible PLE sign-in flow; its account, course membership, and
authorization come from ordinary seeded PLE state.

Each launch first completes owner-scoped cleanup of the previous `ple-live-demo-browser` session,
then creates a fresh seeded installation. Relaunching therefore discards records created in the
previous disposable demo while leaving unrelated Podman projects untouched. The stack is
production-shaped; this is not a browser mock or a separate WebWork2 application.

Use the non-opening form when a browser is unavailable:

```bash
./run_live_demo.sh --headless
```

It starts the same stack and prints the HTTPS origin. Stop the session through its owner when you
finish:

```bash
./run_live_demo.sh stop
```

## Developer tools

The same repo-local Python environment supports the controller and repository checks. Refresh it
without activating an environment when you need the developer tools:

```bash
./devel/setup_python.sh
```

The repository toolchain and Cargo lockfile provide the Rust dependencies. Keep the developer live
demo on its fixed Compose project and runtime identity so its owner-scoped lifecycle remains valid.

## Browser test setup

Install the Chromium and Firefox browsers used by the Playwright lanes after the JavaScript
dependencies are present:

```bash
./devel/setup_playwright.sh
```

The script requires `node_modules`; run `./devel/setup_typescript.sh` first when starting from a
checkout that has not yet run `./run_live_demo.sh`. Browser installation is optional for the
headless live-demo start and for offline Rust, TypeScript, and Python checks.

## Seeded accounts

After the browser opens, use the visible **Explore this live demo** panel on the PLE sign-in page:

- Choose the seeded Instructor, Student, or Sysadmin account, then choose one of that account's
  authorized courses. Current acceptance personas include Elena (Instructor), Mary, Jack, and
  Avery (Students), and Morgan (Sysadmin).
- Role selection only replaces the identity-verification ceremony. The server still resolves the
  ordinary account, session, course membership, role, and authorization state.
- The seeded data belongs to this disposable installation. Relaunching the demo restores the
  baseline and discards changes from the prior session.
- Email-code and passkey adapters are not mounted in this build. Their private schema and
  single-session credential contracts are present; the visible selector is the current demo entry.

## Verify install

From a fresh checkout, use the non-opening launch as the installation verification:

```bash
./run_live_demo.sh --headless
./run_live_demo.sh stop
```

The first command must print a ready HTTPS origin; the second must confirm owner-scoped cleanup.
For an offline cross-language verification after installing the developer tools, run:

```bash
./check_rust.sh
./check_codebase.sh
source source_me.sh && .venv/bin/python -m pytest tests/
```

Run `./check_rust.sh` before `./check_codebase.sh`: it generates the ignored TypeScript API and
fixture projections consumed by the codebase gate. See [DEVELOPMENT.md](DEVELOPMENT.md) for focused
gates and [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the complete Validation suite.

## Troubleshooting

When a lifecycle or cleanup fails, follow [TROUBLESHOOTING.md](TROUBLESHOOTING.md) and
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for the fixed-owner contract. The controller
has read-only diagnostics; [USAGE.md](USAGE.md) lists the supported commands.

## Known gaps

- `WP-INST-G1` is accepted. Its forward closeout migrations `2026081866` through `2026081869`
  provide the clean-volume receipt preflight, receipt writers, commit-v2 authority, and retry V2
  retirement boundary. Final material-tree Validation passed with the affected 99-migration live
  database, RLS, worker, browser, WebWork, and replica evidence. `WP-INST-G2` now owns audited
  student-work inspection and grade-scheme-aware calculated Gradebook work. `WP-RC12` release
  acceptance remains open; this disposable live demo is not release evidence by itself.
- TODO: Verify PG/PGML compatibility beyond the reviewed Chapter 1 MC/MATCH sources with separate
  source and live evidence.
