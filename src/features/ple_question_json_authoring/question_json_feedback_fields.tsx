// question_json_feedback_fields.tsx - author-only outcome feedback controls.

import { Show, type JSX } from "solid-js";

import type { PleQuestionJsonOutcomeFeedback } from "./question_json_source";

export interface PleQuestionJsonFeedbackFieldsProps {
  readonly value: PleQuestionJsonOutcomeFeedback;
  readonly onChange: (patch: Partial<PleQuestionJsonOutcomeFeedback>) => void;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
  readonly disabled?: boolean;
}

/** Correct/incorrect outcome feedback complements, but does not replace, per-choice feedback. */
export function PleQuestionJsonFeedbackFields(
  props: PleQuestionJsonFeedbackFieldsProps,
): JSX.Element {
  const correctError = (): string | undefined => props.fieldErrors?.["feedback.correct"];
  const incorrectError = (): string | undefined => props.fieldErrors?.["feedback.incorrect"];
  return (
    <fieldset>
      <legend>Question Feedback</legend>
      <p class="ple-question-json-authoring__help">
        This appears after the student's answer according to the Student Feedback Release Rule.
      </p>
      <label class="ple-question-json-authoring__field">
        <span>Correct Feedback (optional)</span>
        <textarea
          value={props.value.correct ?? ""}
          disabled={props.disabled}
          aria-invalid={correctError() !== undefined}
          aria-describedby={
            correctError() === undefined ? undefined : "ple-question-json-correct-feedback-error"
          }
          onInput={(event) =>
            props.onChange({
              correct: event.currentTarget.value.trim() === "" ? null : event.currentTarget.value,
            })
          }
        />
      </label>
      <Show when={correctError() !== undefined}>
        <p
          class="ple-question-json-authoring__error"
          id="ple-question-json-correct-feedback-error"
          role="alert"
        >
          {correctError()}
        </p>
      </Show>
      <label class="ple-question-json-authoring__field">
        <span>Incorrect Feedback (optional)</span>
        <textarea
          value={props.value.incorrect ?? ""}
          disabled={props.disabled}
          aria-invalid={incorrectError() !== undefined}
          aria-describedby={
            incorrectError() === undefined
              ? undefined
              : "ple-question-json-incorrect-feedback-error"
          }
          onInput={(event) =>
            props.onChange({
              incorrect: event.currentTarget.value.trim() === "" ? null : event.currentTarget.value,
            })
          }
        />
      </label>
      <Show when={incorrectError() !== undefined}>
        <p
          class="ple-question-json-authoring__error"
          id="ple-question-json-incorrect-feedback-error"
          role="alert"
        >
          {incorrectError()}
        </p>
      </Show>
    </fieldset>
  );
}
