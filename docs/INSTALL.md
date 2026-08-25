# Install

Installation prepares the Rust server and WebAssembly code, Solid browser client, Python test
tools, and local Podman stack. The normal local stack serves native questions and the separate,
stateless `webwork-pg-renderer` engine; it does not install the WebWork2 assignment platform.

## Requirements

- Git, current stable Rust from `rustup`, and the `wasm32-unknown-unknown` target selected by
  [rust-toolchain.toml](../rust-toolchain.toml).
- Current Node.js and npm for the Solid client and generated TypeScript artifacts.
- Python 3.12 with the packages in [pip_requirements-dev.txt](../pip_requirements-dev.txt) for the
  repository test suite.
- Podman plus a usable Compose provider for the all-in-one local stack. macOS users also need a
  Podman machine. The typed lifecycle also requires `curl`, `awk`, `openssl`, `xxd`, and `lsof`; see
  [MACOS_PODMAN.md](MACOS_PODMAN.md).

## Repository setup

```bash
git clone https://github.com/vosslab/peptidyle-learning-engine.git
cd peptidyle-learning-engine
npm run setup
```

`npm run setup` runs `npm install` for the JavaScript dependencies declared in `package.json`.
Install the Python developer tools separately when running the Python checks:

```bash
python3 -m pip install -r pip_requirements-dev.txt
```

Rust uses the repository toolchain file and Cargo lockfile.

## Verify install

Confirm that the repository environment can load the primary local-stack command:

```bash
source source_me.sh && python3 local_stack.py --help
```

Then verify the cross-language build gate and the separate fast documentation and repository-hygiene
suite:

```bash
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest -q tests/
```

Run `./check_rust.sh` first: it generates the ignored TypeScript API and fixture projections that
`./check_codebase.sh` consumes. Success is exit status zero from all three commands.
`./check_rust.sh` is the repository-owned Cargo and Rust gate, `./check_codebase.sh` is the vendored
TypeScript and browser gate, and `pytest tests/` remains the separate fast documentation and hygiene
lane. For a built browser artifact without starting containers, run `./build.sh`; it creates `dist/`
and `dist_wasm/`.

For choosing a focused development gate, use [DEVELOPMENT.md](DEVELOPMENT.md). The durable API,
storage, tenancy, and server-only-grading boundaries are recorded in [CONTRACTS.md](CONTRACTS.md)
and [SECURITY_MODEL.md](SECURITY_MODEL.md), rather than inferred from local lifecycle behavior.

## Local stack setup

The developer session starts PostgreSQL, MinIO, API, worker, gateway, and the
standalone PG renderer in one fixed disposable owner. It builds the production
browser bundle and serves it through the owner-locked HTTPS gateway. The
renderer image and other pinned inputs are selected by the canonical browser
owner; do not create a second local Compose project or supply alternate
environment, identity, or build options.

```bash
source source_me.sh && python3 local_stack.py start
source source_me.sh && python3 local_stack.py start --no-open
```

The first command opens the canonical HTTPS origin. The second starts the same
session without opening a browser and prints the origin. In the visible
production-auth UI, choose one of the five seeded personas and then choose an
authorized course. The server creates the ordinary account and tenant sessions
and derives roles from stored PLE state; no alternate auth switch is part of
this entry point. For the complete
stack and lease/cleanup contract, see
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) and
[LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md).

## Production baseline is separate

The OpenTofu configuration under `deploy/opentofu/` is not installed or applied
by `npm run setup` or the typed local-stack lifecycle. It defines private no-NAT ECS
tasks, RDS, four SSE-KMS S3 domains, CloudFront/ALB TLS-origin admission, and
separate API, worker, and public-asset-publisher roles and secrets. A live AWS
deployment still requires operator-owned DNS, certificates, Secrets Manager
values, exact database-role provisioning, and disposable-account probes. The
external WeBWorK renderer feature remains disabled there until it is separately
attested for private ingress, immutable image provenance, TLS identity, and no
database or object-store authority. See
[MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md#production-baseline-in-opentofu).

## Known gaps

- PG/PGML compatibility beyond the four reviewed Chapter 1 MC/MATCH sources needs its own source
  and live evidence.
