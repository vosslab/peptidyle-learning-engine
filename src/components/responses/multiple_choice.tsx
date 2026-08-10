// multiple_choice.tsx - controlled single- and multiple-selection response entry.

import { createSignal, For, type JSX } from "solid-js";

import type { ChoiceId } from "../../../generated/api/ChoiceId";
import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleWidgetKeyDown } from "../response_widget/keyboard";
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

/** Controlled multiple-choice widget. It validates shape only; grading stays server-only. */
export function MultipleChoiceResponse(props: MultipleChoiceResponseProps): JSX.Element {
  const restored = props.initialResponse?.selected ?? [];
  const [selected, setSelected] = createSignal<ReadonlyArray<ChoiceId>>(restored);
  const controller = createSubmissionController(props, {
    kind: "multipleChoice",
    selected: [...restored],
  });
  const response = (): StudentResponse => ({ kind: "multipleChoice", selected: [...selected()] });
  function choose(choice: ChoiceId): void {
    if (controller.pending()) return;
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
      if (choice !== undefined && !controller.pending()) {
        event.preventDefault();
        choose(choice.id);
        document.getElementById(`${props.attemptId}-choice-${index}`)?.focus();
      }
      return;
    }
    handleWidgetKeyDown(event, props.onEscape, submit, controller.canSubmit);
  }
  return (
    <section class="response-widget" data-phase={controller.phase().kind} onKeyDown={handleKeyDown}>
      <fieldset
        aria-describedby={`${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.pending()}
      >
        <legend>Choose your response</legend>
        <p class="keyboard-hint">{choiceKeyboardHint(props.definition)}</p>
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
        disabled={!controller.canSubmit() || controller.pending()}
        onSubmit={submit}
        onEscape={props.onEscape}
      />
    </section>
  );
}
