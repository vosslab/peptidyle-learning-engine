# BiologyProblems.org Course assembly

## Purpose

This specification assembles base Blueprint Courses from imported BiologyProblems.org content. A
Blueprint Course contains reusable Course content, not Students, deadlines, or relative schedules.
Course-specific files select source sections and record missing coverage. Start with one
Assessment per website topic because those topics already follow Neil's courses. Split long topics
into parts such as `topic03a` and `topic03b`. Hard Questions are usually bonus content. These are
starting choices, not a requirement to make every topic the same length or every source set a Pool.

Each base Blueprint uses its chosen Course Theme. A Course created from it starts with that Theme;
its Instructor can change the Theme afterward.

## Assembly inputs

For each Course, the assembly definition contains:

- the Course classification, short name, and long name;
- the selected source topic and problem-set paths;
- ordered module labels and ordered Blueprint Assessments;
- returned exact Published Question Revision references and any returned Pool references;
- the Assessment type, instructions, points, selection behavior, and reusable defaults;
- the Course color preference and proposed existing PLE Theme; and
- omissions, unselected source content, and any unresolved grouping or settings decision.

The source inventories in this folder are the human-readable starting point. Course assembly
needs the exact selected content and returned PLE references. Its input format belongs to
implementation work; this specification does not prescribe a second editable Question list.

## Assembly steps

1. Confirm that each selected Question was published successfully and has the Question ID and
   exact Revision returned by PLE.
2. Create a Blueprint Course through
   [../BLUEPRINT_COURSE_IMPORT_API_SPEC.md](../BLUEPRINT_COURSE_IMPORT_API_SPEC.md). PLE returns
   generated Blueprint Course, Module, and Assessment identities.
3. Save the complete ordered content using only returned Question and Pool references. Preserve
   each Question Revision selected by the Course definition.
4. Read the Blueprint Course back and compare its module order, Assessment order, entries, points,
   and reusable settings with the Course definition.
5. Publish only when the Course has its intended settled content and satisfies ordinary PLE
   publication rules. Publication status is separate from importing source content.

## Question Pools

Use a Pool only when the selected Published Questions are interchangeable under the Pool rules.
A Pool is an unordered set of Question Revision Tuples. Its editor can sort rows for review, but
that display order does not affect random selection. A direct Question and a Question selected
through a Pool can both appear in an Assessment when an Instructor deliberately constructs that
Assessment; Course assembly does not add speculative overlap gates.

## Open Course choices

The topic-based starting organization is settled. Exact splits, points, selection counts, and
reusable defaults remain ordinary Course content work. This is [Q02 in the central question log](../active_plans/decisions/question_specs_open_questions.md).
Each Course specification records available topics until a Course definition makes those choices
explicit.
