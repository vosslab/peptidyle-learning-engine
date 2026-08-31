// short_text.tsx - controlled short-text response entry.

import { createSignal, type JSX } from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleQuestionResponseControlKeyDown } from "../question_response_controls/keyboard";
import {
  Actions,
  createSubmissionController,
  Status,
  type ShortTextDefinition,
  type QuestionResponseControlBodyProps,
} from "./common";

export function ShortTextResponse(
  props: QuestionResponseControlBodyProps<ShortTextDefinition>,
): JSX.Element {
  const initialText = props.initialResponse?.text ?? "";
  const [text, setText] = createSignal(initialText);
  let control!: HTMLTextAreaElement;
  const controller = createSubmissionController(props, { kind: "shortText", text: initialText });
  const characterCount = (): number => [...text()].length;
  const response = (): StudentResponse => ({ kind: "shortText", text: text() });
  function update(next: string): void {
    setText(next);
    void controller.validate({ kind: "shortText", text: next });
  }
  function submit(): void {
    void controller.submit(response());
  }
  function reset(): void {
    setText(initialText);
    void controller.reset({ kind: "shortText", text: initialText });
    queueMicrotask(() => control.focus());
  }
  return (
    <section
      class="response-widget"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleQuestionResponseControlKeyDown(event, props.onEscape, submit, controller.canSubmit)
      }
    >
      <label for={`${props.attemptId}-short-text`}>Short written response</label>
      <p class="field-help" id={`${props.attemptId}-short-text-help`}>
        Up to {props.definition.maxLength} characters. {characterCount()} used.
      </p>
      <textarea
        id={`${props.attemptId}-short-text`}
        class="response-control"
        value={text()}
        ref={(element) => (control = element)}
        maxlength={props.definition.maxLength}
        aria-describedby={`${props.attemptId}-short-text-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.locked()}
        onInput={(event) => update(event.currentTarget.value)}
      />
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
