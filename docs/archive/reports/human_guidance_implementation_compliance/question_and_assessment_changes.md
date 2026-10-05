# Question and assessment changes

Implementation findings for Questions, Pools, Backends, and Assessments. Open
readings are recorded once in
[unresolved_or_ambiguous_items.md](unresolved_or_ambiguous_items.md).

## Questions and backends

- PLE and WeBWorK implement `BasicQuestionBackend`. iMathAS and H5P stay deferred.
- Library search accepts ordinary words, quoted phrases, a minus exclusion, and
  PLE field tags. `enzyme -inhibitor` keeps enzyme and excludes inhibitor. A
  disposable library of 12,000 Questions narrowed to 6,000.
- Search, tag, Question type, license, and title sort run on a 13,000 Question
  import. Result lists use one server page of 50, 100, or 250 records.
- Watch copies reach active Accounts that have a verified Instructor display name.
- A public ID checksum is checked before a database lookup. A bad checksum does not
  open a connection.
- Large-import cleanup removes the temporary import rows after the library operation.

## Assessments

- An Assessment Attempt is submitted as a whole. Grading stores an immutable credit
  fraction. Point changes recalculate scores from that fraction.
- Unanswered Questions stay unanswered, receive zero credit, and are not sent to a
  Question Backend.
- Time limits, accommodations, reconnect, and expiry follow the server clock.
- Quizzes default to a more restrictive Attempt limit than Weekly Assignments.

## Open reading in this area

- Position 1028 asks whether a Quiz also needs a distinct collaboration control, or
  only that more restrictive Attempt limit.

Question specifications: 186 verified, 0 open, 7 not applicable.
Assessment specifications: 147 verified, 1 open, 1 not applicable.
