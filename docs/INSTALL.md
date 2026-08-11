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

Verify the cross-language build gate and the separate fast documentation and repository-hygiene
suite:

```bash
./check_codebase.sh
source source_me.sh && python3 -m pytest -q tests/
```

Success is exit status zero from both commands. `./check_codebase.sh` reports its own stage
summary; `pytest tests/` intentionally remains a separate fast lane. For a built browser artifact
without starting containers, run `./build.sh`; it creates `dist/` and `dist_wasm/`.

For choosing a focused development gate, use [DEVELOPMENT.md](DEVELOPMENT.md). The durable API,
storage, tenancy, and server-only-grading boundaries are recorded in [CONTRACTS.md](CONTRACTS.md)
and [SECURITY_MODEL.md](SECURITY_MODEL.md), rather than inferred from local launcher behavior.

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
credentials, `PLE_LOCAL_GRADER_PASSWORD`, `PLE_LOCAL_AUTH_HOST_FILE`, the mode-0600
`PLE_INVITATION_TOKEN_SECRET_HOST_FILE`, and all five required
immutable native image pins: `PLE_POSTGRES_IMAGE_SHA256`, `PLE_MINIO_IMAGE_SHA256`,
`PLE_MINIO_MC_IMAGE_SHA256`, `PLE_GATEWAY_IMAGE_SHA256`, and
`PLE_SECRET_INIT_IMAGE_SHA256`.

The custom identity file and any credential file used to sign in are operator-managed inputs. Do
not place bearer credentials, grader passwords, or other secret values in this guide or in tracked
files. The local-stack boundary and identity-file contract are documented in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md).

An external SMTP account is optional. PLE already uses the established Rust
`lettre` client and does not install a mail server. When an operator later
selects a provider, configure the provider hostname, encrypted submission mode,
authorized sender, public HTTPS origin, and an absolute mode-0600 provider-token
file through the opt-in `--with-smtp` path documented in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md#external-smtp-provider). The default local install
continues to support copy-link course invitations without SMTP; canonical email
sign-in requires the configured provider.

## Standalone WeBWorK PG renderer

The renderer is part of the normal stack, so the same launcher command starts it:

```bash
./launch_local_stack.sh --no-open
```

PLE relies on an existing external `webwork-pg-renderer` image rather than building WebWork2. The
default environment names `localhost/pg-renderer:latest`; build or obtain that image through the
separate renderer project before first launch. The PLE launcher verifies the image, records its OCI
identity, and probes real render and grade behavior.

The default bootstrap creates two local renderer JWT secrets in the ignored mode-0600
`containers/env.local`. A custom environment must provide `PLE_WEBWORK_RENDERER_IMAGE`,
`PLE_WEBWORK_RENDERER_ID`, `PLE_WEBWORK_PROBLEM_JWT_SECRET`, and
`PLE_WEBWORK_SESSION_JWT_SECRET`. The renderer has no MariaDB, course credentials, volume, or host
port. See [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md) and
[LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md).

## Known gaps

- WP-RC3's bounded live upstream build, PLE API path, and browser acceptance are accepted. Broad
  OPL compatibility and WeBWorK MATCH remain out of scope for RC3 and are owned by WP-RC5.
