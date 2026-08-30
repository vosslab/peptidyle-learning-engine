# Usage

Use PLE through the fixed local live-demo owner. It runs one disposable, production-shaped HTTPS
stack with real PostgreSQL, MinIO, API, worker, gateway, and the private WebWork renderer; it is not
a separate mock application or a WebWork2 installation.

## Quick start

Start and open the live demo:

```bash
./run_live_demo.sh
```

On a fresh clone, this command creates or refreshes the fixed Python 3.12 `.venv`, installs the
declared Python dependencies, verifies PyYAML, and runs `devel/setup_typescript.sh` when
`node_modules` is absent. The lifecycle then builds production `dist/`, starts
`ple-live-demo-browser`, waits for HTTPS readiness, and opens the canonical origin. Select a seeded
Student, Instructor, or Sysadmin persona in the visible production-auth flow. Those personas use
ordinary server-owned accounts, memberships, roles, and authorization from the disposable seeded
installation.

Running the command again cleans its fixed project and starts a fresh seeded installation. This
replacement discards the prior demo's disposable records and keeps unrelated Podman projects intact.

For a headless start, keep the browser closed and use the printed origin:

```bash
./run_live_demo.sh --headless
```

Stop only through the active owner's cleanup boundary:

```bash
./run_live_demo.sh stop
```

The session may contain normal live records created through the UI. Its owner cleans only its
disposable resources.

## Sign in and switch personas

Use the visible PLE sign-in page after startup. The seeded live-demo panel provides ordinary
server-backed accounts for the Instructor, Student, and Sysadmin roles:

- Choose **Continue as** the persona you want, then choose one of the courses shown for that account.
  The current baseline names Elena as Instructor, Mary/Jack/Avery as Students, and Morgan as Sysadmin.
- The selector supplies only a known seeded persona. It does not grant a browser role claim;
  account identity, session, membership, and authorization are resolved by the server.
- Passwordless email and passkeys remain the intended ordinary sign-in methods. Their adapters are
  not mounted in the current build, and email delivery is not configured for this live demo; use
  the visible seeded-role entry for demo access.
- To switch from Instructor to Student work, sign out and select a seeded Student persona. Do not
  use Instructor **Student view** as a substitute for a graded Student run.

## Instructor assignment workspace

From an Instructor course, open **Assignments** and select an assignment title. The title opens the
assignment's Instructor home and its assignment-local navigation:

- **Overview** - `/instructor/courses/:courseRef/assignments/:assignmentRef`; review the current
  assignment state, readiness, instructions, and delivery summary.
- **Questions** - append `/questions`; organize fixed questions and reusable pools, then save the
  ordered content.
- **Policies** - append `/policies`; configure delivery and visibility rules, then save teaching
  operations.
- **Grading operations** - append `/grading-operations`; review safe automatic-grading operation
  metadata, retry an eligible operation, or request an assignment recalculation.
- **Student view** - append `/student-view`; inspect the current live, answer-free learner landing
  while retaining the Instructor session.

The visible title link is the supported entry into this workspace; `courseRef` and `assignmentRef`
are public route references, not authority. For graded work, sign out, choose a seeded Student and
authorized course, open the assignment title on the Student course page, and choose **Start
assignment**. Submit through visible response controls. When **Response received** appears, use
**Check grading status** until the live worker returns feedback or instructor attention. The
resulting learner run, submission, receipt, grade, and instructor gradebook history are ordinary live
records.

When an accepted submission needs recovery, the Student sees **Response received**, a cleared answer
buffer, and **Check grading status**. Select that control to read the current answer-free status;
the browser does not submit the answer again. If the status becomes **Your response needs instructor
attention**, Elena opens **Grading operations**, reviews the metadata-only operation, and chooses its
currently enabled named retry action when the operation is eligible. Elena follows the operation's
current state and available action, then refreshes the current **Gradebook** to observe the resulting
total after the ordinary worker completes the accepted private response.

The grading-operations API is assignment-local and metadata-only:

```text
GET  /api/courses/{course}/assignments/{assignment}/grading-operations
POST /api/courses/{course}/assignments/{assignment}/grading-operations/{operation}/retry
POST /api/courses/{course}/assignments/{assignment}/grading-operations/recalculate
```

All three routes return `Cache-Control: no-store`. Action requests have an empty body and require
`If-Match` plus `Idempotency-Key`; the server derives Instructor authority from the session and
rechecks it in the Store transaction. Responses contain operation state, safe reason, bounded
grouping, revisions, and action receipts, never learner responses, answer keys, feedback internals,
or score values.

## Build and validation commands

Use the root shell scripts as the primary build and validation interface. Run these commands outside
the normal demo launch when their named evidence is needed:

```bash
./build.sh
./check_rust.sh
./check_codebase.sh
source source_me.sh && .venv/bin/python local_stack.py acceptance
```

`./build.sh` builds the Rust workspace, WebAssembly bridge, generated contracts, fixture evidence,
and Solid client bundle. Run `./check_rust.sh` before `./check_codebase.sh`; the latter consumes its
generated projections. `local_stack.py acceptance` is the connected acceptance lane under the fixed
owner. For the complete final-material Validation suite, run:

```bash
source source_me.sh && ./all_test.sh
```

Use `./run_playwright_tests.sh --build` for one selected production-browser suite after installing
the browsers with `./devel/setup_playwright.sh`. It owns a fresh disposable HTTPS stack and cleanup,
and creates scenario state through visible PLE workflows.

## Controller diagnostics

These read-only commands are the supported exception to the root-wrapper path. Source the repository
shell environment before invoking the controller directly:

```bash
source source_me.sh && .venv/bin/python local_stack.py doctor
source source_me.sh && .venv/bin/python local_stack.py status
source source_me.sh && .venv/bin/python local_stack.py logs --tail 120
source source_me.sh && .venv/bin/python local_stack.py validate
```

`doctor` checks Podman and its Compose provider; `status` reports semantic readiness; `logs` prints
scoped application logs; and `validate` checks configuration and runtime availability without
starting or changing the stack. Add `--json` to `doctor`, `status`, or `validate` for machine output.
Use `./run_live_demo.sh stop` for cleanup rather than a project-wide Compose command.
Logs can contain private local diagnostic data. For deeper operator recovery, see
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) and [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## Known gaps

- `WP-INST-G1` is accepted. Its forward closeout migrations `2026081866` through `2026081869`
  provide the clean-volume receipt preflight, receipt writers, commit-v2 authority, and retry V2
  retirement boundary. Final material-tree Validation passed with the affected 99-migration live
  database, RLS, worker, browser, WebWork, and replica evidence. `WP-INST-G2` now owns audited
  learner-work inspection and grade-scheme-aware calculated Gradebook work. `WP-RC12` release
  acceptance remains open; a successful local demo does not itself establish release readiness.
- TODO: Verify PG/PGML compatibility beyond the reviewed Chapter 1 MC/MATCH sources with separate
  source and live evidence.
