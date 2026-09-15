# Ui And Workflow Changes

## Scope

This is a fresh topical implementation-audit inventory. It collects currently open
Human Guidance checklist records relevant to UI and workflow. The bullet text is copied verbatim
from the generated checklist, and its source location is recorded beside it. This report does not
establish exhaustive or disjoint topical coverage; the checklist remains the authority for each
record's status.

The authoritative exhaustive record is the
[generated checklist](../../audits/human_guidance_implementation_checklist.md).

## Evidence updates

- C57 closes the three Question Library Search-landing and return-state rows. `LibraryPage` begins
  with only the Search entry, then starts the result workflow after input. Its session-bound,
  single-use in-document return snapshot restored query, filter, 80 loaded rows, and virtual-list
  position through visible return and browser Back. Accepted one-time compiled-browser evidence also
  confirmed idle no-fetch and changed-session isolation. The temporary harness and screenshots
  remain outside the repository during review; this is not connected HTTP evidence. C58 and C59
  remain open.

- C77--C78 now have accepted source, strict TypeScript, focused projection-test, and compiled
  SolidJS/mock-API browser evidence. A fresh PostgreSQL 17 run also exercised the actual landing
  Store for Regular Assignment plus scheduled, expired unfinished, active resumable,
  Attempt-limit-reached resumable, and late-work-refused resumable states. This closes the direct
  Course-page state and scannable-list rows without claiming a connected HTTP-server run. The
  actual access Store was exercised too, including Type Regular Assignment and active-Attempt
  resume. The pre-start Assessment page presents title, Type, Question count, points possible,
  time limit, previous Attempts, and the Type-specific action; accepted browser evidence covers a
  Quiz label, start action, and Question count, but not the full facts/history projection or a
  connected HTTP-server run, so that broader row stays open.

## Topical inventory

### Interface design -- General interface design

- Instructor and **Sysadmin** workflows should work well in a 1280 by 800 desktop browser viewport.
  - Source: `docs/HUMAN_GUIDANCE.md:163`

- Design around what users need to find and do.
  - Source: `docs/HUMAN_GUIDANCE.md:164`

- Important information should stand out from supporting information.
  - Source: `docs/HUMAN_GUIDANCE.md:165`

- Related information should be visually grouped and aligned.
  - Source: `docs/HUMAN_GUIDANCE.md:166`

- Similar pages should place similar controls in consistent locations.
  - Source: `docs/HUMAN_GUIDANCE.md:167`

- Primary actions should be easy to find and appear near the content or workflow they affect.
  - Source: `docs/HUMAN_GUIDANCE.md:168`

- Avoid scattering related actions across page headers, menus, navigation, and content areas.
  - Source: `docs/HUMAN_GUIDANCE.md:169`

- Optimize large collections for scanning, searching, filtering, and comparison.
  - Source: `docs/HUMAN_GUIDANCE.md:171`

- Show enough useful information at once to support comparison without excessive scrolling.
  - Source: `docs/HUMAN_GUIDANCE.md:172`

- Search and filters should help users quickly narrow large collections.
  - Source: `docs/HUMAN_GUIDANCE.md:173`

- Dense pages should remain easy to scan.
  - Source: `docs/HUMAN_GUIDANCE.md:174`

- Use spacing to separate meaningful groups rather than simply making pages spacious.
  - Source: `docs/HUMAN_GUIDANCE.md:175`

- Prefer alignment, typography, and dividers over unnecessary cards, boxes, borders, and nested containers.
  - Source: `docs/HUMAN_GUIDANCE.md:176`

- Keep the visual design compact, flat, information dense, and consistent across PLE.
  - Source: `docs/HUMAN_GUIDANCE.md:177`

- Dream big on the UI. Choose one visual philosophy and carry it through the entire interface.
  - Source: `docs/HUMAN_GUIDANCE.md:178`

- Use drag-and-drop where it makes reordering faster and more natural.
  - Source: `docs/HUMAN_GUIDANCE.md:179`

- Reordering must also have a precise keyboard-accessible method.
  - Source: `docs/HUMAN_GUIDANCE.md:180`

- Themes should use biome and habitat names, such as Forest, Grassland, Ocean, and Desert.
  - Source: `docs/HUMAN_GUIDANCE.md:181`

- UUIDs should never appear in visible content, navigation URLs, or copyable links.
  - Source: `docs/HUMAN_GUIDANCE.md:182`

- Use [Atkinson Hyperlegible Mono](https://www.brailleinstitute.org/freefont/) for code and other monospace text.
  - Source: `docs/HUMAN_GUIDANCE.md:184`

- Question Backend-rendered content may use its own fonts when needed for correct display.
  - Source: `docs/HUMAN_GUIDANCE.md:186`

- Students should have no upload capabilities. Instructor-created content should use text boxes.
  - Source: `docs/HUMAN_GUIDANCE.md:187`

### Interface design -- User top bar

- Each Product Role has its own home dashboard and navigation.
  - Source: `docs/HUMAN_GUIDANCE.md:204`

- Profile appears at the far right as an icon-only avatar.
  - Source: `docs/HUMAN_GUIDANCE.md:207`

- Clicking the Profile avatar opens the Profile menu.
  - Source: `docs/HUMAN_GUIDANCE.md:208`

- The Profile menu contains Profile settings, account settings, and Sign Out.
  - Source: `docs/HUMAN_GUIDANCE.md:209`

- Sign Out belongs in the Profile menu rather than the main top bar.
  - Source: `docs/HUMAN_GUIDANCE.md:210`

- The Profile avatar uses a generic user avatar until the user selects another avatar.
  - Source: `docs/HUMAN_GUIDANCE.md:211`

- **Students** select avatars from a PLE-provided collection and cannot upload Profile images.
  - Source: `docs/HUMAN_GUIDANCE.md:212`

- Student avatar selection should be visual and playful, similar to choosing a LEGO avatar.
  - Source: `docs/HUMAN_GUIDANCE.md:213`

- **Instructors** and **Sysadmins** may select a provided avatar or add their own Profile image.
  - Source: `docs/HUMAN_GUIDANCE.md:214`

- The current avatar appears consistently anywhere PLE represents that user.
  - Source: `docs/HUMAN_GUIDANCE.md:215`

### Interface design -- Breadcrumbs

- All signed-in users have a permanent breadcrumb row below the top Ribbon.
  - Source: `docs/HUMAN_GUIDANCE.md:222`

- The breadcrumb row remains in the same location and keeps the same space as users navigate.
  - Source: `docs/HUMAN_GUIDANCE.md:223`

- Breadcrumbs show the path from the user's home dashboard to the current page.
  - Source: `docs/HUMAN_GUIDANCE.md:224`

- Each breadcrumb level links back to its corresponding page.
  - Source: `docs/HUMAN_GUIDANCE.md:225`

### Interface design -- Instructor interface

- The Instructor interface should make frequent teaching tasks fast and easy to find.
  - Source: `docs/HUMAN_GUIDANCE.md:233`

- The Instructor menu has **Courses**, **Questions**, and **Assessments** in one dense top bar.
  - Source: `docs/HUMAN_GUIDANCE.md:234`

- All required ribbon choices remain visible even when their collection is empty.
  - Source: `docs/HUMAN_GUIDANCE.md:236`

- Empty collection pages should explain what the collection is for and provide an obvious action to create or add the first item when the user can do so.
  - Source: `docs/HUMAN_GUIDANCE.md:239`

- Similar pages should place similar actions in consistent locations.
  - Source: `docs/HUMAN_GUIDANCE.md:240`

- Instructor pages should be composed around the teaching task rather than collections of padded components.
  - Source: `docs/HUMAN_GUIDANCE.md:241`

- Instructor Course and Assessment lists should be dense and easy to scan, more like a spreadsheet than cards.
  - Source: `docs/HUMAN_GUIDANCE.md:242`

- Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.
  - Source: `docs/HUMAN_GUIDANCE.md:243`
  - Implementation: `AssessmentWorkspaceStudentViewPage`, `assessment_student_view_router`, `PostgresInstructorStudentViewStore`, and `load_instructor_student_view_question_source` now form the server-authorized answer-free, no-write Assessment projection.
  - Accepted evidence: fresh PostgreSQL exercised the real Store through API roles with a nonempty Ready Asset rendition, read-only SQLSTATE `25006`, and zero Student-state writes. Independently reviewed Chromium component evidence covered native and WeBWorK rendering, navigation, disabled controls, stale and error recovery, and no mutation requests using mock transport.
  - Remaining gap: the Chromium evidence was not connected or live-stack acceptance; the unchanged full server compile remains blocked in the AWS dependency graph; and production iMathAS Student View integration remains deferred outside the pilot. Independent final server source review passed, but it does not establish runtime behavior.

- The **Courses** ribbon must include: My Blueprint Courses, My Active Courses, My Inactive Courses, Search Public Blueprint Courses.
  - Source: `docs/HUMAN_GUIDANCE.md:247`

- The Course Editor should show the Course structure and its ordered Assessments without showing every Question at once.
  - Source: `docs/HUMAN_GUIDANCE.md:249`

- Selecting an Assessment in the Course Editor opens that Assessment for editing.
  - Source: `docs/HUMAN_GUIDANCE.md:250`

- Assessment content and Assessment properties should remain separate editing tasks.
  - Source: `docs/HUMAN_GUIDANCE.md:251`

- My Active Courses and My Inactive Courses should both be available from the Courses area.
  - Source: `docs/HUMAN_GUIDANCE.md:252`

- **Search Public Blueprint Courses** helps Instructors find a Blueprint Course they already have in mind.
  - Source: `docs/HUMAN_GUIDANCE.md:257`

- Public Blueprint Course search should support quickly narrowing a large collection.
  - Source: `docs/HUMAN_GUIDANCE.md:258`

- Blueprint Course editing should follow Course Editor -> Blueprint Assessment Editor.
  - Source: `docs/HUMAN_GUIDANCE.md:260`

- Selecting a Blueprint Assessment in the Course Editor opens the editor for that Blueprint Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:262`

- Only the selected Blueprint Assessment's Questions should appear in its editor.
  - Source: `docs/HUMAN_GUIDANCE.md:263`

- **Blueprint Assessment Question Editor**: Selects, adds, removes, and orders Questions in a Blueprint Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:264`

- **Blueprint Assessment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see.
  - Source: `docs/HUMAN_GUIDANCE.md:265`

- Blueprint Courses follow the lifecycle **Private -> Public -> Archived**.
  - Source: `docs/HUMAN_GUIDANCE.md:267`

- New and forked Blueprint Courses start **Private**.
  - Source: `docs/HUMAN_GUIDANCE.md:268`

- Private Blueprint Courses are visible only to their owner.
  - Source: `docs/HUMAN_GUIDANCE.md:269`

- Instructors may develop and use Private Blueprint Courses without publishing them.
  - Source: `docs/HUMAN_GUIDANCE.md:270`

- Making a Blueprint Course **Public** adds it to the shared Blueprint Course collection.
  - Source: `docs/HUMAN_GUIDANCE.md:271`

- A Public Blueprint Course with no adoptions may return to **Private**.
  - Source: `docs/HUMAN_GUIDANCE.md:272`

- A Public Blueprint Course with one or more adoptions remains **Public**.
  - Source: `docs/HUMAN_GUIDANCE.md:273`

- Instructors may fork a Public Blueprint Course to continue development privately.
  - Source: `docs/HUMAN_GUIDANCE.md:275`

- Blueprint Courses do not have a separate Draft state.
  - Source: `docs/HUMAN_GUIDANCE.md:276`

- **My Active Courses** should emphasize Course Instances the Instructor is currently teaching.
  - Source: `docs/HUMAN_GUIDANCE.md:280`

- Active Course Instances should make upcoming Assessments and important course activity easy to find.
  - Source: `docs/HUMAN_GUIDANCE.md:281`

- **My Inactive Courses** should keep past Course Instances available without competing with active Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:282`

- Creating a Course Instance from a Blueprint Course preserves its Assessments, Questions, pools, and settings.
  - Source: `docs/HUMAN_GUIDANCE.md:283`

- Assessments created from a Blueprint Course start unreleased with dates unset.
  - Source: `docs/HUMAN_GUIDANCE.md:284`

- A Course Instance represents one teaching period and remains Active for at most six months from
  creation.
  - Source: `docs/HUMAN_GUIDANCE.md:285`

- Course banners use a 5:1 aspect ratio.
  - Source: `docs/HUMAN_GUIDANCE.md:287`

- 1280 by 256 pixels is the recommended Course banner authoring size.
  - Source: `docs/HUMAN_GUIDANCE.md:288`

- Higher-resolution 5:1 Course banner images are supported.
  - Source: `docs/HUMAN_GUIDANCE.md:289`

- PLE responsively scales Course banners while preserving their aspect ratio.
  - Source: `docs/HUMAN_GUIDANCE.md:290`

- Course banners appear as small centered banners rather than full-width page heroes.
  - Source: `docs/HUMAN_GUIDANCE.md:291`

- Course Instance Assessments have two editors:
  - Source: `docs/HUMAN_GUIDANCE.md:293`

- **Assessment Question Editor**: Selects, adds, removes, and orders Questions in an Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:294`

- **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and what **Students** can see.
  - Source: `docs/HUMAN_GUIDANCE.md:295`

- The **Questions** ribbon must include: My Questions, My Draft Questions, Starred, Watched, Search Question Library, Browse Question Library.
  - Source: `docs/HUMAN_GUIDANCE.md:299`

- **My Questions** should make the Instructor's Published Questions easy to find and manage.
  - Source: `docs/HUMAN_GUIDANCE.md:300`

- **Starred** should provide a quick personal collection of Questions the Instructor wants to keep handy.
  - Source: `docs/HUMAN_GUIDANCE.md:302`

- **Watched** should help Instructors follow Questions where changes or activity matter to them.
  - Source: `docs/HUMAN_GUIDANCE.md:303`

- Search should support Google-like syntax for more precise queries.
  - Source: `docs/HUMAN_GUIDANCE.md:317`

- Quoted text should search for an exact phrase.
  - Source: `docs/HUMAN_GUIDANCE.md:318`

- A minus sign should exclude matching terms.
  - Source: `docs/HUMAN_GUIDANCE.md:319`

- Search should support PubMed-like field tags such as `topic:genetics`.
  - Source: `docs/HUMAN_GUIDANCE.md:320`

- Field tags should use PLE concepts and vocabulary.
  - Source: `docs/HUMAN_GUIDANCE.md:321`

- Useful fields may include subject, topic, tags, Question Type, and author.
  - Source: `docs/HUMAN_GUIDANCE.md:322`

- Simple and advanced searches should use the same search box.
  - Source: `docs/HUMAN_GUIDANCE.md:323`

- The interface should make useful search syntax discoverable when needed.
  - Source: `docs/HUMAN_GUIDANCE.md:325`

- Search syntax should help expert users quickly narrow a very large Question Library.
  - Source: `docs/HUMAN_GUIDANCE.md:326`

- **Browse Question Library** helps Instructors explore Questions without knowing what to search for.
  - Source: `docs/HUMAN_GUIDANCE.md:333`

- Browse should help Instructors understand what the Question Library contains.
  - Source: `docs/HUMAN_GUIDANCE.md:334`

- Browse should emphasize subjects, topics, tags, Question Types, and other useful groupings.
  - Source: `docs/HUMAN_GUIDANCE.md:335`

- Browse should make moving from broad subjects to narrower topics easy.
  - Source: `docs/HUMAN_GUIDANCE.md:336`

- Browse should show useful counts where they help Instructors choose where to explore.
  - Source: `docs/HUMAN_GUIDANCE.md:337`

- Browse results should use the same dense Question presentation used by Search where practical.
  - Source: `docs/HUMAN_GUIDANCE.md:338`

- Instructors should be able to move from browsing into a more focused search.
  - Source: `docs/HUMAN_GUIDANCE.md:339`

- Search and Browse are different paths into the same **Question Library**.
  - Source: `docs/HUMAN_GUIDANCE.md:340`

- The **Assessments** ribbon must include: Assessments Due Soon, My Assessment Templates.
  - Source: `docs/HUMAN_GUIDANCE.md:344`

- **Assessments Due Soon** should emphasize Assessments that may need the Instructor's attention.
  - Source: `docs/HUMAN_GUIDANCE.md:345`

- Assessment lists should make Course, release status, due date, and other important state easy to scan.
  - Source: `docs/HUMAN_GUIDANCE.md:346`

- **My Assessment Templates** should emphasize reusable Assessment design rather than Course activity.
  - Source: `docs/HUMAN_GUIDANCE.md:347`

- Assessment editing has two editors:
  - Source: `docs/HUMAN_GUIDANCE.md:348`

- **Assessment Question Editor**: Selects, adds, removes, and orders Questions.
  - Source: `docs/HUMAN_GUIDANCE.md:349`

- **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and other Assessment settings.
  - Source: `docs/HUMAN_GUIDANCE.md:350`

- The two Assessment editors should remain clearly distinct.
  - Source: `docs/HUMAN_GUIDANCE.md:351`

- The Assessment Question Editor should make Question order easy to understand at a glance.
  - Source: `docs/HUMAN_GUIDANCE.md:352`

- Adding Questions should provide direct paths to Search and Browse Question Library.
  - Source: `docs/HUMAN_GUIDANCE.md:353`

- Instructors should be able to inspect a Question before adding it to an Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:354`

- Assessment Properties should group related settings so important settings are easy to find.
  - Source: `docs/HUMAN_GUIDANCE.md:355`

- Instructors can randomize Question order for an Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:356`

- Answer-choice randomization belongs to the Question, not the Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:357`

- **Assessments Due Soon** shows upcoming Assessments across the Courses an **Instructor** teaches.
  - Source: `docs/HUMAN_GUIDANCE.md:358`

- Assessments Due Soon shows the Course and due time for each Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:359`

- Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
  - Source: `docs/HUMAN_GUIDANCE.md:363`
  - Current implementation: Assessment Unrelease and Archive Blueprint Course have Instructor controls; Archive Published Question has a browser API but no current Danger Zone interface.
  - Remaining gap: expose the Published Question archive workflow rather than treating its transport contract as a usable action.

- Archive actions should explain the effect on shared availability and require a clear confirmation.
  - Source: `docs/HUMAN_GUIDANCE.md:367`
  - Current implementation: Archive Blueprint Course explains removal from new selection and requires its long name.
  - Remaining gap: no current Archive Published Question interface provides the corresponding availability explanation and confirmation.

### Interface design -- Student interface

- **Coursework** is the Student-facing collective term for Regular Assignments, Practice Question
  Assignments, Bonus Assignments, Quizzes, and Exams.
  - Source: `docs/HUMAN_GUIDANCE.md:373`

- Student-facing interfaces should use the specific Assessment Type when referring to an individual item rather than calling it an Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:375`

- The Student Ribbon should use familiar Student language rather than internal PLE terms such as Assessment.
  - Source: `docs/HUMAN_GUIDANCE.md:376`

- Coursework lists may provide filters for **Regular Assignments**, **Practice Question Assignments**,
  **Bonus Assignments**, **Quizzes**, and **Exams**.
  - Source: `docs/HUMAN_GUIDANCE.md:377`

- Each Coursework item should clearly show its Assessment Type using its label and Type icon.
  - Source: `docs/HUMAN_GUIDANCE.md:379`

- The Student menu is simpler than the Instructor menu.
  - Source: `docs/HUMAN_GUIDANCE.md:381`

- Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.
  - Source: `docs/HUMAN_GUIDANCE.md:382`

- Every Student browser action should be usable with the keyboard alone.
  - Source: `docs/HUMAN_GUIDANCE.md:383`

- Student navigation and pages should contain only Student interfaces and capabilities.
  - Source: `docs/HUMAN_GUIDANCE.md:385`

- Before starting Coursework, Students should see its title, Type, Question count, points possible, time limit, and previous Attempts.
  - Source: `docs/HUMAN_GUIDANCE.md:390`

- The complete Student Ribbon task layout does not have a locked-in design yet.
  - Source: `docs/HUMAN_GUIDANCE.md:401`

### Interface design -- Sysadmin interface

- The Sysadmin menu should make Accounts, Instructors, Courses, and system configuration easy to find.
  - Source: `docs/HUMAN_GUIDANCE.md:406`

- Sysadmins should be able to find users quickly by name or email.
  - Source: `docs/HUMAN_GUIDANCE.md:407`

- Account lists should support searching, filtering, and scanning large numbers of users.
  - Source: `docs/HUMAN_GUIDANCE.md:408`

- User pages should clearly show role, account status, and other important administrative information.
  - Source: `docs/HUMAN_GUIDANCE.md:409`

- Sysadmins approve Instructors before they receive Instructor capabilities.
  - Source: `docs/HUMAN_GUIDANCE.md:411`

- Instructor approval status should be easy to find and change.
  - Source: `docs/HUMAN_GUIDANCE.md:412`

- Sysadmins should be able to find and inspect Courses across the installation.
  - Source: `docs/HUMAN_GUIDANCE.md:413`

- Course administration should show the Instructor and important Course status information.
  - Source: `docs/HUMAN_GUIDANCE.md:414`

- Sysadmins should manage Courses through Sysadmin interfaces and capabilities.
  - Source: `docs/HUMAN_GUIDANCE.md:415`

- System-wide settings should have their own area, separate from user and Course administration.
  - Source: `docs/HUMAN_GUIDANCE.md:416`

- High-consequence administrative actions should have a visually distinct area.
  - Source: `docs/HUMAN_GUIDANCE.md:419`

- Confirmation for destructive actions should clearly state what will happen.
  - Source: `docs/HUMAN_GUIDANCE.md:420`

- The complete Sysadmin Ribbon task layout does not have a locked-in design yet.
  - Source: `docs/HUMAN_GUIDANCE.md:421`

### Courses

- **Courses** organize reusable teaching content and its delivery to **Students**.
  - Source: `docs/HUMAN_GUIDANCE.md:685`

- PLE has two Course forms: **Blueprint Courses** and **Course Instances**.
  - Source: `docs/HUMAN_GUIDANCE.md:686`

- **Blueprint Courses** provide reusable course designs for creating Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:687`

- Course Instances may be created from a Blueprint Course or started empty.
  - Source: `docs/HUMAN_GUIDANCE.md:688`

- A Course can have multiple co-**Instructors** with equal teaching authority.
  - Source: `docs/HUMAN_GUIDANCE.md:689`

- **Sysadmins** can create Courses, but **Instructors** teach them.
  - Source: `docs/HUMAN_GUIDANCE.md:690`

- Creating a Course Instance establishes its first Instructor membership but does not give that Instructor greater Course authority than later co-Instructors.
  - Source: `docs/HUMAN_GUIDANCE.md:692`

### Courses -- Blueprint Courses

- Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
  - Source: `docs/HUMAN_GUIDANCE.md:701`

- Blueprint Courses contain only **Published Questions** and published **Question Pools**.
  - Source: `docs/HUMAN_GUIDANCE.md:702`

- An **Instructor** may deliberately publish an existing Course Instance structure as a new Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:703`

- Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
  - Source: `docs/HUMAN_GUIDANCE.md:707`

- New Blueprint Courses and forks start Private.
  - Source: `docs/HUMAN_GUIDANCE.md:708`

- Private Blueprint Courses are visible only to their owning **Instructor**.
  - Source: `docs/HUMAN_GUIDANCE.md:709`

- Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
  - Source: `docs/HUMAN_GUIDANCE.md:710`

- Public Blueprint Courses can be adopted to create daughter Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:712`

- Archived Blueprint Courses are read-only and no longer actively maintained.
  - Source: `docs/HUMAN_GUIDANCE.md:713`

- Archived Blueprint Courses remain visible by every vetted **Instructor**.
  - Source: `docs/HUMAN_GUIDANCE.md:714`

- Archived Blueprint Courses are excluded from normal search results unless the search explicitly includes them.
  - Source: `docs/HUMAN_GUIDANCE.md:715`

- Archived Blueprint Courses can be forked but not adopted.
  - Source: `docs/HUMAN_GUIDANCE.md:717`

- The owning **Instructor** can return an Archived Blueprint Course to Public before adopting it again.
  - Source: `docs/HUMAN_GUIDANCE.md:718`

- Other **Instructors** can fork an Archived Blueprint Course to create a new Private Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:719`

- **Instructors** can Star or Watch Public and Archived Blueprint Courses.
  - Source: `docs/HUMAN_GUIDANCE.md:735`

- A Star is a visible endorsement and helps **Instructors** save useful Blueprint Courses.
  - Source: `docs/HUMAN_GUIDANCE.md:736`

- Vetted **Instructors** can see who Starred a Blueprint Course and its Star count.
  - Source: `docs/HUMAN_GUIDANCE.md:737`

- Watching a Blueprint Course is private.
  - Source: `docs/HUMAN_GUIDANCE.md:738`

- Watchers are notified about new Blueprint Revisions and other important Blueprint changes.
  - Source: `docs/HUMAN_GUIDANCE.md:739`

- Forking or adopting a Blueprint Course does not automatically Star or Watch it.
  - Source: `docs/HUMAN_GUIDANCE.md:740`

- Stars and Watches belong to the Blueprint Course across all of its Revisions.
  - Source: `docs/HUMAN_GUIDANCE.md:741`

- New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
  - Source: `docs/HUMAN_GUIDANCE.md:747`

- Routine Blueprint updates should be quick for an **Instructor** to review and approve.
  - Source: `docs/HUMAN_GUIDANCE.md:748`

- It should be obvious when a Course Instance is using an older Blueprint Revision.
  - Source: `docs/HUMAN_GUIDANCE.md:749`

- Changes to existing Assessments follow the Blueprint Revision update workflow.
  - Source: `docs/HUMAN_GUIDANCE.md:750`

- Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:751`

- Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Source: `docs/HUMAN_GUIDANCE.md:752`

- An **Instructor** can fork a **Blueprint Course** to create a new independent Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:756`

- A fork records the source Blueprint Course and Blueprint Revision from which it was created.
  - Source: `docs/HUMAN_GUIDANCE.md:757`

- Forked Blueprint Courses develop independently and have their own Blueprint Revisions.
  - Source: `docs/HUMAN_GUIDANCE.md:758`

- Changes to a source Blueprint Course are never automatically applied to its forks.
  - Source: `docs/HUMAN_GUIDANCE.md:759`

- A fork should make newer changes from its source Blueprint Course easy to discover and review.
  - Source: `docs/HUMAN_GUIDANCE.md:760`

- An **Instructor** can selectively bring changes from a source Blueprint Course into their fork.
  - Source: `docs/HUMAN_GUIDANCE.md:761`

- An **Instructor** can create a **Blueprint Course Change Proposal** to propose changes to another Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:762`

- A Change Proposal shows added, removed, and changed Assessments and Question content.
  - Source: `docs/HUMAN_GUIDANCE.md:763`

- The receiving **Instructor** decides which proposed changes to accept.
  - Source: `docs/HUMAN_GUIDANCE.md:764`

- Accepted changes create a new Blueprint Revision of the receiving Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:765`

- Change Proposals never directly change daughter Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:766`

- Daughter Course Instances receive accepted changes through the normal Blueprint update workflow.
  - Source: `docs/HUMAN_GUIDANCE.md:767`

- Blueprint Courses have a canonical JSON representation for comparison, import, export, and exchange.
  - Source: `docs/HUMAN_GUIDANCE.md:771`

- Canonical Blueprint JSON must contain enough information to fully recreate a Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:772`

- Importing exported Blueprint JSON should reproduce the same Blueprint Course content and structure.
  - Source: `docs/HUMAN_GUIDANCE.md:773`

- Blueprint JSON contains Blueprint metadata and an ordered list of Blueprint Assessments.
  - Source: `docs/HUMAN_GUIDANCE.md:774`

- Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
  - Source: `docs/HUMAN_GUIDANCE.md:776`

- Blueprint Revisions can be compared through their canonical JSON representations.
  - Source: `docs/HUMAN_GUIDANCE.md:778`

- Blueprint Course Change Proposals use canonical JSON to identify changes between Blueprint Revisions.
  - Source: `docs/HUMAN_GUIDANCE.md:779`

- Canonical Blueprint JSON is the complete exchange format, not the primary persistence model.
  - Source: `docs/HUMAN_GUIDANCE.md:781`

### Courses -- Course Instances

- An **Instructor** can create a Course Instance from a Public Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:787`

- **Instructors** can also create a new empty Course Instance without a parent Blueprint Course.
  - Source: `docs/HUMAN_GUIDANCE.md:788`

- Course Instances have **Students**, deadlines, releases, and other delivery-specific settings.
  - Source: `docs/HUMAN_GUIDANCE.md:789`

- Course Instances contain only **Published Questions** and published **Question Pools**.
  - Source: `docs/HUMAN_GUIDANCE.md:790`

- Active Courses are current teaching Course Instances.
  - Source: `docs/HUMAN_GUIDANCE.md:792`

- Inactive Courses are past Course Instances and retain Course metadata, including after
  FERPA-sensitive Student data is removed.
  - Source: `docs/HUMAN_GUIDANCE.md:793`

- An **Instructor** may deliberately publish reusable Course Instance structure as a new **Blueprint Course**.
  - Source: `docs/HUMAN_GUIDANCE.md:795`

- Creating a Course Instance from a Blueprint Course copies its Assessments, Questions, Question Pools, and reusable settings.
  - Source: `docs/HUMAN_GUIDANCE.md:806`

- It should be obvious when a daughter Course Instance is using an older Blueprint Revision.
  - Source: `docs/HUMAN_GUIDANCE.md:810`

### Courses -- Course names

- Short names are for compact navigation and should stay under about 16 characters when practical.
  - Source: `docs/HUMAN_GUIDANCE.md:819`
