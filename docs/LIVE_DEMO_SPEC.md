# Live Demo specification

## Current executable boundary

The Live Demo is a disposable HTTPS deployment of the current PLE application.
Its development entry admits the closed five-persona set: Elena Rivera, Instructor;
Mary Okafor, Jack Nguyen, and Avery Thompson, Students; and Morgan Delgado,
Sysadmin. Choosing a persona replaces
identity verification only. The server resolves the configured Account and
issues the ordinary Authenticated Session.
Configuration is evaluated per persona: an absent, malformed, or ambiguously
duplicated mapping is omitted rather than coerced. When at least one valid
mapping remains, seeded entry stays usable for those personas and reports only
the bounded count of unavailable demo Accounts. A stored Account whose Product
Role does not match its fixed persona yields a bounded unavailable result after
the unexposed session is revoked. When no configuration mapping remains, only
seeded entry is absent; health and ordinary session/logout routes remain
available.

Seeded entry does not grant a Product Role, Course Membership, Student record,
course authority, or object access. Every authorization decision derives from
stored Account and relationship data. The application then exposes the
implemented role- and relationship-gated routes listed in
[API_CONTRACTS.md](API_CONTRACTS.md).

The demo's database and object storage are disposable. Regeneration replaces
their seeded state. M12-M18 have their recorded focused acceptance, and M19
accepted a fresh serial production-browser run of the production bundle against
the fixed HTTPS stack on 2026-09-07.

## Purpose

The Live Demo is the connected PLE workflow, not a presentation substitute.
Current implementation includes Instructor Question authoring and publication,
Blueprint and Course Instance setup, roster invitation and claim, Assignment
authoring and release, Student delivery and response controls, submission and
recovery, WeBWorK grading, Gradebook, Sysadmin Instructor Account management,
scoped roster support, and Instructor-only invitation export. Each protected
operation retains its server and Store authorization boundary.

The primary success criterion is human: after `./launchers/run_live_demo.sh`, a
reviewer can use the seeded Instructor and Student personas to exercise the
major launch-critical workflows and decide whether PLE is ready for real users.
Seeded data exists to expose those workflows and their accumulated product
state; it never creates a parallel demonstration model.

## Required teaching-data baseline

A successful `./launchers/run_live_demo.sh` start establishes one fictional,
disposable teaching graph through ordinary product HTTP contracts. The SQL
seed remains limited to the five foundational Accounts, three Student
Authentication Emails, and four Published Questions. After service readiness,
Elena Rivera and the three Student Accounts create the Course-domain records
through the same relationship-gated routes used by ordinary product workflows.

The reusable source is the Blueprint Course `Biochemistry 301: Proteins and
Peptides`. Elena Rivera owns its published Blueprint Revision. Her Fall 2026
Course Instance has the same title, runs from 2026-08-24 through 2026-12-11 in
`America/Chicago`, and has Elena as its Assigned Instructor. Its released
Assignment `Peptide Structure Practice` contains PNE-0001, PNE-0002, PNE-0003,
and PNE-0004 in that order. It has no due date, accepts late work, and tells
Students: `Complete the four practice questions on peptide structure and
properties.`

The roster-driven import and claim workflow establishes the three active
Student Course Memberships and course-scoped Student Records. Their declared
startup facts are:

| Student | Roster ID | Product facts after startup |
| --- | --- | --- |
| Mary Okafor | `BIO301-MARY` | 1 Assignment Attempt; 4 Question Submissions; 4 terminal Grading Results |
| Jack Nguyen | `BIO301-JACK` | 1 open Assignment Attempt; 2 Question Submissions; 2 unanswered Issued Questions |
| Avery Thompson | `BIO301-AVERY` | An available released Assignment and no Assignment Attempt |

The interface derives completed, in-progress, and not-started labels from
those records. Provisioning never writes a parallel demo-only Assignment or
grade state. Repeated starts detect each stage through its product read route
and apply only missing operations. A retained disposable manifest identifies
the created Blueprint Course, Course Instance, and Assignment by their public
References; title matching is only recovery when that manifest fact is absent
or stale. The current mode-0600 baseline report is
`local_stack_state/live_demo_browser/workspace/live_demo_course_report.json`;
its public References and outstanding-stage list are controller evidence, not a
browser data source.

This graph is launch-readiness evidence only when Elena can inspect its roster
and Gradebook, Mary can inspect completed graded work, Jack can resume the open
attempt, and Avery can start from the beginning. Authorization remains bound
to exact Account, Course Membership, Student Record, Assignment Attempt, and
Question Attempt relationships. Student Work Records are FERPA-sensitive, and
Morgan Delgado's Sysadmin Product Role supplies no ambient academic access.
The projections expose only the fields required for each workflow (ASVS
8.1.1, 8.1.2, 14.1.1, and 14.2.6).

## Visual evidence profiles

M20's current role-owned captures use the `laptop` profile (1280 by 800 CSS
pixels) for Instructor, Student, and Sysadmin. Student captures also use
`tablet` (768 by 1024), `phone` (390 by 844), and `square` (800 by 800). The
manifest lists their exact safe surfaces in
`docs/screenshots/current_capture_manifest.json`; rendered captures remain
one-time evidence rather than a substitute for the serial browser owner.
[SCREENSHOT_CONTRACT.md](SCREENSHOT_CONTRACT.md) defines the role ownership: the
Live Demo supplies seeded execution data, not separate presentation chrome.

## Current boundaries

Students receive Questions only through an allowed Assignment Access decision
for their exact Course, Assignment, and Student record. Submission and grading
preserve accepted evidence; recovery does not ask a Student to resend a
response. The status projection is bound to the current Question Presentation
nonce and reports only its grading state. It does not disclose correctness,
point totals, Answer Keys, source, private feedback internals, or raw grader
input; Student Feedback remains a separate policy-evaluated projection.

The browser downloads the protected Course Invitation export but does not send
mail. `launchers/send_invitations.py` remains the local attended mail action.
Sysadmin roster access requires one registered, time-bounded support capability;
it does not create ambient Course or Student-record authority.

## Instructor perspective

The seeded Elena Rivera Instructor Account can use the implemented Question Library,
authoring, Blueprint Course, Course Instance, roster, Assignment Workspace,
release, Gradebook, and invitation-export routes subject to their stored
relationships. The browser cannot send invitation mail.

## Student perspective

Each seeded Student can claim an invitation, enter their Course and released
Assignment landing pages, start an authorized Assignment Attempt, complete
native controls, and submit a response. Student-visible feedback is not implied
by terminal grading state and is released only through its separate policy.

## Sysadmin perspective

The seeded Morgan Delgado Sysadmin Account can use the Instructor Accounts task and a registered
scoped roster-support operation. Those routes do not grant academic Course
authority, Question correction, or unbounded Student-record access.

## Demo authentication

The closed seeded persona set replaces only the normal identity-verification
ceremony. It has no password, first-claim, or setup-code step. The server
creates a host-only, Secure, HttpOnly, first-party session cookie and resolves
the session from durable server storage. Passkey and email-code adapters remain
future authentication acceptance work. The passkey capability is deferred and
supplies no route, setup credential, or Browser Surface; seeded entry is the
current local-demo identity-verification path.

## Connected-browser evidence

M19 accepted `./devel/run_playwright_tests.sh --build` on 2026-09-07. Its serial
owner rebuilt the disposable stack and exercised visible connected journeys.
Focused service or earlier milestone evidence remains narrower evidence and
does not substitute for that completed browser run.
