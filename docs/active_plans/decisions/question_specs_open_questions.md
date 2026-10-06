# Question specification open questions

This is the question log requested by Neil while writing the new Question specifications on
2026-10-05. It records gaps encountered while replacing conflicting Question specifications. It is not an
implementation plan or an automatic interview agenda. Neil clarified that the current task is
Question spec drift, not designing import recovery. Existing server details and implementation
work do not become new product questions. If a later grill is requested, select only gaps that
matter to the work then and ask one narrow question in HG's own language.

Authority: [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md),
[FALL_2026_PILOT.md](../../FALL_2026_PILOT.md), and the settled decisions in
[HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md](HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md).
That interview record retains Neil's reasons and expressed commitment. No numerical confidence
score is inferred here. "Open" means no accepted choice was found; it does not weaken settled rules.

## Latest settled-decision review

[QUESTION_SPEC_SETTLED_DECISIONS_REVIEW_2026_10_05.md](../reports/QUESTION_SPEC_SETTLED_DECISIONS_REVIEW_2026_10_05.md)
checks strong claims as well as open entries against exact HG language. It records corrections,
implementation details, preserved settled rules, and already-deferred choices. This pass produced
no new high-impact interview question. Q32 is closed by the later statistics answers; the log's
history is not an instruction to ask superseded questions again.
The follow-up reconciliation routes every Class 3 finding and fork-audit correction to a concrete
TODO or a documented reason that no implementation change follows. It adds no product question.

## Recorded questions and current decisions

The later [QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md](../reports/QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md)
records six corrected documentation findings and one withdrawn finding. F04 uses Q31's shared-
metadata decision; F05 records manual WeBWorK Type assignment with detection deferred. F06 was a
misunderstanding: a Question fork starts with the source license, with no license choice during
forking. Its code TODO was removed. Other implementation follow-up remains in TODO.
Q29 supersedes Q21's automatic-fork answer: adding a Pool references it, and forking is explicit. Q28 settles saving before release.
The audit findings are documentation follow-up, not seven new questions for Neil or instructions
to implement the disputed rules.

| ID | Source and uncertainty | Why it matters | Later question | Status |
| --- | --- | --- | --- | --- |
| Q01 | [QUESTION_IMPORT_SPEC.md](../../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md): API-generated IDs and exact source mappings are settled; a changed source encountered on a later run has no settled rule for updating imported content | Duplicate Questions or unexpected new Revisions can break Course mappings | When previously imported source changes, should the importer prepare an owner-reviewed new Revision, and when should it instead identify a different Question? | Resolved for BiologyProblems.org: after launch PLE is independent and does not track later source changes. No ongoing synchronization requirement. |
| Q02 | [BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md](../../BIOLOGY_PROBLEMS_SPECS/BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md): source topic/problem-set organization is known; full Assessment grouping and selected Pools are not | Determines what a usable full base Course actually contains | For the first complete base Course, what source grouping should define its Blueprint Assessments and which sets should draw from Pools? | Main grouping resolved: one Assessment per website topic, split long topics into parts; Hard Questions usually bonus. Exact content belongs to later Course assembly. |
| Q03 | [QUESTION_LIBRARY_BULK_EDIT_SPEC.md](../../QUESTION_SPECS/QUESTION_LIBRARY_BULK_EDIT_SPEC.md): HG requires bulk edits of shared Library metadata but does not define mixed-kind failure outcomes | A partial save must be visible and recoverable; affects API and UI design | If shared metadata is saved for Questions and Pools together and one target fails, should nothing save or should successful objects save with failures clearly listed? | Deferred: Neil said Instructor bulk editing should wait. Save behavior is deferred with the feature. |
| Q04 | Existing "Used in my Courses" filter | The code was incorrectly treated as product authority | No product question: Neil never approved this filter | Closed: removal written and retained by Neil; remaining live verification is in docs/TODO.md |
| Q05 | Blueprint Theme on Course creation | A missing implementation field was mistaken for missing product intent | No further product question | Closed: a Course created from a Blueprint starts with its Theme and can be changed independently. Missing support is in docs/TODO.md. |

## Lower-priority notes; no decision needed for this documentation pass

Citation format remains deferred until attribution and import are worked through. The term and
its place on the Question record are settled; URL, text, DOI, and structured formats remain possible.
Existing URL/text fields are implementation evidence.

| ID | Source and uncertainty | Why it matters | Later question or prerequisite | Status |
| --- | --- | --- | --- | --- |
| Q06 | Combined Assessment picker | Existing selection limitations must not become new product restrictions | Check the shared interface against the combined Library Object model | Removed from interview agenda: no user-approved rule requires separate Question and Pool additions. Resolve implementation details within the existing workflow. |
| Q07 | Existing Backend capability filter | Its presence in code does not establish product approval or different Pool rules | Check source authority and consistent treatment of the common Backend field | Engineering review only; Questions remain backend agnostic. |
| Q08 | [BIOLOGY_PROBLEMS_SPECS/README.md](../../BIOLOGY_PROBLEMS_SPECS/README.md): initial Blueprint Course selection | Determines initial Course scope without overstating completeness | Which base Blueprint Courses belong in the first delivery? | Resolved: Genetics, Biotechnology, and Biochemistry first; Molecular Biology and Laboratory later. Biostatistics also deferred under Neil's qualified current preference. |
| Q09 | [BIOTECHNOLOGY_COURSE_SPEC.md](../../BIOLOGY_PROBLEMS_SPECS/BIOTECHNOLOGY_COURSE_SPEC.md): Biotechnology source completeness | Prevents an unsupported claim that a full Course is supplied | No remaining completeness question | Resolved: selected Question collection, not a complete Course. Neil and BP.org identify only Genetics and Biochemistry as complete. |
| Q10 | [QUESTION_EXPORT_SPEC.md](../../QUESTION_SPECS/QUESTION_EXPORT_SPEC.md): Pool export and eligible target formats are not fully defined | Flattening a Pool can lose selection meaning, metadata, and exact member references | Which first export workflow is needed, and must it preserve a reusable Pool or produce selected static Questions for a named external system? | Purpose settled: export selected Questions for another LMS using the existing converter. This is the intended purpose, not merely the first use. Selecting a Pool exports its members together in an LMS-importable package; conversion and packaging belong to the external library. |

## Unclear rule found while checking against HG

| ID | Source and uncertainty | Why it matters | Status |
| --- | --- | --- | --- |
| Q11 | "Questions in no Pool" includes membership in any Pool; Pool forks are ordinary Pools | Neil: "forks of pools are still just pools" | Resolved, firm explicit clarification. No special fork category or filter exception. Applied to HG and the specifications. |

## Metadata authority clarified during the renewed grill

- **Q14 current editing rule:** Neil clarified: "only the owning instructor may edit a Published
  Question; Every Instructor can read, add to an assessment, and/or fork any Published Question;
  Sysadmins may edit any Published Question." This applies to Question content and metadata,
  including Bloom. Firmness: explicit current rule; he cannot currently think of other exceptions.
  It supersedes the narrower Sysadmin wording below and does not establish an AI editor role.
- **Q15 Sysadmin authority:** Neil added, "Sysadmins have god powers, I see no way around iit."
  Full administrative authority applies across PLE. Because: he sees no workable alternative.
  Neil further clarified that useful Sysadmin support requires full access, including Course
  and Student records. Removed wording that implied a separate scoped permission boundary;
  support work remains recorded for audit. Firmness: explicit clarification.
  Firmness: direct, unqualified clarification. Unproven Sysadmin tools stay deferred; their absence
  does not narrow administrative authority. Ordinary Instructor ownership rules still apply.

The notes below retain how the discussion reached that rule; Q14 and Q15 govern current scope.

- **Q12 resolved:** A Question's owner and Sysadmin can edit its Title, Description, Discipline,
  Subject, Topic, Subtopic, and Tags. Neil also named "probably an AI backend."
- **Because:** "instructors will only do the bare minimum on metadata writing."
- **Firmness:** owner and Sysadmin were stated without qualification; AI editing is tentative.
  This does not authorize other Instructors to change those fields. Instructor bulk editing and
  AI implementation remain deferred.
- **Q13 resolved:** Neil supplied the wording: "The owning Instructor can correct either Bloom
  dimension." He then emphasized: "only the owner can change a question, but any instructor can
  fork it." Firmness: direct, unqualified correction. The specs had incorrectly expanded "an
  Instructor" into every Instructor with read access. His answer establishes ownership as the
  Instructor editing rule; it does not approve the earlier recommendation verbatim or retract his
  separately stated Sysadmin permission for the search fields in Q12. Initial AI Bloom assignment
  remains deferred. Code changes are recorded in TODO.

## Grading decisions

- **Q16 resolved:** Matching uses correct pairs divided by all prompts. Pairs have equal weight;
  wrong or blank pairs earn zero, with no additional deduction. Three correct out of five earn
  60%. Neil explicitly selected this rule after asking about partial-credit failures in other LMSs.
- **Because:** Neil would like partial credit but is concerned about correctness. The agent
  checked that PLE already stores fractional credit and identified incomplete-response rejection
  and Native JSON's binary grader as implementation gaps. No runtime behavior was verified.
- **Firmness:** explicit selection, unqualified after discussion of concrete cases. This does
  not settle Multiple Answer, Ordering, or other Question Types. Other LMS failures were not
  diagnosed or asserted from their names alone.
- **Q17 history; superseded by supplied rule below:** The initially accepted Multiple Answer rule was the fraction of correct choices selected minus
  the fraction of incorrect choices selected, with a minimum of zero. Neil tested the rule with
  five correct choices among ten and a Student checking all ten: "I think you got it right."
  The result is zero, not full credit. Selecting all five correct and one incorrect earns 80%.
- **Because:** Neil identified credit for selecting everything as the partial-credit failure he
  was concerned about. **Firmness:** agreement expressed as "I think"; preserve that qualification.
  The rule applies within one Question and cannot deduct points from another Question. Handling
  a zero incorrect-choice count is an implementation detail, not a new publication requirement.
- **Further review:** Neil requested deeper comparison across different answer keys. With two
  correct and eight incorrect choices, selecting both correct and four incorrect earns 50% under
  the initial rule. Neil proposed two alternatives when correct choices are fewer: deduct either
  the credit for one correct selection, or the average of that amount and the original incorrect
  selection deduction. For this example the deductions are 50% and 31.25%, respectively.
  Neil then favored the average for all answer keys, replacing the conditional rule and the
  original formula. Each wrong selection deducts `(1 / correct_count + 1 / incorrect_count) / 2`.
  Credit has a minimum of zero. HG and the specs recorded this candidate at that point.
- **Reason and confidence:** Neil favors a penalty that stays the same when the correct and
  incorrect choice counts exchange places, while preserving substantial partial credit. He called
  it a "better durable default" and "probably the strongest candidate so far." Preserve the latter
  qualification while reviewing scenarios. Selecting all eight correct and two incorrect earns
  37.5%; selecting everything is now allowed to earn some credit. The agent clarified that this
  is always below 50% when incorrect choices exist; whether that passes depends on grading policy.

- **Q17 superseded squared-penalty proposal:** Neil supplied the replacement
  `max(0, (TP - FP) / C) * min(1, C / (TP + FP))^2`, with empty selection earning zero.
  Its dedicated owner is
  [MULTIPLE_ANSWER_SCORING_SPEC.md](../../QUESTION_SPECS/MULTIPLE_ANSWER_SCORING_SPEC.md).
  Reason: separate selection accuracy from selecting too many choices. Firmness: supplied as a
  final decision, but its formula contradicts its required property: adding the eighth correct
  selection to seven correct plus one incorrect lowers 75% to about 69.1%. Selecting everything
  with nine correct choices out of ten earns 72%, raising a further conflict with the guessing
  goal. These findings prompted the replacement decision below.
- **Q17 settled: linear choice-count score.** Neil replaced the squared over-selection penalty
  with `R = min(K, C) / max(K, C)`, applied to `max(0, (TP - FP) / C)`. Empty selection earns zero.
  The rule penalizes selecting too few as well as too many choices. Reason: both the selected
  choices and their count demonstrate knowledge; the linear multiplier is his middle ground
  between a square root that is too lenient and a square that is too punitive. Firmness: explicit
  durable default. It applies to Native JSON MA when Assessment partial credit is enabled.
  Selecting all ten with eight correct intentionally earns 60%; the earlier zero-credit and
  below-50% goals are superseded. The dedicated scoring spec owns the complete decision.
- **Q18 Assessment control and Backend scope:** Neil states that Backends grade themselves and
  Assessment Instructors control whether partial credit is awarded. The MA formula belongs to
  Native JSON. Firmness: Assessment control is direct; Native JSON scope was phrased "I think"
  and agrees with the existing Backend responsibility boundary.
  Neil subsequently clarified that PLE always stores the earned partial-credit fraction. With
  partial credit off, fractions below one award zero; with it on, use the fraction. Full credit
  remains full credit. Changing the setting applies to all Attempts using the stored fractions.
  Reason: "this is fair because it applies to all users." Firmness: direct correction of the
  agent's assumption that a setting change would require regrading or a post-start lock.
  Q30 below settles the initial default: enabled.

- **Q19 resolved: Native JSON MULTI-FIB is multiple FIBs.** Neil: "MUTLIFIB is just multiple
  FIBs." Each blank uses FIB accepted answers and regex support. Applied interpretation of that
  definition: grade blanks independently with equal weight when Assessment partial credit is
  enabled. Three correct out of five earns 60%; wrong or unanswered blanks earn zero. Firmness:
  direct definition, in response to the equal-credit question. The earlier all-or-nothing code
  remains an implementation gap.
- **Type scoring clarified:** Neil explicitly confirmed MC and HOTSPOT all-or-nothing, NUM
  tolerance, and FIB accepted-answer lists with regex support. Regex is required behavior missing
  from the current Native JSON matcher. These are direct decisions; no separate rationale was
  supplied. Backend ownership and Assessment partial-credit control remain in force.
- **Q20 settled: Native JSON ORDER partial credit.** Neil supplied `S = (A + R) / 2`, where
  `A` is the fraction of items in their correct absolute positions and `R` is the fraction of
  correctly ordered item pairs. Reason: position alone is too harsh for shifted sequences, while
  pairs alone give random permutations 50% on average. Both kinds of knowledge receive equal
  weight. Firmness: explicit supplied specification, replacing the agent's pair-only suggestion.
  With an ABCD key, DABC earns 25%. The rule applies when Assessment partial credit is enabled.
  [ORDER_SCORING_SPEC.md](../../QUESTION_SPECS/ORDER_SCORING_SPEC.md)
  owns the complete rule. One-time exact-arithmetic checks reproduced all supplied tables across
  all 5,910 permutations for three through seven items. Implementation remains in TODO.
- **Backend scope correction:** Neil already established that each Backend grades its own
  Questions and Assessment Instructors control partial credit. Matching wording now identifies
  Native JSON and the enabled setting explicitly. This is a documentation correction, not a
  new product choice or a reason to prescribe PLE formulas to WeBWorK.

## Complete Question records and reusable Pools

- **Q21 historical; Assessment-add behavior superseded by Q29 below.** Pool forks are regular Pools. Neil says they have a parent-Pool pointer and
  can be used in hundreds of Assessments. The earlier Assessment-owned subtype and Course-derived
  Pool editing authority were unsupported. Apply ordinary Pool ownership and reuse. Firmness:
  explicit correction; the supplied example explains why Assessment ownership is the wrong model.
  Neil subsequently reaffirmed that adding a Pool to an Assessment automatically forks it into a
  new Pool owned by the Instructor, with a parent field pointing to the original. His reason:
  without the fork, the Assessment owner cannot edit the Pool. This is a firm correction of the
  agent's invented reuse-versus-fork choice; audit finding F01 is corrected in documentation.
- **Q22 corrected: complete Question Revision records.** Neil describes another Revision as another
  full database row. Permitted field edits can update the current row without creating a Revision.
  Title, classification, Type, and Bloom remain fields on that record. Source/answer/grading changes
  retain HG's new-Revision rule. Reason: conserve Revisions and treat attributes consistently.
  Firmness: direct model correction. A Revision Number preserves source identity, not every past
  value of mutable metadata.
- **Q23 corrected: Question Type.** Native JSON has a built-in Type; other Backends use Type as
  classification metadata. Neil notes PGML could be easier to classify from source than legacy PG.
  Preserve required Type, allow ordinary classification correction, and avoid a blanket ban on
  reliable source detection. This does not commission a new detector. Firmness: direct distinction;
  automatic detection is a possibility, not a delivery commitment.
- **Q24 corrected: Pool saves and Bloom.** Pool edits may stay in memory before Save. Save replaces
  current state with no undo; the Edit Number is a concurrency counter, not historical membership.
  Bloom is an ordinary editable field like Title. The dedicated classification counter and special
  correction mechanism were drift. Reason: ordinary fields need ordinary editing. Firmness: explicit.
- **Q25 clarified by Q39 below:** Archive preserves a Question that cannot safely be deleted.
  Existing Pool and Assessment references continue normally. The earlier set-aside note did not
  approve restrictions on selection or new use; Archive alone does not establish such gates.

## Draft requirements clarified

- **Q26 resolved: Drafts have no content or metadata requirements.** Neil: "drafts are drafts
  and have no requirements." Creating, importing, and saving a Draft preserves empty, incomplete,
  or broken working content. Complete source and required metadata are checked at publication.
  Reason: Drafts are working content, not Published Questions; this follows the direct correction
  and the established authoring workflow. Firmness: unqualified instruction. Audit finding F02 is
  corrected in the documentation; implementation verification is in TODO.

## Draft autosave decision

- **Q27 resolved: Drafts autosave with a visible saved status.** Neil: "I agree with those draft
  decisions." He accepted autosave so Instructors can return to unfinished work. Cleanup of
  abandoned Drafts remains deferred, with warning and a recovery period required when introduced;
  no expiration period is set. Reason: temporary working content still needs saving. This was
  the recommendation Neil accepted, rather than a separate rationale he supplied. Firmness:
  explicit agreement. Implementation and verification remain in TODO.

## Pool consolidation and release decisions

- **Q28 resolved: Save unfinished Assessment work; validate release.** Neil supplied explicit
  wording: unreleased editing may leave an Assessment incomplete or inconsistent. Save that work
  and show the specific problem. Each Assessment specifies how many Questions to select from a
  Pool. If it requests too many valid Questions, show the problem and block release.
  Allow the Pool save. Resolve the problem by
  adding members, lowering the entry's count, replacing the Pool, or removing it. Post-issue
  restrictions remain. Reason supplied: Pool validity and Assessment releasability are different
  concepts; a valid two-member Pool cannot satisfy a request for three. Firmness: explicit choice
  and supplied guidance. Audit F03 is corrected in docs; runtime verification remains in TODO.
- **Q29 resolved: one Pool specification and explicit forks.** Neil directed consolidation of all
  six Pool specs into [QUESTION_POOL_SPEC.md](../../QUESTION_SPECS/QUESTION_POOL_SPEC.md) and
  supplied the complete model. This supersedes the earlier answer requiring automatic forks when
  adding a Pool. Adding references the existing ID and stores a selection count on the Assessment
  entry. The Instructor continues to own the Pool; future selections everywhere use current state,
  while existing Attempts retain selected Questions. Fork explicitly for independent edits, then
  replace the Assessment reference if wanted. A fork has a new ID, its creating Instructor as
  owner, copied current members and metadata, Edit Number 1, and a parent pointer. Reason supplied:
  split specs encouraged artificial boundaries and invented behavior. Firmness: direct instruction
  to use this model exactly. Blueprint/Course assembly wording now follows the same existing-Pool
  references rather than implicit Pool copies. Student Work keeps exact Question and Pool evidence;
  the Pool counter supplies neither history nor recovery. Implementation remains in TODO.

## Reviewer cleanup after consolidation

The follow-up review found no human decision requiring Question language. The required-language
statements were removed from product rules; existing schema and Native JSON validation remain
labeled implementation evidence with reconciliation in TODO. This records an unsupported
requirement, not a new human decision about language defaults or representation.

HG still leaves the converter handoff format undecided. The QTI spec now states that boundary
and removes the unapproved exact Native JSON mapping. External conversion ownership, normal Draft
publication, generated identity, and accurate reporting of limitations remain settled. The
optional feedback-timing decision remains separate from Question conversion.

## Explicitly deferred decisions

These have already been deferred. Revisit only when their stated reason is addressed.

| Topic | Settled rule and reason | Commitment |
| --- | --- | --- |
| Instructor bulk editing | Neil: "I think we should defer bulk editing for instructors." Save semantics do not need a decision while the feature is deferred | Explicit deferral; tentative wording retained, no rationale supplied |
| Optional Question Feedback timing | Need concrete examples of its use in PLE before deciding automatic display, following answer visibility, or separate timing | Explicit deferral; support content remains optional |
| Regrading submitted Native JSON responses | Desired benefit acknowledged, infrastructure cost raised; when implemented results replace prior grades | Explicit deferral, not a new grade-correction workflow |
| WeBWorK Question Type detection | Neil: "Type detection is deferred; manual for now" | Explicit deferral; assign Type manually |
| Initial Bloom/AI assignment | AI backend and daemon work deferred; NULL allowed with no time limit | Firm absence rule; AI delivery deferred |
| NC and ND content | Support deferred; future ND blocks forking and future NC example retains compatible license requirements | Explicit deferral; current allowed licenses remain unchanged |
| Importer interchange shape | Reuse Rust QTI Package Maker; whether its shared item model or another supported representation crosses the boundary remains unsettled | Do not duplicate the converter or publish Native JSON by assumption |
| H5P and iMathAS | Desired later Backends, outside current delivery; HG restricts future H5P Assessment types | Explicit deferral |
| Detailed Sysadmin repair work | Needs beyond account creation remain unproven; no Instructor approval pipeline was approved | Explicit deferral, not denial of Sysadmin power |
| Automated abandoned-Draft cleanup | HG permits warning and recovery but gives no inactivity rule, periods, or failed-notice outcome; automated daemons are deferred | Defer deletion automation; ordinary Instructor Draft deletion remains available |

## Flexible choices, not blockers

- Organized search default: Questions in no Pool plus Pools is Neil's preferred default, using
  "probably" because redundant member rows create noise. Combined search itself is firm.
- Course color families are settled, including Molecular Biology magenta and Laboratory teal-green.
  Exact existing Theme matches remain proposals; review actual Light/Dark appearance later.
- Display-mode wording and incidental sorting ties are low value for the grill. Keep the requested
  Compact, List, and Visual styles without inventing a ranking debate.

## Implementation gaps, not questions

- Public IDs retain their hyphen everywhere and use public SHA-256, with no secret. Neil explicitly
  reconfirmed this as the modern choice. Correct stale documentation; do not reopen the decision.
- Both Library Object kinds were in the original design. Missing shared fields in a browser row
  or missing filter handling are implementation gaps, not permission to narrow the model.
- PG/PGML Draft creation and shared publication already exist in the content-loading tools.
  The browser-facing authoring route currently accepts only Native JSON. That route limitation
  belongs in TODO.md; it does not establish separate Question workflows by Backend.
- Supported PLE import operations must generate identities and enforce ordinary validation;
  readback checks the resulting content and relationships. Direct SQL fixtures do not prove that
  workflow. HTTP transport and test-harness choices are implementation work, not settled product
  requirements from this interview.

## Updating this log

Add new questions with a stable ID and concrete source only when the choice changes correctness, maintainability, validation, or delivery. Keep routine engineering details outside this log. When Neil answers, record the answer,
his stated reason, and expressed firmness; distinguish his reason from an agent recommendation.
Update the owning specification and cross-references, then mark the row resolved. Keep deferred
items deferred and record implementation work in the audit/report rather than as another question.

## Grill decisions, 2026-10-05

- **Q08 correction and Q09 resolved:** Neil clarified that Biostatistics is "largely incomplete"
  and said "I would probably defer that one as well." Record deferral as his current preference,
  retaining that qualification. He also confirmed that Genetics and Biochemistry are the only
  complete BP.org courses. The live homepage agrees and places Biotechnology among selected
  Question collections. This resolves its completeness question; completeness alone does not
  decide delivery timing. The earlier four-Course selection below is superseded for Biostatistics.
- **Q08 Biotechnology delivery:** Neil then explicitly selected "Keep Biotechnology in the first
  delivery." Firmness: direct, unqualified choice. No separate reason supplied. The first-delivery
  set is Genetics, Biotechnology, and Biochemistry; only Genetics and Biochemistry have complete
  source-course coverage.

- **Q08 initial choice, later revised above:** Neil selected "Keep the four pilot Courses first; Molecular Biology and
  Laboratory later." The four are Genetics, Biotechnology, Biostatistics, and Biochemistry.
  Firmness: explicit selection, with no qualification added. Keeping the first delivery focused
  was the recommendation's reason; Neil supplied no separate rationale. This settles delivery
  scope without changing the recorded completeness of each source collection.
- **Q11 resolved:** Pool forks are ordinary reusable Pools. Their members do not match
  the Questions in no Pool filter. Neil's reason: "forks of pools are still just pools."
  Firmness: direct, unqualified clarification. This resolves the recorded scope question without
  introducing a new Pool kind. HG and the specifications now include this clarification.
- **Forking reference:** Neil clarified, "for forking we are again mostly using the GH model."
  GitHub is the main design reference; the existing PLE rules define the details. Pool forks
  remain Pools with their own identity and a source-Pool link. Firmness: explicit direction,
  with "mostly" allowing PLE-specific differences already stated in HG.
- **Q03 deferred with Instructor bulk editing:** Neil said, "I think we should defer bulk
  editing for instructors." Record the feature as deferred, not merely the mixed-save question.
  Firmness: explicit deferral expressed with "I think"; no reason supplied. This does not decide
  importer behavior or remove implemented features during the documentation interview.
- **Q02 main grouping resolved:** Start with one Assessment per website topic. Long topics may
  become parts such as `topic03a` and `topic03b`. Hard Questions are usually bonus content.
  Neil's reason: "the topics are designed around my course." Firmness: clear starting rule;
  splitting and the usual treatment of Hard Questions remain flexible. Neil called detailed
  assembly "perhaps too in the weeds"; do not interview on individual splits, counts, or point
  values during the Question-spec discussion.
- **Q01 resolved for BiologyProblems.org:** Neil said, "one PLE is launched, BP.org and PLE
  are no longer connected. PLE no longer cares how BP.org changes." The initial source import
  does not establish continuing source synchronization or automatic updates. Firmness: direct,
  unqualified decision. Reason supplied: the two systems are independent after launch. Existing
  source attribution remains a separate settled requirement. Do not widen this into a general
  future-import reconciliation project.
- **Q10 first export use resolved:** Neil selected "Export selected Questions for another LMS."
  Firmness: explicit selection of the recommendation. No separate reason supplied; reuse of the
  existing QTI Package Maker was the recommendation's stated reason, not a newly stated human
  rationale. PLE backup/transfer is not the first-use requirement. Pool packaging was clarified in the later notes below.
- **Q10 stronger clarification:** Neil added that export is "really only designed" for selected
  Questions going to another LMS. Firmness: strong explicit scope restriction, superseding the
  earlier "first use" wording. Question export is not a PLE backup or PLE-to-PLE transfer feature.
  Existing Blueprint JSON comparison/exchange rules are a separate document concern; this answer
  does not silently remove them.
- **Conversion ownership clarified:** Neil: "PLE uses QTI Package maker-rs as an external library
  to handle all of its conversion." Firmness: explicit requirement. The external Rust library
  owns conversion; PLE must not implement a competing converter. Choosing which Library Objects
  or Pool members to pass into export is separate from format conversion.
- **Q10 Pool selection and packaging:** Neil called exporting the selected Pool's members
  obvious, then challenged the wording: "so if I select a pool of 1,000 questions, as an
  instructor do I want 1,000 individual and similar files?" The intended output is one
  LMS-importable package containing those Questions together, not 1,000 separate downloads.
  Preserve question-bank grouping where the target LMS supports it. `qti-package-maker-rs`
  owns conversion and packaging. The packaging interpretation follows Neil's correction;
  exact target-format support is an engineering fact, not another product interview question.
- **Pool export clarification:** Neil emphasized that Pool members must all have the same
  Question Type and are literally separate Questions. There is no merged-Question concept.
  A Pool of 1,000 Multiple Choice Questions exports 1,000 separate Multiple Choice Questions;
  one package groups them for import and does not combine their content into one Question.
  Firmness: explicit correction of the assistant's confusing distinction between Questions
  and downloaded files. The existing same-Type rule is unchanged.

## Documentation follow-up

The recorded decisions above have been applied to HG and the affected Question, Pool, export,
import, and BiologyProblems.org specifications. Instructor bulk-edit documents are explicitly
marked deferred. Checklist entries preserve the updated HG wording and leave new implementation
claims unverified. No code behavior was changed.

Q05 is closed, Q06 is an implementation review note, and Q07 still needs engineering inspection.
Use ordinary Course content work for individual assembly choices. This log is not evidence that
all statements elsewhere in the refactored specs have been traced to approved intent.

- **Terminology correction:** Neil specified "Pool forks are Pools," not "Pool forks count as
  Pools." Use identity language, not wording that suggests a separate kind receiving special
  treatment. This is firm and applies throughout the specs.

- **HG wording correction:** Neil objected that the additions read like an agent rather than a
  human. Keep HG close to his words. Removed the repeated Pool-filter explanation: the ordinary
  definition already applies because forks of Pools are Pools. Simplified export wording and
  kept the detailed consequences in the specifications and this log. The decisions are unchanged.

- **HG and design decisions:** Neil approved expanding design decisions while keeping HG more
  human. Removed the added fork-identity reminder and Pool-packaging explanation from HG;
  kept the GitHub reference and export purpose. Their practical consequences remain in the
  specs and DESIGN_DECISIONS.md. Restored "I think" in the bulk-edit deferral and marked the
  older bulk-edit design deferred. This changes document ownership, not product behavior.

- **Q04 closed:** Neil: "I never approved a Used in my Courses filter." The assistant mistook
  implementation for an approved requirement. Removed the filter from the product rules and
  interview agenda; recorded its existing code as drift. No rationale or replacement feature
  is inferred. This documentation correction does not remove the runtime control.

- **Q04 removal:** Neil would remove the filter because he considers its server overhead
  unnecessary and questions why Instructors would search for content already in use. Explicit
  removal direction; server cost has not been measured. Remove the control and underlying
  Course-usage computation together.

- **HG wording:** Neil directs HG to describe intended behavior positively and omit irrelevant
  alternatives. Removed the rejected-filter commentary from HG; its decision and reason remain
  here. Reason: unnecessary negative instructions lengthen HG and can lead small LMs toward the
  unwanted behavior. Firmness: explicit writing rule.

- **Documentation scope:** Neil chose to keep the already-written code removal and return to
  documentation. Further code work belongs in [TODO.md](../../TODO.md) for later execution.
  Offline checks passed; the database run was interrupted, browser verification was not run,
  and the Live Demo remains stopped.

## Review of agent-added decisions

Neil challenged the assumptions behind the interview, rather than requesting more detailed
choices based on those assumptions. See
[question_specs_decision_review_2026_10_05.md](../reports/question_specs_decision_review_2026_10_05.md).

- **Q05 correction:** Neil rejected the premise that using the Blueprint's Theme needed a new
  decision: "what is the point of a BP Course if it is not used as BP for the instance."
  Firmness: emphatic correction. The Course starts from the Blueprint, including its Theme;
  HG already permits the Instructor to change a Course Theme independently. The earlier
  interview question was unnecessary. Storage and API support belong in implementation TODOs.
- **Q06 review:** Combined Library Objects and shared Assessment discovery are already settled.
  The agent treated current selection limits as a possible restriction needing product approval.
  Retire that interview question without inventing a new selection limit or a separate Pool kind.
- Removed unsupported import transport, mapping-schema, and status-vocabulary
  requirements. Keep supported PLE operations, generated IDs, publication validation, source
  attribution, and readback. These corrections do not authorize code changes.
- **Backend-agnostic Questions:** Neil: "Questions are backend agnostic, period." Firmness:
  emphatic requirement. Withdraw the review's suggestion that different Backends might need
  different Question creation workflows. Source and execution details belong behind the common
  Backend interface; Questions share authoring, publication, ownership, discovery, and use rules.
- **Authoring is fundamental:** Neil: "not all questions are imported" and Drafts exist "to draft
  questions and test them before publication." Firmness: direct correction. The Draft specification
  now describes writing and testing Questions first; import is another source of Draft content.

## Initial partial-credit default

- **Q30 settled:** new Assessments start with partial credit enabled. Neil selected
  "Enable partial credit by default."
- **Reason presented with the choice:** award the credit earned under the Backend's scoring
  rules while preserving Instructor control. Neil accepted the recommendation without adding
  a separate rationale.
- **Firmness:** explicit selection; no qualification or numerical confidence supplied.
- The existing rule for applying later setting changes to all Attempts remains settled.
  This documentation decision records intended behavior; implementation is tracked in TODO.

## Shared metadata and Backend source

- **Q31 settled:** Title, Description, Tags, license, and citation belong to the complete
  Question record. Native JSON holds Backend-owned material needed to render the Question,
  accept a response, and grade it.
- **Because:** Neil identified the existing Native JSON fields as duplicating the database,
  then explicitly selected shared metadata on the Question record. Ordinary metadata edits
  should have one authoritative location.
- **Firmness:** explicit confirmation without qualification. This settles field ownership,
  not a replacement wire format or database schema. Existing Revision rules still govern edits.
- Documentation aligned: HG, the Native JSON spec and example, shared metadata spec, lifecycle,
  and linked contracts now keep shared metadata on the Question record. Existing duplicate source
  fields are labeled implementation evidence. Decoder/publication/storage reconciliation remains
  in TODO; code is unchanged.

## Statistics question withdrawn

- **Q32 closed; interview premise rejected:** the agent asked whether displayed statistics
  should describe a selected Revision or combine Revisions before establishing useful measures.
  Neil rejected that ordering. His reply does not choose either reporting option.
- HG establishes privacy-safe aggregates kept separately per Question Revision and per Pool,
  with the purpose of helping Instructors understand use and Student performance. Q33-Q36 later
  settled the useful measures; this earlier question is not an outstanding interview item.
- The refactored spec's extra outcome categories, answer-choice outcomes, and cross-Revision
  summaries were not approved merely because the agent wrote them. The later cleanup removed
  those additional product requirements; existing counter details remain implementation evidence.
- Start with what information helps Instructors; define its measures before aggregation or
  presentation rules. A separate derived difficulty rating has not been approved.

- **Q33 purpose settled:** Neil selected "Show how often Students received it and how much
  credit they earned." This establishes the intended information, not the exact measures,
  denominator, reporting layout, or cross-Revision aggregation.
- **Reason presented:** describe use and Student performance directly, without a separate
  derived difficulty rating. Neil selected this recommendation without additional rationale.
- **Firmness:** explicit choice without qualification. Q34 records why the subsequent question
  about Backend credit versus Assessment points was unnecessary.

- **Q34 clarification; unnecessary question:** use the stored Question credit fraction when
  describing Student results on the Question. Assessment point values and partial-credit
  settings determine Assessment scores separately. Neil's reply identifies this distinction
  as already settled, rather than a new product choice.
- Withdraw the agent's phrase "Question Library performance summary." The subject remains
  aggregate Question statistics. Ask about useful concrete measures rather than inventing
  a separate feature or revisiting the Backend/Assessment responsibility boundary.
- Q35 below records the later decision on average credit and its contributing response count.

- **Q35 settled:** include average earned credit and the number of graded responses used to
  calculate it. Neil explicitly selected this option without qualification.
- **Reason presented:** a concrete measure of earned credit, accompanied by the number of
  responses behind that average. Neil supplied no separate rationale or confidence number.
- Q36 below records the subsequent approval of full-credit and zero-credit percentages.

- **Q36 settled:** include full-credit and zero-credit percentages alongside average credit
  and the graded-response count. Neil explicitly selected both.
- **Because:** average credit alone hides the shape of outcomes; half-credit responses differ
  from a split between full and zero credit. Partial-credit responses form the remainder, so
  Neil sees no need for a fifth percentage without a demonstrated use.
- **Firmness:** explicit decision with a supplied rationale. HG and the Library specification
  now name these four measures. The unsupported answer-choice statistics and cross-Revision
  summaries were removed from the Library specification.

## Interview rule for remaining questions

Before presenting alternatives, express the issue using existing PLE concepts and settled rules.
Trace their consequences. If they determine the behavior, correct the conflicting documentation
and record the consequence rather than asking Neil to choose it again. Bring a product question
only when two genuinely plausible behaviors remain. Neil identified invented intermediate
concepts as the recurring cause of unnecessary questions; shrinking the uncertainty log is part
of this review.

## WeBWorK Type assignment

- **Q37 settled:** Neil: "WeBWorK Type detection is deferred; manual for now."
- Assign WeBWorK Question Type manually. Future source detection remains deferred.
- **Firmness:** direct instruction. No further reason was supplied. This settles delivery scope
  without changing editable classification or Backend-owned interaction and grading.
- HG, the Type and WeBWorK specs, frontend documentation, and TODO now use that direction.

## Manually written notices

- **Q38 settled:** Neil: "I do not want to create a social media platform. I would suggest the
  instructor fork and fix it."
- Use the existing fork-and-fix workflow for another Instructor's Question or Pool. Manually
  written notices are removed from intended scope rather than deferred for a permission decision.
- **Because:** Neil wants content improvement through ordinary Question/Pool actions, without
  creating a social-media platform. **Firmness:** direct rejection of the proposed notice feature;
  fork-and-fix is his suggested action.
- HG Watch bullets retain Revisions, forks, and Pool membership edits. Existing notice code is
  removal work in TODO; this docs pass changes no application code.

## Archive purpose and remaining overstatements

- **Q39 settled:** Neil: "The archive question concept exists only because sometimes we cannot
  delete questions, so following the GH repo archive model seemed appropriate."
- **Final clarification:** Neil says "the less architecture for Archives the better" and supplies
  the concise rule: Archive makes the ordinary Question read-only and removes it from normal
  discovery while preserving it and its existing references. Archived Questions can be restored
  or forked. This is the complete settled rule for this pass; intermediate proposals about
  additional workflow restrictions are superseded.
- **Because:** preserve content that cannot safely be deleted. **Firmness:** direct purpose
  clarification. This resolves the earlier broad Archive uncertainty without inventing a new
  selection policy.
- The forwarded correction also reaffirms that Questions in no Pool plus Pools is a tentative
  default candidate, and that cross-Revision statistics rollups lack approval. The Library spec
  already kept statistics per Revision; terminology, design, and data-policy text still promoted
  rollups. Those statements are now corrected, with implementation follow-up in TODO.

## Metadata ownership clarification

Neil's final wording for Q31: Question metadata belongs on the Question record, not in Native
JSON. Native JSON contains the Question content needed to display and grade it. The metadata
spec defines the fields and their editing and Revision rules. Citation format remains deferred.
This is a firm ownership decision; it applies to Draft and Published Questions.

## Pool Save wording correction

Neil removed "re-attests interchangeability" from HG and chose concrete wording for Pool Save:
"When the set of Question Revision Tuples in a Pool changes, saving advances the Pool's Edit Number."
Requirements for the Questions a Pool contains remain separate and use
the existing validation/mismatch rules. This supersedes the review's confirmation wording and its
subsequent repetition of the old HG phrase. The checkbox, attester identity, and timestamp remain
implementation removal work in TODO, not part of the desired product description.
