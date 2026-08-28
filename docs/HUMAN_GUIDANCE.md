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

## Development priorities

- Keep every source file below 1000 lines. Split complete capabilities into focused modules.
- PLE is pre-production with no users or durable production data. Improve the design directly.
- Focus on adaptability so the software can evolve as requirements and insights change.
- Use the latest dependency versions because security bugs are continually fixed.
- When measured behavior is slow, consider moving the hot path to Rust or WebAssembly.

## Teaching priorities

- Make the Fall 2026 start-to-finish teaching workflow the priority over unrelated release breadth.
- Make the demo feel like returning to real teaching, with recognizable courses and learner work.
- Keep PLE question-agnostic. Biology questions are content, not engine policy.
- Keep question workflows strictly and deterministically automated; do not add manual grading.
- Keep answers, keys, grading, and correctness decisions on the server.
- Keep public evidence separate from private, answer-bearing, identifying, or FERPA material.

## Interface priorities

- Design Instructor and Sysadmin workflows for a 1280 by 800 desktop browser viewport.
- Design Student workflows for laptop, portrait tablet, narrow-phone, and square displays.
- Compose pages around the teaching task, not a collection of individually padded components.
- Push harder on visual design. Make the interface less bubbly and reduce excessive padding.
- Do not reserve scarce teaching-workspace height for a persistent slogan footer.
- Make every Student browser action usable with the keyboard alone.
- Give Instructors a clearly labeled, answer-free Student view without changing their identity.
- Use Blackboard Original course themes as the model for course appearance.
- Let an Instructor upload a small centered course banner and select a three-color theme.
- Use biome and habitat names for themes, and remove names whose themes look substantially alike.

## Course and content choices

- Use ordinary teaching-course names in product surfaces; keep installer recipe names internal.
- Keep Blueprints as personal reusable assignments and Alpha curricula as shared curricula.
- Use human-readable titles and identifiers wherever people must recognize, copy, or enter them.
- Never show UUIDs in visible page content, navigation URLs, or copyable links.
- Use one copyable Crockford Base32 Question ID in the form `AAA-BBBB` for published questions.
- Use versioned PLE flat-question JSON as the canonical machine format for simple static questions.
- Treat QTI as import, export, and archival interchange rather than the internal source model.
- Support MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT flat-question families.
- Four Chapter 1 questions per course are enough for the first content release.
- Use one WeBWorK MC, one WeBWorK MATCH, one flat-JSON MC, and one flat-JSON MATCH per chapter.

## Accounts and records

- The only human roles are Sysadmin, Instructor, and Student.
- I am the current Sysadmin and also an Instructor.
- Approve every Instructor manually after validating that they are a real person.
- Student course data falls under FERPA; treat it as radioactive.
- A Sysadmin does not receive general access to FERPA course records.
- Keep the public-asset publisher as a service identity, never a human role.
- Keep PLE accounts independent of institutions and use passwordless email authentication.
- Losing a passkey returns the user to email sign-in; do not add a separate recovery mode.
- Email is not configured for the live demo yet; use the visible seeded-role entry for demo access.
- Collect student data reluctantly, use it deliberately, and purge it predictably.

## Retention and local work

- Default to notice after 30 days, archive after 100 days, and permanent deletion after 365 days.
- Keep tenant-owned assignment definitions when Student records are archived or deleted.
- Podman is normally running on my machine.
- Treat project images and simulated live-stack data as disposable acceptance infrastructure.
- Use `./run_live_demo.sh` as the normal local-stack entry point. For direct controller
  diagnostics, use `source source_me.sh && .venv/bin/python local_stack.py`.

## Agent guidance

- Follow [AGENTS.md](../AGENTS.md) and the repository style documents.
- Choose the robust, clean methodology and keep moving while the next safe task is clear.
- Be efficient with time. Agents and tokens are cheap; wall time is not.
- Break hard work into small independently completable tasks with one owner and one verification.
- Prefer positive prompts that state the intended action directly.
- Avoid overly strict requirements and arbitrary numeric, timing, byte, or pixel equivalence gates.
