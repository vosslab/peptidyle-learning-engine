// question_json_multiple_answer_editor.tsx - compact protected controls for multiple-answer questions.

import { For, Show, type JSX } from "solid-js";

import {
  MAXIMUM_MULTIPLE_ANSWER_CHOICES,
  MINIMUM_MULTIPLE_ANSWER_CHOICES,
  validateMultipleAnswerResponse,
  type MultipleAnswerValidation,
} from "./question_json_multiple_answer_model";
import type {
  PleQuestionJsonChoice,
  PleQuestionJsonMultipleAnswerResponse,
} from "./question_json_source";

export interface PleQuestionJsonMultipleAnswerEditorProps {
  readonly response: () => PleQuestionJsonMultipleAnswerResponse;
  readonly onChoiceTextChange: (choiceId: string, text: string) => void;
  readonly onChoiceFeedbackChange: (choiceId: string, feedback: string | null) => void;
  readonly onCorrectChoiceChange: (choiceId: string, correct: boolean) => void;
  readonly onAddChoice: () => void;
  readonly onRemoveChoice: (choiceId: string) => void;
  readonly onMoveChoice: (choiceId: string, direction: "earlier" | "later") => void;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
  readonly disabled?: boolean;
}

function errorFor(
  props: PleQuestionJsonMultipleAnswerEditorProps,
  localErrors: MultipleAnswerValidation,
  path: string,
): string | undefined {
  return props.fieldErrors?.[path] ?? localErrors[path];
}

function choiceTextError(
  props: PleQuestionJsonMultipleAnswerEditorProps,
  choice: PleQuestionJsonChoice,
): string | undefined {
  return props.fieldErrors?.[`choices.${choice.id}.text`];
}

/**
 * Uses stable choice IDs with Solid's identity-aware For, so typing or moving one choice cannot
 * change another choice's checkbox state.
 */
export function PleQuestionJsonMultipleAnswerEditor(
  props: PleQuestionJsonMultipleAnswerEditorProps,
): JSX.Element {
  const localErrors = (): MultipleAnswerValidation =>
    validateMultipleAnswerResponse(props.response());
  const correctError = (): string | undefined => errorFor(props, localErrors(), "correctChoices");
  const choicesError = (): string | undefined => errorFor(props, localErrors(), "choices");

  return (
    <fieldset class="ple-question-json-authoring__choices">
      <legend>Multiple-answer choices</legend>
      <p class="ple-question-json-authoring__help" id="ple-question-json-multiple-answer-help">
        Mark every answer a student must select. Each checkbox is private authoring information;
        students receive only the choice text.
      </p>
      <Show when={choicesError() !== undefined}>
        <p class="ple-question-json-authoring__error" role="alert">
          {choicesError()}
        </p>
      </Show>
      <Show when={correctError() !== undefined}>
        <p class="ple-question-json-authoring__error" role="alert">
          {correctError()}
        </p>
      </Show>
      <ol
        class="ple-question-json-authoring__choice-list"
        aria-describedby="ple-question-json-multiple-answer-help"
      >
        <For each={props.response().choices.map((choice) => choice.id)}>
          {(choiceId, index): JSX.Element => {
            const choice = (): PleQuestionJsonChoice => {
              const current = props
                .response()
                .choices.find((candidate) => candidate.id === choiceId);
              if (current === undefined) throw new Error("Choice identity is unavailable.");
              return current;
            };
            const textError = (): string | undefined => choiceTextError(props, choice());
            const correct = (): boolean => props.response().correctChoices.includes(choiceId);
            return (
              <li class="ple-question-json-authoring__choice">
                <div class="ple-question-json-authoring__choice-header">
                  <h3 class="ple-question-json-authoring__choice-title">Choice {index() + 1}</h3>
                  <div
                    class="ple-question-json-authoring__row-actions"
                    aria-label={`Choice ${index() + 1} actions`}
                  >
                    <button
                      type="button"
                      class="quiet-action"
                      disabled={props.disabled || index() === 0}
                      onClick={() => props.onMoveChoice(choiceId, "earlier")}
                    >
                      Earlier
                    </button>
                    <button
                      type="button"
                      class="quiet-action"
                      disabled={props.disabled || index() === props.response().choices.length - 1}
                      onClick={() => props.onMoveChoice(choiceId, "later")}
                    >
                      Later
                    </button>
                    <button
                      type="button"
                      class="quiet-action"
                      disabled={
                        props.disabled ||
                        props.response().choices.length <= MINIMUM_MULTIPLE_ANSWER_CHOICES
                      }
                      onClick={() => props.onRemoveChoice(choiceId)}
                    >
                      Remove
                    </button>
                  </div>
                </div>
                <label class="ple-question-json-authoring__field">
                  <span>Choice text</span>
                  <textarea
                    value={choice().text}
                    disabled={props.disabled}
                    aria-invalid={textError() !== undefined}
                    aria-describedby={
                      textError() === undefined ? undefined : `${choiceId}-multiple-text-error`
                    }
                    onInput={(event) =>
                      props.onChoiceTextChange(choiceId, event.currentTarget.value)
                    }
                  />
                </label>
                <Show when={textError() !== undefined}>
                  <p
                    class="ple-question-json-authoring__error"
                    id={`${choiceId}-multiple-text-error`}
                    role="alert"
                  >
                    {textError()}
                  </p>
                </Show>
                <label class="ple-question-json-authoring__field">
                  <span>Choice Feedback (optional)</span>
                  <textarea
                    value={choice().feedback ?? ""}
                    disabled={props.disabled}
                    onInput={(event) => {
                      const value = event.currentTarget.value;
                      props.onChoiceFeedbackChange(choiceId, value.trim() === "" ? null : value);
                    }}
                  />
                </label>
                <label class="ple-question-json-authoring__field">
                  <span>
                    <input
                      type="checkbox"
                      checked={correct()}
                      disabled={props.disabled}
                      onChange={(event) =>
                        props.onCorrectChoiceChange(choiceId, event.currentTarget.checked)
                      }
                    />{" "}
                    Correct answer
                  </span>
                </label>
              </li>
            );
          }}
        </For>
      </ol>
      <div class="ple-question-json-authoring__actions">
        <button
          type="button"
          class="quiet-action"
          disabled={
            props.disabled || props.response().choices.length >= MAXIMUM_MULTIPLE_ANSWER_CHOICES
          }
          onClick={props.onAddChoice}
        >
          Add choice
        </button>
        <p class="ple-question-json-authoring__help">
          {props.response().choices.length} of {MAXIMUM_MULTIPLE_ANSWER_CHOICES} choices
        </p>
      </div>
    </fieldset>
  );
}
