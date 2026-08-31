// flat_choice_list.tsx - accessible editing controls for a single-choice question's options.

import { For, Show, type JSX } from "solid-js";

import type { FlatQuestionChoice } from "./flat_question_source";

const MINIMUM_CHOICES = 2;
const MAXIMUM_CHOICES = 100;

export interface FlatChoiceListProps {
  readonly choices: ReadonlyArray<FlatQuestionChoice>;
  readonly correctChoice: string;
  readonly onChoiceChange: (
    choiceId: string,
    patch: Partial<Pick<FlatQuestionChoice, "text" | "feedback">>,
  ) => void;
  readonly onCorrectChoiceChange: (choiceId: string) => void;
  readonly onAddChoice: () => void;
  readonly onRemoveChoice: (choiceId: string) => void;
  readonly onMoveChoice: (choiceId: string, direction: "up" | "down") => void;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
  readonly disabled?: boolean;
}

function errorFor(props: FlatChoiceListProps, path: string): string | undefined {
  return props.fieldErrors?.[path];
}

/** Uses the source's stable semantic IDs, so reordering never changes a response identity. */
export function FlatChoiceList(props: FlatChoiceListProps): JSX.Element {
  const groupId = "flat-question-correct-choice";
  return (
    <fieldset class="flat-question-authoring__choices">
      <legend>Answer choices</legend>
      <p class="flat-question-authoring__help" id={`${groupId}-help`}>
        Choose one correct answer. Students see choice text only; teaching feedback stays in the
        authoring workflow.
      </p>
      <Show when={errorFor(props, "choices") !== undefined}>
        <p class="flat-question-authoring__error" role="alert">
          {errorFor(props, "choices")}
        </p>
      </Show>
      <div
        role="radiogroup"
        aria-labelledby={`${groupId}-label`}
        aria-describedby={`${groupId}-help`}
      >
        <span id={`${groupId}-label`} class="sr-only">
          Correct answer
        </span>
        <ol class="flat-question-authoring__choice-list">
          <For each={props.choices}>
            {(choice, index): JSX.Element => {
              const choicePath = `choices.${choice.id}`;
              const textError = (): string | undefined => errorFor(props, `${choicePath}.text`);
              const feedbackError = (): string | undefined =>
                errorFor(props, `${choicePath}.feedback`);
              return (
                <li class="flat-question-authoring__choice">
                  <div class="flat-question-authoring__choice-header">
                    <h3 class="flat-question-authoring__choice-title">Choice {index() + 1}</h3>
                    <div
                      class="flat-question-authoring__row-actions"
                      aria-label={`Choice ${index() + 1} actions`}
                    >
                      <button
                        type="button"
                        class="quiet-action"
                        disabled={props.disabled || index() === 0}
                        onClick={() => props.onMoveChoice(choice.id, "up")}
                      >
                        Move up
                      </button>
                      <button
                        type="button"
                        class="quiet-action"
                        disabled={props.disabled || index() === props.choices.length - 1}
                        onClick={() => props.onMoveChoice(choice.id, "down")}
                      >
                        Move down
                      </button>
                      <button
                        type="button"
                        class="quiet-action"
                        disabled={props.disabled || props.choices.length <= MINIMUM_CHOICES}
                        onClick={() => props.onRemoveChoice(choice.id)}
                      >
                        Remove
                      </button>
                    </div>
                  </div>
                  <label class="flat-question-authoring__field">
                    <span>Choice text</span>
                    <textarea
                      value={choice.text}
                      disabled={props.disabled}
                      aria-invalid={textError() !== undefined}
                      aria-describedby={
                        textError() === undefined ? undefined : `${choice.id}-text-error`
                      }
                      onInput={(event) =>
                        props.onChoiceChange(choice.id, { text: event.currentTarget.value })
                      }
                    />
                  </label>
                  <Show when={textError() !== undefined}>
                    <p
                      class="flat-question-authoring__error"
                      id={`${choice.id}-text-error`}
                      role="alert"
                    >
                      {textError()}
                    </p>
                  </Show>
                  <label class="flat-question-authoring__field">
                    <span>Teaching feedback for this choice (optional)</span>
                    <textarea
                      value={choice.feedback ?? ""}
                      disabled={props.disabled}
                      aria-invalid={feedbackError() !== undefined}
                      aria-describedby={
                        feedbackError() === undefined ? undefined : `${choice.id}-feedback-error`
                      }
                      onInput={(event) =>
                        props.onChoiceChange(choice.id, {
                          feedback:
                            event.currentTarget.value.trim() === ""
                              ? null
                              : event.currentTarget.value,
                        })
                      }
                    />
                  </label>
                  <Show when={feedbackError() !== undefined}>
                    <p
                      class="flat-question-authoring__error"
                      id={`${choice.id}-feedback-error`}
                      role="alert"
                    >
                      {feedbackError()}
                    </p>
                  </Show>
                  <label class="flat-question-authoring__field">
                    <span>
                      <input
                        type="radio"
                        name={groupId}
                        aria-label={`Mark choice ${index() + 1} as correct: ${choice.text}`}
                        checked={choice.id === props.correctChoice}
                        disabled={props.disabled}
                        onChange={() => props.onCorrectChoiceChange(choice.id)}
                      />{" "}
                      Correct answer
                    </span>
                  </label>
                  <details>
                    <summary>Advanced choice identity</summary>
                    <p class="flat-question-authoring__identity">{choice.id}</p>
                  </details>
                </li>
              );
            }}
          </For>
        </ol>
      </div>
      <div class="flat-question-authoring__actions">
        <button
          type="button"
          class="quiet-action"
          disabled={props.disabled || props.choices.length >= MAXIMUM_CHOICES}
          onClick={props.onAddChoice}
        >
          Add choice
        </button>
        <p class="flat-question-authoring__help">
          {props.choices.length} of {MAXIMUM_CHOICES} choices
        </p>
      </div>
    </fieldset>
  );
}
