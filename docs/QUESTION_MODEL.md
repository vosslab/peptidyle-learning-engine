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

| Belongs here                                               | Belongs in `crates/grading`     |
| ---------------------------------------------------------- | ------------------------------- |
| The tolerance a number is compared within                  | The expected value              |
| How text is compared (exact, case-insensitive, normalized) | The accepted text               |
| How many choices may be selected                           | Which choices are correct       |
| Whether partial credit applies, and the points available   | The per-part weighting of a key |

The left column is answer-free shared-model information. An individual
Student projection may still omit it when it is not needed to render an input;
for example, the current issued `IssuedQuestionResponseFormatV1` omits numeric tolerance and text match mode.
Everything in the right column decides correctness and remains server-only.

## Types

### Identity

`WorkspaceId`, `AssetId`, `CourseId`, and the activity identifiers are distinct
newtypes over `Uuid`. `QuestionId` is a validated stable text identity and
`QuestionVersionNumber` is a validated positive integer. They cannot substitute for one
another, so passing a draft identifier where published content is expected or
an assignment identifier where a course is required fails to compile.

Fresh server-minted identifiers are UUIDv7: random enough that a catalog number
reveals no volume information, time-ordered enough to index well, and never
sequential. The storage and wire contract is the canonical 36-character UUID
shape backed by PostgreSQL's native 16-byte `uuid` value; it does not require a
v7 nibble when reading an existing deterministic/local identity. Minting sits
behind the `generate` feature, which the server enables and the WebAssembly
bridge leaves off, because identifiers are created server-side on the publish
transition.

`IssuedQuestionId` is the one idempotent exception: the server derives it as a
UUIDv5 from an opaque Assignment Attempt identity and the frozen selection. A
resume therefore resolves the same Issued Question without selecting again.
The value remains a durable server record identity, not browser authority.

UUIDs name durable records; they are not credentials, authorization evidence,
or browser-facing choice codes. A submission places its `QuestionAttemptId`
once in the route. The server resolves Student, assignment, version, seed,
backend, and policy from that authenticated attempt instead of asking the
browser to resend their UUIDs.

The draft rule is carried by separate types rather than a flag:
`DraftQuestionDefinition` has no Question identity, while `QuestionDefinition`
requires both a Question ID and Question Version Number. There is no separate
boolean to fall out of sync with that boundary.

`QuestionId` is stable across one question lineage. Each publication in that
lineage has a fresh immutable `QuestionVersionNumber`, and `QuestionVersionReference` keeps the
exact `(QuestionId, QuestionVersionNumber)` evidence only in trusted delivery, grading,
replay, audit, assignment pins, and optional non-operative provenance records.
An allowed original-lineage correction may retain the `QuestionId` while
archiving the replaced version. A major objective, task, or response-family
change is a fork: its creator edits a private draft and publication gives it a
new `QuestionId`, a new version, and exact source ancestry. Every successful
publication enters one installation-wide shared catalog for approved
Instructors. Private content remains a draft and therefore has no published
identity.

Tags and taxonomy guide search within the shared catalog; they do not partition
questions by subject, author, course, or audience. Every assignment item
resolves a Question ID already present in this published corpus. A draft must
validate and publish before an Instructor can place it in an assignment, so an
assignment cannot contain private question content.

`CatalogProblemSummary` is the hot browse projection. It contains the Question
ID, backend family, capabilities, metadata, lifecycle, and publication time,
but not prompt, response, private source-locator fields, or the opaque internal
pair. Trusted server work resolves the Question ID and loads the separate
internal `QuestionDefinition` payload. Browser catalog detail uses that safe
Question-ID projection and presents one selected immutable version within the
stable lineage. Approved
Instructors may inspect this published content even when another course
references it; that access does not expose the other course's assignment
composition or Student records.

The catalog uses semantic change classes to define compatible evolution.
Transport-size limits protect request handling and do not define compatibility.
The original creator or an authorized lineage steward may publish only an
allowed same-lineage correction or compatible improvement under the existing
Question ID. A grading-semantic correction records an impact and starts a
controlled recalculation operation; it never silently rewrites issued evidence.
Major objective, task, or Question Type changes require a fork and new
identity. Any approved Instructor may create a fork draft, but that draft is
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

Existing assignments pin their exact Question ID and `QuestionVersionReference`.
Future availability changes only through an explicit, revision-checked
assignment update; publication, correction, lifecycle work, and recalculation
never advance an assignment automatically. Star is one vetted-Instructor-visible
endorsement per Question ID; approved Instructors may see its count and the
identities of vetted Instructors who starred. Students and anonymous callers
see neither the identity list nor star state. Watch is a private notification
subscription for versions, forks, improvements, and impact events. Neither
changes catalog visibility or grants course authority.

The shared catalog is not a Student delivery path. A Student receives question
content only after the server grants an exact assignment entitlement for the
authenticated Student, active Student membership, `CourseId`, `AssignmentId`,
assignment audience, assignment lifecycle, and current policy. Anonymous
requests have no catalog authority and cannot use Question IDs to obtain
published content.

`CourseMembershipRole` represents only the student and instructor values that
may be stored on a direct membership. There is no second effective-course-role
enum.

Every Question Catalog Entry is already published, with its immutable Question
Publication Event retained separately from its current Question Version
Availability. The ordinary new-assignment selector accepts only `Available`
versions. `Archived` versions remain discoverable and resolvable for exact
references, evidence, provenance, and retained assignments, with their stated
reason.

Catalog evidence is version-specific and excludes previews and the Instructor
Student view. After the configured privacy threshold, the safe aggregate may
expose accepted-attempt count, graded-attempt count, correct count, and
eligible-choice selection counts for supported choice families. Below the
threshold it exposes availability only; it never exposes raw responses,
small-cell counts, linkable cohorts, or Student identities. Course-local
item-analysis metrics remain separately authorized and never become global
catalog evidence.

### ForcedQuestionCorrection

Published versions are immutable. A Sysadmin may approve a closed
`ForcedQuestionCorrection` only for `security_flaw` or
`critical_correctness_flaw`. It immediately activates the authoritative mapping
from the flawed version to the validated replacement, so new selection and
issuance resolve to the replacement. The old version is preserved solely as
immutable historical evidence and is never edited or deleted.

Replacement publication requires validated content and a closed, privacy-safe
impact manifest. The resulting correction generation is handed to bounded
idempotent, generation-fenced workers for active-binding and remediation
materialization across every active Blueprint, CourseInstance, assignment,
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
Question ID star identity list or any private watch state. `CourseSummary`
and `AssignmentSummary` are Rust-owned browser projections. Their item and
selection-candidate summaries carry Question IDs and safe display metadata,
never an opaque `QuestionVersionReference` or question payload.

### Capabilities

`Capability` has the specification's eight variants, and `BackendCapabilities`
is a set of them. The support question has exactly one implementation,
`supports`, and `missing_from` returns every gap rather than the first, because
an instructor fixing an assignment wants the whole list.

The eight: `algorithmicGeneration`, `clientRendering`, `serverGrading`,
`partialCredit`, `hints`, `perQuestionTiming`, `printExport`,
`offlinePreview`.

An enum rather than eight booleans means a violation can name the capability it
is about, and adding a ninth makes every exhaustive match stop compiling until
it is handled.

`domain::policy::validate_assignment_config` receives selected key-free
question definitions, each adapter's declared capabilities, and any
assignment-wide delivery requirements. It returns every missing
question/capability pair in question order and capability declaration order.
The editor calls it through WebAssembly and the publish route calls the same
Rust function.

Question definitions imply these requirements:

| Question feature                  | Required backend capability         |
| --------------------------------- | ----------------------------------- |
| Seeded randomization              | `algorithmicGeneration`             |
| All-or-nothing grading            | `serverGrading`                     |
| Partial-credit grading            | `serverGrading` and `partialCredit` |
| Immediate correctness with a hint | `hints`                             |
| Per-question timer                | `perQuestionTiming`                 |
| Untimed, ungraded static question | None                                |

Assignment delivery can additionally require `clientRendering`, `printExport`,
or `offlinePreview` from every selected backend. Duplicate requirements produce
one violation. `crates/domain/tests/capability_violation_cases.json` is the
reviewed table covering all eight capabilities and the return-all behavior.

### Question definition

`QuestionDefinition` carries the fields the specification names:

| Field           | Type                      | Purpose                                   |
| --------------- | ------------------------- | ----------------------------------------- |
| `questionId`    | `QuestionId`              | Stable Question lineage                   |
| `versionNumber` | `QuestionVersionNumber`   | Exact immutable version within the lineage |
| `workspace`     | `WorkspaceId`             | Authoring workspace                       |
| `source`        | `QuestionSource`          | Which engine, and where to find it there  |
| `prompt`        | `Vec<ContentBlock>`       | Renderable content, in order              |
| `questionType`  | `QuestionType`            | Educational interaction: MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, or HOTSPOT |
| `response`      | `QuestionResponseFormat`  | Accepted Student Response shape and constraints |
| `attemptPolicy` | `AttemptPolicy`           | Retry bound for this question             |
| `timingPolicy`  | `TimingPolicy`            | Time limits, with grace                   |
| `randomization` | `RandomizationDefinition` | How content varies                        |
| `grading`       | `GradingDefinition`       | How a response is judged                  |
| `metadata`      | `QuestionMetadata`        | Title, tags, taxonomy, license, language  |

### Response shapes

`QuestionType` classifies the educational interaction independently of Question
Format, Question Backend, and the browser control. `QuestionResponseFormat` and
`StudentResponse` are parallel enums: numeric,
multiple choice or multiple answer, short text, multi-blank, matching,
ordering, hotspot, file upload, and external tool. Within a variant, invalid
field combinations are unrepresentable, so a matching response carries only
prompt-to-choice associations and a hotspot response carries only normalized
points. `ChoiceId` is the durable semantic identifier used by this shared
model; it is not a visible letter or display position.

`QuestionResponseControl` names the browser interaction. File Upload and
External Tool are controls rather than Question Types. `ExternalTool` is a
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
them in a different order therefore does not change answer meaning. Hotspot
coordinates use the integer range 0 through 10,000 so zoom, image density, and
responsive layout do not alter the submitted location.

Server-side grading loads the attempt's exact published `QuestionDefinition`
and calls `grading::grade(question, response, key)`. The definition supplies
the response comparison and point policy that are intentionally absent from
the compact attempt row; the key remains in the server-only grading boundary.
Grading is deterministic and automated for every supported Question Type.

### Attempt presentation

`presentation` is a second, narrower contract for an issued Student screen.
It does not replace `QuestionDefinition`, `QuestionEnvelope`, or
`StudentResponse`; it projects their public rendering portion for a specific
attempt and provides a consistency binding for that presentation.

`PresentationEnvelopeV1` contains the immutable version, issued seed,
server-minted nonce, title, prompt, and an answer-free
`IssuedQuestionResponseFormatV1`. It is the issued projection of the durable
Question Response Format: it replaces durable response-item references with
rendered IDs while preserving the exact response shape. The schema currently
covers the eight native flat Question Types:

| `IssuedQuestionResponseFormatV1` | Shared response definition  |
| -------------------------------- | --------------------------- |
| `singleChoice`     | exactly-one multiple choice |
| `multipleAnswer`   | one-or-more multiple choice |
| `fillIn`           | short text                  |
| `multiFillIn`      | multi-blank                 |
| `numerical`        | numeric                     |
| `matching`         | matching                    |
| `ordering`         | ordering                    |
| `hotspot`          | hotspot                     |

`FileUpload` and `ExternalTool` intentionally have no `IssuedQuestionResponseFormatV1`
variant. The presentation builder rejects them as unsupported rather than
inventing a browser contract before the server-issued upload capability and
external-tool route have their own complete delivery contracts.

For selectable and addressable objects, the builder projects durable IDs to
`RenderedItemIdV1`: four lowercase hexadecimal characters produced by
CRC-16/CCITT-FALSE. Its input is domain-separated and includes the
presentation nonce, version, seed, role, ordinal, durable ID, and canonical
public item basis. The builder permits at most 32 addressable items, requires
IDs to be unique across the complete presentation, and retries with a fresh
nonce up to eight times if a CRC16 collision occurs. The server retains the
ability to rebuild the rendered-to-durable mapping from the exact definition
and persisted presentation binding; the four-character value is neither a
durable identity nor a security credential.

The canonical binary descriptor covers the envelope, rendered-item bases, and
asset bindings. PLE stores its full SHA-256 digest with the attempt and gives
the Student only a 128-bit `pd1_` base64url prefix in
`StudentAttemptDescriptorV1`. The browser can rebuild and check the public
descriptor through Wasm; the server checks the full digest when reproducing
the attempt. The digest and rendered IDs detect a stale or incoherent render,
but do not authenticate a Student, authorize a request, or determine whether
an answer is correct.

This is an accepted v1 presentation contract, not a statement that the live
generic run route has already completed its payload cutover. The current route
still issues `QuestionEnvelope` and accepts a tagged `StudentResponse` in
`{ "response": ... }`; its `kind` is therefore part of today's wire shape.
The planned compact response uses the attempt route identity, presentation
digest, and rendered IDs, then resolves the response family and durable IDs
server-side. [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) and
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)
own that transition and its acceptance gates.

### Content blocks

`ContentBlock` is a closed set: text, math, image, code, table. Closed so the
renderer's match is exhaustive and adding a kind points the compiler at the
renderer.

Every variant carrying visual content also carries a required description.
Required rather than optional: a figure with no description is unusable with a
screen reader, and MOD-UI-RENDER surfaces a missing description as an authoring
error.

### Policies

Question-level policies are authored with the question: `AttemptPolicy`
(a retry bound) and `TimingPolicy` (untimed, per question, or per attempt,
each with a grace period for network delay). Student disclosure is not a
question policy: the assignment owns its five-field `StudentDisclosurePolicy`
and the server evaluates it for current Student projections.
For timed work, `QuestionAttemptTiming.deadline` is the server-issued base
deadline. MOD-TIME applies any authorized, audited pause extension before it
evaluates the inclusive grace boundary.

The four explicit Assignment activity rules are chosen per Assignment and are independent enums:
`CompletionRequirement`, `GradePolicy`, `ContinuedPractice`, and
`VariationPolicy`. They stay independent so an instructor can express "mastery
required, highest score kept, practice allowed after completion with fresh
seeds", which is the behavior students were observed using. A single combined
mode enum would offer a fixed menu instead.

Assignment teaching operations form a separate closed contract.
`AssignmentTeachingSettings` stores one `AssignmentLifecycle`, validated
plain-text `AssignmentInstructions`, and the absolute `BaseAssignmentPolicy`.
New assignments default to Draft and therefore are not Student-visible until an
instructor explicitly publishes them. The instructor transport uses
`InstructorAssignmentTeachingSettingsLocal`: local timestamps include
milliseconds and the exact course IANA zone, but the server performs every
DST, term, ordering, and integer-bound conversion before storage.
`InstructorAssignmentCurrentState` is a separate closed server projection for
Draft, scheduled, open, closed, or archived at one authoritative instant. Its
scheduled and clock-closed variants carry only the matching course-local
boundary, so a browser displays current state without inferring it.

Students receive `StudentAssignmentDetail`, not the Instructor aggregate. Its
delivery values are already resolved from exact assignment entitlement and omit
lifecycle intent, base-policy provenance, course identifiers, and evaluation
clocks. `ScoringStatus`
is also independent: Current allows the otherwise authorized score projection;
Recalculating and Failed retain the semantic score state while omitting every
numeric Student score, attempt result, and disclosed point value.

### Generation

`Seed` plus `RandomizationDefinition` fully determine a variant. A seeded
definition pins a `GeneratorReference` containing both the generator ID and its
additive version. Changing generator behavior therefore creates a new generator
version instead of changing an existing published question underneath its
assignments and historical attempts.

Parameters are declared as `ParameterSpec` values rather than computed inline,
so a preview can show an instructor the space a question draws from and the
seed-vector corpus can cover every branch.

`BTreeMap` holds parameters because iteration order reaches generated output,
and byte-identical output on server and browser is what the WP-C5 parity gate
requires.

The exact sampling and parity rules are documented in
`docs/DETERMINISM_CONTRACT.md`.

### Taxonomy and licensing

`Tag` for free-form search labels, `TaxonomyTerm` for controlled vocabularies
that survive export, and `License` as an enum so an export can decide in code
whether redistribution is permitted. `License::Other` carries an SPDX
identifier, which keeps unusual terms representable as themselves.

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

### Flat-question authoring source

[PLE flat-question JSON](QTI-JSON_OBJECT_FORMAT.md) is a narrow answer-bearing
authoring format for ordinary static questions. It is not another public
question model. The native adapter compiles it into this crate's answer-free
`DraftQuestionDefinition` plus separate grader-only material. Published browser
and catalog projections therefore continue to use the shared question model
regardless of whether the author wrote PLE JSON or imported a supported QTI
profile.

The former flat-question v1 `singleChoice` reader and source contract are
retired and unsupported. There is no v1 compatibility reader, source-byte
fallback, or compatibility behavior. Version 2 is the only current native
source shape: a closed contract with eight families, `singleChoice`,
`multipleAnswer`, `fillIn`, `multiFillIn`, `numeric`, `matching`, `ordering`,
and `hotspot`. V2 input is answer-bearing private authoring material, not a
Student payload. It does not claim file-upload or external-tool authoring
support. The compiler emits an answer-free draft/public model and separately
checksummed grader-only key and feedback material.

The distinction matters when evolving either contract: the source format owns
author ergonomics, stable choice IDs, answers, and private feedback; this crate
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
