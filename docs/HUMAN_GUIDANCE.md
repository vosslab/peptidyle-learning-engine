# Human guidance

<!-- VENDORED HEADER: START -->
Record the durable guidance Neil Voss states, or approves for preservation here, in his own words:
first person or close paraphrase, one to three lines per bullet. Material he supplies as a source
may inform [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) once it is settled, and an entry of uncertain
origin belongs there too. Rules: [REPO_STYLE.md](REPO_STYLE.md).
[PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]
<!-- VENDORED HEADER: END -->

## How to use this guidance

- Guidance bullets should start with the subject when practical, making them easier to scan.
- Guidance should stay terse and in my own words.
- Uncertainty should remain when I have not made a final decision.
- This document uses GitHub Flavored Markdown (GFM).
- Bullet duplication is acceptable because many agents only skim read one section at a time.

## Development principles

### Agent working principles

- Read and learn the core principles in docs/REPO_STYLE.md
- Apply the Keep It Simple, Stupid (KISS) philosophy aggressively.
- Prefer the smallest coherent design that meets actual requirements and known failure modes.
- Complexity must earn its place.
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
- Add product states, workflows, background processing, and recovery mechanisms only for a
  demonstrated product or Question Backend need.
- Stay focused on the requested work. Complete the required work and avoid adding unplanned functionality.

### Codebase development rules

- Every source file should stay below 1000 lines. Split complete capabilities into focused modules.
- PLE is pre-production with no users. Fix the design directly rather than preserving legacy behavior.
- Use SQL directly to create the initial PostgreSQL database.
- Before production, edit the main database design directly as the design changes.
- After production, update existing databases without rebuilding them from scratch.
- PLE is pre-production with no users or durable production data. Improve the design directly.
- Use readable `snake_case` whenever possible; see [NAMING_CONVENTIONS.md](/docs/NAMING_CONVENTIONS.md) for details.
- Adaptability should be a focus so the software can evolve as requirements and insights change.
- Cargo, Node, and PyPI dependencies should use the latest versions to include security fixes.
- If an interface is measured as too slow, consider moving the slow code to Rust/WebAssembly.
- Do not create or leave placeholder database tables, states, APIs, workers, or compatibility scaffolding before the feature has an approved product design.

### PLE development rules

- A fresh production installation includes the complete Live Demo by default.
- Treat the initial course content as shipped examples.
- BiologyProblems.org content is free and open source.
- The Genetics Blueprint Course from BiologyProblems.org ships as the example course.
- All Podman content on the Mac-Studio-36G machine belongs to this project.
- Neil pre-approves pruning Podman images, volumes, and containers on Mac-Studio-36G as needed.
- The polished PLE Live Demo is the top priority; see [LIVE_DEMO_SPEC.md](/docs/LIVE_DEMO_SPEC.md).
- PLE should use one global installation with no institution boundaries.
- Project images and simulated live-stack data are disposable acceptance infrastructure.
- `./launchers/run_live_demo.sh` is the normal local-stack entry point. For direct controller
  diagnostics, use `source source_me.sh && python3 local_stack.py`.

## Product vocabulary

- **Blueprint Course**: A reusable course used to create **Course Instances**. It has no enrolled **Students** or deadlines.
- **Blueprint Revision**: A fixed version of a **Blueprint Course** preserved so its content cannot change.
- **Course Instance**: A course used for teaching. It has **Students**, deadlines, releases, and other course settings. It may be created from a Blueprint Course or started empty.
- **Published Question**: A validated question in the global **Question Library**, available to vetted **Instructors**.
- **Draft Question**: A private question being developed by an **Instructor**. It must pass validation before publication.
- **Question Library**: The global collection of Published Questions and published Question Pools available to vetted **Instructors**.
- **User Roles**:
  - **Sysadmin**: A PLE administrator who manages the system, approves **Instructors**, creates accounts, and helps manage courses.
  - **Instructor**: An approved user who teaches courses and can browse, reuse, create, fork, and publish Questions.
  - **Student**: A user enrolled in a **Course Instance** who completes Assessments and other course activities.
- **Assessment Question Editor**: The **Instructor** editor for selecting, adding, removing, and ordering Questions in an Assessment.
- **Assessment Properties Editor**: The **Instructor** editor for settings that apply to the whole Assessment, such as dates, scoring, attempts, late work, and what **Students** can see.

## Accounts and roles

### Account rules

- PLE accounts should be global across PLE and use passwordless passkeys and email authentication.
- Email is not configured for the Live Demo yet; use the visible seeded-role entry for demo access.
- The three major user types are **Sysadmins**, **Instructors**, and **Students**.
- Potential future user roles are **Course Observers**, **Student Observers**, and **Graders**.
- **Students** are required to use their university or institutional (`.edu` in the USA) email accounts.
- **Sysadmin** accounts should require higher security than other accounts, like TOTP authentication
- Every Account has exactly one Product Role: **Student**, **Instructor**, or **Sysadmin**.
- Product Role is locked and cannot change during the lifetime of an Account.
- A person who needs more than one Product Role uses separate Accounts.
- Instructor Accounts may be deactivated without deleting their authored content, Course relationships, or historical records.
- Reactivating an Instructor Account restores access to the same Account and Product Role.

### Instructor role

- All vetted **Instructors** have the same product capabilities.
- A **Sysadmin** vets an Instructor's real identity before creating the Instructor Account.
- Course membership determines which private Course records an Instructor may use.
- **Instructors** can search and browse the global **Question Library**.
- **Instructors** can browse the content of Public and Archived **Blueprint Courses**.
- **Instructors** log in only with a passkey or email code; no passwords.
- **Instructors** should have a clearly labeled, answer-free **Student** view without changing their identity.

### Student role

- **Students** log in only with a passkey or email code; no passwords.
- Students may use multiple passkeys across their devices.
- An **Instructor** can reset Student login access and send a new signup code when needed.
- **Student** data should be collected reluctantly, used deliberately, and purged predictably.
- Student Course data falls under FERPA; treat it as radioactive.
- Student email addresses are immutable.
- Student Accounts persist across Courses and semesters.
- A Student Account is global and is not owned by or permanently tied to a Course Instance.
- Roster import uses institutional email to find an existing Student Account or create one when needed.
- Each Course Instance has its own course-scoped Student Record and enrollment for the Student Account.
- Student Work, Attempts, submissions, and grades follow Course retention independently of the Student Account.
- Removing a **Student** from a Course revokes future Course access but does not immediately delete the Student's Course records or Student Work.
- Student Work and grades remain subject to the normal Course retention policy after enrollment ends.
- An **Instructor** can deactivate a Student's access to their Course.
- Deactivating Course access does not delete the Student Account or Student Work.
- An **Instructor** can restore the Student's Course access later.
- **Instructors** can bulk add Students to a Course Instance through roster import.
- **Instructors** remove Students individually.
- PLE does not provide bulk Student removal from a Course Instance.

### Sysadmin role

- A **Sysadmin** has full administrative authority over PLE.
- Sysadmins vet **Instructors** and create Instructor Accounts.
- Sysadmins can help Instructors repair Courses, Students, and content.
- The human developer, Dr. Neil Voss, is currently both a **Sysadmin** and an **Instructor**.
- Neil uses separate Sysadmin and Instructor logins so the roles remain distinct.
- **Sysadmins** have full platform-administration capability but do not automatically have access to FERPA Course records.
- A Sysadmin may access Course or Student records when needed to resolve a specific support problem.
- Sysadmin support access should be limited to that support task and recorded for audit.
- Sysadmin support does not make the Sysadmin an **Instructor** or Course member.

### Future Course roles

- PLE may eventually support **Course Observer**, **Student Observer**, and **Grader** roles.
- **Course Observers** are read-only participants with access to Course content and non-FERPA aggregate information.
- **Student Observers** are read-only participants with authorized access to a particular Student's Course information.
- **Graders** are not currently needed because Assessment grading is automatic.
- Course authorization should remain adaptable enough to add these relationships later.

## Interface design

### General interface design

- **Sysadmin** uses tomato red as its role color.
- **Instructor** uses teal green as its role color.
- **Student** uses lavender /purple as its role color.
- Role colors should be used consistently in role labels and other appropriate interface cues.
- Demo role selection should clearly state both the user's role and name.
- Instructor and **Sysadmin** workflows should work well in a 1280 by 800 desktop browser viewport.
- Design around what users need to find and do.
- Important information should stand out from supporting information.
- Related information should be visually grouped and aligned.
- Similar pages should place similar controls in consistent locations.
- Primary actions should be easy to find and appear near the content or workflow they affect.
- Avoid scattering related actions across page headers, menus, navigation, and content areas.
- PLE often presents large collections where users need to find a few relevant items.
- Optimize large collections for scanning, searching, filtering, and comparison.
- Show enough useful information at once to support comparison without excessive scrolling.
- Search and filters should help users quickly narrow large collections.
- Dense pages should remain easy to scan.
- Use spacing to separate meaningful groups rather than simply making pages spacious.
- Prefer alignment, typography, and dividers over unnecessary cards, boxes, borders, and nested containers.
- Keep the visual design compact, flat, information dense, and consistent across PLE.
- Dream big on the UI. Choose one visual philosophy and carry it through the entire interface.
- Use drag-and-drop where it makes reordering faster and more natural.
- Reordering must also have a precise keyboard-accessible method.
- Themes should use biome and habitat names.
- Implement the themes as specified in `docs/BIOME_THEME_PALETTES.md`
- UUIDs should never appear in visible content, navigation URLs, or copyable links.
- Use [Atkinson Hyperlegible Next](https://www.brailleinstitute.org/freefont/) as the main PLE font.
- Use [Atkinson Hyperlegible Mono](https://www.brailleinstitute.org/freefont/) for code and other monospace text.
- Prefer the official Braille Institute font files and include the needed weights locally with PLE.
- When a narrow font is needed, use `IBM Plex Sans Condensed` for long unbreakable strings such as URLs.
- With `IBM Plex Sans Condensed`, try `font-variant-numeric: slashed-zero` to better distinguish `0` from `O`.
- Question Backend-rendered content may use its own fonts when needed for correct display.
- Students should have no upload capabilities. Instructor-created content should use text boxes.

### Ribbon and page layout

- The top Ribbon is the persistent navigation area for signed-in PLE pages.
- The Ribbon should remain in the same location and use the same overall structure while navigating.
- Navigation choices should remain in predictable locations as users move between related pages.
- Changing a Ribbon selection changes the content below the Ribbon without moving the main content area up or down.
- Ribbon rows should keep their space when needed so changing selections does not make the content area jump.
- Page actions should appear near the content they affect rather than changing the Ribbon layout.
- See **User top bar** and **Breadcrumbs** for the persistent elements that make up the top of the page.

### User top bar

- All signed-in users share the same basic top bar layout.
- The top bar remains in a consistent location as users navigate.
- The PLE logo and product name appear at the upper left and link to the user's home dashboard.
- Each Product Role has its own home dashboard and navigation.
- Product Role appears once next to the PLE name.
- Role-specific navigation appears between the product identity and Profile.
- Profile appears at the far right as an icon-only avatar.
- Clicking the Profile avatar opens the Profile menu.
- The Profile menu contains Profile settings, account settings, and Sign Out.
- Sign Out belongs in the Profile menu rather than the main top bar.
- The Profile avatar uses a generic user avatar until the user selects another avatar.
- **Students** select avatars from a PLE-provided collection and cannot upload Profile images.
- Student avatar selection should be visual and playful, similar to choosing a LEGO avatar.
- **Instructors** and **Sysadmins** may select a provided avatar or add their own Profile image.
- The current avatar appears consistently anywhere PLE represents that user.
- Instructor Profile includes the Instructor's time zone and profile image.
- Profile images may use any reasonable aspect ratio and are cropped to a consistent rounded square.
- See **Ribbon and page layout** for the overall navigation and page-position rules.

### Breadcrumbs

- All signed-in users have a permanent breadcrumb row below the top Ribbon.
- The breadcrumb row remains in the same location and keeps the same space as users navigate.
- Breadcrumbs show the path from the user's home dashboard to the current page.
- Each breadcrumb level links back to its corresponding page.
- Breadcrumbs use human-readable names rather than internal identifiers.
- Course and Assessment breadcrumbs preserve the current Course context.
- Keeping the breadcrumb row in place prevents the main content from moving up or down as breadcrumb depth changes.
- See **Ribbon and page layout** for the overall page-position rules.

### Instructor interface

- The Instructor interface should make frequent teaching tasks fast and easy to find.
- The Instructor menu has **Courses**, **Questions**, and **Assessments** in one dense top bar.
- Instructor Profile uses a generic user icon until the **Instructor** adds a Profile image.
- All required ribbon choices remain visible even when their collection is empty.
- A working navigation destination remains visible when its collection is empty.
- A future or unavailable capability should not appear as a usable control until its workflow exists.
- Empty collection pages should explain what the collection is for and provide an obvious action to create or add the first item when the user can do so.
- Similar pages should place similar actions in consistent locations.
- Instructor pages should be composed around the teaching task rather than collections of padded components.
- Instructor Course and Assessment lists should be dense and easy to scan, more like a spreadsheet than cards.
- Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.

#### Courses

- The **Courses** ribbon must include: My Blueprint Courses, My Active Courses, My Inactive Courses, Search Public Blueprint Courses.
- Course lists should support scanning and comparison without opening each Course.
- The Course Editor should show the Course structure and its ordered Assessments without showing every Question at once.
- Selecting an Assessment in the Course Editor opens that Assessment for editing.
- Assessment content and Assessment properties should remain separate editing tasks.
- My Active Courses and My Inactive Courses should both be available from the Courses area.

##### Blueprint Courses

- **My Blueprint Courses** should emphasize reusable course design rather than teaching activity.
- **Search Public Blueprint Courses** helps Instructors find a Blueprint Course they already have in mind.
- Public Blueprint Course search should support quickly narrowing a large collection.
- A **Blueprint Course** should provide an obvious action for creating a **Course Instance** from it.

###### Blueprint Course editing

- Blueprint Course editing should follow Course Editor -> Blueprint Assessment Editor.
- The Course Editor should show the Blueprint Course structure without editing every Question on one page.
- Selecting a Blueprint Assessment in the Course Editor opens the editor for that Blueprint Assessment.
- Only the selected Blueprint Assessment's Questions should appear in its editor.
- **Blueprint Assessment Question Editor**: Selects, adds, removes, and orders Questions in a Blueprint Assessment.
- **Blueprint Assessment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see.
- Blueprint Courses should not contain Assessment dates or relative Assessment schedules.

###### Blueprint Course lifecycle

- Blueprint Courses follow the lifecycle **Private -> Public -> Archived**.
- New and forked Blueprint Courses start **Private**.
- Private Blueprint Courses are visible only to their owner.
- Instructors may develop and use Private Blueprint Courses without publishing them.
- Making a Blueprint Course **Public** adds it to the shared Blueprint Course collection.
- Public Blueprint Courses are visible to all **Instructors**.
- A Public Blueprint Course with no adoptions may return to **Private**.
- A Public Blueprint Course with one or more adoptions remains **Public**.
- Archived Blueprint Courses leave normal discovery but remain available where needed for history.
- Archived Blueprint Courses remain viewable when accessed directly or through their history.
- Blueprint Courses do not have a separate Draft state.

###### Blueprint Course forks and changes

- Instructors may fork a Public Blueprint Course to continue development privately.
- Forking a Blueprint Course creates an independent Private Blueprint Course owned by the
  Instructor who created the fork.
- A Blueprint Course fork records the Blueprint Course and Revision it was forked from.
- Blueprint Course forks develop independently after they are created.
- A fork does not automatically receive later changes from its source Blueprint Course.
- A Blueprint Course shows its known forks and the **Instructor** who owns each fork.
- **Instructors** can open a fork and compare it with its source Blueprint Course.
- Fork comparison normally compares the current Revision of the source Blueprint Course with the
  current Revision of the fork.
- The recorded source Revision from which the fork was created provides the common baseline for
  identifying changes made later in the source and changes made in the fork.
- Comparison should make changes unique to the fork and changes added later to the source easy to
  distinguish.
- Blueprint Course differences are calculated from canonical JSON when the comparison is requested.
- Older Revisions remain available through Blueprint history but are not the normal fork-comparison
  workflow.

##### Course Instances

- **My Active Courses** should emphasize Course Instances the Instructor is currently teaching.
- Active Course Instances should make upcoming Assessments and important course activity easy to find.
- **My Inactive Courses** should keep past Course Instances available without competing with active Course Instances.
- Creating a Course Instance from a Blueprint Course preserves its Assessments, Questions, pools, and settings.
- Assessments created from a Blueprint Course start unreleased with dates unset.
- A Course Instance represents one teaching period and remains Active for at most six months from
  creation.
- Course banners use a 5:1 aspect ratio.
- 1280 by 256 pixels is the recommended Course banner authoring size.
- Higher-resolution 5:1 Course banner images are supported.
- PLE responsively scales Course banners while preserving their aspect ratio.
- Course banners appear as small centered banners rather than full-width page heroes.
- An Instructor can upload a Course banner and select a three-color theme.
- Course Instance Assessments have two editors:
  - **Assessment Question Editor**: Selects, adds, removes, and orders Questions in an Assessment.
  - **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and what **Students** can see.

#### Questions

- The **Questions** ribbon must include: My Questions, My Draft Questions, Starred, Watched, Search Question Library, Browse Question Library.
- **My Questions** should make the Instructor's Published Questions easy to find and manage.
- **My Draft Questions** should emphasize Questions that still need work before publication.
- **Starred** should provide a quick personal collection of Questions the Instructor wants to keep handy.
- **Watched** should help Instructors follow Questions where changes or activity matter to them.

##### Search Question Library

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

##### Browse Question Library

- **Browse Question Library** helps Instructors explore Questions without knowing what to search for.
- Browse should help Instructors understand what the Question Library contains.
- Browse should emphasize subjects, topics, tags, Question Types, and other useful groupings.
- Browse should make moving from broad subjects to narrower topics easy.
- Browse should show useful counts where they help Instructors choose where to explore.
- Browse results should use the same dense Question presentation used by Search where practical.
- Instructors should be able to move from browsing into a more focused search.
- Search and Browse are different paths into the same **Question Library**.

#### Assessments

- The **Assessments** ribbon must include: Assessments Due Soon, My Assessment Templates.
- **Assessments Due Soon** should emphasize Assessments that may need the Instructor's attention.
- Assessment lists should make Course, release status, due date, and other important state easy to scan.
- **My Assessment Templates** should emphasize reusable Assessment design rather than Course activity.
- Assessment editing has two editors:
  - **Assessment Question Editor**: Selects, adds, removes, and orders Questions.
  - **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and other Assessment settings.
- The two Assessment editors should remain clearly distinct.
- The Assessment Question Editor should make Question order easy to understand at a glance.
- Adding Questions should provide direct paths to Search and Browse Question Library.
- Instructors should be able to inspect a Question before adding it to an Assessment.
- Assessment Properties should group related settings so important settings are easy to find.
- Instructors can randomize Question order for an Assessment.
- Answer-choice randomization belongs to the Question, not the Assessment.
- **Assessments Due Soon** shows upcoming Assessments across the Courses an **Instructor** teaches.
- Assessments Due Soon shows the Course and due time for each Assessment.

#### High-consequence actions

- Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
- Danger Zone should be visually separate from ordinary editing actions.
- Assessment Unrelease should explain that Student work will be deleted.
- Assessment Unrelease should require typing the Assessment title before confirmation.
- Archive actions should explain the effect on shared availability and require a clear confirmation.
- Restore actions should use ordinary availability controls.

### Student interface

- The Student interface should focus on current Courses, Coursework, and work that needs attention.
- **Coursework** is the Student-facing collective term for Regular Assignments, Practice Question
  Assignments, Bonus Assignments, Quizzes, and Exams.
- Student-facing interfaces should use the specific Assessment Type when referring to an individual item rather than calling it an Assessment.
- The Student Ribbon should use familiar Student language rather than internal PLE terms such as Assessment.
- Coursework lists may provide filters for **Regular Assignments**, **Practice Question Assignments**,
  **Bonus Assignments**, **Quizzes**, and **Exams**.
- Each Coursework item should clearly show its Assessment Type using its label and Type icon.
- The Student interface should make the next useful action easy to find.
- The Student menu is simpler than the Instructor menu.
- Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.
- Every Student browser action should be usable with the keyboard alone.
- Student pages should use names meaningful to Students.
- Student navigation and pages should contain only Student interfaces and capabilities.
- Students enrolled in one active Course should go directly into that Course.
- Students should be able to see their active Courses and Coursework from the main navigation.
- Course pages should make upcoming, available, completed, and missed Coursework easy to distinguish.
- Coursework lists should make due dates, Type, and completion status easy to scan.
- Before starting Coursework, Students should see its title, Type, Question count, points possible, time limit, and previous Attempts.
- Students see one Question at a time while completing Coursework.
- While completing Coursework, navigation should show every Question, its saved status, and allow Students to jump directly between Questions.
- Leaving a Question and returning should preserve its saved response.
- The current Question and overall progress should remain easy to see.
- The timer should be subtle and keep the focus on the Questions.
- For timed Coursework, the remaining time should stay visible while moving between Questions.
- Submission status should be obvious and use plain language.
- Scores and feedback should appear where the Coursework settings allow them.
- Completed Coursework should remain easy to find and review.
- Student content entry should use the response controls provided by Questions and other Student activities.
- The complete Student Ribbon task layout does not have a locked-in design yet.

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
- The complete Sysadmin Ribbon task layout does not have a locked-in design yet.

## Data and history

- Answers, keys, grading, and correctness decisions should stay on the server, out of reach of **Students**.
- Public data should stay separate from private, answer-bearing, identifying, or radioactive FERPA data.
- Human-readable titles and identifiers should be used wherever people must recognize, copy, or enter them.
- FERPA-sensitive Student data should not become ordinary logs, analytics, URLs, or long-lived browser storage.
- Opaque IDs remain FERPA-sensitive when they link a Student to Course activity.

### Human-facing reference IDs

- Human-facing reference IDs should be short, opaque, easy to communicate, and should not reveal creation order, counts, database keys, ownership, or other object metadata.
- Blueprint Course IDs use `BP`, Course Instance IDs use `CI`, Assessment IDs use `A`, and Account IDs use `U`, followed directly by a common cryptographically random Crockford Base32 reference format.
- ID generation enforces uniqueness and retries random collisions.
- Give an internal object a human-facing reference ID when a useful workflow needs to display, search, communicate, or support it.
- Account `U` references are Sysadmin support references and are not automatically exposed to Students or Instructors.
- Published Questions and published Question Pools retain their existing public `AAAA-ZBBB` IDs.

### Student and FERPA data

- **Student** course data falls under FERPA; treat it as radioactive.
- **Student** data should be collected reluctantly, used deliberately, and purged predictably.
- FERPA access should be scoped through exact Course membership and **Student** ownership.
- **Sysadmins** receive only the FERPA access required for a specific administrative task.
- Student Accounts persist independently of Course data and Course retention.
- Course work, Attempts, submissions, grades, and other FERPA-sensitive data follow the Course retention policy.
- Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
- **Student Work** is the collective term for FERPA-sensitive records created by a Student in a Course Instance.
- Student Work includes Assessment Attempts, saved Question responses, grading outcomes, and the evidence needed to interpret that work after an Attempt is submitted.
- Student Work is an umbrella term; the underlying records retain their own identities and purposes.
- Student retention removes identifiable Student evidence, not privacy-safe aggregate Question statistics.
- Privacy-safe aggregate Question statistics remain after the underlying Student records are deleted.
- Aggregate Question statistics must not identify or allow reconstruction of individual Student activity.
- Published Question statistics retain accepted graded Attempt count and correct count.
- Eligible Question Types may also retain aggregate answer-choice counts.
- Question statistics are version-specific first, with clearly labeled Question-level rollups when appropriate.

### Course retention

- Course retention should follow Course Instance dates and its six-month Active lifetime rather than
  a fixed academic calendar.
- The latest Assessment deadline ends normal teaching and starts the Course Instance's FERPA
  retention clock.
- Creating or extending a later Assessment deadline may move those dates, but not beyond the
  six-month Active lifetime.
- Starting the FERPA retention clock does not itself notify, archive, hide, or delete Student data.
- The configured FERPA retention policy determines the later notice, archive, recovery, and
  permanent deletion transitions.
- PLE warns the **Instructors** before the Course Instance becomes Inactive six months after
  creation.
- The six-month Active limit prevents Course reuse or deadline extensions from indefinitely delaying
  FERPA retention and deletion.
- Course inactivity and FERPA deletion are separate transitions; becoming Inactive does not itself
  delete Student records.
- Retention should work equally for semesters, quarters, summer Courses, and other academic calendars.
- PLE should notify the **Instructor** before FERPA-sensitive Student data is archived.
- Archived Student data should leave normal Instructor and Student interfaces but remain recoverable during the retention period.
- FERPA-sensitive Student data should be permanently deleted when its retention period expires.
- Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
- FERPA retention intervals are operational configuration rather than separate product decisions.

### Retention processing

- A background process should periodically find Course Instances whose retention deadlines have passed.
- Retention decisions should come from stored Course dates and the Course Instance creation time.
- The background process should execute retention policy rather than define when retention periods begin or end.
- Running the retention process late should produce the same retention decision as running it on schedule.
- The retention process should be safe to run repeatedly.

### Revisions and history

- Be conservative about creating revisions.
- Assessments, Course Instances, and Draft Questions use current state.
- Published Questions, Question Pools, and Blueprint Courses have immutable revisions.
- Mutable working state uses a monotonic sequential Edit Number when needed for concurrency.
- An Edit Number is only a counter and does not identify a stored historical object.
- Question, Question Pool, and Blueprint Revision Numbers start at 1 and increase sequentially for
  each object.
- A Revision Number identifies a specific immutable Revision stored by PLE.
- Student Work records the exact Assessment Attempt and Published Question Revision delivered to the Student.
- Student Work records the Student's responses and the grading outcome returned by the Question Backend.
- Student Work records the Question Pool Revision and selected Published Question Revision for each response.
- Changes to Question point values recalculate scores from the stored grading outcome without changing the outcome.
- Changes to Assessment settings do not change the recorded history of completed Assessment Attempts.
- Immutable Question source and Question assets use SHA-256 checksums where needed to verify their stored contents.

### Dates and time zones

- Assessment deadlines are stored as instants.
- Instructor dates and times use the Instructor's IANA time zone.
- The Instructor's time zone is used to interpret dates and times the Instructor enters.
- Changing an Instructor's time zone changes how existing deadlines are displayed without changing the deadlines.
- Assessment deadlines are stored as absolute UTC instants.
- Students have their own IANA time zone for displaying dates and times.
- A Student's time zone defaults to the Instructor's time zone during the invite phase.
- Changing a Student's time zone changes how existing deadlines are displayed without changing the deadlines.
- Changing a display time zone changes how a deadline is shown, not the deadline itself.

## Questions

- Questions are subject agnostic. Properly tagged Questions from all subjects belong in the same Question Library.
- Questions are strictly and deterministically automated; grading does not require an **Instructor**.
- Questions have one canonical title. Compact interfaces may truncate that title.
- Every Question stored by PLE has its own internal Question record.

### Draft Questions

- Draft Questions are private working content.
- Draft Questions use current state rather than immutable Revisions.
- Saving a Draft Question replaces its previous working state.
- Instructors may delete Draft Questions they no longer need.
- PLE may clean up abandoned Draft Questions after an appropriate warning and recovery period.

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
- Question Backend feedback is transient unless the backend provides a robust way for PLE to preserve it.
- PLE does not extract or reconstruct transient feedback from Question Backend source or output.
- Questions may have PLE-managed general feedback that remains separate from backend-generated interaction feedback.
- H5P owns its runtime, interactions, state, and scoring.
- iMathAS owns its rendering and evaluation.
- Question Backends may support more complex interactions without requiring PLE to implement those interactions.
- A Question Backend returns an immutable credit fraction for each complete response it evaluates.
- PLE stores the immutable credit fraction as the grading outcome.
- When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
- Assessment scores are calculated from stored credit fractions and current Question point values.
- Changing Question point values recalculates scores without another Question Backend interaction.
- Preserve the distinction between WeBWorK PG and PGML source. A Question should be identified as PGML only when its source is fully PGML-compliant; otherwise identify it as PG.
- BiologyProblems.org imports should preserve whether the canonical algorithmic source is PG or PGML rather than treating both formats generically as PG/PGML.
- When parameterized WeBWorK PG or PGML source exists, prefer it to importing static variants.
- Preserve backend-native algorithmic variation rather than expanding one algorithmic Question into static variants.
- One algorithmic Question remains one Published Question regardless of how many variants its Question Backend can generate.
- Use a Question Pool with algorithmic Questions only when the Instructor wants selection among distinct Questions, not to represent variants of one algorithmic Question.
- BiologyProblems.org WeBWorK problems should be imported from their canonical algorithmic PG or PGML source rather than from generated static variants.
- Multiple static BiologyProblems.org questions generated from one algorithmic source represent one Published Question, not separate Published Questions or a Question Pool.

### Question Pools

- A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
- Pool contents should represent reasonably interchangeable assessments of the intended learning.
- Question Pools may contain Questions from any Question Backend.
- Each member of a Question Pool is a **Published Question**.
- Question Pools are always published and have no draft or unpublished state.
- A Question Pool is an independently reusable Question Library object.
- A Question Pool has its own public `AAAA-ZBBB` Crockford Base32 ID and immutable revisions.
- Importing a Question Pool into a new Assessment automatically forks the Question Pool.
- The fork belongs to the new Assessment and can be changed without changing the source Question Pool.
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
- **Students** access Question content through their Coursework rather than through the Question Library.
- Published content remains discoverable when used by a private **Course Instance**.
- With 13,000 Questions in Neil's first course, manually archiving Questions is unlikely to be a useful primary workflow.
- Question Library workflows should support bulk operations because an Instructor may manage thousands of Questions.
- Instructors should be able to select many Questions and update shared metadata such as tags, subject, topic, or other search fields together.
- Question Library search, filters, sorting, and bulk editing should make large imports practical to clean up.

#### Published Question identity

- Published Questions receive a public `AAAA-ZBBB` Crockford Base32 ID.
- Published Questions and published Question Pools have public Crockford Base32 IDs.
- Public IDs use the form `AAAA-ZBBB`.
- Seven Crockford Base32 characters are cryptographically random and provide the identity.
- The middle character is an HMAC-derived check character calculated from the seven identity characters.
- The check character detects mistyped or malformed IDs; it is not a security boundary.
- ID generation enforces database uniqueness and retries when a random collision occurs.
- IDs never encode creation order, Question Type, ownership, subject, or other metadata.

#### Published Question revisions, edits, and forks

- **Published Questions** maintain immutable revision history.
- Assessments and Student Work remain pinned to exact immutable Published Question Revisions.
- Publishing a new Question Revision does not silently change existing Assessments or Student Work.
- The Question owner may publish corrections, wording changes, accessibility improvements, answer changes, grading changes, and other updates as a new Revision.
- Changing Question source, answer content, grading rules, feedback, or Question assets creates a new Question Revision.
- Changing the Question title, description, tags, subject, topic, or other search metadata does not create a new Question Revision.
- Search metadata belongs to the Published Question as a whole rather than to one Revision.
- Any **Instructor** may fork a Published Question to create a separate Question with a new Question ID.
- A fork starts as a private **Draft Question** with its own authorship and lineage.
- A fork must pass Question Publication Validation before joining the Question Library.
- Published forks retain source attribution.
- Forced corrections are audited **Sysadmin** actions reserved for critical flaws.
- Question authorship, contributor credit, history, attribution, and compatible CC licensing are preserved across Revisions and forks.

#### Question stewardship

- Question stewardship should use a GitHub-like model.
- **Published Questions** can be starred and watched, similar to GitHub.
- Star means favorite and visible endorsement.
- Vetted **Instructors** can see the star count and which vetted **Instructors** starred a Question.
- Watch means subscription.
- Watching drives in-app notifications for revisions, forks, improvement threads, and impact notices.
- An **Instructor's** watch list remains private.
- **Students** and anonymous users do not receive **Instructor** identity lists or watch information.

#### Question statistics

- Privacy-safe aggregate Question statistics remain after the underlying Student records are deleted.
- Question statistics are kept separately for each Published Question Revision.
- Each Published Question Revision may retain aggregate counts of correct, incorrect, partial-credit, and unanswered results.
- Eligible Question Types may also retain aggregate answer-choice counts.
- Question-level statistics may combine Revisions when clearly labeled and privacy thresholds are met.
- Question statistics contain aggregate counts rather than Student Attempts or identifiable Student records.
- Student data retention removes the underlying Student evidence without removing approved aggregate Question statistics.
- Removing Student names alone does not make statistics anonymous.
- Shared Question statistics should be shown only when individual Students cannot reasonably be identified from the aggregate.
- Course-specific Question analysis remains FERPA-sensitive when individual Students could be inferred.

#### Question behavior

- Answer-choice randomization belongs to the Question.
- PLE-native Questions control their own answer-choice randomization.
- Question writers may add optional Question Feedback when it helps.
- Optional Question Feedback is shown when the Question Backend provides it.
- Question Feedback does not use Assessment correct-answer disclosure settings.
- Student workflows remain complete whether or not Students read Question Feedback.

## Courses

- **Courses** organize reusable teaching content and its delivery to **Students**.
- PLE has two Course forms: **Blueprint Courses** and **Course Instances**.
- **Blueprint Courses** provide reusable course designs for creating Course Instances.
- Course Instances may be created from a Blueprint Course or started empty.
- A Course can have multiple co-**Instructors** with equal teaching authority.
- **Sysadmins** can create Courses, but **Instructors** teach them.
- Every Course Instance must have at least one assigned **Instructor**.
- Creating a Course Instance establishes its first Instructor membership but does not give that Instructor greater Course authority than later co-Instructors.


### Blueprint Courses

- **Blueprint Courses** are reusable course definitions for building **Course Instances**.
- Blueprint Courses are a similar concept as LibreTexts' ADAPT alpha courses.
- Blueprint Courses have no **Students**, deadlines, or other teaching-specific delivery settings.
- Blueprint Courses do not contain dates or relative schedules.
- Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
- Blueprint Courses contain only **Published Questions** and published **Question Pools**.
- An **Instructor** may deliberately publish an existing Course Instance structure as a new Blueprint Course.

#### Blueprint Course lifecycle

- Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
- New Blueprint Courses and forks start Private.
- Private Blueprint Courses are visible only to their owning **Instructor**.
- Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
- Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
- Public Blueprint Courses can be adopted to create daughter Course Instances.
- Archived Blueprint Courses are read-only and no longer actively maintained.
- Archived Blueprint Courses remain visible by every vetted **Instructor**.
- Archived Blueprint Courses are excluded from normal search results unless the search explicitly includes them.
- Archived Blueprint Courses cannot be adopted to create new daughter Course Instances.
- Archived Blueprint Courses can be forked but not adopted.
- The owning **Instructor** can return an Archived Blueprint Course to Public before adopting it again.
- Other **Instructors** can fork an Archived Blueprint Course to create a new Private Blueprint Course.
- Blueprint Courses have no separate draft state.

#### Blueprint Course revisions

- Blueprint Courses use immutable **Blueprint Revisions** for saved reusable content.
- Blueprint Course content editing uses explicit Save.
- Saving changed Blueprint content creates the next Blueprint Revision.
- Multiple content edits before Save become one Blueprint Revision.
- Saving unchanged Blueprint content does not create another Revision.
- Blueprint Course metadata can change without creating a Blueprint Revision.
- Blueprint Course names are metadata and identify the Blueprint across Revisions.
- Changing a Blueprint Course name does not create a new Blueprint Revision.

#### Blueprint Course stewardship

- **Instructors** can Star or Watch Public and Archived Blueprint Courses.
- A Star is a visible endorsement and helps **Instructors** save useful Blueprint Courses.
- Vetted **Instructors** can see who Starred a Blueprint Course and its Star count.
- Watching a Blueprint Course is private.
- Watchers are notified about new Blueprint Revisions and other important Blueprint changes.
- Forking or adopting a Blueprint Course does not automatically Star or Watch it.
- Stars and Watches belong to the Blueprint Course across all of its Revisions.

#### Blueprint adoption and updates

- Blueprint adoption copies every Assessment from the Blueprint Course into the Course Instance.
- Course Instances pin the exact Blueprint Revision from which they were adopted.
- New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
- Routine Blueprint updates should be quick for an **Instructor** to review and approve.
- It should be obvious when a Course Instance is using an older Blueprint Revision.
- Changes to existing Assessments follow the Blueprint Revision update workflow.
- Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
- Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.

#### Blueprint Course forks and Change Proposals

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

#### Blueprint Course JSON

- Blueprint Courses have a canonical JSON representation for comparison, import, export, and exchange.
- Canonical Blueprint JSON must contain enough information to fully recreate a Blueprint Course.
- Importing exported Blueprint JSON should reproduce the same Blueprint Course content and structure.
- Blueprint JSON contains Blueprint metadata and an ordered list of Blueprint Assessments.
- Blueprint Assessments contain only reusable teaching settings.
- Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
- Blueprint Assessments have no deadlines, release dates, Student data, or other Course Instance settings.
- Blueprint Revisions can be compared through their canonical JSON representations.
- Blueprint Course Change Proposals use canonical JSON to identify changes between Blueprint Revisions.
- Canonical Blueprint JSON may support offline inspection or editing, even if it is not optimized for hand editing.
- Canonical Blueprint JSON is the complete exchange format, not the primary persistence model.

### Course Instances

#### Course Instance creation

- An **Instructor** can create a Course Instance from a Public Blueprint Course.
- **Instructors** can also create a new empty Course Instance without a parent Blueprint Course.
- Course Instances have **Students**, deadlines, releases, and other delivery-specific settings.
- Course Instances contain only **Published Questions** and published **Question Pools**.
- Course Instances are visible only to their co-**Instructors** and enrolled **Students**.
- Active Courses are current teaching Course Instances.
- Inactive Courses are past Course Instances and retain Course metadata, including after
  FERPA-sensitive Student data is removed.
- An **Instructor** may deliberately publish reusable Course Instance structure as a new **Blueprint Course**.
- A new academic term uses a new Course Instance. Rollover is not a separate product model.

#### Blueprint adoption and daughter Course Instances

- An **adoption** occurs when an **Instructor** creates a Course Instance from a Blueprint Course.
- Blueprint Courses track how many Course Instances have been created from them as their adoption count.
- A Course Instance created from a Blueprint Course is a daughter Course Instance of that Blueprint Course.
- A daughter Course Instance records its parent Blueprint Course and the exact Blueprint Revision used to create it.
- Creating a Course Instance from a Blueprint Course counts as an adoption of that Blueprint Course.
- The new Course Instance receives every Assessment from the selected Blueprint Revision.
- Creating a Course Instance from a Blueprint Course copies its Assessments, Questions, Question Pools, and reusable settings.
- Course Instance Assessments created from a Blueprint Course start unreleased with dates unset.
- New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
- Routine Blueprint updates should be quick for an **Instructor** to review and approve.
- It should be obvious when a daughter Course Instance is using an older Blueprint Revision.
- Changes to existing Assessments follow the Blueprint Revision update workflow.
- Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
- Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.

### Course names

- Blueprint Courses and Course Instances each have their own short name and long name.
- Short names are entered or chosen deliberately by **Instructors**.
- Short names are for compact navigation and should stay under about 16 characters when practical.
- Long names are descriptive names used for headings, breadcrumbs, and Course listings.
- A Blueprint Course might be `Biochemistry` / `Upper-Level Introductory Biochemistry`.
- A Course Instance might be `BCHM 355/455` / `BCHM 355/455 Section 20 Biochemistry (Roosevelt U; Spring 2026)`.
- Course Instance names are properties of the Course Instance and are not derived from Blueprint Course names.

## Assessments

- **Assessment** is the PLE object for organizing Questions into a graded or practice activity.
- PLE has **Blueprint Assessments** and **Course Instance Assessments**.
- Blueprint Assessments define reusable Assessment content and teaching settings.
- Course Instance Assessments deliver Questions to **Students**.
- All Assessments use the same underlying Assessment model.
- **Assignment** is not a separate object or category. The word appears only in the names
  **Regular Assignment**, **Practice Question Assignment**, and **Bonus Assignment**.

### Assessment content

- Assessments contain an ordered sequence of Questions and Question Pools.
- **Instructors** can add, remove, and reorder Questions and Question Pools.
- Questions and Question Pools remain distinct even though both can occupy positions in an Assessment.
- Assessment Question-order randomization is called **Randomize question order**.

### Assessment types

- PLE defines the available Assessment Types.
- Assessment Type describes the pedagogical purpose of an Assessment and provides appropriate defaults.
- Assessment Types are **Regular Assignment**, **Practice Question Assignment**, **Bonus Assignment**, **Quiz**, and **Exam**.
- **Instructors** select an Assessment Type but cannot create new Assessment Types.
- Blueprint Assessments and Course Instance Assessments use the same Assessment Types.
- **Instructors** can change Assessment settings independently of the defaults for its Type.
- Changing Assessment settings does not change its Assessment Type.
- **Regular Assignments** give **Students** regular practice applying course ideas outside class.
- Regular Assignments reinforce current learning and may also introduce new topics.
- Regular Assignments are designed as practice for learning, not merely as one-time assessments.
- **Practice Question Assignments** provide focused review or study-guide practice using material already covered.
- Practice Question Assignments may be worth a small number of points or a small amount of extra credit.
- Practice Question Assignments use the same whole-Attempt submission boundary as every other
  Assessment and show the correct answer immediately after that Assessment Attempt is submitted.
- **Bonus Assignments** provide optional extra credit.
- Bonus Assignments are worth zero points possible and add earned points directly to the grade.
- **Quizzes** assess understanding of recent material.
- Quizzes may use more restrictive Attempt and collaboration settings than Regular Assignments.
- **Exams** are individual assessments associated with scheduled exam periods.
- Exams may use more restrictive Attempt, timing, availability, and feedback settings.
- Quizzes and Exams allow one Assessment Attempt.

### Assessment type appearance

- Each Assessment Type has its own PLE-defined Font Awesome icon.
- Assessment Type icons remain consistent across PLE themes.
- Each Assessment Type also has its own theme-defined color.
- Themes may change Assessment Type colors but preserve the meaning of each Type.
- Assessment Type should never be communicated by color alone.
- Icons and labels should remain sufficient to identify the Assessment Type without color.
- **Regular Assignment** uses the Font Awesome `pen-to-square` icon.
- **Practice Question Assignment** uses the Font Awesome `arrows-spin` icon.
- **Bonus Assignment** uses the Font Awesome `star` icon.
- **Quiz** uses the Font Awesome `circle-question` icon.
- **Exam** uses the Font Awesome `file-signature` icon.

### Blueprint Assessments

- A **Blueprint Assessment** is an Assessment in a **Blueprint Course**.
- Blueprint Assessments define reusable Assessment content and teaching settings.
- Blueprint Assessments have an Assessment Type.
- Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
- Blueprint Assessments define Question point values and points possible.
- Blueprint Assessments have no **Students**, Student Work, due dates, release dates, or other Course Instance delivery settings.
- Blueprint Assessments do not use Assessment Templates.
- Creating a daughter Course Instance from a Blueprint Course copies its Blueprint Assessments into the Course Instance.

### Course Instance Assessments

- A **Course Instance Assessment** is an Assessment in a **Course Instance**.
- Course Instance Assessments are the Assessments delivered to **Students**.
- Course Instance Assessments have an Assessment Type, Questions, Question Pools, point values, and points possible.
- Course Instance Assessments also have delivery settings such as due dates, release status, and Student availability.
- Course Instance Assessments copied from a Blueprint Assessment can be changed for the needs of that Course Instance.
- Newly added Blueprint Assessments are automatically copied to daughter Course Instances as unreleased Course Instance Assessments.

### Assessment Templates

- An **Assessment Template** is a reusable set of settings for creating Course Instance Assessments.
- Assessment Templates are separate from Assessment Types.
- Every Assessment Template has one of the five Assessment Types.
- **Instructors** can create and change their own Assessment Templates.
- Assessment Templates provide defaults for settings such as Attempts, timing, scoring, and disclosure.
- Creating a Course Instance Assessment from a Template copies its settings into the new Assessment.
- The new Course Instance Assessment can be changed independently after it is created.
- Changing an Assessment Template does not change Assessments previously created from it.
- Assessment Templates do not contain Questions or Question Pools.
- Blueprint Assessments do not use Assessment Templates.

### Course Instance Assessment release and defaults

- Course Instance Assessments start unreleased.
- Releasing a Course Instance Assessment requires an automated and interactive **Assessment Release Validation** process.
- Assessment Release Validation checks the Assessment settings and data required for release.
- Validation should catch missing, invalid, or unreasonable values and explain what the **Instructor** needs to fix.
- Release Validation should require a due date at least 24 hours in the future and no later than the
  Course Instance's six-month Active limit.
- Release Validation should check that release, due, and other dates occur in a valid order.
- Release Validation should check required settings such as point values, Attempt limits, and time limits for valid ranges.
- Release Validation should check that the Assessment contains Questions and that required Question settings are valid.
- The **Instructor** should be able to correct validation problems and run Release Validation again.
- An Assessment can be released only after Release Validation passes.
- Releasing an Assessment makes it available to **Students** according to its dates and access settings.
- Student Work begins when a **Student** starts an Assessment Attempt.
- New Course Instance Assessments default to accepting submissions only through the due date.
- New Course Instance Assessments default to starting new Attempts only through the due date.
- Late work defaults to rejected.
- Assessment disclosure settings remain separate and independently configurable.
- **Regular Assignments** and **Bonus Assignments** should rarely show the correct answer.
- Regular and Bonus Assignments show the **Student's** response and whether it was correct or incorrect.
- **Practice Question Assignments** show correct answers immediately after Assessment Attempt
  submission.
- **Quizzes** and **Exams** show correct answers after all **Students** in the Course have completed the Assessment.
- A Quiz or Exam Attempt is complete when the **Student** submits it or its time limit expires and
  PLE submits it automatically.
- Assessment Attempt completion does not depend on correctness or score.
- Until then, Quizzes and Exams do not disclose correct answers.
- Optional Question Feedback is shown when the Question Backend provides it.
- Question Feedback does not use Assessment correct-answer disclosure settings.
- Unreleasing a Course Instance Assessment permanently deletes its Student Work and returns to a pre-release state.

### Assessment Attempts

- An **Assessment Attempt** is one Student attempt at a Course Instance Assessment.
- Blueprint Assessments do not have Assessment Attempts.
- Question responses are saved as the **Student** works and remain part of the Attempt across browser sessions.
- **Instructors** control the number of permitted Assessment Attempts.
- Regular Assignments default to unlimited Attempts.
- **Students** may repeat an Assessment as often as its settings allow, including practicing toward a perfect score.
- When an Assessment permits multiple Attempts, the highest Assessment Attempt score is used as the
  Student's Assessment score.
- Assessment Attempt submission and grading are fully automatic and require no **Instructor** action.
- Automatic grading does not require a separate Student or **Instructor** grading workflow.

### Assessment responses and submission

- The Student submission action submits the whole Assessment Attempt.
- A Question either has a complete saved response or has no saved response.
- PLE saves complete Question responses as the **Student** works.
- The Student may change a saved response while the Assessment Attempt remains open.
- Submitting the Assessment Attempt finalizes all saved Question responses together as Student Work.
- Questions without a saved response remain visibly unanswered when the Attempt is submitted.
- An unanswered Question receives zero credit and counts as incorrect without being sent to the
  Question Backend.
- PLE treats an incomplete Question response as unsaved, although the Question interface may keep the Student's unfinished input while they work.
- A Question Backend may evaluate a response before Assessment submission when needed for its interaction.
- When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
- The **Student** does not see the grading outcome until the Assessment Attempt is submitted.

### Assessment Attempt timing and expiration

- Each Assessment Attempt has a time limit.
- Attempt time limits help **Students** develop an accurate sense of expected working speed.
- Timed Assessment Attempts use wall-clock time.
- The server owns the Attempt start and expiration times.
- Attempt time continues while the **Student** is disconnected or the browser is closed.
- A **Student** may reconnect, reload, or use another browser session to resume the same active Attempt.
- Resuming an Attempt does not reset, pause, or extend its time limit.
- Attempt expiration is checked whenever a **Student** interacts with the Attempt.
- Background processing ensures expired Attempts are submitted even when the **Student** is no longer connected.
- When an Attempt expires, PLE submits the whole Attempt, finalizing its saved responses. Other
  Questions remain visibly unanswered, receive zero credit, and count as incorrect without being
  sent to the Question Backend.

### Student Work

- Student Work keeps the exact Published Question Revision delivered to the **Student**.
- For a Question Pool, Student Work keeps the exact Question Pool Revision and Published Question Revision selected.
- Student Work keeps each saved response as finalized with the submitted Attempt and the grading outcome returned by the Question Backend.
- Changes to Assessment content do not replace Question evidence already delivered in existing Attempts.
- PLE should retain only the additional historical Student Work data needed to interpret or grade that work correctly.

### Assessment scoring

- Blueprint Assessments and Course Instance Assessments assign point values to Questions.
- A Question Backend returns an immutable credit fraction for each complete response it evaluates.
- PLE stores the credit fraction as the Question grading outcome.
- Course Instance Assessment scores are calculated from stored credit fractions and current Question point values.
- An unanswered Question contributes zero points to the Assessment score and counts as incorrect.
- When an Assessment has multiple submitted Attempts, the highest Assessment Attempt score is the
  Student's Assessment score.
- PLE uses Question point values directly to calculate Assessment scores.
- PLE does not use separate Question weights, Grade Categories, weighted categories, Course Grade
  Schemes, or Course percentage calculations.
- For the pilot, grade export uses CSV or TSV only and exports point-based Assessment scores.
- The Instructor handles Course-level weighting or percentage calculations in the home LMS.
- Changing Question point values recalculates affected Assessment scores.
- Score recalculation does not require another Question Backend interaction.
- Score recalculation does not change the stored Question grading outcome.
