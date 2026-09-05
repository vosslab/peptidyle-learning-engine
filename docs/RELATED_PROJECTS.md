# Related projects

Peptidyle Learning Engine (PLE) helps biology instructors deliver reusable, varied practice with
server-owned automated grading. The projects and resources below serve the same instructors,
assessment authors, or learning-platform operators through the same workflow or a closely adjacent
one. Current PLE adapter scope remains in [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md), and
durable release direction remains in [ROADMAP.md](ROADMAP.md).

## Confirmed related projects

### LibreTexts ADAPT

- Relationship: Prior art or inspiration
- Link: https://adapt.libretexts.org/
- Why visitors may care: instructors can compare another course, assignment, question-finding,
  grading, and gradebook workflow designed around reusable open assessment content.
- Evidence: LibreTexts' [official instructor guide](https://chem.libretexts.org/Courses/Remixer_University/Mastering_ADAPT%3A_A_User%27s_Guide/04%3A_Using_ADAPT_as_an_Instructor)
  documents course creation, assignment types, question finding, question editing, submissions, and
  gradebooks. Its [assignment guide](https://chem.libretexts.org/Courses/Remixer_University/Mastering_ADAPT:_A_User's_Guide/04:_Using_ADAPT_as_an_Instructor/4.05:_Creating_Assignments)
  records assignment import choices for "Properties and Questions" or "Just the Properties," and
  its [question-view guide](https://chem.libretexts.org/Courses/Remixer_University/Mastering_ADAPT:_A_User's_Guide/04:_Using_ADAPT_as_an_Instructor/4.07:_Assignments-_Adding,_Removing,_Reordering,_and_Viewing_Questions)
  documents the Instructor/Student view toggle. The [student course guide](https://chem.libretexts.org/Courses/Remixer_University/Mastering_ADAPT:_A_User's_Guide/06:_Using_ADAPT_as_a_Student/6.02:_New_Page)
  documents opening an assignment by its title. PLE records ADAPT as explicit product prior art in
  [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) and
  [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md).
- Implementation evidence: the ignored, read-only ADAPT snapshot separates assignment `questions`
  and `properties` routes in `OTHER_REPOS/adapt/resources/js/router/routes.js`; the official guide
  supplies the corresponding user-facing assignment and question workflows.
- Comparison evidence: PLE adopts the useful navigation shape in its
  [Instructor page map](INSTRUCTOR_PAGE_VISUALS.md): an assignment title opens an assignment-local
  Instructor home, Questions owns question and pool authoring, Policies (the PLE name for delivery
  properties) owns delivery rules, and Student view exposes the current student landing.
- Boundary and advantage: PLE's [live-demo specification](LIVE_DEMO_SPEC.md) currently exposes
  only seeded Account session entry, not a live Student workflow. Its retained
  [adapter contract](ADAPTER_DEVELOPMENT.md) keeps answer keys and grading decisions server-only
  and defines an answer-free browser contract; its
  [test-evidence model](TEST_EVIDENCE_MODEL.md) requires restored browser evidence before teaching
  workflows can be claimed. These are PLE design boundaries, not claims about ADAPT parity.
- Provenance: Confirmed by the explicit PLE prior-art records and the authoritative ADAPT guides
  checked on 2026-08-28. The ignored `OTHER_REPOS/adapt` checkout is read-only corroboration for
  route-level structure, not a PLE dependency or current-upstream compatibility claim.

### WeBWorK2

- Relationship: Prior art or inspiration
- Link: https://openwebwork.org/
- Why visitors may care: instructors can explore a mature open homework application for math and
  science courses, including course management and a large community problem library.
- Evidence: the [official WeBWorK project](https://openwebwork.org/) describes an open-source
  online homework system for STEM courses and its Open Problem Library; the
  [Open Problem Library guide](https://wiki.openwebwork.org/wiki/Open_Problem_Library) documents
  contributed, reviewed, and taxonomy-browsable problems. PLE's
  [question content philosophy](HUMAN_GUIDANCE.md#question-content-philosophy) records the
  deterministic, server-owned question model that bounds this prior-art comparison.
- Notes: PLE owns its own courses, rosters, assignments, attempts, and gradebook; WeBWorK2 is not a
  second application inside the PLE runtime.

### WeBWorK PG

- Relationship: Upstream source, fork, or successor
- Link: https://github.com/openwebwork/pg
- Why visitors may care: problem authors can inspect the PG and PGML language that powers
  algorithmic WeBWorK questions, rendering, randomization, and grading.
- Evidence: the OpenWeBWorK organization identifies PG as its problem-rendering engine, and PLE's
  [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md) documents PG behind the
  private renderer boundary.
- Notes: PG supplies question-engine behavior; PLE retains course, student, attempt, and gradebook
  ownership.

### WeBWorK Open Problem Library

- Relationship: Companion project, extension, or interoperability tool
- Link: https://github.com/openwebwork/webwork-open-problem-library
- Why visitors may care: problem authors can browse and reuse a large open library of algorithmic
  WeBWorK questions when building science and mathematics practice.
- Evidence: the [official OPL guide](https://wiki.openwebwork.org/wiki/Open_Problem_Library) describes
  reviewed contributions, taxonomy browsing, and installation for the WeBWorK Library Browser.
- Notes: the library is a WeBWorK content resource; PLE's accepted renderer profile remains bounded
  to its reviewed Chapter 1 sources.

### webwork-pg-renderer

- Relationship: Companion project, extension, or interoperability tool
- Link: https://github.com/vosslab/webwork-pg-renderer
- Why visitors may care: operators can run the standalone PG renderer that PLE uses for its bounded
  server-side WeBWorK render-and-grade path.
- Evidence: the project identifies itself as a WeBWorK standalone problem renderer, and PLE's
  [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) documents it as the private stateless
  renderer service in the production-shaped stack.
- Notes: PLE consumes the declared image and private HTTP contract rather than importing or loading
  a sibling checkout.

### iMathAS

- Relationship: Companion project, extension, or interoperability tool
- Link: https://www.imathas.com/
- Why visitors may care: instructors can explore another open system for algorithmically generated,
  automatically graded homework and tests with a full gradebook.
- Evidence: the [official iMathAS site](https://www.imathas.com/) documents algorithmically generated
  questions, computer grading, learning management, and a full gradebook; PLE's
  [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md#current-adapter-posture) records its reviewed
  self-hosted iMathAS Question Backend boundary.
- Notes: PLE retains the iMathAS Question Backend Session, verifies each iMathAS Result server-side,
  and gives the browser no authority to supply a score.

### H5P

- Relationship: Companion project, extension, or interoperability tool
- Link: https://h5p.org/
- Why visitors may care: learning-content authors can create, share, reuse, import, and export rich
  interactive activities across supported publishing and learning platforms.
- Evidence: H5P's [official site](https://h5p.org/) documents creating, sharing, reusing, importing,
  and exporting interactive HTML5 content with LMS integrations; its [reuse guide](https://h5p.org/reuse-h5p-content)
  documents copying questions and downloading/uploading packages. PLE's
  [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md#current-adapter-posture) records the supported
  bounded static-import path.
- Notes: current H5P Package content remains ungraded practice in PLE because browser-evaluated answers do not satisfy
  PLE's server-owned grading boundary.

### QTI Package Maker

- Relationship: Companion project, extension, or interoperability tool
- Link: https://github.com/vosslab/qti-package-maker
- Why visitors may care: instructors and assessment developers can convert Blackboard Question
  Upload text into Canvas, Blackboard, HTML self-test, and other teaching formats.
- Evidence: the project's official README documents its conversion workflow, while
  [QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) records the reviewed item semantics used by
  PLE Question JSON version 3.
- Notes: QTI Package Maker is an interoperability and interaction oracle, not a PLE runtime
  dependency or a Rust porting target.

### Biology Problems OER

- Relationship: Closely related sibling project with an overlapping user goal
- Link: https://biologyproblems.org/
- Why visitors may care: biology instructors can explore free problem sets for biochemistry,
  genetics, molecular biology, biostatistics, and laboratory teaching, plus LMS import guides.
- Evidence: the official site describes chapter-based problem collections and Blackboard and Canvas
  import tutorials; PLE's [README.md](../README.md) identifies it as the author's related open
  educational resource project.
- Notes: the site supplies adjacent content and authoring workflows; it is not a runtime dependency, imported, or
  executed by the PLE runtime.

### 1EdTech QTI

- Relationship: Domain standard, guide, dataset, or other visitor resource
- Link: https://www.1edtech.org/standards/qti/index
- Why visitors may care: assessment authors and platform developers can use the governing standard
  for exchanging items, tests, and results among authoring tools, item banks, delivery systems, and
  scoring engines.
- Evidence: 1EdTech's official specification describes that exchange workflow; PLE's
  [CONTRACTS.md](CONTRACTS.md#qti-profile-to-PLE-contract) records its bounded Canvas 1.2 and
  Blackboard 2.1 profile-to-PLE contract.
- Notes: QTI is an interchange boundary rather than PLE's internal source model, and current profile
  support is intentionally narrower than the complete standard.

## Possible related projects

### PrairieLearn

- Relationship: Direct alternative or competitor
- Link: https://docs.prairielearn.com/
- Why visitors may care: instructors can compare another open problem-driven system for homework,
  tests, randomized variants, automatic grading, content sharing, and LMS integration.
- Evidence: PrairieLearn's [official overview](https://docs.prairielearn.com/) describes randomized
  custom questions, Python-based autograding, homework and exams, access controls, and code
  autograders. Its [assessment guide](https://docs.prairielearn.com/assessment/configuration/)
  explicitly distinguishes formative homework with unlimited randomized retries from exams.
- Confidence: likely

### Numbas

- Relationship: Direct alternative or competitor
- Link: https://www.numbas.org.uk/
- Why visitors may care: instructors can create randomized, automatically marked assessments, reuse
  openly licensed questions, and deliver work through an LMS or standalone player.
- Evidence: Numbas' [official site](https://www.numbas.org.uk/) documents randomized questions,
  automatic marking, feedback, and LMS or standalone delivery. Its
  [current documentation](https://docs.numbas.org.uk/en/latest/) documents authoring Questions and
  exams, while its [marking guide](https://www.numbas.org.uk/behind-the-design/marking.html)
  explains the interpretation, marking, and feedback boundary.
- Confidence: likely

### Moodle question banks

- Relationship: Same-workflow project or independent implementation
- Link: https://docs.moodle.org/502/en/Question_banks
- Why visitors may care: instructors can compare a mature course-shared question-bank workflow with
  preview, editing, categories, cross-course reuse, and exact-version selection.
- Evidence: Moodle's [official Question banks guide](https://docs.moodle.org/502/en/Question_banks)
  describes course-shared and quiz-local banks, cross-course reuse, question history, and choosing
  an exact version for a quiz.
- Confidence: likely

### 1EdTech LTI Advantage

- Relationship: Domain standard, guide, dataset, or other visitor resource
- Link: https://standards.1edtech.org/lti/specifications/core/lti-spec1p3p1
- Why visitors may care: learning-platform operators can study the standard launch, role, and
  Assignment and Grade Services contracts used to connect assessment tools with an LMS.
- Evidence: 1EdTech's official LTI documentation defines secure platform-to-tool integration and
  links the final Assignment and Grade Services specification; PLE assigns its future verified
  launch and grade-passback implementation to the future LTI capability in
  [ROADMAP.md](ROADMAP.md).
- Confidence: likely
- Notes: this is a planned registered-protocol reference, not a current PLE Question Backend capability.

## Evidence notes

The confirmed set requires repository evidence of a current adapter, explicit lineage, named prior
art, a same-author companion relationship, or a standard already present in PLE's contracts. The
possible set comes from bounded seed and widening discovery reviewed on 2026-09-05. Official
PrairieLearn, Moodle, iMathAS, WeBWorK, H5P, Numbas, and 1EdTech sources confirm the shared
assessment-authoring, question-reuse, automatic-grading, content-interchange, or LMS-integration
workflows; retained entries preserve their own evidence links.

The distinctions between WeBWorK2, PG, and `webwork-pg-renderer` are intentional: WeBWorK2 is the
full application and prior art, PG is the upstream question engine, and the standalone renderer is
PLE's private integration service. Likewise, QTI is an implemented but bounded interchange
standard, while LTI Advantage remains planned work. These distinctions keep visitor discovery
useful without overstating PLE's current compatibility or runtime dependencies.
