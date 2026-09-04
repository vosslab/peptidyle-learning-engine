# News

PLE remains pre-production; these notes describe the current development snapshot.

## v26.09 - 2026-09-04

### Highlights

- PLE now uses direct Question, Blueprint Course, Assignment, and publication terminology across
  its current contracts, without compatibility aliases.
- Server-only Question Publication now creates immutable new-lineage evidence while keeping the
  browser surface answer-free and unchanged.
- Durable documentation now routes readers to the architecture, contracts, roadmap, database,
  evidence, TODO, and changelog records; historical plans live in `docs/archive/`.

## v26.08 - 2026-08-28

### Highlights

- Instructor assignment work now has a shared Overview, Questions, Policies, and Student view
  workspace with answer-free student presentation and responsive Start/continue actions.
- Blueprint operations now provide previews for creating a Course from a Blueprint, copying a
  Course for a new term, shifting Course dates, applying a Blueprint update, and copying an
  Assignment from a Blueprint. Their records retain exact Course Origin and Assignment Source
  facts.
- Automated grading now has immutable accepted-input receipts, answer-free status and retry flows,
  generation-fenced recalculation, and Instructor exception handling.
- The connected demo runs the production-shaped Rust, PostgreSQL, object-store, worker, and HTTPS
  stack with deterministic cleanup.

### Upgrade notes

- Use `source source_me.sh && python3 ...` for repository Python commands. The canonical live-demo
  entry remains `./run_live_demo.sh`.
