# BiologyProblems.org import

The root-level `ple_import/` package owns source discovery, publication through the Instructor
API, and Course assembly. [launchers/import_bpw.py](../launchers/import_bpw.py) is the thin
command entry point used by both Instructors and the host installer.

Run one complete import through the ordinary Instructor authoring API:

```bash
source source_me.sh
python3 launchers/import_bpw.py \
  --bpw-root ../biology-problems-website \
  --api-url https://localhost \
  --qpm-cli ../qti-package-maker-rs/target/debug/bbq-converter \
  --result-file /tmp/bpw-import-result.json \
  --ca-file /path/to/trusted-local-ca.pem
```

The terminal asks for the Instructor email and the emailed sign-in code. The existing email-code
sign-in establishes a session held in memory. Use the deployment's trusted CA file for a local
stack; public HTTPS uses the system trust store when `--ca-file` is omitted.

The host installer invokes the same command and passes one JSON line on protected stdin:
`{"sessionCookie":"__Host-ple_session=<opaque session value>"}`. Credentials stay out of command
arguments, result files, and progress messages.

The importer discovers Genetics, Biochemistry, Biotechnology, Biostatistics, Molecular Biology,
Laboratory, and Other Questions from the BPW directory tree. It uses BPW topic descriptions and
problem-set titles. Existing bundled Genetics sources absent from BPW are ordinary additional
inputs; their manifest supplies source metadata.
All seven collection directories are required; a missing directory stops the import and appears
in the incomplete result. The package resolves its checkout through Git from its own directory.

Each associated PG/PGML source is published directly as one WeBWorK Question. It replaces its
generated static bank. Standalone PG/PGML is also included. Companion type labels and the small
inspected standalone-family metadata map supply the initial editable Question Type. Source bytes
remain unchanged. QPM converts only the remaining static BBQ banks to `ple_native_json`. Every
emitted Question is published unchanged, including supplied media uploaded through the existing
Draft image endpoint, and each complete bank becomes a Question Pool.

Before publishing PG/PGML, the importer requests diagnostics from the saved Draft's existing
preview API at seed 1. PLE sends the source to the configured WeBWorK `/render-api` and returns
`flags.error_flag` and `debug.pg_warn` as `errorFlag` and `pgWarn`. Renderer errors stop the import
and leave the Draft unpublished; warnings are printed with the filename and seed and do not
block publication. This checks one realization, not every possible seed. No renderer credentials,
HTML, or grading state are returned to the importer. PG/PGML never passes through QPM.

Four public Blueprint Courses retain BPW subject names. Each source topic becomes a module;
Assessments preserve source order and deliver at most nine Questions. Longer topics receive A/B/C
parts. Explicit Hard or Bonus distinctions become Bonus Assessments with extra-credit entries.
Algorithmic Questions are Fixed entries; static Pools select one member. Initial points are one
per ordinary Question, one per MULTIFIB
blank, and half a point per MATCH/ORDER choice. Literal algorithmic cardinality metadata supplies
matching points when available; unavailable counts and differing Pool response counts appear in
the result as editable starting defaults. Bloom remains unset. Blueprint content has no dates.

The result contains `blueprints`, `questions`, `pools`, `pointDefaults`, `complete`, and `errors`.
An incomplete import exits nonzero. It records completed counts and diagnostics, then stops;
unsupported content is never silently omitted. HTTP failures include the server's safe
`X-Request-ID` when supplied, without response bodies. The command performs one import without resuming
or retrying prior work. Run a fresh installation for another complete installation import.
