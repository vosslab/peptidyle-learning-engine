# Question specification authority audit

## Result and scope

This audit originally reported seven findings; six required documentation corrections and the
fork-license finding, F06, was withdrawn after Neil corrected its premise. Some are conflicting
instructions left after the refactor; others are restrictions or workflow choices without a clear
human decision. Finding a rule in an agent-written contract or decision log does not establish
that Neil approved it. Recent changes to HG also require checking against his actual reply.

The review covered the 33 documents in [QUESTION_SPECS/README.md](../../QUESTION_SPECS/README.md),
the BiologyProblems.org specification set, the Blueprint import specification, and relevant
sections of their linked lifecycle, API, terminology, concurrency, and design documents.
Authority was checked against current [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md),
[FALL_2026_PILOT.md](../../FALL_2026_PILOT.md), Neil's replies in this conversation, and the decision
records. Committed HG was also read to check the provenance of the recent Pool changes.

This was a documentation audit, with no application or schema changes. The later human
clarifications and subsequent cleanup correct six findings and withdraw F06, as
recorded below. Implementation reconciliation remains in TODO; documentation corrections do
not establish runtime compliance.
Line numbers describe the files inspected on October 5, 2026.

## Findings in priority order

### F01: Pool ownership correction changed the add workflow

**Priority: high. Status: documentation corrected after Neil's clarification; runtime unverified.**

The review first found inconsistent instructions about referencing or automatically forking a
Pool when it was added to an Assessment. Neil initially reaffirmed automatic forking, and that
answer was applied. He subsequently supplied a replacement model: adding references the existing
Pool; forking is explicit when an Instructor wants independent state. Q29 records this later
instruction as superseding the earlier answer.

**Applied correction:** all Pool behavior is consolidated in
[QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md). Assessments reference existing
Pools and store their own selection counts. Pools retain Instructor ownership; explicit forks are
ordinary Pools. Current Pool edits affect future selections across references, and existing
Attempts retain their selected Questions. Code/API/schema/UI/test reconciliation is in TODO.

### F02: Import inherits publication requirements too early

**Priority: high. Status: documentation corrected after Neil's clarification; runtime unverified.**

Neil subsequently stated: "drafts are drafts and have no requirements." HG and the linked Draft,
Backend, metadata, Type, Native JSON, WeBWorK, and import specs now allow empty, incomplete, or
broken Draft content to be created, imported, and saved. Publication checks complete source and
required metadata. Implementation work is recorded in [TODO.md](../../TODO.md).
The excerpts below preserve the conflict found by this audit, before that correction.

- [QUESTION_BACKEND_SPEC.md](../../QUESTION_SPECS/QUESTION_BACKEND_SPEC.md), line 53, requires
  complete supported source before "publication or import."
- [QUESTION_TYPE_SPEC.md](../../QUESTION_SPECS/QUESTION_TYPE_SPEC.md), lines 12-13, requires
  import to reject a missing Type while also allowing incomplete Drafts.
- [QUESTION_LIBRARY_METADATA_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_METADATA_SPEC.md),
  lines 14-16, applies required-field validation to import and API writes without distinguishing
  Draft saving from Library entry.

HG, lines 1073-1083, establishes import or write a Draft, preview, test, refine, complete metadata,
then publish. [DRAFT_QUESTION_SPEC.md](../../QUESTION_SPECS/DRAFT_QUESTION_SPEC.md), lines 25-26,
correctly allows incomplete metadata while saving a Draft.

**Why it matters:** the broader wording can prevent an Instructor from importing content into a
Draft to fix its source or complete its classification. That repeats the authoring problem Neil
already corrected.

**Correction direction:** distinguish accepting content into a Draft from admitting a Published
Question to the Library. Preserve ordinary input-safety checks; apply complete publication
requirements at publication. No new import lifecycle or Backend-specific exception is needed.

### F03: Pool requirements became a universal edit rejection

**Priority: high. Status: documentation corrected after Neil's clarification; runtime unverified.**

Neil explicitly chose saving incomplete unreleased Assessment work and validating at release.
An entry requesting more Questions than available creates a Pool-use mismatch for that Assessment;
the Pool may still be valid. Membership saves proceed, the specific problem is shown, and release
is blocked until resolved. Post-issue restrictions remain. Q28 records the decision and reason.
The following paragraphs preserve the original finding, not the corrected specification.

[QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md), lines
49-51, makes insufficient membership an invalid removal for an affected use.
[QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md), line 50,
goes further: "Reject the edit; do not store the invalid membership state."
[QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md), line 27, applies
post-issue rules wherever the Pool is used.

HG requires enough members for an Assessment's selection count, explains mismatch, blocks an
affected release, and lets already-released Assessments continue as-is (lines 1289-1290 and
1316-1322). Neil called too few members a valid concern while saying where to put the checks was
unclear. The interview log subsequently describes that placement as an implementation matter and
adds a blanket prevention rule; it is not evidence of a separate human choice.

**Why it matters:** a shared Pool may be used by many Assessments with different selection counts.
Treating every use as a veto on saving its membership creates a broader restriction than checking
whether an Assessment can release. It also differs from keeping temporary edits in browser memory.

**Correction direction:** preserve the approved membership, fairness, and release requirements.
Separate them from the unconfirmed timing and scope of write rejection. Specify that scope before
implementation needs it, rather than treating the current example as human-approved behavior.

### F04: Native JSON still blurs editable metadata and fixed source

**Priority: high. Status: documentation corrected using Q31; implementation remains in TODO.**

HG and the Native JSON, metadata, lifecycle, and linked contracts now put shared metadata on the
complete Question record. The source field table and example show Backend-owned content. Existing
duplicate metadata fields are explicitly labeled implementation evidence. The original finding
below records the contradiction before this correction.

[NATIVE_JSON_SPEC.md](../../QUESTION_SPECS/NATIVE_JSON_SPEC.md), lines 7 and 16-22, describes a
frozen source document containing Title, Description, and Tags. Lines 112-114 require a new
Revision for a source change. It does not explain how an ordinary Title or Tag edit fits that
description. [CONTRACTS.md](../../CONTRACTS.md), line 186, separately retains a rule that Tags are
seeded from source once and then live as separate current metadata. The lifecycle document also
retains "lineage metadata" wording at lines 68-69.

HG, lines 1022-1023 and 1229-1240, now says a Revision is a complete record and those fields may
change on the current record without another Revision. The owning
[QUESTION_REVISION_SPEC.md](../../QUESTION_SPECS/QUESTION_REVISION_SPEC.md) reflects that decision.

**Why it matters:** the remaining instructions can lead to either unnecessary Revisions for
metadata corrections or competing copies of the same Title, Description, and Tags. A developer
should not have to invent which copy is authoritative.

**Correction direction:** align the Native JSON explanation and linked contracts with the complete
Question record and its permitted field edits. Label the existing source serialization and
metadata split as implementation evidence where relevant. Storage changes remain later code work;
this audit does not prescribe a replacement schema.

### F05: WeBWorK still forbids Type detection

**Priority: medium. Status: documentation corrected; automatic detection explicitly deferred.**

The WeBWorK spec and frontend architecture now distinguish Backend-owned rendering and grading
from editable Question Type classification. Neil then explicitly directed manual WeBWorK Type
assignment for now and deferred automatic detection. HG and the Type spec record that direction.
The original finding follows; the earlier blanket prohibition is superseded by an explicit deferral.

[WEBWORK_SPEC.md](../../QUESTION_SPECS/WEBWORK_SPEC.md), lines 12-14, says PLE does not "infer
Question Type." HG, lines 1104-1108, and
[QUESTION_TYPE_SPEC.md](../../QUESTION_SPECS/QUESTION_TYPE_SPEC.md), lines 14-19, permit reliable
detection from source to help supply the editable classification tag. Neil specifically raised
PGML detection as easier than legacy PG detection.

**Why it matters:** the blanket ban discourages metadata automation Neil wants and confuses
classifying source with taking over the Backend's rendering or grading.

**Correction direction:** preserve Backend execution ownership while allowing the stated source
classification assistance. This does not make building a detector a current delivery requirement.

### F06: Fork-license finding withdrawn

**Status: withdrawn after Neil's correction.**

A Question fork copies the source Question, receives a new ID and source pointer, and starts
with the same license. There is no license choice during the fork operation.

The audit incorrectly treated compatible-license wording as a requirement to allow a different
license for a fork. That was a misunderstanding. The proposed compatibility change and its code
TODO have been removed; this finding calls for no implementation change.

### F07: Conversion depends on unsettled feedback timing

**Priority: medium. Status: documentation corrected; runtime unverified.**

The reviewer cleanup separates Question conversion from Assessment disclosure timing and leaves
the converter handoff format explicitly undecided, as HG requires. The exact Native JSON mapping
table was also removed. The paragraphs below preserve the original finding before correction.

[QTI_INTERCHANGE_SPEC.md](../../QUESTION_SPECS/QTI_INTERCHANGE_SPEC.md), line 42, says correct
answers and feedback map only when "timing and meaning are exact."

HG places disclosure at the Assessment level and explicitly defers optional Question Feedback
timing (lines 1745-1762). It assigns conversion to the external Rust package. No decision requires
matching another LMS's feedback timing before its Question content can enter a PLE Draft.

**Why it matters:** the wording can make otherwise usable Question conversion depend on an
unsettled Assessment behavior. It adds a gate unrelated to preserving the Question's content.

**Correction direction:** keep content fidelity, accurate conversion-limit reporting, and external
conversion ownership. Treat Assessment disclosure separately from Question content mapping.

## Other inconsistencies to retain

These are lower priority and are not a new interview agenda:

- [ASSESSMENT_LIFECYCLE.md](../../ASSESSMENT_LIFECYCLE.md), lines 136-141, disables Bloom sorting
  if any entry lacks a complete pair. HG permits missing Bloom indefinitely and wants it useful
  for sorting. The all-entries gate has no identified human source in this review. Label it as
  current implementation behavior; this audit does not choose where unclassified items belong.
- The earlier Pool-fork Bloom ambiguity is resolved by Q29's explicit copying of current Pool
  metadata. The consolidated spec and HG preserve existing Bloom fields on Pool forks. Initial
  AI assignment remains deferred; copying metadata introduces no new classification operation.


## Supported rules and audit limits

The combined Library Object model, ordinary Pool identity, unordered distinct members, common
Pool Type/Backend/Discipline/Subject, own Pool search metadata, owner/Sysadmin editing, nullable
Bloom, generated IDs, the settled Native JSON grading formulas, stored Backend credit, and
Assessment control of awarded partial credit have direct support. They are not findings merely
because the implementation is incomplete.

Existing request names, size limits, counters, and parsing details are not automatically invented
product behavior. Where they are clearly labeled implementation evidence and do not conflict with
settled intent, this audit does not demand a human decision for each one. Historical reports are
not current product authority. Archived Questions inside Pools remain set aside as requested.

The repeated failure is broader than wording: an agent expands a rule, records the expansion as
settled, and later cites that record as approval. The original F01 review showed why even a recent HG edit needs provenance; Q29 now
records the later explicit replacement decision. Future corrections must trace a changed behavior to the human statement, while
keeping implementation choices and open interpretations visibly separate.

No runtime behavior was tested. Documentation format and link checks establish document hygiene,
not approval of these rules or application compliance. This audit does not clear unrelated PLE
specifications or every historical HG bullet.

## Follow-up before another interview

A bounded recheck of the question log, Question specs, and linked lifecycle guidance found:

- Q11 still used "Assessment-owned forks" in two resolved entries. Corrected both to ordinary
  reusable Pools under Q29. This requires no new product decision.
- The Published Question and Library specs treated Archive as blocking new selection, although HG
  only names its effect on shared availability. Removed that unsupported certainty. Q25 still
  sets aside archived Questions inside Pools; no replacement selection behavior was chosen.
- The Assessment lifecycle spec described the existing all-entries-classified Bloom sorting gate
  as a requirement. Labeled it implementation evidence and recorded review in TODO. No sorting
  placement policy was chosen. Also replaced residual "reusable source Pool" wording with the
  referenced Pool's own fields, independent of its parent and members.
- HG mentions "impact notices" without defining their purpose. The existing SQL stores notice
  text and provides create, update, and cancel operations. Earlier discussion focused on who
  could manage them. The interview now asks whether manually written notices belong in scope
  before considering those permissions. Evidence:
  [library_discussion.sql](../../../schemas/base_schema/20_tables/library_discussion.sql) and
  [library_discussion_operations.sql](../../../schemas/base_schema/50_functions/library_discussion_operations.sql).

This recheck does not certify every statement in the doc set. Existing deferred topics remain
set aside, and implementation presence is evidence of behavior rather than product approval.

**Notice question resolved (Q38):** Neil chose fork-and-fix and rejected creating a social-media
platform. Manually written notices are removed from intended scope. HG and current specifications
are aligned; removing the existing notice implementation is recorded in TODO.

## Stored notice history

The history trace used Git pickaxe searches across available refs and inspected the matching
changes. Dates below use the recorded Chicago offset. These are commit times, not necessarily
the times individual edits were made.

| Date | Commit | Recorded change |
| --- | --- | --- |
| August 29, 2026, 11:47 CDT | `71bf53b7` | First matching HG addition names notifications for versions, forks, improvement threads, and impact notices. It gives no notice-authoring workflow. |
| September 16, 2026, 09:02 CDT | `eecc03ac` | HG repeats the notice language for Published Questions and Pools. |
| September 17, 2026, 10:02 CDT | `59d54560` | Adds the stored notice table and create/update/cancel SQL, alongside improvement threads and replies. The new design decision describes retained text, Question-owner/Sysadmin administration, Sysadmin-only Pool administration, and retained cancelled notices. |
| October 4, 2026, 20:41 CDT | `a28afc75` | Removes improvement threads and posts but retains impact notices. The revised design decision leaves notice administration unresolved. |

The September 17 design rationale says reusable content needs a "visible stewardship conversation"
and "durable impact history." That is the recorded implementation rationale, not evidence of
Neil approving a social feature. The same commit's changelog calls these boundaries "settled."
The inspected earlier HG text supplies the term, but not those storage, authorship, cancellation,
or retention rules. This supports the concern that a brief phrase was expanded into functionality;
Git authorship alone cannot identify who wrote or approved the original sentence.

The current implementation still contains:

- [library_discussion.sql](../../../schemas/base_schema/20_tables/library_discussion.sql): stored
  body text, author, affected Revision, timestamps, active/cancelled state, and cancellation data.
- [library_discussion_operations.sql](../../../schemas/base_schema/50_functions/library_discussion_operations.sql):
  create, update, and cancel operations with Question-owner or Sysadmin checks; Pool notices use
  Sysadmin authority.
- [library_discussion.rs](../../../crates/server/src/library_discussion.rs): HTTP create, update,
  and cancel routes, wired by [composition.rs](../../../crates/server/src/composition.rs).
- [library_discussion.rs](../../../crates/learning-data-access/src/postgres/library_discussion.rs):
  PostgreSQL access for those operations.

Q38 now removes this feature from intended scope. The code removal belongs to the existing
focused TODO. This was static source/history inspection; it does not establish runtime delivery
or which browser controls expose these operations.

## Archive and statistics clarification

Q39 now settles Archive as a state on the ordinary Question: read-only, excluded from normal
discovery, existing references preserved, and available to restore or fork. The final short rule
supersedes the earlier open Archive note and intermediate proposals for more workflow rules.

The search default remains explicitly tentative. A further check found cross-Revision statistics
still promoted in terminology, design decisions, and the data policy even though the Question
Library spec had been corrected. Those authorities now use per-Revision statistics; the former
rollup rule is removed from intended behavior and implementation follow-up is in TODO. This
illustrates why checking only the newly split specs is insufficient.
