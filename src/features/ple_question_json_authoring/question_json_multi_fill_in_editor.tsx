// question_json_multi_fill_in_editor.tsx - compact private controls for MULTI-FIB blanks.

import { For, Show, type JSX } from "solid-js";

import type {
  PleQuestionJsonBlank,
  PleQuestionJsonTextResponseMatchRule,
} from "./question_json_source";

const MAX_BLANKS = 50;

export interface PleQuestionJsonMultiFillInEditorProps {
  readonly blanks: () => ReadonlyArray<PleQuestionJsonBlank>;
  readonly onBlankLabelChange: (blankId: string, label: string) => void;
  readonly onBlankMatchModeChange: (
    blankId: string,
    matchMode: PleQuestionJsonTextResponseMatchRule,
  ) => void;
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

function errorFor(props: PleQuestionJsonMultiFillInEditorProps, path: string): string | undefined {
  return props.fieldErrors?.[path];
}

function isTextResponseMatchRule(value: string): value is PleQuestionJsonTextResponseMatchRule {
  return value === "exact" || value === "caseInsensitive" || value === "normalized";
}

/** Keeps each blank's label, accepted answers, and matching rule together while it moves. */
export function PleQuestionJsonMultiFillInEditor(
  props: PleQuestionJsonMultiFillInEditorProps,
): JSX.Element {
  function announce(message: string): void {
    props.onStatus?.(message);
  }

  return (
    <fieldset class="ple-question-json-authoring__multi-fill-in">
      <legend>Multiple fill in the blank</legend>
      <p class="ple-question-json-authoring__help">
        Define each blank once. Its label, accepted answers, and comparison rule stay together when
        you change the reading order.
      </p>
      <Show when={errorFor(props, "response.blanks") !== undefined}>
        <p class="ple-question-json-authoring__error" role="alert">
          {errorFor(props, "response.blanks")}
        </p>
      </Show>
      <ol class="ple-question-json-authoring__choice-list">
        <For each={props.blanks().map((blank) => blank.id)}>
          {(blankId, index): JSX.Element => {
            const blank = (): PleQuestionJsonBlank => {
              const current = props.blanks().find((candidate) => candidate.id === blankId);
              if (current === undefined) throw new Error("Blank identity is unavailable.");
              return current;
            };
            const blankPath = `response.blanks.${blankId}`;
            const labelError = (): string | undefined => errorFor(props, `${blankPath}.label`);
            const answerError = (): string | undefined => errorFor(props, `${blankPath}.answers`);
            return (
              <li class="ple-question-json-authoring__choice">
                <div class="ple-question-json-authoring__choice-header">
                  <h3 class="ple-question-json-authoring__choice-title">Blank {index() + 1}</h3>
                  <div
                    class="ple-question-json-authoring__row-actions"
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
                <label class="ple-question-json-authoring__field">
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
                    class="ple-question-json-authoring__error"
                    id={`${blankId}-label-error`}
                    role="alert"
                  >
                    {labelError()}
                  </p>
                </Show>
                <div class="ple-question-json-authoring__grid">
                  <label class="ple-question-json-authoring__field">
                    <span>Answer comparison</span>
                    <select
                      value={blank().matchMode}
                      disabled={props.disabled}
                      onChange={(event) => {
                        const matchMode = event.currentTarget.value;
                        if (isTextResponseMatchRule(matchMode)) {
                          props.onBlankMatchModeChange(blankId, matchMode);
                        }
                      }}
                    >
                      <option value="exact">Exact</option>
                      <option value="caseInsensitive">Ignore capitalization</option>
                      <option value="normalized">Normalize spacing and capitalization</option>
                    </select>
                  </label>
                  <label class="ple-question-json-authoring__field">
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
                  <p class="ple-question-json-authoring__help">
                    Add acceptable equivalents for this specific blank. Students never see this
                    list.
                  </p>
                  <Show when={answerError() !== undefined}>
                    <p class="ple-question-json-authoring__error" role="alert">
                      {answerError()}
                    </p>
                  </Show>
                  <For each={blank().answers}>
                    {(answer, answerIndex): JSX.Element => (
                      <div class="ple-question-json-authoring__row-actions">
                        <label class="ple-question-json-authoring__field">
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
                  <p class="ple-question-json-authoring__identity">{blankId}</p>
                </details>
              </li>
            );
          }}
        </For>
      </ol>
      <div class="ple-question-json-authoring__actions">
        <button
          type="button"
          class="quiet-action"
          disabled={props.disabled || props.blanks().length >= MAX_BLANKS}
          onClick={props.onAddBlank}
        >
          Add blank
        </button>
        <p class="ple-question-json-authoring__help">
          {props.blanks().length} of {MAX_BLANKS} blanks
        </p>
      </div>
    </fieldset>
  );
}
