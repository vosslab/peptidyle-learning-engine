# Question ID specification

One Question ID identifies either one Published Question or one Question Pool. Both share a
public-ID namespace and lookup path; a value identifies one kind, never both. This document owns
the format and allocation rules. Authority:
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#human-facing-public-ids).

## Canonical representation

```text
XXXX-ZXXX
```

Each `X` is a cryptographically random character from the uppercase Crockford Base32 alphabet:

```text
0123456789ABCDEFGHJKMNPQRSTVWXYZ
```

`Z` marks the calculated check-character position; it is not a required literal letter.
The seven random characters provide `32^7 = 34,359,738,368` identities. The checksum adds no
identity space. The canonical string has eight alphabet characters and one hyphen, nine ASCII
bytes in total. The hyphen is part of the stored value, not a display decoration.

Store, transmit, hash, log, copy, and display that same uppercase hyphenated value across SQL,
Rust, JSON, URLs, object storage, and browser UI. Do not introduce an unhyphenated internal
Question ID or a separate display-ID transformation. IDs do not encode creation order, counts,
ownership, Question Type, Backend, or classification.

## Human entry and checksum

At a human-entry boundary, accept lowercase, `O`/`o` for `0`, `I`/`i`/`L`/`l` for `1`, and an
omitted hyphen. Normalize those accepted forms to the canonical string, then validate syntax
and checksum before lookup. Other persisted or transmitted boundaries use the canonical form.
Do not treat arbitrary punctuation or character substitutions as additional accepted aliases.

Calculate the checksum without a secret:

```text
identity_characters = the seven uppercase characters, excluding hyphen and checksum
digest = SHA-256(ASCII(identity_characters))
checksum_index = digest[0] >> 3
checksum_character = CrockfordAlphabet[checksum_index]
question_id = first_four + "-" + checksum_character + last_three
```

This is public unsalted SHA-256, not HMAC. Browser and server use the same calculation. A valid
checksum detects some typing errors; it proves neither existence nor permission. Authorization
still applies after validation. The ID notation above is schematic, not a checksum-valid fixture.

## Creation and reservation

PLE generates public IDs. Callers supply content and relationships, not a preassigned public ID
for a new object. A newly published Question has its own ID and Revision 1. Creating or forking a Pool creates its
own ID. A Question fork also receives a new ID. HG does not specify the allocation moment for a
private Draft; the existing fork command reserves the ID at Draft creation. Library entry still
requires publication. See [QUESTION_FORK_SPEC.md](QUESTION_FORK_SPEC.md).

1. Generate seven characters from a cryptographically secure source.
2. Calculate and insert the check character and hyphen.
3. Reserve the full canonical ID in the global public-ID registry for the new Question
   or Pool. Reserving a Question identity does not publish its Draft.
4. Retry a confirmed random-ID collision. Preserve other failures as failures rather than
   interpreting them as collisions.

All public-ID object types use the global uniqueness boundary. The shared Question/Pool namespace
particularly prevents the same `XXXX-ZXXX` value from naming both. Never reuse an issued ID,
including after archival or deletion. A second identity-only uniqueness key is unnecessary when
the full ID is minted and reserved correctly.

The publication storage contract writes source to a server-created address and handles the
registry's PostgreSQL `QP001` collision signal. Only a conclusive collision permits deleting that
just-written candidate object before retry. An uncertain database/object-store outcome is not
evidence that nothing committed; do not delete potentially committed evidence as collision cleanup.
See [OBJECT_STORAGE.md](../OBJECT_STORAGE.md) for storage responsibilities.

## IDs and change counters

| Action | Question ID result | Counter result |
| --- | --- | --- |
| Save a private Draft | Remains private; a reserved Question ID does not establish publication | Draft Edit Number checks current state |
| Publish new Question | New ID | Question Revision 1 |
| Publish next Question Revision | Same ID | Next positive Question Revision Number |
| Edit Question search metadata | Same ID | Same Revision Number; ordinary record Edit Number where needed |
| Publish Question fork | Its own ID, distinct from source; may already be reserved | Revision 1 in new Question |
| Create Pool or Pool fork | New ID | Initial positive Pool Edit Number |
| Save changes to the set of Question Revision Tuples in a Pool | Same ID | Pool Edit Number advances; no Pool Revision history |
| Archive Question | Same ID, never reassigned | Availability changes independently of content |

An exact Published Question reference combines its ID and positive Question Revision Number in
`PublishedQuestionRevisionTuple`. A Pool ID and Pool Edit Number identify the Pool selection
context; they do not provide earlier sets of Question Revision Tuples for a Pool. The delivered Question
Revision remains the grading evidence. Details belong to
[QUESTION_REVISION_SPEC.md](QUESTION_REVISION_SPEC.md) and
[QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md).

## Lookup and display

Resolve the authorized Library Object kind before opening its detail view. Use the same canonical
ID in the shared `/library/{questionId}` link for either kind. Ordinary current discovery and exact
historical Question Revision access are different operations; an archived Question can leave
search without breaking authorized exact-reference access.

Show a human-readable Title first and its public ID second. Copy actions copy only the canonical
ID. Internal UUIDs and database keys remain implementation details, not visible or copyable
content identities. Code names such as `PublishedQuestionId` do not make Pools secondary or create
a second public namespace.

Assessments, Pools, and Student Work retain exact Question Revision Tuples. A new
publication never means "resolve latest" for an existing reference. Question ID validity and
presentation-consistency checks do not authorize grading or Student access.

## Related boundaries

- [IDENTITY_CONTRACTS.md](../IDENTITY_CONTRACTS.md): other PLE IDs and internal identities.
- [QUESTION_IMPORT_SPEC.md](QUESTION_IMPORT_SPEC.md): creation returns generated IDs.
- [AUTHORIZATION_CONTRACTS.md](../AUTHORIZATION_CONTRACTS.md): actor and resource access.
- [ASSESSMENT_PAYLOAD_DESIGN.md](../ASSESSMENT_PAYLOAD_DESIGN.md): exact delivery evidence.
