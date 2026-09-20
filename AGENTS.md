# Agent instructions

## Durable authorities

### Human edited primary authority

- docs/HUMAN_GUIDANCE.md
- docs/FALL_2026_PILOT.md

### LLM but human lightly inspected authorities

- docs/DATABASE_STYLE.md
- docs/TERMINOLOGY_CONTRACT.md
- docs/DESIGN_DECISIONS.md
- docs/REPO_STYLE.md
- docs/*_STYLE.md

### Other files of note

- docs/CONTRACTS.md
- docs/ROADMAP.md
- docs/TODO.md

### external content of note

- OTHER_REPOS/adapt/ # platform that we are building from and making better
- OTHER_REPOS/biology-problems-website/ # source of pilot content

## Workflow

- Follow the main authoritative docs/HUMAN_GUIDANCE.md and the language, naming, repository,
  Markdown, and test style guides in docs/.
- Use `source ./source_me.sh && ./launchers/run_fast_checks.sh` for a quicker compliance check
- Use `source ./source_me.sh && ./launchers/all_test.sh` for a complete compliance check
- Use `source ./source_me.sh && ./devel/capture_screenshots.sh` for UI work. A running Live Demo rebuilds only the stale bundle; TypeScript/CSS/WASM edits do not replace containers.
- Complete one bounded task, pass its narrow gate, then update docs/CHANGELOG.md.
- Run Python commands through `source source_me.sh && python3`.
- Eval schema tables with `source source_me.sh && devel/generate_schema_tables_doc.py && schema_style/check_schema_style.py`

## Development principles

### Agent working principles

- Read and learn the core principles in docs/REPO_STYLE.md
- Apply the Keep It Simple, Stupid (KISS) philosophy aggressively.
- Prefer the smallest coherent design that meets actual requirements and known failure modes.
- Complexity must earn its place.
- Time should be used efficiently. Agents and tokens are cheap; wall time is not.
- Hard work should be broken into small, independently completable tasks.
- Write content in plain, concrete language. Use technical terms when they add precision.
- Prioritize positive prompting. Phrase instructions as concrete actions such as "Do X" or "Use Y".
- Name only the tools and responsibilities needed for the assigned task. Positive prompting plus
  omission keeps agent instructions focused on the intended actions.
- Small LMs may interpret negative instructions as actions to perform. State the desired behavior
  directly, including when assigning responsibilities to agents.
- Long local operations must be robust and informative: keep going through imperfect state where
  useful, recover gracefully, and tell me what is happening while I wait. I am impatient.
- Classify one-time checks separately from permanent tests.
- Finish the obvious. Continue while the next safe step is defined by the plan, implied by the current task.
- Robust means the software continues to function despite imperfect inputs, data, state, or behavior.
- Treat tests as liabilities as well as protection. Keep only requirements and gates grounded in actual needs.
- Plans should be finishable by the manager and subagents without additional human interaction.
- Prefer more small, independently verifiable milestones over a few large milestones.
- Fix the design that causes a problem rather than adding a workaround for its symptom.
- Prefer durable long-term fixes when the additional cost is justified.
- Prefer adaptable boundaries and simple domain concepts over speculative edge-case machinery.
- Add product states, workflows, background processing, and recovery mechanisms only for a
  demonstrated product or Question Backend need.
- Stay focused on the requested work. Complete the required work and avoid adding unplanned functionality.
- Every source file should stay below 1000 lines. Split complete capabilities into focused modules.

### Codebase development rules

- Language-native casing is the right system: SQL stays account_id, Rust/TS types stay AccountId, JSON stays accountId.
- PLE is pre-production with no users or durable production data. Fix the design directly;
  there is no legacy behavior to preserve.
- Use the pre-production state to improve foundational schemas, contracts, and abstractions
  whenever that produces a stronger long-term system.
- Use SQL directly to create the initial PostgreSQL database structure. Insertions have more flexibility.
- Before production, edit the main database design directly as the design changes.
- After production, update existing databases without rebuilding them from scratch.
- Use readable `snake_case` whenever possible; see [NAMING_CONVENTIONS.md](/docs/NAMING_CONVENTIONS.md) for details.
- Adaptability should be a focus so the software can evolve as requirements and insights change.
- Cargo, Node, and PyPI dependencies should use the latest versions to include security fixes.
- If an interface is measured as too slow, consider moving the slow code to Rust/WebAssembly.
- All fields, identifiers, domain concepts, and terminology in the PostgreSQL database structure, Rust code, TypeScript code, JSON/API
  contracts, Terminology Contract, and Human Guidance are in alignment.
