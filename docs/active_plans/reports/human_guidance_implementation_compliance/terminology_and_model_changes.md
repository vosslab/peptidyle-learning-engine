# Terminology And Model Changes

## Current heading reconciliation

Current verbatim Human Guidance coverage is 998 bullets: 449 verified, 499 open
(490 owning), and 50 N/A, at SHA256
`e81d5bb0a63cfb7d3ca5f4f34287e5155dc9d20b0b91cb856fdbefa1ef6fa82b`.
The generated checklist owns occurrence status and current first-owner pointers. All nine part
gates, identity diff, and consistency pass for this snapshot. Unchanged scoring, timing,
Blueprint, terminal-Attempt, and bounded MATCH evidence is retained. Product compliance remains
unfinished; these gates establish inventory fidelity, not acceptance of the open requirements.

Part 01 now owns Product vocabulary and glossary, including its five topical subheadings;
new product definitions remain open absent independently accepted evidence. Part 03 owns Profile
avatar interface, Student avatars, and Instructor and Sysadmin Profile images. Account-creation
avatar persistence has a bounded source/SQL receipt, not deployed gallery/upload/cropping or
all-location acceptance. Part 06 owns the shared Content classification requirements; Part 07
owns Library metadata, Part 08 Course classification, and Part 09 Assessment classification.
The shared system requires exactly one Discipline per content object, optional narrower levels,
and a Subject associated with one or more Sysadmin-managed Disciplines. The prior single-parent
four-table foundation receipt does not satisfy this latest association shape or establish commands,
normalization, content attachments, hierarchical selection, or discovery. Those gaps remain open.
KISS/design constraints are audited N/A where not independently closable, but still bind reviews.

Accepted R-4 desktop/phone terminal receipts hide active navigation and visibly label three
no-response records Unanswered, incorrect `0 / 1`; the four exact MATCH pairs remain correct
`1 / 1`, total `1 / 4`. Native diagnostic `AZA01TD` / `R-5` follow-up saved/reloaded FIB, MA,
MULTI-FIB, NUM, and ORDER, then submitted the whole Attempt: four correct `1 / 1` responses,
deliberately partial-reordered ORDER incorrect `0 / 1`, total `4 / 5`. Practice-default permitted
correct answers are displayed separately from retained responses. The supplied ledgers and
`submitted-review-1280.png` / `submitted-review-390.png` under
`/private/tmp/ple-student-types-proof/` are bounded receipts, not all-eight-type, complete keyboard,
touch, or contrast acceptance. HOTSPOT and WeBWorK coverage remain open. Earlier topical
inventories/correction IDs and superseded contradictions below are historical provenance.

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

Latest Student response distinction rewrite: live HG requires visually distinct current Question,
saved-response status and keyboard focus, plus response-effect labels distinguishing Save/Clear/
change from whole Coursework submission. Both rows are open; earlier navigation styling receipts
are retained as partial proof, not blanket acceptance of native response controls/actions.
