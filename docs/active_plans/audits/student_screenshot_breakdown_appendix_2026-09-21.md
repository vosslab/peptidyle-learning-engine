# Student screenshot breakdown appendix

Date: 2026-09-21

Parent audit: [Student UI stability and density audit](student_ui_stability_and_density_audit_2026-09-21.md)

## Purpose and reading notes

This appendix records one observation for every PNG in the current Student
screenshot corpus. The links point to the actual role-owned files, not to
scenario definitions or temporary visual references.

The corpus contains 56 Student captures:

- 30 laptop captures at 1280 x 800
- 22 phone captures at 393 x 852
- 3 tablet captures at 800 x 1280
- 1 square capture at 800 x 800

The `Finding` column points to the parent audit findings. `Working` means that
the screenshot provides useful evidence or that the observed behavior is
intentional; it does not mean that the entire interface has passed runtime,
keyboard, accessibility, or participant validation.

The response-format rows are deliberately repetitive about the shared shell.
That repetition is evidence: each format should remain inside the same Student
Attempt frame while the response control changes. Format-specific control
behavior is identified from the named capture and its scenario contract; this
appendix does not claim that a static PNG proves completion or grading behavior.

## Product, Course, invitation, and authorization surfaces

| Screenshot | Viewport | Observation | Finding |
| --- | --- | --- | --- |
| [course_list.png](../../screenshots/student/laptop/course_list.png) | Laptop | Product-level `Courses` is selected. A visibly empty reserved task band separates the top row from the breadcrumb. The Course card uses a wide desktop row with the action at the right. | SUI-01, SUI-08 |
| [course_list.png](../../screenshots/student/phone/course_list.png) | Phone | `Courses` moves to a second Ribbon line while the reserved band remains. The Course title wraps and the card action moves below the title, confirming the phone action-order change outside the Coursework landing route. | SUI-01, SUI-05, SUI-08 |
| [invitation_index.png](../../screenshots/student/laptop/invitation_index.png) | Laptop | The invitation index keeps the product-level `Courses` shell and the same empty task band. The invitation object has instructor and term information with a right-aligned review action. | SUI-01, SUI-08 |
| [invitation_detail.png](../../screenshots/student/laptop/invitation_detail.png) | Laptop | The invitation detail keeps the product shell and moves the acceptance decision into the page body. The Course name, instructor, term, explanatory copy, and `Accept invitation` action form one meaningful object. | SUI-01, SUI-08 |
| [denial.png](../../screenshots/student/laptop/denial.png) | Laptop | The authorization denial uses a centered recovery card rather than the ordinary reading rail. The heading, explanation, and return action are grouped as one error-and-recovery object. | Working; SUI-08 only if the card is made taller or more padded |
| [denial.png](../../screenshots/student/phone/denial.png) | Phone | The denial card remains a distinct recovery object at narrow width. The heading wraps and the persistent upper shell still spends space on the reserved band before the error content. | SUI-01, SUI-08 |

## Course landing states

| Screenshot | Viewport | Observation | Finding |
| --- | --- | --- | --- |
| [not_started.png](../../screenshots/student/laptop/not_started.png) | Laptop | Course context and `Coursework` remain in the Ribbon. The not-started Coursework card presents type, completion, availability, timing, and a right-column start action. | SUI-01, SUI-05, SUI-08 |
| [in_progress.png](../../screenshots/student/laptop/in_progress.png) | Laptop | The card geometry stays stable while the action changes to `Resume` and the saved-response count appears. This is a good state-aware label change, not unexplained movement. | Working state evidence; SUI-05 for the shared card density |
| [completed.png](../../screenshots/student/laptop/completed.png) | Laptop | The same Coursework object changes to `Review` and shows the graded result. The stable card geometry helps the state change read as progress rather than a new object. | Working state evidence; SUI-05, SUI-06 for terminology review |
| [not_started.png](../../screenshots/student/phone/not_started.png) | Phone | The course identity and `Coursework` destination occupy separate Ribbon lines. Facts stack into one column and the primary action follows the detailed access and timing rules near the bottom of the card. | SUI-01, SUI-05, SUI-08 |

## Assignment overview and history surfaces

| Screenshot | Viewport | Observation | Finding |
| --- | --- | --- | --- |
| [unanswered.png](../../screenshots/student/laptop/unanswered.png) | Laptop | The overview places the Assessment Type and title on the main reading rail, then groups `Before you start` facts before the start action. The course-level title rail is wider than the active Attempt rail. | SUI-02, SUI-05, SUI-08 |
| [unanswered.png](../../screenshots/student/tablet/unanswered.png) | Portrait tablet | The same overview hierarchy survives at 800 pixels wide: title, two-column facts, start action, and previous-attempt area. This is useful responsive evidence, but does not cover intermediate widths between tablet and phone. | Working responsive evidence; SUI-02, SUI-05 |
| [overview_history.png](../../screenshots/student/laptop/overview_history.png) | Laptop | The overview adds previous attempts below the rules and start summary. The facts are readable, but the page accumulates several vertically separated groups before history becomes visible. | SUI-02, SUI-07, SUI-08 |
| [selected_history.png](../../screenshots/student/laptop/selected_history.png) | Laptop | The submitted Attempt history is a flatter, compact review surface, but its 44rem rail begins farther right than the active Attempt and repeats the assessment title in a new heading position. | SUI-02, SUI-07 |

## Active Attempt shell and submission states

| Screenshot | Viewport | Observation | Finding |
| --- | --- | --- | --- |
| [response_selected.png](../../screenshots/student/laptop/response_selected.png) | Laptop | The active Attempt adds assessment context, `Attempt`, and a `Back to Coursework` task row. The outer page rail is narrower than the Course and overview rails, with a still narrower Question card. | SUI-01, SUI-02, SUI-04 |
| [assessment_navigation.png](../../screenshots/student/laptop/assessment_navigation.png) | Laptop | The timer, current Question, saved count, and horizontal navigation are all visible. The navigation model is appropriate, but its width strategy should be evaluated with longer Question sets and the phone geometry. | Working interaction model; SUI-04 for width allocation |
| [resume_selected.png](../../screenshots/student/tablet/resume_selected.png) | Portrait tablet | The active Attempt shell remains readable on the portrait tablet, including the task row, title, timer, Question navigation, and response card. The shell is still more vertically layered than the Course shell. | SUI-01, SUI-02, SUI-04 |
| [submitted.png](../../screenshots/student/phone/submitted.png) | Phone | The three-level phone Attempt topology remains visible after submission. The breadcrumb is end-scrolled so the Course prefix is clipped, and the history rail is narrower than the active Question rail. | SUI-01, SUI-02, SUI-03, SUI-07 |

## Question-format captures: laptop, unanswered

| Screenshot | Viewport | Observation | Finding |
| --- | --- | --- | --- |
| [question_unanswered_mc.png](../../screenshots/student/laptop/question_unanswered_mc.png) | Laptop | Multiple-choice response controls sit in the shared Question card. The active Attempt shell, timer, and navigation remain consistent with the other formats. | SUI-01, SUI-02, SUI-04 |
| [question_unanswered_ma.png](../../screenshots/student/laptop/question_unanswered_ma.png) | Laptop | Multiple-answer controls occupy the Question card while the shell remains unchanged. The format adds response content, not a new navigation mode. | Working format-shell consistency; SUI-01, SUI-02 |
| [question_unanswered_fib.png](../../screenshots/student/laptop/question_unanswered_fib.png) | Laptop | Fill-in-the-blank input is presented inside the same Attempt and Question-card frame. The title, timer, and navigation retain their positions. | Working format-shell consistency; SUI-01, SUI-02 |
| [question_unanswered_multi_fib.png](../../screenshots/student/laptop/question_unanswered_multi_fib.png) | Laptop | Multiple fill-in inputs make the response body taller without changing the outer Attempt identity or navigation. The response control remains the task focus. | Working format-shell consistency; SUI-01, SUI-02 |
| [question_unanswered_num.png](../../screenshots/student/laptop/question_unanswered_num.png) | Laptop | Numeric response input remains within the shared Question card and inherits the same title, timer, and navigation context. | Working format-shell consistency; SUI-01, SUI-02 |
| [question_unanswered_match.png](../../screenshots/student/laptop/question_unanswered_match.png) | Laptop | Matching response content uses a wider internal arrangement while the Attempt shell stays stable. This is a useful case for checking inner content width separately from the outer workflow rail. | SUI-02, SUI-04 |
| [question_unanswered_order.png](../../screenshots/student/laptop/question_unanswered_order.png) | Laptop | Ordering controls add a structured response area inside the same Question card. The outer navigation and task identity remain recognizable. | Working format-shell consistency; SUI-01, SUI-02 |
| [question_unanswered_hotspot.png](../../screenshots/student/laptop/question_unanswered_hotspot.png) | Laptop | The uploaded-image hotspot response adds a visual interaction surface inside the Question card. It should not force the Ribbon or assessment identity to change. | SUI-01, SUI-02 |
| [question_unanswered_webwork.png](../../screenshots/student/laptop/question_unanswered_webwork.png) | Laptop | The WeBWorK response is delivered inside the same Student Attempt context. The external response surface is a format boundary, not a reason to rename or relocate the surrounding task identity. | SUI-01, SUI-02, SUI-06 |

## Question-format captures: laptop, answered

| Screenshot | Viewport | Observation | Finding |
| --- | --- | --- | --- |
| [question_answered_mc.png](../../screenshots/student/laptop/question_answered_mc.png) | Laptop | The answered multiple-choice state adds the selected response and saved feedback while keeping the Question title, navigation, and Attempt frame stable. | Working saved-state evidence; SUI-04 for navigation density |
| [question_answered_ma.png](../../screenshots/student/laptop/question_answered_ma.png) | Laptop | The answered multiple-answer state confirms that saved status can be shown without changing the surrounding shell or object identity. | Working saved-state evidence; SUI-04 |
| [question_answered_fib.png](../../screenshots/student/laptop/question_answered_fib.png) | Laptop | The answered fill-in state keeps the response feedback near the response control and preserves the common Attempt geometry. | Working saved-state evidence; SUI-01, SUI-04 |
| [question_answered_multi_fib.png](../../screenshots/student/laptop/question_answered_multi_fib.png) | Laptop | The answered multiple fill-in state makes a taller response body but does not move the Ribbon or rename the task. | Working saved-state evidence; SUI-01, SUI-02 |
| [question_answered_num.png](../../screenshots/student/laptop/question_answered_num.png) | Laptop | The answered numeric state preserves the same response feedback and navigation relationship as the other saved formats. | Working saved-state evidence; SUI-01, SUI-04 |
| [question_answered_match.png](../../screenshots/student/laptop/question_answered_match.png) | Laptop | The answered matching state is a useful wide-inner-content case: the response structure may need width, but the outer title and task rail should remain stable. | SUI-02, SUI-04 |
| [question_answered_order.png](../../screenshots/student/laptop/question_answered_order.png) | Laptop | The answered ordering state preserves the common Question frame while showing saved response feedback near the control. | Working saved-state evidence; SUI-01, SUI-04 |
| [question_answered_hotspot.png](../../screenshots/student/laptop/question_answered_hotspot.png) | Laptop | The answered hotspot state keeps the visual response interaction inside the same Attempt shell and shows that format-specific content need not alter navigation identity. | Working saved-state evidence; SUI-01, SUI-02 |
| [question_answered_webwork.png](../../screenshots/student/laptop/question_answered_webwork.png) | Laptop | The answered WeBWorK state preserves the Student Attempt context around the delivered response surface. The surrounding language remains the important consistency boundary. | SUI-01, SUI-02, SUI-06 |

## Question-format captures: phone, unanswered

| Screenshot | Viewport | Observation | Finding |
| --- | --- | --- | --- |
| [question_unanswered_mc.png](../../screenshots/student/phone/question_unanswered_mc.png) | Phone | The native PNG visibly shows `Prev 1 3 9 Next` with unused row width and no useful adjacent range. The breadcrumb is clipped at the left and the three-level Attempt Ribbon consumes substantial vertical space. | SUI-01, SUI-03, SUI-04 |
| [question_unanswered_ma.png](../../screenshots/student/phone/question_unanswered_ma.png) | Phone | Multiple-answer controls use the same narrow Attempt shell. The phone layout preserves the response task but inherits the clipped breadcrumb and under-filled Question navigation row. | SUI-01, SUI-03, SUI-04 |
| [question_unanswered_fib.png](../../screenshots/student/phone/question_unanswered_fib.png) | Phone | Fill-in response content remains in the common Question card. The shell and navigation occupy fixed space before the input, reducing the visible prompt/control area. | SUI-01, SUI-03, SUI-04 |
| [question_unanswered_multi_fib.png](../../screenshots/student/phone/question_unanswered_multi_fib.png) | Phone | Multiple fill-in response content makes the vertical task longer while the same three-row shell and clipped breadcrumb remain. | SUI-01, SUI-03, SUI-04 |
| [question_unanswered_num.png](../../screenshots/student/phone/question_unanswered_num.png) | Phone | Numeric response content inherits the same navigation row and phone breadcrumb behavior. The response surface changes, but the shell does not need to grow another identity row. | SUI-01, SUI-03, SUI-04 |
| [question_unanswered_match.png](../../screenshots/student/phone/question_unanswered_match.png) | Phone | Matching response content is a stress case for horizontal space inside a narrow Question card. It should use the available inner width without further shrinking the outer navigation context. | SUI-02, SUI-03, SUI-04 |
| [question_unanswered_order.png](../../screenshots/student/phone/question_unanswered_order.png) | Phone | Ordering response controls remain inside the common Attempt frame. The fixed shell overhead is more consequential because the response area begins lower in the first viewport. | SUI-01, SUI-03, SUI-04 |
| [question_unanswered_hotspot.png](../../screenshots/student/phone/question_unanswered_hotspot.png) | Phone | The hotspot image interaction is a format-specific visual surface within the same Attempt. The phone shell still presents the clipped breadcrumb and sparse Question range. | SUI-01, SUI-03, SUI-04 |
| [question_unanswered_webwork.png](../../screenshots/student/phone/question_unanswered_webwork.png) | Phone | The WeBWorK response surface remains inside the Student Attempt context. The phone capture is still governed by the shared Ribbon, breadcrumb, and Question-navigation constraints. | SUI-01, SUI-03, SUI-04, SUI-06 |

## Question-format captures: phone, answered

| Screenshot | Viewport | Observation | Finding |
| --- | --- | --- | --- |
| [question_answered_mc.png](../../screenshots/student/phone/question_answered_mc.png) | Phone | The saved multiple-choice state adds the selected response and saved feedback without changing the phone shell. The visible Question range remains too sparse for the available width. | SUI-01, SUI-03, SUI-04 |
| [question_answered_ma.png](../../screenshots/student/phone/question_answered_ma.png) | Phone | The saved multiple-answer state preserves the shared shell and response feedback, while inheriting the clipped breadcrumb and sparse number row. | SUI-01, SUI-03, SUI-04 |
| [question_answered_fib.png](../../screenshots/student/phone/question_answered_fib.png) | Phone | The saved fill-in state shows that response feedback can remain near the input, but the shell still consumes fixed vertical space before the work. | SUI-01, SUI-03, SUI-04 |
| [question_answered_multi_fib.png](../../screenshots/student/phone/question_answered_multi_fib.png) | Phone | The saved multiple fill-in state confirms that a taller response body does not require a new heading vocabulary or Ribbon arrangement. | SUI-01, SUI-03, SUI-04, SUI-06 |
| [question_answered_num.png](../../screenshots/student/phone/question_answered_num.png) | Phone | The saved numeric state keeps the response feedback and navigation relationship consistent with other formats. | Working saved-state evidence; SUI-03, SUI-04 |
| [question_answered_match.png](../../screenshots/student/phone/question_answered_match.png) | Phone | The saved matching state is the strongest narrow-width case for preserving inner response width while improving the outer Question range. | SUI-02, SUI-03, SUI-04 |
| [question_answered_order.png](../../screenshots/student/phone/question_answered_order.png) | Phone | The saved ordering state keeps format feedback in the Question area and does not alter the surrounding task identity. | Working saved-state evidence; SUI-01, SUI-03, SUI-04 |
| [question_answered_hotspot.png](../../screenshots/student/phone/question_answered_hotspot.png) | Phone | The saved hotspot state retains the visual response surface and common navigation context. The shell findings are independent of the response format. | SUI-01, SUI-03, SUI-04 |
| [question_answered_webwork.png](../../screenshots/student/phone/question_answered_webwork.png) | Phone | The saved WeBWorK state confirms that the delivered response boundary can retain Student context. Generic `Assessment` wording remains a separate terminology concern. | SUI-03, SUI-04, SUI-06 |

## Additional responsive Question captures

| Screenshot | Viewport | Observation | Finding |
| --- | --- | --- | --- |
| [question_unanswered_mc.png](../../screenshots/student/tablet/question_unanswered_mc.png) | Portrait tablet | The MC Question remains readable in the active Attempt shell at 800 pixels wide. This view is wider than the phone and narrower than the laptop, but it does not by itself prove that the visible Question range scales smoothly through intermediate widths. | SUI-02, SUI-04 |
| [question_unanswered_mc.png](../../screenshots/student/square/question_unanswered_mc.png) | Square | The square MC view preserves the active Attempt identity and Question card while changing the available vertical space. It is useful for checking that the response task does not depend on a tall portrait viewport. | SUI-01, SUI-02, SUI-04 |

## Appendix conclusions

The screenshot-by-screenshot evidence supports five conclusions from the parent
audit:

1. The largest repeated shell issue is not response-format variation. It is the
   shared Ribbon and breadcrumb behavior around every active Attempt capture.
2. The phone Question navigation problem is visible in the native PNGs, not
   only inferred from TypeScript. The row shows too few numbers and leaves
   useful width unused.
3. The Course landing action-order problem is visible in both the Course list
   and Coursework landing family: phone cards move actions below content that
   could be secondary or disclosed.
4. The response-format corpus is valuable because it shows that the Attempt
   identity can remain stable while the Question control changes. Future UI
   work should preserve that contract.
5. Laptop, phone, tablet, and square captures provide a strong visual baseline,
   but they do not replace a fresh runtime walkthrough, keyboard traversal,
   accessibility inspection, or participant evaluation.

## Evidence boundaries

This appendix is a visual/source inspection record. It does not claim:

- that the current Live Demo was freshly replayed during this appendix pass;
- that every responsive breakpoint between 393 and 1280 pixels was inspected;
- that keyboard focus, touch targets, screen-reader names, or browser zoom are
  correct;
- that different Student avatars represent a visual inconsistency, because the
  corpus intentionally uses multiple seeded personas; or
- that dynamic timer values are stable visual content.

The next screenshot pass should preserve these exact file links while adding
the 600-pixel and enlarged-text Question-navigation cases required by SUI-04.
