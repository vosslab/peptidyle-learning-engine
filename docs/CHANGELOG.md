# Changelog

## 2026-08-31

- Tightened the terminology contract around distinct meanings rather than forbidden words. Checksum now names integrity verification; Digest remains available for a demonstrated content-derived identity, fingerprint, cache discriminator, or deduplication value; Definition and Canonicalization remain technical vocabulary where they are more exact than established PLE terms. Assignment now owns its current Instructor-authored content directly, ordinary saves advance Assignment Edit Number, and Assignment Release creates immutable Assignment Revisions. The vocabulary checklist tracks the coordinated removal of separate Assignment and Question Working Copy lifecycles, integrity values still named Digest, generic PLE-owned Definition names, and misleading canonicalization helpers. The temporary terminology audit reports those categories separately as context-reviewed migrations rather than raw zero-count targets. Documentation formatting, shell syntax, diff validation, and all 192 Markdown-link and guidance-format tests pass.

- Completed the clean Question Version to Question Revision cutover: the Question Model, fresh PostgreSQL schema, adapters, generated contracts, strict browser decoders, Blueprint references, Assignment and Issued Question records, deterministic fixtures, and active documentation now use `QuestionRevisionReference { question_id, revision_number }`; aggregate acceptance is green. Service Login Setup now consistently names the local-stack creation and refresh of its disposable least-privilege database login. Browser course and authoring-workspace routes now use complete `CourseInstanceReference` and `AuthoringWorkspaceReference` runtime narrowing while preserving `C-` and `W-` wire locators; Gradebook navigation names its `AssignmentAttemptReference` directly. Object Address now names the typed server-created physical object location; hostile `objectKey` fixture fields remain only to prove decoder refusal. Renamed fresh migration boundaries for Authenticated Session resolution, authorization checks, and credential-authentication completion; their database functions now name exact Account, Session, workspace, and credential decisions. Browser authoring now names its answer-free comparison Question Publication Review, including its Working Copy Edit Number, summary, route, strict decoder, and test contract; a mounted server publication boundary remains open. Native adapter registration now uses one source-contract key of Question Format, Question Type, and Question Source generator reference; software release metadata remains reproduction evidence. Fork Blueprint Course, Adopt Blueprint Assignment, and Instantiate Blueprint Course now each own exact readiness, blocker, and command-error contracts through previews, server-held apply records, and generated browser types. Forced Question Corrections now store both critical-question endpoints as exact Question Revision References, with revision-named schema constraints and PostgreSQL oracle coverage. Question Change Proposal Revisions now retain their exact base Question Revision Reference and revision-named schema constraints through merge validation. Question authoring now separates Question Publication Requirements, Validation, and ordered Issues; browser validation distinguishes unavailable validation from capability issues. Generic entitlement-materialization naming is retired in favor of exact operation records and receipts. Assignment Working Copy saves now explicitly use Assignment Edit Number; Successor Assignment Revision recovery retains its separate released-history reference. Assignment release now uses Requirements, Validation, and Issues rather than a generic publication-readiness state. The retained authoring review contract is confirmed as Question Publication Review rather than a generic publication diff. Print Exam export now distinguishes borrowed Print Exam Question Input from validated answer-free Print Question output. Object Store metadata now inherits licensing from its owning Question relationship instead of accepting a duplicated generic Object License. Object Store records now derive provenance from typed ownership relationships rather than accepting free-text provenance. PLE Question Backend now uses trusted Question Asset References and Object References rather than an ambiguous Asset Object Binding; the former Native product label is replaced across its package, source and backend discriminators, generated contracts, browser selector, fixtures, and current documentation. Print Exam now accepts borrowed Question Revision and Question Backend Capabilities pairs directly, retaining Print Question as its sole answer-free export record. Object Delivery now has an exact retrieval root, owner-specific delivery relationships, and Pending, Available, or Retired delivery state. Active documentation now distinguishes Assignment Activity Rules from Student Work Records and instructor-facing activity patterns. Question Attempt Reproduction Details now names trusted server-held reproduction facts across model, schema, adapters, and strict Student disclosure boundaries. Assignment structural-change recovery now directs Instructors to the exact successor Assignment Working Copy they must create, while preserving the pinned released Assignment Revision. Human Guidance and active native-adapter contracts now name the eight supported Question Types directly instead of treating them as a generic family. Question Pool Entries now replace the former candidate model across source, schema, generated contracts, strict browser decoders, fixtures, and PostgreSQL acceptance. The implementation-status registry now names its preparatory private-organization receipt Question Folder and Question Folder Share rather than retired collection records. Assignment creation now uses Create Assignment and Assignment Working Copy language across the visible browser flow, route contract, generated documentation, tests, and current docs. Question Pool Selection now consistently names Question Pool Entries in its bounds, selected-entry evidence, Blueprint content, browser contract, editor, and validation messages. The Question Model and browser contract now use `QuestionContentBlock` for the closed shared renderable unit across Question Prompts, response fields, and policy-released Student Feedback. The multi-fill-in presentation contract now uses `PresentedTextEntrySlot` across its generated declaration, strict decoder, response validation, and browser controls. Hotspot presentation records now use `PresentedHotspotRegion` and `PresentedHotspotSurface` across their model, presentation builder, generated contracts, strict decoder, and validation; The private pre-presentation hotspot helper is now `PendingHotspotRegionGeometry`. Current construction, reproduction, verification, and binding helpers now use Question Presentation, Response Item, and current-descriptor names rather than a generation suffix; Student Attempt Descriptor and the consolidated Student Assignment Attempt Screen now use their complete generated contract names; Question Variation Presentation now owns the pre-issuance answer-free rendering input, while Question Presentation owns the issued attempt-bound public contract and Student screen field; The presentation contract now distinguishes Question Choices, Matching Prompts, Matching Choices, and Ordering Items instead of reusing a generic presented-choice record; Issued Attempt Capability now names the current protected-attempt capability enum through the model, generated browser contract, strict decoder, and payload design; Rendered Student Response translation and Student Response Inspection records now use their complete current names through private translation, generated browser contracts, and focused tests; Question Presentation Response Format now names the issued response shape separately from the authored Question Response Format through model, generated API, strict decoder, validation, and terminology contract; Question Asset Id and Question Asset Reference now name the exact Question-owned asset identity and authored checksum across Question content, response formats, object-rendering paths, generated API, and strict decoders; Question Asset Rendition now names the browser-safe issued rendition record and question_asset_renditions collection through the presentation model, descriptor codec, generated API, browser consumers, and focused tests; Instructor-entered Question IDs now pass through parseExactQuestionIds across Assignment editor and Question Pool entry workflows; BlueprintCourseReference now names the reusable Blueprint Course locator throughout the active plan; canonicalCourseLocalDateAndTime now names the Instructor-facing Course Local Date and Time normalizer; Question Revision Substitutions now name Blueprint-operation replacement records and generated contracts; Blueprint Assignment now replaces Reusable Assignment through the complete Blueprint Course contract and Instructor editor; remaining current presentation suffixes remain tracked separately.
- Replaced the source-shaped iMathAS draft locator with `ImathasQuestionLocation`, composed from External Question Provider Reference and iMathAS Item Reference. Source bytes remain separately owned by the immutable Question Source boundary through Source Object Reference and Source Object Checksum; focused iMathAS and workspace compilation gates pass.
- Moved the draft-preview assertion from a host-invoked `wasm_bindgen` export to the real Wasm Node bridge. The test now uses the canonical `backendLocator` contract, proves the key-free PLE preview through generated `wasm32-unknown-unknown` glue, and lets the host Rust suite validate only host-safe bridge behavior.
- Aligned the active terminology plan with the established Question Revision contract; browser draft DTOs and preview fixtures now use the exact `backendLocator` boundary. Formatted the active TypeScript cutover and restored the full aggregate suite, including fresh PostgreSQL and PostgreSQL-MinIO acceptance.
- Replaced the browser and generated `sourceBackend` field with `questionBackend`: a workspace summary or Question Publication Review exposes only its selected Question Backend, while the immutable private Question Source remains separately owned by its exact revision.
- Separated Source Object Reference from Source Object Checksum across Question Attempt Reproduction Details, immutable WeBWorK and iMathAS source bindings, render caches, launch contracts, fresh schema columns, and current terminology. The reference now identifies only the Object Record; the checksum verifies its bytes. Focused adapter, model, source-search, Markdown, and fresh PostgreSQL 17 acceptance gates pass.
- Hardened Source Object Checksum as a validated value: construction and deserialization now require canonical lowercase 64-character SHA-256 text, matching the Question Source schema; WeBWorK and iMathAS source, cache, and replay paths consume it through an explicit accessor.
- Added immutable database-authoritative Object Records and bound every stored Question Source to its required exact private Question Source Object Record, typed owner address, and checksum, with the inline source-data alternative removed. The Workspace Question Source Object Record Store now writes the record only after object bytes exist, current workspace authorization, and exact typed-address validation; the Draft Question Source Store now creates the immutable source only for that authorized revision, closed backend/location shape, and exact prior object/checksum, returning only an identical retry. The fresh PostgreSQL oracle proves foreign keys, RLS, immutable-owner triggers, authorized registration, retry, changed-fact refusal, and unauthorized-workspace denial. Browser decoding now calls the corresponding `decodeDraftQuestionRevision` across HTTP, PLE authoring, QTI import, and behavior tests; payload documentation consistently calls learner-proposed data Student Response; browser and accessibility boundaries consistently call the interaction Question Response Control; lifecycle documentation identifies each Question Backend explicitly; private Question records name their distinct Answer Key, Question Feedback, Question Answer Explanation, and Question Grading Input roles.
- Split the oversized terminology contract without changing its terms: the main contract remains the concise product-term authority, while `docs/INTERFACE_TERMINOLOGY.md` owns the canonical Ribbon and interface-surface vocabulary.
- Split the combined Assignment later-Attempt policy into independent Question Pool Reuse Rule and Question Variation Rule contracts. The Question Model, browser contracts, policy controls, Blueprint defaults, fixtures, and fresh PostgreSQL 17 schema use Reuse Selection or Select Again separately from Reuse Variation or New Variation. Assignment Attempts have an exact Student-and-Assignment sequence; immutable Question Pool Selections retain issued scoring facts and validated same-Student reuse provenance. The unmounted authenticated Assignment Attempt Start Store resolves one active Student session in its transaction and atomically starts or resumes exact Student work; mounted delivery and replay remain open.
- Added immutable released Assignment Revision Entry, Fixed Question, Question Pool, and Candidate snapshots. Selection and Issued Question writes now validate their exact released source, scoring, Question Revision, and reused candidate order. Issued Question identity uses UUID storage across Student Work and correction records and deterministic UUIDv5 derivation from frozen attempt content. The fresh PostgreSQL 17 oracle now proves session-broker RLS, Course Instance and Course Membership Event trigger paths, released-revision locking, exact start/resume, and derived issued scoring facts; aggregate acceptance is green.
- Removed the retired Assignment Audience policy from active browser contracts, dead workspace CSS, and current technical documentation. Active Student Course Membership and exact Student ownership determine ordinary access; a direct Student Accommodation changes only that Student's effective policy. Browser consumer, Markdown/source-limit, terminology-search, and diff gates pass.
- Replaced Question Pool draw vocabulary and algorithm versions with Instructor-owned
  Question Pool Selection Count and Selected Question Order. Rust, generated API,
  strict decoders, Blueprint Course inputs, editor and preview controls, tests, and Instructor documentation now use the direct selection contract and canonical preview route. Student contracts name their ordinal-only field `questionPoolSelectionPosition`; the server owns its one
  current selection implementation, which samples available candidates without replacement from transient server entropy. Stored selected-candidate persistence and replay remain open.
- Replaced generic private Question grading-record tables with exact Answer Key,
  Question Feedback, Question Answer Explanation, and format-specific Question
  Grading Input records; public scoring rule is now `QuestionGradingRule`; persistent
  response-control help uses Keyboard Instructions. Trusted Native and Flat Question
  producers, the Student Feedback Release evaluator, generated contracts, strict decoder, policy controls,
  and UI now preserve Question Feedback, Question Answer, and Question Answer
  Explanation independently. The fresh PostgreSQL catalog oracle proves record
  names, revision/import binding, checksums, immutability triggers, and forced RLS.
  Store-backed persistence, publication, and delivery remain explicitly open.
- Established the Assignment Working Copy persistence boundary. Ordinary
  Questions, Policies, and fixed-question saves now carry the exact Assignment
  Edit Number; the fresh SD1 schema stores one edit-number-guarded working copy
  per stable Assignment, reserves immutable Assignment Revisions for release,
  and permits an Assignment Attempt only for the stable Assignment's selected
  Released Assignment Revision. The focused Rust and browser client gates plus
  the disposable PostgreSQL 17 staged-database acceptance are green. The
  unmounted Assignment Release route and its remaining terminology rows stay
  explicitly open.
- Moved Assignment Status out of editable Assignment Working Copy contracts.
  Question Model, domain policy gates, generated browser types, strict browser
  decoders, workspace requests, UI copy, focused fixtures, and contract tests
  now distinguish Unreleased, Released, Closed, and Archived from replaceable
  working-copy content and Assignment Release Validation.
- Corrected the course-appearance acceptance lane to prove the current typed
  Course Banner object contract only. The retired PostgreSQL current-pointer
  assertions are now explicitly open work under the fresh SD1 schema.
- Completed the Question Attempt State correction: the closed three-state
  contract, persistence integrity, generated API, Student View, and strict
  decoder now share one vocabulary without retaining retired wire values.
- Replaced the PostgreSQL migration-inspection API with Migration Check and
  Migration Check Result. Applied, Pending, Changed, and Incomplete now name
  exact read results; SQLx's ledger remains platform vocabulary.
### Changes

- Added a CELT-based Bloom taxonomy guide for PLE Question classification. It defines the two
  independently selected dimensions, their derived 4 by 6 matrix position, biology-oriented
  examples for all 24 intersections, automatic AI assignment and later Instructor correction, independent
  search facets, and accessible use of the six Cognitive Process hue families.
- Defined Question Bloom Classification as the two-field pair assigned by AI
  work discovered through unassigned Published Question Revisions. Publication
  completes with classification unassigned; AI supplies the initial values, and an
  Instructor may edit either value later without creating a Question Revision.
  Question Search exposes each dimension plus their derived 4 by 6 intersection.
  Added the classification rubric, kept cohort-measured Question Difficulty
  separate, and recorded the six Cognitive Process hue families as accessible
  interface reference anchors. The implementation checklist and current handoff
  keep the source, schema, work discovery, AI integration, API, search,
  metadata editing, and browser cutover explicitly open.
- Separated Blackboard assessment-source language from PLE Assignment
  composition. Question Pool now names an Assignment-local selection from
  explicit Question Revision candidates; Blackboard Question Pool remains a
  reusable source container, Question Set maps after exact candidate
  resolution, and Random Block remains source evidence while import resolves
  its dynamic criteria. Replaced draw and versioned-algorithm terminology with
  Questions to Select, Selected Question Order, durable Question Pool Selection,
  Question Pool Preview, and independent pool-reuse and Question Variation
  rules. The migration checklist and vocabulary detector expose the remaining
  source, API, schema, and interface work.
- Chose one canonical pre-production generation for every PLE-owned internal
  contract. The Terminology Contract now gives domain types and internal
  serializations complete role names while reserving numeric versions for
  registered external standards and independently deployed software evidence.
  Named the internal structured format PLE Question JSON, made stored Question
  Source own deterministic generator definitions, and made a generator repair
  create a Question Revision. The migration checklist and vocabulary detector
  now cover the Presentation `V1` family, WeBWorK replay `V1` records,
  flat-question and `V2` JSON naming, additive generator dispatch, and
  hard-coded Question implementations.
- Defined the complete Published Question Metadata boundary for terminology
  migration: searchable Question Title and Question Description; required,
  separate Question Authorship, Question Owner, and exact versioned Question
  License; optional Question Citation; Question Subject and Subsubject; free-form
  Question Tags; and optional external Question Classifications. Reopened the
  generic Object License migration because stored bytes must resolve legal terms
  through their owning Question Revision, Question Source, or Question Asset.
  Chose Question Revision over Question Version for immutable published history
  and added the required Reason for Edit plus an explicit, receipted choice for
  advancing the editor's Assignment Working Copies.
- Simplified the Question Attempt terminology boundary. The complete Question
  Attempt remains the durable server record; Student Question Attempt View is
  the safe read shape; Question Attempt Reproduction Details hold server-only
  source, software-version, asset, and checksum facts. Reopened the migration
  rows for the current Source Record and Backend, Renderer, and Grader Release
  types, clarified Submission Accepted versus Closed at Deadline, and reserved
  projection for low-level database or query mechanics.

- Completed the Question Attempt Reproduction Details cutover. The trusted
  Rust record is excluded from generated browser contracts and Student HTTP
  serialization; strict browser decoding rejects injected reproduction details,
  while stored fixture validation, adapters, and the fresh `question_attempt`
  schema column retain the complete record.

- Replaced the browser's inherited durable Question Attempt shape with the
  generated Student Question Attempt View. Server construction selects the
  answer-free fields explicitly, generated contracts omit the durable record,
  and Student decoders and submission receipts consume the View directly.

- Replaced the generic Response Definition and Response Schema language with
  Question Response Format across the presentation codec, Native authoring
  behavior test, contracts, plans, and durable documentation.

- Strengthened the fresh Question Attempt persistence contract: its exact
  seed, generated-parameter checksum, state, and reproduction details are
  stored together; deferred constraints require an accepted submission to
  match `SubmissionAccepted` and preserve an empty deadline-closed attempt.

- Replaced Question Backend and Question Grader release records with
  role-specific Version records. All three executable roles now use stable
  implementation names plus exact software versions in trusted reproduction
  details; Native, WeBWorK, iMathAS, fixtures, cache validation, and current
  documentation use the same terms.

- Replaced generic scoring status and witness contracts with Assignment Scoring
  State and Assignment Scoring Snapshot. Rust, generated types, strict browser
  decoding, Gradebook verification, Student feedback gating, fixtures, and
  visible state messages now use the exact assignment-scoring boundary.

- Split object reuse terms from data sensitivity. Object records now carry an
  optional typed Object License and an Object Data Class derived from the exact
  Object Address; memory and S3 backends enforce the same address-derived
  classification.

- Replaced the underspecified iMathAS `ScoredEmbedRenderCacheKey` with the
  server-held `ExternalQuestionProviderCacheEntry`. It binds the exact Question
  Version, normalized provider seed, provider profile, payload digest, and
  expiry; the cache implementation retains its storage address privately.

- Removed the redundant `ObjectCategory` field and enum. Exact Object Address
  variants and their owning relationships now carry object meaning directly;
  storage adapters validate the complete address and immutable record facts.

- Replaced the PLE-owned `Bucket` policy enum with `ObjectStorageArea` and
  `ObjectRecord.storage_area`. The provider adapters retain `BucketNames`,
  MinIO probes, and AWS SDK bucket operations at the storage-provider boundary.

- Replaced `ObjectKey` with `ObjectAddress` across the typed object-store
  contract, object records and writes, S3 and in-memory backends, provider
  caches, conformance tests, and storage terminology. Object Reference remains
  the distinct ownership and delivery-authority relationship.

- Replaced the generic `ImplementationVersion` attempt-evidence record with
  exact Question Backend, Renderer, and Grader releases. Native Question
  Implementations now have their own release type; Rust producers, generated
  contracts, strict browser decoding, fixtures, focused adapters, and durable
  documentation use role-specific fields directly.

- Moved the exact `AssignmentPointValue` contract and its fixed-precision
  validation into the focused Assignment point-value module. The public model
  continues to expose the same type while the primary Assignment model stays
  below the repository source-size limit.

- Split presented-asset validation and selection from the Question Presentation
  builder into its own focused module. The builder is now 979 lines and retains
  construction while the asset module owns reference collection, validation,
  and rendition selection.

- Renamed the server-only presentation integrity binding to
  `QuestionPresentationBinding` across presentation construction,
  verification, Question Model tests, and the active contract documents.

- Renamed the server-minted presentation nonce to
  `QuestionPresentationNonce` through the binding, builder, generated browser
  contract, strict TypeScript check, and identity documentation.

- Renamed the server-held full presentation digest to
  `QuestionPresentationDigest`, keeping it separate from the browser-visible
  public presentation digest token.

- Renamed the browser-visible public digest prefix to
  `QuestionPresentationToken`; generated contracts and the WebAssembly
  presentation verifier retain its exact non-authorizing role.

- Named the complete server-owned issued aggregate
  `IssuedQuestionPresentation`, distinguishing its private bindings and full
  digest from its browser-safe envelope.

- Renamed the authorized score-only inspection record to
  `StudentResponseInspection` across Rust projection, generated contracts,
  Gradebook decoding, Student Work inspection, and focused tests.

- Renamed the public issued-rendition record to `PresentedQuestionAsset` across
  the presentation builder, descriptor codec, generated contract, WebAssembly
  verifier, and Gradebook decoder.

- Confirmed the Question Type boundary: Human Guidance and current contracts
  use MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT as Question Types;
  no PLE-owned Response Family contract remains.

- Corrected the active saved-search receipt to `QuestionSearchFilter` and
  `question_types`; current Question Library contracts carry no parallel
  catalog-named summary, detail, search, or repository record.

- Qualified the two closed components of a Question Pool Selection Rule as
  Question Pool Draw Algorithm and Question Pool Selection Ordering. Rust
  exports, generated browser contracts, strict decoding, Blueprint Course
  inputs, and Question Pool previews now use the owned terminology directly.

- Replaced generic renderer provenance with Question Renderer Version in the
  Local Stack Controller's private selected-image record. Question Attempt
  Reproduction Details now use the same role-specific version type through
  WeBWorK configuration, iMathAS rendering, cache validation, and focused
  adapter tests.

- Replaced the generic generation `Seed` type with `QuestionSeed` through
  Question definitions and variations, deterministic generation, all adapters,
  object cache keys, WASM, generated browser contracts, fixtures, and focused
  Rust and TypeScript validation.

- Replaced `GeneratedVariant` and `GeneratedValue` with private Question
  Variation Parameters and Question Variation Parameter Values. The generated
  parameter output remains separate from the durable issued Question Variation.

- Corrected the generated-output vocabulary boundary: a transient generated
  parameter map is distinct from the durable issued Question Variation, which
  owns the Question Version, Question Seed, and declared generator recipe.

- Replaced `GeneratorReference` and `ParameterSpec` with
  `QuestionGeneratorReference` and `QuestionGeneratorParameter` through the
  Question Variation definition, generation engine, native adapters, generated
  browser API, strict decoding, documentation, and focused tests.

- Replaced `RandomizationDefinition` and its `randomization` contract field
  with `QuestionVariationDefinition` and `questionVariationDefinition` across
  Question definitions, generation, adapters, generated browser contracts,
  strict decoders, editor state, fixtures, and focused tests.

- Clarified the durable grading vocabulary: Question Grading Rule, Answer Key,
  Question Feedback Asset, Student Feedback Attachment, and format-specific
  grading records are separate owned concepts. "Material" remains ordinary
  teaching prose and "materialization" remains its separate Tier 2 concern.

- Separated Question Hints from Question Feedback. The Native adapter now
  verifies the exact issued Question on a dedicated pre-response hint path;
  post-grade feedback retains its selected-choice, correct-outcome, and
  incorrect-outcome levels through automatic grading, Student release,
  browser decoding, display, and focused tests.

- Expanded the terminology and flat-question JSON contracts around Question
  Hint, Choice/Correct/Incorrect Feedback, Question Answer Explanation, Keyboard
  Instructions, Keyboard Tooltip, Response Format Message, and exact Hint,
  Feedback, Answer, and Answer Explanation Asset roles. QTI imports now map by
  instructional meaning even when QTI uses a feedback block as the container.
  Student Feedback remains a transient authorized view whose audit trail comes
  from the Assignment Revision and exact grading and Question records.

- Separated Question Answer from Question Answer Explanation in the terminology
  and migration contracts. Question Answer is the display-ready accepted
  response derived by a trusted backend from the private Answer Key; Question
  Answer Explanation is the optional teaching explanation of how or why. The
  checklist now requires three explicit grading outputs and independent Show
  Question Answer and Show Explanation release fields instead of the current
  combined release field.

- Reopened the Response Definition and Response Schema vocabulary rows after a
  current audit found 21 active matches across documentation, the active plan,
  a presentation-codec helper, and a browser test. Both wordings map to one
  Question Response Format. The solution audit now tracks the combined release
  field, combined visibility flags, and solution-free boundary prose while
  retaining exact QTI and WeBWorK protocol vocabulary in their adapters.

- Corrected the presentation-asset terminology after the type-only
  `AssetBindingV1` rename left binding-shaped fields and helpers around a
  `PresentedQuestionAsset` record that actually describes a selected rendition.
  The contract now distinguishes Question Asset, Question Asset Reference,
  Question Asset Rendition, and Object Delivery. Reopened the asset and Response
  Item Binding rows and added migrations for role-specific presented Question
  Choices, Matching Prompts, Matching Choices, Ordering Items, and Text Entry
  Slots.

- Separated Question Publication from Assignment Release. An Assignment now
  has one stable Course-owned identity, at most one replaceable Assignment
  Working Copy, a working-copy Assignment Edit Number, and immutable Assignment
  Revisions created only by successful releases. Assignment Status belongs to
  the stable Assignment; Assignment Access combines Released status with
  schedule, membership, and policy. The migration checklist and vocabulary
  detector now track the current edit-per-revision, Draft/Published Assignment,
  Publication Readiness, and revision-local lifecycle boundaries as one
  foundational storage correction.

- Audited every vocabulary-replacement checkbox against the current material
  tree. Reopened fifteen rows whose source, durable documentation, Human
  Guidance, or active plans still carry the outgoing term. Each reopened row
  now states the exact replacement action, affected owners and consumers, and
  validation step. Clarified Assignment Point Value, Hotspot Region, Course
  Date, Relative Assignment Schedule, and Question Library route rows so their
  checked state describes a completed contextual correction.

- Replaced generic private `FeedbackContent` with Question Feedback and the
  opaque `DisclosedFeedback` and version-suffixed
  `InspectedStudentScoreFeedbackV1` contracts with Student Feedback and
  Student Response Inspection Feedback. The Question Model, grading, release
  evaluation, generated browser API, strict decoders, Student state, Gradebook,
  documentation, and tests now distinguish Question-attached automatic
  feedback from its policy-released Student result.

- Consolidated browser test data onto the approved published-Question fixture
  set. Deleted its stale duplicate JSON projection, which carried retired
  Question identities and fewer attempt records. Browser tests now make their
  one required wire-shape conversion explicitly, and Rust export tests load
  the shared stored JSON at runtime rather than embedding its contents.

- Replaced the published-content `QuestionDefinition` contract with Question
  Version across the Question Model, generated browser API, domain policy,
  adapters, grading, export, project tools, strict decoding, Wasm, tests, and
  documentation. Draft Question Definition remains the private authoring
  contract. Updated pilot payload fields and fixture Assignment Revision pins
  to their current immutable contracts.

- Corrected the active Question Model, payload, data-contract, accessibility,
  grading, and implementation-status documentation to list only the supported
  eight Question Response Formats. The Student UI continues to select its
  control exclusively from issued Question Response Format, not Question Type
  or Question Format.

- Replaced the overloaded `ChoiceOption` with Question Choice, Matching Prompt,
  Matching Choice, and Ordering Item records. Each response format now carries
  its own exact learner-visible item type through adapters, validation,
  grading, presentation, generated browser contracts, decoders, controls, and
  tests.

- Retired the unmounted File Upload Question Response Format, browser control,
  generated contracts, and free-form object-key Student Response. A future
  Student Upload Reference requires a server-issued Object Reference bound to
  the exact Student Record and Question Attempt.

- Replaced `RenderedItemIdV1`, `RenderedItemRoleV1`, and
  `RenderedItemBindingV1` with Presentation Response Item Reference, Response
  Item Role, and Response Item Binding across Question Presentation
  construction, response translation, strict browser contracts, generated
  types, documentation, and tests. Roles now name Question Choice, Text Entry
  Slot, Matching Prompt, Matching Choice, Ordering Item, Hotspot Surface, and
  Hotspot Region precisely.

- Replaced the coordinate-shaped `HotspotPoint` response with Student Hotspot
  Selection. Student Responses now identify selected presentation-scoped
  Hotspot Regions that translate server-side to durable region references;
  authored geometry remains in the Question Response Format.

- Replaced `MatchPair` with Student Match across Student Response, matching
  validation, presentation translation, native flat-question grading, Question
  Response Controls, generated browser types, and tests.

- Replaced `TextEntryAnswer` with Student Text Entry across Student Response,
  domain validation, presentation translation, native flat-question grading,
  generated browser types, and tests.

- Replaced the overloaded `ChoiceId` with Response Item Reference across
  Question Response Formats, Student Responses, presentation translation,
  validation, grading, adapters, generated types, strict decoders, and WASM.

- Replaced `SelectionCardinality` with the explicit Response Selection Rule
  through Question Response Formats, adapters, validation, grading,
  presentation construction, generated types, decoders, WASM, and tests.

- Renamed the Question Response validation result and issue contracts to
  Student Response Format Check and Student Response Format Issue across
  domain validation, grading, WASM, strict decoding, controls, and tests.

- Replaced product-owned Account and Course Instance provisioning language with
  Account Creation and Course Instance Creation across authorization, identity,
  enrollment, security, design, and schema documentation. Service Login Setup
  remains the distinct disposable operational term.

- Corrected the Student-facing Assignment Summary and Activity Model to state
  the complete eight-rule Assignment Activity contract.

- Made the complete eight-rule Assignment Activity contract part of the
  immutable Assignment Revision model and generated browser contract. The
  disposable PostgreSQL baseline now represents every rule explicitly, with
  threshold and continuation-cap constraints that apply only to their matching
  rule variants.

- Named the Assignment Attempt time limit explicitly across the Course delivery
  schema, Base and Effective Assignment Policy, reusable Blueprint defaults,
  Instructor schedule input, direct Student Accommodation, strict browser
  contracts, and Student-facing delivery copy. The Question timing cutover
  remains a separate retained vocabulary task.

- Replaced the mixed Question `TimingPolicy` with the closed Question Attempt
  Time Limit contract. A Question now names only an Unlimited or Limited
  Question Attempt; the whole Assignment Attempt deadline remains in the
  explicit Assignment policy field.

- Renamed the generic timer verdict, evaluation, WASM export, browser fallback,
  strict decoder, and validation route to the exact Question Attempt Timing
  Decision boundary.

- Replaced Assignment policy labels for Late Work Rule, Student Late Work
  Status, and Assignment Deadline Rule across schema, effective policy,
  delivery, strict browser contracts, previews, Blueprint defaults, and
  Instructor controls.

- Replaced the generic answer-free Assignment Landing Presentation aggregate
  with Assignment Overview in the Course model and direct Student and
  Instructor view composition contracts.

- Recorded the exact Student Response and Question Submission boundary. Active
  source retains no Student Answer identifier or ambiguous Question response
  aggregate; Question Response Format and Question Response Control remain
  separate exact terms.

- Replaced the generic Attempt Policy contract with Question Attempt Limit
  across Question Model, native flat-question source JSON, generated browser
  types, strict decoders, authoring UI, adapter contracts, fixtures, and
  focused behavior tests.

- Replaced the last active Response Definition customer-spec example with
  Question Response Format and recorded the current response-shape and
  browser-control contract evidence in the retained terminology checklist.

- Retired the mechanically renamed `QUESTION_IDENTITY.md` after assigning its
  useful rules to their canonical owners. `QUESTION_ID_SPEC.md` owns the
  human-facing Question ID and exact Question Version Reference;
  `IDENTITY_CONTRACTS.md` owns internal record and relationship scopes;
  `ASSESSMENT_PAYLOAD_DESIGN.md` owns presentation-scoped values; and the
  terminology contract owns publication and availability meanings.

- Reconciled the active identity plan, contract register, cache design, and
  retained vocabulary evidence with the current Question ID and Question
  Version Number schema; retired Problem UUID identity language no longer
  describes the PLE-owned model.

- Renamed the browser Assignment Attempt decoder, completion presentation,
  and their focused regression test from generic `run` paths to exact
  Assignment Attempt paths, with no compatibility export.

- Replaced the generic browser Pool Selection contract, wire field, visible
  Student status, fixture, and focused decoder/client tests with the exact
  Question Pool Selection boundary. Durable selection storage remains tracked
  separately in the retained vocabulary checklist.

- Replaced the generic domain `run` module and active contract package with
  `assignment_activity`; continuation, completion, scoring, direct design
  links, and the 31st-Assignment-Attempt regression test now name the exact
  Student-work boundary.

- Replaced the retired `RunScreenData` and `RunSummaryResponse` names in the
  course-appearance contract with the current Assignment Attempt screen and
  summary contracts.

- Replaced the retired `getRunScreen` description in the assessment-payload
  contract with the current `getAssignmentAttemptScreen` client and exact
  Student-work hierarchy.

- Replaced retired run-model and run-route wording in the failure-recovery
  contract with Assignment Activity and Assignment Attempt terms.

- Replaced the retired Student Assignment Summary aggregate in enrollment
  documentation with the separate Assignment Grade and Assignment Progress
  records already used by current code and the gradebook schema.

- Replaced the generic `PointValue` contract with `AssignmentPointValue`
  across assignment entries, reusable course contracts, Course Grade
  calculation, generated TypeScript, and strict browser decoding.

- Bound every Assignment Attempt to its exact Published Assignment Revision
  across the generated contract, strict browser decoder, immutable Student-work
  fixture, and PostgreSQL same-Assignment/published-revision guards.

- Replaced opaque Assignment edit revision tokens with exact Assignment
  Revision Reference preconditions across the Questions, Policies, and
  fixed-question replacement contracts.

- Replaced generic shared Assignment title text with the validated Assignment
  Title contract across requests, reusable definitions, browser projections,
  and the direct immutable Assignment Revision schema.

- Replaced the separate Assignment Teaching Settings contract and opaque
  revision delivery JSON with an Assignment Revision Definition and direct,
  immutable Assignment Revision delivery fields.

- Replaced `WeightedGradeCategory` with `GradeCategory` across the Course Grade
  Scheme model and Domain calculation contract.

- Replaced opaque Forced Question Correction remediation JSON with direct
  immutable manifest targets for Assignments, Assignment Attempts, Issued
  Questions, and Assignment Grades.

- Recorded the exact Question Version pairs and immutable Forced Question
  Correction Reference already enforced by the correction-manifest schema.

- Recorded Question Change Proposal as the schema-backed, immutable
  improvement path for one exact published Question Version, including its
  foreign-keyed `base_question_id` and `base_version_number` pair.

- Recorded Course Origin as the verified immutable Course Instance creation
  record across schema, contracts, rollover receipts, and generated API.

- Recorded Question provenance as separated current ownership: `QuestionSource`
  for reproducible material and `QuestionSearchAuthorship` for reviewed public
  attribution, with Question Attempt Source Record retained for reproduction.

- Replaced generic preview projections with `EffectiveAssignmentPolicyView`
  and `StudentFeedbackReleaseView`, including exact public contract members.

- Split the preview identity boundary. Authorized Instructor roster selection
  is `SelectedStudent`; the identity-free transport model is
  `StudentViewScenario` with a `student_view_scenario` contract member.

- Recorded the retired Assignment Materialization plan vocabulary as absent.
  Copy Assignment from Blueprint remains the sole named Course Instance
  operation for creating an Assignment from one Blueprint Assignment.

- Removed the unused generic materialization model from Question Model. It had
  no schema, contract, or receipt consumer and duplicated the direct Student
  Course Membership already carried by the membership gate. Operation-specific
  receipt records remain the required future replacement.

- Recorded the policy-scope correction as complete. Effective Assignment Policy
  resolution accepts the Base Assignment Policy and one exact direct Student
  Accommodation, with no roster-subdivision scope collection or transport
  field.

- Corrected the Assignment Access ownership boundary. The full present-time
  decision is now `AssignmentAccessDecision`; the former membership-only gate
  is `ActiveStudentCourseMembershipDecision`. Browser contracts name that
  prerequisite directly, while Assignment start and late-work results use the
  corresponding exact closed terms.

- Replaced `ResolvedField` with `EffectiveAssignmentPolicyValue`. Each
  resolver value now names both its Effective Assignment Policy role and its
  exact Assignment Policy Source.

- Replaced the generic Policy Patch domain and browser contracts with
  Accommodation Adjustment. Synthetic previews and direct Accommodation
  updates now carry the closed `adjustment` member, and the strict decoder
  rejects the retired `patch` member.

- Removed the obsolete browser test for a schedule-offset request. Assignment
  Access now tests only the current policy-adjustment contract rather than a
  retired, unsupported request surface.

- Replaced `PolicyModificationMode` with `AccommodationApplicationRule` and
  its browser-safe view. Direct and hypothetical Student Accommodations now
  name the closed rule that applies their adjustment to the Base Assignment
  Policy.

- Replaced the PLE-owned Entitlement decision, grant, denial, facts, preview
  outcome, and browser field with Assignment Access terms. The explicit
  active-membership gate remains separate from later lifecycle and effective
  policy evaluation, and denial reasons now name missing Course Membership
  rather than an opaque entitlement state.

- Replaced the competing preview-provenance labels with `AssignmentPolicySource`
  and `AssignmentPolicySourceKind`. Instructor previews retain their exact
  direct Student Course Membership source when authorized; identity-free
  previews retain only the safe source kind.

- Removed `CourseInstanceApplicationBinding`. Existing-Course Instance apply
  records now retain their `CourseInstanceSnapshot` precondition and
  `CourseOrigin` source history as direct, independently named fields.

- Replaced `AppliedAssignmentImportEvidence` with `AssignmentSourceRecord`
  and `CreateSelectedBlueprintAssignmentReceipt` with
  `CopyAssignmentFromBlueprintReceipt`. Immutable completion evidence now
  separates the server-held committed Assignment source from the browser-safe
  source projection and names the exact copied Blueprint Assignment operation.

- Replaced `CourseInstanceImportWitness` with `AssignmentSourceSnapshot`.
  Assignment import preconditions now name their exact Blueprint Assignment
  Revision source, destination Assignment Revision, and import revision.

- Replaced `CourseInstanceReceiptTarget` with
  `CourseInstanceOperationReceipt`. Reconciliation now explicitly selects one
  closed immutable Course Instance operation receipt rather than a generic
  target wrapper.

- Replaced `RolloverCourseInstanceManifest` with `CourseRolloverManifest`.
  The manifest now names the closed reusable state copied into a new Course
  Instance and its mandatory exclusion of all Student and delivery records.

- Replaced `CourseInstanceBlueprintApplication` with `CourseOrigin`. The
  immutable source relation now records the exact Blueprint Revision and, for
  rollover, the exact source Course Instance rather than collapsing both paths
  into a generic application label.

- Replaced `BlueprintCourseCreationWitness` with `BlueprintForkReservation`.
  The fork path now explicitly names its server-held source revision, authorizing
  Account, request digest, Retry Token, and reserved Blueprint Course Reference.

- Replaced `CourseInstanceCreationWitness` with
  `CourseInstanceCreationReservation`. The server-held pre-creation record now
  explicitly names its source, target Course Term, authorizing Account, request
  digest, Retry Token, and reserved Course Instance Reference.

- Replaced `CourseInstanceWitness` with `CourseInstanceSnapshot` across
  Blueprint-operation previews, commands, records, receipts, and generated
  contracts. The snapshot remains immutable observed state and is explicitly
  separate from the Course Instance Creation Reservation.

- Recorded the existing Course Date boundary as complete: Course Dates are
  exact proleptic-Gregorian calendar values in Course Terms and Course Schedule
  Revisions, never timestamps or implicit time-zone conversions.

- Replaced the standalone `CourseScheduleRevision` compare-and-swap counter
  with `CourseScheduleRevisionReference` and
  `CourseScheduleRevisionNumber`. Every Course Instance witness now binds the
  referenced Course Instance explicitly and rejects a revision from another
  course before an operation can proceed.

- Moved Course Term and resolved delivery-time ownership from mutable
  Assignments into immutable Course Schedule Revisions and Assignment Revisions.
  The clean PostgreSQL baseline now proves the same-course foreign keys,
  immutable revision records, exact timestamp ordering, and revision-owned
  availability index.

- Replaced `ResolvedRelativeAssignmentSchedule` and
  `ResolvedRelativeScheduleMoment` with `ResolvedAssignmentSchedule` and
  `ResolvedAssignmentScheduleMoment` across Course Term resolution, operation
  records, receipts, and generated contracts. Each Assignment Revision now
  stores the resulting durable times.

- Replaced `RelativeScheduleMoment` with
  `RelativeAssignmentScheduleMoment`. Blueprint Revision Content, Course Term
  resolution, the Blueprint Course editor, and generated contracts now name the
  Assignment-scoped reusable schedule moment directly.

- Replaced `CourseLocalDateTime` with `CourseLocalDateAndTime` across Course
  Term, teaching scheduling, domain projection, browser decoders, and generated
  contracts. The value remains a wall-clock input resolved only by Course Time
  Zone, with DST gaps and ambiguities refused.

- Replaced `IanaTimeZone` with `CourseTimeZone` throughout Course Term and
  teaching contracts. The product term now identifies the course-owned zone,
  while validation continues to require one exact case-sensitive IANA name.

- Removed `CurriculumReplayStatus` and the browser-visible Applied/Replayed
  completion field. A repeated Retry Token now resolves the same server-held
  receipt without creating a second product state.

- Replaced generic `CourseInstanceEligibility` with exact readiness types for
  Copy Course for New Term, Shift Course Dates, Apply Blueprint Update, Copy
  Assignment from Blueprint, and reconciliation. Each preview and apply record
  now accepts only its operation's `Ready` or `Blocked` result.

- Replaced `BlueprintAdoptionEligibility` and its refusal wrapper with
  `BlueprintOperationReadiness` and `BlueprintOperationBlocker`. Blueprint
  previews now state `Ready` or `Blocked` directly, and commands require that
  exact readiness before construction.

- Replaced `AssignmentDefinitionSourceView` with
  `BlueprintAssignmentRevisionReference`. The generated browser contract and
  every Blueprint-operation preview, command, record, and receipt now retain
  the Blueprint Assignment lineage with its exact Blueprint Revision.

- Replaced `ObservedBlueprintSource` with `BlueprintRevisionReference`. Every
  Blueprint-operation preview, command, witness, and receipt now names the
  exact Blueprint Course and immutable Blueprint Revision pair directly.

- Replaced the residual curriculum-pin boundary with exact Question Version
  substitutions. Blueprint-operation requests and recovery choices now carry
  immutable Question Version References, while Blueprint Question Position
  identifies only the corrected content location.

- Replaced the generic curriculum semantic payload model with Blueprint
  Revision Content. The exact Blueprint Course, Blueprint Assignment, module,
  assignment-entry, and Question Pool content components now use the same
  versioned encoding domain, digest, and terminology-contract definition.

- Replaced `CurriculumSemanticComparison` with `BlueprintContentCheck`.
  The comparison and durable assignment-import evidence now name complete
  Blueprint Revision Content and its Blueprint Content Digest directly.

- Replaced `CurriculumSemanticDigest` and `CurriculumSemanticEnvelope` with
  `BlueprintContentDigest` and `BlueprintRevisionContentRecord`. Canonical
  immutable Blueprint Revision content now names its value and digest directly.

- Replaced `CurriculumPinReplacements` with `QuestionVersionSubstitutions`.
  Blueprint-operation previews, commands, and receipts now name the reviewed
  exact Question Version replacement boundary directly.

- Replaced `CurriculumAdoptionIdempotencyKey` with
  `BlueprintOperationRetryToken`. The opaque retry binding now names the exact
  Blueprint-operation boundary across commands, server records, and receipts.

- Corrected the retained Blueprint-operation inventory to match the closed
  seven-variant contract: Adopt Blueprint Assignment and Create Selected
  Blueprint Assignment are distinct operations with their own receipts.

- Replaced the reusable curriculum root with Blueprint Course across the Rust
  model, browser API, strict decoder, feature directory, tests, and current
  documentation. Immutable versions now consistently read as Blueprint
  Revisions; retained screenshot asset paths remain historical evidence.

- Corrected the temporary vocabulary inventory to count complete terms. The
  The generic-authorization inventory now reports zero because Factory is a distinct term, rather
  than being counted as a substring match.

- Completed the Factory terminology audit. Factory now appears only in its
  narrow terminology definition, retained correction map, changelog history,
  and audit evidence; PLE implementation uses direct constructors or names the
  injected action.

- Replaced `AttemptResult` and `GradeOutcome` with the explicit Grading Result
  boundary. Question Submission now owns the accepted Student Response and its
  optional Grading Result; trusted checkers return `QuestionGradingOutcome`.
  The fresh PostgreSQL baseline binds each result to its exact submission,
  grading operation, and immutable receipt with composite foreign keys.

- Replaced the generic stored Activity Model boundary with Student Work
  Records. `question_model::student_work` now owns the exact Student Record,
  Assignment Attempt, Issued Question, Question Attempt, Question Submission,
  and Assignment Grade contracts; durable documentation names the same
  ownership structure.

- Replaced the generic Local Stack injection names `lease_factory` and
  `reset_runner_factory` with `acquire_browser_suite_lease` and
  `create_command_runner`. The acceptance-profile, SD1 staged-database,
  course-appearance, and developer-supervisor owners now state their injected
  actions directly.

- Replaced `SourceArtifact` with `SourceObjectReference`. The Question Attempt
  source record now names the exact immutable Object ID and SHA-256 checksum;
  native, WeBWorK, and iMathAS reproduction paths, generated API, strict
  browser decoding, fixtures, and terminology contracts use that boundary.

- Split the retired `QuestionEnvelope` into `QuestionVariation` and
  `QuestionPresentation`. The variation now preserves exact version, seed,
  generator, and declared parameters; the presentation retains answer-free
  rendered material. Adapters, caches, generated API, strict browser decoding,
  and issued-presentation consumers use the new boundary.

- Replaced `TextMatchMode` and `NumericTolerance` with the precise public
  `TextResponseMatchRule` and `NumericResponseTolerance` contracts. Question
  Response Format, native flat-question JSON, grading, generated API, strict
  decoder, and authoring now preserve the same answer-free rules while the
  Answer Key remains server-held.

- Aligned native flat-question import documentation with its existing direct
  construction boundary: `ImportedFlatQuestion::from_imported` and
  `ImportedFlatQuestionError` now name trusted import construction precisely.

- Replaced generic recorded iMathAS Factory wrappers with direct, named
  recorded-provider and recorded-transport construction. Feature-gated test
  support now exposes the exact provider, transport, and paired construction
  outcomes.

- Replaced `BackendCapabilities` with `QuestionBackendCapabilities` across
  the Question Model, Question Backend adapters, assignment policy evaluation,
  generated API, strict browser decoder, and current contracts. The closed
  capability declaration now names the exact Question Backend boundary.

- Defined Factory as the narrow technical pattern that chooses among multiple
  construction strategies. Added concrete correction tasks for Local Stack
  injected actions, direct native import construction, and iMathAS recorded
  test constructors; the vocabulary count script now tracks Factory for
  contextual review.

- Replaced `ProviderLaunchHandle` with `ExternalToolLaunchReference` in the
  iMathAS adapter. The opaque reference now names its exact External Tool
  Launch Session boundary.

- Replaced the iMathAS `broker_provider` module with
  `external_question_provider`. The adapter now names its External Question
  Provider boundary while retaining exact launch, exchange, render, and grade
  transport contracts.

- Replaced Response Widget paths and exports with Question Response Controls.
  `question_response_controls/` now owns the dispatcher and concrete controls;
  `QuestionResponseControl` collects a Student Response compatible with the
  declared Question Response Format.

- Replaced the unqualified browser Attempt feature with Question Attempt
  state. `src/features/question_attempt/` now exports
  `QuestionAttemptExperienceState` and its state machine, distinct from the
  generated server `QuestionAttemptState` contract.

- Replaced the Local Stack Controller's generic Consumer component with the
  Disposable Stack Adapter and Disposable Stack Command. Controller, E2E,
  focused test, and operational paths now use `disposable_stack_adapter.py`
  and `disposable_stack_command.py`.

- Replaced the browser-wide `ApiRuntime` boundary with Application API. The
  application now injects `ApplicationApi` through `ApplicationApiProvider`
  from `src/api/application_api.tsx`, leaving runtime terminology for actual
  execution environments and lifecycle.

- Keyed Ribbon Schema terminology by Ribbon Scope and immutable Product Role,
  classified Account and Profile as Ribbon Context Controls, and defined
  Attempt, Back to Assignments, Assignment Attempt Progress, and No Selected
  Ribbon Tab. The UI Design Guide now owns the per-role Slot order, Task
  grouping, Context Row placement, relationship-dependent suffix, and
  Assignment Attempt composition. The vocabulary checklist points each
  correction to its proper documentation owner.

- Replaced the residual `run_policy.rs` implementation path with
  `assignment_activity_rules.rs`. The Question Model now exports the six
  independent Assignment rules from the same canonical module named in the
  terminology contract and replacement checklist.

- Replaced `StudentDisclosurePolicy` and `StudentDisclosureTiming` with the
  Student Feedback Release Rule and its per-field release timing. The Question
  Model, domain release evaluator and preview projection, generated contract,
  strict browser decoders, workspace controls, reusable curricula, Student
  presentation, fixtures, and terminology now use
  `studentFeedbackReleaseRule`; the domain module and wire path use
  `student_feedback_release`.

- Defined the canonical Question Format, Question Type, Question Backend,
  Question Presentation, Question Response Control, Student Response,
  Question Submission, Assignment Submission, Question Search, and Question
  Picker boundaries in the terminology contract. The retained vocabulary
  checklist and terminology boundary audit now assign the active ambiguous
  source paths to six exact implementation owners with explicit success and
  validation conditions.

- Replaced `ContinuedPractice` with the exact Assignment Attempt Continuation
  Rule. `assignmentAttemptContinuationRule` now carries Unlimited, Capped, or
  Closed through the Question Model, domain eligibility evaluator, generated
  contract, strict browser decoders, workspace controls, reusable curricula,
  fixtures, and terminology.

- Replaced `GradePolicy` with the exact Assignment Attempt Grade Rule.
  `assignmentAttemptGradeRule` now drives First, Latest, Highest, and
  Instructor Selected gradebook choice through the Question Model, domain
  evaluation, generated contract, strict browser decoders, workspace controls,
  reusable curricula, fixtures, and terminology.

- Replaced `CompletionRequirement` with the exact Assignment Completion Rule.
  `assignmentCompletionRule` now carries the closed Answer All, All Correct,
  or Score At Least definition through the Question Model, domain evaluator,
  generated contract, strict browser decoders, workspace controls, reusable
  curriculum, fixtures, and terminology.

- Replaced the generic `VariationPolicy` contract with the exact Question
  Variation Rule. `questionVariationRule` now closes the three distinct
  later-Attempt choices: retain Questions with fresh Question Seeds, use
  Instructor-selected Question Variants, or redraw Question Pools. The private
  Question Pool Selection Basis derives the corresponding server input without
  becoming an Instructor-facing policy.

- Combined the static Question Pool selection method and output ordering into
  `QuestionPoolSelectionRule`. The Rust model, generated browser contract,
  strict editor and preview decoders, reusable-curriculum semantic digest,
  pool-preview projection, editor input, fixtures, and durable terminology now
  share the same `selectionRule` / `selection_rule` boundary. Question
  Variation Rule remains the separate owner of later-attempt reuse or redraw.

- Replaced generic Assignment scoring mode with Assignment Entry Scoring Rule.
  Fixed Questions and Question Pools now carry the explicit Normal, Full Credit,
  Extra Credit, or Excluded rule through the Question Model, generated browser
  contract, strict decoder, reusable-curriculum semantic digest, editor input,
  fixtures, and durable terminology.

- Split generic Assignment delivery state into exact ownership records.
  Fixed Questions and Question Pools now own Assignment Entry Availability;
  Question Pool Candidates own their distinct Question Pool Candidate
  Availability. Rust, generated browser types, strict decoding, editor requests,
  Student presentation, fixture behavior, terminology, and the retained
  replacement checklist now use Available or Retired at the correct boundary.

- Replaced mutable Assignment publication-readiness wording with the explicit
  Draft Assignment Revision boundary. The Rust and generated browser contracts,
  strict decoder, validation discriminator, workspace pages, fixtures, and
  durable terminology/API contracts now use
  `DraftAssignmentRevisionPublicationReadiness` /
  `draftRevisionPublicationReadiness` for the closed blockers of one exact
  Draft Assignment Revision.

- Closed the retained Assignment Instance and mutable Assignment aggregate checklist rows with
  current evidence. The active tree uses the separate Course Instance-owned Assignment, Student
  Record-owned Assignment Attempt, stable `assignment`, and immutable `assignment_revision`
  boundaries directly.

- Replaced mixed `AttemptStatus` with the exact `QuestionAttemptState` record field. Its closed
  lifecycle is Open, Submitted, or Automatically Submitted; exclusion and exemption no longer
  appear as operational state variants. Rust, generated contracts, strict browser decoding, active
  attempt selection, fixtures, durable activity documentation, and the active status registry now
  use the same state boundary.

- Removed the final active "run model" wording from the Question Backend contract. Native Question
  Implementation now states its exact Question Format, Question Type, and Question Generator
  boundary without suggesting a bundled Assignment Run Mode; the retained checklist records that
  Assignment Activity Rules own the independent delivery policies.

- Replaced the mixed `StudentSubmissionStatus` transport with a Question Submission
  Acknowledgement that carries its accepted Receipt and current grading state separately. The
  strict browser decoder, HTTP client, attempt state machine, pending UI, fixture tests, Rust
  grading-state contracts, and generated types now distinguish full Question Submission Grading
  State from its answer-free Student projection.

- Replaced the generic `AssignmentRevision` counter with the exact immutable
  `AssignmentRevisionNumber` and `AssignmentRevisionReference` pair. Course Instance witnesses,
  reconciliation receipts, generated contracts, naming guidance, and the strict portable contract
  now carry the stable Assignment Reference with its immutable revision number.

- Replaced the residual "Student Assignment Attempt" labels with the exact Assignment Attempt
  record across the SD1 schema commentary and browser, data, and security contracts. The direct
  Student Record and Assignment relationships now supply the ownership context without inventing
  a second product term.

- Corrected the fixed-role browser boundary. Sysadmin sessions no longer enter Instructor, Question
  Library, course, Gradebook, or Student-work routes by role alone. Instructor approval now has its
  own Sysadmin platform route; course-specific help remains an explicit support-capability boundary.
  The Account/session schema, Store, browser contract, and role documentation now describe the same
  one-role-per-Account model.

- Replaced the stale deterministic-RNG output snapshot with its actual durable contract: one valid
  seed and stable label reproduce the same unsigned decision stream. The helper has no published
  byte-sequence protocol, so the test now protects deterministic behavior rather than an obsolete
  literal sequence.

- Removed two obsolete Node wrappers for the retired screenshot corpus. They imported corpus and
  capture helpers deleted with that acceptance system; the deterministic browser-contract lane now
  runs independently, while real-browser restoration remains a separate acceptance task.

- Corrected the visual-evidence documentation to match the current tree. The retained Instructor,
  Student, and theme galleries are now labeled historical; documents no longer prescribe the
  deleted screenshot corpus or capture command, and they retain fresh real-browser plus human
  visual review as the required future acceptance boundary.

- Corrected the active package and release records to distinguish current database/object acceptance
  from the retired browser owner. Browser behavior and visual evidence now remain explicitly open
  rather than being claimed through removed scripts, manifests, or historical screenshots.

- Removed the retired `RunBackend` documentation seam. The Question Backend contract now names
  the existing native, WeBWorK, iMathAS, and QTI adapters and the unmounted server delivery
  boundary, rather than deleted server run modules.

- Replaced `AttemptProvenance` with the exact `QuestionAttemptSourceRecord` across the Question
  Model, issued native/WeBWorK/iMathAS attempts, reproduction and grading checks, generated API,
  strict decoder, fixtures, and durable documentation. Question Attempts now expose the precise
  `source_record`/`sourceRecord` boundary. Attempt advancement now also requires the exact Question
  Version Reference as well as its seed; stale tests now use Assignment Attempt buffer keys and
  current source-record wire fields.

- Replaced the Course Instance inspection contract's `BlueprintAssignmentProvenance` record with
  `AssignmentSource`. The exact bounded browser field is now `assignment_sources`, making the
  Blueprint Assignment source and import revision explicit through Rust and regenerated types.

- Replaced the stale single-installation implementation blueprint with the current terminology and
  contract-alignment plan. It now records the fresh schema baseline, exact Account-owned
  authorization relationships, Question Library/Search boundaries, mounted `server_core` surface,
  retained vocabulary checklist, and proportional evidence model without carrying retired product
  terminology as current work.

- Removed obsolete roster-group wording from the direct Assignment preview and Student delivery
  model, and removed two orphaned grouping CSS selectors. The remaining policy wording now names
  direct Course and individual adjustments without suggesting a PLE roster-group feature.

- Corrected the Server Application file-structure map to the mounted `server_core` surface:
  authentication, composition, health, request lifecycle, and HTTP security. The map now records
  Course, Question Library, delivery, and worker routes as downstream reconstruction work instead
  of listing deleted server modules.

- Replaced the retired Catalog helper and documentation vocabulary at the Question Search boundary.
  Saved Question Search and Question Search facet decoders now name their exact filters, facets,
  and Question Statistics availability; the authorization and Question Model contracts now name
  the actual browser-safe `QuestionSummary`, `QuestionSearchResult`, `QuestionSearchPage`, and
  `QuestionDetails` data objects and their serialized fields.

- Replaced the browser Course Theme Catalog with the exact Course Theme Registry boundary. The
  module, exported registry, route-scope consumers, focused theme checks, and design documentation
  now distinguish the closed Course Theme definitions from the global Question Library. The stale
  Course Instance curriculum URL was also removed from the theme-scope test and instructor
  documentation because Blueprint Course adoption is the global `/curriculum` workflow.

- Expanded the visible-endorsement reassignment row to cover its singular and plural outgoing
  labels. The Tier 1 audit counter uses the singular search form so one contextual review covers
  both forms; Question Star remains the canonical visible-endorsement relationship.

- Clarified that Question Star is the visible endorsement relationship, Question Folder owns
  private organization, and Saved Question Search owns stored search criteria. Ordinary English in
  owner guidance and authored Question content remains contextual prose rather than a product
  alias. Defined Stored Question Fixture Set and Pilot Question Set, then added the corresponding
  migration row so authored examples live in explicit data files and executable source owns
  behavior. The flat-question format document now links to its stored example instead of embedding
  a second complete Question record.

- Classified the focused Consumer, Adoption, Content Block, Fixture Corpus, Activity, Transport,
  Broker, Chapter, Type Variant, Runtime, Curriculum, Decoder, Curation, Private Question, payload,
  and HTTP vocabulary by its exact boundary. Added checklist rows for broad component and domain
  labels that need exact replacements, while retaining useful terms as documented technical or
  workflow vocabulary. Question Content Block now names the shared presentation primitive, and
  Question Curation remains an Instructor workflow while its durable records keep exact names.
  Recorded an open Course Assessment proposal that shares delivery machinery across Assignments,
  Quizzes, and Exams while keeping their teaching meanings and explicit policies.
