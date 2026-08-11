# Input and exchange formats

This page maps the files PLE accepts or produces at its authoring, roster, and manual grade-export
boundaries. It is not a replacement for the authoritative schema, profile, or API contracts.

## Supported authoring inputs

| Input | Accepted surface | Boundary | Exact contract |
| --- | --- | --- | --- |
| PLE flat-question JSON | `application/vnd.peptidyle.flat-question+json` on the private flat-question source route | One v1 single-choice or v2 eight-family document, including answers and private feedback; at most 256 KiB | [QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) |
| Canvas QTI 1.2 ZIP | `application/zip` on the private QTI import route | At most 32 MiB; the original ZIP stays private while a worker creates an answer-free review report | [qti_profile_mapping_plan.md](active_plans/decisions/qti_profile_mapping_plan.md) |
| Blackboard QTI 2.1 ZIP | `application/zip` on the private QTI import route | At most 32 MiB; the original ZIP stays private while a worker creates an answer-free review report | [qti_profile_mapping_plan.md](active_plans/decisions/qti_profile_mapping_plan.md) |

The PLE JSON source is a Peptidyle contract, not a QTI variant. Its exact fields, validation rules,
canonicalization, source-to-public/private compilation boundary, and v1/v2 scope are in
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md). The public runtime model produced from any
accepted source is described separately in [QUESTION_MODEL.md](QUESTION_MODEL.md).

## WeBWorK source

PLE can publish a private immutable PG or PGML source artifact for its configured external
`webwork-pg-renderer`. This is an author-controlled source artifact, not a learner upload, a
browser-accessible renderer file, or a general-purpose WebWork2 import route. The only
live-accepted source shape is the licensed, user-authored PGML pilot with one `RadioButtons` group.
Its projection, grading, source-artifact handling, and exact compatibility boundary are defined in
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md). Broader PG controls and
Open Problem Library compatibility require separate source examples and live acceptance evidence.

## Roster CSV import

Course managers can preview and explicitly commit a UTF-8 CSV roster at the private roster-import
route. The current generic file has exactly these headers, in this order:

```csv
email,roster_id
student@example.edu,900123456
```

- The import accepts `text/csv`, at most one MiB and 500 data rows.
- `email` is the course invitation destination; `roster_id` is the protected course-scoped value
  used to match a later manual LMS or gradebook export.
- The preview normalizes and classifies rows before the instructor selects the rows to commit.
  PLE discards raw CSV bytes after staging the normalized preview.
- The generic parser does not make an email address an account key or a roster ID an authentication
  credential. A reviewed institution profile may map alternate headings and validate its documented
  roster-ID grammar.

The invitation, preview, atomic commit, retention, and privacy semantics are in
[ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md), not in the CSV grammar.

## Manual grade CSV export

For one course and assignment, a course manager can download a synchronous `text/csv; charset=utf-8`
file from the private grade-export route. The current output uses CRLF records and these headers:

```csv
roster_id,email,display_name,score
900123456,student@example.edu,Student Name,93
```

The response is an attachment with `Cache-Control: no-store`; PLE does not persist a grade-export
object. It contains only the course roster ID, course roster email, display label, and the selected
assignment score. It excludes global account IDs, passkey state, invitation secrets, and unrelated
course activity. [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md) and
[API_CONTRACTS.md](API_CONTRACTS.md) define its authorization, auditing, and retention boundary.

## Import handling

- Only an authenticated author with access to the target workspace may submit either source type.
- The flat-question route parses and canonicalizes the complete answer-bearing JSON before staging it
  as private source material.
- The QTI route requires the exact `application/zip` media type and rejects an empty or oversized
  archive before it is queued.
- QTI recognition is intentionally bounded to Canvas QTI 1.2 and Blackboard QTI 2.1. A recognized
  package can report accepted and rejected items together; unsupported features remain in the private
  review report instead of being silently discarded.
- A reviewed QTI item converts through the profile-to-native boundary. It does not expose its original
  archive or answer bindings to a learner-facing or public route.

## Formats not yet accepted

- YAML is not an input or output interface. It may later become a human-editing format that compiles
  once to canonical PLE JSON; until then, no YAML schema is defined.
- QTI-JSONL is not a current PLE upload format. A future accepted external contract may receive a
  versioned adapter, but PLE flat JSON v2 already owns native all-family source semantics.
- A generic browser route for arbitrary PG, PGML, or Open Problem Library imports is not a current
  contract. The configured private renderer accepts only the documented bounded source path.
