# Human guidance

<!-- VENDORED HEADER: START -->
Record the durable guidance Neil Voss states, or approves for preservation here, in his own words:
first person or close paraphrase, one to three lines per bullet. Material he supplies as a source
may inform [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) once it is settled, and an entry of uncertain
origin belongs there too. Rules: [REPO_STYLE.md](REPO_STYLE.md).
[PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]
<!-- VENDORED HEADER: END -->

## Guidance Format

- Guidance bullets should start with the subject when practical. This makes the guidance easier to scan.
- Guidance should stay terse and in my own words.
- Uncertainty should be preserved when I have not made a final decision.

## Agent guidance

- Read and learn the core principles in docs/REPO_STYLE.md
- Time should be used efficiently. Agents and tokens are cheap; wall time is not.
- Hard work should be broken into small, independently completable tasks with one owner and one verification.
- Requirements should avoid being overly strict or using arbitrary numeric, timing, byte, or pixel equivalence gates.
- This codebase is not in production yet, no one is using it, so we can fix the design and not have to worry about legacy support. Use the pre-production state of the codebase to improve foundational schemas, contracts, abstractions, and ownership boundaries when that produces the stronger long-term system.
- Prioritize positive prompting. Small LMs often mishandle negative prompting and may flip negative instructions into positive actions, producing poor code and egregious results. Phrase instructions as "Do X" or "Use Y" whenever possible, rather than "Do not do W" or "You are not allowed to do Z." Things like 'leave git to the manager' is a negative prompt in disguise, it is better to not mention git, but just encourages small LMs. Avoid naming unwanted tools unless needed. Positive prompting plus omission is often stronger than a negative boundary.
- Classify one-time checks separately from permanent tests. Several checks are useful for proving the rebuild during implementation but may not deserve permanent residence in the suite. Use the checklist in docs/PYTEST_STYLE.md of what makes a permanent test, use it. Temporary tests are fine, but should not become permanent. When in doubt, remove the test.
- Finish the obvious. Continue while the next safe step is defined by the plan, implied by the current task, or required to verify the work. Stop at a real blocker: missing information that cannot be inferred from the repo or plan, a risky or irreversible action, or work that changes the user's requested outcome. When one option is clearly best, take it, document the assumption, and continue.

## Glossary

- **Blueprint Course**: A reusable course definition used to create **Course Instances**. It has no enrolled **Students** or deadlines.
- **Course Instance**: A course created from a **Blueprint Course**. It contains enrolled **Students**, deadlines, releases, and other delivery-specific settings.
- **Published Question**: A validated question that is part of the global question library and available to vetted **Instructors**.
- **Draft Question**: A private, unpublished question being developed by an **Instructor**. It must pass Question Publication Validation before joining the global question library.
- **Question Library**: The global collection of **Published Questions** available to vetted **Instructors**. All questions used in assignments are part of the library.
- **Sysadmin**: A god-level PLE administrator responsible for system administration, **Instructor** vetting, account creation, and helping **Instructors** manage courses.
- **Instructor**: A vetted user who teaches courses and can browse, reuse, create, fork, and publish question content.
- **Student**: A user enrolled in a **Course Instance** who completes assigned questions and other course activities.
- **Course Observer**: A read-only course participant who can view course content and assignment completion, but not individual **Student** scores.
- **Student Observer**: A read-only participant who can view the information associated with a particular **Student**.
- **Grader**: A planned course role for grading workflows. It is not currently needed because PLE does not use manual grading.

## Development philosophy

- Every source file should stay below 1000 lines. Split complete capabilities into focused modules.
- PLE is pre-production with no users or durable production data. Improve the design directly.
- Readable `snake_case` should be used whenever possible; see [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) for details.
- Adaptability should be a focus so the software can evolve as requirements and insights change.
- Dependency versions should be the latest because security bugs are continually fixed.
- Slow measured interface behavior may justify moving the hot path to Rust/WebAssembly.
- The polished Live Demo is the top priority; see [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md).
- The three major user types are **Sysadmins**, **Instructors**, and **Students**.
  - Exceptions are **Course Observers**, **Student Observers**, and **Graders**.
- PLE should use one installation with no institution boundaries.
- PLE accounts should be global within that installation and use passwordless email authentication.
- Email is not configured for the Live Demo yet; use the visible seeded-role entry for demo access.
- Project images and simulated live-stack data are disposable acceptance infrastructure.
- `./run_live_demo.sh` is the normal local-stack entry point. For direct controller
  diagnostics, use `source source_me.sh && python3 local_stack.py`.

## Interface philosophy

- Visual design should be pushed harder. Make the interface less bubbly and reduce excessive padding.
- Themes should use biome and habitat names, with names removed when their themes look substantially alike.
- UUIDs should never appear in visible page content, navigation URLs, or copyable links.
- Atkinson HyperLegible https://www.brailleinstitute.org/freefont/ is my favorite for written text and mononoki font https://madmalik.github.io/mononoki/ for monospace
- we should not have any upload capabilities for students and all instructor content is created via text boxes

## Data philosophy

- Answers, keys, grading, and correctness decisions should stay on the server, out of reach of **Students**.
- Public evidence should stay separate from private, answer-bearing, identifying, or radioactive FERPA material.
- Users log in only with a passkey or email code; no passwords. Higher-security exceptions will be considered for **Sysadmin** accounts.
- Human-readable titles and identifiers should be used wherever people must recognize, copy, or enter them.
- **Student** course data falls under FERPA; treat it as radioactive.
- A **Sysadmin** does not receive general access to FERPA course records.
- FERPA access should be scoped through exact course membership and **Student** ownership.
- Losing a passkey returns the user to email sign-in; do not add a separate recovery mode.
- **Student** data should be collected reluctantly, used deliberately, and purged predictably.
- **Course Instance** data defaults to notice after 30 days, archive after 100 days, and permanent deletion after 365 days.
- Course-owned assignment definitions should be kept when **Student** records are archived or deleted.
- Course work, attempts, submissions, and grades follow the course retention policy independently of the lifetime of the Student Account.

## Question philosophy

- Questions are subject agnostic. Questions must be properly tagged, but all are part of the same library.
- **Draft Questions** are kept private until publication so unfinished material does not reduce Question Library quality.
- **Draft Questions** must go through a validation process before being added to the library.
- Questions are strictly and deterministically automated; do not add manual grading.
- MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT Question Types should be supported.
- Versioned PLE flat-question JSON is the canonical machine format for simple static questions.
- QTI is for import, export, and archival interchange rather than the internal source model.
- WeBWorK and iMathAS are PLE-managed Question Backends. Use exact
  backend-specific terms when a concrete implementation or lifecycle matters.
- **Published Questions** use one copyable Crockford Base32 Question ID in the form `AAA-BBBB`. where the final character is a checksum.
- **Published Questions** maintain version history, so updates can be propagated to other courses.
- **Published Questions** should have a limit on the amount of change allowed, to avoid trolling or completely changing the content.
- **Published Questions** have two editing paths: moderate edits by the question owner and full forks by any **Instructor**.
- Moderate edits update the owner's **Published Question** while maintaining the original question authorship and CC licensing.
- Full forks create a separate **Draft Question** with its own authorship while maintaining the source question's CC licensing and attribution.
- Editing of a full fork happens in the **Instructor's** private **Draft Question**. Question Publication Validation is required before the fork joins the library as a **Published Question**.
- Assignments and grading evidence pin an exact version. A newer version becomes an available controlled update. It never silently changes issued or graded work. Not sure if a security or major flaw override makes sense here. Maybe overrides are only approved by a **Sysadmin**?
- Question writers may add Question Feedback when it helps. It remains optional Question-authored
  teaching content, and Student workflows remain complete whether or not Students read it.
- The platform is question agnostic, but for its initial run, the primary question formats/backends are the native flat question style PLE JSON (which is compatible with QTI) and WeBWorK; IMathAS and H5P are included but are considered secondary.

## Question library philosophy

- Question sharing, discovery, and reuse are a high-priority **Instructor** workflow.
- All **Published Questions** are public to vetted **Instructors**. By published, I mean part of the global question library.
- All questions in assignments are part of the global question library.
- **Students** cannot see the question library. They only see questions in their assignments.
- **Published Questions** are still discoverable even when used by a private **Course Instance**.
- Question stewardship should use a GitHub-like model.
- **Published Questions** can be forked and edited by any **Instructor**.
- **Published Questions** can be starred and watched, just like GitHub.
- Forks of **Published Questions** can be viewed by other **Instructors**.
- Anonymized counts are maintained for every **Published Question**: # attempts, # correct, and, for certain types of flat questions, # times each choice was selected.
- Star = favorite/visible endorsement. Every vetted **Instructor** can see the star count and which vetted **Instructors** starred the question.
- Watch = subscription. It drives the watching **Instructor's** in-app notifications for versions, forks, improvement threads, and impact notices; the watch list remains private unless you later choose otherwise.
- **Students** and anonymous users see neither the **Instructor** identity list nor watch state.
- Statistics are version-specific first: accepted graded attempt count, correct count, and eligible choice counts. Privacy-safe question-level rollups may combine versions only when clearly labeled and disclosure thresholds are met.
- **Published Questions** support four change paths: moderate edits, change proposals, full forks, and forced corrections.
- Moderate edits are made by the question owner and create a new immutable version in the same lineage.
- A **Change Proposal** can be submitted by any **Instructor** against an exact version. The owner reviews the validated proposal, and acceptance creates a new version in the same lineage with contributor credit.
- Full forks can be created by any **Instructor** as private **Draft Questions** and later published as separate lineages with source attribution.
- Forced corrections are audited **Sysadmin** actions reserved for critical flaws.
- **Change Proposals** must pass Question Publication Validation before submission and show their semantic and grading impact.
- A **Change Proposal** must be rebased or resubmitted if the question lineage advances before acceptance.
- Question authorship, contributor credit, history, and compatible CC licensing are preserved across edits, proposals, and forks.
- Assignments and graded work remain pinned to exact immutable versions and are never changed automatically by later revisions.

## Course content philosophy

- **Course Instances** are built from **Blueprint Courses**.
- **Blueprint Courses** are reusable course definitions and serve as blueprints for building courses.
- **Blueprint Courses** have no **Students** enrolled and no deadlines.
- **Blueprint Courses** are the same concept as LibreTexts' ADAPT alpha courses.
- A new assignment added to a **Blueprint Course** is added to its **Course Instances** as unreleased.
- A **Course Instance** is a course created from a **Blueprint Course**.
- **Course Instances** have **Students**, deadlines, releases, and other delivery-specific settings.
- **Blueprint Courses** are visible and reusable by every vetted **Instructor**.
- **Course Instances** are visible only to their current co-**Instructors** and enrolled **Students** because they contain delivery
  choices and FERPA-bearing activity.
- **Blueprint Courses** and **Course Instances** can only contain **Published Questions**.
- All **Course Instances** have a parent **Blueprint Course**.
- The reuse path lets an **Instructor** deliberately publish a **Course Instance's** reusable structure as a new **Blueprint Course** or propose controlled updates to its parent.
- A course can have multiple co-**Instructors** with equal teaching authority for that course.
- **Sysadmins** can create courses, but **Instructors** teach them. Every course must have an assigned **Instructor** who owns the course.

## Sysadmin philosophy

- A **Sysadmin** must be a god-level account:
  - **Instructor** vetting and account creation.
  - Help for non-tech **Instructors** fixing their courses, including **Students** and content.
- The human developer, Dr. Neil Voss, is the current **Sysadmin** and is also an **Instructor**.
- Neil will have two logins, one for Sysadmin and one for Instructor, so the user roles remain distinct
- Every **Instructor** is manually approved after validation that the **Instructor** is a real person.
- A **Sysadmin** does not receive general access to FERPA course records.
- Sysadmins stay out of Student rosters, grades, and other FERPA course records during normal
  operation. They may access them when helping an Instructor resolve a specific course problem.

## Instructor philosophy

- All vetted **Instructors** are equal.
- **Instructor** accounts are created once the **Instructor's** real identity is vetted by a **Sysadmin**.
- **Instructors** can browse and search the global question library.
- **Instructors** can browse the question content of all **Blueprint Courses**.
- **Instructor** and **Sysadmin** workflows should be designed for a 1280 by 800 desktop 16:10 aspect browser viewport.
- Pages should be composed around the teaching task, not a collection of individually padded components.
- **Instructors** log in only with a passkey or email code; no passwords.
- **Instructors** should have a clearly labeled, answer-free **Student** view without changing their identity.
- An **Instructor** can upload a small centered course banner and select a three-color theme.
- Blueprint Courses are the shared reusable course definitions.
- Every approved **Instructor** has the same product capabilities; course membership determines which
  course records each **Instructor** may use.
- students might use a iphone, chrome laptop, and windows desktop, so they could need multiple. If a student loses there login the instructor should be able to reset and send a new signup code

## Student philosophy

- **Student** workflows should be designed for laptop, portrait tablet, narrow-phone, and square displays.
- Every **Student** browser action should be usable with the keyboard alone.
- **Students** log in only with a passkey or email code; no passwords.
- **Student** data should be collected reluctantly, used deliberately, and purged predictably.
- **Student** course data falls under FERPA; treat it as radioactive.
- Student email addresses are immmutable. Even students who change their legal name do not usually get a new address during the semester and if they did I might just make them create a new account
- Student Accounts persist across courses and semesters.
- A Student Account is global and is not owned by or permanently tied to a Course Instance.
- When an Instructor uploads a roster, PLE uses the institutional email to find an existing Student Account or creates one when none exists.
- Each course creates its own course-scoped Student Record and enrollment relationship for that Student Account.
- Course work, attempts, submissions, and grades follow the course retention policy independently of the lifetime of the Student Account.

## Course observers, student observers, and graders philosophy

- Both observer types are read-only participants.
- **Course Observers** can see assignments and questions for a course and which **Students** have completed the assignments.
- **Course Observers** do not see scores.
- **Student Observers** can see everything about a particular **Student**. PLE will assume FERPA rights to the **Student** have been waived.
- **Graders** are not needed right now because we do not have manual grading.
- Course authorization should stay adaptable for future **Grader** and **Course Observer** relationships. A
  **Course Observer** receives anonymous aggregate grades without **Student**-level FERPA information.
