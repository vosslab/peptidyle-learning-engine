# Related projects

PLE uses a small number of external question ecosystems through explicit adapter boundaries. This
page names those relationships without implying broad format compatibility, browser trust, or a
runtime dependency where none exists. Current adapter scope and acceptance state remain in
[ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md) and
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Confirmed related projects

### WeBWorK and PG

- Relationship: direct dependency
- Link: https://github.com/openwebwork/webwork2
- Evidence: The optional PLE renderer profile builds exact upstream `webwork2` and `pg` revisions
  and uses the authenticated `/render_rpc` endpoint described in
  [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).
- Notes: PLE is the sole renderer client. Its accepted RC3 boundary supports one licensed,
  user-authored PGML `RadioButtons` fixture; it does not claim Open Problem Library or generic PG
  compatibility.

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
`OTHER_REPOS/` is an ignored, read-only reference area rather than a dependency directory; its
ADAPT, PG, WeBWorK, Biology Problems, and QTI Package Maker snapshots provide comparison evidence.
Official project pages and QTI Package Maker's PyPI metadata corroborate the external project
links and its seven supported item families.
