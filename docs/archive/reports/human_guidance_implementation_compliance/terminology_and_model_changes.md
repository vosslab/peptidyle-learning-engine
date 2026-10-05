# Terminology and model changes

Implementation findings for product terms. Open readings are recorded once in
[unresolved_or_ambiguous_items.md](unresolved_or_ambiguous_items.md).

## Terms the running system uses

- Assessment is the generic object. Assignment appears in Assessment Type names.
- A Course covers Blueprint Courses and Course Instances. A Blueprint Course has no
  enrolled Students. A Course Instance is one teaching period.
- Public IDs are the canonical strings. Account, Course, Assessment, Blueprint, and
  Question parsers reject an internal UUID. Profile hides a UUID.
- One value that identifies one object is an Id. A composite identity is a Tuple.
  A WeBWorK source location is a Binding. No Reference type was added for that
  location.
- Question and Pool IDs use `XXXX-ZXXX`. Other public IDs use a prefix and no hyphen.
- Content classification is Discipline, Subject, Topic, and Subtopic, shared by
  Courses and Library Objects.
- Bloom teaching values are the six Cognitive Process names and the four Knowledge
  Dimension names. AI assignment of Bloom values stays deferred.
- Native PLE Question JSON stays private, unversioned, and unpublished.

## Open readings in this area

- Position 40 asks whether terminology alignment is the Terminology Contract plus
  its registered surfaces, or a proof over every column, field, key, and label.
- Position 538 asks whether Theme and provided-avatar tokens stay text vocabulary
  keys or become UUID primary keys.
- Position 546 asks whether Id, Tuple, and Binding are the simplest terms, or every
  remaining value still needs a naming judgment.

Development principles: 20 verified, 1 open, 30 not applicable. Data and history:
159 verified, 2 open, 0 not applicable.
