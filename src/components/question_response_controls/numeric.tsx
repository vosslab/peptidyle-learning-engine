// numeric.tsx - controlled numeric response entry.

import { createSignal, type JSX } from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleQuestionResponseControlKeyDown } from "../question_response_controls/keyboard";
import {
  Actions,
  createSubmissionController,
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
  let control!: HTMLInputElement;
  const controller = createSubmissionController(props, numericResponseFromInput(initialValue));
  const response = (): StudentResponse => numericResponseFromInput(value());
  function update(next: string): void {
    setValue(next);
    void controller.validate(numericResponseFromInput(next));
  }
  function submit(): void {
    void controller.submit(response());
  }
  function reset(): void {
    setValue(initialValue);
    void controller.reset(numericResponseFromInput(initialValue));
    queueMicrotask(() => control.focus());
  }
  return (
    <section
      class="question-response-control"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleQuestionResponseControlKeyDown(event, props.onEscape, submit, controller.canSubmit)
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
        ref={(element) => (control = element)}
        aria-describedby={`${props.attemptId}-format-status`}
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
