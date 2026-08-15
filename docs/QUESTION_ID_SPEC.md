# Question ID specification

## Purpose

This specification records a human-directed product decision developed by the repository owner with ChatGPT. It is the guide going forward for question identity and question reuse in PLE.

PLE uses one short, human-approachable Question ID for instructors to recognize, copy, communicate, and enter. Public Question IDs are not sequential numeric IDs and are not UUIDs. Internal UUIDs may remain hidden where useful for persistence or implementation, but instructors should neither see nor use them as question identities.

One Question ID names one immutable published question. Every authored content change, including a
bug fix, publishes a new Question ID and fresh hidden publication evidence. An optional one-way
provenance link may identify the source question without changing it or its assignments.

## Format

A Question ID contains seven Crockford Base32 characters, displayed in a `3-4` grouping:

```text
7K3-M9QP
```

The first six characters identify the question. The seventh character is a validation character derived from HMAC-SHA-256. The display format does not distinguish the validation character from the rest of the ID.

Canonical form:

```text
AAA-BBBB
```

where all characters use the Crockford Base32 alphabet.

The hyphen is presentation-only and is not part of the underlying identifier.

## Namespace

PLE supports at most 100,000,000 human-facing Question IDs.

Six Crockford Base32 characters provide 32^6 = 1,073,741,824 possible values, which provides substantially more namespace than the intended product limit.

Question IDs should be assigned randomly from this six-character space without exposing creation order. Sequential numbering is not part of the public identifier contract.

For context, ADAPT uses sequential numeric question IDs and had reached approximately question `#345849` by August 2026. That scale remains far below PLE's 100,000,000-question design cap. PLE deliberately does not adopt ADAPT's sequential public numbering because the PLE identifier is intended to be compact, copiable, resistant to guessing neighboring questions, and protected against common transcription errors.

## Crockford Base32 alphabet

Use the Crockford Base32 alphabet:

```text
0123456789ABCDEFGHJKMNPQRSTVWXYZ
```

The alphabet intentionally excludes `I`, `L`, `O`, and `U`.

Canonical stored and displayed IDs use uppercase characters.

Input parsing should be forgiving:

- Ignore the display hyphen.
- Accept lowercase and normalize to uppercase.
- Accept `O` or `o` as `0`.
- Accept `I`, `i`, `L`, or `l` as `1`.

After normalization, reject any character outside the Crockford Base32 alphabet.

## Validation character

The seventh character is derived from HMAC-SHA-256 using a server-owned secret.

The validation character is intended to detect common transcription errors and reject arbitrary malformed IDs before database lookup. It is not an authentication or authorization mechanism.

The fixed version 1 extraction rule is:

```text
identifier = six canonical Crockford Base32 characters
digest = HMAC-SHA-256(question_id_secret, identifier)
validation_value = the high five bits of digest byte zero
validation_character = CrockfordBase32(validation_value)
```

The HMAC input is the six uppercase ASCII identifier characters with no hyphen or domain prefix.
Stable test vectors cover this rule so every server implementation produces the same result.

A single validation character provides 32 possible values. An arbitrary incorrect identifier therefore has a 1/32 probability of passing validation by chance.

## Secret handling

The HMAC key is server-owned secret material.

The key must not be included in browser code, generated TypeScript contracts, WebAssembly, logs, public documentation containing deployed values, or client-visible configuration.

Question ID validation performed by the browser may check syntax only. Validation of the HMAC character is authoritative on the server.

Changing the HMAC key would invalidate existing Question IDs. The deployed key must therefore be treated as durable application state and included in the production secret-management and recovery design.

## Generation

When creating a new Question ID:

1. Generate a candidate six-character Crockford Base32 identifier from a cryptographically secure random source.
2. Confirm that the candidate identifier is not already assigned.
3. Derive the HMAC-SHA-256 validation character.
4. Persist the resulting Question ID with the question identity.
5. Never reassign a previously issued Question ID to a different question.

Collision handling should generate another candidate rather than modifying an existing Question ID.

The 100,000,000-question product cap must be enforced independently of the larger encoded namespace.

## Question identity and authored changes

One Question ID identifies one immutable published question across its lifetime. PLE does not
maintain an update-in-place, successor, or version-selection path for it.

An author publishes a new Question ID for every content change. The new question may retain
explicit provenance to its source for attribution and licensing, but provenance does not grant
authority over the source or advance an assignment.

Internal identifiers, snapshots, audit evidence, or immutable records may still exist where required for grading history, reproducibility, security, or persistence. These mechanisms are hidden implementation details and do not create multiple human-facing versions of the Question ID.

## Instructor workflows

Question IDs are intended for occasional direct lookup and communication, not as the primary
mechanism for organizing groups of questions.

An instructor should be able to import or copy an entire assignment, or select a checklist of
questions from an existing assignment, rather than reconstructing an assignment through Question
ID ranges or repeated manual ID entry. When an Instructor deliberately changes an assigned question,
the editor shows the existing and replacement Question IDs and requires a revision-checked
replacement. Issued work retains its original exact evidence; future runs use the selected replacement.

When an instructor enters a Question ID, the interface should resolve it to recognizable question information before an irreversible or significant action. A syntactically valid ID or valid HMAC character does not establish authorization or prove that the intended question was selected.

## Security boundary

Question IDs are public-facing references, not bearer credentials.

A valid Question ID:

- does not grant access to a question;
- does not bypass tenant, course, role, or publication authorization;
- does not establish ownership;
- does not prove that a question exists to an unauthorized caller.

All normal PLE authorization and disclosure rules continue to apply after Question ID validation.

## Display

Display Question IDs in canonical uppercase `3-4` form:

```text
7K3-M9QP
```

Use a monospace or otherwise highly distinguishable treatment where appropriate for copy-oriented controls, while keeping the ID visually subordinate to the human-readable question title.

Copy actions should copy the canonical form.

Search and entry controls should accept both:

```text
7K3-M9QP
7K3M9QP
```

and normalize them to the canonical display form.

## Required behavior

The implementation is complete when:

- generated Question IDs use six random Crockford Base32 identifier characters plus one HMAC-SHA-256 validation character;
- IDs display canonically as `AAA-BBBB`;
- equivalent Crockford input forms normalize consistently;
- malformed or checksum-invalid input is rejected before question resolution;
- valid IDs resolve only through normal authorized server paths;
- collisions cannot assign one Question ID to multiple questions;
- issued Question IDs remain stable;
- the HMAC secret remains outside browser and WebAssembly boundaries;
- deterministic test vectors prove generation and validation behavior;
- the product enforces the 100,000,000-question limit independently of the encoded namespace.

## Migration from the current design

This specification replaces the current instructor-facing `P-<number>-v<version>` identifier design and the assumption that instructors work with multiple published versions of one question.

The manager should identify all documentation, schemas, generated contracts, API routes, browser controls, tests, fixtures, and content workflows that currently treat sequential IDs or versioned question IDs as the human-facing contract.

The resulting product uses one Crockford Base32 Question ID per immutable published question. Every
content change, including a bug fix, publishes a new Question ID. An Instructor deliberately uses
a revision-checked assignment-item replacement when future runs should use that new question;
issued work retains its original exact evidence. Assignment-level or checklist-based reuse remains
the preferred way to reuse collections of questions. Hidden internal identity and historical
evidence remain available where required to preserve grading records, provenance, reproducibility,
and security.
