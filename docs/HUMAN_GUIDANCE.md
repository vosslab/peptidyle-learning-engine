# Human guidance

<!-- VENDORED HEADER: START -->
Record the durable guidance Neil Voss states, or approves for preservation here, in his own words:
first person or close paraphrase, one to three lines per bullet. Material he supplies as a source
may inform [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) once it is settled, and an entry of uncertain
origin belongs there too. Rules: [REPO_STYLE.md](REPO_STYLE.md).
[PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]
<!-- VENDORED HEADER: END -->

## Guidance Format

- Guidance bullets should start with the subject when practical, making them easier to scan.
- Guidance should stay terse and in my own words.
- Uncertainty should remain when I have not made a final decision.
- This document uses GitHub Flavored Markdown (GFM).
- Bullet duplication is acceptable because many agents only skim read one section at a time.

## General Development Agent guidance

- Read and learn the core principles in docs/REPO_STYLE.md
- Apply the Keep It Simple, Stupid (KISS) philosophy aggressively.
- Time should be used efficiently. Agents and tokens are cheap; wall time is not.
- Hard work should be broken into small, independently completable tasks.
- Write plans in plain, concrete language. Use technical terms when they add precision.
- Prioritize positive prompting. Avoid naming unneeded tools. Positive prompting plus omission is better.
- Small LMs mishandle negative prompting and flip negative instructions producing poor code and egregious results.
- Classify one-time checks separately from permanent tests.
- Finish the obvious. Continue while the next safe step is defined by the plan, implied by the current task.
- Robust means the software continues to function despite imperfect inputs, data, state, or behavior.
- Treat tests as liabilities as well as assets. Keep only requirements and gates grounded in actual needs.
- Plans should be finishable by the manager and subagents without additional human interaction.
- Prefer more small, independently verifiable milestones over a few large milestones.
- Fix the design that causes a problem rather than adding a workaround for its symptom.
- Prefer durable long-term fixes when the additional cost is justified.
- Prefer adaptable boundaries and simple domain concepts over speculative edge-case machinery.
- Stay focused on the requested work. Complete the required work and avoid adding unplanned functionality.
- Every source file should stay below 1000 lines. Split complete capabilities into focused modules.

## Codebase Development Agent guidance

- PLE is pre-production with no users. Fix the design directly rather than preserving legacy behavior.
- A fresh production installation includes the complete Live Demo by default.
- Treat the initial course content as shipped examples.
- BiologyProblems.org content is free and open source.
- The Genetics Blueprint Course from BiologyProblems.org ships as the example course.
- Use SQL directly to create the initial PostgreSQL database.
- Before production, edit the main database design directly as the design changes.
- After production, update existing databases without rebuilding them from scratch.
- All Podman content on the Mac-Studio-36G machine belongs to this project.
- Neil pre-approves pruning Podman images, volumes, and containers on Mac-Studio-36G as needed.

## Glossary

- **Blueprint Course**: A reusable course used to create **Course Instances**. It has no enrolled **Students** or deadlines.
- **Blueprint Revision**: A fixed version of a **Blueprint Course** preserved so its content cannot change.
- **Course Instance**: A course created from a **Blueprint Course** for teaching. It has **Students**, deadlines, releases, and other course settings.
- **Published Question**: A validated question in the global **Question Library**, available to vetted **Instructors**.
- **Draft Question**: A private question being developed by an **Instructor**. It must pass validation before publication.
- **Question Library**: The global collection of **Published Questions** available to vetted **Instructors**. Assignment questions come from this library.

- **User Roles**:
  - **Sysadmin**: A PLE administrator who manages the system, approves **Instructors**, creates accounts, and helps manage courses.
  - **Instructor**: An approved user who teaches courses and can browse, reuse, create, fork, and publish Questions.
  - **Student**: A user enrolled in a **Course Instance** who completes Assignments and other course activities.

- **Assignment Question Editor**: The **Instructor** editor for selecting, adding, removing, and ordering Questions in an Assignment.
- **Assignment Properties Editor**: The **Instructor** editor for settings that apply to the whole Assignment, such as dates, scoring, attempts, late work, and what **Students** can see.

## Development philosophy

- PLE is pre-production with no users or durable production data. Improve the design directly.
- Use readable `snake_case` whenever possible; see [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md) for details.
- Adaptability should be a focus so the software can evolve as requirements and insights change.
- Cargo, Node, and PyPI dependencies should use the latest versions to include security fixes.
- If an interface is measured as too slow, consider moving the slow code to Rust/WebAssembly.
- The polished PLE Live Demo is the top priority; see [LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md).
- PLE should use one global installation with no institution boundaries.
- PLE accounts should be global across PLE and use passwordless passkeys and email authentication.
- Project images and simulated live-stack data are disposable acceptance infrastructure.
- `./launchers/run_live_demo.sh` is the normal local-stack entry point. For direct controller
  diagnostics, use `source source_me.sh && python3 local_stack.py`.

## User account design

- Email is not configured for the Live Demo yet; use the visible seeded-role entry for demo access.
- The three major user types are **Sysadmins**, **Instructors**, and **Students**.
- Potential future user roles are **Course Observers**, **Student Observers**, and **Graders**.
- **Students** are required to use their university or institutional (`.edu` in the USA) email accounts.
- **Sysadmin** uses tomato red as its role color.
- **Instructor** uses teal green as its role color.
- **Student** uses lavender /purple as its role color.
- Role colors should be used consistently in role labels and other appropriate interface cues.
- Demo role selection should clearly state both the user's role and name.
- **Sysadmin** accounts should require higher security than other accounts, like TOTP authentication

### Course interface

- A **Blueprint Course** should provide an obvious action for creating a **Course Instance** from it.
- Blueprint Course editing should follow Course Editor -> Assignment Editor.
- The Course Editor should show the Course structure without editing every Question on one page.
- Selecting an Assignment in the Course Editor opens the editor for that Assignment.
- Only the selected Assignment's Questions should appear in its Assignment Editor.
- Creating a Course Instance from a Blueprint Course preserves its Assignments, Questions, pools, and settings.
- Assignments created from a Blueprint Course start unreleased with dates unset.
- Blueprint Courses should not contain relative Assignment schedules.

## Interface philosophy

- Design around what users need to find and do.
- PLE often presents large collections where users need to find a few relevant items.
- Optimize large collections for scanning, searching, filtering, and comparison.
- Important information should stand out from supporting information.
- Related information should be visually grouped and aligned.
- Dense pages should still be easy to scan.
- Use spacing to separate meaningful groups, not simply to make pages feel spacious.
- Use containers and borders only when they clarify structure.
- Prefer alignment and dividers over nested cards when they communicate the structure clearly.
- Show enough information at once to support comparison without excessive scrolling.
- Search and filters should help users quickly narrow large collections.
- Secondary details should not compete visually with the main task.
- Interface density should serve finding information, not density for its own sake.
- Dream big on the UI. Choose one visual philosophy and carry it through the entire interface.
- Visual design should be information dense, less bubbly, and use less unnecessary padding.
- Visual design should be compact and flat. Minimize padding, rounded containers, nested boxes, and unused space.
- Prefer dividers, alignment, and typography over cards and boxes for grouping related content.
- Pages should show substantially more useful content without scrolling when practical.
- Themes should use biome and habitat names, such as Forest, Grassland, Ocean, and Desert.
- UUIDs should never appear in visible content, navigation URLs, or copyable links.
- Use [Atkinson Hyperlegible Next](https://www.brailleinstitute.org/freefont/) as the main PLE font.
- Use [Atkinson Hyperlegible Mono](https://www.brailleinstitute.org/freefont/) for code and other monospace text.
- Prefer the official Braille Institute font files and include the needed weights locally with PLE.
- Question Backend-rendered content may use its own fonts when needed for correct display.
- Students should have no upload capabilities. Instructor-created content should use text boxes.
- Deep pages should use breadcrumbs to make the current Course, Assignment, or other context clear.

### User top bar

- All signed-in users share the same basic top bar layout.
- The PLE logo and product name appear at the upper left.
- Clicking the PLE logo or product name returns the user to their home dashboard.
- Each role has a clearly defined home dashboard.
- Product Role appears once, next to the PLE name.
- Role-specific navigation appears between the product identity and Profile.
- Navigation choices remain in consistent locations as users move between pages.
- Profile appears at the far right as an icon-only avatar.
- Clicking the Profile avatar opens the Profile menu.
- The Profile menu contains Profile settings, account settings, and Sign Out.
- Sign Out belongs in the Profile menu rather than the main top bar.
- The Profile avatar uses a generic user avatar until the user selects another avatar.
- Students select Profile avatars from a PLE-provided collection and cannot upload images.
- Student avatar selection should be visual and playful, similar to choosing a LEGO avatar.
- Instructors and Sysadmins may select a provided avatar or add their own Profile image.
- The current avatar appears consistently anywhere PLE represents that user.

### Breadcrumbs

- All signed-in accounts use the same permanent breadcrumb row below the top bar.
- The breadcrumb row keeps the same space even when there is only one breadcrumb level.
- Breadcrumbs show the path from the user's home dashboard to the current page.
- Each breadcrumb level links back to its corresponding page.
- Breadcrumbs use human-readable names rather than internal identifiers.
- Course and Assignment breadcrumbs preserve the current Course context.
- The permanent breadcrumb row keeps page content from moving up or down as users navigate.

## Instructor interface

- The Instructor interface should make frequent teaching tasks fast and easy to find.
- The Instructor menu has **Courses**, **Questions**, and **Assignments** in one dense top bar.
- Instructor Profile uses a generic user icon until the **Instructor** adds a Profile image.
- All required ribbon choices remain visible even when their collection is empty.

### Courses interface

- The **Courses** ribbon must include: My Blueprint Courses, My Active Courses, My Inactive Courses, Search Public Blueprint Courses.
- Course lists should support scanning and comparison without opening each Course.

#### Blueprint Courses

- **My Blueprint Courses** should emphasize reusable course design rather than teaching activity.
- **Search Public Blueprint Courses** helps Instructors find a Blueprint Course they already have in mind.
- Public Blueprint Course search should support quickly narrowing a large collection.
- A **Blueprint Course** should provide an obvious action for creating a **Course Instance** from it.
- Blueprint Course editing should follow Course Editor -> Blueprint Assignment Editor.
- The Course Editor should show the Blueprint Course structure without editing every Question on one page.
- Selecting a Blueprint Assignment in the Course Editor opens the editor for that Blueprint Assignment.
- Only the selected Blueprint Assignment's Questions should appear in its editor.
- **Blueprint Assignment Question Editor**: Selects, adds, removes, and orders Questions in a Blueprint Assignment.
- **Blueprint Assignment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see.
- Blueprint Courses should not contain Assignment dates or relative Assignment schedules.
- Blueprint Courses follow the lifecycle **Private -> Public -> Archived**.
- New and forked Blueprint Courses start **Private**.
- Private Blueprint Courses are visible only to their owner.
- Instructors may develop and use Private Blueprint Courses without publishing them.
- Making a Blueprint Course **Public** adds it to the shared Blueprint Course collection.
- A Public Blueprint Course with no adoptions may return to **Private**.
- A Public Blueprint Course with one or more adoptions remains **Public**.
- Archived Blueprint Courses leave normal discovery but remain available where needed for history.
- Instructors may fork a Public Blueprint Course to continue development privately.
- Blueprint Courses do not have a separate Draft state.

#### Course Instances

- **My Active Courses** should emphasize Course Instances the Instructor is currently teaching.
- Active Course Instances should make upcoming Assignments and important course activity easy to find.
- **My Inactive Courses** should keep past Course Instances available without competing with active Course Instances.
- Creating a Course Instance from a Blueprint Course preserves its Assignments, Questions, pools, and settings.
- Assignments created from a Blueprint Course start unreleased with dates unset.
- Course Instance Assignments have two editors:
  - **Assignment Question Editor**: Selects, adds, removes, and orders Questions in an Assignment.
  - **Assignment Properties Editor**: Controls dates, scoring, attempts, late work, and what **Students** can see.

### Questions interface

- The **Questions** ribbon must include: My Questions, My Draft Questions, Starred, Watched, Search Question Library, Browse Question Library.
- **My Questions** should make the Instructor's Published Questions easy to find and manage.
- **My Draft Questions** should emphasize Questions that still need work before publication.
- **Starred** should provide a quick personal collection of Questions the Instructor wants to keep handy.
- **Watched** should help Instructors follow Questions where changes or activity matter to them.

#### Search Question Library

- **Search Question Library** helps Instructors find specific Questions in a large library.
- Search should begin with a prominent search box, similar to Google Search.
- The initial Search page should stay simple and focus attention on entering a search.
- Search should assume the Instructor has some idea what they want to find.
- Question Library search should work well with ordinary words by default.
- Search results should switch to a dense, information-rich layout.
- Results should make it easy to scan many Questions quickly.
- Results should show the information needed to judge relevance without opening each Question.
- Search results should support filters for narrowing the Question Library.
- Filters should update the current search rather than start a separate workflow.
- Search should support Google-like syntax for more precise queries.
- Quoted text should search for an exact phrase.
- A minus sign should exclude matching terms.
- Search should support PubMed-like field tags such as `topic:genetics`.
- Field tags should use PLE concepts and vocabulary.
- Useful fields may include subject, topic, tags, Question Type, and author.
- Simple and advanced searches should use the same search box.
- Instructors should not need to learn search syntax to use Search Question Library.
- The interface should make useful search syntax discoverable when needed.
- Search syntax should help expert users quickly narrow a very large Question Library.
- Search terms and active filters should remain visible while reviewing results.
- Clearing or changing part of a search should be quick.
- Opening a result and returning should preserve the Instructor's search and position.

#### Browse Question Library

- **Browse Question Library** helps Instructors explore Questions without knowing what to search for.
- Browse should help Instructors understand what the Question Library contains.
- Browse should emphasize subjects, topics, tags, Question Types, and other useful groupings.
- Browse should make moving from broad subjects to narrower topics easy.
- Browse should show useful counts where they help Instructors choose where to explore.
- Browse results should use the same dense Question presentation used by Search where practical.
- Instructors should be able to move from browsing into a more focused search.
- Search and Browse are different paths into the same **Question Library**.

### Assignments interface

- The **Assignments** ribbon must include: Assignments Due Soon, My Assignment Templates.
- **Assignments Due Soon** should emphasize Assignments that may need the Instructor's attention.
- Assignment lists should make Course, release status, due date, and other important state easy to scan.
- **My Assignment Templates** should emphasize reusable Assignment design rather than Course activity.
- Assignment editing has two editors:
  - **Assignment Question Editor**: Selects, adds, removes, and orders Questions.
  - **Assignment Properties Editor**: Controls dates, scoring, attempts, late work, and other Assignment settings.
- The two Assignment editors should remain clearly distinct.
- The Assignment Question Editor should make Question order easy to understand at a glance.
- Adding Questions should provide direct paths to Search and Browse Question Library.
- Instructors should be able to inspect a Question before adding it to an Assignment.
- Assignment Properties should group related settings so important settings are easy to find.
- Instructors can randomize Question order for an Assignment.
- Answer-choice randomization belongs to the Question, not the Assignment.

### High-consequence actions interface

- Danger Zone contains **Assignment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
- Danger Zone should be visually separate from ordinary editing actions.
- Assignment Unrelease should explain that Student work will be deleted.
- Assignment Unrelease should require typing the Assignment title before confirmation.
- Archive actions should explain the effect on shared availability and require a clear confirmation.
- Restore actions should use ordinary availability controls.

### Student interface

- The Student interface should focus on current Courses, Assignments, and work that needs attention.
- The Student interface should make the next useful action easy to find.
- The Student menu is simpler than the Instructor menu.
- Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.
- Every Student browser action should be usable with the keyboard alone.
- Student pages should use Student-facing language and names meaningful to Students.
- Student navigation and pages should contain only Student interfaces and capabilities.
- Students enrolled in one active Course should go directly into that Course.
- Students should be able to see their active Courses and Assignments from the main navigation.
- Course pages should make upcoming, available, completed, and missed Assignments easy to distinguish.
- Assignment lists should make due dates and completion status easy to scan.
- Students should see the Assignment title, Question count, points possible, time limit, and previous Attempts before starting.
- Students only ever see one Question at a time while taking Assignments, quizzes, and exams.
- While taking an Assignment, navigation should show every Question, its saved status, and allow Students to jump directly between Questions.
- Leaving a Question and returning should preserve its saved response.
- The current Question and overall Assignment progress should remain easy to see.
- The Assignment timer should be subtle and keep the focus on the Questions.
- While taking a timed Assignment, the remaining time should stay visible while moving between Questions.
- Submission status should be obvious and use plain language.
- Scores and feedback should appear where the Assignment settings allow them.
- Completed Assignments should remain easy to find and review.
- Student content entry should use the response controls provided by Questions and other Student activities.

### Sysadmin interface

- The Sysadmin interface should focus on system administration.
- The Sysadmin menu should make Accounts, Instructors, Courses, and system configuration easy to find.
- Sysadmins should be able to find users quickly by name or email.
- Account lists should support searching, filtering, and scanning large numbers of users.
- User pages should clearly show role, account status, and other important administrative information.
- Sysadmins create accounts and manage account access.
- Sysadmins approve Instructors before they receive Instructor capabilities.
- Instructor approval status should be easy to find and change.
- Sysadmins should be able to find and inspect Courses across the installation.
- Course administration should show the Instructor and important Course status information.
- Sysadmins should manage Courses through Sysadmin interfaces and capabilities.
- System-wide settings should have their own area, separate from user and Course administration.
- Everyday navigation should emphasize frequently used administrative tasks.
- Rare installation and configuration tasks should remain available through secondary navigation.
- High-consequence administrative actions should have a visually distinct area.
- Confirmation for destructive actions should clearly state what will happen.

-----

## Data philosophy

- Answers, keys, grading, and correctness decisions should stay on the server, out of reach of **Students**.
- Public data should stay separate from private, answer-bearing, identifying, or radioactive FERPA data.
- Human-readable titles and identifiers should be used wherever people must recognize, copy, or enter them.

### Student and FERPA data

- Student retention removes identifiable Student evidence, not privacy-safe aggregate Question statistics.
- **Student** course data falls under FERPA; treat it as radioactive.
- **Student** data should be collected reluctantly, used deliberately, and purged predictably.
- FERPA access should be scoped through exact Course membership and **Student** ownership.
- **Sysadmins** receive only the FERPA access required for a specific administrative task.
- Student Accounts persist independently of Course data and Course retention.
- Course work, Attempts, submissions, grades, and other FERPA-sensitive data follow the Course retention policy.
- Course metadata, Assignment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
- Privacy-safe aggregate Question statistics remain after the underlying Student records are deleted.
- Aggregate Question statistics must not identify or allow reconstruction of individual Student activity.
- Published Question statistics retain accepted graded Attempt count and correct count.
- Eligible Question Types may also retain aggregate answer-choice counts.
- Question statistics are version-specific first, with clearly labeled Question-level rollups when appropriate.

### Course retention

- Course retention should follow Course dates and Student activity rather than a fixed academic calendar.
- The final Assignment deadline in a **Course Instance** starts its retention clock.
- Student activity after the final Assignment deadline should reset the inactivity clock.
- Instructors should not need to mark a Course Instance inactive to start retention.
- Retention should work equally for semesters, quarters, summer Courses, and other academic calendars.
- PLE should notify the **Instructor** before FERPA-sensitive Student data is archived.
- Archived Student data should leave normal Instructor and Student interfaces but remain recoverable during the retention period.
- FERPA-sensitive Student data should be permanently deleted when its retention period expires.
- Course metadata, Assignment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
- A Course Instance with its FERPA-sensitive Student data removed becomes an Inactive Course.

### Retention processing

- A background process should periodically find Course Instances whose retention deadlines have passed.
- Retention decisions should come from stored Course dates and Student activity.
- The background process should execute retention policy rather than define when retention periods begin or end.
- Running the retention process late should produce the same retention decision as running it on schedule.
- The retention process should be safe to run repeatedly.
- Student activity should be checked again before archival or permanent deletion.

### Revisions and history

- Be conservative about creating revisions.
- Assignments, Course Instances, and Draft Questions use current state.
- Published Questions and Blueprint Courses have immutable revisions.
- Mutable working state gets Edit Numbers when needed for concurrency.
- Student Work retains the exact evidence needed to interpret what the Student received, submitted, and was graded on.
- Teaching configuration may change without changing the historical meaning of existing Student Work.

### Dates and time zones

- Assignment deadlines are stored as instants.
- Instructor dates and times use the Instructor's IANA time zone.
- Students have their own IANA time zone for displaying dates and times.
- Changing a display time zone changes how a deadline is shown, not the deadline itself.

-----

## Question philosophy

- Questions are subject agnostic. Properly tagged Questions from all subjects belong in the same Question Library.
- **Draft Questions** remain private until publication.
- **Draft Questions** must pass an automated and interactive Question Publication Validation process before joining the Question Library.
- Questions are strictly and deterministically automated; grading does not require an **Instructor**.
- Questions have one canonical title. Compact interfaces may truncate that title.
- Every Question stored by PLE has its own internal Question record.
- Published Questions receive a public `AAAA-ZBBB` Crockford Base32 ID.
- A Question Pool has its own public `AAAA-ZBBB` Crockford Base32 ID and immutable revisions.

### Question formats and types

- PLE flat-question JSON is the canonical machine format for simple static Questions.
- QTI is for import, export, and archival interchange rather than the internal source model.
- MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT Question Types should be supported.
- Question Type is immutable author-declared educational metadata on a Published Question Revision.
- PLE uses Question Type for search, filtering, labeling, and presentation.
- Question Type comes from the author rather than inference from backend controls.
- Question importers are transient translators from external formats into PLE-managed Question representations.

### Native PLE JSON Questions

- The native PLE JSON Question format is private, unversioned, and unpublished.
- Stored native JSON Questions may be upgraded together when the internal format changes.
- The native PLE JSON Question format is a strictly validated internal source shape without an external API.
- Native JSON Questions are static, not algorithmic nor random, and receive no random seed.
- Native PLE JSON supports MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT.
- External URLs used by native JSON Questions are explicitly recorded and reviewable.
- Recorded external URLs include links, images, scripts, stylesheets, and other resources.

#### Native PLE JSON Questions and JavaScript

- Native JSON Questions may contain author-supplied JavaScript, including chemistry content using RDKit.
- Author-supplied JavaScript may provide client-side rendering or interaction without access to a random seed.
- Author-supplied JavaScript runs in an isolated browser environment.
- Author-supplied JavaScript is treated as untrusted content.
- Author-supplied JavaScript is isolated from PLE application state, credentials, and privileged browser context.
- Author-supplied JavaScript is limited to client-side rendering and interaction.
- Author-supplied JavaScript operates independently of PLE application APIs and privileged state.
- Native interactive Question Types such as HOTSPOT use PLE-owned interaction code.
- HOTSPOT content uses supported static assets such as images and SVG.
- Grading and correctness decisions remain server-owned and independent of author-supplied JavaScript.
- External JavaScript dependencies and CDN domains are explicitly recorded and reviewable.
- Approved external dependencies may initially load from recorded CDN sources.
- Supported external dependencies should eventually become PLE-owned and served locally.

### Question Backends

- WeBWorK, iMathAS, and H5P are PLE-managed Question Backends.
- The initial primary Question Backends are PLE-native JSON and WeBWorK.
- iMathAS and H5P are supported secondary Question Backends.
- PLE owns and stores the Question representation used for each Question Backend.
- Imported backend source may be transformed into the form PLE stores and manages.
- PLE preserves the information needed to reproduce the Question through its backend.
- PLE-managed Question representations participate in Question revision history.
- Question Backends own rendering, interaction, response, grading, feedback, and backend-specific state.
- PLE owns authorization, Question ID, revisions, persistence, lifecycle, and stored outcomes.
- PLE uses the same basic interface for every Question Backend, each backend handles its own internal details.
- Each Question Backend adapter retains its backend-specific interaction knowledge.
- PLE-native Questions use the PLE Question Backend.
- WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
- H5P owns its runtime, interactions, state, and scoring.
- iMathAS owns its rendering and evaluation.
- Question Backends may support more complex interactions without requiring PLE to implement those interactions.
- A Question Backend returns an immutable credit fraction for a submitted response.
- PLE stores the immutable credit fraction as the grading outcome.
- Assignment scores are calculated from stored credit fractions and current Question point values.
- Changing Question point values recalculates scores without another Question Backend interaction.
- When parameterized WeBWorK source exists, prefer it to importing static variants.

### Question Pools

- A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
- Question Pools are primarily designed for static Question variations.
- Pool contents should represent reasonably interchangeable assessments of the intended learning.
- Question Backend Questions may also be included in Question Pools.
- Each member of a Question Pool is a **Published Question**.
- Question Pools are always published and have no draft or unpublished state.
- A Question Pool is an independently reusable Question Library object.
- A Question Pool has its own public `AAAA-ZBBB` Crockford Base32 ID.
- Importing a Question Pool into a new Assignment automatically forks the Question Pool.
- The fork belongs to the new Assignment and can be changed without changing the source Question Pool.
- Forking a Question Pool preserves its Published Questions by their public `AAAA-ZBBB` IDs.
- Question Pools work the same way regardless of the Question Backend.
- **Instructors** choose the contents of a Question Pool and how many Questions are selected.
- PLE selects from the Question Pool; the selected Question Backend controls the Question interaction.
- Question Pool selection and backend-native randomization are separate forms of variation.
- Returning to an Attempt preserves the Question Pool selections already made.
- Starting a new Attempt makes fresh selections from its Question Pools.
- Student Work preserves the exact Question Pool Revision and Published Question Revision delivered.
- Grading and historical evidence follow the exact Published Question Revision delivered to the Student.

### Question Library

- Question sharing, discovery, and reuse are a high-priority **Instructor** workflow.
- The Question Library is one global collection of published Question content.
- **Published Questions** are available to all vetted **Instructors**.
- Published Question Pools are available to all vetted **Instructors**.
- **Students** access Question content through their Assignments rather than through the Question Library.
- Published content remains discoverable when used by a private **Course Instance**.
- Question stewardship should use a GitHub-like model.

#### Published Question identity

- Published Questions and published Question Pools have public Crockford Base32 IDs.
- Public IDs use the form `AAAA-ZBBB`.
- Seven Crockford Base32 characters are cryptographically random and provide the identity.
- The middle character is an HMAC-derived check character calculated from the seven identity characters.
- The check character detects mistyped or malformed IDs; it is not a security boundary.
- ID generation enforces database uniqueness and retries when a random collision occurs.
- IDs never encode creation order, Question Type, ownership, subject, or other metadata.

#### Revisions, edits, and forks

- **Published Questions** maintain immutable revision history.
- Assignments and Student Work remain pinned to exact immutable Published Question Revisions.
- New revisions become controlled updates rather than silently changing issued or graded work.
- Teaching changes preserve the historical meaning of existing Student Work.
- Moderate edits create a new immutable revision in the same Question lineage.
- Moderate edits are made by the Question owner.
- Moderate edits should remain within limits that preserve the identity and meaning of the Question.
- Full forks may be created by any **Instructor**.
- Full forks begin as private **Draft Questions** with their own authorship and lineage.
- Full forks must pass Question Publication Validation before joining the Question Library.
- Published forks become separate Question lineages with source attribution.
- Forced corrections are audited **Sysadmin** actions reserved for critical flaws.
- Question authorship, contributor credit, history, attribution, and compatible CC licensing are preserved across revisions and forks.
- Question change proposals are a future workflow requiring their own approved product design.

#### Question stewardship

- **Published Questions** can be starred and watched, similar to GitHub.
- Star means favorite and visible endorsement.
- Vetted **Instructors** can see the star count and which vetted **Instructors** starred a Question.
- Watch means subscription.
- Watching drives in-app notifications for revisions, forks, improvement threads, and impact notices.
- An **Instructor's** watch list remains private.
- **Students** and anonymous users do not receive **Instructor** identity lists or watch information.
- Question writers may add optional Question Feedback when it helps.
- Student workflows remain complete whether or not Students read Question Feedback.

#### Question statistics

- Privacy-safe aggregate Question statistics remain after the underlying Student records are deleted.
- Question statistics are revision-specific first.
- Each Published Question Revision retains its accepted graded Attempt count and correct count.
- Eligible Question Types may also retain aggregate answer-choice counts.
- Question-level statistics may combine revisions when clearly labeled and privacy thresholds are met.
- Question statistics expose aggregate Student behavior rather than identifiable Student records.
- Student data retention removes identifiable Student evidence rather than privacy-safe aggregate Question statistics.

#### Question behavior

- Answer-choice randomization belongs to the Question.
- PLE-native Questions control their own answer-choice randomization.

-----

## Course philosophy

- **Courses** organize reusable teaching content and its delivery to **Students**.
- PLE has two Course forms: **Blueprint Courses** and **Course Instances**.
- **Course Instances** are built from **Blueprint Courses**.
- A Course can have multiple co-**Instructors** with equal teaching authority.
- **Sysadmins** can create Courses, but **Instructors** teach them.
- Every Course must have an assigned **Instructor** who owns the Course.

### Blueprint Courses

- **Blueprint Courses** are reusable course definitions for building **Course Instances**.
- Blueprint Courses are a similar concept as LibreTexts' ADAPT alpha courses.
- Blueprint Courses have no **Students**, deadlines, or other teaching-specific delivery settings.
- Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
- Blueprint Courses contain only **Published Questions** and published **Question Pools**.
- An **Instructor** may deliberately publish an existing Course Instance structure as a new Blueprint Course.

### Blueprint Course lifecycle

- Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
- New Blueprint Courses and forks start Private.
- Private Blueprint Courses are visible only to their owning **Instructor**.
- Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
- Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
- Public Blueprint Courses can be adopted to create daughter Course Instances.
- Archived Blueprint Courses are read-only and no longer actively maintained.
- Archived Blueprint Courses remain visible and reusable by every vetted **Instructor**.
- Archived Blueprint Courses are excluded from normal search results unless the search explicitly includes them.
- Archived Blueprint Courses cannot be adopted to create new daughter Course Instances.
- The owning **Instructor** can return an Archived Blueprint Course to Public before adopting it again.
- Other **Instructors** can fork an Archived Blueprint Course to create a new Private Blueprint Course.
- Blueprint Courses have no separate draft state.

### Blueprint Course revisions

- Blueprint Courses use immutable **Blueprint Revisions** for saved history and concurrency.
- Blueprint Course editing uses explicit Save.
- Each successful Save creates the next Blueprint Revision.
- Multiple edits before Save become one Blueprint Revision.
- **Instructors** cannot accidentally navigate away from a Blueprint Course with unsaved changes.
- Blueprint Course names identify the Blueprint across revisions.
- Changing a Blueprint Course name does not create a new Blueprint Revision.
- Blueprint Courses maintain a changelog visible to all vetted **Instructors**.
- The changelog should make meaningful changes between Blueprint Revisions easy to understand.

### Blueprint adoption and updates

- Blueprint adoption copies every Assessment from the Blueprint Course into the Course Instance.
- Course Instances pin the exact Blueprint Revision from which they were adopted.
- New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
- Routine Blueprint updates should be quick for an **Instructor** to review and approve.
- It should be obvious when a Course Instance is using an older Blueprint Revision.
- Changes to existing Assessments follow the Blueprint Revision update workflow.
- Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
- Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.

### Blueprint Course forks and Change Proposals

- An **Instructor** can fork a **Blueprint Course** to create a new independent Blueprint Course.
- A fork records the source Blueprint Course and Blueprint Revision from which it was created.
- Forked Blueprint Courses develop independently and have their own Blueprint Revisions.
- Changes to a source Blueprint Course are never automatically applied to its forks.
- A fork should make newer changes from its source Blueprint Course easy to discover and review.
- An **Instructor** can selectively bring changes from a source Blueprint Course into their fork.
- An **Instructor** can create a **Blueprint Course Change Proposal** to propose changes to another Blueprint Course.
- A Change Proposal shows added, removed, and changed Assessments and Question content.
- The receiving **Instructor** decides which proposed changes to accept.
- Accepted changes create a new Blueprint Revision of the receiving Blueprint Course.
- Change Proposals never directly change daughter Course Instances.
- Daughter Course Instances receive accepted changes through the normal Blueprint update workflow.

### Blueprint Course JSON

- Blueprint Courses have a canonical JSON representation for comparison and exchange.
- Blueprint JSON contains Blueprint metadata and an ordered list of Blueprint Assessments.
- Blueprint Assessments contain only reusable teaching settings, not Course Instance delivery settings.
- Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
- Blueprint Assessments have no deadlines, release dates, Student data, or other Course Instance settings.
- Blueprint Revisions can be compared through their canonical JSON representations.
- Blueprint Course Change Proposals use the canonical JSON to identify changes between Blueprint Revisions.
- Canonical Blueprint JSON is the comparison and exchange format, not the primary persistence model.

### Course Instances

- A **Course Instance** may be created from a **Blueprint Course** or started as a new Course.
- Creating a Course Instance from a Blueprint Course uses Blueprint adoption.
- **Instructors** can create a new empty **Course Instance** without a parent **Blueprint Course**.
- Course Instances have **Students**, deadlines, releases, and other delivery-specific settings.
- Course Instances contain only **Published Questions**.
- Course Instances are visible only to their co-**Instructors** and enrolled **Students**.
- Active Courses are current teaching Course Instances.
- Inactive Courses retain Course metadata after FERPA-sensitive Student data is removed.
- An **Instructor** may deliberately publish reusable Course Instance structure as a new **Blueprint Course**.

### Blueprint adoption and updates

- Blueprint adoption creates every Assessment from the Blueprint Course in the Course Instance.
- Course Instances pin the exact Blueprint Revision from which they were adopted.
- New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
- Routine Blueprint updates should be quick for an **Instructor** to review and approve.
- It should be obvious when a Course Instance is using an older Blueprint Revision.
- Changes to existing Assessments follow the Blueprint Revision update workflow.
- Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
- Blueprint changes to existing Assessments are never silently applied to Course Instances.

### Course names

- Blueprint Courses and Course Instances each have their own short name and long name.
- Short names are entered or chosen deliberately by **Instructors**.
- Short names are for compact navigation and should stay under about 16 characters when practical.
- Long names are descriptive names used for headings, breadcrumbs, and Course listings.
- A Blueprint Course might be `Biochemistry` / `Upper-Level Introductory Biochemistry`.
- A Course Instance might be `BCHM 355/455` / `BCHM 355/455 Section 20 Biochemistry (Roosevelt University; Spring 2026)`.
- Course Instance names are properties of the Course Instance and are not derived from Blueprint Course names.

-----

## Assessment philosophy

- **Assessment** is the PLE object used to deliver Questions to **Students**.
- Assessment types are **Regular Assignment**, **Practice Question Assignment**, **Bonus Assignment**, **Quiz**, and **Exam**.
- All Assessment types use the same underlying Assessment model.
- Assessment type describes pedagogical purpose and provides appropriate defaults.
- **Assessment Attempt** is one Student attempt at an Assessment.
- Use **Assignment** only for Regular Assignments, Practice Question Assignments, and Bonus Assignments.


### Assessment types

- **Regular Assignments** give **Students** regular practice applying course ideas outside class.
- Regular Assignments reinforce current learning and may also introduce new topics.
- Regular Assignments are designed as practice for learning, not merely as one-time assessments.
- **Practice Question Assignments** provide focused review or study-guide practice using material already covered.
- Practice Question Assignments may be worth a small number of points or a small amount of extra credit.
- Practice Question Assignments always show the correct answer after the **Student** responds.
- **Bonus Assignments** provide optional extra credit.
- Bonus Assignments are worth zero points possible and add earned points directly to the grade.
- **Quizzes** assess understanding of recent material.
- Quizzes may use more restrictive Attempt and collaboration settings than Regular Assignments.
- **Exams** are individual assessments associated with scheduled exam periods.
- Exams may use more restrictive Attempt, timing, availability, and feedback settings.


### Assessment lifecycle and defaults

- New Assessments default to accepting submissions only through the due date.
- New Assessments default to starting new Attempts only through the due date.
- Late work defaults to rejected.
- Assessment disclosure settings remain separate and independently configurable.
- **Regular Assignments** and **Bonus Assignments** should rarely show the correct answer.
- Regular and Bonus Assignments show the **Student's** response and whether it was correct or incorrect.
- **Practice Question Assignments** always show the correct answer after the **Student** responds.
- **Quizzes** and **Exams** show correct answers after all **Students** in the Course have completed the Assessment.
- Until then, Quizzes and Exams do not disclose correct answers.
- Question Feedback is always shown when a Question includes it.
- Unreleasing an Assessment permanently deletes its Student Work and returns it to a pre-release state.
- Assessment Question-order randomization is called **Randomize question order**.
- **Assessments Due Soon** shows upcoming Assessments across the Courses an **Instructor** teaches.
- Assessments Due Soon shows the Course and due time for each Assessment.


### Assessment Attempts

- Each **Assessment Attempt** has a time limit.
- Attempt time limits help **Students** develop an accurate sense of expected working speed.
- Timed Assessment Attempts use wall-clock time.
- The server owns the Attempt start and expiration times.
- Attempt time continues while the **Student** is disconnected or the browser is closed.
- A **Student** may reconnect, reload, or use another authenticated browser session to resume the same active Attempt.
- Resuming an Attempt does not reset, pause, or extend its time limit.
- Question responses are saved as the **Student** works and remain part of the Attempt across browser sessions.
- Submission belongs to the Assessment Attempt, not to individual Questions.
- When an Assessment Attempt expires, its saved responses are submitted automatically.
- Questions without saved responses close unanswered when the Attempt expires.
- **Instructors** control the number of permitted Assessment Attempts.
- Regular Assignments default to unlimited Attempts.
- **Students** may repeat an Assessment as often as its settings allow, including practicing toward a perfect score.
- Assessment Attempt submission and grading are automatic.
- PLE has no **Instructor** grading, regrading, or retry-grading workflow.
- Attempt expiration is checked whenever a **Student** interacts with the Attempt.
- Background processing also submits expired Attempts when no browser remains open.
- Background processing uses the ordinary submission path and completes Question Backend interactions that require polling.


### Assessment scoring

- A Question Backend returns an immutable credit fraction for each submitted response.
- PLE stores the credit fraction as the Question grading outcome.
- Each Question in an Assessment has a point value.
- Assessment scores are calculated from stored credit fractions and current Question point values.
- Changing Question point values recalculates affected Assessment scores.
- Score recalculation does not require another Question Backend interaction.
- Score recalculation does not change the stored Question grading outcome.

-----

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
- Instructor Course and Assignment lists should be dense and easy to scan, more like a spreadsheet than cards.
- Assignment title and due date should be editable directly from the Course Assignment list.
- New Assignments should default to 11:59 PM in the Instructor's time zone.
- Question answer visibility should be controlled by the Instructor. By default, Students should see the answer they selected and whether it was correct or incorrect. When an answer is incorrect, the correct answer should remain hidden. Instructors can choose a more restrictive setting where Students see only whether their response was correct or incorrect.
- Instructor Profile should include the Instructor's time zone and profile image.
- Profile images can be uploaded at any reasonable aspect ratio. Crop the image to a consistent rounded square before committing it as the Profile image.
- Assignment Preview should open separately from the editing surface.
- Instructors can randomize Question order for an Assignment. Default is that questions are randomly presented.
- My Active Courses and My Inactive Courses should both be available from the Courses area.
- The Instructor's time zone should be used to interpret dates and times the Instructor enters.
- Changing a Instructor's time zone should update how existing deadlines are displayed while preserving the deadline itself.
- Assignment deadlines should be stored as absolute UTC instants.

## Student philosophy

- Each **Assignment Attempt** has a time limit so one Question set does not remain open for days.
- **Students** may start another **Assignment Attempt** as often as needed, including practicing to a perfect score.
- **Students** log in only with a passkey or email code; no passwords.
- **Student** data should be collected reluctantly, used deliberately, and purged predictably.
- **Student** course data falls under FERPA; treat it as radioactive.
- Student email addresses are immutable.
- Student Accounts persist across Courses and semesters.
- A Student Account is global and is not owned by or permanently tied to a Course Instance.
- When an Instructor uploads a roster, PLE uses the institutional email to find an existing Student Account or creates one when none exists.
- Each Course Instance creates its own course-scoped Student Record and enrollment relationship for that Student Account.
- Course work, Attempts, submissions, and grades follow the Course retention policy independently of the lifetime of the Student Account.
- Students have their own time zone for displaying dates and times.
- A Student's time zone defaults to the Instructor's time zone during the invite phase.
- Changing a Student's time zone updates how existing deadlines are displayed while preserving the deadline itself.

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

## Course observers, student observers, and graders philosophy

- Both observer types are read-only participants.
- **Course Observers** can see assignments and questions for a course and which **Students** have completed the assignments.
- **Course Observers** do not see scores.
- **Student Observers** can see everything about a particular **Student**. PLE will assume FERPA rights to the **Student** have been waived.
- **Graders** are not needed right now because we do not have manual grading.
- Course authorization should stay adaptable for future **Grader** and **Course Observer** relationships. A
  **Course Observer** receives anonymous aggregate grades without **Student**-level FERPA information.
