# Question specifications documentation plan

## Goal and authority

Create a focused specification set in `docs/QUESTION_SPECS/` that explains PLE's Question model
well enough to implement and review it without guessing. Prefer explicit rules, field definitions,
and examples over brevity when they prevent different interpretations. Use plain PLE language.

Published Questions and Question Pools were both part of the original design. Both are Library
Objects. A Library search result represents a Library Object with shared fields and kind-specific
fields. Separate behavior does not make either kind secondary.

[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) and [FALL_2026_PILOT.md](../../FALL_2026_PILOT.md)
remain primary authorities. Preserve settled decisions and distinguish deferred or unanswered
choices. Existing code provides implementation evidence; it does not settle product rules.
The earlier [library_object_documentation_audit_2026_10_05.md](../audits/library_object_documentation_audit_2026_10_05.md)
identifies integration gaps that the new specifications must make possible to assess.

## Completion status

The approved specification files are written and the substantive fidelity review has finished.
The report records authority-backed corrections and unresolved gaps separately from mechanical checks. See
[Question specifications](../../QUESTION_SPECS/README.md),
[BiologyProblems.org specifications](../../BIOLOGY_PROBLEMS_SPECS/README.md), the
[alignment report](../reports/question_specs_alignment_report.md), and the
[uncertainty log](../decisions/question_specs_open_questions.md).
Application implementation and runtime compliance remain separate work. Neil clarified that this
pass repairs specification drift; import protocol design is outside its scope.

This completed plan remains here because `.git` is not writable in this session, preventing the
required `git mv` into `docs/archive/`. The move was not attempted after the permission check.

## Final folder structure

```text
docs/QUESTION_SPECS/
    README.md
    LIBRARY_OBJECT_SPEC.md
    QUESTION_LIBRARY_SPEC.md
    QUESTION_LIBRARY_SEARCH_SPEC.md
    QUESTION_LIBRARY_SEARCH_SPEC.md
    QUESTION_LIBRARY_FILTER_SPEC.md
    QUESTION_LIBRARY_BULK_EDIT_SPEC.md
    QUESTION_ID_SPEC.md
    QUESTION_REVISION_SPEC.md
    DRAFT_QUESTION_SPEC.md
    PUBLISHED_QUESTION_SPEC.md
    QUESTION_FORK_SPEC.md
    QUESTION_POOL_SPEC.md
    QUESTION_POOL_SPEC.md
    QUESTION_POOL_SPEC.md
    QUESTION_POOL_SPEC.md
    QUESTION_POOL_SPEC.md
    QUESTION_POOL_SPEC.md
    QUESTION_LIBRARY_METADATA_SPEC.md
    QUESTION_CLASSIFICATION_SPEC.md
    QUESTION_BLOOM_CLASSIFICATION_SPEC.md
    QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md
    QUESTION_BACKEND_SPEC.md
    QUESTION_TYPE_SPEC.md
    NATIVE_JSON_SPEC.md
    WEBWORK_SPEC.md
    QUESTION_IMPORT_SPEC.md
    QUESTION_IMPORT_SPEC.md
    QUESTION_EXPORT_SPEC.md
    QTI_INTERCHANGE_SPEC.md
    QUESTION_LIBRARY_BULK_EDIT_SPEC.md
```

Give BiologyProblems.org content its own specification folder:

```text
docs/BIOLOGY_PROBLEMS_SPECS/
    README.md
    BIOLOGY_PROBLEMS_SOURCE_SPEC.md
    BIOLOGY_PROBLEMS_IMPORT_SPEC.md
    BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md
    BIOCHEMISTRY_COURSE_SPEC.md
    GENETICS_COURSE_SPEC.md
    MOLECULAR_BIOLOGY_COURSE_SPEC.md
    BIOSTATISTICS_COURSE_SPEC.md
    LABORATORY_COURSE_SPEC.md
    BIOTECHNOLOGY_COURSE_SPEC.md
```

Add the reusable Blueprint Course API specification outside the source-specific folder:

```text
docs/BLUEPRINT_COURSE_IMPORT_API_SPEC.md
```

Use 31 files in `QUESTION_SPECS/`, ten in `BIOLOGY_PROBLEMS_SPECS/`, and the Blueprint Course
import API specification as the starting scope (42 files total).
The Question set defines reusable PLE behavior. The BiologyProblems.org set specifies the actual
source collections and base Blueprint Courses built from them. This replaces the earlier proposal
to keep one source file in the Question folder and one Course import file at the docs root.
Prefer more files with narrow, obvious responsibilities over fewer broad documents. Add another
file when drafting exposes a distinct responsibility that cannot be explained clearly in its
assigned document. Do not compress necessary detail just to preserve the file count.

## Document responsibilities

| File | Rules and detail it owns |
| --- | --- |
| `README.md` | Model overview, reading order, document map, authority, and links to related specifications outside this folder. |
| `LIBRARY_OBJECT_SPEC.md` | Small foundational model: what a Library Object is, its two kinds, shared responsibilities, kind-specific behavior, and links to the detailed rules. |
| `QUESTION_LIBRARY_SPEC.md` | One global Library; included objects and access; shared Library Object result fields; display modes; selection and picker reuse; Stars and Watches within established scope; links to Search, Browse, filters, and bulk editing. |
| `QUESTION_LIBRARY_SEARCH_SPEC.md` | Search and Browse, text syntax, shared results, sorting, paging, navigation, and temporary search state; filter meaning belongs to the filter spec |
| `QUESTION_LIBRARY_FILTER_SPEC.md` | Each filter's meaning for each object kind; combining filters; Questions in no Pool; default membership; absent values; counts; consistent filtering before sorting and paging. |
| `QUESTION_LIBRARY_BULK_EDIT_SPEC.md` | Deferred Instructor workflow, intended semantics, unresolved mixed-save behavior, and current API evidence |
| `QUESTION_ID_SPEC.md` | Internal and public IDs; shared Question/Pool public namespace; generation; validation; display; lookup; which actions retain or create an ID. |
| `QUESTION_REVISION_SPEC.md` | Exact Published Question Revisions; immutable content versus editable metadata; Revision numbering; exact references retained by Assessments and Student Work. Points to the Pool Edit Number rules rather than treating them as Revisions. |
| `DRAFT_QUESTION_SPEC.md` | Private working content; creation and editing; save behavior; deletion; publication readiness; exact handoff to Published Question creation. |
| `PUBLISHED_QUESTION_SPEC.md` | Publication and availability; permitted changes; corrections; archive; effects on existing uses. Links to the exact Revision, fork, and ownership rules. |
| `QUESTION_FORK_SPEC.md` | Forking a Published Question into a private Draft; copied content and metadata; attribution and source reference; ownership; publication under a new Question ID. |
| `QUESTION_POOL_SPEC.md` | All Pool behavior: current state, membership, metadata, license, references, selection, explicit forks, mismatch, release checks, and evidence. |
| `QUESTION_LIBRARY_METADATA_SPEC.md` | Authoritative shared field table: meaning, location, type, required/optional status, NULL/empty behavior, assigned/derived values, editing, and applicable object kinds. Links to detailed classification, authorship, and license rules. |
| `QUESTION_CLASSIFICATION_SPEC.md` | Discipline, Subject, Topic, Subtopic, and Tags; hierarchy and vocabulary rules for Questions and Pools; required and optional classification; changes and their effects. |
| `QUESTION_BLOOM_CLASSIFICATION_SPEC.md` | Two independent Bloom dimensions; Question Revision versus Pool assignment; NULL while pending; deferred initial AI assignment without a deadline; Instructor corrections and edit checks. Links to the teaching guide. |
| `QUESTION_AUTHORSHIP_AND_OWNERSHIP_SPEC.md` | Author versus owner; permitted actions; attribution; differences between Questions and Pools; relationships to forks and imported content. |
| `QUESTION_BACKEND_SPEC.md` | Common division of work between PLE and a Question Backend; source validation; rendering; response interpretation; grading and partial credit; feedback; assets; saved state; failures and protected information. |
| `QUESTION_TYPE_SPEC.md` | PLE Question Types and their meanings; required author/import declaration; no NULL Published Question Type; distinction from Backend and source format; common Pool Type. |
| `NATIVE_JSON_SPEC.md` | Complete internal JSON fields; source examples; supported Question Types; allowed responses; grading rules; validation; answer-choice randomization; presentation; images; permitted JavaScript and dependencies; changes to the internal format. |
| `WEBWORK_SPEC.md` | PG versus PGML source; algorithmic Questions; backend-owned rendering, interaction, and grading; randomization; saved response/state; resume behavior; feedback limits; connection to the common backend contract. |
| `QUESTION_IMPORT_SPEC.md` | Import workflow, Drafts, publication, generated PLE identities, source mapping, readback, and current implementation evidence |
| `QUESTION_EXPORT_SPEC.md` | PLE export workflow; eligible content; exact content selected for export; included metadata, attribution, licenses, and assets; supported outputs; unsupported features and known information loss. |
| `QTI_INTERCHANGE_SPEC.md` | Format-specific QTI import/export mappings; supported constructs; response and grading mappings; identifiers; assets and packaging; unsupported constructs; preservation and loss of information. |

The ten files in `BIOLOGY_PROBLEMS_SPECS/` have these responsibilities:

| File | Rules and detail it owns |
| --- | --- |
| `README.md` | Source-course inventory and completeness; relation to the pilot; map of the six Course files and common source/import rules. |
| `BIOLOGY_PROBLEMS_SOURCE_SPEC.md` | Collection scope and exclusions; source-to-Question mapping; preferred original source; PG versus PGML; static Native JSON conversion; images/assets; duplicate and generated-variant handling; attribution. |
| `BIOLOGY_PROBLEMS_IMPORT_SPEC.md` | Complete importer sequence: read the source inventory, reuse Rust conversion tools, call Question APIs, record returned IDs/Revisions, invoke Course assembly, report failures, handle another run, and verify the imported content through APIs. Links to source mapping and Course definitions rather than repeating them. |
| `BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md` | Common steps for assembling Blueprint Courses from imported Published Questions and Pools; preserving exact references, order, and reusable settings; recording omissions and checking assembled content. |
| `BIOCHEMISTRY_COURSE_SPEC.md` | Complete Biochemistry source course mapped to its intended Blueprint Course content. |
| `GENETICS_COURSE_SPEC.md` | Complete Genetics source course mapped to its intended Blueprint Course content. |
| `MOLECULAR_BIOLOGY_COURSE_SPEC.md` | Available Molecular Biology content, intended Course structure, and missing coverage. |
| `BIOSTATISTICS_COURSE_SPEC.md` | Available Biostatistics content, intended Course structure, missing coverage, and pilot scope. |
| `LABORATORY_COURSE_SPEC.md` | Available Laboratory content, intended Course structure, and missing coverage. |
| `BIOTECHNOLOGY_COURSE_SPEC.md` | Biotechnology content and Course mapping required by the pilot; source completeness remains unclassified pending inspection. |

Each Course file records its source sections/problem sets, included and excluded content, ordered
Blueprint Assessments, Question and Pool choices, exact source references, reusable settings,
the Course color preference and its PLE Theme mapping,
and missing content or unanswered Course choices. The common Course import file describes how
those mappings are applied; it does not repeat six Course inventories. Refer to a maintained
machine-readable inventory when one exists rather than maintaining two editable Question lists.
Add topic-level files only when a Course's detailed mapping needs them. These are content-specific
specifications, not a new specification set for all PLE Course behavior.

`docs/BLUEPRINT_COURSE_IMPORT_API_SPEC.md` owns reusable authenticated API requests and responses
for creating Blueprint Courses and Assessments from supplied structure. PLE generates Blueprint
Course and Assessment IDs, checks authorization and references, and returns the created identities
and Revisions. The API accepts returned Question/Pool references rather than requiring preassigned
IDs for newly created objects. It defines validation, failures, repeat requests, and readback.
This specification serves any supported Course importer, including BiologyProblems.org.

## Boundaries that prevent duplication

- `LIBRARY_OBJECT_SPEC.md` stays small. `QUESTION_LIBRARY_METADATA_SPEC.md` owns the shared field
  table, including the object, Question Revision, or Pool to which each value belongs. The detailed
  metadata specifications own their rules; the table links to them.
- `QUESTION_LIBRARY_SPEC.md` owns the common result fields and presentation. Search and Browse
  describe different entry paths into that Library. `QUESTION_LIBRARY_FILTER_SPEC.md` owns filter
  meanings, result membership, and count rules used by both paths.
- `QUESTION_ID_SPEC.md` owns shared IDs; `QUESTION_REVISION_SPEC.md` owns Question Revisions;
  `QUESTION_POOL_SPEC.md` owns Pool Edit Numbers. Fork specifications own the copying operation
  and refer to those definitions. An Edit Number does not imply retained Pool Revisions.
- `QUESTION_LIBRARY_BULK_EDIT_SPEC.md` explains bulk editing from the Instructor's perspective.
  `QUESTION_LIBRARY_BULK_EDIT_SPEC.md` specifies requests and outcomes. Its scope includes Library
  Objects, despite the shorter Question filename; it must not assume Question-only editing.
- Import and Export own PLE workflows. QTI owns the format mappings used by those workflows.
  Naming an export document does not decide that every object or feature can be exported.
- Backend specifications explain how a selected Question runs. The Pool specification explains
  how PLE selects the Question. Pool selection and backend randomization remain separate.
- Existing Assessment lifecycle, authorization, FERPA, and Bloom teaching documents retain their
  responsibilities. Link to them. State the Question-side requirements at each connection without
  duplicating complete external specifications.

## Reuse QTI Package Maker

The existing Rust implementation is `~/nsh/PROBLEMS/qti-package-maker-rs/`, maintained in
[qti-package-maker-rs](https://github.com/vosslab/qti-package-maker-rs). The user reports that the
Rust port is faster and is considering a later Rust/WASM build. Treat WASM as planned direction,
not an already available PLE dependency or a prerequisite for this documentation task.

The local README, workspace manifest, and engine documentation identify reusable `qti-core`,
`qti-engines`, and `qti-integrity` crates. Document PLE's use of their existing item models,
readers, writers, and package checks rather than proposing a second implementation. PLE owns
its Question lifecycle, authorization, metadata requirements, and mapping to Native JSON.

The current engine inventory lists different reader and writer coverage. Verify each requested
input and output against that inventory; export support does not imply matching import support.
Specify missing format behavior as a gap in the shared project where appropriate. Before
implementation, inspect the actual public Rust APIs and select the smallest integration needed.
Preserve the reusable conversion work when WASM becomes available. Do not assume that native
filesystem, browser-rendering, or process dependencies already run in WASM.

`QUESTION_IMPORT_SPEC.md` and `QUESTION_EXPORT_SPEC.md` own PLE workflows;
`QTI_INTERCHANGE_SPEC.md` owns PLE field mappings and references shared conversion behavior.
`NATIVE_JSON_SPEC.md` continues to define PLE's internal format. Reusing conversion code does
not make QTI the PLE runtime model. Update stale claims about a future Rust port when replacing
the old documents.

## Import through PLE APIs

**Imported content supplies source data and relationships. PLE assigns PLE identities.**

Question importing and base Blueprint Course building must use authenticated PLE APIs. The
content loader supplies source content, metadata, and intended Course structure. PLE creates
the objects and generates their public IDs through its normal creation operations. Capture the
returned Question IDs, exact Question Revisions, Pool IDs/Edit Numbers where applicable, and
Blueprint Course IDs/Revisions. Use those returned values in subsequent requests.

Keep source identifiers and readable source names in the import inventory. They identify source
content; they are not preassigned PLE public IDs. Maintain the mapping from each source record
to the IDs and Revisions returned by PLE so Course assembly does not depend on hard-coded IDs.
The existing source-to-publication mapping can inform this design.

Use the same API operations and validation as normal Instructor use: create and edit a Draft,
attach source and assets, publish a Question, create any justified Pools, create a Blueprint Course,
save its ordered content, and make it Public when required. Inspect existing routes before adding
endpoints. If an operation required by the import is missing, document the API gap and implement
that operation instead of bypassing it through direct database writes. This requirement does not
create a new approval workflow, import queue, or privileged validation bypass.

`QUESTION_IMPORT_SPEC.md` describes the PLE workflow; `QUESTION_IMPORT_SPEC.md` owns the
concrete API sequence, requests/responses, returned IDs, and errors. The Blueprint Course import
API specification owns the equivalent Course operations. These specifications may use existing
endpoints; adding a document does not require a new bulk endpoint or a second publication path.
`BIOLOGY_PROBLEMS_IMPORT_SPEC.md` connects the two API workflows for the actual source inventory.
`BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md` describes how Course definitions use the returned
references to assemble each Course. Test scenarios obtain their expected public IDs from
creation responses rather than assuming fixed IDs.

The system test loads content through the running HTTP API, then reads back the resulting Questions,
Pools, and Blueprint Courses to check metadata, exact references, membership, and Assessment order.
Relevant invalid content must be rejected by the same rules as normal use. Include an actual
Assessment use of imported content in the later implementation checks. Define how a failed or
repeated import reports already completed work without inventing duplicate objects; settle any
missing repeat-import rule explicitly before claiming a repeatable import path.

Database creation still uses the repository's normal SQL schema definitions. Focused database
tests may use SQL fixtures. Source installation and Course assembly through direct store calls
or SQL do not provide evidence that the application API works end to end. Reusing an internal
service routine can avoid duplicate implementation, but calling it directly is not an HTTP API
system test. The Live Demo content loader and future importer use the same supported API path.

Current source inspection on 2026-10-05 found that
[publication.rs](../../../crates/project-tools/src/pilot_content/publication.rs) already uses the
normal Question ID issuer and returns exact published Question references. It directly calls
database-backed stores and the Question publisher.
[installation_data_blueprint.rs](../../../crates/project-tools/src/installation_data_blueprint.rs)
already receives generated Blueprint and Assessment IDs from the Blueprint creation store, then
passes IDs to the remaining demo SQL setup. Thus the known gap is API coverage, not a claim that
every current Question or Blueprint ID is hard-coded. Preserve useful existing creation logic
while moving the loader to the application API.

Existing API entry points include `/api/authoring/drafts`, Draft source and publication routes,
and `/api/course-blueprints` creation/import/content/publication routes. Their presence is a
starting point for inspecting coverage, not proof that the full import already works through them.

## BiologyProblems.org source and Courses

The [BiologyProblems.org site](https://biologyproblems.org/) organizes content by subject, topic,
and problem set. Use [FALL_2026_PILOT.md](../../FALL_2026_PILOT.md) and HG to select the intended
content; the website's full inventory does not define PLE's import scope. The local source mirror
is `OTHER_REPOS/biology-problems-website/site_docs/`.

The user identifies two complete source courses and three partially complete source courses:

| Source course | Completeness | Content |
| --- | --- | --- |
| Biochemistry | Complete | First-semester biochemistry, including macromolecules, pKa, and enzyme kinetics. |
| Genetics | Complete | Post-general biology inheritance genetics, including Punnett squares, gene mapping, and deletion mutants. |
| Molecular Biology | Partially complete | PCR, cloning, gene expression analysis, and core molecular biology techniques. |
| Biostatistics | Partially complete | Measures of central tendency and statistical tests. |
| Laboratory | Partially complete | Lab measurement basics, solutions, and dilutions. |

Carry this distinction into the source inventory and Blueprint Course mapping. Record available
sections and gaps for the partial courses. Source-course completeness and selection for the Fall
2026 pilot are separate facts. Biotechnology remains a named pilot source; this clarification
does not assign it a completeness level or remove it from pilot scope. Import and verification
status in PLE must be recorded separately from source-course completeness.

The source specification must cover these settled rules:

- The pilot names Genetics, Biotechnology, Biostatistics, and Biochemistry as source collections.
  Questions explicitly labeled BIOL 301 are excluded from the Fall 2026 pilot.
- Prefer original parameterized WeBWorK source when it exists. Preserve PG versus PGML; use PGML
  only for fully PGML-compliant source. One algorithmic source maps to one Published Question,
  not a Pool or many Published Questions corresponding to generated static variants.
- Use Native JSON for suitable static Questions. QTI is a possible import step to investigate,
  not a mandatory route or the internal PLE format. Reuse the existing Rust QTI Package Maker.
- Preserve source references, authorship, applicable licenses, and necessary images/assets.
  Record how source records correspond to PLE Question IDs so duplicate imports and changes to
  source content can be assessed. Document unresolved repeat-import behavior explicitly; this
  does not authorize a background synchronization service.
- Keep source Questions, source problem sets, PLE Question Pools, and Blueprint Assessments
  distinct. A source problem set does not automatically satisfy Pool membership requirements.

The Course specification must provide a concrete mapping table: source collection/section/problem
set, target Blueprint Course, Blueprint Assessment and order, selected Published Questions or
Pools, and reusable settings. State how choices preserve source organization and teaching intent.
Do not assume that every website section becomes one Assessment or every bank becomes one Pool.
Blueprint Courses contain reusable content without Students, deadlines, or relative schedules.

HG explicitly names Genetics as the shipped example Blueprint Course; the pilot names all four
source collections for production Course content. Document the intended base Course inventory
and any unsettled Course grouping/settings rather than presenting an inferred mapping as settled.
[PILOT_CONTENT.md](../../PILOT_CONTENT.md) currently describes an eight-Question teaching set;
it is evidence about that subset, not the complete imported Course inventory or a limit on it.

## Course color preferences

The user's source for Course colors is `~/nsh/syllabus/docs/COURSE_COLORS.md`. Its active
Course metadata in `~/nsh/syllabus/site_docs/fall_2026/` was checked on 2026-10-05. Record these
preferences in the corresponding Course specifications:

| Course | Color family | Header/light accent | Dark content accent |
| --- | --- | --- | --- |
| Biostatistics | Dark lime | `#477427` | `#a8d58a` |
| Genetics | Blue | `#1565c0` | `#8ab4f8` |
| Biotechnology | Brick red | `#9e3d32` | `#f28b82` |
| Biochemistry | Purple | `#7b1fa2` | Not recorded |
| Molecular Biology | Magenta | Not specified | Not specified |
| Laboratory | Teal-green | Not specified | Not specified |

The user added Molecular Biology as magenta and Laboratory as teal-green in this planning
conversation. These are settled color-family preferences; exact RGB values were not specified.
They supplement the four preferences in the syllabus guide. Biochemistry's purple
is a recorded preference; the syllabus has no active metadata file for that Course. Keep these
unknowns explicit. The syllabus guide and active Course metadata remain the source for the
preferences; this table records the planning evidence, not a second editable syllabus palette.

Use the existing Blueprint Course Theme to express the user's Course color preference. PLE
provides coordinated Light and Dark appearances in
[BIOME_THEME_PALETTES.md](../../BIOME_THEME_PALETTES.md). Proposed matches from that catalog are:

| Blueprint Course | Proposed Theme | Match |
| --- | --- | --- |
| Genetics | Ocean (`ocean`) | Blue surfaces and blue accents. |
| Biostatistics | Grassland (`grass`) | Lime-green surfaces with green accents. |
| Biotechnology | Magma (`magma`) | Warm red surfaces and brick-red Light accent. |
| Biochemistry | Tundra (`tundra`), tentative | Muted lavender/purple surfaces; green accents make this an approximate match rather than a fully purple Theme. |
| Molecular Biology | To be matched | Match the user's magenta preference. |
| Laboratory | To be matched | Match the user's teal-green preference. |

These are proposed starting Themes based on the catalog's palette values, not new Theme
definitions or visually approved Course assignments. Biochemistry needs the closest review
because the current catalog lacks a clearly purple Theme throughout both appearances. Preserve
the existing ability for an Instructor to change a Course Theme. Check the eventual Course
mapping in Light and Dark appearances and retain Course titles as visible identifiers alongside
color. The syllabus RGB values express preferences; the selected PLE Theme supplies its complete
palette. No per-Course custom RGB setting is needed for this approach.

## Detail required in each specification

Use sections that fit the subject; complete the following where applicable:

1. Purpose, terms, scope, and authoritative source references.
2. Objects and fields: exact meaning, type, allowed values, required versus optional, NULL versus
   empty, who supplies the value, where it belongs, whether it can change, and how changes are checked.
3. Operations: who may act, required starting state, checks, successful result, and failure behavior.
4. Effects on related objects, existing Assessments, open Attempts, and retained Student Work.
5. Concrete valid and invalid examples, including important differences between Questions and Pools.
6. Deferred or unanswered rules, with links to decision records. Preserve the user's reason and
   stated confidence. Separate a firm rule from a preference such as the default search membership.

For JSON and API specifications, document exact fields and representative requests/responses.
For Native JSON, describe each Question Type's accepted response and grading behavior explicitly;
a list of supported type names is insufficient. For search, state each filter's behavior for each
Library Object kind, including absent metadata and its effect on counts and paging.

Record gaps instead of filling them with invented product behavior. Deferred AI classification,
feedback timing, regrading, and license support retain their existing status. Distinguish an
unimplemented settled requirement from an undecided requirement. Keep implementation progress,
audit results, and test logs in reports outside the specification folder.

## Language and names

Use [TERMINOLOGY_CONTRACT.md](../../TERMINOLOGY_CONTRACT.md) as subordinate to HG. Correct conflicting
terminology rather than carrying it into the new set.

- Use Instructor, Student, Library Object, Published Question, Question Pool, Question Backend,
  Question Revision, Pool Edit Number, Assessment, Attempt, and Student Work with their PLE meanings.
- Say owner and author distinctly. A Pool has an owner and no separate Pool Author field.
- Say co-Instructor for the established Course role. Avoid introducing curators, maintainers,
  verification tiers, approval workflows, or discussion forums through specification wording.
- Prefer direct phrases such as "fields returned by search," "check before publication," and
  "what happens when saving fails." Explain a technical term where it first becomes necessary.
- Use Question Type, filter, search result, membership, and selection. Avoid introducing product
  vocabulary such as projection, discriminator, polymorphic result, entity, resolver, or contract
  shape when existing PLE terms express the rule. Keep necessary software terminology in the
  relevant technical sections.
- Use descriptive field and variable names. Follow SQL, Rust, TypeScript, and JSON naming rules;
  do not abbreviate away distinctions between object ID, Question Revision, and Pool Edit Number.
- Reserve required wording for established requirements. Label examples, implementation facts,
  proposed choices, and deferred features clearly.

## Writing and replacement order

1. Inventory the relevant HG bullets, settled decisions, and existing specification sections.
   Assign each rule one owning document and identify contradictions or missing decisions.
2. Write the README and foundational Library Object, metadata, Question ID, Question Revision,
   and Pool specifications. Establish the shared fields, identities, and Pool Edit Number.
3. Write the detailed classification, Bloom, authorship/ownership, and Question Type specifications.
   Write the Library, Search, Browse, filter, and bulk-edit behavior specifications against them.
4. Write Draft, Published Question, Question fork, and the detailed Pool specifications. Then write
   the common backend specification and Native JSON and WeBWorK specifications.
5. Write Import, Export, QTI mappings, and the bulk-edit API using the established object rules
   and existing Rust QTI Package Maker capabilities. Then write the BiologyProblems.org source
   specification, common Course import specification, and six Course content specifications
   using those import rules.
6. Review high-impact examples across documents: publication with pending Bloom, a Pool membership
   change, a mixed Library search, bulk metadata edits, an Assessment Pool fork, and exact Student
   Work references. Record unsupported assumptions and omissions as explicit findings.
7. Replace the old owning documents and update references after their material is accounted for.
   Use `git mv` for moves and renames. Keep one current definition for each rule.

## File scope

Create the 31 documents under `docs/QUESTION_SPECS/`, ten documents under
`docs/BIOLOGY_PROBLEMS_SPECS/`, and `docs/BLUEPRINT_COURSE_IMPORT_API_SPEC.md`
during the later writing task. Redistribute
material from [QUESTION_SPECS/README.md](../../QUESTION_SPECS/README.md),
[QUESTION_ID_SPEC.md](../../QUESTION_SPECS/QUESTION_ID_SPEC.md),
[QUESTION_BACKEND_SPEC.md](../../QUESTION_SPECS/QUESTION_BACKEND_SPEC.md), and
[QTI-JSON_OBJECT_FORMAT.md](../../QTI-JSON_OBJECT_FORMAT.md). Retire their duplicated ownership
after replacements exist; update referring documentation and relevant code comments/tests that
contain document paths. Preserve implementation evidence without promoting it to product authority.

Update the documentation index and [CHANGELOG.md](../../CHANGELOG.md). Reorganizing these
specifications does not itself require changing HG, its evidence checklist, application code,
database schemas, or generated API types.

## Completion checks

- Every in-scope existing requirement has a new home or an explicit unresolved/deferred record.
- Shared metadata, identity, search, Pool membership, and Assessment-use rules agree across files.
- The source specification preserves pilot exclusions and original algorithmic source; the Course
  specification makes the base Blueprint Course mapping explicit without treating the eight-Question
  teaching set as the entire collection. Remaining Course assembly decisions are clearly identified.
- A reader can find each important field definition and operation without inspecting source code
  to discover undocumented product rules. Technical examples agree with the stated specification.
- The remaining code/spec differences are named in an audit report; documentation checks are not
  claimed as implementation compliance.
- The import and Course-building specifications require API-created objects and returned IDs.
  Their implementation checks exercise the API and read back assembled content; a successful SQL
  seed alone does not satisfy those checks.
- Old document references resolve to the new owners, and duplicate current definitions are removed.
- Run the repository Markdown link check and `git diff --check`. Run HG format and checklist
  checks only if HG or its copied evidence changes. Add no permanent tests solely for the reorganization.
