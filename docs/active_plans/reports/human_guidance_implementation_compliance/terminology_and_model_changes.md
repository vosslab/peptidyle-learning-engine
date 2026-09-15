# Terminology And Model Changes

## Scope

This is a fresh implementation-audit inventory. Each record below is an owning `[ ]`
Human Guidance bullet: the behavior is unverified or differs from the current implementation.
The bullet text is copied verbatim from the generated checklist, and its source location is
recorded beside it. Later duplicate bullets that carry an `Owner:` pointer are excluded because
their earlier owning record is the single inventory entry.

The authoritative exhaustive record is the
[generated checklist](../../audits/human_guidance_implementation_checklist.md).

## Owning inventory

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

## Count method

This report owns **10** checklist records. The count is the number of `[ ]` bullets
in the listed sections after excluding records with a later-duplicate `Owner:` pointer.
It is mechanically reconciled with the other topical inventories by the temporary report
generation check; it is not a permanent test.
