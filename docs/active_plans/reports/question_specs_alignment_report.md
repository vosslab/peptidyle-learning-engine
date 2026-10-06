# Question specification alignment report

See [QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md](QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md) for
the later audit of unsupported behavior and remaining contradictions. This earlier alignment
report is not evidence that every expanded rule has human approval.

## Scope and authority

This report records the 2026-10-05 documentation rewrite authorized by Neil. It separates document
coverage, corrected contradictions, remaining implementation gaps, and unanswered product choices.
It is not a runtime compliance audit. No application, database, generated API, or test behavior is
changed by this task.

Primary authorities are [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) and
[FALL_2026_PILOT.md](../../FALL_2026_PILOT.md). The approved documentation plan supplies file
ownership and Neil's additional source/import/color decisions. The new entry points are
[QUESTION_SPECS/README.md](../../QUESTION_SPECS/README.md) and
[BIOLOGY_PROBLEMS_SPECS/README.md](../../BIOLOGY_PROBLEMS_SPECS/README.md).

## Later human corrections

The complete Question record and reusable Pool model supersede the initial refactor's split
metadata ownership and Assessment-owned Pool language. Each Question Revision contains all
attributes; permitted current-row metadata edits preserve its Revision Number. Bloom uses ordinary
metadata editing. Pool forks are reusable Pools with Instructor owners and parent pointers, and
Pool saves replace current state using an Edit Number for concurrency. These are documentation
corrections; implementation differences are recorded in [TODO.md](../../TODO.md).

## Pool consolidation

All six Pool specifications are now one [QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md).
The five narrower files are removed and their links point to the consolidated owner. Adding a Pool
references it; explicit forking provides independent customization. Each Assessment entry owns its
selection count. Save incomplete unreleased editing state and report Pool-use mismatches at the
Assessment; release validates completeness. Q28 and Q29 preserve Neil's instructions and reasons.
This supersedes the earlier automatic-fork answer. Runtime reconciliation remains in TODO.

## Related workflow consolidation

Import workflow and API evidence now share
[QUESTION_IMPORT_SPEC.md](../../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md).
Browse is a section in
[QUESTION_LIBRARY_SEARCH_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_SEARCH_SPEC.md#browse);
[QUESTION_LIBRARY_FILTER_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_FILTER_SPEC.md)
remains the owner of filter semantics, cross-kind behavior, combinations, and count scope.
The deferred bulk-edit workflow and its API evidence now share
[QUESTION_LIBRARY_BULK_EDIT_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_BULK_EDIT_SPEC.md).
The three redundant files are removed. Implementation evidence is labeled separately, and the
consolidation introduces no new product choices or implementation requirements.

## Reviewer cleanup

Removed duplicate Pool and Search links and corrected the Bloom index description to ordinary
metadata correction. Required Question language has no established human authority: the product
requirement is removed, existing schema/Native JSON constraints are labeled implementation
evidence, and reconciliation is in TODO. The QTI handoff remains undecided; the unsupported exact
Native JSON mapping table and feedback-timing conversion gate are removed. Current Pool and
Assessment guidance consistently uses Pool-use mismatch for an insufficient Assessment selection
count. This cleanup does not settle the remaining audit findings or certify runtime behavior.

## Requirement coverage

The old broad documents are replaced as rule owners. Neil requested deletion of the old Question
Model, Question ID, and Question Backend navigation files. Those three files are removed, and
Markdown links now lead directly to their replacements. Historical audit prose retains the names
of files inspected at the time. The old QTI page remains a navigation page.

| Previous material | Current owning specification |
| --- | --- |
| Question Model: identity and shared collection | [LIBRARY_OBJECT_SPEC.md](../../QUESTION_SPECS/LIBRARY_OBJECT_SPEC.md), [QUESTION_ID_SPEC.md](../../QUESTION_SPECS/QUESTION_ID_SPEC.md) |
| Draft state, saving, deletion, abandoned cleanup | [DRAFT_QUESTION_SPEC.md](../../QUESTION_SPECS/DRAFT_QUESTION_SPEC.md) |
| Publication, availability, archive and corrections | [PUBLISHED_QUESTION_SPEC.md](../../QUESTION_SPECS/PUBLISHED_QUESTION_SPEC.md), [QUESTION_REVISION_SPEC.md](../../QUESTION_SPECS/QUESTION_REVISION_SPEC.md) |
| Forks, authors and owner distinction | [QUESTION_FORK_SPEC.md](../../QUESTION_SPECS/QUESTION_FORK_SPEC.md), [QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md](../../QUESTION_SPECS/QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md) |
| Required metadata versus assigned/pending values | [QUESTION_LIBRARY_METADATA_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_METADATA_SPEC.md) |
| Shared vocabulary and hierarchy | [QUESTION_CLASSIFICATION_SPEC.md](../../QUESTION_SPECS/QUESTION_CLASSIFICATION_SPEC.md) |
| Bloom values, storage, corrections, API and search | [QUESTION_BLOOM_CLASSIFICATION_SPEC.md](../../QUESTION_SPECS/QUESTION_BLOOM_CLASSIFICATION_SPEC.md) |
| Pool current state and Edit Number | [QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md) |
| Distinct exact members and common Type/Backend/classification | [QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md) |
| Pool license calculation, member rights, NC/ND deferral | [QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md) |
| Assessment fork and independent Pool state | [QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md) |
| Pool draw, resume, exact delivery evidence | [QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md) |
| Pool mismatch and already-released Assessments | [QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md) |
| Combined result facts, display, stewardship, statistics | [QUESTION_LIBRARY_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_SPEC.md) |
| Combined filter meanings, no-Pool scope and counts | [QUESTION_LIBRARY_FILTER_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_FILTER_SPEC.md) |
| Search syntax, page state, navigation, sorting | [QUESTION_LIBRARY_SEARCH_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_SEARCH_SPEC.md) |
| Exploration through shared Library | [QUESTION_LIBRARY_SEARCH_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_SEARCH_SPEC.md) |
| Bulk metadata workflow and concrete commands | [QUESTION_LIBRARY_BULK_EDIT_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_BULK_EDIT_SPEC.md) |
| Old ID spec: alphabet, checksum, generation, collision handling and exact pins | [QUESTION_ID_SPEC.md](../../QUESTION_SPECS/QUESTION_ID_SPEC.md); exact immutable content is in Question Revision specification |
| Backend ownership, submission, stored credit, errors and future Backends | [QUESTION_BACKEND_SPEC.md](../../QUESTION_SPECS/QUESTION_BACKEND_SPEC.md) |
| WeBWorK source, ordered form response, review and sandbox | [WEBWORK_SPEC.md](../../QUESTION_SPECS/WEBWORK_SPEC.md) |
| Native source shape, validation, eight Types, grading, images and scripts | [NATIVE_JSON_SPEC.md](../../QUESTION_SPECS/NATIVE_JSON_SPEC.md), [QUESTION_TYPE_SPEC.md](../../QUESTION_SPECS/QUESTION_TYPE_SPEC.md) |
| QTI mappings and native-versus-interchange boundary | [QTI_INTERCHANGE_SPEC.md](../../QUESTION_SPECS/QTI_INTERCHANGE_SPEC.md) |
| Import/export workflow and generated-ID creation | [QUESTION_IMPORT_SPEC.md](../../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md), [QUESTION_EXPORT_SPEC.md](../../QUESTION_SPECS/QUESTION_EXPORT_SPEC.md) |
| BiologyProblems.org source, exclusions, complete/partial Course inventory | [BIOLOGY_PROBLEMS_SPECS/README.md](../../BIOLOGY_PROBLEMS_SPECS/README.md) and its source/Course specifications |
| General Blueprint import and returned Course/Assessment IDs | [BLUEPRINT_COURSE_IMPORT_API_SPEC.md](../../BLUEPRINT_COURSE_IMPORT_API_SPEC.md) |

Detailed supporting evidence is in [question_backend_spec_handoff.md](question_backend_spec_handoff.md),
[question_pool_spec_handoff.md](question_pool_spec_handoff.md), and
[biology_course_spec_handoff.md](biology_course_spec_handoff.md).

## Contradictions and assumptions corrected

- The old model and native-format document described display-only hyphens and an HMAC Question-ID
  checksum. HG and the dedicated ID spec require the stored canonical hyphen and public unsalted
  SHA-256. Neil explicitly reconfirmed the latter. The new ID specification is the single owner.
- Native JSON and QTI now have separate names and owners. Native JSON remains private, static,
  unversioned internal source. The completed Rust QTI Package Maker work is reused; it is not a
  proposed second PLE converter or an assumed WASM-ready runtime.
- Pools and Questions are defined from the shared Library Object model. Their necessary different
  fields remain explicit; a Pool gains neither a fabricated Author nor immutable Pool Revisions.
- Bloom absence is permitted without a deadline. It cannot justify NULL Question Type or another
  missing required field. Pool forks copy existing Bloom fields with their other current metadata;
  copying those values is not a new AI classification.
- Optional feedback content does not establish its Student disclosure timing. Old language claiming
  an independently configurable feedback-release rule is replaced with the explicit deferral.
- The Pool spec separates fields derived from members from independently assigned Pool metadata.
  ND support remains deferred, with future fork blocking already settled rather than reopened.
- A Question fork is a private Draft first. HG does not fix ID-allocation timing; the existing
  fork design reserves the ID at Draft creation. Publication adds it to the Library; reserving an ID does not.
- Current loader inspection shows generated IDs already used by several direct-store paths.
  Verify supported PLE API creation and readback rather than asserting that all current IDs are
  hard-coded. No specific HTTP transport or test harness is required by this documentation pass.

## Review against HG after the initial checks

The initial file, link, and JSON checks did not establish faithful coverage. A second review read
HG's Question interfaces, Question specifications, Library rules, and the settled interview record
against the new files. Independent read-only reviews covered Question lifecycle, Pools, Library behavior, Backends, and source content. Findings were checked against HG and recorded decisions before changes were made.

| Accepted finding | Authority and correction |
| --- | --- |
| Native MATCH interaction was omitted | [HG Native response presentation](../../HUMAN_GUIDANCE.md#native-question-response-presentation) requires a shared choice bank and equally capable keyboard interaction. Restored those rules in [NATIVE_JSON_SPEC.md](../../QUESTION_SPECS/NATIVE_JSON_SPEC.md). This is a documentation omission, not evidence that runtime MATCH is broken. |
| Approved recorded CDN dependencies were obscured by a closed library-name list | [HG JavaScript rules](../../HUMAN_GUIDANCE.md#native-ple-json-questions-and-javascript) permit recorded approved CDNs initially and call for eventual local serving. Restored that distinction in the Native spec. |
| Fork-ID allocation timing was asserted without HG support | [HG fork rules](../../HUMAN_GUIDANCE.md#published-question-revisions-edits-and-forks) require a private Draft and publication validation, not an allocation moment. The existing [design decision](../../DESIGN_DECISIONS.md#published-question-forks-create-a-new-private-draft-through-one-server-command) reserves an ID early. Removed the contrary no-ID-until-publication claims across the ID, Draft, fork, Revision, and report text. |
| Assessment-owned Pool scope was asserted without a recorded decision | [HG Library rules](../../HUMAN_GUIDANCE.md#question-library-specifications) say "Questions in no Pool"; the interview does not explicitly resolve which Pools count. Initially recorded Q11 as uncertain. Neil then clarified that forks are still Pools; HG and the filter spec now explicitly include them. |
| Pool creation interaction was omitted | [HG Question interface](../../HUMAN_GUIDANCE.md#question-interface) specifies Create Pool from Question, visible starting Question/classification, and filtered member search. Restored these in the Pool spec. |
| Archive confirmation was omitted | [HG high-consequence actions](../../HUMAN_GUIDANCE.md#high-consequence-actions) requires a separate Danger Zone, explanation, and clear confirmation. Restored these in the Published Question spec. |
| Instructor images were omitted at author/owner representations | [HG Profile visibility](../../HUMAN_GUIDANCE.md#instructor-profile-visibility) requires images when viewing Question authors or Pool owners. Restored the direct rule without changing their distinct meanings. |
| Library entry paths were incomplete | [HG Question and Assessment interfaces](../../HUMAN_GUIDANCE.md#question-interface) name the Ribbon views, direct Search/Browse, inspection before adding, and bulk addition by Question ID. Added these intended workflows without inventing picker mechanics. |
| Current implementation details read as product requirements | Marked concrete search protocol as implementation reference, removed detailed WeBWorK route/header prose, and removed the new import-failure status discussion. Existing implementation details do not decide product rules. |

Two reviewer suggestions were not accepted as new findings. API-created identities and readback
remain because Neil explicitly requested imports that prove ordinary PLE behavior. Export metadata
was already conditional on what the target can represent and required reporting omissions; that
text was not evidence of a new universal format guarantee. Review suggestions alone are not authority.

The cross-check confirmed the central settled Pool rules: unordered distinct exact members; common
Discipline, Subject, Type and Backend; Pool-owned search metadata; calculated compatible license;
independent Assessment forks; blank initial Bloom with no deadline; exact delivered Question pins;
and continued operation when mismatch develops after release. It did not test runtime enforcement.

## Remaining implementation differences

These are source-inspection findings or retained audit handoffs, not fresh runtime test results.

| Area | Remaining work or evidence limit |
| --- | --- |
| Library result integration | Common Question fields are lost or differently placed between Rust result, browser row and display; see the [earlier audit](../audits/library_object_documentation_audit_2026_10_05.md) |
| Type search | Structured Type filter and text Type matching differ for Pools |
| Unapproved Course-use filter | Neil never approved "Used in my Courses." UI/API/SQL removal is written and retained by Neil. Offline checks passed; database/browser verification remains in [TODO.md](../../TODO.md#question-spec-implementation-follow-up) |
| Pool storage and validation | Current member-position storage, sortable editor, classification re-checks, mismatch notices and release blocking require focused compliance verification; correct docs do not close that handoff |
| Bulk edits and picker selection | Instructor bulk editing is now deferred. Existing picker selection details remain implementation evidence, not new product rules |
| Browser-facing Draft creation | The route accepts Native JSON only. PG/PGML Draft creation and shared publication already exist in the content-loading tools; this is a route coverage gap, not absent WeBWorK support. See QUESTION_IMPORT_SPEC.md for current source evidence. |
| BiologyProblems.org source updates | Resolved by Neil: PLE is independent after launch and does not track later source changes; no synchronization work is required |
| Source/BP course loading | Distinguish raw SQL fixtures from content-loading tools that already use Draft storage and the shared publisher. Verify the intended supported API workflow and returned identities; do not describe all loading as SQL fixtures. |
| Course Themes | A new Course starts with its Blueprint's Theme; missing Blueprint storage/API support is implementation work recorded in TODO.md, not a product question |
| Saved search remnants | The follow-up source search found stale comments and test names in `question_search.rs`, not a saved-search schema table. Correct terminology and verify callers in TODO; permanent prompt/result storage remains outside intended behavior. |

## Questions for the later grill

The later [decision review](question_specs_decision_review_2026_10_05.md) found unsupported
requirements in this refactor despite the earlier formatting and link checks. Those checks do not
prove that product decisions have sound sources. Q05 was an unnecessary Theme question, and Q06
does not establish a restriction on adding Questions and Pools together.

The maintained log is
[question_specs_open_questions.md](../decisions/question_specs_open_questions.md). It separates
unresolved specification gaps, lower-priority notes, flexible defaults, and explicit deferrals.
These notes are not a new import-design task or an automatic interview agenda. The
specifications link there rather than silently selecting behavior. No interview question is a
new implementation approval gate for an already settled requirement.

## Validation

- All 42 planned specification files exist: 31 Question files, ten BiologyProblems.org files,
  and the Blueprint Course import specification.
- The repository Markdown link check passes. Historical reports retain retired evidence paths
  as text rather than broken links; 43 links in 14 historical documents were corrected.
- JSON examples pass a one-time syntax check, and each new specification stays below 1000 lines.
- `git diff --check` passes. The later grill updated HG and its evidence checklist together;
  new checklist entries remain unverified implementation requirements.
- No application code, schema, or permanent tests were changed. These checks establish document
  consistency, not runtime compliance.

Import protocol design was removed following Neil's scope correction. The import specifications
state intended behavior and record implementation drift without prescribing a new set of HTTP error rules,
retry mechanism, or replacement implementation.

## Later grill decisions

Neil confirmed that Pool forks remain Pools, deferred Instructor bulk editing, chose website
topics as the starting Assessment grouping, and made PLE independent of later BiologyProblems.org
changes. Instructor Question export is specifically for another LMS. The external
`qti-package-maker-rs` library owns conversion and packaging. A selected Pool exports its members
as separate Questions of the same Type, together in an import package; nothing merges Questions.
Reasons, firmness, and remaining notes are in the uncertainty log.
