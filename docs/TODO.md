# TODO

## Current work

- [ ] Select one dependency-ready bounded work item from this roadmap and its durable contracts,
      pass its named gates, and transfer accepted evidence to [CHANGELOG.md](CHANGELOG.md).
- [ ] Keep execution-only coordinates outside permanent documentation. Record durable outcomes in
      [CONTRACTS.md](CONTRACTS.md), [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md), or the focused guide.

## Deferred interface capabilities

- [ ] Build public Blueprint Course search when a public Blueprint projection and its authorization
      boundary are designed; the settled Courses task-row position remains Unavailable.
- [ ] Build My Questions, Starred, and Watched when their cross-course ownership and saved-state
      models exist; their settled Questions task-row positions remain Unavailable.
- [ ] Build My Assignment Templates when a reusable Assignment-template domain and read path exist;
      its settled Assignments task-row position remains Unavailable.
- [ ] Build Active and Inactive Courses as part of a dedicated Course Retention capability. Existing
      retention-plan/job/event scaffolding has no executable atomic Course-wide FERPA-stripping
      transition or Course-bound receipt that attests that transition. The existing
      `course_retention_event` is indirect preparation, not attestation that stripping occurred,
      so both settled Courses task-row positions remain Unavailable. This capability may use
      retention-specific evidence without requiring the broad mutable-state Edit Number/revision
      migration.
- [ ] Decide and implement Blueprint short and long names as mutable Blueprint-aggregate metadata.
      Those names identify a Blueprint across revisions; Blueprint storage and Blueprint
      Revision/checksum semantics remain deferred, so this item makes no current schema or content
      revision change.

## Delivered Course Appearance

- Course-scoped Theme and Banner appearance is delivered: active Course Members read one current
  appearance, Instructor Course Members independently save Theme or Banner, and the Instructor
  Appearance task has focused accessibility, Ribbon, and cross-member evidence.
- [ ] Evaluate optional Course Banner visual refinement only from later Instructor use evidence;
      preserve the fixed 6:1 hero and 5:2 card contract until a bounded replacement is accepted.
- [ ] Route the inactive `--ple-theme-primary` token and neutralized scope-style work to their
      separate theme-system plan; activating either remains a behavior change, not Appearance cleanup.

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
