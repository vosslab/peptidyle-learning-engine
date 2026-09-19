# Question ID specification

## Purpose

PLE uses one short, human-usable Question ID for Instructors and Sysadmins to
recognize, copy, communicate, and enter. It is the canonical human-facing
identity of one stable published Question lineage. A Question ID is not a UUID,
sequence number, or credential.

Each published `QuestionRevision` is immutable. A stable Question ID may therefore
have multiple exact versions without changing the identity that instructors use.
Assessments, issued work, and evidence retain their exact Revision pins; no
operation resolves an Assessment through an implicit latest Revision.

## Format

`XXXX-ZXXX` is the canonical Question ID form everywhere, including
PostgreSQL, object storage, serialization, and the browser. Its `4-4` grouping
and hyphen make the value immediately recognizable as a Question ID:

```text
XXXX-ZXXX
```

The four characters before the check character and the three characters after
it are the seven random lineage identity characters. In ID format notation,
`X` denotes a cryptographically random Crockford Base32 character and `Z`
denotes the embedded calculated checksum character. Both are stored characters;
`Z` is not a literal required character or separate metadata. The hyphen is
part of the canonical Question ID.

The identifier is non-sequential and copyable. Seven Crockford Base32 identity
characters provide 32^7 = 34,359,738,368 possible identities without exposing
creation order; the checksum adds no identity space.

## Crockford alphabet

Use this Crockford Base32 alphabet:

```text
0123456789ABCDEFGHJKMNPQRSTVWXYZ
```

Question IDs use the uppercase ASCII Crockford Base32 alphabet and canonical
`4-4` hyphen position. Store, transmit, display, copy, and generate only that
canonical form.

## Human entry

Human-entered Question IDs may use lowercase Crockford characters, `O` or `o`
for `0`, `I`, `i`, `L`, or `l` for `1`, and may omit the Question-ID hyphen.
Normalize only those accepted entry forms to canonical uppercase hyphenated
form, then validate canonical syntax and checksum before lookup. No other
translation or reformatted representation crosses the input boundary.

## Validation character

The validation character detects common transcription errors. It is not an
authentication, authorization, or existence proof.

The check character is derived as follows:

```text
checksum_input = ASCII bytes of the seven uppercase Crockford Base32 identity characters
digest = SHA-256(checksum_input)
validation_value = the high five bits of digest byte zero
validation_character = CrockfordBase32(validation_value)
```

The SHA-256 input is the ASCII bytes of the seven uppercase identity characters
in Question-ID order. Calculation excludes only the hyphen and checksum
position: `XXXX-ZXXX` supplies `XXXXXXX`. The derived character is the first
character after the hyphen; the canonical Question ID always retains both the
hyphen and checksum.

## Checksum handling

The checksum uses public unsalted SHA-256. It detects typos only; it is not an
authentication, authorization, or existence proof.

Validate the checksum whenever a Question ID is entered, without changing the
canonical value. Browser and server validation use the same public calculation.

## Generation

The publish transition mints a Question ID only for a new published lineage:

1. Generate seven random Crockford Base32 characters from a cryptographically
   secure source.
2. Derive the validation character and insert it in the documented middle
   position.
3. Copy the immutable source bytes to their server-created target address and
   atomically persist the Question ID with its new lineage and first immutable
   version.
4. Never reassign an issued Question ID to another object, including after
   deletion or archival.

Every public ID is globally unique across every public-ID object type. Published
Questions and Question Pools share the `XXXX-ZXXX` namespace because one lookup
path resolves either object; a value identifies one of them, never both. The
allocator reserves the full canonical Question ID in the global public-ID
namespace while atomically persisting its lineage and first immutable Revision.
The base schema checks the exact uppercase `XXXX-ZXXX` shape; every
identifier-entry boundary verifies its embedded checksum before database lookup
or resolution. Correctly minted full IDs already uniquely determine their
seven-character identity, so the schema does not need a second identity-only
uniqueness constraint.

The publisher retries the shared public-ID registry's PostgreSQL `QP001`
collision signal. That signal covers a conflicting Question or Question Pool
reservation in the shared public-ID namespace. It deletes the just-written
target object before that conclusive retry; another database or object-store
outcome is reported without treating it as an ID collision or deleting
potentially committed evidence.

## Lineage and versions

One Question ID names one stable published lineage. A **Question Revision** names
one immutable published meaning within that lineage and is identified by the
exact `QuestionRevisionTuple { question_id, revision_number }` pair. Its
Question Revision Number is a positive monotonic integer assigned within that
Question lineage. A draft has a private workspace identity, but no published
Question ID or Question Revision Number.

An accepted same-lineage publication keeps the Question ID and assigns the next
Question Revision Number. Publication of a separate lineage mints a new Question
ID and starts that lineage at Question Revision Number 1. The
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns the change-operation
meanings and [QUESTION_MODEL.md](QUESTION_MODEL.md) owns their typed model.

## Exact pins and evidence

Every fixed Assessment entry and Question Pool item pins one exact
`QuestionRevisionTuple`. Current browser-safe `AssignmentSummary` entries expose a
Question ID through legacy `FixedQuestionAssignmentEntrySummary` and
`QuestionPoolItemSummary`, without exposing the server-owned exact-version
reference. Those implementation type names do not preserve Assignment as the
generic product term. An explicit,
Edit-Number-checked Assessment update may choose a newer Published Question Revision.
Publication, availability changes, correction processing, and background work
preserve the Assessment's selected reference.

Student Work retains that exact Question Revision Tuple and selection
evidence. A backend that uses randomization retains its server-generated seed
or opaque state; native PLE Question JSON is static and receives no random
seed. The grading outcome resolves the same exact Revision. A Student receives
content only through server-authorized Assessment access for that reference.

## Publication and availability

[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) owns the canonical lifecycle
meanings. Question Publication Requirements name the conditions for one Draft
Question; Question Publication Validation returns its ordered Question
Publication Issues. A Question Publication Event creates the first Question Revision
in a new lineage. Current lineage metadata records whether the Published
Question is Archived for ordinary browsing and new selection. Either condition
preserves exact historical resolution through the same Question Revision Tuple.

## Display and entry

Display IDs in canonical uppercase `4-4` form and keep them visually subordinate
to the human-readable title. Copy actions and entry controls use only the
canonical `XXXX-ZXXX` value and validate its embedded checksum.

## Required behavior

The implementation is complete when:

- one visible `XXXX-ZXXX` ID names each stable published lineage;
- each publication has an immutable Question Revision identified by one exact
  Question Revision Tuple;
- same-lineage publication advances the Question Revision Number and a separate lineage
  receives a new Question ID;
- Assessments, Attempts, and evidence retain exact Revision pins;
- only the canonical `XXXX-ZXXX` value is accepted at every boundary, and its
  embedded checksum is validated whenever it is entered.

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
