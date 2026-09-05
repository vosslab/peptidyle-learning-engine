# Ribbon task model

## Purpose and boundary

This task model explains how the Application Shell helps an Instructor, Student, or Sysadmin keep
their place while content changes. It records a navigation model, not a promise that every designed
destination is usable today. Canonical surface names come from
[INTERFACE_TERMINOLOGY.md](../INTERFACE_TERMINOLOGY.md); placement and responsive behavior come from
[UI_DESIGN_GUIDE.md](../UI_DESIGN_GUIDE.md).

The current production capability registry declares every Ribbon destination `unbacked`. The current
Live Demo stops at seeded Account session entry. Consequently, the current shell may truthfully show
its Context, Tab, and Task Rows with no admitted navigation link. An unavailable destination is not
a disabled promise, an empty-state substitute, or permission evidence. It remains absent until a
complete usable path has backing evidence in
[capability_registry.ts](../../src/ribbon/capability_registry.ts).

The models below therefore distinguish two states:

- **Current implementation:** the behavior presently supported by the shell and its truthful
  capability admission boundary.
- **Designed path after admission:** the interaction that applies only when the named destination
  has a backed route and is available to the signed-in Product Role. This is not a new workflow or
  authorization rule; it is the stable navigation behavior an admitted capability must join.

The design relies on three durable spatial cues. The Context Row answers "where am I and who am I?";
the Tab Row answers "which destination am I in?"; and the Task Row answers "which part of that work
can I enter?". Those row roles remain in the same order while route content changes. This applies
the supporting UI-literature survey's principles of stable geometry, proximity, discrete responsive
states, and readable keyboard focus, touch, and contrast without turning the survey into product
requirements.

## Shared interaction model

### Re-orientation after a content change

An admitted Ribbon link acknowledges activation at the clicked control: the selected link receives
`aria-current="page"` and pending feedback before route content resolves. The Application Shell stays
mounted, the three Ribbon rows retain their geometry, and focus moves to the one `#main-content`
target after a pathname change. A content error stays inside that target and offers recovery without
removing the Ribbon. The skip link reaches the same target directly.

This means a person can re-orient from the Context Row, then confirm the selected Tab or Task, then
resume reading or working in the content area. A scoped course label and Assignment Attempt labels
add context; they never replace the page heading or cause a row to appear, disappear, or move.

### Course Instance Tabs

When a backed Course Instance route is available, the Course Instance Ribbon Scope supplies its
Product Role's ordered schema. An Instructor's designed Tabs are **Assignments**, **Students**,
**Gradebook**, **Teaching Operations**, **Blueprint Updates**, and **Course Setup**; a Student's is
**Assignments**; a Sysadmin's is **Teaching Operations**. Capability admission may omit an unavailable
destination, but the schema never changes order because loading, a title, or a page error occurs.

An Instructor's assignment-workspace Tasks are **Overview**, **Questions**, **Policies**, **Grading
Operations**, and **Student View** when backed. **Grade Settings** and **Appearance** are Course Setup
Tasks when backed. **Create Assignment** is a Page Action in Assignments content, not a Ribbon Tab or
Task. The Task Row intentionally remains reserved when no task is admitted, so the content origin
does not move.

### Assignment Attempt boundary

The Assignment Attempt Ribbon Scope is a Student scope. Its designed schema contains **Attempt** and
the **Back to Assignments** Task; **Assignment Attempt Progress** belongs in the Context Row, while
question navigation and timing remain in Attempt content. Instructor and Sysadmin Assignment Attempt
schemas contain no Tabs. The current `attempt` and `backToAssignments` entries are unbacked, so no
current account has an admitted Ribbon attempt workflow. A future backed implementation must preserve
these roles rather than adding question positions or timers as variable Ribbon navigation.

## Instructor task model

**Trigger.** An Instructor enters a backed Product or Course Instance destination, changes an
admitted Course Instance Tab, or opens an assignment-workspace Task from Assignments.

**Goal.** Teach from the correct Course Instance while recognizing the active destination and keeping
the next course-management decision available without having to rediscover navigation after content
changes.

**Decision points.**

- At Product scope, choose among the admitted **Courses**, **Question Library**, and **Blueprint
  Courses** Tabs.
- At Course Instance scope, confirm the course identity in the Context Row, then choose the admitted
  Course Instance Tab appropriate to the teaching decision.
- On Assignments, use the content-local **Create Assignment** Page Action when creating rather than
  navigating; use an admitted assignment-workspace Task only after an assignment is selected.
- On a route reached through a Context Control, recognize **No Selected Ribbon Tab** as an intentional
  state and use the persistent schema to return to a teaching destination.

**Information needs.** The Instructor needs the Product Role, Account label, selected Course Instance
label, selected Tab or Task, page heading, and the route-local teaching data. The Instructor does not
need an opaque identifier, a second course navigation surface, or a loading replacement for the
Ribbon.

**Error and recovery.** If route content fails, the content-region recovery explains that the learning
space, navigation, and any active Assignment Attempt remain available. The Instructor may retry that
page or use a visible Ribbon link when one is admitted. If an intended destination is absent, the
current honest explanation is that it has no backed usable path; the UI does not imply that an
Instructor can obtain it by retrying, changing a role, or entering a guessed URL. Sign-out failure
keeps the session open, reports the failure through the shell's live status, and permits a retry.

**Completion evidence.** The selected link and the content heading agree on the teaching location;
the Context Row names the same Course Instance; the focused `#main-content` content is available
after navigation; and no course-navigation control changes position during loading, recovery, or
content replacement.

**Current implementation boundary.** The Instructor's current Product and Course Instance schemas
exist as catalog/schema data, but all their destinations are unbacked and omitted. No current
Instructor Ribbon link admits a Course Instance Tab or assignment-workspace Task. This model states
the required behavior after a future backend capability is admitted; it does not claim that course
or assignment management is presently usable from the Live Demo.

**Assignment Attempt entry and exit.** An Instructor has no designed Assignment Attempt Ribbon Tab.
Entering a Student's attempt is not an Instructor Ribbon workflow in the current model, and there is
no admitted Instructor attempt route to document. If a future instructor-facing review capability is
designed, it needs its own declared scope, route, authorization, and task model; it must not borrow
the Student **Attempt** Tab.

## Student task model

**Trigger.** A Student enters an admitted Course Instance destination, chooses **Assignments**, enters
an admitted Assignment Attempt, or activates **Back to Assignments**.

**Goal.** Find the current Course Instance, begin or resume the correct Assignment Attempt, answer
questions in content, and return to Assignments without losing orientation or keyboard reachability.

**Decision points.**

- At Product scope, choose the admitted **Courses** Tab to find the relevant Course Instance.
- At Course Instance scope, confirm the course label, then choose the Student's sole designed Tab,
  **Assignments**.
- At Assignment Attempt scope, confirm **Attempt** and **Assignment Attempt Progress** before working
  in Attempt content. Use content-local question controls for sequence and responses; use **Back to
  Assignments** to leave the attempt surface.
- After a content update, use the selected Tab or Task and the page heading to decide whether to
  continue, retry the current page, or return to Assignments.

**Information needs.** The Student needs the course name, selected **Assignments** or **Attempt**
state, readable Assignment Attempt Progress, the current question or recovery content, and a clear
way to return to Assignments. The Student does not need a Question Library, Instructor controls,
unavailable destinations, answer-bearing data outside the active content boundary, or an opaque
resource identifier.

**Error and recovery.** The persistent shell keeps the selected location visible while the content
region reports a loading or error state. The Student can use the skip link or keyboard focus to reach
`#main-content`, retry the page when offered, or select an admitted persistent destination. A missing
or unbacked control is not a denied action: it is absent because there is no complete usable path to
offer. Sign-out failure leaves the session open and presents a retryable live-status message.

**Completion evidence.** On an admitted Course Instance route, **Assignments** is selected and the
course label is stable. On an admitted Assignment Attempt route, **Attempt** is selected, progress
is contextual rather than a changing Tab, and **Back to Assignments** returns to the Course Instance
Assignments destination. Keyboard operation reaches identity, Ribbon Tabs, Ribbon Tasks, then the
content target in logical order; focus and selection remain visible without depending on color alone.

**Current implementation boundary.** The Student's designed Product, Course Instance, and Assignment
Attempt schemas exist, but **Courses**, **Assignments**, **Attempt**, and **Back to Assignments** are
currently unbacked and omitted. The current Live Demo therefore provides no admitted Student course
or Assignment Attempt workflow. This document records the navigation behavior required once those
capabilities have a complete backed path; it does not fabricate a current assignment, submission,
timer, grade, or recovery workflow.

## Sysadmin task model

**Trigger.** A Sysadmin enters a backed Product or Course Instance destination, changes an admitted
Course Instance Tab, or returns from a Context Control route with No Selected Ribbon Tab.

**Goal.** Orient to the active account and course context, reach the backed system or course-support
surface, and recover from content failure without gaining unintended access to Student records.

**Decision points.**

- At Product scope, choose an admitted **Courses** destination; **Instructor Accounts** remains a
  designed future position until a route, client operation, and registered handler exist.
- At Course Instance scope, confirm the Course Instance label and choose admitted **Teaching
  Operations** only when it has a backed usable path.
- Use a Context Control route when account-level work is appropriate, recognizing that no Tab may be
  selected while the same Ribbon schema persists.
- Treat a missing Student roster, Gradebook, or Assignment Attempt control as an intentional boundary,
  not a cue to seek general FERPA access.

**Information needs.** The Sysadmin needs Product Role, Account label, selected Course Instance
context when present, selected destination, and route-local support information. The Sysadmin does
not need permanent Student roster, grade, or Assignment Attempt navigation in the general schema;
the product's FERPA boundary remains enforced by route and server authorization.

**Error and recovery.** A content failure leaves the Application Shell intact and offers retry or a
return to a supported location. If a support capability is absent, the appropriate recovery is to
use an admitted path or await the complete capability package; the Ribbon does not advertise a
placeholder or infer access from the Sysadmin role. Sign-out recovery follows the shared shell
behavior and does not leave a false signed-out state.

**Completion evidence.** The Context Row and selected Tab identify the same supported context; a
content recovery preserves the same Ribbon instance and focus target; and no general Student,
Gradebook, or Assignment Attempt control is introduced merely because the account is Sysadmin.

**Current implementation boundary.** The Sysadmin's designed **Courses**, **Instructor Accounts**,
and **Teaching Operations** positions are all unbacked; no Sysadmin Ribbon destination is admitted
today. The current route contract has no sysadmin-only route. This task model therefore documents a
future backed support path, not a present course-administration or FERPA-access workflow.

**Assignment Attempt entry and exit.** The Sysadmin Assignment Attempt schema intentionally has no
Tabs, and the current product admits no attempt workflow. A request to inspect a particular Student's
attempt must be designed as an explicit, course-scoped, authorized support capability before it can
appear; this model supplies no implied entry or exit path.

## Heuristic and accessibility ledger

Each row ties a user-facing reason to a concrete acceptance check. The named evidence files are the
current implementation checks; future capability admission must keep the same checks meaningful.

| Guideline                                             | User-facing rationale                                                                                                                                                                   | Concrete acceptance check                                                                                                                                                                                                                                                                                                      |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Stable spatial memory and visibility of system status | A person can keep their place because selection changes at the chosen control while the Context, Tabs, Tasks, and content origin remain recognizable.                                   | `tests/playwright/ribbon_m10_shell_evidence.mjs` proves one persistent Ribbon instance, stable element identity across route transitions, and content-only recovery; the non-browser E2E [pending-navigation check](../../tests/e2e/e2e_ribbon_pending_navigation.mjs) proves pending feedback is local to a link.             |
| Nielsen: match between system and teaching task       | Course identity, **Assignments**, **Attempt**, and **Back to Assignments** use the terms a teacher or student needs to recognize work, rather than internal route or database names.    | `tests/test_ribbon_catalog.mjs` and `tests/test_ribbon_contract.mjs` assert catalog labels and declared route mapping; a documentation review checks vocabulary against [INTERFACE_TERMINOLOGY.md](../INTERFACE_TERMINOLOGY.md).                                                                                               |
| Nielsen: user control and recovery                    | A content problem does not trap a person or erase navigation: they can retry the page, return to courses, or use an admitted destination.                                               | `tests/playwright/ribbon_m10_shell_evidence.mjs` triggers content recovery, then activates Tabs with mouse and keyboard while confirming the same Ribbon remains mounted.                                                                                                                                                      |
| Nielsen: consistency and standards                    | The same row role and link treatment communicate the same thing across Product, Course Instance, and Assignment Attempt scopes.                                                         | `tests/test_ribbon_schema.mjs` and `tests/test_ribbon_catalog.mjs` assert closed schemas and ordered controls; the non-browser E2E [Application Shell component check](../../tests/e2e/e2e_ribbon_app_component.mjs) asserts selected-link semantics.                                                                          |
| WCAG 2.2 SC 3.2.3 Consistent Navigation               | Repeated navigation stays in a predictable order, so a keyboard or screen-reader user does not have to relearn the shell after content changes.                                         | `tests/test_ribbon_schema.mjs` proves ordered schema positions; `tests/playwright/ribbon_m10_shell_evidence.mjs` checks persistent shell identity and one Ribbon navigation surface across transitions.                                                                                                                        |
| WCAG 2.2 SC 3.2.4 Consistent Identification           | A control with the same purpose keeps the same accessible label and visual name wherever it appears.                                                                                    | `tests/test_ribbon_catalog.mjs` and `tests/test_ribbon_icons.mjs` keep catalog labels as link names and verify icon treatment supplements rather than replaces the label; the non-browser E2E [Application Shell component check](../../tests/e2e/e2e_ribbon_app_component.mjs) verifies the rendered selected-link semantics. |
| Keyboard operation and focus order                    | Students can reach navigation and learning content without a pointer, and a route change has one predictable content destination.                                                       | `tests/playwright/ribbon_m10_shell_evidence.mjs` checks the focused skip link and its `#main-content` target; `src/application_shell.tsx` moves focus to that target after pathname changes.                                                                                                                                   |
| Visible focus and non-color selection                 | A keyboard user can see the active control and current location even when hue is not distinguishable.                                                                                   | `tests/playwright/ribbon_m9b_density_evidence.mjs` measures focus and selection styles; `src/ribbon/app_ribbon.css` pairs focus outline with selected weight and underline or task background treatment.                                                                                                                       |
| Reflow, text resizing, and discrete responsive states | A smaller viewport or larger text changes presentation deliberately without changing navigation meaning, hiding a normal destination label, or making the selected control unreachable. | `tests/playwright/ribbon_m9_responsive_evidence.mjs` and `tests/playwright/ribbon_geometry_evidence.mjs` cover narrow, tablet, and 200% text geometry, overflow cues, and selected-control reveal.                                                                                                                             |
| Contrast and forced colors                            | Text, selection, focus, and essential boundaries remain distinguishable for people using a course theme or a high-contrast system mode.                                                 | `tests/playwright/ribbon_m9b_density_evidence.mjs` covers contrast, forced-colors, and reduced-motion behavior; [UI_DESIGN_GUIDE.md](../UI_DESIGN_GUIDE.md) records the ordinary-text and focus treatment requirements.                                                                                                        |
| Motion preference                                     | Understanding the current location does not depend on an animation, and people who reduce motion do not receive unnecessary movement.                                                   | `tests/playwright/ribbon_m9b_density_evidence.mjs` checks the reduced-motion projection; `src/ribbon/app_ribbon.css` contains the `prefers-reduced-motion` treatment.                                                                                                                                                          |
| Truthful capability admission                         | A person never spends effort activating a dead control or interpreting an unavailable feature as a role failure.                                                                        | `tests/test_ribbon_capability_registry.mjs` proves `Available` is the only visible admission state; `tests/playwright/ribbon_m10_shell_evidence.mjs` separates the truthfully empty production case from the explicitly populated structural fixture.                                                                          |

## Maintenance rule

When a backend capability becomes complete, update its registry evidence and run the integration
checklist in [FRONTEND_CAPABILITY_INTEGRATION.md](FRONTEND_CAPABILITY_INTEGRATION.md). Update this task
model only if the capability changes a person's trigger, decision, information need, recovery, or
completion evidence. Do not add a control simply to make a row look occupied, and do not move an
existing control to make a new capability fit.
