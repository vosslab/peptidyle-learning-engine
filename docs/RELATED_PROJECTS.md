# Related projects

PLE connects selected question ecosystems through small, explicit adapter boundaries. This page
identifies those relationships without implying broad format compatibility, browser trust, or a
runtime dependency where none exists. Current adapter scope and acceptance evidence remain in
[ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md) and
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Confirmed related projects

### WeBWorK PG

- Relationship: indirect runtime rendering-engine dependency
- Link: https://github.com/openwebwork/pg
- Evidence: PLE's external renderer uses PG/PGML to render and grade source under PLE-controlled
  attempt state. The PLE runtime image and protocol are documented in
  [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).
- Notes: PG provides the problem language, randomization, rendering, and grading engine. It does
  not own a PLE course, roster, assignment, attempt, or gradebook.

### webwork-pg-renderer

- Relationship: direct integration target and same-author sibling repository
- Link: https://github.com/vosslab/webwork-pg-renderer
- Evidence: `containers/compose.yaml` starts the declared external renderer image, while
  `crates/adapters/webwork` consumes its private `/render-api` contract.
- Notes: The maintained sibling checkout is `../webwork-pg-renderer`.
  `OTHER_REPOS/webwork-pg-renderer` is an identical read-only reference mirror. PLE consumes a
  pinned image or declared artifact; it does not import, mount, or build from `OTHER_REPOS/`.

### WeBWorK2

- Relationship: prior art and application-layer reference
- Link: https://github.com/openwebwork/webwork2
- Evidence: PLE's local-stack architecture names WeBWorK2 as reference material and explicitly
  omits both its course-management runtime and MariaDB.
- Notes: WeBWorK2 is a full homework application. PLE retains ownership of courses, rosters,
  assignments, attempts, feedback, and grades, so deploying it would create a parallel assignment
  system. It remains useful for understanding PG application behavior.

### QTI Package Maker

- Relationship: same-author interoperability oracle
- Link: https://github.com/vosslab/qti_package_maker
- Evidence: [QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) records the reviewed MC, MA,
  MATCH, NUM, FIB, MULTI_FIB, and ORDER semantics used by PLE flat-question JSON v2.
- Notes: This Python project creates QTI and teaching-format exports. PLE treats it as reviewed
  interoperability evidence, not as a runtime dependency or a Rust porting target.

### Biology Problems OER

- Relationship: same-author content companion
- Link: https://biologyproblems.org/
- Evidence: [README.md](../README.md) identifies the Biology Problems open educational resource
  project as related work, and `crates/project-tools` records source provenance for the initial
  WebWork pilot content.
- Notes: Biology Problems supplies adjacent biology content and LMS-export workflows. PLE does not
  build, import, or execute the project as part of its runtime.

### LibreTexts ADAPT

- Relationship: prior art and independent learning-platform alternative
- Link: https://adapt.libretexts.org/
- Evidence: ADAPT's public guide documents instructor course and gradebook workflows, while its
  student guide documents direct account creation followed by enrollment with a course access code.
  PLE's comparison is recorded in [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) and
  [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md).
- Notes: ADAPT usefully demonstrates course-level enrollment and one gradebook across multiple
  question engines. Its public documentation also shows direct student accounts and, in
  institution-specific guidance, campus-login paths. Those sources do not establish one universal
  institutional-SSO contract. PLE instead owns a global opaque account, uses invitation claim for
  roster membership, and keeps course authorization as the disclosure boundary.

### iMathAS

- Relationship: optional integration target
- Link: https://www.imathas.com/
- Evidence: `crates/adapters/imathas` is a workspace adapter, and
  [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md) defines its immutable server snapshot and
  server-brokered verified-result boundary.
- Notes: PLE supports only the contracted backend seam. Generic hosted execution and
  browser-trusted launch or score flows are outside the supported contract.

### H5P

- Relationship: optional integration target
- Link: https://h5p.org/
- Evidence: `crates/adapters/h5p` is a workspace adapter, and
  [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md) defines its static-import posture. H5P's
  official documentation describes browser-based interactive HTML5 content.
- Notes: PLE accepts the bounded static path for ungraded practice. Browser-side H5P execution is
  not presented as a server-verifiable grade.

## Evidence notes

The workspace manifest confirms the adapter ownership boundaries. The renderer configuration,
[LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md), and
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md) establish the three distinct
WeBWorK layers: PG is the engine, `webwork-pg-renderer` is PLE's private HTTP integration, and
WeBWorK2 is a separate full application retained only as reference material. `OTHER_REPOS/` is a
read-only comparison area, never a build context, mount, import source, or runtime dependency.

The ADAPT relationship is supported by its public
[student account and enrollment guide](https://chem.libretexts.org/Courses/Remixer_University/Mastering_ADAPT%3A_A_User%27s_Guide/06%3A_Using_ADAPT_as_a_Student/6.01%3A_New_Page),
[account guide](https://chem.libretexts.org/Courses/Remixer_University/Mastering_ADAPT%3A_A_User%27s_Guide/04%3A_Using_ADAPT_as_an_Instructor/4.01%3A_Accounts),
and [course-properties guide](https://chem.libretexts.org/Courses/Remixer_University/Mastering_ADAPT%3A_A_User%27s_Guide/04%3A_Using_ADAPT_as_an_Instructor/4.03%3A_Course_Properties).
Those documents support the stated enrollment and gradebook lessons; they are not evidence for a
general ADAPT institutional-identity architecture. Official project pages corroborate the external
links for PG, WeBWorK2, H5P, iMathAS, and the standalone renderer.
