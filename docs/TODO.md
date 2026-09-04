# TODO

## Current work

- [ ] Select one dependency-ready bounded work item from this roadmap and its durable contracts,
      pass its named gates, and transfer accepted evidence to [CHANGELOG.md](CHANGELOG.md).
- [ ] Keep execution-only coordinates outside permanent documentation. Record durable outcomes in
      [CONTRACTS.md](CONTRACTS.md), [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md), or the focused guide.

## Before first production deployment

- Follow the release stages and external activation checks in [ROADMAP.md](ROADMAP.md). The
  disposable [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) is product evidence, not production activation.
- Use [ROADMAP.md](ROADMAP.md) for the final schema-freeze procedure, release gates, and recovery
  rules; do not duplicate that sequenced work here.

## Work routing

- Durable product boundaries remain in [CONTRACTS.md](CONTRACTS.md) and the focused contract guides.
- Architecture and storage boundaries remain in [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) and
  [OBJECT_STORAGE.md](OBJECT_STORAGE.md).
- Schema ownership and current migration inventory remain in
  [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).
- The future baseline procedure, gates, and recovery rules are in
  [ROADMAP.md](ROADMAP.md).

## Future allocation and evidence gates

- Keep accepted migration history stable during bounded feature acceptance; allocate any future
  repair or schema delta forward under [DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md).
- Focus pre-production work on the current live PLE; route data-adoption, compatibility, or
  legacy-reader work to a bounded work item only when durable user-data evidence establishes that need.
- [ ] Evaluate complementary container-query adoption as evidence-driven responsive maintenance;
      use [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) as the authority and record an implementation claim
      only after evidence from a representative surface test supports it.
