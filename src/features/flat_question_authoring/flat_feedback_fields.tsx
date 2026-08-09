// flat_feedback_fields.tsx - author-only outcome feedback controls.

import { Show, type JSX } from "solid-js";

import type { FlatQuestionOutcomeFeedback } from "./flat_question_source";

export interface FlatFeedbackFieldsProps {
  readonly value: FlatQuestionOutcomeFeedback;
  readonly onChange: (patch: Partial<FlatQuestionOutcomeFeedback>) => void;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
  readonly disabled?: boolean;
}

/** Correct/incorrect outcome feedback complements, but does not replace, per-choice feedback. */
export function FlatFeedbackFields(props: FlatFeedbackFieldsProps): JSX.Element {
  const correctError = (): string | undefined => props.fieldErrors?.["feedback.correct"];
  const incorrectError = (): string | undefined => props.fieldErrors?.["feedback.incorrect"];
  return (
    <fieldset>
      <legend>Outcome feedback</legend>
      <p class="flat-question-authoring__help">
        This appears after the learner's answer according to the feedback policy.
      </p>
      <label class="flat-question-authoring__field">
        <span>Correct-answer feedback (optional)</span>
        <textarea
          value={props.value.correct ?? ""}
          disabled={props.disabled}
          aria-invalid={correctError() !== undefined}
          aria-describedby={
            correctError() === undefined ? undefined : "flat-correct-feedback-error"
          }
          onInput={(event) =>
            props.onChange({
              correct: event.currentTarget.value.trim() === "" ? null : event.currentTarget.value,
            })
          }
        />
      </label>
      <Show when={correctError() !== undefined}>
        <p class="flat-question-authoring__error" id="flat-correct-feedback-error" role="alert">
          {correctError()}
        </p>
      </Show>
      <label class="flat-question-authoring__field">
        <span>Incorrect-answer feedback (optional)</span>
        <textarea
          value={props.value.incorrect ?? ""}
          disabled={props.disabled}
          aria-invalid={incorrectError() !== undefined}
          aria-describedby={
            incorrectError() === undefined ? undefined : "flat-incorrect-feedback-error"
          }
          onInput={(event) =>
            props.onChange({
              incorrect: event.currentTarget.value.trim() === "" ? null : event.currentTarget.value,
            })
          }
        />
      </label>
      <Show when={incorrectError() !== undefined}>
        <p class="flat-question-authoring__error" id="flat-incorrect-feedback-error" role="alert">
          {incorrectError()}
        </p>
      </Show>
    </fieldset>
  );
}
