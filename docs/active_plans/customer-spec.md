> **Historical discovery input, not current instructions.** Current authority is
> [implementation_plan.md](implementation_plan.md),
> [release_completion_plan.md](active/release_completion_plan.md), and
> [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md). The M0 result is concluded evidence.

# Peptidyle Learning Engine

Peptidyle Learning Engine is a backend-agnostic assignment and assessment platform built around repeated attempts, algorithmic questions, and question-level timing.

The platform does not own a single question format. It provides a common execution layer for multiple question backends, including WeBWorK, MyMathWorks, H5P, and QTI question pools.

## Core behavior

The default assignment mode is mastery-based.

A student opens an assignment and continues working until:

1. every question is correct, or
2. the student gives up or reaches an instructor-defined stopping condition.

Questions are usually algorithmic. A new attempt may generate different values while testing the same concept.

The instructor may configure a time limit for each question, such as 90 seconds. The timer applies to the active question rather than the entire assignment.

The platform also supports:

- single-attempt quizzes
- optional whole-quiz time limits
- exam-compatible question pools
- DOCX and PDF exam export
- multiple question engines and content formats

## Proposed architecture

The codebase should separate the platform into four main layers:

1. TypeScript application layer
2. Rust domain and execution layer
3. WebAssembly client runtime
4. Backend adapters and external services

## TypeScript application layer

TypeScript handles the web application, user interface, API routing, and platform integration.

Likely responsibilities:

- instructor interface
- student assignment interface
- assignment editor
- question library browser
- timer display
- session state
- authentication
- authorization
- LMS integration
- analytics views
- API client code
- DOCX and PDF export requests
- adapter orchestration

A possible stack could include:

- TypeScript
- React, Svelte, or another component framework
- Node.js or a TypeScript server framework
- PostgreSQL
- REST or typed RPC APIs
- Web Workers for isolated question execution

The TypeScript layer should not contain the core grading rules. TypeScript should call the Rust domain layer so the same behavior is used on the client and server.

## Rust domain layer

Rust should contain the rules that must remain deterministic, secure, and portable.

Likely responsibilities:

- assignment state transitions
- attempt validation
- mastery completion rules
- question timing rules
- scoring
- retry policies
- random seed handling
- algorithmic parameter generation
- response normalization
- grading result normalization
- question backend capability checks
- import and export validation
- audit event generation

The Rust code should be usable in two forms:

- as a native server library
- as a WebAssembly module in the browser

The Rust domain layer should not depend directly on a web framework or database.

## WebAssembly runtime

The WebAssembly module provides fast and consistent execution in the browser.

Possible uses:

- question parameter generation
- answer preprocessing
- local response validation
- timer state validation
- assignment state transitions
- offline-safe state preparation
- previewing algorithmic questions
- rendering backend-neutral question data

The server remains authoritative for grading and completion.

The browser may perform preliminary validation, but the server must reproduce the same operation before accepting a result.

## Backend-neutral question model

All question engines should map into a shared internal representation.

For example:

```ts
interface QuestionDefinition {
	id: string
	source: QuestionSource
	version: string
	prompt: QuestionContent
	response: ResponseDefinition
	attemptPolicy: AttemptPolicy
	timingPolicy: TimingPolicy
	randomization?: RandomizationDefinition
	grading: GradingDefinition
	metadata: QuestionMetadata
}
```

The internal model should represent platform behavior without requiring every backend to support every feature.

Each backend adapter should publish its capabilities.

```ts
interface BackendCapabilities {
	algorithmicGeneration: boolean
	clientRendering: boolean
	serverGrading: boolean
	partialCredit: boolean
	hints: boolean
	perQuestionTiming: boolean
	printExport: boolean
	offlinePreview: boolean
}
```

The platform can then reject unsupported assignment configurations before publication.

## Backend adapter interface

Each question system should implement a common adapter boundary.

```ts
interface QuestionBackendAdapter {
	getCapabilities(): BackendCapabilities

	loadQuestion(
		reference: ExternalQuestionReference
	): Promise<QuestionDefinition>

	createAttempt(
		question: QuestionDefinition,
		context: AttemptContext
	): Promise<QuestionAttempt>

	gradeAttempt(
		attempt: QuestionAttempt,
		response: StudentResponse
	): Promise<GradeResult>

	exportQuestion?(
		question: QuestionDefinition,
		format: ExportFormat
	): Promise<ExportedQuestion>
}
```

Initial adapters may include:

- WeBWorK adapter
- MyMathWorks adapter
- H5P adapter
- QTI adapter

Adapters may run locally, call a remote service, or delegate to another execution environment.

## Assignment model

An assignment contains question references and policy definitions.

```ts
interface Assignment {
	id: string
	title: string
	mode: AssignmentMode
	questions: AssignmentQuestion[]
	completionPolicy: CompletionPolicy
	timingPolicy: TimingPolicy
	attemptPolicy: AttemptPolicy
	availability: AvailabilityPolicy
}
```

Primary modes:

```ts
type AssignmentMode =
	| "mastery"
	| "quiz"
	| "exam"
	| "practice"
```

### Mastery mode

- multiple attempts
- completion requires all questions correct
- questions may regenerate parameters
- optional question-level timer
- instructor may permit giving up
- scoring may use completion, attempts, time, or another policy

### Quiz mode

- single attempt or limited attempts
- optional whole-quiz timer
- fixed or randomized question selection
- results may be delayed until submission

### Exam mode

- secure delivery where supported
- fixed attempt rules
- optional question and exam timers
- printable export through DOCX or PDF

## Attempt state machine

The attempt lifecycle should be explicit.

```text
not_started
    |
    v
active
    |
    +--> submitted
    |        |
    |        +--> correct
    |        |
    |        +--> incorrect
    |                 |
    |                 +--> retry_available
    |                 |
    |                 +--> exhausted
    |
    +--> timed_out
    |
    +--> abandoned
```

Assignment completion should be derived from question states rather than stored as an unrelated boolean.

## Timing model

Timers need server-authoritative timestamps.

Each timed question attempt should include:

- issued timestamp
- expiration timestamp
- submission timestamp
- server clock source
- optional pause policy
- optional grace period
- audit data

The browser timer is only a display.

The server decides whether the response arrived before expiration.

## Randomization and reproducibility

Algorithmic questions require reproducible generation.

Each attempt should store:

- generator identifier
- generator version
- random seed
- generated parameters
- backend version
- rendered question hash

This allows the platform to reproduce the exact question shown to a student.

## Security model

Secure question content should remain unavailable to students outside an active attempt.

The system should separate:

- instructor-visible question definitions
- student-visible rendered questions
- grading logic
- answer keys
- generation logic
- export permissions

Sensitive grading data should remain on the server unless a backend requires client-side execution.

Client-delivered WebAssembly should not be treated as secret code.

## Persistence

The database will likely need entities for:

- users
- organizations
- courses
- enrollments
- assignments
- question references
- backend configurations
- question versions
- assignment attempts
- question attempts
- submissions
- grade results
- timers
- exports
- audit events

External questions should be referenced by stable source identifiers and version metadata.

The platform should avoid copying proprietary question content unless the source license and integration require local storage.

## API boundaries

Suggested service boundaries:

```text
/auth
/courses
/assignments
/questions
/attempts
/submissions
/grading
/backends
/exports
/analytics
```

Important API operations include:

- create assignment
- validate assignment configuration
- begin assignment attempt
- issue question attempt
- submit response
- grade response
- request retry
- give up on question
- complete assignment
- export exam
- retrieve instructor analytics

## DOCX and PDF export

Exam export should use a backend-neutral print representation.

Each adapter should convert supported questions into:

- prompt blocks
- figures
- answer spaces
- instructor answer key
- solution metadata
- accessibility text

The export service then generates:

- student DOCX
- student PDF
- answer-key DOCX
- answer-key PDF

Questions that cannot be exported should be flagged before the instructor builds the exam.

## Repository layout

A possible monorepo structure:

```text
peptidyle/
	apps/
		web/
		api/
		worker/
	packages/
		ui/
		api-client/
		question-model/
		backend-sdk/
		export-model/
	crates/
		domain/
		grading/
		timing/
		randomization/
		question-model/
		wasm/
		adapters/
			webwork/
			mymathworks/
			h5p/
			qti/
	services/
		export/
		analytics/
	schemas/
		database/
		qti/
		h5p/
	docs/
		architecture/
		adapter-development/
		question-model/
```

## Implementation priorities

The first useful implementation should prove the shared execution model rather than support every backend.

A reasonable initial slice is:

1. shared question model
2. mastery assignment state machine
3. question-level timing
4. deterministic algorithmic question generation
5. one backend adapter
6. student assignment interface
7. instructor assignment editor
8. server-side grading validation
9. basic DOCX and PDF export
10. adapter capability validation

The central technical goal is to keep assignment behavior independent from the question backend.

The platform should be able to answer:

> Can this question backend support this assignment policy?

before an instructor publishes an assignment.
