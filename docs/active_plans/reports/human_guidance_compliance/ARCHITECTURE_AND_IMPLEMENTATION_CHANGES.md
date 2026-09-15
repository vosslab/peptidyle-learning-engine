# Architecture and implementation changes

Temporary working report for the corpus-wide Human Guidance compliance pass.

## Product architecture corrections

- Current source, route, table, DTO, and generated-view names are implementation evidence rather
  than product authority. Legacy `assignment`, Blueprint `available`, response-finalization, and job
  identifiers are called out at their owning documentation boundary.
- Course adoption copies an exact Public Blueprint Revision. New Revisions are offered to daughter
  Courses for review, newly added Blueprint Assessments copy automatically as Unreleased, and
  existing Assessments never change silently.
- Fork ancestry supports later source-update discovery and selective application. Blueprint Course
  Change Proposals compare canonical Blueprint JSON and create a new receiving Revision only for
  owner-accepted changes.
- Assessment, Course Instance, Draft Question, Attempt, Student Work, metadata, and retention remain
  current state. Edit Numbers are concurrency counters, not history.
- Published Question, published Question Pool, and Blueprint content use their explicit immutable
  Revision evidence; the Pool wording mismatch remains recorded rather than guessed away.
- Browser/server, database, object, and service boundaries derive authority from the authenticated
  Account and exact stored relationship, never from a URL, ID, queue payload, or generated UI state.
- A Question Backend may evaluate a complete saved response early when its interaction requires
  that work, but no Student-visible grading outcome appears before whole-Assessment submission.
  Public grading queues, Retry/regrade workflows, mutable results, compatibility readers, generic
  recovery states, and speculative snapshot/receipt families were removed from target contracts.
- Background processing is tied to exact needs: expired-Attempt submission and Course retention.
  Operation-specific asset publication may retain its current implementation machinery without
  creating product lifecycle states.
- QTI is interchange, native PLE Question JSON is one unversioned internal source shape, and H5P,
  iMathAS, and WeBWorK remain behind the common Question Backend ownership model.
- Adapter guidance now retains only demonstrated source/Revision, opaque backend state, response,
  and outcome evidence. Current reproduction-detail, version, hash, and snapshot fields are legacy
  implementation evidence rather than a universal adapter requirement.
- Existing Course Banner upload, rendition, and delivery routes remain implementation evidence, but
  their former 6:1 hero and 5:2 card geometry no longer defines the product presentation.

## Implementation gaps preserved as evidence

The API map continues to show current `/assignments` paths and old DTO spellings next to the target
Assessment contract. The database and file-structure guides likewise name real legacy modules only
so a later code migration can find them. None is a compatibility requirement.

The generated screenshot atlas, Ribbon destination ledger, source graph, capture manifest, and image
corpus cannot be durably corrected from docs alone because their generators or rendered product live
outside `docs/`. They are classified in
[COMPLIANCE_SUMMARY.md](COMPLIANCE_SUMMARY.md) and
[UNRESOLVED_OR_AMBIGUOUS_ITEMS.md](UNRESOLVED_OR_AMBIGUOUS_ITEMS.md).
