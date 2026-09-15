# Architecture And Implementation Changes

## Scope

This is a fresh implementation-audit inventory. Each record below is an owning `[ ]`
Human Guidance bullet: the behavior is unverified or differs from the current implementation.
The bullet text is copied verbatim from the generated checklist, and its source location is
recorded beside it. Later duplicate bullets that carry an `Owner:` pointer are excluded because
their earlier owning record is the single inventory entry.

The authoritative exhaustive record is the
[generated checklist](../../audits/human_guidance_implementation_checklist.md).

## Owning inventory

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

## Count method

This report owns **9** checklist records. The count is the number of `[ ]` bullets
in the listed sections after excluding records with a later-duplicate `Owner:` pointer.
It is mechanically reconciled with the other topical inventories by the temporary report
generation check; it is not a permanent test.
