# PLE vocabulary replacement checklist

This document is the implementation checklist for PLE-owned wording.
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) defines canonical meanings.
This checklist connects source, schema, API, test, and documentation wording to
those meanings and retains every correction until the final audit.

Use each row within its stated context. Apply the structural instruction when a
current label combines multiple concepts. Use
[NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) for identifier spelling after the
semantic target is clear.

## Correction workflow

1. Select one unchecked row and resolve its wording in the owning type,
   relation, API, or document.
2. Apply the canonical target and structural instruction from the same row.
3. Update source, stored fields, generated contracts, tests, and documentation
   together for that boundary.
4. Run the boundary's narrow gate and required consumer gate.
5. Search every active PLE-owned boundary for the wording to replace and inspect
   each remaining match in context.
6. Record the completed correction and its evidence in
   [CHANGELOG.md](CHANGELOG.md).
7. Change the row from `[ ]` to `[x]` after it satisfies the completion rule.

A checked row stays in this checklist. Retaining completed rows makes progress
visible and gives the final audit a closed inventory.

### Row completion rule

Check one replacement row only when all of these statements are true:

- The wording to replace is absent from active PLE-owned source, schemas, APIs,
  generated contracts, tests, durable documentation, and active plans in the
  row's stated context.
- Every remaining repository match is the checklist row itself, exact
  registered or platform vocabulary, or immutable historical evidence, and a
  person or agent has inspected each match.
- The canonical target and structural instruction are present at the owning
  boundary and its consumers.
- The narrow gate, required consumer gate, and documentation checks pass on
  the same material tree.

For ordinary words that may have several meanings, audit the row's stated
context rather than accepting a raw zero-match count. For identifiers and
quoted phrases, use exact searches plus case-insensitive searches for prose
variants.

## Local Stack Controller

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [x] | `local_runtime/` | Ignored repository-root files written by the Local Stack Controller | `local_stack_state/` | Keep the tracked `local_stack_control/` package as the Local Stack Controller. Store only its disposable private host files beneath the canonical Local Stack State directory. |

## Assignment and Student activity

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [ ] | Student Assignment Attempt | One Student's pass through an Assignment | Assignment Attempt | Bind the Assignment Attempt directly to one exact Student Record and Assignment. |
| [ ] | Assignment Instance | Mixed wording for a live Assignment or one Student's work | Assignment or Assignment Attempt | Use Assignment for the Course Instance-owned delivery definition. Use Assignment Attempt for one Student's pass. |
| [ ] | `AssignmentRecord` or `StoredAssignment` as one mutable authored aggregate | Stable Assignment ownership mixed with an editable teaching definition | Assignment plus Assignment Revision | Keep the Assignment as the stable Course Instance-owned record. Store every complete authored definition as an immutable Assignment Revision. |
| [ ] | `AssignmentTeachingSettings` | Bundle of lifecycle, instructions, and Base Assignment Policy | Assignment Revision | Store these as explicit fields in one complete immutable Assignment Revision instead of giving the bundle separate identity or history. |
| [ ] | generic Assignment `title` or `instructions` at a shared boundary | Learner-facing Assignment text | Assignment Title or Assignment Instructions | Use Assignment Title for the short name and Assignment Instructions for whole-Assignment directions. Keep Question-specific tasks in Question Prompts. |
| [ ] | `AssignmentRevision` as a compare-and-swap counter | Mutable aggregate edit sequence | Assignment Revision Number and Assignment Revision Reference | Scope the positive number to the stable Assignment and use the exact Assignment Reference plus Revision Number pair. An accepted edit creates the next immutable revision. |
| [ ] | `expected_revision` on an Assignment edit | Conditional write against the mutable Assignment aggregate | Assignment Revision Reference as edit precondition | Name the exact revision the Instructor edited. Accepting the change creates a new Assignment Revision only when that revision is still the current draft. |
| [ ] | `AssignmentContentIssuedWorkConflict` or `IssuedLearnerWork` | Structural edit refused because an Assignment already produced Student activity | New Assignment Revision | Create a successor Draft Assignment Revision. Publishing it changes future Assignment Attempts while existing attempts retain their pinned revision. |
| [ ] | `AssignmentPublicationReadiness` applied to a mutable Assignment | Publication check for the current in-place definition | Assignment Publication Readiness for one Draft Assignment Revision | Derive the closed blocking issues from the exact immutable revision proposed for publication. |
| [ ] | `assignment_attempt.assignment_id` without authored-definition identity | Student activity bound only to the stable Assignment | Assignment plus Published Assignment Revision Reference | Bind every Assignment Attempt to the exact Published Assignment Revision expanded into its Issued Questions. |
| [x] | Assignment Run | Current activity model and API | Assignment Attempt | Preserve one complete pass, its pass-level variation, timing, completion, and ordered Issued Questions. |
| [x] | Assignment Run Item | Current expansion of an Assignment Entry for one run | Issued Question | Store the source Assignment Entry, exact Question Version, delivery order, scoring treatment, and selection evidence here. |
| [x] | Assignment Enrollment | Current aggregate joining one enrollment to one Assignment | Direct Student Record and Assignment relationships | Bind each Assignment Attempt and Assignment Grade to the exact Student Record and Assignment. Place attempt policy facts on Assignment Attempt and selected Gradebook results on Assignment Grade. |
| [ ] | Student Assignment Summary | Current mixed activity and Gradebook view | Assignment Grade plus an activity summary view | Keep the selected course-record result in Assignment Grade. Derive attempt counts and recent activity as a read view. |
| [x] | `ple_data.assignment_enrollment` | Per-Assignment join currently placed between Student Record and work | Direct Student Record and Assignment relationships | Reference the exact Student Record and Assignment from Assignment Attempt and Assignment Grade. |
| [x] | `ple_private.assignment_run` | Stored Student pass through an Assignment | Assignment Attempt | Store pass-level timing, variation, completion, and state on Assignment Attempt. Store explicit whole-attempt finalization as Assignment Submission. |
| [x] | `assignment_run.submitted_at` and submitted state | Mixed completion and whole-attempt finalization | Assignment Attempt Completion and Assignment Submission | Derive completion from the Assignment Completion Rule. Create Assignment Submission only for an accepted explicit finalization event. |
| [x] | Assignment Selection Group | Current random-selection definition inside an Assignment | Question Pool | Store exact Question Version candidates, draw count, points, and Question Pool Selection Rule on the Assignment Entry. |
| [x] | Assignment Selection Candidate | Current candidate inside a selection group | Question Pool Candidate | Reference one exact Question Version and preserve its authored order and availability for future selection. |
| [x] | Question Group, Question Set, or Random Block | Assignment composition that selects Questions from explicit candidates | Question Pool | Represent these interface variants with one Question Pool, its exact Question Pool Candidates, and its Question Pool Selection Rule. |
| [x] | `AssignmentItem`, `AssignmentItemSummary`, or `AssignmentEditorFixedEntry` | Current source types for a saved or edited fixed entry | Fixed Question Assignment Entry | Use the same Assignment Entry model as a Question Pool and select the Fixed Question variant. |
| [x] | `AssignmentSelectionGroup`, `AssignmentSelectionGroupSummary`, or `AssignmentEditorSelectionGroupEntry` | Current source types for a saved or edited random-selection entry | Question Pool Assignment Entry | Use the same Assignment Entry model as a Fixed Question and select the Question Pool variant. |
| [x] | `AssignmentItemId` | One type currently shared by fixed entries and candidates inside pools | Assignment Entry Reference or Question Pool Candidate Reference | Use Assignment Entry Reference for every top-level Fixed Question or Question Pool. Use Question Pool Candidate Reference only within its owning Question Pool. |
| [x] | `AssignmentSelectionGroupId` | Separate identity type for a Question Pool entry | Assignment Entry Reference | Give Fixed Questions and Question Pools one Assignment Entry reference model. Let the entry variant establish that the reference names a Question Pool. |
| [x] | `selection_groups` or `selectionGroups` | Question Pools stored beside fixed Assignment entries | Assignment Entries | Store one ordered Assignment Entry collection whose closed variants are Fixed Question and Question Pool. |
| [x] | `position` on both an Assignment entry and its containing array | Two representations claiming authored order | Ordered Assignment Entries | Let the ordered collection own authored order. Use a storage ordinal only where normalized persistence requires it. |
| [x] | candidate `position` plus candidate array order | Two representations claiming authored candidate order | Ordered Question Pool Candidates | Let the ordered candidate collection own authored order. Use a storage ordinal only where normalized persistence requires it. |
| [x] | `groupPosition` in pool preview | Position used as a saved Question Pool locator | Assignment Entry Reference | Resolve one exact saved Question Pool through its Assignment Entry Reference. Derive visible order from the Assignment's ordered entries. |
| [ ] | `PointValue`, `points_possible`, and `points_per_item` | Assignment-owned value applied after Question grading | Assignment Entry Point Value | Store one exact value on every Assignment Entry and freeze the applied value on each Issued Question. A Question Pool applies the value to each selected candidate. |
| [ ] | `AssignmentScoringMode` and `scoring_mode` | Assignment-owned treatment of normalized Question credit | Assignment Entry Scoring Rule | Store Normal, Full Credit, Extra Credit, or Excluded on every Assignment Entry and freeze the applied rule on each Issued Question. |
| [ ] | `AssignmentDeliveryState` on an item or candidate | Eligibility for future Assignment Attempts | Assignment Entry Availability or Question Pool Candidate Availability | Put Assignment Entry Availability on every top-level Fixed Question or Question Pool. Put Question Pool Candidate Availability on each candidate. Use Available or Retired and preserve historical Issued Questions. |
| [ ] | `PoolDrawAlgorithm`, `SelectionOrdering`, and `PoolDrawBasis` | Split pool-selection behavior | Question Pool Selection Rule | Combine the reviewed selection method, output ordering, and reuse or redraw behavior into one explicit rule. |
| [ ] | Assignment Run Item selection fields | Selected pool result for one run | Question Pool Selection and Issued Question | Record the pool result once for the Assignment Attempt. Link each resulting Issued Question through the Question Pool Selection Reference and its source Assignment Entry Reference. |
| [ ] | pool seed | Deterministic selection among Question Version candidates | Question Pool Seed | Use it only to reproduce Question Pool Selection. A selected Generated Question receives its separate Question Seed. |
| [ ] | WeBWorK random question or WeBWorK pool | Seeded variation of one WeBWorK Question Version | Generated Question, Question Generator, Question Seed, and Question Variation | Treat WeBWorK randomness as generation within one Question Version. Use Question Pool only when selecting among multiple Question Version References. |
| [ ] | Run Mode | Bundled label for Assignment behavior | Assignment Activity | Expand the Activity into explicit completion, grading, continuation, variation, timing, and Student Feedback Release Rules. |
| [ ] | `StudentSubmissionStatus` | Mixed Question Submission acceptance, grading progress, and completed response | Question Submission Receipt plus Question Submission Grading State | Return the accepted Receipt and current grading state separately; derive any next action from that state. |
| [ ] | `AttemptStatus::InProgress`, `Submitted`, or `AutoSubmitted` | Operational state of one exact try | Question Attempt State | Use Open, Submitted, or Automatically Submitted and derive submission states from the accepted Question Submission. |
| [ ] | `AttemptStatus::Cleared` | Instructor removal from current scoring | Question Attempt Exclusion | Retain the Question Attempt, Question Submission, Student Response, Grading Result, authorizing Account, reason, and Receipt. |
| [ ] | `AttemptStatus::Exempt` | Instructor decision that one issued requirement does not contribute | Issued Question Exemption | Record the exact Issued Question, Instructor authority, reason, and effect on completion and scoring. |
| [ ] | `AttemptResult` | Server grading outcome for one accepted response | Grading Result | Bind it to the Question Submission and exact Question Attempt Source Record. |
| [ ] | `GradeOutcome` | Graded result or explicit ungraded outcome for one accepted response | Grading Result | Create a Grading Result only when the Question Grading Rule grades that response; retain the Question Submission without a fabricated result when it is ungraded. |
| [ ] | `SubmissionEvaluationStatus` | Grading progress for one Question Submission | Question Submission Grading State | Use Pending, Graded, Instructor Attention, or Exempt. |
| [ ] | `AutomatedGradingStatus` | Student-visible subset of Question Submission grading progress | Question Submission Grading State | Present Pending, Graded, or Instructor Attention from the same authoritative state and keep private failure detail server-held. |
| [ ] | Clear Assignment Attempt | Instructor recovery after a technical or teaching issue | Assignment Attempt Exclusion plus authorized replacement work | Preserve the original activity and scoring evidence, record the authorizing Account and reason, and apply the exact policy adjustment that permits replacement work. |
| [ ] | Submit Assignment Attempt for Student | Instructor recovery using verified saved Student Responses | Instructor Assignment Finalization | Create an attributed Assignment Submission and Receipt for the exact Assignment Attempt. |
| [ ] | Test Access Log or attempt access log | Instructor troubleshooting history | Assignment Attempt Activity Log | Present immutable server Events and Receipts in order and report observed timing without inferring its cause. |
| [ ] | Run Policies or `RunPolicies` | Bundle of run-level behavior enums | Explicit Assignment rules | Store the Assignment Completion Rule, Assignment Attempt Grade Rule, Assignment Attempt Continuation Rule, Question Variation Rule, timing rules, and Student Feedback Release Rule separately. |
| [ ] | `CompletionRequirement` | Rule deciding whether one Assignment Attempt is complete | Assignment Completion Rule | Use Answer All, All Correct, or Score At Least with its explicit threshold. |
| [ ] | `GradePolicy` | Rule selecting which completed Assignment Attempt contributes to the Gradebook | Assignment Attempt Grade Rule | Use First, Latest, Highest, or Instructor Selected and preserve every other Assignment Attempt. |
| [ ] | `ContinuedPractice` | Rule for starting later practice after completion | Assignment Attempt Continuation Rule | State Unlimited, a specific additional-attempt limit, or Closed. |
| [ ] | `VariationPolicy` | Rule deciding what changes in a later Assignment Attempt | Question Variation Rule | State whether later work keeps selected Questions, uses fresh Question Seeds, or redraws Question Pools. |
| [ ] | `StudentDisclosurePolicy` or `StudentDisclosureTiming` | Per-field schedule for Student-visible results and teaching material | Student Feedback Release Rule | State when score, correctness, feedback, solutions, and class statistics become visible. |
| [ ] | `AttemptPolicy` | Question-owned maximum number of tries | Question Attempt Limit | Apply the limit to Question Attempts for one Issued Question. |
| [ ] | `TimingPolicy::PerQuestion` | Duration for one question try | Question Attempt Time Limit | Bind the server deadline to the exact Question Attempt. |
| [ ] | `TimingPolicy::PerAttempt` | Duration for a whole pass currently carried by question policy | Assignment Attempt Time Limit | Move whole-pass timing to the Assignment and bind the resolved deadline to the Assignment Attempt. |
| [ ] | `TimerVerdict` | Server result of evaluating one exact timed attempt | Question Attempt Timing Decision or Assignment Attempt Timing Decision | Use the qualified subject and the closed Untimed, Open, Grace Period, Submitted On Time, Submitted Within Grace, or Timed Out result. |
| [ ] | `LateSubmissionPolicy` | Treatment of work after the Assignment due time | Late Work Rule | State whether late work is accepted, marked late, or refused. |
| [ ] | `StudentLateStatus` | Result of applying the Late Work Rule to accepted Student work | Student Late Work Status | Use On Time, Accepted Late, or Marked Late. Refused work produces no accepted submission status. |
| [ ] | `AssignmentDeadlineBehavior` | Server action at the effective Assignment deadline | Assignment Deadline Rule | State the server-owned deadline action explicitly, including automatic submission where selected. |
| [ ] | `AssignmentLandingPresentation` | Answer-free Student view before starting or resuming work | Assignment Overview | Present instructions, resolved schedule, question count, variation, timing, and Student Feedback Release summary. |
| [ ] | Force Completion | Rule controlling whether a Student may leave and return to current work | Assignment Attempt Resume Rule | Select Resumable or Single Session and preserve the server-owned Assignment Attempt state. |
| [ ] | All at Once or One at a Time | Number of Issued Questions presented together | Assignment Question Display Rule | Select All Questions or One Question at a Time independently of navigation and order. |
| [ ] | Prohibit Backtracking | Restriction on movement through the current Assignment Attempt | Assignment Navigation Rule | Select Forward Only; use Free Navigation for movement among available Issued Questions. |
| [ ] | Randomize Questions | Whole-Assignment ordering after fixed and pooled entries expand | Assignment Question Order Rule | Select Authored Order or Shuffled and freeze the resulting position on every Issued Question. |

## Answers and submissions

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [ ] | Student Answer | Correctness-neutral learner payload | Student Response | Carry learner-supplied data inside the accepted Question Submission boundary. Reserve Answer for correct-response material. |
| [ ] | Response Definition | Server-owned accepted-response shape | Question Response Format | Keep response shape, item references, cardinality, numeric constraints, and validation rules server-owned. |
| [ ] | Response Schema | Documentation and wire wording for accepted-response shape | Question Response Format | Use Question Response Format for the public response-ready structure and server validation contract. |
| [ ] | Response Control | Browser interaction that collects a Student Response | Question Response Control | Use the control declared by the Question Presentation and compatible with Question Response Format. |
| [ ] | Response Widget | Browser component that collects a Student Response | Question Response Control | Name the product concept Question Response Control; apply framework-specific component spelling locally. |
| [ ] | Question response | Ambiguous Student data or accepted event | Student Response or Question Submission | Use Student Response for learner data. Use Question Submission for the immutable acceptance event and receipt boundary. |
| [ ] | `RenderedItemIdV1` | Temporary browser reference to one selectable object in an issued presentation | Response Item Reference | Scope the reference to one exact Question Presentation and resolve it through the server-held Response Item Binding. |
| [ ] | `RenderedItemRoleV1` | Semantic use of one referenced item in an issued response contract | Response Item Role | Use Question Choice, Text Entry Slot, Matching Prompt, Matching Choice, Ordering Item, or Hotspot Surface and validate each Reference against its expected Role. |
| [ ] | `RenderedItemBindingV1` | Server-only mapping from a temporary rendered reference to a durable response item | Response Item Binding | Bind the Response Item Reference, item role, and durable response item inside one exact Question Presentation. |
| [ ] | `PresentationNonceV1`, `PresentationDigestV1`, or `PresentationBindingV1` | Server-held integrity facts for one issued answer-free presentation | Question Presentation Binding | Store the nonce and full digest with the exact Question Attempt and verify them before translating a Student Response. |
| [ ] | `PresentationDigestTokenV1` | Public digest prefix returned with one issued presentation | Question Presentation Token | Check it against the server-held Question Presentation Binding without treating it as access authority. |
| [ ] | `PresentationEnvelopeV1` or `PresentationV1` | Complete answer-free state issued for one Question Attempt | Question Presentation | Carry the Question Prompt, public assets, Question Response Format, Response Item References, and control description. |
| [ ] | `SanitizedMarkupProjection` | Allowlisted rendered markup supplied for one prompt position | Question Prompt Block | Represent the content through the normalized Text, Math, Image, Code, or Table block and its presented Assets. |
| [ ] | `InspectedStudentResponseV1` | Authorized solution-free rendering of an immutable Student Response | Student Response Inspection | Render submitted values through the exact issued Response Item Bindings and keep grading material and provider state server-held. |
| [ ] | Assignment response | Ambiguous whole-Assignment action | Assignment Submission | Use Assignment Submission for explicit whole-attempt finalization. |
| [ ] | `question_attempt.response` | Mutable learner data stored on the current try | Browser draft or Student Response | Keep the current draft in browser state. Store the accepted Student Response in its immutable Question Submission. |
| [ ] | `question_submission.response` | Accepted learner payload | Student Response | Name the stored payload Student Response and bind it to the immutable Question Submission. |
| [ ] | `question_submission.grading_receipt` | Grading data embedded in a submission row | Automated Grading Receipt and Grading Result | Store the grading receipt and result as separately governed records linked to the Question Submission. |
| [ ] | `ChoiceId` | Opaque reference reused for choices, slots, prompts, ordering items, and hotspot regions | Response Item Reference | Name the referenced response item by its exact role where the surrounding Question Response Format supports a narrower type. |
| [ ] | `ChoiceOption` | One learner-visible selectable option | Question Choice or Matching Choice | Use Matching Choice for a possible matching answer in MATCH. Keep the Response Item Reference and learner-visible content together. |
| [ ] | matching left/right item or source/target | The two semantic sides of MATCH | Matching Prompt and Matching Choice | Use Matching Prompt for an item to be matched and Matching Choice for a possible matching answer. Store correct pairings in the Answer Key. |
| [ ] | `SelectionCardinality` | Public rule for how many responses may be selected | Response Selection Rule | Store Exactly One, Exactly a stated count, At Least One, or Any Number in Question Response Format. |
| [ ] | `TextEntryAnswer` | Correctness-neutral text supplied for one named slot | Student Text Entry | Store the Text Entry Slot reference and learner-supplied text inside Student Response. |
| [ ] | `MatchPair` | One submitted prompt-to-choice association | Student Match | Pair one Matching Prompt reference with one Matching Choice reference in Student Response. |
| [ ] | `HotspotRegion` | Public selectable region with a learner label | Hotspot Region | Keep its Response Item Reference, label, and normalized bounds in Question Response Format. |
| [ ] | `HotspotPoint` | Learner-selected normalized coordinate | Student Hotspot Point | Store scale-independent coordinates in Student Response and interpret them against the issued Hotspot Surface. |
| [ ] | `fileUpload.object_key` | Browser-carried reference to an accepted Student upload | Student Upload Reference | Resolve one server-issued Object Reference owned by the exact Student Record and Question Attempt. |
| [x] | `ResponseDefinition::MultipleChoice` | One response variant currently covering MC and MA | Question Response Format plus MC or MA Question Type | Derive MC from Exactly One and MA from a multi-selection Response Selection Rule. Keep the shared control implementation behind Question Response Control. |
| [x] | `ResponseDefinition::FileUpload` or `ExternalTool` treated as a Question Type | Delivery mechanism confused with learner interaction | Question Response Control and Question Backend Capability | Declare the educational Question Type separately and let the normalized Question Presentation select the upload or external-tool control. |
| [x] | `ResponseDefinition` or `ResponseSchemaV1` | Authored or issued answer-free shape of an accepted Student Response | Question Response Format | Use one format-agnostic contract whose closed variants carry the exact response items and shape constraints for the Question Presentation. |
| [ ] | `ResponseFormatReport` | Result of checking a proposed Student Response shape | Student Response Format Check | Report every format issue without evaluating correctness or credit. |
| [ ] | `ResponseFormatViolation` | One reason a proposed Student Response has an unacceptable shape | Student Response Format Issue | Name the exact response-kind, item-reference, selection-count, or numeric-shape problem. |

## Question content and execution

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [x] | Response Family | Catalog filtering and learner interaction grouping | Question Type | Represent MC and MA separately. Derive Question Type from learner interaction and Question Response Format. Express upload and external-tool support through their exact Question Backend capabilities. |
| [x] | `CatalogResponseFamily` and `response_families` Catalog fields | Question interaction categories mixed with File Upload and External Tool controls | Question Type and Question Response Control facets | Publish MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, or HOTSPOT as Question Type. Publish File Upload or External Tool through the separate Question Response Control facet. |
| [x] | Native question family | First-party native implementation grouping | Question Format, Question Type, Question Generator, and Native Question Implementation | Split representation, learner interaction, seeded variation, and implementation selection into explicit fields or registrations. |
| [x] | `QuestionSource::Native.family` | Stored native source discriminator | Question Format, Question Type, and optional Question Generator | Current native source fixtures use the closed unit `Native` variant and explicit Question Format and Question Type fields. Wasm, browser, and committed-corpus gates pass. |
| [x] | `source_family`, `published_family`, or `draft_family` | Source discriminator reused for representation and learner interaction | Question Format, Question Type, and optional Question Generator | Store and compare each concept explicitly during draft validation and publication. |
| [ ] | `TextMatchMode` | Public text-comparison behavior paired with private correct values | Text Response Match Rule | Keep Exact, Case Insensitive, or Normalized comparison in Question Response Format and correct values in the Answer Key. |
| [ ] | `NumericTolerance` | Public numeric-comparison behavior paired with a private correct value | Numeric Response Tolerance | Keep the exact, absolute, relative, or significant-figure rule in Question Response Format and the correct value in the Answer Key. |
| [x] | `NativeQuestionFamily` | Native adapter trait | Native Question Implementation | Register the implementation for an explicit Question Format, Question Type, and optional Question Generator combination. Keep orchestration in the Question Backend. |
| [x] | `FamilyRegistrationKey`, `register_family`, and `families` in the native adapter registry | Native implementation registration and lookup | Native Question Implementation registration | Index the registration by its explicit Question Format, Question Type, Question Generator, and implementation version. |
| [ ] | UI branch by Question Type or Question Format | Type- or source-specific learner interface selection | Question Presentation, Question Response Control, and Question Response Format | Render the Question Response Control declared by the issued Question Presentation and validate the resulting Student Response against Question Response Format. Keep Question Type in Instructor classification and discovery, and keep Question Format in authoring/import/export and server adapters. |
| [x] | Adapter family | Catalog and adapter grouping | Question Backend | Use the exact server-side adapter identity. Keep Question Format and Question Type separate. |
| [ ] | `BrokerProvider`, provider broker, or broker adapter | Server-side integration with an external question system | External Question Provider | Name the configured provider and its exact launch, exchange, render, or grade operation. Keep transport mechanics inside the adapter. |
| [ ] | `ProviderLaunchHandle` | Opaque server-held locator for one provider launch | External Tool Launch Reference | Bind it to the exact External Tool Launch Session and keep URLs, credentials, cookies, and access authority separate. |
| [ ] | `IssuedQuestionFamilyWitnessV1` | Issued server-only source and execution evidence | Question Backend plus Question Attempt Source Record | Record the exact Question Backend separately. Retain source artifacts, implementation versions, asset objects, and digests in the Question Attempt Source Record. |
| [ ] | `ImplementationVersion` used for an adapter, renderer, or grader | Generic software component identity | Question Backend Release, Question Renderer Release, or Question Grader Release | Name the exact component role and retain its stable implementation name and release version in the Question Attempt Source Record. |
| [ ] | `RendererIdentity` | Exact renderer name and installed implementation release | Question Renderer Release | Bind the exact renderer release to each applicable Question Attempt Source Record. |
| [ ] | `BackendCapabilities` | Set of abilities declared by one Question Backend | Question Backend Capabilities | Store the closed Question Backend Capability values and compare the Assignment's requirements before publication. |
| [ ] | `SourceArtifact` | Immutable authored or imported bytes and checksum used for reproduction | Source Object Reference | Name the exact Source Object and checksum inside its owning Draft Question Revision, Question Version, Workspace Import, or Question Attempt Source Record. |
| [ ] | `DraftLocator` in an external Question adapter | Private provider and provider-local item reference for draft content | Question Source | Keep the provider reference and item reference inside the exact Draft Question Revision's private source material. |
| [x] | `UnknownFamily`, `DuplicateFamily`, and `InvalidFamilyDefinition` | Native adapter registration and validation failures | Native Question Implementation failures | `UnknownQuestionImplementation`, `DuplicateQuestionImplementation`, and `IncompatibleQuestionImplementation` name the explicit Question Format, Question Type, Question Generator, and implementation reference. Native Adapter tests pass. |
| [ ] | `QuestionEnvelope` | Answer-free generated payload before or during issuance | Question Variation and Question Presentation | Keep the reproducible Question Version, Question Generator, parameters, and Question Seed in Question Variation. Bind the answer-free prompt and Question Response Format to the exact Question Attempt in Question Presentation. |
| [ ] | `DraftQuestionDefinition` | One complete private authored Question state | Draft Question Revision | Bind the complete authored content, Question Source, Question Format, and Question Grading Material to its exact Draft Question Revision Reference. |
| [ ] | `DraftQuestionSource` | Private authored or imported material used by one draft | Question Source | Bind it to one exact Draft Question Revision and keep Question Format, Question Type, and Question Backend as separate facts. |
| [ ] | `QuestionDefinition` | One complete immutable published Question state | Question Version | Bind the answer-free content, private grading material, source records, metadata, and publication evidence through the exact Question Version Reference. |
| [ ] | `ContentBlock` | One normalized renderable part of a learner prompt | Question Prompt Block | Use the closed Text, Math, Image, Code, or Table form inside an ordered Question Prompt. Require accessible descriptions for visual or structural content. |
| [ ] | `AssetRef` | Logical Question presentation Asset and authored checksum | Question Asset Reference | Name the Question Asset and its authored checksum without granting Object retrieval authority. |
| [ ] | `AssetBindingV1` | Exact public rendition selected for one issued presentation | Presented Question Asset | Bind the Question Asset Reference, rendition checksum, and intrinsic dimensions to the exact Question Presentation. |
| [ ] | `GradingDefinition` | Private correct-response and scoring package | Question Grading Material | Store the Answer Key, Question Grading Rule, and any format-specific private grader input together under one Draft Question revision or exact Question Version. |
| [ ] | `FeedbackContent` | Private teaching hint, correct-response explanation, and rationale | Question Feedback Material | Bind it to one Question Version and release only the policy-approved subset. |
| [ ] | `DisclosedFeedback` | Browser-safe result and teaching view after policy evaluation | Student Feedback | Derive it from the Grading Result, Question Feedback Material, and Student Feedback Release Rule. |
| [ ] | `InspectedStudentScoreFeedbackV1` | Score-only fields attached to an authorized Student Response read | Student Response Inspection | Include only the score fields authorized for that inspection. |
| [ ] | `RandomizationDefinition::Static` | Declaration that one Question Version has fixed authored content | Static Question | Deliver the fixed Question Prompt and Question Response Format. |
| [ ] | `RandomizationDefinition::Seeded`, `GeneratorReference`, and `ParameterSpec` | Seeded content-generation contract | Question Generator Reference and Question Generator Parameter | Name the generator's stable implementation and exact implementation version, preserve its declared parameters, and use a Question Seed to create one Question Variation. |
| [ ] | `GeneratedVariant` | Deterministic output from one Question Generator and parameter set | Question Variation | Bind it to the exact Question Version, Question Generator Reference, parameters, and Question Seed. |
| [ ] | `Seed` in question generation | Value selecting one generated variation | Question Seed | Bind it to one exact Question Version, Question Generator, parameter set, and Question Attempt. |
| [ ] | `PublicByline` and `PublicAuthorName` | Reviewed public attribution attached to a publication | Question Authorship | Preserve ordered reviewed display names as attribution while exact Account relationships supply authority. |
| [ ] | `CatalogPromptProjection` | Static prompt or one generated catalog example | Question Catalog view of Question Prompt | Show answer-free static content or one explicitly labeled generated example. Keep generation inputs and grading material server-held. |
| [ ] | `CatalogProblemSummary` or `CatalogDiscoveryItem` | Answer-free Question listing returned by Catalog search | Question Catalog Entry | Identify one exact Question Version and include its Question ID, type, backend, capabilities, metadata, authorship, availability, and publication time. |
| [ ] | generic Question `title` at a shared boundary | Short answer-free Question label | Question Title | Use it for learner and Catalog identification while keeping the task itself in the Question Prompt. |
| [ ] | `CatalogProblemDetail` | Expanded answer-free Catalog record | Question Catalog Detail | Build it from one Question Catalog Entry plus an answer-free prompt or labeled example, released Question Statistics, and bounded Question Use Summary. |
| [ ] | `PublicationDiff` or `PublicationSemanticProjection` | Answer-free review of one exact Draft Question Revision proposed for publication | Question Publication Review | Name the exact Draft Question Revision and explicit base Question Version, or New Question, and list the changed canonical Question aspects. |
| [ ] | `CatalogSearchQuery`, `CatalogSearchFilter`, or `CatalogSearchPage` | Normalized Catalog discovery request, saved meaning, or result page | Question Catalog Search | Keep filters, continuation, result entries, and same-state facet counts in one search boundary. Saved Question Search stores only the normalized filter meaning. |
| [ ] | `CatalogEvidenceAvailability` | Catalog filter for safely released Question Statistics | Question Statistics Availability | Use Available or Unavailable as release-state filters without implying Question quality. |
| [ ] | Attempt provenance or `AttemptProvenance` | Reproduction data for a Question Attempt | Question Attempt Source Record | Retain the exact backend, generator, source artifacts, assets, component releases, and digests needed for reproduction. |
| [ ] | Course provenance | How a Course Instance was created | Course Origin | Record the exact Blueprint Course and Blueprint Revision, plus source Course Instance for rollover. |
| [ ] | Assignment provenance or `BlueprintAssignmentProvenance` | How an Assignment entered a Course Instance | Assignment Source | Record the exact Blueprint Assignment and Blueprint Revision used for adoption, update, or copy. |
| [ ] | Question provenance | Authored source or contributor history | Question Source or Question Authorship | Use Question Source for private reproducible material. Use Question Authorship for reviewed public attribution and contribution history. |
| [ ] | Source repair | Instructor proposal to improve a Published Question | Question Change Proposal | Open one proposal against one exact Question Version; merging creates a new Question Version in the same lineage. Use Question Pull Request or Question PR as backend aliases. |
| [ ] | `forced_question_correction.flawed_problem_id` or `replacement_problem_id` | Question Versions in a critical correction | Question Version Reference | Store the exact Question ID and Question Version Number pair for both flawed and replacement versions. |
| [ ] | `forced_question_correction.correction_id` | Correction Manifest identity | Forced Question Correction Reference | Name the exact closed Manifest used by every correction Job and recalculation record. |
| [ ] | `forced_question_correction.remediation` without exact affected teaching references | Open-ended correction scope | Forced Question Correction Manifest | Store the closed affected Assignment, Assignment Attempt, Issued Question, and Gradebook targets before execution. |
| [ ] | correction `generation` | Remediation pass sequence | Correction Generation | Scope the positive sequence to one Forced Question Correction Manifest and bind every Job and evidence record to it. |
| [ ] | `question_change_proposal.base_problem_id` | Question Version selected as the PR base | Question Version Reference | Store the exact Question ID and Question Version Number pair. |
| [ ] | Validated Question lifecycle state | Successful publication checks treated as stored content state | Question Publication Readiness | Calculate the complete blocking-issue set for the exact Draft Question Revision without creating an intermediate state. |
| [ ] | Institution or public publication scope | Published Question visibility | Published Question | Give every Approved Instructor access to every Published Question. Use exact relationships for drafts and Student work. |
| [ ] | Published, deprecated, and archived content lifecycle | Mixed publication and selection state | Published Question plus Question Version Availability | Use publication for entry into the Question Corpus. Use Available or Archived for ordinary discovery and new selection. |

## Current Group labels

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [ ] | `EntitlementDecision`, `EntitlementGrant`, or `EntitlementDenial` | Server evaluation of whether one Student may use one Assignment | Assignment Access | Calculate the current access decision from the Student Record, Course Membership, Assignment, effective policy, and lifecycle facts. Return an exact denial reason where access is absent. |
| [ ] | `ApplicablePolicyScopes` or selected policy scopes | Roster-subdivision policy inputs | Exact Course Instance policy and Accommodation facts | Resolve the Base Assignment Policy and direct Student Record-and-Assignment Accommodations without an intermediate scope collection. |
| [ ] | `EntitlementMaterialization`, `EntitlementPurpose`, `MaterializationBasis`, `MaterializationAuthority`, or `MaterializationDisposition` | One generic receipt model spanning unrelated Assignment actions | Exact operation record and Receipt | Let Assignment Attempt creation record accepted start access, Issued Question record issuance, Question Submission Receipt record response acceptance, and Automated Grading Receipt record grading. |
| [ ] | `AssignmentMaterializationPlan`, `AssignmentMaterializationEntry`, or `AssignmentMaterializationCandidate` | Internal creation of an Assignment from one Blueprint Assignment | Assignment Adoption and Assignment Entry | Calculate the complete ordered Assignment Entries, then create the Course Instance-owned Assignment through Assignment Adoption. |
| [ ] | policy preview provenance, Field provenance, `PreviewPolicySourceLayer`, or `TeachingPreviewFieldSource` | Explanation of which rule supplied one Effective Assignment Policy value | Assignment Policy Source | Name the exact Base Assignment Policy, Accommodation Revision, or Student Schedule Adjustment and present those Sources in application order. |
| [ ] | `PolicyPatchSet` or `PolicyPatch` | Sparse direct-Student changes to Assignment Access windows and limits | Accommodation Adjustment | Store only the active specific value or Unrestricted adjustment in the immutable Accommodation Revision. |
| [ ] | `PolicyModificationMode` | Rule for combining an Accommodation with Base Assignment Policy | Accommodation Application Rule | Use Extend Only or Replace and apply it consistently to every adjusted field. |
| [ ] | `ResolvedField` | One calculated Effective Assignment Policy value paired with its supplying rule | Effective Assignment Policy value plus Assignment Policy Source | Keep the resolved value and exact Base Assignment Policy, Accommodation Revision, or Student Schedule Adjustment together for explanation. |
| [ ] | `EffectivePolicyDecision`, `StartVerdict`, or access entitlement result | Server decision for one Student Record and Assignment | Assignment Access | Derive the decision from current membership, lifecycle, Effective Assignment Policy, and evaluation time. |
| [ ] | `LateVerdict` | Late-work result for accepted or refused work | Student Late Work Status or Assignment Access denial | Use On Time, Accepted Late, or Marked Late for accepted work and deny work refused by the Late Work Rule. |
| [x] | `PreviewSyntheticGroupReferences`, `PreviewGroupFact`, `PreviewGroupRole`, and Teaching Preview Group Source | Preview inputs derived from roster subdivisions | Student View Scenario and direct policy facts | The current synthetic preview request carries only its selected moment and explicit direct modifiers. The disconnected MemoryStore conformance corpus that retained the stale input is removed; source and generated-contract searches are clear. |
| [ ] | `PreviewSubject` or `PreviewSubjectKind` | Identity-free calculation input that hides whether a real Student or hypothetical case was selected | Selected Student Record or Student View Scenario | Authorize a selected Student Record before deriving identity-free calculation facts. Represent a hypothetical case explicitly as a Student View Scenario. |
| [ ] | `PreviewScheduleProjection` or `PreviewDisclosureProjection` | Calculated Student-visible schedule or feedback-release preview | Effective Assignment Policy and Student View | Present the resolved schedule and policy-approved visibility for one selected Student Record or Student View Scenario. |
| [x] | `GradingOperationGroupBy` and `groupBy` | Choice to arrange an Instructor Grading Operation list by Question or Student | Grading Operation Focus | Use Question or Student as the closed focus values and bind pagination to that focus. |
| [x] | `GradingOperationGroup` and grading-operation row `group` | Question, Student, or Assignment named by one operation row | Grading Operation Subject | Carry the exact Question, Student Record, or Assignment subject as a closed variant. |
| [x] | "Group by question" or "Group by learner" | Instructor Grading Operation view control | "Show by Question" or "Show by Student" | Label the read choice with its visible effect and use Student consistently. |
| [x] | Catalog facet aggregate `group` | Facet dimension such as Question Type, Backend, Tag, or License | Question Catalog Facet | Name the exact facet and store its value and server-calculated count. |
| [x] | `--ple-radius-group` | Shared radius between control and outer-surface sizes | inset radius | `--ple-radius-inset` is the sole shared inset-radius token across global, page, feature, and component styles. |
| [x] | `groupingCanvas` | Color mix used to calculate the soft surface | soft-surface canvas mix | `THEME_MIX.softSurfaceCanvas` now names the `surfaceSoft` output role directly; focused theme-token tests and TypeScript checking pass. |
| [x] | `PoolDrawBasis.group` and local Question Pool capacity `groupRemaining` | Exact Assignment Entry that identifies one Question Pool | `question_pool_entry` and `poolRemaining` | The server-owned Pool Draw Basis now names the exact Question Pool Assignment Entry; the editor's local capacity calculation uses the same Question Pool term. |
| [ ] | PLE-owned `group`, `groups`, or `grouping` | Generic collection, partition, view, or layout label | Exact owned concept | Use Question Pool, Course Instance, Assignment Entry, Facet, Focus, Subject, Partition, Section, Collection, or another noun that states the structure's purpose. |

## Question identity

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [ ] | Problem | PLE-authored assessment content | Question | Use Question for the stable lineage and Question Version for one immutable publication. Keep "problem" only inside registered external systems that use it. |
| [x] | `ProblemDisplayRef` | Instructor-entered public lookup value | Question ID | Removed the redundant wrapper. The catalog boundary accepts `QuestionId` directly, preserving canonical parsing and a version-free public lookup identity. |
| [ ] | `ProblemId` plus `VersionId` | Two server UUIDs currently assigned to one immutable publication | Question ID plus Question Version Number | Use the stable Question ID and its positive monotonic Question Version Number. The first publication is 1, and each accepted same-lineage change advances by exactly one. |
| [ ] | `ProblemVersionRef` | Composite server reference to one immutable publication | Question Version Reference | Carry the exact Question ID and Question Version Number pair. |
| [ ] | `published_question_version.problem_id`, `published_question_version.version_id`, or a PostgreSQL `(problem_id, version_id)` reference | Stored identity for one immutable Question Version | `(question_id, version_number)` | Use a positive integer assigned under the Question's publication lock, make the pair unique, and use it as the referenced identity. Keep `published_at` as a separate timestamp with time zone. |
| [ ] | standalone draft revision counter without its Draft Question parent | Ambiguous private authored state | Draft Question Revision Reference | Carry the exact Draft Question Reference and positive Draft Question Revision Number pair. |
| [ ] | `WorkspaceId` used as the identity of authored Question content | Workspace authority mixed with Draft Question identity | Authoring Workspace Reference plus Draft Question Reference | Use the workspace relationship for access and the Draft Question Reference for the authored lineage. Use a Draft Question Revision Reference when exact content matters. |
| [ ] | Instructor workspace or workspace draft in Question-authoring UI | Interface label that hides whether the workspace or Question is meant | Question Editor, Authoring Workspace, or Draft Question | Use Question Editor for the interface, Authoring Workspace for the private ownership root, and Draft Question for one authored lineage. |
| [ ] | Assignment workspace treated as a stored Assignment aggregate | Instructor editing interface | Assignment Workspace | Build the interface from the Assignment and its exact Revisions, access, schedule, readiness, and Student View. Give the Workspace no separate identity or authority. |
| [ ] | `NamedQuestionCollectionShare` | Recipient-specific access to one Question Collection | Question Collection Share | Preserve the collection owner, give the recipient answer-free inspection and copy access, and keep editing and Course authority separate. |
| [x] | `ProblemCollectionSelectionAvailability::Retained` | Stored selection state on a Question Collection Entry | Current Question Version Availability | Question Collection members now carry the shared structured `QuestionVersionAvailability`; the strict decoder, repository filter, and UI use its exact Available or Archived state and reason. |

## Identity, course creation, and ownership

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [ ] | `capability_brokers.sql`, `session_resolution_broker.sql`, or `authentication_ceremony_brokers.sql` | Protected database checks and Account-session operations | Exact authorization check or authentication operation | Name each file and operation by the Account, Session, Course, workspace, or support decision it owns. Use the ordinary `ple_api` boundary for callable database operations. |
| [ ] | deleting or role-changing an Account to disable access | Authentication shutdown mixed with identity history | Account State and Account State Event | Suspend or close the Account after the exact authorized readiness check and preserve its immutable Product Role and retained records. |
| [x] | service-login provisioning in local stack tooling | Creation or refresh of disposable database service logins | Service Login Setup | `process_logins.setup_service_logins` and its failure contract name the operation; lifecycle, focused tests, and local-stack operations use the same boundary. |
| [ ] | owning instructor | Responsibility for a Question | Question Owner | Name the owned Question and preserve shared visibility for every Approved Instructor. |
| [ ] | owning instructor | Responsibility for a reusable course | Blueprint Course Owner | Name the owned Blueprint Course and its publication, fork, and update decisions. |
| [ ] | owning instructor | Responsibility for private draft work | Authoring Workspace Owner | Name the owned Authoring Workspace; use Workspace Collaborator for an explicit contributor relationship. |
| [ ] | owner | Course Instance accountability | Assigned Instructor and Teaching Team | Use Assigned Instructor for the required accountable member and Teaching Team for every current Instructor Course Membership with equal teaching authority. |
| [ ] | generic user | Global login identity | Account | Use Student Record, Question Owner, Course Membership, or another exact relationship where the context concerns authority. |
| [ ] | Account Role | Immutable global Account classification | Product Role | Use Course Membership Role separately for participation inside one Course Instance. |
| [ ] | `AccountId`, `UserId`, or `StudentId` | Generic or role-shaped internal Account identity | Account Reference | Name the global Account directly. Use Student Record Reference when the value instead names course-scoped Student work. |
| [ ] | `CourseId`, `CourseMembershipId`, or `StudentRecordId` | Internal identity for a Course Instance relationship | Course Instance Reference, Course Membership Reference, or Student Record Reference | Use the complete owned subject regardless of whether storage represents the Reference as an integer or UUID. |
| [ ] | `student_record.membership_id` as the Student Record's unique parent | Educational history bound to one enrollment episode | Student Record owned by Student Account and Course Instance | Keep one stable Student Record for the Account-and-Course pair. Bind each new Student Course Membership to that record so re-enrollment preserves the same course history. |
| [x] | co-instructor invitation | Instructor-only Teaching Team target search, creation, list, and revocation | Instructor Course Invitation | The Instructor-only endpoint has its own request and view contracts, canonical paths, generated types, strict decoder, and visible label. The generic Course Invitation remains the Account-pending acceptance boundary and carries the exact Course Membership Role in the domain model. |
| [x] | backend family, response family, or closed eight-family set | PLE Question implementation and answer-shape terminology | Question Backend and Question Format | The Question Model, authoring/import documentation, preview result, and private grading boundaries now identify the exact Question Backend or Question Format rather than an invented family aggregate. |
| [x] | operation family | Closed Sysadmin support capability classification | Operation Kind | Authorization and Product Role documentation now use the stored `operation_kind` meaning and the matching wrong-kind failure condition. |
| [ ] | `AssignmentId`, `AssignmentAttemptId`, `IssuedQuestionId`, or `QuestionAttemptId` | Internal identity in the Assignment activity hierarchy | Assignment Reference, Assignment Attempt Reference, Issued Question Reference, or Question Attempt Reference | Preserve the exact hierarchy level in the type and field name. |
| [ ] | `WorkspaceId`, `AssetId`, or `ObjectId` | Internal identity for authoring or stored media | Authoring Workspace Reference, Asset Reference, or Object Reference | Name the owned record directly and keep authorization in its exact owning relationship. |
| [ ] | `SessionId`, `EmailAuthenticationChallengeId`, `PasskeyId`, or `WebauthnCeremonyId` | Internal identity for authentication records | Authenticated Session Reference, Email Authentication Challenge Reference, Passkey Reference, or Passkey Ceremony Reference | Use the complete authentication subject and let its Account relationship and current state supply meaning. |
| [ ] | `StartedAssignmentAttemptId` | Redundant type alias for a newly started Assignment Attempt | Assignment Attempt Reference | Name the Assignment Attempt directly; its state states whether it was newly started. |
| [ ] | `WorkspaceImportId` or `CourseRosterImportId` | Internal identity for a staged import | Workspace Import Reference or Course Roster Import Reference | Name the exact import and keep its Authoring Workspace or Course Instance parent in the stored relationship. |
| [ ] | `NamedQuestionCollectionId` or `NamedQuestionSavedSearchId` | Internal identity for private Question organization | Question Collection Reference or Saved Question Search Reference | Name the owned Question curation record directly and resolve its exact Account relationship. |
| [ ] | `BlueprintAssignmentId` | Internal identity for one reusable Assignment definition | Blueprint Assignment Reference | Name the exact Blueprint Assignment within its Blueprint Course and Revision context. |
| [x] | `CourseReference` | Public route locator for a live teaching course | Course Instance Reference | `CourseInstanceReference` now carries the `C-` locator through the Question Model, curriculum-adoption contracts, regenerated browser types, strict decoders, typed clients, route helpers, tests, and durable documentation. |
| [x] | `RunReference` | Public route locator for one Student pass through an Assignment | Assignment Attempt Reference | Active source, tests, generated contracts, and durable docs use `AssignmentAttemptReference`; no `RunReference` remains. |
| [x] | `runRef` in browser route parameters | Public route parameter for one Student pass through an Assignment | `assignmentAttemptRef` / `assignmentAttemptReference` | Active route parameters use `assignmentAttemptRef`, and route-resolution contracts use `assignmentAttemptReference`; no `runRef` remains. |
| [x] | `run_page.tsx`, `run_summary_page.tsx`, or `run_page_recovery.ts` | Browser implementation files for one Student pass through an Assignment | `assignment_attempt_page.tsx`, `assignment_attempt_summary_page.tsx`, and `assignment_attempt_page_recovery.ts` | The Assignment Attempt page and page-local recovery owner are current; durable architecture documentation now names them directly. |
| [x] | `WorkspaceReference` | Public route locator for private draft-authoring work | Authoring Workspace Reference | `AuthoringWorkspaceReference` now carries the `W-` locator through the Question Model, generated contracts, strict decoder, route helper, editor surface, tests, and identity documentation. |
| [x] | `ProblemCollectionReference` | Public route locator for private Question organization | Question Collection Reference | `QuestionCollectionReference` now uses the canonical `QC-` locator across Rust, generated contracts, strict browser decoding, typed clients, and curation tests. |
| [x] | `SavedProblemSearchReference` | Public route locator for one stored Question Catalog filter | Saved Question Search Reference | Rust, generated contracts, strict browser decoding, typed clients, and tests now use `SavedQuestionSearchReference` with the canonical `QS-` public locator. |
| [x] | `CatalogProblemSummary`, `CatalogProblemDetail`, and Catalog Problem client operations | Answer-free Catalog Question browse and detail projections | Catalog Question Summary, Catalog Question Detail, and Catalog Question operations | The Rust model, regenerated browser contracts, strict decoders, clients, runtime cache, authoring consumers, and assignment lookup now use the exact Question terminology with no compatibility aliases. |
| [x] | `BlueprintReference` | Public route locator for one reusable course | Blueprint Course Reference | `BlueprintCourseReference` is now the Rust, generated-contract, decoder, client, curriculum-adoption, and durable-document contract for the `BP-` locator. |
| [x] | `GradingOperationReference` | Public route locator for Instructor-requested grading work | Instructor Grading Operation Reference | `InstructorGradingOperationReference` now carries the `GO-` locator through the Question Model, generated contracts, Gradebook decoders and clients, Instructor workspace state, route helpers, and focused tests. |
| [x] | `GradingOperationState` | Current progress of one Instructor-requested grading operation | Instructor Grading Operation State | `InstructorGradingOperationState` is the closed Rust, generated-contract, and strict Gradebook decoder state. It remains separate from automated grading, scoring, and Job states. |
| [x] | `NavigationResolution::Run` | Route resolution for one Student Assignment pass | Assignment Attempt route resolution | `NavigationResolution::AssignmentAttempt` carries the exact Course Instance, Assignment, Student Record, and Assignment Attempt IDs; no Run variant remains. |
| [ ] | Course Invitation email status | Transport outcome for an invitation message | Course Invitation Email Delivery | Report queued, provider-accepted, needs-attention, or cancelled separately from invitation acceptance. |
| [ ] | idempotency key | Opaque mutation retry binding | Retry Token | Reuse the same Retry Token with the same request to resolve the same accepted Receipt. Qualify the token by its operation where multiple mutations share a boundary. |
| [ ] | `GradingOperationActionId` used as an idempotency value | Retry binding for one Instructor Grading Operation action | Instructor Grading Operation Retry Token | Bind the token to the exact operation, action, request digest, and accepted Receipt. |
| [ ] | `CurriculumAdoptionIdempotencyKey` | Browser-supplied retry binding for Blueprint operations | Blueprint Adoption Retry Token | Bind one Retry Token to the exact request digest and completed Blueprint Adoption Receipt. |
| [ ] | provider key | Configured opaque external-provider selector | External Question Provider Reference | Store the provider reference separately from endpoints, credentials, and provider-local resource references. |
| [ ] | `CourseThemeId` | Closed visual palette selector | Course Theme | Select one reviewed design-system palette for the complete Course Appearance. |
| [ ] | `CourseBannerId` | Same-origin route identity for the current banner | Course Banner Reference | Resolve it only through the Course Instance-owned Asset delivery route. |
| [ ] | `CourseBannerCandidateId` and `CourseBannerCandidateReceipt` | Temporary uploaded banner awaiting an atomic save | Pending Course Banner and upload Receipt | Bind the upload to the exact Course Instance, Account, expiry, Object Reference, and accepted upload Receipt. |
| [ ] | `CourseBannerAltText` and `CourseBannerAlternativeText` | Accessibility treatment for the Course Banner | Course Banner Alternative Text | Store Decorative or concise Informative text with the Course Banner. |
| [ ] | `CourseBannerPresentation` | Browser-safe current banner read | Course Banner | Return its Course Banner Reference and Course Banner Alternative Text. |
| [ ] | `CourseBannerMutation` | Keep, remove, or replace action in an appearance update | Course Appearance Update | Apply the complete desired banner action atomically with the Course Theme. |
| [ ] | Course Appearance projection | Browser-safe theme and banner read | Course Appearance | Return the Course Theme, Course Banner when present, and Course Appearance Revision. |

## Grades and statistics

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [ ] | `CourseGradeScheme` | Course-owned final-grade configuration | Course Grade Scheme | Calculate one Course Grade per Student Record from selected Assignment Grades. |
| [ ] | `CourseGradeMode` | Total-points or weighted-category calculation | Course Grade Mode | Keep Total Points and Weighted Categories as the closed calculation choices. |
| [ ] | `WeightedGradeCategory` | Ordered weighted Assignment grouping | Grade Category | Store its title, weight, order, and Drop Lowest count in the Course Grade Scheme. |
| [ ] | `GradeCategoryId` | Stored identity of one category in a Course Grade Scheme | Grade Category Reference | Name the exact Grade Category within its owning Course Grade Scheme. |
| [ ] | `dropped_assignment_ids` | Assignment references omitted by a Drop Lowest calculation | Dropped Assignment Grades | Report the exact Assignment Grades omitted from this Course Grade calculation while preserving the Assignments and Student work. |
| [ ] | mutable Course Grade Scheme plus `CourseGradeSchemeRevision` counter | Course Grade configuration and its edit sequence | Course Grade Scheme plus Course Grade Scheme Revision | Store every complete scheme and Assignment setting set as an immutable revision. Bind calculations and Gradebook reads to the exact revision. |
| [ ] | `LetterBand` | Final-score threshold mapped to an Instructor label | Letter Grade Band | Apply its inclusive threshold after the Course Grade Rounding Rule. |
| [ ] | `CourseGradeRoundingRule` | Final Course Grade rounding | Course Grade Rounding Rule | Apply it once before Letter Grade Bands. |
| [ ] | `StatisticsContribution` | Legacy name that conflates one accepted grade with a cohort rollup | Question Statistics Observation | Record one accepted graded Question Attempt exactly once at its receipt boundary, with correctness and eligible-choice selections when the Question format supports them. |
| [ ] | `QuestionVersionStatistics` | Exact accepted-grade counts for one immutable Question Version | Question Version Statistics | Retain accepted graded-attempt count, correct count, and eligible-choice selection counts without Account, Course, Student Record, response, or receipt identity. |
| [ ] | `QuestionVersionStatisticsSnapshot` | Retention-safe persisted exact-count state | Question Version Statistics Snapshot | Validate that correct and any one choice-selection count cannot exceed accepted graded-attempt count. |
| [ ] | `QuestionStatisticsDisclosure` | Result of applying the minimum-cohort rule to one statistics read | Question Statistics Availability plus released Question Statistics | Return Unavailable without counts or partial measures, or return the complete permitted Question Statistics. |
| [ ] | `StudentClassStatistics` | Student-visible course-local aggregate after policy evaluation | Class Statistics | Return only the current course-local measures allowed by the Student Feedback Release Rule. |
| [ ] | `statistics_eligible` | Issue-time eligibility for global aggregation | Question Statistics Eligibility | Derive it from Assignment Entry scoring facts and freeze it on Issued Question. |
| [ ] | statistics contribution receipt | Idempotent aggregation witness | Question Statistics Observation Receipt | Bind the source accepted grade and exact observation so it contributes once before its private evidence is deleted. |
| [ ] | `difficulty_index` or item-analysis `difficulty` | Mean normalized Question score in a stated cohort | Question Difficulty | State the cohort and remember that a larger value means the Question was easier for that cohort. |
| [ ] | `discrimination_index` or item-analysis `discrimination` | Correlation of Question credit with rest-of-Assignment credit | Question Discrimination | Calculate it only for a cohort with sufficient variation and label its scope. |
| [ ] | `StatisticsDisclosurePolicy` | Minimum cohort for safe global statistics visibility | Question Statistics Release Rule | Release only identity-free metrics after the independent cohort reaches the configured floor. |
| [ ] | `CatalogDiscoveryEvidence` | Question Catalog view of safe aggregate measures | Question Statistics | Present the released exact-version measures without creating a second evidence model. |
| [ ] | `CatalogUsageSummary` and `CatalogUsageDetail` | Aggregate and Instructor-authorized placement counts | Question Use Summary | Separate bounded use counts from performance statistics and Student records. |
| [ ] | `AssignmentItemAnalysis`, `CourseItemAnalysisReport`, or `assignment_item_analysis` | Course-local Question outcomes inside one Assignment | Assignment Question Analysis | Bind each row to the source Assignment Entry, exact Question Version, and Scoring Generation. |
| [ ] | `ItemAnalysisResponseBucket` | Correct, partial, incorrect, or unanswered outcome category | Question Outcome Category | Use Correct, Partial Credit, Incorrect, or Unanswered and keep Unscored separate. |
| [ ] | `ItemAnalysisResponseDistribution` | Identity-free counts of course-local Question outcomes | Question Outcome Distribution | Count the closed Question Outcome Categories for one Assignment Question Analysis. |

## Database records

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [ ] | `student_feedback_release.projection` | Policy-approved result and teaching information released to a Student | Student Feedback | Store the exact released Student Feedback and bind it to the Question Submission and Student Feedback Release Rule used. |
| [ ] | `ple_private.workspace_qti_import` | QTI-specific staged import used as the general authoring import root | Workspace Import | Store the exact Question Format on the Workspace Import and keep QTI package details inside its registered-format payload. |
| [ ] | `workspace_qti_import.state` with Prepared or Committed | Review state for one private staged import | Workspace Import State | Use Staged or Committed and keep Draft Question publication as a separate decision. |
| [ ] | `QtiItemImportStatus`, `QtiProfileItemStatus`, `QtiSafeItemStatus`, or `QtiProfileItemDisposition` | Per-source-item accepted or rejected QTI import outcome | Workspace Import Item Result | Use the format-agnostic item result at the Workspace Import boundary and retain QTI diagnostics inside the registered adapter payload. |
| [ ] | `published_flat_question_grading`, `published_qti_question_grading`, or `workspace_qti_import_grading` | Private answer-bearing data for one import or exact Question Version | Question Grading Material | Bind the private material to its exact Workspace Import or Question Version and keep Question Format details inside the private package. |
| [ ] | `automated_grading_operation.state` with Queued, Running, Completed, or Failed | Background execution mixed with Student-facing grading progress | Job State, Question Submission Grading State, and Automated Grading Receipt | Let the Job own execution, derive Student-facing grading state, and use the Receipt as immutable completion evidence. |
| [ ] | `ple_data.course_assignment_analysis` | Course-local Assignment calculation for one scoring generation | Assignment Analysis | Bind the exact Course Instance, Assignment, Scoring Generation, cohort facts, and calculated aggregate. |
| [ ] | `ple_audit.course_analysis_evidence` | Immutable evidence for an Assignment Analysis calculation | Assignment Analysis Receipt | Bind the exact Assignment Analysis, digest, and completion time. |
| [ ] | `RosterImportRowStatus` | Preview result for one normalized roster-import row | Course Roster Import Row Result | Use Ready to Invite, Already a Member, Invitation Pending, Duplicate, or Invalid and commit only Ready to Invite rows. |
| [ ] | `ple_audit.object_delivery_access_event` | Immutable use of one authorized Object Delivery | Object Delivery Access Event | Record the exact Object Delivery, Account, access decision, and time. |
| [ ] | `ple_audit.retention_lifecycle_event` | Immutable Course Retention Plan action | Course Retention Event | Record the exact Plan Revision, action, Job result, digest, and time. |
| [ ] | `ple_private.course_object_metadata` | Course-owned relationship to immutable stored bytes | Object Reference | Bind the Object to its exact Course Instance purpose and keep the physical Object Address server-held. |
| [ ] | `course_object_metadata.scope` with Student Upload, Student Artifact, Course Export, or Protected Feedback | Broad label standing in for Object ownership | Exact Student Upload, Artifact, Assignment Export Artifact, or Question Feedback Material relationship | Store the exact owning relationship and derive data class, access, delivery, and retention from that owner. |
| [ ] | `ple_private.external_tool_provider_cache` | Encrypted expiring provider data | External Question Provider Cache Entry | Bind the provider reference, resource digest, payload digest, fetch time, and expiry. |
| [ ] | `external_tool_exchange.state` with Verifying, Verified Pending, or Committed | Verification and commit progress for one provider interaction | External Tool Exchange State | Use Verifying, Ready to Commit, Committed, Failed, or Cancelled and require the exact state-owned fields at each transition. |
| [ ] | `ple_private.external_tool_passback_state` | Outbound state attached to an External Question Provider exchange | External Tool Result or LTI Grade Return | Commit provider output as the External Tool Result inside its Exchange. Store LMS grade delivery only in an LTI Grade Return bound to an exact Assignment Grade. |
| [ ] | `assignment_export_request.export_id` | Stable export-request identity | Assignment Export Reference | Name the exact Account-attributed Request and use the Reference for its Job, Manifest, Artifacts, and Receipt. |
| [ ] | `assignment_export_request.manifest_object_id` without typed export scope | Opaque Object standing in for frozen export inputs | Assignment Export Manifest | Bind the exact Assignment Revision, Question Versions, permitted answer-bearing material, Objects, requested formats, and component releases. |
| [ ] | export `artifact_kind` | Requested file or package representation | Assignment Export Format | Use DOCX, PDF, QTI, or authorized Answer Key Package as the closed formats. |
| [ ] | export request `state` using queued or ready | Request outcome mixed with Worker Job execution | Assignment Export State | Use Requested, Completed, Failed, or Cancelled for the durable Request and retain Ready or Leased only in Worker Job State. |

## Authentication records

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [ ] | `webauthn_ceremony` or `WebauthnCeremonyId` | Short-lived passkey registration or authentication exchange | Passkey Ceremony | Bind the existing Account, purpose, challenge hash, browser-binding hash, expiry, and one-use state. Keep WebAuthn spelling inside the protocol adapter. |
| [ ] | `ple_private.account_authentication_email` | Private verified email credential for an existing Account | Authentication Email | Keep the normalized lookup value and delivery form private and use a server-owned verified-email change to replace them. |
| [ ] | `ServerCorrelation`, `PersistedCorrelation`, or `CorrelationIssuer` | Server-authenticated value binding one provider launch to its exact grading context | External Tool Launch Session authentication state | Create it from the External Tool Grading Context, store it only with the protected Launch Session, and validate it before accepting a provider result. |
| [ ] | `ScoredEmbedLaunchLedger` | Exact server-held state for one external-provider launch and result match | External Tool Launch Session | Bind the exact grading context, provider, source digest, profile, seed, expiry, Challenge, and single-use state. |
| [ ] | `ScoredEmbedNonce` | Unpredictable per-launch value that the provider must echo in its signed result | External Tool Launch Challenge | Create it from server randomness, bind it to one Launch Session, expire it with that Session, and accept it once. |
| [ ] | scored-embed result JWT or `result_token` | Provider-signed response carrying a result and exact launch-match claims | External Tool Provider Result Token | Verify signature, Challenge, launch-binding digest, applicable expiry, and single use before accepting an External Tool Result. |
| [ ] | `GradeBinding` in an external Question adapter | Exact attempt, Question Version, and seed named for provider grading | External Tool Grading Context | Bind every provider request and returned result to one exact context. |
| [ ] | `VerifiedProviderGrade` | Provider output accepted after authentication and context checks | External Tool Result plus Grading Result | Commit the verified External Tool Result through the ordinary Question Submission grading boundary before creating the Grading Result. |

## Blueprint and Course Instance records

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [ ] | Reusable Curriculum | Reusable answer-free course aggregate | Blueprint Course | Use Blueprint Course for the reusable aggregate and Blueprint Revision for one immutable version. |
| [ ] | Reusable Assignment | Assignment definition inside reusable course content | Blueprint Assignment | Keep live release, schedules, Student Records, and Student work in the Course Instance. |
| [ ] | `BlueprintCourseModuleView` or `CurriculumSemanticModule` | Labeled ordered reusable section within one Blueprint Revision | Blueprint Module | Store its label and ordered Blueprint Assignments as part of the immutable Blueprint Revision Content. |
| [ ] | `blueprint_revision_id` plus `blueprint_id` and `revision` | Redundant identity for one immutable Blueprint Revision | Blueprint Revision Reference | Use the exact Blueprint Course Reference and positive monotonic Blueprint Revision Number pair as the stored identity. |
| [ ] | `BlueprintModuleId` or `module_id` in Blueprint content | Opaque locator for one retained reusable module | Blueprint Module Reference | Name the exact Blueprint Module within its Blueprint Course and let the ordered module collection own authored order. |
| [ ] | `BlueprintModuleEditHandle` or `BlueprintAssignmentEditHandle` | Retained-or-new choice inside a complete Blueprint Revision edit | Blueprint Module Edit Choice or Blueprint Assignment Edit Choice | Carry the exact retained Reference or New choice and let the ordered revision content own position. |
| [ ] | Blueprint Instantiation that flattens modules into Assignments | Applying a Blueprint Revision to a Course Instance | Course Module | Create the ordered Course Modules and their ordered Assignments so the live course retains the authored Blueprint structure. |
| [ ] | Curriculum Adoption | Applying reusable Blueprint content | Blueprint Adoption | Name the exact operation as Blueprint Instantiation, Assignment Adoption, Course Rollover, Course Term Shift, Controlled Blueprint Update, or Selected Assignment Copy where applicable. |
| [ ] | Alpha Course | Reusable source course | Blueprint Course | Use the ordinary Blueprint Course publication, revision, adoption, and ownership model. |
| [ ] | `CourseInstanceWitness` | Exact observed Course Instance revision state | Course Instance Snapshot | Capture the schedule revision and ordered Assignment revisions as an immutable operation precondition. |
| [ ] | `ObservedBlueprintSource` | Blueprint Course locator plus exact observed revision | Blueprint Revision Reference | Carry the exact Blueprint Course Reference and Blueprint Revision Number pair selected for review or adoption. |
| [ ] | `AssignmentDefinitionSourceView` | One Blueprint Assignment selected from an exact Blueprint Revision | Blueprint Assignment Reference plus Blueprint Revision Reference | Name the reusable Assignment lineage and exact revision without turning the read view into durable Assignment Source evidence. |
| [ ] | `BlueprintAdoptionEligibility` | Server decision that one exact adoption request may proceed | Blueprint Adoption Readiness | Report the complete blocking issue set for the exact source, operation, and destination snapshot. |
| [ ] | `CourseInstanceEligibility` | One generic readiness result shared by rollover, term shift, controlled update, and selected Assignment copy | Blueprint Adoption Readiness | Bind the complete blocking issues to the exact operation, source revisions, destination snapshot, and proposed changes. |
| [ ] | `CurriculumPinReplacements` | Reviewed unavailable-Question replacements for one adoption request | Question Version Substitutions | Store each exact unavailable and replacement Question Version Reference and preserve the source Blueprint Revision. |
| [ ] | `CourseScheduleRevision` as a compare-and-swap counter | Course-wide schedule edit sequence | Course Schedule Revision Reference | Make the value name one immutable Course Schedule Revision containing the exact Course Term and course-wide scheduling facts. Create the next revision for an accepted schedule change. |
| [ ] | `CourseDate` | Validated date inside a Course Term | Course Date | Use the exact `YYYY-MM-DD` calendar value without treating it as an instant. |
| [ ] | `IanaTimeZone` in teaching contracts | Course-owned scheduling zone | Course Time Zone | Store one exact IANA name on the Course Term and use it for every Instructor-facing local schedule value. |
| [ ] | `CourseLocalDateTime` | Instructor-facing wall-clock schedule input | Course Local Date and Time | Resolve it only with the Course Term's Course Time Zone and refuse nonexistent or ambiguous local times. |
| [ ] | `RelativeAssignmentSchedule` or `RelativeScheduleMoment` | Reusable Blueprint schedule intent | Relative Assignment Schedule | Store each moment as a calendar-day offset from Course Term start plus a local time. |
| [ ] | `ResolvedRelativeAssignmentSchedule` or `ResolvedRelativeScheduleMoment` | Live Course Instance schedule calculated from reusable intent | Resolved Assignment Schedule | Store the exact absolute available, due, and close times on the resulting Assignment Revision. |
| [ ] | `ActivityTimestamp` | Generic absolute-time type used across unrelated facts | Exact subject-qualified time | Name the meaning at each boundary, such as Question Publication Time, Submission Acceptance Time, Issued Time, or Event Recorded Time. |
| [ ] | term shift that rewrites Assignment schedules in place | Course Term change and its resolved Assignment schedule changes | Course Schedule Revision plus successor Assignment Revisions | Commit the new Course Schedule Revision and every affected Assignment Revision atomically. Preserve the issued timing facts of existing Assignment Attempts. |
| [ ] | `CourseInstanceCreationWitness` | Reserved pre-creation authority and source binding | Course Instance Creation Reservation | Bind the exact Course Origin source, target Course Term, authorizing Account, request digest, Retry Token, and reserved Course Instance reference. |
| [ ] | `BlueprintCourseCreationWitness` | Reserved Blueprint Course identity and source binding for a fork | Blueprint Fork Reservation | Bind the exact source Blueprint Revision, authorizing Account, request digest, Retry Token, and reserved Blueprint Course Reference. |
| [ ] | `CourseInstanceBlueprintApplication` | Blueprint source that established a Course Instance | Course Origin | Record the exact Blueprint Course and Blueprint Revision, plus the source Course Instance for rollover. |
| [ ] | `CourseInstanceApplicationBinding` | Destination snapshot paired with its established Blueprint source | Course Instance Snapshot plus Course Origin | Keep the immutable Course Origin as source history and use the Course Instance Snapshot only as the operation precondition. |
| [ ] | `RolloverCourseInstanceManifest` | Closed copied and excluded state for one rollover | Course Rollover Manifest | List the exact reusable teaching state to copy and explicitly exclude Student Records, Student activity, and delivery records. |
| [ ] | `CourseInstanceReceiptTarget` | Generic closed variant containing several Blueprint-operation receipts | Exact Blueprint Adoption Receipt | Return the receipt for Course Rollover, Course Term Shift, Controlled Blueprint Update, Selected Assignment Copy, or authorized record correction directly. |
| [ ] | `CourseInstanceImportWitness` | Observed Assignment source and destination revision | Assignment Source Snapshot | Capture the exact Assignment Source and destination Assignment revision used as an operation precondition. |
| [ ] | `AppliedAssignmentImportEvidence` | Committed Blueprint Assignment application | Assignment Source and Blueprint Adoption Receipt | Store durable Assignment Source on the destination and immutable completion evidence in the Receipt. |
| [ ] | `CurriculumReplayStatus` or a separate Applied/Replayed response field | Whether a Retry Token caused a new commit or resolved an existing result | Blueprint Adoption Receipt | Return the same accepted Receipt for the same request and Retry Token. Keep replay handling as operation behavior rather than a second product state. |
| [ ] | Curriculum semantic payload | Canonical reusable Assignment or Course content | Blueprint Revision Content | Include the exact answer-free structure, policy defaults, relative schedules, and Question Version References owned by the Blueprint Revision. |
| [ ] | `CurriculumSemanticDigest` | SHA-256 of canonical reusable content | Blueprint Content Digest | Hash the complete versioned canonical Blueprint Revision Content encoding. |
| [ ] | `CurriculumSemanticEnvelope` | Canonical reusable content bytes and digest | Blueprint Revision Content record | Store the encoding version, canonical Blueprint Revision Content, and complete Blueprint Content Digest together. |
| [ ] | `CurriculumSemanticComparison` | Equality check for reusable content | Blueprint Content Check | Compare complete Blueprint Content Digests and report whether the revision content matches. |
| [ ] | curriculum pin | Exact immutable Question publication selected in Blueprint content | Question Version Reference | Store the one Question Version identity and resolve its parent Question through the required relationship. |
| [ ] | `CourseAppearanceRevision` as a compare-and-swap counter | Course visual edit sequence | Course Appearance Revision | Store each complete Course Theme, Course Banner, and alternative-text state as an immutable revision. Make the Course Instance point to the current revision. |
| [ ] | `CourseAppearanceUpdate` that rewrites the current appearance | Atomic theme or banner edit | New Course Appearance Revision | Name the exact current revision as the edit precondition and create the next immutable revision on acceptance. |
| [ ] | `TeachingOperationRevision` or `TeachingOperationRevisionResponse` | One generic counter reused across unrelated teaching records | Exact subject revision, change number, or state precondition | Use Assignment Revision Reference, Accommodation Revision Reference, Course Grade Scheme Revision Reference, Course Retention Plan Revision Reference, or Course Roster Change Number as appropriate. Use the exact current Course Invitation or Instructor Approval state for their transitions. |
| [ ] | `RosterRevision` | Concurrency value for membership changes in one Course Instance | Course Roster Change Number | Bind paginated Course Roster and Gradebook reads to one roster state without implying retained roster revisions. |
| [ ] | mutable Course Retention Plan plus generic teaching revision | Retention schedule and its edit history | Course Retention Plan Revision | Store each complete plan state as an immutable revision and bind every retention Job, Manifest, Event, and Receipt to it. |
| [ ] | `course_retention_plan.stage` with Archive, Delete Private Artifacts, or Purge | Scheduled retention work kind | Course Retention Action | Name Archive Student Records, Delete Private Artifacts, or Purge Student Records and bind the exact action to the Plan Revision and Manifest. |
| [ ] | `course_retention_plan.state` with Scheduled, Running, Completed, or Cancelled | Retention intent mixed with background execution | Course Retention Plan Revision plus Job State and Course Retention Event | Keep the scheduled intent in the Plan Revision, execution in the Job, and completed fact in the Event. |
| [ ] | `RetentionStateView` or coarse retention `state` | Current data-retention stage for one Course Instance | Course Retention State | Use Active, Notice Due, Student Records Archived, or Student Records Purged and keep this axis separate from teaching, membership, and Job states. |
| [ ] | `RetentionNotificationView` | Instructor-facing notice of an upcoming retention choice | Course Retention Notice | Record the archive, purge, or extension intent and creation time without treating display as message delivery. |
| [ ] | `RetentionDispositionView::Delete` for assignment definitions | Broad permission to remove Assignment content | Assignment Revision Retention Rule plus Object Cleanup Manifest | Preserve every Assignment Revision and issued fact needed by retained Student work. Limit cleanup to exact unreferenced Draft Assignment Revisions and private Objects. |
| [ ] | `RetentionReadView` | Browser read combining current data stage, plan revision, notice, and Assignment handling | Course Retention State, Course Retention Plan Revision, Course Retention Notice, and Assignment Revision Retention Rule | Return each exact concept as a named field and keep current data state separate from future scheduled work. |
| [ ] | `RetentionArchiveRequest` or `RetentionExtendRequest` | Requested archive choice or schedule extension | New Course Retention Plan Revision | Name the current Plan Revision as the precondition and create the next complete immutable Revision with its exact action, time, and Manifest. |
| [ ] | `RetentionActionResponse` | Immediate response combining retained-data state and background action progress | Course Retention Plan Revision plus accepted Job or Course Retention Event | Return the accepted Plan Revision and Job when work is scheduled; return the Event and Receipt only after the action commits. |

## Background work

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [x] | Job Family | Paired preparation handler and durable committer | Job Kind Registration | `worker_job.handler_kind` names the closed Job Kind; documentation now identifies the complete Job Handler/Effect Committer registration. Markdown and active-source checks pass. |
| [x] | handler family | Selecting background behavior by queued payload | Job Kind or Job Kind Registration | Current worker documentation uses Job Kind for payload classification and Job Kind Registration for the executable registration. Markdown and active-source checks pass. |
| [x] | payload family | Closed variants of queued work | Job Kind | Current worker documentation uses the exact Job Kind and typed Job Payload. Markdown and active-source checks pass. |
| [x] | effect family | Closed variants of prepared worker output | Prepared Effect | Active source has no PLE-owned effect-family boundary; the existing Effect Committer remains the visibility boundary. Markdown and active-source checks pass. |
| [x] | `DispatchFamily` | Worker polling split between generic and accepted-submission work | Job Kind and Job Kind Registration | Active source has no `DispatchFamily`; one Worker claims registered Job Kinds. Markdown and active-source checks pass. |
| [x] | `wait_for_job_family`, `required_job_family`, or worker family readiness | Readiness check for executable background behavior | Job Kind Registration readiness | Active source has no family-shaped worker readiness identifier; readiness verifies the exact Job Kind Registration. Markdown and active-source checks pass. |
| [ ] | `ple_private.worker_job` | Durable queued background work record | Job | Store the immutable Job Kind, Job Target, typed payload, generation, state, and lease facts. |
| [ ] | `worker_job.handler_kind` | Closed background behavior classification | Job Kind | Name the requested background work independently of the Worker process. |
| [ ] | `worker_job.target_kind` and nullable target columns | Exact bounded subject of a Job | Job Target | Use one closed Job Target variant whose fields make each target shape complete. |
| [ ] | `worker_job.state` with Ready, Leased, Completed, or Dead | Authoritative background-work transition state | Job State | Use Ready, Leased, Completed, or Failed. Return an expired retryable Lease to Ready while attempts remain. |
| [ ] | `worker_job.failure_kind` with Transient, Permanent, or Timed Out | Worker decision about failed Job handling | Job Failure Kind | Use Retryable, Final, or Timed Out and keep it separate from Job State. |
| [ ] | `JobTargetSelector` | Rust closed target variant | Job Target | Keep the exact subject, generation fence, and digest scope together. |

## Checks, recalculation, and repair

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [x] | Object reconciliation | Comparison of database references with stored bytes | Object Storage Check | `object_storage_check` records a verified, missing, or mismatched result. `object_cleanup_manifest` separately authorizes cleanup. Fresh PostgreSQL acceptance passes. |
| [x] | Score reconciliation | Recomputing derived scoring state | Recalculation | `select_assignment_attempt_grade` names batch grade selection during Recalculation; immutable accepted submissions, prior results, and receipts remain distinct. Targeted rustfmt, focused domain tests, and the active-source search pass. |
| [ ] | `ScoringStatus` | Authoritative Current, Recalculating, or Failed scoring axis | Assignment Scoring State | Use State for the authoritative transition value and present a separately derived status message when needed. |
| [ ] | `AssignmentScoringWitness` | Assignment, generation, and scoring state observed by one Gradebook read | Assignment Scoring Snapshot | Return the exact Assignment Reference, Scoring Generation, and Assignment Scoring State so mixed-generation reads are detectable. |
| [ ] | Migration reconciliation | Comparing installed schema with the required migration registry | Migration Check | Compare the exact installed migration set, order, and digest with the required registry. |
| [ ] | `MigrationDisposition`, `MigrationStatus`, or SQLx migration ledger | Read comparison of required and installed migrations | Migration Check Result | Report Applied, Pending, Changed, Incomplete, and unexpected installed migrations without implying that the read performs migration application. |
| [ ] | Projection | A calculated or rearranged read model | Derived View | Name the view by its subject when a more specific term exists, such as Question Catalog or Gradebook. |
| [ ] | Projection reconciliation | Checking or rebuilding a Derived View | Derived View Check or Derived View Repair | Use Check for comparison and Repair for an authorized change. Preserve the source records that own meaning. |
| [ ] | Curriculum adoption reconciliation | Checking and rebuilding records derived from an applied Blueprint decision | Blueprint Adoption Record Repair | Compare Course Origin and Assignment Source records with the exact Blueprint Adoption Receipt, then rebuild the affected Derived Views. |

## Object storage

| Done | Wording to replace | Context | Canonical target | Structural instruction |
| ---- | ------------------ | ------- | ---------------- | ---------------------- |
| [ ] | Object key or `ObjectKey` | Opaque object-store location | Object Address | Let the server create the Object Address and use the owning Object Reference for meaning and authority. |
| [ ] | `ScoredEmbedRenderCacheKey` | Exact private lookup inputs for one external-provider rendered cache entry | External Question Provider Cache Entry | Bind the exact Question Version, provider seed, provider profile, payload digest, and expiry in the cache entry while keeping its storage address server-held. |
| [ ] | `AssetObjectBinding` | Trusted server mapping from one logical Question Asset to immutable stored bytes | Question Asset Reference plus Object Reference | Resolve the exact Object through the owning Question Version before issuance and keep Object Delivery as the later retrieval boundary. |
| [ ] | `RendererProvenance`, `webwork-renderer.provenance`, or renderer provenance | Exact renderer container reference and OCI image identity selected by the server | Question Renderer Release | Record the exact installed renderer release and bind each applicable Question Attempt Source Record to it. |
| [ ] | `Bucket` with PublicAssets, PrivateContent, StudentRecords, or TempProcessing | Physical storage partition selected by PLE policy | Object Storage Area | Name the policy area in PLE and keep bucket naming inside the S3 or MinIO adapter. |
| [ ] | `ObjectCategory` | Generic label duplicating the Object's owning use | Source Object, Asset, Artifact, or Temporary Processing Object | Let the exact owning relationship state the semantic use instead of storing a second broad category. |
| [ ] | `ObjectRecord.license` used for either licensing or educational-record handling | Reuse terms and sensitivity combined in one string | Object License plus Object Data Class | Store optional reuse terms separately from the required data class inherited from meaning and ownership. |
| [ ] | `ObjectRecord.provenance` or `PutObject.provenance` free text | Unstructured explanation of source or generation | Exact source relationship, Manifest, Event, or Receipt | Bind the Object to its Draft Question Revision, Question Version, import, export, render, or Student Record owner and retain structured evidence for the producing operation. |
| [ ] | `ple_data.object_delivery_record` | Authorized retrievable Object mapping | Object Delivery | Bind one exact Object Reference and owning scope to the bounded delivery route. |
| [ ] | `object_delivery_record.delivery_kind` with Catalog Asset, Course Banner, or Course Record | Delivery type mixed with the Object's owning relationship | Question Asset Reference, Course Banner, or exact Object Reference | Let the owning relationship name the delivered content and let Object Delivery own only authorized retrieval. |
| [ ] | `object_delivery_record.publication_state` with Pending, Active, or Retired | Current retrievability of one Object Delivery | Object Delivery State | Use Pending, Available, or Retired and keep Question or Blueprint publication state separate. |
| [x] | `ple_private.object_reconciliation_record` | Stored comparison of an Object Delivery with bytes | Object Storage Check | `ple_private.object_storage_check` stores the expected metadata and one completed check result. Fresh PostgreSQL acceptance passes. |
| [x] | Object Storage Check state containing Pending or Cleaned | Check execution and later cleanup mixed with the check result | Object Storage Check Result plus Object Cleanup Receipt | A worker Job owns pending execution; `check_result` stores Verified, Missing, or Mismatched, and `object_cleanup_receipt` records cleanup separately. Fresh PostgreSQL acceptance passes. |
| [x] | `ple_private.object_cleanup_authorization` | Closed authorized Object cleanup scope | Object Cleanup Manifest | `ple_private.object_cleanup_manifest` binds the exact Object Storage Check, permitted disposition, and Job; `ple_audit.object_cleanup_receipt` records completion. Fresh PostgreSQL acceptance passes. |
| [x] | object reconciliation event | Immutable result of an Object Storage Check | Object Storage Check Event | `ple_audit.object_storage_check_event` records the exact result and digest. Fresh PostgreSQL acceptance passes. |

## Registered and platform vocabulary

Registered protocol and platform terms retain their owner-defined spelling
inside the matching boundary. This includes AWS Security Group, SQL `GROUP BY`,
POSIX process group, regex capture group, ARIA `group` and `radiogroup`, Python
`ExceptionGroup`, argparse mutually exclusive group, CSS `font-family`, and the
scientific term biochemical functional group. External protocol terms such as
WebAuthn and registered media-type fields follow their governing specifications.
PLE-owned concepts surrounding those boundaries use the canonical terminology
contract.

## Final audit and retirement

Begin this audit after every replacement row is checked.

- [ ] Confirm that every replacement table row contains `[x]` and that no row
  was removed to hide unfinished work.
- [ ] Repeat the repository-wide searches for every wording-to-replace value
  and inspect every remaining match in context.
- [ ] Confirm that remaining matches belong only to registered or platform
  vocabulary, immutable historical evidence, or this checklist.
- [ ] Confirm that active source, schemas, APIs, generated contracts, tests,
  durable documentation, and active plans use the canonical targets and
  structures.
- [ ] Confirm that [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) contains
  only canonical vocabulary and remains consistent with Human Guidance and
  Naming Conventions.
- [ ] Run `source source_me.sh && ./all_test.sh` on the final material tree and
  record every required gate result.
- [ ] Record an independent terminology audit under
  `docs/active_plans/audits/` with the searches, inspected exceptions, gate
  results, and conclusion.
- [ ] Record checklist completion and the audit location in
  [CHANGELOG.md](CHANGELOG.md).

After every final-audit item passes, remove this checklist from the active
documentation and remove its workflow references from the Terminology
Contract. Git history, the final audit, and the Changelog retain the completion
evidence.
