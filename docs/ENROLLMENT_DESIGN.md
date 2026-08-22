# Enrollment design

PLE has implemented passwordless-account, course-roster, invitation,
assignment-enrollment, and manual grade-export components. This document
defines that boundary and distinguishes implemented production composition
from the still-open acceptance work.

The primary audience is a contributor implementing course membership,
assignment enrollment, roster management, identity resolution, or the
instructor and learner enrollment journeys. The exact active work-package
order remains in the
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Status and authority

**Implemented source and generic-route components:** PLE owns opaque global
accounts, short-lived browser-bound email-authentication and account-session
components, discoverable WebAuthn registration and authentication, multiple
passkeys, verified account-email replacement, and account-to-course-context
selection. The generic router mounts those account routes with the course
roster, invitation, bulk import, atomic enrollment, and manual grade-export
routes. Memory and PostgreSQL implement the same Store contract, including
canonical course-membership episodes, the tenant learner identity, derived
assignment entitlement, and first-event materialization of one assignment
receipt with its typed empty summary and immutable provenance. Roster and
assignment writes do not eagerly create learner records.

**Current startable composition:** `production_router_from_env` builds the
persistent dependencies and composes the PLE passwordless/account/session graph
with an eight-hour `FirstPartyHttps` policy and explicit `ReviewNotRequired`.
The local browser uses that same graph: email authentication is canonical,
ordinary passkeys are optional shortcuts, and a seeded persona selector is
available only when its deployment configuration is complete. The selector
enters the same persisted account/session state; it is not a separate identity
or membership model.

**Production acceptance still open:** canonical email-authentication evidence
needs a live operator-selected external SMTP provider test account. Optional-
passkey and multi-replica journeys plus deployment acceptance remain. PLE does
not own a mail server, sender reputation, or deliverability stack.

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
3. creates a fresh active `course_member` episode; and
4. stores course-local display/contact evidence in the subordinate roster
   profile.

When an instructor later creates an assignment, PLE stores the assignment and
its explicit audience only. The sole entitlement evaluator derives current
access from active membership, audience, and typed group membership. The first
run start, grade-bearing action, or explicit instructor issue atomically
creates the assignment receipt, typed empty summary, grant basis, applicable
policy scopes, and immutable actor-or-rule provenance.

This gives instructors the simple course-enrollment model used successfully by
LibreTexts ADAPT without weakening PLE's more precise activity model:

```text
Instructor action                 Durable PLE records

Add learner to course      ->     course_member
                                  tenant learner identity
                                  course roster profile

Create later assignment    ->     assignment
                                  explicit course-wide or group audience

First entitlement-bearing  ->     assignment enrollment receipt
event                             typed empty summary
                                  sealed grant/scopes/provenance
```

The normal UI does not ask an instructor to add the same student separately to
every assignment. A public assignment-enrollment endpoint is therefore not the
primary product workflow. Assignment targeting is the explicit audience
contract; absence of a materialized receipt means only that no entitlement-
bearing event has occurred. It is never interpreted as current denial or
current grant.

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

Learner-scoped Store operations re-evaluate active `Student` membership,
assignment audience, and applicable groups, then bind the result's stable
`StudentId` to any retained receipt at the database/Store boundary. Thus a
revoked learner cannot continue to read a run, attempt, summary, feedback
release, or prefetch that was issued before removal. Direct course instructors
use distinct Instructor-history operations for records retained for grade, audit, and
retention work; membership removal does not accidentally erase that explicit
Instructor authority. Sysadmin status grants no general access to those
records; its closed, audited roster-support capability is the explicit
support exception.

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

The Store remains the authority beneath HTTP. Roster commands create or update
membership and its course-local profile. The entitlement evaluator derives
assignment access from that current membership, the assignment audience, and
typed group membership. Only its bounded `StartRun`, `GradeBearingAction`, and
`InstructorIssue` transitions may atomically create an `enrollment` and its
typed empty `student_assignment_summary`.

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

The direct passwordless email/passkey route family owns the account-session
boundary and mints a tenant `__Host-ple_session` only after an authorized course
relationship is chosen or claimed. The deployment-gated seeded persona selector
uses the same account/session records for connected local evidence. The product
direction is:

- email authentication is the canonical registration and sign-in path;
- passkeys are optional convenience credentials for the same account;
- the existing opaque, hashed server-side session and host-only HttpOnly
  `__Host-` cookie remain the browser credential; and
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

### Account and local browser session boundary

The local browser and deployed product use the same PLE-owned account contract:

| Session | Issuer and purpose | What it establishes |
| --- | --- | --- |
| `__Host-ple_session` | PLE account course selection or invitation claim after email, passkey, or deployment-gated seeded-persona entry | One tenant-scoped `SessionSubject` for course, assignment, run, and roster actions |
| `__Host-ple_account_session` | Passwordless email, an already registered passkey, or the deployment-gated seeded selector | One tenant-independent PLE account backed by persisted account and account-session records |

Invitation redemption requires the account session before the tenant session.
Passkey registration begins from an authenticated PLE account, so a passkey can
shorten later sign-in but cannot bootstrap the first account by itself. The
seeded selector is disabled when its deployment settings are absent. Email
start fails closed unless both the invitation-token secret and a complete
external SMTP configuration are present; mounting a route is not evidence of a
live email-authentication ceremony.

ENR6 therefore uses canonical email authentication to create or restore the PLE
account before invitation redemption. Copy-link delivery removes SMTP from the
invitation handoff, but it does not replace account authentication. The local
browser exercises the real account and account-session records; the seeded
selector is a deployment convenience for connected evidence, not a parallel
identity or invitation path.

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
    -> PLE creates course membership and its roster profile atomically
    -> learner enrolls one or more passkeys
```

An existing PLE user and a new user follow the same outward flow. Only after
successful email authentication may the server match the verified email to an
existing account or create a new `UserId`. The instructor cannot query whether
an address already has an account, and existing and nonexistent addresses
receive the same outward invitation result.

The invitation link is a bearer secret. PLE returns it only in the Instructor's
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
course instructors so mistyped, expired, and unresolved addresses can be
corrected or revoked.

Optional OIDC, SAML, or LTI integrations converge on the same authenticated
`UserId` and Store claim command. They are account-linking and course-launch
integrations, not prerequisites for PLE registration or enrollment.

## Authorization contract

Roster reads and mutations use the existing course authorization order:

```text
session -> TenantContext -> course lookup -> direct Instructor membership
```

The rules are:

- A direct course Instructor may view and manage the student roster.
- A Sysadmin may help an Instructor through the closed roster list, invitation,
  policy, revoke, preview, and commit operations. The Store records
  actor/course/action/time for each Sysadmin support access; this capability
  does not include grade export, responses, runs, item analysis, or general
  course access.
- A student member may view the course but cannot enumerate or mutate the
  roster.
- A nonmember or foreign-tenant caller receives the same not-found response as
  an absent course.
- Instructor access is manually approved after real-person validation and
  persisted as direct `Instructor` membership. There is no self-service
  promotion path.
- A membership request cannot create Sysadmin authority because `Sysadmin` is
  an operator-approved account role, not a course membership role.
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

The Store owns three connected but intentionally separate invariants:

1. Active course membership, assignment audience, and typed group membership
   are the sole inputs to current assignment entitlement.
2. Merely joining a course, creating an assignment, listing work, or reading a
   summary creates no assignment receipt.
3. The first bounded entitlement-bearing transition atomically creates one
   enrollment and one typed empty summary with its sealed grant and provenance.

The following operations preserve them:

| Operation | Atomic effect |
| --- | --- |
| Claim invitation | Consume the invitation, resolve the authenticated account, bind the roster identifier, and create the membership episode and profile |
| Create assignment | Store the assignment and its explicit audience; create no learner activity rows |
| Read entitled pre-activity summary | Return a key-free `noActivity` projection without creating an enrollment or summary |
| Start run, grade-bearing action, or instructor issue | Re-evaluate entitlement and atomically create or reuse the enrollment and summary receipt |
| Remove student access | Remove current membership and group membership; retain existing educational records for authorized grade, audit, and retention workflows |
| Re-add former student | Reuse learner identity and existing activity while deriving current access from the new membership episode |

Memory uses one write lock and rollback snapshot for compound transitions.
PostgreSQL uses one transaction and a consistent lock order when materializing
the first receipt. The database retains unique constraints for both `(tenant,
assignment, student)` and `(tenant, assignment, user)`. Once materialization
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
| `PUT /api/courses/{course}/enrollment-policy` | Replace allowed email domains and signup posture | Exact roster revision plus direct Instructor or audited Sysadmin roster-support authorization |
| `DELETE /api/courses/{course}/members/{member}` | Revoke current course access without deleting records | Existing member path plus exact roster revision |
| `POST /api/courses/{course}/roster-imports/preview` | Parse and stage bounded `email,roster_id` CSV | Exact roster revision plus direct Instructor or audited Sysadmin roster-support authorization |
| `POST /api/courses/{course}/roster-imports/{import}/commit` | Commit the reviewed ready rows atomically | Import revision plus idempotency key |
| `POST /api/courses/{course}/assignments/{assignment}/grade-export.csv` | Download the current manual grade export | Course and assignment from path plus direct Instructor authorization |

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
    "invitationId": "opaque-invitation-id",
    "email": "netid@mail.roosevelt.edu",
    "rosterId": "900654321",
    "status": "pending",
    "expiresAt": "2026-08-17T12:00:00Z"
  },
  "redemptionPath": "/course-invitations/redeem#token=one-time-base64url-secret",
  "emailDelivery": "queued"
}
```

`emailDelivery` is `queued` when the invitation is accepted for processing,
including pending or retryable work. It is never proof that a provider accepted
the message or that a mailbox received it. `sentToProvider` means only that the
configured provider accepted the submission. `needsAttention` covers an
ambiguous result or a failure that remains after retry processing, including a
permanent failure, and requires explicit operator action.
`cancelled` is fenced, so its link must not be shared. Without SMTP, the
copy-link path remains usable. The browser decoder rejects absolute or
cross-origin redemption URLs. An exact idempotent retry reproduces the same
path from server-held key material so the server does not persist plaintext
solely to support retry.

Mutations return `Cache-Control: no-store`. Creating or claiming an invitation
uses an `Idempotency-Key`; policy replacement, revocation, and bulk commit use
a strong roster revision. The server mints member, learner, enrollment, and
invitation identities. A browser never supplies them as new record identities.

### Failure shape

| Condition | Result |
| --- | --- |
| Missing or expired session | `401` with the normal reauthentication path |
| Missing, foreign, or concealed course | `404` |
| Student tries an Instructor action | `403` after valid course membership is known |
| Malformed email or roster identifier | Safe `422` without account-existence detail |
| Existing or nonexistent PLE account at that email | Identical accepted invitation response |
| SMTP absent | Accepted single invitation with `emailDelivery: queued` and the copy-link path |
| Retryable delivery work | `emailDelivery: queued`; processing may continue without provider or mailbox evidence |
| Ambiguous or permanent delivery failure | `emailDelivery: needsAttention`; operator action is required |
| Cancelled invitation | `emailDelivery: cancelled`; the link is fenced and must not be shared |
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
   it creates the membership/profile boundary without bypassing authorization
   or pre-materializing assignment activity.

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
`mail.roosevelt.edu`. Subdomains are accepted only when an Instructor explicitly
configures a subdomain policy; substring matching is forbidden.

An allowed domain is not proof that the person is a student and does not
replace email authentication or the exact invitation binding. Course instructors
may add or remove domains with the roster revision and audit trail. An empty
list means invitation-only enrollment has no additional domain restriction;
future open signup must require at least one allowed domain. An Instructor who
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
- let course instructors restrict invitation and signup addresses to allowed
  email domains;
- let the authenticated learner claim an invitation;
- validate LMS-backed membership against the LMS roster when that integration
  is configured; and
- derive assignment access immediately from membership while materializing
  per-assignment activity only at the first bounded educational event.

PLE intentionally improves several implementation details:

| ADAPT behavior observed in `OTHER_REPOS/adapt` | PLE decision |
| --- | --- |
| Controllers combine identity provisioning, email, LMS checks, enrollment, analytics, and assignment distribution. | Separate account authentication, invitation delivery, authorization, and Store-owned roster reconciliation. |
| A roster upload is parsed, then the browser sends one invitation request per row. | Stage one bounded import and commit the reviewed set idempotently. |
| An instructor invitation may create a user row by email before that learner authenticates. | Create only a pending invitation; resolve or create `UserId` after the learner authenticates the address. |
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
creates a fresh active membership episode without eager assignment receipts.
PLE then offers an optional passkey shortcut without blocking course entry. An
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
- Existing group membership is removed with course membership.
- Re-adding the same learner creates a fresh membership episode, reuses the
  stable learner identity, and preserves existing assignment receipts and
  their original membership provenance.
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
| Authentication email | Register and sign in to the PLE account | Global account attribute; never the account key; not exposed as cross-course instructor data |
| Course roster email | Invite, correct, apply allowed-domain policy, and match a manual institutional export | Course-scoped protected snapshot; direct course Instructors plus audited Sysadmin roster support; follows course learner-record retention |
| Institutional roster ID | Match PLE results to an LMS/gradebook row | Course-scoped protected record; no global lookup or authentication use |
| Display name or handle | Let the instructor distinguish roster members | Learner-controlled account projection copied only where the course workflow needs it; no legal-name requirement |
| Raw roster CSV | Import 50 learners at once | Parse in memory or controlled temporary storage, then delete raw bytes after normalized preview creation |
| Normalized import preview | Review errors before sending invitations | Expires after one hour; direct-Instructor access; no account-existence signal |
| Grade export | Upload results to the institutional system | Contains only the destination profile's required roster ID, course roster email, display label, and selected result fields; never global `UserId`, passkey state, or unrelated activity; protected, audited, and short-lived |

The current implementation expires a course invitation after seven days and an
email-authentication challenge after ten minutes. Resending creates new
secrets and invalidates the old delivery. Those bounds are server constants,
not browser choices.

A grade export is generated synchronously for one course and assignment under
the existing direct-Instructor authorization boundary. It uses the course roster
ID as the join key and the server-calculated assignment summary as the value.
The response is `Cache-Control: no-store`, is not persisted as an export
object, and carries a server-issued opaque export ID. The database retains only
a PII-free audit row with the export identity, actor, course, assignment, row
count, and time.

## Implementation packages

The safe dependency order is:

### ENR1 status

The source and generic route components are implemented. The repository-owned
production composition is implemented and independently reviewed; package
acceptance remains open.

- Add a production PLE-owned account store keyed by opaque global `UserId`.
- Add short-lived, single-use email authentication for registration and sign-in,
  including browser binding where practical, uniform outward responses,
  rate limits, secret hashing, and redacted diagnostics.
- Add discoverable WebAuthn credentials on the PLE account boundary using an
  established implementation; support multiple passkeys and account-managed
  credential revocation.
- Keep email mutable and separate from `UserId`; require verified account
  control for email changes.
- Add the stable `(TenantId, UserId) <-> StudentId` pedagogical mapping without
  letting browser input select tenant or identity.
- Keep the PLE-owned email-account path in the production composition;
  optional SSO links to an existing account through the same identity boundary
  when enabled.

### ENR2: Atomic roster Store - implemented

- Add focused add, revoke, list, invitation, and bulk-reconcile capabilities.
- Persist course-scoped roster email, roster identifier, allowed domains, and
  invitation lifecycle with the learner-record retention boundary.
- Add a roster revision and course-level lock order.
- Derive assignment entitlement from current membership, audience, and groups.
- Materialize enrollment and summary together only at a bounded first event.

### ENR3: Single-member HTTP - implemented

- Mount roster list, invitation creation/copy/redemption, optional established
  SMTP delivery, allowed-domain
  settings, and access-revocation routes.
- Use existing course concealment and direct-Instructor authorization.
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
- Keep course and assignment creation as the only arranged setup steps until
  their instructor UI exists; every later membership, login, course-entry,
  learner-work, and gradebook action is walked through the supported surface.
- Exercise copy-link invitation handoff without a PLE-owned mail system. Use a
  test account at the operator-selected SMTP provider for the canonical
  email-authentication ceremony. For connected local evidence, the deployment-
  gated seeded persona selector may enter a seeded account, and the ordinary
  passkey path applies only after the learner account exists.
- Prove gradebook, item analysis, learner isolation, assignment creation after
  roster creation, and roster addition after assignment creation.
- Keep LTI Names and Roles roster synchronization in its separately authorized
  integration package; it must call the same Store command.
- Prove a deterministic manual gradebook export keyed by the protected
  course-scoped roster identifier; do not claim it changes the institutional
  system of record.

## Acceptance gates

Permanent behavior and contract tests must prove:

- Instructor, Sysadmin-without-membership, Student, nonmember, and
  foreign-tenant authorization outcomes;
- email-authentication and invitation secrets are single-use, bounded, hashed,
  rate-limited, and cannot cross account, browser binding, course, or expiry;
- existing and nonexistent email addresses have the same outward invitation
  and authentication response shape;
- one account may hold multiple passkeys, and revoking one credential does not
  revoke the others or expose credential metadata to course instructors;
- a changed or reassigned email cannot inherit another `UserId` or educational
  record;
- allowed-domain matching uses the complete normalized domain and rejects
  substring, suffix-confusion, and malformed-IDNA cases;
- course roster IDs are unique inside the course, absent from account lookup,
  and present in the intended manual grade export;
- Student membership cannot create Instructor or Sysadmin authority;
- adding a member creates the membership episode and profile without assignment
  activity rows;
- creating an assignment stores its audience without creating learner rows;
- pre-activity summary reads return `noActivity` without a write;
- concurrent first-event materialization creates exactly one enrollment and
  summary receipt;
- Memory and PostgreSQL implement the same idempotent behavior;
- a failed summary insert rolls back first-receipt enrollment creation;
- removal revokes access without deleting educational records;
- re-addition reuses the learner identity and existing activity;
- bulk preview is bounded and commit is revisioned and idempotent;
- raw roster bytes and expired previews are removed at their documented
  boundary;
- error bodies, exports, and logs exclude provider subjects, passkey metadata,
  invitation secrets, raw CSV cells, and unrelated learner data; and
- current gradebook reads remain empty until an educational event creates a
  summary.

Disposable integration evidence must prove:

- a real non-superuser PostgreSQL role and RLS context enforce the boundary;
- one API replica can create or redeem membership and another can serve the
  resulting course and assignment;
- an instructor can create and copy a learner invitation through the browser
  without SQL, `cargo tools e2e-seed`, or configured SMTP;
- the learner authenticates by email through the configured provider, optionally
  enrolls a passkey, enters the course, starts the
  assignment, submits, and appears in the instructor gradebook;
- the instructor downloads a protected manual grade export whose roster IDs
  match the imported rows and whose contents exclude account email and global
  `UserId`; and
- the deployment-gated seeded persona selector, PLE passwordless composition,
  optional OIDC/SAML connector, and future LTI adapter converge on the same
  `UserId`, session, and Store operation rather than implementing separate
  roster semantics.

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
4. Does membership remain sufficient to derive current assignment entitlement
   without creating learner activity rows?
5. Does only a bounded first event atomically create the enrollment, empty
   summary, grant basis, scopes, and provenance?
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
