# Live Demo Specification

The demo is the real PLE system. The baseline is seeded data. Any visitor may directly select each seeded demo
role: Student, Instructor, or Sysadmin. The selection replaces only the identity-verification ceremony; the server
resolves the ordinary account, session, course, membership, role, and authorization state. All resulting data is
ordinary live data. The data is disposable because the entire demo environment can be regenerated, not because demo
records or roles behave differently.

## Purpose

The live demo is a fully functional PLE installation, not a fixed walkthrough or read-only demonstration. The
demo uses the normal PLE application workflows, authorization, database, and storage.

The demo starts with seeded baseline data that gives people something useful to explore immediately. Once the
system starts, the seeded data is normal live data rather than special demo content.

## Seeded baseline

The initial data contains recognizable ordinary teaching courses with representative instructors, students,
assignments, problems, active memberships, and learner work. People may explore and modify this data and create
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

Blueprint and Alpha vocabulary stays aggregate-specific. A Blueprint is a personal reusable assignment that cannot be
enrolled in. An Alpha curriculum is a shared reusable curriculum that cannot be enrolled in. Their names identify
those reusable aggregates and remain separate from teaching-course names, teaching assignments, memberships, and
learner work.

### Seeded discovery evidence

Five deterministic live learner observations are distributed across meaningful ordinary Chapter 1 assignments titled
`Molecular Foundations: Charged Functional Groups` in the Genetics and Biochemistry teaching courses. They use
ordinary active memberships and learner work, so the existing item-analysis and discovery surfaces can show useful
evidence in context. The observations are presented through those ordinary course and evidence surfaces.

Course navigation presents the recognizable teaching courses through active server-owned relationships, including the
ASVS 8.2.2 and 8.3.1 authorization boundary. An Instructor sees courses where an active teaching membership grants
authority; a Student sees courses with an active learner membership; and a Sysadmin reaches academic course records
through a direct teaching membership or the separately audited support relation. Seeded memberships provide
representative ordinary teaching course context for the visual walkthrough.

Data created or modified while using the demo persists normally in the database and storage. The data does not
need to survive regeneration of the demo. Preserving the demo database and storage preserves the current live
data.

Preview resolves the current state of these ordinary live courses, assignments, published questions, and graders.
Instructors validate delivery and automated grading through the visible production workflow. Student runs,
submissions, grades, and instructor review are ordinary PLE records created by those workflows.

### WebWork catalog baseline

The frozen baseline contains one reviewed WebWork catalog item, **Biochemistry: Identify hydrophobic compounds
from formulas**. The host-only baseline installer validates the tracked source provenance and digest, writes its
immutable private source, and reconciles the catalog publication. It provides the browser-suite owner only the
public Question ID and title needed to find the item through the Library.

This catalog publication is infrastructure bootstrap rather than teaching state. It creates no course,
assignment, roster member, invitation, learner run, or submission. Instructor and Student journeys create those
ordinary PLE records through the visible interface. The private source, object identity, renderer configuration,
credentials, and answer material remain outside browser receipts and screenshots.

The catalog-only installer accepts the tenant, publishing Instructor, migration/database, and private-storage
coordinates required to reconcile that publication. It does not accept or retain a Student identity because no
learner record belongs to this bootstrap boundary.

## Instructor perspective

The demo should allow an Instructor to use the normal instructor workflows, including:

- Create courses.
- Create assignments.
- Create problems.
- Build one ordered assignment from fixed questions and reusable question pools by using public Question IDs. Configure
  each pool's draw count and delivery order, then request fresh server-generated preview draws without creating learner
  work or changing grades.
- Create and revise a private reusable assignment Blueprint from live Library questions, reload its persisted revision,
  and adapt its ordered questions for ordinary assignment authoring.
- Create and revise a public Alpha curriculum, inspect its answer-free modules and reusable assignments, and reuse an
  Alpha assignment's ordered live questions through the shared assignment picker.
- Create an independent Alpha from its visible public curriculum detail, with a reviewable proposal
  and retained source-lineage evidence.
- Adopt reusable curriculum from the visible course page: instantiate a Blueprint or Alpha, roll a
  course into a target term, shift an unissued course term with server-owned DST correction, inspect
  imports, fast-forward an eligible untouched import, or preserve a divergent import by creating a
  new source-derived draft.
- Review an answer-free server preview before every live change. Completed changes return an immutable receipt and a
  reconciliation result; the visible destination and next action remain available after apply.
- Add students to courses.
- Preview current assignment policy, then exercise delivery and automated grading through the normal Student
  workflow.
- Manage and review student activity and grades.

Instructors invite already-approved colleagues into their own teaching course. Sysadmins own global
Instructor approval; an invitation grants only the accepted course membership.

Live acceptance uses the seeded Elena Instructor. After direct role entry, Elena visibly enrolls a passkey, signs
out, and signs back in through the ordinary passkey path. Her Instructor authorization remains available throughout
that acceptance flow.

### Curriculum adoption workflow

The Instructor route `/instructor/courses/:courseRef/curriculum` is a production-shaped,
server-owned workflow. Elena chooses one course operation, selects a Blueprint or Alpha source when
required, and enters the target term and title through labeled controls. The public Alpha detail at
`/curriculum/:curriculumRef` owns the independent-copy action. Each action returns an
operation-specific, answer-free preview. A preview may identify a missing exact source pin or an
ambiguous/nonexistent local time; the page preserves the choices, exposes the named correction, and
regenerates the proposal before apply.

The supported live operations are:

- Fork an Alpha from its public detail into an independently editable source with lineage evidence.
- Instantiate a Blueprint as an ordinary draft assignment or an Alpha as a new ordinary teaching course.
- Rollover a course into the selected target term without roster, learner records, attempts, grades, or issued work.
- Shift an existing course term atomically when no assignment has issued learner work. Relative calendar-day and
  local-wall-clock schedules resolve in the target IANA zone; DST gaps and ambiguities require visible correction.
- Inspect imports, fast-forward an eligible untouched assignment, or create a new source-derived draft when the
  destination diverged. The existing assignment is never silently overwritten.

Apply accepts only the exact eligible preview. The server derives authority and destination identity from the session
and route, commits atomically, and returns an immutable receipt. Receipt reconciliation repairs only B2-owned derived
rows and refuses when immutable evidence is incomplete. This is ordinary Instructor functionality in the disposable
live installation.

## Student perspective

The demo should allow a Student to use the normal student workflows, including:

- Enter courses in which the Student is enrolled.
- Complete assignments.
- Receive the fixed questions and server-selected pool questions issued for that run. The issued selection remains
  attached to the learner work even when an Instructor later changes teaching metadata or prepares future content.
- Submit answers.
- View permitted feedback and grades.
- Repeat assignments where allowed.

Student activity in the demo is normal live PLE data.

## Sysadmin perspective

The demo should provide the full Sysadmin experience, including adding and approving instructors and the other
normal Sysadmin functions. The demo Sysadmin is a normal PLE Sysadmin with the same capabilities as a Sysadmin
in any other PLE installation. PLE continues to have only the normal Student, Instructor, and Sysadmin human
roles. USER_ROLES.md

Any visitor may select the seeded Sysadmin directly. Direct entry preserves the full ordinary Sysadmin capability
set and the normal account security surfaces, including passkey enrollment and passkey sign-in. Regenerating the
demo restores the seeded baseline and its ordinary live state.

Live acceptance uses the seeded Morgan Sysadmin. After direct role entry, Morgan visibly enrolls a passkey, signs
out, and signs back in through the ordinary passkey path while retaining ordinary Sysadmin authorization and the
full capability set.

The integrity of the demo data does not need to be protected from changes made through normal Sysadmin
capabilities. The entire installation is a disposable demonstration environment. A Sysadmin may modify or delete
seeded or user-created data just as in a normal installation. Regenerating the demo database and storage
restores the seeded baseline.

## Demo authentication

Every seeded demo role can be entered directly through the public demo role selector: Student, Instructor, and
Sysadmin.

Selecting a seeded role replaces only the normal passwordless identity-verification ceremony. The selector supplies
only a known seeded persona key. The server resolves the configured account, creates the ordinary account session,
and applies the ordinary course, membership, role, and authorization rules from live PLE state. The selector does
not supply or grant a browser-controlled role claim.

The browser selects only a known demo persona. Account identity, roles, tenant context, course membership, and
authorization continue to be derived by the server from normal PLE state.

Bound selector traffic by caller network and by the live-demo deployment as a whole. Seeded personas are public
shared entries, so independent visitors remain able to choose the same role without sharing a persona-specific
lockout budget.

Conceptually:

    Select any seeded Student, Instructor, or Sysadmin role
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

After authentication, there is no separate demo application path.

## Passkey enrollment and sign-in

Direct role entry leaves the normal passkey workflow available. Live acceptance explicitly covers Elena (Instructor)
and Morgan (Sysadmin): after selecting each seeded persona, the visitor visibly uses the ordinary account-security
surface to enroll a passkey, sign out, and sign back in with that passkey. Elena retains Instructor authorization and
Morgan retains ordinary Sysadmin authorization and the full capability set throughout the flow.

Direct role entry has no first-claim, password, or setup-code step. It replaces only identity verification; account
ownership, session issuance, course context, and authorization continue to use normal PLE contracts.

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

SOPS is reserved for a later deployment design that needs persistent or externally supplied credentials. The public
live demo does not require SOPS to protect its disposable internal process-isolation credentials.

## Demo lifecycle

The live demo uses one implementation. Courses, assignments, problems, accounts, memberships, student work,
grades, previews, and other application state use the normal PLE data model. Instructor validation of delivery
uses the same learner-run, submission, deterministic-grading, receipt, and gradebook paths used by a live course.

The distinction between a live demo and another PLE installation is primarily:

1. The installation begins with a known seeded baseline.
2. Public seeded-role entry provides a convenient entry into normal account, session, course, and role handling for
   Student, Instructor, and Sysadmin.
3. Direct entry keeps ordinary passkey enrollment and sign-in demonstrable, including for Sysadmin.
4. The database, storage, and disposable process-isolation credentials may be discarded and regenerated from the
   seeded baseline.

The live demo therefore remains the real PLE system. The initial state is simply seeded data for people to play with.
