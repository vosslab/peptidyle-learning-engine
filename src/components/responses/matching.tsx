// matching.tsx - keyboard-first one-to-one matching response.

import { createSignal, For, type JSX } from "solid-js";

import type { ChoiceId } from "../../../generated/api/ChoiceId";
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

type MatchPair = Extract<StudentResponse, { readonly kind: "matching" }>["matches"][number];

/** Retain only the first public restored pairing for a choice, so the UI never starts duplicated. */
function initialMatches(props: WidgetBodyProps<MatchingDefinition>): ReadonlyArray<MatchPair> {
  const restored = new Map(
    props.initialResponse?.matches.map((pair) => [pair.prompt, pair.choice]),
  );
  const assignedChoices = new Set<ChoiceId>();
  const matches = props.definition.prompts.map((prompt) => {
    const choice = restored.get(prompt.id) ?? "";
    const uniqueChoice = assignedChoices.has(choice) ? "" : choice;
    if (uniqueChoice !== "") assignedChoices.add(uniqueChoice);
    return { prompt: prompt.id, choice: uniqueChoice };
  });
  return matches;
}

export function MatchingResponse(props: WidgetBodyProps<MatchingDefinition>): JSX.Element {
  const initial = initialMatches(props);
  const [matches, setMatches] = createSignal<ReadonlyArray<MatchPair>>(initial);
  let firstChoice!: HTMLButtonElement;
  const response = (): StudentResponse => ({ kind: "matching", matches: [...matches()] });
  const controller = createSubmissionController(props, response());

  function selectedChoice(prompt: ChoiceId): ChoiceId {
    const pair = matches().find((match) => match.prompt === prompt);
    return pair?.choice ?? "";
  }

  function choiceIsUsedByAnotherPrompt(prompt: ChoiceId, choice: ChoiceId): boolean {
    return matches().some((match) => match.prompt !== prompt && match.choice === choice);
  }

  function matchedPromptCount(): number {
    return matches().filter((match) => match.choice !== "").length;
  }

  function update(prompt: string, choice: string): void {
    const next = matches().map((pair) => (pair.prompt === prompt ? { prompt, choice } : pair));
    setMatches(next);
    void controller.validate({ kind: "matching", matches: [...next] });
  }

  /**
   * Each pairing is intentionally its own Tab stop. Native radio inputs collapse a same-name
   * group to one stop, which prevents a learner from reaching every visible pairing by Tab.
   * Arrow keys retain the familiar radio shortcut without being required for completion.
   */
  function moveWithArrow(
    event: KeyboardEvent,
    prompt: ChoiceId,
    choice: ChoiceId,
    promptIndex: number,
  ): void {
    if (!["ArrowDown", "ArrowRight", "ArrowUp", "ArrowLeft"].includes(event.key)) return;
    event.preventDefault();
    if (controller.locked() || choiceIsUsedByAnotherPrompt(prompt, choice)) return;

    const availableChoices = props.definition.choices.filter(
      (candidate) => !choiceIsUsedByAnotherPrompt(prompt, candidate.id),
    );
    const currentIndex = availableChoices.findIndex((candidate) => candidate.id === choice);
    if (currentIndex < 0 || availableChoices.length === 0) return;
    const direction = event.key === "ArrowDown" || event.key === "ArrowRight" ? 1 : -1;
    const nextIndex =
      (currentIndex + direction + availableChoices.length) % availableChoices.length;
    const nextChoice = availableChoices[nextIndex];
    if (nextChoice === undefined) return;

    update(prompt, nextChoice.id);
    const nextChoiceIndex = props.definition.choices.findIndex(
      (candidate) => candidate.id === nextChoice.id,
    );
    if (nextChoiceIndex < 0) return;
    document.getElementById(`${props.attemptId}-match-${promptIndex}-${nextChoiceIndex}`)?.focus();
  }
  function submit(): void {
    void controller.submit(response());
  }
  function reset(): void {
    const next = initial.map((pair) => ({ ...pair }));
    setMatches(next);
    void controller.reset({ kind: "matching", matches: next });
    queueMicrotask(() => firstChoice.focus());
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
        disabled={controller.locked()}
      >
        <legend>Match each prompt</legend>
        <p class="keyboard-hint" id={`${props.attemptId}-matching-help`}>
          Tab to every available pairing and press Space to select it. Arrow keys are an optional
          shortcut within a prompt. Each choice may be used once.
        </p>
        <p
          class="matching-progress"
          id={`${props.attemptId}-matching-progress`}
          role="status"
          aria-live="polite"
          aria-atomic="true"
          aria-label={`${matchedPromptCount()} of ${props.definition.prompts.length} prompts matched`}
        >
          {matchedPromptCount()} of {props.definition.prompts.length} prompts matched
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
                <div
                  class="choice-list"
                  role="radiogroup"
                  aria-labelledby={`${props.attemptId}-match-prompt-${index()}`}
                >
                  <For each={props.definition.choices}>
                    {(choice, choiceIndex) => {
                      const selected = (): boolean => selectedChoice(prompt.id) === choice.id;
                      const unavailable = (): boolean =>
                        choiceIsUsedByAnotherPrompt(prompt.id, choice.id);
                      return (
                        <button
                          id={`${props.attemptId}-match-${index()}-${choiceIndex()}`}
                          type="button"
                          role="radio"
                          data-choice-id={choice.id}
                          aria-checked={selected()}
                          ref={
                            index() === 0 && choiceIndex() === 0
                              ? (element): void => {
                                  firstChoice = element;
                                }
                              : undefined
                          }
                          aria-disabled={unavailable() || controller.locked()}
                          tabIndex={unavailable() || controller.locked() ? -1 : 0}
                          class="choice-card matching-choice-card"
                          classList={{ selected: selected(), unavailable: unavailable() }}
                          onClick={() => {
                            if (!unavailable() && !controller.locked())
                              update(prompt.id, choice.id);
                          }}
                          onKeyDown={(event) => moveWithArrow(event, prompt.id, choice.id, index())}
                        >
                          <span class="matching-choice-content">
                            <span>{textFromBlocks(choice.body)}</span>
                            <span class="matching-choice-state">
                              {selected()
                                ? "Selected for this prompt."
                                : unavailable()
                                  ? "Already selected for another prompt."
                                  : "Available."}
                            </span>
                          </span>
                        </button>
                      );
                    }}
                  </For>
                </div>
              </div>
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
