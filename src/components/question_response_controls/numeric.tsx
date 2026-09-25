// numeric.tsx - controlled numeric response entry.

import { createSignal, type JSX } from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleQuestionResponseControlKeyDown } from "../question_response_controls/keyboard";
import {
  Actions,
  createResponseController,
  numericResponseFromInput,
  Status,
  type NumericResponseFormat,
  type QuestionResponseControlBodyProps,
} from "./common";

export function NumericResponse(
  props: QuestionResponseControlBodyProps<NumericResponseFormat>,
): JSX.Element {
  const restored =
    props.initialResponse?.kind === "numeric" ? props.initialResponse.value : undefined;
  const initialValue = restored === undefined ? "" : String(restored);
  const [value, setValue] = createSignal(initialValue);
  const controller = createResponseController(props, numericResponseFromInput(initialValue));
  const response = (): StudentResponse => numericResponseFromInput(value());
  function update(next: string): void {
    setValue(next);
    void controller.edit(numericResponseFromInput(next));
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
      <label for={`${props.attemptId}-numeric`}>
        Numeric response
        {(props.responseFormat.kind === "numerical"
          ? props.responseFormat.displayedUnit
          : props.responseFormat.unit) === null
          ? ""
          : ` (${props.responseFormat.kind === "numerical" ? props.responseFormat.displayedUnit : props.responseFormat.unit})`}
      </label>
      <input
        id={`${props.attemptId}-numeric`}
        class="question-response-control__input"
        type="number"
        inputmode="decimal"
        value={value()}
        aria-describedby={`${props.attemptId}-format-status`}
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
