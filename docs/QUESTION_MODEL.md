# Question model

The backend-neutral representation every question engine maps into (MOD-QM,
WP-C1). It lives in `crates/question_model` and is the root contract: adapters
translate into it, and everything downstream reads only it.

One shared shape is what lets a WeBWorK problem, a QTI item, an H5P activity,
an iMathAS item, and a first-party algorithmic question flow through the same
attempt loop, gradebook, and export path.

## The rule for what belongs here

A public type belongs in this crate when it is answer-free and may cross a
browser-facing boundary. A type that would let a caller learn a correct
response belongs in `crates/grading`, which runs server-side and sits outside
the WebAssembly dependency closure. This is a security classification, not a
claim that every answer-free model field belongs in every Student payload:
the presentation projection further removes provenance and grading-adjacent
details the renderer does not need.

Applied to answers, the split is:

| Belongs here                                                       | Belongs in `crates/grading`     |
| ------------------------------------------------------------------ | ------------------------------- |
| The Numeric Response Tolerance a number is compared within         | The expected value              |
| The Text Response Match Rule (Exact, Case Insensitive, Normalized) | The accepted text               |
| How many choices may be selected                                   | Which choices are correct       |
| Whether partial credit applies, and the points available           | The per-part weighting of a key |

The left column is answer-free shared-model information. An individual
Student projection may still omit it when it is not needed to render an input;
for example, the current issued `QuestionPresentationResponseFormat` omits Numeric
Response Tolerance and Text Response Match Rule.
Everything in the right column decides correctness and remains server-only.

## Types

### Identity

`WorkspaceId`, `QuestionAssetId`, `CourseId`, and the activity identifiers are distinct
newtypes over `Uuid`. `QuestionId` is a validated stable text identity and
`QuestionRevisionNumber` is a validated positive integer. They cannot substitute for one
another, so passing a draft identifier where published content is expected or
an assignment identifier where a course is required fails to compile.

Fresh server-minted identifiers are UUIDv7: random enough that a Question Library record number
reveals no volume information, time-ordered enough to index well, and never
sequential. The storage and wire contract is the canonical 36-character UUID
shape backed by PostgreSQL's native 16-byte `uuid` value; it does not require a
v7 nibble when reading an existing deterministic/local identity. Minting sits
behind the `generate` feature, which the server enables and the WebAssembly
bridge leaves off, because identifiers are created server-side on the publish
transition.

`IssuedQuestionId` is the one idempotent exception: the server derives it as a
UUIDv5 from the opaque Assignment Attempt, exact Assignment Entry, and the
optional frozen Question Pool Item. A resume therefore resolves the same
Issued Question without selecting again. The value remains a durable server
record identity, not browser authority.

Every Assignment Attempt also retains its exact Released Assignment Revision
Reference. The stable Assignment owns its later revisions; the retained revision
is the immutable definition and delivery policy expanded into that Student's
Issued Questions. A Student cannot begin an Assignment Attempt unless the
stable Assignment is Released and selects that exact revision; Closed and
Archived Assignments also refuse new access.

UUIDs name durable records; they are not credentials, authorization evidence,
or browser-facing choice codes. A submission places its `QuestionAttemptId`
once in the route. The server resolves Student, assignment, version, seed,
backend, and policy from that authenticated attempt instead of asking the
browser to resend their UUIDs.

The draft rule is carried by separate types rather than a flag:
`DraftQuestionRevision` has no Question identity, while `QuestionRevision`
requires both a Question ID and Question Revision Number. There is no separate
boolean to fall out of sync with that boundary.

`QuestionId` is stable across one question lineage. Each publication in that
lineage has a fresh immutable `QuestionRevisionNumber`, and `QuestionRevisionReference` keeps the
exact `(QuestionId, QuestionRevisionNumber)` evidence only in trusted delivery, grading,
replay, audit, assignment pins, and optional non-operative Question Attempt Reproduction Details.
An allowed original-lineage correction may retain the `QuestionId` while
archiving the replaced version. A major objective, task, or Question Type
change is a fork: its creator edits a private draft and publication gives it a
new `QuestionId`, a new version, and exact source ancestry. Every successful
publication enters one installation-wide shared Question Library for approved
Instructors. Private content remains a draft and therefore has no published
identity.

Question Library is the shared authoritative set of Published Questions.
My Questions filters it to Published Questions owned by the current Account,
and My Question Drafts filters private Draft Questions the current Account may
edit. Tags and Question Classifications guide search within the Question Library; they do not
partition questions by subject, author, course, or audience. Every assignment item
resolves a Question ID already present in the Question Library. A draft must
validate and publish before an Instructor can place it in an assignment, so an
assignment cannot contain private question content.

`QuestionSummary` is the current hot Question Search projection. It contains the Question
ID, Question Backend, capabilities, metadata, Current Question Revision Availability, and publication time,
but not prompt, response, private source-locator fields, or the opaque internal
pair. Trusted server work resolves the Question ID and loads the separate
internal `QuestionRevision` payload. Question Details uses that safe
Question-ID projection and presents one selected immutable version within the
stable lineage. Approved
Instructors may inspect this published content even when another course
references it; that access does not expose the other course's assignment
composition or Student records.

The Question Library uses semantic change classes to define compatible evolution.
Transport-size limits protect request handling and do not define compatibility.
The original creator or an authorized lineage steward may publish only an
allowed same-lineage correction or compatible improvement under the existing
Question ID. A grading-semantic correction records an impact and starts a
controlled recalculation operation; it never silently rewrites issued evidence.
Major objective, task, or Question Type changes require a fork and new
identity. Any active Instructor may create a fork draft, but that draft is
private to its creator until validation succeeds. The published fork is global,
records exact Question ID and version ancestry, and preserves the improvement
thread without granting the fork author access to edit the source.

`QuestionChangeProposal` is the lightweight improvement workflow. Any vetted
Instructor submits one patch and rationale against one exact immutable base
version. Publication validation and semantic/grading-impact analysis complete
before submission reaches the lineage owner, who accepts or rejects it. A
stale base requires rebase and resubmission. An accepted `ModerateEdit` creates
a new immutable version in the original `QuestionId` lineage, preserves the
canonical author and compatible CC license, records contributor credit and
proposal ancestry, and leaves all assignment and evidence pins unchanged.
`ModerateEdit` is a compatible same-lineage operation; `FullFork` is the
separate major-change operation that creates a creator-private draft and, after
validation, a new global Question ID; `ForcedQuestionCorrection` is the
separate Sysadmin-only emergency replacement operation. The user-facing action
is **Suggest an improvement**. A GitHub analogy is documentation-only and
does not define the domain or authorization contract.

Existing assignments pin their exact Question ID and `QuestionRevisionReference`.
Future availability changes only through an explicit, revision-checked
assignment update; publication, correction, lifecycle work, and recalculation
never advance an assignment automatically. Star is one vetted-Instructor-visible
endorsement per Question ID; active Instructors may see its count and the
identities of vetted Instructors who starred. Students and anonymous callers
see neither the identity list nor Star state. Watch is a private notification
subscription for versions, forks, improvements, and impact events. Neither
changes Question Library visibility or grants course authority.

The shared Question Library is not a Student delivery path. A Student receives question
content only after the server confirms the authenticated Student's Active Student
Course Membership, exact `CourseId`, `AssignmentId`, Assignment Status, and
current policy. Anonymous requests have no Question Library access authority and cannot use Question IDs to obtain
published content.

`CourseMembershipRole` represents only the student and instructor values that
may be stored on a direct membership. There is no second effective-course-role
enum.

Every `QuestionSearchResult` is already published, with its immutable Question
Publication Event retained separately from its current Question Revision
Availability. The ordinary new-assignment selector accepts only `Available`
versions. `Archived` versions remain discoverable and resolvable for exact
references, evidence, provenance, and retained assignments, with their stated
reason.

Question Statistics evidence is version-specific and excludes previews and the Instructor
Student view. After the configured privacy threshold, the safe aggregate may
expose accepted-attempt count, graded-attempt count, correct count, and
eligible-choice selection counts for supported choice Question Types. Below the
threshold it exposes availability only; it never exposes raw responses,
small-cell counts, linkable cohorts, or Student identities. Course-local
item-analysis metrics remain separately authorized and never become global
Question Statistics.

### ForcedQuestionCorrection

Published versions are immutable. A Sysadmin may approve a closed
`ForcedQuestionCorrection` only for `security_flaw` or
`critical_correctness_flaw`. It immediately activates the authoritative mapping
from the flawed version to the validated replacement, so new selection and
issuance resolve to the replacement. The old version is preserved solely as
immutable historical evidence and is never edited or deleted.

Replacement publication requires validated content and a closed, privacy-safe
impact manifest. The resulting Correction Generation is handed to bounded
idempotent, Correction-Generation-fenced workers for active-binding and remediation
updates across every active Blueprint, CourseInstance, assignment,
selection-pool, and future-issuance reference. A deterministic compatibility
check governs reissue or excuse for in-progress work. Issued or graded evidence
remains pinned to the original immutable version; completed work receives
superseding receipts and deterministic recalculation under the correction.

The operation has no per-course approval step. Instructors receive audited,
course-authorized results, while Sysadmin projections contain no FERPA-bearing
course or Student records. Every approval, validation, manifest, atomic
advance, reissue, excuse, superseding receipt, recalculation, and publication
event is append-only audited.

`Sysadmin` is a Product Role, never a Course Membership Role; it cannot
replace direct Instructor membership for general FERPA access or view the
Question ID Star identity list or any private Watch state. `CourseSummary`
and `AssignmentSummary` are Rust-owned browser projections. Their Question Pool
selection summaries carry Question IDs and safe display metadata,
never an opaque `QuestionRevisionReference` or question payload.

### Capabilities

`Capability` has the specification's eight variants, and `QuestionBackendCapabilities`
is a set of them. The support question has exactly one implementation,
`supports`, and `missing_from` returns every gap rather than the first, because
an instructor fixing an assignment wants the whole list.

The eight: `algorithmicGeneration`, `clientRendering`, `serverGrading`,
`partialCredit`, `hints`, `questionAttemptTimeLimit`, `printExport`,
`offlinePreview`.

An enum rather than eight booleans means a violation can name the capability it
is about, and adding a ninth makes every exhaustive match stop compiling until
it is handled.

`domain::policy::validate_assignment_config` receives selected key-free
Question Revisions, each adapter's declared capabilities, and any
assignment-wide delivery requirements. It returns every missing
question/capability pair in question order and capability declaration order.
The editor calls it through WebAssembly and the publish route calls the same
Rust function.

Question Revisions imply these requirements:

| Question feature                  | Required backend capability         |
| --------------------------------- | ----------------------------------- |
| Seeded randomization              | `algorithmicGeneration`             |
| All-or-nothing grading            | `serverGrading`                     |
| Partial-credit grading            | `serverGrading` and `partialCredit` |
| Immediate correctness with a hint | `hints`                             |
| Per-question timer                | `questionAttemptTimeLimit`          |
| Untimed, ungraded static question | None                                |

Assignment delivery can additionally require `clientRendering`, `printExport`,
or `offlinePreview` from every selected backend. Duplicate requirements produce
one violation. `crates/domain/tests/capability_violation_cases.json` is the
reviewed table covering all eight capabilities and the return-all behavior.

### Question Revision

`QuestionRevision` carries the fields the specification names:

| Field                         | Type                          | Purpose                                                                        |
| ----------------------------- | ----------------------------- | ------------------------------------------------------------------------------ |
| `questionId`                  | `QuestionId`                  | Stable Question lineage                                                        |
| `revisionNumber`               | `QuestionRevisionNumber`       | Exact immutable version within the lineage                                     |
| `workspace`                   | `WorkspaceId`                 | Authoring workspace                                                            |
| `backendLocator`               | `QuestionBackendLocator`                            | Backend-specific location, separate from the stored Question Source            |
| `prompt`                      | `Vec<QuestionContentBlock>`   | Renderable content, in order                                                   |
| `questionType`                | `QuestionType`                | Educational interaction: MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, or HOTSPOT |
| `response`                    | `QuestionResponseFormat`      | Accepted Student Response shape and constraints                                |
| `questionAttemptLimit`        | `QuestionAttemptLimit`        | Retry bound for this Question                                                  |
| `questionAttemptTimeLimit`    | `QuestionAttemptTimeLimit`    | Time limits, with grace                                                        |
| `questionVariationRule`       | `QuestionVariationRule`       | Static or seeded rule for how this Question varies                             |
| `grading`                     | `QuestionGradingRule`         | How a response is judged                                                       |
| `metadata`                    | `QuestionMetadata`            | Title, tags, Question Classifications, Question License, language              |

### Response shapes

`QuestionType` classifies the educational interaction independently of Question
Format, Question Backend, and the browser control. `QuestionResponseFormat` and
`StudentResponse` are parallel enums: numeric, multiple choice or multiple
answer, short text, multi-blank, matching, ordering, hotspot, and external
tool. Within a variant, invalid field combinations are unrepresentable, so a
matching response carries only prompt-to-choice associations and a hotspot
response carries only selected Hotspot Region references.
`ResponseItemReference` is the durable semantic identifier used by this shared
model; it is not a visible letter or display position.

`QuestionResponseControl` names the browser interaction. `ExternalTool` is a
fieldless marker variant in both response enums. It carries no
provider, launch, answer, score, token, or completion material. The server
owns the later provider exchange through its external-tool broker, so the
question envelope and generic submission record remain answer-free.

Agreement _between_ the two and variant-specific format rules live in
`domain::validation::validate_response_format`. The browser calls that pure
function through WebAssembly, and the server repeats it before grading. A
client-side check is a convenience rather than an authority.

Choices, blanks, matching prompts, ordering items, and hotspot regions are
compared by identifier rather than by displayed label or position. Presenting
them in a different order therefore does not change answer meaning. A Hotspot
selection submits an authored Hotspot Region Reference; its rectangle or ellipse
geometry belongs to the Question Response Format rather than the Student Response.

Server-side grading loads the attempt's exact published `QuestionRevision`
and calls `grading::grade(question, response, key)`. The definition supplies
the response comparison and point policy that are intentionally absent from
the compact attempt row; the key remains in the server-only grading boundary.
Grading is deterministic and automated for every supported Question Type.

### Attempt presentation

`QuestionPresentation` is the narrow, issued contract for a Student screen.
It projects the public rendering portion of one `QuestionVariationPresentation`
for a specific attempt and provides a consistency binding for that presentation.

`QuestionPresentation` contains the immutable version, issued seed,
server-minted nonce, title, prompt, and an answer-free
`QuestionPresentationResponseFormat`. It is the issued projection of the durable
Question Response Format: it replaces durable response-item references with
rendered IDs while preserving the exact response shape. The schema currently
covers the eight PLE Question JSON Question Types:

| `QuestionPresentationResponseFormat` | Shared Question Response Format |
| -------------------------------- | ------------------------------- |
| `singleChoice`                   | exactly-one multiple choice     |
| `multipleAnswer`                 | one-or-more multiple choice     |
| `fillIn`                         | short text                      |
| `multiFillIn`                    | multi-blank                     |
| `numerical`                      | numeric                         |
| `matching`                       | matching                        |
| `ordering`                       | ordering                        |
| `hotspot`                        | hotspot                         |

The durable Question Response Format names the item role at the model boundary:
Multiple Choice has Question Choices; MATCH has Matching Prompts and Matching
Choices; Ordering has Ordering Items. Each record combines its Response Item
Reference with the learner-visible content. This keeps similar wire shapes from
becoming interchangeable application meanings.

`ExternalTool` intentionally has no `QuestionPresentationResponseFormat` variant.
The presentation builder rejects it until its server-owned provider route has a
complete delivery contract.

For selectable and addressable objects, the builder projects durable IDs to a
presentation-scoped Response Item Reference (`PresentationResponseItemReference` in the current
machine contract): four lowercase hexadecimal characters produced by
CRC-16/CCITT-FALSE. Its input is domain-separated and includes the
presentation nonce, version, seed, role, ordinal, durable ID, and canonical
public item basis. Choices, blanks, matching sides, ordering items, and each
Hotspot Region are separately addressable. The builder permits at most 32 addressable items, requires
IDs to be unique across the complete presentation, and retries with a fresh
nonce up to eight times if a CRC16 collision occurs. The server retains the
ability to rebuild the rendered-to-durable mapping from the exact definition
and persisted presentation binding; the four-character value is neither a
durable identity nor a security credential.

The canonical binary descriptor covers the Question Presentation, rendered-item
bases, and Question Asset Renditions. PLE stores its full Question Presentation Checksum with the attempt and gives
the Student only a 128-bit `pd1_` base64url prefix in
`StudentAttemptDescriptor`. The browser can rebuild and check the public
descriptor through Wasm; the server checks the full digest when reproducing
the attempt. The digest and rendered IDs detect a stale or incoherent render,
but do not authenticate a Student, authorize a request, or determine whether
an answer is correct.

This is the accepted current presentation contract, not a statement that the live
generic run route has already completed its payload cutover. The current route
still issues `QuestionVariationPresentation` and accepts a tagged `StudentResponse` in
`{ "response": ... }`; its `kind` is therefore part of today's wire shape.
The planned compact response uses the attempt route identity, presentation
digest, and rendered IDs, then resolves the Question Type and durable IDs
server-side. [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)
own that transition and its acceptance gates.

### Content blocks

`QuestionContentBlock` is a closed set: text, math, image, code, table. Closed so the
renderer's match is exhaustive and adding a kind points the compiler at the
renderer.

Every variant carrying visual content also carries a required description.
Required rather than optional: a figure with no description is unusable with a
screen reader, and MOD-UI-RENDER surfaces a missing description as an authoring
error.

### Policies

Question-level rules are authored with the Question: `QuestionAttemptLimit`
(a retry bound) and `QuestionAttemptTimeLimit` (unlimited or limited for one Question Attempt,
each with a grace period for network delay). Student Feedback Release is not a
question policy: the assignment owns its five-field `StudentFeedbackReleaseRule`
and the server evaluates it for current Student projections.

A Question Hint is requested before a Student selects or submits a response.
Question Feedback is selected only after automatic grading and remains three
separate authoring levels: selected-choice feedback, correct-outcome feedback,
and incorrect-outcome feedback. The assignment release rule controls whether
the applicable post-grade feedback reaches the Student; it does not govern a
Question Hint.
For timed work, `QuestionAttemptTiming.deadline` is the server-issued base
deadline. MOD-TIME applies any authorized, audited pause extension before it
evaluates the inclusive grace boundary.

The explicit Assignment activity rules are chosen per Assignment and are independent enums.
The later-Attempt rules split Question Pool membership from Question Variation:
`QuestionPoolReuseRule` chooses Reuse Selection or Select Again, while
`QuestionVariationRule` chooses Reuse Variation or New Variation. This lets an
instructor express mastery-required practice that keeps the selected Questions
while using new Question Variations, or any other meaningful combination,
without a combined mode hiding either decision.

Each top-level Fixed Question or Question Pool has Assignment Entry Availability:
Available includes it in future Assignment Attempts and Retired preserves historical
Issued Questions without future delivery. Each Question Pool Item has its own
Question Pool Item Availability, separate from the owning pool's availability.
Each top-level entry also carries its Assignment Entry Scoring Rule: Normal,
Full Credit, Extra Credit, or Excluded. An Issued Question freezes the rule and
point value from its source entry.
Question Pool Selection Rule combines the reviewed selection algorithm
and output ordering for one pool. Question Variation Rule separately controls
whether later Assignment Attempts reuse or redraw pool selections.

Assignment editing is a direct, closed Assignment contract.
`AssignmentAuthoredContent` carries validated plain-text
`AssignmentInstructions` and the absolute `BaseAssignmentPolicy` for the
editable Assignment. `AssignmentStatus` belongs to that Assignment and selects
a Released Assignment Revision only after Assignment Release.
`AssignmentTitle` is the separate validated short name for the Assignment or
released revision, rather than generic text at a shared contract boundary.
Every Questions, Policies, and fixed-question replacement request carries its
reviewed `AssignmentEditNumber` as `baseEditNumber`; the HTTP strong ETag is
the transport concurrency condition for that exact Assignment.
New Assignments default to Unreleased and therefore are not Student-visible
until an Instructor explicitly releases one immutable Assignment Revision. The
Instructor transport uses `InstructorAssignmentAuthoredContentLocal`:
local timestamps include
milliseconds and the exact course IANA zone, but the server performs every
DST, term, ordering, and integer-bound conversion before storage.
`InstructorAssignmentAvailabilityView` is a separate closed server projection
for Unreleased, Scheduled, Available, Closed, or Archived at one authoritative
instant. Its scheduled and clock-closed variants carry only the matching
course-local boundary, so a browser displays Assignment availability without
inferring it.

Students receive `StudentAssignmentDetail`, not the Instructor aggregate. Its
delivery values are already resolved from exact assignment entitlement and omit
Assignment Status, base-policy provenance, course identifiers, and evaluation
clocks. `AssignmentScoringState`
is also independent: Current allows the otherwise authorized score projection;
Recalculating and Failed retain the semantic score state while omitting every
numeric Student score, Grading Result, and disclosed point value.

### Generation

`Question Seed` plus `QuestionVariationRule` fully determine a Question
Variation. A seeded
definition pins a `QuestionGeneratorReference` containing both the generator ID and its
additive version. Changing generator behavior therefore creates a new generator
version instead of changing an existing published question underneath its
assignments and historical attempts.

Parameters are declared as `QuestionGeneratorParameter` values rather than computed inline,
so a preview can show an instructor the space a question draws from and the
seed-vector corpus can cover every branch.

`BTreeMap` holds parameters because iteration order reaches generated output,
and byte-identical output on server and browser is what the WP-C5 parity gate
requires.

The exact sampling and parity rules are documented in
`docs/DETERMINISM_CONTRACT.md`.

### Classification and licensing

Question Tags are free-form search labels. A Question Classification maps one
Question Revision to a real external or institutional system through its
Classification System, Classification Code, and Classification Name. Question
Bloom Classification is PLE's dedicated two-dimensional classification and
therefore has its own closed fields instead of using that generic mapping.

Question License is the exact versioned SPDX expression governing one Question
Revision. Publication accepts a license compatible with Question Library
sharing and full forks. See
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md#question-content-and-stewardship)
for the complete Question Metadata vocabulary.

#### Bloom classification

Once automatic AI classification completes, every Published Question Revision
has one current Question Bloom Classification derived from exactly two independently selected
closed fields:

- Bloom Cognitive Process: Remember, Understand, Apply, Analyze, Evaluate, or
  Create.
- Bloom Knowledge Dimension: Factual Knowledge, Conceptual Knowledge,
  Procedural Knowledge, or Metacognitive Knowledge.

The Instructor interface labels the fields Cognitive Process Dimension and
Knowledge Dimension. Their ordered pair alone determines the combined label
and matrix position. Question Search exposes each field as an independent facet
and may show their derived 4 by 6 intersection.

Classify the performance required for full credit on the exact Question
Revision:

1. Read the complete Question Prompt, Question Response Format, Answer Key,
   scoring criteria, and any rubric that determines full credit.
2. Identify the primary thing the Student must know and select its Knowledge
   Dimension.
3. Identify the primary cognitive work the Student must perform with that
   knowledge and select its Cognitive Process Dimension.
4. Check the pair against the actual grading requirement. The complete task
   determines the pair; a command verb or Question Type alone does not.
5. For a Question with several tasks, use the best-supported pair representing
   the dominant full-credit performance. An Instructor can correct the assigned
   pair later.

Use these short category meanings:

| Cognitive process | Student performance required for full credit              |
| ----------------- | --------------------------------------------------------- |
| Remember          | Retrieve relevant knowledge                               |
| Understand        | Construct meaning from presented or recalled knowledge    |
| Apply             | Use a procedure in a situation                            |
| Analyze           | Separate material into parts and relate those parts       |
| Evaluate          | Make a judgment using stated or appropriate criteria      |
| Create            | Assemble elements into a coherent or functional new whole |

| Knowledge dimension     | Primary knowledge used by the Question                |
| ----------------------- | ----------------------------------------------------- |
| Factual Knowledge       | Terminology, specific details, and elements           |
| Conceptual Knowledge    | Categories, principles, theories, models, and systems |
| Procedural Knowledge    | Skills, algorithms, techniques, methods, and use      |
| Metacognitive Knowledge | Strategies and awareness of one's own cognition       |

Prior learning and course context can change the cognitive work a task demands.
Publishing creates the exact immutable Question Revision with Bloom classification
unassigned. AI classification work searches for unassigned Published Question
Revisions and supplies each initial two-enum pair. The Question remains Published
and discoverable while unassigned. An Instructor may later edit either value for
that exact Question Revision. This classification edit changes discovery metadata
without changing Question content or creating a Question Revision.

The later AI integration plan owns model execution, scheduling, the answer-bearing
input boundary, concurrent work claims, retry behavior, and operational evidence.
This Question Model defines the classification result and its timing without selecting
those implementation details.

The reference two-dimensional graphic uses one hue family for each Cognitive
Process column. PLE retains these sampled associations:

| Cognitive process | Reference hue | Reference anchor |
| ----------------- | ------------- | ---------------- |
| Remember          | Blue          | `#64A4D9`        |
| Understand        | Green         | `#A2D4B4`        |
| Apply             | Yellow-green  | `#B9D438`        |
| Analyze           | Yellow        | `#E7E028`        |
| Evaluate          | Orange        | `#E8A264`        |
| Create            | Pink          | `#E3759F`        |

Interface owners derive accessible surface, border, text, focus, selected, and
dark-mode tokens from these anchors. Every control and matrix cell shows its
text labels alongside color. The Knowledge Dimension remains the labeled
second axis.

The local reference image identifies itself as Rex Heer's Iowa State
University graphic under CC BY-NC-SA 3.0, while PLE's distributable non-code
work permits commercial reuse. PLE keeps the image as an external design
reference and distributes its own accessible components. The Anderson and
Krathwohl 2001 revision owns the dimensions and category terminology. Marzano's
New Taxonomy remains a separate learning-goal framework.

## Wire format

**Current pre-WN1 behavior:** serialization is JSON with camelCase field names. WN1-A assigns each
public serializable type to one atomic `WN1-QM` closure after C routes project route-only values to
`browser-api-contract`. The approved target uses direct `snake_case` Rust and TypeScript data-object
properties. Enums carrying data are internally tagged, so a client can switch on one discriminant:

```json
{ "kind": "per_attempt", "seconds": 1800, "grace_seconds": 30 }
```

Unit-only enums serialize as plain strings:

```json
"case_insensitive"
```

Two serde rules are in play and are easy to confuse. On an enum, `rename_all`
renames the _variants_, while `rename_all_fields` renames the fields _inside_
variants. Both move with their complete type closure so fields and portable values become snake_case
together; WN1-B does not change their effective Serde spelling.

### PLE Question JSON authoring source

[PLE Question JSON](QTI-JSON_OBJECT_FORMAT.md) is a narrow answer-bearing
authoring format for ordinary static questions. It is not another public
question model. The PLE Question Backend compiles it into this crate's answer-free
`DraftQuestionRevision` plus separate grader-only material. Published browser
and Question Library browser projections therefore continue to use the shared question model
regardless of whether the author wrote PLE JSON or imported a supported QTI
profile.

The former PLE Question JSON schema version 1 `singleChoice` reader and source contract are
retired and unsupported. There is no v1 compatibility reader, source-byte
fallback, or compatibility behavior. Version 2 is the only current PLE
source shape: a closed contract with eight Question Types, `singleChoice`,
`multipleAnswer`, `fillIn`, `multiFillIn`, `numeric`, `matching`, `ordering`,
and `hotspot`. V2 input is answer-bearing private authoring material, not a
Student payload. It does not claim file-upload or external-tool authoring
support. The compiler emits an answer-free draft/public model and separately
checksummed grader-only Answer Key and Question Feedback.

The distinction matters when evolving either contract: the source format owns
author ergonomics, durable Question Choice References, Question Answers, and private Question Feedback; this crate
owns the engine-neutral public runtime shape. Neither layer grows a vendor QTI
extension container.

## Generated TypeScript

The project tools (`crates/project-tools`) read this crate's source and write TypeScript into the ignored
root `generated/api/` directory, one file per type. Regenerate with `./build.sh`,
or `cargo tsgen` while iterating.

The rule for what gets exported is the boundary rule stated above: every public
struct or enum that derives `Serialize` or `Deserialize`. A type that must stay
server-side stays out of the client bundle by not being serializable here.

Mapping:

| Rust                     | TypeScript                     |
| ------------------------ | ------------------------------ |
| unit-only enum           | string union                   |
| tagged enum with data    | discriminated union on the tag |
| struct with named fields | object type                    |
| newtype struct           | alias to the inner type        |
| `Option<T>`              | `T \| null`                    |
| `Vec<T>`, `BTreeSet<T>`  | `Array<T>`                     |
| `BTreeMap<K, V>`         | `Record<K, V>`                 |
| `Uuid`, `String`         | `string`                       |
| integer and float types  | `number`                       |

The generated files pass `tsc --noEmit`, ESLint, and `prettier --check`
unchanged, which is why the generator emits Prettier-shaped output rather than
relying on a reformatting pass.

An enum carrying data is required to declare `#[serde(tag = "...")]`. The
generator refuses an untagged one, because serde's externally tagged form
produces TypeScript a client cannot switch on cleanly.

## Related documents

- [active_plans/implementation_plan.md](active_plans/implementation_plan.md):
  the milestone plan and the module catalog.
- [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md): crate boundaries and the two
  guarantees the structure enforces.
- [RUST_STYLE.md](RUST_STYLE.md): section 9 on encoding invalid states out of
  existence, which is the rule the capability and policy types follow.
