// blueprint_assessment_feedback_fields.tsx - reusable Student feedback disclosure controls.

import { For, Show, type JSX } from "solid-js";

import type { BlueprintAssessmentContentInput } from "../../../generated/api/BlueprintAssessmentContentInput";
import type { StudentFeedbackReleaseRule } from "../../../generated/api/StudentFeedbackReleaseRule";
import type { StudentFeedbackReleaseTiming } from "../../../generated/api/StudentFeedbackReleaseTiming";
import { updateReusableDefaults } from "./blueprint_course_model";

const FEEDBACK_FIELDS = [
  ["score", "Score", ""],
  ["per_item_correctness", "Per-item correctness", ""],
  [
    "submitted_response",
    "Previous-attempt response",
    "Controls the Student's recorded response in previous attempts. Never leaves correctness visible when Per-item correctness permits it.",
  ],
  ["question_answer", "Correct answer", ""],
  ["question_answer_explanation", "Question answer explanation", ""],
  [
    "class_statistics",
    "Class statistics",
    "Default: Never. Choose a later timing only when sharing class statistics is appropriate.",
  ],
] as const satisfies ReadonlyArray<readonly [keyof StudentFeedbackReleaseRule, string, string]>;

const FEEDBACK_TIMINGS = [
  ["during_attempt", "During attempt"],
  ["after_submit", "After submit"],
  ["after_due", "After due"],
  ["after_close", "After close"],
  ["never", "Never"],
] as const satisfies ReadonlyArray<readonly [StudentFeedbackReleaseTiming, string]>;

export interface BlueprintAssessmentFeedbackFieldsProps {
  readonly content: BlueprintAssessmentContentInput;
  readonly editable: boolean;
  readonly onChange: (content: BlueprintAssessmentContentInput, message: string) => void;
}

/** Keeps all six disclosure fields independent and uses the existing local draft owner. */
export function BlueprintAssessmentFeedbackFields(
  props: BlueprintAssessmentFeedbackFieldsProps,
): JSX.Element {
  function changeTiming(field: keyof StudentFeedbackReleaseRule, value: string): void {
    // ASVS 2.2.1: allow only contract timings; trusted server validation remains authoritative.
    const timing = FEEDBACK_TIMINGS.find(([candidate]) => candidate === value)?.[0];
    if (!props.editable || timing === undefined) return;
    const defaults = {
      ...props.content.defaults,
      student_feedback_release_rule: {
        ...props.content.defaults.student_feedback_release_rule,
        [field]: timing,
      },
    };
    props.onChange(
      updateReusableDefaults(props.content, defaults),
      "Student feedback defaults updated. Save the Blueprint Course to keep these changes.",
    );
  }

  return (
    <fieldset disabled={!props.editable}>
      <legend>What Students can see</legend>
      <p class="blueprint-course-field-help">
        Each field has its own disclosure timing. After due and After close use dates set in the
        daughter Course Instance, not dates in this Blueprint Assessment.
      </p>
      <Show
        when={props.content.assessment_type === "quiz" || props.content.assessment_type === "exam"}
      >
        <p class="blueprint-course-field-help">
          Quiz and Exam correct answers and answer explanations also remain hidden until all current
          Students in the Course have completed the Assessment.
        </p>
      </Show>
      <div class="blueprint-course-form-grid">
        <For each={FEEDBACK_FIELDS}>
          {([field, label, help]) => (
            <label>
              {label}
              <select
                value={props.content.defaults.student_feedback_release_rule[field]}
                onChange={(event) => changeTiming(field, event.currentTarget.value)}
              >
                <For each={FEEDBACK_TIMINGS}>
                  {([value, timingLabel]) => <option value={value}>{timingLabel}</option>}
                </For>
              </select>
              <Show when={help}>
                <small>{help}</small>
              </Show>
            </label>
          )}
        </For>
      </div>
    </fieldset>
  );
}
