# Related projects

Peptidyle Learning Engine (PLE) helps biology instructors deliver reusable, varied practice with
server-owned automated grading. The projects and resources below serve the same instructors,
assessment authors, or learning-platform operators through the same workflow or a closely adjacent
one. Current PLE adapter scope remains in [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md), and
current acceptance status remains in
[active_plans/implementation_status.md](active_plans/implementation_status.md).

## Confirmed related projects

### LibreTexts ADAPT

- Relationship: Prior art or inspiration
- Link: https://adapt.libretexts.org/
- Why visitors may care: instructors can compare another course, assignment, question-finding,
  grading, and gradebook workflow designed around reusable open assessment content.
- Evidence: LibreTexts' [official instructor guide](https://chem.libretexts.org/Courses/Remixer_University/Mastering_ADAPT%3A_A_User%27s_Guide/04%3A_Using_ADAPT_as_an_Instructor)
  documents course creation, assignment types, question finding, question editing, submissions, and
  gradebooks; PLE records ADAPT as explicit product prior art in
  [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) and
  [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md).
- Notes: PLE preserves its own deterministic server-grading, answer-free browser, identity, and
  authorization boundaries instead of treating ADAPT behavior as automatic parity.

### WeBWorK2

- Relationship: Prior art or inspiration
- Link: https://github.com/openwebwork/webwork2
- Why visitors may care: instructors can explore a mature open homework application for math and
  science courses, including course management and a large community problem library.
- Evidence: WeBWorK's official README describes the application and its Open Problem Library, while
  [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#webwork-project-boundary) names WeBWorK2 as implementation
  history and reference material for PLE.
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
- Notes: PG supplies question-engine behavior; PLE retains course, learner, attempt, and gradebook
  ownership.

### webwork-pg-renderer

- Relationship: Companion project, extension, or interoperability tool
- Link: https://github.com/vosslab/webwork-pg-renderer
- Why visitors may care: operators can run the standalone PG renderer that PLE uses for its bounded
  server-side WeBWorK render-and-grade path.
- Evidence: the project identifies itself as a WeBWorK standalone problem renderer, and PLE's
  [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) documents it as the private stateless
  renderer service in the production-shaped stack.
- Notes: PLE consumes the declared image and private HTTP contract rather than importing or mounting
  a sibling checkout.

### iMathAS

- Relationship: Companion project, extension, or interoperability tool
- Link: https://www.imathas.com/
- Why visitors may care: instructors can explore another open system for algorithmically generated,
  automatically graded homework and tests with a full gradebook.
- Evidence: the official iMathAS site documents generated questions, computer grading, learning
  management, and gradebook features; PLE's
  [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md#current-adapter-posture) records its contracted or
  self-hosted scored-embed boundary.
- Notes: PLE supports its reviewed server-brokered provider contract, not a generic browser-trusted
  hosted score flow.

### H5P

- Relationship: Companion project, extension, or interoperability tool
- Link: https://h5p.org/
- Why visitors may care: learning-content authors can create, share, reuse, import, and export rich
  interactive activities across supported publishing and learning platforms.
- Evidence: H5P's official site documents reusable interactive HTML5 content and LMS integrations;
  PLE's [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md#current-adapter-posture) records the supported
  bounded static-import path.
- Notes: native H5P remains ungraded practice in PLE because browser-evaluated answers do not satisfy
  PLE's server-owned grading boundary.

### QTI Package Maker

- Relationship: Companion project, extension, or interoperability tool
- Link: https://github.com/vosslab/qti-package-maker
- Why visitors may care: instructors and assessment developers can convert Blackboard Question
  Upload text into Canvas, Blackboard, HTML self-test, and other teaching formats.
- Evidence: the project's official README documents its conversion workflow, while
  [QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) records the reviewed item semantics used by
  PLE flat-question JSON v2.
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
- Notes: the site supplies adjacent content and authoring workflows; it is not mounted, imported, or
  executed by the PLE runtime.

### 1EdTech QTI

- Relationship: Domain standard, guide, dataset, or other visitor resource
- Link: https://www.1edtech.org/standards/qti/index
- Why visitors may care: assessment authors and platform developers can use the governing standard
  for exchanging items, tests, and results among authoring tools, item banks, delivery systems, and
  scoring engines.
- Evidence: 1EdTech's official specification describes that exchange workflow; PLE's
  [CONTRACTS.md](CONTRACTS.md#qti-profile-to-native-contract) records its bounded Canvas 1.2 and
  Blackboard 2.1 profile-to-native contract.
- Notes: QTI is an interchange boundary rather than PLE's internal source model, and current profile
  support is intentionally narrower than the complete standard.

## Possible related projects

### PrairieLearn

- Relationship: Direct alternative or competitor
- Link: https://docs.prairielearn.com/
- Why visitors may care: instructors can compare another open problem-driven system for homework,
  tests, randomized variants, automatic grading, content sharing, and LMS integration.
- Evidence: PrairieLearn's official documentation describes Python-generated randomized questions,
  automatic grading, homework and exams, access controls, and code autograders.
- Confidence: likely

### Numbas

- Relationship: Direct alternative or competitor
- Link: https://www.numbas.org.uk/
- Why visitors may care: instructors can create randomized, automatically marked assessments, reuse
  openly licensed questions, and deliver work through an LMS or standalone player.
- Evidence: Numbas' official site documents randomized variants, instant marking, feedback, public
  question reuse, accessibility, statistics, remarking, and LTI delivery.
- Confidence: likely

### Moodle question banks

- Relationship: Same-workflow project or independent implementation
- Link: https://docs.moodle.org/500/en/Question_banks
- Why visitors may care: instructors can compare a mature course-shared question-bank workflow with
  preview, editing, categories, cross-course reuse, and exact-version selection.
- Evidence: Moodle's official documentation describes course-shared and quiz-local banks, reuse
  across courses, propagated question updates, and version pinning.
- Confidence: likely

### 1EdTech LTI Advantage

- Relationship: Domain standard, guide, dataset, or other visitor resource
- Link: https://standards.1edtech.org/lti/specifications/core/lti-spec1p3p1
- Why visitors may care: learning-platform operators can study the standard launch, role, and
  Assignment and Grade Services contracts used to connect assessment tools with an LMS.
- Evidence: 1EdTech's official LTI documentation defines secure platform-to-tool integration and
  links the final Assignment and Grade Services specification; PLE assigns its future verified
  launch and grade-passback implementation to WP-RC9 in the
  [active release plan](active_plans/active/release_completion_plan.md).
- Confidence: likely
- Notes: this is a planned PLE integration reference, not a currently accepted provider capability.

## Evidence notes

The confirmed set requires repository evidence of a current adapter, explicit lineage, named prior
art, a same-author companion relationship, or a standard already present in PLE's contracts. The
possible set comes from two bounded discovery rounds and official project documentation showing a
shared instructor, assessment-authoring, question-reuse, automatic-grading, or LMS-integration
workflow.

The distinctions between WeBWorK2, PG, and `webwork-pg-renderer` are intentional: WeBWorK2 is the
full application and prior art, PG is the upstream question engine, and the standalone renderer is
PLE's private integration service. Likewise, QTI is an implemented but bounded interchange
standard, while LTI Advantage remains planned work. These distinctions keep visitor discovery
useful without overstating PLE's current compatibility or runtime dependencies.
