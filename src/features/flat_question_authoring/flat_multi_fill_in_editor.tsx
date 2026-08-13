// flat_multi_fill_in_editor.tsx - compact private controls for MULTI-FIB blanks.

import { For, Show, type JSX } from "solid-js";

import type { FlatQuestionBlank, FlatQuestionTextMatchMode } from "./flat_question_source";

const MAX_BLANKS = 50;

export interface FlatMultiFillInEditorProps {
  readonly blanks: () => ReadonlyArray<FlatQuestionBlank>;
  readonly onBlankLabelChange: (blankId: string, label: string) => void;
  readonly onBlankMatchModeChange: (blankId: string, matchMode: FlatQuestionTextMatchMode) => void;
  readonly onBlankMaxLengthChange: (blankId: string, maxLength: number) => void;
  readonly onAnswerChange: (blankId: string, answerIndex: number, answer: string) => void;
  readonly onAddAnswer: (blankId: string) => void;
  readonly onRemoveAnswer: (blankId: string, answerIndex: number) => void;
  readonly onAddBlank: () => void;
  readonly onRemoveBlank: (blankId: string) => void;
  readonly onMoveBlank: (blankId: string, direction: "earlier" | "later") => void;
  readonly onStatus?: (message: string) => void;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
  readonly disabled?: boolean;
}

function errorFor(props: FlatMultiFillInEditorProps, path: string): string | undefined {
  return props.fieldErrors?.[path];
}

function isTextMatchMode(value: string): value is FlatQuestionTextMatchMode {
  return value === "exact" || value === "caseInsensitive" || value === "normalized";
}

/** Keeps each blank's label, accepted answers, and matching rule together while it moves. */
export function FlatMultiFillInEditor(props: FlatMultiFillInEditorProps): JSX.Element {
  function announce(message: string): void {
    props.onStatus?.(message);
  }

  return (
    <fieldset class="flat-question-authoring__multi-fill-in">
      <legend>Multiple fill in the blank</legend>
      <p class="flat-question-authoring__help">
        Define each blank once. Its label, accepted answers, and comparison rule stay together when
        you change the reading order.
      </p>
      <Show when={errorFor(props, "response.blanks") !== undefined}>
        <p class="flat-question-authoring__error" role="alert">
          {errorFor(props, "response.blanks")}
        </p>
      </Show>
      <ol class="flat-question-authoring__choice-list">
        <For each={props.blanks().map((blank) => blank.id)}>
          {(blankId, index): JSX.Element => {
            const blank = (): FlatQuestionBlank => {
              const current = props.blanks().find((candidate) => candidate.id === blankId);
              if (current === undefined) throw new Error("Blank identity is unavailable.");
              return current;
            };
            const blankPath = `response.blanks.${blankId}`;
            const labelError = (): string | undefined => errorFor(props, `${blankPath}.label`);
            const answerError = (): string | undefined => errorFor(props, `${blankPath}.answers`);
            return (
              <li class="flat-question-authoring__choice">
                <div class="flat-question-authoring__choice-header">
                  <h3 class="flat-question-authoring__choice-title">Blank {index() + 1}</h3>
                  <div
                    class="flat-question-authoring__row-actions"
                    aria-label={`Blank ${index() + 1} actions`}
                  >
                    <button
                      type="button"
                      class="quiet-action"
                      disabled={props.disabled || index() === 0}
                      onClick={() => {
                        props.onMoveBlank(blankId, "earlier");
                        announce(`Moved blank ${index() + 1} earlier.`);
                      }}
                    >
                      Earlier
                    </button>
                    <button
                      type="button"
                      class="quiet-action"
                      disabled={props.disabled || index() === props.blanks().length - 1}
                      onClick={() => {
                        props.onMoveBlank(blankId, "later");
                        announce(`Moved blank ${index() + 1} later.`);
                      }}
                    >
                      Later
                    </button>
                    <button
                      type="button"
                      class="quiet-action"
                      disabled={props.disabled || props.blanks().length <= 1}
                      onClick={() => props.onRemoveBlank(blankId)}
                    >
                      Remove blank
                    </button>
                  </div>
                </div>
                <label class="flat-question-authoring__field">
                  <span>Visible blank label</span>
                  <input
                    value={blank().label}
                    disabled={props.disabled}
                    aria-invalid={labelError() !== undefined}
                    aria-describedby={
                      labelError() === undefined ? undefined : `${blankId}-label-error`
                    }
                    onInput={(event) =>
                      props.onBlankLabelChange(blankId, event.currentTarget.value)
                    }
                  />
                </label>
                <Show when={labelError() !== undefined}>
                  <p
                    class="flat-question-authoring__error"
                    id={`${blankId}-label-error`}
                    role="alert"
                  >
                    {labelError()}
                  </p>
                </Show>
                <div class="flat-question-authoring__grid">
                  <label class="flat-question-authoring__field">
                    <span>Answer comparison</span>
                    <select
                      value={blank().matchMode}
                      disabled={props.disabled}
                      onChange={(event) => {
                        const matchMode = event.currentTarget.value;
                        if (isTextMatchMode(matchMode)) {
                          props.onBlankMatchModeChange(blankId, matchMode);
                        }
                      }}
                    >
                      <option value="exact">Exact</option>
                      <option value="caseInsensitive">Ignore capitalization</option>
                      <option value="normalized">Normalize spacing and capitalization</option>
                    </select>
                  </label>
                  <label class="flat-question-authoring__field">
                    <span>Maximum answer length</span>
                    <input
                      type="number"
                      min="1"
                      max="16384"
                      value={blank().maxLength}
                      disabled={props.disabled}
                      onInput={(event) =>
                        props.onBlankMaxLengthChange(blankId, Number(event.currentTarget.value))
                      }
                    />
                  </label>
                </div>
                <section aria-labelledby={`${blankId}-answers-heading`}>
                  <h4 id={`${blankId}-answers-heading`}>Accepted answers</h4>
                  <p class="flat-question-authoring__help">
                    Add acceptable equivalents for this specific blank. Learners never see this
                    list.
                  </p>
                  <Show when={answerError() !== undefined}>
                    <p class="flat-question-authoring__error" role="alert">
                      {answerError()}
                    </p>
                  </Show>
                  <For each={blank().answers}>
                    {(answer, answerIndex): JSX.Element => (
                      <div class="flat-question-authoring__row-actions">
                        <label class="flat-question-authoring__field">
                          <span class="sr-only">Accepted answer {answerIndex() + 1}</span>
                          <input
                            value={answer}
                            maxlength={blank().maxLength}
                            disabled={props.disabled}
                            aria-label={`Accepted answer ${answerIndex() + 1} for blank ${index() + 1}`}
                            onInput={(event) =>
                              props.onAnswerChange(
                                blankId,
                                answerIndex(),
                                event.currentTarget.value,
                              )
                            }
                          />
                        </label>
                        <button
                          type="button"
                          class="quiet-action"
                          disabled={props.disabled || blank().answers.length <= 1}
                          onClick={() => props.onRemoveAnswer(blankId, answerIndex())}
                        >
                          Remove answer
                        </button>
                      </div>
                    )}
                  </For>
                  <button
                    type="button"
                    class="quiet-action"
                    disabled={props.disabled}
                    onClick={() => props.onAddAnswer(blankId)}
                  >
                    Add accepted answer
                  </button>
                </section>
                <details>
                  <summary>Advanced blank identity</summary>
                  <p class="flat-question-authoring__identity">{blankId}</p>
                </details>
              </li>
            );
          }}
        </For>
      </ol>
      <div class="flat-question-authoring__actions">
        <button
          type="button"
          class="quiet-action"
          disabled={props.disabled || props.blanks().length >= MAX_BLANKS}
          onClick={props.onAddBlank}
        >
          Add blank
        </button>
        <p class="flat-question-authoring__help">
          {props.blanks().length} of {MAX_BLANKS} blanks
        </p>
      </div>
    </fieldset>
  );
}
