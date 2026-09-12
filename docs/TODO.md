# TODO

This list routes genuinely unfinished work. Product and schema boundaries are
authoritative in [CONTRACTS.md](CONTRACTS.md),
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md), and
[DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md); release evidence and sequencing
are in [ROADMAP.md](ROADMAP.md).

## Future product capabilities

- [ ] Build public Blueprint Course search as one bounded projection, Store,
      Server, authorization, and browser workflow capability.
- [ ] Build My Questions, Starred Questions, and Watched Questions as bounded
      ownership or saved-state capabilities with their required read paths,
      authorization, and browser workflows.
- [ ] Build reusable Assignment Templates as a bounded domain, Store, Server,
      authorization, and browser workflow capability.
- [ ] Build Blueprint short- and long-name editing as mutable Blueprint-lineage
      metadata with the required Store, Server, authorization, and browser
      workflow.
- [ ] Build Question Change Proposal as a complete domain, Store, Server,
      authorization, and browser workflow capability. Its design begins from
      the implemented Question publication and Forced Question Correction
      boundaries; it has no pre-existing persistence contract to extend.
- [ ] Build Course Retention as a complete domain, Store, Server,
      authorization, worker, and browser workflow capability. Its design must
      define the Course-wide Student Work lifecycle and evidence it requires;
      it has no pre-existing retention-plan or revision persistence contract to
      extend.

## Production release

- [ ] Obtain the explicit human production-release decision after the Roadmap's
      completed implementation gates. This decision freezes the canonical base
      schema and starts the forward-only migration lifecycle.

## Evidence discipline

Temporary investigation material supports decisions without becoming a parallel
documentation layer. Keep accepted decisions in the owning contract, code, test,
or operational guide; retire duplicate or superseded working evidence. Permanent
tests protect durable behavior, authorization, evidence integrity, or lifecycle
contracts.
