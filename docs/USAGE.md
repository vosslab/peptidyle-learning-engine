# Usage

Use PLE through the fixed local live-demo owner. It runs one disposable, production-shaped HTTPS
stack with real PostgreSQL, MinIO, API, worker, gateway, and the private WebWork renderer; it is not
a separate mock application or a WebWork2 installation.

## Quick start

Start and open the live demo:

```bash
./run_live_demo.sh
```

On a fresh clone, this command runs `devel/setup_typescript.sh` when `node_modules` is absent. The
lifecycle then builds production `dist/`, starts `ple-live-demo-browser`, waits for HTTPS readiness,
and opens the canonical origin. Select a seeded Student, Instructor, or Sysadmin persona in the
visible production-auth flow. Those personas use ordinary server-owned accounts, memberships, roles,
and authorization from the disposable seeded installation.

Running the command again authenticates to the active demo owner, cleans its fixed project, and
starts a fresh seeded installation. This replacement discards the prior demo's disposable records
and keeps unrelated Podman projects intact.

For a headless start, keep the browser closed and use the printed origin:

```bash
./run_live_demo.sh --headless
```

Stop only through the active owner's cleanup boundary:

```bash
./run_live_demo.sh stop
```

The session may contain normal live records created through the UI. Its owner cleans only its
disposable resources and retains unrelated Podman projects.

## Teaching workflows

The demo starts with useful ordinary courses, assignments, questions, and student activity. Use the
visible PLE workflows to create or modify teaching state; instructor validation and student work use
the same normal course, assignment, submission, grading, receipt, and review paths.

- Follow [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md) to create a course, manage roster membership,
  build an assignment from the published corpus, and review grades.
- Follow [STUDENT_GUIDE.md](STUDENT_GUIDE.md) to complete, score, correct, and repeat an assignment
  with the keyboard-capable student workflow.
- Read [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) for the seeded baseline and live-data boundary.

## Build and validation

Use these commands outside the normal demo launch when their named evidence is needed:

```bash
./build.sh
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 local_stack.py acceptance
```

`./build.sh` builds the Rust workspace, WebAssembly bridge, generated TypeScript contracts, fixture
evidence, and Solid client bundle. `./check_rust.sh` is the offline Rust gate; run it before
`./check_codebase.sh`, which consumes its generated projections. `local_stack.py acceptance` is the
connected acceptance lane under the fixed owner; it refuses if a conflicting default or live-demo
browser stack already exists. For the complete final-material Validation suite, run:

```bash
source source_me.sh && ./all_test.sh
```

Use `./run_playwright_tests.sh --build` for one selected production-browser suite. It owns a fresh
disposable HTTPS stack and cleanup, and creates scenario state through visible PLE workflows.

## Read-only operations

The local-stack controller reports the state of the canonical stack. Source the repository shell
setup before calling it directly:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs --tail 120
source source_me.sh && python3 local_stack.py validate
```

`doctor` reports Podman and configuration diagnostics. `status` reports semantic readiness, `logs`
shows scoped application logs, and `validate` checks initialized canonical configuration and runtime
state without starting or changing the stack. Add `--json` to `doctor`, `status`, or `validate` for
machine-readable output. Logs can contain private local diagnostic data.

## Advanced operator commands

Use these only while deliberately operating the existing local stack; keep the primary demo path
above for ordinary development.

```bash
source source_me.sh && python3 local_stack.py restart api
source source_me.sh && python3 local_stack.py service stop webwork-renderer
source source_me.sh && python3 local_stack.py reset --confirm-project containers --dry-run
```

`restart` accepts a supported stateless service. `service stop` is limited to the default WebWork
renderer and is useful for an intentional renderer-outage check. `reset --dry-run` previews removal
of confirmed default-stack Compose data; omit `--dry-run` only when intentionally regenerating that
disposable data. Recover the demo through its fixed owner and confirmed project boundary. Consult
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) and [TROUBLESHOOTING.md](TROUBLESHOOTING.md),
then use the scoped `local_stack.py` diagnostics and reset commands.

## Known gaps

- Verify PG/PGML compatibility beyond the reviewed Chapter 1 MC/MATCH sources with separate source
  and live evidence.
