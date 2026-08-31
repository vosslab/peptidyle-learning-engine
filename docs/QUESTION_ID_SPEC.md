# Question ID specification

## Purpose

PLE uses one short, human-usable Question ID for instructors to recognize, copy,
communicate, and enter. The Question ID is the visible identity of one stable
published question lineage. It is not a UUID, a sequence number, a credential,
or an authorization decision.

Each published `QuestionVersion` is immutable. A stable Question ID may therefore
have multiple exact versions without changing the identity that instructors use.
Assignments, issued work, and evidence retain their exact version pins; no
operation resolves an assignment through an implicit latest version.

## Format

The canonical Question ID is `AAA-BBBB`, using seven Crockford Base32 characters
in a `3-4` display grouping:

```text
7K3-M9QP
```

The first six characters are the random lineage identity. The seventh character
is a server-validated HMAC-SHA-256 check character. The hyphen is presentation
only and is not part of the stored identifier.

The identifier is non-sequential and copyable. The product may enforce its
independent 100,000,000-question limit without exposing creation order. Six
Crockford Base32 identity characters provide 32^6 possible values, so the
encoded namespace is larger than that product limit.

## Crockford alphabet

Use this Crockford Base32 alphabet:

```text
0123456789ABCDEFGHJKMNPQRSTVWXYZ
```

Canonical stored and displayed IDs use uppercase characters. Input parsing is
forgiving at the transcription boundary:

- Ignore the display hyphen.
- Accept lowercase and normalize to uppercase.
- Accept `O` or `o` as `0`.
- Accept `I`, `i`, `L`, or `l` as `1`.
- Reject every character outside the Crockford alphabet after normalization.

## Validation character

The validation character detects common transcription errors. It is not an
authentication, authorization, or existence proof.

Version 1 derives it as follows:

```text
identifier = six canonical Crockford Base32 identity characters
digest = HMAC-SHA-256(question_id_secret, identifier)
validation_value = the high five bits of digest byte zero
validation_character = CrockfordBase32(validation_value)
```

The HMAC input is the six uppercase ASCII identity characters without the
hyphen or a domain prefix. Stable vectors use this exact rule.

## Secret handling

The HMAC key is server-owned secret material. It never appears in browser code,
generated TypeScript, WebAssembly, logs, deployed public documentation, or
client configuration.

Browser validation may check syntax only. Server validation is authoritative.
Changing the deployed key would invalidate existing IDs, so key rotation and
recovery are application-state operations rather than ordinary configuration.

## Generation

The publish transition mints a Question ID only for a new published lineage:

1. Generate six random Crockford Base32 characters from a cryptographically
   secure source.
2. Reject a candidate already assigned to another lineage.
3. Derive its validation character.
4. Persist the Question ID with the new lineage and its first immutable version.
5. Never reassign an issued Question ID to another lineage.

On collision, generate another candidate. The product question-count limit is
enforced independently of the larger encoded namespace.

## Lineage and versions

One Question ID names one stable published lineage. A **Question Version** names
one immutable published meaning within that lineage and has one hidden internal
UUID. A draft has a private workspace identity, but no published
Question ID or published version.

The closed semantic change classes are:

- Presentation, accessibility, or metadata work that preserves grading meaning
  publishes a new immutable version in the same lineage.
- A compatible student-content improvement that preserves the objective, task,
  and Question Type publishes a new immutable version in the same lineage.
- A grading-semantic correction publishes a new immutable version in the same
  lineage, records its impact, and starts the required recalculation workflow.
- An incompatible objective, Question Type, task, or educational purpose is a
  fork. Publication of the fork creates a new Question ID and a new version.

**Moderate Edit** is available only to the Question Owner or original-lineage
steward. It publishes a new immutable Question Version in the same Question ID
lineage, preserves the original authorship, and retains the existing CC license.
Any approved (vetted) Instructor may use **Full Fork** on a published
version. Full Fork creates a private Draft Question with the Instructor's own
authorship, source attribution, and a source-compatible CC license. The draft
remains private to its creator's workspace until validation succeeds. Its
successful publication enters the one shared catalog with a new Question ID and
visible source/version ancestry. The fork author writes the independent fork.

**Question Change Proposal** is a separate contribution path. Any approved (vetted)
Instructor may submit a validated patch and rationale against one exact base
Question Version. The Question Owner accepts or rejects it. Acceptance creates a
new immutable version in the original Question ID lineage, preserves canonical
authorship and the existing CC license, and records contributor credit and
source history. Assignment and evidence pins remain fixed. A stale base requires
rebase and resubmission.

Semantic class is determined by meaning, not a byte-size threshold. Transport
limits protect request handling and do not decide whether a change is compatible.

## Exact pins and evidence

Every fixed Assignment Entry and Question Pool candidate pins one exact
Question Version by its internal UUID. The Question Version's parent
relationship relates that pin to its visible Question ID lineage; a browser-safe
projection may include the Question ID without creating a second exact-version
identity. An explicit, revision-checked
Assignment update may choose a new Available version. Publication, availability
changes, correction processing, and background work preserve the Assignment's
selected version.

Every Issued Question retains that exact Question Version pin and
selection evidence. Every Question Attempt retains its server-generated seed
and reproduction evidence. Grading evidence and audit records resolve the
same Question Version UUID rather than a mutable latest pointer. A Student
receives content only through server-authorized Assignment Access for that
exact pin.

Catalog metrics are keyed to the exact QuestionVersion. The version-specific
evidence family may contain accepted graded-attempt count, correct count, and
eligible choice-selection counts for supported Question Types. The
formula version and disclosure threshold travel with the safe rollup. Below the
threshold the projection reports insufficient evidence and contains only the
safe aggregate. Catalog metrics count accepted Student work; preview traffic and
Instructor Student View remain separate.

## Question Star and Question Watch

Star is one Account-owned endorsement per Question ID. Approved (vetted)
Instructors may see the Star count and the identities of approved Instructors
who starred. This is an Approved-Instructor projection.

Watch is a private Account-scoped in-app subscription for version, fork,
improvement, and impact notices. Each Account sees only its own watch list and watch
state. Exact stored relationships continue to supply course, Student,
workspace, publication, and grading authority.

## Forced Question Correction

Every published `QuestionVersion` remains immutable, including during an
emergency. A Sysadmin alone may approve a closed **Forced Question Correction**
after a validated replacement and privacy-safe impact manifest exist. The
manifest binds the flawed exact version, replacement exact version, reason
(`security_flaw` or `critical_correctness_flaw`), affected bindings, generation,
and deterministic remediation.

Approval atomically activates one authoritative version-to-version mapping.
New selection and issuance resolve to the replacement immediately. The
replacement normally is a new version in the same lineage; an incompatible
replacement follows the fork rule and carries its new Question ID in the
mapping. The flawed version remains immutable historical evidence and is never
edited or deleted.

Bounded, idempotent, generation-fenced workers materialize the mapping across
active reusable courses, course instances, assignments, pools, and future
issuance references. A deterministic work-impact check classifies in-progress
work for reissue or excuse. Issued and graded work keeps its original exact
pin; completed work receives an immutable superseding receipt and deterministic
recalculation when required. There is no per-course approval step.

Instructors receive only audited, course-authorized results. The Sysadmin impact
projection contains aggregate affected-version, assignment, and course counts,
manifest status, and exact version references, but no Student names, responses,
grades, private course-instance identity, or other FERPA-bearing records.

## Authorization boundary

Question IDs are public references, not bearer credentials. A valid ID does not
grant catalog access, reveal whether a question exists to an unauthorized
caller, establish ownership, or grant course or Student authority. Catalog
resolution requires an authenticated approved Instructor. Student delivery
requires the exact assignment entitlement. Anonymous callers cannot browse,
search, resolve, or inspect a Question ID.

## Display and entry

Display IDs in canonical uppercase `3-4` form and keep them visually subordinate
to the human-readable title. Copy actions copy the canonical form. Search and
entry controls accept both `7K3-M9QP` and `7K3M9QP`, then normalize to the
canonical display form.

## Required behavior

The implementation is complete when:

- one visible `AAA-BBBB` ID names each stable published lineage;
- each publication has an immutable Question Version and one hidden internal
  UUID;
- same-lineage and fork classes follow the semantic rules above;
- assignments, attempts, and evidence retain exact version pins;
- fork drafts remain private until validated publication and published forks are
  visible in the approved-Instructor catalog;
- Star projections are approved-Instructor-only and Watch state is private;
- version metrics are withheld until their formula-versioned threshold is met;
- correction mappings are Sysadmin-approved, atomic, bounded, and audited;
- equivalent Crockford input normalizes consistently and malformed input is
  rejected before authorized resolution; and
- the HMAC secret remains outside browser and WebAssembly boundaries.

## Related documents

- [PROBLEM_IDENTITY.md](PROBLEM_IDENTITY.md) defines hidden identity domains,
  lifecycle, attempts, and presentation-scoped IDs.
- [QUESTION_MODEL.md](QUESTION_MODEL.md) defines the answer-free question model,
  catalog projections, semantic changes, and correction boundary.
- [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) defines Account,
  membership, projection, and Sysadmin support authority.
- [API_CONTRACTS.md](API_CONTRACTS.md) maps these rules to routes and payloads.
