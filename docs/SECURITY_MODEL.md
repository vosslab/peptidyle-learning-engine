# Security model

This document applies [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) to trust,
authorization, Question Backends, FERPA records, and browser boundaries. It
does not infer product features from current tables, workers, routes, or old
plans.

## Trust model

Treat browsers, URLs, form fields, JSON, uploaded archives, imported Questions,
backend documents, callback payloads, cache entries, queue messages, and object
paths as untrusted input. The server derives authority from an authenticated
Account plus the exact stored relationship required by the operation.

Each Account has one immutable Product Role: Student, Instructor, or Sysadmin.
Course relationships supply Course authority. Product Role, object possession,
or a visible ID never substitutes for that relationship.

## Authorization boundaries

### Student

A Student may access only Courses with an active Student relationship and only
that Student's own Coursework, Assessment Attempts, saved responses, submitted
work, and disclosed results. Removing Course access or deactivating the Account
blocks new access without deleting Student Work.

### Instructor

All current co-Instructors have equal authority within the Course Instance. The
creator or first Instructor has no greater access. Teaching actions and FERPA
reads are limited to the exact Course relationship.

### Sysadmin

A Sysadmin administers the platform but has no ambient Course membership or
FERPA access. Support access to Student records is deliberate, narrowly scoped,
and recorded. Creating or configuring a Course does not silently give the
Sysadmin an Instructor relationship.

### Future Course roles

Course Observer, Student Observer, and Grader are future Course relationships.
They have no authority until their capability, consent/disclosure, and privacy
contracts are explicitly implemented. Grader is not currently needed because
Question grading is automatic.

## Database enforcement

Protected PostgreSQL operations use server-installed Account context, exact
Course/workspace/Blueprint relationships, forced RLS, and least-privilege
roles. Security-definer functions are narrow, have a fixed trusted search path,
are unavailable to `PUBLIC`, and recheck the exact target.

Foreign protected records use non-enumerating failures where revealing
existence would leak information. A route check and database check defend the
same relationship; neither trusts the other layer's caller-supplied ID.

See [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md).

## Question authoring and publication

A Draft Question is private, mutable, unpublished, and unversioned. Its source,
Answer Key, private feedback, provider configuration, and preview remain inside
the authorized workspace.

Publication validates server-held source and creates or advances a stable
Published Question lineage with immutable Question Revisions. Published
answer-free content may be discovered by vetted Instructors. Private source,
Answer Keys, grading inputs, workspace identifiers, and credentials remain
private.

A Published Question ID is a human reference, not access authority. Question
Archive is a high-consequence owner action and must not erase immutable Revision
evidence used by Assessments or Student Work.

## Question Backend boundary

The selected Question Backend is the only authority for its render, controls,
response interpretation, grading, feedback, and backend-specific state. PLE
must not parse backend-owned controls or trust browser claims about Question
Type, source, correct answers, or credit.

PLE supplies trusted source, exact Revision, and randomization/backend state.
It receives an answer-free presentation plus opaque state, then later a credit
fraction and protected feedback. The fraction is immutable. PLE calculates
scores from current Assessment Question point values without regrading.

Backend failure never becomes an incorrect Student response. A backend-specific
continuation, callback, token, or signature remains server-only and scoped to
the exact Student, Course, Attempt, Question Revision, and operation. It cannot
create a public grading queue, Retry action, mutable result, or second Attempt
lifecycle.

## Native PLE Question JSON

The native format is private, unpublished, unversioned, static, and strictly
validated. Server grading is authoritative. Author JavaScript runs in an
isolated untrusted context with no session credential, Answer Key authority,
same-origin application access, storage authority, or network authority beyond
the deliberately supplied sandbox.

Strict decoders reject unknown fields, invalid types, unsupported controls,
unsafe URLs, and out-of-bound data. Browser validation improves feedback but
does not replace server validation.

## Assessment Attempt boundary

The server binds each operation to:

```text
authenticated Account
  -> active Student Course relationship
  -> Student record
  -> released Assessment
  -> open Assessment Attempt
  -> Question position and exact Revision/backend state
```

A complete response can be saved and replaced while the Attempt is open. An
incomplete response is not saved as complete or graded. Saving changes only the
working response and exposes no Student-visible grading outcome.

The Student submits the whole Assessment Attempt, or the deadline submits it
automatically. That action finalizes all saved responses together. Other
positions remain visibly unanswered, receive zero credit, and count as
incorrect without being sent to a backend. Repeating submission is idempotent. After
submission, responses and credit fractions are immutable.

Protected Attempt routes return only the answer-free presentation, saved-state
information, the Student's response, authoritative timing, and results or
feedback permitted by policy. They never return Answer Keys, private grading
inputs, backend credentials, worker state, or raw provider output.

## Assessment Unrelease

Unrelease requires current equal co-Instructor authority, Released state, and
typed confirmation of the exact Assessment title. It atomically changes the
Assessment to Unreleased and deletes all Student Work for that Assessment while
preserving its definition, Course relationships, and shared Published content.
Failure leaves both state and Student Work unchanged.

## Blueprint boundary

Private Blueprints are owner-only and cannot be adopted. Public Blueprints are
visible to vetted Instructors and adoptable. Archived Blueprints are read-only,
excluded from ordinary discovery and new adoption, available only through
explicit archived inclusion, and forkable.

Only the owner saves or changes lifecycle state. Public can return to Private
only before any adoption. Archived restores to Public. Blueprint content has no
Students, dates, time zones, or relative schedules.

Newer Blueprint Revisions are offered to daughter Course Instructors for
review; existing Assessment changes are never silently applied. A newly added
Blueprint Assessment is automatically copied as an Unreleased Assessment, but
that system action gives the Blueprint owner no access to daughter Course or
Student records. A Blueprint Course Change Proposal is accepted only by the
receiving Blueprint owner and never changes a daughter Course directly.

## Import and backend-document boundaries

QTI and other imports are hostile archives. Enforce size/count/path/media
bounds, reject traversal and external entity/network resolution, and convert
only supported items. Imported runtime Questions become native Draft Questions;
the archive does not remain a second runtime model.

WeBWorK and similar documents remain opaque. The browser may receive one
authorized document in an isolated frame and return bounded ordered form data.
PLE does not inspect hidden fields or controls to reproduce backend semantics.
Credentials, renderer URLs, source bytes, cookies, and raw grading output remain
private.

## Object and asset boundary

The database owns logical object identity and scope. The server constructs
typed storage keys, verifies checksums and media types, and returns only
authorized bytes or short-lived delivery results. A bucket name, prefix, object
ID, checksum, or signed URL is not durable authorization.

Only answer-free published assets are eligible for shared delivery. Draft
source, Answer Keys, backend state, Student responses, grades, and FERPA records
remain in private scopes.

## Retention boundary

The Course Instance becomes Inactive six months after creation. That limit
prevents Course reuse or deadline extensions from indefinitely delaying FERPA
retention and deletion, but becoming Inactive does not itself delete Student
records. The latest Assessment deadline starts the FERPA retention clock; it
does not itself archive or remove Student data. The configured policy later
sends notice, removes FERPA-protected records from normal interfaces, preserves
them during recovery, and permanently deletes them. Course metadata, Assessment
definitions, Questions, and settings remain.

The processing pass is idempotent and cannot report a stage that did not
complete. Backup and operational-log policy must not become an undeclared
Student-record archive. See [RETENTION_POLICY.md](RETENTION_POLICY.md).

## Browser, cache, export, and diagnostics

- Use secure, HttpOnly, host-bound session cookies and same-origin protected
  requests.
- Keep credentials, private source, Answer Keys, responses, grades, backend
  state, and FERPA payloads out of URLs, persistent browser storage, logs,
  traces, screenshots, and generic analytics.
- Use `no-store` for protected Student and Assessment responses.
- Cache only answer-free immutable data under exact keys; a cache never
  authorizes, grades, submits, or extends an Attempt.
- Exports require an explicit field allowlist, exact authorization, and a
  retention owner. Human Guidance does not define an Assessment Export service.
- Browser errors are bounded and accessible without exposing protected details.

## Change control

A new security-sensitive path names its authenticated principal, exact stored
relationship, protected data class, browser projection, storage scope,
retention behavior, failure behavior, and boundary-level acceptance evidence.
Do not add generalized audit, snapshot, recovery, background-worker, or
compatibility machinery without a concrete Human Guidance-compatible need.
