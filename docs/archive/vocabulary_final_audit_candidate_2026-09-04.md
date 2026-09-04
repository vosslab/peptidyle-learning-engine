# Vocabulary final audit - 2026-09-04

## Scope and review boundary

This record combines the code-cleaning manager's candidate evidence for
`VOCABULARY_REPLACEMENTS.md` with the terminology
reviewer's independent conclusion on the resulting repository tree.

`HUMAN_GUIDANCE.md`, `TERMINOLOGY_CONTRACT.md`, and `DESIGN_DECISIONS.md` were
read-only authorities throughout this pass.

## Checklist and plan reconciliation

- `rg -n '^\\| \\[ \\]' docs/VOCABULARY_REPLACEMENTS.md` returned no rows.
- The former QTI-feedback and authored-Answer-Explanation rows were removed
  because current QTI profiles refuse feedback-bearing imports and current PLE
  source already keeps accepted-response facts in Answer Key/Question Answer
  roles. Implementing QTI semantic mapping or a new authored Explanation field
  is a separately owned product capability, not an outstanding rename.
- Operative current-handoff and active-plan statements now distinguish a
  completed vocabulary boundary from the separately open product capability.
  Historical row-status receipts remain historical and do not allocate current
  vocabulary work. Open product capabilities remain with their owning package
  or plan.

## Current-state search evidence

`./_temp_vocabulary_counts.sh` was run at 2026-09-04 09:00 CDT. Its exact-owner
detectors are zero for retired External Tool/Question Provider, PLE Question
JSON file authoring, sole-current presentation V1, Question Attempt Source
Record, Question Version, generic Bloom, solution-free, unsupported seeded
Variation, and retired Question Folder forms. Its nonzero contextual terms were
inspected as follows:

| Search family                             | Current classification                                                                                                  |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Flat Question                             | Read-only authority or historical descriptive wording; mutable PLE source uses PLE Question JSON.                       |
| mounted/unmounted                         | Seven physical filesystem, volume, or container attachment uses; no application-capability use.                         |
| Question Seed                             | Six exact, subject-qualified generation values in Question Model/domain source.                                         |
| Private Question/Question Folder          | Exact lifecycle, disclosure, or Account-owned organization terminology.                                                 |
| Random Block, Question Set, Question Pool | Exact response-control, static collection, or Assignment selection terminology.                                         |
| `Id`/`Key`/`Version` suffixes             | Private identity, exact reference, cryptographic/platform key, or versioned contract vocabulary; not a raw-zero target. |

The final direct Question Asset Reference search confirmed `questionAsset`
through Question Model, PLE Question JSON, QTI conversion, generated
declarations, strict browser decoding, and rendering. The strict PLE JSON
reader rejects the retired `asset` member. Remaining asset names are private
QTI worker state, registered-format data, or technical delivery wording.

## High-reach semantic review

Graphify was consulted against the current map. `QuestionRevisionReference`
has 84 current connections across issuance, source resolution, adapter replay,
Question Presentation, Blueprint operations, and analysis. Its use remains an
exact immutable Question Revision locator. `Timestamp` and `StoreError` have
separate Question Model/Domain and Store-specific definitions respectively;
their ambiguous graph names were resolved by file rather than treated as one
overloaded product concept. No new PLE-owned naming gap was identified in this
connector review.

## Required gates

| Gate                                         | Result                                                                                                                                                                                                                                                                                  |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `source source_me.sh && ./all_test.sh`       | Passed independently on the final material tree; exit status 0. The aggregate regenerated 416 contracts, validated 3 tracked fixtures, passed Rust checks/strict Clippy/tests/doctests/Wasm, all 5 frontend checks including 290 Node tests, 4,878 Python tests, and both disposable PostgreSQL and PostgreSQL-plus-MinIO acceptance lanes. |
| Documentation/source-line and Markdown links | Passed within the final aggregate.                                                                                                                                                                                                                                                      |
| `git diff --check`                           | Passed after the final-tree aggregate.                                                                                                                                                                                                                                                  |
| Independent completion review                | **PASS.** The reviewer repeated the contextual inventory, inspected the operative plan corrections, and ran the complete final-tree gate.                                                                                                                                               |

## Conclusion

**PASS.** All 417 vocabulary replacement rows are complete. Current contextual
matches are registered/platform vocabulary, exact technical qualifiers, or
historical evidence. Operative plans distinguish completed vocabulary boundaries
from separately open product capabilities. Remaining Store, Server Route,
Browser Surface, QSOM1, and other feature work does not reopen these terminology
boundaries.
