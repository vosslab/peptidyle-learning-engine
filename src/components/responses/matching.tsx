// matching.tsx - native select-based matching response.

import { createSignal, For, type JSX } from "solid-js";

import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleWidgetKeyDown } from "../response_widget/keyboard";
import {
  Actions,
  createSubmissionController,
  Status,
  textFromBlocks,
  type MatchingDefinition,
  type WidgetBodyProps,
} from "./common";

export function MatchingResponse(props: WidgetBodyProps<MatchingDefinition>): JSX.Element {
  const restored = new Map(
    props.initialResponse?.matches.map((pair) => [pair.prompt, pair.choice]),
  );
  const [matches, setMatches] = createSignal(
    props.definition.prompts.map((prompt) => ({
      prompt: prompt.id,
      choice: restored.get(prompt.id) ?? "",
    })),
  );
  const response = (): StudentResponse => ({ kind: "matching", matches: [...matches()] });
  const controller = createSubmissionController(props, response());
  function update(prompt: string, choice: string): void {
    const next = matches().map((pair) => (pair.prompt === prompt ? { prompt, choice } : pair));
    setMatches(next);
    void controller.validate({ kind: "matching", matches: [...next] });
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
        aria-describedby={`${props.attemptId}-matching-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.pending()}
      >
        <legend>Match each prompt</legend>
        <p class="keyboard-hint" id={`${props.attemptId}-matching-help`}>
          Tab to each prompt's choices. Press Space to select, or use the Arrow keys within the
          browser's native radio group. Each choice may be used once.
        </p>
        <div class="response-fields">
          <For each={props.definition.prompts}>
            {(prompt, index) => (
              <div
                class="matching-group"
                role="group"
                aria-labelledby={`${props.attemptId}-match-prompt-${index()}`}
              >
                <p id={`${props.attemptId}-match-prompt-${index()}`}>
                  {textFromBlocks(prompt.body)}
                </p>
                <div class="choice-list">
                  <For each={props.definition.choices}>
                    {(choice, choiceIndex) => (
                      <label class="choice-card">
                        <input
                          id={`${props.attemptId}-match-${index()}-${choiceIndex()}`}
                          type="radio"
                          name={`${props.attemptId}-match-${prompt.id}`}
                          value={choice.id}
                          checked={
                            matches().find((pair) => pair.prompt === prompt.id)?.choice ===
                            choice.id
                          }
                          onChange={() => update(prompt.id, choice.id)}
                        />
                        <span>{textFromBlocks(choice.body)}</span>
                      </label>
                    )}
                  </For>
                </div>
              </div>
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
