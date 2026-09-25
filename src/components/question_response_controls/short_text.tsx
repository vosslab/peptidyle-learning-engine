// short_text.tsx - controlled short-text response entry.

import { createSignal, type JSX } from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleQuestionResponseControlKeyDown } from "../question_response_controls/keyboard";
import {
  Actions,
  createResponseController,
  Status,
  type ShortTextResponseFormat,
  type QuestionResponseControlBodyProps,
} from "./common";

export function ShortTextResponse(
  props: QuestionResponseControlBodyProps<ShortTextResponseFormat>,
): JSX.Element {
  const initialText = props.initialResponse?.kind === "shortText" ? props.initialResponse.text : "";
  const [text, setText] = createSignal(initialText);
  const controller = createResponseController(props, { kind: "shortText", text: initialText });
  const characterCount = (): number => [...text()].length;
  const response = (): StudentResponse => ({ kind: "shortText", text: text() });
  function update(next: string): void {
    setText(next);
    void controller.edit({ kind: "shortText", text: next });
  }
  function save(): void {
    void controller.save(response());
  }
  return (
    <section
      class="question-response-control"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleQuestionResponseControlKeyDown(event, props.onEscape, save, controller.canSave)
      }
    >
      <label for={`${props.attemptId}-short-text`}>Short written response</label>
      <p class="field-help" id={`${props.attemptId}-short-text-help`}>
        Up to{" "}
        {props.responseFormat.kind === "fillIn"
          ? props.responseFormat.maxCharacters
          : props.responseFormat.maxLength}{" "}
        characters. {characterCount()} used.
      </p>
      <textarea
        id={`${props.attemptId}-short-text`}
        class="question-response-control__input"
        value={text()}
        maxlength={
          props.responseFormat.kind === "fillIn"
            ? props.responseFormat.maxCharacters
            : props.responseFormat.maxLength
        }
        aria-describedby={`${props.attemptId}-short-text-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.locked()}
        onInput={(event) => update(event.currentTarget.value)}
      />
      <Status attemptId={props.attemptId} controller={controller} />
      <Actions
        disabled={!controller.canSave() || controller.locked()}
        onSave={save}
        saveLabel={props.saveLabel}
        onEscape={props.onEscape}
      />
    </section>
  );
}
