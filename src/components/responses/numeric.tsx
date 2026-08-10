// numeric.tsx - controlled numeric response entry.

import { createSignal, type JSX } from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleWidgetKeyDown } from "../response_widget/keyboard";
import {
  Actions,
  createSubmissionController,
  numericResponseFromInput,
  Status,
  type NumericDefinition,
  type WidgetBodyProps,
} from "./common";

export function NumericResponse(props: WidgetBodyProps<NumericDefinition>): JSX.Element {
  const restored = props.initialResponse?.value;
  const initialValue = restored === undefined ? "" : String(restored);
  const [value, setValue] = createSignal(initialValue);
  const controller = createSubmissionController(props, numericResponseFromInput(initialValue));
  const response = (): StudentResponse => numericResponseFromInput(value());
  function update(next: string): void {
    setValue(next);
    void controller.validate(numericResponseFromInput(next));
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
      <label for={`${props.attemptId}-numeric`}>
        Numeric response{props.definition.unit === null ? "" : ` (${props.definition.unit})`}
      </label>
      <input
        id={`${props.attemptId}-numeric`}
        class="response-control"
        type="number"
        inputmode="decimal"
        value={value()}
        aria-describedby={`${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.pending()}
        onInput={(event) => update(event.currentTarget.value)}
      />
      <Status attemptId={props.attemptId} controller={controller} />
      <Actions
        disabled={!controller.canSubmit() || controller.pending()}
        onSubmit={submit}
        onEscape={props.onEscape}
      />
    </section>
  );
}
