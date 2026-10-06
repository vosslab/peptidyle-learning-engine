# Library Object documentation audit

## Scope and conclusion

Published Questions and Question Pools were both part of the original design. The current code
structure does not fully reflect their intended integration as Library Objects. This is a gap
between specification and implementation, not evidence that Pools were a later product addition.

A Library search result represents a Library Object, with shared fields and kind-specific fields.
The audit follows that model through SQL, Rust, API decoding, browser rows, and Assessment selection.
This is source inspection of a changing working tree, not runtime acceptance or a complete
implementation-compliance audit. No application code was changed.

Authority: [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md), especially Question Pool specifications,
Question Library specifications, public IDs, and Assessment content. The user's current
clarification makes the shared Library Object result explicit.

## Important findings

### Search rules differ by kind

In [question_library_operations.sql](../../../schemas/base_schema/50_functions/question_library_operations.sql),
the combined query supports the common Question Type filter for both kinds. Its text-search
`question_type` branch and ordinary text matching of type names explicitly require
`row_kind = 'question'`. A Pool's own required Question Type therefore participates differently
depending on how the filter is expressed. Define matching once for the shared field and use it
consistently across supported search controls and syntax.

The earlier Course-use finding is withdrawn: Neil never approved "Used in my Courses"
and requested its removal. It is not a missing Pool behavior.
Pool exclusion from Author filters is different: HG explicitly gives Pools no Author field.

### Shared fields lose a common shape

[library_search.rs](../../../crates/question_model/src/library_search.rs) has a tagged result with
Question and Pool variants. However, it wraps two separate summaries without a shared field
contract. Question ownership sits outside its summary; Pool ownership sits inside its summary.

[question_library_repository.ts](../../../src/api/question_library_repository.ts) discards the
Question result's owner when building the browser row. It also omits Question Type, Backend,
and Tags from that row while retaining those fields for Pools. These facts were available earlier
in the result path. [library_page_model.ts](../../../src/pages/library_page_model.ts) then uses
`questionTitle`/`summary` for Questions and `title`/`description` for Pools. Shared rendering and
actions must repeatedly recover or branch around ordinary common fields.

Define the shared discovery fields once, with explicit required/optional rules, and carry them
through to consumers that need them. Keep Question Revision pins and Pool membership/Edit Number
in kind-specific data. Separate summary types are reasonable when they implement that boundary;
deleting or renaming `QuestionPoolLibrarySummary` alone would not fix it.

### Combined discovery stops at selection

[assessment_content_picker.tsx](../../../src/features/assessment_content_picker/assessment_content_picker.tsx)
shows both kinds, but selecting a Pool clears selected Questions, selecting a Question clears a
selected Pool, and selecting loaded results selects only Questions. Its return type permits many
Questions or one Pool. This is an implementation restriction to review against the intended
Assessment workflow, not a restriction established by having two object kinds. A combined search
does not by itself settle whether mixed batch import is required.

[library_bulk_actions.tsx](../../../src/pages/library_bulk_actions.tsx) likewise presents separate
Question and Pool editing actions. HG calls for bulk edits of shared Library metadata. Determine
which operations can share one user action while preserving each kind's ownership checks,
concurrency checks, and membership constraints. Do not make shared fields imply identical writes.

## Integration already present

- SQL combines both kinds before global filtering and paging; the search is not a browser merge
  of independently paged lists.
- Rust and browser decoders discriminate the result kind.
- Both use the shared search presentation and one Library detail URL, with authorized kind
  resolution in [library_object_detail_page.tsx](../../../src/pages/library_object_detail_page.tsx).
- Assessment selection preserves exact Question Revisions and Pool import identifiers/Edit Numbers.
- Kind-specific details remain necessary: a Published Question has immutable source Revisions;
  a Pool has current membership and an Edit Number, with selection delegated to PLE.

## Documentation problems

[QUESTION_SPECS/README.md](../../QUESTION_SPECS/README.md) combines the domain model, detailed Bloom API behavior,
implementation status, and pending verification. Its opening does not establish the common Library
Object model. Readers must reconstruct that model from later sections and other files.

[QTI-JSON_OBJECT_FORMAT.md](../../QTI-JSON_OBJECT_FORMAT.md) explicitly says the native format is
not QTI JSON, contrary to its filename. It repeats publication and identity rules and states that
feedback disclosure is independently configurable even though the feedback decision is deferred.
Native source rules need a clearly named home separate from interchange adapters.

[QUESTION_ID_SPEC.md](../../QUESTION_SPECS/QUESTION_ID_SPEC.md) describes the shared namespace inside a
Question-lineage-focused document. Its ID-generation details remain useful; the new overview
should make the shared namespace and different lifecycle counters obvious before those details.

## Proposed replacement structure

Start the explanation afresh from HG while retaining verified technical detail. Proposed filenames
below are a documentation outline, not newly established product requirements.

| Document | Owns |
| --- | --- |
| `QUESTION_MODEL.md` | Entry point: Library Object model, shared fields, two kinds, boundaries and links |
| `QUESTION_LIBRARY.md` | Shared metadata, combined result contract, search, filters, bulk operations, access |
| `PUBLISHED_QUESTIONS.md` | Draft, publication, ownership/authorship, Revisions, edits, forks, archive |
| `QUESTION_POOLS.md` | Membership, derived and Pool-owned metadata, licenses, mismatch, forks, selection |
| `NATIVE_QUESTION_JSON.md` | Internal source fields, examples, validation, rendering and grading by Question Type |
| `QUESTION_BACKEND_CONTRACTS.md` | Backend responsibilities, answer-free presentation, response/grading boundary |
| `QUESTION_ID_SPEC.md` | Shared public namespace, ID validation, exact Question Revision references |

Keep QTI interchange in a separate focused document when moving the native material. Link to the
existing Assessment and Student Work contracts for their behavior rather than copying them.

For each shared field, specify meaning, location (object, Revision, or Pool), who supplies it,
whether it is required, when it can be absent, and what changes affect. In particular: Question Type
is required; Bloom may await AI assignment without a deadline; Pool Author does not exist.

For each operation, state who can perform it, checks, resulting changes, and effect on existing
Assessments and Student Work. Put implementation gaps and test status in audit reports. Keep
unsettled product choices in decision records with the user's reason and confidence.

Rewrite one responsibility at a time, replace duplicate sections with links, and preserve HG as
the primary authority. A documentation rewrite does not authorize new behavior. Check local links
and verify every moved requirement against HG and the source document before retiring old content.
