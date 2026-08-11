# Usage

Use the root launcher for a complete local teaching-system test, including build, migration, seed,
health check, and browser entry. Native questions are the default path; the WeBWorK renderer is an
explicit private profile.

This is a local development workflow, not a production deployment guide. The durable route,
storage, tenancy, and grading boundaries live in [CONTRACTS.md](CONTRACTS.md) and
[SECURITY_MODEL.md](SECURITY_MODEL.md);
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) owns the local service
topology and recovery commands.

## Quick start

Build and open the native local stack:

```bash
./launch_local_stack.sh
```

The launcher prints the loopback application URL and the path to ignored local instructor and
student credentials. Paste one value from `containers/local-login.txt` into the local sign-in form.
The resulting browser session is HttpOnly; the bearer value is not stored in browser storage.

For a headless run or a quick restart with a known-current browser bundle:

```bash
./launch_local_stack.sh --no-open
./launch_local_stack.sh --skip-build --no-open
```

`--skip-build` requires an already-built `dist/index.html` and `dist/main.js`; use it only after a
successful `./build.sh` or normal launcher run.

## Configuration preflight

```bash
./launch_local_stack.sh --check
```

This checks tool availability, required environment values, and Compose configuration without
starting a Podman machine, building artifacts, creating local secrets, or changing containers. A
first local installation has no `containers/env.local`, so use the normal launcher once to bootstrap
it before expecting `--check` to succeed.

`--check` never bootstraps an environment. This is equally true for
`./launch_local_stack.sh --env-file path/to/env.local --check`: the custom file must already exist
and satisfy the launcher contract.

## Standalone WeBWorK PG renderer

```bash
./launch_local_stack.sh --no-open
```

This normal command starts PLE with the private external PG renderer, waits for its semantic
render-and-grade probe, seeds the bounded pilot, and then starts the application. The browser
communicates with PLE only; it does not receive renderer credentials, source, or upstream state.

For older local configuration, a normal launch safely adds the ignored renderer settings and local
JWT secrets. Validate the migrated configuration without changing state afterward:

```bash
./launch_local_stack.sh --check --no-open
```

Keep `containers/env.local` and `containers/local-login.txt` local. The renderer has no database,
persistent volume, or published host port; see
[LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md).

For a custom environment, provide the normal native settings plus the external renderer image,
renderer identity, and two independent JWT secrets named in
[containers/env.example](../containers/env.example). PLE does not require WebWork2 source pins,
render-course credentials, or a MariaDB password.

## Build and validation commands

```bash
./build.sh                 # Rust, Wasm, generated contracts, fixtures, and Solid bundle
./check_codebase.sh        # repository-wide quality gate
cargo test --workspace     # Rust tests
npm run test:playwright    # built-browser tests
```

`npm run build` and `npm run launch` are aliases for `./build.sh` and `./launch_local_stack.sh`.
`./build.sh --release` builds optimized host artifacts; the launcher accepts `--release` for the
same mode.

To validate or run a pre-existing non-default environment file, pass its path explicitly. The
launcher does not bootstrap, rewrite, seed, or create credentials for a custom file. Before running
it, provide the required PostgreSQL and MinIO credentials, local grader secret, local-auth identity
file, invitation-secret host file, and the five native image-digest fields from
[containers/env.example](../containers/env.example). The invitation issuer enables
copy-link enrollment without configuring SMTP; canonical email authentication still
requires an operator-selected SMTP provider.

```bash
./launch_local_stack.sh --env-file path/to/env.local --check
./launch_local_stack.sh --env-file path/to/env.local --no-open
```

To activate an external SMTP account later, fill the seven `PLE_SMTP_*` and
`PLE_PUBLIC_APP_BASE_URL` fields in the custom environment as described in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md#external-smtp-provider), then select the opt-in
overlay explicitly:

```bash
./launch_local_stack.sh --env-file path/to/env.local --with-smtp --check
./launch_local_stack.sh --env-file path/to/env.local --with-smtp --no-open
```

This connects to the selected provider with authenticated encrypted submission.
It does not start a PLE mail service. Omitting `--with-smtp` leaves copy-link
invitations available and email authentication unconfigured.

The live WebWork acceptance command also needs operator-provided demo inputs because custom runs do
not create `local-login.txt` or `local-webwork-demo.json`. Give
[tests/e2e/e2e_webwork_render_rpc.sh](../tests/e2e/e2e_webwork_render_rpc.sh) a readable custom
environment with `PLE_E2E_ENV_FILE`, plus a mode-0600 student credential file through
`PLE_E2E_STUDENT_CREDENTIAL_FILE` and a WebWork-pilot manifest through
`PLE_E2E_WEBWORK_MANIFEST_FILE`. The manifest supplies the assignment ID; the credential file must
match the custom identity records. The test then launches the selected environment and passes only
these safe inputs to the browser gate.

## Stack inspection

After a local run, health is served through the one loopback gateway origin. The chosen port is
normally 3000; first-run bootstrap records another free port from 3000 through 3099 when needed.
Use the printed URL, or read `PLE_GATEWAY_HOST_PORT` in `containers/env.local` before using `curl`.

```bash
curl -s http://127.0.0.1:3000/health
podman compose -f containers/compose.yaml --env-file containers/env.local ps
podman compose -f containers/compose.yaml --env-file containers/env.local down
```

The normal `down` command stops containers while retaining named data volumes. Removing volumes or
pruning images destroys local state or build cache, so make that decision deliberately. See
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for health behavior and service-specific logs.

## Known gaps

- The opt-in RC3 profile's live upstream build, PLE API path, and browser acceptance were accepted
  on 2026-08-10. For later changes, neither `--check` nor static tests replaces that recorded live
  evidence. Broad OPL compatibility and WeBWorK MATCH remain separately owned by WP-RC5.
