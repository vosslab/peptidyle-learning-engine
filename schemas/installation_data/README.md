# PLE installation data

The host imports complete Genetics, Biochemistry, Biotechnology, and Biostatistics
content through [the shared Python importer](../../launchers/import_bpw.py).
PG and PGML reach WeBWorK unchanged. BBQ sources pass through the QPM CLI to
Native JSON. Question publication, Pool creation, Blueprint creation, Course
adoption, Assessment release, and Student activity use ordinary HTTP routes.

The default local installation includes fictional Live Demo Accounts and adopts
Biochemistry as BCHM 355. To import into an already ready local installation:

```bash
source source_me.sh && python3 local_stack.py install-data
```

To retain curriculum and omit the fictional teaching graph on a fresh installation:

```bash
source source_me.sh && python3 local_stack.py install-data --without-live-demo
```

Set `PLE_BPW_ROOT` and `PLE_QPM_CLI` to explicit source/tool paths when adjacent
development checkouts are unavailable. The host trusts its selected gateway's
Caddy root certificate for this import process.

The private Rust `installation-data prepare`, `finalize`, and `revoke` phases
provision trusted fictional identities/enrollment and bracket the import with an
ordinary temporary Instructor session. The normal eight-hour session lifetime
supports long imports; the host revokes the exact session after success or
failure. Credentials cross captured stdin/stdout pipes rather than argv, logs,
or result files. `--without-live-demo` uses a non-login PLE Example Content
Instructor, retaining ordinary ownership without creating fictional Accounts.

The schema manifest is DDL-only. `live_demo.sql` creates only fictional
relationships around the API-created Course. It creates no reusable content,
Course, Assessment, Attempt, response, grading, or statistics records.

See [the Live Demo specification](../../docs/LIVE_DEMO_SPEC.md) and
[the installation guide](../../docs/INSTALL.md).
