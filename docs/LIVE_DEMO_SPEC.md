# Live Demo Specification

The demo is the real PLE system: one installation with global accounts and server-issued account sessions. The
baseline is seeded data. Any visitor may directly select one of five seeded-account personas representing Student,
Instructor, or Sysadmin. The selection replaces only identity verification; the server resolves the global `UserId`,
issues the ordinary account session, and derives `ActorContext { user_id, session_id }`. Visible course selection is
limited by exact current course membership, Student records by Student ownership, and the shared tagged
published-question catalog by approved-Instructor authority. All resulting data is ordinary live data. The data is
disposable because the entire installation can be regenerated, not because accounts, records, or roles behave
differently.

## Purpose

The live demo is a fully functional PLE installation, not a fixed walkthrough or read-only demonstration. The
demo uses the normal PLE application workflows, authorization, database, and storage.

The demo starts with seeded baseline data that gives people something useful to explore immediately. Once the
system starts, the seeded data is normal live data rather than special demo content.

## Seeded baseline

The initial data contains recognizable ordinary teaching courses with representative instructors, students,
assignments, problems, active memberships, and Student work. People may explore and modify this data and create
additional data through the normal PLE workflows.

The seeded Student and Instructor personas share ordinary course relationships. Elena teaches `Biochemistry:
Protein Structure and Function`; Mary and Jack are active students in that same course. Work completed as Mary or
Jack persists under that course enrollment, and Elena sees the resulting best score, latest score, completed-run
count, and authorized run history in the course gradebook after a fresh server read.

The ordinary teaching-course baseline includes `Biochemistry: Protein Structure and Function`, `Genetics: Foundations
of Inheritance`, and `Biochemistry: Molecular Foundations`. Installer diagnostics call the installed Biochemistry
course's seeded installation recipe `Base Course`; product surfaces use the teaching-course title. Morgan and Avery
retain their separate ordinary authorization course.

Before first production deployment, the reviewed clean-cluster baseline reissues migration `2026081818` with the final
visible Biochemistry teaching title. Regenerating disposable live-demo volumes applies that baseline, whose resulting
checksum is the canonical immutable v1 baseline. The first shipped baseline therefore begins with this coherent
teaching-course topology; the established forward-only migration ledger applies after v1 ships.

`BlueprintCourse` is course-level reusable content and structure. It is visible to all vetted (approved) Instructors,
has no enrolled Students, and has no live deadlines, releases, accommodations, grades, or delivery settings.
`CourseInstance` is created from exactly one Blueprint Course and is private to its current equal co-Instructors and
enrolled Students. Its parent identity is immutable. It owns deadlines, releases, accommodations, grades, and delivery
settings for that teaching instance; delivery settings are never inherited as Blueprint authority.

Creating a Course Instance through the UI selects an existing Blueprint Course or first creates a minimal new Blueprint
Course. A referenced Blueprint Course archives instead of being hard-deleted. The Course Instance shows its applied
Blueprint revision and any controlled-update state so co-Instructors can review propagation before release.

When an Instructor adds an assignment to a Blueprint Course, the assignment propagates to each daughter Course
Instance as unreleased. The current co-Instructors review and release it in the Course Instance before enrolled
Students can receive it. ADAPT's alpha course is comparison vocabulary only; it is not a PLE aggregate, route, or
browser workflow.

### Seeded discovery evidence

Five deterministic live Student observations are distributed across meaningful ordinary Chapter 1 assignments titled
`Molecular Foundations: Charged Functional Groups` in the Genetics and Biochemistry teaching courses. They use
ordinary active memberships and Student work, so the existing item-analysis and discovery surfaces can show useful
evidence in context. The observations are presented through those ordinary course and evidence surfaces.

Course navigation presents the recognizable teaching courses through active server-owned relationships, including the
ASVS 8.2.2 and 8.3.1 authorization boundary. An Instructor sees courses where an active teaching membership grants
authority; a Student sees courses with an active Student membership and only that Student's records; and a Sysadmin
reaches academic course records through a direct teaching membership or the separately audited support relation.
Seeded memberships provide representative ordinary teaching-course context for the visual walkthrough.

Data created or modified while using the demo persists normally in the database and storage. The data does not
need to survive regeneration of the demo. Preserving the demo database and storage preserves the current live
data.

Preview resolves the current state of these ordinary live courses, assignments, published questions, and graders.
Instructors validate delivery and automated grading through the visible production workflow. Student runs,
submissions, grades, and instructor review are ordinary PLE records created by those workflows.

### Visual evidence profiles

Instructor and Sysadmin captures use the fixed `laptop` profile at exactly 1280 by 800 CSS pixels in a desktop 16:10
viewport. Student captures use the maintained `laptop` (1280 by 800), `tablet` (800 by 1280), `iphone_pro` (393 by
852), and `square` (800 by 800) profiles. These names and dimensions are defined by
`tests/e2e/browser_screenshot_corpus.json`, the durable visual-evidence authority. Review each applicable capture for
semantic usability, accessibility, privacy, and task completion.

### WebWork catalog baseline

The frozen baseline contains one reviewed WebWork catalog item, **Biochemistry: Identify hydrophobic compounds
from formulas**. The host-only baseline installer validates the tracked source provenance and digest, writes its
immutable private source, and reconciles the catalog publication. It provides the browser-suite owner only the
public Question ID and title needed to find the item through the Library.

This catalog publication is infrastructure bootstrap rather than teaching state. It creates no course,
assignment, roster member, invitation, Student run, or submission. Instructor and Student journeys create those
ordinary PLE records through the visible interface. The private source, object identity, renderer configuration,
credentials, and answer material remain outside browser receipts and screenshots.

The catalog-only installer accepts the publishing Instructor, migration/database, and private-storage coordinates
required to reconcile that publication. It writes to the one shared catalog and does not accept or retain a Student
identity because no Student record belongs to this bootstrap boundary.

### Question stewardship workflow

The live stewardship journey uses one vetted Instructor and the shared catalog. Star is the canonical visible
endorsement: the Instructor stars a published question, and vetted-Instructor question detail shows its star count
and the identities of vetted Instructors who starred it. Watch is separate and private to the current Instructor's
in-app subscription; changes appear in Watched activity because email delivery is not configured. Student and
anonymous flows expose neither star identities nor watch state. Collections and saved searches remain separate
features. The question detail shows the immutable published version represented by its Question ID and its visible
fork lineage. The hidden exact `(ProblemId, VersionId)` remains server evidence and never becomes browser authority.

The Instructor forks that publication into a private draft, validates it, and publishes the fork as a new immutable
Question ID. The catalog then shows the visible source/fork lineage and a controlled-update impact item. The Instructor
can inspect privacy-safe, version-specific aggregate evidence for attempts, correct outcomes, and eligible choices.
Those aggregates apply the catalog disclosure threshold and contain no Student identity, raw response, answer key,
grading payload, or private source.

Assignments in a Blueprint Course and Course Instance remain pinned to their exact published Question ID and hidden
exact version evidence until an Instructor explicitly adopts a controlled update. No assignment silently resolves the
latest version, and a watched update does not alter existing Course Instance or Student work.

## Instructor perspective

The demo should allow an Instructor to use the normal instructor workflows, including:

- Create courses.
- Create assignments.
- Create problems.
- Use the Library to browse the one global tagged published-question corpus available to approved Instructors,
  inspect its safe evidence, save searches, organize collections, and select a question for an
  assignment. Keep each unpublished question inside its private authoring workspace until publication succeeds.
- Star and watch a published question, confirm the vetted-Instructor star count and identities on its detail, and
  find watched changes in the private in-app Watched activity view. Inspect its immutable version and visible fork
  lineage, and review privacy-safe version-specific attempt, correct-outcome, and eligible-choice evidence. Student
  and anonymous views expose neither star identities nor watch state; email delivery is not configured.
- Create and revise a Blueprint Course's reusable content and structure. Confirm that every vetted Instructor can see
  it, that no Student is enrolled, and that it has no live deadline or delivery state.
- Create a Course Instance from the Blueprint Course. Confirm that its current equal co-Instructors and enrolled
  Students are the only people who can see its private teaching state, including deadlines, releases, accommodations,
  grades, and delivery settings.
- Add a new assignment to the Blueprint Course, then open its daughter Course Instance and observe the propagated
  assignment marked unreleased. Release it explicitly before the enrolled Student can receive it.
- Select an assignment title to open that assignment's Instructor Overview. Use the separate Questions page for
  fixed questions, pools, ordering, and reuse; use the separate Policies page for delivery and lifecycle rules.
- Open Student view for a stable-identity, answer-free inspection of the current live assignment while retaining
  the Instructor session. Use ordinary Student entry when graded work is needed.
- Build one ordered assignment from fixed questions and reusable question pools by using public Question IDs. Configure
  each pool's draw count and delivery order, then request fresh server-generated preview draws without creating Student
  work or changing grades.
- Review an answer-free server preview before every live change. Completed changes return an immutable receipt and a
  reconciliation result; the visible destination and next action remain available after apply.
- Add students to courses.
- Preview current assignment policy, then exercise delivery and automated grading through the normal Student
  workflow.
- Open **Grading operations** to review assignment-local automatic-grading interruptions, retry one eligible
  operation, or request an assignment recalculation. The page exposes metadata and safe next actions only.
- Manage and review student activity and grades.

The assignment workspace keeps one clear path for teaching work: the linked title opens Overview, local navigation
connects Overview, Questions, Policies, Grading operations, and Student view, and each page reports its current state. Student view
creates no Student work. Entering as the ordinary demo Student does create a real run, submission, and grade through
the normal workflow; the Instructor can see that graded work in the gradebook after a fresh read.

### Automated grading recovery

The connected recovery journey uses ordinary visible Student and Instructor actions. The Student submits one answer
and sees **Response received** with a cleared answer buffer and **Check grading status**. The answer POST is not
replayed after acknowledgement; each status GET is answer-free and `no-store`.

The acceptance-only fault profile then records one deterministic grader exception. The Student sees **Your response
needs instructor attention**. Elena opens the assignment's **Grading operations** page, reviews the safe metadata row,
and selects **Retry automated grading for [question]** exactly once. The ordinary worker claims the
new execution generation and runs the accepted private response through the shared server handler. After completion
and current-score publication, Elena
opens the current Gradebook and observes Mary's resulting total. Student status, operation responses, and receipts do
not contain the answer, feedback internals, grading source, or score values.

Instructors invite already-approved colleagues into their own teaching course. Sysadmins own global
Instructor approval; an invitation grants only the accepted course membership.

Live acceptance uses the seeded Elena Instructor. After direct role entry, Elena visibly enrolls a passkey, signs
out, and signs back in through the ordinary passkey path. Her Instructor authorization remains available throughout
that acceptance flow.

### Blueprint Course workflow

The Blueprint Course workflow is a production-shaped, server-owned Instructor workflow. Elena creates and revises a
Blueprint Course through labeled controls, then creates a Course Instance from its reusable content and structure (or
selects an existing Blueprint Course). The Course Instance has exactly one immutable Blueprint Course parent.
The Course Instance has its own private current co-Instructor and Student relationships and owns its deadlines,
releases, accommodations, grades, and delivery settings. A new assignment added to the Blueprint Course appears in
the daughter Course Instance as unreleased; Elena observes it there, reviews its answer-free preview, and explicitly
releases it before a Student can receive it. The server derives authority and destination identity from the account
session and exact course membership, commits each change atomically, and returns an immutable receipt.

## Student perspective

The demo should allow a Student to use the normal student workflows, including:

- Enter courses in which the Student is enrolled.
- Complete assignments.
- Receive the fixed questions and server-selected pool questions issued for that run. The issued selection remains
  attached to the Student work even when an Instructor later changes teaching metadata or prepares future content.
- Submit answers.
- View permitted feedback and grades.
- Repeat assignments where allowed.

Student activity in the demo is normal live PLE data. Ordinary demo Student entry creates real graded work through
the visible assignment flow, and the Instructor sees the resulting score and authorized evidence in the gradebook.
Students cannot browse the shared published-question catalog; they receive only the exact questions entitled by their
current course membership and assignment.

## Sysadmin perspective

The demo should provide the full Sysadmin experience, including adding and approving Instructors and the other
normal Sysadmin functions. The demo Sysadmin is a normal PLE Sysadmin with the same capabilities as a Sysadmin
in any other PLE installation. PLE continues to have only the normal Student, Instructor, and Sysadmin human
roles. See [USER_ROLES.md](USER_ROLES.md) for the role authority.

Any visitor may select the seeded Sysadmin directly. Direct entry preserves the full ordinary Sysadmin capability
set and the normal account security surfaces, including passkey enrollment and passkey sign-in. Regenerating the
demo restores the seeded baseline and its ordinary live state.

Live acceptance uses the seeded Morgan Sysadmin. After direct role entry, Morgan visibly enrolls a passkey, signs
out, and signs back in through the ordinary passkey path while retaining ordinary Sysadmin authorization and the
full capability set.

### Forced question correction workflow

The live correction journey uses the named `ForcedQuestionCorrection` workflow. A vetted Instructor submits a
validated replacement for a published question and reviews the FERPA-safe impact summary. Morgan approves the
correction through the ordinary Sysadmin UI; approval atomically advances the active reference to the replacement
without mutating the immutable original question, its version-specific evidence, or prior work. The browser then
shows the deterministic result for affected delivery: reissue, excuse, or recalculation as selected by the workflow,
along with the immutable original evidence and the superseding correction receipt.

The originating Instructor sees the correction impact and resulting outcome in the normal teaching UI. No per-course
approval is required. The correction does not expose Student identity, raw responses, answer keys, grading payloads,
or private source.

The integrity of the demo data does not need to be protected from changes made through normal Sysadmin
capabilities. The entire installation is a disposable demonstration environment. A Sysadmin may modify or delete
seeded or user-created data just as in a normal installation. Regenerating the demo database and storage
restores the seeded baseline.

## Demo authentication

Every seeded demo persona can be entered directly through the public selector. The five closed persona keys map to
global account identities; they are not PLE roles, course memberships, or authorization claims.

Selecting a seeded persona replaces only the normal identity-verification ceremony. The selector supplies only a
known closed persona key. The server resolves the configured global account, creates the ordinary account session,
derives `ActorContext { user_id, session_id }`, and applies exact course-membership, Student-ownership,
approved-Instructor, and other authorization predicates from live PLE state. The selector does not supply or grant a
browser-controlled role, course, membership, or account claim.

The browser selects only a known demo persona. Account identity, Instructor approval, exact course membership,
Student ownership, shared-catalog visibility, and authorization continue to be derived by the server from normal PLE
state.

Bound selector traffic by caller network and by the live-demo deployment as a whole. Seeded personas are public
shared entries, so independent visitors remain able to choose the same role without sharing a persona-specific
lockout budget.

Conceptually:

    Select any seeded Student, Instructor, or Sysadmin account persona
                |
                v
    Server resolves seeded PLE account
                |
                v
       Ordinary account session
                |
                v
    Ordinary course/role authorization
                |
                v
         Ordinary PLE session

After authentication, there is no separate demo application path. The selector's fixed seed correlation value, if
used by the disposable harness, remains private bootstrap metadata and never becomes account, session, course,
catalog, or browser authority.

## Passkey enrollment and sign-in

Direct persona entry leaves the normal account-security workflow available. Vetted Instructors use ordinary
passkey/email-code login. Live acceptance explicitly covers Elena (Instructor) and Morgan (Sysadmin): after selecting
each seeded persona, the visitor visibly enrolls a passkey, signs out, and signs back in through the ordinary passkey
path. Elena retains Instructor authorization and Morgan retains ordinary Sysadmin authorization and the full
capability set throughout the flow.

Direct persona entry has no first-claim, password, or setup-code step. It replaces only identity verification; account
ownership, session issuance, exact course membership, Student ownership, and authorization continue to use normal PLE
contracts.

Conceptually:

    Fresh demo baseline
            |
            v
    Select seeded Morgan Sysadmin role
            |
            v
    Ordinary Sysadmin account session
            |
            v
    Passkey enrollment
            |
            v
    Sign out and passkey sign-in
            |
            v
    Ordinary Sysadmin session

Regenerating the demo restores the seeded baseline and discards the passkey state with the disposable installation.

## Demo credentials and deployment secrets

PLE generates and manages any internal demo credentials needed for process isolation, service startup, or reset.
These credentials are disposable process-isolation capabilities for the current demo installation; they are not
visitor secrets, role claims, or durable application credentials, and they stay out of public browser evidence.
The accepted-submission API fast path and recovery worker receive separate generated
credentials through `PLE_ACCEPTED_SUBMISSION_FAST_PATH_DATABASE_URL` and
`PLE_ACCEPTED_SUBMISSION_RECOVERY_DATABASE_URL`; the fast-path value stays API-only
and the recovery value stays worker-only. The private baseline runtime carries both
values without placing either in the browser or its evidence.

SOPS is reserved for a later deployment design that needs persistent or externally supplied credentials. The public
live demo does not require SOPS to protect its disposable internal process-isolation credentials.

## Demo lifecycle

The live demo uses one implementation. Courses, assignments, problems, accounts, memberships, student work,
grades, previews, and other application state use the normal PLE data model. Instructor validation of delivery
uses the same Student-run, submission, deterministic-grading, receipt, and gradebook paths used by a live course.

The distinction between a live demo and another PLE installation is primarily:

1. The installation begins with a known seeded baseline.
2. Public seeded-role entry provides a convenient entry into normal account, session, course, and role handling for
   Student, Instructor, and Sysadmin.
3. Direct entry keeps ordinary passkey enrollment and sign-in demonstrable, including for Sysadmin.
4. The database, storage, and disposable process-isolation credentials may be discarded and regenerated from the
   seeded baseline.

The live demo therefore remains the real PLE system. The initial state is simply seeded data for people to play with.
