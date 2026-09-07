# Install

For a developer checkout, installation means starting the real, disposable PLE session-entry demo.
The primary command prepares missing JavaScript dependencies, builds the production browser artifact,
and starts the production-shaped HTTPS stack with PostgreSQL, MinIO, API, gateway, and a private
WeBWorK renderer. It establishes only the current seeded session-entry boundary.

## Requirements

- Git and Bash.
- Node.js and npm. The first launch installs the dependencies declared by `package.json`.
- Current stable Rust through `rustup`; [rust-toolchain.toml](../rust-toolchain.toml) selects
  `rustfmt`, Clippy, and `wasm32-unknown-unknown`.
- Homebrew on macOS. `brew bundle` installs the system-wide Python interpreter and Podman
  declared by [Brewfile](../Brewfile).
- Python available as `python3`, with the runtime dependencies from
  [pip_requirements.txt](../pip_requirements.txt) installed.
- Podman and a usable Compose provider for the local stack. On macOS, also start a Podman machine;
  see [MACOS_PODMAN.md](MACOS_PODMAN.md).
- `curl`, `awk`, `openssl`, `xxd`, and `lsof`, which the typed stack lifecycle uses.

## Fresh-clone success

Clone the repository and run its one supported developer front door:

```bash
git clone https://github.com/vosslab/peptidyle-learning-engine.git
cd peptidyle-learning-engine
brew bundle
source source_me.sh && python3 -m pip install --requirement pip_requirements.txt
./launchers/run_live_demo.sh
```

`./launchers/run_live_demo.sh` is the supported live-demo front door. It resolves the checkout
through Git, sources the repository `source_me.sh`, and invokes `python3 local_stack.py`. When
starting, it runs `devel/setup_typescript.sh`; that helper owns TypeScript dependency setup. Cargo
restores its checkout-local build artifacts during the build. The launcher then builds the
production `dist/` bundle and creates the
disposable `ple-live-demo-browser` HTTPS session, and prints its ready origin. Open that URL in your
browser, or run `./launchers/run_live_demo.sh open` to open an already-running demo automatically. Use
`./launchers/run_live_demo.sh start --open` to create a fresh demo and open it. Select a seeded persona in the
visible PLE sign-in flow. The server derives its Account and ordinary
Authenticated Session from disposable seeded state, then permits the
role- and relationship-gated product routes supported by that session. The
browser can download protected invitation-export input but cannot send mail.
M19 recorded fresh sealed-stack production-browser proof on 2026-09-07.

Each launch first completes owner-scoped cleanup of the previous `ple-live-demo-browser` session,
then creates a fresh seeded installation. Relaunching therefore discards records created in the
previous disposable demo while leaving unrelated Podman projects untouched. The stack is
production-shaped; this is not a browser mock or a separate WebWork2 application.

Use the non-opening form when a browser is unavailable:

```bash
./launchers/run_live_demo.sh --headless
```

It starts the same stack and prints the HTTPS origin. Stop the session through its owner when you
finish:

```bash
./launchers/run_live_demo.sh stop
```

## Developer tools

Install or refresh the declared runtime and developer dependencies for the selected `python3`
before running developer tools or tests:

```bash
source source_me.sh && python3 -m pip install --requirement pip_requirements.txt --requirement pip_requirements-dev.txt
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
checkout that has not yet run `./launchers/run_live_demo.sh`. Browser installation is optional for the
headless live-demo start and for offline Rust, TypeScript, and Python checks.

## Seeded accounts

After the browser opens, use the visible **Explore this live demo** panel on the PLE sign-in page:

- Choose the seeded Instructor, Student, or Sysadmin persona. Current personas are Elena
  (Instructor), Mary, Jack, and Avery (Students), and Morgan (Sysadmin).
- The resulting session permits only its role- and relationship-gated product routes. The
  browser can download protected invitation-export input without receiving permission to send
  mail.
- Persona selection only replaces the identity-verification ceremony. The server still resolves the
  ordinary Account and session; course and authorization decisions remain server-derived.
- The seeded data belongs to this disposable installation. Relaunching the demo restores the
  baseline and discards changes from the prior session.
- Email-code authentication remains future work. The passkey capability is deferred: this build has
  no passkey configuration, setup credential, installation command, Server Route, or Browser Surface.
  Seeded entry is the current local-demo identity-verification path.

## Verify install

From a fresh checkout, use the non-opening launch as the installation verification:

```bash
./launchers/run_live_demo.sh --headless
./launchers/run_live_demo.sh stop
```

The first command must print a ready HTTPS origin; the second must confirm owner-scoped cleanup.
This proves the named disposable deployment and session-entry boundary, not a browser teaching journey.
For an offline cross-language verification after installing the developer tools, run:

```bash
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
```

Run `./check_rust.sh` before `./check_codebase.sh`: it generates the ignored TypeScript API and
fixture projections consumed by the codebase gate. See [DEVELOPMENT.md](DEVELOPMENT.md) for focused
gates and [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the complete Validation suite.

## Troubleshooting

When a lifecycle or cleanup fails, follow [TROUBLESHOOTING.md](TROUBLESHOOTING.md) and
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for the fixed-owner contract. The controller
has read-only diagnostics; [USAGE.md](USAGE.md) lists the supported commands.

## Known gaps

- The local demo proves its named deployment and session boundaries only. It does not provide a
  visible Course, Question Library, authoring, delivery, grading, Gradebook, or administration
  journey, and it is not release evidence. See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) and
  [ROADMAP.md](ROADMAP.md).
- TODO: Verify PG/PGML compatibility beyond the reviewed Chapter 1 MC/MATCH sources with separate
  source and live evidence.
