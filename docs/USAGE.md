# Usage

PLE's current local Live Demo is a disposable HTTPS stack with real PostgreSQL,
MinIO, API, gateway, worker dependencies, and the private WebWork renderer. Its
currently available Browser Surface is deliberately limited to the account/session
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

The current implemented HTTP surface is:

- `GET /health`
- `GET /api/auth/session`
- `POST /api/auth/logout`
- `GET` and `POST /api/auth/live-demo/accounts` when seeded demo configuration
  is present

Email-code authentication remains future work. The passkey capability is deferred:
it has no configuration, setup credential, Server Route, Browser Surface, or
completed ceremony in the current local demo.

## Retained teaching workflows

Course navigation, Question Library, authoring workspace, Blueprint Course and
Course Instance operations, roster and invitation handling, assignment delivery,
Question submission, automated grading, Gradebook, worker operations, and
course retention remain future Store-backed product workflows. Their browser
Server Routes do not exist in the current demo, so this document does not instruct a
reader to use them.

When those workflows are implemented, their design retains these boundaries:

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

## Temporary attended signup-email tool

The local macOS invitation mailer sends signup URLs that were created elsewhere;
it does not implement or claim PLE roster import, Account creation, Course
Enrollment, or signup completion.

Create `output-email/roster_export.json` with owner-only permissions (`0600`):

```json
{
  "course_name": "Genetics 301",
  "students": [
    {
      "email": "student@mail.roosevelt.edu",
      "signup_url": "https://example.edu/signup/opaque-value",
      "display_name": "Student Name",
      "roster_id": "optional-local-reference"
    }
  ]
}
```

Recipient domains must appear in
[`invitation_mailer.yaml`](../invitation_mailer.yaml), and signup URLs must use
HTTPS. Preview the batch first:

```bash
source source_me.sh && python3 launchers/send_invitations.py output-email/roster_export.json
```

Then perform a small attended send:

```bash
source source_me.sh && python3 launchers/send_invitations.py \
  output-email/roster_export.json --send --limit 5
```

Mail.app visibly composes and sends each message through its configured account.
Remain at the Mac, verify the first messages in the Sent mailbox, and stop if the
sender or content is wrong. macOS may request Automation permission for the
terminal or Python process. Tell students the sender address and subject through
the normal course channel before the batch.

The owner-private `output-email/invitation_status.json` suppresses duplicates.
Both `sent` and interrupted `indeterminate` recipients are held on a normal rerun.
After checking Mail.app, deliberately resend exactly one held recipient with:

```bash
source source_me.sh && python3 launchers/send_invitations.py \
  output-email/roster_export.json --send \
  --only student@mail.roosevelt.edu --force-resend
```

Failed and dry-run observations remain eligible for a later normal send.
`output-email/sent_log.csv` is a readable projection of confirmed local
observations; the Sent mailbox remains the operator's delivery evidence. Signup
URLs are not written to status files or progress output.

After reconciling the batch, retain or destroy `output-email/` according to the
course-record policy. The tool is disposable: remove `invitation_mailer/`,
`launchers/send_invitations.py`, `invitation_mailer.yaml`, its focused tests, and the
`py-applescript` dependency when it is no longer needed.

## Build and validation commands

Use the named build and validation entry points:

```bash
./build.sh
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 local_stack.py acceptance
source source_me.sh && ./launchers/all_test.sh
```

`local_stack.py acceptance` currently runs the declared PostgreSQL and Course
Appearance PostgreSQL/MinIO service lanes. `launchers/all_test.sh` proves its named current
lanes; neither command proves a visible production-browser teaching journey.

## Controller diagnostics

Source the repository shell environment before directly invoking the local-stack
controller:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py projects
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs --tail 120
source source_me.sh && python3 local_stack.py validate
```

`doctor` checks Podman and its Compose provider. `status` reports semantic
readiness. `projects` lists labelled Compose projects. `logs` prints scoped
application logs. `validate` checks configuration and runtime availability
without starting the stack. Add `--json`
to `doctor`, `projects`, `status`, or `validate` for machine output. `logs`
accepts `--follow`, an explicit `--project`, and optional service names while
diagnosing a stack.

For recovery guidance, see [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md)
and [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## Known gaps

- TODO: Restore the canonical production-browser owner and record visible,
  accessible teaching-workflow acceptance before documenting a course workflow
  as current.
