# Human guidance

<!-- VENDORED HEADER: START -->
Record the durable guidance Neil Voss states, or approves for preservation here, in his own words:
first person or close paraphrase, one to three lines per bullet. Material he supplies as a source
may inform [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) once it is settled, and an entry of uncertain
origin belongs there too. Rules: [REPO_STYLE.md](REPO_STYLE.md).
[PROPAGATED HEADER - ENTRIES BELOW ARE YOURS]
<!-- VENDORED HEADER: END -->

This file contains terse owner guidance. Engineering interpretation belongs in
[DESIGN_DECISIONS.md](DESIGN_DECISIONS.md), the active plans, or focused technical documents.

## Guidance Format

bullet points start with a subject? would be more clear than action verb. I dunno

## Agent guidance

- Follow [AGENTS.md](../AGENTS.md) and the repository style documents.
- Choose the robust, clean methodology and keep moving while the next safe task is clear.
- Be efficient with time. Agents and tokens are cheap; wall time is not.
- Break hard work into small independently completable tasks with one owner and one verification.
- Prefer positive prompts that state the intended action directly.
- Avoid overly strict requirements and arbitrary numeric, timing, byte, or pixel equivalence gates.

## Development philosophy

- Keep every source file below 1000 lines. Split complete capabilities into focused modules.
- PLE is pre-production with no users or durable production data. Improve the design directly.
- use readable `snake_case` whenever possible; see docs/NAMING_CONVENTIONS.md for details
- Focus on adaptability so the software can evolve as requirements and insights change.
- Use the latest dependency versions because security bugs are continually fixed.
- When measured interface behavior is slow, consider moving the hot path to Rust/WebAssembly.
- The poliished Live Demo is top priority, see docs/LIVE_DEMO_SPEC.md
- There are three major user types: Sysadmins, Instructors, and Students.
  - exceptions course observers, student observers, and graders
- Use one PLE installation with no institution or tenant boundaries.
- Keep PLE accounts global within that installation and use passwordless email authentication.
- Email is not configured for the live demo yet; use the visible seeded-role entry for demo access.
- Treat project images and simulated live-stack data as disposable acceptance infrastructure.
- Use `./run_live_demo.sh` as the normal local-stack entry point. For direct controller
  diagnostics, use `source source_me.sh && .venv/bin/python local_stack.py`.

## Interface philosophy

- Push harder on visual design. Make the interface less bubbly and reduce excessive padding.
- Use biome and habitat names for themes, and remove names whose themes look substantially alike.
- Never show UUIDs in visible page content, navigation URLs, or copyable links.

## Data philosophy

- Keep answers, keys, grading, and correctness decisions on the server; out of reach of students.
- Keep public evidence separate from private, answer-bearing, identifying, or radioactive FERPA material.
- Users login only with passkey or email code; no passwords. Higher seecurity Exceptions will be considered for Sysadmin accounts.
- Use human-readable titles and identifiers wherever people must recognize, copy, or enter them.
- Student course data falls under FERPA; treat it as radioactive.
- A Sysadmin does not receive general access to FERPA course records.
- Scope FERPA access through exact course membership and Student ownership.
- Losing a passkey returns the user to email sign-in; do not add a separate recovery mode.
- Collect student data reluctantly, use it deliberately, and purge it predictably.
- Course Instance data Default to notice after 30 days, archive after 100 days, and permanent deletion after 365 days.
- Keep course-owned assignment definitions when Student records are archived or deleted.

## Question content philosophy

- Make problem sharing, discovery, and reuse a high-priority Instructor workflow.
- All published questions are public to vetted instructors. By published I am mean as part of the global question corpus.
- All questions in assignments are part of the global question corpus.
- Students cannot see the question corpus. They only see questions in their assignments.
- Question are subject agnostic. Questions must be properly tagged, but all are part of the same corpus.
- Draft questions are kept private until publication so unfinished
  material does not reduce shared-catalog quality.
- Draft questions must go through a validation process before being added to the corpus
- Questions are strictly and deterministically automated; do not add manual grading.
- Support MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT flat-question families.
- Use versioned PLE flat-question JSON as the canonical machine format for simple static questions.
- Treat QTI as import, export, and archival interchange rather than the internal source model.
- Use one copyable Crockford Base32 Question ID in the form `AAA-BBBB` for published questions.
- Published questions are still discoverable even when used by a private Course Instance.
- use a GitHub-like question stewardship model
- Published Questions maintain version history, so updates can be propagated to other courses
- Published Questions should have a limit to the amount of change allowed, to avoid trolling or completely changing the content
- Published Questions can be forked and edited by any instructor
- Published Questions can be starred, watched just like Github
- The forks of Published Questions can be viewed by other instructors
- The anonymized counts for every Published Question are maintained # attempts, # correct, for certain types of flat questions # times each choices was selected.
- Editing happens in the Instructor's private fork draft. Publication validation is required before the fork joins the
    corpus.
- Assignments and grading evidence pin an exact version. A newer version becomes an available controlled update-it never silently changes issued or graded work. Not sure if a security or major flaw override makes sense here. Maybe overrides are only approved by sysadmin?
- Star = favorite/visible endorsement. Every vetted Instructor can see the star count and which vetted Instructors starred the question.
- Watch = subscription. It drives the watching Instructor's in-app notifications for versions, forks, improvement threads, and impact notices; the watch list remains private unless you later choose otherwise.
- Students and anonymous users see neither Instructor identity list nor watch state.
- Statistics are version-specific first: accepted graded attempt count, correct count, and eligible choice counts. Privacy-safe question-level rollups may combine versions only when clearly labeled and disclosure thresholds are met.

## Course content philosophy

- Courses Instances are built from Blueprint courses
- Blueprint courses are reusable course definition and serve as blueprints for building courses
- Blueprint courses have no students enrolled and no deadlines
- Blueprint courses are the same concept as LibreTexts' ADAPT alpha courses
- When a blueprint courses has a new assignment added, it is added to daughter courses as unreleased
- A Course Instance is a course created from a Blueprint Course.
- Course Instance have students, deadlines, releases, and other delivery-specific settings.
- Blueprint Courses: visible and reusable by every vetted Instructor.
- Course Instances: visible only to their current co-Instructors and enrolled Students, because they contain delivery
  choices and FERPA-bearing activity.
- Blueprint courses and Course Instances can only contain Published questions
- All course instances have a parent Blueprint courses
- Reuse path: let an Instructor deliberately publish a Course Instance's reusable structure as a new Blueprint Course or propose controlled updates to its parent.
- Let a course have multiple co-Instructors with equal teaching authority for that course.
- Sysadmins can create coures, but Instructors teach them. Every course must have an assigned Instructor that owns the course.

## Sysadmin philosophy

- Sysadmin must be a god-level account
  - vetting instructors and creating accounts
  - helping non-tech instructors fix their courses, including students and content
- The human developer, Dr. Neil Voss, is the current Sysadmin and is also an Instructor.
- Approve every Instructor manually after validating that they are a real person.
- A Sysadmin does not receive general access to FERPA course records.

## Instructor philosophy

- All vetted instructors are equal
- Instructor accounts are created once they instructor's real identity is vetted by a sysadmin
- Instructors can browse and search the global question corpus
- Instructors can browse the question content of all Blueprint courses
- Design Instructor and Sysadmin workflows for a 1280 by 800 desktop 16:10 aspect browser viewport.
- Compose pages around the teaching task, not a collection of individually padded components.
- Instructors login only with passkey or email code; no passwords.
- Give Instructors a clearly labeled, answer-free Student view without changing their identity.
- Let an Instructor upload a small centered course banner and select a three-color theme.
- Keep Blueprints as personal reusable assignments and Alpha curricula as shared curricula.
- Give every approved Instructor the same product capabilities; course membership determines which
  course records each Instructor may use.

## Student philosophy

- Design Student workflows for laptop, portrait tablet, narrow-phone, and square displays.
- Make every Student browser action usable with the keyboard alone.
- Students login only with passkey or email code; no passwords.
- Collect student data reluctantly, use it deliberately, and purge it predictably.

## Course observers, student observers, and graders philosophy

- Both observers are read-only participants
- Course observers can see assignments and questions for a course and which students have completed the assignments
- Course observers do not see scores
- Students observers can see everything about a particular student, PLE will assume FERPA rights to the student have been waived.
- Graders are not needed right now, because we do not have manual grading
- Keep course authorization adaptable for future Grader and Course Observer relationships. Give a
  Course Observer anonymous aggregate grades without Student-level FERPA information.
