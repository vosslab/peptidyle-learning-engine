# Live Demo specification

## Current executable boundary

The Live Demo is a disposable HTTPS deployment of the current PLE application.
Its development entry admits the closed five-persona set: Elena Instructor;
Mary, Jack, and Avery Student; and Morgan Sysadmin. Choosing a persona replaces
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

The seeded Elena Instructor can use the implemented Question Library,
authoring, Blueprint Course, Course Instance, roster, Assignment Workspace,
release, Gradebook, and invitation-export routes subject to their stored
relationships. The browser cannot send invitation mail.

## Student perspective

Each seeded Student can claim an invitation, enter their Course and released
Assignment landing pages, start an authorized Assignment Attempt, complete
native controls, and submit a response. Student-visible feedback is not implied
by terminal grading state and is released only through its separate policy.

## Sysadmin perspective

The seeded Morgan Sysadmin can use the Instructor Accounts task and a registered
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
