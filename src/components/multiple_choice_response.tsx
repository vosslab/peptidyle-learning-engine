// multiple_choice_response.tsx - the reference response-widget contract.

import { createSignal, For, Show, type JSX } from "solid-js";

import type { ChoiceId } from "../../generated/api/ChoiceId";
import type { ContentBlock } from "../../generated/api/ContentBlock";
import type { ResponseDefinition } from "../../generated/api/ResponseDefinition";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { ResponseFormatReport, ResponseFormatViolation, WasmFacade } from "../wasm/index";

type MultipleChoiceDefinition = Extract<ResponseDefinition, { kind: "multipleChoice" }>;

type WidgetPhase =
  | { readonly kind: "idle" }
  | { readonly kind: "validating" }
  | { readonly kind: "ready" }
  | { readonly kind: "invalid"; readonly message: string }
  | { readonly kind: "submitted" }
  | { readonly kind: "failed"; readonly message: string };

export interface MultipleChoiceResponseProps {
  readonly attemptId: string;
  readonly definition: MultipleChoiceDefinition;
  readonly validator: WasmFacade;
  readonly onSubmit: (response: StudentResponse) => Promise<void>;
  readonly onEscape: () => void;
}

function textFromBlocks(blocks: ReadonlyArray<ContentBlock>): string {
  return blocks
    .map((block) => {
      switch (block.kind) {
        case "text":
          return block.markdown;
        case "math":
          return block.description;
        case "image":
          return block.description;
        case "code":
          return block.source;
        case "table":
          return block.description;
      }
    })
    .join(" ");
}

function violationMessage(violation: ResponseFormatViolation): string {
  switch (violation.kind) {
    case "selectionCount":
      return "Choose the requested number of responses before submitting.";
    case "duplicateChoice":
      return "Each response may be selected only once.";
    case "unknownChoice":
      return "That response is not available for this question.";
    case "responseKindMismatch":
      return "This response does not match the question format.";
    case "numericNotFinite":
    case "textTooLong":
    case "orderingItemsMismatch":
    case "missingUploadReference":
      return "This response needs a format correction before submitting.";
  }
}

function reportMessage(report: ResponseFormatReport): string {
  const first = report.violations[0];
  return first === undefined ? "Response format is ready to submit." : violationMessage(first);
}

function phaseMessage(phase: WidgetPhase): string {
  switch (phase.kind) {
    case "idle":
      return "Choose one response.";
    case "validating":
      return "Checking response format...";
    case "ready":
      return "Response format is ready to submit.";
    case "invalid":
    case "failed":
      return phase.message;
    case "submitted":
      return "Answer submitted. Server feedback will appear when it is released.";
  }
}

/** Single-selection pattern for the remaining browser-safe response widgets. */
export function MultipleChoiceResponse(props: MultipleChoiceResponseProps): JSX.Element {
  const [selected, setSelected] = createSignal<ChoiceId>();
  const [phase, setPhase] = createSignal<WidgetPhase>({ kind: "idle" });
  let validationRequest = 0;
  let widgetElement: HTMLElement | undefined;

  const responseFor = (choice: ChoiceId): StudentResponse => ({
    kind: "multipleChoice",
    selected: [choice],
  });

  async function validate(choice: ChoiceId): Promise<ResponseFormatReport> {
    validationRequest += 1;
    const request = validationRequest;
    setPhase({ kind: "validating" });
    try {
      const report = await props.validator.validateResponseFormat(
        props.definition,
        responseFor(choice),
      );
      if (request === validationRequest) {
        setPhase(
          report.violations.length === 0
            ? { kind: "ready" }
            : { kind: "invalid", message: reportMessage(report) },
        );
      }
      return report;
    } catch (error: unknown) {
      const message =
        error instanceof Error
          ? `Response format check failed: ${error.message}. Try again.`
          : "Response format check failed. Try again.";
      if (request === validationRequest) {
        setPhase({ kind: "failed", message });
      }
      throw error;
    }
  }

  function choose(choice: ChoiceId): void {
    setSelected(choice);
    void validate(choice).catch(() => undefined);
  }

  async function submit(): Promise<void> {
    const choice = selected();
    if (choice === undefined) {
      setPhase({ kind: "invalid", message: "Choose one response before submitting." });
      return;
    }
    try {
      const report = await validate(choice);
      if (report.violations.length > 0) {
        return;
      }
      setPhase({ kind: "submitted" });
      await props.onSubmit(responseFor(choice));
    } catch (error: unknown) {
      const message =
        error instanceof Error
          ? `Your answer is still selected. Submission failed: ${error.message}. Try again.`
          : "Your answer is still selected. Submission failed. Try again.";
      setPhase({ kind: "failed", message });
    }
  }

  function handleKeyDown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      props.onEscape();
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      void submit();
      return;
    }
    if (/^[1-9]$/.test(event.key)) {
      const choiceIndex = Number(event.key) - 1;
      const choice = props.definition.choices[choiceIndex];
      if (choice !== undefined) {
        event.preventDefault();
        choose(choice.id);
        queueMicrotask(() => {
          const radios = widgetElement?.querySelectorAll<HTMLInputElement>('input[type="radio"]');
          radios?.[choiceIndex]?.focus();
        });
      }
    }
  }

  function captureWidget(element: HTMLElement): void {
    widgetElement = element;
  }

  const invalid = (): boolean => phase().kind === "invalid" || phase().kind === "failed";
  const busy = (): boolean => phase().kind === "validating";

  return (
    <section
      ref={captureWidget}
      class="response-widget"
      data-phase={phase().kind}
      onKeyDown={handleKeyDown}
    >
      <fieldset aria-describedby={`${props.attemptId}-format-status`} aria-invalid={invalid()}>
        <legend>Choose one response</legend>
        <p class="keyboard-hint">Press 1-{props.definition.choices.length} to choose.</p>
        <div class="choice-list">
          <For each={props.definition.choices}>
            {(choice, index) => (
              <label class="choice-card" classList={{ selected: selected() === choice.id }}>
                <input
                  type="radio"
                  name={`response-${props.attemptId}`}
                  value={choice.id}
                  checked={selected() === choice.id}
                  onChange={() => choose(choice.id)}
                />
                <span class="choice-number" aria-hidden="true">
                  {index() + 1}
                </span>
                <span>{textFromBlocks(choice.body)}</span>
              </label>
            )}
          </For>
        </div>
      </fieldset>
      <p
        id={`${props.attemptId}-format-status`}
        class="format-status"
        classList={{
          error: invalid(),
          ready: phase().kind === "ready" || phase().kind === "submitted",
        }}
        role="status"
        aria-live="polite"
      >
        <Show when={busy()} fallback={phaseMessage(phase())}>
          <span class="status-spinner" aria-hidden="true" /> Checking response format...
        </Show>
      </p>
      <button
        class="primary-action"
        type="button"
        disabled={selected() === undefined || busy()}
        onClick={() => void submit()}
      >
        Submit answer
      </button>
      <button class="quiet-action" type="button" onClick={props.onEscape}>
        Return to assignment
        <span aria-hidden="true">&nbsp;(Esc)</span>
      </button>
    </section>
  );
}
