# Live Demo specification

## Purpose

The Live Demo is ordinary PLE teaching data. A fresh installation imports complete
Genetics, Biochemistry, Biotechnology, and Biostatistics collections through the
shared Python importer, then includes the fictional teaching graph by default.
`local_stack.py install-data --without-live-demo` retains imported BPW curriculum
while omitting that graph on a fresh installation.

## Installation boundary

The canonical base schema is DDL-only. The host owns BPW source discovery, Python,
and QPM CLI conversion. PG and PGML source reaches WeBWorK directly and unchanged;
BBQ source uses QPM Native JSON. Publication, Question Pools, all four public
Blueprint Courses, Course adoption, Assessment release, and Student Work use
ordinary Instructor and Student HTTP workflows.

The trusted installer provisions fictional Accounts and enrollment separately.
It supplies the loader an ordinary Instructor cookie through private stdin, using
the existing eight-hour lifetime, and revokes that session after completion or
failure. Content-only installation uses a non-login PLE Example Content Instructor
without creating the fictional Accounts. The migrator container runs Rust only;
it does not own host Python, BPW, or QPM.

## Known-good teaching graph

Elena Martinez adopts the complete public Biochemistry Blueprint as **BCHM 355**,
also named **Biochemistry**, from 2026-08-24 through 2026-12-11. All imported
Modules and Assessments follow ordinary adoption. One ordinary native Assessment
is released for representative delivery; other imported Assessments begin
Unreleased and remain editable by the teaching team.

The activity installer derives sample responses from answer-free native
presentations, submits Mary's work, saves Jack's open work, and leaves Avery's
work unstarted. Entry counts come from ordinary content; no fixed Pilot Question
count is required.

| Student | Roster ID | Demonstration state |
| --- | --- | --- |
| Mary Okafor | `BCHM355-MARY` | Completed work |
| Jack Nguyen | `BCHM355-JACK` | Open work with saved responses |
| Avery Thompson | `BCHM355-AVERY` | Released Assessment available to start |

## Local identity selector

The disposable local HTTPS entry currently offers a fixed six-persona
identity selector: Elena Martinez and Priya Shah (Instructors); Mary Okafor, Jack Nguyen, and
Avery Thompson (Students); and Morgan Delgado (Sysadmin). It replaces only
identity verification, then the server resolves the configured Account and
issues the ordinary authenticated session for a Student or Instructor. Morgan
selection creates only opaque pending MFA. PostgreSQL derives Morgan's stored
Sysadmin role and issues a session only after it atomically consumes one
unused, short-lived, Account- and browser-bound TOTP attestation. Stored roles
and relationships still decide every authorization result.

Priya has her own ordinary active Instructor Account and personal authoring
workspace, initially without Course Membership. She can
participate in multi-Instructor Blueprint collaboration through the ordinary
sharing and Proposal workflows; selecting her identity grants no collaboration
authority itself. This fixed fictional identity is disposable demonstration
data, not production Account provisioning or arbitrary-account impersonation.

The local controller provisions Morgan's genuine TOTP seed from the
operating-system CSPRNG and writes a restricted, ignored, mode-0600 operator
artifact, logging only the artifact path. A separate local authenticator
consumes the artifact. It is not a browser credential or a fixed shared secret.
The selector has no TOTP bypass, and PLE has no recovery or self-service flow
for Morgan.

The selector remains available in the default local browser Live Demo after
provisioning, so its ordinary teaching journeys remain usable on restart.
`--without-live-demo`, non-browser profiles, and public deployments use normal
authentication instead. It grants no Course Membership, Student record,
academic authority, or object access by itself. Morgan's Sysadmin role likewise grants no ambient
academic access.

## Product boundaries

Students receive Questions only through authorized Assessment access for their
exact Course, Assessment, and Student record. Retained Attempt evidence keeps
old work interpretable after later Assessment edits.
Submission, grading, feedback, asset access, and Gradebook projections retain
their own authorization and disclosure boundaries; a terminal grade does not
by itself disclose answers or feedback.

The Instructor can use the ordinary Question Library, Blueprint Course, Course
Instance, roster, Assessment, release, Gradebook, and invitation-export
workflows within stored authority. Invitation export does not send mail;
`launchers/send_invitations.py` remains the attended local mail action.

The current local browser surfaces use the `laptop` profile (1280 by 800 CSS
pixels) for the principal Instructor, Student, and Sysadmin workflows.
`tablet`, `phone`, and `square` captures cover materially different responsive
or access presentation. The selected surfaces are listed in
[`docs/screenshots/current_capture_manifest.json`](screenshots/current_capture_manifest.json).
