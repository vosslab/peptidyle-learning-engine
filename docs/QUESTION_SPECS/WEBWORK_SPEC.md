# WeBWorK Question specification

## Purpose and source

`webwork` is a Question Backend for canonical WeBWorK PG or PGML source. PG and PGML remain
separate classifications: call a Question PGML only when all of its source is PGML-compliant;
otherwise call it PG. One parameterized source is one Published Question, regardless of variants.

## Execution contract

PLE sends exact trusted source, exact Published Question Revision, and issued randomization state
to a private WeBWorK renderer. The renderer returns opaque presentation and state. It alone
interprets submitted name/value pairs and grading. PLE hosts that interaction and passes responses
through the Backend interface.

Question Type is editable classification metadata. Assign it manually for now; automatic
PG/PGML Type detection is deferred. See [QUESTION_TYPE_SPEC.md](QUESTION_TYPE_SPEC.md).
The renderer continues to own interaction and grading regardless of the classification tag.

The browser receives an authorized backend-owned document in PLE's application frame. It submits a
bounded canonical ordered list of UTF-8 name/value pairs, preserving order, repeated names, and
legitimate hidden fields. The raw response is limited to 64 KiB. Renderer credentials, source
bytes, private renderer URLs, and grading results remain server-side.

## Student work and teaching material

The renderer returns the immutable credit fraction required by the common Backend contract. PLE
stores it and recalculates Assessment points when point values change without another renderer
call. Backend feedback is transient unless the Backend provides a robust way for PLE to
preserve it, as HG specifies. PLE does not extract feedback or answer logic from PG/PGML.

PLE-managed Hints, Question Feedback, and Worked Solutions may be attached independently. Correct
answer review, if authorized by Assessment visibility, uses a server-only operation with retained
source and Attempt seed. It accepts no Student response and returns transient backend material. A
rendering failure leaves recorded work and grades intact; a document retry does not grade again.

The review document is a current authorized rendering of retained source and seed. It does not
alter issued presentation evidence, submitted responses, stored credit, or Student Work. Waiting
for all Students to complete a Quiz or Exam is an Assessment policy choice, not a WeBWorK rule.

Current renderer protocol and implementation details are documented separately in
[WEBWORK_PG_RENDERER_API_USAGE.md](../WEBWORK_PG_RENDERER_API_USAGE.md). Their route names, browser
markers, and HTTP headers are implementation evidence, not additional Question product rules.

## Import and revisions

Instructors can author and test Questions in Drafts before publication, as defined in
[DRAFT_QUESTION_SPEC.md](DRAFT_QUESTION_SPEC.md). The WeBWorK renderer and evaluator provide the
PG/PGML preview and response checking within that same workflow. Source import is an additional
way to supply content, not a prerequisite for authoring.

An import preserves source and PG versus PGML in a Draft, including unfinished or broken source.
Drafts have no content or metadata requirements. Publication checks the source, Question Type,
and required Library metadata. A defective WeBWorK Question
affecting issued work is addressed by setting its Assessment point value to zero. A source
correction requires a new Revision and does not rewrite stored grading outcomes.
