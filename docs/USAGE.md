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
- To exercise passkeys, open **Account** -> **Your passkeys**, enter a passkey name, choose **Add
  passkey**, and complete the device prompt. Use **Sign out**, then **Sign in with a passkey** and
  choose a course again. Email remains the normal account sign-in method.
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
- **Student view** - append `/student-view`; inspect the current live, answer-free learner landing
  while retaining the Instructor session.

The visible title link is the supported entry into this workspace; `courseRef` and `assignmentRef`
are public route references, not authority. For graded work, sign out, choose a seeded Student and
authorized course, open the assignment title on the Student course page, and choose **Start
assignment**. Submit through visible response controls. The resulting learner run, submission,
receipt, grade, and instructor gradebook history are ordinary live records.

## Build and validation

Use these commands outside the normal demo launch when their named evidence is needed:

```bash
./build.sh
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 local_stack.py acceptance
```

`./build.sh` builds the Rust workspace, WebAssembly bridge, generated contracts, fixture evidence,
and Solid client bundle. Run `./check_rust.sh` before `./check_codebase.sh`; the latter consumes its
generated projections. `local_stack.py acceptance` is the connected acceptance lane under the fixed
owner. For the complete final-material Validation suite, run:

```bash
source source_me.sh && ./all_test.sh
```

Use `./run_playwright_tests.sh --build` for one selected production-browser suite. It owns a fresh
disposable HTTPS stack and cleanup, and creates scenario state through visible PLE workflows.

## Read-only operations

The local-stack controller reports canonical stack state. Source the repository shell setup first:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs --tail 120
source source_me.sh && python3 local_stack.py validate
```

`doctor` reports Podman/configuration diagnostics; `status` reports semantic readiness; `logs` shows
scoped application logs; and `validate` checks initialized configuration and runtime state without
starting or changing the stack. Add `--json` to `doctor`, `status`, or `validate` for machine output.
Logs can contain private local diagnostic data. For deeper operator recovery, see
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) and [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## Known gaps

- TODO: Verify PG/PGML compatibility beyond the reviewed Chapter 1 MC/MATCH sources with separate
  source and live evidence.
