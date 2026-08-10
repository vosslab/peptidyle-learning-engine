# Input formats

This page identifies the authoring files accepted by Peptidyle Learning Engine. It is a format map,
not a replacement for the authoritative schema and profile contracts.

## Supported authoring inputs

| Input                  | Accepted surface                                                                         | Boundary                                                                                                   | Exact contract                                                                    |
| ---------------------- | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| PLE flat-question JSON | `application/vnd.peptidyle.flat-question+json` on the private flat-question source route | One v1 `singleChoice` document, including answers and private feedback; at most 256 KiB                    | [QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md)                            |
| Canvas QTI 1.2 ZIP     | `application/zip` on the private QTI import route                                        | At most 32 MiB; the original ZIP is retained privately while a worker creates an answer-free review report | [qti_profile_mapping_plan.md](active_plans/decisions/qti_profile_mapping_plan.md) |
| Blackboard QTI 2.1 ZIP | `application/zip` on the private QTI import route                                        | At most 32 MiB; the original ZIP is retained privately while a worker creates an answer-free review report | [qti_profile_mapping_plan.md](active_plans/decisions/qti_profile_mapping_plan.md) |

The PLE JSON source is a Peptidyle contract, not a QTI variant. Its exact fields, validation rules,
canonicalization, source-to-public/private compilation boundary, and v1 scope are in
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md). The public runtime model produced from any
accepted source is described separately in [QUESTION_MODEL.md](QUESTION_MODEL.md).

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
- QTI-JSONL is not a current PLE upload format. QTI Package Maker owns its specification and reference
  artifacts; PLE will adopt an accepted version through its versioned adapter/compiler plan.
