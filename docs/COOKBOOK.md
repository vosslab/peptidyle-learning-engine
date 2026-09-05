# Cookbook

Use these recipes to operate the current disposable PLE developer stack. They establish a real
HTTPS deployment and seeded Account session; they do not make retained Course, Question Library,
authoring, delivery, grading, or Gradebook contracts into available browser workflows.

Read [INSTALL.md](INSTALL.md) for prerequisites, [USAGE.md](USAGE.md) for the supported command
surface, and [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) for the current executable boundary.

## Start the local session

Start the fixed disposable owner and open the browser:

```bash
./run_live_demo.sh
```

For a headless result, use:

```bash
./run_live_demo.sh --headless
```

The command builds `dist/`, starts the `ple-live-demo-browser` stack, and prints its HTTPS origin.
When at least one configured seeded mapping is valid, the visible selector can establish an ordinary
session for Elena Instructor; Mary, Jack, or Avery Student; or Morgan Sysadmin. It does not grant a
role or relationship chosen by the browser.

## Inspect before changing state

Run read-only diagnostics from the repository root:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py projects
source source_me.sh && python3 local_stack.py status --project ple-live-demo-browser
source source_me.sh && python3 local_stack.py logs --project ple-live-demo-browser --tail 120 gateway api
source source_me.sh && python3 local_stack.py validate
```

`doctor` verifies Podman and the Compose provider. `status` reports semantic readiness rather than
only container state. `logs` remains scoped to the fixed project. See
[TROUBLESHOOTING.md](TROUBLESHOOTING.md) before any reset or repair.

## Verify the installed stack

Use the non-opening launch to verify the full local installation, then stop it through its owner:

```bash
./run_live_demo.sh --headless
./run_live_demo.sh stop
```

Success means the first command prints a ready HTTPS origin and the second confirms owner-scoped
cleanup. The stop removes only this disposable demo's resources and data.

## Run contract and service evidence

Use the smallest relevant gate while editing:

```bash
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
source source_me.sh && python3 local_stack.py acceptance
```

Run the aggregate only when its full material-tree scope is required:

```bash
source source_me.sh && ./all_test.sh
```

The current acceptance lanes prove named Rust, TypeScript, Python, PostgreSQL, and object-store
boundaries. They do not prove a current visible teaching workflow; see
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

## Use future workflows correctly

The following are retained product contracts, not current cookbook actions:

- Instructor Question Library, Blueprint Course, Course Instance, roster, and assignment workflows.
- Student Assignment delivery, Question Submission, feedback, and completed-work review.
- Instructor Gradebook, grading-operation, retention, and course-administration workflows.

Their intended boundaries are documented in [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md),
[STUDENT_GUIDE.md](STUDENT_GUIDE.md), [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md), and
[CONTRACTS.md](CONTRACTS.md). Do not represent a contract page or historical screenshot as an
available local-demo route.

## Known gaps

- Restore one production browser owner that creates visible teaching state through the disposable
  stack, captures current screenshots, and records accessibility and human visual review evidence.
- Verify broader PG/PGML compatibility beyond the reviewed Chapter 1 MC and MATCH sources before
  expanding the adapter claim.
