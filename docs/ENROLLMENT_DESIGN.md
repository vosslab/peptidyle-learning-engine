# Enrollment design

PLE has a supported passwordless account, course-roster, invitation,
assignment-enrollment, and manual grade-export slice. This document defines
that boundary and distinguishes the implemented slice from the remaining
production acceptance work.

The primary audience is a contributor implementing course membership,
assignment enrollment, roster management, identity resolution, or the
instructor and learner enrollment journeys. The exact active work-package
order remains in the
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Status and authority

**Implemented, acceptance open:** PLE owns opaque global accounts, short-lived
browser-bound email authentication, discoverable WebAuthn registration and
authentication, multiple passkeys, verified account-email replacement, and
course-context selection. An instructor can list a cursor-paged roster, set an
exact-domain policy, create a copyable one-time invitation link, optionally send
that link through configured SMTP, revoke invitations, preview and atomically
commit a bounded CSV roster, revoke access, and download a no-store
course-assignment grade CSV. A learner can authenticate and claim an
invitation; the Store then creates the course membership, tenant learner
identity, every assignment enrollment, and every empty summary atomically.
Memory and PostgreSQL implement the same contract, and later assignment
creation enrolls all current learners.

**Acceptance still open:** the canonical email-authentication journey still needs
an off-the-shelf disposable SMTP sink or configured provider, optional-passkey and
multi-replica evidence, and independent security/HCI closeout. PLE does not own a
mail server or delivery stack.

This document is the durable enrollment contract. Current route truth remains in
[API_CONTRACTS.md](API_CONTRACTS.md) and
[crates/server/src/course/routing.rs](../crates/server/src/course/routing.rs).

## Product decision

PLE presents **one course-level roster workflow** to instructors while keeping
course membership and assignment enrollment as separate internal concepts.

An instructor invites a student to the course once. After that student
authenticates and claims the invitation, the same Store-owned transaction:

1. resolves the authenticated PLE `UserId`;
2. creates or reuses that user's tenant-scoped pedagogical `StudentId`;
3. creates the student's `course_member` row;
4. creates one enrollment for every existing assignment in the course; and
5. creates the empty summary paired with every new enrollment.

When an instructor later creates an assignment, PLE creates an enrollment and
empty summary for every current student member in that same transaction.

This gives instructors the simple course-enrollment model used successfully by
LibreTexts ADAPT without weakening PLE's more precise activity model:

```text
Instructor action                 Durable PLE records

Add learner to course      ->     course_member
                                  tenant learner identity
                                  assignment enrollment 1 + empty summary
                                  assignment enrollment 2 + empty summary
                                  ...

Create later assignment    ->     assignment
                                  one enrollment + summary per student member
```

The normal UI does not ask an instructor to add the same student separately to
every assignment. A public assignment-enrollment endpoint is therefore not the
primary product workflow. If PLE later supports assignments offered to only a
subset of a course, that feature must add an explicit assignment-audience
contract and reconcile enrollments from it. Absence of an enrollment must not
silently become an undocumented targeting mechanism.

## Why records remain separate

The product workflow is course-level, but the records answer different
questions:

| Record | Question it answers | Lifetime |
| --- | --- | --- |
| Course membership | May this authenticated person enter or manage this course? | Current access relationship |
| Tenant learner identity | Which pedagogical student record belongs to this authenticated user? | Stable within the tenant |
| Assignment enrollment | What cross-run mastery and grade state does this student have for this assignment? | Educational record |
| Assignment summary | What compact current result should the gradebook read? | Updated transactionally with learner activity |

Removing course access therefore does not erase enrollments, attempts,
submissions, or grades. Roster removal revokes future course access. Record
archive and deletion continue through the explicit retention workflow in
[RETENTION_POLICY.md](RETENTION_POLICY.md).

The activity hierarchy remains the one in
[ACTIVITY_MODEL.md](ACTIVITY_MODEL.md): enrollment owns repeated runs, and a
run owns issued question attempts. Course membership never becomes an answer,
score, completion flag, or attempt authority.

## Gap closed by the current slice

The original missing seam was visible at three boundaries:

- [crates/server/src/course/routing.rs](../crates/server/src/course/routing.rs)
  previously mounted course reads, course creation, gradebook reads, and
  assignment creation/update without roster mutation.
- [crates/server/src/run/routes.rs](../crates/server/src/run/routes.rs) mounts
  still exposes enrollment reads and run history; enrollment creation now
  occurs through course-level invitation claim rather than a public
  assignment-enrollment mutation.
- [crates/server/src/course/queries.rs](../crates/server/src/course/queries.rs)
  creates a course with the authenticated creator as instructor; the focused
  roster routes now add learners afterward.

The Store remains the authority beneath HTTP. `CourseAssignmentStore` owns
`create_enrollment_impl`, and both backends enforce the important compound
write. The PostgreSQL implementation first verifies that the target user is a
student member of the assignment's course, then inserts both `enrollment` and
`student_assignment_summary` before committing.

The existing whole-course `upsert_course` operation is not the roster
mutation. It replaces the complete member list and has no browser-facing
revision. A route-level read-modify-write through that method could lose a
concurrent roster edit. The current implementation therefore uses focused
atomic member, invitation, policy, import, and roster commands with a strong
roster revision.

## Identity prerequisite

PLE owns its user accounts. A `UserId` is the stable opaque identity of one PLE
account across courses and institutions; it is not issued by an instructor,
course, university, or email provider. Course membership and tenant-scoped RLS
control access to educational records.

The existing `IdentityProvider` trait remains the credential-verification
boundary. WP-RC8 replaces the current deployment assumption that an
institutional OIDC provider is required with a production passwordless
provider behind that trait:

- email authentication is the canonical registration and sign-in path;
- passkeys are optional convenience credentials for the same account;
- the existing opaque, hashed server-side session and host-only HttpOnly
  cookie remain the browser credential; and
- optional institutional SSO may link a verified external identity to an
  existing PLE account, but it does not own `UserId`, select a tenant, or block
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
| `UserId` | PLE identity system | Stable opaque PLE account identity across courses and institutions |
| Email | PLE identity system | Verified, mutable authentication attribute and canonical sign-in address; never the primary key |
| Passkey credentials | PLE identity system | Optional convenience credentials; multiple credentials are allowed per account |
| Display name or handle | User account profile | User-controlled safe label; no legal-name requirement |
| `StudentId` | PLE learner-identity store | Stable pedagogical identity associated with the PLE user inside an educational-record tenant |
| Optional SSO binding | PLE identity system | Verified external issuer/subject linked to an existing `UserId`; server-only and never roster authority |

The account-to-learner mapping remains tenant-scoped because `StudentId`
belongs to the educational-record and retention boundary, not because the PLE
account belongs to that tenant:

```text
user_id                         -> one PLE account
(tenant_id, user_id)            -> student_id
(tenant_id, student_id)         -> user_id
```

Both tenant mapping directions are unique. The same account can therefore
participate in courses owned by different institutions while each institution
retains an independently scoped pedagogical record. The current
`SessionSubject` carries one tenant context; WP-RC8 preserves that safe RLS
context while deriving it only from an authorized course or tenant
relationship, never from a browser-supplied tenant identifier.

### Person, course, and email

The account belongs to the learner:

```text
PLE account
  UserId 42
  authentication email: verified and changeable
  passkeys: laptop, phone
  Tenant A learner identity: StudentId 91
    course membership: Biochemistry, student
  Tenant B learner identity: StudentId 37
    course membership: Genetics, student
```

The two instructors see only records in courses they are authorized to manage.
Neither instructor receives the learner's global `UserId`, passkey metadata, or
activity from another course. Course authorization, learner ownership, query
scope, and RLS remain the disclosure controls.

PLE does not create a different account for each course enrollment. Per-course
account fragmentation would complicate record inspection, correction,
retention, and deletion without improving course authorization.
`EnrollmentId` already gives each student-assignment relationship a durable
identity without pretending one human is several people.

Email is mutable personally identifiable information. PLE stores a normalized
verified address as the canonical authentication attribute and stores the
delivery form only where email delivery requires it. Email is not a database
primary key, tenant selector, course authority, or durable person identity. An
email change requires appropriate account verification; an address later
reassigned to another person cannot silently inherit the old account's
memberships or grades.

### Data minimization and roster metadata

PLE retains enough identifying information to make enrollment and grade export
practical, but only while it has a concrete teaching, authentication, audit,
or export purpose. In short: **collect reluctantly, use
deliberately, purge predictably**.

An active course roster may therefore contain:

- the institutional email supplied by the instructor for invitation, roster
  management, permitted-domain policy, and manual grade export;
- the institution-issued student number supplied by the instructor for
  reliable LMS/gradebook row matching; and
- a learner-selected display name or handle that helps the instructor manage
  the class.

These are protected course/tenant operational metadata. They do not establish
the PLE account identity. `UserId` remains the opaque account identifier, while
the roster email and student number must not become credentials, primary keys,
or cross-course search fields. The course roster email is a retained snapshot
of the instructor's mapping and may differ later from the learner's mutable
account sign-in email.

Course roster metadata follows [RETENTION_POLICY.md](RETENTION_POLICY.md): the
default 30-day notice leaves it available for corrections and final export,
the 100-day archive removes it from ordinary learner access, and the 365-day
delete removes the learner graph unless an authorized extension or earlier
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
    -> learner completes short-lived, single-use email authentication
    -> PLE resolves or creates the learner's opaque UserId
    -> learner claims the invitation
    -> PLE creates course membership and assignment enrollments atomically
    -> learner enrolls one or more passkeys
```

An existing PLE user and a new user follow the same outward flow. Only after
successful email authentication may the server match the verified email to an
existing account or create a new `UserId`. The instructor cannot query whether
an address already has an account, and existing and nonexistent addresses
receive the same outward invitation result.

The invitation link is a bearer secret. PLE returns it only in the manager's
no-store creation response, keeps it in browser memory for that page session,
and never places it in roster reads, storage, logs, or analytics. The server
stores only its hash. The instructor must share it only with the intended
learner and revoke the pending invitation if it reaches the wrong person. The
link proves possession of the invitation, not control of the roster email, so it
never replaces the learner's email-authentication ceremony.

Email authentication tokens are short-lived and single-use. They are stored
only as hashes, excluded from logs and analytics, rate-limited by normalized
address and IP, and bound to the initiating browser where practical. Pending
invitations reveal no student activity and are visible only to authorized
course managers so mistyped, expired, and unresolved addresses can be
corrected or revoked.

Optional OIDC, SAML, or LTI integrations converge on the same authenticated
`UserId` and Store claim command. They are account-linking and course-launch
integrations, not prerequisites for PLE registration or enrollment.

## Authorization contract

Roster reads and mutations use the existing course authorization order:

```text
session -> TenantContext -> course lookup -> direct course role or tenant administrator
```

The rules are:

- A direct course instructor or tenant administrator may view and manage the
  student roster.
- A student member may view the course but cannot enumerate or mutate the
  roster.
- A nonmember or foreign-tenant caller receives the same not-found response as
  an absent course.
- The current implementation creates only `Student` memberships. Adding or
  promoting instructors is a separate, higher-risk administrator workflow.
- A membership request cannot create tenant-administrator authority because
  `Administrator` is not a persistable course membership role.
- Invitation redemption uses the authenticated learner as the target. The
  request never carries another user's ID.
- Membership authority is checked before identity-candidate, invitation, or
  roster-revision detail is disclosed.

These rules extend, rather than replace, the course boundary in
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md#course-and-educational-records).
PostgreSQL still establishes the trusted role and tenant context before any
membership or educational-record access described in
[DATABASE_TENANCY.md](DATABASE_TENANCY.md).

## Store invariants

The Store owns the invariant that connects the otherwise separate records:

> Every current student course member has exactly one enrollment and one
> summary for every current assignment in that course.

The following operations preserve it:

| Operation | Atomic effect |
| --- | --- |
| Claim invitation | Consume the invitation, resolve the authenticated account, bind the roster identifier, add membership, and add all missing assignment enrollments and summaries |
| Create assignment | Create assignment, then add enrollment and summary for every current student member |
| Retry an accepted add | Return the existing member/enrollment result without duplicates |
| Remove student access | Remove current membership and group membership; retain educational records for authorized grade, audit, and retention workflows |
| Re-add former student | Reuse learner identity and existing assignment enrollments; create only genuinely missing pairs |

Memory uses one write lock and rollback snapshot for the compound change.
PostgreSQL uses one transaction and a consistent course-level lock before it
reads either the roster or the assignment set. Both add-member and
create-assignment acquire that lock in the same order so neither race can leave
a missing cell in the member-by-assignment cross product.

The database retains unique constraints for both `(tenant, assignment,
student)` and `(tenant, assignment, user)`. Inserting an enrollment and its
empty summary remains one transaction. No route or migration may hand-write
only the enrollment row.

## HTTP contract

The current usable slice exposes a small course-roster API. The authority and
payload rules are normative.

| Method and path | Purpose | Request authority |
| --- | --- | --- |
| `GET /api/courses/{course}/roster` | Cursor-paged current members and pending invitations | Course from path plus manager authorization |
| `POST /api/courses/{course}/invitations` | Create one pending invitation and return its one-time copy link; configured SMTP may also deliver it | Email, course-scoped roster identifier, and idempotency key |
| `POST /api/course-invitations/redeem` | Claim a pending invitation | Opaque invitation secret plus the authenticated account session |
| `PUT /api/courses/{course}/enrollment-policy` | Replace allowed email domains and signup posture | Exact roster revision plus manager authorization |
| `DELETE /api/courses/{course}/members/{member}` | Revoke current course access without deleting records | Existing member path plus exact roster revision |
| `POST /api/courses/{course}/roster-imports/preview` | Parse and stage bounded `email,roster_id` CSV | Exact roster revision plus manager authorization |
| `POST /api/courses/{course}/roster-imports/{import}/commit` | Commit the reviewed ready rows atomically | Import revision plus idempotency key |
| `GET /api/courses/{course}/assignments/{assignment}/grade-export.csv` | Download the current manual grade export | Course and assignment from path plus manager authorization |

The roster response is deliberately small:

```json
{
  "members": [
    {
      "memberId": "opaque-member-id",
      "displayName": "Student Name",
      "rosterEmail": "netid@mail.roosevelt.edu",
      "rosterId": "900123456",
      "role": "student",
      "status": "active"
    }
  ],
  "pendingInvitations": [
    {
      "invitationId": "opaque-invitation-id",
      "email": "netid@mail.roosevelt.edu",
      "rosterId": "900654321",
      "status": "pending",
      "expiresAt": "2026-08-17T12:00:00Z"
    }
  ],
  "allowedEmailDomains": ["mail.roosevelt.edu"],
  "nextCursor": null,
  "rosterRevision": 4
}
```

It does not return provider subjects, passkey state, raw invitation tokens,
tenant-selection fields, assignment enrollments, attempts, submissions, or
grades. A pending row may show the exact address and roster identifier entered
by that course's manager so a typo or mismatch can be corrected. After claim,
that address becomes protected course roster metadata; a later account-email
change does not silently rewrite it. Invitation and email-authentication
secrets are stored only as hashes. Diagnostics and later reads show coarse
status and expiry, never a secret. The invitation-creation response is the sole
exception: it returns the one-time secret in a same-origin relative fragment so
an authorized manager can copy it without exposing it in an HTTP request,
server log, or later roster response:

```json
{
  "invitation": {
    "invitationId": "opaque-invitation-id",
    "email": "netid@mail.roosevelt.edu",
    "rosterId": "900654321",
    "status": "pending",
    "expiresAt": "2026-08-17T12:00:00Z"
  },
  "redemptionPath": "/course-invitations/redeem#token=one-time-base64url-secret",
  "emailDelivery": "notSent"
}
```

`emailDelivery` is `sent` only when the established SMTP adapter accepted the
message; `notSent` leaves the copy link as the delivery path. The browser
decoder rejects absolute or cross-origin redemption URLs. An exact idempotent
retry reproduces the same path from server-held key material so the server does
not persist plaintext solely to support retry.

Mutations return `Cache-Control: no-store`. Creating or claiming an invitation
uses an `Idempotency-Key`; policy replacement, revocation, and bulk commit use
a strong roster revision. The server mints member, learner, enrollment, and
invitation identities. A browser never supplies them as new record identities.

### Failure shape

| Condition | Result |
| --- | --- |
| Missing or expired session | `401` with the normal reauthentication path |
| Missing, foreign, or concealed course | `404` |
| Student tries a manager action | `403` after valid course membership is known |
| Malformed email or roster identifier | Safe `422` without account-existence detail |
| Existing or nonexistent PLE account at that email | Identical accepted invitation response |
| SMTP absent or rejects delivery | Accepted single invitation with `emailDelivery: notSent` and the copy-link path |
| Reused invitation by the same resulting member | Idempotent existing-membership result |
| Reused invitation by another user | Safe conflict; no course or claimant detail |
| Stale roster revision or changed import | `409` with reload guidance |
| Store or directory unavailable | `503` without row, email, provider, or database detail |

## Bulk roster workflow

A teaching-first roster must handle a normal class of about 50 students without
requiring 50 modal submissions. The bulk path uses a preview and commit flow,
not a series of unrelated browser requests.

1. The instructor downloads a simple CSV template or selects a configured
   manual LMS/registrar export profile.
2. PLE accepts a bounded CSV body, parses it server-side, and discards the raw
   file after producing a staged normalized import.
3. The preview reports row-numbered states such as `readyToInvite`,
   `alreadyMember`, `alreadyPending`, `duplicate`, and `invalid` without
   revealing whether an unrelated PLE account exists.
4. The instructor reviews the exact accepted set.
5. Commit uses the staged import ID, its strong revision, and an idempotency
   key. PLE commits the selected ready rows atomically.
6. Each committed row invokes the same Store command as one-member addition;
   it cannot omit an assignment summary or bypass authorization.

The initial bounds are one MiB, 500 data rows, and a closed UTF-8 CSV grammar.
The implementation keeps those limits as constants covered at their boundary.
The parser reports row numbers and safe categories rather than
echoing raw names, email addresses, student labels, or malformed cells into
logs and error bodies.

The generic CSV contract requires `email` and `roster_id`. `roster_id` is the
institutional identifier needed to match a PLE result back to the manual LMS
or gradebook export. For a Roosevelt roster that may be the `900xxxxxx`
student number paired with the student's `netID@mail.roosevelt.edu` address.
It is stored as protected course-scoped roster metadata, never as `UserId`, an
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
created only after that learner authenticates the address and claims the
invitation.

### Allowed email domains

Each course may define a revisioned list of permitted invitation and signup
domains. For example, a Roosevelt course can permit
`mail.roosevelt.edu`. The policy serves two purposes:

- catch likely instructor typos during single or bulk invitation; and
- constrain any future course-code or self-signup flow to addresses from an
  explicitly allowed domain.

PLE parses the domain after the final `@`, lowercases and IDNA-normalizes it,
and compares the complete domain. A value such as
`student@mail.roosevelt.edu.attacker.example` must not match
`mail.roosevelt.edu`. Subdomains are accepted only when a manager explicitly
configures a subdomain policy; substring matching is forbidden.

An allowed domain is not proof that the person is a student and does not
replace email authentication or the exact invitation binding. Course managers
may add or remove domains with the roster revision and audit trail. An empty
list means invitation-only enrollment has no additional domain restriction;
future open signup must require at least one allowed domain. A manager who
needs an outside address must add the exact domain or create an explicit,
audited one-invitation exception rather than silently bypassing the rule.

## LibreTexts ADAPT comparison

ADAPT provides a useful human model that PLE should adopt at the workflow
level. Its instructor Students screen supports a single invitation, a comma
separated email list, a downloadable CSV template, roster upload, per-row
status, pending invitations, roster download, section movement, and
unenrollment. Its learner enrollment accepts an access code, checks duplicate
course membership, and can verify LMS roster membership. Course enrollment
then creates per-assignment assignment-to-user records.

ADAPT also uses one account across courses rather than one identity per
enrollment. Its `users` table has one numeric primary key and a unique email,
while `enrollments` has a unique `(user_id, course_id)` relationship. A student
therefore keeps the same ADAPT user account in courses taught by different
instructors. Newer migrations add a central identity identifier, but the
invitation path still finds or creates users by email and stores the
institutional student label on the user record.

PLE adopts ADAPT's one-person/many-course-memberships shape and its practical
single, list, CSV, pending, and revocation workflow. It improves the identity
key by using a PLE-owned opaque `UserId`, mutable verified email, and multiple
passkeys rather than making email the account primary key. PLE also keeps the
pedagogical `StudentId` distinct from both the account identity and the
course-scoped institutional roster identifier.

The strongest ADAPT ideas for PLE are:

- treat enrollment as a course roster task rather than an assignment-by-
  assignment instructor chore;
- support single invite, bulk roster preview, pending status, and revocation;
- retain the instructor-supplied student number beside the course roster so a
  manual LMS/gradebook export can identify the correct row;
- let course managers restrict invitation and signup addresses to allowed
  email domains;
- let the authenticated learner claim an invitation;
- validate LMS-backed membership against the LMS roster when that integration
  is configured; and
- create the per-assignment state as part of enrollment rather than waiting for
  the first answer.

PLE intentionally improves several implementation details:

| ADAPT behavior observed in `OTHER_REPOS/adapt` | PLE decision |
| --- | --- |
| Controllers combine identity provisioning, email, LMS checks, enrollment, analytics, and assignment distribution. | Separate account authentication, invitation delivery, authorization, and Store-owned roster reconciliation. |
| A roster upload is parsed, then the browser sends one invitation request per row. | Stage one bounded import and commit the reviewed set idempotently. |
| An instructor invitation may create a user row by email before that learner authenticates. | Create only a pending invitation; resolve or create `UserId` after the learner authenticates the address. |
| `student_id` is stored on the global ADAPT user. | Store an institution-provided roster identifier only on the protected course roster/export mapping. |
| Domain whitelist validation uses substring matching. | Compare a parsed, normalized complete domain or an explicitly configured subdomain boundary. |
| Access codes are visible, reusable course/invitation values. | Use random, expiring, single-purpose invitation secrets stored only as hashes. |
| Course enrollment and assignment distribution are coupled procedurally. | Keep membership and assignment enrollment as typed records connected by one atomic invariant. |
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

- `Active` means the learner can enter the course and has assignment access.
- `Invitation pending` means no authenticated user has claimed the invitation.
- `Invalid email`, `domain not permitted`, or `roster ID already used` gives a
  row-level correction before commit.
- `Access removed` is historical/audit state, not a promise that records were
  deleted.

The UI does not expose `CourseMembershipRole`, `StudentId`, enrollment IDs, or
the distinction between membership and enrollment as routine settings. It
provides keyboard-operable upload, preview, correction, commit, invite-copy,
domain-policy, and revoke controls consistent with
[NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md).

## Learner experience

A learner opens the invitation in the browser that requested email
authentication or enters the short code in that browser. PLE verifies the
short-lived email challenge before revealing the course. Confirming once
creates or restores membership and the required assignment enrollments. PLE
then offers an optional passkey shortcut without blocking course entry. An
existing account may always authenticate through email; a registered passkey
provides an additional direct sign-in option.

A learner who is already a member receives a normal success result. A learner
whose session needs reauthentication keeps the invitation only in the URL or
controlled sign-in state; PLE does not copy it into local storage, analytics,
or logs. An expired or wrong-user invitation gives safe retry guidance
without disclosing another learner or course.

## Account access

Email authentication is the ordinary account-access path rather than a
separate recovery mode:

- a learner may register no passkey, one passkey, or several passkeys;
- losing or revoking a passkey returns the learner to the same email sign-in
  path used during registration;
- a signed-in learner may replace the authentication email only after
  verification of the new address in the bound browser;
- an instructor may re-invite a learner at a corrected or replacement address;
  this creates or reaches the account proven by that email and never merges
  accounts or transfers records based on email alone; and
- if the learner no longer controls the current account email, version 1 has
  no identity-recovery or record-transfer workflow. The instructor may revoke
  the old course membership and invite a new address, while the institutional
  LMS remains the grade system of record for any manual correction.

This refusal is deliberate. A course manager can manage course access but
cannot prove that two PLE accounts belong to the same person strongly enough
to move educational records. Any future account merge or record-transfer
feature requires a separate identity-proofing, authorization, audit, and
retention design.

## Removal and retention

Roster removal is an access transition, not record destruction.

- New runs, attempts, asset grants, and invitation redemption are refused after
  membership removal.
- Existing enrollments and summaries remain available to authorized course
  managers under course retention policy.
- Existing group membership is removed with course membership.
- Re-adding the same learner reuses the stable learner identity and existing
  enrollments.
- Archive/delete jobs remain the only path that disposes learner records and
  associated protected objects.
- Every roster mutation records actor, course, target member, source
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
course-scoped `roster_id` so PLE can export a deterministic manual gradebook
file. An instructor may transfer or later synchronize selected PLE results;
PLE membership does not assert official university enrollment.

This boundary is why PLE does not require legal names or place institutional
student numbers on the global account. It retains the minimum protected roster
mapping needed for the instructor's export and keeps all official-record
interpretation in the institutional system.

### Operational lifetimes and export

PLE treats directly identifying roster data as radioactive: collect it only
for a named teaching operation, keep its authority narrow, and remove copies
that no longer serve that operation. That principle must not force an
instructor to hand-match 50 scores.

| Data | Instructor convenience | Minimization control |
| --- | --- | --- |
| Authentication email | Register and sign in to the PLE account | Global account attribute; never the account key; not exposed as cross-course instructor data |
| Course roster email | Invite, correct, apply allowed-domain policy, and match a manual institutional export | Course-scoped protected snapshot; course managers only; follows course learner-record retention |
| Institutional roster ID | Match PLE results to an LMS/gradebook row | Course-scoped protected record; no global lookup or authentication use |
| Display name or handle | Let the instructor distinguish roster members | Learner-controlled account projection copied only where the course workflow needs it; no legal-name requirement |
| Raw roster CSV | Import 50 learners at once | Parse in memory or controlled temporary storage, then delete raw bytes after normalized preview creation |
| Normalized import preview | Review errors before sending invitations | Expires after one hour; course-manager access; no account-existence signal |
| Grade export | Upload results to the institutional system | Contains only the destination profile's required roster ID, course roster email, display label, and selected result fields; never global `UserId`, passkey state, or unrelated activity; protected, audited, and short-lived |

The current implementation expires a course invitation after seven days and an
email-authentication challenge after ten minutes. Resending creates new
secrets and invalidates the old delivery. Those bounds are server constants,
not browser choices.

A grade export is generated synchronously for one course and assignment under
the existing course-manager authorization boundary. It uses the course roster
ID as the join key and the server-calculated assignment summary as the value.
The response is `Cache-Control: no-store`, is not persisted as an export
object, and carries a server-issued opaque export ID. The database retains only
a PII-free audit row with the export identity, actor, course, assignment, row
count, and time.

## Implementation packages

The safe dependency order is:

### ENR1: Passwordless identity and account mapping - implemented

- Add a production PLE-owned account store keyed by opaque global `UserId`.
- Add short-lived, single-use email authentication for registration and sign-in,
  including browser binding where practical, uniform outward responses,
  rate limits, secret hashing, and redacted diagnostics.
- Add discoverable WebAuthn credentials behind `IdentityProvider` using an
  established implementation; support multiple passkeys and account-managed
  credential revocation.
- Keep email mutable and separate from `UserId`; require verified account
  control for email changes.
- Add the stable `(TenantId, UserId) <-> StudentId` pedagogical mapping without
  letting browser input select tenant or identity.
- Replace the production assumption that institutional OIDC is required;
  optional SSO links to an existing account through the same identity
  boundary.

### ENR2: Atomic roster Store - implemented

- Add focused add, revoke, list, invitation, and bulk-reconcile capabilities.
- Persist course-scoped roster email, roster identifier, allowed domains, and
  invitation lifecycle with the learner-record retention boundary.
- Add a roster revision and course-level lock order.
- Reconcile the full student-member by assignment cross product.
- Extend assignment creation to enroll all current student members atomically.

### ENR3: Single-member HTTP - implemented

- Mount roster list, invitation creation/copy/redemption, optional established
  SMTP delivery, allowed-domain
  settings, and access-revocation routes.
- Use existing course concealment and manager authorization.
- Add strict request/response DTOs, no-store behavior, idempotency, revisions,
  and safe errors.

### ENR4: Instructor and learner UI - implemented, acceptance open

- Add the Students screen, invitation flow, learner claim page, initial passkey
  enrollment, multiple-passkey management, and email change.
- Keep the common path course-level and hide internal enrollment mechanics.
- Prove the platform keyboard path before optional shortcuts.

### ENR5: Bulk roster - implemented

- Add bounded `email` plus `roster_id` CSV template, manual LMS export profiles,
  parse/preview, staged revision, and atomic invitation commit.
- Add row status and correction guidance without raw-PII diagnostics.
- Add exact normalized allowed-domain validation and reviewed institutional
  roster-ID profiles without arbitrary instructor regexes.

### ENR6: Integration evidence - acceptance open

- Replace SQL-arranged membership in the primary multi-actor walkthrough with
  instructor roster action and learner claim/login.
- Exercise copy-link invitation handoff without a PLE-owned mail system. Use an
  off-the-shelf disposable SMTP sink only for the canonical email-authentication
  ceremony; do not add a test-only authentication bypass.
- Prove gradebook, item analysis, learner isolation, assignment creation after
  roster creation, and roster addition after assignment creation.
- Keep LTI Names and Roles roster synchronization in its separately authorized
  integration package; it must call the same Store command.
- Prove a deterministic manual gradebook export keyed by the protected
  course-scoped roster identifier; do not claim it changes the institutional
  system of record.

## Acceptance gates

Permanent behavior and contract tests must prove:

- manager, student, nonmember, and foreign-tenant authorization outcomes;
- email-authentication and invitation secrets are single-use, bounded, hashed,
  rate-limited, and cannot cross account, browser binding, course, or expiry;
- existing and nonexistent email addresses have the same outward invitation
  and authentication response shape;
- one account may hold multiple passkeys, and revoking one credential does not
  revoke the others or expose credential metadata to course managers;
- a changed or reassigned email cannot inherit another `UserId` or educational
  record;
- allowed-domain matching uses the complete normalized domain and rejects
  substring, suffix-confusion, and malformed-IDNA cases;
- course roster IDs are unique inside the course, absent from account lookup,
  and present in the intended manual grade export;
- student membership cannot create instructor or administrator authority;
- adding a member creates all missing enrollments and summaries atomically;
- creating an assignment creates all required enrollments and summaries;
- both operation orders produce the same complete cross product;
- concurrent add-member and create-assignment operations leave no missing or
  duplicate enrollment;
- Memory and PostgreSQL implement the same idempotent behavior;
- a failed summary insert rolls back membership and enrollment changes;
- removal revokes access without deleting educational records;
- re-addition reuses the learner identity and existing activity;
- bulk preview is bounded and commit is revisioned and idempotent;
- raw roster bytes and expired previews are removed at their documented
  boundary;
- error bodies, exports, and logs exclude provider subjects, passkey metadata,
  invitation secrets, raw CSV cells, and unrelated learner data; and
- current gradebook reads succeed immediately for newly created empty
  summaries.

Disposable integration evidence must prove:

- a real non-superuser PostgreSQL role and RLS context enforce the boundary;
- one API replica can create or redeem membership and another can serve the
  resulting course and assignment;
- an instructor can create and copy a learner invitation through the browser
  without SQL, `cargo tools e2e-seed`, or configured SMTP;
- the learner authenticates by email through an off-the-shelf sink or configured
  provider, optionally enrolls a passkey, enters the course, starts the
  assignment, submits, and appears in the instructor gradebook;
- the instructor downloads a protected manual grade export whose roster IDs
  match the imported rows and whose contents exclude account email and global
  `UserId`; and
- the local provider, production passwordless provider, optional OIDC/SAML
  connector, and future LTI adapter converge on the same `UserId`, session, and
  Store operation rather than implementing separate roster semantics.

One-time implementation probes may inspect lock order, query plans, migration
backfill, and representative CSV timing. Keep a probe as a permanent test only
when it meets the behavior-focused criteria in
[PYTEST_STYLE.md](PYTEST_STYLE.md) and
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

## Non-goals

This enrollment slice keeps the following work in its existing owner:

- automatic LMS Names and Roles roster synchronization remains integration
  work after the core roster and manual export contract;
- optional institutional OIDC/SAML account linking remains an integration
  inside WP-RC8 and is not required for production passwordless login;
- legal-identity verification and proctoring are not implied by email,
  passkeys, roster IDs, or authenticator user verification;
- course sections use separate PLE courses until teaching evidence requires a
  nested section model;
- co-instructor and grader invitation use a separate role-elevation review;
- assignment subsets require an explicit assignment-audience design; and
- roster removal does not replace retention, archival, legal hold, or account
  deletion.

## Maintainer checklist

Before changing enrollment behavior, verify:

1. Is this a course access relationship, a learner identity, or assignment
   activity state?
2. Does the authenticated session establish the account, then derive tenant
   context from an authorized course relationship rather than browser input?
3. Does an email remain a verified, mutable authentication attribute rather
   than becoming account identity or course authority?
4. Does one Store transaction preserve membership, every required enrollment,
   and every empty summary?
5. Does creating an assignment preserve the same cross-product invariant?
6. Does removal revoke access while retention remains authoritative for
   records?
7. Are bulk and single paths the same command with the same idempotency and
   authorization?
8. Can the primary instructor-to-learner browser journey run without SQL or a
   seeding CLI?
9. Can the instructor import and export by course roster ID without exposing
   email, global `UserId`, or unrelated course activity?

## Related documents

- [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md) defines enrollment, run, attempt,
  mastery, and grade-selection semantics.
- [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) defines course and
  learner-record authority.
- [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md) distinguishes `UserId`,
  `StudentId`, course, enrollment, and browser identities.
- [DATABASE_TENANCY.md](DATABASE_TENANCY.md) defines RLS and trusted database
  context.
- [API_CONTRACTS.md](API_CONTRACTS.md) records the routes that currently ship.
- [MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md) defines the
  primary teaching activity the enrolled learner experiences.
- [RETENTION_POLICY.md](RETENTION_POLICY.md) owns archive and deletion after
  access changes.
- [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) distinguishes permanent
  behavior tests from disposable integration evidence.
