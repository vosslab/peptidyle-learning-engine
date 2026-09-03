# Usage

PLE's current local Live Demo is a disposable HTTPS stack with real PostgreSQL,
MinIO, API, gateway, worker dependencies, and the private WebWork renderer. Its
currently mounted browser surface is deliberately limited to the account/session
entry boundary; it is not yet a runnable course-delivery demonstration.

## Quick start

Start and open the Live Demo:

```bash
./run_live_demo.sh
```

On a fresh clone, this command sources the repository shell environment through
its fixed `source_me.sh` path, invokes `python3 local_stack.py`, and runs
`devel/setup_typescript.sh` when `node_modules` is absent. It builds production
`dist/`, starts `ple-live-demo-browser`, waits for HTTPS readiness, and opens the
HTTPS origin.

For a headless start, use the printed origin:

```bash
./run_live_demo.sh --headless
```

Stop the disposable stack through its owner:

```bash
./run_live_demo.sh stop
```

Starting again replaces this project's disposable resources and seeded state. It
does not change unrelated Podman projects.

## Current Live Demo entry

Use the visible account page. The deployment-gated seeded selector can create an
ordinary server-owned Authenticated Session for Elena Instructor, Mary Student,
Jack Student, Avery Student, or Morgan Sysadmin. The selector supplies a closed
persona key only. The server resolves the configured Account and derives role,
membership, Student ownership, and every later authorization decision from
stored PLE state.

The current mounted HTTP surface is:

- `GET /health`
- `GET /api/auth/session`
- `POST /api/auth/logout`
- `GET` and `POST /api/auth/live-demo/accounts` when seeded demo configuration
  is present

Email-code and passkey sign-in remain retained product requirements. Their
browser adapters and delivery path are not mounted in the current local demo.

## Retained teaching workflows

Course navigation, Question Library, authoring workspace, Blueprint Course and
Course Instance operations, roster and invitation handling, assignment delivery,
Question submission, automated grading, Gradebook, worker operations, and
course retention remain future Store-backed product workflows. Their browser
routes are not mounted in the current demo, so this document does not instruct a
reader to use them.

When those workflows are mounted, their design retains these boundaries:

- exact Course Membership and Student ownership determine access;
- Answer Keys, Question Graders, private Question Source data, and grading input
  remain server-held;
- Question submission and grading recovery preserve accepted evidence rather
  than replaying a Student response; and
- Course, Assignment, and workspace references locate a record but never grant
  authority.

The intended product behavior and its contracts are documented in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md),
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md), and
[API_CONTRACTS.md](API_CONTRACTS.md). A restored browser acceptance owner must
establish a visible journey before any guide can call these workflows current.

## Build and validation commands

Use the root shell scripts for named build and validation evidence:

```bash
./build.sh
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 local_stack.py acceptance
source source_me.sh && ./all_test.sh
```

`local_stack.py acceptance` currently runs the declared PostgreSQL and Course
Appearance PostgreSQL/MinIO service lanes. `all_test.sh` proves its named current
lanes; neither command proves a visible production-browser teaching journey.

## Controller diagnostics

Source the repository shell environment before directly invoking the local-stack
controller:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs --tail 120
source source_me.sh && python3 local_stack.py validate
```

`doctor` checks Podman and its Compose provider. `status` reports semantic
readiness. `logs` prints scoped application logs. `validate` checks
configuration and runtime availability without starting the stack. Add `--json`
to `doctor`, `status`, or `validate` for machine output.

For recovery guidance, see [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md)
and [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
