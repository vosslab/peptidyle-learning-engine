# Agent instructions

- `docs/active_plans/implementation_plan.md` and its active release plan are the source of truth for
  scope, architecture, contracts, dependency order, validation, and acceptance.
- `docs/active_plans/implementation_status.md` is the sole source of truth for the changing global
  current-package handoff and shared migration allocation registry. Plans own scope, dependency
  order, validation, and acceptance; they link to status instead of copying mutable handoff values.
- Package identities that cross plans, status, migration allocation, or changelog evidence must be
  globally unique and plan-namespaced. Reserve `WP-PROF-*` for the active professor roadmap; keep
  accepted cross-roadmap keys (`WP-R0`, `WP-R1`, `WP-R2`, and `WP-PY-L1`) unchanged.
- Preserve durable owner decisions in `docs/HUMAN_GUIDANCE.md`.
- Complete one defined task in dependency order. Use its narrow validation and acceptance gate before
  continuing, then update `docs/CHANGELOG.md`.
- A goal is complete only when its entire Validation test suite is green under
  [docs/TEST_EVIDENCE_MODEL.md](docs/TEST_EVIDENCE_MODEL.md#validation-test-suite). Required gates
  must pass on the final material tree; an unrun or required skipped gate keeps the goal incomplete.

## Working references

- `docs/DEVELOPMENT.md`, `docs/INSTALL.md`, `docs/USAGE.md`, and `docs/TROUBLESHOOTING.md`
- `docs/CODE_ARCHITECTURE.md` and `docs/FILE_STRUCTURE.md`
- `docs/CONTRACTS.md`, `docs/DESIGN_DECISIONS.md`, `docs/NAMING_CONVENTIONS.md`,
  `docs/REPO_STYLE.md`, and `docs/MARKDOWN_STYLE.md`
- `docs/RUST_STYLE.md`, `docs/TYPESCRIPT_STYLE.md`, and `docs/PYTHON_STYLE.md`
- `docs/PYTEST_STYLE.md` and `docs/PLAYWRIGHT_TEST_STYLE.md`
