// multi_blank.tsx - keyboard-first named text-entry response.

import { createSignal, For, type JSX } from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleQuestionResponseControlKeyDown } from "../question_response_controls/keyboard";
import {
  Actions,
  createSubmissionController,
  Status,
  textFromBlocks,
  type MultiBlankDefinition,
  type QuestionResponseControlBodyProps,
} from "./common";

export function MultiBlankResponse(
  props: QuestionResponseControlBodyProps<MultiBlankDefinition>,
): JSX.Element {
  const restored = new Map(
    props.initialResponse?.answers.map((answer) => [answer.slot, answer.text]),
  );
  const initialAnswers = props.definition.blanks.map((blank) => ({
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
  const controller = createSubmissionController(props, response());
  function update(slot: string, text: string): void {
    const next = answers().map((answer) => (answer.slot === slot ? { ...answer, text } : answer));
    setAnswers(next);
    void controller.validate({ kind: "multiBlank", answers: [...next] });
  }
  function submit(): void {
    void controller.submit(response());
  }
  function reset(): void {
    const next = initialAnswers.map((answer) => ({ ...answer }));
    setAnswers(next);
    void controller.reset({ kind: "multiBlank", answers: next });
    queueMicrotask(() => firstBlank.focus());
  }
  return (
    <section
      class="response-widget"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleQuestionResponseControlKeyDown(event, props.onEscape, submit, controller.canSubmit)
      }
    >
      <fieldset
        aria-describedby={`${props.attemptId}-multi-blank-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.locked()}
      >
        <legend>Complete each blank</legend>
        <p class="keyboard-instructions" id={`${props.attemptId}-multi-blank-help`}>
          Use Tab and Shift+Tab to move between blanks. Type each response, then use the Submit
          answer button. Enter is an optional submit shortcut.
        </p>
        <p
          class="completion-progress"
          role="status"
          aria-label="Blank completion"
          aria-live="polite"
        >
          {completedBlankCount()} of {props.definition.blanks.length} blanks completed.
        </p>
        <div class="response-fields">
          <For each={props.definition.blanks}>
            {(blank, index) => (
              <label for={`${props.attemptId}-blank-${index()}`}>
                {textFromBlocks(blank.label)}
                <input
                  id={`${props.attemptId}-blank-${index()}`}
                  class="response-control"
                  type="text"
                  maxlength={blank.maxLength}
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
        disabled={!controller.canSubmit() || controller.locked()}
        resetDisabled={controller.locked()}
        onSubmit={submit}
        onReset={reset}
        onEscape={props.onEscape}
      />
    </section>
  );
}
