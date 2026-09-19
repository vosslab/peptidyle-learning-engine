# Live Demo specification

## Purpose

The Live Demo is one complete, known-good teaching environment built with the
ordinary PLE product model. It gives a new installation useful data for
end-to-end demonstration and acceptance; it is not a second schema, a mock
application, or a special lifecycle of its own.

After the Bloom publication cutover and canonical database initialization, the
operator will run the installation-data provision command. That operation will
include this environment by default; `--without-live-demo` will omit the
fictional teaching graph but retain ordinary shipped Genetics content. The
choice will not change the installed schema or application model. Once
provisioned, every record will follow the same archive, retention, Unrelease,
and deletion rules as corresponding product data. There will be no
demo-specific marker, role, teardown capability, or report artifact.

Local disposable stacks will use the same installation-data path. Resetting one
of those stacks replaces its storage; it is a development convenience, not a
separate product-data contract.

## Installation boundary

The canonical base schema is DDL-only and creates no product data. After the
Bloom publication cutover, the separate installation-data provision command
will include the Live Demo by default. It will first use the ordinary Pilot
Question publisher to create the required published Question Revisions and
their object bindings, then apply the database-owned Live Demo graph. The
publisher will return exact Question Revision references to the SQL manifest;
the manifest will validate that mapping before it creates dependent records.
`--without-live-demo` will leave the fictional Live Demo graph unprovisioned
while retaining the shipped Genetics content.

Automated initial Bloom Classification is deferred, and the PLE runtime does
not configure an AI provider. The current SQL and data-access publication
contract still requires a prepared Bloom receipt, so fresh Question publication
for this graph remains unavailable until that separate cutover is complete.
This limitation does not add another Live Demo topology or gateway path.

Question identifiers are ordinary opaque public IDs. Published Questions and
Question Pools use the canonical `XXXX-ZXXX` form unchanged in storage, APIs,
URLs, and the browser. Pilot source slugs and source checksums select the
reviewed content internally, but are neither public Question IDs nor a special
Question namespace.

The idempotent SQL installation-data layer owns the facts for which PostgreSQL
owns the complete invariant. It creates the same ordinary data graph on a
fresh installation:

- fictional Accounts and their ordinary roles, with server-minted public IDs;
- subject organization;
- a Blueprint Course, its save-created Blueprint Revision, and exact
  Question Revision pins;
- a Course Instance, Instructor relationship, invitations, Student records,
  and Course Memberships; and
- the released Assessment and its exact Question Revision pins.

The manifest is intentionally not a substitute for systems that own effects
outside PostgreSQL. The established owner paths create Attempts, retained
presentation bindings, responses, submissions, grading, and related
statistics when a demonstration needs them. This preserves the same object
storage, renderer, worker, and grading behavior used by product workflows.

## Known-good teaching graph

The reusable Blueprint Course is **Biochemistry 301: Proteins and Peptides**.
Elena Rivera owns its ordinary save-created Blueprint Revision. The resulting
Course Instance is `BCHM 301`, also named **Biochemistry 301: Proteins and
Peptides**, runs from 2026-08-24 through 2026-12-11 in `America/Chicago`, and
has Elena as its first ordinary co-Instructor, with no greater authority than a
later co-Instructor.

The ordinary publisher makes all eight reviewed Pilot Questions available in
the Question Library, including the four WeBWorK Questions described in
[`content/pilot/chapter_1_assessments.yaml`](../content/pilot/chapter_1_assessments.yaml).
Its released Assessment, **Chapter 1 Pilot Practice**, has Type **Practice
Question Assignment** and uses the four
PLE-native PLE Question JSON Pilot Questions. Each entry pins the exact
Published Question Revision. The Assessment instructions are: "Complete the
four reviewed Chapter 1 practice questions."

The database-owned graph includes the three ordinary Student records and
Course Memberships below. The cross-system activity owner may then establish
the demonstration work states through normal delivery and grading paths.

| Student        | Roster ID      | Demonstration state                    |
| -------------- | -------------- | -------------------------------------- |
| Mary Okafor    | `BIO301-MARY`  | Completed and graded work              |
| Jack Nguyen    | `BIO301-JACK`  | Open work with saved responses         |
| Avery Thompson | `BIO301-AVERY` | Released Assessment available to start |

These are ordinary relationships and Student Work records. Instructor views,
Student views, and Gradebook results derive from them under the normal
authorization and evidence rules.

## Local identity selector

The disposable local HTTPS entry currently offers a fixed six-persona
identity selector: Elena Rivera and Priya Shah (Instructors); Mary Okafor, Jack Nguyen, and
Avery Thompson (Students); and Morgan Delgado (Sysadmin). It replaces only
identity verification, then the server resolves the configured Account and
issues the ordinary authenticated session for a Student or Instructor. Morgan
selection creates only opaque pending MFA. PostgreSQL derives Morgan's stored
Sysadmin role and issues a session only after it atomically consumes one
unused, short-lived, Account- and browser-bound TOTP attestation. Stored roles
and relationships still decide every authorization result.

Priya has her own ordinary vetted Instructor Account and private authoring
workspace, initially without Course Membership or Blueprint access. She can
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
