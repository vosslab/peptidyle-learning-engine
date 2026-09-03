# News

PLE remains pre-production; these notes describe the current development snapshot.

## v26.08 - 2026-08-28

### Highlights

- Instructor assignment work now has a shared Overview, Questions, Policies, and Student view
  workspace with answer-free student presentation and responsive Start/continue actions.
- Curriculum adoption now supports preview-before-save, rollover, term shifting, recorded Course
  Origin and Assignment Source Record history, controlled fast-forward, and divergence recovery.
- Automated grading now has immutable accepted-input receipts, answer-free status and retry flows,
  generation-fenced recalculation, and Instructor exception handling.
- The connected demo runs the production-shaped Rust, PostgreSQL, object-store, worker, and HTTPS
  stack with deterministic cleanup.

### Upgrade notes

- Use `source source_me.sh && python3 ...` for repository Python commands. The canonical live-demo
  entry remains `./run_live_demo.sh`.
