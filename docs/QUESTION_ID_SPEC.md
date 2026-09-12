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

The canonical stored Question ID is seven compact Crockford Base32 characters.
Its canonical browser display is `AAA-BBBB`, using a `3-4` grouping:

```text
7K3-M9QP
```

The compact stored form of this example is `7K3M9QP`. The first six characters
are the random lineage identity. The seventh character is a server-validated
HMAC-SHA-256 check character. The hyphen is presentation-only and is not part
of the stored identifier.

The identifier is non-sequential and copyable. Six Crockford Base32 identity
characters provide 32^6 possible identities without exposing creation order.

## Crockford alphabet

Use this Crockford Base32 alphabet:

```text
0123456789ABCDEFGHJKMNPQRSTVWXYZ
```

Canonical stored and displayed IDs use uppercase characters. Input parsing is
forgiving at the transcription boundary:

- Accept either the compact seven-character form or one hyphen in the canonical
  `3-4` display position.
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
hmac_output = HMAC-SHA-256(question_id_secret, identifier)
validation_value = the high five bits of hmac_output byte zero
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
2. Derive its validation character.
3. Copy the immutable source bytes to their server-created target address and
   atomically persist the Question ID with its new lineage and first immutable
   version.
4. Never reassign an issued Question ID to another lineage.

`published_question.question_id` is the sole database uniqueness boundary for
Question ID allocation. The base schema checks the compact uppercase Crockford
shape; the trusted server verifies the HMAC character before identifier-based
resolution. Correctly minted full IDs already uniquely determine their
six-character identity for one deployment secret, so the schema has no
redundant first-six-character uniqueness constraint.

The publisher retries only a PostgreSQL `23505` violation of that primary key.
It deletes the just-written target object before that conclusive retry; another
database or object-store outcome is reported without treating it as an ID
collision or deleting potentially committed evidence.

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
in a new lineage. A Question Availability Event records the stable lineage's current
Available or Archived state for ordinary browsing and new selection. Either state
preserves exact historical resolution through the same Question Revision Reference.

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
  Attempt, Presentation Response Item Reference, and presentation-consistency values.
- [QUESTION_MODEL.md](QUESTION_MODEL.md) defines the answer-free question model,
  Question Library browser results, semantic changes, and correction boundary.
- [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md) defines Account,
  membership, browser-reader boundaries, and Sysadmin support authority.
- [API_CONTRACTS.md](API_CONTRACTS.md) maps these rules to routes and payloads.
