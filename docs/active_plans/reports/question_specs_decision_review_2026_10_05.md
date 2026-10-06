# Question specification decision review

The later [QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md](QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md)
finds remaining conflicts and unsupported choices, including overreach in the Pool add-workflow
correction. Statements below about completed alignment describe the earlier pass, not clearance
of all current specifications.

## Scope and reason

Neil questioned whether the refactor had recorded agent assumptions as settled product decisions.
This focused review checks the shared Library model, Question and Pool lifecycle, import workflow,
metadata ownership, and the remaining interview questions against HG and his recorded replies.
It is not runtime verification or a claim that every statement in all 42 specs has been cleared.

Primary evidence is [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md),
[FALL_2026_PILOT.md](../../FALL_2026_PILOT.md), and Neil's direct replies recorded in
[question_specs_open_questions.md](../decisions/question_specs_open_questions.md).
Existing code explains implementation; it does not by itself establish approved product behavior.

## Findings and corrections

| Finding | Evidence and consequence | Documentation correction |
| --- | --- | --- |
| This review initially suggested separate creation workflows by Backend | Neil corrected this directly: "Questions are backend agnostic, period." The review had mistaken implementation coverage for product rules. | Withdraw that finding. All Questions follow the same Draft, publication, ownership, Revision, Library, and Assessment rules. Backend source and execution handling stay behind the common interface. |
| WeBWorK creation support was described too broadly as missing | Current PG/PGML content-loading tools create Drafts and call the shared Question publisher. The browser-facing creation route alone is limited to Native JSON. | Correct the import spec and implementation report; record the route limitation in TODO.md without implying rendering, grading, or publication are absent. |
| Import displaced ordinary authoring in the explanation of Questions | Neil emphasized that Drafts exist to write and test Questions before publication; not all Questions are imported. | Lead the spec index and Draft spec with that workflow. Import supplies content to the same backend-agnostic model. |
| Existing preview code could be mistaken for complete Draft testing | Historical editor docs describe preview; current WeBWorK Library inspection loads a Published Question Revision. Neither proves the current browser can author and test an unpublished PG/PGML Draft. | Specify rendering and response checking as ordinary Draft authoring. Record the browser workflow gap in TODO.md; keep existing renderer support distinct from authoring completeness. |
| The import specs required a running HTTP service as the test boundary | Neil requested supported APIs and generated PLE identities, then explicitly limited this work to observable behavior and spec drift. No particular test transport was selected. | Keep API creation and readback requirements; leave transport and harness decisions to implementation work. |
| Source import acquired a mandatory record schema and status vocabulary | The source spec required a read-only record for every attempted item with fixed fields and statuses. Course assembly depended on an `Imported` status. These details were not established by HG or Neil's decisions. | Retain source attribution, returned identities, content mapping, and visible omissions without prescribing a new tracking format or status system. |
| A fork Draft was described as having its own revision history | HG says Drafts use current state rather than immutable Revisions. | State that Question Revision history begins at publication. |
| Missing Blueprint Theme support became a product question | Neil wants Blueprint Course colors and expects a Course to start from its Blueprint. HG already gives Instructors independent Course Theme control. | Close Q05; document the starting Theme and put missing implementation support in TODO.md. |
| Picker limitations became an interview choice about treating Questions and Pools separately | The combined Library Object model and shared Assessment discovery are settled. No approved restriction requires separate additions. | Remove Q06 from the interview agenda; retain implementation review without adding a new product gate. |
| The invented phrase `Library Pool` remained in two specs | Neil explicitly rejected that separate category. | Use Question Pool or source Pool. |
| Blueprint import repeated stale Course completeness and topic-grouping claims | Later decisions establish topic-based Assessments and the initial Course set. | Reference the current pilot and source inventory; state the topic-based starting rule. |

QTI documentation also required retaining a fixed list of private import evidence and declaring
transaction outcomes before a format could ship. The review removed those extra requirements;
conversion ownership, accurate support claims, asset safety, and reporting conversion limitations
remain intact.

## Rules with direct support

The review found direct HG support for the following; it did not reopen them:

- Questions are backend agnostic; Neil explicitly reaffirmed this during the review.
- Questions and Pools are first-class Library Objects in one combined search.
- Pool members are distinct Published Questions pinned to exact Revisions, with common Type,
  Backend, Discipline, and Subject.
- Pool membership is unordered; spreadsheet sorting is display-only.
- Pool forks are Pools, with independent current state and source references.
- A Pool owns its discovery metadata and has an owner rather than a separate Author.
- Bloom can remain absent while AI assignment is deferred, without a deadline.
- A mismatch blocks affected new release; already-released Assessments continue as-is.
- Native JSON regrading, feedback timing, NC/ND support, and Instructor bulk editing are deferred.

These findings distinguish actual errors from rules already supported by the primary authority.
They do not independently prove the authorship of every historical HG bullet.

## Complete-record and Pool corrections

Neil's later review found broader drift than the earlier interview uncovered:

- The specs made Pool forks belong to Assessments and granted Course-derived edit authority.
  Pool forks are regular reusable Pools with Instructor owners and parent-Pool pointers.
- Question metadata was split from the Question Revision as a separate conceptual object.
  Each Revision is a complete Question record; permitted fields on the current record may be
  edited in place. Source and grading content remain fixed under the existing publication rules.
- Type was made specially immutable for all Backends. Native JSON has a built-in interaction;
  other Backends use an editable classification tag. Required Type remains required.
- Bloom received a separate counter and correction subsystem. It follows ordinary metadata editing,
  like Title. Existing tables and APIs are implementation drift, not product authority.
- Pool Edit Numbers were described as if they recovered earlier selection context. A Pool Save
  replaces current state, with no undo. The counter serves concurrency; exact selected Question
  references preserve delivered content.

HG, the owning specs, and current contract summaries now reflect these corrections. The schema,
application, generated schema tables, and historical audit evidence remain unchanged; focused
implementation tasks are in TODO. Archived Question behavior inside Pools is set aside in Q25.
This is a focused correction pass, not a claim that all other decisions are verified.

## Remaining limits

### Grading assumption found in the renewed grill

The Native JSON spec copied the current zero-or-one grader into its grading rules without a
recorded decision that Matching should be all-or-nothing. Neil selected proportional Matching
credit: correct pairs divided by all prompts, equal weight, wrong or blank pairs zero, no extra
deduction. The Question Type and Native JSON specs now state that intended rule. The shared
grading result accepts fractions, but the current Native JSON grader returns only zero or one
and its Matching response validator rejects incomplete responses. Those implementation gaps
are in TODO; no grading code was changed or runtime tests run. Neil settled Native JSON Multiple Answer scoring as
`max(0, (TP - FP) / C) * min(K, C) / max(K, C)`, with empty selection earning zero. The dedicated
scoring spec owns the formula and examples. This linear choice-count rule replaces the squared
proposal and resolves its score decrease when a correct selection was added. Selecting all ten
with eight correct intentionally earns 60%. Assessment Instructors control whether partial credit
is awarded; Backends own their grading. Other Types are not settled by the MA formula.

### Further grading review

The later Backend clarification applies to Matching as well as MA: the Native JSON formula
applies when Assessment partial credit is enabled. Corrected the unqualified Matching wording
without reopening the accepted equal-credit rule. Also corrected a historical log sentence that
still called the superseded MA average formula current.

Neil resolved Q19 by defining MULTI-FIB as multiple FIBs, independently graded. He also
confirmed MC and HOTSPOT all-or-nothing, NUM tolerance, and FIB accepted answers with regex.
Native JSON's binary MULTI-FIB grading and missing regex matching remain implementation gaps.
Neil subsequently settled ORDER partial credit as the equal-weight average of correct absolute
positions and correctly ordered pairs. DABC earns 25% against ABCD. The dedicated
[ORDER_SCORING_SPEC.md](../../QUESTION_SPECS/ORDER_SCORING_SPEC.md)
owns the formula, pair-counting explanation, examples, and required properties. One-time
exact-arithmetic checks reproduced every supplied table across all 5,910 permutations for three
through seven items. Current binary ORDER grading remains an implementation gap in TODO.

### Stored credit clarification

Neil corrected another mistaken interview premise: changing the Assessment partial-credit setting
does not require Backend regrading or a post-start lock. PLE always stores the earned fraction.
The current setting decides whether a fraction below one earns points or zero; full credit earns
full points either way. Apply a setting change to every Attempt, including submitted Attempts,
using stored fractions. His reason is fairness through the same rule for all Students.

The earlier phrase "when partial credit is enabled" refers to awarding points, not whether the
Backend calculates and PLE stores the fraction. The Backend, Native JSON, type-specific scoring,
and Assessment documents now state this distinction. The initial setting default remains open.

The response-saving wording also needs a concrete definition: valid MATCH and MULTI-FIB responses
can contain unanswered parts. Saving and grading those responses follows the settled rules that
answered parts earn credit and blank parts earn zero. Code alignment remains in TODO.

### Follow-up source check

The renewed grill found three remaining documentation errors despite the earlier report:

- The Published Question spec still assigned revision history to a private fork Draft. Corrected
  it to start Question Revision history at publication, consistent with HG and the fork spec.
- The alignment report still called authenticated HTTP import a missing guarantee. Corrected it
  to supported PLE API creation and readback without prescribing transport.
- The alignment report claimed archive confirmation was restored, but the Published Question
  spec omitted it. Restored HG's separate Danger Zone, explanation, and clear confirmation.

The ownership spec also points ordinary search-metadata editing to a deferred bulk-edit API and
warns against assuming owner-only editing. HG's rule that metadata edits preserve the Revision
does not establish who may make them. Neil resolved this for Question search metadata: owner and
Sysadmin can edit it, probably an AI backend too, because Instructors will do the bare minimum
on metadata writing. Other Instructors do not gain that permission from the deferred bulk-edit
design. For Bloom, Neil supplied "The owning Instructor can correct either Bloom dimension"
and emphasized that only the owner changes a Question while any Instructor may fork it.
Removed the unsupported read-access editing permission and categorical Sysadmin prohibition.
Neil subsequently clarified the complete rule: every Instructor may read, add to an Assessment,
or fork any Published Question; the owning Instructor and Sysadmins may edit it. Sysadmins have
full administrative authority across PLE. That supersedes both a metadata-only Sysadmin permission
and the old restriction to critical-flaw corrections. Deferred administrative tools do not narrow
that authority. Implementation follow-up remains in TODO; this is not evidence that code checks
are corrected.

The open-question log is not an approval record for every sentence in the new specs. In particular,
an empty interview queue does not prove that the documentation is free of unsupported assumptions.
Backend capability filters remain an engineering review item; their presence in code alone is not
approval for a new user-facing filter or a restriction on Pool discovery.

Concrete existing API routes, payload bounds, and storage counters are implementation references.
They must not silently narrow the product model. This review does not certify their runtime
correctness or reconsider every pre-existing technical constraint.

All changes in this pass are documentation changes. Further application work belongs in
[TODO.md](../../TODO.md).

## Documentation verification

- Guidance-format and Markdown-link checks: 457 passed.
- HG and checklist wording/order and duplicate consistency: 1,203 bullets on each side.
- New backend-agnostic and Draft-authoring checklist entries remain unchecked; documentation is
  not evidence that the browser workflow is complete.
- `git diff --check` passed. No full-stack build or runtime test was run for this documentation pass.

## Current source check

- [authoring.rs](../../../crates/server/src/authoring.rs): `create_draft` checks the Native JSON
  media type and passes `QuestionFormat::PleQuestionJson` to Draft creation.
- [authoring.rs](../../../crates/learning-data-access/src/authoring.rs): shared Draft input and
  validation support both PG and PGML source bindings.
- [parameterized_publication.rs](../../../crates/project-tools/src/curriculum_content/parameterized_publication.rs)
  and [publication.rs](../../../crates/project-tools/src/pilot_content/publication.rs): existing
  loaders create Drafts and use `NewQuestionLineagePublisher` for WeBWorK Questions.
- [grade.rs](../../../crates/adapters/webwork/src/lib/grade.rs) and
  [tests.rs](../../../crates/adapters/webwork/src/http_renderer/tests.rs): grading implementation
  and render/grade test coverage exist. No tests of that runtime were rerun in this docs pass.
- [question_library.rs](../../../crates/server/src/question_library.rs): the WeBWorK Library
  preview loads an exact Published Question Revision. It is not an unpublished Draft preview.
- [draft_preview.rs](../../../crates/domain/src/draft_preview.rs): the local preview helper only
  handles Native JSON and does not evaluate responses. That is an implementation limit, not a
  rule that removes preview or testing from other Questions.
- [ple_question_json_editor_hci_brief.md](../../archive/ple_question_json_editor_hci_brief.md)
  and [ple_question_json_editor_implementation.md](../../archive/ple_question_json_editor_implementation.md)
  document earlier authoring-preview intent and work. Historical notes do not prove current
  PG/PGML Draft testing works end-to-end.
