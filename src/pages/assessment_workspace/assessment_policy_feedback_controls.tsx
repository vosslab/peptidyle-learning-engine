import { For, Show, type JSX } from "solid-js";

import type { LiveAssessmentWorkspace } from "../../api/assessment_release";

const FEEDBACK_FIELDS = [
  ["question_answer", "Correct answer"],
  ["question_answer_explanation", "Question answer explanation"],
  [
    "class_statistics",
    "Class statistics",
    "Default: Never. Choose a later timing only when sharing class statistics is appropriate.",
  ],
  ["hints", "Hints", "Default: Never. This timing is separate from correct-answer disclosure."],
  [
    "worked_solutions",
    "Worked Solutions",
    "Default: Never. This timing is separate from Hints and from correct-answer disclosure.",
  ],
] as const;

export type AssessmentFeedbackField = (typeof FEEDBACK_FIELDS)[number][0];

type AssessmentPolicyFeedbackControlsProps = {
  readonly rules: LiveAssessmentWorkspace["studentFeedbackReleaseRule"];
  readonly onChange: (field: AssessmentFeedbackField, value: string) => void;
};

/** Presents the independently timed Student feedback settings. */
export function AssessmentPolicyFeedbackControls(
  props: AssessmentPolicyFeedbackControlsProps,
): JSX.Element {
  return (
    <section class="assessment-editor-policy-panel">
      <h2>Student feedback</h2>
      <For each={FEEDBACK_FIELDS}>
        {([field, label, help]) => (
          <label class="assessment-editor-field">
            {label}
            <select
              value={props.rules[field]}
              aria-describedby={help === undefined ? undefined : `feedback-${field}-help`}
              onChange={(event) => {
                props.onChange(field, event.currentTarget.value);
              }}
            >
              <option value="during_attempt">During attempt</option>
              <option value="after_submit">After submit</option>
              <option value="after_due">After due</option>
              <option value="after_close">After close</option>
              <Show when={field === "question_answer" || field === "question_answer_explanation"}>
                <option value="after_all_students_complete">After all Students complete</option>
              </Show>
              <option value="never">Never</option>
            </select>
            <Show when={help !== undefined}>
              <small id={`feedback-${field}-help`}>{help}</small>
            </Show>
          </label>
        )}
      </For>
    </section>
  );
}
