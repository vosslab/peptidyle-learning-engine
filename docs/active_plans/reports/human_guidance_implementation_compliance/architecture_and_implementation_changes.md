# Architecture And Implementation Changes

## Scope

This is a fresh topical implementation-audit inventory. It collects currently open
Human Guidance checklist records relevant to architecture and implementation. The bullet text is
copied verbatim from the generated checklist, and its source location is recorded beside it.
This report does not establish exhaustive or disjoint topical coverage; the checklist remains the
authority for each record's status.

The authoritative exhaustive record is the
[generated checklist](../../audits/human_guidance_implementation_checklist.md).

## Topical inventory

### Development principles -- Codebase development rules

- PLE is pre-production with no users. Fix the design directly rather than preserving legacy behavior.
  - Source: `docs/HUMAN_GUIDANCE.md:46`

- PLE is pre-production with no users or durable production data. Improve the design directly.
  - Source: `docs/HUMAN_GUIDANCE.md:50`

- Every source file should stay below 1000 lines. Split complete capabilities into focused modules.
  - Source: `docs/HUMAN_GUIDANCE.md:45`

- Use readable `snake_case` whenever possible; see [NAMING_CONVENTIONS.md](/docs/NAMING_CONVENTIONS.md) for details.
  - Source: `docs/HUMAN_GUIDANCE.md:51`

- Adaptability should be a focus so the software can evolve as requirements and insights change.
  - Source: `docs/HUMAN_GUIDANCE.md:52`

- Cargo, Node, and PyPI dependencies should use the latest versions to include security fixes.
  - Source: `docs/HUMAN_GUIDANCE.md:53`

- If an interface is measured as too slow, consider moving the slow code to Rust/WebAssembly.
  - Source: `docs/HUMAN_GUIDANCE.md:54`

- Do not create or leave placeholder database tables, states, APIs, workers, or compatibility scaffolding before the feature has an approved product design.
  - Source: `docs/HUMAN_GUIDANCE.md:55`

### Development principles -- PLE development rules

- The polished PLE Live Demo is the top priority; see [LIVE_DEMO_SPEC.md](/docs/LIVE_DEMO_SPEC.md).
  - Source: `docs/HUMAN_GUIDANCE.md:65`
