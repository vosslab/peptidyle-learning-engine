# Question specifications

Published Questions and Question Pools are two kinds of Library Object in one Question Library.
This specification set defines their shared fields, different lifecycles, discovery, Assessment
use, execution, and import/export. Both kinds were part of PLE's original design.

Questions are backend agnostic. The workflow is: import or write a Draft, preview it, test it,
refine it, add the metadata, then publish. Start with
[DRAFT_QUESTION_SPEC.md](DRAFT_QUESTION_SPEC.md) for authoring.

## Authority and reading order

[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md) and [FALL_2026_PILOT.md](../FALL_2026_PILOT.md) remain
the primary authorities. These specifications expand settled rules into concrete fields and
operations. Existing code explains technical shapes; it does not decide product behavior when
HG conflicts or is silent. PLE is pre-production. Use PLE product vocabulary consistently across
schema, API, Rust, TypeScript, JSON, and specifications. Current parameter, field, and type names
are implementation evidence, not compatibility constraints. Rename them when needed to express
the intended model clearly. Record implementation changes in TODO during this documentation
pass. Repeated summaries link to the rule's owning specification.

Start with [LIBRARY_OBJECT_SPEC.md](LIBRARY_OBJECT_SPEC.md), then
[QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_LIBRARY_METADATA_SPEC.md),
[QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md), and [QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md).
Follow the workflow-specific documents below. Keep each coherent concept in one owning
specification, with current implementation evidence clearly separated from product guidance.

## Shared Library model

| Specification | Owns |
| --- | --- |
| [LIBRARY_OBJECT_SPEC.md](LIBRARY_OBJECT_SPEC.md) | Common model and the two object kinds |
| [QUESTION_LIBRARY_SPEC.md](QUESTION_LIBRARY_SPEC.md) | Access, common result facts, display, picker reuse, Stars/Watches, usage statistics |
| [QUESTION_LIBRARY_SEARCH_SPEC.md](QUESTION_LIBRARY_SEARCH_SPEC.md) | Search and Browse, text syntax, shared results, sorting, paging, navigation, and temporary search state; filter meaning belongs to the filter spec |
| [QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md) | Filter meaning by kind, Questions in no Pool, counts |
| [QUESTION_LIBRARY_BULK_EDIT_SPEC.md](QUESTION_LIBRARY_BULK_EDIT_SPEC.md) | Deferred Instructor workflow, intended semantics, unresolved mixed-save behavior, and current API evidence |

## Identity and Question lifecycle

| Specification | Owns |
| --- | --- |
| [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md) | Shared public namespace, canonical hyphen, public SHA-256 checksum, generated IDs |
| [QUESTION_REVISION_SPEC.md](QUESTION_REVISION_SPEC.md) | Immutable Question content and exact retained references |
| [DRAFT_QUESTION_SPEC.md](DRAFT_QUESTION_SPEC.md) | Private working content and publication readiness |
| [PUBLISHED_QUESTION_SPEC.md](PUBLISHED_QUESTION_SPEC.md) | Publication, allowed changes, availability and existing uses |
| [QUESTION_FORK_SPEC.md](QUESTION_FORK_SPEC.md) | A separate Question with its source attribution |

## Question Pools

[QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md) owns all Pool behavior: identity, which Questions it contains,
metadata, license, Assessment references and selection, explicit forks, problems, release checks,
and Student Work evidence. Published Question forks remain in
[QUESTION_FORK_SPEC.md](QUESTION_FORK_SPEC.md) because they use a Draft/Revision lifecycle.

## Shared metadata

| Specification | Owns |
| --- | --- |
| [QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_LIBRARY_METADATA_SPEC.md) | Authoritative field inventory, location, required/NULL/empty distinctions |
| [QUESTION_CLASSIFICATION_SPEC.md](QUESTION_CLASSIFICATION_SPEC.md) | Discipline, Subject, Topic, Subtopic, Tags, vocabulary choices |
| [QUESTION_BLOOM_CLASSIFICATION_SPEC.md](QUESTION_BLOOM_CLASSIFICATION_SPEC.md) | Values, missing assignment, and ordinary metadata correction |
| [QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md](QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md) | Owner, authors, attribution, and different edit authority |

## Backends and source formats

| Specification | Owns |
| --- | --- |
| [QUESTION_BACKEND_SPEC.md](QUESTION_BACKEND_SPEC.md) | PLE versus Backend responsibilities, responses, grading, saved state and failures |
| [QUESTION_TYPE_SPEC.md](QUESTION_TYPE_SPEC.md) | Required author/import-declared Question Type |
| [NATIVE_JSON_SPEC.md](NATIVE_JSON_SPEC.md) | Internal static JSON, per-Type source/response/grading, assets, allowed JavaScript |
| [MULTIPLE_ANSWER_SCORING_SPEC.md](MULTIPLE_ANSWER_SCORING_SPEC.md) | Native JSON MA partial-credit formula, choice-count score, worked examples, and Assessment control |
| [ORDER_SCORING_SPEC.md](ORDER_SCORING_SPEC.md) | Native JSON ORDER partial credit, equal position/pair weighting, examples, and required properties |
| [WEBWORK_SPEC.md](WEBWORK_SPEC.md) | PG/PGML, parameterized Questions, backend-owned presentation and state |

## Import, export, and APIs

| Specification | Owns |
| --- | --- |
| [QUESTION_IMPORT_SPEC.md](QUESTION_IMPORT_SPEC.md) | Import workflow, Drafts, publication, generated PLE identities, source mapping, readback, and current implementation evidence |
| [QUESTION_EXPORT_SPEC.md](QUESTION_EXPORT_SPEC.md) | Eligible content, format choices, attribution, loss |
| [QTI_INTERCHANGE_SPEC.md](QTI_INTERCHANGE_SPEC.md) | Format mapping and reuse of Rust QTI Package Maker |

[BIOLOGY_PROBLEMS_SPECS/README.md](../BIOLOGY_PROBLEMS_SPECS/README.md) owns source inventory and
base Course mappings. [BLUEPRINT_COURSE_IMPORT_API_SPEC.md](../BLUEPRINT_COURSE_IMPORT_API_SPEC.md)
owns reusable Course assembly through normal APIs. Imported content supplies source data and
relationships; PLE assigns PLE identities. QTI Package Maker is reused, not rebuilt here.

## External rule owners

- [ASSESSMENT_LIFECYCLE.md](../ASSESSMENT_LIFECYCLE.md) and
  [ASSESSMENT_PAYLOAD_DESIGN.md](../ASSESSMENT_PAYLOAD_DESIGN.md): release, issue, Attempts and scoring.
- [AUTHORIZATION_CONTRACTS.md](../AUTHORIZATION_CONTRACTS.md): authenticated roles and resource scope.
- [FERPA_DATA_POLICY.md](../FERPA_DATA_POLICY.md): Student Work and statistics privacy.
- [BLOOM_TAXONOMY_GUIDE.md](../BLOOM_TAXONOMY_GUIDE.md): teaching interpretation and Assessment sorting.
- [OBJECT_STORAGE.md](../OBJECT_STORAGE.md): private source and asset storage.
- [TERMINOLOGY_CONTRACT.md](../TERMINOLOGY_CONTRACT.md): PLE vocabulary, subordinate to HG.

## Open choices and evidence

[question_specs_open_questions.md](../active_plans/decisions/question_specs_open_questions.md)
collects uncertainty for the later grill, ordered by impact. It records what is unknown and why
it matters rather than silently completing missing product rules. The earlier
[HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md](../active_plans/decisions/HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md)
retains the reasons and strength of decisions already made.

Explicit deferrals remain in force: Instructor bulk editing, optional feedback timing, regrading after Native JSON answer
changes, initial AI/Bloom assignment, NC/ND support, later Backends, and undefined Sysadmin work.
Missing code for settled behavior belongs in the implementation gap report, not the decision queue.
These files specify intended behavior and concrete contracts; passing documentation checks does
not establish runtime compliance. Coverage, contradictions corrected, and verification are in
[question_specs_alignment_report.md](../active_plans/reports/question_specs_alignment_report.md).

The [settled-decision review](../active_plans/reports/QUESTION_SPEC_SETTLED_DECISIONS_REVIEW_2026_10_05.md)
traces strong requirements back to HG and separates implementation choices from product decisions.
