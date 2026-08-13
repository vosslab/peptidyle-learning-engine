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
./check_rust.sh
source source_me.sh && python3 -m pytest -q tests/
```

Success is exit status zero from all three commands. `./check_codebase.sh` is the vendored
TypeScript and browser gate, `./check_rust.sh` is the repository-owned Cargo and Rust gate, and
`pytest tests/` remains the separate fast documentation and hygiene lane. For a built browser
artifact without starting containers, run `./build.sh`; it creates `dist/` and `dist_wasm/`.

For choosing a focused development gate, use [DEVELOPMENT.md](DEVELOPMENT.md). The durable API,
storage, tenancy, and server-only-grading boundaries are recorded in [CONTRACTS.md](CONTRACTS.md)
and [SECURITY_MODEL.md](SECURITY_MODEL.md), rather than inferred from local launcher behavior.

## Local stack setup

The normal stack starts PostgreSQL, MinIO, API, worker, gateway, and the standalone PG renderer.
Before first launch, build or obtain the renderer image from the separate
`webwork-pg-renderer` project under the tag named by `PLE_WEBWORK_RENDERER_IMAGE` (the default is
`localhost/pg-renderer:latest`). PLE does not build the renderer, run WebWork2, or run MariaDB.
All local ports are loopback-only, the gateway is intentionally HTTP-only, and
it does not set HSTS. Local startup is not evidence for the production TLS edge
or a deployment of the external renderer.

On its first normal run, the launcher creates ignored local configuration, credentials, and
secrets beneath `containers/`; it does not require copied development secrets.

```bash
./launch_local_stack.sh --no-open
./launch_local_stack.sh --check
```

The first command creates a mode-0600 environment file, credentials, and secrets (with a
non-secret identity-hash file), builds the repository, migrates and seeds the local database,
starts the stack, and prints the loopback URL and local sign-in file. `--check` then reads the
existing environment and validates Compose without building, starting a Podman machine, creating
local files, or changing containers.

## Local identity boundary

The generated values in `containers/local-login.txt` are a local development convenience. They
create a tenant-scoped `ple_session` for the seeded instructor or student and keep its bearer
value out of browser storage after sign-in. They do not create a PLE account or
`ple_account_session`, so they cannot claim an invitation, bootstrap a passkey, or prove the
canonical email-authentication journey. Canonical account/session composition is available to a
production environment, but no SMTP provider or email-activation path is configured today. The
local teaching walkthrough deliberately uses local identities and does not require email, a
mailbox, invitation delivery, or canonical-account acceptance. Fastmail is the intended future
external provider; do not claim canonical email sign-in works until its operator credentials,
authorized sender, live delivery, and browser sign-in have been verified.

## Custom environment files

The bootstrap belongs only to the default `containers/env.local` path. A non-default
`--env-file` is never created, rewritten, seeded, or supplied with local sign-in credentials;
prepare it from [containers/env.example](../containers/env.example). The launcher validates its
database, object-store, local-auth, invitation-secret, image-pin, and renderer settings. Keep
operator-managed credentials and secret values out of tracked files; see
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for the complete contract
and `docs/CONTAINER_PORT_MAPPING.md` for loopback port use.

An external SMTP account is an optional production connection. PLE uses the established Rust
`lettre` client to connect to an operator-selected provider; it does not install, operate, or
maintain a mail server. No provider is configured today; Fastmail is the intended future provider.
After an operator account, authorized sender, and application credential exist, `--with-smtp`
validates its hostname, encrypted submission mode, public HTTPS origin, and an absolute mode-0600
provider-token file as described in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md#external-smtp-provider). The production
composition can then use the canonical account/session route graph, but live email delivery and
browser sign-in still need their own acceptance evidence. Copy-link invitations remain available
without the overlay.

The launcher creates the renderer's local JWT secrets, records the selected OCI image identity,
and probes real render and grade behavior before PLE starts. The renderer has no MariaDB, course
credentials, volume, or host port; see [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md)
and [LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md).

## Production baseline is separate

The OpenTofu configuration under `deploy/opentofu/` is not installed or applied
by `npm run setup` or `launch_local_stack.sh`. It defines private no-NAT ECS
tasks, RDS, four SSE-KMS S3 domains, CloudFront/ALB TLS-origin admission, and
separate API, worker, and public-asset-publisher roles and secrets. A live AWS
deployment still requires operator-owned DNS, certificates, Secrets Manager
values, exact database-role provisioning, and disposable-account probes. The
external WeBWorK renderer feature remains disabled there until it is separately
attested for private ingress, immutable image provenance, TLS identity, and no
database or object-store authority. See
[MULTI_SERVER_SETUP.md](MULTI_SERVER_SETUP.md#production-baseline-in-opentofu).

## Known gaps

- Configure the selected external provider, then verify its sender approval, live delivery, and
  browser sign-in before using canonical email sign-in with real learners. This does not block the
  deliberately no-email local teaching walkthrough.
- PG/PGML compatibility beyond the four reviewed Chapter 1 MC/MATCH sources needs its own source
  and live evidence.
