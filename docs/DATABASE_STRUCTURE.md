# Database structure

This document maps the checked-in pre-production PostgreSQL baseline. The SQL
files under [schemas/migrations/](../schemas/migrations/) are the physical
schema authority. [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) owns product
decisions; [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns product
meaning; [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md) owns database
authorization; and [implementation_status.md](active_plans/implementation_status.md)
owns migration allocation and package acceptance.

## Baseline and migration rules

The current disposable baseline contains the 48 checked migration files from
`2026082901_principal_baseline.sql` through
`2026090304_qpv1_question_publication_validation.sql`. The numbered range has no
`2026082905` or `2026082927` file; those numbers are not migrations. Apply the
complete checked-in sequence only to a clean disposable database. The prior
migration epoch was removed during the fresh pre-production migration reset; it is neither an
upgrade path nor a source of current table, role, or policy names.

Each accepted migration is immutable. Before acceptance, correct a wrong
baseline migration directly, update its owning package evidence, and rebuild a
clean database. The baseline has no compatibility migrations, backfills,
legacy readers, or parallel authorization model.

## Physical ownership map

| Migration range      | Physical owner                                                          | Principal records                                                                                                                                                                                                                                                                                                                          |
| -------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 2901-2904, 2906      | Global Account and Authenticated Session                                | PostgreSQL roles, `account`, Account State Events, `authenticated_session`, Authentication Email, email challenges, WebAuthn ceremonies, passkeys, and Authenticated Session Resolution.                                                                                                                                                   |
| 2907-2910            | Question Library and private authoring                                  | Published Questions and immutable Question Revisions, publication and availability events, Question Change Proposals, Authoring Workspaces, mutable Draft Questions, qualified Question Source Bindings, Question Authorship, Question License, Question Citation, Question Ownership, Workspace Imports, and private QTI import evidence. |
| 2911-2915            | Reusable and live course roots                                          | Blueprint Courses and Revisions, Account-owned Question Folders and Saved Question Searches, Course Instances and Course Origin, Course Membership Events, Course Invitations, and Student Records.                                                                                                                                        |
| 2916-2918            | Assignment delivery and Student work                                    | Assignments and immutable Assignment Revisions, Assignment Attempts, Issued Questions, Question Attempts, Question Submissions, Assignment Submissions, and Student Feedback Release.                                                                                                                                                      |
| 2919-2924            | Course objects, grading, analysis, and correction                       | Course Object References, Question Submission Grading, Grading Results, and Automated Grading Receipts; Assignment Grades and Events; Assignment and Assignment Question Analysis; Forced Question Corrections; Question Change Events; and correction evidence.                                                                           |
| 2925-2926, 2928-2931 | Jobs, retention schema foundation, objects, and authorization closure   | Typed Jobs and leases, Course Retention Plan Revisions and Events, Object Deliveries, Object Storage Checks, Object Cleanup Manifests and Receipts, Authorization Checks, forced RLS policies, and final ACL closure. No retention-execution Service is implemented.                                                                       |
| 2932-2936            | Baseline witness and current root extensions                            | Baseline Acceptance Witness, Credential Authentication Completion (present in the baseline; its application Server Route does not exist), Sysadmin Create Instructor Account, Blueprint publication/collaboration/availability events, and identity-free Question Revision Statistics.                                                     |
| 2937-2940            | Released Assignment snapshots and Object Record/source-object authority | Independent Question Pool Reuse and Variation Rules, immutable released Assignment Entries and Question Pool Items, authenticated Assignment Attempt start, and immutable Object Records that directly bind Question Source Object References and Source Object Checksums in the fresh baseline.                                           |
| 2942                 | Bind Question Source                                                    | The session-authorized operation binds an existing immutable source object to one Draft Question at its exact Edit Number; matching facts preserve that Binding.                                                                                                                                                                           |
| 2943                 | Question credit and stewardship                                         | Immutable Question Revision acceptance, authorship, license, citation, and stewardship facts.                                                                                                                                                                                                                                              |
| 2944                 | Question Source Binding publication boundary                            | The publication-event completeness predicate requires a Question Revision-owned Source Binding.                                                                                                                                                                                                                                            |
| 2945                 | Question fork source                                                    | Immutable source-lineage relationships for Draft Questions and separately published Question lineages.                                                                                                                                                                                                                                     |
| 2026090101           | Latest Question Revision summary                                        | The answer-free Question Summary projection derives one Latest Question Revision from immutable acceptance evidence; availability remains separate.                                                                                                                                                                                        |
| 2026090102           | iMathAS Question Backend Session                                        | Durable iMathAS Question Backend Session, render-cache, launch, challenge, result, grading, lease, and authorization records.                                                                                                                                                                                                              |
| 2026090301           | Draft and Published Question Metadata ownership                         | Separate Draft Question Metadata and Published Question Metadata tables and their exact reader ownership.                                                                                                                                                                                                                                  |
| 2026090302           | New-lineage Question Publication persistence                            | Trusted server-only transaction creates one complete immutable first Question Revision aggregate from an exact current Draft Question and verified bytes-first target Object Record.                                                                                                                                                       |
| 2026090303           | Draft Question publication source resolution                            | Session-authorized server-only read resolves the exact current Draft Question Source Object Record for verified immutable publication copying.                                                                                                                                                                                             |
| 2026090304           | Question Publication Validation evidence                                | Question Change Proposal Revision stores its exact calculated `question_publication_validation`; the bare predecessor column and constraint names are absent from the current schema.                                                                                                                                                      |

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

The required target uses parallel `ple_private.draft_question_metadata` and
`ple_data.published_question_metadata` tables for corresponding bounded discovery fields, with
shared field validation and separate owner keys, mutability, RLS, indexes, and retention. It
separates Draft Question rows, editable metadata, and Draft Question Source Bindings from
Question Revision Source Bindings and Published Question metadata. Publication copies the
validated draft metadata values into the stable Published Question-owned table and writes
a new immutable Question Revision-owned source object and its own Source Object Reference and Source
Object Checksum. Published tables contain no Draft Question foreign key or draft object path, so
expiration of sandbox drafts adds no joins, tombstones, or retained draft metadata to Question
Library reads. Question Title, Question Description, and other metadata explicitly owned by the
stable Published Question remain mutable without changing immutable Question Revision source.
M1 removes the former inline Draft Question Title and Question Revision discovery-metadata shapes.
The fresh private-authoring baseline directly creates two qualified Source Binding relationships;
M1 adds the parallel metadata tables and their exact reader ownership. P1 adds the new-lineage
publication Store and database transaction after trusted bytes-first storage. P2 adds the
session-authorized exact draft-source resolver used by the server-only immutable object-copy
coordinator. Same-lineage publication, cleanup, Question Search, Server Routes, and Browser
Surfaces remain parent QSOM1 work.

## Current relational chains

```text
Account
  -> Authenticated Session
  -> Course Membership -> Student Record
  -> Course Instance -> Assignment -> Assignment Revision
  -> Assignment Attempt -> Issued Question -> Question Attempt -> Question Submission

Blueprint Course -> (Blueprint Course Reference, Blueprint Revision Number)
  -> Blueprint Revision -> Course Instance -> Course Origin
Published Question -> Question Revision -> Question Revision Availability Event
Authoring Workspace -> Draft Question
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
Bind Question Source capability accepts only the exact workspace-owned address
after object bytes are written. Every Question Source has one required Source
Object Reference and Source Object Checksum, with no inline source-data
alternative; the reference names that exact record and verifies its owner
address and checksum before use.
The Draft Question Source Binding Store has no binding identity to mint. It resolves the
current session, verifies that the Draft Question belongs to the requested Authoring Workspace,
locks its exact positive Edit Number, validates the Question Backend/Question Format pairing, and
binds that exact pre-registered object. An identical retry is a no-op; changed facts, a stale Edit
Number, or an unauthorized workspace are refused.
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
