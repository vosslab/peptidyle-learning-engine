# Frequently asked questions

This page answers common orientation questions about PLE's learning model, reusable Blueprint
Courses, private Course Instances, security boundaries, and local services. It links to the
authoritative contracts for implementation detail.

## What is a Blueprint Course?

A **Blueprint Course** is the one reusable course-level aggregate. It holds ordered modules,
assignments, reusable question selections, policies, and relative schedule defaults. Published
Blueprint Courses are visible and reusable by every vetted (approved) Instructor. Drafts are private
to their owning workspace and authorized collaborators. A Blueprint Course has no Students, live
deadlines, releases, accommodations, grades, or delivery settings.

## What is a Course Instance?

A **Course Instance** is the teaching and delivery aggregate created from exactly one Blueprint
Course. Its parent and applied Blueprint revision are immutable. It is private to its current equal
co-Instructors and enrolled Students, and owns enrollment, deadlines, releases, accommodations,
grades, and delivery settings. It is the only course type that receives learner work.

## How do I create a course?

Course creation chooses an existing published Blueprint Course or first creates a minimal new
Blueprint Course. The UI then creates the Course Instance with its own teaching title, term, and
IANA time zone. The instance receives reusable definitions and reviewed relative schedule offsets,
then resolves live dates against its term. Students, invitations, grades, and other delivery state
are never copied from another instance.

## How do Blueprint updates reach a Course Instance?

An Instructor publishes a new Blueprint revision explicitly. A new assignment in that revision
appears in each daughter Course Instance as **Unreleased**. The current equal co-Instructors review
the source revision, assignment manifest, question replacements, and resolved schedule, then use
**Prepare update proposal** and **Apply proposal**. Propagation never silently releases an assignment
or overwrites instance-owned delivery changes. Divergent work uses an explicit selected copy or new
assignment action; PLE does not perform an implicit merge.

## What are fork, publish, rollover, and term shift?

- **Fork Blueprint** creates an independently editable Blueprint with immutable source-lineage
  evidence and no live tether.
- **Publish Blueprint** makes a reviewed draft revision reusable by all vetted Instructors.
- **Rollover Course Instance** creates a new teaching instance for a target term without Students,
  invitations, attempts, responses, grades, retention state, or issued evidence.
- **Shift Course Instance term** changes an existing instance's unissued schedules after a full
  preview. Every relative date resolves in the target IANA time zone; DST gaps and ambiguities need
  correction. An instance with issued learner work uses rollover instead.

## Is PLE tied to one format?

No. PLE gives Instructors one learning and assignment model while adapters bring different question
sources into it. Native flat-question JSON supports multiple choice, multiple answer,
fill-in-the-blank, multiple blanks, numerical entry, matching, ordering, and image hotspots. The
current external WeBWorK path supports the four reviewed Chapter 1 MC/MATCH PGML sources; QTI, H5P,
and iMathAS each have their own documented runtime boundary. See [QUESTION_MODEL.md](QUESTION_MODEL.md)
and [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md).

## Does mastery end practice?

No. Mastery, scoring, continued practice, and variation are independent assignment policies. An
Instructor can require mastery, keep the highest score, allow unlimited practice after completion,
and issue fresh parameter seeds for each new run. A resumed attempt keeps its original seed so its
question does not change mid-attempt. See [ACTIVITY_MODEL.md](ACTIVITY_MODEL.md).

## How does an exam differ?

An activity type gives Instructors a teaching-intent starting point rather than asking them to
compose implementation policies. A mastery assignment gives immediate full feedback, permits
retries, and can offer fresh later practice. An exam uses a controlled run, restricted feedback, and
no continued practice. PLE keeps completion, grading, variation, and feedback policies separate so
a Course Instance can use either activity honestly. See [MASTERY_ASSIGNMENT_DESIGN.md](MASTERY_ASSIGNMENT_DESIGN.md).

## What runs in Solid and Wasm?

The Solid single-page application presents routes, input controls, progress, and recovery states.
Its one browser-safe Rust WebAssembly module generates allowed parameters and validates response
format. `src/wasm/index.ts` is the sole browser import boundary for generated `wasm-bindgen` glue;
components use its typed facade rather than raw exports. See [FRONTEND_ARCHITECTURE.md](FRONTEND_ARCHITECTURE.md)
and [SOLID_MODEL.md](SOLID_MODEL.md).

## Why is grading server-only?

The browser may check response format, but it never receives answer keys, grading implementations,
or correctness decisions. Those live in `crates/grading`, outside the WebAssembly dependency
closure. The server repeats format validation and makes the authoritative grading decision. If
WebAssembly is unavailable, the browser uses a key-free server format-validation route; it does not
fall back to local grading. A Student submission is graded on the server, and an authorized
Instructor Gradebook reads the resulting server-owned record. See [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md)
and [QUESTION_MODEL.md](QUESTION_MODEL.md).

## Is PLE flat-question JSON QTI?

No. PLE flat-question JSON is the small, versioned, answer-bearing authoring format for ordinary
static questions. The native adapter compiles it into an answer-free public question model and
separate grader-only material. QTI is a bounded import/export adapter and archival interchange
format, so vendor XML and QTI expression trees do not become PLE's internal schema. See
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) and the current
[implementation_plan.md](active_plans/implementation_plan.md).

## Can a Student browser contact WeBWorK?

No. PLE is the sole WeBWorK client. The renderer is private; the browser calls PLE through its
same-origin gateway. The current integration is limited to the four reviewed Genetics and
Biochemistry Chapter 1 PGML sources: two multiple-choice and two matching questions, with matching
partial credit bound to each exact source digest. Broader compatibility and unreviewed PG controls
remain future work. See [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md) and
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md).

## Why PostgreSQL and a renderer?

They have separate jobs. PostgreSQL stores PLE-owned accounts, Blueprint Courses, Course Instances,
assignments, attempts, scores, and retention state under exact relationship authorization and row-
level security. The private external PG renderer evaluates a bounded WeBWorK question and has no PLE
database, learner credentials, persistent volume, or host-published port. PLE remains the only
assignment, roster, and Gradebook system. See [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) and
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).

## How do learners sign in?

PLE Accounts use stable opaque Account IDs and one immutable Product Role. Sysadmin provisioning
creates an Account; email-code authentication restores an existing Account, and passkeys are
optional additional credentials for that same Account. The ordinary email-code and passkey browser
adapters are being reconstructed on the canonical Authenticated Session foundation. The current
Live Demo uses its visible seeded Account selector. An Instructor can create a one-time Course
Invitation link for an existing Account through a trusted course channel or configured SMTP. The
implementation-status registry is the source for what has been verified in a deployment. See
[ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md) and
[implementation status](active_plans/implementation_status.md).

## Is PLE ready for production?

Not yet. PLE is still pre-production. The live demo is a functional, disposable installation, but
it is not release acceptance. `WP-INST-G1` is accepted; `WP-INST-G2` remains acceptance-open behind
the current naming and visual/documentation close-out, and `WP-RC8` remains acceptance-open for its
provider, mailbox, passkey, multi-replica, security, HCI, and release gates. See
[implementation_status.md](active_plans/implementation_status.md) and [release completion plan](active_plans/active/release_completion_plan.md).

## Is the live demo read-only?

No. The live demo uses the ordinary PLE application, authorization, database, and storage paths.
Visitors can create or change Course Instances, assignments, roster membership, submissions, grades,
and other permitted records. Those changes remain in the current disposable installation until it
is regenerated; regeneration restores the seeded baseline and discards disposable state. See
[LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md).

## Are live-demo roles isolated from one another?

No. Seeded personas use ordinary accounts, memberships, courses, and records in the same installation.
An Instructor's Course Instance is private to its current equal co-Instructors and enrolled Students;
the data is disposable because the installation can be regenerated, not because each role has a
private sandbox. See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) and [USER_ROLES.md](USER_ROLES.md).

## Is Student view the same as entering as a Student?

No. **Student view** is an answer-free, no-store inspection of the current assignment that retains
the Instructor session and creates no learner work. Ordinary Student entry uses the enrolled
Student's authority and can create a run, attempt, submission, score, and Gradebook evidence. Use
Student view to inspect delivery; use Student entry to exercise graded work. See
[ACTIVITY_MODEL.md](ACTIVITY_MODEL.md) and [API_CONTRACTS.md](API_CONTRACTS.md).

## Why does ADAPT appear in historical discussions?

ADAPT is prior art, not a PLE product model. Its documentation used the term **Alpha course** for a
shared reusable course tree. PLE learned from that surface while adopting the single canonical
**Blueprint Course** aggregate. PLE defines no Alpha type, route, Store, schema branch, or
compatibility alias. See [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md).

## Can I reuse a question or assignment?

Yes, but reuse is explicit and versioned. Select a published question by its human-readable Question
ID, reuse an assignment's ordered questions, or draw from a reusable pool. A published Blueprint
Course can supply ordered modules and assignments to a new Course Instance through the adoption
workflow. Existing issued learner runs keep their immutable question snapshot. See
[LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) and [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md).

## What happens if automated grading stalls?

The learner submits once and receives **Response received** while the server keeps the accepted
response private. **Check grading status** is an answer-free read, and the normal path proceeds to
feedback and **View completed run** without Instructor intervention. If the status says **Your
response needs instructor attention**, an authorized Instructor reviews **Grading operations** and
chooses the currently enabled retry action when the operation is eligible. After **Your completed run
is recorded.**, confirm the current result in **Gradebook**. The browser never receives an answer,
grading internals, or a hidden key. See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md),
[FAILURE_RECOVERY.md](FAILURE_RECOVERY.md), and [implementation_status.md](active_plans/implementation_status.md).

## Can the browser or another Student see answer keys?

No. Public question and learner projections are answer-free. Answer keys, private source material,
grading rules, and provider credentials remain on the server; exact relationship authorization also
restricts educational records. See [SECURITY_MODEL.md](SECURITY_MODEL.md) and [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md).

## Does the demo role selector grant a role?

No. It replaces only the normal identity-verification ceremony. The server resolves the selected
seeded account and derives the ordinary session, course membership, role, and authorization from live
PLE state. After entry, the browser uses the same application and authorization paths as any other
session. See [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) and [AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md).

## Why does a submission identify an attempt?

A durable question attempt binds the authenticated learner, Course Instance, assignment, immutable
question version, seed, timing state, and grading backend. The browser therefore sends only that
attempt's route identity, an idempotency key, and the learner's answer. Presentation digests and
compact rendered-item IDs detect a stale or mismatched display; they are consistency checks, not
authentication or grading proof. See [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).

## Can a Student upload a file answer?

Not yet. The current browser widget and submission route fail closed because a browser-supplied object
key cannot prove course, learner, attempt, storage, or inspection ownership. The planned capability
creates one server-issued, attempt-bound upload record and later accepts only that opaque upload ID.
See [DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md).

## Where should a contributor record a durable decision?

Use [CONTRACTS.md](CONTRACTS.md) for frozen module and service boundaries, and the focused durable
document for the subject, such as [OBJECT_STORAGE.md](OBJECT_STORAGE.md), [RETENTION_POLICY.md](RETENTION_POLICY.md),
or [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](NO_MOUSE_ACCESSIBILITY_CONTRACT.md). Use active plans for
dependency order and unfinished work. The current implementation handoff distinguishes accepted
behavior from planned behavior in [implementation_status.md](active_plans/implementation_status.md).
