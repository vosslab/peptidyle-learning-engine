# Authorization And Ferpa Changes

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
Human Guidance checklist records relevant to authorization and FERPA. The bullet text is copied
verbatim from the generated checklist, and its source location is recorded beside it. This report
does not establish exhaustive or disjoint topical coverage; the checklist remains the authority
for each record's status.

The authoritative exhaustive record is the
[generated checklist](../../../active_plans/audits/human_guidance_implementation_checklist.md).

## Topical inventory

## Evidence updates

- C15's higher-security Sysadmin row is verified. Private TOTP credentials and browser-bound,
  expiring one-use attestations prevent generic stored-role Sysadmin session issuance. Accepted
  independent SQL and actual-server HTTP proof covered genuine valid TOTP success, pending
  no-session/protected denial, missing/wrong binding, bad codes, replay/expiry/counter reuse, and
  a five-attempt lock refusing a fresh unused valid counter. Ordinary Student/Instructor sessions
  and limited grants remain preserved. Artifacts:
  `/private/tmp/ple-sysadmin-session-boundary-artifacts.kSMr1H` and
  `/private/tmp/ple-sysadmin-session-boundary-http-artifacts.zexsoO`. Loopback HTTP is not deployed
  TLS evidence; broader Sysadmin authority and full authentication acceptance remain outside scope.

- Human-reference source review accepted typed `BP`, `CI`, `A`, and `U` parsing and browser/server
  read/use boundaries; `U` use is limited to authenticated Account-management paths in the reviewed
  code. This is not creation, randomness, collision-retry, or full authorization evidence, so the
  Human Guidance reference-ID behaviors remain open.

- C207 has accepted independent PostgreSQL 17 actual-API receipts for its two deadline-cap rows.
  All three Assessment save APIs and release acquire the Course first, reject a Due date beyond the
  immutable six-month Active cutoff, and synchronize the current maximum Due date. An active
  Course uses that maximum, or its Active cutoff when no Due date remains, as its retention anchor;
  archived and deleted anchors stay frozen while the current latest-Due fact remains current. The
  receipts covered stale CAS, wrong-Instructor denial, cap rollback, deterministic concurrent saves
  to two Assessments, an archive race, private-helper ACL denial, and the canonical Live Demo seed.
  This does not close retention notification, archive/delete processing, or broader FERPA-policy
  behavior. The ignored proof was removed after independent acceptance.

### Accounts and roles -- Account rules

- Email is not configured for the Live Demo yet; use the visible seeded-role entry for demo access.
  - Source: `docs/HUMAN_GUIDANCE.md:91`

- **Students** are required to use their university or institutional (`.edu` in the USA) email accounts.
  - Source: `docs/HUMAN_GUIDANCE.md:94`

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

- Starting the FERPA retention clock does not itself notify, archive, hide, or delete Student data.
  - Source: `docs/HUMAN_GUIDANCE.md:458`

- The configured FERPA retention policy determines the later notice, archive, recovery, and
  permanent deletion transitions.
  - Source: `docs/HUMAN_GUIDANCE.md:459`

- PLE warns the **Instructors** before the Course Instance becomes Inactive six months after
  creation.
  - Source: `docs/HUMAN_GUIDANCE.md:461`

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

Latest Student response distinction rewrite: live HG requires visually distinct current Question,
saved-response status and keyboard focus, plus response-effect labels distinguishing Save/Clear/
change from whole Coursework submission. Both rows are open; earlier navigation styling receipts
are retained as partial proof, not blanket acceptance of native response controls/actions.
