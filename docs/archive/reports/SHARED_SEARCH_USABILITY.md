# Shared search usability review

## Method and scope

This is an independent expert cognitive walkthrough for M15, rather than a participant study.
It evaluates whether an Instructor can recognize an available action, predict its result, take it,
and recognize completion while preparing or finding teaching content in a desktop browser. It does
not establish task times, satisfaction, or usability for the Instructor population.

The review uses the five task paths specified in the approved
[shared-search plan](../read-docs-active-plans-active-shared-sea-peaceful-mountain.md). The
method follows the local `human-interact-expert` guidance: it records each action, the information
visible when the action is made, recovery, and observable completion. Keyboard, focus, narrow
screen, and automated accessibility coverage belong to the companion accessibility review; this
inspection did not represent that as a complete WCAG conformance claim.

The user is an Instructor who understands Questions, Pools, and course content, but does not need
to know field-search syntax before seeing it. Completion means that a relevant row or detail page
is visible and the original exploration state remains available.

The acceptance rationale uses W3C's explanatory guidance for
[Keyboard](https://www.w3.org/WAI/WCAG22/Understanding/keyboard.html),
[Focus Visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html), and
[Reflow](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html), checked October 5, 2026.

## Live walkthrough evidence

The current-source Live Demo at `https://localhost:8292` was opened as the built-in Instructor
with its trusted local gateway certificate. The completed actions used the normal rendered UI and
created no content or database state. Temporary drivers are retained until M16 classifies and
removes ephemeral checks:

- `tests/_temp/m15_usability_walkthrough.mjs` - known Question path.
- `tests/_temp/m15_browse_probe.mjs` - Browse classification path.
- `tests/_temp/m15_library_pool_probe.mjs` - Pool and membership paths.
- Screenshots: `/tmp/ple_m15_task1_field_query.png`,
  `/tmp/ple_m15_task2_browse_path.png`, `/tmp/ple_m15_task3_pool_detail.png`, and
  `/tmp/ple_m15_task4_membership_all.png`.

| Task | Actual action and visible cue | Result and recovery evidence | Status |
| --- | --- | --- | --- |
| Find a known Question with field syntax | In **Search Question Library**, enter `subject:Genetics` and press Enter. The Search tips state the field vocabulary; after submit, the applied `Search: subject:Genetics` chip is visible. | Results appeared; **Open** opened the Question in a new tab while the original tab retained the exact query. **Clear all** returned the text box to empty and removed results. | Pass |
| Explore by classification | In **Browse Question Library**, choose **Biology** from Discipline. Subject then offered Biochemistry, DNA Profiling, Genetics, Inheritance Genetics, and Molecular Biology. Choose the first available Subject. | Four matching rows appeared. The selected Discipline and Subject stayed in their controls, and the Browse page supplied a **Start over** recovery action. | Pass |
| Find and open a Pool | Submit the Library's empty default search; change **Show** to **Question Pools**; choose **Shared Search Live Verification Pool**. The row visibly says **Question Pool**, **Members 1**, Question Type, Backend, calculated license, and owner. | **Open Question Pool** opened a new tab. The detail page showed the exact Pool title, **Exact Question Revisions**, and `Genetic disorders: Which one?`. | Pass |
| Find a Question in a Pool | The default result controls visibly showed **Questions and Pools** and **Questions in no Pool**. `Genetic disorders: Which one?` was absent. Change membership to **All Questions**. | The known member appeared as its own Question row. The selected **All Questions** value remained visible. | Pass |
| Sort Blueprints by Stars | Open **Courses**, then **Search Public Blueprint Courses**; submit an empty search; choose **Stars** from the labelled sort control. | The sort menu offered Name, Adoptions, Students having taken the course, Stars, Watches, and Most recently edited. **Stars** remained selected and two Public Blueprint rows remained visible, led by `Biochemistry 301: Proteins and Peptides` with its Stars, Watches, and course-use facts. | Pass |

The shared search deliberately begins idle until the Instructor submits Search. That behavior was
discoverable because the Search control is present immediately, avoids half-typed field queries,
and is consistent with the approved plan's submitted-search decision. It is not a finding.

## Guideline ledger

| User need | Observable acceptance criterion | Walkthrough evidence | Status |
| --- | --- | --- | --- |
| Recognize a field search and undo it | Search tips name fields; a committed query becomes a readable removable chip; Clear visibly resets it. | `subject:Genetics` query, chip, new-tab preservation, and Clear all completed. | Pass |
| Explore from a broad classification | A discipline choice limits Subject choices and produces visible rows without remembering identifiers. | Biology supplied five named Subjects and a selected Subject produced four rows. | Pass |
| Tell Pools from Questions before opening | Pool result exposes its kind, member count, ownership, content type, Backend, and calculated license. | Verification Pool row exposed all of those facts before its explicit Open action. | Pass |
| Understand the default that hides Pool members | The active default is visible and switching to All Questions visibly exposes the known member. | Default `both` / `noPool` controls and member appearance after `all`. | Pass |
| Retain exploration while inspecting a record | Result opens in a protected new tab, leaving the submitted search unchanged. | Known Question and Pool both opened detail tabs; Question query remained in source tab. | Pass |
| Sort public Blueprints by stewardship signal | Stars is an available selected sort and returned ordering remains visible. | The labelled Stars sort was selected in the live Public Blueprint search and the result rows retained their stewardship facts. | Pass |

## Findings

No high or medium usability finding was observed in the five completed live task paths. No product
change is requested from this inspection. The local stack was rebuilt between the first four paths
and the Public Blueprint path; the final path ran against the rebuilt 8292 gateway with the same
Instructor account and ordinary UI route.

## Limitations

This is one expert inspection against seeded Instructor data at desktop width. It does not replace
observed Instructor sessions, screen-reader testing, narrow-screen review, or automated
accessibility scanning. It deliberately separates those claims from the companion M15 reviews.
