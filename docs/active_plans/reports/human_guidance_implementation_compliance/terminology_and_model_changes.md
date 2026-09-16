# Terminology And Model Changes

## Current heading reconciliation

Current status and retained evidence are recorded in [compliance_summary.md](compliance_summary.md)
and the [implementation checklist](../../audits/human_guidance_implementation_checklist.md).
Blueprint lifecycle/forks/comparison now belong to Course specifications (Part 08); Assessment
type appearance belongs to Instructor interface (Part 04). Earlier topical inventories are
historical context, not current wording, counts, source-line pointers, or ownership.

## Scope

This is a fresh topical implementation-audit inventory. It collects currently open
Human Guidance checklist records relevant to terminology and the model. The bullet text is copied
verbatim from the generated checklist, and its source location is recorded beside it. This report
does not establish exhaustive or disjoint topical coverage; the checklist remains the authority
for each record's status.

The authoritative exhaustive record is the
[generated checklist](../../audits/human_guidance_implementation_checklist.md).

## Topical inventory

### Product vocabulary

- **Blueprint Course**: A reusable course used to create **Course Instances**. It has no enrolled **Students** or deadlines.
  - Source: `docs/HUMAN_GUIDANCE.md:73`

- **Course Instance**: A course used for teaching. It has **Students**, deadlines, releases, and other course settings. It may be created from a Blueprint Course or started empty.
  - Source: `docs/HUMAN_GUIDANCE.md:75`

- **Published Question**: A validated question in the global **Question Library**, available to vetted **Instructors**.
  - Source: `docs/HUMAN_GUIDANCE.md:76`

- **Draft Question**: A private question being developed by an **Instructor**. It must pass validation before publication.
  - Source: `docs/HUMAN_GUIDANCE.md:77`

- **Question Library**: The global collection of Published Questions and published Question Pools available to vetted **Instructors**.
  - Source: `docs/HUMAN_GUIDANCE.md:78`

- **Sysadmin**: A PLE administrator who manages the system, approves **Instructors**, creates accounts, and helps manage courses.
  - Source: `docs/HUMAN_GUIDANCE.md:80`

- **Instructor**: An approved user who teaches courses and can browse, reuse, create, fork, and publish Questions.
  - Source: `docs/HUMAN_GUIDANCE.md:81`

- **Student**: A user enrolled in a **Course Instance** who completes Assessments and other course activities.
  - Source: `docs/HUMAN_GUIDANCE.md:82`

- **Assessment Question Editor**: The **Instructor** editor for selecting, adding, removing, and ordering Questions in an Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:83`

- **Assessment Properties Editor**: The **Instructor** editor for settings that apply to the whole Assessment, such as dates, scoring, attempts, late work, and what **Students** can see.
  - Source: `docs/HUMAN_GUIDANCE.md:84`
