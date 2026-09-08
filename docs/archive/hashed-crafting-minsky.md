# Live Demo teaching-data baseline: connect Students, Course, and Assignment

## Context

The Live Demo's five personas exist but are strangers to each other. `local_stack_control/live_demo_seed.py`
installs exactly five Accounts, three Student Authentication Emails, and four Published Questions
(PNE-0001..0004, authored by Elena). It creates no Blueprint Course, Course Instance, roster, Student
Record, or Assignment -- a boundary `docs/LOCAL_STACK_OPERATIONS.md` lines 142-146 states outright.

So "Continue as Mary Student" lands on an empty course list, and the seeded peptide questions are
reachable only through Elena's Question Library. Courses are built today only inside e2e scripts,
never by an operator starting the demo.

`docs/ROADMAP.md` lines 98-104 already names the target: step 2 of the live-demo lifecycle is "a
data-only host installation creates the known fictional, disposable teaching-data baseline." This
plan implements that step for the course domain, and closes one product gap it depends on.

The deeper goal is that the five personas stop being isolated Accounts and become a seeded teaching
graph that demonstrates how Peptidyle is actually used:
**instructor -> course -> roster -> assignment -> questions -> student work.**

Target state after `./launchers/run_live_demo.sh`, with no further commands:

| Account | Course state | Assignment state | Demo purpose |
| --- | --- | --- | --- |
| Elena Rivera, Instructor | Instructor for the seeded Course Instance | Released Assignment with PNE-0001..0004 | Instructor workflow |
| Mary Okafor, Student | Active enrollment | Completed and scored | Gradebook, completed work, Student results |
| Jack Nguyen, Student | Active enrollment | Started, in progress | Active attempt and unfinished work |
| Avery Thompson, Student | Active enrollment | Not started | Fresh Student experience |
| Morgan Delgado, Sysadmin | Administrative only | N/A | Sysadmin workflow |

Elena opens the Gradebook and immediately sees three meaningful states side by side: completed and
scored, in progress, and not started. Mary and Jack show what the course looks like after use; Avery
is the clean Student Account for demonstrating an Assignment from the beginning.

The Blueprint Course and the Course Instance are deliberately both present, so the distinction between
reusable curriculum and one taught section is visible in the demo rather than implied.

**Why all three Students are enrolled.** A Student Account is global and persists across courses and
semesters, but it is deliberately FERPA-plain -- no profile, no standing content of its own. So a
seeded Student with no course participation demonstrates nothing: signing in would show an empty course
list and offer no action. All three seeded Students participate in Elena's course, and they differ by
how far through the work they have got, which is what actually distinguishes real students.

The two layers divide as follows: the disposable SQL seed precreates the three fictional Accounts so
visible-role sign-in is deterministic, and the roster-import stage then exercises the normal
Account-email resolution and course-scoped Student Record and enrollment path.

**The baseline represents an already active fictional course.** It seeds representative Student work,
created through the normal product workflows rather than manufactured as database rows. That makes
live-demo startup a genuine end-to-end exercise of course -> enrollment -> assignment -> attempt ->
responses -> scoring, which is worth considerably more than populating the interface would be.

Everything here is fictional, so "avoid invented Student activity" is not a useful objective. The
FERPA principle that does apply is that the Student model stays appropriately minimal, and it does.

## Purpose: the demo is launch-readiness evidence

The Live Demo exists to answer one question -- is PLE ready to launch? That makes it an instrument of
evidence, not a presentation. Two consequences follow, and they govern every decision below.

**Course-domain state is established through normal product contracts, never by direct database
insertion.** A course assembled by inserting rows would prove only that rows can be inserted. Because
every course-domain record is instead produced by the same routes a real Instructor and real Students
use, a working demo is a passing end-to-end test of course creation, enrollment, assignment authoring,
release, delivery, submission, and grading. If any of that is broken, startup fails loudly and the
answer is "not yet" -- which is exactly the answer worth having.

This rule governs the course domain. The foundational SQL seed keeps its existing charter of Accounts,
Authentication Emails, and Published Questions, and needs no redesign.

**The demo must be walkable end to end.** Elena opens a course that looks used, sees her roster and
Gradebook, and can act; Mary sees completed graded work; Jack resumes; Avery starts fresh. If any step
is empty or dead, the verdict is visible immediately rather than hidden behind a persona that signs in
and does nothing.

**Primary success criterion.** After `./launchers/run_live_demo.sh`, the human reviewer can use the
seeded personas to exercise the major launch-critical Instructor and Student workflows and decide
whether PLE is ready for real users. The seeded data exists to expose those workflows and their
accumulated state, not to create a parallel demonstration model.

The baseline is judged by which launch-critical paths it puts under realistic state:

- course ownership and roster state
- released Assignment delivery
- completed graded work
- in-progress work and resume
- untouched work from a fresh Student's perspective
- Gradebook aggregation across those states
- real Question presentation, submission, and grading
- authorization boundaries between Instructor, Student, and Sysadmin

Anything that exists only to make the demo look good without proving one of those is suspect and
should be cut. Anything that exposes a launch-critical path under realistic state earns its keep even
when it makes provisioning more involved.

## Architecture decision: two seed layers, one write path

The SQL seed keeps its narrow charter -- foundational Accounts, Authentication Emails, and Published
Questions. Course-domain state is built exclusively through the product's own HTTP routes, which
reach the `SECURITY DEFINER` functions in `ple_api` that own membership event-sourcing, audit rows,
invitation lifecycle, assignment edit-number concurrency, and release immutability.

Record this as a durable rule in `docs/DESIGN_DECISIONS.md`, not merely as how this implementation
happens to work: **course-domain demo data is created through product contracts; the SQL seed installs
only foundational records that predate any course.** A second implementation of membership or release
semantics beside `ple_api` would be the failure this rule prevents.

### Resolved from the code (previously open)

- **Elena creates her own Course Instance.** `course_creator_session_hash`
  (`crates/server/src/course_instance.rs:152-160`) admits Instructor or Sysadmin, and
  `CreateCourseInstanceInput::assigned_instructor` is `Option`, documented at
  `crates/learning-data-access/src/course_instance.rs:28-30` as "omitted by an Instructor creating for
  self." Morgan and the account-reference lookup are both unnecessary.
- **Claiming is the correct mechanism and is self-convergent.**
  `ple_api.claim_live_demo_course_invitation` (`schemas/migrations/2026090605_live_demo_course_roster.sql:388-444`)
  resolves the student account from the session and returns `true` immediately when an active student
  membership already exists. `import_live_demo_course_roster` resolves accounts by normalized email,
  which the seeded Authentication Emails satisfy exactly.

### Convergence contract

Provisioning is a sequence of stages, each a *detect* read followed by an *apply* write performed only
when detect reports the stage incomplete. Rerunning after any interruption resumes; no stage assumes a
previous run finished. This replaces the weak "does a course with this title exist" early return.

| Stage | Detect | Apply |
| --- | --- | --- |
| `blueprint` | `GET /api/course-blueprints`, manifest reference else fixed title | `POST /api/course-blueprints` |
| `course` | `GET /api/course-instances` as Elena, manifest reference else fixed title | `POST /api/course-instances` (no `assignedInstructor`) |
| `roster` | `GET /api/course-instances/{c}/roster`, all three roster ids present | `POST .../roster` with Mary, Jack, and Avery (idempotent by contract) |
| `claims` | roster projection reports `activeStudent` for all three | each Student `POST .../roster/claim` (returns `true` when already active) |
| `assignment` | `GET .../assignments` (added by M3), manifest reference else fixed title | `POST .../assignments` |
| `selection` | `GET .../assignments/{a}` title, instructions, and question ids match the declared baseline | `PUT .../assignments/{a}` with `If-Match` edit number, which carries all three |
| `release` | assignment status is `released` | `GET .../release-validation` then `POST .../release` |
| `attempts` | Mary and Jack each hold an Assignment Attempt; Avery holds none | Mary and Jack `GET .../access` then `POST .../start` |
| `work` | Mary has a terminal graded submission per issued Question; Jack has two | submit per Question through `POST .../presentations/{nonce}/submissions`, then poll each nonce status to a terminal grading state |

### Identity: machine key first, title second

Detecting by human-facing title alone is the weak point of a convergent design: renaming the course
would silently provision a second one. Titles are presentation data and must stay editable.

So provisioning records the references it created -- `BP-n`, `C-n`, `A-n` -- in the disposable
workspace manifest at `local_stack_state/live_demo_browser/workspace/`, alongside the state the
controller already keeps there, and resolves them from that manifest first. Title matching is the
fallback used only when the manifest has no record, which is exactly the case where the manifest was
cleared but the database volume survived. A resolved reference that no longer exists in the product
falls back to title matching too, so a database reset with a stale manifest converges rather than
failing.

Both stores are disposable and are removed together by
`local_stack.py reset --confirm-project containers`, so they cannot drift permanently.

### What converges, and what does not

State the guarantee at exactly the strength the product routes support, so nobody relies on more:

- **Converges.** A partially provisioned stack, at any stage boundary. Missing roster members, missing
  claims, missing attempts, and unanswered Questions. A changed Assignment title, instructions, or
  question selection, because `PUT .../assignments/{a}` carries all three and the `selection` stage
  compares all three.
- **Does not converge, by absence of a supported route.** A renamed Blueprint Course or Course
  Instance: manifest-first identity means the existing object is found rather than duplicated, but no
  update route exists to rename it, so the recorded object keeps its original title. A roster entry
  removed from the declared baseline likewise stays enrolled; `POST .../roster/{id}/revoke` exists and
  could be adopted later, but revoking a Student on a config edit is a decision worth making
  deliberately rather than inheriting.

For those cases the documented remedy is the one the disposable demo already has:
`local_stack.py reset --confirm-project containers`, then start again. Say so in
`docs/LOCAL_STACK_OPERATIONS.md` rather than implying full convergence.

## Milestones

Ten milestones, M0 through M9, each independently dispatchable to one subagent, each ending in a command the
manager runs unattended. No milestone requires a person to look at anything.

Shared preconditions for every milestone that touches a running stack:
`source source_me.sh && python3 local_stack.py start --headless` brings up the fixed HTTPS stack
without a browser; every e2e script below reads its gateway port from
`local_stack_state/live_demo_browser/workspace/env.local`.

---

### M0 -- Persona identity and course fiction

Owner: one coder subagent. Runs before M1 because it changes the seeded emails M1 reads.

The personas currently carry role-suffixed placeholder display names ("Elena Instructor") and
role-suffixed emails (`mary.student@live-demo.invalid`). `docs/INSTRUCTOR_PAGE_VISUALS.md:17` already
establishes **Elena Rivera** and **Mary Okafor** as the fictional identities; this milestone finishes
that work so the demo reads as a real class rather than a fixture.

The persona wire keys (`elenaInstructor`, `maryStudent`, `jackStudent`, `averyStudent`,
`morganSysadmin`) are the API contract and stay exactly as they are. Only human-facing text changes.

| Persona | Display name | Seeded email |
| --- | --- | --- |
| `elenaInstructor` | Elena Rivera | (no Authentication Email; Instructor) |
| `maryStudent` | Mary Okafor | `mary.okafor@live-demo.invalid` |
| `jackStudent` | Jack Nguyen | `jack.nguyen@live-demo.invalid` |
| `averyStudent` | Avery Thompson | `avery.thompson@live-demo.invalid` |
| `morganSysadmin` | Morgan Delgado | (no Authentication Email; Sysadmin) |

Every address keeps the `.invalid` TLD. That is a safety property, not a placeholder:
`launchers/send_invitations.py` performs real attended mail delivery, and `.invalid` is guaranteed
non-routable, so seeded fiction can never reach a real inbox.

Course fiction, consumed by M1 as constants:

- Blueprint Course: `Biochemistry 301: Proteins and Peptides`
- Course Instance: `Biochemistry 301: Proteins and Peptides`, Fall 2026 term `2026-08-24` to
  `2026-12-11`, `America/Chicago`
- Assignment: `Peptide Structure Practice`, instructions
  `Complete the four practice questions on peptide structure and properties.`, no due date, late work
  accepted
- Roster ids are stable course-scoped identifiers, not email addresses: `BIO301-MARY`, `BIO301-JACK`,
  and `BIO301-AVERY`

Files: `crates/server/src/composition.rs:36-62` (display names),
`src/pages/live_demo_auth_model.ts:12-25` (per-persona blurbs, reworded to the new names),
`local_stack_control/live_demo_seed.py:85-89` (`SEEDED_STUDENT_AUTHENTICATION_EMAILS`).

Callers that must move with the rename: `tests/test_live_demo_transport.mjs:30-34,64-65,92`
(expected persona/displayName pairs), the `Continue as ...` selectors across
`tests/playwright/e2e_live_demo_*_browser.mjs` and `tests/playwright/e2e/real_stack_ui.ts:89`, the
hardcoded `mary.student@live-demo.invalid` in `tests/e2e/e2e_live_demo_assignment_attempt.sh` and
`tests/e2e/e2e_live_demo_roster.sh`, and `crates/server/src/composition.rs:590-640` tests. Search for
the literal old strings; leave none behind.

Screenshots under `docs/screenshots/` show the old display names. Regenerate them in M9 with
`./devel/capture_screenshots.sh`, which is scripted and needs no attendance.

Success: the sign-in page offers five fully named people, and no old display name or role-suffixed
email survives anywhere in the tree.
Validation: `source source_me.sh && pytest tests/`, `node --test tests/test_live_demo_transport.mjs`,
and `bash tests/e2e/e2e_live_demo_roster.sh --import` on a restarted stack.

---

### M1 -- Baseline contract and convergence planner

Owner: one coder subagent.

**The baseline contract comes first.** Write the authoritative definition of the five personas and the
PLE records that exist after startup into `docs/LIVE_DEMO_SPEC.md` before any code, so it lives in one
durable place instead of being reconstructed from the provisioner. Everything downstream -- the
executor, the E2Es, the browser journey, the screenshots -- cites that section rather than restating it.

It specifies actual PLE objects and relationships, not a scripted showcase. Phrases like "completed
and scored", "in progress", and "not started" belong in that document and in acceptance criteria as
descriptions of what a person sees. They are deliberately **not** identifiers the software implements:
introducing them into the provisioner would build a second assignment model beside PLE's own, and the
whole value of this work is that Mary, Jack, and Avery differ only because their real records differ.

Each Student is therefore declared in real assignment-domain terms:

| Student | Declared product facts |
| --- | --- |
| Mary | Enrolled; one Assignment Attempt; four submissions; terminal grading results |
| Jack | Enrolled; one open Assignment Attempt; two submissions; remaining Questions unanswered |
| Avery | Enrolled; released Assignment available; no Assignment Attempt |

The interface and the Gradebook derive whatever labels PLE normally derives from those facts.

New `local_stack_control/live_demo_course_seed.py`, pure and I/O-free, mirroring the style of
`live_demo_seed.py`:

- Frozen dataclasses and constants: `SEEDED_BLUEPRINT_TITLE`, `SEEDED_COURSE_TITLE`,
  `SEEDED_ASSIGNMENT_TITLE`, instructions text, and `SEEDED_COURSE_TERM`
  (`2026-08-24`, `2026-12-11`, `America/Chicago`) -- the single Fall 2026 fiction used everywhere.
- `SEEDED_ROSTER_ENTRIES`: all three Students -- `BIO301-MARY`, `BIO301-JACK`, `BIO301-AVERY` -- with
  emails taken from `live_demo_seed.SEEDED_STUDENT_AUTHENTICATION_EMAILS` rather than retyped. All
  three enroll; they differ only in how far they have got, which M6 establishes.
- `SEEDED_STUDENT_WORK`: the number of issued Questions each Student answers -- Mary all four, Jack
  two, Avery none -- kept beside the roster so the declared product facts sit in one place. A Student
  who answers none starts no attempt.
- Question ids derived from `live_demo_seed.SEEDED_PUBLISHED_QUESTIONS`, so the layers cannot drift.
- Payload builders returning `dict`: `blueprint_payload()`, `course_payload()`, `roster_payload()`,
  `assignment_save_payload(question_ids)`. `dueAt` is `null` and `lateWorkRule` is `accept`, so no
  baseline fact expires with the wall clock.
- `ObservedState` dataclass (what the detect reads found) and
  `plan_stages(observed) -> tuple[Stage, ...]` returning exactly the stages still needing work.

New `tests/test_live_demo_course_seed.py` -- covers `plan_stages`, which is real branching logic that
could plausibly regress, rather than constant-copying:

- A fully provisioned `ObservedState` plans no stages.
- An `ObservedState` with the course present but no claims plans `claims` onward and not `blueprint`
  or `course`.
- An `ObservedState` where Mary has answered two of four plans `work` and nothing earlier, proving the
  planner resumes partial work rather than restarting it.

Success: `plan_stages` is total over the observed states the executor can produce.
Validation: `source source_me.sh && pytest tests/test_live_demo_course_seed.py`.

---

### M2 -- Gateway request and session helpers

Owner: one coder subagent.

Extend `local_stack_control/live_demo_gateway.py`, which already owns gateway origin and TLS policy
(`gateway_url`, `health_probe_argv`, `seeded_session_probe_argv`):

- `persona_session_argv(url, persona, cookie_jar_path)` -- POSTs the persona to
  `/api/auth/live-demo/accounts`, writing the session to a cookie jar.
- `demo_request_argv(url, path, cookie_jar_path, method, body, if_match)` -- first-party `Origin`,
  JSON content type, optional `If-Match`, `--write-out` of the status code.

Session cookies live only in mode-0600 cookie-jar files under the private workspace directory (see
`local_stack_control/private_files.py`) and are removed when provisioning ends.

New `tests/test_live_demo_gateway_requests.py`: no session token or cookie value appears in any
returned argv -- a real security invariant, since `ControllerError` messages quote argv.

Success: every request the executor needs is expressible through these two builders.
Validation: `source source_me.sh && pytest tests/test_live_demo_gateway_requests.py`.

---

### M3 -- Instructor Assignment list capability (product gap)

Owner: one expert coder subagent. This is real product work, not scaffolding: an Instructor currently
has no route that lists her own Course Instance's Assignments. Only the student-scoped
`/api/course-instances/{course}/assignment-landing` exists
(`crates/server/src/live_student_course_landing.rs:129-141`). The convergence contract needs it, and
the product needs it regardless.

- New migration allocated forward per `docs/DATABASE_STRUCTURE.md`:
  `ple_api.list_course_assignments(p_course_reference_number bigint)` returning assignment reference,
  title, status, and edit number for the session's exact active Instructor membership, with
  `REVOKE ... FROM PUBLIC` and `GRANT EXECUTE ... TO ple_app` matching the neighbouring functions in
  `2026090606_live_demo_assignment_release.sql`.

  **Naming is deliberate.** Its neighbours carry a `live_demo_` prefix from the milestone that
  introduced them, but an Instructor listing her own Assignments is ordinary product capability, not
  demo scaffolding. The new function, store method, and route take product names, and the Live Demo is
  simply their first consumer. Record the naming divergence in `docs/DESIGN_DECISIONS.md` so the next
  author knows the prefix is historical rather than a pattern to copy.
- Store method on `LiveAssignmentStore` plus its Postgres implementation, following
  `crates/learning-data-access/src/postgres/assignment_release.rs` conventions.
- Route `GET /api/course-instances/{course}/assignments` registered beside the existing `post` on that
  path in `crates/server/src/assignment_release.rs:42-45`, gated by the same `instructor()` helper.
- Projection carries no Student identity, response, answer, or grading state.

New `tests/e2e/e2e_live_demo_assignment_list.sh`, shaped like `e2e_live_demo_roster.sh`: the assigned
Instructor sees her released and unreleased Assignments; anonymous, student, and sysadmin sessions all
receive 404 concealment; a foreign Instructor receives 404.

Success: the route returns the Instructor's assignments and conceals from everyone else.
Validation: `bash tests/e2e/e2e_live_demo_assignment_list.sh`.

---

### M4 -- Provisioning executor with a debug harness

Owner: one coder subagent.

New `local_stack_control/live_demo_course_provision.py`: reads observed state through the M2 helpers,
calls `plan_stages`, and applies each planned stage in order. Each stage checks its status code and
raises `ControllerError` naming the stage; expected codes are 201 for creates, 200 for saves and
claims, 201 for release with `revisionNumber: 1`.

Expose it standalone so every later milestone can drive it without a full restart, following the
existing `seed-inventory` subcommand pattern in
`local_stack_control/disposable_stack_command.py:76-77,278`:

```bash
source source_me.sh && python3 -m local_stack_control.disposable_stack_command provision-course \
    --manifest local_stack_state/live_demo_browser/workspace/disposable.manifest
```

Two debug flags, both existing to make partial states reachable without a person:

- `--stop-after <stage>` -- apply stages up to and including one stage, then return. This is the
  synthetic-transition harness M7 uses to manufacture every partial state.
- `--report` -- print the planned stages and exit without writing, so a test can assert convergence
  reached zero planned stages.

**Baseline report.** After provisioning, write a machine-readable JSON report into the disposable
workspace describing the resolved product state and nothing more: `blueprint_reference`,
`course_reference`, `assignment_reference`, each Student's roster id and membership, their Assignment
Attempt if any, their submission count and grading state, and the stages still outstanding. It adds no
interpretation layer on top of those facts.

Every downstream consumer reads that report instead of rediscovering seeded objects by title or by
picking the highest `C-n`: the M7 E2E asserts against it, the M8 browser journey takes its references
from it, and a person debugging a stack reads the demo's exact shape without writing a query. It is
the same artifact the identity resolution above depends on, so it costs nothing extra.

Success: on a stack with no course, one invocation produces the baseline through `release`; a second
invocation reports zero planned stages. M6 extends the same executor with the remaining two stages.
Validation: run the command twice against a started stack; the second `--report` prints no stages.

---

### M5 -- Representative Student activity

Owner: one expert coder subagent. This is the milestone that makes the course look used, and it
completes the executor before any lifecycle wiring claims startup is finished.

Extends the executor with the `attempts` and `work` stages, so Elena's Gradebook shows three
distinguishable Students on first sight.

- Mary and Jack each `GET .../assignments/{A}/access`, require a startable decision, then
  `POST .../start`, which returns the Question Presentation nonce per issued Question.
- Mary answers all four issued Questions. Jack answers two and stops, leaving a real in-progress
  attempt. Avery starts nothing, so `access` reports startable and no attempt exists.
- After each submission, poll the nonce-bound status projection until grading reaches a terminal
  state, so the baseline never leaves half-graded work behind for the first visitor to trip over.

**Responses come from the presented Question, never from the seed source.** The controller reads the
choices the Student's own presentation exposes and picks by a fixed rotation that differs per Student;
it never opens `live_demo_seed_data/*.json` or an Answer Key. Grading is therefore genuine: Mary's
score is a real mixture of correct and incorrect answers produced by the real grader, not a staged
number. Reuse the response encoders that `tests/e2e/e2e_live_demo_native_controls.sh` already
exercises for the eight native PLE formats rather than writing a second encoder.

**Choose the fixed responses once, from evidence.** A pattern chosen blind could land Mary on 0% or
100%, either of which makes a poor Gradebook demonstration. Run the candidate responses against
PNE-0001..0004 on a live stack, read the resulting grading out of the baseline report, and settle on
one deterministic set that produces a mixed result. Then pin it and assert that the real grading
outcome for those fixed responses stays correct -- a stable expectation about known questions, not a
provisioner that keeps searching for a flattering score. If no response set yields a mixed result,
report that rather than inventing a grade.

**The pinned responses live in the declarative baseline**, as `SEEDED_STUDENT_RESPONSES` in
`live_demo_course_seed.py`, expressed in the shape the real Question Presentation and submission
contracts accept. Startup is then deterministic, and a later change to Mary's answers is a reviewable
diff in a declaration rather than a behavioural change buried in executor logic.

Detect for `work` is the per-Student count of terminally graded submissions, so an interruption partway
through Mary's four resumes at the next unanswered Question rather than restarting or double-submitting.

Success: Mary holds four terminally graded submissions with a score between zero and full marks, Jack
holds two with an open attempt, Avery holds no attempt, and Elena's `GET .../gradebook` returns
`gradedStudentWork` distinguishing all three. With this milestone the executor covers every stage, so
`--report` on a provisioned stack plans nothing.
Validation: `bash tests/e2e/e2e_live_demo_gradebook.sh --api` against the provisioned stack, plus
`--report` planning no stages.

---

### M6 -- Lifecycle wiring

Owner: one coder subagent. Runs after M5 so that "startup finished" and "baseline complete" mean the
same thing.

Import the module in `local_stack_control/lifecycle.py`, re-export its entry point beside the existing
`seed_live_demo_baseline` re-exports (lines 111-112), and call it in `start_lifecycle` immediately
after `wait_for_complete_ready` (line 407) and before `open_browser`, guarded by
`live_demo_gateway.is_tls_target`. It must run there and not beside the SQL seed, because the routes it
calls need the application services that come up at line 398. A provisioning failure fails the start,
matching `require_live_demo_session_entry`.

Success: a cold `python3 local_stack.py start --headless` yields the complete baseline -- through
Student work and grading -- with no extra command.
Validation: `python3 local_stack.py stop`, then `start --headless`, then `--report` prints no planned
stages.

---

### M7 -- Convergence and recovery E2E

Owner: one tester subagent.

New `tests/e2e/e2e_live_demo_course_seed.sh`, shaped like `e2e_live_demo_roster.sh` (project labels,
`env.local` port, `podman exec gateway curl`, persona cookies, `psql` `DO $$` evidence blocks), with
modes `--state`, `--converge`, `--recover`:

- `--state`: Elena sees the fixed Course Instance and its released Assignment. Mary, Jack, and Avery
  each hold an active `ple_data.course_membership` bound to a `ple_data.student_record`. Avery's
  `GET .../assignments/{A}/access` reports a startable state with no attempt. A `psql` block proves the released `ple_data.assignment_revision` carries
  exactly the four seeded question ids through `ple_data.assignment_revision_fixed_question` -- closing
  the gap between provisioning input and persisted result. Mary holds four terminally graded
  `ple_private.question_submission` rows, Jack holds two with an open
  `ple_private.assignment_attempt`, and Avery holds an active Student Record with no attempt.
- `--authorization`: the launch-critical boundaries hold against the seeded course under realistic
  state. A Student receives 404 concealment on the roster, the Gradebook, and the Assignment Workspace;
  Morgan receives the same, holding no ambient academic authority over Elena's course; one Student
  cannot read another's work; and anonymous requests are concealed throughout. These are the paths a
  launch decision actually turns on, so they are asserted against the used course rather than only
  against the empty fixtures the existing scripts build.
- `--converge`: run provisioning again; exactly one course carries the fixed title, and `--report`
  plans no stages.
- `--recover`: for each stage, reset the stack, run `--stop-after <stage>`, then run provisioning to
  completion and re-assert `--state`. This proves interrupted provisioning at every boundary recovers,
  using the M4 harness rather than a person pulling a plug.

Anchor the evidence in observable product behaviour. API responses and, in M8, browser behaviour are
the primary proof; the `psql` blocks are for invariants the product interface cannot establish cleanly,
such as exactly which question ids the immutable released revision carries. This stays an
end-to-end behaviour test, not a schema inspection.

Success: all four modes pass on a fresh stack, with `--authorization` a first-class criterion rather
than a supporting check -- realistic FERPA and role boundaries are among the strongest launch-readiness
evidence the plan produces.
Validation: `bash tests/e2e/e2e_live_demo_course_seed.sh`.

---

### M8 -- Headless browser acceptance

Owner: one tester subagent.

New `tests/playwright/e2e_live_demo_course_seed_browser.mjs`, following the existing
`e2e_live_demo_roster_browser.mjs` and using `chooseSeededIdentity` from
`tests/playwright/e2e/real_stack_ui.ts`:

**Elena's populated Instructor view is the primary acceptance criterion.** The demo succeeds when she
opens it cold and immediately sees a believable used course: the roster carrying all three Students,
and the Gradebook distinguishing completed and scored, in progress, and not started. Assert that
first; the Student journeys support it.

- Elena signs in, sees all three on the roster, and sees three distinguishable Gradebook states.
- Mary signs in, lands on her course, and sees her completed Assignment with a score.
- Jack signs in and resumes his in-progress attempt where he left off.
- Avery signs in, lands on the same course, and starts the Assignment from the beginning.

Course and Assignment references come from the M4 baseline report, so the journey never guesses.

Register it in `CURRENT_MILESTONE_JOURNEYS` in `tests/e2e/e2e_live_demo_production_browser.py` so the
serial owner covers it.

Success: the journey passes headless, with no attended step.
Validation: `./devel/run_playwright_tests.sh --build`.

---

### M9 -- Regression sweep and documentation

Owner: one maintainer subagent.

Regression risk to verify rather than assume: several existing e2e scripts identify their fixture by
picking the highest `C-n` from `GET /api/course-instances`. The seeded course is created at stack start
and therefore holds the lowest reference, so those scripts still select their own newer course. Confirm
by running them; if any script instead selects the seeded course, give that script a stable fixture
identity as part of this milestone rather than leaving the ordering dependency in place.

```bash
source source_me.sh && pytest tests/
bash tests/e2e/e2e_live_demo_seeded_baseline.sh --replay
bash tests/e2e/e2e_live_demo_roster.sh --import
bash tests/e2e/e2e_live_demo_assignment_release.sh --service
bash tests/e2e/e2e_live_demo_assignment_attempt.sh --start
bash tests/e2e/e2e_live_demo_gradebook.sh --api
```

`seeded_baseline --replay` must still report the inventory receipt `5|4|4|4|4|1|1|1|1`, proving the SQL
seed transaction gained no rows.

Documentation:

- `docs/DESIGN_DECISIONS.md` -- three entries, each with `Decision`, `Why`, `Consequence`, `Owner`:
  1. **The Live Demo is an acceptance environment for the real PLE.** It establishes representative
     real product state through normal product contracts, then uses real user workflows to prove
     launch readiness. Future domains follow that rule where it applies. State it as a product
     principle, not as a demo framework future work is expected to build on -- the Live Demo is not a
     product of its own.
  2. **How seeded Student Accounts reach Elena's course.** The real path is roster-driven: an
     Instructor's roster import resolves an institutional email to an existing global Student Account
     or creates one when none exists, then the claim creates the course-scoped Student Record and
     enrollment. The demo exercises exactly that path. The disposable-demo exception is narrower and
     worth naming precisely: the SQL seed precreates the three fictional Accounts so visible-role
     sign-in is deterministic.
  3. **Product naming for the Instructor Assignment list**, per M3.
- `docs/LOCAL_STACK_OPERATIONS.md` lines 142-146 -- replace the "seeds no Course Instance, membership,
  roster, or Student Record" statement with the two layers, the convergence contract, manifest-first
  identity, the baseline report and where to read it, and the `provision-course` command with its two
  debug flags.
- `docs/LIVE_DEMO_SPEC.md` -- already authoritative for the baseline after M1; verify it matches what
  shipped, add the baseline-report location, and state the primary success criterion above.
- `docs/COOKBOOK.md` -- the **attended email invitation workflow**, explicitly optional and outside the
  automated path: Elena adds a fourth fictional Student to the already-active course using an address
  the operator controls, `launchers/send_invitations.py` delivers the real invitation, the recipient
  claims it, and the new Student appears on Elena's roster. Email and signup are themselves
  launch-readiness questions, so this stays a real workflow run against the used course -- and it is
  why the seeded personas keep non-routable `.invalid` addresses, so routine demo startup can never
  send mail to anyone.
- `docs/API_CONTRACTS.md` -- the M3 Instructor Assignment list route.
- `docs/DATABASE_STRUCTURE.md` -- the M3 migration in the ownership map.
- `docs/CHANGELOG.md` -- entries under `### Additions and New Features` and
  `### Behavior or Interface Changes`.

Success: the full sweep passes and no doc still claims the baseline seeds no course data.
Validation: the command block above, plus `pytest tests/test_markdown_links.py`.

## Dependencies

```text
            M1 ---+
M0 -------> M2 ---+--> M4 --> M5 --> M6 --> M7 --> M8 --> M9
            M3 ---+
```

M0 comes first because it changes the seeded emails and display names everything else reads. M1, M2,
and M3 are then independent and dispatch in parallel; M3 is the long pole (migration, store, route,
e2e), so start it first among them. M4 builds the executor through `release`, M5 completes it with
`attempts` and `work`, and only then does M6 wire it into startup -- so "startup finished" and
"baseline complete" never disagree.
