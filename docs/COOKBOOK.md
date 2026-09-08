# Cookbook

Use these recipes to operate the current disposable PLE developer stack. Startup establishes a real
HTTPS deployment and a representative used Course through ordinary product contracts. The seeded
personas can exercise the current Question Library, Course, roster, Assignment, Student delivery,
grading, and Gradebook workflows in the browser.

Read [INSTALL.md](INSTALL.md) for prerequisites, [USAGE.md](USAGE.md) for the supported command
surface, and [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) for the current executable boundary.

## Start the local session

Start the fixed disposable owner and copy its printed HTTPS URL into your browser:

```bash
./launchers/run_live_demo.sh
```

To open the ready URL of the running demo without replacing it, use:

```bash
./launchers/run_live_demo.sh open
```

Use `./launchers/run_live_demo.sh start --open` only when you intentionally want to replace the disposable
demo and open the new one.

The command builds `dist/`, starts the `ple-live-demo-browser` stack, and prints its HTTPS origin.
When at least one configured seeded mapping is valid, the visible selector can establish an ordinary
session for Elena Rivera; Mary Okafor, Jack Nguyen, or Avery Thompson; or Morgan Delgado. It does
not grant a role or relationship chosen by the browser.

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

Use the default non-opening launch to verify the full local installation, then stop it through its
owner:

```bash
./launchers/run_live_demo.sh
./launchers/run_live_demo.sh stop
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
source source_me.sh && ./launchers/all_test.sh
```

The current acceptance lanes prove named Rust, TypeScript, Python, PostgreSQL, and object-store
boundaries. They do not prove a current visible teaching workflow; see
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

## Review launch readiness

Use the seeded course as accumulated product state, not a scripted presentation:

1. Sign in as Elena Rivera. Open `Biochemistry 301: Proteins and Peptides`; inspect its released
   Assignment, roster, and Gradebook. Mary, Jack, and Avery should appear as completed and scored,
   in progress, and not started.
2. Sign in as Mary Okafor. Open the same Course Instance and confirm her completed Assignment and
   score.
3. Sign in as Jack Nguyen. Open the Assignment and resume his existing in-progress Assignment
   Attempt.
4. Sign in as Avery Thompson. Confirm the Assignment is not started, then start it from the
   beginning. This intentionally mutates the disposable acceptance state; restart the Live Demo to
   restore the baseline.

The current boundaries and deferred capabilities are documented in
[INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md), [STUDENT_GUIDE.md](STUDENT_GUIDE.md),
[ACTIVITY_MODEL.md](ACTIVITY_MODEL.md), and [CONTRACTS.md](CONTRACTS.md).

## Rehearse attended email invitation delivery

Email delivery is optional and outside automated startup. To evaluate it against the used Course,
Elena can add a fourth fictional Student through the normal roster form using an address controlled
by the operator, then download the pending invitation export. Keep the three seeded `.invalid`
addresses unchanged; they make routine startup incapable of sending mail to anyone.

Move the downloaded private JSON into `output-email/`, keep it mode 0600, and preview it before any
send:

```bash
source source_me.sh && python3 launchers/send_invitations.py \
  output-email/ple-invitations.json --dry-run
```

If the preview is correct, perform one visible attended send to the controlled address:

```bash
source source_me.sh && python3 launchers/send_invitations.py \
  output-email/ple-invitations.json --send --limit 1
```

Remain at the Mac and verify Mail.app's Sent mailbox. The current Live Demo intentionally has no
email-code authentication adapter, so this rehearsal proves export and real delivery but not the
recipient's authentication or invitation claim. Configure and accept the real email-code path
before treating end-to-end signup and claim as launch-ready.

## Known gaps

- Configure and accept passwordless email authentication before claiming that a newly invited,
  non-seeded Student can sign in and claim a Course Invitation.
- Verify broader PG/PGML compatibility beyond the reviewed Chapter 1 MC and MATCH sources before
  expanding the adapter claim.
