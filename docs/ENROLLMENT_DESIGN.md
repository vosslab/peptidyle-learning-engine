# Enrollment design

## Binding single-installation model

PLE uses global Accounts and exact Course Membership. One course may have multiple equal
Teaching Team Members, and every Student Record belongs to one exact Student Account and Course Instance.
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md) owns the
durable schema and authorization contract.

This document defines the durable identity, authorization, Store, HTTP, privacy,
and user-experience contract for that boundary. [ROADMAP.md](ROADMAP.md) and
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) own release direction and acceptance gates.

The primary audience is a contributor implementing course membership, Student
Records, Assignment delivery, roster management, identity resolution, or the
instructor and student enrollment journeys. Unfinished release work is routed through
[TODO.md](TODO.md).

## Status and authority

This document is the durable enrollment contract. Current route truth remains in
[API_CONTRACTS.md](API_CONTRACTS.md). Accepted implementation evidence for those routes remains in
[CHANGELOG.md](CHANGELOG.md).

## Product decision

The future PLE product will present **one course-level roster workflow** to
instructors. Course
Enrollment establishes a Student Course Membership and its Student Record;
Assignments derive activity authorization from that exact relationship and the
Assignment Status, effective Assignment policy, and direct Student Record access facts.

In that future workflow, an instructor invites a student to the course once.
After that student authenticates and claims the invitation, the same
Store-owned transaction will:

1. will resolve the authenticated PLE `AccountId`;
2. will create or reuse that Student Account's `StudentRecordId` for the exact Course Instance;
3. will create a fresh active `course_membership` episode bound to that Student Record; and
4. will store course-local display/contact evidence in the subordinate roster
   profile.

When an instructor later creates an assignment, PLE stores the assignment for
the Course Instance without eagerly creating Student Work Records. The sole Assignment Access
evaluator derives current access from the exact Student Record, Active Student
Course Membership, Assignment Status, Effective Assignment Policy, and direct Student accommodation facts. The
planned first Assignment Attempt start, grade-bearing action, or explicit
Instructor issue will create the assignment receipt, typed empty Assignment Attempt
Summary, direct access basis, and immutable Assignment Access evidence. Current
source has no durable Store transaction that creates that planned
enrollment-plus-empty-summary receipt, so this remains a design requirement
rather than an implemented creation boundary.

This gives instructors the simple course-enrollment model used successfully by
LibreTexts ADAPT without weakening PLE's more precise Student Work Records model:

```text
Instructor action                 Durable PLE records

Add Student to course      ->     course_membership
                                  Student Record
                                  course roster profile

Create later assignment    ->     assignment
                                  no eager Student Work Record

Assignment Access-authorized ->   Assignment Attempt and Issued Question
event                             selected Assignment Grade state
                                  sealed Assignment Access evidence
```

The normal UI does not ask an instructor to add the same student separately to
every assignment. Active Student Course Membership is the prerequisite for the
per-Student, per-Assignment Assignment Access decision; absence of the planned
Assignment Attempt Summary receipt means only that no access-bearing event has
occurred. It is never interpreted as current denial or current grant.

## Why records remain separate

The product workflow is course-level, but the records answer different
questions:

| Record             | Question it answers                                                                     | Lifetime                                 |
| ------------------ | --------------------------------------------------------------------------------------- | ---------------------------------------- |
| Course membership  | May this authenticated person enter or manage this course?                              | Current access relationship              |
| Student Record     | Which protected educational record belongs to this Student Account and Course Instance? | Stable across course-membership episodes |
| Assignment Attempt | What immutable Student activity exists for this Student Record and Assignment?          | Educational record                       |
| Assignment Grade   | What selected course result should the Gradebook read?                                  | Updated from retained Student activity   |

Removing course access therefore does not erase Student Records, Assignment Attempts, Question Attempts,
submissions, or grades. Roster removal revokes future course access. Record
archive and deletion continue through the explicit retention workflow in
[RETENTION_POLICY.md](RETENTION_POLICY.md).

Student-scoped Store operations re-evaluate active `Student` membership and the
exact Student Record's Assignment Access, then bind the result's stable
`StudentRecordId` to any retained receipt at the database/Store boundary. Thus a
revoked student cannot continue to read an Assignment Attempt, Question Attempt, summary, feedback
release, or prefetch that was issued before removal. Direct course instructors
use distinct Instructor-history operations for records retained for grade, audit, and
retention work; membership removal does not accidentally erase that explicit
Instructor authority. Sysadmin status grants no general access to those
records; its closed, audited roster-support capability is the explicit
support exception.

The activity hierarchy remains the one in
[ACTIVITY_MODEL.md](ACTIVITY_MODEL.md): a Student Record and Assignment own
repeated Assignment Attempts; an Assignment Attempt owns ordered Issued
Questions; an Issued Question owns its Question Attempts. Course membership
never becomes an answer, score, completion flag, or attempt authority.

## Gap closed by the current slice

The original missing seam was visible at three boundaries:

- The earlier course-route surface provided course reads, course creation,
  gradebook reads, and assignment creation/update without roster mutation.
- The deferred course-delivery route surface will expose Student-owned
  Assignment Attempt history. Course Enrollment occurs through course-level
  invitation claim rather than a public assignment-enrollment mutation.
- The deferred course-creation command creates a Course Instance with its
  authenticated Instructor membership; roster commands then establish Student
  memberships and Student Records.

The future Store boundary will remain the authority beneath HTTP. Its roster
commands will create or update membership and its course-local profile.
Assignment Access will derive from that membership and the Assignment's direct
policy. The exact Student-work operations will create their own records:
Assignment Attempt creation records the authorized start, Issued Question
creation records delivery, Question Submission acceptance records the Student
response, and Automated Grading records immutable grading evidence.
`AssignmentGrade` will own the selected course-record result;
`AssignmentProgressRecord` will remain the separate derived activity result.

The existing whole-course `upsert_course` operation is not a suitable future
roster mutation: it replaces the complete member list and has no
browser-facing revision. A route-level read-modify-write through that method
could lose a concurrent roster edit. The planned implementation instead uses
focused atomic member, invitation, policy, import, and roster commands with a
strong roster revision.

## Identity prerequisite

PLE owns its Accounts. An `AccountId` is the stable opaque identity of one PLE
account across courses and institutions; it is not issued by an instructor,
course, university, or email provider. Course membership and account-and-relationship-scoped forced
RLS control access to educational records.

The following is the future enrollment authentication design, not a description
of a current route surface. It will own the account-session boundary and mint an
`__Host-ple_session` only after an authorized course relationship is chosen or
claimed. Today, the deployment-gated seeded persona selector is the only
available browser entry; it uses the same Account/session records for connected
local evidence. The future product direction is:

- email authentication is the registration and sign-in path;
- passkeys are optional convenience credentials for the same account;
- the existing opaque, hashed server-side session and host-only HttpOnly
  `__Host-` cookie remain the browser credential; and
- optional institutional SSO may link a verified external identity to an
  existing PLE account, but it does not own `AccountId`, select a course, or block
  institution-independent deployment.

When the deferred passkey capability is implemented and accepted, PLE will use
an established WebAuthn implementation rather than implement the protocol. The
future design supports discoverable credentials for usernameless login,
multiple passkeys per Account, normal authenticator biometric or PIN user
verification, and passkey enrollment during registration. Attestation is not
required without a future managed-device use case. Authenticator user
verification proves access to the Account; it is not proctoring or proof of a
student's legal identity. The current schema and typed ceremony roots are
foundations only: there is no active passkey configuration, route, Browser
Surface, setup credential, ceremony/store implementation, or session issuance.

The minimum identity contract distinguishes role-qualified authentication email:

| Value                           | Owner                        | Rule                                                                                                                                |
| ------------------------------- | ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `AccountId`                     | PLE identity system          | Stable opaque PLE Account identity across Course Instances                                                                          |
| Student Authentication Email    | PLE identity system          | Immutable normalized institutional email for a Student Account; never the primary key                                               |
| Instructor Authentication Email | PLE identity system          | Private normalized and delivery values for an Instructor Account; a later verified replacement design must preserve that Account's relationships |
| Passkey credentials             | PLE identity system          | Deferred future convenience-credential design; no active credential exists or may be used today                                      |
| Display name or handle          | User account profile         | User-controlled safe label; no legal-name requirement                                                                               |
| `StudentRecordId`               | PLE educational-record store | Stable protected educational record for one Student Account inside one exact Course Instance                                        |
| Optional SSO binding            | PLE identity system          | Verified external issuer/subject linked to an existing `AccountId`; server-only and never roster authority                          |

The Account-to-Student Record mapping remains course-scoped because `StudentRecordId` belongs
to the educational-record and retention boundary, not because the PLE account
belongs to a separate installation:

```text
account_id                      -> one PLE account
(course_id, account_id)         -> student_record_id
(course_id, student_record_id)  -> account_id
```

Both course mapping directions are unique. The same account can therefore
participate in multiple Course Instances while each Course Instance retains an
independently scoped pedagogical record. The current
`AuthenticatedSession` carries the authenticated account and session; it is used with
the exact course relationship and is resolved by the server
or course fields.

### Account and local browser session boundary

The local browser and deployed product use the same PLE-owned account contract:

| Session              | Issuer and purpose                                            | What it establishes                                                                            |
| -------------------- | ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `__Host-ple_session` | Deployment-gated seeded-persona entry today; future email or passkey ceremony only when accepted | One Authenticated Session used with exact Course, Assignment Attempt, and roster relationships |

Future invitation redemption will use an authenticated Account session before
exact course relationship resolution. In that future design, passkey
registration begins from an authenticated PLE Account, so a passkey can shorten
later sign-in but cannot bootstrap the first Account by itself. The seeded
selector is disabled when its deployment settings are absent. Any future email
start must fail closed unless both the invitation-token secret and a complete
external SMTP configuration are present; a Server Route's existence is not
evidence of a live email-authentication ceremony. No such email ceremony or
route is active today.

ENR6 therefore specifies email authentication to restore an existing PLE
Account before future invitation redemption. Future authentication ceremonies
must authenticate existing Accounts; they must not create Student Accounts.
Copy-link delivery would remove SMTP from the invitation handoff, but would not
replace Account authentication. The local browser currently exercises real
Account and account-session records through seeded entry; that selector is a
deployment convenience for connected evidence, not a parallel identity or
invitation path.

### Person, course, and email

The account belongs to the student:

```text
Illustrative future Student Account
  AccountId 42
  Student Authentication Email: verified and immutable
  passkeys: laptop, phone
  Biochemistry Student Record: StudentRecordId 91
    course membership: Biochemistry, student
  Genetics Student Record: StudentRecordId 37
    course membership: Genetics, student
```

The two instructors see only records in courses they are authorized to manage.
Neither Instructor receives the student's global `AccountId`, passkey metadata, or
activity from another course. Course authorization, student ownership, query
scope, and RLS remain the disclosure controls.

PLE does not create a different account for each course enrollment. Per-course
account fragmentation would complicate record inspection, correction,
retention, and deletion without improving course authorization.
The course-owned Student Record and the Assignment directly identify each
Student's activity. An Assignment Attempt preserves one pass without
pretending one human is several people.

Authentication Email is role-qualified private personally identifiable
information. A Student Authentication Email is the immutable normalized
institutional email for a Student Account. An Instructor Authentication Email
can be replaced only through a future verified replacement operation; that
operation preserves the persistent Instructor Account and its relationships.
`AccountId`, rather than either email, remains the stable Account identity.
Neither Authentication Email is a database primary key, course selector, or
course authority. A different institutional email identifies a different
Student Account.

### Data minimization and roster metadata

PLE retains enough identifying information to make enrollment and grade export
practical, but only while it has a concrete teaching, authentication, audit,
or export purpose. In short: **collect reluctantly, use
deliberately, purge predictably**.

An active course roster may therefore contain:

- the course roster email supplied by the instructor for invitation, roster
  management, permitted-domain policy, and calculated roster score CSV export;
- the institution-issued student number supplied by the instructor for
  reliable LMS/gradebook row matching; and
- a student-selected display name or handle that helps the instructor manage
  the class.

These are protected course operational metadata. They do not establish
the PLE Account identity. `AccountId` remains the opaque Account identifier, while
the roster email and student number must not become credentials, primary keys,
or cross-course search fields. The course roster email is a retained snapshot
of the instructor's mapping; it neither replaces nor mutates the Student
Authentication Email. A different institutional email identifies a different
Student Account.

Course roster metadata follows [RETENTION_POLICY.md](RETENTION_POLICY.md): the
default 30-day notice leaves it available for corrections and final export,
the 100-day archive removes it from ordinary student access, and the 365-day
delete removes the student graph unless an authorized extension or earlier
archive/delete action applies. The 100-day threshold is an archive, not a
claim of permanent deletion.

This is also the appropriate FERPA-oriented engineering posture when PLE is
used for an institutional course. FERPA focuses on controlled disclosure to a
school official with a legitimate educational interest, not on creating one
login per class. PLE enforces that boundary through course authorization and
retrievable per-enrollment records. See the
[Department of Education access-control guidance](https://studentprivacy.ed.gov/faq/what-must-educational-agencies-or-institutions-do-ensure-only-school-officials-legitimate),
[school-official guidance](https://studentprivacy.ed.gov/faq/who-school-official-under-ferpa),
and [FERPA inspection rule](https://studentprivacy.ed.gov/ferpa?exp=8).

PLE minimizes the records needed to provide its service, but this document is
an engineering design rather than institution-specific legal advice. An
adopting institution remains responsible for its definition of legitimate
educational interest, vendor terms, annual notice, and required enrollment
procedure.

### Invite-by-email account binding

In the future enrollment design, invitation at a verified email address is the
normal enrollment path. The instructor may copy the returned one-time link into
an existing trusted LMS or let a configured SMTP provider deliver the same
link:

```text
instructor enters email and roster ID
    -> PLE creates a pending invitation
    -> PLE returns one copyable invitation link in the no-store create response
    -> instructor shares it through an LMS, or configured SMTP sends it
    -> student completes future short-lived, single-use email authentication
    -> PLE resolves the student's created opaque AccountId
    -> student claims the invitation
    -> PLE creates course membership and its roster profile atomically
    -> student may later enroll one or more passkeys if the deferred capability is accepted
```

An existing PLE Account and a newly created Account will follow the same
outward flow. The future server must match the verified email to its existing
Account; it must never create an Account during authentication. The Instructor
must not query whether an address already has an Account, and existing and
nonexistent addresses must receive the same outward invitation result.

The future invitation link is a bearer secret. PLE must return it only in the
Instructor's no-store creation response, keep it in browser memory for that
page session, and never place it in roster reads, storage, logs, or analytics.
The server must store only its hash. The instructor must share it only with the
intended student and revoke the pending invitation if it reaches the wrong
person. The link proves possession of the invitation, not control of the roster
email, so it must never replace the student's email-authentication ceremony.

Future email-authentication tokens must be short-lived and single-use. They
must be stored only as hashes, excluded from logs and analytics, rate-limited by
normalized address and IP, and bound to the initiating browser where practical.
Pending invitations must reveal no student activity and be visible only to
authorized course instructors so mistyped, expired, and unresolved addresses
can be corrected or revoked.

Optional OIDC, SAML, or LTI integrations converge on the same authenticated
`AccountId` and Store claim command. They are account-linking and course-launch
integrations, not prerequisites for PLE registration or enrollment.

## Future roster authorization contract

The planned roster reads and mutations will use this course authorization
order:

```text
session -> AuthenticatedSession -> exact course lookup -> current Instructor Course Membership
```

The future rules are:

- A direct course Instructor will be able to view and manage the student
  roster.
- A Sysadmin will be able to help an Instructor through the closed roster list,
  invitation, policy, revoke, preview, and commit operations. The Store will
  record authenticated account/course/action/time for each Sysadmin support
  access; this capability will not include grade export, responses, Assignment
  Attempts, Assignment Analysis, or general course access.
- A student member will be able to view the course but will not enumerate or
  mutate the roster.
- A nonmember or foreign-course caller will receive the same not-found response
  as an absent course.
- Instructor access will be manually approved after real-person validation and
  persisted as direct `Instructor` membership. There will be no self-service
  promotion path.
- A membership request will not create Sysadmin authority because `Sysadmin`
  is an operator-approved Product Role, not a Course Membership Role.
- Invitation redemption will use the authenticated student as the target. The
  request will never carry another user's ID.
- Membership authority will be checked before identity-candidate, invitation,
  or roster-revision detail is disclosed.

These planned rules extend, rather than replace, the course boundary in
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md#course-and-educational-records).
When this boundary is implemented, PostgreSQL will establish the trusted role
and authenticated Account context before any membership or educational-record
access described in
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#row-level-security).
Today, the deployment-gated seeded persona selector remains the only browser
entry; no roster or invitation route is active.

## Store invariants

The planned Store boundary owns three connected but intentionally separate
invariants:

1. Active Student Course Membership, the exact Student Record, and the Assignment's
   direct policy are the sole inputs to current Assignment Access.
2. Merely joining a course, creating an assignment, listing work, or reading a
   summary creates no assignment receipt.
3. The first bounded Assignment Access-authorized transition will atomically
   create one enrollment and one typed empty summary with its sealed Assignment
   Access evidence.

The following describes the intended operations:

| Operation                                                           | Atomic effect                                                                                                                        |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Claim invitation (future)                                           | Consume the invitation, resolve the authenticated account, bind the roster identifier, and create the membership episode and profile |
| Create assignment                                                   | Store the assignment; create no student activity rows                                                                                |
| Read pre-activity summary authorized by Assignment Access           | Return a key-free `no_activity` result without creating an enrollment or summary                                                     |
| Start Assignment Attempt, grade-bearing action, or instructor issue | Re-evaluate Assignment Access and, at the planned creation boundary, atomically create or reuse the enrollment and summary receipt   |
| Remove student access                                               | Remove current membership; retain existing educational records for authorized grade, audit, and retention workflows                  |
| Re-add former student                                               | Reuse the stable Student Record and existing activity while deriving current access from the new membership episode                  |

The planned Memory implementation will use one write lock and rollback snapshot
for this compound transition. The planned PostgreSQL implementation will use one
transaction and a consistent lock order when creating the first enrollment and
empty summary receipt. It will retain unique constraints for both `(course,
assignment, student)` and `(course, assignment, user)`. Once that creation
begins, inserting the enrollment and its empty summary must remain one
transaction; no route or migration may hand-write only one side.

## HTTP contract

Course-roster delivery is planned and unavailable. Its future Store, Server Routes, invitation workflow, and atomic
Student Account resolve-or-create transaction; no roster, roster-import,
invitation, or invitation-redemption Server Route currently exists.

The planned Course Roster Import will receive normalized Student Authentication
Emails and course-scoped roster metadata. Before authentication, its atomic
transaction will resolve an existing Student Account by Student Authentication
Email or create a new Student Account when none exists. It will then record the
pending Course Invitation without creating Course relationships or Student Work
Records. A future authentication ceremony will authenticate that existing
Student Account. Invitation redemption will then create the Course Membership
and Student Record relationships for the exact Course Instance.

The future delivery must keep roster data course-scoped and protected: a roster
email snapshot neither replaces nor mutates a Student Authentication Email.
Its invitation secret and email-authentication code remain private, and future
responses and diagnostics must not expose those values, passkey state,
provider subjects, course work, or an unrelated Account's existence. The future
bulk import design remains bounded, parses a reviewed `email,roster_id` set,
and commits its selected rows atomically without creating Student Work Records.

## LibreTexts ADAPT comparison

ADAPT provides a useful human model that PLE should adopt at the workflow
level. Its instructor Students screen supports a single invitation, a comma
separated email list, a downloadable CSV template, roster upload, per-row
status, pending invitations, roster download, section movement, and
unenrollment. Its student enrollment accepts an access code, checks duplicate
course membership, and can verify LMS roster membership. Course enrollment
then creates per-assignment assignment-to-user records.

ADAPT also uses one account across courses rather than one identity per
enrollment. Its `users` table has one numeric primary key and a unique email,
while `enrollments` has a unique `(account_id, course_id)` relationship. A student
therefore keeps the same ADAPT user account in courses taught by different
instructors. Newer migrations add a central identity identifier, but the
invitation path still finds or creates users by email and stores the
institutional student label on the user record.

The future enrollment design adopts ADAPT's one-person/many-course-memberships
shape and its practical single, list, CSV, pending, and revocation workflow. It
improves the identity key by using a PLE-owned opaque `AccountId`, an immutable
Student Authentication Email, a role-qualified Instructor Authentication Email
replacement path, and (if accepted) multiple passkeys rather than making email
the Account primary key. It also keeps the protected `StudentRecordId` distinct
from both the Account identity and the course-scoped institutional roster
identifier.

The strongest ADAPT ideas for PLE are:

- treat enrollment as a course roster task rather than an assignment-by-
  assignment instructor chore;
- in its future implementation, support single invite, bulk roster preview,
  pending status, and revocation;
- retain the instructor-supplied student number beside the course roster so a
  institutional LMS/gradebook export can identify the correct row;
- let course instructors apply exact email domains to Course Invitations;
- let the authenticated student claim an invitation;
- validate LMS-backed membership against the LMS roster when that integration
  is configured; and
- evaluate Assignment Access from Active Student Course Membership; the design
  creates activity only at the first bounded educational event.

PLE intentionally improves several implementation details:

| ADAPT behavior observed in `OTHER_REPOS/adapt`                                                               | PLE decision                                                                                                                           |
| ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| Controllers combine Account Creation, email, LMS checks, enrollment, analytics, and assignment distribution. | Separate account authentication, invitation delivery, authorization, and Store-owned Course Roster Import and Course Membership state. |
| A roster upload is parsed, then the browser sends one invitation request per row.                            | Stage one bounded import and commit the reviewed set idempotently.                                                                     |
| An Instructor invitation may create an Account row by email before that student authenticates.               | Create only a pending invitation. The planned Course Roster Import transaction resolves or creates Student Accounts.                   |
| `student_id` is stored on the global ADAPT user.                                                             | Store an institution-provided roster identifier only on the protected course roster/export mapping.                                    |
| Domain whitelist validation uses substring matching.                                                         | Compare a parsed, normalized complete domain or an explicitly configured subdomain boundary.                                           |
| Access codes are visible, reusable course/invitation values.                                                 | Use random, expiring, single-purpose invitation secrets stored only as hashes.                                                         |
| Course enrollment and assignment distribution are coupled procedurally.                                      | Keep Active Student Course Membership and Assignment Access separate from lazily created assignment activity.                          |
| Unenrollment can permanently remove submissions and scores.                                                  | Revoke access while retaining educational records until the explicit retention workflow acts.                                          |
| Section is a second course subdivision.                                                                      | Treat a PLE `CourseId` as the current course or section boundary; add another hierarchy only from demonstrated need.                   |

The relevant ADAPT evidence is in the local reference checkout at
`OTHER_REPOS/adapt/routes/api.php`,
`OTHER_REPOS/adapt/app/Http/Controllers/EnrollmentController.php`,
`OTHER_REPOS/adapt/app/Http/Controllers/UserController.php`,
`OTHER_REPOS/adapt/app/Enrollment.php`, and
`OTHER_REPOS/adapt/resources/js/pages/instructors/course_properties/students.vue`.
The checkout is comparative evidence, not a PLE runtime dependency.

## Instructor experience

The future ordinary instructor journey will be:

```text
Course -> Students -> Create invitation -> Copy link -> Share through trusted LMS
```

In the future design, configured SMTP may deliver the same link, but no
ordinary enrollment action may depend on PLE operating a mail server. Bulk
roster commit remains SMTP-dependent until a separately reviewed bounded
multi-link handoff exists; it must not return a large page of bearer secrets by
accident.

The screen emphasizes outcomes rather than internal record types:

- `Active` means the student has Active Student Course Membership and can enter
  the course; Assignment Access is evaluated separately for each Assignment.
- `Invitation pending` means no authenticated user has claimed the invitation.
- `Invalid email`, `domain not permitted`, or `roster ID already used` gives a
  row-level correction before commit.
- `Access removed` is historical/audit state, not a promise that records were
  deleted.

The UI does not expose `CourseMembershipRole`, `StudentRecordId`, enrollment IDs, or
the distinction between membership and enrollment as routine settings. It
provides keyboard-operable upload, preview, correction, commit, invite-copy,
domain-policy, and revoke controls consistent with
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md).

## Student experience

In the future enrollment flow, a student will open the invitation in the
browser that requested email authentication or enter the short code there. PLE
will verify the short-lived email challenge before revealing the course.
Confirming once will create a fresh active membership episode without eager
assignment receipts. That future flow may offer an optional passkey shortcut
without blocking course entry only after the deferred passkey capability is
accepted. No email or passkey sign-in route is active today.

In that future flow, a student who is already a member receives a normal
success result. A student whose session needs reauthentication keeps the
invitation only in the URL or controlled sign-in state; PLE does not copy it
into local storage, analytics, or logs. An expired or wrong-user invitation
gives safe retry guidance without disclosing another student or course.

## Account access

The future email-authentication design treats email as the ordinary
account-access path rather than a separate recovery mode:

- a student may register no passkey, one passkey, or several passkeys only
  after the deferred passkey capability is accepted;
- losing or revoking a future passkey returns the student to the same future
  email sign-in path used during registration;
- a Student Authentication Email is immutable; a different institutional email
  identifies a different Student Account;
- the planned Course Roster Import transaction is the normal
  Student Account resolve-or-create boundary; and
- if the student no longer controls the current account email, version 1 has
  no identity-recovery or record-transfer workflow. The instructor may revoke
  the old course membership and invite a new address, while the institutional
  LMS remains the grade system of record for any manual correction.

This refusal is deliberate. A course Instructor can manage course access but
cannot prove that two PLE accounts belong to the same person strongly enough
to move educational records. Any future account merge, record transfer, or
Instructor Authentication Email replacement requires a separate identity-proofing, authorization, audit, and
retention design.

## Removal and retention

Roster removal is an access transition, not record destruction.

- New Assignment Attempts, Question Attempts, asset grants, and invitation redemption are refused after
  membership removal.
- Existing Enrollment and Assignment Attempt Summary receipts and issued evidence remain
  available to authorized direct course instructors under course retention
  policy.
- Re-adding the same student creates a fresh membership episode, reuses the
  stable Student Record, and preserves existing assignment receipts and
  their original Course Membership history.
- Archive/delete jobs remain the only path that disposes student records and
  associated protected objects.
- Every roster mutation records the authenticated account, course, target member, source
  (`single`, `bulk`, `invitation`, or future `lti`), time, and coarse outcome.
  Audit records exclude raw invitation secrets and roster PII.

## Product data boundary

PLE is authoritative for its own account credentials, PLE sessions, course
access, issued Questions, Question Attempts, responses, feedback, practice history, and calculated
score summaries. Those records are required to operate and explain the PLE
learning experience.

The institution's LMS, registrar roster, and gradebook remain authoritative
for official institutional enrollment, legal student identity, final course
grades, transcripts, and institutional retention. The instructor supplies the
course-scoped `roster_id` so PLE can export a deterministic calculated roster
score CSV. An instructor may transfer or later synchronize selected PLE results;
PLE membership does not assert official university enrollment.

This boundary is why PLE does not require legal names or place institutional
student numbers on the global account. It retains the minimum protected roster
mapping needed for the instructor's export and keeps all official-record
interpretation in the institutional system.

### Operational lifetimes and export

PLE treats all course-linked student educational records as FERPA data and
radioactive. This is broader than directly identifying roster fields: it
includes membership, enrollment, Assignment Attempts, Question Attempts, responses, feedback, grades,
exports, artifacts, audit evidence, and opaque values that link a person to
them. Collect a value only for a named teaching operation, keep its authority
narrow, exclude it from general logs and analytics, and remove copies that no
longer serve that operation. That principle must not force an instructor to
hand-match 50 scores.

| Data                            | Instructor convenience                                                          | Minimization control                                                                                                                                                                                                            |
| ------------------------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Student Authentication Email    | Future sign-in to the Student Account                                           | Immutable global Student Account credential; never the account key; not exposed as cross-course instructor data                                                                                                                 |
| Instructor Authentication Email | Future sign-in to the Instructor Account                                        | Private global Instructor Account credential; a future verified replacement preserves that Account's relationships                                                                                                              |
| Course roster email             | Future invite/correction/domain-policy work and institutional-export matching  | Course-scoped protected snapshot; direct course Instructors plus audited Sysadmin roster support; follows course student-record retention                                                                                       |
| Institutional roster ID         | Match PLE results to an LMS/gradebook row                                       | Course-scoped protected record; no global lookup or authentication use                                                                                                                                                          |
| Display name or handle          | Let the instructor distinguish roster members                                   | Student-controlled account data copied only where the course workflow needs it; no legal-name requirement                                                                                                                       |
| Raw roster CSV                  | Import 50 students at once                                                      | Parse in memory or controlled temporary storage, then delete raw bytes after normalized preview creation                                                                                                                        |
| Normalized import preview       | Review errors before sending invitations                                        | Expires after one hour; current Instructor Course Membership access; no account-existence signal                                                                                                                                |
| Grade export                    | Upload results to the institutional system                                      | Contains only the destination profile's required roster ID, course roster email, display label, and selected result fields; never global `AccountId`, passkey state, or unrelated activity; protected, audited, and short-lived |

The planned Course Roster delivery will expire a Course Invitation after
seven days and an email-authentication challenge after ten minutes. Resending
will create new secrets and invalidate the old delivery. Those bounds will be
server constants, not browser choices.

A grade export is generated synchronously for one course and assignment under
the existing current Instructor Course Membership authorization boundary. It uses the course roster
ID as the join key and the server-calculated assignment summary as the value.
The response is `Cache-Control: no-store`, is not persisted as an export
object, and carries a server-issued opaque export ID. The database retains only
a PII-free audit row with the export identity, authenticated account, course, assignment, row
count, and time.

## Related documents

- [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md) defines enrollment, Assignment Attempt, Question Attempt,
  mastery, and grade-selection semantics.
- [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) defines course and
  student-record authority.
- [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md) distinguishes `AccountId`,
  `StudentRecordId`, Course Instance, Course Membership, and browser identities.
- [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#row-level-security)
  defines forced RLS and trusted authenticated Account context.
- [API_CONTRACTS.md](API_CONTRACTS.md) records the routes that currently ship.
- [MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md) defines the
  primary teaching activity the enrolled student experiences.
- [RETENTION_POLICY.md](RETENTION_POLICY.md) owns archive and deletion after
  access changes.
- [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) distinguishes permanent
  behavior tests from disposable integration evidence.
