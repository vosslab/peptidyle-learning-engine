# Install

For a developer checkout, installation means starting the real, disposable PLE live demo. The
primary command prepares missing JavaScript dependencies, builds the production browser artifact,
and starts the production-shaped HTTPS stack.

## Requirements

- Git and Bash.
- Current Node.js and npm. The first launch installs the dependencies locked in `package-lock.json`.
- Current stable Rust through `rustup`; [rust-toolchain.toml](../rust-toolchain.toml) selects
  `rustfmt`, Clippy, and `wasm32-unknown-unknown`.
- Python 3.12. Install [pip_requirements-dev.txt](../pip_requirements-dev.txt) when running the
  repository's Python-based developer checks.
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

When `node_modules` is absent, `run_live_demo.sh` visibly runs
`devel/setup_typescript.sh` before it starts the local-stack owner. That owner builds the production
`dist/` bundle, creates the disposable `ple-live-demo-browser` HTTPS session, waits for readiness,
and opens the printed origin. Select a seeded role in the visible PLE sign-in flow; its account,
course membership, and authorization come from ordinary seeded PLE state.

Each launch first completes owner-scoped cleanup of the previous `ple-live-demo-browser` session,
then creates a fresh seeded installation. Relaunching therefore discards records created in the
previous disposable demo while leaving unrelated Podman projects untouched.

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

Install the Python tooling only for repository checks:

```bash
python3 -m pip install -r pip_requirements-dev.txt
```

The repository toolchain and Cargo lockfile provide the Rust dependencies. Keep the developer live
demo on its fixed Compose project and runtime identity so its owner-scoped lifecycle remains valid.

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
source source_me.sh && pytest tests/
```

Run `./check_rust.sh` before `./check_codebase.sh`: it generates the ignored TypeScript API and
fixture projections consumed by the codebase gate. See [DEVELOPMENT.md](DEVELOPMENT.md) for focused
gates and [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the complete Validation suite.

## Troubleshooting

When a lifecycle or cleanup fails, follow [TROUBLESHOOTING.md](TROUBLESHOOTING.md) and
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for the fixed-owner contract. The controller
has read-only diagnostics; [USAGE.md](USAGE.md) lists the supported commands.

## Known gaps

- Verify PG/PGML compatibility beyond the reviewed Chapter 1 MC/MATCH sources with separate source
  and live evidence.
