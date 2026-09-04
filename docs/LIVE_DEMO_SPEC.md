# Live Demo specification

## Current executable boundary

The Live Demo is a disposable HTTPS deployment of the current PLE application.
Its current browser entry is a deployment-gated, visible selector for five
seeded personas: Elena Instructor; Mary, Jack, and Avery Student; and Morgan
Sysadmin. Selecting one persona replaces only identity verification. The server
resolves the configured Account and issues the ordinary Authenticated Session.

The selector does not grant a Product Role, Course Membership, Student record,
course authority, or object access. Every future authorization decision remains
derived from stored Account and relationship data. The implemented API surface is
limited to health, session resolution/logout, and the seeded account selector;
see [API_CONTRACTS.md](API_CONTRACTS.md).

The demo's database and object storage are disposable. Regeneration replaces
their seeded state. Current implementation acceptance covers the declared
PostgreSQL and Course Appearance PostgreSQL/MinIO service lanes; it does not
prove a visible browser teaching journey.

## Purpose

The current demo provides a real session and deployment boundary for local
development. It is not presently a fully executable course-delivery,
authoring, grading, or administration walkthrough.

## Visual evidence profiles

When a production browser workflow is restored, Instructor and Sysadmin
captures use the `laptop` profile at 1280 by 800 CSS pixels. Student captures
also use `tablet` (800 by 1280), `iphone_pro` (393 by 852), and `square`
(800 by 800). The restored browser owner records semantic usability,
accessibility, privacy, and task-completion review before claiming visual
acceptance.

## Illustrative future teaching workflows

The following product workflows remain retained design contracts and are not
implemented Live Demo actions:

- Instructors author shared Question Library content and reusable Blueprint
  Course content, then create Course Instances for teaching.
- Course Instances own exact memberships, deadlines, releases,
  accommodations, delivery settings, grades, and Student work.
- Students receive Questions only through an allowed Assignment Access decision
  for their exact Course, Assignment, and Student record.
- Question submission and automated grading preserve accepted evidence;
  recovery does not ask a Student to resend private response content.
- Instructor and Sysadmin workflows use exact relationship authority and
  privacy-safe receipts. Answer Keys, Question Graders, private Question
  Source data, credentials, and raw grading input remain server-held.

A future Blueprint update never silently rewrites Course Instance-owned
teaching records or issued Student evidence. A future Question Library surface
does not grant Student delivery. These are security and ownership requirements,
not current route claims.

## Instructor perspective

The current demo can establish the seeded Elena Instructor's Authenticated
Session. Course, Blueprint, assignment workspace, roster, grading-operation,
and Gradebook Server Routes do not exist. The intended Instructor workflow is
documented as a future contract in [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md).

## Student perspective

The current demo can establish a seeded Student's Authenticated Session. Course
delivery, Assignment Attempt, Question submission, Student Feedback, and grade
Server Routes do not exist. The intended Student workflow is documented as a
future contract in [STUDENT_GUIDE.md](STUDENT_GUIDE.md).

## Sysadmin perspective

The current demo can establish the seeded Morgan Sysadmin's Authenticated
Session. Its Browser Surface does not provide academic-course administration, Question correction,
or Instructor-vetting workflows. Those retained product boundaries must
derive authority from their exact stored relationships when their service routes
and browser acceptance are restored.

## Demo authentication

The closed seeded persona set replaces only the normal identity-verification
ceremony. It has no password, first-claim, or setup-code step. The server
creates a host-only, Secure, HttpOnly, first-party session cookie and resolves
the session from durable server storage. Passkey and email-code adapters remain
future authentication acceptance work; this selector is the current local-demo
entry path.

## Browser restoration requirement

Production browser acceptance remains a release-blocking restoration
requirement. A restored owner must implement and expose the intended routes, exercise a
visible browser journey against the disposable stack, and record its separate
evidence. Passing service-only acceptance cannot substitute for that evidence.
