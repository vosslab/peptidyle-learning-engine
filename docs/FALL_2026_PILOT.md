### Fall 2026 teaching

The Fall 2026 semester started on August 31, 2026. Neil and his colleague Sarah Scheinmann are teaching five Courses that can provide real teaching content and use cases for PLE.

#### Neil's Fall 2026 Courses

Neil is teaching three Courses.

**BIOL 351/451-20 Genetics**

Course content:

https://biologyproblems.org/genetics/

Fall 2026 schedule:

https://vosslab.github.io/syllabus/fall_2026/genetics/SCHEDULE/

**BIOL 480 Biotechnology**

Course content:

https://biologyproblems.org/biotechnology/

Fall 2026 schedule:

https://vosslab.github.io/syllabus/fall_2026/biotech/SCHEDULE/

**BIOL 318/418-20 Biostatistics**

Course content:

https://biologyproblems.org/biostatistics/

Fall 2026 schedule:

https://vosslab.github.io/syllabus/fall_2026/biostats/SCHEDULE/

#### Sarah Scheinmann's Fall 2026 Courses

Sarah Scheinmann is teaching two Courses.

**BIOL 351/451-01 Genetics**

Course content:

https://biologyproblems.org/genetics/

**BIOL 355/455-01 Biochemistry**

Course content:

https://biologyproblems.org/biochemistry/

#### PLE production content

When PLE goes into production, use the course content from BiologyProblems.org as the source for these Courses:

https://biologyproblems.org/

The same content is available from the local mirror:

`OTHER_REPOS/biology-problems-website/site_docs/`

The local mirror may be useful when repository access makes the source content easier to inspect or import.

Questions explicitly labeled for **BIOL 301** are watered-down variants and are excluded from the Fall 2026 pilot.

#### Question source and import guidance

BiologyProblems.org contains Questions in several forms. Preserve the best available canonical source rather than treating every problem as a static Question.

For algorithmic WeBWorK Questions, preserve whether the canonical source is **PG** or **PGML**. Identify a Question as PGML only when its source is fully PGML-compliant. Otherwise, identify it as PG.

When parameterized PG or PGML source exists, use that canonical algorithmic source rather than generated static variants. One algorithmic Question remains one Published Question even when its Question Backend can generate many variants.

For simple static Questions, **PLE-native JSON** is the canonical internal machine format. Native JSON Questions are static rather than algorithmic.

BiologyProblems.org can export static Question content through **QTI**. For the Fall 2026 pilot, investigate using that QTI content to bring suitable BiologyProblems.org Questions into PLE-native JSON.

QTI serves as an interchange format for this content rather than the canonical PLE representation. The existing BiologyProblems.org and PLE tooling should guide the appropriate import path and transformations.

The initial primary Question Backends are **PLE-native JSON** and **WeBWorK**. WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
