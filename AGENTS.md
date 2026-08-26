# Agent instructions

- Use docs/active_plans/implementation_plan.md and
  docs/active_plans/active/release_completion_plan.md as the source of truth for scope,
  architecture, contracts, dependency order, validation, and acceptance.
- Use docs/active_plans/implementation_status.md as the sole mutable current-package and
  migration-allocation registry; plans own scope and link to status.
- Keep package identities globally unique and plan-namespaced. Reserve WP-PROF-* for the
  professor roadmap; keep WP-R0, WP-R1, WP-R2, and WP-PY-L1 unchanged.
- Preserve durable owner decisions in docs/HUMAN_GUIDANCE.md.
- Complete one defined task in dependency order, pass its narrow gate, then update
  docs/CHANGELOG.md.
- A goal is complete only when the full Validation test suite in docs/TEST_EVIDENCE_MODEL.md
  is green on the final material tree; required unrun or skipped gates keep it incomplete.

## Working references

- docs/DEVELOPMENT.md, docs/INSTALL.md, docs/USAGE.md, docs/TROUBLESHOOTING.md
- docs/CODE_ARCHITECTURE.md, docs/FILE_STRUCTURE.md, docs/CONTRACTS.md
- docs/DESIGN_DECISIONS.md, docs/NAMING_CONVENTIONS.md, docs/REPO_STYLE.md
- docs/MARKDOWN_STYLE.md, docs/RUST_STYLE.md, docs/TYPESCRIPT_STYLE.md, docs/PYTHON_STYLE.md
- docs/PYTEST_STYLE.md, docs/PLAYWRIGHT_TEST_STYLE.md
