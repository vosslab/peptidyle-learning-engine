# Terminology and model changes

Temporary working report for the corpus-wide Human Guidance compliance pass.

## Canonical terminology changes

| Stale or overloaded term/model | Current term/model |
| --- | --- |
| Assignment as generic object | Assessment |
| Assignment editor / Policies | Assessment Question Editor / Assessment Properties Editor |
| Draft, Available, or Published Blueprint state | Private, Public, or Archived Blueprint Course |
| Course owner or primary Instructor | Equal current co-Instructor relationship |
| Course-owned Student identity | Global Student Account plus course-scoped Student Record and relationship |
| Response-level Attempt/finalization objects | Assessment Attempt with saved responses finalized by whole-Attempt submission |
| Stored score or regrade result | Immutable backend credit fraction plus current Assessment Question point value |
| Assessment, Course Schedule, Draft, retention, or receipt Revision | Current state plus Edit Number where concurrency needs it |
| `AAA-BBBB`, sequential, or type-encoded public Question ID | `AAAA-ZBBB`, seven random identity characters plus middle HMAC check character |
| Versioned or seeded native JSON | Private, unpublished, unversioned, static PLE Question JSON with no seed |
| QTI runtime model | Import, export, and archival interchange translated to PLE-managed Questions |
| Assessment Template containing Questions | Instructor-owned reusable settings with one Type and no Questions or Pools |
| Generic Student Assignments navigation | Coursework collectively; specific Assessment Type for each item |

Instructor Ribbon wording now follows Human Guidance exactly:

- Courses: My Blueprint Courses, My Active Courses, My Inactive Courses, Search Public Blueprint Courses.
- Questions: My Questions, My Draft Questions, Starred, Watched, Search Question Library, Browse Question Library.
- Assessments: Assessments Due Soon, My Assessment Templates.

## Intentional retained terms

Assignment remains correct inside Regular Assignment, Practice Question Assignment, and Bonus
Assignment. Source paths, SQL names, routes, test titles, archived plans, dated plans and audits,
changelogs, and generated evidence may retain older identifiers only when the surrounding document
labels them as implementation or historical evidence.

Published Question Pools retain stable public IDs and immutable Pool Revisions because the Pool
section of Human Guidance explicitly requires them. The general history summary's omission of Pools
is recorded as unresolved rather than used to delete that model.

Examples and comparisons now use Assessment-owned disclosure and the current Assessment editor
names. External projects may retain their own Assignment terminology when the text clearly labels
it as external vocabulary.
