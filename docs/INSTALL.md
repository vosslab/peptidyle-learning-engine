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
- The local stack supplies PostgreSQL and its migrator image supplies the client
  for canonical schema operations.
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

## Refresh the QPM distribution

Normal PLE builds use the committed [QPM distribution](../assets/qpm_wasm/source.json); a QPM checkout is
needed only to refresh it. With Rust, Node, npm, and QPM's browser build tooling available:

```bash
source source_me.sh && python3 devel/vendor_qpm_wasm.py
source source_me.sh && python3 devel/vendor_qpm_wasm.py -q /path/to/qti-package-maker-rs
source source_me.sh && ./build.sh
```

The default source is the sibling `../qti-package-maker-rs`; use `-q/--qpm-root` to select another
checkout explicitly. The script
runs QPM's `npm ci` and browser build, copies the wrapper, generated runtime, type declarations,
and license, then records
the checkout HEAD and consumed-file SHA-256 hashes in `source.json`. HEAD identifies the checkout;
the hashes identify the actual built bytes, including any uncommitted engine edits. Review both
repositories when refreshing a locally changed engine. PLE serves the distribution under
`dist/wasm/qpm-<hash>/`; the wrapper and WASM load only when Import opens.
PLE-owned paths use QPM names. Upstream package paths and generated filenames remain unchanged
inside the distribution, so its wrapper imports and recorded hashes stay reproducible.
Upstream browser and Node demos are excluded because PLE uses its own import screen.

After refresh, run the seven-kind import journey described in [USAGE.md](USAGE.md), plus the
ordinary PLE checks. A failing preview calls for an engine or decoder correction against
[NATIVE_JSON_SPEC.md](QUESTION_SPECS/NATIVE_JSON_SPEC.md), followed by rebuilding and re-vendoring.

## Instructor setup email

Instructor creation works without a configured email provider and reports that the
setup email was not sent. Configure `PLE_INSTRUCTOR_SETUP_SMTP_URL` and
`PLE_INSTRUCTOR_SETUP_FROM` to enable setup notices and ordinary email-code sign-in.
The SMTP URL must use `smtps` or `smtp` with `tls=required`. The setup link uses
the same HTTPS `PLE_BROWSER_ORIGIN` as the browser session boundary.

Keep provider credentials in the installation's private environment configuration.
The setup notice points to `/sign-in`; the sign-in code is short-lived and bound
to the browser that requested it. The seeded Live Demo entry remains available
when email delivery is unconfigured. Local capture tests verify the application
flow; a configured provider and real inbox are needed to verify external delivery.

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

The ready host imports complete Genetics, Biochemistry, Biotechnology, and
Biostatistics collections through the shared Python importer. PG and PGML remain
unchanged WeBWorK source. BBQ banks use the QPM CLI Native JSON export; PLE owns
Native JSON interpretation and image rendering.

The default local startup imports curriculum and adopts the complete public
Biochemistry Blueprint as BCHM 355, then provisions fictional enrollment and
representative Student Work through ordinary delivery. To import into a ready
local installation explicitly:

```bash
source source_me.sh && python3 local_stack.py install-data
source source_me.sh && python3 local_stack.py install-data --without-live-demo
```

The opt-out retains all curriculum, including bundled Genetics PGML, and uses a
non-login PLE Example Content Instructor. It omits fictional Accounts, Course,
enrollment, and Student Work on a fresh installation. Add `PLE_BPW_ROOT` and
`PLE_QPM_CLI` to the selected stack configuration file when adjacent development
checkouts or the `bbq-converter` executable on `PATH` are unavailable. The host uses
the selected gateway CA for verified HTTPS; it keeps
the ordinary temporary Instructor cookie in private pipes and revokes the session
after completion or failure. See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md).

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

After the structural base, API, worker, publisher, object storage, and browser
origin are ready, the audited installation environment brackets the shared Python
importer with trusted `installation-data prepare`, `finalize`, and `revoke`
phases. These internal phases receive private JSON through captured pipes. The
host owns BPW and QPM; the Rust migrator owns trusted Account and enrollment
provisioning. Every reusable content write and Course adoption uses ordinary
HTTP routes. OpenTofu leaves this final product-data step to the audited host
workflow and does not receive application or database secrets.
