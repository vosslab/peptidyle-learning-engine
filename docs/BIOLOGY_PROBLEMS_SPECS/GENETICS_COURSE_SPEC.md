# Genetics base Course

## Scope

Genetics is a complete post-general-biology inheritance source course. It includes Punnett-square,
gene-mapping, and deletion-mutant work. This is an inventory and mapping specification, not a
claim that the full Course has already been imported or grouped into final Assessments.

The source root is `OTHER_REPOS/biology-problems-website/site_docs/genetics/`. The Course color
preference is blue (`#1565c0` header/light accent and `#8ab4f8` dark accent). `Ocean` is the
proposed PLE Theme because it supplies blue surfaces and accents. Verify both appearances before
assigning it to the Course.

## Source inventory

| Source topic | Topic title | Intended coverage |
| --- | --- | --- |
| `topic01` | Genetic Disorders | Disorders and basic inheritance patterns. |
| `topic02` | DNA Structure | DNA composition, base pairing, and sequences. |
| `topic03` | DNA Profiling | Blood typing, genotyping, gels, and identification. |
| `topic04` | Mendelian Genetics | Crosses, Mendel's principles, and pedigrees. |
| `topic05` | Gene Interactions | Epistasis and multi-gene phenotypes. |
| `topic06` | Chromosomal Inheritance | Sex linkage and chromosomal inheritance. |
| `topic07` | Chi Square Analysis | Goodness-of-fit testing of cross data. |
| `topic08` | Gene Mapping | Recombination, map distance, and deletion mapping. |
| `topic09` | Chromosomal Disorders | Karyotypes, aneuploidy, and rearrangements. |
| `topic10` | Population Genetics | Hardy-Weinberg and evolutionary change. |
| `topic11` | Gene Trees | Phylogenetic interpretation. |

## Blueprint Course mapping

HG names Genetics as the shipped example Blueprint Course. The base Course draws from the listed
complete source inventory after ordinary import and review. The mapping below is deliberately
specific about what is settled: start with one Assessment per website topic, split long topics
into parts when useful, and usually use Hard Questions as bonus content.

| Input | Target | Status |
| --- | --- | --- |
| Listed Genetics topics except excluded BIOL 301 variants | Genetics Blueprint Course | Intended base Course and pilot source. |
| Selected source problem sets | Ordered modules and Assessments | One Assessment per topic as the starting organization; split long topics into parts. |
| Parameterized PG/PGML Questions | One WeBWorK Published Question per source | Required source mapping. |
| Suitable static Questions | Native JSON Published Questions | Required source mapping when selected. |
| Interchangeable selected Questions | Question Pool | Optional only after Pool validation. |

The Chapter 1 pilot teaching set contains four Genetics Questions in one Assessment: one WeBWorK
multiple-choice Question, one WeBWorK matching Question, one native JSON multiple-choice Question,
and one native JSON matching Question. It is a useful valid assembly example, not the final full
Course structure. See [../PILOT_CONTENT.md](../PILOT_CONTENT.md).

## Remaining Course content work

Use the topic-based organization in
[BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md](BIOLOGY_PROBLEMS_COURSE_IMPORT_SPEC.md): one Assessment
per topic to start, splitting long topics into parts and usually treating Hard Questions as bonus.


Exact topic splits, entry order, points, Pool use, and reusable defaults remain Course content work under
[Q02 in the central question log](../active_plans/decisions/question_specs_open_questions.md).
Course classification and Ocean Theme review remain ordinary Course-definition work.
