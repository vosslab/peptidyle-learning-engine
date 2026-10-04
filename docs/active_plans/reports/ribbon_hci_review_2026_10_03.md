# Instructor Ribbon HCI Review - 2026-10-03

## Decision and scope

**Decision supported.** Does the current Instructor Ribbon let an instructor recognize their location and move through common course, content, and grading work without remembering navigation structure?

**Method.** A small expert cognitive walkthrough plus heuristic inspection. This is the bounded method for an existing interface when the immediate decision is findability and location cues, not measured user success. I inspected static desktop originals and supplied state evidence; I did not run the application.

**Primary instructor.** An upper-level biology instructor managing a course, roster, grades, reusable Blueprint, and question collection from a laptop. The normal target is a 1280 x 800 browser. Narrow captures test whether the same location model remains intelligible when horizontal space is scarce.

**Expected outcome.** At each step, the instructor can answer: "Where am I?", "What related destination can I choose?", and "How do I return to the Course or collection that contains this record?" without relying only on memory.

## Evidence inspected

The six 1280 x 800 originals were:

- [My Active Courses](../../screenshots/instructor/courses-active-course_list.webp)
- [Blueprint detail](../../screenshots/instructor/courses-blueprint-blueprint_detail.webp)
- [Course roster](../../screenshots/instructor/courses-students-course_roster_active.webp)
- [Gradebook](../../screenshots/instructor/courses-gradebook-gradebook.webp)
- [Browse Question Library](../../screenshots/instructor/questions-browse-library_browse.webp)
- [Assessment Question Editor](../../screenshots/instructor/courses-assessments-assignment_questions_draft.webp)

The supplied state renders were:

- [320 px grass light Course page](ribbon_state_contrast_2026_10_03/page_grass_light_320.png)
- [1280 px tundra dark Course page](ribbon_state_contrast_2026_10_03/page_tundra_dark_1280.png)

The sources show a continuous Tier 1 bar, a square Tier 2 row joined to the content surface when selected, and a permanent breadcrumb row. They include light and dark states and a narrow Course page. These are static visual evidence, not browser behavior or participant evidence.

## Task model and walkthrough result

| Task | Recognition cues inspected | Static walkthrough judgment |
| --- | --- | --- |
| Locate and open a Course | `Courses`, `My Active Courses`, matching breadcrumb, course title, `Open Course` | Pass on desktop. The route and next action are conspicuous. |
| Reach roster or Gradebook | Course breadcrumb, `Students` or `Gradebook` heading, Course tools | Pass with a location-model ambiguity below. Course context remains visible. |
| Edit a reusable Blueprint | `Courses` / `My Blueprint Courses`, Blueprint breadcrumb, `Open Course Editor` | Pass. Reusable, student-free status is explicit. |
| Find a Question | `Questions`, selected `Browse Question Library`, filters, subject paths | Pass. The empty/filter result gives recovery direction. |
| Keep orientation at narrow width | selected tiers, breadcrumb, page heading | Conditional. Selected labels survive; clipped neighbors weaken discovery. |

## Findings

### HCI-1 - Make narrow navigation overflow visibly discoverable

**Priority:** High. **Confidence:** High for the observed condition; medium for task delay because no participants were observed.

**Observed fact.** In the [320 px grass light capture](ribbon_state_contrast_2026_10_03/page_grass_light_320.png), both Ribbon rows contain partially clipped labels at their edges. Small arrow-like fragments appear, but the capture does not make scroll state or additional choices obvious. `Courses` and `My Active Courses` remain readable.

**Judgment.** An instructor arriving on a Course detail page can recognize the current route, but may not discover a neighboring Course destination without trying horizontal scrolling. This creates a recall burden where the Ribbon should support recognition, particularly when changing Course lists or collections.

**Recommendation.** Owner: Ribbon/navigation implementation owner. Preserve the selected tab fully in view and add a clear overflow cue at each scrollable row: a visible partial next tab plus an edge fade or dedicated scroll control whose purpose remains understandable. Keep the selected item visible after navigation and resize.

**Success and validation.** At 320 px and 393 px, a fresh render shows complete selected Tier 1 and Tier 2 labels, an unambiguous indication that more choices exist, and a reachable next/previous choice. Then conduct a five-instructor moderated task: from a Course page, find `My Blueprint Courses` and return to the active-course list. Record completion, wrong taps, and confidence; target 5/5 completion without facilitator navigation hints.

### HCI-2 - Give every Course-descendant page one unambiguous Tier 2 meaning

**Priority:** High. **Confidence:** Medium-high.

**Observed fact.** The [Gradebook](../../screenshots/instructor/courses-gradebook-gradebook.webp) selects `Assessments` in Tier 1 while Tier 2 shows `Assessments Due Soon` and `My Assessment Templates`; the breadcrumb identifies the specific Course and `Gradebook`. The [Assessment Question Editor](../../screenshots/instructor/courses-assessments-assignment_questions_draft.webp) has the same Tier 2 choices while editing a Course assessment. Neither label describes Gradebook or the Question Editor.

**Judgment.** Tier 1 correctly classifies these pages as assessment-related, but Tier 2 can read as either current location or an unrelated global menu. That asks instructors to use breadcrumbs as the sole reliable current-location cue and conflicts with the intended descendant rule for a selected Tier 2 control.

**Recommendation.** Owner: navigation information-architecture owner. Define and apply one stable selected-Tier-2 parent for every Course-specific assessment and grading descendant. If neither current Tier 2 destination is its genuine parent, add the smallest stable Course-workflow destination needed to make the hierarchy truthful; do not style an unrelated global page as active.

**Success and validation.** In fresh Gradebook and Assessment Editor renders, an instructor can name the active Tier 2 parent and Course context from Ribbon plus breadcrumb without explanation. In a short cognitive walkthrough, ask, "Which top-level area are you in, and what does the selected second-row item mean here?" All reviewers should give the same accurate answer.

**Integration caveat.** This is a navigation-design proposal, not an established routing defect.
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) requires a stable Tier 2 set for each Tier 1.
First inspect actual entry paths and route ancestry. Any proposed new destination must represent
a demonstrated workflow and preserve that fixed-set rule; a missing highlight alone does not
justify adding a destination or selecting an unrelated parent.

### HCI-3 - Retain strong selected-surface contrast across all themes

**Priority:** Medium. **Confidence:** High for inspected themes; limited for themes not directly inspected.

**Observed fact.** In the [desktop dark render](ribbon_state_contrast_2026_10_03/page_tundra_dark_1280.png), selected curved `Courses` emerges from the Tier 1 bar and selected square `My Active Courses` joins the page surface. The analogous selection appears in the [narrow light render](ribbon_state_contrast_2026_10_03/page_grass_light_320.png). Original Course, Blueprint, roster, and Question Library pages retain this geometry while themes vary.

**Judgment.** This is a material strength. Different tab shapes and surface relationships give Tier 1 and Tier 2 distinct meanings without card-like controls, and current location is normally available at a glance. Contrast must remain an explicit surface relationship, never a subtle border or curve that vanishes in a theme.

**Recommendation.** Owner: theme/Ribbon implementation owner. Treat selected Tier 1 surface, selected Tier 2 content join, and inactive-tab recession as a three-part cross-theme visual contract. Retain readable, enabled-looking labels for inactive choices.

**Success and validation.** Review fresh light and dark renders for every supported theme at desktop and narrow widths. A reviewer should identify both selected levels at a glance before reading the breadcrumb. Run separate measured text, icon, focus, and control contrast checks; surface distinction alone does not establish accessibility conformance.

### HCI-4 - Use breadcrumbs as Course-context recovery, not a substitute for selected navigation

**Priority:** Medium. **Confidence:** High.

**Observed fact.** Roster, Gradebook, Blueprint, and Assessment Editor captures use readable breadcrumb names and keep the Course or Blueprint title between broad navigation and the current page. The Course page itself exposes `Open Students` and `Gradebook` together under Course tools.

**Judgment.** This provides a robust return path for instructors reaching a record through search, an editor, or a Course tool. It is useful because Course identity remains available while operating under global `Assessments`. The breadcrumb supports recovery but does not resolve HCI-2's Tier 2 ambiguity.

**Recommendation.** Owner: shared breadcrumb/page-frame owner. Preserve the full human-readable Course ancestor and stable breadcrumb space. On narrow layouts, truncate only after retaining a recognizable Course short name and current page; ensure every retained ancestor is an actual return destination.

**Success and validation.** At desktop and narrow width, reviewers can return from roster, Gradebook, and Question Editor to the containing Course using only a visible breadcrumb or Course tool. Keyboard and screen-reader behavior remain to be tested live.

**Integration caveat.** Any visual truncation must preserve the authored Course name and access
to its full text. A compact visual treatment is not permission to replace breadcrumb identity
with an inferred or stripped-down name.

## Strengths

- Desktop course-list, Blueprint, roster, and Question Library paths expose current Tier 1, Tier 2, breadcrumb, page title, and next task action in a consistent vertical order.
- The Question Library pairs broad-to-narrow filters with subject paths and an actionable empty-state recovery message; this supports recognition rather than memorized query syntax.
- Blueprint detail distinguishes reusable course structure from a Course Instance before presenting its create-instance action.
- Course tools place roster and Gradebook near Course information, matching the stated navigation rule.

## Limitations and next evidence

This inspection did not observe instructors, timed completion, pointer/touch scrolling, keyboard focus, screen-reader names and states, browser overflow, zoom, or responsive transitions. It examined only the eight named static captures and cannot establish WCAG conformance or verify all themes meet the same state-contrast contract.

The next safe evidence step is a live task-completion walkthrough followed by the narrow five-instructor study in HCI-1. Test actual scroll discovery, Tier 2 interpretation for Gradebook and editor pages, and breadcrumb recovery. No product change follows from this report alone.
