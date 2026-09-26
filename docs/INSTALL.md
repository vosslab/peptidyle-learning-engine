# Install

Installation prepares a contributor checkout and creates PLE's canonical database
structure. Every installation includes the complete, free and open-source Biology
Problems Website Genetics Blueprint as its example course. The ordinary Live Demo is
separate optional teaching data. PLE remains pre-production; these instructions
describe local development and controlled installation operations, not a production
deployment procedure.

## Requirements

- Bash, Git, `curl`, `awk`, `openssl`, `xxd`, and `lsof`.
- Node.js and npm for the browser build; dependencies are declared in
  [package.json](../package.json).
- Stable Rust via `rustup`; [rust-toolchain.toml](../rust-toolchain.toml) selects
  `rustfmt`, Clippy, and `wasm32-unknown-unknown`.
- Python available as `python3`; [pip_requirements.txt](../pip_requirements.txt)
  declares PyYAML and `podman-compose`. Developer checks also use
  [pip_requirements-dev.txt](../pip_requirements-dev.txt).
- Podman and a usable Compose adapter for the local stack. On macOS, use
  [Brewfile](../Brewfile) and [MACOS_PODMAN.md](MACOS_PODMAN.md).
- PostgreSQL 17 is supplied by the local stack's migrator image; do not substitute
  an unreviewed client for canonical schema operations.
## Set up a checkout

```bash
git clone https://github.com/vosslab/peptidyle-learning-engine.git
cd peptidyle-learning-engine
brew bundle
source source_me.sh && python3 -m pip install --requirement pip_requirements.txt
./devel/setup_typescript.sh
```

`brew bundle` is the documented macOS dependency path. Rust is installed through
`rustup`; its selected toolchain downloads required components when Cargo first runs.
Install developer-only Python requirements before the full local test lanes:

```bash
source source_me.sh && python3 -m pip install \
  --requirement pip_requirements.txt --requirement pip_requirements-dev.txt
```

Install the Chromium headless shell only when running browser checks. The setup skips full
Chromium and Firefox:

```bash
./devel/setup_playwright.sh
```

## Initialize a database

The DDL-only base manifest is [schemas/base_schema/install.sql](../schemas/base_schema/install.sql).
First run the platform bootstrap transaction that creates every PLE PostgreSQL
role. The canonical administration command then reads
`PLE_MIGRATION_DATABASE_URL`, requires the `ple_migrator` PostgreSQL role,
initializes only an empty PLE database, applies recognized forward migrations
after production freeze, and verifies the resulting schema:

```bash
cargo tools database initialize
```

While the base identity is `pre-production`, `initialize` installs and verifies
only the editable base and `migrate` rejects the request. After the base schema
has been frozen by the first approved production deployment, use the same
migration credential for later structural changes:

```bash
cargo tools database migrate
```

Do not use `migrate` for an empty database. The base schema stays editable only until
that freeze; see [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) and
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md).

## Installation content and Live Demo

Automated initial Bloom Classification is deferred. PLE does not currently
configure an AI provider. Publication persistence still requires a prepared
Bloom receipt, so installation-data publication remains unavailable until that
separate cutover is complete. Structural installation and ordinary
non-publication runtime do not require an AI backend.

After the Bloom publication cutover, installation data will publish the complete
Genetics Blueprint through the ordinary content-publication path. It will remain
installation content, not a fixture, SQL dump, or separate pilot deployment. The
same operation will create the complete ordinary, removable Live Demo teaching graph:

```bash
cargo tools installation-data provision
```

The Genetics Blueprint will have eleven ordered topic Assessments and be reusable
by vetted Instructors when Public. `--without-live-demo` will omit only the
fictional Live Demo teaching graph. `provision` will run content publication and
the database-owned Live Demo graph, then create cross-system Student Work and
grading effects through their owning product paths. `apply` will remain the
narrower convergent installation operation. The local-stack controller supplies
the required migrator, publisher, storage, API, and worker capabilities. The
graph is documented in [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md).

## Verify install

Use a restricted application connection in `DATABASE_URL`; it has no DDL or
migration-ledger authority. The command checks the application-safe schema
projection:

```bash
cargo tools database verify
```

After the Bloom publication cutover, start the default local Live Demo and stop it
through its owner:

```bash
./launchers/run_live_demo.sh --headless
./launchers/run_live_demo.sh stop
```

The local-stack start establishes its named disposable environment. It does not by
itself prove the connected service or visible browser acceptance gates. See
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the applicable evidence.

## Production installation

After the Bloom publication cutover and after the structural base, API, worker,
publisher, object storage, and browser origin are ready, the short-lived audited
administration environment runs
`cargo tools installation-data provision` to publish the bundled Genetics example
and create the complete known-good Live Demo. An installation owner can instead
run `cargo tools installation-data provision --without-live-demo`; that still
publishes the Genetics Blueprint while omitting the Live Demo. OpenTofu
intentionally leaves this final product-data step to that audited workflow; it
does not receive database or application secrets or create a one-shot
provisioning subsystem.
