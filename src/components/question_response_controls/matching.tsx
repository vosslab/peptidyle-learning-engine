// matching.tsx - keyboard-first one-to-one matching response.

import { createSignal, For, type JSX } from "solid-js";

import type { ResponseItemReference } from "../../../generated/api/ResponseItemReference";
import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleQuestionResponseControlKeyDown } from "../question_response_controls/keyboard";
import {
  Actions,
  createSubmissionController,
  Status,
  textFromBlocks,
  type MatchingResponseFormat,
  type QuestionResponseControlBodyProps,
} from "./common";

type StudentMatch = Extract<StudentResponse, { readonly kind: "matching" }>["matches"][number];

/** Retain only the first public restored pairing for a choice, so the UI never starts duplicated. */
function initialMatches(
  props: QuestionResponseControlBodyProps<MatchingResponseFormat>,
): ReadonlyArray<StudentMatch> {
  const restored = new Map(
    props.initialResponse?.kind === "matching"
      ? props.initialResponse.matches.map((pair) => [pair.prompt, pair.choice])
      : [],
  );
  const assignedChoices = new Set<ResponseItemReference>();
  const matches = props.responseFormat.prompts.map((prompt) => {
    const choice = restored.get(prompt.id) ?? "";
    const uniqueChoice = assignedChoices.has(choice) ? "" : choice;
    if (uniqueChoice !== "") assignedChoices.add(uniqueChoice);
    return { prompt: prompt.id, choice: uniqueChoice };
  });
  return matches;
}

export function MatchingResponse(
  props: QuestionResponseControlBodyProps<MatchingResponseFormat>,
): JSX.Element {
  const initial = initialMatches(props);
  const [matches, setMatches] = createSignal<ReadonlyArray<StudentMatch>>(initial);
  let firstChoice!: HTMLButtonElement;
  const response = (): StudentResponse => ({ kind: "matching", matches: [...matches()] });
  const controller = createSubmissionController(props, response());

  function selectedChoice(prompt: ResponseItemReference): ResponseItemReference {
    const pair = matches().find((match) => match.prompt === prompt);
    return pair?.choice ?? "";
  }

  function choiceIsUsedByAnotherPrompt(
    prompt: ResponseItemReference,
    choice: ResponseItemReference,
  ): boolean {
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
   * group to one stop, which prevents a student from reaching every visible pairing by Tab.
   * Arrow keys retain the familiar radio shortcut without being required for completion.
   */
  function moveWithArrow(
    event: KeyboardEvent,
    prompt: ResponseItemReference,
    choice: ResponseItemReference,
    promptIndex: number,
  ): void {
    if (!["ArrowDown", "ArrowRight", "ArrowUp", "ArrowLeft"].includes(event.key)) return;
    event.preventDefault();
    if (controller.locked() || choiceIsUsedByAnotherPrompt(prompt, choice)) return;

    const availableChoices = props.responseFormat.choices.filter(
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
    const nextChoiceIndex = props.responseFormat.choices.findIndex(
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
      class="question-response-control"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleQuestionResponseControlKeyDown(event, props.onEscape, submit, controller.canSubmit)
      }
    >
      <fieldset
        aria-describedby={`${props.attemptId}-matching-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.locked()}
      >
        <legend>Match each prompt</legend>
        <p class="keyboard-instructions" id={`${props.attemptId}-matching-help`}>
          Tab to every available pairing and press Space to select it. Arrow keys are an optional
          shortcut within a prompt. Each choice may be used once.
        </p>
        <p
          class="matching-progress"
          id={`${props.attemptId}-matching-progress`}
          role="status"
          aria-live="polite"
          aria-atomic="true"
          aria-label={`${matchedPromptCount()} of ${props.responseFormat.prompts.length} prompts matched`}
        >
          {matchedPromptCount()} of {props.responseFormat.prompts.length} prompts matched
        </p>
        <div class="response-fields">
          <For each={props.responseFormat.prompts}>
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
                  <For each={props.responseFormat.choices}>
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
