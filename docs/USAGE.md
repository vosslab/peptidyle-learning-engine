# Usage

PLE's local Live Demo is a disposable HTTPS stack with real PostgreSQL, MinIO,
API, gateway, worker dependencies, and the private WebWork renderer. It serves
the current role- and relationship-gated PLE workflow; it is not a presentation
substitute. M19 recorded its fresh sealed-stack production-browser proof on
2026-09-07.

## Quick start

Start the Live Demo:

```bash
./launchers/run_live_demo.sh
```

After the Python prerequisites in [INSTALL.md](INSTALL.md) are present, this command sources the
repository shell environment through its fixed `source_me.sh` path, invokes
`python3 local_stack.py`, and runs `devel/setup_typescript.sh` before a start. It builds production
`dist/`, starts
`ple-live-demo-browser`, waits for HTTPS readiness, and prints the HTTPS origin. Open that URL in
your browser.

To open the ready URL of an already-running demo, use:

```bash
./launchers/run_live_demo.sh open
```

`--open` is a compatible shorthand for `open`. To create a fresh demo and open it automatically,
use `./launchers/run_live_demo.sh start --open`. `--headless` remains an accepted explicit spelling of the
default non-opening behavior.

Stop the disposable stack through its owner:

```bash
./launchers/run_live_demo.sh stop
```

Starting again replaces this project's disposable resources and seeded state. It
does not change unrelated Podman projects.

## M20 Live Demo capture

Run the dedicated capture command to rebuild M20's role-owned current corpus:

```bash
./devel/capture_screenshots.sh
```

The command starts a fresh disposable stack, navigates the visible connected
workflows, validates a complete staging corpus, publishes the flat public and
Product Role folders, receipt, and generated atlas, then stops that stack. Use
`./devel/capture_screenshots.sh --verify` to validate the published corpus and
replay every declaration through a new clean Live Demo. The temporary replay
and atlas remain under `test-results/screenshot-corpus/verify/` for visual
review. Neither command uses pixel equality as a pass/fail gate.

## Current Live Demo entry

Use the visible seeded-entry page. It replaces only the normal
identity-verification ceremony and can create an ordinary server-owned
Authenticated Session for Elena Rivera, Mary Okafor, Jack Nguyen, Avery
Thompson, or Morgan Delgado. The entry supplies a closed persona key only. The
server resolves the configured Account and derives Product Role, Course
Membership, Student ownership, and every later authorization decision from
stored PLE state. The resulting session uses the applicable protected product
routes; the browser can download invitation-export input but cannot send mail.

The current HTTP route inventory is in [API_CONTRACTS.md](API_CONTRACTS.md).

Email-code authentication remains future work. The passkey capability is deferred:
it has no configuration, setup credential, Server Route, Browser Surface, or
completed ceremony in the current local demo.

## Teaching workflow boundaries

The implemented Instructor, Student, and Sysadmin routes retain these
boundaries:

- exact Course Membership and Student ownership determine access;
- Answer Keys, Question Graders, private Question Source data, and grading input
  remain server-held;
- Question submission and grading recovery preserve accepted evidence rather
  than replaying a Student response; and
- Course, Assignment, and workspace references locate a record but never grant
  authority.

The product behavior and its contracts are documented in
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md),
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md), and
[API_CONTRACTS.md](API_CONTRACTS.md). The serial M19 production-browser proof
is recorded separately from the service and permanent test lanes.

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

`doctor` reports reachable Podman metadata, including root mode, macOS machine
provider, and the selected Compose adapter. Missing optional metadata is a
warning rather than a runtime gate. `status` reports semantic
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
