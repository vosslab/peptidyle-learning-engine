# Agent instructions

## Project direction

- `docs/active_plans/implementation_plan.md` and its active release plan are the source of truth for scope, architecture, contracts, dependency order, validation, and acceptance.
- Apply durable owner decisions in `docs/HUMAN_GUIDANCE.md`.
- Complete one defined task at a time in documented dependency order, and meet its acceptance criteria before continuing.
- Before editing, state the task, expected files, intended behavior, and relevant validation commands.
- Make small, direct edits; inspect changed files and the diff after each edit.
- Reproduce failures narrowly, correct one evidence-backed cause, and summarize evidence, hypothesis, and next step after two unsuccessful corrections.
- Run the narrowest behavior check first, then the active task gate; update `docs/CHANGELOG.md` after successful validation.

## Working references

- `docs/DEVELOPMENT.md`
- `docs/INSTALL.md`
- `docs/USAGE.md`
- `docs/CONTRACTS.md`
- `docs/REPO_STYLE.md`
- `docs/TYPESCRIPT_STYLE.md`
- `docs/RUST_STYLE.md`
- `docs/PYTHON_STYLE.md`
- `docs/PYTEST_STYLE.md`
- `docs/PLAYWRIGHT_TEST_STYLE.md`
- `docs/MARKDOWN_STYLE.md`
