# Student UI stability and density audit

Date: 2026-09-21

Status: Current-corpus heuristic review. Findings are ready for a bounded UI
follow-up; this file does not implement the changes.

## Conclusion

The Student interface is visually coherent enough to support a useful
interface-by-interface review, but the current layout spends stability budget
on empty space and spends cognitive budget on moving identity, navigation, and
actions.

The most important issue is the difference between a stable shell and a stable
mental model. The Ribbon keeps a reserved task row and the breadcrumb row keeps
its vertical location, so the document does not jump. However, the visible
Ribbon topology changes from a product row, to a course row, to a course plus
task row, and then to a three-row phone arrangement. At the same time, the
working rail changes between course, Attempt, Question, and history views.
Students can therefore see a page that is technically well aligned while the
same task appears to move and be renamed as they proceed.

The Question navigation needs a specific density correction. On the phone, the
current nine-Question pattern can collapse to a first number, one
current-area number, and a last number, visually equivalent to:

    < Prev 1 .. [4] .. 9 Next >

That is not a useful range. The available width should show adjacent context,
for example:

    < Prev 1 2 .. 3 [4] 5 .. 8 9 Next >

or, when the width is genuinely limited:

    < Prev 1 .. 3 [4] 5 .. 9 Next >

The current phone screenshot has unused horizontal space around the Question
navigation, so the implementation should first spend available width on
Question context before adding more vertical space or accepting an isolated
current number.

This review treats the user's About Face concern as a consistency and
predictability hypothesis: changing labels, positions, or object names during
one workflow can weaken recognition and confidence even when each individual
screen is polished. This is a UI heuristic conclusion, not a claim that a
participant study has already measured loss of faith.

## Method and evidence

This is a heuristic evaluation with a task and guideline ledger. The review
follows the Student task sequence:

1. Choose a Course.
2. Find Coursework.
3. Open, resume, or review one item.
4. Read the rules and start or resume the work.
5. Answer, save, and navigate among Questions.
6. Submit and review the Attempt.

The evidence is the current Student screenshot corpus, the screenshot manifest
and atlas, the rendered UI source, and the named product authorities. The
current manifest contains 56 Student captures across 30 laptop, 22 phone, 3
tablet, and 1 square captures. Representative evidence includes:

- [Course list](../../screenshots/student/laptop/course_list.png) and
  [phone Course list](../../screenshots/student/phone/course_list.png)
- [Not started Coursework](../../screenshots/student/laptop/not_started.png) and
  [phone Not started Coursework](../../screenshots/student/phone/not_started.png)
- [Before you start](../../screenshots/student/laptop/overview_history.png)
- [Unanswered Question](../../screenshots/student/laptop/question_unanswered_mc.png),
  [phone Unanswered Question](../../screenshots/student/phone/question_unanswered_mc.png),
  [tablet Unanswered Question](../../screenshots/student/tablet/question_unanswered_mc.png),
  and [square Unanswered Question](../../screenshots/student/square/question_unanswered_mc.png)
- [Question navigation](../../screenshots/student/laptop/assessment_navigation.png)
- [Submitted Attempt](../../screenshots/student/phone/submitted.png)
- [Screenshot-by-screenshot appendix](student_screenshot_breakdown_appendix_2026-09-21.md)

The review uses [Human Guidance](../../HUMAN_GUIDANCE.md), especially the
requirements for consistent controls, efficient screen space, persistent
Ribbon and breadcrumb placement, Student terminology, Coursework density, and
adaptive Question navigation. The terminology check also uses the
[Terminology Contract](../../TERMINOLOGY_CONTRACT.md).

This evidence does not establish participant confidence, task completion
rates, keyboard behavior, focus order, or assistive-technology behavior. The
corpus also uses multiple seeded Student personas, so different avatars are
not evidence that the profile control is unstable. Timers are dynamic capture
values, not visual-regression targets.

## Interface-by-interface breakdown

| Interface | Stable anchors | What moves or changes | Space conclusion |
| --- | --- | --- | --- |
| Course list and invitations | Product identity, Courses destination, breadcrumb row, left content edge | Course cards move the action below the title on phone; invitations use a related but different card vocabulary | The card is a real Course or invitation object, but the desktop cards leave a large amount of unused card space. The phone order is readable but should keep the next action close to the object identity. |
| Course landing | Course context, Coursework destination, course title, state-aware action | `Open`, `Resume`, and `Review` are appropriately state-aware; the phone moves the action below all facts and timing details | Desktop scanning is good, but the mobile action becomes the last element in a tall card. Essential details and the main action should remain visible together. |
| Before you start | Course context and Coursework destination; compact fact groups | The content rail changes from the course page; previous attempts appear below the start summary | The facts are mostly well grouped. The start action is near the summary on laptop and tablet, but a long settings block can still push the action down on phone. |
| Active Attempt | Attempt identity, timer, Question navigation, task link back to Coursework | The Ribbon adds an assessment title, Attempt 1, a selected Attempt tab, and a task row with `Back to Coursework`; the body rail narrows; the breadcrumb is end-scrolled on phone | One-Question focus is appropriate, but the shell becomes a different topology and the Question navigation underuses the available width. |
| Attempt history | Attempt identity and history breadcrumb | The body rail narrows again; the assessment title is repeated; the review surface becomes a compact flat list | Flat Question units are good for review, but the title and summary should not jump to a much narrower desktop rail without a clear reason. |
| Account denial | Product identity and Courses context | Error heading and action are presented in a centered card rather than the ordinary Student page rail | The distinct error object earns a container. Keep its heading, explanation, and return action as one clear recovery group. |

## Findings

### SUI-01 - Ribbon topology changes while the task is still one Student workflow

Priority: P1

The product and course pages show a top identity/navigation row followed by an
apparently empty task band. The active Attempt adds an assessment context and a
task row with `Back to Coursework`. On the phone, product and course pages use
two visible navigation levels, while an active Attempt uses three. Context
details that are visible on the laptop are hidden on the phone, and the
destination moves to its own line.

The cause is explicit in `src/ribbon/app_ribbon.css`: the desktop Ribbon
reserves a 2.5rem task row even when no task area is present, and the phone
Student layout reserves a 2.75rem task row while also changing the top bar to
two rows. The route contract only supplies a task area for the active Attempt
and Attempt history, so the reserved row is empty on the other Student routes.

This is a useful distinction: preserving the breadcrumb's vertical origin is
good, but an empty band is still a visible use of limited screen space. The
current arrangement satisfies the literal no-jump rule more than the user's
need to recognize a stable interface.

Acceptance direction:

- Compare Course, Coursework overview, active Attempt, and history at 1280,
  800 portrait, 800 square, and 393 pixels wide.
- Decide whether the Student shell needs one stable task topology or whether a
  task row should appear only when it contains a useful task control.
- If a row remains reserved, make its purpose and content hierarchy visibly
  intentional rather than leaving a blank band.
- Keep the product identity, current location, Profile, and task return path
  readable and keyboard reachable at narrow widths.

### SUI-02 - The same task changes its desktop content rail

Priority: P1

The course and overview headings begin on the main reading rail near x=64 in
the laptop captures. The active Attempt uses a 60rem page rail near x=160, the
Question card is capped at 56rem, and Attempt history uses a 44rem summary rail
near x=288. These are individually plausible reading measures, but the
assessment title and primary task appear to jump horizontally as the Student
moves through one workflow.

The relevant boundaries are `.page`, `.assessment-attempt-page`,
`.question-card`, and `.attempt-summary` in `src/style.css`. The narrower
Question card is defensible for focused response work. The larger concern is
that the title and task header do not retain a stable outer anchor.

Acceptance direction:

- Measure the left edge of the same assessment title, status, and primary
  action through landing, overview, Attempt, and history.
- Keep one stable outer Student workflow rail, or make a deliberate mode
  change visually obvious.
- Allow the Question content itself to use a narrower inner measure without
  moving the surrounding task identity.

### SUI-03 - Phone breadcrumbs preserve the current leaf by hiding the path prefix

Priority: P1

The current phone Attempt capture begins with a clipped breadcrumb fragment
similar to `nd Peptides / Cell biology response practice / Attempt`. The
submitted history capture similarly begins with `nd Peptides / Chapter 1 Pilot
Practice / Attempt history`. The user can infer the current leaf, but cannot
read the complete Course context from the breadcrumb.

The cause is direct: `src/application_shell.tsx` sets the breadcrumb scroll
position to `scrollWidth` so the resolved current location is visible. The
breadcrumb density CSS permits horizontal overflow. This is a reasonable
attempt to avoid vertical movement, but it makes the persistent path less
useful on the narrowest screen.

Acceptance direction:

- At 393 pixels and enlarged text, keep a recognizable home or Course context
  visible, or provide an always-visible equivalent identity outside the
  scrolled path.
- Preserve access to every breadcrumb link without presenting an unexplained
  clipped prefix in the ordinary resting state.
- Verify the result in the rendered PNG and with keyboard focus, not only in
  source CSS.

### SUI-04 - Question navigation spends its width on fixed controls and too few numbers

Priority: P1

The current implementation starts with `numberSlots = 5`, reserves four of
those slots for the first number, last number, and two omitted-range markers,
and therefore shows only one number in the middle when the set is larger than
the slot count. The phone capture visibly presents `Prev 1 3 9 Next` for a
nine-Question Attempt, with no useful adjacent range and no visible ellipsis.
The source intends to render omitted-range markers, so the rendered absence is
also a separate screenshot acceptance issue rather than something to infer
from the source alone.

That pattern communicates the endpoints but not the nearby work. The Human
Guidance explicitly asks for a range around the current Question and for the
visible range to adapt to available width. The phone navigation row has room
that should be spent on adjacent Question numbers before accepting this
minimum pattern. The current source evidence is in
`src/components/student_assessment_attempt_navigation.tsx:38-80`; the fixed
2.75rem control minimum and non-wrapping row are in
`src/components/student_assessment_attempt_navigation.css:8-45`.

Acceptance direction:

- Treat the visible range as a measured width budget, not a fixed five-number
  minimum.
- Prefer `1 2 .. 3 [4] 5 .. 8 9` when the actual row fits.
- Use `1 .. 3 [4] 5 .. 9` as the constrained fallback, rather than only the
  first, current, and last numbers.
- Require the native PNG to show the intended omitted-range markers when a
  range is omitted; a source-level marker that is invisible in the interface
  does not satisfy the requirement.
- Recalculate for 393, 600, 800, and 1280 pixel layouts, enlarged text, and
  long localized labels for Previous and Next.
- Keep every Question reachable and retain distinct current and saved cues.

### SUI-05 - The main Coursework action moves after all details on phone

Priority: P1

On laptop, the Course landing action sits in the right column beside the
Coursework title and state. On phone, the responsive grid places the action
after the facts, grade/progress, decision details, and timing content. The
result is a tall card whose primary action can be near the bottom of the
viewport, even though it is the next useful step.

The cause is the mobile grid order in
`src/pages/student_course_landing_page.css:94-107` and the shared mobile card
rules in `src/style_responsive.css:23-46`.

Acceptance direction:

- Preserve a clear reading order of state, title, essential status, and action.
- Keep the action close to the title/state group while leaving the detailed
  rules available in a compact summary or disclosure.
- Check `Open`, `Resume`, and `Review` states with long titles at laptop,
  tablet, and phone widths.

### SUI-06 - Student-facing object names drift during the Attempt journey

Priority: P1

The Course landing and overview use the specific type label `Practice Question
Assignment`, while the active Attempt uses the eyebrow `Assessment Attempt 1`
and the submission section says `Finish Assessment` and `Submit Assessment`.
`Coursework` is the correct collective term, and `Attempt` is a useful term
for the submitted unit, but the generic `Assessment` language reappears at the
moment when the Student needs the most confidence about what will happen.

The relevant strings are in `src/pages/assessment_attempt_page.tsx:407,
574-586`, with the history heading in
`src/pages/assessment_attempt_summary_page.tsx:64-65`. This is not a request
to make every heading identical: state and effect labels should change. It is
a request to keep the object noun stable and reserve `Coursework` for the
collection, the Assessment Type for the item, and `Attempt` for the whole
submission unit.

Acceptance direction:

- Use the Assessment Type consistently when naming the item Student actions
  operate on.
- Use `Attempt` when naming the whole saved/submitted work unit.
- Name submission actions by their effect without reintroducing an ambiguous
  generic object, for example by making the assessment type and Attempt context
  adjacent to the action.
- Review Ribbon labels, breadcrumbs, headings, buttons, and status messages as
  one terminology set.

### SUI-07 - Review/history is flatter but narrower than the active task

Priority: P2

The submitted history surface correctly groups each reviewed Question into a
compact unit and avoids recreating a large interactive card. However, the
laptop history heading and content begin on the narrow 44rem summary rail,
while the active task uses a wider outer rail. Structured recorded responses
then leave a large unused area beside the summary in the capture.

This is a density and continuity issue, not a recommendation to make review a
wide dashboard. Keep the flat review treatment, but make its outer title/status
rail consistent with the active task and let individual response tables choose
their own readable width.

Acceptance direction:

- Keep the title, submitted status, score, and return action on the same outer
  Student workflow rail used by the Attempt.
- Let Question review units remain compact and let response content expand only
  when its structure benefits from the width.
- Check long feedback, multi-part responses, and several Questions rather than
  validating only the short current capture.

### SUI-08 - Cards generally represent objects, but several are taller than the task needs

Priority: P2

Course and invitation cards represent real objects and therefore earn a
container. Question cards also represent a focused Question/response
interaction. The concern is the amount of padding and repeated timing content
inside Coursework cards: on phone a single item can consume most of the first
viewport before the next item or the action becomes visible. The large empty
area on the desktop course and invitation surfaces is also more noticeable
because the page contains only one or two objects.

This is a bounded density review, not a blanket request to remove rounded
containers. Apply the space rule to each object: keep the title, state,
essential status, and action in the primary row; place the fuller timing rules
in a compact aligned group or progressive disclosure.

## What is working

- The persistent breadcrumb row and shell reservation make the vertical origin
  predictable across route depth changes.
- Course state actions are meaningful: `Open`, `Resume`, and `Review` tell the
  Student what the next action does.
- Coursework type, completion, saved-response count, grades, and timing facts
  are visible rather than hidden behind unexplained icons.
- Question navigation is a horizontal numbered control with current and saved
  cues, Previous/Next actions, and accessible labels. Its width allocation,
  rather than its overall interaction model, is the current problem.
- The prompt and response control remain the focus of the active Attempt across
  laptop, phone, tablet, and square captures.
- Submitted review uses compact Question units instead of repeating the full
  active-response card.
- The current screenshots provide enough route and viewport variation to find
  cross-interface drift instead of reviewing isolated mockups.

## Guideline and validation ledger

| User need | Current evidence | Product rule | Required validation |
| --- | --- | --- | --- |
| Recognize where I am | Ribbon, breadcrumb, body title, and task row all carry parts of the identity | Keep product, Course, Coursework, Assessment Type, and Attempt roles distinct and predictable | Walk the complete task and record every repeated or renamed identity label |
| Find the next action | Course action is right-aligned on laptop and last on phone; Attempt submission is below the Question | Put the primary action near the content it affects and keep it in task order | Test long titles and settings at 393, 600, 800, and 1280 pixels |
| Scan Coursework | Desktop facts form rows; phone facts stack and the card becomes tall | Keep entries compact, show essential information, and disclose fuller rules | Compare visible item count and action visibility in a first viewport |
| Move among Questions | Phone shows endpoints and too little adjacent context | Use a width-adaptive range around the current Question with ellipses | Verify the exact visible sequence at each width and with enlarged text |
| Preserve context | Phone breadcrumb is end-scrolled and clipped at the start | Keep the Course path recognizable and every breadcrumb reachable | Inspect resting and keyboard-focused states in rendered captures |
| Trust completion/review | Saved cues, submitted state, score, and recorded work are visible | Use plain state/effect labels and compact review units | Walk unanswered, saved, submitted, and completed paths as one Student |

## Recommended bounded follow-up

The next implementation task should be a single Student workflow pass, not a
general redesign:

1. Define the intended Student Ribbon topology for product, Course,
   Coursework overview, active Attempt, and history.
2. Keep one outer workflow rail and preserve the current useful inner Question
   measure.
3. Rework Question range calculation to maximize adjacent numbers within the
   measured row width.
4. Reorder the phone Coursework card so the state, title, essential status, and
   action remain easy to find before fuller timing details.
5. Reconcile Student labels across Ribbon, breadcrumbs, headings, buttons, and
   status messages.
6. Capture the same route matrix again, then inspect the native PNGs at actual
   dimensions before treating the gate as passed.

Do not treat static screenshot inspection as proof of keyboard, focus, or
participant acceptance. Those require separate validation lanes.

## Source and authority map

- [Human Guidance](../../HUMAN_GUIDANCE.md): interface consistency, screen
  space, Ribbon, breadcrumbs, Student Coursework, and Question navigation.
- [Terminology Contract](../../TERMINOLOGY_CONTRACT.md): Student-facing
  Coursework, Assessment Type, Attempt, and action language.
- [Screenshot Atlas](../../SCREENSHOT_ATLAS.md): route and viewport coverage.
- [Current changelog](../../CHANGELOG.md): current corpus provenance and
  capture limitations.
- `src/ribbon/app_ribbon.css`: Ribbon geometry and narrow Student topology.
- `src/application_shell.tsx`: permanent breadcrumb row and current-leaf
  scroll behavior.
- `src/components/student_assessment_attempt_navigation.tsx` and `.css`:
  Question range calculation and control width.
- `src/style.css`, `src/style_responsive.css`, and
  `src/pages/student_course_landing_page.css`: content rails and responsive
  Coursework card order.
