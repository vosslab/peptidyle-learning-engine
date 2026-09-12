# Live Demo specification

## Purpose

The Live Demo is one complete, known-good teaching environment built with the
ordinary PLE product model. It gives a new installation useful data for
end-to-end demonstration and acceptance; it is not a second schema, a mock
application, or a special lifecycle of its own.

After canonical database structure initialization, the operator runs the
explicit installation-data provision command. That operation includes this
environment by default; `--without-live-demo` makes it a no-data opt-out. The
choice never changes the installed schema or application model. Once
provisioned, every record follows the same archive, retention, Unrelease, and
deletion rules as corresponding product data. There is no demo-specific marker,
role, teardown capability, or report artifact.

Local disposable stacks use the same installation-data path. Resetting one of
those stacks replaces its storage; it is a development convenience, not a
separate product-data contract.

## Installation boundary

The canonical base schema is DDL-only and creates no product data. The separate,
explicit installation-data provision command includes the Live Demo by default. It
first uses the ordinary Pilot Question publisher to create the required
published Question Revisions and their object bindings, then applies the
database-owned Live Demo graph. The publisher returns exact Question Revision
references to the SQL manifest; the manifest validates that mapping before it
creates dependent records. `--without-live-demo` leaves all installation data
unprovisioned.

Question identifiers are ordinary opaque identifiers. Their canonical stored
form is the compact seven-character Crockford Base32 value; APIs display the
same value as `AAA-BBBB`. Pilot source slugs and source checksums select the
reviewed content internally, but are neither public Question IDs nor a special
Question namespace.

The idempotent SQL installation-data layer owns the facts for which PostgreSQL
owns the complete invariant. It creates the same ordinary data graph on a
fresh installation:

- fixed fictional Accounts and their ordinary roles;
- subject organization;
- a Blueprint Course, its Draft, published Blueprint Revision, and exact
  Question Revision pins;
- a Course Instance, Instructor relationship, invitations, Student records,
  and Course Memberships; and
- the released Assignment and its exact Question Revision pins.

The manifest is intentionally not a substitute for systems that own effects
outside PostgreSQL. The established owner paths create Attempts, retained
presentation bindings, responses, submissions, grading, and related
statistics when a demonstration needs them. This preserves the same object
storage, renderer, worker, and grading behavior used by product workflows.

## Known-good teaching graph

The reusable Blueprint Course is **Biochemistry 301: Proteins and Peptides**.
Elena Rivera owns its ordinary published Blueprint Revision. The resulting
Course Instance is `BCHM 301`, also named **Biochemistry 301: Proteins and
Peptides**, runs from 2026-08-24 through 2026-12-11 in `America/Chicago`, and
has Elena as its Assigned Instructor.

Its released Assignment, **Chapter 1 Pilot Practice**, uses the eight reviewed
Pilot Questions selected by
[`content/pilot/chapter_1_assignments.yaml`](../content/pilot/chapter_1_assignments.yaml).
Each entry pins the exact published Question Revision supplied by the ordinary
publisher. The Assignment instructions are: "Complete the eight reviewed
Chapter 1 practice questions."

The database-owned graph includes the three ordinary Student records and
Course Memberships below. The cross-system activity owner may then establish
the demonstration work states through normal delivery and grading paths.

| Student | Roster ID | Demonstration state |
| --- | --- | --- |
| Mary Okafor | `BIO301-MARY` | Completed and graded work |
| Jack Nguyen | `BIO301-JACK` | Open work with saved responses |
| Avery Thompson | `BIO301-AVERY` | Released Assignment available to start |

These are ordinary relationships and Student Work records. Instructor views,
Student views, and Gradebook results derive from them under the normal
authorization and evidence rules.

## Local identity selector

The disposable local HTTPS entry currently offers a fixed five-persona
identity selector: Elena Rivera (Instructor); Mary Okafor, Jack Nguyen, and
Avery Thompson (Students); and Morgan Delgado (Sysadmin). It replaces only
identity verification, then the server resolves the configured Account and
issues an ordinary authenticated session. Stored roles and relationships still
decide every authorization result.

The selector remains available in the default local browser Live Demo after
provisioning, so its ordinary teaching journeys remain usable on restart.
`--without-live-demo`, non-browser profiles, and public deployments use normal
authentication instead. It grants no Course Membership, Student record,
academic authority, or object access by itself. Morgan's Sysadmin role likewise grants no ambient
academic access.

## Product boundaries

Students receive Questions only through authorized Assignment access for their
exact Course, Assignment, and Student record. Retained Attempt and Issued
Question evidence keeps old work interpretable after later Assignment edits.
Submission, grading, feedback, asset access, and Gradebook projections retain
their own authorization and disclosure boundaries; a terminal grade does not
by itself disclose answers or feedback.

The Instructor can use the ordinary Question Library, Blueprint Course, Course
Instance, roster, Assignment, release, Gradebook, and invitation-export
workflows within stored authority. Invitation export does not send mail;
`launchers/send_invitations.py` remains the attended local mail action.

The current local browser surfaces use the `laptop` profile (1280 by 800 CSS
pixels) for the principal Instructor, Student, and Sysadmin workflows.
`tablet`, `phone`, and `square` captures cover materially different responsive
or access presentation. The selected surfaces are listed in
[`docs/screenshots/current_capture_manifest.json`](screenshots/current_capture_manifest.json).
