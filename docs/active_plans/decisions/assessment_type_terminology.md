# Assessment Type terminology

## Status

Superseded and resolved by [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) on
2026-09-14. This file remains at its old plan path so links do not send future
agents to an obsolete open decision.

## Accepted model

**Assessment** is the shared structural root. PLE defines exactly five
Assessment Types:

- Regular Assignment
- Practice Question Assignment
- Bonus Assignment
- Quiz
- Exam

Assignment is used only inside those three Type names. Assessment Type states
the pedagogical purpose and supplies defaults. An Instructor may change
Assessment settings without changing the Type and cannot create additional
Types.

Blueprint Assessments and Course Instance Assessments use the same Types.
Assessment Templates are Instructor-owned reusable settings with one Type and
contain no Questions or Pools.

## Attempt and submission model

```text
Course Instance Assessment
  -> Assessment Attempt
      -> ordered Question and Pool selections
      -> complete saved responses
      -> whole-Assessment submission
      -> immutable backend credit fractions
```

The whole Assessment Attempt is the submission target, and that transition
finalizes its saved responses together. Internal implementation rows must not
become another public action or lifecycle.

## Scoring boundary

Question Backends return immutable credit fractions. PLE calculates scores from
those fractions and current Question point values. Human Guidance does not
define a universal multiple-Attempt grade-selection rule, Grade Categories, or
a Course Grade Scheme.

## Interface

Instructor primary navigation uses Assessments. Course composition uses
Assessment Question Editor and Assessment Properties Editor. Student work is
Coursework, and each item is labeled with its Assessment Type.
