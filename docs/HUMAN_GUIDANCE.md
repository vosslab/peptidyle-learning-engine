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
- Describe intended PLE behavior with positive instructions and omit irrelevant alternatives.
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
- Much of the Sysadmin functionality beyond creating Instructor Accounts is deferred until concrete needs are established. I know more will be needed, but what is needed is still unproven.
- Bulk editing of Published Question metadata for Instructors is delayed.
- Automatic WeBWorK Question Type detection is deferred. Assign the Type manually for now.
- System-wide settings are deferred; the settings themselves have not been defined.
- System-wide settings should have their own area, separate from user and Course administration.
- Regrading submitted responses after a Native JSON answer-key correction is deferred for now.
- When implemented, regrading replaces the previous grading result rather than keeping old grading results. Changing the answer key back runs grading again.
- The decision about when optional Question Feedback is shown is deferred until I better understand how feedback is used in PLE.
- all automated daemon backends are deferred until final server location
- all automated AI/LLM backends are deferred until final server location
- AI-backed Bloom classification is desired but deferred until a later release.
- Initial Bloom Classification is deferred with the AI backend.
- Bloom Classification does not block publication or Question Library entry.
- Any AI backend is desired but deferred and low priority for the current PLE.
- No specific AI backend is selected.
- iMathAS is a desired Question Backend deferred until a later release.
- H5P is a desired Question Backend deferred until a later release.
- Future H5P use is limited to Weekly Assignments, Bonus Assignments, and Practice Question
  Assignments.
- Quizzes and Exams do not use H5P because its runtime exposes answers and correctness to the
  Student browser.

### Deferred content tools and public APIs

- public API for instructors to use AI to control their classes.
- public API perhaps modeled after BrickLink OAuth `https://www.bricklink.com/v3/api.page?page=auth`
- I am developing a rust version of qti-package-maker for importing content to Native JSON, see
  `~/nsh/PROBLEMS/qti-package-maker-rs/`; so do not implement here;
- decide what format we want to receive from the new qti-package-maker; since we are not publishing the Native
  JSON, should we take BBQ text format `bbq_text_upload`, or something more parsable;
- list of engines is here: `https://github.com/vosslab/qti-package-maker-rs/blob/main/docs/ENGINES.md`;
- we could build a QTI v2.1 style JSON or human editable YAML format, or something closer to our Native JSON
  format.

### Deferred content licenses

- Support for CC BY-NC and CC BY-NC-SA content is deferred.
- Support for Creative Commons NoDerivatives licenses is planned but deferred.
  When supported, ND Questions must be blocked from forking.
- When NC content is supported, a Pool containing public-domain, CC BY, CC BY-NC,
  and CC BY-NC-SA Questions has a CC BY-NC-SA license and cannot accept a CC BY-SA
  Question because it is incompatible.

## Development principles

### Agent working principles

- The stack takes a long time to rebuild. Use `./launchers/run_fast_checks.sh` for faster
  interface checks.
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
- Treat tests as liabilities as well as protection. Keep only requirements and gates grounded in actual needs.
- Plans should be finishable by the manager and subagents without additional human interaction.
- Prefer more small, independently verifiable milestones over a few large milestones.
- Fix the design that causes a problem rather than adding a workaround for its symptom.
- Prefer durable long-term fixes when the additional cost is justified.
- Prefer adaptable boundaries and simple domain concepts over speculative edge-case machinery.
- Add product states, workflows, background processing, and recovery mechanisms only for a
  demonstrated product or Question Backend need.
- Stay focused on the requested work. Complete the required work and avoid adding unplanned functionality.

### Codebase development rules

- Language-native casing is the right system: SQL stays account_id, Rust/TS types stay AccountId, JSON stays accountId.
- Use `source ./source_me.sh && ./launchers/run_fast_checks.sh` for a quicker compliance check
- Use `source ./source_me.sh && ./launchers/all_test.sh` for a complete compliance check
- Use `source ./source_me.sh && ./devel/capture_screenshots.sh` for UI work to capture fresh screenshots
- Every source file should stay below 1000 lines. Split complete capabilities into focused modules.
- PLE is pre-production with no users or durable production data. Fix the design directly;
  there is no legacy behavior to preserve.
- Use the pre-production state to improve foundational schemas, contracts, and abstractions
  whenever that produces a stronger long-term system.
- Use SQL directly to create the initial PostgreSQL database structure. Insertions have more flexibility.
- Before production, edit the main database design directly as the design changes.
- After production, update existing databases without rebuilding them from scratch.
- Use readable `snake_case` whenever possible; see [NAMING_CONVENTIONS.md](/docs/NAMING_CONVENTIONS.md) for details.
- Give variables for distinct concepts distinct names. For example, `qti_package_upload_file`,
  `qti_package_archive`, `qti_package_extracted_image`, `question_image_asset`,
  `question_image_rendition`, and `object_id` name different roles.
- Use the clearest variable name, including a longer name when it better expresses the value; name length has no runtime cost.
- Adaptability should be a focus so the software can evolve as requirements and insights change.
- Cargo, Node, and PyPI dependencies should use the latest versions to include security fixes.
- If an interface is measured as too slow, consider moving the slow code to Rust/WebAssembly.
- All fields, identifiers, domain concepts, and terminology in the PostgreSQL database structure, Rust code,
  TypeScript code, JSON/API contracts, Terminology Contract, and Human Guidance are in alignment.

### PLE development rules

- A fresh production installation includes the complete Live Demo by default.
- Treat the initial course content as shipped examples.
- BiologyProblems.org content is free and open source.
- The Genetics Blueprint Course from BiologyProblems.org ships as the example course.
- All Podman content on the Mac-Studio-36G machine belongs to this project.
- Neil pre-approves pruning Podman images, volumes, and containers on Mac-Studio-36G as needed and
  has provided the script `./devel/prune_podman.sh` for doing the pruning
- The polished PLE Live Demo is the top priority; see [LIVE_DEMO_SPEC.md](/docs/LIVE_DEMO_SPEC.md).
- PLE should use one global installation with no institution boundaries.
- Project images and simulated test data are disposable and can be recreated for testing.
- `./launchers/run_live_demo.sh` is the normal local-stack entry point. For direct controller
  diagnostics, use `source source_me.sh && python3 local_stack.py`.

## Product vocabulary and glossary

- **Account**: A global PLE user account with exactly one User Role.
- **User Role**: The Account's global role in PLE: **Student**, **Instructor**, or **Sysadmin**.
- **Sysadmin**: A PLE administrator with full administrative authority over the system.
- **Instructor**: A user who teaches Courses and can browse, reuse, create, fork, and publish Questions.
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
- **Blueprint Assessment**: An Assessment in a **Blueprint Course** that contains reusable content and teaching settings, without Students, dates, or other Course Instance delivery settings.
- **Course Instance Assessment**: An Assessment in a **Course Instance** that can be released and delivered to **Students**.
- **Assessment Type**: The pedagogical type of an Assessment: **Weekly Assignment**, **Unit Review Assignment**, **Bonus Assignment**, **Quiz**, or **Exam**.
  - **Weekly Assignment**: A weekly assignment with unlimited Attempts and a regular point value. For example, 10 Questions worth 1 point each, for a total of 10 points.
  - **Unit Review Assignment**: An exam-review assignment with unlimited Attempts and a low point value. For example, 10 Questions worth 0.1 point each, for a total of 1 point.
  - **Bonus Assignment**: A challenge assignment with unlimited Attempts and a small number of Questions. For example, 1-2 Questions worth 1 point each, for a total of 1-2 points, but it is extra credit, worth 0 points in the grade book.
  - **Quiz**: A single-Attempt, timed Assessment. A typical Quiz has 20 Questions worth 1 point each, for a total of 20 points, with a strict completion time of about 30 minutes.
  - **Exam**: A single-Attempt, timed Assessment. A typical Exam has two parts of 50 Questions each, for a total of 100 points, with a strict completion time of about 75 minutes per part.
- **Coursework**: The Student-facing collective term for Weekly Assignments, Unit Review Assignments/Exam Reviews, Bonus Assignments, Quizzes, and Exams.
- **Assessment Attempt**: One **Student** attempt at a Course Instance Assessment.
- **Assessment Template**: A reusable set of settings for creating Course Instance Assessments. It contains settings rather than Questions.
- **Assessment Question Editor**: The **Instructor** editor for selecting, adding, removing, and
  ordering Library Objects in an Assessment.
- **Assessment Properties Editor**: The **Instructor** editor for settings that apply to the whole Assessment, such as dates, scoring, Attempts, late work, and what **Students** can see.

### Question vocabulary

- **Question**: A PLE object containing the content, metadata, and Backend information needed
  to present and grade one automatically graded item.
- **Draft Question**: A private Question being developed by an **Instructor**. It must pass publication validation before becoming a Published Question.
- **Published Question**: A Question with numbered Revisions available for reuse through the global **Question Library**.
- **Question Revision**: A fixed version of a **Published Question** preserved so Assessments and Student Work can refer to the exact Question delivered.
- **Question Pool**: A published **Library Object** containing interchangeable **Published Questions** from which PLE selects Questions for a **Student**.
- **Question Backend**: The component responsible for a Question's rendering, interaction, response handling, grading, feedback, and backend-specific state.
- **Question Type**: Author-declared educational metadata describing the Question's interaction type, such as MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, or HOTSPOT.
- **Question Library**: The global collection of **Published Questions** and **Question Pools** available to **Instructors**.
- **Library Object**: A **Published Question** or **Question Pool** in the **Question Library**.
- **Question Image Asset**: A still image bound to an exact **Question Revision**. Current kinds
  are PNG, JPEG, and WebP.
- **Question Image Rendition**: The authorized delivered form of a **Question Image Asset**.

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
- Every Account has exactly one User Role: **Student**, **Instructor**, or **Sysadmin**.
- User Role is locked and cannot change during the lifetime of an Account.
- A person who needs more than one User Role uses separate Accounts.
- Instructor Accounts can be deactivated while preserving their content, Course relationships,
  and historical records.
- Reactivating an Instructor Account restores access to the same Account and User Role.

### Instructor role

- All **Instructors** have the same product capabilities.
- Once admitted to PLE, all Instructors are equal; there is no separate Verified Instructor role or status.
- A **Sysadmin** vets an Instructor before creating their Account; vetting happens outside PLE.
- PLE has no Instructor approval workflow or approval status.
- A **Sysadmin** creates an Instructor Account with an email address, first name, last name, and affiliation.
- PLE sends a setup email to that address so the Instructor can set up their Account.
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
- Each Course Instance has its own Student Record and enrollment for the Student Account.
- Student Work, Attempts, submissions, and grades are kept or deleted under the Course retention
  policy, independently of the Student Account.
- Removing a **Student** from a Course revokes future Course access but does not immediately delete the Student's Course records or Student Work.
- Student Work and grades remain subject to the normal Course retention policy after enrollment ends.
- An **Instructor** can deactivate a Student's access to their Course.
- Deactivating Course access does not delete the Student Account or Student Work.
- An **Instructor** can restore the Student's Course access later.
- **Instructors** can bulk add Students to a Course Instance through roster import.
- **Instructors** remove Students individually.
- PLE does not provide bulk Student removal from a Course Instance.

### Sysadmin role

- **Sysadmins** have god powers: full administrative authority over PLE.
- Sysadmins vet **Instructors** and create Instructor Accounts.
- Sysadmins can help Instructors repair Courses, Students, and content.
- Sysadmins should be reluctant to get involved in content.
- The human developer, Dr. Neil Voss, is currently both a **Sysadmin** and an **Instructor**.
- Neil uses separate Sysadmin and Instructor logins so the roles remain distinct.
- **Sysadmins** can access Course and Student records for administrative work.
- Before accessing FERPA-sensitive Student data, a Sysadmin confirms that access is needed
  for administrative work.
- Sysadmin access to FERPA-sensitive Student data is recorded for audit.
- Sysadmins use their full administrative access to support Instructors and resolve problems.

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
- Each page defines what its records show and what users can do. Shared components handle layout
  and spacing, keeping the same information and actions readable as the page gets narrower.
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

- Keep common tasks compact, with supporting details easy to open when needed.
- Use tooltips for brief supplementary explanations, available on hover and keyboard focus.
- Tooltips provide brief text; hover previews add too much overhead and slow the interface.
- Opening a list item opens it in a new browser tab or window, including search results and
  Assessments in an Assessment list.
- Ribbon navigation stays in the current tab. Buttons perform their stated action.
- Confirm before navigation discards an existing search and its results; leave empty search
  pages directly.
- PLE does not store old search results for later restoration.
- Search prompts and search results are never stored permanently; keep database size and bloat under control.
- Every search offers three display sizes: Compact, List, and Visual boxes.
- Use clearly labeled expandable sections with chevrons for longer details and secondary settings,
  supporting keyboard, pointer, and touch interaction.
- Keep essential information, primary actions, and current status visible in the main interface.
- Use drag-and-drop where it makes reordering faster and more natural.
- Reordering must also have a precise keyboard-accessible method.
- UUIDs should never appear in visible content, navigation URLs, or copyable links.

### Role badges

- The role badge is always in the upper left, just right of the logo.
- **Sysadmin** uses tomato red as its role color.
- **Instructor** uses teal green as its role color.
- **Student** uses lavender purple as its role color.
- The student role badge in mobile is shortened.
- Role colors should be used consistently in role labels and other appropriate interface cues.
- Demo role selection should clearly state both the user's role and name.

### Themes

- The interface uses a fixed set of visually distinct biome and habitat themes.
- Each theme has coordinated Light and Dark appearances.
- An Instructor has a personal theme for global Instructor pages and independently controls the theme assigned to each Course.
- Course pages use the Course theme. Global Instructor pages use the Instructor's personal theme.
- Light/Dark is a separate user preference that applies across all themes and pages.
- Light/Dark has only two selectable values: Light and Dark. When no explicit preference is saved, follow the browser setting.
- Changing Light/Dark must not change the selected theme. Changing a theme must not change Light/Dark.
- Themes should affect major page surfaces so each theme is visually distinct across the whole interface.
- Each Light or Dark theme appearance is defined by five source colors: Canvas, Surface, Secondary, Accent, and Highlight.
- Theme colors should remain accessible in their actual interface uses.
- Light themes should use clearly light page backgrounds. Dark themes should use clearly dark page backgrounds.
- Check text, controls, borders, and interaction states against their actual rendered backgrounds.
- Apply the contrast requirements for text, controls, and other semantic uses in
  [BIOME_THEME_PALETTES.md](/docs/BIOME_THEME_PALETTES.md) to rendered components in both Light
  and Dark modes, including gradients and state backgrounds.
- Pair color cues with text, icons, or shapes so selection, focus, saved status, and results remain
  recognizable across themes and color-vision differences.
- Theme IDs are durable. Changing a theme's display name or colors should not require a new ID.
- Follow `docs/BIOME_THEME_PALETTES.md` for theme names, palettes, accessibility, and implementation.

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
- Each Tier 1 choice has one Tier 2 set. If Tier 1 stays the same, Tier 2 stays the same. Opening a Course, Assessment, Question, or other item does not change the Tier 2 choices or their order.
- Tier 1 is a continuous colored Ribbon/bar. Its selected file-folder tab emerges through a contrasting surface and curved shoulders; unselected choices stay integrated into the bar.
- Tier 1 uses a compact tab silhouette around its label, with curved transitions into the Ribbon surface below it. Keep the Tier 1 surface separate from Tier 2; individual choices are not miniature folders or enclosed cards.
- The selected Tier 1 tab should look open. Choose its color relative to the Ribbon for each
  Light and Dark theme; the selected tab need not always be darker or always lighter.
- Tier 2 uses simple rectangular tabs with square corners immediately against the content surface. Its selected tab flows into that surface with a matching background and open bottom edge; unselected tabs stay integrated into their row.
- Active and inactive tabs in both Ribbon tiers must be clearly distinguishable by surface contrast in every theme and display mode. Inactive tabs visibly recede while retaining readable, enabled-looking labels; text accessibility contrast is a separate requirement.
- Verify selection visibility from fresh light and dark renders across themes. The active tab should be obvious at a glance without close inspection of its curve, border, or surface connection.
- Selection preserves each tab's basic geometry and the reserved row heights. Tier 1 widths follow their labels; Tier 2 labels may naturally need more width. Neither tier becomes substantially larger when selected.
- Keep these relationships on narrow screens, use the available Tier 2 width, keep the selected item visible, and make additional choices discoverable through overflow/scrolling cues.
- Use modern restrained styling. Reference images illustrate structure rather than gradients, gloss, heavy shadows, bevels, pills, or other decoration.
- Judge visual work from fresh renders of the whole composition: proportions, spacing, silhouettes, surface transitions, edges, and hierarchy. Requirements and tests protect behavior; visual completion means the interface looks finished and intentional.
- Tier 1 and Tier 2 tab geometry and hierarchy stay consistent across roles and themes; themes change their colors rather than their structure.
- The selected Tier 1 tab and the selected Tier 2 control stay selected when the current page is a descendant of those choices.
- Changing a Ribbon selection should change the content below the Ribbon without moving the main content area up or down.
- Ribbon rows should keep their space when needed so changing selections does not make the content area jump.
- Page actions should appear near the content they affect rather than changing the Ribbon layout.
- See **User top bar** and **Breadcrumbs** for the persistent elements that make up the top of the page.

### User top bar interface

- On laptop, tablet, and desktop, the top bar has one fixed left-to-right structure: **PLE logo, product name, User Role badge, Tier 1 navigation, flexible gap, Light/Dark control, Profile image**.
- On laptop, tablet, and desktop, the top-left structure is fixed: **PLE logo, then product name, then User Role badge**.
- On phones, the top-left structure is fixed: **PLE logo, then User Role badge**. The product name is omitted.
- Tier 1 navigation immediately follows the User Role badge.
- The Light/Dark control and Profile image stay together at the far right.
- The top bar structure and element order remain fixed as users navigate or change the selected Tier 1 choice.
- The PLE logo and product name link to the user's home dashboard.
- Each User Role has its own home dashboard and Tier 1 navigation.
- The User Role appears once in the top bar.
- The Profile image is an icon-only control that opens a dropdown menu containing **Profile settings** and **Sign Out**.
- **Profile settings** is the only page that names and sets the Account's time zone. Other pages show times without repeating the time zone name.

#### Phone top bar

- The phone top bar keeps the same identifying order at the left: **PLE logo, User Role badge**, followed by Tier 1 navigation.
- The phone layout uses the available width aggressively for navigation.
- Tier 1 navigation should expand into available horizontal space before requiring horizontal scrolling.
- When Tier 1 choices cannot fit, the Tier 1 navigation scrolls horizontally while preserving choice order.
- The Light/Dark control and Profile image remain at the right when space permits; compact phone behavior may adapt these controls without changing the identity and navigation order.
- Tier 2 navigation uses the full available screen width and may scroll horizontally when its choices do not fit.
- Tier 1 and Tier 2 navigation remain compact so the Ribbon leaves as much vertical space as practical for page content.

- See **Ribbon and page layout** for the overall navigation and page-position rules.


### Profile avatar interface

- Every Account is randomly assigned an avatar from the PLE avatar gallery when the Account is created.
- The same avatar gallery collection is available to all User Roles.
- The current avatar or Profile image appears consistently anywhere PLE represents that user.

#### Instructor Profile visibility

- Instructor Profiles are public within PLE to everyone with an Account, including Students.
- Instructor Profile images appear beside Published Question authors, Question Pool owners,
  and wherever else PLE shows an Instructor. Profile images have no separate permissions.

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
- The normal breadcrumb hierarchy is Home / Ribbon Tier 1 / Ribbon Tier 2 / Page / SubPage ...
- Each breadcrumb level links back to its corresponding page.
- Breadcrumbs use human-readable names rather than internal identifiers.
- Course and Assessment breadcrumbs preserve the current Course context.
- Keeping the breadcrumb row in place prevents the main content from moving up or down as breadcrumb depth changes.
- Breadcrumbs preserve useful intermediate navigation levels rather than collapsing the path to only broad and current pages.
- Include each meaningful ancestor that gives the user a useful place to navigate back to.
- Ribbon Tier 1 and Tier 2 selections remain in the breadcrumb list when the current page is a descendant of those navigation choices.
- Breadcrumbs represent the navigation hierarchy rather than a unique URL. Tier 1 and Tier 2 stay in the trail even when they link to the same page. Collapse two adjacent levels only when they show the same name.
- See **Ribbon and page layout** for the overall page-position rules.

### Instructor interface

- The Instructor interface should make frequent teaching tasks fast and easy to find.
- Keep teaching content central when Instructors create or review it. Arrange metadata and
  supporting explanations compactly around the content.
- Gradebook rows should identify Students by their Course roster names and Coursework by title, with IDs as supporting information where useful.
- The Instructor menu has **Courses**, **Questions**, and **Assessments** in one dense top bar.
- Instructor Profile uses a generic user icon until the **Instructor** adds a Profile image.
- All required Ribbon choices remain visible even when their collection is empty.
- Required Instructor Ribbon choices stay visible but disabled, without extra annotations,
  until their pages and actions are available.
- A working navigation destination remains visible when its collection is empty.
- A future or unavailable capability should not appear as a usable control until its workflow exists.
- Empty collection pages should explain what the collection is for and provide an obvious action to create or add the first item when the user can do so.
- Similar pages should place similar actions in consistent locations.
- Instructor pages should be composed around the teaching task rather than collections of padded components.
- Instructor lists and repeated records should be dense and easy to scan, more like a spreadsheet than cards.
- Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.
- Instructor lists should favor compact rows or tables with clear columns over cards or
  text run together without clear separation.
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
- Results should show Course name, classification, author, institution, and useful information
  about Course use and upkeep directly in the list for easy scanning and comparison.
- Public Blueprint Course search supports sorting by Stars, Watches, Adoptions, number of students
  having taken the course, and most recent edit, plus other relevant search metadata.
- Number of students having taken the course is an aggregate count, not identifiable Student records.
- Search terms, active filters, and the selected sort should remain visible while reviewing results.
- Clearing or changing part of a search should be quick.
- Opening a Blueprint Course search result opens a new tab or window, leaving the search,
  filters, sort, and scroll position in the original tab.
- A **Blueprint Course** should provide an obvious action for creating a **Course Instance** from it.

##### Blueprint Course editing interface

- Blueprint Course editing should follow Course Editor -> Blueprint Assessment Editor.
- The Course Editor should show the Blueprint Course structure without editing every Question on one page.
- Selecting a Blueprint Assessment in the Course Editor opens the editor for that Blueprint Assessment.
- Only the selected Blueprint Assessment's Library Objects should appear in its editor.
- **Blueprint Assessment Question Editor**: Selects, adds, removes, and orders
  Library Objects in a Blueprint Assessment.
- **Blueprint Assessment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see.
- Blueprint Courses should not contain Assessment dates or relative Assessment schedules.

##### Course Instance interface

- **My Active Courses** should emphasize Course Instances the Instructor is currently teaching.
- Active Course Instances should make upcoming Assessments and important course activity easy to find.
- **My Inactive Courses** should keep past Course Instances available without competing with active Course Instances.
- Creating a Course Instance from a Blueprint Course preserves its Assessments, Library Objects,
  and settings.
- Assessments created from a Blueprint Course start unreleased with dates unset.
- A Course Instance represents one teaching period and remains Active for at most six months from
  creation.
- Course banners use a 5:1 aspect ratio.
- 1280 by 256 pixels is the recommended Course banner authoring size.
- Higher-resolution 5:1 Course banner images are supported.
- PLE responsively scales Course banners while preserving their aspect ratio.
- Course banners appear as small centered banners rather than full-width page heroes.
- An Instructor can upload a Course banner and select a Course Theme from the fixed theme catalog.
- Course Instance Assessments have two editors:
  - **Assessment Question Editor**: Selects, adds, removes, and orders Library Objects in an Assessment.
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
- Adding Published Questions to a Pool should begin with Question Library results filtered to the Pool's
  Discipline and Subject.

##### Search Question Library interface

- **Search Question Library** helps Instructors find specific Library Objects in a large library.
- Instructors sometimes explore and sometimes know what they are looking for; the search interface
  should support both.
- Instructors can begin exploring or searching directly in the shared spreadsheet-style interface,
  like Google Sheets with filters.
- Simple search returns the same advanced spreadsheet-style results, with the entered search and
  filters available for further refinement.
- The modular spreadsheet-style interface supports compact, list, and movie-poster-style boxes;
  the display mode names are not settled.
- Image-focused boxes can help Instructors find visual Questions.
- Question Library search should work well with ordinary words by default.
- Results should make it easy to scan many Library Objects quickly.
- Results should show the information needed to judge relevance without opening each Library Object.
- Search terms and active filters should remain visible while reviewing results.
- Clearing or changing part of a search should be quick.
- Opening a Question Library list item opens a new tab or window, preserving the search and its position
  in the original tab.
- The interface should show options by priority without overwhelming new users.

##### Question Library design references

- Reddit's multiple display modes are a reference for the shared interface.
- OER Commons has a simple search worth considering and image-focused boxes useful for finding
  visual content: https://oercommons.org/
- MovieLens is a reference for tiered filters: https://movielens.org/explore/
- IMDb has a well-designed advanced search page: https://www.imdb.com/search/title/
- Google advanced search is a user-friendly design reference: https://www.google.com/advanced_search
- PubMed is clean but not obvious to use: https://pubmed.ncbi.nlm.nih.gov/advanced/
- eBay is dated but may be a useful comparison: https://www.ebay.com/sch/ebayadvsearch

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

- **Browse Question Library** helps Instructors explore Library Objects without knowing what to search for.
- Browse should help Instructors understand what the Question Library contains.
- Browse should begin with Discipline and make moving through Subject, Topic, and Subtopic easy.
- Selecting a Discipline limits browsing to Subjects associated with that Discipline.
- Browse should also offer Tags, Question Types, and other useful groupings.
- Browse should show useful counts where they help Instructors choose where to explore.
- Browse results should use the same dense Library Object presentation used by Search where practical.
- Instructors should be able to move from browsing into a more focused search.
- Search and Browse are different paths into the same **Question Library**.

#### Assessment interface

- The **Assessments** ribbon must include: Assessments Due Soon, My Assessment Templates.
- **Assessments Due Soon** should emphasize Assessments that may need the Instructor's attention.
- Assessment lists should make Course, release status, due date, and other important state easy to scan.
- **My Assessment Templates** should emphasize reusable Assessment design rather than Course activity.
- Assessment editing has two editors:
  - **Assessment Question Editor**: Selects, adds, removes, and orders Library Objects.
  - **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and other Assessment settings.
- The two Assessment editors should remain clearly distinct.
- The Assessment Question Editor should make the order of Library Objects easy to understand
  at a glance.
- Adding Library Objects should provide direct paths to Search and Browse Question Library.
- Instructors should be able to inspect a Library Object before adding it to an Assessment.
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
- **Weekly Assignment** uses the Font Awesome `pen-to-square` icon.
- **Unit Review Assignment** uses the Font Awesome `arrows-spin` icon.
- **Bonus Assignment** uses the Font Awesome `star` icon.
- **Quiz** uses the Font Awesome `circle-question` icon.
- **Exam** uses the Font Awesome `file-signature` icon.

#### High-consequence actions

- Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
- Danger Zone should be visually separate from ordinary editing actions.
- Assessment Unrelease should explain that Student work will be deleted.
- Assessment Unrelease should require typing the Assessment title before confirmation.
- Archive actions should explain how archiving affects access to the shared content and require
  a clear confirmation.
- Restore actions should use ordinary availability controls.

### Student interface

- Guidance about the student interface

#### General Student interface

- The Student interface should focus on current Courses, Coursework, and work that needs attention.
- **Coursework** is the Student-facing collective term for Weekly Assignments, Unit Review Assignments, Bonus Assignments, Quizzes, and Exams.
- Student-facing interfaces should use the specific Assessment Type when referring to an individual item rather than calling it an Assessment.
- The Student interface should make the next useful action easy to find.
- Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.
- Student layouts should adapt smoothly at intermediate widths, with readable long titles and controls that wrap or rearrange in the task's reading order.
- Every Student browser action should be usable with the keyboard alone.
- Student pages should use names meaningful to Students.
- Student navigation and pages should contain only Student interfaces and capabilities.
- Student content entry should use the response controls provided by Questions and other Student activities.
- Students should have no upload capabilities. Instructor-created content should use text boxes.

#### Student Ribbon interface

- The Student Ribbon should use familiar Student language rather than internal PLE terms such as Assessment.
- Student Tier 1 navigation uses **Coursework**, **Grades**, and **Courses**, in that order.
- **Coursework** and **Grades** show relevant information across all of the Student's enrolled Courses. When a Student has more than one Course, records clearly identify their Course.
- **Coursework** and **Grades** each have a stable Tier 2 set. Opening Coursework, an Attempt, a review, or another item does not change the Tier 2 choices or their order.
- I accept grayed-out Tier 2 choices; keep each in place instead of hiding or removing it, then dynamically inserting or restoring it.
- Student Tier 2 must include the choices below. Additional choices may be added when they provide a useful Student navigation destination.

  - **Coursework**
    - **All Coursework**: Shows all Coursework across the Student's enrolled Courses.
    - **Due Soon**: Shows Coursework across the Student's enrolled Courses that is approaching its due date.
    - **Completed**: Shows Coursework across the Student's enrolled Courses for which the Student has submitted at least one Attempt. Completed does not mean that the Student earned a perfect score.
    - **Active Attempt**: I want a quick return to an Assessment with a running clock. Keep it visible but disabled when no Attempt's clock is running.

  - **Grades**
    - **Scores**: Shows the Student's automatically graded Coursework scores across their enrolled Courses as soon as each Attempt is submitted and graded.
    - **Response Stats**: Shows statistics about the Student's responses in submitted Attempts across Coursework, subject to per-Question feedback rules.
    - **Attempt History**: Shows the Student's previous Assessment Attempts across their enrolled Courses and provides access to review them when permitted.
    - **Latest Feedback**: Provides quick access to the most recent feedback available to the Student across their enrolled Courses. It remains visible but disabled when no feedback is available.

  - **Courses**
    - Tier 2 shows the short names of the Student's currently enrolled Courses.
    - Selecting a Course opens that Course.
    - The current Course is visually identified when the Student is viewing Course-specific content.
    - The Course list changes when the Student's Course enrollment changes; navigating within a Course does not change the list or its order.

- On narrow screens, use a compact navigation arrangement that keeps the product identity, current location, navigation controls, and Profile readable and reachable.

#### Student Course and Coursework interface

- I use Coursework and Grades across all my enrolled Courses; opening a Course from Courses does not pin or filter those global views.
- When I view Course-specific content, the Ribbon, breadcrumb, PageFrame, and relevant records should make the Course clear.
- Students should be able to see their active Courses and Coursework from the main navigation.
- Course invitations should show the Course name and relevant Instructor and term information before the Student accepts the invitation.
- Course pages should make upcoming, available, completed, and missed Coursework easy to distinguish.
- Coursework lists should make due dates, Type, and completion status easy to scan.
- Progress should keep Coursework visible while an Attempt is in progress and show its score once the Attempt is submitted and graded.
- Submitted work should remain distinct from a perfect score; **Completed** means at least one submitted Attempt.
- **Response Stats** should show actual Student response outcomes from submitted Attempts across Assessment Types, subject to per-Question feedback rules.
- Measured Question display time should be labeled as approximate **time shown with the Question**, with its sample count. It does not measure attention or effort and does not affect grades.
- Keep Coursework entries compact in height so Students can scan several items at once.
- Keep essential Coursework information and the main action visible. Make fuller access and
  timing details easy to open when needed.
- Coursework lists may provide filters for **Weekly Assignments**, **Unit Review Assignments**, **Bonus Assignments**, **Quizzes**, and **Exams**.
- Each Coursework item should clearly show its Assessment Type using its label and Type icon.
- Before starting Coursework, Students should see its title, Type, Question count, points possible, time limit, and previous Attempts.
- Present the "Before you start" settings as a compact summary. Keep each label beside its value in aligned rows, using a compact grid when width permits.
- Group Question count and points together, and group availability, deadlines, and Attempt rules into clearly readable sections with concise spacing.
- Express unset or unlimited settings in Student language, such as "No closing time" or "Unlimited Attempts". Format dates in the selected display zone.
- Keep the start action close to this summary so Students can review the rules and begin with minimal scrolling.

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

- Question scores and the Assessment total are visible once the Attempt is submitted and graded; optional Question Feedback timing remains deferred.
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
- Sysadmins create Instructor Accounts only after vetting the Instructor outside PLE.
- Sysadmins control existing Instructor access through deactivation and reactivation.
- Sysadmins should be able to find and inspect Courses across the installation.
- Course administration should show the Instructor and important Course status information.
- Sysadmins should manage Courses through Sysadmin interfaces and capabilities.
- Everyday navigation should emphasize frequently used administrative tasks.
- Rare installation and configuration tasks should remain available through secondary navigation.
- High-consequence administrative actions should have a visually distinct area.
- Confirmation for destructive actions should clearly state what will happen.
- The complete Sysadmin Ribbon task layout does not have a locked-in design yet.

## Data and history

- Answers, keys, grading, and correctness decisions should stay on the server, out of reach of **Students**.
- Keep public data separate from private data, answers, identifying information, and radioactive
  FERPA data.
- Human-readable titles and identifiers should be used wherever people must recognize, copy, or enter them.
- FERPA-sensitive Student data should not become ordinary logs, analytics, URLs, or long-lived browser storage.
- Opaque IDs remain FERPA-sensitive when they link a Student to Course activity.

### Human-facing public IDs

- Public IDs are intended for content creators (Instructors) and for Sysadmins providing Instructor support.
- Here, "public" refers to Instructors and Sysadmins.
- Human-facing public IDs should be short, opaque, and easy to communicate.
- Human-facing public IDs should not reveal creation order, counts, database keys, ownership, or other object metadata.
- A public ID is the one universal, canonical human-facing identifier for a PLE object that needs one.
- Give an internal object a human-facing public ID when a useful workflow needs it.
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
- When an ID is entered, normalize it to the canonical form, then validate its syntax and checksum.
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

- Library Objects use the public `XXXX-ZXXX` format.
- Question ID is the shared term for Published Question IDs and Question Pool IDs.
- The hyphen is specific to Question IDs. Other public IDs use a prefix without a hyphen.
- Published Questions and Question Pools share the same public-ID namespace.
- An `XXXX-ZXXX` value identifies either a Published Question or a Question Pool, never both.
- Blueprint Course IDs use `BPXXXXXXXZ`.
- Course Instance IDs use `CIXXXXXXXZ`.
- Assessment IDs use `AXXXXXXXZ`.
- Account IDs use `UXXXXXXXZ`.
- Each prefixed public ID uses seven cryptographically random Crockford Base32 characters and a final embedded checksum.
- Each prefixed public-ID random namespace contains 32^7 = 34,359,738,368 values.
- Sysadmins use Account IDs (`UXXXXXXXZ`) to look up Accounts for support.
- Students and Instructors see their Account ID (`UXXXXXXXZ`) only on their Profile page.

#### Database keys and clocks

- An object with a public ID uses that public ID as its primary key and as the target of every
  foreign key to it.
- An object without a public ID uses a native UUID primary key, or a combined key when it is owned
  by a parent (for example, a Revision identified by its parent object ID and Revision Number).
- One value that identifies one object is an Id, such as a Public ID or UUID.
- Multiple values that together identify one exact object, state, or version are a Tuple.
- Tuple is the general cross-language term and suffix for a composite identity made from multiple values.
- A Question Revision Tuple is one example: Question ID plus Question Revision Number.
- A Blueprint Course Revision Tuple is a Blueprint Course ID plus Blueprint Revision Number.
- Use Reference for a genuine indirect, scoped, or external locator.
- Do not name an Id or a Tuple as a Reference; "Reference" reads like a pointer, not a composite identity.
- Use the simplest term that accurately describes what the value represents.
- An object without a public ID uses its UUID Id in routes and JSON. Secondary Ids are not allowed.
- Internal UUIDs never substitute for or appear as public identities.
- Table shape and clocks follow [DATABASE_STYLE.md](/docs/DATABASE_STYLE.md).

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
- **Instructors** can create new Subjects within a selected Discipline.
- When an Instructor attempts to create a Subject whose globally unique name already exists, PLE
  offers the existing Subject.
- PLE requires explicit Instructor acceptance before associating the existing Subject with the
  selected Discipline.
- **Instructors** can create new Topics within a Subject.
- **Instructors** can create new Subtopics within a Topic.
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
- Courses and Library Objects can have any number of Tags, including none.
- Subject, Topic, and Subtopic names must satisfy length limits and formatting requirements.
- Length allowances increase from Subject to Topic to Subtopic, supporting more specific names as classification becomes narrower.
- Strip leading and trailing whitespace from Subject, Topic, and Subtopic names and validate the resulting names consistently.

### Student and FERPA data

- **Student** course data falls under FERPA; treat it as radioactive.
- **Student** data should be collected reluctantly, used deliberately, and purged predictably.
- FERPA access should be scoped through exact Course membership and **Student** ownership.
- **Sysadmins** should use Course and Student records only for administrative work.
- Student Accounts persist independently of Course data and Course retention.
- Course work, Attempts, submissions, grades, and other FERPA-sensitive data follow the Course retention policy.
- Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
- **Student Work** is the collective term for FERPA-sensitive records created by a Student in a Course Instance.
- Student Work includes Assessment Attempts, saved Question responses, grading outcomes, and the evidence needed to interpret that work after an Attempt is submitted.
- Student Work is an umbrella term; the underlying records retain their own identities and purposes.
- Student retention removes identifiable Student evidence and leaves Question usage statistics
  unchanged.

### Course retention and lifecycle

- Course retention should follow Course Instance dates and its six-month Active lifetime rather than
  a fixed academic calendar.
- The latest Assessment deadline ends normal teaching and starts the Course Instance's FERPA
  retention clock.
- Creating or extending a later Assessment deadline may move those dates, but not beyond the
  six-month Active lifetime.
- Starting the FERPA retention clock does not itself notify, archive, hide, or delete Student data.
- The configured FERPA retention policy determines when to send notices, archive Student data,
  allow recovery, and permanently delete it.
- PLE warns the **Instructors** before the Course Instance becomes Inactive six months after
  creation.
- The six-month Active limit prevents Course reuse or deadline extensions from indefinitely delaying
  FERPA retention and deletion.
- A Course becoming Inactive does not itself delete Student records. FERPA deletion follows its
  own schedule.
- Retention should work equally for semesters, quarters, summer Courses, and other academic calendars.
- PLE should notify the **Instructor** before FERPA-sensitive Student data is archived.
- Archived Student data should leave normal Instructor and Student interfaces but remain recoverable during the retention period.
- FERPA-sensitive Student data should be permanently deleted when its retention period expires.
- Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
- FERPA retention periods are set in the server configuration.

### Course retention processing

- A background process should periodically find Course Instances whose retention deadlines have passed.
- Retention decisions should come from stored Course dates and the Course Instance creation time.
- The background process should execute retention policy rather than define when retention periods begin or end.
- Running the retention process late should produce the same retention decision as running it on schedule.
- The retention process should be safe to run repeatedly.

### Common revision and history specifications

- Be conservative about creating revisions.
- Assessments, Course Instances, and Draft Questions use current state.
- Published Questions and Blueprint Courses use numbered Revisions for their saved content.
- A Revision is a complete database record. Permitted metadata fields can change without
  creating another Revision.
- Use an Edit Number when current-state editing needs conflict detection.
- An Edit Number is only a counter and does not identify a stored historical object.
- Question and Blueprint Revision Numbers start at 1 and increase sequentially for each object.
- A Revision Number identifies a specific complete Revision record stored by PLE.
- A new Revision keeps the same Published Question ID.
- Forking a Library Object creates a new public ID.
- A Published Question fork starts at Revision 1 under its new ID.
- A Question Pool fork starts at Edit Number 1 under its new ID.
- Student Work records the exact Assessment Attempt and Published Question Revision delivered to the Student.
- Student Work records the Student's responses and the grading outcome returned by the Question Backend.
- For a Published Question selected from a Pool, Student Work records the Question Revision Tuple,
  Pool ID, and Pool Edit Number at selection time.
- Changes to Question point values recalculate scores from the stored grading outcome without changing the outcome.
- Changes to Assessment settings do not change the recorded history of completed Assessment Attempts.
- Immutable Question source and Question Image Assets use SHA-256 checksums where needed to verify their stored contents.
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
- I want times in my account formatted in the selected display zone, with the zone name shown only on
  Profile.

## Question specifications

- Questions are backend agnostic.
- Questions are subject agnostic. Properly classified Published Questions from all subjects belong in
  the same Question Library.
- Questions are graded automatically. Grading the same response under the same conditions gives
  the same result and requires no **Instructor** action.
- Questions have one canonical title. Compact interfaces may truncate that title.
- Every Question stored by PLE has its own internal Question record.
- Answer-choice randomization belongs to the Question.
- PLE-native Questions control their own answer-choice randomization.

### Draft Question specifications

- The backend-agnostic Question workflow is: import or write a Draft, preview it, test it,
  refine it, add the metadata, then publish.
- Drafts are Drafts and have no content or metadata requirements. Those requirements apply at
  publication.
- Draft Questions are private working content.
- Draft Questions are not part of the Question Library.
- Draft Questions use current state rather than immutable Revisions.
- Draft Questions autosave with a visible saved status so Instructors can return to unfinished work.
- Saving a Draft Question replaces its previous working state.
- **Instructors** can delete Draft Questions they no longer need.
- PLE may clean up abandoned Draft Questions after an appropriate warning and recovery period.
- A Draft Question must pass Question Publication Validation before becoming a Published Question.
- Question Publication Validation requires Discipline, Subject, and all other required Question
  Library metadata before publication.

### Question formats and type specifications

- PLE flat-question JSON is the canonical machine format for simple static Questions.
- QTI is for import, export, and archival interchange rather than the internal source model.
- QTI ZIP files, retained QTI archives, and images extracted from QTI packages are import/export
  files, separate from Question Image Assets.
- MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT Question Types should be supported.
- For Native JSON Matching Questions with partial credit enabled, each correct pair earns equal
  credit. Wrong or unanswered pairs earn zero, with no additional deduction. Three correct pairs
  out of five earn 60% credit.
- Native JSON owns its Multiple Answer partial-credit formula; Question Backends grade their own Questions.
- Assessment Instructors decide whether to award partial credit. New Assessments start with partial credit enabled.
- Native JSON MC and HOTSPOT Questions are graded all-or-nothing.
- Native JSON NUM Questions use a tolerance to judge the answer.
- Native JSON FIB Questions accept a list of possible answers and support regular expressions.
- Native JSON MULTI-FIB Questions are multiple independently graded FIBs.
- Native JSON ORDER partial credit gives equal weight to correct positions and correct relative
  order. DABC earns 25% for an ABCD answer key.
- Question Type cannot be NULL. Other required fields must also be present before publication.
- Native JSON has a built-in Question Type; other Question Backends use Question Type as
  editable classification metadata.
- PLE uses Question Type for search, filtering, labeling, and presentation.
- Use the Type built into Native JSON. Assign WeBWorK Question Type manually for now; automatic
  detection is deferred. For other Backends, the author or importer supplies the classification.
- Question importers convert external content into the Question format PLE stores; they are used
  during import.
- PLE uses `qti-package-maker-rs` as an external library to handle all of its conversion.
- Export is for selected Questions to use in another LMS.
- Once PLE launches, BiologyProblems.org and PLE are no longer connected. PLE no longer cares
  how BiologyProblems.org changes.

### Native PLE JSON Question specifications

- Question metadata belongs on the Question record, not in Native JSON.
- Native JSON contains the Question content needed to display and grade the Question.
- The native PLE JSON Question format is private, unversioned, and unpublished.
- Stored native JSON Questions may be upgraded together when the internal format changes.
- The native PLE JSON Question format is internal and strictly validated. It has no external API.
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
- HOTSPOT content uses supported still images and SVG.
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
- Each Question Revision stores the Question content used by its Backend.
- Question Backends own rendering, interaction, response, grading, feedback, and backend-specific state.
- PLE owns authorization, Question ID, revisions, persistence, lifecycle, and stored outcomes.
- PLE uses the same basic interface for every Question Backend. Each Backend handles its own
  internal details.
- Each Question Backend adapter retains its backend-specific interaction knowledge.
- Question Backends may support more complex interactions without requiring PLE to implement those interactions.

#### Question Backend grading and feedback

- Question Backend feedback is transient unless the backend provides a robust way for PLE to preserve it.
- PLE does not extract or reconstruct transient feedback from Question Backend source or output.
- PLE-managed Hints, Question Feedback, and Worked Solutions remain separate from backend-generated
  interaction feedback.
- A Question Backend returns an immutable credit fraction for each complete response it evaluates.
- PLE stores the immutable credit fraction as the grading outcome.
- When PLE requests a grading outcome, the Question Backend returns the result in that response.
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

- A Published Question is a Question with numbered Revisions available for reuse through the
  Question Library.
- Every **Instructor** can read, add to an Assessment, or fork any Published Question.
- Only the owning **Instructor** can edit a Published Question; **Sysadmins** can edit any
  Published Question.

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
- Published Question metadata includes its owner, authors, license, and citation.
- Published Questions can include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
- Published Questions also use the shared Question Library metadata required for publication.

#### Published Question revisions, edits, and forks

- Each Published Question Revision is a complete Question record, with permitted fields that
  can be edited in place without creating a new Revision.
- Assessments and Student Work record exact Question Revision Tuples. The source and grading
  content for those Revisions stay fixed.
- Publishing a new Question Revision does not silently change existing Assessments or Student Work.
- The owning Instructor or a Sysadmin can publish a new Question Revision.
- Changing Question source, answer content, grading rules, Hints, Question Feedback, Worked Solutions,
  or Question Image Assets creates a new Question Revision.
- Editing Title, Description, Discipline, Subject, Topic, Subtopic, Tags, or Bloom updates the
  current Published Question record and keeps the same Revision Number.
- A new Question Revision starts as another complete record, carrying forward its attributes
  except for the changes being published.
- Any **Instructor** can fork a Published Question. The fork has a new Question ID, its own owner,
  and starts at Revision 1.
- A Question fork starts with the source Question's license, authors, metadata, and Question
  content, and records the source Question as its parent.
- A fork starts as a private **Draft Question** owned by the Instructor who creates it.
- A fork must pass Question Publication Validation before joining the Question Library.
- Published forks retain source attribution.
- Archive preserves a Published Question that cannot safely be deleted because it is used or referenced.
- Archive makes the ordinary Question read-only, removes it from normal discovery, preserves
  existing references, and allows restore or fork.
- Keep Archive behavior simple for both Published Questions and Blueprint Courses. The less
  Archive-specific architecture, the better; otherwise follow the GitHub repository archive model.
- Question authors are preserved across Revisions and forks.
- Instructors watching a Published Question receive notifications in PLE about new Revisions
  and forks.

#### Published Question behavior specifications

- Published Questions can include optional PLE-managed **Hints**, **Question Feedback**, and **Worked Solutions**.
- PLE-managed Hints, Question Feedback, and Worked Solutions are separate from Question Backend-generated content.
- WeBWorK Questions can use PLE-managed Hints, Question Feedback, and Worked Solutions even when similar material also exists in the WeBWorK source.
- Question Feedback is shown when its display settings allow it.
- Hints and Worked Solutions each have their own settings for when Students can see them.
- Students can complete a Question with or without Hints, Question Feedback, or Worked Solutions.

### Question Pool specifications

- A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
- A Question Pool contains Published Questions only; it cannot contain another Question Pool.
- When editing a Pool, Instructors should be able to sort its Published Questions like a
  spreadsheet. Sorting changes only the display; it leaves the Pool contents and random selection
  unchanged.
- Published Questions in a Pool should be reasonably interchangeable for assessing the intended
  learning.
- All members of a Question Pool use the same Question Backend.
- All members of a Question Pool have the same Question Type.
- Question Pools are created from a Published Question and enter the Question Library immediately.
- A Question Pool is an independently reusable Question Library object, designed to be forked often.
  The Instructor who owns it is its owner.
- Question Pools are available to all **Instructors**.
- A Question Pool has its own public `XXXX-ZXXX` Crockford Base32 ID.
- For forking, we are mostly using the GitHub model.
- Changes to a Pool take effect when the Instructor saves them. There is no undo after saving.
- A Pool Edit Number is only a counter and does not identify a stored historical Pool.
- A Question Pool contains an unordered set of Question Revision Tuples.
  When the set changes, saving advances the Pool's Edit Number; no Revision is created.
  Removing ten Questions and saving once is one Edit.
- A Question Pool can contain a Question ID only once.
- A Question Pool has its own metadata. Some metadata belongs directly to the Pool, while other
  metadata is derived from the Published Questions it contains.

#### Question Pool use and selection

- An Assessment can use an existing Question Pool and specifies how many Questions to select from it.
- The number selected belongs to the Assessment, not the Pool.
- Adding a Question Pool to an Assessment uses that existing Pool; adding it does not create a fork.
- One Assessment or hundreds can reference the same Pool. Its owner stays the same.
- Pool changes affect future selections wherever that Pool is referenced. Existing Attempts retain
  the Questions already selected for them.
- Forking a Question Pool is an explicit action used when an Instructor wants an independent Pool.
- A Pool fork is a new Question Pool with a new Pool ID and its own owner, and records its source Pool.
- A Pool fork begins with the parent Pool's current Question Revision Tuples
  and starts at Edit Number 1.
- A Pool fork is a normal reusable Question Pool and can be used in any number of Assessments.
- A Pool fork has its own Instructor owner and can be changed without changing its parent Pool.
- Question Pools work the same way regardless of the Question Backend.
- **Instructors** choose the Questions in a Pool. Each Assessment specifies how many Questions
  to select from it.
- PLE randomly selects from the Question Pool; the selected Question Backend controls the Question interaction.
- At release, each Pool must contain enough valid Published Questions to provide the number
  the Assessment requests.
- Question Pool selection and backend-native randomization are separate forms of variation.
- Returning to an Attempt preserves the Question Pool selections already made.
- Starting a new Attempt makes fresh selections from its Question Pools.
- For each Published Question selected from a Pool, Student Work records its Question Revision
  Tuple, the Pool ID, and the Pool Edit Number. That Question Revision provides what PLE needs
  to interpret and grade the response later.
- Grading and historical evidence follow the exact Published Question Revision delivered to the Student.
- Each Question Pool member retains its own owner and authors; owners and authors may differ between members.
- Each member of a Question Pool is a **Published Question**.
- Question Pools contain only **Published Questions**; Question Pools cannot be members of Question Pools.
- Instructors watching a Question Pool receive notifications in PLE when its Questions change
  or when it is forked.

- Each Question Pool must have one license compatible with every member Question's license; its license cannot be "Mixed".
  PLE calculates this license automatically from the member licenses and rejects incompatible combinations.

#### Question Pool metadata

- Text search matches only the Question Pool's own text and metadata, not the text or metadata of its member Questions.
- Topic/Subtopic, Tags, and Bloom filters match only the Question Pool's own metadata, not its members' metadata.
- Question Pools have metadata specific to the individual Question Pool.
- Question Pool metadata includes its own Title, Description, Topic, Subtopic, Tags,
  and both Bloom dimensions: Knowledge Dimension and Cognitive Process.
- The first Published Question establishes the Question Pool's Discipline and Subject.
- Every additional Published Question added to the Pool has the same Discipline and Subject as the Pool.
  Topic and Subtopic do not have to match.
- A Pool must continue to meet its requirements. Show the specific problem when a Published
  Question's Discipline or Subject differs from the Pool's, the same Published Question appears
  more than once, or another Pool requirement is not met.
- If an Assessment asks for more Questions than a Pool can provide, show the problem and block
  release. The Pool itself may still be valid.
- If a Pool develops a problem after an Assessment is already released, allow that Assessment
  to continue as-is.
- Published Questions retain their own Topic, Subtopic, Tags, and other Library Object metadata
  when included in a Question Pool.
- A Question Pool identifies its owner and its source Pool when forked; it has no separate Author field.
- Question Pool metadata describes the Pool rather than duplicating metadata from its member
  Published Questions.
- Question Pools can include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
- Question Pools also use the shared Question Library metadata required for publication.

### Question Library specifications

- Question sharing, discovery, and reuse are a high-priority **Instructor** workflow.
- The Question Library is one global collection of Published Questions and Question Pools.
  Both appear in the same search, with filters for Published Questions, Question Pools,
  and whether a Published Question belongs to a Pool.
- Search supports a "Questions in no Pool" filter: Published Questions that belong to no Pool.
- Default searches would probably be best showing Questions in no Pool together with Question
  Pools; showing individual Pool members as well adds significant noise.
- Search filters can also include individual Pool members so Instructors can find all
  Published Questions.
- Draft Questions are not part of the Question Library.
- **Library Objects** are available to all **Instructors**.
- **Students** access Question content through their Coursework rather than through the Question Library.
- Question Library content remains discoverable when used by a private **Course Instance**.
- With 13,000 Questions in Neil's first course, manually archiving Questions is unlikely to be a useful primary workflow.
- Question Library workflows should support bulk operations because an **Instructor** may manage thousands of Questions.
- **Instructors** should be able to select many Library objects and update shared metadata such as
  Discipline, Subject, Topic, Subtopic, Tags, or other search fields together.
- Question Library search, filters, sorting, and bulk editing should make large imports practical to clean up.

#### Question Library metadata

- **Library Objects** use shared metadata for organization, search, filtering, and discovery.
- Instructors may not fill out all metadata. Assign it automatically where possible and require
  the remaining necessary information before publication.
- Required Question Library metadata must be complete before a Library Object enters the Question Library.
- Library metadata should describe the Library Object rather than its location in a Course, Assessment, or textbook.
- Library Objects use the shared **Discipline**, **Subject**, **Topic**, **Subtopic**, and **Tag**
  vocabulary.
- Every Library Object has exactly one **Discipline** and one **Subject**.
- **Topic** and **Subtopic** are optional for Library Objects.
- Library Objects can have any number of **Tags**, including none.
- Question Publication Validation requires Discipline and Subject before publication.
- Library Object classification follows Discipline -> Subject -> Topic -> Subtopic.
- Library Objects retain their classification when used in an Assessment.
- Library classification supports searching, filtering, sorting, and bulk editing.
- Library Objects can have PLE-managed **Hints**, **Question Feedback**, and **Worked Solutions**.
- Support content may be attached at the level where it applies rather than duplicated across individual Questions.

#### Question Library object usage statistics

- Library Objects keep privacy-safe aggregate usage statistics.
- Keep statistics separate for each Published Question Revision and each Question Pool.
- Aggregate statistics contain counts and sums, never Student Attempts or identifiable Student
  records.
- Statistics show how often Students received each Published Question Revision or Questions from
  each Pool and how much credit they earned.
- Include the number of graded responses, average earned credit, and percentages earning full credit and zero credit.
- Use the stored Question credit fraction for these statistics; Assessment settings determine awarded points separately.
- Removing Student names alone does not make statistics anonymous.
- Shared statistics should be shown only when individual Students cannot reasonably be identified
  from the aggregate.
- Course-specific analysis remains FERPA-sensitive when individual Students could be inferred.
- Database details and rules for updating statistics are in
  [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) and [FERPA_DATA_POLICY.md](/docs/FERPA_DATA_POLICY.md).

#### Question Library Bloom classification metadata

- Published Question Revisions and Question Pools can have a Bloom Cognitive Process and Bloom
  Knowledge Dimension.
- The two Bloom dimensions are independent and together determine the object's Bloom Classification.
- Bloom Classification supports Question Library search and Assessment item sorting.
- A Question Pool's Bloom Classification describes the intended cognitive work of the Pool as a whole.
- Bloom Classification starts blank for new content awaiting AI assignment. A Pool fork copies
  the parent Pool's existing Bloom fields along with its other Pool metadata.
- Bloom Classification may be NULL while awaiting AI assignment. Do not enforce a time limit.
- AI assigns the initial Bloom Classification using a daemon after publication.
- The owning **Instructor** can correct either Bloom dimension without creating a new Published Question
  Revision.
- Question Library search and reporting should make both Bloom dimensions useful to **Instructors**.
- Bloom is an editable field like Title. Save it like other metadata and check for conflicting edits.
- Follow `docs/BLOOM_TAXONOMY_GUIDE.md` for Bloom classification and teaching interpretation.

#### Question Library stewardship specifications

- Question Library stewardship should use a GitHub-like model.
- An Instructor who finds a problem in another Instructor's Library Object can fork it and fix it.
- PLE is not social media or an online forum.
- Use **Change Proposals** as the Instructor-facing term for proposed content changes.
- Stars follow the GitHub model: Instructors can Star or unstar content, see its Star count, and see who Starred content they can access.
- Use **Stars** and **who Starred** rather than a separate "endorsement list" concept.
- Library Objects can be starred and watched.
- Star means favorite and visible endorsement.
- **Instructors** can see the Star count and which **Instructors** starred a Library Object.
- Watch means subscription.
- Instructors watching a Published Question receive notifications in PLE about new Revisions
  and forks.
- Instructors watching a Question Pool receive notifications in PLE when its Questions change
  or when it is forked.
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
- The Instructor who creates a Course Instance becomes its first Instructor and has the same
  Course authority as co-Instructors added later.
- **Adoption** connects a Blueprint Course and a Course Instance when an **Instructor** creates a new Course Instance from a Blueprint Course or creates a new Blueprint Course from an existing Course Instance's reusable structure.
- An Instructor can create a new Blueprint Course from an existing Course Instance's reusable structure. The new Blueprint Course records that Course Instance as its source, and the Course Instance remains the same teaching instance.
- A Course Instance created from a Blueprint Course is a daughter Course Instance of that Blueprint Course.

### Course classification specifications

- **Blueprint Courses** and **Course Instances** use the shared content classification system.
- Course classification describes the Course as a whole.
- Every Blueprint Course and Course Instance has exactly one **Discipline**.
- **Subject**, **Topic**, and **Subtopic** are optional for Courses, with at most one of each.
- Courses can have any number of **Tags**, including none.
- Course classification follows the shared Discipline -> Subject -> Topic -> Subtopic hierarchy.
- Course Discipline selection should provide a clear way to request a new Discipline when the needed
  Discipline is unavailable.
- **Sysadmins** exclusively create and manage Disciplines.
- Course classification supports Course search, filtering, organization, and discovery where applicable.
- A Course Instance may have classification that differs from its Blueprint Course.

### Blueprint Course specifications

- **Blueprint Courses** are reusable course definitions for building **Course Instances**.
- The structure of Blueprint Courses is very similar to GitHub repositories, with the PLE-specific differences described in this guidance.
- Blueprint Courses are a similar concept as LibreTexts' ADAPT alpha courses.
- Blueprint Courses have no **Students**, deadlines, or other teaching-specific delivery settings.
- Blueprint Courses do not contain dates or relative schedules.
- Public Blueprint Courses are visible and reusable by every **Instructor**.
- Blueprint Courses contain only **Library Objects**.
- An **Instructor** can create a new Blueprint Course from an existing Course Instance's reusable structure. The new Blueprint Course records that Course Instance as its source.
- Creating a Blueprint Course from a Course Instance copies its Assessments as Blueprint
  Assessments in the same order.

#### Blueprint Course lifecycle specifications

- Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
- New Blueprint Courses and forks start Private.
- Private Blueprint Courses are visible only to their owning **Instructor**.
- Instructors can develop and use Private Blueprint Courses without publishing them.
- Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
- Making a Blueprint Course Public adds it to the shared Blueprint Course collection.
- Public Blueprint Courses and their Revision history are visible to all **Instructors**.
- Public Blueprint Courses can be adopted to create daughter Course Instances.
- A Public Blueprint Course with no adoptions can return to Private.
- A Public Blueprint Course with one or more adoptions remains Public.
- Blueprint Courses have no separate Draft state.

#### Archived Blueprint Course specifications

- Archived Blueprint Courses are read-only and no longer actively maintained.
- Archived Blueprint Courses and their Revision history remain visible to all **Instructors**.
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
- **Instructors** can see who Starred a Blueprint Course and its Star count.
- Blueprint Course Watches follow the GitHub repository model.
- Watchers are notified about new Blueprint Revisions and other important Blueprint changes.
- Forking or adopting a Blueprint Course does not automatically Star or Watch it.
- Stars and Watches belong to the Blueprint Course across all of its Revisions.

#### Blueprint adoption and incorporation specifications

- Blueprint adoption copies every Assessment from the Blueprint Course into the Course Instance.
- Course Instances record the Blueprint Course Revision Tuple from which they were adopted.
- New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review.
- Routine Blueprint changes should be quick for an **Instructor** to review and incorporate.
- It should be obvious when a Course Instance is based on an older Blueprint Revision.
- The **Instructor** decides which changes to existing Assessments to incorporate.
- Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
- Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.

#### Blueprint Course fork specifications

- An **Instructor** can fork a Public or Archived **Blueprint Course** to create a new Private Blueprint Course.
- A fork is owned by the **Instructor** who created it.
- A fork records the source Blueprint Course Revision Tuple from which it was created.
- Forking a Blueprint Course creates new Blueprint Assessments.
- Published Questions in the new Blueprint Assessments retain the same Question Revision Tuples.
- Question Pools in the new Blueprint Assessments keep their existing Pool IDs. An Instructor
  who wants to change a Pool independently forks it first.
- Forked Question Pools initially contain the same Question Revision Tuples as their source.
- Forked Blueprint Courses develop independently and have their own Blueprint Revisions.
- Changes to a source Blueprint Course are never automatically applied to its forks.
- A Blueprint Course shows its known forks and the **Instructor** who owns each fork.
- PLE should make newer source Revisions easy for the fork owner to discover and review.
- PLE should show newer Revisions of forked Blueprint Courses from their source Blueprint Course.
- The fork owner decides whether to incorporate source changes into the fork.
- PLE should make it easy for the fork owner to incorporate selected source changes.

#### Blueprint Course Change Proposal specifications

- A **Blueprint Course Change Proposal** proposes changes from one Blueprint Course to another.
- An **Instructor** can create a Change Proposal for a Blueprint Course they do not own.
- A Change Proposal records the source Blueprint Course Revision Tuple.
- A Change Proposal records the target Blueprint Course Revision Tuple used for comparison.
- The proposed changes are represented using the canonical Blueprint Course JSON format.
- PLE compares the proposed JSON with the target Blueprint Revision to determine the proposed changes.
- A Change Proposal should present those changes in a human-readable interface rather than requiring
  the receiving **Instructor** to review raw JSON.
- A Change Proposal may include any Blueprint Course content represented in its canonical JSON.
- Changes may include Course names and metadata, Assessment names and settings, Assessment additions
  and removals, and changes to the Library Objects in an Assessment.
- Question content changes belong to the Published Question and are not Blueprint Course changes.
- PLE should present proposed changes in terms meaningful to Instructors rather than as raw JSON changes.
- The receiving **Instructor** can review proposed changes before changing the target Blueprint Course.
- The receiving Instructor decides which proposed changes to accept.
- The receiving Instructor can accept the entire Change Proposal or selected proposed changes.
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
- Comparison should show shared, added, removed, and changed Assessments and Library Objects.
- Comparison should remain useful when Assessment names, order, or structure have changed.
- Comparison visibility follows Blueprint Course visibility rather than fork ownership.

#### Blueprint Course JSON specifications

- Blueprint Courses have a canonical JSON representation for comparison, import, export, and exchange.
- Canonical Blueprint JSON must contain enough information to fully recreate a Blueprint Course.
- Importing exported Blueprint JSON should reproduce the same Blueprint Course content and structure.
- Blueprint JSON contains Blueprint metadata and an ordered list of Blueprint Assessments.
- Blueprint Assessments contain only reusable teaching settings.
- Blueprint Assessments contain ordered **Library Objects**.
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
- Course Instances contain only **Library Objects**.
- Course Instances are visible only to their co-**Instructors** and enrolled **Students**.
- Active Courses are current teaching Course Instances.
- Inactive Courses are past Course Instances and retain Course metadata, including after
  FERPA-sensitive Student data is removed.
- An **Instructor** can create a new **Blueprint Course** from an existing Course Instance's reusable structure. The new Blueprint Course records that Course Instance as its source.
- A new academic term uses a new Course Instance. Rollover is not a separate product model.

#### Blueprint adoption and daughter Course Instances

- **Adoption** connects a Blueprint Course and a Course Instance through either Course creation workflow.
- Creating a new Course Instance from a Blueprint Course establishes an Adoption and increases that Blueprint Course's **Adoption count** by one.
- Creating a new Blueprint Course from an existing Course Instance's reusable structure establishes the originating Course Instance as that Blueprint Course's first Adoption, giving the new Blueprint Course an Adoption count of one.
- A Course Instance created from a Blueprint Course is a **daughter Course Instance** of that Blueprint Course.
- A daughter Course Instance records the parent Blueprint Course Revision Tuple used to create it.
- A daughter Course Instance receives every Assessment from the selected Blueprint Revision.
- Creating a daughter Course Instance copies the Blueprint Course's Assessments, Library Objects,
  and reusable settings.
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
  **Weekly Assignment**, **Unit Review Assignment**, and **Bonus Assignment**.

### Assessment content specifications

- Assessments are organized by their Course and position within its ordered sequence.
- Assessments contain an ordered sequence of Library Objects.
- Each Published Question in an Assessment is identified by its Question Revision Tuple.
- Adding a Question Pool to an Assessment stores a reference to the existing Pool.
- A newly forked Question Pool initially contains the same Question Revision Tuples
  as its source.
- A forked Question Pool can be changed independently without changing its source Question Pool.
- Published Questions and Question Pools remain distinct even though both can occupy positions in an Assessment.
- **Instructors** can add, remove, and reorder Library Objects, subject to the post-issue limits below.
- Assessment Question-order randomization is called **Randomize question order**.
- Before release, an Assessment can be incomplete or inconsistent. Save the current work and
  show specific problems. Require it to pass Release Validation before release.
- Save changes to a Pool even when an unreleased Assessment asks for more Questions than the Pool
  can provide.
- Show the problem and block release until the Instructor adds Published Questions to the Pool,
  lowers the number selected, replaces the Pool, or removes it from the Assessment.
- After an Assessment has been issued to a Student, follow the editing rules below.

#### Assessment content edits after issue

- Once an Assessment has been issued to a Student, only a limited set of content edits should be allowed.
- Instructors can change the points and order of Library Objects.
- Instructors can remove an individual Question from an Assessment's Question Pool only if it has not been issued to any Student in that Assessment.
- If a Question Pool is bad, Instructors can remove the whole Pool from the Assessment and exclude its earned and possible points from every Attempt. Err on the side of caution; fairness to all Students takes precedence.
- I would probably allow completely removing a standalone Question from the Assessment.
- If a Question or Question Pool is completely removed from an Assessment, its earned and possible points must be removed from all Attempts, including existing Attempts, for fairness to all Students.
- Assessment Unrelease remains the destructive reset described below: it deletes all Student Work and permits normal editing before a new release.

### Assessment type specifications

- PLE defines the available Assessment Types.
- Assessment Type describes the pedagogical purpose of an Assessment and provides appropriate defaults.
- Assessment Types are **Weekly Assignment**, **Unit Review Assignment**, **Bonus Assignment**, **Quiz**, and **Exam**.
- **Instructors** select an Assessment Type but cannot create new Assessment Types.
- Blueprint Assessments and Course Instance Assessments use the same Assessment Types.
- **Instructors** can change Assessment settings independently of the defaults for its Type.
- Changing Assessment settings does not change its Assessment Type.
- **Weekly Assignments** give **Students** regular practice applying course ideas outside class.
- Weekly Assignments reinforce current learning and may also introduce new topics.
- Weekly Assignments are designed as practice for learning, not merely as one-time assessments.
- **Unit Review Assignments** provide focused review or study-guide practice using material already covered.
- Unit Review Assignments may be worth a small number of points or a small amount of extra credit.
- Students submit the whole Attempt for Unit Review Assignments, as for every Assessment.
  By default, correct answers are shown immediately after submission.
- **Bonus Assignments** provide optional extra credit.
- Bonus Assignments are worth zero points possible and add earned points directly to the grade.
- **Quizzes** assess understanding of recent material.
- **Exams** are individual assessments associated with scheduled exam periods.
- Exams may use more restrictive Attempt, timing, availability, and feedback settings.
- Quizzes and Exams allow one Assessment Attempt.

### Blueprint Assessment specifications

- A **Blueprint Assessment** is an Assessment in a **Blueprint Course**.
- Blueprint Assessments define reusable Assessment content and teaching settings.
- Blueprint Assessments have an Assessment Type.
- Blueprint Assessments contain ordered **Library Objects**.
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
- Assessment Templates provide defaults for Attempts, timing, scoring, and what Students can see.
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
- Release Validation should check that the Assessment contains at least one Library Object
  and that required settings for those entries are valid.
- The **Instructor** should be able to correct validation problems and run Release Validation again.
- An Assessment can be released only after Release Validation passes.
- Releasing an Assessment makes it available to **Students** according to its dates and access settings.
- Student Work begins when a **Student** starts an Assessment Attempt.

#### Assessment submission defaults

- New Course Instance Assessments default to accepting submissions only through the due date.
- New Course Instance Assessments default to starting new Attempts only through the due date.
- Late work defaults to rejected.

#### Assessment answer and feedback disclosure

- Settings for what Students can see are separate and can be changed independently.
- Use the Blackboard Ultra model of separate availability and timing settings for viewing submissions and correct answers. Optional Question Feedback timing remains undecided.
- Question scores and the total Assessment score are visible as soon as the Attempt is submitted and automatically graded.
- Instructors cannot hide or delay these scores. PLE has no separate score-posting step, and score visibility does not depend on other Students completing the Assessment.
- The correct-answer settings below are defaults. Instructors can change them for each Assessment.
- **Weekly Assignments** and **Bonus Assignments** should rarely show the correct answer.
- Regular and Bonus Assignments show the **Student's** response and whether it was correct or incorrect.
- **Unit Review Assignments** default to showing correct answers immediately after Assessment Attempt
  submission.
- **Quizzes** and **Exams** default to showing correct answers after all **Students** in the Course have completed the Assessment.
- Waiting for all Students to complete a Quiz is a necessary evil. If a Student goes AWOL, the Instructor can change the correct-answer setting; Quizzes require more Instructor attention.
- A Quiz or Exam Attempt is complete when the **Student** submits it or its time limit expires and
  PLE submits it automatically.
- Assessment Attempt completion does not depend on correctness or score.
- Quizzes and Exams do not disclose correct answers until the selected release condition is met.
- Question Feedback is optional; its display timing and relationship to correct-answer visibility are deferred pending review of its use in PLE.

#### Assessment unrelease

- Unreleasing returns a Course Instance Assessment to an unreleased state and deletes its
  Student Work.
- Unreleasing permanently deletes all Student Work for that Assessment.
- Student Work deletion includes Assessment Attempts, saved responses, submissions, and grading outcomes.
- Students can no longer access an unreleased Assessment.
- The Assessment itself, its Questions, settings, and other Instructor-created content remain.
- The Instructor can edit the unreleased Assessment normally after Student Work is deleted.
- Releasing the Assessment again follows the normal Assessment Release Validation process.
- A later release starts with no Student Work or Assessment Attempts from the earlier release.

### Assessment Attempt specifications

- An **Assessment Attempt** is one Student attempt at a Course Instance Assessment.
- Blueprint Assessments do not have Assessment Attempts.
- Question responses are saved as the **Student** works and remain part of the Attempt across browser sessions.
- **Instructors** control the number of permitted Assessment Attempts.
- Weekly Assignments default to unlimited Attempts.
- **Students** can repeat an Assessment as often as its settings allow, including practicing toward
  a perfect score.
- When an Assessment permits multiple Attempts, the highest Assessment Attempt score is used as the
  Student's Assessment score.
- Assessment Attempt submission and grading are fully automatic and require no **Instructor** action.
- Automatic grading does not require a separate Student or **Instructor** grading workflow.

### Assessment response and submission specifications

- The Student submission action submits the whole Assessment Attempt.
- A Question either has a complete saved response or has no saved response.
- PLE saves complete Question responses as the **Student** works.
- The Student can change a saved response while the Assessment Attempt remains open.
- Submitting the Assessment Attempt finalizes all saved Question responses together as Student Work.
- Questions without a saved response remain visibly unanswered when the Attempt is submitted.
- An unanswered Question receives zero credit and counts as incorrect without being sent to the
  Question Backend.
- PLE treats an incomplete Question response as unsaved, although the Question interface may keep the Student's unfinished input while they work.
- Valid MATCH and MULTI-FIB responses can include unanswered parts. Save the answered parts and
  award zero for the unanswered parts.
- A Question Backend may evaluate a response before Assessment submission when needed for its interaction.
- When PLE requests a grading outcome, the Question Backend returns the result in that response.
- The **Student** does not see the grading outcome until the Assessment Attempt is submitted.

### Assessment Attempt timing and expiration specifications

- Each Assessment Attempt has a time limit.
- Each Assessment can contain at most 250 Questions.
- A Question Pool counts as the number of Questions selected from it for the Assessment Question
  limit and default time calculation; selecting 3 of 199 Questions counts as 3.
- The default time limit is 1.5 minutes per Question, rounded up to the nearest whole minute.
- Instructors can override the default time limit up to 12 hours.
- The interface should show the calculated default time limit and let the Instructor change it.
- Time limits must support individual **Students** with accommodations, such as 1.5X or 2X time.
- Student accommodations are applied after the Assessment time limit and may extend that Student's effective time limit
  up to 24 hours.
- Attempt time limits help **Students** develop an accurate sense of expected working speed.
- Assessment Attempts use wall-clock time.
- The server owns the Attempt start and expiration times.
- Attempt time continues while the **Student** is disconnected or the browser is closed.
- A **Student** can reconnect, reload, or use another browser session to resume the same active Attempt.
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

- Blueprint Assessments and Course Instance Assessments assign point values to Library Objects.
- A Question Backend returns an immutable credit fraction for each complete response it evaluates.
- PLE stores the credit fraction as the Question grading outcome.
- PLE stores earned partial credit whether the Assessment awards partial credit or not.
- With partial credit on, use the stored fraction for points. With it off, full credit earns full
  points and smaller fractions earn zero.
- Instructors can change the partial-credit setting for all Attempts, including submitted Attempts.
  Use stored fractions to recalculate scores fairly for all Students.
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
- For a flawed Question, the usual remedy is to set its point value to zero in the Assessment.
- The owning Instructor or a Sysadmin can correct a published Native JSON Question, following the
  Published Question revision rules. An Instructor using someone else's Published Question can set
  its Assessment point value to zero.
- Correcting a published WeBWorK Question requires a new Revision, so setting its Assessment point
  value to zero is the remedy for affected work.
- Score recalculation does not require another Question Backend interaction.
- Score recalculation does not change the stored Question grading outcome.
