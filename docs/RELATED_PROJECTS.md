# Related projects

PLE uses a small number of external question ecosystems through explicit adapter boundaries. This
page names those relationships without implying broad format compatibility, browser trust, or a
runtime dependency where none exists. Current adapter scope and acceptance state remain in
[ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md) and
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Confirmed related projects

### WeBWorK PG

- Relationship: direct rendering-engine dependency
- Link: https://github.com/openwebwork/pg
- Evidence: PG is the problem-language, randomization, rendering, and grading engine used by the
  standalone renderer. Its `WeBWorK::PG` API accepts problem source, seed, and submitted inputs
  without owning a course, roster, assignment, or SQL database.
- Notes: PLE does not call PG directly from the browser or Rust API. PG runs inside the private
  renderer and remains subordinate to PLE's attempt, grading, and disclosure contracts.

### WebWork PG renderer

- Relationship: direct integration target and same-author sibling repo
- Link: https://github.com/vosslab/webwork-pg-renderer
- Evidence: The renderer wraps `WeBWorK::PG` in a small HTTP service with `/health`, `/`, and
  `/render-api` routes. It renders and grades PG/PGML without implementing a second course,
  assignment, enrollment, or gradebook system.
- Notes: This is PLE's required runtime destination. The maintained sibling checkout is
  `../webwork-pg-renderer`; `OTHER_REPOS/webwork-pg-renderer` is an identical read-only reference
  mirror. PLE must pin an upstream revision or built artifact and must not import, mount, or build
  from `OTHER_REPOS/`.

### WeBWorK2

- Relationship: prior art only
- Link: https://github.com/openwebwork/webwork2
- Evidence: WeBWorK2 is the complete course-management and online-homework application. PLE instead
  uses the smaller external `webwork-pg-renderer` service and has no WebWork2 or MariaDB runtime.
- Notes: WeBWorK2 is not PLE's product boundary. PLE already owns courses, assignments, enrollment,
  attempts, feedback, and grades, so retaining the full application would create a parallel
  assignment system. It remains useful reference material for PG application behavior.

### QTI Package Maker

- Relationship: companion CLI, library, or demo repo
- Link: https://github.com/vosslab/qti_package_maker
- Evidence: [QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) preserves the reviewed MC, MA,
  MATCH, NUM, FIB, MULTI_FIB, and ORDER item semantics in PLE flat JSON v2.
- Notes: This same-author Python project creates QTI and teaching-format exports. PLE uses it as an
  interoperability oracle, not as a runtime dependency or a Rust porting target.

### Biology Problems OER

- Relationship: same-author or same-org sibling repo
- Link: https://biologyproblems.org/
- Evidence: [README.md](../README.md) identifies Neil R. Voss as maintaining both PLE and the
  Biology Problems open educational resource project.
- Notes: Biology Problems supplies adjacent biology content and LMS-export workflows; it is not
  built, imported, or executed by PLE.

### LibreTexts ADAPT

- Relationship: prior art or inspiration
- Link: https://adapt.libretexts.org/
- Evidence: The local ADAPT reference snapshot and
  [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) compare its multi-engine gradebook
  and learner payload paths with PLE's contract.
- Notes: ADAPT demonstrates the value of one gradebook over WeBWorK, iMathAS, H5P, and QTI. PLE
  intentionally differs by binding submissions to a server-owned attempt, returning answer-free
  PLE envelopes, and keeping the WeBWorK exchange private rather than browser-mediated.

### iMathAS

- Relationship: optional integration target
- Link: https://www.imathas.com/
- Evidence: `crates/adapters/imathas` is a workspace adapter, and
  [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md) defines its immutable server snapshot and
  server-brokered verified-result boundary.
- Notes: PLE supports only the contracted backend seam. Generic hosted execution and
  browser-trusted launch or score flows remain outside its supported contract.

### H5P

- Relationship: optional integration target
- Link: https://h5p.org/
- Evidence: `crates/adapters/h5p` is a workspace adapter, and
  [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md) records the supported static import posture.
- Notes: PLE imports the bounded static path for ungraded practice. It does not claim that an H5P
  browser activity supplies a server-verifiable score.

## Evidence notes

The PLE workspace manifest confirms the adapter ownership boundaries. The current release plan,
adapter guide, and private renderer contract establish the exact WeBWorK and provider scope.
`OTHER_REPOS/` is an ignored, read-only reference area rather than a dependency directory. Its
ADAPT, PG, WebWork PG renderer, WeBWorK2, Biology Problems, and QTI Package Maker snapshots provide
comparison evidence only. Build scripts, manifests, containers, and tests must resolve maintained
dependencies from pinned upstream revisions or declared artifacts, never from those snapshots.
Official project pages and QTI Package Maker's PyPI metadata corroborate the external project links
and its seven supported item families.
