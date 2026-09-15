# Question Backend contracts

This document defines the execution boundary required by
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). Current code links show implementation
evidence. They do not authorize a second PLE-native parser for backend-owned
controls or an additional grading lifecycle.

## Ownership rule

Every Question Backend owns:

- rendering and the interaction it presents;
- response interpretation;
- grading and partial-credit calculation;
- Question feedback and correct-answer material; and
- backend-specific state needed to preserve the Question across saves and an
  Assessment Attempt.

PLE owns:

- Account, Course, Student, Assessment, and Attempt authorization;
- Published Question and exact Revision selection;
- Question Pool selection evidence;
- response-saving and whole-Assessment submission;
- storage of the backend's immutable credit fraction;
- score calculation from current Assessment Question point values;
- feedback-disclosure policy; and
- FERPA retention.

PLE treats a backend's presentation and state as opaque. It must not inspect
HTML controls, hidden inputs, form names, or output structure to reproduce the
backend's behavior. Question Type is author-declared metadata for discovery and
labels; PLE does not infer it from rendered controls.

## Common operations

| Operation | Contract |
| --- | --- |
| Validate | The backend validates its complete source and reports supported behavior. |
| Render | PLE supplies trusted server-derived source, exact Revision, and randomization state; the backend returns an answer-free presentation plus opaque state. |
| Save response | PLE stores a complete typed native response or bounded opaque backend response while the Attempt is open. |
| Evaluate | The backend interprets the exact response and state and returns an immutable credit fraction plus protected feedback. |
| Submit Assessment | PLE submits the whole Attempt and finalizes all saved responses together as Student Work. |
| Disclose | PLE exposes only the result and feedback allowed by Assessment policy. |
| Fail | Unsupported, invalid, or unavailable backend work does not become an incorrect Student response. |

A complete response may be evaluated before the whole Assessment Attempt is
submitted, but saving it creates no Student-visible grading outcome. PLE stores
the backend's credit fraction without reinterpretation. A point-value change
recalculates the score and never regrades the response.

PLE has no public grading job, pending-result lifecycle, Retry action, regrading
operation, result replacement, or generic grading receipt. When PLE requests a
grading outcome, the Question Backend returns it without a deferred grading
state. If it cannot return the credit fraction, that processing does not
complete.

## Browser boundary

The browser receives only an answer-free native presentation or an authorized
backend-owned document. It never receives private source, Answer Keys, rubrics,
backend credentials, provider tokens, raw provider results, or server-only
state.

The browser sends only the response required by the selected backend. It cannot
choose the backend, source, Revision, seed, correct answer, credit fraction, or
score. PLE rederives the exact binding from the authenticated Attempt and
Question position.

## Native PLE Question JSON

Native PLE Question JSON is private, unpublished, unversioned, and static. It
supports exactly the eight current Question types:

- multiple choice;
- multiple answer;
- fill in the blank;
- multiple blank;
- numerical;
- matching;
- ordering; and
- hotspot.

The native backend strictly validates the complete source, produces the native
controls, validates the native response, grades on the server, and supplies
feedback. Author-provided JavaScript runs only in an isolated untrusted
environment and cannot become grading or authorization authority.

Native PLE Question JSON receives no random seed. Answer-choice randomization
is authored native Question behavior. Cache entries, when used, contain only
answer-free output and are never the authority for a response, grade, or Student
relationship.

## QTI import

QTI is an import pathway, not a runtime Question Backend. An authorized
Instructor imports a bounded archive into a private workspace. Supported items
become complete native Draft Questions and then use the normal Draft,
publication, Assessment, Attempt, response, and evaluation boundaries.
Unsupported items are reported rather than approximated into different
Questions.

The original archive and mapping may remain private import evidence. They do not
create another runtime object model.

## WeBWorK

PLE sends the exact trusted PG/PGML source and randomization state to the private
WeBWorK renderer. The renderer returns an opaque presentation plus opaque state,
and it alone interprets the submitted form values and grading semantics.

PLE hosts the authorized document in its application frame and captures the
form as a bounded canonical ordered list of name/value pairs. It preserves
order, repeated names, and legitimate hidden fields. PLE does not classify the
controls, rewrite them into native Question controls, or infer Question Type.

The shared raw UTF-8 response bound is 64 KiB. It is separate from PG/PGML
source, renderer document/envelope, and asset limits. Renderer credentials,
private URLs, source bytes, and grading results remain server-only.

The current E1 WeBWorK design is stateless across render and grade beyond the
opaque presentation/response evidence PLE must retain. There is no PLE replay
mapping or control-specific compatibility layer. See
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).

## Other backends

iMathAS is a possible secondary Question Backend and, if delivered, uses this
same common boundary. Any backend-specific session, signature, callback, or
token remains server-only, scoped to the exact Student, Course, Attempt,
Question Revision, and backend operation. It does not create a parallel PLE
Account, Assessment, Attempt, or grading model. No backend is presented as
available until its real composition and connected acceptance are complete.

H5P is not a current PLE Question Backend. Its placeholder schema, adapter,
DTO, API, and workspace seams were removed. Future H5P delivery is blocked
until Human Guidance's recorded product question fixes the first supported
content type and exact pinned library versions, authoritative terminal xAPI
event/score semantics, and whether scoreless activities are non-assessment
only. A later approved H5P backend must satisfy this document's ownership and
evidence rules; it does not inherit a dormant implementation claim.

## Retained evidence

PLE retains only the exact source or Revision reference, Question/Pool selection,
randomization value or opaque backend state, response, immutable credit
fraction, and protected feedback needed to interpret Student Work.

Current implementation structures such as `QuestionAttemptReproductionDetails`
may carry part of that evidence. Their current fields are not a Human Guidance
requirement for software-version snapshots, rendered-page archives, public
receipts, or a generalized replay service.

## Extension checklist

Before adding a backend:

1. define its complete private source and immutable published identity;
2. keep secrets and Answer Keys out of browser shapes and logs;
3. return an answer-free native or opaque presentation;
4. define the exact typed or opaque response and its bounds;
5. preserve the state needed to interpret that response without PLE parsing
   backend controls;
6. return one immutable credit fraction through server authority;
7. declare only capabilities proved by the configured implementation; and
8. test authorization, failure, whole-Assessment submission, and disclosure at
   the real boundary.

Primary implementation areas include `crates/question_model`,
`crates/adapters`, `crates/server`, and the PostgreSQL access layer. Their
current identifiers do not override this product contract.
