# TODO

This list routes genuinely unfinished work. Product and schema boundaries are
authoritative in [CONTRACTS.md](CONTRACTS.md),
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md), and
[DATABASE_STRUCTURE.md](DATABASE_STRUCTURE.md); release evidence and sequencing
are in [ROADMAP.md](ROADMAP.md).

## Future product capabilities

- [ ] Build email-code authentication and the Gmail API Email Delivery Backend
      as the complete challenge, Store, Server, operator credential, abuse-control,
      session, browser, and connected-delivery capability specified in
      [GMAIL_EMAIL_DELIVERY_BACKEND.md](GMAIL_EMAIL_DELIVERY_BACKEND.md).
- [ ] Audit and map the 11 existing parameterized WeBWorK sources in the
      Biology Problems Website Genetics downloads to the bundled Genetics
      content. Where coverage is the same, replace static wrapper variants
      through ordinary Question publication.
- [ ] Build public Blueprint Course search as one bounded projection, Store,
      Server, authorization, and browser workflow capability.
- [ ] Build My Questions, Starred Questions, and Watched Questions as bounded
      ownership or saved-state capabilities with their required read paths,
      authorization, and browser workflows.
- [ ] Build reusable Assignment Templates as a bounded domain, Store, Server,
      authorization, and browser workflow capability.
- [ ] Build Question Change Proposal as a complete domain, Store, Server,
      authorization, and browser workflow capability. Its design begins from
      the implemented Question publication and Forced Question Correction
      boundaries; it has no pre-existing persistence contract to extend.
- [ ] Remove the generic Question Seed input from static PLE Question JSON
      issuance; static declarative sources do not vary and do not execute code.
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
