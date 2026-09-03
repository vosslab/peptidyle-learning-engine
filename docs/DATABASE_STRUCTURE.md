# Database structure

This document maps the checked-in pre-production PostgreSQL baseline. The SQL
files under [schemas/migrations/](../schemas/migrations/) are the physical
schema authority. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) owns product
decisions; [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns product
meaning; [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md) owns database
authorization; and [implementation_status.md](active_plans/implementation_status.md)
owns migration allocation and package acceptance.

## Baseline and migration rules

The current disposable baseline contains the 45 checked migration files from
`2026082901_principal_baseline.sql` through
`2026090102_imathas_question_backend_session.sql`. The numbered range has no
`2026082905` or `2026082927` file; those numbers are not migrations. Apply the
complete checked-in sequence only to a clean disposable database. The prior
migration epoch was removed during the fresh pre-production migration reset; it is neither an
upgrade path nor a source of current table, role, or policy names.

Each accepted migration is immutable. Before acceptance, correct a wrong
baseline migration directly, update its owning package evidence, and rebuild a
clean database. The baseline has no compatibility migrations, backfills,
legacy readers, or parallel authorization model.

## Physical ownership map

| Migration range      | Physical owner                                                        | Principal records                                                                                                                                                                                                                                                                                                                   |
| -------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2901-2904, 2906      | Global Account and Authenticated Session                              | PostgreSQL roles, `account`, Account State Events, `authenticated_session`, Authentication Email, email challenges, WebAuthn ceremonies, passkeys, and Authenticated Session Resolution.                                                                                                                                            |
| 2907-2910            | Question Library and private authoring                                | Published Questions and immutable Question Revisions, publication and availability events, Question Change Proposals, Authoring Workspaces, Draft Questions and their Revisions, Question Source, Answer Key, Question Feedback, Question Answer Explanation, format-specific Question Grading Input, and private QTI import facts. |
| 2911-2915            | Reusable and live course roots                                        | Blueprint Courses and Revisions, Account-owned Question Folders and Saved Question Searches, Course Instances and Course Origin, Course Membership Events, Course Invitations, and Student Records.                                                                                                                                 |
| 2916-2918            | Assignment delivery and Student work                                  | Assignments and immutable Assignment Revisions, Assignment Attempts, Issued Questions, Question Attempts, Question Submissions, Assignment Submissions, and Student Feedback Release.                                                                                                                                               |
| 2919-2924            | Course objects, grading, analysis, and correction                     | Course Object References, Question Submission Grading, Grading Results, and Automated Grading Receipts; Assignment Grades and Events; Assignment and Assignment Question Analysis; Forced Question Corrections; Question Change Events; and correction evidence.                                                                    |
| 2925-2926, 2928-2931 | Jobs, retention schema foundation, objects, and authorization closure | Typed Jobs and leases, Course Retention Plan Revisions and Events, Object Deliveries, Object Storage Checks, Object Cleanup Manifests and Receipts, Authorization Checks, forced RLS policies, and final ACL closure. The schema does not mount retention execution.                                                                |
| 2932-2936            | Baseline witness and current root extensions                          | Baseline Acceptance Witness, Credential Authentication Completion (present in the baseline; its application route remains unmounted), Sysadmin Create Instructor Account, Blueprint publication/collaboration/availability events, and identity-free Question Revision Statistics.                                                  |
| 2937-2940            | Released Assignment snapshots and Object Record authority             | Independent Question Pool Reuse and Variation Rules, immutable released Assignment Entries and Question Pool Items, authenticated Assignment Attempt start, and immutable Object Records that bind Question Source object references.                                                                                               |
| 2941                 | Question Source object-only relationship                              | Every Question Source requires one immutable Source Object Reference and Source Object Checksum; inline source bytes are absent.                                                                                                                                                                                                    |
| 2942                 | Draft Question Source registration                                    | The session-authorized, idempotent registration procedure binds a pre-registered immutable source object to one Draft Question Revision.                                                                                                                                                                                            |
| 2943                 | Question credit and stewardship                                       | Immutable Question Revision acceptance, authorship, license, citation, and stewardship facts.                                                                                                                                                                                                                                       |
| 2944                 | Question Source publication transfer                                  | The protected transfer records the exact immutable Draft Question Source relationship for one published Question Revision.                                                                                                                                                                                                          |
| 2945                 | Question fork source                                                  | Immutable source-lineage relationships for Draft Questions and separately published Question lineages.                                                                                                                                                                                                                              |
| 2026090101           | Latest Question Revision summary                                      | The answer-free Question Summary projection derives one Latest Question Revision from immutable acceptance evidence; availability remains separate.                                                                                                                                                                                 |
| 2026090102           | iMathAS Question Backend Session                                      | Durable iMathAS Question Backend Session, render-cache, launch, challenge, result, grading, lease, and authorization records.                                                                                                                                                                                                       |

## Ownership boundaries

- `ple_private` contains Account credentials, sessions, invitations, draft-authoring facts,
  Student work, operational state, and private grading or provider records.
- `ple_data` contains shared Question, Blueprint Course, Course Instance, Membership,
  Student Record, Assignment, delivery, correction, and aggregate analysis facts.
- `ple_audit` contains immutable visible security, grading, retention, correction, and
  object-storage evidence.
- The browser never connects to PostgreSQL. Server code resolves an Authenticated Session,
  authorizes exact Account and durable-resource relationships, and invokes narrow Store or
  protected authorization functions.

## Current relational chains

```text
Account
  -> Authenticated Session
  -> Course Membership -> Student Record
  -> Course Instance -> Assignment -> Assignment Revision
  -> Assignment Attempt -> Issued Question -> Question Attempt -> Question Submission

Blueprint Course -> Blueprint Course Revision -> Course Instance -> Course Origin
Published Question -> Question Revision -> Question Revision Availability Event
Authoring Workspace -> Draft Question -> Draft Question Revision
```

An immutable Assignment Revision carries the exact resolved delivery schedule,
Assignment Attempt Time Limit, Attempt Limit, Late Work Rule, Assignment
Deadline Rule, and all eight independent Assignment Activity Rules. Its
completion threshold and continuation cap exist only for the rule variants
that require them. A later Instructor edit creates a new Assignment Revision;
an Assignment Attempt uses the delivery facts of its referenced revision
rather than a mutable current policy.

Question Folders organize Question Library lineages for an Account; they do
not grant visibility or Course authority. Course Invitations are target-bound
Course Membership operations. An Instructor Course Invitation is the
Instructor-only Teaching Team operation; pending account acceptance remains a
generic Course Invitation boundary.

## Object, grading, and retention boundaries

An Object Delivery authorizes retrieval of one exact Object Reference. An Object
Storage Check records a completed verified, missing, or mismatched observation.
Object Cleanup requires a separate manifest, Job, and immutable receipt.
An immutable Object Record is the database-authoritative existence record for
one typed Object Address, Object Storage Area, Object Data Class, checksum,
size, media type, and creation time. The session-authorized Workspace Question
Source registration capability accepts only the exact workspace-owned address
after object bytes are written. Every Question Source has one required Source
Object Reference and Source Object Checksum, with no inline source-data
alternative; the reference names that exact record and verifies its owner
address and checksum before use.
The Draft Question Source Store then mints the source identity only after it
resolves the current session, verifies the Draft Question Revision belongs to
the requested Authoring Workspace, validates the Question Backend/Question
Format pairing, and binds that exact pre-registered object. An identical retry
returns the established source; changed facts or an unauthorized workspace are
refused.
Object Data Class derives from the exact Object Address and owning relationship.
Reuse rights resolve through the owning Question Revision's Question License, or
through an exact Question Source or Question Asset License when it differs.

Accepted Student responses, automated grading, and Gradebook calculations
each retain their own immutable receipts or events. The current Course
Retention schema foundation records Plan Revisions, typed Jobs, retention
Events, Object Cleanup Manifests, and Object Cleanup Receipts; it does not
execute a retention lifecycle. No aggregate read result replaces the exact
record that proves it.

Each private Question Attempt stores its issued Question, unsigned Question
Seed, generated-parameter SHA-256, issued/deadline times, closed Question
Attempt State, and Question Attempt Reproduction Details. A Question
Submission separately owns the accepted Student Response and submission time;
the attempt state never invents a response when a deadline closes work.

## Verification

The disposable schema acceptance lane is the authoritative connected check for
migration order, ACL closure, RLS, and protected authorization-function behavior. Permanent tests prove
stable value and transport contracts; they do not substitute for a database
acceptance run. See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the
required evidence classes.
