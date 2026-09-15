# Authorization And Ferpa Changes

## Scope

This is a fresh implementation-audit inventory. Each record below is an owning `[ ]`
Human Guidance bullet: the behavior is unverified or differs from the current implementation.
The bullet text is copied verbatim from the generated checklist, and its source location is
recorded beside it. Later duplicate bullets that carry an `Owner:` pointer are excluded because
their earlier owning record is the single inventory entry.

The authoritative exhaustive record is the
[generated checklist](../../audits/human_guidance_implementation_checklist.md).

## Owning inventory

### Accounts and roles -- Account rules

- Email is not configured for the Live Demo yet; use the visible seeded-role entry for demo access.
  - Source: `docs/HUMAN_GUIDANCE.md:91`

- **Students** are required to use their university or institutional (`.edu` in the USA) email accounts.
  - Source: `docs/HUMAN_GUIDANCE.md:94`

- **Sysadmin** accounts should require higher security than other accounts, like TOTP authentication
  - Source: `docs/HUMAN_GUIDANCE.md:95`

- Instructor Accounts may be deactivated without deleting their authored content, Course relationships, or historical records.
  - Source: `docs/HUMAN_GUIDANCE.md:99`

### Accounts and roles -- Instructor role

- A **Sysadmin** vets an Instructor's real identity before creating the Instructor Account.
  - Source: `docs/HUMAN_GUIDANCE.md:105`

- **Instructors** can browse the content of Public and Archived **Blueprint Courses**.
  - Source: `docs/HUMAN_GUIDANCE.md:108`

### Accounts and roles -- Student role

- An **Instructor** can reset Student login access and send a new signup code when needed.
  - Source: `docs/HUMAN_GUIDANCE.md:116`

- **Student** data should be collected reluctantly, used deliberately, and purged predictably.
  - Source: `docs/HUMAN_GUIDANCE.md:117`

- Student Course data falls under FERPA; treat it as radioactive.
  - Source: `docs/HUMAN_GUIDANCE.md:118`

- Roster import uses institutional email to find an existing Student Account or create one when needed.
  - Source: `docs/HUMAN_GUIDANCE.md:122`

- Student Work, Attempts, submissions, and grades follow Course retention independently of the Student Account.
  - Source: `docs/HUMAN_GUIDANCE.md:124`

- Removing a **Student** from a Course revokes future Course access but does not immediately delete the Student's Course records or Student Work.
  - Source: `docs/HUMAN_GUIDANCE.md:125`

- Student Work and grades remain subject to the normal Course retention policy after enrollment ends.
  - Source: `docs/HUMAN_GUIDANCE.md:126`

- Deactivating Course access does not delete the Student Account or Student Work.
  - Source: `docs/HUMAN_GUIDANCE.md:128`

- An **Instructor** can restore the Student's Course access later.
  - Source: `docs/HUMAN_GUIDANCE.md:129`

### Accounts and roles -- Sysadmin role

- A **Sysadmin** has full administrative authority over PLE.
  - Source: `docs/HUMAN_GUIDANCE.md:136`

- Sysadmins vet **Instructors** and create Instructor Accounts.
  - Source: `docs/HUMAN_GUIDANCE.md:137`

- Sysadmins can help Instructors repair Courses, Students, and content.
  - Source: `docs/HUMAN_GUIDANCE.md:138`

- **Sysadmins** have full platform-administration capability but do not automatically have access to FERPA Course records.
  - Source: `docs/HUMAN_GUIDANCE.md:141`

- Sysadmin support does not make the Sysadmin an **Instructor** or Course member.
  - Source: `docs/HUMAN_GUIDANCE.md:144`

### Data and history

- Answers, keys, grading, and correctness decisions should stay on the server, out of reach of **Students**.
  - Source: `docs/HUMAN_GUIDANCE.md:425`

- Public data should stay separate from private, answer-bearing, identifying, or radioactive FERPA data.
  - Source: `docs/HUMAN_GUIDANCE.md:426`

- Human-readable titles and identifiers should be used wherever people must recognize, copy, or enter them.
  - Source: `docs/HUMAN_GUIDANCE.md:427`

- FERPA-sensitive Student data should not become ordinary logs, analytics, URLs, or long-lived browser storage.
  - Source: `docs/HUMAN_GUIDANCE.md:428`

### Data and history -- Student and FERPA data

- FERPA access should be scoped through exact Course membership and **Student** ownership.
  - Source: `docs/HUMAN_GUIDANCE.md:435`

- Course work, Attempts, submissions, grades, and other FERPA-sensitive data follow the Course retention policy.
  - Source: `docs/HUMAN_GUIDANCE.md:438`

- Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
  - Source: `docs/HUMAN_GUIDANCE.md:439`

- **Student Work** is the collective term for FERPA-sensitive records created by a Student in a Course Instance.
  - Source: `docs/HUMAN_GUIDANCE.md:440`

- Student Work is an umbrella term; the underlying records retain their own identities and purposes.
  - Source: `docs/HUMAN_GUIDANCE.md:442`

- Student retention removes identifiable Student evidence, not privacy-safe aggregate Question statistics.
  - Source: `docs/HUMAN_GUIDANCE.md:443`

- Privacy-safe aggregate Question statistics remain after the underlying Student records are deleted.
  - Source: `docs/HUMAN_GUIDANCE.md:444`

- Aggregate Question statistics must not identify or allow reconstruction of individual Student activity.
  - Source: `docs/HUMAN_GUIDANCE.md:445`

### Data and history -- Course retention

- Course retention should follow Course Instance dates and its six-month Active lifetime rather than
  a fixed academic calendar.
  - Source: `docs/HUMAN_GUIDANCE.md:452`

- The latest Assessment deadline ends normal teaching and starts the Course Instance's FERPA
  retention clock.
  - Source: `docs/HUMAN_GUIDANCE.md:454`

- Creating or extending a later Assessment deadline may move those dates, but not beyond the
  six-month Active lifetime.
  - Source: `docs/HUMAN_GUIDANCE.md:456`

- Starting the FERPA retention clock does not itself notify, archive, hide, or delete Student data.
  - Source: `docs/HUMAN_GUIDANCE.md:458`

- The configured FERPA retention policy determines the later notice, archive, recovery, and
  permanent deletion transitions.
  - Source: `docs/HUMAN_GUIDANCE.md:459`

- PLE warns the **Instructors** before the Course Instance becomes Inactive six months after
  creation.
  - Source: `docs/HUMAN_GUIDANCE.md:461`

- The six-month Active limit prevents Course reuse or deadline extensions from indefinitely delaying
  FERPA retention and deletion.
  - Source: `docs/HUMAN_GUIDANCE.md:463`

- Course inactivity and FERPA deletion are separate transitions; becoming Inactive does not itself
  delete Student records.
  - Source: `docs/HUMAN_GUIDANCE.md:465`

- Retention should work equally for semesters, quarters, summer Courses, and other academic calendars.
  - Source: `docs/HUMAN_GUIDANCE.md:467`

- PLE should notify the **Instructor** before FERPA-sensitive Student data is archived.
  - Source: `docs/HUMAN_GUIDANCE.md:468`

- Archived Student data should leave normal Instructor and Student interfaces but remain recoverable during the retention period.
  - Source: `docs/HUMAN_GUIDANCE.md:469`

- FERPA-sensitive Student data should be permanently deleted when its retention period expires.
  - Source: `docs/HUMAN_GUIDANCE.md:470`

- FERPA retention intervals are operational configuration rather than separate product decisions.
  - Source: `docs/HUMAN_GUIDANCE.md:472`

### Data and history -- Retention processing

- A background process should periodically find Course Instances whose retention deadlines have passed.
  - Source: `docs/HUMAN_GUIDANCE.md:476`

- Retention decisions should come from stored Course dates and the Course Instance creation time.
  - Source: `docs/HUMAN_GUIDANCE.md:477`

- The background process should execute retention policy rather than define when retention periods begin or end.
  - Source: `docs/HUMAN_GUIDANCE.md:478`

- Running the retention process late should produce the same retention decision as running it on schedule.
  - Source: `docs/HUMAN_GUIDANCE.md:479`

- The retention process should be safe to run repeatedly.
  - Source: `docs/HUMAN_GUIDANCE.md:480`

### Data and history -- Revisions and history

- Be conservative about creating revisions.
  - Source: `docs/HUMAN_GUIDANCE.md:484`

- Published Questions, Question Pools, and Blueprint Courses have immutable revisions.
  - Source: `docs/HUMAN_GUIDANCE.md:486`

- Question, Question Pool, and Blueprint Revision Numbers start at 1 and increase sequentially for
  each object.
  - Source: `docs/HUMAN_GUIDANCE.md:489`

- A Revision Number identifies a specific immutable Revision stored by PLE.
  - Source: `docs/HUMAN_GUIDANCE.md:491`

- Student Work records the Question Pool Revision and selected Published Question Revision for each response.
  - Source: `docs/HUMAN_GUIDANCE.md:494`

## Count method

This report owns **55** checklist records. The count is the number of `[ ]` bullets
in the listed sections after excluding records with a later-duplicate `Owner:` pointer.
It is mechanically reconciled with the other topical inventories by the temporary report
generation check; it is not a permanent test.
