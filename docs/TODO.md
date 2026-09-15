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
- [ ] Replace each BiologyProblems.org WeBWorK static expansion with its one
      canonical algorithmic PG/PGML source and one Published Question lineage. Verify
      provenance, parameter behavior, and representative rendering/grading; retire
      redundant generated-variant Pool members when present while retaining intentionally pooled
      distinct algorithmic Questions; preserve historical pins while retiring replaced
      static Questions, redundant Pools, and source copies.
- [ ] Build public Blueprint Course search as one bounded projection, Store,
      Server, authorization, and browser workflow capability.
- [ ] Build My Questions, Starred, and Watched as bounded
      ownership or saved-state capabilities with their required read paths,
      authorization, and browser workflows.
- [ ] Build reusable Assessment Templates as a bounded domain, Store, Server,
      authorization, and browser workflow capability.
- [ ] Build pilot grade export as a direct authorized CSV or TSV download of
      point-based Assessment scores. Do not add LMS synchronization, separate
      Question weights, Grade Categories, weighted categories, Course Grade
      Schemes, or Course percentage calculations.
- [ ] Build Blueprint update review, source-fork update discovery and selective
      application, Blueprint Course Change Proposals, and canonical Blueprint
      JSON import/export as complete, authorized workflows. Existing Assessment
      changes require review; newly added Blueprint Assessments copy to daughter
      Courses automatically as Unreleased Assessments.
- [ ] Remove the generic Question Seed input from static PLE Question JSON
      issuance; static declarative sources do not vary and do not execute code.
- [ ] Enforce the Course Instance six-month maximum Active lifetime from
      creation: warn the Instructors, reject later Assessment deadlines, make
      the Course Inactive at the limit, preserve bulk roster import for adding
      Students, and prohibit bulk Student removal while preserving individual
      enrollment corrections.
- [ ] Implement the Course retention behavior already defined by Human
      Guidance: latest-Assessment-deadline FERPA clock, Instructor notice, FERPA archive
      from normal interfaces, recoverability during the retention period,
      and permanent deletion. FERPA retention durations are tunable operational
      configuration; exact Store/worker shapes remain implementation choices,
      and no retention Revision family is required.

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
