# Question and Assessment changes

Temporary working report for the corpus-wide Human Guidance compliance pass.

## Questions and Pools

- Draft Questions are private, mutable, unpublished current state. Saving replaces working state;
  cleanup requires warning and a recovery period.
- Published Questions use one canonical title, stable `AAAA-ZBBB` identity, and immutable source
  Revisions. Search metadata changes do not create a Revision.
- Question Pools are published reusable objects with stable IDs and immutable Pool Revisions. Import
  into an Assessment forks the Pool; each new Attempt makes fresh Pool selections and retains them.
- Question Type is immutable author-declared educational metadata on a Published Question Revision;
  it is never inferred from backend controls.
- The Question Library is global and subject agnostic. Search, Browse, dense filtering, and bulk
  metadata work support very large collections.
- Question Stars are visible endorsements; Watches are private subscriptions. Revision-specific,
  privacy-safe aggregate statistics may survive Student-record deletion.

## Question Backend boundary

- Every Question Backend owns rendering, interaction, response interpretation, grading, feedback,
  and backend-specific state. PLE owns authorization, IDs, Revision selection, persistence,
  Assessment workflow, outcomes, and retention.
- Native PLE Question JSON is private, unpublished, unversioned, static, strictly validated,
  supports all eight named Types, and receives no random seed. Author JavaScript is isolated,
  untrusted, and never grading authority.
- WeBWorK remains opaque. H5P and iMathAS are supported secondary backends; their current incomplete
  adapters do not redefine the product target.

## Assessments and Attempts

- Blueprint and Course Instance Assessments use the same five Assessment Types. Assessment Templates
  contain settings only and no Questions or Pools.
- Course Instance Assessments begin Unreleased. Automated, interactive Release Validation covers
  Questions, point values, settings, date order, a due date at least 24 hours ahead, and the Course
  Instance's six-month Active limit.
- Complete Question responses save and remain editable while the Attempt is open. Incomplete input
  remains unsaved for product purposes. Submitting the whole Assessment Attempt finalizes all saved
  responses together.
- A backend may evaluate a complete saved response early when its interaction requires it, but the
  Student sees no grading outcome until whole-Assessment submission.
- When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
- Student submission and deadline submission use the same whole-Assessment boundary. Student
  interaction checks expiration, and background processing ensures an expired Attempt is submitted
  even after the Student leaves.
- Practice Question Assignments use the same submission boundary and show the correct answer
  immediately after submission. Optional Question Feedback is shown when the Question Backend
  provides it and does not use Assessment correct-answer disclosure settings.
- A Question without a complete saved response remains visibly unanswered, receives zero credit,
  counts as incorrect, and is not sent to the Question Backend.
- The backend returns an immutable credit fraction. PLE stores it and derives scores from current
  Question point values without regrading.
- The highest submitted Assessment Attempt score is used. PLE has no separate Question weights,
  Grade Categories, weighted categories, Course Grade Scheme, or Course percentage calculation.
  Pilot grade export is CSV or TSV point data.
- Regular Assignment defaults to unlimited Attempts; due-date submission/start limits and rejected
  late work are the defaults. Disclosure remains separately configurable.

All Question, Assessment, scoring, and Pool Revision issues found by this pass are resolved in Human
Guidance. The remaining product decisions are listed in
[UNRESOLVED_OR_AMBIGUOUS_ITEMS.md](UNRESOLVED_OR_AMBIGUOUS_ITEMS.md).
