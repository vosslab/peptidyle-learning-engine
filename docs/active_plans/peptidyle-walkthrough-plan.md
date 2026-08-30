# Plan: Instructor-to-student assignment walkthrough

> **Historical/accepted plan notice (2026-08-22).** This is an accepted historical plan. Its
> objectives and dated evidence below remain a record of the corrected walkthrough scope; they are
> not the current browser runbook or current-package handoff. Current browser evidence is owned by
> [real_stack_browser_suite_plan.md](active/real_stack_browser_suite_plan.md), and
> [implementation_status.md](implementation_status.md) records the sole current package.

## Context

The repository owner has corrected the walkthrough charter. Its purpose is to
demonstrate the ordinary teaching loop, not production identity onboarding:
an instructor creates a course, adds an active student to its roster, builds an
assignment from the published problem corpus, and then observes the student's
scored and repeated work. The student must take the assignment entirely through
the visible keyboard platform path.

The earlier walkthrough implementation proved a valuable but narrower slice.
Manager and independent retained-volume runs passed learner J1 through J5 and
cross-actor J8 against API-arranged assignments in a launcher-seeded course.
That evidence remains valid, but it does not prove the newly binding instructor
setup path. The earlier plan also made canonical email onboarding a walkthrough
milestone. That was a scope error: canonical email identity remains a separate
release package and is not a prerequisite, journey, report row, or blocker for
this walkthrough.

This corrected plan is a clean continuation from the accepted learner evidence.
It adds the missing instructor product surfaces and replaces API course,
membership, and assignment setup with visible browser actions. Supported API
publication may still arrange the small answer-private problem corpus because
problem authoring itself is not part of this teaching-loop acceptance.

This is the priority Fall 2026 pilot-readiness path, with an owner target of
approximately August 25, 2026. Unrelated release breadth must not delay it.

> **Accepted on 2026-08-12.** The M10/M11 executions remain historical evidence for the narrower
> prior walkthrough baseline. Current acceptance comes from the rebuilt clean-stack no-email
> teaching loop, visible four-ID J13 setup, timed keyboard take/repeat/gradebook story, separate
> all-eight Chapter 1 sweep, refreshed eleven-image screenshot set, full disposable PostgreSQL
> baseline, and independent review. Full HOTSPOT author-to-learner lifecycle acceptance remains
> outside this walkthrough in WP-RC5.

## Objectives

- Let the local instructor visibly create a new course through the application.
- Let that instructor visibly add the configured local student as an active
  roster member without any email address, mailbox, challenge, invitation link,
  SMTP provider, or canonical account.
- Let the instructor visibly create an assignment, select an immutable problem
  from the published corpus, configure continued-practice policies, and confirm
  the assignment in the new course.
- Let the local student take, submit, complete, and repeat that exact assignment
  through Tab, Shift+Tab, Space, and native Enter controls.
- Prove scoring as a visible product outcome by observing the exact assignment
  and learner row in the instructor gradebook, including its percentage and
  completed-run count, without calculating or reading grading internals.
- Produce a deterministic, redacted, retained-stack report that distinguishes
  corpus arrangement from every visibly walked action.
- Deliver the complete local teaching loop before the Fall pilot without
  waiting for production email-account work.

## Design philosophy

Follow **fix the design, not the symptom** and **make invalid states hard to
represent**. If the instructor cannot create the course, activate the local
student, or construct the assignment through visible controls, the product path
is incomplete; the simulator must not replace it with an API call and call the
result coverage. The one deliberate local-development seam is an explicit
two-actor testing capability, not a weakened production identity path.

- Evidence strategy for uncertain methods: implement the narrowest visible
  product capability, exercise it first with focused behavior tests, then run
  the fixed retained-stack walkthrough twice before promoting the report
  baseline.

## Scope

- Preserve the accepted real-stack runner, Python orchestration, IPv4 gateway,
  redacted report, no-volume cleanup, and student keyboard helpers.
- Publish the minimal retry-capable native problem through the existing
  supported authoring API as an explicitly named arrangement.
- Add an instructor-visible course-creation surface backed by the existing
  authorized course-creation route.
- Add a local-only visible roster adapter that resolves a configured learner
  alias and invokes the sole canonical `UpsertCourseMember` operation. Local
  identity configuration is composition metadata only; production composition
  exposes neither this adapter nor its control.
- Add an instructor-visible new-assignment route that searches the published
  problem corpus, selects immutable problem versions, configures policies, and
  creates the assignment.
- Make the displayed `AAA-BBBB` Question ID operational: the
  instructor copies and pastes the four exact Genetics Chapter 1 references
  into the visible add-by-ID control, then observes the four selected immutable
  versions before creating the assignment. UUIDs are not part of this human
  workflow.
- Walk the corrected core sequence J11, J12, J13, J1, J2, J3, J4, J5, and J8.
- Strengthen J5/J8 so the exact learner and assignment visibly show a scored
  result and the expected repeated-run count.
- Replace the old static baseline and documentation with the corrected charter
  only after the full retained-stack sequence passes independent review.

## Non-goals

- Add or test email authentication, email delivery, SMTP, mailbox access,
  one-time links, invitation delivery, passkeys, or canonical account creation.
- Give an agent an email account or treat missing email infrastructure as a
  walkthrough blocker.
- Claim the local roster capability as production enrollment, canonical
  onboarding, or invitation acceptance.
- Walk visual problem authoring or require the instructor to create the corpus
  problem; corpus publication remains a private supported-API arrangement.
- Require all eight response families, multiple independent learners, exam
  policy contrast, or unrelated release gates for this walkthrough to pass.
- Inspect SQL, private answer material, grading code, internal score records,
  cookies, browser storage, or direct application APIs from journey code.
- Put credentials, answer material, response values, traces, or child output in the final report.
  Retain the approved visible-stage screenshots as required public evidence, using only the
  unmistakably fake local identities named in `docs/HUMAN_GUIDANCE.md`.
- Reset retained volumes or delete prior course data to make selectors easy.

## Current state summary

The accepted runner launches the real local stack, validates live inputs,
builds automatically unless explicitly overridden, and runs fixed Playwright
children. Its report directory and file are mode 0700 and 0600, and runner-owned
cleanup removes containers without removing retained volumes.

The walkthrough's documentation gate also retains eleven approved instructor and student stage
PNGs: course, roster, catalog selection, assignment policies, post-create confirmation, assignment
list, assignment overview, timed run 1, scored completion with the retake action, the resulting run
2, and the two-run gradebook. Fake-user screenshots are required evidence and must not be omitted
under a student-privacy claim; credential, answer-key, trace, and raw child-output prohibitions
remain security boundaries.

The launcher already creates separate local instructor and student credentials.
Existing J1/J2/J3/J4 use the student through visible local sign-in and keyboard
controls; J5 uses a separate instructor context. The retained-volume run proves
retry, leave/return, fresh practice, gradebook navigation, and cross-actor
visibility against an API-arranged Mastery assignment.

The corrected charter is accepted. Its three formerly blocking product gaps are closed:

- `POST /api/courses` exists and authorizes course creation, but the browser
  client and course list have no create-course form.
- The roster page can create a pending email invitation, but a local session
  cannot turn that invitation into an active member. A new course therefore has
  no email-free visible path for adding the configured local student.
- The assignment API and editor repository can create an assignment and select
  published catalog items, but there is no visible new-assignment entry route.

The gradebook already renders exact assignment rows with Best, Latest,
Completed, and opt-in run history. Existing J5 intentionally stops before
asserting those scoring cells; the corrected charter requires that visible
evidence.

The old baseline that lists J9/J10 email onboarding, all-family, and
multi-learner blockers is historical evidence for the superseded charter. It is
not the acceptance baseline for this plan.

## User-facing contract

The fixed walkthrough tells one coherent story:

1. The local instructor signs in through the visible local form.
2. The instructor creates a uniquely named course and opens it.
3. The instructor opens Students and adds the configured local learner as an
   active member. The application visibly confirms the active roster row.
4. The instructor creates a Mastery assignment, searches the published problem
   corpus for the four Genetics Chapter 1 questions, copies their displayed
   `AAA-BBBB` Question IDs, pastes them into the visible add-by-ID control, observes
   all four selected immutable versions, selects continued-practice policies,
   and confirms the assignment in the course.
5. The local student signs in through a separate browser context, opens that
   exact course and assignment, responds, submits, sees feedback, corrects an
   intentional first error, and completes the run by keyboard.
6. The student activates Start another practice run, receives a fresh attempt
   with cleared response controls, and completes a second run by keyboard.
7. The instructor returns to the exact assignment's gradebook row and visibly
   observes Best `100%`, Latest `100%`, Completed `2`, and two completed history
   entries for the local learner.

Every student action uses visible platform controls under
`docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md`. Browser code may observe displayed
outcomes but never derive the correct response or compute the score.

## Architecture boundaries and ownership

The published problem is arrangement data. Course creation, roster activation,
assignment creation, student work, repeat practice, scoring display, and
gradebook inspection are journey evidence and must occur through the browser.

The local roster capability belongs to local-development composition. It must
accept only a server-configured learner alias, derive account and Student identity
from the local identity provider, authorize the instructor against the target
course, and atomically create the active membership, roster projection, and any
required assignment enrollments. Production composition must not mount the
route or advertise the control. The browser never submits a credential, email,
arbitrary user ID, installation-wide scope value, or account record to this capability.

Public course and assignment IDs created by the instructor browser may cross
fixed child boundaries through one runner-generated schema-versioned private
input file passed by explicit argument. The Python runner validates that state,
removes inherited walkthrough `PLE_*` overrides for owned children, and
supplies only the required public IDs to each later fixed stage. Credentials
remain path-only private inputs and never enter the handoff or report.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / workstream | Component                                      | Review boundary                         |
| ---------------------- | ---------------------------------------------- | --------------------------------------- |
| M8 / WS-CHARTER        | Human guidance, active plan, status            | Owner intent and scope separation       |
| M9 / WS-COURSE         | Course API client and course list UI           | Authorized visible course creation      |
| M9 / WS-ROSTER         | Local identity composition, Store, roster UI   | Local-only active membership            |
| M9 / WS-ASSIGN         | Assignment create route and editor             | Corpus-backed assignment construction   |
| M10 / WS-WALK          | Playwright setup, learner, gradebook journeys  | Visible action, score, repeat, keyboard |
| M10-M11 / WS-EVID      | Python runner, private state, report, baseline | Redaction and exact outcome schema      |
| M11 / WS-DOCS          | Operator docs, status, changelog               | Honest close-out without email gates    |

## Milestone plan

| M   | Title                  | Summary                                                        | Goal                                     |
| --- | ---------------------- | -------------------------------------------------------------- | ---------------------------------------- |
| M8  | Correct the charter    | Preserve owner intent and supersede the email-gated baseline   | One binding definition of success        |
| M9  | Build instructor setup | Add visible course, local roster, and assignment construction  | Instructor can prepare real student work |
| M10 | Walk the teaching loop | Run instructor setup, student keyboard work, repeat, and score | One complete retained-stack story passes |
| M11 | Close evidence         | Promote report schema, baseline, docs, and independent review  | Durable truthful acceptance              |

### Milestone: M8 correct the charter

- Depends on: none; this is the corrected source of truth.
- Deliverables: durable human guidance, this active plan, supersession notes,
  and corrected implementation status.
- Workstreams: WS-CHARTER.
- Entry criteria: owner intent is recorded verbatim enough to preserve meaning.
- Exit criteria: no active walkthrough document names email, canonical
  onboarding, J9/J10, all-family, or multi-learner work as a completion gate.
- Parallel-plan ready: no; one documentation owner must reconcile terminology.

### Milestone: M9 build instructor setup

- Depends on: M8, because the browser-versus-arrangement boundary is binding.
- Deliverables: visible course creation, local active-roster addition, visible
  corpus-backed assignment creation, focused browser tests, and Store/backend
  conformance for canonical roster upsert.
- Workstreams: WS-COURSE, WS-ROSTER, and WS-ASSIGN.
- Entry criteria: the M8 exit criterion holds.
- Exit criteria:
  - A local instructor can create a course and see its course shell.
  - The instructor can add the configured local student and see an active row.
  - The instructor can create the Mastery assignment from an immutable corpus
    version and see it in the course.
  - Production composition exposes neither the local roster adapter route nor
    control.
- Parallel-plan ready: yes - max parallel doers: three. The three product
  owners share only generated/API contracts and merge after those contracts are
  agreed.

### Milestone: M10 walk the teaching loop

- Depends on: M9, because the walkthrough may not arrange missing instructor
  actions through APIs.
- Deliverables: fixed instructor-setup child, protected public-ID handoff,
  refocused learner take/retry/repeat journey, visible scoring assertions, and
  report schema version 2.
- Workstreams: WS-WALK and WS-EVID.
- Entry criteria: all M9 focused product gates and independent reviews pass.
- Exit criteria:
  - One exact retained-stack `--build` run passes J11, J12, J13, J1, J2, J3,
    J4, J5, and J8 in order with empty diagnostics.
  - The student completes two runs by keyboard and the exact gradebook row
    shows Best and Latest `100%`, Completed `2`, and two completed history rows.
  - Only corpus publication remains an API arrangement.
- **Historical evidence, superseded for final walkthrough acceptance:** the
  retained-stack seed-42 `--build` run completed J11, J12, J13, J1, J2, J3,
  J4, J5, and J8 in the required order. The student completed two runs through
  the keyboard platform path and J5 visibly confirmed Best `100%`, Latest
  `100%`, Completed `2`, and two history rows. It did not yet prove the
  strengthened four-reference copy/paste construction contract; WP-HG1 owns
  the required rebuilt run and independent acceptance. This is a narrower
  baseline, not a current acceptance claim.
- Parallel-plan ready: no; setup produces IDs consumed by serial learner and
  instructor children.

### Milestone: M11 close evidence

- Depends on: M10, because static evidence cannot anticipate live results.
- Deliverables: duplicate-safe static baseline, operator documentation,
  changelog entry, final retained-stack replay, and independent HCI/security
  review.
- Workstreams: WS-EVID and WS-DOCS.
- Entry criteria: M10 has one manager PASS.
- Exit criteria:
  - A second same-seed retained-stack `--build` run independently passes with
    newly created course and assignment IDs.
  - Both reports are canonical, redacted, mode 0700/0600, and contain no email
    or identity-onboarding rows.
  - Runner-owned no-volume cleanup leaves no containers or private state.
  - The final checklist and implementation status describe the walkthrough as
    accepted independently of production email work.
- **Historical narrower-baseline evidence (2026-08-11):** an independent second seed-42 `--build` replay
  created a fresh instructor course and assignment and passed the same nine
  rows. Both reports were canonical redacted schema-v2 output in mode-0700
  directories with mode-0600 files; cleanup left no containers or private
  walkthrough state. Independent HCI, security, report-security, and final
  walkthrough reviews accepted their then-scoped boundaries; see the retained-live final
  review recorded under `docs/active_plans/audits/`. That evidence does not close the
  reopened WP-HG1 live acceptance checklist.
- Parallel-plan ready: yes - max parallel doers: two. Evidence review and docs
  may proceed after the manager report stabilizes.

## Workstream breakdown

### Workstream: WS-CHARTER owner intent

- Goal: keep the corrected teaching-loop goal authoritative.
- Owner: documentation architect.
- Work packages: WP-C0.
- Needs: `docs/HUMAN_GUIDANCE.md` and current status evidence.
- Provides: scope, non-goals, and journey vocabulary.
- Review boundary, when modifying the repository: documentation only.

### Workstream: WS-COURSE visible course creation

- Goal: let authorized instructors create a course through the browser.
- Owner: SolidJS and TypeScript engineer.
- Work packages: WP-I1.
- Needs: existing `POST /api/courses` contract.
- Provides: a new course ID and visible course shell.
- Review boundary, when modifying the repository: browser client and course UI;
  no authorization weakening.

### Workstream: WS-ROSTER canonical local active membership

- Goal: add the configured local student without email.
- Owner: Rust, PostgreSQL, and local-composition engineer.
- Work packages: WP-I2.
- Needs: local identity composition metadata, course manager authorization, and
  canonical roster/enrollment invariants.
- Provides: active roster membership and assignment reconciliation.
- Review boundary, when modifying the repository: exact local adapter
  composition only; production route/control absence is mandatory. The adapter
  resolves an alias, then calls the canonical upsert rather than owning a
  roster source, provenance, Store command, or migration.

### Workstream: WS-ASSIGN corpus-backed assignment construction

- Goal: create an assignment from immutable published problem versions through
  the instructor UI.
- Owner: SolidJS and TypeScript engineer.
- Work packages: WP-I3.
- Needs: course ID and arranged public problem reference.
- Provides: visible Mastery assignment and public assignment ID.
- Review boundary, when modifying the repository: create mode and catalog
  selection; grading remains server-owned.

### Workstream: WS-WALK real browser journeys

- Goal: prove the complete instructor-to-student story.
- Owner: Playwright and HCI engineer.
- Work packages: WP-I4, WP-S1, and WP-S2.
- Needs: M9 product surfaces and existing keyboard helpers.
- Provides: ordered journey fragments and visible outcome evidence.
- Review boundary, when modifying the repository: no pointer, direct-route,
  API, storage, answer, or score-calculation shortcuts.

### Workstream: WS-EVID protected orchestration

- Goal: safely transfer public IDs and render the corrected report.
- Owner: Python and TypeScript test engineer.
- Work packages: WP-E1.
- Needs: fixed journey fragment contracts.
- Provides: schema-v2 report and duplicate-safe baseline.
- Review boundary, when modifying the repository: descriptor-safe private state,
  exact child list, and no child output.

### Workstream: WS-DOCS operator close-out

- Goal: make the command and evidence limits easy to understand.
- Owner: documentation engineer.
- Work packages: WP-D1.
- Needs: accepted report and independent review.
- Provides: status, usage, E2E, troubleshooting, and changelog truth.
- Review boundary, when modifying the repository: documentation only.

## Work packages

### Work package: WP-C0 reconcile the corrected charter

- Owner: documentation architect.
- Touch points: `docs/HUMAN_GUIDANCE.md`, this plan, implementation status,
  release-plan cross-reference, historical baseline/workstream/audit addenda.
- Depends on: none.
- Acceptance criteria:
  - The core goal is instructor course/roster/assignment setup followed by
    student keyboard take/score/repeat and instructor gradebook confirmation.
  - Email and canonical onboarding are explicitly outside the walkthrough.
  - Historical accepted learner evidence is preserved without claiming the
    corrected charter already passes.
- Evidence or review, when useful: Markdown, ASCII, links, line limits, and an
  independent plan review against owner guidance.
- Obvious follow-ons: dispatch WP-I1 through WP-I3.

### Work package: WP-I1 add visible course creation

- Owner: SolidJS and TypeScript engineer.
- Touch points: course request/decoder client, `course_list_page.tsx`, focused
  component and Playwright tests.
- Depends on: WP-C0.
- Acceptance criteria:
  - Only instructor or administrator sessions see the create-course form.
  - A labelled title input and Create course button submit through the public
    client, preserve input on recoverable failure, announce progress/error, and
    focus the newly created course link on success.
  - Student sessions cannot render or call course creation.
  - The created course opens through its visible link.
- Evidence or review, when useful: strict decoder tests, production-component
  keyboard tests, server authorization regression, and independent HCI review.
- Obvious follow-ons: WP-I2 and WP-I4.

### Work package: WP-I2 add canonical local roster activation

- Owner: Rust, PostgreSQL, and local-composition engineer.
- Touch points: local identity composition, local-only visible adapter route,
  the canonical `UpsertCourseMember` Store capability and its Memory/PostgreSQL
  owners, roster browser client/page, generated contracts, and focused
  route/conformance/Playwright tests. No local-roster migration exists in the
  current pre-production schema.
- Depends on: WP-C0.
- Acceptance criteria:
  - The adapter route is mounted only by exact local-development authentication
    composition and is absent from production composition.
  - Every local identity record has one exact, bounded, unique ASCII alias. The
    request accepts only a configured learner alias; account, Student, display name,
    and roles come from the server's local identity provider.
  - The authenticated course manager can activate that learner through the
    canonical upsert, atomically creating or reviving the student member,
    active roster row, and enrollment for existing assignments.
  - Repeating the action is idempotent; foreign-course, nonmanager, arbitrary
    user, instructor alias, unknown alias, and production requests fail closed.
  - The visible roster control uses no email field and confirms the active local
    learner row by keyboard.
  - `learner_alias` remains local identity composition metadata only. The
    canonical roster record has no local source or provenance field, and the
    adapter creates no invitation, email identity, or account-acceptance event.
  - Memory and PostgreSQL implement the same canonical `UpsertCourseMember`
    transaction. The fresh pre-production schema owns that model directly;
    there is no `2026080913_local_development_roster.sql` migration, duplicate
    Store path, or retained compatibility shape.
- Evidence or review, when useful: Memory/PostgreSQL conformance, forced-RLS
  live route gate, production-router absence test, and independent security
  review.
- Obvious follow-ons: WP-I4.

### Work package: WP-I3 add visible assignment creation from the corpus

- Owner: SolidJS and TypeScript engineer.
- Touch points: route contract, course management navigation, assignment editor
  create mode, catalog repository, focused component and Playwright tests.
- Depends on: WP-C0; may proceed in parallel with WP-I1 and WP-I2.
- Acceptance criteria:
  - The course exposes a visible New assignment action only to managers.
  - Create mode accepts a title, searches the public catalog, selects exact
    immutable problem/version tuples, configures Mastery continued-practice
    policy, and creates through the existing strict public client.
  - Validation, transport, permission, and stale-state failures preserve the
    instructor's work and provide labelled recovery.
  - Success visibly confirms the new assignment and navigates through a real
    course/assignment link.
  - No private problem source or answer-bearing field enters browser state.
- Evidence or review, when useful: create/edit behavior tests, strict payload
  inspection, keyboard production-component test, and independent HCI/security
  review.
- Obvious follow-ons: WP-I4.

### Work package: WP-I4 walk instructor setup and hand off public IDs

- Owner: Playwright and HCI engineer.
- Touch points: fixed instructor-setup spec, protected journey state, runner
  stage list, live configuration.
- Depends on: WP-I1, WP-I2, and WP-I3.
- Acceptance criteria:
  - J11 creates and opens a unique course through visible controls.
  - J12 adds and observes the active local student through Students.
  - J13 searches the published catalog through visible controls, copies and
    reuses the Genetics Chapter 1 assignment or pastes its four displayed `AAA-BBBB` Question IDs into the
    add-by-ID control, observes exactly four selected immutable versions, then
    creates the corpus-backed Mastery assignment and observes its course
    card/link.
  - J13 validates malformed, unavailable, unauthorized, and duplicate
    add-by-ID recovery in focused product tests; those cases preserve the
    pasted value and existing draft. The real walkthrough proves the successful
    clipboard/paste path, not every error state.
  - The spec appends only exact public course, assignment, and problem IDs to
    protected private state after all visible assertions pass.
  - The runner validates the fragment and supplies IDs only to fixed later
    children; stdout/stderr and credentials remain discarded.
- Evidence or review, when useful: hostile state/symlink/subprocess tests and a
  real-stack instructor-only run.
- Obvious follow-ons: WP-S1 and WP-S2.

### Work package: WP-I5 connect visible whole-run timing

- Owner: Rust, PostgreSQL, TypeScript/Solid, and HCI engineers.
- Touch points: assignment editor request/response contract, assignment editor Store capabilities,
  existing timing resolver, generated browser contracts/defaults, editor model/page, and focused
  Rust/TypeScript/Playwright tests.
- Depends on: WP-I3 and the then-existing `AssignmentTimingPolicy`; this is historical dependency
  evidence only, because that legacy model and API are removed. Its Rust/store/editor boundary precedes
  the Solid form and the live J13/J1 sequence. Current policy authority is accepted WP-INST-S3's
  current resolved verdict after S5 entitlement; sealed receipts are historical attempt evidence.
- Acceptance criteria:
  - At acceptance, the course-owned whole-run timing field was editor-only
    `assignmentTiming.timeLimitSeconds: positive u32 | null`; it is not a `RunPolicies` field or a
    published-question setting.
    WP-INST-T1 supersedes that transport with the single revisioned teaching-settings aggregate; no
    current browser, Store, or route accepts `assignmentTiming`.
  - New mastery assignments start from the Rust-generated `900`-second default. The editor visibly
    presents 15 minutes with accessible Timed/Untimed controls and saves `null` only for an explicit
    Untimed choice.
  - Editor create, update, and GET compose definition and timing atomically under one shared
    revision. A stale save has no partial definition, item, or timing result; question versions and
    immutable question-level timing remain unchanged.
  - A timed student run displays the server-backed countdown. A fresh practice run resolves a fresh
    server deadline. Only a saved null value displays `Untimed`.
  - Invalid timing input and request/revision failure preserve the visible radio state and literal
    minutes input with labelled recovery.
- Permanent evidence: Store/route/decoder/component behavior tests plus keyboard-focused Playwright
  coverage for default, conversion, toggle, validation, recovery, and timed/untimed display. These
  do not retain timing snapshots or implementation-string checks.
- One-time acceptance evidence: a real Podman PostgreSQL plus `webwork-pg-renderer` J13/J1--J4 run
  creates the Genetics assignment through assignment reuse or copied `AAA-BBBB` Question IDs, visibly keeps the 15-minute
  default, and captures the server-backed countdown, retry, and fresh timed run for guide review.
- Obvious follow-ons: WP-S1, WP-S2, and WP-E1.

### Work package: WP-S1 prove student keyboard take and repeat

- Owner: Playwright and HCI engineer.
- Touch points: refocused J1/J2/J3/J4 specs and shared keyboard helpers.
- Depends on: WP-I4 and WP-I5.
- Acceptance criteria:
  - The separate local student context opens the exact newly created course and
    assignment through visible links.
  - The first run includes a visible incorrect response, feedback, retry,
    correct response, and completion without reading feedback body or grading
    data.
  - Start another practice run creates a fresh run with cleared response
    controls; the student completes the second run.
  - Every student action uses Tab, Shift+Tab, Space, or native Enter, with
    focus assertions and no pointer, direct focus, route, API, cookie, storage,
    history, Arrow, digit, or Escape shortcut.
- Evidence or review, when useful: focused native-control fixtures, source
  scanner, exact retained-stack J1-J4 run, and independent HCI review.
- Obvious follow-ons: WP-S2.

### Work package: WP-S2 prove visible scoring and instructor outcome

- Owner: Playwright and HCI engineer.
- Touch points: J5 gradebook spec, J8 cross-actor fragment, score-cell helpers,
  report milestone vocabulary.
- Depends on: WP-S1.
- Acceptance criteria:
  - The instructor opens the exact course and assignment row through visible
    controls after the student's second completion.
  - The row visibly shows Best `100%`, Latest `100%`, and Completed `2` for the
    configured local learner.
  - Run history visibly contains two completed entries.
  - J8 binds the learner completion and instructor row to the same public course
    and assignment IDs without reporting the score, learner identity, or run
    details.
- Evidence or review, when useful: focused gradebook component tests, retained
  live J5/J8 evidence, and independent privacy/HCI review.
- Obvious follow-ons: WP-E1.

### Work package: WP-E1 publish the corrected report and baseline

- Owner: Python and TypeScript test engineer.
- Touch points: runner, private-state parser, report renderer, report tests,
  and the one-time `walked_journey_baseline.json` evidence record.
- Depends on: WP-I4, WP-S1, and WP-S2.
- Acceptance criteria:
  - Schema version 2 requires ordered PASS rows J11, J12, J13, J1, J2, J3, J4,
    J5, and J8, with closed visible milestone vocabularies.
  - Arrangement labels contain only local identity availability and corpus
    publication; course, roster, assignment, score, and repeat are journeys.
  - J6/J7, J9/J10, all-family, multi-learner, email, mailbox, and release-gate
    rows are absent from this charter's report and baseline.
  - Every child failure reports only its fixed stage and bounded diagnostic;
    cleanup and 0700/0600 report guarantees remain unchanged.
  - The maintained live-report parser rejects duplicate members and false PASS
    changes; no permanent test freezes the dated baseline's exact rows or keys.
- Evidence or review, when useful: focused Python/Node hostile report-contract
  tests, two live reports, and independent report-security review.
- Obvious follow-ons: WP-D1.

### Work package: WP-D1 close operator documentation

- Owner: documentation engineer.
- Touch points: `docs/E2E_TESTS.md`, `docs/USAGE.md`, troubleshooting,
  implementation status, release-plan cross-reference, workstreams, audits,
  and `docs/CHANGELOG.md`.
- Depends on: WP-E1.
- Acceptance criteria:
  - Documentation describes the exact instructor-to-student command and visible
    evidence without calling email a prerequisite or blocker.
  - Production email/account work remains accurately documented under WP-RC8
    only.
  - Historical reports and audits receive supersession addenda rather than
    rewritten execution history.
- Evidence or review, when useful: docs gates and final independent review.
- Obvious follow-ons: none; this closes the corrected walkthrough.

### Work package: WP-E2 package the accepted runner

- Owner: Python test engineer.
- Status: ACCEPTED. The canonical command passed the retained-stack no-email
  walkthrough after the package move; independent architecture, security,
  retained-live, and closeout reviews found no P1/P2 issue.
- Touch points: the new `tests/walkthrough/` owner and `walklib/` package,
  compatibility E2E entry points, runner tests, harness discovery, and
  architecture documentation.
- Depends on: accepted M10/M11 evidence and WP-D1.
- Acceptance criteria:
  - The new canonical shell command and report behavior retain the accepted
    contract; historical E2E paths remain compatibility launchers.
  - The Python entry point is a thin executable facade over importable modules.
  - Configuration, typed process values, strict arrangement/report contracts,
    and runner orchestration have focused owners under one package.
  - Playwright journeys remain under `tests/playwright/` and the anti-shortcut
    scanner covers the complete runner package recursively.
  - The walkthrough does not import another E2E runner or its test support.
- Evidence or review, when useful: focused runner/cleanup/security suites,
  source-line and import-policy gates, unchanged CLI help, and independent
  architecture review.
- Obvious follow-ons: split private-state or report capabilities only when
  their ownership changes or the runner composition module grows materially.

## Acceptance criteria and gates

- Charter gate: durable human guidance, this plan, status, and baseline use the
  same instructor-to-student goal and email exclusion.
- Product gate: course creation, local active-roster addition, and assignment
  creation each pass focused authorization, keyboard, recovery, and independent
  review before live integration.
- Human-reference gate: browser and server accept the same bounded exact
  `AAA-BBBB` Question ID domain; no UUID is an instructor entry value or test-only DOM
  extraction path. The successful J13 path uses copied visible references and
  selects exactly the requested four immutable versions. Batch recovery leaves
  the assignment draft unchanged until all references resolve.
- Composition gate: production router tests prove the local roster route and UI
  capability are absent; local requests cannot select arbitrary identities.
- Arrangement gate: only problem-corpus publication may be arranged by API.
  Any API-created course, membership, roster row, enrollment, or assignment
  invalidates the walkthrough PASS.
- Student keyboard gate: all student actions use the platform path and visible
  focus. Widget shortcuts remain supplementary only.
- Timing gate: the instructor visibly retains the new-mastery 15-minute default while creating the
  assignment; the student's timed run uses a server-backed countdown, and a fresh run has a fresh
  deadline. `Untimed` is visible only for an explicit null course-owned setting.
- Scoring gate: the exact instructor gradebook row visibly shows the expected
  score and two completions after two browser-completed runs; no internal score
  endpoint or calculation is used.
- Repeat gate: the second run begins from the visible fresh-practice action,
  presents cleared response controls, and completes independently.
- Integration gate: two retained-stack same-seed `--build` runs create new
  course/assignment IDs and pass the exact fixed journey order.
- Report gate: canonical ASCII schema-v2 output is redacted, mode 0700/0600,
  and contains no email/onboarding or sensitive fields.
- Independent review gate: separate Rust/PostgreSQL security, TypeScript/Solid
  HCI, report-security, and final walkthrough reviewers accept their boundaries.

## Test and verification strategy

| Tier                   | Evidence                                                       | Failure semantics     |
| ---------------------- | -------------------------------------------------------------- | --------------------- |
| Rust unit/route        | Local composition absence, alias validation, authorization     | Blocks WP-I2          |
| Store conformance      | Atomic local member/roster/enrollment on Memory/PostgreSQL     | Blocks WP-I2          |
| Node/TypeScript        | Strict client decoders, forms, state, report schema            | Blocks owning package |
| Focused Playwright     | Production course, roster, ID recovery, keyboard, gradebook UI | Blocks owning package |
| Static harness scanner | No pointer, shortcut, direct API/storage, answer, hidden PASS  | Blocks WS-WALK        |
| Real-stack runner      | Fixed instructor setup plus student take/score/repeat          | Blocks M10/M11        |
| Independent review     | Security, HCI, privacy, report, and final charter audit        | Blocks acceptance     |

The parser/domain/RLS, mock/live error parity, editor recovery, focused
copy/paste and timing Playwright coverage, assignment timing Store/route
conformance, and explicit child-input parsing/environment isolation are
permanent behavior tests. A rebuilt retained-stack run with real clipboard
permission, redacted report, explicit child handoff, server-backed timed run,
and refreshed public screenshots is one-time acceptance evidence; it is not
replaced by fixture-count or exact-source-string tests. The isolated
PostgreSQL/MinIO two-chapter eight-question publication sweep is a separate
release-content oracle and does not substitute for instructor construction.

Repository Python commands use `source source_me.sh && python ...`. Browser
launches use the existing fixed shell front door. PostgreSQL evidence uses a
disposable or launcher-owned test database without resetting the owner's
retained walkthrough volumes.

## Migration and compatibility policy

The local roster adapter is a current local-composition capability, not a
second roster model. It resolves `learner_alias` from the configured local
identity directory and invokes the sole canonical `UpsertCourseMember`
transaction. The roster row has no local source/provenance, and the current
fresh schema has no `2026080913_local_development_roster.sql` migration or
compatibility reader. Production composition omits the adapter and its control;
its invitation, passwordless, course, assignment, and gradebook contracts stay
independent of this local walkthrough aid.

Report schema version 1 remains historical evidence for the accepted learner
slice. The corrected charter uses version 2 rather than silently changing the
meaning of old report rows.

## Risk register

| Risk                                         | Impact | Trigger                                           | Owner      | Mitigation                                                |
| -------------------------------------------- | ------ | ------------------------------------------------- | ---------- | --------------------------------------------------------- |
| Local adapter leaks into production          | High   | Route/control appears without local auth          | WS-ROSTER  | Composition-only mount plus production absence tests      |
| API setup masquerades as instructor coverage | High   | Course/assignment ID exists before browser action | WS-EVID    | Fixed stage order and arrangement allowlist               |
| Pending invitation is called a roster member | High   | Student cannot open new course                    | WS-ROSTER  | Require active row and student browser access             |
| Score proof is too weak                      | High   | Gradebook only opens history                      | WS-WALK    | Assert exact visible percent and completed count          |
| Repeat reuses stale response state           | High   | Second run keeps prior selection                  | WS-WALK    | Attempt-keyed widget and cleared-control assertion        |
| Retained rows hide current targets           | Medium | New course/assignment is off page one             | WS-WALK    | Accepted visible pagination and exact public IDs          |
| Answer material leaks                        | High   | Test/report logs choice or key                    | WS-EVID    | Existing private source boundary and closed report schema |
| Email scope returns                          | High   | Mailbox/provider appears in gate or row           | WS-CHARTER | Durable human guidance and static baseline exclusion      |

## Rollout and release checklist

- [x] Owner correction is recorded in `docs/HUMAN_GUIDANCE.md`.
- [x] Active status and historical baseline/audits are marked superseded for
      the corrected charter.
- [x] Visible instructor course creation passes focused review.
- [x] Local-only canonical roster addition has focused Memory/PostgreSQL,
      production-absence, security, browser, and live empty-stack teaching-loop evidence.
- [x] Visible corpus-backed assignment creation passes focused review.
- [x] Fixed instructor setup emits only validated public IDs.
- [x] Student completes two runs through the keyboard platform path.
- [x] Exact gradebook row visibly proves Best/Latest `100%`, Completed `2`, and
      two completed histories.
- [x] Manager and independent retained-stack `--build` runs pass schema v2.
- [x] Reports remain redacted 0700/0600 and cleanup leaves no containers or
      private state.
- [x] Operator docs and changelog close without an email/onboarding gate.
- [x] Accepted runner lives behind a thin facade in its dedicated package.

## Documentation close-out requirements

- Active plan / progress tracker updates: this plan, implementation status,
  relevant workstreams, report audits, and release-plan cross-reference.
- `docs/CHANGELOG.md` entry: record the corrected instructor-to-student
  walkthrough only after accepted implementation; keep WP-RC8 identity work
  separate.
- Archive / closure notes: retain old schema-v1 evidence as historical learner
  coverage and add explicit supersession notes. Do not rewrite old run facts.

## Patch plan and reporting format

- Patch 1: WP-C0 corrected charter and status.
- Patch 2: WP-I1 visible course creation.
- Patch 3: WP-I2 canonical local roster activation.
- Patch 4: WP-I3 visible assignment construction.
- Patch 5: WP-I4 protected instructor setup and public-ID handoff.
- Patch 6: WP-S1 student keyboard take and repeat.
- Patch 7: WP-S2 visible score and gradebook evidence.
- Patch 8: WP-E1 schema-v2 report/baseline and WP-D1 close-out.
- Patch 9: WP-E2 importable walkthrough-runner package refactor.

Each package reports its owner, touched files, focused commands, observed
behavior, independent review, dependency state, and next package. Live reports
never include credentials, email, student identity, answers, raw score values,
or child output.

## Resolved decisions

- The walkthrough is a local instructor-to-student teaching workflow, not a
  production account-onboarding test.
- Agents do not need and must not be given email accounts for this work.
- An active local roster member is required; a pending invitation is not enough.
- Course, roster, and assignment creation are visible journey coverage, not API
  arrangements.
- Problem publication may remain a supported-API arrangement because visual
  problem authoring is outside this charter.
- J6/J7, J9/J10, all-family, and multi-learner outcomes do not gate or appear in
  the corrected walkthrough baseline.
- Existing learner evidence is preserved. The strengthened four-ID copy/paste path,
  explicit-input real-stack run, refreshed screenshots, and independent review passed on
  2026-08-12; broader release and integrated HOTSPOT acceptance remain separate.
- **Corrective evidence, 2026-08-13:** the opt-in disposable Podman run now waits for successful
  one-shots and every required long-running service before seed or Playwright. It passed the
  instructor setup and two complete four-question learner chapters in two tests in 9.4 seconds
  after the full stack reported ready. The renderer's PID-only, secret-free configuration passed its
  real render/grade probe, and cleanup exited 0 after removing the exact generated project and
  gateway image. This live proof repairs stack and journey composition only; it adds no permanent
  pytest, fixture, or ordinary networked test.

## Open questions and decisions needed

- Manager/subagent decision procedure: no execution-blocking decision remains;
  WP-I2 owns the local alias adapter and its canonical upsert contract above.
- Non-blocking follow-up: retain the accepted closed-exam contrast as focused
  supplemental coverage, but do not include it in the corrected core report.

## WP-HG1 human-guidance workflow

### Context

WP-HG1 is the accepted 2026-08-12 cross-cutting workflow package. This final section is the detailed
authority for its Question ID, instructor construction, whole-run timing, runner-input, and evidence
contracts. The [release completion plan](active/release_completion_plan.md) retains the package summary,
dependency order, and release boundary.

### Objectives

- Give instructors a selectable, copyable human reference for one immutable published question.
- Make pasted Question IDs resolve atomically for the authenticated actor and preserve recoverable work.
- Connect the instructor's whole-run timing choice to the server-backed learner countdown.
- Make the canonical J13 and J1--J8 workflow use explicit, private, schema-versioned runner inputs.

### Design philosophy

Use one human-facing Question ID as the instructor's durable reference while keeping immutable
snapshots, version identity, grading, and authorization server-owned. Keep visible UI actions,
recoverable drafts, and explicit runner inputs as the evidence boundary for teaching workflows.

### Scope

The package owns the cross-boundary reference contract, editor recovery and atomicity, the explicit
runner-input boundary, and the whole-run timing workflow. Catalog/domain, browser/HCI, PostgreSQL,
architecture, security, and HCI owners review their respective boundaries. The package's visible
instructor result is a deliberately constructed four-question Genetics Chapter 1 assignment. The
separate two-chapter eight-question Genetics-plus-Biochemistry publication and learner sweep remains
the RC5 release-content oracle.

The package files are `crates/question_model/src/catalog.rs`, the catalog migration and PostgreSQL
resolver, generated public contracts and strict decoders, `src/api/`, `src/pages/assignment_editor_*`,
mock catalog handlers, focused Rust/TypeScript/Playwright tests, canonical `tests/playwright/e2e/`
scenarios, status, and post-acceptance documentation.

### Non-goals

The release completion plan continues to own RC4--RC12 sequencing, external production activation,
institutional sign-off, and credentials. The RC5 eight-question release oracle remains separate from
the four-question instructor construction. Current WP-INST-S3 and WP-INST-T1 authorities remain the
policy and transport sources for current timing; this package records their accepted predecessor
contract and evidence.

### WP-HG1 contract

- The editor displays and copies a human reference, accepts exact pasted Question IDs in one obvious
  add-by-ID control, resolves the one immutable published question named by that ID under the current
  actor authorization, and changes the assignment only after a whole pasted batch resolves.
- Malformed, unavailable, unauthorized, duplicate, race, and network cases preserve pasted text and the
  existing draft with labelled recovery. Displayed Question IDs remain selectable and copyable in
  canonical `AAA-BBBB` form. The browser uses no UUID as a question identifier and exposes no
  UUID-valued DOM helper solely for test extraction.
- One seven-character Crockford Base32 Question ID is displayed as `AAA-BBBB`. The server validates its
  HMAC-derived checksum before resolving that exact published question through current actor
  authorization.
- Live and mock resolver semantics agree: malformed or checksum-invalid is 400, unavailable is 404,
  unauthorized is 403, and an accessible exact published question succeeds. PostgreSQL conformance
  proves that a valid Question ID cannot resolve an inaccessible restricted question.
- Hidden immutable snapshots and version identity remain internal for authorized replay, grading, audit,
  provenance, and transport. Instructor-facing selectors and latest-resolution paths use the human
  Question ID contract.

### Canonical walkthrough

J13 searches the published catalog to find displayed human references, copies and pastes the four
Genetics Chapter 1 `AAA-BBBB` Question IDs, visibly observes four selected questions, creates the
assignment, and hands only public course/assignment identifiers to later student stages. The setup
uses visible UI actions to arrange the assignment; an API-created assignment is not walkthrough
evidence. Later stages receive no answer material or UUID-derived selector. The eight-question
Genetics-plus-Biochemistry sweep remains a separate release-content oracle.

The Python runner exposes operator choices through documented arguments or the selected Compose file,
clears inherited `PLE_*` walkthrough overrides from owned children, and hands fixed Node/Playwright
stages one schema-versioned mode-0600 private input file by explicit argument. This narrow process
contract keeps child configuration bounded and reviewable.

### WP-HG1.T timing record

WP-HG1.T was accepted on 2026-08-12 and closes the timed-problem gap in the human-guidance walkthrough.
Its historical dependency order was the then-course-owned `AssignmentTimingPolicy`, the visible
assignment editor, and the shared assignment revision. The `AssignmentTimingPolicy` and its API have
since been removed. The accepted implementation order was the Rust/store/editor contract, followed by
the Solid form, focused behavior gates, and the current-stack walkthrough.

At acceptance, whole-run timing was course-owned through `AssignmentTimingPolicy`. The editor exposed
`assignmentTiming: { timeLimitSeconds: positive u32 | null }`; `null` represented an intentional
untimed assignment; and a new mastery draft received the Rust-generated `900`-second default. Create,
update, and editor GET composed assignment definition and timing atomically under one revision.
Published question versions and immutable question-level `TimingPolicy` stayed unchanged.

Current policy authority is accepted WP-INST-S3's current S3-resolved effective-policy verdict/decision
after S5 entitlement; sealed receipts preserve the historical acceptance evidence. WP-INST-T1 owns the
current transport and whole-run timing in the single `AssignmentTeachingSettings` aggregate with
lifecycle, instructions, schedule, limits, late, and deadline behavior. The accepted WP-HG1.T record is
historical evidence and does not define a compatibility reader or writer.

The instructor sees and saves an accessible `Time limit for each practice run` fieldset with
Timed/Untimed choices and a minutes input; a new mastery assignment visibly starts at 15 minutes. The
student sees a server-backed countdown for a timed run, or `Untimed` when the saved value is null.
Invalid input and conflicts preserve the instructor's draft.

### Permanent tests

- Parser/domain bounds; Rust/PostgreSQL resolver and RLS conformance; strict browser
  decoder/client/repository recovery; mock/live error-class parity; editor batch atomicity, duplicate
  recovery, and keyboard submit; and a focused visible Playwright copy/paste setup test.
- Rust Memory/PostgreSQL editor conformance for timing default, atomic create/replace, stale revision,
  and active-run deadline handling; strict HTTP/decoder/client tests for the nullable field; and
  keyboard-focused editor/student Playwright tests for timing default, toggle, validation, recovery,
  and saved display.

These tests assert behavior and contracts rather than exact fixture counts, CSS strings, source strings,
or implementation names.

### One-time acceptance evidence

- A rebuilt current-stack J13/J1--J8 run with clipboard permission, redacted report, and refreshed public
  instructor screenshots.
- The isolated PostgreSQL/MinIO eight-question publication oracle.
- Independent architecture, security, and HCI review.
- The real Podman PostgreSQL plus `webwork-pg-renderer` walkthrough: the instructor creates the Genetics
  assignment from copied `AAA-BBBB` Question IDs with the 15-minute default visible; the student sees
  the server-backed countdown, completes/retries, and starts a fresh timed practice run; public
  screenshots are refreshed and visually reviewed at the guide boundary.

The clean-stack walkthrough and screenshot capture supplied this one-time evidence. The evidence is
recorded separately from permanent behavior tests and remains distinct from the RC5 release-content
oracle.

### Evidence boundary and success

Permanent evidence covers Question ID parsing/resolution, editor recovery and atomicity, explicit runner
configuration, assignment timing, keyboard use, and responsive task completion. These checks protect
teaching behavior, authorization boundaries, and recoverable work.

The accepted success condition is a truthful instructor-to-student walkthrough: the instructor copies
and pastes human-readable Question IDs, adds the exact published questions, constructs the four-question
Chapter 1 assignment, and saves a course-owned 15-minute whole-run limit. The student receives a
server-backed countdown, can complete/retry, and can start a fresh timed practice run. Whole-run timing
is not duplicated into flat or WeBWorK question sources.
