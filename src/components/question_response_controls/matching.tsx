// matching.tsx - one shared choice bank with keyboard-first prompt slots.

import { createSignal, For, type JSX } from "solid-js";

import type { ResponseItemId } from "../../../generated/api/ResponseItemId";
import type { StudentResponse } from "../../../generated/api/StudentResponse";

import { handleQuestionResponseControlKeyDown } from "./keyboard";
import {
  Actions,
  createResponseController,
  Status,
  textFromBlocks,
  type MatchingResponseFormat,
  type QuestionResponseControlBodyProps,
} from "./common";

type StudentMatch = Extract<StudentResponse, { readonly kind: "matching" }>["matches"][number];

function matchingResponseFromSlots(matches: ReadonlyArray<StudentMatch>): StudentResponse {
  // ASVS 2.2.1: empty UI slots are absent pairs, never public choice identifiers.
  // The format validator still rejects missing prompts and actual unavailable choices.
  return { kind: "matching", matches: matches.filter((pair) => pair.choice !== "") };
}

function choicesMayBeReused(format: MatchingResponseFormat): boolean {
  return "reuseChoices" in format && format.reuseChoices;
}

/** Restore public pairs, preserving repeated choices only when the format permits reuse. */
function initialMatches(
  props: QuestionResponseControlBodyProps<MatchingResponseFormat>,
): ReadonlyArray<StudentMatch> {
  const restored = new Map(
    props.initialResponse?.kind === "matching"
      ? props.initialResponse.matches.map((pair) => [pair.prompt, pair.choice])
      : [],
  );
  const assignedChoices = new Set<ResponseItemId>();
  return props.responseFormat.prompts.map((prompt) => {
    const choice = restored.get(prompt.id) ?? "";
    const available = props.responseFormat.choices.some((candidate) => candidate.id === choice);
    const retained =
      available && (choicesMayBeReused(props.responseFormat) || !assignedChoices.has(choice))
        ? choice
        : "";
    if (retained !== "") assignedChoices.add(retained);
    return { prompt: prompt.id, choice: retained };
  });
}

export function MatchingResponse(
  props: QuestionResponseControlBodyProps<MatchingResponseFormat>,
): JSX.Element {
  const initial = initialMatches(props);
  const [matches, setMatches] = createSignal<ReadonlyArray<StudentMatch>>(initial);
  const [pendingChoice, setPendingChoice] = createSignal<ResponseItemId>("");
  const [announcement, setAnnouncement] = createSignal("");
  let bank!: HTMLDivElement;
  // A drop must originate from this bank, not arbitrary external drag data.
  let draggedChoice: ResponseItemId = "";
  const response = (): StudentResponse => matchingResponseFromSlots(matches());
  const controller = createResponseController(props, response());
  const reuseChoices = (): boolean => choicesMayBeReused(props.responseFormat);

  function assignedChoice(prompt: ResponseItemId): ResponseItemId {
    return matches().find((match) => match.prompt === prompt)?.choice ?? "";
  }
  function choiceText(choice: ResponseItemId): string {
    const item = props.responseFormat.choices.find((candidate) => candidate.id === choice);
    return item === undefined ? "" : textFromBlocks(item.body);
  }
  function usageCount(choice: ResponseItemId): number {
    return matches().filter((match) => match.choice === choice).length;
  }
  function unavailable(choice: ResponseItemId): boolean {
    return !reuseChoices() && usageCount(choice) > 0;
  }
  function select(choice: ResponseItemId): void {
    if (controller.locked() || unavailable(choice)) return;
    setPendingChoice(choice);
    setAnnouncement("");
  }

  function update(prompt: ResponseItemId, choice: ResponseItemId): void {
    // ASVS 2.2.1: constrain all assignment paths to the public format and its reuse rule.
    if (controller.locked()) return;
    if (!props.responseFormat.prompts.some((candidate) => candidate.id === prompt)) return;
    if (choice !== "") {
      if (!props.responseFormat.choices.some((candidate) => candidate.id === choice)) return;
      if (
        !reuseChoices() &&
        matches().some((pair) => pair.prompt !== prompt && pair.choice === choice)
      )
        return;
    }
    if (assignedChoice(prompt) === choice) return;
    const next = matches().map((pair) => (pair.prompt === prompt ? { prompt, choice } : pair));
    setMatches(next);
    void controller.edit(matchingResponseFromSlots(next));
    const item = props.responseFormat.prompts.find((candidate) => candidate.id === prompt);
    const promptText = item === undefined ? "" : textFromBlocks(item.body);
    setAnnouncement(choice === "" ? `Cleared ${promptText}.` : `Assigned to ${promptText}.`);
  }
  function assignPending(prompt: ResponseItemId): void {
    if (controller.locked()) return;
    if (pendingChoice() === "") {
      setAnnouncement("Select a choice from the bank first.");
      return;
    }
    if (unavailable(pendingChoice()) && assignedChoice(prompt) !== pendingChoice()) {
      setAnnouncement("That choice is already assigned. Clear its slot or select another choice.");
      return;
    }
    update(prompt, pendingChoice());
  }
  function selectionStatus(): string {
    if (controller.locked() || pendingChoice() === "") return "";
    return unavailable(pendingChoice())
      ? `Selected: ${choiceText(pendingChoice())}. Already assigned; select another choice or clear its slot.`
      : `Selected: ${choiceText(pendingChoice())}. Activate a prompt slot to assign it.`;
  }
  function save(): void {
    void controller.save(response());
  }
  function reset(): void {
    if (controller.locked()) return;
    const next = initial.map((pair) => ({ ...pair }));
    setMatches(next);
    setPendingChoice("");
    draggedChoice = "";
    setAnnouncement("Original response restored.");
    void controller.reset(matchingResponseFromSlots(next));
    queueMicrotask(() => bank.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus());
  }

  return (
    <section
      class="question-response-control"
      data-phase={controller.phase().kind}
      onKeyDown={(event) =>
        handleQuestionResponseControlKeyDown(event, props.onEscape, save, controller.canSave)
      }
    >
      <fieldset
        aria-describedby={`${props.attemptId}-matching-help ${props.attemptId}-format-status`}
        aria-invalid={controller.invalid()}
        disabled={controller.locked()}
      >
        <legend>Match each prompt</legend>
        <p class="keyboard-instructions" id={`${props.attemptId}-matching-help`}>
          Select a bank choice, then a prompt slot. Use Tab and Space or Enter, or click or tap.
          Dragging is optional.{" "}
          {reuseChoices() ? "Choices may be reused." : "Use each choice once."}
        </p>
        <p class="matching-progress">
          {matches().filter((pair) => pair.choice !== "").length} of{" "}
          {props.responseFormat.prompts.length} prompts matched
        </p>
        <p class="matching-selection-status" role="status" aria-live="polite" aria-atomic="true">
          {announcement()} {selectionStatus()}
        </p>
        <div class="matching-layout">
          <div
            class="matching-bank"
            ref={(element): void => {
              bank = element;
            }}
            role="group"
            aria-labelledby={`${props.attemptId}-matching-bank-title`}
          >
            <h3 id={`${props.attemptId}-matching-bank-title`}>Choice bank</h3>
            <div class="choice-list">
              <For each={props.responseFormat.choices}>
                {(choice) => (
                  <button
                    type="button"
                    data-choice-id={choice.id}
                    aria-pressed={pendingChoice() === choice.id}
                    disabled={controller.locked() || unavailable(choice.id)}
                    class="choice-card matching-choice-card"
                    classList={{
                      selected: pendingChoice() === choice.id,
                      unavailable: unavailable(choice.id),
                    }}
                    draggable={!controller.locked() && !unavailable(choice.id)}
                    onClick={() => select(choice.id)}
                    onDragStart={(event) => {
                      if (controller.locked() || unavailable(choice.id)) {
                        event.preventDefault();
                        return;
                      }
                      select(choice.id);
                      draggedChoice = choice.id;
                      event.dataTransfer?.setData("text/plain", choice.id);
                      if (event.dataTransfer !== null) event.dataTransfer.effectAllowed = "copy";
                    }}
                    onDragEnd={() => {
                      draggedChoice = "";
                    }}
                  >
                    <span class="matching-choice-content">
                      {/* ASVS 1.2.1: authored labels remain escaped text, never injected markup. */}
                      <span>{textFromBlocks(choice.body)}</span>
                      <span class="matching-choice-state">
                        {usageCount(choice.id) > 0
                          ? `Used in ${usageCount(choice.id)} ${usageCount(choice.id) === 1 ? "slot" : "slots"}${reuseChoices() ? "; reusable" : ""}.`
                          : "Available."}
                      </span>
                    </span>
                  </button>
                )}
              </For>
            </div>
          </div>
          <div class="matching-prompts">
            <h3>Prompt slots</h3>
            <For each={props.responseFormat.prompts}>
              {(prompt, index) => (
                <div
                  class="matching-group"
                  role="group"
                  aria-labelledby={`${props.attemptId}-match-prompt-${index()}`}
                >
                  <p class="matching-prompt" id={`${props.attemptId}-match-prompt-${index()}`}>
                    {textFromBlocks(prompt.body)}
                  </p>
                  <div class="matching-slot-actions">
                    <button
                      type="button"
                      class="matching-slot"
                      data-prompt-id={prompt.id}
                      disabled={controller.locked()}
                      aria-label={`${textFromBlocks(prompt.body)}: ${assignedChoice(prompt.id) === "" ? "Unanswered" : choiceText(assignedChoice(prompt.id))}. Assign selected choice.`}
                      onClick={() => assignPending(prompt.id)}
                      onDragOver={(event) => {
                        if (
                          !controller.locked() &&
                          draggedChoice !== "" &&
                          !unavailable(draggedChoice)
                        ) {
                          event.preventDefault();
                          if (event.dataTransfer !== null) event.dataTransfer.dropEffect = "copy";
                        }
                      }}
                      onDrop={(event) => {
                        event.preventDefault();
                        if (draggedChoice !== "") update(prompt.id, draggedChoice);
                        draggedChoice = "";
                      }}
                    >
                      {assignedChoice(prompt.id) === ""
                        ? "Assign selected choice"
                        : choiceText(assignedChoice(prompt.id))}
                    </button>
                    <button
                      type="button"
                      class="quiet-action matching-clear"
                      disabled={controller.locked() || assignedChoice(prompt.id) === ""}
                      aria-label={`Clear response for ${textFromBlocks(prompt.body)}`}
                      onClick={(event) => {
                        update(prompt.id, "");
                        event.currentTarget.parentElement
                          ?.querySelector<HTMLButtonElement>(".matching-slot")
                          ?.focus();
                      }}
                    >
                      Clear
                    </button>
                  </div>
                </div>
              )}
            </For>
          </div>
        </div>
      </fieldset>
      <Status attemptId={props.attemptId} controller={controller} />
      <Actions
        disabled={!controller.canSave() || controller.locked()}
        resetDisabled={controller.locked()}
        onSave={save}
        saveLabel={props.saveLabel}
        onReset={reset}
        onEscape={props.onEscape}
      />
    </section>
  );
}
