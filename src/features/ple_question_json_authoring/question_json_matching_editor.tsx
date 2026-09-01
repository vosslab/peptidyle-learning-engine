// question_json_matching_editor.tsx - compact private controls for semantic MATCH pairs.

import { For, Index, Show, type JSX } from "solid-js";

import type { PleQuestionJsonItem, PleQuestionJsonMatch } from "./question_json_source";

export interface PleQuestionJsonMatchingEditorProps {
  readonly prompts: ReadonlyArray<PleQuestionJsonItem>;
  readonly choices: ReadonlyArray<PleQuestionJsonItem>;
  readonly matches: ReadonlyArray<PleQuestionJsonMatch>;
  readonly onPromptTextChange: (id: string, text: string) => void;
  readonly onChoiceTextChange: (id: string, text: string) => void;
  readonly onMatchChange: (prompt: string, choice: string) => void;
  readonly onAddPair: () => void;
  readonly onRemovePair: (prompt: string) => void;
  readonly onMoveItem: (
    side: "prompts" | "choices",
    id: string,
    direction: "earlier" | "later",
  ) => void;
  readonly onStatus?: (message: string) => void;
  readonly disabled?: boolean;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
}

/** Keeps the author task in reading order: prompt, student choices, then an explicit private pairing. */
export function PleQuestionJsonMatchingEditor(
  props: PleQuestionJsonMatchingEditorProps,
): JSX.Element {
  function matchedChoice(prompt: string): string {
    return props.matches.find((pair) => pair.prompt === prompt)?.choice ?? "";
  }
  function choiceUsedElsewhere(prompt: string, choice: string): boolean {
    return props.matches.some((pair) => pair.prompt !== prompt && pair.choice === choice);
  }
  return (
    <fieldset class="ple-question-json-authoring__matching">
      <legend>Matching pairs</legend>
      <p class="ple-question-json-authoring__help">
        Give each side a clear student label, then pair every prompt with one different choice.
      </p>
      <Show when={props.fieldErrors?.["response.matches"] !== undefined}>
        <p class="ple-question-json-authoring__error" role="alert">
          {props.fieldErrors?.["response.matches"]}
        </p>
      </Show>
      <div class="ple-question-json-authoring__grid">
        <section aria-labelledby="ple-question-json-match-prompts-heading">
          <h3 id="ple-question-json-match-prompts-heading">Prompts</h3>
          <Index each={props.prompts}>
            {(item, index) => (
              <div class="ple-question-json-authoring__choice">
                <label class="ple-question-json-authoring__field">
                  <span>Prompt {index + 1}</span>
                  <textarea
                    value={item().text}
                    disabled={props.disabled}
                    onInput={(event) =>
                      props.onPromptTextChange(item().id, event.currentTarget.value)
                    }
                  />
                </label>
                <div class="ple-question-json-authoring__row-actions">
                  <button
                    type="button"
                    class="quiet-action"
                    disabled={props.disabled || index === 0}
                    onClick={() => props.onMoveItem("prompts", item().id, "earlier")}
                  >
                    Earlier
                  </button>
                  <button
                    type="button"
                    class="quiet-action"
                    disabled={props.disabled || index === props.prompts.length - 1}
                    onClick={() => props.onMoveItem("prompts", item().id, "later")}
                  >
                    Later
                  </button>
                  <button
                    type="button"
                    class="quiet-action"
                    disabled={props.disabled || props.prompts.length <= 2}
                    onClick={() => props.onRemovePair(item().id)}
                  >
                    Remove pair
                  </button>
                </div>
              </div>
            )}
          </Index>
        </section>
        <section aria-labelledby="ple-question-json-match-choices-heading">
          <h3 id="ple-question-json-match-choices-heading">Choices</h3>
          <Index each={props.choices}>
            {(item, index) => (
              <div class="ple-question-json-authoring__choice">
                <label class="ple-question-json-authoring__field">
                  <span>Choice {index + 1}</span>
                  <textarea
                    value={item().text}
                    disabled={props.disabled}
                    onInput={(event) =>
                      props.onChoiceTextChange(item().id, event.currentTarget.value)
                    }
                  />
                </label>
                <div class="ple-question-json-authoring__row-actions">
                  <button
                    type="button"
                    class="quiet-action"
                    disabled={props.disabled || index === 0}
                    onClick={() => props.onMoveItem("choices", item().id, "earlier")}
                  >
                    Earlier
                  </button>
                  <button
                    type="button"
                    class="quiet-action"
                    disabled={props.disabled || index === props.choices.length - 1}
                    onClick={() => props.onMoveItem("choices", item().id, "later")}
                  >
                    Later
                  </button>
                </div>
              </div>
            )}
          </Index>
        </section>
      </div>
      <div class="ple-question-json-authoring__actions">
        <button
          type="button"
          class="quiet-action"
          disabled={props.disabled || props.prompts.length >= 100}
          onClick={() => {
            props.onAddPair();
            props.onStatus?.("Added a matched prompt and choice.");
          }}
        >
          Add pair
        </button>
      </div>
      <section aria-labelledby="ple-question-json-match-pairing-heading">
        <h3 id="ple-question-json-match-pairing-heading">Private pairing check</h3>
        <p class="ple-question-json-authoring__help">
          This answer map is never included in student preview.
        </p>
        <For each={props.prompts}>
          {(prompt, index) => (
            <label class="ple-question-json-authoring__field">
              <span>
                Pair prompt {index() + 1}: {prompt.text}
              </span>
              <select
                value={matchedChoice(prompt.id)}
                disabled={props.disabled}
                onChange={(event) => props.onMatchChange(prompt.id, event.currentTarget.value)}
              >
                <option value="">Choose its match</option>
                <For each={props.choices}>
                  {(choice) => (
                    <option value={choice.id} disabled={choiceUsedElsewhere(prompt.id, choice.id)}>
                      {choice.text}
                    </option>
                  )}
                </For>
              </select>
            </label>
          )}
        </For>
      </section>
    </fieldset>
  );
}
