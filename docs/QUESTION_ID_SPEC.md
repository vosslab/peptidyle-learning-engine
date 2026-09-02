# Question ID specification

## Purpose

PLE uses one short, human-usable Question ID for instructors to recognize, copy,
communicate, and enter. The Question ID is the visible identity of one stable
published question lineage. It is not a UUID, a sequence number, a credential,
or an authorization decision.

Each published `QuestionRevision` is immutable. A stable Question ID may therefore
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

One Question ID names one stable published lineage. A **Question Revision** names
one immutable published meaning within that lineage and is identified by the
exact `QuestionRevisionReference { question_id, revision_number }` pair. Its
Question Revision Number is a positive monotonic integer assigned within that
Question lineage. A draft has a private workspace identity, but no published
Question ID or Question Revision Number.

An accepted same-lineage publication keeps the Question ID and assigns the next
Question Revision Number. Publication of a separate lineage mints a new Question
ID and starts that lineage at Version Number 1. The
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns the change-operation
meanings and [QUESTION_MODEL.md](QUESTION_MODEL.md) owns their typed model.

## Exact pins and evidence

Every fixed Assignment Entry and Question Pool Item pins one exact
`QuestionRevisionReference`. Browser-safe `AssignmentSummary` entries expose a
Question ID through `FixedQuestionAssignmentEntrySummary` and
`QuestionPoolItemSummary`, without exposing the server-owned exact-version
reference. An explicit,
revision-checked Assignment update may choose a new Available version.
Publication, availability changes, correction processing, and background work
preserve the Assignment's selected reference.

Every Issued Question retains that exact Question Revision Reference and
selection evidence. Every Question Attempt retains its server-generated seed
and reproduction evidence. Grading evidence and audit records resolve the
same exact pair. A Student receives content only through server-authorized
Assignment Access for that reference.

## Publication and availability

[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns the canonical lifecycle
meanings. Question Publication Requirements name the conditions for one Draft
Question Revision; Question Publication Validation returns its ordered Question
Publication Issues. A Question Publication Event creates the first Question Revision
in a new lineage. A Question Revision Availability Event records whether an immutable
revision is Available or Archived for ordinary selection.
Both availability values preserve exact historical resolution through the same
Question Revision Reference.

## Authorization boundary

Question IDs are public references, not bearer credentials. A valid ID does not
grant Question Library access, reveal whether a question exists to an unauthorized
caller, establish ownership, or grant course or Student authority. Question Library
resolution requires an authenticated active Instructor. Student delivery
requires exact Assignment Access for that Student and Assignment. Anonymous callers cannot browse,
search, resolve, or inspect a Question ID.

## Display and entry

Display IDs in canonical uppercase `3-4` form and keep them visually subordinate
to the human-readable title. Copy actions copy the canonical form. Search and
entry controls accept both `7K3-M9QP` and `7K3M9QP`, then normalize to the
canonical display form.

## Required behavior

The implementation is complete when:

- one visible `AAA-BBBB` ID names each stable published lineage;
- each publication has an immutable Question Revision identified by one exact
  Question Revision Reference;
- same-lineage publication advances the Version Number and a separate lineage
  receives a new Question ID;
- assignments, attempts, and evidence retain exact version pins;
- equivalent Crockford input normalizes consistently and malformed input is
  rejected before authorized resolution; and
- the HMAC secret remains outside browser and WebAssembly boundaries.

## Related documents

- [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines canonical Question,
  publication, availability, and stewardship terms.
- [IDENTITY_CONTRACTS.md](IDENTITY_CONTRACTS.md) defines internal record
  identifiers, relationship scopes, and human-facing references.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) defines Question
  Attempt, rendered-item, and presentation-consistency values.
- [QUESTION_MODEL.md](QUESTION_MODEL.md) defines the answer-free question model,
  Question Library browser results, semantic changes, and correction boundary.
- [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) defines Account,
  membership, browser-reader boundaries, and Sysadmin support authority.
- [API_CONTRACTS.md](API_CONTRACTS.md) maps these rules to routes and payloads.
