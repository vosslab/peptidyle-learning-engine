# TODO

## Current release blockers

- [ ] Have the repository owner track the exact current repository-owned manifest in
  [implementation_status.md](active_plans/implementation_status.md) so the documentation-link gate
  can inspect its physical targets.
- [ ] Rerun the exact final-tracked-tree `source source_me.sh && ./all_test.sh` gate, then
  advance `WP-PROF-G1` to `WP-PROF-G2` if it passes; see
  [implementation_status.md](active_plans/implementation_status.md).

## Before first production deployment

- Follow the dependency-ordered release packages and external activation checks in the
  [release completion plan](active_plans/active/release_completion_plan.md). The disposable
  [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) is product evidence, not production activation.
- Use [ROADMAP.md](ROADMAP.md) for the final schema-freeze procedure, release gates, and recovery
  rules; do not duplicate that sequenced work here.

## Work routing

- Product and release work remains owned by the
  [release completion plan](active_plans/active/release_completion_plan.md).
- Current architecture and storage boundaries remain owned by the
  [implementation plan](active_plans/implementation_plan.md).
- Schema ownership and current migration inventory remain in
  [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).
- The future baseline procedure, gates, and recovery rules are in
  [ROADMAP.md](ROADMAP.md).

## Future allocation and evidence gates

- Keep accepted migration history stable during active feature acceptance; allocate any future
  repair or schema delta forward under the current rule in
  [implementation_status.md](active_plans/implementation_status.md).
- Focus pre-production work on the current live PLE; route data-adoption, compatibility, or
  legacy-reader work to an active plan only when durable user-data evidence establishes that need.
- [ ] Evaluate complementary container-query adoption as evidence-driven responsive maintenance;
  use [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) as the authority and record an implementation claim
  only after evidence from a representative surface test supports it.
