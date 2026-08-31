# Bloom taxonomy guide

This guide translates the Iowa State University Center for Excellence in Learning and Teaching
(CELT) Bloom's Taxonomy guide into the Question classification used by PLE. It helps Instructors
classify the cognitive work required by one exact Question Revision and helps the Bloom
Classification Assistant prepare a useful suggestion for Instructor review.

[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md#content-and-delivery-relationships) owns the
canonical names. [QUESTION_MODEL.md](QUESTION_MODEL.md#bloom-classification) owns the Question data
contract. This guide owns the practical teaching interpretation.

## Why use it

The revised Bloom taxonomy gives Instructors a shared language for aligning learning objectives,
instruction, and assessment. CELT recommends using it to sequence learning, align assessments with
the cognitive work students should perform, and track objectives across courses and programs.

PLE applies that purpose narrowly to Questions:

- Classify the work required for full credit on one exact Question Revision.
- Search the Question Library by either dimension or by their intersection.
- Review the balance of cognitive work across an Assignment or Course.
- Keep intended cognitive demand separate from observed Question Difficulty.

The classification describes the Question's assessed task in its teaching context. It does not
rank the Student, the Instructor, the Question Type, or the value of the Question.

## Two independent dimensions

The revised taxonomy has two independent dimensions. A Question receives one value from each.

| Dimension                   | Question answered                                      |
| --------------------------- | ------------------------------------------------------ |
| Cognitive Process Dimension | What cognitive work must the Student perform?          |
| Knowledge Dimension         | What kind of knowledge must the Student work with?     |

The two selected enum values determine one derived Question Bloom Classification:

```text
Question Bloom Classification =
  (Bloom Cognitive Process, Bloom Knowledge Dimension)
```

For example, Analyze and Conceptual Knowledge determine the classification commonly written as
"analyze conceptual knowledge." PLE stores the two enum values. Their ordered pair determines the
combined label and one position in the 4 by 6 Bloom Classification Matrix.

The phrases inside CELT's three-dimensional chart are example learning objectives. They illustrate
what a task in a matrix position might look like. Many different learning objectives and Questions
can occupy the same position, so those examples are reference guidance rather than a third
classification field.

## Cognitive process dimension

The Cognitive Process Dimension uses six closed values.

| Value      | Full-credit performance                                      | Reference hue |
| ---------- | ------------------------------------------------------------ | ------------- |
| Remember   | Retrieve relevant knowledge                                  | Blue          |
| Understand | Construct meaning from presented or recalled knowledge       | Green         |
| Apply      | Use a procedure in a situation                               | Yellow-green  |
| Analyze    | Separate material into parts and relate those parts          | Yellow        |
| Evaluate   | Make a judgment using stated or appropriate criteria         | Orange        |
| Create     | Assemble elements into a coherent or functional new whole    | Pink          |

These categories describe different kinds of cognitive work. Use the category the Question
actually requires. A sound course uses foundational and complex work where each serves the learning
goal.

Action verbs provide clues, but the complete task controls the classification. For example, "list"
may ask a Student to recall a supplied list or to analyze several sources and construct a new list.
Those tasks belong to different cognitive processes despite sharing a verb.

## Knowledge dimension

The Knowledge Dimension uses four closed values.

| Value                     | Primary knowledge used by the Question                   |
| ------------------------- | -------------------------------------------------------- |
| Factual Knowledge         | Terminology, specific details, and discrete elements     |
| Conceptual Knowledge      | Categories, principles, theories, models, and systems    |
| Procedural Knowledge      | Skills, algorithms, techniques, methods, and their use   |
| Metacognitive Knowledge   | Strategies and awareness of one's own cognition          |

The knowledge named in the prompt is not always the knowledge being assessed. A Question may use a
protein structure as presented material while assessing a procedure for evaluating binding sites.
In that case, Procedural Knowledge can be the primary dimension even though the visible subject is
protein structure.

## Derived matrix positions

Each example below shows one possible Question task for a position in the matrix. The examples are
original biology examples for classification practice. They are neither required wording nor
stored Question metadata.

### Remember

- Factual Knowledge: Name the four bases found in DNA.
- Conceptual Knowledge: Recognize a diagram as a model of the lac operon.
- Procedural Knowledge: Recall the ordered steps of a Western blot.
- Metacognitive Knowledge: Identify the study strategy that helped you distinguish amino acid
  classes.

### Understand

- Factual Knowledge: Summarize the defining features of a missense mutation.
- Conceptual Knowledge: Explain how negative feedback stabilizes blood glucose.
- Procedural Knowledge: Explain why an ELISA includes a wash step.
- Metacognitive Knowledge: Explain when a concept map is more useful than flash cards for this
  topic.

### Apply

- Factual Knowledge: Use supplied amino acid properties to determine the likely charge of a short
  peptide.
- Conceptual Knowledge: Use Mendel's law of segregation to predict possible gametes.
- Procedural Knowledge: Calculate allele frequency from a set of genotype counts.
- Metacognitive Knowledge: Choose and use a checking strategy for a multistep pedigree problem.

### Analyze

- Factual Knowledge: Separate observations supporting a pathogenic variant from unrelated case
  details.
- Conceptual Knowledge: Relate changes in Km and Vmax to competitive and noncompetitive inhibition.
- Procedural Knowledge: Identify the experimental step that introduced bias into a sequencing
  workflow.
- Metacognitive Knowledge: Diagnose which part of your problem-solving approach caused a genetics
  error.

### Evaluate

- Factual Knowledge: Check whether a protein annotation agrees with the listed sequence features.
- Conceptual Knowledge: Judge which inheritance model best explains a pedigree using stated
  criteria.
- Procedural Knowledge: Evaluate whether a PCR control plan distinguishes contamination from
  amplification failure.
- Metacognitive Knowledge: Evaluate which study strategy produced the most reliable recall and
  explain why.

### Create

- Factual Knowledge: Construct a concise reference table from supplied amino acid properties.
- Conceptual Knowledge: Build a model connecting mutation, protein structure, and phenotype.
- Procedural Knowledge: Design an experiment to test whether a variant changes enzyme activity.
- Metacognitive Knowledge: Create a personal strategy for checking assumptions in quantitative
  biology problems.

## Classification workflow

Classify the exact performance required for full credit:

1. Read the complete Question Prompt, Question Response Format, Answer Key, scoring criteria, and
   any rubric.
2. Identify the primary knowledge the Student must use and select the Knowledge Dimension.
3. Identify the primary cognitive work the Student must perform and select the Cognitive Process
   Dimension.
4. Check the ordered pair against the actual grading requirement and the expected prior learning.
5. For a Question with several tasks, select the pair representing the dominant full-credit work.
6. Send a genuinely co-dominant or ambiguous Question to the Instructor with the candidate pairs
   and a concise reason for the uncertainty.

Course context matters. Following a practiced algorithm may be Apply in one course, while choosing
or adapting an unfamiliar method may require Analyze, Evaluate, or Create in another. The exact
Question text alone may therefore be insufficient without its intended audience and expected prior
learning.

## Assistant and Instructor roles

The Bloom Classification Assistant evaluates one exact Draft Question Revision and fills a Bloom
Classification Suggestion before Instructor review. A useful review presentation shows:

- the suggested Cognitive Process Dimension;
- the suggested Knowledge Dimension;
- the task evidence that supports each selection; and
- any uncertainty caused by multiple scored tasks or missing teaching context.

The Instructor keeps or changes either independent value. Publication stores the accepted pair on
the immutable Question Revision. A later classification correction follows the ordinary Question
Revision and Reason for Edit workflow.

## Search and reporting

Question Search exposes two independent facets:

- Cognitive Process Dimension;
- Knowledge Dimension.

The interface may also show a 4 by 6 matrix. A matrix selection applies both facets together. The
matrix position, combined label, and result count are derived from the same two values rather than
maintained as separate Question metadata.

Report both dimensions when precision matters. "Analyze / Conceptual Knowledge" communicates more
than a one-dimensional phrase such as "Bloom level 4." The pair also prevents the cognitive process
from being mistaken for Question Difficulty.

## Color use

CELT's three-dimensional guide assigns one hue family to each Cognitive Process column. PLE uses
the following sampled anchors as design references:

| Cognitive process | Hue          | Reference anchor |
| ----------------- | ------------ | ---------------- |
| Remember          | Blue         | `#64A4D9`        |
| Understand        | Green        | `#A2D4B4`        |
| Apply             | Yellow-green | `#B9D438`        |
| Analyze           | Yellow       | `#E7E028`        |
| Evaluate          | Orange       | `#E8A264`        |
| Create            | Pink         | `#E3759F`        |

Interface colors reinforce the Cognitive Process label. Every field, filter, matrix position, and
legend also displays text. Interface owners derive accessible light-mode, dark-mode, border, text,
focus, and selected-state tokens from the reference anchors.

Color represents Cognitive Process only. Knowledge Dimension remains the labeled second axis, and
color does not represent correctness, Question Difficulty, point value, or mastery.

## Review checklist

- [ ] The exact Question Revision has one Cognitive Process value.
- [ ] The exact Question Revision has one Knowledge Dimension value.
- [ ] The combined classification and matrix position are derived from that ordered pair.
- [ ] The full-credit task, rather than a command verb alone, supports the pair.
- [ ] Expected prior learning and course context support the selected cognitive process.
- [ ] The classification remains separate from Question Difficulty and Question Type.
- [ ] Every color-coded presentation also displays both dimension labels.
- [ ] The Instructor has accepted or edited the suggestion before publication.

## Sources

- [Iowa State University CELT Bloom's Taxonomy](https://celt.iastate.edu/prepare-and-teach/design-your-course/blooms-taxonomy/)
  explains the course-design, assessment-alignment, and program-tracking uses of the revised
  taxonomy.
- Anderson, L. W., and Krathwohl, D. R., editors. 2001. *A Taxonomy for Learning, Teaching, and
  Assessing: A Revision of Bloom's Taxonomy of Educational Objectives.* Longman.
- Rex Heer's Iowa State University CELT three-dimensional model, updated January 2012, provides the
  two-axis presentation, process-column color families, and example-objective structure. The image
  identifies its license as Creative Commons Attribution-NonCommercial-ShareAlike 3.0 Unported.
  This document supplies an original PLE explanation and original biology examples.
