# Frequently asked questions

This page answers common orientation questions about Peptidyle's learning model,
question families, security boundaries, and local services. It links to the
authoritative contracts for readers who need implementation detail.

## Is PLE tied to one format?

No. PLE gives instructors one learning and assignment model while adapters
bring different question sources into it. Native flat-question JSON supports
multiple choice, multiple answer, fill-in-the-blank, multiple blanks, numerical
entry, matching, ordering, and image hotspots. The current external WeBWorK path supports the four
reviewed Chapter 1 MC/MATCH PGML sources; QTI, H5P, and iMathAS
each have their own documented runtime boundary. See
[QUESTION_MODEL.md](QUESTION_MODEL.md) and
[QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md).

## Does mastery end practice?

No. Mastery, scoring, continued practice, and variation are independent
assignment policies. An instructor can require mastery, keep the highest score,
allow unlimited practice after completion, and issue fresh parameter seeds for
each new run. A resumed attempt keeps its original seed so its question does not
change mid-attempt. See [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md).

## How does an exam differ?

An activity type gives instructors a teaching-intent starting point rather than
asking them to compose implementation policies. A mastery assignment gives
immediate full feedback, permits retries, and can offer fresh later practice.
An exam uses a controlled run, restricted feedback, and no continued practice.
PLE keeps the underlying completion, grading, variation, and feedback policies
separate so a course can use either activity honestly. See
[MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md).

## What runs in Solid and Wasm?

The Solid single-page application presents routes, input controls, progress, and
recovery states. Its one browser-safe Rust WebAssembly module generates allowed
parameters and validates response format. `src/wasm/index.ts` is the sole
browser import boundary for generated `wasm-bindgen` glue; components use its
typed facade rather than raw exports. See [FRONTEND_ARCHITECTURE.md](FRONTEND_ARCHITECTURE.md)
and [SOLID_MODEL.md](SOLID_MODEL.md).

## Why is grading server-only?

The browser may check response format, but it never receives answer keys,
grading implementations, or correctness decisions. Those live in
`crates/grading`, which is outside the WebAssembly dependency closure. The
server repeats format validation and then makes the authoritative grading
decision. If WebAssembly is unavailable, the browser uses a key-free server
format-validation route; it does not fall back to local grading. See
[CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) and
[QUESTION_MODEL.md](QUESTION_MODEL.md).

## Is PLE flat-question JSON QTI?

No. PLE flat-question JSON is the small, versioned, answer-bearing authoring
format for ordinary static questions. The native adapter compiles it into an
answer-free public question model and separate grader-only material. QTI is a
bounded import/export adapter and archival interchange format, so vendor XML
and QTI expression trees do not become PLE's internal schema. PLE flat JSON
version 2 is the closed native source contract for all eight families, including
`singleChoice`. A future QTI-JSONL format
would be an external adapter, not the internal source model. See
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) and
[flat_question_family_evolution_plan.md](active_plans/active/flat_question_family_evolution_plan.md).

## Can a student browser contact WeBWorK?

No. PLE is the sole WeBWorK client. The renderer is private; the browser continues to call PLE
through its same-origin gateway. Native-only test paths can omit it, but the supported normal local
launcher requires the private renderer image because it publishes the reviewed Chapter 1 WeBWorK
questions. The current
integration is limited to the four reviewed Genetics and Biochemistry Chapter 1 PGML sources: two
multiple-choice and two matching questions, with matching partial credit bound to each exact source
digest. Broader OPL
compatibility and unreviewed PG controls remain future work rather than being implied by that narrow
acceptance. See
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md) and
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md).

## Why PostgreSQL and a renderer?

They have separate jobs. PostgreSQL stores PLE-owned courses, assignments,
attempts, scores, and retention state under tenant row-level security. The
private external PG renderer evaluates a bounded WeBWorK PG question and has
no PLE database, learner credentials, persistent volume, or host-published
port. PLE remains the only assignment, roster, and grade system; WeBWorK2 and
MariaDB are not PLE runtime services. See
[DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) and
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).

## How do learners sign in?

PLE accounts use a stable opaque account ID. Email authentication is the
canonical sign-in and account-bootstrap path; passkeys are optional convenience
credentials, and multiple passkeys may be registered. An instructor gives a
learner a one-time course invitation link, which can be copied into an existing
trusted course channel or sent through configured SMTP. The implemented
passwordless and roster slice still has production acceptance work, so the
current status report is the source for what has been verified in a deployment.
See [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md).

## Is the live demo read-only?

No. The live demo uses the ordinary PLE application, authorization, database,
and storage paths. Visitors can create or change courses, assignments, roster
membership, submissions, grades, and other permitted records. Those changes
remain in the current disposable installation until it is regenerated;
regeneration restores the seeded baseline and discards the demo's disposable
state. See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md).

## Why does a submission identify an attempt?

A durable question attempt already binds the authenticated learner, course,
assignment, immutable question version, seed, timing state, and grading
backend. The browser therefore needs to send only that attempt's route identity,
an idempotency key, and the learner's answer. The planned presentation digest
and compact rendered-item IDs detect a stale or mismatched question display;
they are consistency checks, not authentication or grading proof. See
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

## Can a learner upload a file answer?

Not yet. The current browser widget and submission route fail closed because a
browser-supplied object key cannot prove tenant, learner, attempt, storage, or
inspection ownership. The planned capability creates one server-issued,
attempt-bound upload record and later accepts only that opaque upload ID. See
[secure_learner_file_upload_plan.md](active_plans/active/secure_learner_file_upload_plan.md).

## Where should a contributor record a durable decision?

Use [CONTRACTS.md](CONTRACTS.md) for frozen module and service boundaries, and
the focused durable document for the subject, such as
[OBJECT_STORAGE.md](OBJECT_STORAGE.md), [RETENTION_POLICY.md](RETENTION_POLICY.md),
or [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md).
Use the active plans for dependency order and unfinished work. The current
implementation handoff distinguishes accepted behavior from planned behavior in
[implementation_status.md](active_plans/implementation_status.md).
