# Related projects

## Confirmed related projects

### WeBWorK

- Relationship: direct dependency
- Link: https://github.com/openwebwork/webwork2
- Evidence: PLE builds pinned upstream `webwork2` and `pg` revisions and calls the authenticated
  `render_rpc` protocol through its private adapter.
- Notes: PLE supports one bounded RadioButtons rendering and grading path; it does not claim broad
  Open Problem Library compatibility.

### QTI Package Maker

- Relationship: companion CLI, library, or demo repo
- Link: https://pypi.org/project/qti-package-maker/
- Evidence: [QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) records that PLE flat JSON v2
  follows the reviewed QTI Package Maker MC, MA, MATCH, NUM, FIB, MULTI_FIB, and ORDER semantics.
- Notes: The package creates assessment packages for Canvas, Blackboard, Moodle, and LibreTexts
  ADAPT. PLE reuses its item semantics rather than duplicating its exporters; a future QTI-JSONL
  contract would remain a separate interchange adapter.

### Biology Problems OER

- Relationship: same-author or same-org sibling repo
- Link: https://biologyproblems.org/
- Evidence: [README.md](../README.md) identifies Neil R. Voss as maintaining both PLE and Biology
  Problems, which publishes open biology problem sets and Peptidyle daily puzzles.
- Notes: Biology Problems supplies an adjacent open-education content workflow, not a PLE runtime
  dependency.

### LibreTexts ADAPT

- Relationship: prior art or inspiration
- Link: https://adapt.libretexts.org/
- Evidence: [implementation_plan.md](active_plans/implementation_plan.md) names a specific ADAPT
  draft-problem leakage failure that PLE's publish-only `ProblemId` boundary is designed to avoid.
- Notes: The projects overlap in assessment delivery, but PLE deliberately keeps version 1 narrower
  than ADAPT parity.

## Evidence notes

Repository evidence establishes the direct WeBWorK integration and the QTI Package Maker contract
handoff. The PLE README establishes the shared-maintainer relationship with Biology Problems, while
the active implementation plan records ADAPT as examined prior art and a security boundary reference.
Official project pages and package metadata provide the external links above.
