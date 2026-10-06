# Question classification specification

Library Objects use the same global vocabulary as Courses. The hierarchy is
Discipline -> Subject -> Topic -> Subtopic. Authority:
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#content-classification).

## Hierarchy and required values

| Level | Meaning | Required for Library Objects | Relationship |
| --- | --- | --- | --- |
| Discipline | Broad field, such as Biology | Exactly one | Sysadmin-managed vocabulary |
| Subject | Global area, such as Genetics | Exactly one | Associated with selected Discipline; may associate with several Disciplines |
| Topic | Major area, such as Chromosomal Inheritance | Optional | Belongs to one Subject |
| Subtopic | Narrow area, such as X-Linked Recessive Crosses | Optional | Belongs to one Topic |
| Tag | Flexible label outside the hierarchy | Zero or more | Does not replace any required hierarchy level |

A Subject has one global identity and a globally unique name. Biology/Genetics and
Chemistry/Genetics can refer to the same Subject if both associations exist. A Topic selected
under Genetics must belong to Genetics. A Subtopic cannot exist on an object without its Topic.
When a parent selection changes, revalidate or clear dependent selections rather than retaining
an impossible combination.

## Creating and selecting vocabulary

Sysadmins create and manage Disciplines. Instructors select Disciplines and may create Subjects,
Topics, and Subtopics as part of classification. When a proposed Subject name already exists,
offer the existing Subject. Require explicit acceptance before associating it with another
Discipline. This acceptance concerns a vocabulary relationship, not account vetting or content
publication approval.

Trim leading and trailing whitespace and apply consistent name validation. Subject, Topic, and
Subtopic length allowances increase as names become more specific. The exact shared vocabulary
constraints in [content_classification.sql](../../schemas/base_schema/20_tables/content_classification.sql)
are 1-120 characters for Discipline and Subject, 1-240 for Topic, and 1-480 for Subtopic,
with no control characters. These are implementation bounds; HG establishes their increasing
allowances rather than these numeric constants. Each Tag is nonempty trimmed text up to 120
characters; the shared write validation rejects duplicate Tags and control characters.

## Question and Pool ownership

A Question's hierarchy and Tags are fields on its complete Question Revision record. Permitted
edits update the current record in place, preserving its Revision Number, source, and grading content. A Pool starts with the first
member's Discipline and Subject; all members must match those two values.

The Pool independently selects Topic, Subtopic, and Tags. They need not match its members. Members
retain their own values. Pool search uses Pool values only. Moving a Question to another Subject
can leave a Pool mismatch even though the Pool still pins the same Question Revision. Apply
[QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md), including its already-released
Assessment rule.

## Discovery and examples

Search and Browse start with Discipline and narrow through the hierarchy. Subject selection may
explicitly include that Subject across other Disciplines. Such a choice relaxes only the
Discipline restriction, not the other active filters. See
[QUESTION_LIBRARY_FILTER_SPEC.md](QUESTION_LIBRARY_FILTER_SPEC.md).

- Valid: a Biology/Genetics Pool on Chromosomal Inheritance with members covering different
  Subtopics in Genetics.
- Invalid: adding a Biology/Biochemistry Question to that Pool, even if both have a `review` Tag.
- Valid: an object with Discipline and Subject, no Topic or Subtopic, and an empty Tag collection.
- Invalid: a Subtopic whose parent Topic differs from the object's Topic.

Classification describes content rather than its position in a textbook, Course, or Assessment.
Moving a Library Object between Assessments leaves its classification intact.
