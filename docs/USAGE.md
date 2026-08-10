# Usage

Use the root launcher for a complete local teaching-system test, including build, migration, seed,
health check, and browser entry. Native questions are the default path; the WeBWorK renderer is an
explicit private profile.

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

## Private WeBWorK run

```bash
./launch_local_stack.sh --with-webwork --no-open
```

This command opts into the private WebWork renderer and MariaDB profile. It builds upstream WebWork2
and PG from the configured full source revisions, waits for the authenticated private renderer, then
seeds the opt-in pilot before starting the PLE application. The browser communicates with PLE only;
it does not receive renderer credentials, source, or upstream session fields.

For older local configuration, the normal profile command safely creates the new ignored settings
and secrets. Validate the migrated configuration without changing state afterward:

```bash
./launch_local_stack.sh --with-webwork --check --no-open
```

The profile uses two independent local mode-0600 files under `containers/.secrets/`. Keep those,
`containers/env.local`, and `containers/local-login.txt` local. The renderer and its database have
no published host port; see [CONTAINER.md](CONTAINER.md) for the topology.

For a custom environment, `--with-webwork` requires all native and WebWork pins and credentials in
the selected environment file, including `PLE_LOCAL_GRADER_PASSWORD` and
`PLE_LOCAL_AUTH_HOST_FILE`. It also requires the official WebWork2 and PG URLs, their exact
40-character lowercase SHA-1 revisions, and two distinct host-secret paths:
`PLE_WEBWORK_RENDER_PASSWORD_HOST_FILE` and `PLE_WEBWORK_MOJO_SECRET_HOST_FILE`. Each host-secret
file must be readable and have exact mode 0600. Use the current field names and non-secret pin
values in [containers/env.example](../containers/env.example); keep secret values out of the file,
shell history, and logs.

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
file, and the four native image-digest fields from [containers/env.example](../containers/env.example).

```bash
./launch_local_stack.sh --env-file path/to/env.local --check
./launch_local_stack.sh --env-file path/to/env.local --no-open
```

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
[CONTAINER.md](CONTAINER.md) for health behavior and service-specific logs.

## Known gaps

- The opt-in profile is documented from its checked-in launcher contract. Record a successful live
  upstream build, PLE API path, and browser acceptance separately; neither `--check` nor static
  tests establishes live WeBWorK acceptance.
