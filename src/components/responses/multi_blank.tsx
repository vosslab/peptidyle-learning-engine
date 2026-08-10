// multi_blank.tsx - keyboard-first named text-entry response.

import { createSignal, For, type JSX } from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleWidgetKeyDown } from "../response_widget/keyboard";
import {
  Actions,
  createSubmissionController,
  Status,
  textFromBlocks,
  type MultiBlankDefinition,
  type WidgetBodyProps,
} from "./common";

export function MultiBlankResponse(props: WidgetBodyProps<MultiBlankDefinition>): JSX.Element {
  const restored = new Map(
    props.initialResponse?.answers.map((answer) => [answer.slot, answer.text]),
  );
  const [answers, setAnswers] = createSignal(
    props.definition.blanks.map((blank) => ({
      slot: blank.id,
      text: restored.get(blank.id) ?? "",
    })),
  );
  const response = (): StudentResponse => ({ kind: "multiBlank", answers: [...answers()] });
  const controller = createSubmissionController(props, response());
  function update(slot: string, text: string): void {
    const next = answers().map((answer) => (answer.slot === slot ? { ...answer, text } : answer));
    setAnswers(next);
    void controller.validate({ kind: "multiBlank", answers: [...next] });
  }
  function submit(): void {
    void controller.submit(response());
  }
  return (
    <section
      class="response-widget"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleWidgetKeyDown(event, props.onEscape, submit, controller.canSubmit)
      }
    >
      <fieldset
        aria-describedby={`${props.attemptId}-multi-blank-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.pending()}
      >
        <legend>Complete each blank</legend>
        <p class="keyboard-hint" id={`${props.attemptId}-multi-blank-help`}>
          Use Tab and Shift+Tab to move between blanks. Type each response, then use the Submit
          answer button. Enter is an optional submit shortcut.
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
                  onInput={(event) => update(blank.id, event.currentTarget.value)}
                />
              </label>
            )}
          </For>
        </div>
      </fieldset>
      <Status attemptId={props.attemptId} controller={controller} />
      <Actions
        disabled={!controller.canSubmit() || controller.pending()}
        onSubmit={submit}
        onEscape={props.onEscape}
      />
    </section>
  );
}
