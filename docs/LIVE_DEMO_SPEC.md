# Live Demo Specification

The demo is the real PLE system. The baseline is seeded data. Student and Instructor login only replaces the
identity-verification ceremony. Sysadmin exercises the real passkey path and has full Sysadmin capabilities. All
resulting data is ordinary live data. The data is disposable because the entire demo environment can be
regenerated, not because demo records or roles behave differently.

## Purpose

The live demo is a fully functional PLE installation, not a fixed walkthrough or read-only demonstration. The
demo uses the normal PLE application workflows, authorization, database, and storage.

The demo starts with seeded baseline data that gives people something useful to explore immediately. Once the
system starts, the seeded data is normal live data rather than special demo content.

## Seeded baseline

The initial data includes representative instructors, students, courses, assignments, problems, and student
activity. People may explore and modify this data and create additional data through the normal PLE workflows.

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
- Add students to courses.
- Preview current assignment policy, then exercise delivery and automated grading through the normal Student
  workflow.
- Manage and review student activity and grades.

Instructors do not add or approve other instructors. Instructor approval remains a Sysadmin function.

## Student perspective

The demo should allow a Student to use the normal student workflows, including:

- Enter courses in which the Student is enrolled.
- Complete assignments.
- Submit answers.
- View permitted feedback and grades.
- Repeat assignments where allowed.

Student activity in the demo is normal live PLE data.

## Sysadmin perspective

The demo should provide the full Sysadmin experience, including adding and approving instructors and the other
normal Sysadmin functions. The demo Sysadmin is a normal PLE Sysadmin with the same capabilities as a Sysadmin
in any other PLE installation. PLE continues to have only the normal Student, Instructor, and Sysadmin human
roles. USER_ROLES.md

The seeded Sysadmin account starts in an unclaimed state. First access requires completing the normal account
ownership setup, including passkey enrollment. This allows the live demo to exercise the real passkey workflow
immediately after a fresh deployment. Regenerating the demo restores the seeded Sysadmin account to its original
unclaimed state.

The integrity of the demo data does not need to be protected from changes made through normal Sysadmin
capabilities. The entire installation is a disposable demonstration environment. A Sysadmin may modify or delete
seeded or user-created data just as in a normal installation. Regenerating the demo database and storage
restores the seeded baseline.

## Demo authentication

Student and Instructor seeded accounts can be entered directly through the demo account selector.

Selecting a seeded account replaces only the normal passwordless identity-verification ceremony. The server
resolves the configured seeded account and creates the ordinary account session. Normal persisted course and
role selection then creates the ordinary PLE session.

The browser selects only a known demo persona. Account identity, roles, tenant context, course membership, and
authorization continue to be derived by the server from normal PLE state.

Conceptually:

    Select seeded Student or Instructor
                |
                v
    Server resolves seeded PLE account
                |
                v
       Ordinary account session
                |
                v
      Normal course/role selection
                |
                v
         Ordinary PLE session

After authentication, there is no separate demo application path.

## Sysadmin authentication and passkey setup

The seeded Sysadmin account exists in the baseline data, but first access requires completing the normal account
ownership setup, including passkey enrollment.

After setup, the account uses the normal Sysadmin authentication and session path. This allows a fresh live demo
to exercise the real passkey workflow immediately rather than bypassing it for the Sysadmin.

Conceptually:

    Fresh demo baseline
            |
            v
    Seeded Sysadmin account
            |
            v
    Account ownership setup
            |
            v
      Passkey enrollment
            |
            v
    Normal authentication
            |
            v
    Ordinary Sysadmin session

Regenerating the demo restores the Sysadmin account to its original seeded, unclaimed state.

## Demo lifecycle

The live demo uses one implementation. Courses, assignments, problems, accounts, memberships, student work,
grades, previews, and other application state use the normal PLE data model. Instructor validation of delivery
uses the same learner-run, submission, deterministic-grading, receipt, and gradebook paths used by a live course.

The distinction between a live demo and another PLE installation is primarily:

1. The installation begins with a known seeded baseline.
2. Student and Instructor demo personas provide a convenient entry into normal authentication and session handling.
3. The seeded Sysadmin provides a path for demonstrating normal account ownership and passkey enrollment.
4. The database and storage may be discarded and regenerated from the seeded baseline.

The live demo therefore remains the real PLE system. The initial state is simply seeded data for people to play with.
