# TODO

## Before first production deployment

- [ ] Execute the reviewed clean-cluster database-baseline replacement in
  [ROADMAP.md](ROADMAP.md) after the active release packages are accepted.
- [ ] Replace the unreleased SQLx migration history with one immutable baseline migration; the
  current inventory is maintained in [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md) and the shared
  migration ledger.
- [ ] Start the durable forward-only migration ledger from the shipped baseline.

## Current work routing

- Product and release work remains owned by the
  [release completion plan](active_plans/active/release_completion_plan.md).
- Current architecture and storage boundaries remain owned by the
  [implementation plan](active_plans/implementation_plan.md).
- Schema ownership and current migration inventory remain in
  [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).
- The future baseline procedure, gates, and recovery rules are in
  [ROADMAP.md](ROADMAP.md).

## Not now

- Do not modify, squash, or renumber the current migration history during active feature acceptance.
- Do not add data-adoption, compatibility, or legacy-reader work: PLE remains pre-production with
  no durable user data.
- [ ] Evaluate complementary container-query adoption as evidence-driven responsive maintenance;
  keep [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) authoritative and do not claim current implementation
  until evidence from a representative surface test supports it.
