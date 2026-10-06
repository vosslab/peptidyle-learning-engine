# BiologyProblems.org specifications

This folder specifies how BiologyProblems.org content becomes reusable PLE content and base
Blueprint Courses. It does not make the source website itself the authority for PLE behavior.
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md) and [FALL_2026_PILOT.md](../FALL_2026_PILOT.md) remain
the primary authorities. The local source mirror is
`OTHER_REPOS/biology-problems-website/site_docs/`.

## Reading order

1. Read [BIOLOGY_PROBLEMS_SOURCE_SPEC.md](BIOLOGY_PROBLEMS_SOURCE_SPEC.md) for source selection
   and Question conversion.
2. Read [BIOLOGY_PROBLEMS_IMPORT_SPEC.md](BIOLOGY_PROBLEMS_IMPORT_SPEC.md) for the importer.
3. Read [BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md](BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md) for
   base Blueprint Course assembly.
4. Read the applicable Course file for its source inventory, then
   [question_specs_open_questions.md](../active_plans/decisions/question_specs_open_questions.md)
   for unresolved Course decisions.

## Source-course inventory

| Source course | Completeness | Course specification |
| --- | --- | --- |
| Biochemistry | Complete | [BIOCHEMISTRY_COURSE_SPEC.md](BIOCHEMISTRY_COURSE_SPEC.md) |
| Genetics | Complete | [GENETICS_COURSE_SPEC.md](GENETICS_COURSE_SPEC.md) |
| Molecular Biology | Partially complete | [MOLECULAR_BIOLOGY_COURSE_SPEC.md](MOLECULAR_BIOLOGY_COURSE_SPEC.md) |
| Biostatistics | Largely incomplete | [BIOSTATISTICS_COURSE_SPEC.md](BIOSTATISTICS_COURSE_SPEC.md) |
| Laboratory | Partially complete | [LABORATORY_COURSE_SPEC.md](LABORATORY_COURSE_SPEC.md) |
| Biotechnology | Selected Question collection; incomplete Course coverage | [BIOTECHNOLOGY_COURSE_SPEC.md](BIOTECHNOLOGY_COURSE_SPEC.md) |

[BiologyProblems.org](https://biologyproblems.org/) identifies Genetics and Biochemistry as its
two complete courses. The remaining subjects supply selected Question collections.

Completeness describes available BiologyProblems.org source content. It does not claim that the
content has been imported, that a base Blueprint Course exists, or that the Course is selected for
the Fall 2026 pilot. The first delivery includes base Blueprint Courses for Genetics,
Biotechnology, and Biochemistry. Molecular Biology and Laboratory come later. Neil's current
preference also defers Biostatistics because it is largely incomplete, as recorded in
[../FALL_2026_PILOT.md](../FALL_2026_PILOT.md).

## Related specifications

- [../BLUEPRINT_COURSE_IMPORT_API_SPEC.md](../BLUEPRINT_COURSE_IMPORT_API_SPEC.md) defines the
  reusable authenticated Course API.
- [../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md](../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md) and
  [../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md](../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md)
  define PLE Question import behavior.
- [../QUESTION_SPECS/QUESTION_POOL_SPEC.md](../QUESTION_SPECS/QUESTION_POOL_SPEC.md) defines
  Question Pools. A source problem set is not automatically a Question Pool.
- [../QUESTION_SPECS/QTI_INTERCHANGE_SPEC.md](../QUESTION_SPECS/QTI_INTERCHANGE_SPEC.md) defines
  QTI mapping. QTI is interchange, not PLE's runtime Question format.
