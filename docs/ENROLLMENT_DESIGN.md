# Enrollment design

## Binding single-installation model

PLE uses global Accounts and exact Course Membership. One course may have multiple equal
Teaching Team Members, and every Student Record belongs to one exact Student Account and Course Instance.
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md) owns the
durable schema and authorization contract.

This document defines the durable identity, authorization, Store, HTTP, privacy,
and user-experience contract for that boundary. Delivery status, implementation
packages, acceptance gates, and maintainer checklist live in the current
[implementation status](active_plans/implementation_status.md).

The primary audience is a contributor implementing course membership, Student
Records, Assignment delivery, roster management, identity resolution, or the
instructor and student enrollment journeys. The release plan remains in
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Status and authority

This document is the durable enrollment contract. Current route truth remains in
[API_CONTRACTS.md](API_CONTRACTS.md).
Current delivery status and implementation evidence for those routes remain in
[implementation status](active_plans/implementation_status.md).

## Product decision

PLE presents **one course-level roster workflow** to instructors. Course
Enrollment establishes a Student Course Membership and its Student Record;
Assignments derive activity authorization from that exact relationship and the
Assignment's current lifecycle and direct Student Record access facts.

An instructor invites a student to the course once. After that student
authenticates and claims the invitation, the same Store-owned transaction:

1. resolves the authenticated PLE `AccountId`;
2. creates or reuses that Student Account's `StudentRecordId` for the exact Course Instance;
3. creates a fresh active `course_membership` episode bound to that Student Record; and
4. stores course-local display/contact evidence in the subordinate roster
   profile.

When an instructor later creates an assignment, PLE stores the assignment for
the Course Instance's active Student Records. The sole Assignment Access
evaluator derives current access from the exact Student Record, Course
Membership, Assignment lifecycle, and direct Student accommodation facts. The
first Assignment Attempt start, grade-bearing action, or explicit Instructor
issue atomically creates the assignment receipt, typed empty summary, direct
access basis, and immutable account-or-rule provenance.

This gives instructors the simple course-enrollment model used successfully by
LibreTexts ADAPT without weakening PLE's more precise activity model:

```text
Instructor action                 Durable PLE records

Add Student to course      ->     course_membership
                                  Student Record
                                  course roster profile

Create later assignment    ->     assignment
                                  direct active Student Record access

First entitlement-bearing  ->     Assignment Attempt and Issued Question
event                             selected Assignment Grade state
                                  sealed authorization and provenance
```

The normal UI does not ask an instructor to add the same student separately to
every assignment. Course Membership and direct Student Record access are the
assignment-delivery contract; absence of a materialized receipt means only that
no access-bearing event has occurred. It is never interpreted as current denial
or current grant.

## Why records remain separate

The product workflow is course-level, but the records answer different
questions:

| Record | Question it answers | Lifetime |
| --- | --- | --- |
| Course membership | May this authenticated person enter or manage this course? | Current access relationship |
| Student Record | Which protected educational record belongs to this Student Account and Course Instance? | Stable across course-membership episodes |
| Assignment Attempt | What immutable Student activity exists for this Student Record and Assignment? | Educational record |
| Assignment Grade | What selected course result should the Gradebook read? | Updated from retained Student activity |

Removing course access therefore does not erase Student Records, attempts,
submissions, or grades. Roster removal revokes future course access. Record
archive and deletion continue through the explicit retention workflow in
[RETENTION_POLICY.md](RETENTION_POLICY.md).

Student-scoped Store operations re-evaluate active `Student` membership and the
exact Student Record's Assignment Access, then bind the result's stable
`StudentRecordId` to any retained receipt at the database/Store boundary. Thus a
revoked student cannot continue to read a run, attempt, summary, feedback
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

- The earlier course-route surface mounted course reads, course creation,
  gradebook reads, and assignment creation/update without roster mutation.
- The deferred course-delivery route family will expose Student-owned
  Assignment Attempt history. Course Enrollment occurs through course-level
  invitation claim rather than a public assignment-enrollment mutation.
- The deferred course-creation command creates a Course Instance with its
  authenticated Instructor membership; roster commands then establish Student
  memberships and Student Records.

The Store remains the authority beneath HTTP. Roster commands create or update
membership and its course-local profile. The entitlement evaluator derives
assignment access from that current membership and the Assignment's direct
policy. Only its bounded `StartRun`, `GradeBearingAction`, and
`InstructorIssue` transitions may atomically create an `enrollment` and its
typed empty `student_assignment_summary`.

The existing whole-course `upsert_course` operation is not the roster
mutation. It replaces the complete member list and has no browser-facing
revision. A route-level read-modify-write through that method could lose a
concurrent roster edit. The current implementation therefore uses focused
atomic member, invitation, policy, import, and roster commands with a strong
roster revision.

## Identity prerequisite

PLE owns its Accounts. An `AccountId` is the stable opaque identity of one PLE
account across courses and institutions; it is not issued by an instructor,
course, university, or email provider. Course membership and account-and-relationship-scoped forced
RLS control access to educational records.

The direct passwordless email/passkey route family owns the account-session
boundary and mints an `__Host-ple_session` only after an authorized course
relationship is chosen or claimed. The deployment-gated seeded persona selector
uses the same account/session records for connected local evidence. The product
direction is:

- email authentication is the canonical registration and sign-in path;
- passkeys are optional convenience credentials for the same account;
- the existing opaque, hashed server-side session and host-only HttpOnly
  `__Host-` cookie remain the browser credential; and
- optional institutional SSO may link a verified external identity to an
  existing PLE account, but it does not own `AccountId`, select a course, or block
  institution-independent deployment.

PLE uses an established WebAuthn implementation rather than implementing the
protocol. It supports discoverable credentials for usernameless login,
multiple passkeys per account, normal authenticator biometric or PIN user
verification, and passkey enrollment during registration. Attestation is not
required without a future managed-device use case. Authenticator user
verification proves access to the account; it is not proctoring or proof of a
student's legal identity.

The minimum identity contract is:

| Value | Owner | Rule |
| --- | --- | --- |
| `AccountId` | PLE identity system | Stable opaque PLE Account identity across Course Instances |
| Email | PLE identity system | Verified, mutable authentication attribute and canonical sign-in address; never the primary key |
| Passkey credentials | PLE identity system | Optional convenience credentials; multiple credentials are allowed per account |
| Display name or handle | User account profile | User-controlled safe label; no legal-name requirement |
| `StudentRecordId` | PLE educational-record store | Stable protected educational record for one Student Account inside one exact Course Instance |
| Optional SSO binding | PLE identity system | Verified external issuer/subject linked to an existing `AccountId`; server-only and never roster authority |

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

| Session | Issuer and purpose | What it establishes |
| --- | --- | --- |
| `__Host-ple_session` | Email code, passkey, or deployment-gated seeded-persona entry | One Authenticated Session used with exact course, assignment, run, and roster relationships |

Invitation redemption uses the authenticated Account session before exact course relationship resolution.
Passkey registration begins from an authenticated PLE account, so a passkey can
shorten later sign-in but cannot bootstrap the first account by itself. The
seeded selector is disabled when its deployment settings are absent. Email
start fails closed unless both the invitation-token secret and a complete
external SMTP configuration are present; mounting a route is not evidence of a
live email-authentication ceremony.

ENR6 therefore uses canonical email authentication to restore a provisioned PLE
Account before invitation redemption. Copy-link delivery removes SMTP from the
invitation handoff, but it does not replace account authentication. The local
browser exercises the real account and account-session records; the seeded
selector is a deployment convenience for connected evidence, not a parallel
identity or invitation path.

### Person, course, and email

The account belongs to the student:

```text
PLE account
  AccountId 42
  authentication email: verified and changeable
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

Email is mutable personally identifiable information. PLE stores a normalized
verified address as the canonical authentication attribute and stores the
delivery form only where email delivery requires it. Email is not a database
primary key, course selector, course authority, or durable person identity. An
email change requires appropriate account verification; an address later
reassigned to another person cannot silently inherit the old account's
memberships or grades.

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
of the instructor's mapping and may differ later from the student's mutable
account sign-in email.

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

Invitation at a verified email address is the normal enrollment path. The
instructor may copy the returned one-time link into an existing trusted LMS or
let a configured SMTP provider deliver the same link:

```text
instructor enters email and roster ID
    -> PLE creates a pending invitation
    -> PLE returns one copyable invitation link in the no-store create response
    -> instructor shares it through an LMS, or configured SMTP sends it
    -> student completes short-lived, single-use email authentication
    -> PLE resolves the student's provisioned opaque AccountId
    -> student claims the invitation
    -> PLE creates course membership and its roster profile atomically
    -> student enrolls one or more passkeys
```

An existing PLE Account and a newly provisioned Account follow the same outward
flow. The server matches the verified email to its existing Account; it never
creates an Account during authentication. The Instructor cannot query whether
an address already has an Account, and existing and nonexistent addresses
receive the same outward invitation result.

The invitation link is a bearer secret. PLE returns it only in the Instructor's
no-store creation response, keeps it in browser memory for that page session,
and never places it in roster reads, storage, logs, or analytics. The server
stores only its hash. The instructor must share it only with the intended
student and revoke the pending invitation if it reaches the wrong person. The
link proves possession of the invitation, not control of the roster email, so it
never replaces the student's email-authentication ceremony.

Email authentication tokens are short-lived and single-use. They are stored
only as hashes, excluded from logs and analytics, rate-limited by normalized
address and IP, and bound to the initiating browser where practical. Pending
invitations reveal no student activity and are visible only to authorized
course instructors so mistyped, expired, and unresolved addresses can be
corrected or revoked.

Optional OIDC, SAML, or LTI integrations converge on the same authenticated
`AccountId` and Store claim command. They are account-linking and course-launch
integrations, not prerequisites for PLE registration or enrollment.

## Authorization contract

Roster reads and mutations use the existing course authorization order:

```text
session -> AuthenticatedSession -> exact course lookup -> direct Instructor membership
```

The rules are:

- A direct course Instructor may view and manage the student roster.
- A Sysadmin may help an Instructor through the closed roster list, invitation,
  policy, revoke, preview, and commit operations. The Store records
  authenticated account/course/action/time for each Sysadmin support access; this capability
  does not include grade export, responses, runs, item analysis, or general
  course access.
- A student member may view the course but cannot enumerate or mutate the
  roster.
- A nonmember or foreign-course caller receives the same not-found response as
  an absent course.
- Instructor access is manually approved after real-person validation and
  persisted as direct `Instructor` membership. There is no self-service
  promotion path.
- A membership request cannot create Sysadmin authority because `Sysadmin` is
  an operator-approved Product Role, not a Course Membership Role.
- Invitation redemption uses the authenticated student as the target. The
  request never carries another user's ID.
- Membership authority is checked before identity-candidate, invitation, or
  roster-revision detail is disclosed.

These rules extend, rather than replace, the course boundary in
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md#course-and-educational-records).
PostgreSQL still establishes the trusted role and authenticated Account context before any
membership or educational-record access described in
[DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#row-level-security).

## Store invariants

The Store owns three connected but intentionally separate invariants:

1. Active course membership, the exact Student Record, and the Assignment's
   direct policy are the sole inputs to current Assignment Access.
2. Merely joining a course, creating an assignment, listing work, or reading a
   summary creates no assignment receipt.
3. The first bounded entitlement-bearing transition atomically creates one
   enrollment and one typed empty summary with its sealed grant and provenance.

The following operations preserve them:

| Operation | Atomic effect |
| --- | --- |
| Claim invitation | Consume the invitation, resolve the authenticated account, bind the roster identifier, and create the membership episode and profile |
| Create assignment | Store the assignment; create no student activity rows |
| Read entitled pre-activity summary | Return a key-free `no_activity` projection without creating an enrollment or summary |
| Start run, grade-bearing action, or instructor issue | Re-evaluate entitlement and atomically create or reuse the enrollment and summary receipt |
| Remove student access | Remove current membership; retain existing educational records for authorized grade, audit, and retention workflows |
| Re-add former student | Reuse the stable Student Record and existing activity while deriving current access from the new membership episode |

Memory uses one write lock and rollback snapshot for compound transitions.
PostgreSQL uses one transaction and a consistent lock order when materializing
the first receipt. The database retains unique constraints for both `(course,
assignment, student)` and `(course, assignment, user)`. Once materialization
begins, inserting the enrollment and its empty summary remains one transaction;
no route or migration may hand-write only one side.

## HTTP contract

The current usable slice exposes a small course-roster API. The authority and
payload rules are normative.

| Method and path | Purpose | Request authority |
| --- | --- | --- |
| `GET /api/courses/{course}/roster` | Cursor-paged current members and pending invitations | Course from path plus direct Instructor or audited Sysadmin roster-support authorization |
| `POST /api/courses/{course}/invitations` | Create one pending invitation and return its one-time copy link; configured SMTP may also deliver it | Email, course-scoped roster identifier, and idempotency key |
| `POST /api/course-invitations/redeem` | Claim a pending invitation | Opaque invitation secret plus the authenticated account session |
| `PUT /api/courses/{course}/invitation-email-rule` | Replace the allowed email domains applied to Instructor-issued Course Invitations | Exact roster revision plus direct Instructor or audited Sysadmin roster-support authorization |
| `DELETE /api/courses/{course}/members/{member}` | Revoke current course access without deleting records | Existing member path plus exact roster revision |
| `POST /api/courses/{course}/roster-imports/preview` | Parse and stage bounded `email,roster_id` CSV | Exact roster revision plus direct Instructor or audited Sysadmin roster-support authorization |
| `POST /api/courses/{course}/roster-imports/{import}/commit` | Commit the reviewed ready rows atomically | Import revision plus idempotency key |

The roster response is deliberately small:

```json
{
  "members": [
    {
      "member_id": "opaque-member-id",
      "display_name": "Student Name",
      "roster_email": "netid@mail.roosevelt.edu",
      "roster_id": "900123456",
      "role": "student",
      "status": "active"
    }
  ],
  "pending_invitations": [
    {
      "invitation_id": "opaque-invitation-id",
      "email": "netid@mail.roosevelt.edu",
      "roster_id": "900654321",
      "status": "pending",
      "expires_at": "2026-08-17T12:00:00Z"
    }
  ],
  "allowed_email_domains": ["mail.roosevelt.edu"],
  "next_cursor": null,
  "roster_revision": 4
}
```

**Current pre-WN1 note:** route payloads may retain direct lower-camel fields until their migration packages close.
The object above is the approved direct-Serde `snake_case` contract; browser DOM and framework values retain their upstream spellings.

It does not return provider subjects, passkey state, raw invitation tokens,
course-selection fields, assignment enrollments, attempts, submissions, or
grades. A pending row may show the exact address and roster identifier entered
by that course's Instructor so a typo or mismatch can be corrected. After claim,
that address becomes protected course roster metadata; a later account-email
change does not silently rewrite it. Invitation and email-authentication
secrets are stored only as hashes. Diagnostics and later reads show coarse
status and expiry, never a secret. The invitation-creation response is the sole
exception: it returns the one-time secret in a same-origin relative fragment so
an authorized Instructor can copy it without exposing it in an HTTP request,
server log, or later roster response:

```json
{
  "invitation": {
    "invitation_id": "opaque-invitation-id",
    "email": "netid@mail.roosevelt.edu",
    "roster_id": "900654321",
    "status": "pending",
    "expires_at": "2026-08-17T12:00:00Z"
  },
  "redemption_path": "/course-invitations/redeem#token=one-time-base64url-secret",
  "email_delivery": "queued"
}
```

`email_delivery` is `queued` when the invitation is accepted for processing,
including pending or retryable work. It is never proof that a provider accepted
the message or that a mailbox received it. `sent_to_provider` means only that the
configured provider accepted the submission. `needs_attention` covers an
ambiguous result or a failure that remains after retry processing, including a
permanent failure, and requires explicit operator action.
`cancelled` is fenced, so its link must not be shared. Without SMTP, the
copy-link path remains usable. The browser decoder rejects absolute or
cross-origin redemption URLs. An exact idempotent retry reproduces the same
path from server-held key material so the server does not persist plaintext
solely to support retry.

Mutations return `Cache-Control: no-store`. Creating or claiming an invitation
uses an `Idempotency-Key`; policy replacement, revocation, and bulk commit use
a strong roster revision. The server mints member, student, enrollment, and
invitation identities. A browser never supplies them as new record identities.

### Failure shape

| Condition | Result |
| --- | --- |
| Missing or expired session | `401` with the normal reauthentication path |
| Missing, foreign, or concealed course | `404` |
| Student tries an Instructor action | `403` after valid course membership is known |
| Malformed email or roster identifier | Safe `422` without account-existence detail |
| Existing or nonexistent PLE account at that email | Identical accepted invitation response |
| SMTP absent | Accepted single invitation with `email_delivery: queued` and the copy-link path |
| Retryable delivery work | `email_delivery: queued`; processing may continue without provider or mailbox evidence |
| Ambiguous or permanent delivery failure | `email_delivery: needs_attention`; operator action is required |
| Cancelled invitation | `email_delivery: cancelled`; the link is fenced and must not be shared |
| Reused invitation by the same resulting member | Idempotent existing-membership result |
| Reused invitation by another user | Safe conflict; no course or claimant detail |
| Stale roster revision or changed import | `409` with reload guidance |
| Store or directory unavailable | `503` without row, email, provider, or database detail |

## Bulk roster workflow

A teaching-first roster must handle a normal class of about 50 students without
requiring 50 modal submissions. The bulk path uses a preview and commit flow,
not a series of unrelated browser requests.

1. The instructor downloads a simple CSV template or selects a configured
   institutional LMS/registrar export profile.
2. PLE accepts a bounded CSV body, parses it server-side, and discards the raw
   file after producing a staged normalized import.
3. The preview reports row-numbered states such as `ready_to_invite`,
   `already_member`, `already_pending`, `duplicate`, and `invalid` without
   revealing whether an unrelated PLE account exists.
4. The instructor reviews the exact accepted set.
5. Commit uses the staged import ID, its strong revision, and an idempotency
   key. PLE commits the selected ready rows atomically.
6. Each committed row invokes the same Store command as one-member addition;
   it creates the membership/profile boundary without bypassing authorization
   or pre-materializing assignment activity.

The initial bounds are one MiB, 500 data rows, and a closed UTF-8 CSV grammar.
The implementation keeps those limits as constants covered at their boundary.
The parser reports row numbers and safe categories rather than
echoing raw names, email addresses, student labels, or malformed cells into
logs and error bodies.

The generic CSV contract requires `email` and `roster_id`. `roster_id` is the
institutional identifier needed to match a PLE result back to the institutional LMS
or gradebook export. For a Roosevelt roster that may be the `900xxxxxx`
student number paired with the student's `netID@mail.roosevelt.edu` address.
It is stored as protected course-scoped roster metadata, never as `AccountId`, an
authentication claim, or a globally searchable account attribute. It is
unique within that course and follows the course's educational-record
retention policy.

An institution profile may map alternate input headings onto those two fields
and validate a bounded identifier grammar. PLE does not accept an
instructor-supplied regular expression. The generic profile accepts a bounded
opaque roster identifier; a reviewed Roosevelt profile may require exactly the
documented nine-digit `900xxxxxx` form. Names from the source roster are
optional preview aids and are not required account attributes.

Each committed valid row creates a pending invitation. It does not create an
authenticated identity during preview or commit. The account is resolved or
created only after that student authenticates the address and claims the
invitation.

### Allowed email domains

Each course may define a revisioned Course Invitation Email Rule. For example,
a Roosevelt course can permit `mail.roosevelt.edu`. The rule catches likely
Instructor typos during single or bulk invitation and limits invitation
destinations to reviewed domains.

PLE parses the domain after the final `@`, lowercases and IDNA-normalizes it,
and compares the complete domain. A value such as
`student@mail.roosevelt.edu.attacker.example` must not match
`mail.roosevelt.edu`. Subdomains are accepted only when an Instructor explicitly
configures a subdomain policy; substring matching is forbidden.

An allowed domain is not proof that the person is a Student and does not
replace email authentication or the exact invitation binding. Course Instructors
may add or remove domains with the roster revision and audit trail. An empty
list leaves Instructor-issued invitations unrestricted. An Instructor who needs
an outside address can add the exact domain or issue one explicit, audited
invitation.

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

PLE adopts ADAPT's one-person/many-course-memberships shape and its practical
single, list, CSV, pending, and revocation workflow. It improves the identity
key by using a PLE-owned opaque `AccountId`, mutable verified email, and multiple
passkeys rather than making email the account primary key. PLE also keeps the
protected `StudentRecordId` distinct from both the Account identity and the
course-scoped institutional roster identifier.

The strongest ADAPT ideas for PLE are:

- treat enrollment as a course roster task rather than an assignment-by-
  assignment instructor chore;
- support single invite, bulk roster preview, pending status, and revocation;
- retain the instructor-supplied student number beside the course roster so a
  institutional LMS/gradebook export can identify the correct row;
- let course instructors apply exact email domains to Course Invitations;
- let the authenticated student claim an invitation;
- validate LMS-backed membership against the LMS roster when that integration
  is configured; and
- derive assignment access immediately from membership while materializing
  per-assignment activity only at the first bounded educational event.

PLE intentionally improves several implementation details:

| ADAPT behavior observed in `OTHER_REPOS/adapt` | PLE decision |
| --- | --- |
| Controllers combine identity provisioning, email, LMS checks, enrollment, analytics, and assignment distribution. | Separate account authentication, invitation delivery, authorization, and Store-owned roster reconciliation. |
| A roster upload is parsed, then the browser sends one invitation request per row. | Stage one bounded import and commit the reviewed set idempotently. |
| An Instructor invitation may create an Account row by email before that student authenticates. | Create only a pending invitation; a Sysadmin-owned provisioning workflow creates the Account before its first email authentication. |
| `student_id` is stored on the global ADAPT user. | Store an institution-provided roster identifier only on the protected course roster/export mapping. |
| Domain whitelist validation uses substring matching. | Compare a parsed, normalized complete domain or an explicitly configured subdomain boundary. |
| Access codes are visible, reusable course/invitation values. | Use random, expiring, single-purpose invitation secrets stored only as hashes. |
| Course enrollment and assignment distribution are coupled procedurally. | Keep current membership/entitlement separate from lazily materialized assignment activity. |
| Unenrollment can permanently remove submissions and scores. | Revoke access while retaining educational records until the explicit retention workflow acts. |
| Section is a second course subdivision. | Treat a PLE `CourseId` as the current course or section boundary; add another hierarchy only from demonstrated need. |

The relevant ADAPT evidence is in the local reference checkout at
`OTHER_REPOS/adapt/routes/api.php`,
`OTHER_REPOS/adapt/app/Http/Controllers/EnrollmentController.php`,
`OTHER_REPOS/adapt/app/Http/Controllers/UserController.php`,
`OTHER_REPOS/adapt/app/Enrollment.php`, and
`OTHER_REPOS/adapt/resources/js/pages/instructors/course_properties/students.vue`.
The checkout is comparative evidence, not a PLE runtime dependency.

## Instructor experience

The ordinary instructor journey is:

```text
Course -> Students -> Create invitation -> Copy link -> Share through trusted LMS
```

Configured SMTP may deliver the same link, but no ordinary enrollment action
depends on PLE operating a mail server. Bulk roster commit remains
SMTP-dependent until a separately reviewed bounded multi-link handoff exists;
it must not return a large page of bearer secrets by accident.

The screen emphasizes outcomes rather than internal record types:

- `Active` means the student can enter the course and has assignment access.
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

A student opens the invitation in the browser that requested email
authentication or enters the short code in that browser. PLE verifies the
short-lived email challenge before revealing the course. Confirming once
creates a fresh active membership episode without eager assignment receipts.
PLE then offers an optional passkey shortcut without blocking course entry. An
existing account may always authenticate through email; a registered passkey
provides an additional direct sign-in option.

A student who is already a member receives a normal success result. A student
whose session needs reauthentication keeps the invitation only in the URL or
controlled sign-in state; PLE does not copy it into local storage, analytics,
or logs. An expired or wrong-user invitation gives safe retry guidance
without disclosing another student or course.

## Account access

Email authentication is the ordinary account-access path rather than a
separate recovery mode:

- a student may register no passkey, one passkey, or several passkeys;
- losing or revoking a passkey returns the student to the same email sign-in
  path used during registration;
- a signed-in student may replace the authentication email only after
  verification of the new address in the bound browser;
- an instructor may re-invite a student at a corrected or replacement address;
  this reaches the provisioned Account proven by that email and never merges
  Accounts or transfers records based on email alone; and
- if the student no longer controls the current account email, version 1 has
  no identity-recovery or record-transfer workflow. The instructor may revoke
  the old course membership and invite a new address, while the institutional
  LMS remains the grade system of record for any manual correction.

This refusal is deliberate. A course Instructor can manage course access but
cannot prove that two PLE accounts belong to the same person strongly enough
to move educational records. Any future account merge or record-transfer
feature requires a separate identity-proofing, authorization, audit, and
retention design.

## Removal and retention

Roster removal is an access transition, not record destruction.

- New runs, attempts, asset grants, and invitation redemption are refused after
  membership removal.
- Existing materialized receipts, summaries, and issued evidence remain
  available to authorized direct course instructors under course retention
  policy.
- Re-adding the same student creates a fresh membership episode, reuses the
  stable Student Record, and preserves existing assignment receipts and
  their original membership provenance.
- Archive/delete jobs remain the only path that disposes student records and
  associated protected objects.
- Every roster mutation records the authenticated account, course, target member, source
  (`single`, `bulk`, `invitation`, or future `lti`), time, and coarse outcome.
  Audit records exclude raw invitation secrets and roster PII.

## Product data boundary

PLE is authoritative for its own account credentials, PLE sessions, course
access, issued attempts, responses, feedback, practice history, and calculated
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
includes membership, enrollment, attempts, responses, feedback, grades,
exports, artifacts, audit evidence, and opaque values that link a person to
them. Collect a value only for a named teaching operation, keep its authority
narrow, exclude it from general logs and analytics, and remove copies that no
longer serve that operation. That principle must not force an instructor to
hand-match 50 scores.

| Data | Instructor convenience | Minimization control |
| --- | --- | --- |
| Authentication email | Sign in to the provisioned PLE Account | Global Account credential; never the account key; not exposed as cross-course instructor data |
| Course roster email | Invite, correct, apply allowed-domain policy, and match an institutional export | Course-scoped protected snapshot; direct course Instructors plus audited Sysadmin roster support; follows course student-record retention |
| Institutional roster ID | Match PLE results to an LMS/gradebook row | Course-scoped protected record; no global lookup or authentication use |
| Display name or handle | Let the instructor distinguish roster members | Student-controlled account projection copied only where the course workflow needs it; no legal-name requirement |
| Raw roster CSV | Import 50 students at once | Parse in memory or controlled temporary storage, then delete raw bytes after normalized preview creation |
| Normalized import preview | Review errors before sending invitations | Expires after one hour; direct-Instructor access; no account-existence signal |
| Grade export | Upload results to the institutional system | Contains only the destination profile's required roster ID, course roster email, display label, and selected result fields; never global `AccountId`, passkey state, or unrelated activity; protected, audited, and short-lived |

The current implementation expires a course invitation after seven days and an
email-authentication challenge after ten minutes. Resending creates new
secrets and invalidates the old delivery. Those bounds are server constants,
not browser choices.

A grade export is generated synchronously for one course and assignment under
the existing direct-Instructor authorization boundary. It uses the course roster
ID as the join key and the server-calculated assignment summary as the value.
The response is `Cache-Control: no-store`, is not persisted as an export
object, and carries a server-issued opaque export ID. The database retains only
a PII-free audit row with the export identity, authenticated account, course, assignment, row
count, and time.

## Related documents

- [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md) defines enrollment, run, attempt,
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
