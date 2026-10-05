# Product conflicts

This implementation audit did not leave a shipped behavior that contradicts one clear
Human Guidance rule.

The rows that still look like conflicts are two-sided readings. They are recorded in
[unresolved_or_ambiguous_items.md](unresolved_or_ambiguous_items.md):

- Public Blueprint sorts name Stars, Watches, and most recent edit. Those are not
  summary fields. The shipped sorts are name, adoptions, and students.
- Avatar text says the current image appears anywhere a user is represented. The Ribbon
  shows the signed-in Account's own image. Instructor Accounts shows another Account's
  gallery avatar.

Corrected behavior that earlier reviews treated as a conflict, and that the checklist
now verifies, includes these patterns:

- Public Blueprint results show the owner's verified display name as Author.
  Position 278 is decided. Institution stays off the row because one global
  installation has no institution boundaries.
- Support repair is an Instructor-issued, one-hour, audited read of one Student roster
  entry, one Course, or one Course Assessment. It does not grant membership or edit
  the record.
- Public IDs stay in canonical form across parsers, JSON, PostgreSQL, object paths,
  hashes, and the retention log. An internal UUID is not accepted as a public ID.
- PLE and WeBWorK share `BasicQuestionBackend`. iMathAS and H5P stay deferred.
- Sign out is only the Profile menu command.
- A fresh install provisions the Live Demo and the Genetics example by default.
- Large Question Library collections stay semantic record rows, with search, filters,
  and title sort.
- Support repair resource class is the enum `ple_data.support_repair_resource_class`.
- Frequent Instructor tasks are the linked Courses, Questions, and Assessments groups.
  Teaching Operations, Blueprint Updates, Course Setup, and Grade Settings stay future
  destinations and are not usable links.

Deferred product behavior is outside this audit. It is not a conflict with a current
rule.
