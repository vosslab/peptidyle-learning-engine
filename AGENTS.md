# AGENTS.md

## Project Direction

Use the active implementation plan as the source of truth for architecture, scope, milestone order,
contracts, security boundaries, storage rules, frontend behavior, testing, and acceptance criteria.
Complete work in the documented dependency order.
Finish one defined task before starting the next.
Use the acceptance criteria and validation gates for the current task before proceeding.

## Working Method

Read the relevant plan section, source files, tests, and style guides before editing.
State the current task in a few sentences.

Identify:
- the files expected to change
- the intended behavior
- the relevant validation commands

Make small, direct edits to the relevant files.
Inspect the changed files and diff after each edit.
Run the narrowest relevant validation first.
Run the complete task gate after the focused checks pass.
Update docs/CHANGELOG.md after successful validation.
Then continue to the next task in dependency order.

## Debugging

Reproduce the failure with the smallest relevant command.
Read the complete affected function, module, or configuration block.
Identify one likely cause supported by the available evidence.
Apply one focused correction.
Run the reproducing command again.
Use the new result to select the next focused step.
After two unsuccessful corrections, summarize the evidence, current hypothesis, and next step before
continuing.

## Large Tasks

Break broad goals into small, sequential tasks.
Keep the current task narrow enough to validate in one cycle.
Preserve established contracts while implementing dependent code.
Record architectural decisions in the repository documentation named by the implementation plan.
Use existing repository abstractions and shared contracts as the integration boundary.

## Editing Strategy

Use direct repository edits for source and configuration changes.
Keep shell commands short, focused, and readable.
Use scripts when the script is a maintained project artifact or a repeatable validation tool.
Preserve existing formatting and file organization.
Review generated files through their owning generator or build process.

## Validation

Use behavior-focused validation.
Begin with the smallest test covering the changed behavior.
Run formatting, linting, type checking, compilation, and tests required by the active task.
Verify cross-language boundaries through their documented contract tests.
Verify security and tenancy behavior through the documented gates.
Proceed after the current acceptance criteria pass.

## Coding Style

See TypeScript coding style in docs/TYPESCRIPT_STYLE.md.
See Rust style in docs/RUST_STYLE.md.
See Markdown style in docs/MARKDOWN_STYLE.md.
See repository style in docs/REPO_STYLE.md.
Document completed changes in docs/CHANGELOG.md.
