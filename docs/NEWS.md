# News

PLE remains pre-production; these notes describe the current development snapshot.

## v26.08 - 2026-08-28

### Highlights

- Instructor assignment work now has a shared Overview, Questions, Policies, and Student view
  workspace with answer-free learner presentation and responsive Start/continue actions.
- Curriculum adoption now supports preview-before-save, rollover, term shifting, provenance,
  controlled fast-forward, and divergence recovery.
- Automated grading now has immutable accepted-input receipts, answer-free status and retry flows,
  generation-fenced recalculation, and Instructor exception handling.
- The connected demo runs the production-shaped Rust, PostgreSQL, object-store, worker, and HTTPS
  stack with deterministic cleanup.

### Upgrade notes

- Use the repository-owned Python 3.12 `.venv` for live-demo controller and test commands after
  sourcing repository settings; the canonical entry remains `./run_live_demo.sh`.
