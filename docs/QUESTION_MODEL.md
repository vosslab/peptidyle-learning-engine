# Question model

The backend-neutral representation every question engine maps into (MOD-QM,
WP-C1). It lives in `crates/question_model` and is the root contract: adapters
translate into it, and everything downstream reads only it.

One shared shape is what lets a WeBWorK problem, a QTI item, an H5P activity,
and a first-party algorithmic question flow through the same attempt loop,
gradebook, and export path.

## The rule for what belongs here

A type belongs in this crate when a browser may safely see it. A type that
would let a caller learn a correct response belongs in `crates/grading`, which
runs server-side and sits outside the WebAssembly dependency closure.

Applied to answers, the split is:

| Belongs here                                               | Belongs in `crates/grading`     |
| ---------------------------------------------------------- | ------------------------------- |
| The tolerance a number is compared within                  | The expected value              |
| How text is compared (exact, case-insensitive, normalized) | The accepted text               |
| How many choices may be selected                           | Which choices are correct       |
| Whether partial credit applies, and the points available   | The per-part weighting of a key |

Everything in the left column is shown to students anyway. Everything in the
right column decides correctness.

## Types

### Identity

`WorkspaceId`, `ProblemId`, `VersionId`, `AssetId`, `CourseId`, and the activity
identifiers are distinct newtypes over `Uuid`. They cannot substitute for one
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

UUIDs name durable records; they are not credentials, authorization evidence,
or browser-facing choice codes. A submission places its `QuestionAttemptId`
once in the route. The server resolves learner, assignment, version, seed,
backend, and policy from that authenticated attempt instead of asking the
browser to resend their UUIDs.

The draft rule is carried by the type rather than a flag:
`QuestionDefinition::problem` is `Option<ProblemId>`, and `is_draft()` reads
that option. There is no separate boolean to fall out of sync with it.

`ProblemVersionRef` carries the exact `(ProblemId, VersionId)` pair used by
assignments and lineage. `PublicationScope` distinguishes institution-visible
and public immutable versions; private content remains a draft and therefore
has no publication-scope variant or `ProblemId`.

`CatalogProblemSummary` is the hot browse projection. It contains identity,
backend family, capabilities, metadata, scope, lifecycle, authors, lineage,
and publication time, but not prompt, response, or private source-locator
fields. Exact version lookup loads the separate `QuestionDefinition` payload.

`CourseMembershipRole` represents only the student and instructor values that
may be stored on a direct membership. `CourseRole` adds the effective
administrator value returned when tenant-wide authority is applied; using two
types makes an administrator membership unrepresentable. `CourseSummary` and
`AssignmentSummary` are Rust-owned browser projections. Assignment summaries
carry an ordered list of exact `ProblemVersionRef` values and never embed a
question payload.

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

| Field           | Type                      | Purpose                                  |
| --------------- | ------------------------- | ---------------------------------------- |
| `version`       | `VersionId`               | This immutable version                   |
| `problem`       | `Option<ProblemId>`       | Present once published                   |
| `workspace`     | `WorkspaceId`             | Authoring workspace                      |
| `source`        | `QuestionSource`          | Which engine, and where to find it there |
| `prompt`        | `Vec<ContentBlock>`       | Renderable content, in order             |
| `response`      | `ResponseDefinition`      | Expected response shape                  |
| `attemptPolicy` | `AttemptPolicy`           | Attempts allowed, feedback disclosure    |
| `timingPolicy`  | `TimingPolicy`            | Time limits, with grace                  |
| `randomization` | `RandomizationDefinition` | How content varies                       |
| `grading`       | `GradingDefinition`       | How a response is judged                 |
| `metadata`      | `QuestionMetadata`        | Title, tags, taxonomy, license, language |

### Response shapes

`ResponseDefinition` and `StudentResponse` are parallel enums: numeric,
multiple choice or multiple answer, short text, multi-blank, matching,
ordering, hotspot, file upload, and external tool. Within a variant, invalid
field combinations are unrepresentable, so a matching response carries only
prompt-to-choice associations and a hotspot response carries only normalized
points.

`ExternalTool` is a fieldless marker variant in both enums. It carries no
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
(attempts allowed, when feedback appears) and `TimingPolicy` (untimed, per
question, or per attempt, each with a grace period for network delay).
For timed work, `AttemptTimerRecord.deadline` is the server-issued base
deadline. MOD-TIME applies any authorized, audited pause extension before it
evaluates the inclusive grace boundary.

The four run policies are chosen per assignment and are independent enums:
`CompletionRequirement`, `GradePolicy`, `ContinuedPractice`, and
`VariationPolicy`. They stay independent so an instructor can express "mastery
required, highest score kept, practice allowed after completion with fresh
seeds", which is the behavior students were observed using. A single combined
mode enum would offer a fixed menu instead.

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

Serialization is JSON with camelCase field names. Enums carrying data are
internally tagged, so a client can switch on one discriminant:

```json
{ "kind": "perAttempt", "seconds": 1800, "graceSeconds": 30 }
```

Unit-only enums serialize as plain strings:

```json
"caseInsensitive"
```

Two serde rules are in play and are easy to confuse. On an enum, `rename_all`
renames the _variants_, while `rename_all_fields` renames the fields _inside_
variants. Both are set on every tagged enum here so the whole wire format is
camelCase.

### Flat-question authoring source

[PLE flat-question JSON](QTI-JSON_OBJECT_FORMAT.md) is a narrow answer-bearing
authoring format for ordinary static questions. It is not another public
question model. The native adapter compiles it into this crate's answer-free
`DraftQuestionDefinition` plus separate grader-only material. Published browser
and catalog projections therefore continue to use the shared question model
regardless of whether the author wrote PLE JSON or imported a supported QTI
profile.

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
