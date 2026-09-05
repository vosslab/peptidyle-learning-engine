# Completed plan: interactive Mail.app sender for student signup emails

Archived on 2026-09-05 after the temporary attended mailer, focused offline tests,
and disposable end-to-end runner were implemented. The real Mail.app delivery
check remains operator acceptance, as this plan specifies.

Implementation kept four narrow adaptations to current repository evidence:

- the launcher lives in `launchers/send_invitations.py` instead of adding a ninth root
  script; the root `invitation_mailer/` package owns reusable behavior;
- AppleScript receives all message values as arguments, so ordinary quotes and
  backslashes remain usable while input control characters are rejected; and
- permanent pytest covers durable input, status, rendering, and interruption
  safety; the disposable E2E covers the real launcher in dry-run mode plus the
  fake-sender rerun workflow; and native Mail.app delivery remains a separate
  one-time operator check;
- the aggregate gate was attempted and its missing-`node_modules` environment blocker was
  recorded in the changelog; the full Python lane passed independently.

## Context

**The motivation, stated plainly so it stays visible: this saves money on a fall trial
run.** A hosted transactional email provider (SendGrid, Postmark, SES) costs real money to
send a few dozen messages a semester. An already-paid-for mailbox on the Mac, driven by
AppleScript against Mail.app, costs nothing. The tool exists for one semester, ahead of a
paid or built-out system later, and it is sized accordingly.

Students need a signup link in their inbox. The script pulls the students who still need
mailing, renders a message, and sends it.

The input is a file, not a database connection. A web export keeps credentials out of the
script and keeps its blast radius to one directory. That was the request and it is also the
right shape.

## Objectives

- Mail a signup link to each student in an exported file who has not already received one.
- Suppress accidental duplicates, and require a deliberate flag to resend.
- Keep every send attended and visible, with a dry run as the default.
- Leave a readable record of who was mailed and when.

## Design philosophy

Small, single-purpose, and disposable, because the motivation is cost and the lifespan is
one semester. The script is a delivery tool: it renders a message and hands it to Mail.app.
It holds no PLE authority, connects to no database, and runs only when invoked. **Perfect is
the enemy of good** from `docs/REPO_STYLE.md` applies directly, since a paid provider would
cost money to solve a problem one script solves for a semester.

Keep the tool at this size. If it needs a service, a daemon, or a schema to do its job, the
right response is to question the requirement rather than grow the tool.
`crates/server/src/application.rs:153` records `--local-invitation-delivery-worker` as an
explicitly invalid process mode, and `docs/ROADMAP.md:69` reserves **Course Invitation Email
Delivery** as a future in-product name. This tool stays outside both.

**Two tiers, two standards:**

> Build the Student Account and enrollment lifecycle to last. Build the Fall email
> delivery mechanism to work.

Account identity and persistence, Student Records, Course Membership, Instructor roster
ownership, invitation state, redemption, and recovery are foundational PLE concepts and
deserve careful design. The status log, mail-client behavior, link-rewriter quirks, the
export format, resend flags, and operator workflow need only enough engineering to make the
trial dependable and understandable.

The original instinct - "a web export if security to the database is a concern" - is the
same principle: move the boundary rather than engineer around every concern. The export
file is exactly that boundary, which is why it stays a file.

Two seams keep it adaptable without adding machinery now: the sender is an interface, so a
paid provider later becomes another implementation; and the message body comes from a
template file, so a new message kind is a new template.

- Evidence strategy for uncertain methods: mail delivery is verified against a stub
  `send_func`, the injection point the proven sender in `~/nsh/protein-image-grader/` already
  uses, so every send outcome is reproducible with no mail client and nobody present (M7).
  The one claim no fake can make is that a real message arrives, which is operator acceptance
  rather than a gate.

## Scope

- Read an exported JSON file of students, requiring `email` and a signup URL per row and a
  course name in its header.
- Skip anyone the status log shows was already mailed for that course.
- Render a plain-text message from a template.
- Send through Mail.app using AppleScript, one message at a time, with a throttle.
- Record a per-recipient status cell and a readable `sent_log.csv`.
- Document invocation, the macOS Automation grant, and the attended-operation rule.
- Carry the durable enrollment findings forward into this document so they survive the
  ignored root drafts, and close those drafts into `docs/archive/`.

## Non-goals

- Connect to the PLE database or call PLE routes.
- Generate, mint, or validate signup links; the export supplies them.
- Run unattended, on a schedule, or as a background service.
- Deliver login codes or any message the export does not carry.
- Expand `docs/TERMINOLOGY_CONTRACT.md`. The temporary mailer needs no contract expansion;
  the later durable enrollment plan owns that work, and `## Findings for the enrollment
  boundary, worth building to last` states what it must settle.
- Claim, in code, tests, docs, or the changelog, that any student was invited, enrolled, or
  activated. A local status cell records only what this tool observed.

## Manual operator contract

With the live demo stack running (`./run_live_demo.sh`, per `docs/HUMAN_GUIDANCE.md:59`),
the Instructor's whole semester mailing is three steps:

```text
1. upload the roster CSV in PLE          email,roster_id
2. export the mailing list               JSON, adds signup_url per student
3. source source_me.sh && python3 launchers/send_invitations.py output-email/roster_export.json --send
```

Two files, two formats, chosen by who touches them. A human creates the import, so it is CSV
with headers exactly `email,roster_id` (`docs/INPUT_FORMATS.md:61-77`). Only this script
reads the export, so it is JSON. The same rule keeps `sent_log.csv` a CSV: a person skims
that one.

Everything else is set up already. `invitation_mailer.yaml` is edited once at install (from
allowed domains, throttle) and read without prompting; the status log and `sent_log.csv` sit
in `output-email/` beside the export, and the course name rides in the export header, so a
new semester needs no edited file at all. macOS asks for the
Automation grant once, on the first real send. The script asks no questions and reads no state beyond the config, the export, and its
own status log.

Adding students to the export and repeating step 3 mails only the new students, so
mid-semester additions need no extra command, flag, or edited file. Dropping `--send`
prints the plan and mails nobody, which stays available for a first look at an unfamiliar
export but is not part of the normal three steps.

Steps 1 and 2 are PLE's, not this tool's. The Ribbon route contract gives them a declared
home at `/instructor/courses/:courseRef/students`, whose surface is named "Course roster,
invitations, and import" (`src/route_contract.ts:277-283`), but nothing mounts it yet
(`crates/server/src/composition.rs:64-75`). Until it lands, steps 1 and 2 come from the
registrar or an LMS export instead, and step 3 is unchanged either way, because the export
file is the boundary.

## User-facing contract

`signup_url` is the tool's one load-bearing input. Every student has their own individual
invite link; there is no shared course URL, so a row without one is an error. The script
checks that it is an absolute `https` URL and treats it as opaque after that, minting,
hashing, validating, and consuming nothing.

No such URL exists in PLE today. The Ribbon work declared the surfaces in
`src/route_contract.ts` -- `courseRoster` at `/instructor/courses/:courseRef/students`,
described as "Course roster, invitations, and import" (`:277-283`), and
`pendingCourseInvitations` at `/account/course-invitations` (`:93-99`) -- but declaring a
browser route is not mounting a server one, and
`crates/server/src/composition.rs:64-75` still mounts only `/health`, the session router,
and the seeded live-demo router. The tool is fully buildable and testable now and useful the
day links exist; supplying them is the operator's responsibility. That dependency is the
plan's top risk.

## Current state summary

The drafts were written on 2026-09-02 against a codebase that has moved. Claim-by-claim
recheck against current source:

| Draft claim | Status | Current evidence |
| --- | --- | --- |
| `course_invitation.target_account_id` is NOT NULL, so an Account must exist before inviting | Holds | `schemas/migrations/2026082914_course_memberships.sql:162-171` |
| No invitation token or credential hash column exists | Holds | same table: `invitation_id`, `course_id`, `target_account_id`, `membership_role`, `issued_at`, `expires_at` |
| The token rides in a URL fragment, so redemption needs JavaScript | Holds, and the route is unmounted | `src/api/enrollment.ts:323` still enforces `/course-invitations/redeem#token=...` |
| `ple_api.create_account` is sysadmin-gated | Superseded | replaced by `ple_api.create_instructor_account`, `schemas/migrations/2026082934_instructor_account_creation.sql:103-117`; there is no Student Account creation function at all |
| Account creation authority conflicts with roster ownership | Resolved in the contract | `docs/TERMINOLOGY_CONTRACT.md:124-127` gives Course Roster Import the resolve-or-create authority, matching `docs/HUMAN_GUIDANCE.md:190` |
| `verified_at` means two different things | Holds | `schemas/migrations/2026082903_email_challenges_and_rate_limits.sql:36`, with the role trigger at `:50-53` making Student bindings immutable |
| `updated_at` sits on a row the owner rule makes immutable | Holds | same file, `:37` and the ordering constraint at `:45-47` |
| Domain matching is specified loosely | Holds, and now has a contract neighbor | `docs/TERMINOLOGY_CONTRACT.md:337-339` names the Course Invitation Email Rule as a revisioned set of normalized domains |
| Unused provisioned Accounts have no retention stage | Holds, citation stale | that file is now 82 lines; the surviving statement is `docs/RETENTION_POLICY.md:18` |
| The Account lifecycle across semesters already works | Holds | `docs/TERMINOLOGY_CONTRACT.md:121-127` |
| No mounted route mints per-student signup links | Still true, now with a declared home | The Ribbon route contract declares `courseRoster` and `pendingCourseInvitations` (`src/route_contract.ts:93-99, 277-283`), but `crates/server/src/composition.rs:64-75` mounts only `/health`, the session router, and the seeded live-demo router; `docs/ROADMAP.md:36-38` records the absent teaching routes as the accepted current state and `:46` makes restoring them an open item |

Two contract facts that arrived after the drafts and belong to the durable plan rather than
this tool:

- `docs/ENROLLMENT_DESIGN.md:339-345` now states the future invitation link is a bearer
  secret returned only in the Instructor's no-store creation response, kept in browser
  memory for that page session, never placed in roster reads, storage, logs, or analytics,
  with the server storing only its hash. When PLE begins minting per-student links, that
  rule governs how they reach students, and an exported file of live one-time links is not
  the sanctioned path. It does not constrain this tool today, which mails whatever link the
  export carries.
- `docs/TERMINOLOGY_CONTRACT.md:121-127, 326-347` now owns Student Account, Student
  Authentication Email, Course Roster Import, Course Invitation and Course Invitation State,
  Course Invitation Email Rule, Course Enrollment, Course Membership and its role, and
  Student Record. This is the coverage the terminology review expected, so the mailer keeps
  `signup_url`, `status_log`, and its other mechanics as local tool vocabulary and takes no
  canonical term.

Repository shape this plan must fit:

- Gates are `./check_rust.sh`, `./check_codebase.sh`,
  `source source_me.sh && python3 -m pytest tests/`, and
  `source source_me.sh && python3 local_stack.py acceptance`, run in that order by
  `launchers/all_test.sh`.
- `tests/conftest.py:9-14` inserts the repo root onto `sys.path`, so a repo-root package is
  importable from tests with no configuration, exactly as `local_stack_control/` is.
- Non-browser E2E runners register in `tests/e2e/e2e_run_all.sh` through its `run_check` helper.
- `PyYAML` is already the one runtime dependency in `pip_requirements.txt`, so the dependency
  set is unchanged.
- Bandit runs at medium severity and medium confidence (`tests/test_bandit_security.py:51-61`),
  so a fixed-argv `subprocess.run` without a shell stays clean.

## Milestone plan

Ten milestones, each finishable and verifiable by a manager and subagents with no human in
the loop. Every done check is a command that exits zero or nonzero. The real send is
deliberately outside this list; see `## Operator acceptance, outside the completion path`.

Three lanes open at once after M1 and converge at M8: config (M2), export and identity
(M3-M5), and message and sender (M6-M7).

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M1 | Package skeleton | `launchers/send_invitations.py` and the package, importable and lint-clean | A repo-shaped foundation the other milestones land in |
| M2 | Config | Load and validate `invitation_mailer.yaml` | A drifted config fails before any send |
| M3 | Export read | Parse and validate the JSON mailing list | A malformed export never reaches the sender |
| M4 | Recipient identity | Normalization, domain allowlist, dedup key | One student is one recipient, scoped to one course |
| M5 | Status log | Atomic JSON cells, pending filter, recovery | A rerun mails nobody twice |
| M6 | Message rendering | Template substitution and name safety | The body is correct and cannot break composition |
| M7 | Sender | The `send_func` seam, its dispatch, throttle and progress | Every send outcome is reproducible offline |
| M8 | Command surface | Argparse, dry-run default, summary, `sent_log.csv` | The batch runs from one command |
| M9 | End-to-end runner | Shell runner and `run_check` registration | The real launcher and rerun workflow are gated |
| M10 | Documentation | Usage, requirements, changelog | The tool is documented and its claims stay honest |

Each milestone is one owner, one outcome, one command. `pytest` invocations assume
`source source_me.sh &&` in front, per `docs/PYTHON_STYLE.md`.

| M | Depends on | Deliverable | Done check | Parallel |
| --- | --- | --- | --- | --- |
| M1 | none | `launchers/send_invitations.py` entry point with `main()`, the `invitation_mailer/` package, shebang plus executable bit | `pytest tests/test_pyflakes_code_lint.py tests/test_shebangs.py tests/test_function_typing.py` passes | yes |
| M2 | M1 | `invitation_mailer.yaml` plus its loader in `input.py`: `allowed_recipient_domains` and throttle seconds, every key required, with `output-email/` as the fixed working directory | `pytest tests/test_invitation_config.py` passes, covering a valid config and a missing or malformed key | yes, with M3 and M6 |
| M3 | M1 | JSON export reader: `course_name` header, `students` array, `email` and `signup_url` required, `display_name` and `roster_id` optional, absolute-`https` links, in-file duplicates rejected | `pytest tests/test_invitation_export.py` passes, covering a clean file and each rejection | yes, with M2 and M6 |
| M4 | M3 | `normalize_address` with `lower(strip())`, the full-domain allowlist check after the final `@`, and `dedup_key(course_name, normalized_email)` | `pytest tests/test_invitation_recipient.py` passes, including a subdomain-suffix lookalike rejected and two courses yielding two keys | no |
| M5 | M4 | `status_log.py`: atomic JSON load and save through `tempfile` plus `os.replace`, the closed status set, the pending filter, `indeterminate` held and cleared by `--force-resend` | `pytest tests/test_invitation_status_log.py` passes | no |
| M6 | M1 | `templates/invitation.txt` and rendering, URL alone on its line with no trailing punctuation, quotes and backslashes preserved as AppleScript arguments, and control characters rejected | `pytest tests/test_invitation_sender.py` passes, including an unknown placeholder raising | yes, with M2 and M3 |
| M7 | M6 | The `send_func` seam and its fake, the `py-applescript` dispatch behind it, the throttle, and one progress line per message | `pytest tests/test_invitation_sender.py` passes across success, failure, and a raised exception, without importing `applescript` | no |
| M8 | M2, M5, M7 | `launchers/send_invitations.py`: export path positional, `--dry-run` default with `--send`, plus `--limit`, `--only`, `--force-resend`, the closing summary, and the `sent_log.csv` projection | Focused pytest and the disposable E2E pass their classifications in `## Verification` | no |
| M9 | M8 | `tests/e2e/e2e_invitation_mailer.sh` and its `run_check` line in `tests/e2e/e2e_run_all.sh` | `bash tests/e2e/e2e_invitation_mailer.sh` passes its launcher and rerun cases in `## Verification` | no |
| M10 | M9 | `docs/USAGE.md`, `py-applescript` in `pip_requirements.txt`, `docs/CHANGELOG.md` | Markdown links pass; `./launchers/all_test.sh` is attempted and any unrelated blocker is recorded | no |

## Operator acceptance, outside the completion path

The plan is complete when M10's done check passes. One real send stays valuable and is not a
milestone, because a plan that waits on a person is a plan that stalls overnight: mail two
messages to your own address with `--send` and confirm they arrive and that the link is
clickable on a phone. Record it in `docs/CHANGELOG.md` when it happens.

Mail.app account selection needs no verification: exactly one account is configured,
intentionally, so the tool sets no sender property and cannot pick the wrong one.

## Sender seam and status log

`~/nsh/protein-image-grader/` already mails a class every semester, so read it for what it
learned the hard way rather than for code to lift. Three ideas earn their way in here:

- **An injected `send_func` is the sender seam.** `applescript_dispatch.py` states the rule
  in its own docstring: tests never import `applescript`; they inject a fake. That one
  injection point is what makes every send outcome reproducible with no mail client and
  nobody present, and it is why this plan needs no stub binary or captured fixture.
- **Current-state-only status, written atomically.** `email_log.py` keeps one cell per
  student with no history list and writes through `tempfile` plus `os.replace`, so a crash
  mid-write cannot truncate the log. Same here.
- **A closed status set that raises on anything else.** `sent`, `failed`, `dry_run`, and
  `indeterminate` for a run that died between dispatch and recording.

Three deliberate departures. The log is JSON, not YAML, by the same rule that picked the
export format: only this script writes or reads it, so it gets the format that holds exact
strings rather than the one that is pleasant to hand-edit. Cells key on
`(course_name, normalized_email)` rather than student ID and image number, since one student
may legitimately be mailed for two courses. Unlike that older `compose_script`, this tool
passes message values as AppleScript arguments. Quotes and backslashes therefore remain
ordinary message text, while the input boundary rejects control characters; M6 owns that
check.

## Files to modify

Mirrors the existing `local_stack.py` plus `local_stack_control/` entry-point-and-package
pattern, with four small modules rather than a module per concern:

```
launchers/send_invitations.py           # thin user-facing entry point
invitation_mailer/
  cli.py                                # argparse, root-owned paths, progress summary
  input.py                              # config load and validation, export read,
                                        #   address normalization, link validation
  status_log.py                         # JSON status cells, atomic write, pending filter
  sender.py                             # template rendering, dispatch through an injected send_func
  templates/invitation.txt
invitation_mailer.yaml
output-email/                           # export in, status log and sent_log.csv out;
                                        #   already covered by the /output*/ gitignore rule
tests/test_invitation_*.py              # fast pytest lane
tests/e2e/e2e_invitation_mailer.sh      # launcher dry run and fake-sender workflow
tests/e2e/e2e_run_all.sh                # register the runner through run_check
pip_requirements.txt                    # add py-applescript
docs/USAGE.md                           # invocation, Automation grant, attended rule
docs/CHANGELOG.md                       # one dated entry
```

Read before starting, for ideas rather than code:
`~/nsh/protein-image-grader/protein_image_grader/applescript_dispatch.py` and `email_log.py`,
for the send seam and the atomic current-state log. Read for shape: `local_stack.py` plus
`local_stack_control/` for the entry-point-and-package pattern; `tests/e2e/e2e_run_all.sh`
for runner registration; `tests/conftest.py` for the `sys.path` insert.

These four modules are responsibility boundaries, not a size budget: command composition,
what comes in, what is recorded, and what goes out. Start here, and split a module when
implementation reveals a
distinct responsibility that improves navigation or maintainability. Treat the initial module
count as a starting point rather than a constraint to preserve, and keep every file under the
999-line cap in `docs/REPO_STYLE.md`.

`py-applescript` is the one added dependency, already standard on this machine per
`docs/PYTHON_STYLE.md`. The import-name mismatch is already handled:
`tests/test_import_requirements.py:32` maps `applescript` to `py-applescript`, so declaring
it in `pip_requirements.txt` satisfies that gate with no test edit. PyYAML, which the config
needs, is already declared; the status log and the export use `json` from the standard
library.

Every file this tool reads or writes lives in `output-email/`: the operator drops the export
there, and the status log and `sent_log.csv` are written beside it. The root-anchored
`/output*/` rule already in `.gitignore:6` covers all three, so no ignore rule is added.
A hyphen is fine here because nothing imports this directory; `invitation_mailer/` keeps its
underscore precisely because Python does import it.

## Verification

Fast pytest per `docs/PYTEST_STYLE.md`, covering the failures that would actually cost
something. Write inputs inline, using `tmp_path` for anything file-shaped:

- The export reader accepts a clean file and rejects a missing `course_name`, a missing
  `signup_url`, an address outside the allowlist, an in-file duplicate, and a non-`https`
  URL. Wrong-recipient and malformed-input failures are the expensive ones, so this is the
  test that earns its permanent place.
- Address normalization collapses case variants to one student, and the same student in two
  courses keys to two distinct cells.
- The pending filter returns exactly the recipients without a closing status.
- Rendering substitutes fields, raises on an unknown placeholder, and preserves ordinary
  punctuation in display names while the input boundary rejects control characters.
- The sender, driven against a fake `send_func`, records `sent` on success, `failed` with a
  bounded message on a returned failure, and `failed` rather than a crash when the dispatcher
  raises.

Disposable end-to-end evidence, run as `bash tests/e2e/e2e_invitation_mailer.sh`:

- The real launcher defaults to dry run, honors `--limit 1`, and exits successfully without
  contacting Mail.app.
- Three rows send three; a rerun sends zero; a fourth row sends exactly one.
- `--force-resend` on an already-mailed address sends one and records a deliberate resend.

Permanent offline pytest owns input validation, duplicate suppression, rendering,
per-recipient failure continuation, interruption/`indeterminate` handling, dry-run sender
isolation, and the narrow `--force-resend` argument guard. This keeps stable safety behavior
in pytest while whole-command sequencing stays in the disposable E2E lane.

Repository gates, in order: `source source_me.sh && python3 -m pytest tests/`, then
`bash tests/e2e/e2e_run_all.sh`, then `./launchers/all_test.sh` once before the changelog entry lands.
New Python files pass the hygiene lane already in place: tabs
(`tests/test_indentation.py`), ASCII (`tests/test_ascii_compliance.py`), pyflakes,
annotations on every parameter and return (`tests/test_function_typing.py`), shebang and
executable-bit agreement (`tests/test_shebangs.py`), absolute imports
(`tests/test_import_dot.py`), declared dependencies (`tests/test_import_requirements.py`),
and the 999-line cap.

The real send is operator acceptance rather than a gate; see
`## Operator acceptance, outside the completion path`.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| No PLE signup link exists to mail | The tool is finished but idle | Fall term arrives before any student entry route is restored | M10 | The link is an input, so the tool ships complete and waits; `docs/USAGE.md` states that supplying a working link is the operator's responsibility |
| Interrupted run leaves an ambiguous outcome | One student may miss a link | Crash between dispatch and recording the status | M5 | Record `indeterminate` and hold; `--force-resend` is the recovery path |
| Export holds live signup links | A leaked file grants course seats | File kept after mailing | M10 | Mode `0600`, gitignored, and `docs/USAGE.md` names when it is safe to delete |
| Faculty-sent link reads as phishing | Students ignore or filter the mail | First batch arrives unannounced | M10 | `docs/USAGE.md` records announcing the sender address and subject beforehand |
| Institutional sending limits | A large batch stalls partway | Many sections mailed at once | M7 | Throttle between messages; class-sized batches are small enough that the limit is unlikely to matter, and the per-message progress line shows where a stall happened |
| Tool vocabulary leaks into PLE contract terms | A disposable mechanic acquires durable authority | A status field or module is named after a canonical term | M10 | Local names only (`signup_url`, `status_log`); the changelog entry states the tool observes dispatch and claims no Course Invitation, acceptance, or Course Enrollment |

## Findings for the enrollment boundary, worth building to last

Planning this script exposed something more consequential than the mailer: PLE does not yet
have a coherent enough Student Account lifecycle to produce signup links correctly. That is
durable domain work, and it belongs to the "build to last" tier - a plan of its own, landing
in `docs/DESIGN_DECISIONS.md` and `docs/ENROLLMENT_DESIGN.md`.

Carrying it out is what makes per-student links possible; this mailer runs on whatever link
the export carries until then.

These are findings, not decisions. Where a direction looks promising it is marked as such,
and the durable plan makes the call. This temporary tool's plan is not the authority for
enrollment architecture.

- **PLE mounts no student entry surface today.** `crates/server/src/composition.rs:64-75`
  mounts `/health`, the session router, and the seeded live-demo router. Restoring the
  Course, authoring, delivery, grading, Gradebook, and administration route surface is an
  open roadmap item (`docs/ROADMAP.md:46`), and `docs/HUMAN_GUIDANCE.md:57` records that
  email is not configured for the Live Demo yet.
- **The declared route contract routes a student through sign-in, not through a redemption
  page.** The Ribbon work declares 24 browser routes in `src/route_contract.ts`, including
  `signIn` at `/sign-in` (`:86-92`) and `pendingCourseInvitations` at
  `/account/course-invitations` (`:93-99`), and declares no redemption page at all, while
  `src/api/http_client/enrollment.ts:97` still posts to `/api/course-invitations/redeem`.
  That points at a design where a student authenticates first and then claims a pending
  invitation from their own account page, which would make the emailed link a pointer to
  sign-in rather than a bearer credential. Promising direction, and the durable plan's call.
  This tool is unaffected either way, because the link is opaque to it.
- **Invitations require an existing Account.**
  `ple_private.course_invitation.target_account_id` is `NOT NULL REFERENCES
  ple_private.account` (`schemas/migrations/2026082914_course_memberships.sql:165`), so
  provisioning necessarily precedes inviting. Inviting a bare email address is not
  expressible today.
- **No invitation credential column exists.** `course_invitation` carries no hash, while
  `docs/ENROLLMENT_DESIGN.md:342` states the server "must store only its hash". A
  link-bearing credential needs one added, following the pattern at
  `schemas/migrations/2026082903_email_challenges_and_rate_limits.sql:75`.
- **The token rides in a URL fragment.** `src/api/enrollment.ts:323` enforces `#token=...`,
  which never reaches the server, so redemption depends on JavaScript reading
  `location.hash`. Promising direction: a query-string token, which works without
  JavaScript, though it carries its own exposures in logs and history.
- **The invitation link is now a no-store bearer secret.**
  `docs/ENROLLMENT_DESIGN.md:339-345` returns it only in the Instructor's no-store creation
  response and keeps it out of roster reads, storage, logs, and analytics. The durable plan
  decides how a link reaches a student under that rule: an Instructor copying it into an
  LMS, or the future Course Invitation Email Delivery capability reserved at
  `docs/ROADMAP.md:69`.
- **Automated mail processing can fetch links before a person clicks.** Link protection and
  scanning services follow URLs in delivered mail. The durable requirement is that a GET must
  not consume a one-time credential; consumption belongs on an explicit action. This holds
  regardless of any particular scanner's current behavior.
- **Roster ownership of account creation is now settled, and unimplemented.**
  `docs/TERMINOLOGY_CONTRACT.md:124-127` gives Course Roster Import the authority to resolve
  a normalized email to an existing Student Account or create one, matching
  `docs/HUMAN_GUIDANCE.md:190`. No schema or Store owns that operation yet; the only account
  creation function is `ple_api.create_instructor_account`
  (`schemas/migrations/2026082934_instructor_account_creation.sql:103`), which is
  Sysadmin-gated and Instructor-only.
- **`verified_at` means two different things.** On a roster-provisioned
  `account_authentication_email` row the registrar asserted the address and the student
  proved nothing, which differs from a completed ceremony
  (`schemas/migrations/2026082903_email_challenges_and_rate_limits.sql:36`). Worth an
  explicit decision, since email sign-in still proves possession at each login.
- **`updated_at` sits on a row the owner rule makes immutable.** Student email addresses are
  immutable (`docs/HUMAN_GUIDANCE.md:187`, enforced by the role trigger at
  `schemas/migrations/2026082903_email_challenges_and_rate_limits.sql:50-53`), so
  `updated_at` and its ordering constraint describe an event that cannot occur. What to do
  with the column belongs to the durable plan.
- **Domain matching is specified loosely.** `docs/TERMINOLOGY_CONTRACT.md:337-339` names the
  Course Invitation Email Rule as a revisioned set of normalized domains without fixing the
  match. Substring-style matching would accept
  `student@mail.roosevelt.edu.attacker.example`. Promising direction: exact full-domain
  equality, noting the consequence that `roosevelt.edu` would then not admit
  `akirpane@mail.roosevelt.edu`, so a course lists both.
- **Unused provisioned Accounts have no retention stage.** `docs/RETENTION_POLICY.md:18`
  keeps the course-scoped graph on the course schedule while the global Account survives by
  design, so a student who never signs up leaves an Account nothing removes.
- **The Account lifecycle across semesters already works.** Confirmed rather than broken: a
  returning student resolves by institutional email to the same Account
  (`docs/TERMINOLOGY_CONTRACT.md:121-127`), and course records come and go under their own
  retention schedule.

### Terminology the durable plan must settle

The temporary mailer needs no terminology-contract expansion. The later durable enrollment
plan becomes terminology-ready when the invitation credential, explicit claim, delivery
handoff, and email-evidence distinctions have exact owners and names. The contract already
carries Student Account, Student Authentication Email, Course Roster Import, Course
Invitation and Course Invitation State, Course Invitation Email Rule, Course Enrollment,
Course Membership and Course Membership Role, and Student Record, so add no generic Student
Activation state unless implementation reveals a real, separately owned lifecycle requiring
one.

The naming must preserve these boundaries:

- Sending or handing off email does not accept a Course Invitation.
- Retrieving a link does not consume its one-time credential.
- Course Invitation acceptance and Course Enrollment are related but distinct facts.
- A roster assertion and successful passwordless email authentication are different
  evidence, even when they name the same Student Authentication Email.

Prefer qualified names such as **Course Invitation Claim** and **Course Invitation
Credential** if those become durable cross-layer concepts. Keep `signup_url`,
`status_log`, and similar names local to the temporary tool.

## Open questions and decisions needed

- Non-blocking: the export's exact column names, which follow whatever the web export
  produces. The reader requires `email` and `signup_url` and treats the rest as optional, so
  a differing header costs a config line rather than a redesign.
- Blocking for use, not for build: every student needs their own `signup_url`, and PLE mints
  none today because no student entry route is mounted. The tool reaches its own completion
  without them; the fall workflow does not.
- Non-blocking: whether a future per-student one-time link reaches students through an
  Instructor copy-out or the reserved Course Invitation Email Delivery capability is the
  durable plan's call. Either way this tool is unaffected, because the link is an input.
