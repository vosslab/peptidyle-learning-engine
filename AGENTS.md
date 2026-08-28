# Agent instructions

- Read docs/active_plans/implementation_plan.md and
  docs/active_plans/active/release_completion_plan.md for scope, architecture, dependency
  order, validation, and acceptance.
- Read docs/active_plans/implementation_status.md as the sole current-package and
  migration-allocation registry.
- Keep package identities globally unique and plan-namespaced; preserve WP-R0, WP-R1,
  WP-R2, and WP-PY-L1 in the active plans.
- Preserve owner decisions in docs/HUMAN_GUIDANCE.md; record settled technical decisions in
  docs/DESIGN_DECISIONS.md.
- Complete one defined task in dependency order, pass its narrow gate, then update
  docs/CHANGELOG.md.
- Treat docs/TEST_EVIDENCE_MODEL.md as the validation authority; required unrun or skipped
  gates keep work incomplete.

## Working references

- docs/DEVELOPMENT.md, docs/INSTALL.md, docs/USAGE.md, docs/TROUBLESHOOTING.md
- docs/CODE_ARCHITECTURE.md, docs/FILE_STRUCTURE.md, docs/CONTRACTS.md, docs/API_CONTRACTS.md
- docs/LIVE_DEMO_SPEC.md, docs/LOCAL_STACK_OPERATIONS.md, docs/SECURITY_MODEL.md
- docs/REPO_STYLE.md, docs/MARKDOWN_STYLE.md, docs/NAMING_CONVENTIONS.md
- docs/PYTHON_STYLE.md, docs/RUST_STYLE.md, docs/TYPESCRIPT_STYLE.md, docs/PYTEST_STYLE.md,
  docs/PLAYWRIGHT_TEST_STYLE.md
