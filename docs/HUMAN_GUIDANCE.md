# Human guidance

This file records durable project guidance from the repository owner. Apply it
alongside [AGENTS.md](../AGENTS.md) and the active implementation plan.

## Plan status

- Treat `docs/active_plans/implementation_plan.md` as the source of truth for
  implementation order, architecture, contracts, security, tests, and gates.
- `docs/active_plans/m0-results.md` is concluded M0 evidence. Read it when M0
  history matters; do not treat it as an active task or reopen M0 without new
  evidence.
- Finish and validate one work package before advancing to its dependency-order
  successor.

## Agent-specific guidance

- Codex follows `AGENTS.md` and the repository style documents.
- `docs/CLAUDE_HOOK_USAGE_GUIDE.md` is specific to Claude tooling and does not
  govern Codex commands or file-search behavior.

## Local services

- Podman is normally running on the owner's machine.
- Use the local containers when the active work package reaches a documented
  PostgreSQL, MinIO, health, tenancy, or other container-dependent gate.
- Keep offline contract work on memory backends when its work-package gate does
  not require containers.

## Teaching and product priorities

- The product supports learning through repeated algorithmic practice. A first
  completion or a 100 percent score must not end continued practice when policy
  permits another run.
- Fresh variation is more important pedagogically than seed replay. Give every
  newly issued parameterized question instance a fresh server-owned seed;
  preserve an existing attempt's seed only for resume, re-render, audit, and
  debugging of that same instance.
- Preserve server-only grading and answer secrecy. The browser may validate
  response format but must not receive answer keys or grading implementations.
- Keep student and course records tenant-owned while published educational
  content remains shared and immutable.
- Favor behavior-focused evidence that reflects what instructors and students
  actually do over implementation-detail tests.

## Performance choices

- When measured behavior is slow, consider implementing the hot path in Rust
  or WebAssembly.
- Keep the security boundary intact when optimizing: deterministic generation,
  response-format validation, timer display, and state transitions may run in
  WebAssembly; answers, keys, and correctness decisions remain server-only.

## Dependency versions

- Focus on the latest versions of all code because many security bugs are being
  fixed.
- Never pin versions; `>=` version requirements are acceptable.

## Generated artifacts

- Put reproducible generated content under the repository-root `generated/`
  directory and keep that directory out of Git.
- Regenerate required artifacts through their tracked owning generator before
  builds and validation; ignored output must not become an unverified input.
- Link documentation to the tracked generator or authoritative source rather
  than to files under `generated/`, which do not exist in a clean checkout.
- Track small, deliberately reviewed golden baselines when they define a
  compatibility contract or record work evidence. These are authoritative test
  inputs rather than disposable generated build output.
- Treat `tests/fixtures/published_problem/` as reviewed cross-layer test
  evidence. Keep its fully derivative TypeScript projection under ignored
  `generated/fixtures/` and regenerate it before TypeScript validation.
