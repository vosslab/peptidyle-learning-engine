# Release history

PLE remains pre-production. This page records development snapshots; production release acceptance
remains governed by [ROADMAP.md](ROADMAP.md) and [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

## v26.09 - 2026-09-04

### Highlights

- Completed a direct PLE terminology convergence across Question Source, Question Asset Reference,
  Question Revision, Blueprint Course, Assignment, and publication contracts without retaining
  compatibility readers or duplicate product concepts.
- Added the server-only new-lineage Question Publication Store and coordinator. It verifies the
  authorized current Draft Question source, creates immutable publication evidence, and keeps the
  executable browser surface unchanged.
- Replaced permanent links to retired planning artifacts with durable architecture, contract,
  roadmap, database, evidence, TODO, and changelog authorities. Historical plans and reviews now
  live under `docs/archive/`.

### Notable fixes

- Corrected source, object, retry, variation, receipt, and availability language so each contract
  names its exact owner rather than a generic operation or lifecycle term.
- Rotated the 2026-09-02 changelog block into
  [CHANGELOG-2026-09c.md](CHANGELOG-2026-09c.md), retaining the two newest date blocks in the active
  changelog.

### Compatibility notes

- The direct terminology cutovers intentionally add no PLE compatibility aliases, readers, or
  fixtures. Historical migrations and changelog entries remain evidence, not runtime contracts.
- This is a development snapshot. The local demo and browser entry remain pre-production and do not
  establish deployment or release acceptance.

### Validation

- The recorded work includes focused Rust, browser-contract, TypeScript, PostgreSQL, and aggregate
  evidence for the affected contracts. The final documentation repair passes the Markdown-link gate
  across 196 documents.

## v26.08 - 2026-08-28

### Highlights

- Added the Instructor assignment workspace with Overview, Questions, Policies, and Student view
  surfaces backed by one course-authorized assignment load and shared management navigation.
- Added an answer-free student assignment presentation for ordinary student and Instructor Student
  views, with identity-first layout, responsive Start/continue behavior, and stable-identity
  guidance.
- Added curriculum-adoption workflows for preview-before-save adoption, rollover, term shifting,
  provenance receipts, controlled fast-forward, and divergence recovery.
- Delivered the production-shaped live-demo path with real Rust API, PostgreSQL, MinIO, worker,
  HTTPS gateway, visible workflows, seeded teaching data, and exact disposable cleanup.
- Advanced automated grading through G1-W7b: immutable accepted student input, answer-free status
  and retry behavior, generation-fenced recalculation, durable receipts, and Instructor exception
  handling.
- Added the versioned flat-question, catalog, course, assignment, grading, QTI, WeBWorK, iMathAS,
  object, retention, and identity boundaries recorded across the August changelog.

### Notable fixes

- Reordered the student presentation so assignment identity precedes instructions and progress,
  while the single primary action remains usable across widths and disappears when no action is
  supplied.
- Preserved response secrecy across answer-free summaries, pending grading, score publication, and
  retry through durable server-owned receipts, leases, and generation fences.
- Reconciled browser, service, PostgreSQL, migration, replica-restart, and cleanup oracles around
  production contracts and exact owner-scoped disposable resources.
- Unified live-demo and validation commands on the Python 3.12 interpreter selected after sourcing
  `source_me.sh`, with dependencies declared by the requirements manifests.
- Reused the accessible Question ID copy control in Instructor grading operations and clarified
  response-redaction, selector ownership, and retry identity.
- Applied audit repairs covering source ownership, generated artifacts, documentation links,
  accessibility, contrast, browser selectors, and maintained-source size limits.

### Compatibility notes

- The repository CalVer remains `26.08` in `VERSION`; Cargo and npm express the same release as
  `26.8.0` where semver is required. The separate `OTHER_REPOS/qti-package-maker` package remains
  version `26.07.14`.
- Browser-facing question, response, grading, and identity contracts remain answer-free and
  server-owned; integrations must use the documented typed adapters and routes.
- The production-shaped live demo and aggregate commands use the repository shell setup and invoke
  `python3` directly.

### Validation

- Focused TypeScript, ESLint, Prettier, student-presentation Node, Student-view contract, Rust,
  migration, PostgreSQL, WebWork, replica-restart, source-policy, documentation, and cleanup gates
  are recorded as passing in the August changelog.
- The connected production-shaped stack completed the registered browser scenarios and published
  the current 64-artifact screenshot corpus with privacy, provenance, and cleanup checks.
- The exact aggregate attempt passed Rust and codebase gates and reported 7,913 Python checks plus
  tracking-dependent Markdown-link failures for physical targets awaiting Git tracking. Final tracked-tree `all_test.sh`, the remaining
  active packages, and human release approval remain open; this is not a production release.
