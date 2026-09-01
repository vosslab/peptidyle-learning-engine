// question_json_numeric_editor.tsx - compact private controls for one numeric accepted response.

import { Show, type JSX } from "solid-js";

import {
  numericToleranceField,
  setNumericResponseToleranceKind,
  setNumericResponseToleranceValue,
  validateNumericResponse,
} from "./question_json_numeric_model";
import type { PleQuestionJsonNumericResponseTolerance } from "./question_json_source";

function isNumericResponseToleranceKind(
  value: string,
): value is PleQuestionJsonNumericResponseTolerance["kind"] {
  return (
    value === "exact" ||
    value === "absolute" ||
    value === "relative" ||
    value === "significantFigures"
  );
}

export interface PleQuestionJsonNumericEditorProps {
  /** Owned by the parent so an in-progress literal such as 6.02e remains visible while editing. */
  readonly answerLiteral: string;
  readonly tolerance: () => PleQuestionJsonNumericResponseTolerance;
  readonly unit: () => string | null;
  readonly onAnswerLiteralChange: (literal: string) => void;
  readonly onToleranceChange: (tolerance: PleQuestionJsonNumericResponseTolerance) => void;
  readonly onUnitChange: (unit: string | null) => void;
  readonly disabled?: boolean;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
}

/** Shows one tolerance field at a time, keeping its meaning adjacent to the author decision. */
export function PleQuestionJsonNumericEditor(
  props: PleQuestionJsonNumericEditorProps,
): JSX.Element {
  const validation = (): ReturnType<typeof validateNumericResponse> =>
    validateNumericResponse(props.answerLiteral, props.tolerance(), props.unit());
  const errorFor = (field: string): string | undefined =>
    props.fieldErrors?.[`response.${field}`] ??
    props.fieldErrors?.[field] ??
    validation().issues[field];
  const toleranceField = (): ReturnType<typeof numericToleranceField> =>
    numericToleranceField(props.tolerance());
  const toleranceLabel = (): string => {
    if (props.tolerance().kind === "absolute") return "Allowed difference";
    if (props.tolerance().kind === "relative") return "Allowed fraction";
    return "Significant figures";
  };
  const toleranceInput = (): JSX.Element | null => {
    const field = toleranceField();
    if (field === null) return null;
    const tolerance = props.tolerance();
    const value =
      tolerance.kind === "absolute"
        ? tolerance.epsilon
        : tolerance.kind === "relative"
          ? tolerance.fraction
          : tolerance.kind === "significantFigures"
            ? tolerance.digits
            : 0;
    return (
      <>
        <label class="ple-question-json-authoring__field">
          <span>{toleranceLabel()}</span>
          <input
            type="number"
            min={field === "digits" ? "1" : "0"}
            max={field === "digits" ? "255" : undefined}
            step={field === "digits" ? "1" : "any"}
            value={value}
            disabled={props.disabled}
            aria-invalid={errorFor(field) !== undefined}
            onInput={(event) =>
              props.onToleranceChange(
                setNumericResponseToleranceValue(
                  props.tolerance(),
                  Number(event.currentTarget.value),
                ),
              )
            }
          />
          <span class="ple-question-json-authoring__help">
            {field === "epsilon"
              ? "Accept values this far above or below the answer."
              : field === "fraction"
                ? "Use a decimal fraction, for example 0.01 for one percent."
                : "Accept values rounded to this many significant figures."}
          </span>
        </label>
        <Show when={errorFor(field) !== undefined}>
          <p class="ple-question-json-authoring__error" role="alert">
            {errorFor(field)}
          </p>
        </Show>
      </>
    );
  };
  return (
    <fieldset class="ple-question-json-authoring__numeric">
      <legend>Accepted numeric answer</legend>
      <p class="ple-question-json-authoring__help">
        Enter the number you will accept. Scientific notation is kept exactly as you type it.
      </p>
      <label class="ple-question-json-authoring__field">
        <span>Accepted numeric value</span>
        <input
          type="text"
          inputmode="decimal"
          value={props.answerLiteral}
          disabled={props.disabled}
          aria-invalid={errorFor("answer") !== undefined}
          aria-describedby={
            errorFor("answer") === undefined ? undefined : "ple-question-json-numeric-answer-error"
          }
          onInput={(event) => props.onAnswerLiteralChange(event.currentTarget.value)}
        />
      </label>
      <Show when={errorFor("answer") !== undefined}>
        <p
          class="ple-question-json-authoring__error"
          id="ple-question-json-numeric-answer-error"
          role="alert"
        >
          {errorFor("answer")}
        </p>
      </Show>
      <label class="ple-question-json-authoring__field">
        <span>How should the number be checked?</span>
        <select
          value={props.tolerance().kind}
          disabled={props.disabled}
          onChange={(event) => {
            const kind = event.currentTarget.value;
            if (isNumericResponseToleranceKind(kind)) {
              props.onToleranceChange(setNumericResponseToleranceKind(kind));
            }
          }}
        >
          <option value="exact">Exact value</option>
          <option value="absolute">Within an absolute difference</option>
          <option value="relative">Within a relative fraction</option>
          <option value="significantFigures">By significant figures</option>
        </select>
      </label>
      {toleranceInput()}
      <label class="ple-question-json-authoring__field">
        <span>Unit (optional)</span>
        <input
          type="text"
          value={props.unit() ?? ""}
          disabled={props.disabled}
          aria-invalid={errorFor("unit") !== undefined}
          onInput={(event) =>
            props.onUnitChange(
              event.currentTarget.value.trim() === "" ? null : event.currentTarget.value,
            )
          }
        />
        <span class="ple-question-json-authoring__help">
          This tells students what unit to report; it does not convert the accepted value.
        </span>
      </label>
      <Show when={errorFor("unit") !== undefined}>
        <p class="ple-question-json-authoring__error" role="alert">
          {errorFor("unit")}
        </p>
      </Show>
    </fieldset>
  );
}
