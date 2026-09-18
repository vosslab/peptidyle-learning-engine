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
- Headings should identify their broader section when practical so they remain clear in isolation.
- Avoid repeating that context when the immediate parent heading already makes it clear.
- Sections should contain less than 25 bulleted items; split a larger section with subheadings.
  `devel/markdown_section_sizes.py -i docs/HUMAN_GUIDANCE.md -m 25` lists any that grew past it.

## Deferred product behavior

- Items in this section remain desired product behavior but are not current implementation
  requirements.
- This section overrides implementation language elsewhere in this document until an item is moved
  out of this section.
- all automated daemon backends are deferred until final server location
- all automated AI/LLM backends are deferred until final server location
- AI-backed Bloom classification is desired but deferred until a later release.
- Initial Bloom Classification is deferred with the AI backend.
- Bloom Classification does not block publication or Question Library entry.
- Any AI backend is desired but deferred and low priority for the current PLE.
- No specific AI backend is selected.
- iMathAS is a desired Question Backend deferred until a later release.
- H5P is a desired Question Backend deferred until a later release.
- Future H5P use is limited to Regular Assignments, Bonus Assignments, and Practice Question
  Assignments.
- Quizzes and Exams do not use H5P because its runtime exposes answers and correctness to the
  Student browser.
- public API for instructors to use AI to control their classes.
- public API perhaps modeled after BrickLink OAuth https://www.bricklink.com/v3/api.page?page=auth

## Development principles

### Agent working principles

- Read and learn the core principles in docs/REPO_STYLE.md
- Apply the Keep It Simple, Stupid (KISS) philosophy aggressively.
- Prefer the smallest coherent design that meets actual requirements and known failure modes.
- Complexity must earn its place.
- Time should be used efficiently. Agents and tokens are cheap; wall time is not.
- Hard work should be broken into small, independently completable tasks.
- Write plans in plain, concrete language. Use technical terms when they add precision.
- Prioritize positive prompting. Phrase instructions as concrete actions such as "Do X" or "Use Y".
- Name only the tools and responsibilities needed for the assigned task. Positive prompting plus
  omission keeps agent instructions focused on the intended actions.
- Small LMs may interpret negative instructions as actions to perform. State the desired behavior
  directly, including when assigning responsibilities to agents.
- Python code uses the current interpreter's defaults; `from __future__ import ...` belongs nowhere
  in this repo. `tests/test_no_future_imports.py` enforces it.
- Long local operations must be robust and informative: keep going through imperfect state where
  useful, recover gracefully, and tell me what is happening while I wait. I am impatient.
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
- PLE is pre-production with no users or durable production data. Fix the design directly;
  there is no legacy behavior to preserve.
- Use the pre-production state to improve foundational schemas, contracts, and abstractions
  whenever that produces a stronger long-term system.
- Use SQL directly to create the initial PostgreSQL database.
- Before production, edit the main database design directly as the design changes.
- After production, update existing databases without rebuilding them from scratch.
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

## Product vocabulary and glossary

- **Account**: A global PLE user account with exactly one Product Role.
- **Product Role**: The Account's global role in PLE: **Student**, **Instructor**, or **Sysadmin**.
- **Sysadmin**: A PLE administrator who manages the system, approves **Instructors**, creates Accounts, and provides scoped administrative support.
- **Instructor**: An approved user who teaches Courses and can browse, reuse, create, fork, and publish Questions.
- **Student**: A user who enrolls in **Course Instances** and completes Coursework.

### Course vocabulary

- **Course**: The general term covering both **Blueprint Courses** and **Course Instances**.
- **Blueprint Course**: A reusable Course used to create **Course Instances**. It has no enrolled **Students**, deadlines, or other teaching-specific delivery settings.
- **Blueprint Revision**: A fixed version of a **Blueprint Course** preserved so its reusable content cannot change.
- **Course Instance**: A Course used for teaching. It has **Students**, deadlines, releases, and other delivery settings. It may start independently with no parent **Blueprint Course**, or an **Instructor** may create it from a **Blueprint Course**.
- **Adoption**: A connection between a **Blueprint Course** and a **Course Instance**. An **Instructor** establishes Adoption by creating a new Course Instance from a Blueprint Course or by creating a new Blueprint Course from an existing Course Instance's reusable structure.
- **Create Blueprint from Course Instance**: Creating a new **Blueprint Course** from an existing Course Instance's reusable structure. The new Blueprint Course records the existing Course Instance as its source, and that Course Instance remains the same teaching instance.

### Assessment vocabulary

- **Assessment**: The PLE object that organizes Questions into a graded or practice activity.
- **Blueprint Assessment**: An Assessment in a **Blueprint Course** containing reusable content and teaching settings without Students, dates, or other Course Instance delivery settings.
- **Course Instance Assessment**: An Assessment in a **Course Instance** that can be released and delivered to **Students**.
- **Assessment Type**: The pedagogical type of an Assessment: **Regular Assignment**, **Practice Question Assignment**, **Bonus Assignment**, **Quiz**, or **Exam**.
- **Coursework**: The Student-facing collective term for Regular Assignments, Practice Question Assignments, Bonus Assignments, Quizzes, and Exams.
- **Assessment Attempt**: One **Student** attempt at a Course Instance Assessment.
- **Assessment Template**: A reusable set of settings for creating Course Instance Assessments. It contains settings rather than Questions.
- **Assessment Question Editor**: The **Instructor** editor for selecting, adding, removing, and ordering Questions in an Assessment.
- **Assessment Properties Editor**: The **Instructor** editor for settings that apply to the whole Assessment, such as dates, scoring, Attempts, late work, and what **Students** can see.

### Question vocabulary

- **Question**: The general PLE object representing one automatically evaluated question, regardless of its Question Backend.
- **Draft Question**: A private Question being developed by an **Instructor**. It must pass publication validation before becoming a Published Question.
- **Published Question**: An immutable-revision Question available for reuse through the global **Question Library**.
- **Question Revision**: A fixed version of a **Published Question** preserved so Assessments and Student Work can refer to the exact Question delivered.
- **Question Pool**: A published **Library Object** containing interchangeable **Published Questions** from which PLE selects Questions for a **Student**.
- **Question Backend**: The component responsible for a Question's rendering, interaction, response handling, grading, feedback, and backend-specific state.
- **Question Type**: Author-declared educational metadata describing the Question's interaction type, such as MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, or HOTSPOT.
- **Question Library**: The global collection of **Published Questions** and **Question Pools** available to vetted **Instructors**.
- **Library Object**: A **Published Question** or **Question Pool** in the **Question Library**.

### Student Work vocabulary

- **Student Work**: The collective term for FERPA-sensitive records created by a **Student** in a **Course Instance**, including Assessment Attempts, saved Question responses, grading outcomes, and the evidence needed to interpret submitted work.
- **Grading Outcome**: The immutable credit fraction returned by a **Question Backend** for a complete evaluated response and stored by PLE.

### Content classification vocabulary

- **Discipline**: The broadest academic classification, such as Biology, Chemistry, or Mathematics.
- **Subject**: A globally named area associated with one or more Disciplines, such as Genetics,
  Biochemistry, or Ecology.
- **Topic**: A major area within a Subject.
- **Subtopic**: A narrower classification within a Topic.
- **Tag**: An optional label attached to a Course or Library Object. Each may have any number of Tags.

## Accounts and roles

### Account rules

- PLE accounts should be global across PLE and use passwordless passkeys and email authentication.
- Email is not configured for the Live Demo yet; use the visible seeded-role entry for demo access.
- The local Live Demo should not enforce a single browser origin; I want to reach it over a
  firewalled LAN or Tailscale by binding to 0.0.0.0. The local TLS certificate stays because the
  login security tests depend on it.
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

- Design around what users need to find and do.
- Important information should stand out from supporting information.
- Related information should be visually grouped and aligned.
- Similar pages should place similar controls in consistent locations.
- Use headings and action labels that reflect the current state and next useful step.
- Match feedback wording and visual emphasis to the outcome: success, information, warning, or
  error. Make the result and any next action easy to recognize.
- Primary actions should be easy to find and appear near the content or workflow they affect.
- Identify the object and relevant context before an action that changes membership or stored
  settings, so users can recognize what they are accepting or changing.
- Use concise helper text near the control it explains. Present shared explanations once per
  relevant group and keep the main task information easy to scan.
- Avoid scattering related actions across page headers, menus, navigation, and content areas.
- Dream big on the UI. Choose one visual philosophy and carry it through the entire interface.
- Students should have no upload capabilities. Instructor-created content should use text boxes.
- Buttons should look intentionally designed rather than like native browser controls.

### Rounded rectangles preference

- Rounded rectangles are preferred for all interface objects, especially buttons, input fields, cards, avatars, tags, and interactive controls.
- Rounded corners generally feel softer, friendlier, and more contemporary.
- Rounding also helps users visually distinguish discrete objects from the surrounding page.
- Use corner radius to reinforce interface hierarchy.
- Interactive and self-contained objects should generally be more rounded than structural containers.
- Use moderate rounding for buttons, input fields, answer choices, dialogs, and similar interactive controls.
- Use subtle rounding for cards, tables, panels, and other content containers.
- Keep large page regions, navigation bars, breadcrumbs, and other structural layout elements square or nearly square.
- Pills and fully rounded shapes should be reserved for compact objects such as tags, badges, timers, and avatars.
- Apply corner radii consistently to objects that serve the same purpose.
- Use the application's typography, spacing, corner radius, borders, and interaction states consistently.
- Primary, secondary, and low-emphasis actions should be visually distinct.

### Information density and layout

- Design Instructor and **Sysadmin** workflows for laptop browsers, using a 1280 by 800 viewport
  as the layout target.
- PLE often presents large collections where users need to find a few relevant items.
- Optimize large collections for scanning, searching, filtering, and comparison.
- Show enough useful information at once to support comparison without excessive scrolling.
- Search and filters should help users quickly narrow large collections.
- Dense pages should remain easy to scan.
- Treat screen space as a limited resource. Prefer useful information over decorative whitespace.
- Use spacing to separate meaningful groups rather than simply making pages spacious. Large gaps should communicate a meaningful change in section or task.
- Prefer alignment, typography, and dividers over unnecessary cards, boxes, borders, and nested containers.
- Cards and rounded containers should earn their space by representing a distinct object or interaction, not merely grouping nearby content.
- Avoid the modern dashboard style of large rounded cards, generous padding, and isolated islands of content.
- Use horizontal and vertical space efficiently without crowding information together. Related information should form clearly readable rows, columns, or groups.
- Size controls and content regions for their contents and task. Avoid unnecessarily tall panels, empty states, Question previews, and other fixed-height regions.
- Keep the visual design compact, flat, information dense, and consistent across PLE.
- Use compact rows, restrained corner rounding, and controls sized to their task.
- Present short labels and values in aligned rows or compact grids, adapting to stacked groups
  when the available width requires them.
- Give each object one clear title within its list entry. Group its metadata and actions beneath
  or alongside that title.
- Preserve readable text and reachable controls as users enlarge text or zoom the page.

### Interaction design

- Use progressive disclosure to keep common tasks compact while making supporting details easy
  to find.
- Use tooltips for brief supplementary explanations, available on hover and keyboard focus.
- Use clearly labeled expandable sections with chevrons for longer details and secondary settings,
  supporting keyboard, pointer, and touch interaction.
- Keep essential information, primary actions, and current status visible in the main interface.
- Use drag-and-drop where it makes reordering faster and more natural.
- Reordering must also have a precise keyboard-accessible method.
- UUIDs should never appear in visible content, navigation URLs, or copyable links.

### Role colors and themes

- **Sysadmin** uses tomato red as its role color.
- **Instructor** uses teal green as its role color.
- **Student** uses lavender purple as its role color.
- Role colors should be used consistently in role labels and other appropriate interface cues.
- Demo role selection should clearly state both the user's role and name.
- Courses use a fixed set of visually distinct biome and habitat themes.
- Course Themes should have coordinated light and dark appearances.
- Course Theme colors should remain accessible in their actual interface uses.
- Light themes should use clearly light page backgrounds; dark themes should use clearly dark page
  backgrounds. Use theme colors as accents on surfaces with readable contrast.
- Check text, controls, borders, and interaction states against their actual rendered backgrounds.
- Apply the contrast requirements for text, controls, and other semantic uses in
  [BIOME_THEME_PALETTES.md](/docs/BIOME_THEME_PALETTES.md)
  to rendered components in both light and dark themes, including gradients and state backgrounds.
- Pair color cues with text, icons, or shapes so selection, focus, saved status, and results remain
  recognizable across themes and color-vision differences.
- Course Theme IDs are durable; changing a theme's display name or colors should not require a new ID.
- Follow `docs/BIOME_THEME_PALETTES.md` for Course Theme names, palettes, accessibility, and implementation.

### Typography

- Use [Atkinson Hyperlegible Next](https://www.brailleinstitute.org/freefont/) as the main PLE font.
- Use [Atkinson Hyperlegible Mono](https://www.brailleinstitute.org/freefont/) for code and other monospace text.
- Prefer the official Braille Institute font files and include the needed weights locally with PLE.
- When a narrow font is needed, use `IBM Plex Sans Condensed` for long unbreakable strings such as URLs.
- With `IBM Plex Sans Condensed`, try `font-variant-numeric: slashed-zero` to better distinguish `0` from `O`.
- Question Backend-rendered content may use its own fonts when needed for correct display.

### Ribbon and page layout

- The top Ribbon is the persistent navigation area for signed-in PLE pages.
- The Ribbon should remain in the same location and use the same overall structure while navigating.
- Navigation choices should remain in predictable locations as users move between related pages.
- Changing a Ribbon selection changes the content below the Ribbon without moving the main content area up or down.
- Ribbon rows should keep their space when needed so changing selections does not make the content area jump.
- Page actions should appear near the content they affect rather than changing the Ribbon layout.
- On narrow Student screens, use a compact navigation arrangement that keeps the product identity,
  current location, navigation controls, and Profile readable and reachable.
- See **User top bar** and **Breadcrumbs** for the persistent elements that make up the top of the page.

### User top bar interface

- All signed-in users share the same top-left logo/account and top-right profile bar layout.
- The top bar remains in a consistent location as users navigate.
- The PLE logo and product name appear at the upper left and link to the user's home dashboard.
- Each Product Role has its own home dashboard and navigation.
- Product Role appears once next to the PLE name.
- Role-specific navigation appears between the product identity and Profile.
- Profile appears at the far right as an icon-only avatar.
- Clicking the Profile avatar opens the Profile menu.
- The Profile menu contains Profile settings, account settings, and Sign Out.
- Sign Out belongs in the Profile menu rather than the main top bar.
- See **Ribbon and page layout** for the overall navigation and page-position rules.

### Profile avatar interface

- Every Account is randomly assigned an avatar from the PLE avatar gallery when the Account is created.
- The same avatar gallery collection is available to all Product Roles.
- The current avatar or Profile image appears consistently anywhere PLE represents that user.

#### Student avatars

- **Students** select avatars from the PLE-provided avatar gallery collection and cannot upload Profile images.
- Student avatar selection should be visual and playful.
- All avatars in the gallery are available for selection.
- Students may select another avatar at any time.

#### Instructor and Sysadmin Profile images

- **Instructors** and **Sysadmins** share the same Profile backend and functionality.
- **Instructors** and **Sysadmins** may select from the PLE avatar gallery or upload their own Profile image.
- Image upload accepts any aspect ratio with a minimum of 128 pixels in both dimensions.
- After upload, Instructors and Sysadmins can position and crop the image within a square Profile preview.
- Instructors and Sysadmins may replace their Profile image or select a provided avatar at any time.
- The current avatar or Profile image appears consistently anywhere PLE represents that user.

### Breadcrumbs interface

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
- Keep the teaching content central in authoring and inspection workflows, with metadata and
  supporting explanations arranged compactly around it.
- Gradebook rows should identify Students by their Course roster names and Coursework by title,
  with reference IDs as supporting information where useful.
- The Instructor menu has **Courses**, **Questions**, and **Assessments** in one dense top bar.
- Instructor Profile uses a generic user icon until the **Instructor** adds a Profile image.
- All required ribbon choices remain visible even when their collection is empty.
- All required Instructor ribbon choices remain visible even when their target page is not implemented or complete.
- A working navigation destination remains visible when its collection is empty.
- A future or unavailable capability should not appear as a usable control until its workflow exists.
- Empty collection pages should explain what the collection is for and provide an obvious action to create or add the first item when the user can do so.
- Similar pages should place similar actions in consistent locations.
- Instructor pages should be composed around the teaching task rather than collections of padded components.
- Instructor lists and repeated records should be dense and easy to scan, more like a spreadsheet than cards.
- Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.
- Instructor lists and repeated records should favor compact rows or tables with clear columns over cards or loosely concatenated text.
- At 1280 x 800, Instructor pages should expose enough of the current workflow to minimize unnecessary scrolling.

#### Course interfaces

- The **Courses** ribbon must include: My Blueprint Courses, My Active Courses, My Inactive Courses, Search Public Blueprint Courses.
- Course lists should support scanning and comparison without opening each Course.
- The Course Editor should show the Course structure and its ordered Assessments without showing every Question at once.
- Selecting an Assessment in the Course Editor opens that Assessment for editing.
- Assessment content and Assessment properties should remain separate editing tasks.
- My Active Courses and My Inactive Courses should both be available from the Courses area.
- Course Discipline selection should provide a clear way to request a new Discipline when the needed
  Discipline is unavailable.

##### Blueprint Course interface

- **My Blueprint Courses** should emphasize reusable course design rather than teaching activity.
- **Search Public Blueprint Courses** helps Instructors find relevant Blueprint Courses in a growing shared collection.
- Public Blueprint Course search should combine ordinary text search with shared classification
  filters beginning with Discipline and following Discipline -> Subject -> Topic -> Subtopic.
- Selecting a Discipline should limit Subject choices to Subjects associated with that Discipline.
- After selecting a Subject, Instructors should have an explicit option to include Blueprint Courses
  associated with that Subject across its other Disciplines.
- Tags should provide additional filters outside the hierarchy.
- Search results should use a compact, information-rich layout that supports scanning and comparison.
- Results should show Course name, classification, author, institution, and useful usage or
  stewardship signals directly in the result list to support scanning and comparison.
- Public Blueprint Course search should support sorting by relevant fields such as Stars, Watches, Adoptions, Students who have taken the Course, and most recent edit.
- Search terms, active filters, and the selected sort should remain visible while reviewing results.
- Clearing or changing part of a search should be quick.
- Opening a result and returning should preserve the Instructor's search, filters, sort, and scroll
  position.
- A **Blueprint Course** should provide an obvious action for creating a **Course Instance** from it.

##### Blueprint Course editing interface

- Blueprint Course editing should follow Course Editor -> Blueprint Assessment Editor.
- The Course Editor should show the Blueprint Course structure without editing every Question on one page.
- Selecting a Blueprint Assessment in the Course Editor opens the editor for that Blueprint Assessment.
- Only the selected Blueprint Assessment's Questions should appear in its editor.
- **Blueprint Assessment Question Editor**: Selects, adds, removes, and orders Questions in a Blueprint Assessment.
- **Blueprint Assessment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see.
- Blueprint Courses should not contain Assessment dates or relative Assessment schedules.

##### Course Instance interface

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

#### Question interface

- The **Questions** ribbon must include: My Questions, My Draft Questions, Starred, Watched, Search Question Library, Browse Question Library.
- **My Questions** should make the Instructor's Published Questions easy to find and manage.
- **My Draft Questions** should emphasize Questions that still need work before publication.
- **Starred** should provide a quick personal collection of Questions the Instructor wants to keep handy.
- **Watched** should help Instructors follow Questions where changes or activity matter to them.
- Published Questions should offer a **Create Pool from Question** action.
- Pool creation should show the starting Question and its Discipline and Subject alongside the
  Pool Title field.
- Creating the Pool includes the starting Question and uses its Discipline and Subject.
- Adding Questions to a Pool should begin with Question Library results filtered to the Pool's
  Discipline and Subject.

##### Search Question Library interface

- **Search Question Library** helps Instructors find specific Questions in a large library.
- Search should begin with a prominent search box, similar to Google Search.
- The initial Search page should stay simple and focus attention on entering a search.
- Search should assume the Instructor has some idea what they want to find.
- Question Library search should work well with ordinary words by default.
- Instructors should not need to learn search syntax to use Search Question Library.
- Search results should switch to a dense, information-rich layout.
- Results should make it easy to scan many Questions quickly.
- Results should show the information needed to judge relevance without opening each Question.
- Search terms and active filters should remain visible while reviewing results.
- Clearing or changing part of a search should be quick.
- Should opening a result and returning preserve the Instructor's search and position.
  - we should offer some hover preview and open items in a new browser tab by default
- Advanced Search considerations:
  - Simple and advanced searches could use the same search box, but we should seriously consider
    advanced search versus simple search interfaces forms.
  - The interface should be minimal, show options by priority, not overwhelming to new users;
  - Movie Lens as a tiered filter system https://movielens.org/explore/
  - IMDB advanced search page is wel designed, https://www.imdb.com/search/title/ but questions
    would not be displayed as movie posters
  - Google advanced image search is more user friendly design https://www.google.com/advanced_search
  - Pubmed is clean, but not obvious to use https://pubmed.ncbi.nlm.nih.gov/advanced/
  - Ebay is dated, but perhaps a useful comparison https://www.ebay.com/sch/ebayadvsearch
  - Should Question IDs have a preview image/movie poster style?

##### Search Question Library filters

- Search results should support filters for narrowing the Question Library.
- Classification browsing and filtering should begin with Discipline and follow the shared
  Discipline -> Subject -> Topic -> Subtopic hierarchy.
- Tags should provide additional filters outside the hierarchy.
- Selecting a Discipline should limit Subject choices to Subjects associated with that Discipline.
- After selecting a Subject, Instructors should have an explicit option to include Library Objects
  associated with that Subject across its other Disciplines.
- Filters should update the current search rather than start a separate workflow.

##### Search Question Library syntax

- Search should support Google-like syntax for more precise queries.
- Quoted text should search for an exact phrase.
- A minus sign should exclude matching terms.
- Search should support PubMed-like field syntax such as `discipline:biology` and
  `subject:genetics`.
- Classification field examples include `topic:"chromosomal inheritance"` and `tags:review`.
- A Subtopic field example is `subtopic:"x-linked recessive crosses"`.
- Search fields should use PLE concepts and vocabulary.
- Useful fields may include Discipline, Subject, Topic, Subtopic, Tags, Question Type, and author.
- The interface should make useful search syntax discoverable when needed.
- Search syntax should help expert users quickly narrow a very large Question Library.

##### Browse Question Library interface

- **Browse Question Library** helps Instructors explore Questions without knowing what to search for.
- Browse should help Instructors understand what the Question Library contains.
- Browse should begin with Discipline and make moving through Subject, Topic, and Subtopic easy.
- Selecting a Discipline limits browsing to Subjects associated with that Discipline.
- Browse should also offer Tags, Question Types, and other useful groupings.
- Browse should show useful counts where they help Instructors choose where to explore.
- Browse results should use the same dense Question presentation used by Search where practical.
- Instructors should be able to move from browsing into a more focused search.
- Search and Browse are different paths into the same **Question Library**.

#### Assessment interface

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
- Instructors should be able to quickly add questions by Question ID to an assessment in bulk.
- Assessment Properties should group related settings so important settings are easy to find.
- Present timing settings in familiar units such as minutes, with explicit units and clear
  meanings for optional or unlimited values.
- Instructors can randomize Question order for an Assessment.
- Answer-choice randomization belongs to the Question, not the Assessment.
- **Assessments Due Soon** shows upcoming Assessments across the Courses an **Instructor** teaches.
- Assessments Due Soon shows the Course and due time for each Assessment.

#### Assessment type appearance

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

#### High-consequence actions

- Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
- Danger Zone should be visually separate from ordinary editing actions.
- Assessment Unrelease should explain that Student work will be deleted.
- Assessment Unrelease should require typing the Assessment title before confirmation.
- Archive actions should explain the effect on shared availability and require a clear confirmation.
- Restore actions should use ordinary availability controls.

### Student interface

#### General Student interface

- The Student interface should focus on current Courses, Coursework, and work that needs attention.
- **Coursework** is the Student-facing collective term for Regular Assignments, Practice Question
  Assignments, Bonus Assignments, Quizzes, and Exams.
- Student-facing interfaces should use the specific Assessment Type when referring to an individual item rather than calling it an Assessment.
- The Student Ribbon should use familiar Student language rather than internal PLE terms such as Assessment.
- The Student interface should make the next useful action easy to find.
- The Student menu is simpler than the Instructor menu.
- Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.
- Student layouts should adapt smoothly at intermediate widths, with readable long titles and
  controls that wrap or rearrange in the task's reading order.
- Every Student browser action should be usable with the keyboard alone.
- Student pages should use names meaningful to Students.
- Student navigation and pages should contain only Student interfaces and capabilities.
- Student content entry should use the response controls provided by Questions and other Student activities.
- Students should have no upload capabilities. Instructor-created content should use text boxes.
- The complete Student Ribbon task layout does not have a locked-in design yet.

#### Student Course and Coursework interface

- Students enrolled in one active Course should go directly into that Course.
- Students should be able to see their active Courses and Coursework from the main navigation.
- Course invitations should show the Course name and relevant Instructor and term information
  before the Student accepts the invitation.
- Course pages should make upcoming, available, completed, and missed Coursework easy to distinguish.
- Coursework lists should make due dates, Type, and completion status easy to scan.
- Keep Coursework entries compact in height so Students can scan several items at once.
- Keep essential Coursework information and the main action visible, with fuller access and timing
  details available through progressive disclosure.
- Coursework lists may provide filters for **Regular Assignments**, **Practice Question Assignments**,
  **Bonus Assignments**, **Quizzes**, and **Exams**.
- Each Coursework item should clearly show its Assessment Type using its label and Type icon.
- Before starting Coursework, Students should see its title, Type, Question count, points possible,
  time limit, and previous Attempts.
- Present the "Before you start" settings as a compact summary. Keep each label beside its value
  in aligned rows, using a compact grid when width permits.
- Group Question count and points together, and group availability, deadlines, and Attempt rules
  into clearly readable sections with concise spacing.
- Express unset or unlimited settings in Student language, such as "No closing time" or
  "Unlimited Attempts", and show the time zone once beside the timing group.
- Keep the start action close to this summary so Students can review the rules and begin with
  minimal scrolling.

#### Student Coursework interface

- Students see one Question at a time while completing Coursework.
- While completing Coursework, navigation should provide access to every Question and its saved
  status, with direct jumps between Questions.
- Leaving a Question and returning should preserve its saved response.
- The current Question and overall progress should remain easy to see.
- The timer should be subtle and keep the focus on the Questions.
- For timed Coursework, the remaining time should stay visible while moving between Questions.
- Submission status should be obvious and use plain language.
- Present Question navigation as a compact horizontal row of numbered controls, with distinct
  current-Question and saved-status cues.
- For long Question sets, use forum-style pagination with Previous and Next controls, the first
  and last Question numbers, a range around the current Question, and ellipses for omitted ranges.
- Adapt the visible number range to the available width while keeping every Question reachable.
- Keep the Question prompt and response controls near the top of the working area. Give the
  title, timing summary, and Question navigation only the space needed to orient Students.
- Make the current Question, saved-response status, and keyboard-focused control visually distinct
  so Students can recognize where they are, what work is saved, and which action they will activate.
- Label response actions by their effect, such as "Save response" and "Clear response", so Students
  can distinguish recording their work from changing it or submitting the whole Coursework.
- Group response feedback near the response controls and keep routine saved-status messages brief.

#### Student Coursework review interface

- Scores and feedback should appear where the Coursework settings allow them.
- Completed Coursework should remain easy to find and review.
- Group each reviewed Question's number, result, points, recorded response, and permitted feedback
  into a compact, clearly separated unit.

### Sysadmin interface

- Sysadmin interface work is LOW, LOW priority and can be done on an as-needed basis.
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

- Human-facing reference IDs should be short, opaque, and easy to communicate.
- Human-facing reference IDs should not reveal creation order, counts, database keys, ownership, or other object metadata.
- A public ID is the one universal, canonical human-facing identifier for a PLE object that needs one.
- Give an internal object a human-facing reference ID when a useful workflow needs it.
- Useful human-facing ID workflows include display, search, communication, and support.
- Store and use the exact same public ID in the database, Rust, JSON, URLs, object storage, hashes, logs, and browser UI.
- Preserve the canonical ID exactly across system boundaries.
- Parsing, serialization, API transport, persistence, and display do not reformat or translate the canonical ID.
- ID generation enforces global uniqueness across all public IDs and retries random collisions.
- Once issued, a public ID permanently identifies that object.
- Never reuse a public ID for another object, including after deletion or archival.

#### Public ID alphabet and canonical form

- Public IDs use the Crockford Base32 alphabet `0123456789ABCDEFGHJKMNPQRSTVWXYZ`.
- Public IDs have one canonical uppercase ASCII form.
- In ID format notation, `X` denotes a cryptographically random Crockford Base32 character.
- In ID format notation, `Z` denotes the calculated checksum character.
- Both `X` and `Z` represent characters stored as part of the canonical ID.
- `Z` is not a literal character or separate metadata.
- Human-entered IDs may use lowercase Crockford characters.
- Human-entered IDs may use `O` or `o` for `0`.
- Human-entered IDs may use `I`, `i`, `L`, or `l` for `1`.
- Normalize human-entered IDs to canonical form, then validate the canonical syntax and checksum at the human-input boundary.
- Store, transmit, display, copy, and generate only the canonical form.

#### Public ID checksum

- Calculate the checksum from the ASCII bytes of every other uppercase canonical-ID character.
- Include type prefixes in the checksum input.
- Exclude only separators and the checksum position from the checksum input.
- `XXXX-ZXXX` has checksum input `XXXXXXX`.
- For `BPXXXXXXXZ`, `CIXXXXXXXZ`, `UXXXXXXXZ`, and `AXXXXXXXZ`, calculate the checksum from every preceding character.
- Use public unsalted SHA-256 for the checksum.
- Map the high five bits of SHA-256 digest byte 0 through the Crockford alphabet.
- Validate the public-ID syntax and embedded checksum before database lookup or resolution.
- The embedded checksum detects typos.
- The checksum adds no identity space.

#### Public ID formats by object

- Published Questions and Question Pools use the public `XXXX-ZXXX` format.
- Published Questions and Question Pools share the same public-ID namespace.
- An `XXXX-ZXXX` value identifies either a Published Question or a Question Pool, never both.
- Blueprint Course IDs use `BPXXXXXXXZ`.
- Course Instance IDs use `CIXXXXXXXZ`.
- Assessment IDs use `AXXXXXXXZ`.
- Account IDs use `UXXXXXXXZ`.
- Each prefixed public ID uses seven cryptographically random Crockford Base32 characters and a final embedded checksum.
- Each prefixed public-ID random namespace contains 32^7 = 34,359,738,368 values.
- Account `U` references are Sysadmin support references.
- Account `U` references are not automatically exposed to Students or Instructors.

#### Database keys and clocks

- An object with a public ID uses that public ID as its primary key and as the target of every
  foreign key to it.
- An object without a public ID uses a native UUID primary key, or a composite natural key when
  it is owned by a parent (for example a Revision keyed by its lineage ID and Revision Number).
- Internal UUIDs never substitute for or appear as public identities.
- Every table has one creation clock on every row: a full-precision `timestamptz` where the
  server enforces, orders, or audits (Student Work, sessions, events, Courses, Accounts), and a
  `date` for authored content (Published Questions, Question Pools, Blueprint Courses, their
  Revisions, Draft Questions, usage statistics).
- Table shape follows [DATABASE_STYLE.md](/docs/DATABASE_STYLE.md).

### Content classification

- PLE uses one shared global content classification vocabulary for **Courses** and **Library
  Objects**.
- Content classification uses **Discipline** -> **Subject** -> **Topic** -> **Subtopic**.
- **Discipline** is the broad academic field, such as Biology, Chemistry, or Mathematics.
- **Subject** identifies a global area associated with one or more Disciplines, such as Genetics,
  Biochemistry, or Ecology.
- **Topic** identifies a major area within a Subject, such as Enzyme Inhibition or Chromosomal Inheritance.
- **Subtopic** provides a narrower classification within a Topic, such as Enzyme Catalysis Mechanisms or X-Linked Recessive Crosses.
- Subjects have a global identity across PLE, and Subject names are unique across PLE.
- A Subject may be associated with one or more Disciplines.
- A Topic belongs to one Subject.
- A Subtopic belongs to one Topic.
- Every Course has exactly one **Discipline**.
- **Subject**, **Topic**, and **Subtopic** are optional for Courses.
- Every Library Object has exactly one **Discipline** and one **Subject**.
- **Topic** and **Subtopic** are optional for Library Objects.
- Courses retain the hierarchy because their classification supports Course organization, search,
  filtering, and discovery.
- Course and Library Object selections follow the hierarchy: the Subject is associated with the
  selected Discipline, the Topic belongs to that Subject, and the Subtopic belongs to that Topic.
- Courses and Library Objects select from the same shared global vocabulary.

#### Classification vocabulary management

- **Sysadmins** exclusively create and manage the Discipline vocabulary and its lifecycle.
- Discipline is a stable vocabulary expected to change infrequently.
- **Instructors** classify content by selecting from the Sysadmin-managed Disciplines.
- **Instructors** may create new Subjects within a selected Discipline.
- When an Instructor attempts to create a Subject whose globally unique name already exists, PLE
  offers the existing Subject.
- PLE requires explicit Instructor acceptance before associating the existing Subject with the
  selected Discipline.
- **Instructors** may create new Topics within a Subject.
- **Instructors** may create new Subtopics within a Topic.
- Creating or selecting vocabulary should fit naturally into the classification workflow.

#### Classification selection and discovery

- Classification selection, browsing, and filtering begin with Discipline.
- Course and Library Object classification follow Discipline -> Subject -> Topic -> Subtopic,
  progressively narrowing the available choices at each level.
- Selecting a Discipline limits Subject choices to Subjects associated with that Discipline.
- After selecting a Subject, search interfaces may offer an explicit option to include content
  associated with that Subject across its other Disciplines.
- Classification supports searching, filtering, sorting, organization, and discovery wherever those capabilities are useful.

#### Tags and classification names

- **Tags** provide flexible labels outside the Discipline, Subject, Topic, and Subtopic hierarchy.
- Courses and Library Objects may have any number of Tags, including none.
- Subject, Topic, and Subtopic names must satisfy length limits and formatting requirements.
- Length allowances increase from Subject to Topic to Subtopic, supporting more specific names as classification becomes narrower.
- Strip leading and trailing whitespace from Subject, Topic, and Subtopic names and validate the resulting names consistently.

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
- Student retention removes identifiable Student evidence and leaves Question usage statistics
  unchanged. Question statistics are stored per Revision and displayed as a Question-level
  rollup by default; see "Question Library object usage statistics".

### Course retention and lifecycle

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

### Course retention processing

- A background process should periodically find Course Instances whose retention deadlines have passed.
- Retention decisions should come from stored Course dates and the Course Instance creation time.
- The background process should execute retention policy rather than define when retention periods begin or end.
- Running the retention process late should produce the same retention decision as running it on schedule.
- The retention process should be safe to run repeatedly.

### Common revision and history specifications

- Be conservative about creating revisions.
- Assessments, Course Instances, Draft Questions, and Question Pools use current state.
- Published Questions and Blueprint Courses have immutable revisions.
- Mutable working state uses a monotonic sequential Edit Number when needed for concurrency.
- An Edit Number is only a counter and does not identify a stored historical object.
- Question and Blueprint Revision Numbers start at 1 and increase sequentially for each object.
- A Revision Number identifies a specific immutable Revision stored by PLE.
- A new Revision keeps the same Published Question ID.
- Forking a Published Question or Question Pool creates a new public ID.
- A fork starts at Revision 1 under its new ID.
- Student Work records the exact Assessment Attempt and Published Question Revision delivered to the Student.
- Student Work records the Student's responses and the grading outcome returned by the Question Backend.
- For a Question served from a Question Pool, Student Work pins all four: the Published Question
  ID, its Revision Number, the Question Pool ID, and the Pool's Edit Number at selection time.
- Changes to Question point values recalculate scores from the stored grading outcome without changing the outcome.
- Changes to Assessment settings do not change the recorded history of completed Assessment Attempts.
- Immutable Question source and Question assets use SHA-256 checksums where needed to verify their stored contents.
- A public-ID checksum is one embedded character derived from other ID characters.
- A stored-content checksum is a full SHA-256 value verifying exact bytes.
- Public-ID checksums and stored-content checksums are not interchangeable.

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

## Question specifications

- Questions are subject agnostic. Properly classified Published Questions from all subjects belong in
  the same Question Library.
- Questions are strictly and deterministically automated; grading does not require an **Instructor**.
- Questions have one canonical title. Compact interfaces may truncate that title.
- Every Question stored by PLE has its own internal Question record.
- Answer-choice randomization belongs to the Question.
- PLE-native Questions control their own answer-choice randomization.

### Draft Question specifications

- Draft Questions are private working content.
- Draft Questions are not part of the Question Library.
- Draft Questions use current state rather than immutable Revisions.
- Saving a Draft Question replaces its previous working state.
- **Instructors** may delete Draft Questions they no longer need.
- PLE may clean up abandoned Draft Questions after an appropriate warning and recovery period.
- A Draft Question must pass Question Publication Validation before becoming a Published Question.
- Question Publication Validation requires Discipline, Subject, and all other required Question
  Library metadata before publication.

### Question formats and type specifications

- PLE flat-question JSON is the canonical machine format for simple static Questions.
- QTI is for import, export, and archival interchange rather than the internal source model.
- MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT Question Types should be supported.
- Question Type is immutable author-declared educational metadata on a Published Question Revision.
- PLE uses Question Type for search, filtering, labeling, and presentation.
- Question Type comes from the author rather than inference from backend controls.
- Question importers are transient translators from external formats into PLE-managed Question representations.

### Native PLE JSON Question specifications

- The native PLE JSON Question format is private, unversioned, and unpublished.
- Stored native JSON Questions may be upgraded together when the internal format changes.
- The native PLE JSON Question format is a strictly validated internal source shape without an external API.
- Native JSON Questions are static, not algorithmic nor random, and receive no random seed.
- Native PLE JSON supports MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT.
- External URLs used by native JSON Questions are explicitly recorded and reviewable.
- Recorded external URLs include links, images, scripts, stylesheets, and other resources.

#### Native Question response presentation

- Native MATCH Questions should present prompts with a shared choice bank on laptop and desktop
  screens. Display the full set of choices once alongside the prompts.
- MATCH Questions should support drag-and-drop and an equally capable keyboard-only method for
  assigning, changing, and clearing matches.
- Question response layouts may adapt to available screen space while preserving the same content,
  response meaning, and grading behavior. Narrow layouts may repeat choices when that improves use.
- MATCH Questions should make each prompt's assigned choice easy to recognize and keep the choice
  bank reachable while Students assign, change, and clear matches using keyboard, pointer, or touch.

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

### Question Backend specifications

#### Supported Question Backends

- WeBWorK is a PLE-managed Question Backend.
- The initial primary Question Backends are PLE-native JSON and WeBWorK.
- iMathAS and H5P are desired secondary Question Backends governed by Deferred product behavior.
- PLE-native Questions use the PLE Question Backend.
- WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
- H5P owns its runtime, interactions, state, and scoring.
- iMathAS owns its rendering and evaluation.

#### Question Backend responsibilities

- PLE owns and stores the Question representation used for each Question Backend.
- Imported backend source may be transformed into the form PLE stores and manages.
- PLE preserves the information needed to reproduce the Question through its backend.
- PLE-managed Question representations participate in Question revision history.
- Question Backends own rendering, interaction, response, grading, feedback, and backend-specific state.
- PLE owns authorization, Question ID, revisions, persistence, lifecycle, and stored outcomes.
- PLE uses the same basic interface for every Question Backend, each backend handles its own internal details.
- Each Question Backend adapter retains its backend-specific interaction knowledge.
- Question Backends may support more complex interactions without requiring PLE to implement those interactions.

#### Question Backend grading and feedback

- Question Backend feedback is transient unless the backend provides a robust way for PLE to preserve it.
- PLE does not extract or reconstruct transient feedback from Question Backend source or output.
- PLE-managed Hints, Question Feedback, and Worked Solutions remain separate from backend-generated
  interaction feedback.
- A Question Backend returns an immutable credit fraction for each complete response it evaluates.
- PLE stores the immutable credit fraction as the grading outcome.
- When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
- Assessment scores are calculated from stored credit fractions and current Question point values.
- Changing Question point values recalculates scores without another Question Backend interaction.

#### WeBWorK source and algorithmic Questions

- Preserve the distinction between WeBWorK PG and PGML source. A Question should be identified as PGML only when its source is fully PGML-compliant; otherwise identify it as PG.
- BiologyProblems.org imports should preserve whether the canonical algorithmic source is PG or PGML rather than treating both formats generically as PG/PGML.
- When parameterized WeBWorK PG or PGML source exists, prefer it to importing static variants.
- Preserve backend-native algorithmic variation rather than expanding one algorithmic Question into static variants.
- One algorithmic Question remains one Published Question regardless of how many variants its Question Backend can generate.
- Use a Question Pool with algorithmic Questions only when the **Instructor** wants selection among distinct Questions, not to represent variants of one algorithmic Question.
- BiologyProblems.org WeBWorK problems should be imported from their canonical algorithmic PG or PGML source rather than from generated static variants.
- Multiple static BiologyProblems.org questions generated from one algorithmic source represent one Published Question, not separate Published Questions or a Question Pool.

### Published Question specifications

- A Published Question is an immutable-revision Question available for reuse through the Question Library.
- Published Questions are available to all vetted **Instructors**.

#### Published Question identity specifications

- Published Questions receive a public `XXXX-ZXXX` Crockford Base32 ID.
- Question IDs have the canonical form `XXXX-ZXXX`.
- The hyphen is part of the canonical ID and makes Question IDs immediately recognizable.
- Human-entered Question IDs may omit the hyphen.
- Normalize accepted human input to the canonical hyphenated form before validation and lookup.
- PLE always stores, transmits, displays, and copies the canonical hyphenated form.
- Seven Crockford Base32 characters are cryptographically random and provide the identity.
- IDs never encode creation order, Question Type, ownership, subject, or other metadata.

#### Published Question metadata

- Published Questions have metadata specific to the individual Question.
- Published Question metadata includes Title and Description.
- Published Question metadata may include authorship, attribution, license, and source information.
- Published Questions may include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
- Published Questions also use the shared Question Library metadata required for publication.

#### Published Question revisions, edits, and forks

- **Published Questions** maintain immutable revision history.
- Assessments and Student Work remain pinned to exact immutable Published Question Revisions.
- Publishing a new Question Revision does not silently change existing Assessments or Student Work.
- The Question owner may publish corrections, wording changes, accessibility improvements, answer changes, grading changes, and other updates as a new Revision.
- Changing Question source, answer content, grading rules, Hints, Question Feedback, Worked Solutions,
  or Question assets creates a new Question Revision.
- Changes to the Question title, description, Discipline, Subject, Topic, Subtopic, Tags, or other
  search metadata update the Published Question metadata while preserving the current Question Revision.
- Search metadata belongs to the Published Question as a whole rather than to one Revision.
- Any **Instructor** may fork a Published Question to create a separate Question with a new Question ID.
- A fork starts as a private **Draft Question** with its own authorship and lineage.
- A fork must pass Question Publication Validation before joining the Question Library.
- Published forks retain source attribution.
- Forced corrections are audited **Sysadmin** actions reserved for critical flaws.
- Question authorship, contributor credit, history, attribution, and compatible CC licensing are preserved across Revisions and forks.
- Watching a Published Question drives in-app notifications for new Revisions, forks, improvement
  threads, and impact notices.

#### Published Question behavior specifications

- Published Questions may include optional PLE-managed **Hints**, **Question Feedback**, and **Worked Solutions**.
- PLE-managed Hints, Question Feedback, and Worked Solutions are separate from Question Backend-generated content.
- WeBWorK Questions may use PLE-managed Hints, Question Feedback, and Worked Solutions even when similar material also exists in the WeBWorK source.
- Question Feedback is shown when its disclosure rules allow it.
- Hints and Worked Solutions use their own disclosure settings.
- Student workflows remain complete when a Question has none of this optional support content.

### Question Pool specifications

- A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
- Pool contents should represent reasonably interchangeable assessments of the intended learning.
- Question Pools may contain Questions from any Question Backend.
- Question Pools are created from a Published Question and enter the Question Library immediately.
- A Question Pool is an independently reusable Question Library object.
- Question Pools are available to all vetted **Instructors**.
- A Question Pool has its own public `XXXX-ZXXX` Crockford Base32 ID.
- A Question Pool is a current ordered list of exact Published Question Revisions plus its
  metadata. Saving the list re-attests interchangeability and advances the Pool's Edit Number;
  no Revision is created. Removing ten Questions and saving once is one Edit.
- Importing a Question Pool into a new Assessment automatically forks the Question Pool.
- The fork belongs to the new Assessment and can be changed without changing the source Question Pool.
- Forking a Question Pool preserves its list of Published Questions by their public `XXXX-ZXXX` IDs.
- Question Pools work the same way regardless of the Question Backend.
- **Instructors** choose the contents of a Question Pool and how many Questions are selected.
- PLE selects from the Question Pool; the selected Question Backend controls the Question interaction.
- Question Pool selection and backend-native randomization are separate forms of variation.
- Returning to an Attempt preserves the Question Pool selections already made.
- Starting a new Attempt makes fresh selections from its Question Pools.
- Student Work pins the Published Question ID, its Revision Number, the Question Pool ID, and the
  Pool's Edit Number for every Question served from a Pool; the pinned Published Question
  Revision is all that later interpretation and grading need.
- Grading and historical evidence follow the exact Published Question Revision delivered to the Student.
- Each member of a Question Pool is a **Published Question**.
- Question Pools contain only **Published Questions**; Question Pools cannot be members of Question Pools.
- Watching a Question Pool drives in-app notifications for new Revisions, forks, improvement
  threads, and impact notices.

#### Question Pool metadata

- Question Pools have metadata specific to the individual Question Pool.
- Question Pool metadata includes Title and Description.
- The first Published Question establishes the Question Pool's Discipline and Subject.
- Every additional Published Question added to the Pool has the same Discipline and Subject as the Pool.
- Published Questions retain their own Topic, Subtopic, Tags, and other Library Object metadata
  when included in a Question Pool.
- Question Pools may have their own authorship, attribution, license, and source information where
  appropriate.
- Question Pool metadata describes the Pool rather than duplicating metadata from its member
  Published Questions.
- Question Pools may include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
- Question Pools also use the shared Question Library metadata required for publication.

### Question Library specifications

- Question sharing, discovery, and reuse are a high-priority **Instructor** workflow.
- The Question Library is one global collection of Published Questions and Question Pools.
- Draft Questions are not part of the Question Library.
- **Published Questions** and Question Pools are available to all vetted **Instructors**.
- **Students** access Question content through their Coursework rather than through the Question Library.
- Question Library content remains discoverable when used by a private **Course Instance**.
- With 13,000 Questions in Neil's first course, manually archiving Questions is unlikely to be a useful primary workflow.
- Question Library workflows should support bulk operations because an **Instructor** may manage thousands of Questions.
- **Instructors** should be able to select many Library objects and update shared metadata such as
  Discipline, Subject, Topic, Subtopic, Tags, or other search fields together.
- Question Library search, filters, sorting, and bulk editing should make large imports practical to clean up.

#### Question Library metadata

- **Library Objects** use shared metadata for organization, search, filtering, and discovery.
- Required Question Library metadata must be complete before a Library Object enters the Question Library.
- Library metadata should describe the Library Object rather than its location in a Course, Assessment, or textbook.
- Library Objects use the shared **Discipline**, **Subject**, **Topic**, **Subtopic**, and **Tag**
  vocabulary.
- Every Library Object has exactly one **Discipline** and one **Subject**.
- **Topic** and **Subtopic** are optional for Library Objects.
- Library Objects may have any number of **Tags**, including none.
- Question Publication Validation requires Discipline and Subject before publication.
- Library Object classification follows Discipline -> Subject -> Topic -> Subtopic.
- Questions and Question Pools retain their Library Object classification when used in an Assessment.
- Library classification supports searching, filtering, sorting, and bulk editing.
- Published Questions and Question Pools may have PLE-managed **Hints**, **Question Feedback**, and **Worked Solutions**.
- Support content may be attached at the level where it applies rather than duplicated across individual Questions.

#### Question Library object usage statistics

- Published Questions and Question Pools keep privacy-safe aggregate usage statistics.
- Statistics are kept separately for each Published Question Revision and for each Question
  Pool.
- Aggregate statistics contain counts and sums, never Student Attempts or identifiable Student
  records.
- Every Published Question Revision keeps one global usage statistic so Instructors can judge
  how often a Question is used and how hard it is.
- The statistic is six counters and two sums: `issued_count`, `blank_count`, `answered_count`,
  `correct_count` (full credit), `partial_count`, `incorrect_count` (zero credit), `credit_sum`,
  and `credit_sum_sq` (the sum of squared credit fractions). Mean and standard deviation of
  credit derive from the sums; sorting Questions in bulk uses them.
- Every counter increments when the Assessment Attempt is submitted, so an Attempt that is
  Unreleased or deleted before submission contributes nothing. `issued_count = blank_count +
  answered_count` and `answered_count = correct_count + partial_count + incorrect_count`.
- A blank Question is one submitted with no saved response. It counts as blank, separately from
  incorrect, and is never sent to a Question Backend.
- Every Attempt counts as one observation, including practice Attempts after full credit.
- The statistic records the exact Revision, the outcome class, the credit fraction, and the
  calendar date of the most recent increment (`updated_on`), and nothing else: no Course,
  Student, Account, Attempt, roster, time of day, point value, timing, ordering, seed, selected
  choice, or response text. Day granularity on a global counter identifies no one.
- The per-observation receipt that makes each increment exactly-once is Student Work and is
  purged with the Attempt. The aggregate is global content and survives Unrelease and Student
  data deletion unchanged; PLE never rebuilds it from Student Work.
- Each Question Pool keeps two stored counters that membership changes do not invalidate: an
  `issued_count` for the Pool, and a `selected_count` per member Published Question (how often
  that member was drawn from this Pool). A member's `selected_count` row is removed with the
  member and starts at zero if the member is re-added.
- A member Question's outcomes count in that Question Revision's own statistic, never in a
  Pool-level copy.
- A Question Pool's difficulty (mean and standard deviation of credit) is derived when read from
  its current members' Question statistics, so removing or adding a member changes it with no
  stored value to update. Search and sorting compute it from the member join; a cached value on
  the Pool row is a measured exception, never the default.
- Storage is per Revision; display is per Question. Every bulk view (Library lists, search
  results, Assessment editors) shows the rollup across all Revisions of a Question; the Question
  detail page is the one place that adds the per-Revision breakdown.
- The Question Library shows every rate beside its observation count.
- Students see Course-scoped class statistics through the Assessment feedback policy; those are
  protected Student Work projections and are purged with the Course.
- [FERPA_DATA_POLICY.md](/docs/FERPA_DATA_POLICY.md) "Question Library object usage statistics" and
  [DATABASE_STYLE.md](/docs/DATABASE_STYLE.md) "Every table has a clock" carry the same rule.
- Removing Student names alone does not make statistics anonymous.
- Shared statistics should be shown only when individual Students cannot reasonably be identified
  from the aggregate.
- Course-specific analysis remains FERPA-sensitive when individual Students could be inferred.

#### Question Library Bloom classification metadata

- Published Question Revisions and Question Pools can have a Bloom Cognitive Process and Bloom
  Knowledge Dimension.
- The two Bloom dimensions are independent and together determine the object's Bloom Classification.
- Bloom Classification supports Question Library search and Assessment item sorting.
- A Question Pool's Bloom Classification describes the intended cognitive work of the Pool as a whole.
- Bloom Classification is left blank when a Published Question or Question Pool enters the Question
  Library, to be updated by AI later.
- AI assigns the initial Bloom Classification using a daemon after publication.
- An **Instructor** can correct either Bloom dimension without creating a new Published Question
  Revision.
- Question Library search and reporting should make both Bloom dimensions useful to **Instructors**.
- Follow `docs/BLOOM_TAXONOMY_GUIDE.md` for Bloom classification and teaching interpretation.

#### Question Library stewardship specifications

- Question Library stewardship should use a GitHub-like model.
- Published Questions and Question Pools can be starred and watched.
- Star means favorite and visible endorsement.
- Vetted **Instructors** can see the star count and which vetted **Instructors** starred a Published
  Question or Question Pool.
- Watch means subscription.
- Watching a Published Question or Question Pool drives in-app notifications for new Revisions,
  forks, improvement threads, and impact notices.
- An **Instructor's** watch list remains private.
- **Students** and anonymous users do not receive **Instructor** identity lists or watch information.

## Course specifications

- **Courses** organize reusable teaching content and its delivery to **Students**.
- PLE has two Course forms: **Blueprint Courses** and **Course Instances**.
- **Blueprint Courses** provide reusable course designs for **Course Instances**.
- Course Instances may start independently with no parent Blueprint Course, or an **Instructor** may create them from a Blueprint Course.
- A Course can have multiple co-**Instructors** with equal teaching authority.
- **Sysadmins** can create Courses, but **Instructors** teach them.
- Every Course Instance must have at least one assigned **Instructor**.
- Creating a Course Instance establishes its first Instructor membership but does not give that Instructor greater Course authority than later co-Instructors.
- **Adoption** connects a Blueprint Course and a Course Instance when an **Instructor** creates a new Course Instance from a Blueprint Course or creates a new Blueprint Course from an existing Course Instance's reusable structure.
- An Instructor may create a new Blueprint Course from an existing Course Instance's reusable structure. The new Blueprint Course records that Course Instance as its source, and the Course Instance remains the same teaching instance.
- A Course Instance created from a Blueprint Course is a daughter Course Instance of that Blueprint Course.

### Course classification specifications

- **Blueprint Courses** and **Course Instances** use the shared content classification system.
- Course classification describes the Course as a whole.
- Every Blueprint Course and Course Instance has exactly one **Discipline**.
- Courses may optionally have one **Subject**, one **Topic**, and one **Subtopic**.
- Courses may have any number of **Tags**, including none.
- Course classification follows the shared Discipline -> Subject -> Topic -> Subtopic hierarchy.
- Course Discipline selection should provide a clear way to request a new Discipline when the needed
  Discipline is unavailable.
- **Sysadmins** exclusively create and manage Disciplines.
- Course classification supports Course search, filtering, organization, and discovery where applicable.
- A Course Instance may have classification that differs from its Blueprint Course.

### Blueprint Course specifications

- **Blueprint Courses** are reusable course definitions for building **Course Instances**.
- Blueprint Courses are a similar concept as LibreTexts' ADAPT alpha courses.
- Blueprint Courses have no **Students**, deadlines, or other teaching-specific delivery settings.
- Blueprint Courses do not contain dates or relative schedules.
- Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
- Blueprint Courses contain only **Published Questions** and published **Question Pools**.
- An **Instructor** may create a new Blueprint Course from an existing Course Instance's reusable structure. The new Blueprint Course records that Course Instance as its source.
- Creating a Blueprint Course from a Course Instance copies the ordered Course Instance Assessment list as ordered Blueprint Assessments, preserving order.

#### Blueprint Course lifecycle specifications

- Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
- New Blueprint Courses and forks start Private.
- Private Blueprint Courses are visible only to their owning **Instructor**.
- Instructors may develop and use Private Blueprint Courses without publishing them.
- Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
- Making a Blueprint Course Public adds it to the shared Blueprint Course collection.
- Public Blueprint Courses and their Revision history are visible to all vetted **Instructors**.
- Public Blueprint Courses can be adopted to create daughter Course Instances.
- A Public Blueprint Course with no adoptions may return to Private.
- A Public Blueprint Course with one or more adoptions remains Public.
- Blueprint Courses have no separate Draft state.

#### Archived Blueprint Course specifications

- Archived Blueprint Courses are read-only and no longer actively maintained.
- Archived Blueprint Courses and their Revision history remain visible to all vetted **Instructors**.
- Archived Blueprint Courses do not appear in normal discovery unless explicitly included.
- Archived Blueprint Courses cannot be adopted to create new daughter Course Instances.
- Archived Blueprint Courses can be forked.
- Forking an Archived Blueprint Course creates a new Private Blueprint Course.
- The owning **Instructor** can return an Archived Blueprint Course to Public.
- Blueprint Course visibility includes its content, Revision history, and recorded changes.
- Visibility does not grant editing authority.

#### Blueprint Course revision specifications

- Blueprint Courses use immutable **Blueprint Revisions** for saved reusable content.
- Blueprint Course content editing uses explicit Save.
- Saving changed Blueprint content creates the next Blueprint Revision.
- Multiple content edits before Save become one Blueprint Revision.
- Saving unchanged Blueprint content does not create another Revision.
- Blueprint Course metadata can change without creating a Blueprint Revision.
- Blueprint Course names are metadata and identify the Blueprint across Revisions.
- Changing a Blueprint Course name does not create a new Blueprint Revision.

#### Blueprint Course stewardship specifications

- Blueprint Courses have a searchable boolean Promoted flag.
- Sysadmins exclusively control the Promoted flag.
- **Instructors** can Star or Watch Public and Archived Blueprint Courses.
- A Star is a visible endorsement and helps **Instructors** save useful Blueprint Courses.
- Vetted **Instructors** can see who Starred a Blueprint Course and its Star count.
- Watching a Blueprint Course is private.
- Watchers are notified about new Blueprint Revisions and other important Blueprint changes.
- Forking or adopting a Blueprint Course does not automatically Star or Watch it.
- Stars and Watches belong to the Blueprint Course across all of its Revisions.

#### Blueprint adoption and incorporation specifications

- Blueprint adoption copies every Assessment from the Blueprint Course into the Course Instance.
- Course Instances pin the exact Blueprint Revision from which they were adopted.
- New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review.
- Routine Blueprint changes should be quick for an **Instructor** to review and incorporate.
- It should be obvious when a Course Instance is based on an older Blueprint Revision.
- The **Instructor** decides which changes to existing Assessments to incorporate.
- Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
- Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.

#### Blueprint Course fork specifications

- An **Instructor** can fork a Public or Archived **Blueprint Course** to create a new Private Blueprint Course.
- A fork is owned by the **Instructor** who created it.
- A fork records the source Blueprint Course and Blueprint Revision from which it was created.
- Forking a Blueprint Course creates new Blueprint Assessments.
- Published Questions in the new Blueprint Assessments retain the same Published Question IDs and exact Revisions.
- Question Pools in the new Blueprint Assessments are forked and receive new Question Pool IDs.
- Forked Question Pools initially contain the same Published Question IDs and exact Revisions as their source.
- Forked Blueprint Courses develop independently and have their own Blueprint Revisions.
- Changes to a source Blueprint Course are never automatically applied to its forks.
- A Blueprint Course shows its known forks and the **Instructor** who owns each fork.
- PLE should make newer source Revisions easy for the fork owner to discover and review.
- PLE should make newer Revisions in downstream forks visible from their source Blueprint Course.
- The fork owner decides whether to incorporate source changes into the fork.
- PLE should make it easy for the fork owner to incorporate selected source changes.

#### Blueprint Course Change Proposal specifications

- A **Blueprint Course Change Proposal** proposes changes from one Blueprint Course to another.
- An **Instructor** can create a Change Proposal for a Blueprint Course they do not own.
- A Change Proposal records the source Blueprint Course and exact Blueprint Revision.
- A Change Proposal records the target Blueprint Course and exact Blueprint Revision used for comparison.
- The proposed changes are represented using the canonical Blueprint Course JSON format.
- PLE compares the proposed JSON with the target Blueprint Revision to determine the proposed changes.
- A Change Proposal should present those changes in a human-readable interface rather than requiring
  the receiving **Instructor** to review raw JSON.
- A Change Proposal may include any Blueprint Course content represented in its canonical JSON.
- Changes may include Course names and metadata, Assessment names and settings, Assessment additions
  and removals, and Question membership changes.
- Question content changes belong to the Published Question and are not Blueprint Course changes.
- PLE should present proposed changes in terms meaningful to Instructors rather than as raw JSON changes.
- The receiving **Instructor** can review proposed changes before changing the target Blueprint Course.
- The receiving Instructor decides which proposed changes to accept.
- The receiving Instructor may accept the entire Change Proposal or selected proposed changes.
- Accepted changes are applied to the current target Blueprint Course and create a new Blueprint Revision.
- The Change Proposal remains a record of what was proposed and what was accepted.
- If the target Blueprint Course changes after the proposal was created, PLE should show that the
  proposal was based on an older target Revision.
- PLE should not silently apply a proposal against a newer target Revision when the changes no longer
  apply cleanly.
- Change Proposals never directly change daughter Course Instances.
- Daughter Course Instances receive accepted changes through the normal Blueprint incorporation workflow.

#### Blueprint Course comparison specifications

- Any **Instructor** can compare related Blueprint Courses in the same fork lineage when both are visible to that Instructor.
- Fork comparison normally compares the newest Revision of the source Blueprint Course with the newest Revision of the fork.
- Older Revisions remain available through Blueprint history but are not the normal comparison workflow.
- Blueprint Course differences are calculated from canonical JSON when the Instructor requests the comparison.
- Shared Published Question IDs provide durable relationships between Published Questions across Blueprint Course forks.
- Blueprint Course comparison does not require Blueprint Assessment identity or history across forks.
- Comparison should show shared, added, removed, and changed Assessments, Published Questions, and Question Pools.
- Comparison should remain useful when Assessment names, order, or structure have changed.
- Comparison visibility follows Blueprint Course visibility rather than fork ownership.

#### Blueprint Course JSON specifications

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

### Course Instance specifications

#### Course Instance creation specifications

- An **Instructor** can create a Course Instance from a Public Blueprint Course.
- **Instructors** can also create a new empty Course Instance without a parent Blueprint Course.
- Course Instances have **Students**, deadlines, releases, and other delivery-specific settings.
- Course Instances contain only **Published Questions** and published **Question Pools**.
- Course Instances are visible only to their co-**Instructors** and enrolled **Students**.
- Active Courses are current teaching Course Instances.
- Inactive Courses are past Course Instances and retain Course metadata, including after
  FERPA-sensitive Student data is removed.
- An **Instructor** may create a new **Blueprint Course** from an existing Course Instance's reusable structure. The new Blueprint Course records that Course Instance as its source.
- A new academic term uses a new Course Instance. Rollover is not a separate product model.

#### Blueprint adoption and daughter Course Instances

- **Adoption** connects a Blueprint Course and a Course Instance through either Course creation workflow.
- Creating a new Course Instance from a Blueprint Course establishes an Adoption and increases that Blueprint Course's **Adoption count** by one.
- Creating a new Blueprint Course from an existing Course Instance's reusable structure establishes the originating Course Instance as that Blueprint Course's first Adoption, giving the new Blueprint Course an Adoption count of one.
- A Course Instance created from a Blueprint Course is a **daughter Course Instance** of that Blueprint Course.
- A daughter Course Instance records its parent Blueprint Course and the exact Blueprint Revision used to create it.
- A daughter Course Instance receives every Assessment from the selected Blueprint Revision.
- Creating a daughter Course Instance copies the Blueprint Course's Assessments, Questions, Question Pools, and reusable settings.
- Course Instance Assessments created from Blueprint Assessments start unreleased with dates unset.
- New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review.
- Routine Blueprint changes should be quick for an **Instructor** to review and incorporate.
- It should be obvious when a daughter Course Instance is based on an older Blueprint Revision.
- The **Instructor** decides which changes to existing Assessments to incorporate.
- Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
- Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.

### Course short and long name specifications

- Blueprint Courses and Course Instances each have their own short name and long name.
- Short names are entered or chosen deliberately by **Instructors**.
- Short names are for compact navigation and should stay under about 16 characters when practical.
- Long names are descriptive names used for headings, breadcrumbs, and Course listings.
- A Blueprint Course might be `Biochemistry` / `Upper-Level Introductory Biochemistry`.
- A Course Instance might be `BCHM 355/455` / `BCHM 355/455 Section 20 Biochemistry (Roosevelt U; Spring 2026)`.
- Course Instance names are properties of the Course Instance and are not derived from Blueprint Course names.

## Assessment specifications

- **Assessment** is the PLE object for organizing Questions into a graded or practice activity.
- PLE has **Blueprint Assessments** and **Course Instance Assessments**.
- Blueprint Assessments define reusable Assessment content and teaching settings.
- Course Instance Assessments deliver Questions to **Students**.
- All Assessments use the same underlying Assessment model.
- **Assignment** is not a separate object or category. The word appears only in the names
  **Regular Assignment**, **Practice Question Assignment**, and **Bonus Assignment**.

### Assessment content specifications

- Assessments are organized by their Course and position within its ordered sequence.
- Assessments contain an ordered sequence of Published Questions and Question Pools.
- Published Questions stay references to the same Question ID and exact Revision.
- Question Pools are copied by forking when added to another Assessment.
- A newly forked Question Pool initially contains the same Published Question IDs and exact Revisions
  as its source.
- A forked Question Pool can be changed independently without changing its source Question Pool.
- Published Questions and Question Pools remain distinct even though both can occupy positions in an Assessment.
- **Instructors** can add, remove, and reorder Published Questions and Question Pools.
- Assessment Question-order randomization is called **Randomize question order**.

### Assessment type specifications

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

### Blueprint Assessment specifications

- A **Blueprint Assessment** is an Assessment in a **Blueprint Course**.
- Blueprint Assessments define reusable Assessment content and teaching settings.
- Blueprint Assessments have an Assessment Type.
- Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
- Blueprint Assessments define Question point values and points possible.
- Blueprint Assessments have no **Students**, Student Work, due dates, release dates, or other Course Instance delivery settings.
- Blueprint Assessments do not use Assessment Templates.
- Creating a daughter Course Instance from a Blueprint Course copies its Blueprint Assessments into the Course Instance.

### Course Instance Assessment specifications

- A **Course Instance Assessment** is an Assessment in a **Course Instance**.
- Course Instance Assessments are the Assessments delivered to **Students**.
- Course Instance Assessments have an Assessment Type, Questions, Question Pools, point values, and points possible.
- Course Instance Assessments also have delivery settings such as due dates, release status, and Student availability.
- Course Instance Assessments copied from a Blueprint Assessment can be changed for the needs of that Course Instance.
- Newly added Blueprint Assessments are automatically copied to daughter Course Instances as unreleased Course Instance Assessments.

### Assessment Template specifications

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

#### Assessment release validation

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

#### Assessment submission defaults

- New Course Instance Assessments default to accepting submissions only through the due date.
- New Course Instance Assessments default to starting new Attempts only through the due date.
- Late work defaults to rejected.

#### Assessment answer and feedback disclosure

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

#### Assessment unrelease

- Unreleasing is the destructive reversal of releasing a Course Instance Assessment.
- Unreleasing permanently deletes all Student Work for that Assessment.
- Student Work deletion includes Assessment Attempts, saved responses, submissions, and grading outcomes.
- Unreleasing removes the Assessment from Student availability and returns it to a pre-release state.
- The Assessment itself, its Questions, settings, and other Instructor-created content remain.
- The Instructor can edit the unreleased Assessment normally after Student Work is deleted.
- Releasing the Assessment again follows the normal Assessment Release Validation process.
- A later release starts with no Student Work or Assessment Attempts from the earlier release.

### Assessment Attempt specifications

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

### Assessment response and submission specifications

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

### Assessment Attempt timing and expiration specifications

- Each Assessment Attempt has a time limit.
- Each Assessment may contain at most 250 Questions.
- A Question Pool counts as the number of Questions selected from it for the Assessment Question
  limit and default time calculation; selecting 3 of 199 Questions counts as 3.
- The default time limit is 1.5 minutes per Question, rounded up to the nearest whole minute.
- Instructors can override the default time limit up to 12 hours.
- The interface should show the calculated default time limit and provide a specific Instructor override.
- Time limits must support individual **Students** with accommodations, such as 1.5X or 2X time.
- Student accommodations are applied after the Assessment time limit and may extend that Student's effective time limit
  up to 24 hours.
- Attempt time limits help **Students** develop an accurate sense of expected working speed.
- Assessment Attempts use wall-clock time.
- The server owns the Attempt start and expiration times.
- Attempt time continues while the **Student** is disconnected or the browser is closed.
- A **Student** may reconnect, reload, or use another browser session to resume the same active Attempt.
- Resuming an Attempt does not reset or extend its expiration time.
- Attempt expiration is checked whenever a **Student** interacts with the Attempt.
- Background processing ensures expired Attempts are submitted even when the **Student** is no longer connected.
- When an Attempt expires, PLE submits the whole Attempt and finalizes its saved responses.
- Unanswered Questions remain visibly unanswered, receive zero credit, and count as incorrect.
- Unanswered Questions are not sent to the Question Backend.

### Student Work specifications

- Student Work keeps the exact Published Question Revision delivered to the **Student**.
- For a Question Pool, Student Work keeps the Question Pool ID, the Pool's Edit Number at
  selection, and the exact Published Question Revision selected.
- Student Work keeps each saved response as finalized with the submitted Attempt and the grading outcome returned by the Question Backend.
- Changes to Assessment content do not replace Question evidence already delivered in existing Attempts.
- PLE should retain only the additional historical Student Work data needed to interpret or grade that work correctly.

### Assessment scoring specifications

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
