# Question specification decision review

## Result and scope

This pass reviewed the 25-file Question specification set, the uncertainty log, and relevant
linked guidance. It examined statements presented as settled as well as open entries. HG and
Neil's direct corrections governed the review; implementation and prior agent-written decisions
were evidence, not substitutes for approval.

The pass corrected unsupported requirements, narrowed ambiguous wording, and separated existing
implementation limits from product rules. It found no new high-impact question that needs Neil's
answer now. Existing deferrals remain deferred. Application changes are recorded in
[TODO.md](../../TODO.md#question-spec-implementation-follow-up).

The classifications below use Neil's requested distinction:

1. Clearly settled by HG or an explicit human correction.
2. Reasonable implementation detail that needs no product interview.
3. Unsupported interpretation to remove or simplify.
4. Genuine product uncertainty, asked only when it matters to current work.

An unsupported claim of authority does not by itself prove the implementation is defective.
Likewise, removing a requirement does not approve a new feature to replace it.

## Corrections and implementation evidence

### R01: Classification became execution work

**Class 3: corrected.**

[QUESTION_TYPE_SPEC.md](../../QUESTION_SPECS/QUESTION_TYPE_SPEC.md) said adding a Type requires
source/response validation, presentation, grading, accessible interaction, import/export mappings,
Library filtering, and Pool rules. It did not distinguish a Native JSON interaction from a
classification label on a Backend that already handles the interaction.

Exact HG: "Native JSON has a built-in Question Type; other Question Backends use Question Type as
editable classification metadata." HG also says "Question Backends own rendering, interaction,
response, grading, feedback, and backend-specific state."
See [Question formats](../../HUMAN_GUIDANCE.md#question-formats-and-type-specifications) and
[Backend responsibilities](../../HUMAN_GUIDANCE.md#question-backend-responsibilities).

The spec now separates metadata changes from implementing a new Native JSON interaction. Its
eight API values are identified as the current vocabulary rather than an exhaustive description
of interactions a Backend may render. This adds no new Type or Backend capability.

### R02: Bloom transport became completion policy

**Class 3 for the product requirement; class 2 for transporting existing values together. Corrected.**

The metadata table said Bloom Knowledge Dimension was "saved with complete pair." The existing
[SQL validator](../../../schemas/base_schema/50_functions/question_bloom.sql) indeed rejects a
missing dimension in that request. A transport requirement had appeared in the product field table.

Exact HG: "The two Bloom dimensions are independent"; the owner "can correct either Bloom
dimension"; "Bloom is an editable field like Title. Use the ordinary record save and concurrency
checks." HG permits NULL while awaiting assignment with no deadline.
See [Bloom metadata](../../HUMAN_GUIDANCE.md#question-library-bloom-classification-metadata).

Removed the complete-pair product requirement. The
[Bloom spec](../../QUESTION_SPECS/QUESTION_BLOOM_CLASSIFICATION_SPEC.md) labels that request shape
as implementation evidence. Sending both existing values together can be an ordinary engineering
choice; requiring an Instructor to complete a second nullable field is a different requirement.
The existing metadata-concurrency TODO now includes this distinction.

### R03: Pool checking could become an edit prohibition

**Class 3: ambiguous expansion removed.**

[QUESTION_REVISION_SPEC.md](../../QUESTION_SPECS/QUESTION_REVISION_SPEC.md) said saving metadata
"validates required fields and affected Pool constraints." Read as a Save gate, that can prohibit
a valid Question classification correction because another object references it.

Exact HG: "If a member's Discipline or Subject changes so it no longer matches the Pool, flag a
Pool mismatch and prevent release of an affected Assessment until the mismatch is resolved."
See [Pool metadata](../../HUMAN_GUIDANCE.md#question-pool-metadata).

The spec now distinguishes validating the Question's own fields from reporting the resulting Pool
mismatch and applying release rules. This is a clarification of existing HG, not a new permission
or a claim that every current write path was tested. TODO records checking the implementation.

### R04: Interchangeability became a certification step

**Class 1 for interchangeable members; class 3 for presenting an additional certification mechanism as required.**

HG says: "Pool contents should represent reasonably interchangeable assessments of the intended
learning." That describes the Questions a Pool contains. When the set of Question Revision Tuples
in a Pool changes, saving advances the Pool's Edit Number.
See [Question Pools](../../HUMAN_GUIDANCE.md#question-pool-specifications).

Earlier HG combined Save with the phrase "re-attests interchangeability." Neil explicitly removed
that wording in this follow-up. The previous review repeated the phrase as authority instead of
questioning the added concept. The current specs keep membership requirements separate from Save.

The current [creation dialog](../../../src/components/question_pool_create_dialog.tsx) and
[member editor](../../../src/pages/assessment_workspace/assessment_pool_entry_editor.tsx) require
a separate checkbox. [Pool SQL](../../../schemas/base_schema/50_functions/question_pools.sql)
requires an attestation flag and stores an attesting Account and time. Those implementation
additions remain concrete removal work in TODO. When the set of Question Revision Tuples in a Pool changes, saving
advances the Pool's Edit Number.
Validation and mismatch follow the existing Pool rules.

### R05: A conditional feedback rule became absolute

**Class 3: corrected.**

[WEBWORK_SPEC.md](../../QUESTION_SPECS/WEBWORK_SPEC.md) said "Backend feedback is transient."
Exact HG says "Question Backend feedback is transient unless the backend provides a robust way
for PLE to preserve it."
See [Backend grading and feedback](../../HUMAN_GUIDANCE.md#question-backend-grading-and-feedback).

Restored the exception. This neither commissions feedback storage nor settles optional feedback
timing. It keeps the rule consistent across Backends rather than making an unconditional WeBWorK
restriction out of a conditional common rule.

### R06: Existing counters became pedagogy

**Class 3 for the claimed product rationale; class 2 for documented current counters. Corrected.**

[FERPA_DATA_POLICY.md](../../FERPA_DATA_POLICY.md) called every Attempt an observation because
"the pedagogy" treats it that way, and introduced the detailed counter scheme as exactly what the
statistic collects and shows. The earlier cross-Revision rule had been removed, but this broader
claim of authority remained.

Exact HG: "Question statistics show how often Students received the Question and how much credit
they earned." It specifies graded-response count, average credit, full-credit percentage, and
zero-credit percentage, using stored Backend credit. It keeps statistics separately per Revision
and per Pool. See [usage statistics](../../HUMAN_GUIDANCE.md#question-library-object-usage-statistics).

The data policy now leads with those measures. The existing collection timing and additional
counters are under Current implementation evidence. Removed the unsupported pedagogical reason.
The SQL increments `issued_count` through submission collection; a name alone does not establish
that it counts all deliveries, including unsubmitted Attempts. The existing statistics TODO now
calls out that comparison, without designing a replacement collection system.

The text also no longer claims that identifier removal and calendar-date precision by themselves
prove anonymity. HG explicitly says shared statistics must prevent reasonable identification.
This is an alignment of PLE's stated requirement, not a new legal assessment.

### R07: Compiler limits looked like Question Type definitions

**Class 2 as current implementation limits; class 3 if presented as human-chosen teaching rules. Relabeled.**

The Native JSON spec presented these without a clear evidence boundary: 2-100 choices, at least
two MATCH prompts, at least three ORDER items, and nonoverlapping rectangular HOTSPOT regions.
[source_compile.rs](../../../crates/adapters/ple/src/question_json/source_compile.rs) implements
those limits and also enforces one-to-one MATCH answer bindings.

Exact HG: "MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT Question Types should be supported."
It specifies the selected grading rules and says HOTSPOT uses supported still images and SVG.
It does not choose those counts or geometry limits.
See [formats and Types](../../HUMAN_GUIDANCE.md#question-formats-and-type-specifications) and
[Native interaction](../../HUMAN_GUIDANCE.md#native-ple-json-questions-and-javascript).

[NATIVE_JSON_SPEC.md](../../QUESTION_SPECS/NATIVE_JSON_SPEC.md#current-implementation-limits) now
labels them current compiler limits. The ORDER examples for three through seven items remain
formula validation examples, not a minimum or maximum count. This pass neither removes runtime
validation nor creates requirements for new geometry, reusable MATCH choices, or other input forms.
Those should be driven by actual content needs, not a speculative interview.

### R08: A rejected question remained open

**Class 3: log corrected.**

Q32 still said useful statistics needed a human decision, despite Q33-Q36 recording the subsequent
answers now present in HG. Marked Q32 closed, retained why its original premise was rejected, and
removed language presenting completed clarification as a future question.
The [question log](../decisions/question_specs_open_questions.md) keeps the original decision
history with the current status made explicit.

### R09: First publication stood in for all publication

**Class 3: scope corrected.**

The Draft operation table labeled its row simply "Publish" and said the result is a new
Published Question at Revision 1. That describes first publication but omits publication of a new
Revision of an existing Question. Read as exhaustive workflow guidance, it can turn every source
correction into a new object.

Exact HG: "A new Revision keeps the same Published Question ID" and "The owning Instructor or a
Sysadmin may publish a new Question Revision."
See [common revision rules](../../HUMAN_GUIDANCE.md#common-revision-and-history-specifications) and
[Published Question edits](../../HUMAN_GUIDANCE.md#published-question-revisions-edits-and-forks).

The [authoring routes](../../../crates/server/src/authoring.rs) already expose both `publish` and
`publish-revision` for Drafts. The table now distinguishes first publication, new Revision, and
fork publication. The Published Question spec also labels its first-publication section precisely.
No new lifecycle or endpoint is proposed, and no product question is needed.

## Settled rules preserved

These suspicious-looking strong rules have direct authority. They were not weakened merely
because they use "only," "required," or "automatic."

| Rule checked | Exact HG or direct-decision basis | Classification |
| --- | --- | --- |
| Combined Questions/Pools; tentative initial filter | HG says both appear in combined search; the initial view "would probably be best" showing Questions in no Pool plus Pools | 1: combined model settled; default stays tentative |
| Explicit Pool forks and direct Assessment references | "Adding a Question Pool to an Assessment uses that existing Pool; adding it does not create a fork" | 1 |
| Pool membership is unordered and distinct | "A Question Pool cannot contain two copies of the same Published Question" and ordering is for display | 1 |
| Same Pool Type, Backend, Discipline, Subject | Explicit matching rules; HG also says Topic and Subtopic do not have to match | 1 |
| Pool license is calculated | "PLE calculates this license automatically from the member licenses" | 1; ordinary Question forks still copy their source license |
| Drafts preserve incomplete content | "Drafts are Drafts and have no content or metadata requirements" before publication | 1 |
| Complete Revision records and ordinary metadata fields | "A Question Revision is a complete database record. Some fields can change in place" | 1 |
| Owner/Sysadmin editing and Instructor reuse/forking | Explicit Published Question editing and reuse rules; Q14/Q15 retain the human statements | 1 |
| Grading formulas and Assessment partial credit | Direct supplied MA/ORDER decisions and HG's explicit MATCH, MULTI-FIB, MC/HOTSPOT, NUM, and FIB rules | 1 |
| Partial credit initially enabled | "New Assessments start with partial credit enabled"; Q30 records explicit selection | 1 |
| Archive and fork licensing | Latest short Archive rule and explicit same-source-license fork clarification | 1; no extra state machine or license choice |
| WeBWorK Type detection | "Assign WeBWorK Question Type manually for now; automatic detection is deferred" | 1 |
| Export conversion ownership | "Export is for selected Questions to use in another LMS" and the external Rust library handles conversion | 1 |

Each row is checked against the corresponding section of
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md), rather than treating a lower-level spec as its own
authority. Ordinary questions about field names, stable IDs, numerical bounds, parsing, paging,
and transport were not automatically promoted into product questions.

## Implementation choices retained

**Class 2.** Existing search syntax spellings, API page sizes, cursor encoding, ID-collision
handling, strict JSON parsing, finite numeric values, and stable item identifiers can concretely
implement the settled behavior. Their numeric constants and type names are not human decisions.
The search, identity, classification, and Native JSON specs identify their implementation sources.
Nothing in this review requires interviewing Neil on each constant or removing necessary validation.

The bulk-edit spec is prominently deferred and labels its current endpoint/transaction details
as evidence. A separate Question and Pool command does not settle future mixed-save behavior.
Its existing code was not treated as approval to resume bulk-edit delivery.

## Genuine uncertainty and interview outcome

**Class 4, already deferred:** optional Question Feedback timing, converter handoff representation,
Instructor bulk-edit mixed-save behavior, and future regrading details still have meaningful
choices. Their prerequisites have not changed. This pass neither reopens them nor invents a
replacement choice. The tentative initial search filter is also not a delivery blocker.

No newly identified item required an immediate interview. The corrections above follow existing
rules or remove unsupported certainty. The separate Pool attestation mechanism has a concrete removal TODO. Native JSON input
limitations remain implementation evidence until an actual content need warrants a change.

## Coverage and verification

Reviewed groups include shared Library discovery/filtering/bulk edit, identity and Revision rules,
Draft/Published/fork behavior, the consolidated Pool model, shared metadata/classification/Bloom/
ownership, Backend and Native JSON/WeBWorK contracts, the two scoring algorithms, and import/export/
QTI boundaries. The uncertainty log and linked HG, terminology, design, data-policy, and lifecycle
sections were checked where they supplied authority for those rules.

Targeted source reads established what the questioned compiler limits, Bloom request, Pool
attestation controls, and statistics counters actually implement. This is a documentation and
static-source review, not a runtime audit or a claim that every historical document is clean.
No code, schema, or application tests were changed. Documentation gates check consistency and
links; they do not establish product approval or runtime compliance.

Validation: 451 documentation-format and Markdown-link checks passed. HG and its checklist each
contain 1,239 bullets; diff and consistency checks passed. `git diff --check` passed.

## Implementation follow-up reconciliation

Neil requested a final reconciliation against all Class 3 findings, the earlier authority audit,
the fork-model audit, and other known drift. Each row below names its disposition in
[TODO.md](../../TODO.md#question-spec-implementation-follow-up). Documentation resolution means
that intended behavior is clear; it does not mean the implementation task is complete.

### Desired outcomes

- **Pool Save:** When the set of Question Revision Tuples in a Pool changes, saving
  advances the Pool's Edit Number.
- **Pre-release mismatch:** save unfinished Assessment work and explain the specific problem.
  Requesting three Questions from two valid Pool members blocks release, not Save. Established
  restrictions after issue still apply.
- **Bloom edits:** the owner or Sysadmin can correct either dimension while the other keeps its
  value, including NULL. This ordinary metadata edit preserves the Question Revision Number.
- **Question metadata:** Question metadata belongs on the Question record, not in Native JSON.
  Native JSON contains the Question content needed to display and grade it. The metadata spec
  defines the fields; citation format remains deferred.
- **Statistics:** delivery contributes to times received even without submission. A graded
  response contributes its stored Backend credit, so `0.60` contributes 60% even if Assessment
  settings award zero points. Keep statistics separate per Question Revision and per Pool.

### Finding dispositions

| Finding | Intended behavior and follow-up |
| --- | --- |
| R01: Type classification vs execution | Backend Type is editable classification; Native JSON interaction changes remain source changes. The Type-correction TODO covers existing restrictions and Pool mismatch reporting. No additional Type or interaction is commissioned. |
| R02: Bloom completion gate | Either dimension can be corrected while the other remains unchanged or NULL. The ordinary Bloom editing TODO names validator, schema, API, forms, permissions, concurrency, and tests. A request may carry both existing values without requiring both to be populated. |
| R03: Question correction and Pool mismatch | Save valid Question metadata, report the resulting mismatch, and apply release rules. The metadata/mismatch TODO now distinguishes admission checks from missing current-member release checks. |
| R04: Separate Pool certification | When the set of Question Revision Tuples in a Pool changes, saving advances the Pool's Edit Number. The Pool Save TODO explicitly removes the checkbox, required flag, and attester/time storage across layers and tests. |
| R05: Conditional Backend feedback | Transient unless the Backend provides robust preservation. The blanket spec restriction is corrected. No implementation that provides robust preservation but is blocked by that removed rule was identified; no speculative storage task is added. Feedback timing remains deferred. |
| R06: Statistics collection | Count deliveries and graded responses separately; use stored Backend credit per Question Revision and Pool. The statistics TODO covers submission-only delivery counts, cross-Revision reads, derived Pool outcomes, API/display, and tests. Extra internal counters are not automatically defects. |
| R07: Native compiler limits | Current counts, geometry, and matching shapes remain Class 2 implementation limits. Relabeling their authority requires no compiler change without a demonstrated content/workflow problem. |
| R08: Superseded statistics question | Q32 is closed by Q33-Q36. Documentation-only correction; no code counterpart. |
| R09: First vs later publication | Preserve first publication and same-ID new Revisions. Both routes already exist. Corrected documentation scope; no missing route was found or invented. |
| F01/F03: Pool references and pre-release Save | Ordinary Pool references, explicit forks, and release validation are settled. Pool model and Pool-use Save TODOs cover forced copies, attachment constraints, count gates, UI, and tests. |
| F02: Draft requirements | Drafts save incomplete or broken work; publication validates. Draft autosave and shared Backend authoring TODOs cover the existing save/import/preview gates and verification. |
| F04/Q31: Native metadata duplication | All Library and lifecycle metadata belong to the Question record; the named fields are examples, not exhaustive exceptions. The complete-Revision/metadata TODO now explicitly covers removing duplicate source fields, callers, examples, fixtures, tests, and readback after metadata edits. |
| F05: WeBWorK Type | Manual assignment now; detection deferred. The Type TODO covers ordinary editable classification, without commissioning detection. |
| F06: Fork license | Withdrawn finding. A fork starts with the source license; no license choice or compatibility-change TODO. |
| F07: QTI mapping and feedback timing | Content conversion and Assessment disclosure are separate. The unsupported spec gate was removed; no corresponding implemented timing gate was identified. Handoff remains deferred, so no replacement mapping or timing mechanism is specified. |

### Confirmed Save and release evidence

The current [assessment_pool_forks.sql](../../../schemas/base_schema/50_functions/assessment_pool_forks.sql)
member-save path raises if the selection count exceeds the resulting membership, even before
release. [assessment_pool_selection.sql](../../../schemas/base_schema/50_functions/assessment_pool_selection.sql)
also rejects a larger count immediately. These are concrete Save gates to change, not hypothetical
edge cases. The TODO retains post-issue restrictions while permitting incomplete unreleased work.

The inspected [published_question_metadata_operations.sql](../../../schemas/base_schema/50_functions/published_question_metadata_operations.sql)
does not veto a classification correction because of Pool membership.
[question_pools.sql](../../../schemas/base_schema/50_functions/question_pools.sql) checks
Discipline/Subject on admission and explicitly leaves later reclassification alone. The inspected
[assessment_release_validation.sql](../../../schemas/base_schema/50_functions/assessment_release_validation.sql)
counts all Pool members for insufficient membership, without checking current classification.
The correction is to complete mismatch reporting and release validation, not to claim a Question
edit veto was found. The TODO gives the expected save/report/release behavior and verification.

### Fork audit coverage

Every correction in [FORK_MODEL_CODE_AUDIT_2026_10_05.md](FORK_MODEL_CODE_AUDIT_2026_10_05.md)
has a later implementation item:

| Audit evidence | TODO coverage |
| --- | --- |
| `assessment_question_pool_fork`, unique Pool use, required attachment, source-parent trigger | Remove special association/attachment rules; use ordinary Pool references and Instructor ownership. |
| Blueprint single-use/fresh-fork rules, Course adoption copies, `BlueprintPoolInputChoice`, Assessment-owned views | Pool model alignment and special-association TODOs cover SQL, Rust, browser consumers, generated types, tests, and fixtures together. |
| Question/Draft/Blueprint source tables | Put parent references on ordinary records; preserve exact source Revision, immediate parent, normal creation, and child Revision 1. |
| Retry data in Draft source and Blueprint receipt tables | Preserve ordinary repeated-request handling while simplifying storage. This is engineering work, not a new fork lifecycle. |
| Pool fork omits Bloom | Copy existing metadata, including Bloom, then permit independent parent/child edits. |
| Member order treated as state; Edit Number mistaken for history | Pool model TODO verifies display-only sorting and ordinary Pool saves; Student Work keeps exact delivered Question references and Pool ID/Edit Number. |
| Tests/fixtures enforce the old model | Update them with the implementation; regenerate API/schema documentation. A passing old assertion does not approve that behavior. |

### Other known drift coverage

The existing TODOs retain Archive alignment, manually written notice removal, optional Question
language, ordinary Revision metadata, owner/Sysadmin permissions, full Sysadmin authority,
Assessment partial-credit handling, each settled Native scoring change, WeBWorK Draft authoring,
and Blueprint Theme inheritance. The ORDER TODO's stale "default unspecified" sentence is now
corrected to the already-settled enabled default.

The earlier [alignment report](question_specs_alignment_report.md) also named gaps that needed
clearer routing. TODO now covers shared Library fields lost between result and browser row,
Question-only Type text matching despite the shared Type filter, supported import/Blueprint
assembly verification, and stale saved-search terminology. The current targeted search found
saved-search comments/test names in
[question_search.rs](../../../crates/question_model/src/question_search.rs), not a saved-search
table. The earlier storage claim is narrowed accordingly; a filter serialization type is not
itself permanent storage. Existing removal of the unapproved Course-use filter retains its
pending database/browser checks.

Mixed batch picker selection and Instructor bulk-edit transaction behavior are not inferred from
combined discovery. Their existing implementation shapes do not establish extra product rules.
The revised [API_CONTRACTS.md](../../API_CONTRACTS.md) also describes Bloom as ordinary metadata
editing, replacing its stale description of a dedicated complete-pair correction workflow.

### Remaining uncertainty

No unresolved product choice prevents these corrected specs from stating the supported model.
The existing deferrals in
[question_specs_open_questions.md](../decisions/question_specs_open_questions.md#explicitly-deferred-decisions)
remain explicit, including converter handoff, optional feedback timing, bulk editing, AI assignment,
NC/ND content, and regrading. The initial organized search filter remains tentative. This pass
settles none of those by implementation inference. Runtime compliance and the listed later
verification work remain open; this is a complete routing of known findings in these reviews,
not a claim to have discovered every possible defect in PLE.

Reconciliation validation: all 451 documentation-format and Markdown-link checks passed again.
HG and checklist remain aligned at 1,239 bullets each; diff, consistency, and whitespace checks
passed. This follow-up changed documentation only; application, schema, UI, and test corrections
remain the explicit later work in TODO.

## Supporting-document alignment after the HG wording review

Checked the Question specs and the related terminology, identity, Assessment lifecycle,
architecture, database, concurrency, security, privacy, FAQ, and decision documents against
HG and Neil's latest wording decisions. Corrected these remaining inconsistencies:

- Revisions are complete records. Their saved content stays fixed; permitted metadata edits
  preserve the Revision Number. Removed descriptions of Question metadata as a separate object.
- Pools contain unordered sets of Question Revision Tuples plus metadata. Save and Edit Number
  explain changes. An Assessment asking for too many Questions is distinct from a Pool violating
  its own requirements.
- Library Object remains the shared product term. Blueprint Course Revision Tuple remains the
  explicit name for the Blueprint Course ID and Blueprint Revision Number. Existing code
  identifiers remain implementation evidence.
- Question metadata belongs on the Question record. Citation format remains deferred.
- Statistics cover actual Question deliveries and stored credit separately per Question Revision
  and per Pool. The initial Library search default remains tentative.
- Sysadmin authority includes administrative access to Course and Student records; access to
  FERPA-sensitive Student data requires confirmation and is recorded for audit.

Existing TODO items cover complete Question records, Pool Save and Assessment release checks,
statistics, Native JSON metadata, and Sysadmin access. This pass changes supporting documentation;
implementation work remains in TODO. Historical reports, changelogs, and generated schema
evidence retain their descriptions of what existed at the time.

Validation: all 1,237 HG/checklist bullets align; 451 documentation-format and Markdown-link
checks passed. `git diff --check` passed. No code or runtime tests were changed.
