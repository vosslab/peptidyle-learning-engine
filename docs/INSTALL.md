# Install

Installation prepares the Rust server and WebAssembly code, Solid browser client, Python test
tools, and optional local Podman stack. The standard development path is native PLE questions;
private upstream WeBWorK is an explicit, source-pinned opt-in profile.

## Requirements

- Git, current stable Rust from `rustup`, and the `wasm32-unknown-unknown` target selected by
  [rust-toolchain.toml](../rust-toolchain.toml).
- Current Node.js and npm for the Solid client and generated TypeScript artifacts.
- Python 3.12 with the packages in [pip_requirements-dev.txt](../pip_requirements-dev.txt) for the
  repository test suite.
- Podman plus a usable Compose provider for the all-in-one local stack. macOS users also need a
  Podman machine. The launcher also requires `curl`, `awk`, `openssl`, `xxd`, and `lsof`; see
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

Run the complete repository gate before changing code:

```bash
./check_codebase.sh
```

Success is an all-PASS summary. For a built browser artifact without starting containers, run
`./build.sh`; it creates `dist/` and `dist_wasm/`.

## Local stack setup

The native default starts PostgreSQL, MinIO, API, worker, and gateway. On its first normal run,
the launcher creates ignored local configuration, credentials, and secrets beneath `containers/`;
it does not require copied development secrets.

```bash
./launch_local_stack.sh --check
./launch_local_stack.sh --no-open
```

`--check` reads the existing environment and validates Compose without building, starting a Podman
machine, creating local files, or changing containers. The normal command creates a mode-0600
environment file, credentials, and secrets (with a non-secret identity-hash file), builds the
repository, migrates and seeds the local database, starts the stack, and prints the loopback URL
and local sign-in file.

## Custom environment files

The bootstrap belongs only to the default `containers/env.local` path. A non-default
`--env-file` is never created, rewritten, seeded, or supplied with local sign-in credentials;
`--check` creates neither default nor custom files. Prepare a custom file from
[containers/env.example](../containers/env.example) and provide its own PostgreSQL and MinIO
credentials, `PLE_LOCAL_GRADER_PASSWORD`, `PLE_LOCAL_AUTH_HOST_FILE`, and all four required
immutable native image pins: `PLE_POSTGRES_IMAGE_SHA256`, `PLE_MINIO_IMAGE_SHA256`,
`PLE_MINIO_MC_IMAGE_SHA256`, and `PLE_GATEWAY_IMAGE_SHA256`.

The custom identity file and any credential file used to sign in are operator-managed inputs. Do
not place bearer credentials, grader passwords, or other secret values in this guide or in tracked
files. The local-stack boundary and identity-file contract are documented in
[CONTAINER.md](CONTAINER.md).

## Private WeBWorK profile

Use the profile only when testing the shipped upstream renderer path:

```bash
./launch_local_stack.sh --with-webwork --no-open
```

For an existing pre-RC3 `containers/env.local`, run that normal command once. It safely adds the
required generated local values and two distinct mode-0600 secret files; then the following
read-only preflight succeeds:

```bash
./launch_local_stack.sh --with-webwork --check --no-open
```

The launcher requires the official WebWork2 and PG URLs, full immutable source revisions, a private
renderer/database topology, and local secret files rather than plaintext secret environment values.
Do not copy `containers/env.local`, `containers/local-login.txt`, or `containers/.secrets/` into a
deployed environment. See [CONTAINER.md](CONTAINER.md) for the service and network boundary.

With a custom environment, the operator must also provide the WebWork-specific image pins,
database credentials, render-course/user settings, renderer identity, and the exact official URL
and 40-character lowercase SHA-1 values named in [containers/env.example](../containers/env.example).
`PLE_WEBWORK_RENDER_PASSWORD_HOST_FILE` and `PLE_WEBWORK_MOJO_SECRET_HOST_FILE` must name two
different readable regular host files, each with exact mode 0600; do not put either secret in the
environment file. The authoritative source-pin and private-renderer contract is
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).

The first WebWork build fetches and compiles upstream sources and can consume substantial Podman VM
storage. Before starting it, inspect local capacity with `podman system df`; on macOS, adjust the
machine resources only using the documented steps in [MACOS_PODMAN.md](MACOS_PODMAN.md). Do not
remove volumes or images unless their contents are intentionally disposable.

## Known gaps

- WP-RC3's bounded live upstream build, PLE API path, and browser acceptance are accepted. Broad
  OPL compatibility and WeBWorK MATCH remain out of scope for RC3 and are owned by WP-RC5.
