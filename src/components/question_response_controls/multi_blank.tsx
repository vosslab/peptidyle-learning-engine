// multi_blank.tsx - keyboard-first named text-entry response.

import { createSignal, For, type JSX } from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleQuestionResponseControlKeyDown } from "../question_response_controls/keyboard";
import {
  Actions,
  createResponseController,
  Status,
  textFromBlocks,
  type MultiBlankResponseFormat,
  type QuestionResponseControlBodyProps,
} from "./common";

export function MultiBlankResponse(
  props: QuestionResponseControlBodyProps<MultiBlankResponseFormat>,
): JSX.Element {
  const restored = new Map(
    props.initialResponse?.kind === "multiBlank"
      ? props.initialResponse.answers.map((answer) => [answer.slot, answer.text])
      : [],
  );
  const initialAnswers = props.responseFormat.blanks.map((blank) => ({
    slot: blank.id,
    text: restored.get(blank.id) ?? "",
  }));
  const [answers, setAnswers] = createSignal(initialAnswers);
  let firstBlank!: HTMLInputElement;
  const response = (): StudentResponse => ({ kind: "multiBlank", answers: [...answers()] });
  // Completion is a local progress cue only.
  // It deliberately does not normalize or grade text.
  const completedBlankCount = (): number =>
    answers().filter((answer) => answer.text.length > 0).length;
  const controller = createResponseController(props, response());
  function update(slot: string, text: string): void {
    const next = answers().map((answer) => (answer.slot === slot ? { ...answer, text } : answer));
    setAnswers(next);
    void controller.edit({ kind: "multiBlank", answers: [...next] });
  }
  function save(): void {
    void controller.save(response());
  }
  function reset(): void {
    const next = initialAnswers.map((answer) => ({ ...answer }));
    setAnswers(next);
    void controller.reset({ kind: "multiBlank", answers: next });
    queueMicrotask(() => firstBlank.focus());
  }
  return (
    <section
      class="question-response-control"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleQuestionResponseControlKeyDown(event, props.onEscape, save, controller.canSave)
      }
    >
      <fieldset
        aria-describedby={`${props.attemptId}-multi-blank-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.locked()}
      >
        <legend>Complete each blank</legend>
        <p class="keyboard-instructions" id={`${props.attemptId}-multi-blank-help`}>
          Use Tab and Shift+Tab to move between blanks. Type a response in each blank.
        </p>
        <p
          class="completion-progress"
          role="status"
          aria-label="Blank completion"
          aria-live="polite"
        >
          {completedBlankCount()} of {props.responseFormat.blanks.length} blanks completed.
        </p>
        <div class="response-fields">
          <For each={props.responseFormat.blanks}>
            {(blank, index) => (
              <label for={`${props.attemptId}-blank-${index()}`}>
                {textFromBlocks(blank.label)}
                <input
                  id={`${props.attemptId}-blank-${index()}`}
                  class="question-response-control__input"
                  type="text"
                  maxlength={"maxCharacters" in blank ? blank.maxCharacters : blank.maxLength}
                  value={answers().find((answer) => answer.slot === blank.id)?.text ?? ""}
                  ref={
                    index() === 0
                      ? (element): void => {
                          firstBlank = element;
                        }
                      : undefined
                  }
                  onInput={(event) => update(blank.id, event.currentTarget.value)}
                />
              </label>
            )}
          </For>
        </div>
      </fieldset>
      <Status attemptId={props.attemptId} controller={controller} />
      <Actions
        disabled={!controller.canSave() || controller.locked()}
        resetDisabled={controller.locked()}
        onSave={save}
        saveLabel={props.saveLabel}
        onReset={reset}
        onEscape={props.onEscape}
      />
    </section>
  );
}
