// multiple_choice.tsx - controlled single- and multiple-selection response entry.

import { createSignal, For, type JSX } from "solid-js";

import type { ResponseItemReference } from "../../../generated/api/ResponseItemReference";
import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleQuestionResponseControlKeyDown } from "../question_response_controls/keyboard";
import {
  Actions,
  createSubmissionController,
  Status,
  textFromBlocks,
  type MultipleChoiceDefinition,
  type MultipleChoiceResponseProps,
} from "./common";

function choiceInputType(definition: MultipleChoiceDefinition): "radio" | "checkbox" {
  return definition.selection.kind === "exactlyOne" ? "radio" : "checkbox";
}

function choiceKeyboardHint(definition: MultipleChoiceDefinition): string {
  const count = definition.choices.length;
  return definition.selection.kind === "exactlyOne"
    ? `Tab to the choices and press Space to select. Shortcuts: use the Arrow keys or press 1-${count}.`
    : `Tab to each choice and press Space to toggle it. Shortcuts: use the Arrow keys to move focus or press 1-${count}.`;
}

function multipleAnswerProgress(
  definition: MultipleChoiceDefinition,
  count: number,
): string | null {
  switch (definition.selection.kind) {
    case "exactlyOne":
      return null;
    case "exactly":
      return `${count} selected. Select exactly ${definition.selection.count}.`;
    case "atLeastOne":
      return `${count} selected. Select at least 1.`;
    case "anyNumber":
      return `${count} selected.`;
  }
}

/** Controlled multiple-choice widget. It validates shape only; grading stays server-only. */
export function MultipleChoiceResponse(props: MultipleChoiceResponseProps): JSX.Element {
  const restored = props.initialResponse?.selected ?? [];
  const [selected, setSelected] = createSignal<ReadonlyArray<ResponseItemReference>>(restored);
  let firstChoice!: HTMLInputElement;
  const controller = createSubmissionController(props, {
    kind: "multipleChoice",
    selected: [...restored],
  });
  const response = (): StudentResponse => ({ kind: "multipleChoice", selected: [...selected()] });
  const progress = (): string | null => multipleAnswerProgress(props.definition, selected().length);
  function choose(choice: ResponseItemReference): void {
    if (controller.locked()) return;
    const next =
      props.definition.selection.kind === "exactlyOne"
        ? [choice]
        : selected().includes(choice)
          ? selected().filter((item) => item !== choice)
          : [...selected(), choice];
    setSelected(next);
    void controller.validate({ kind: "multipleChoice", selected: [...next] });
  }
  function submit(): void {
    void controller.submit(response());
  }
  function reset(): void {
    const next = [...restored];
    setSelected(next);
    void controller.reset({ kind: "multipleChoice", selected: next });
    queueMicrotask(() => firstChoice.focus());
  }
  function handleKeyDown(event: KeyboardEvent): void {
    const target = event.target;
    if (
      target instanceof HTMLInputElement &&
      target.type === "checkbox" &&
      ["ArrowDown", "ArrowRight", "ArrowUp", "ArrowLeft"].includes(event.key)
    ) {
      const current = props.definition.choices.findIndex((choice) => choice.id === target.value);
      const direction = event.key === "ArrowDown" || event.key === "ArrowRight" ? 1 : -1;
      const next =
        (current + direction + props.definition.choices.length) % props.definition.choices.length;
      event.preventDefault();
      document.getElementById(`${props.attemptId}-choice-${next}`)?.focus();
      return;
    }
    if (
      target instanceof HTMLInputElement &&
      (target.type === "radio" || target.type === "checkbox") &&
      /^[1-9]$/.test(event.key)
    ) {
      const index = Number(event.key) - 1;
      const choice = props.definition.choices[index];
      if (choice !== undefined && !controller.locked()) {
        event.preventDefault();
        choose(choice.id);
        document.getElementById(`${props.attemptId}-choice-${index}`)?.focus();
      }
      return;
    }
    handleQuestionResponseControlKeyDown(event, props.onEscape, submit, controller.canSubmit);
  }
  return (
    <section class="response-widget" data-phase={controller.phase().kind} onKeyDown={handleKeyDown}>
      <fieldset
        aria-describedby={`${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.locked()}
      >
        <legend>Choose your response</legend>
        <p class="keyboard-hint">{choiceKeyboardHint(props.definition)}</p>
        {progress() === null ? null : (
          <p
            class="completion-progress"
            role="status"
            aria-label="Selection count"
            aria-live="polite"
          >
            {progress()}
          </p>
        )}
        <div class="choice-list">
          <For each={props.definition.choices}>
            {(choice, index) => (
              <label class="choice-card" classList={{ selected: selected().includes(choice.id) }}>
                <input
                  id={`${props.attemptId}-choice-${index()}`}
                  type={choiceInputType(props.definition)}
                  name={`response-${props.attemptId}`}
                  value={choice.id}
                  checked={selected().includes(choice.id)}
                  ref={
                    index() === 0
                      ? (element): void => {
                          firstChoice = element;
                        }
                      : undefined
                  }
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
