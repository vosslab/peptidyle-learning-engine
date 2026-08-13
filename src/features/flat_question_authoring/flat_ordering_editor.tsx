// flat_ordering_editor.tsx - compact private controls for an ORDER response.

import { For, Show, type JSX } from "solid-js";

import type { FlatQuestionItem } from "./flat_question_source";

const MINIMUM_ITEMS = 3;
const MAXIMUM_ITEMS = 100;

export interface FlatOrderingEditorProps {
  /** The ordered items are the private answer key; correctOrder is derived by the model. */
  readonly items: () => ReadonlyArray<FlatQuestionItem>;
  readonly onItemTextChange: (itemId: string, text: string) => void;
  readonly onAddItem: () => void;
  readonly onRemoveItem: (itemId: string) => void;
  readonly onMoveItem: (itemId: string, direction: "earlier" | "later") => void;
  readonly onStatus?: (message: string) => void;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
  readonly disabled?: boolean;
}

function errorFor(props: FlatOrderingEditorProps, path: string): string | undefined {
  return props.fieldErrors?.[path];
}

/** Uses an identity-preserving For list so editing and movement never remount a neighboring row. */
export function FlatOrderingEditor(props: FlatOrderingEditorProps): JSX.Element {
  function announce(message: string): void {
    props.onStatus?.(message);
  }

  return (
    <fieldset class="flat-question-authoring__ordering">
      <legend>Correct order</legend>
      <p class="flat-question-authoring__help">
        This private list is the intended sequence. Move an item earlier or later to set the answer
        learners must arrange.
      </p>
      <Show when={errorFor(props, "response.items") !== undefined}>
        <p class="flat-question-authoring__error" role="alert">
          {errorFor(props, "response.items")}
        </p>
      </Show>
      <ol class="flat-question-authoring__choice-list">
        <For each={props.items().map((item) => item.id)}>
          {(itemId, index): JSX.Element => {
            const item = (): FlatQuestionItem => {
              const current = props.items().find((candidate) => candidate.id === itemId);
              if (current === undefined) throw new Error("Ordering item identity is unavailable.");
              return current;
            };
            const textError = (): string | undefined =>
              errorFor(props, `response.items.${itemId}.text`);
            return (
              <li class="flat-question-authoring__choice">
                <div class="flat-question-authoring__choice-header">
                  <h3 class="flat-question-authoring__choice-title">Position {index() + 1}</h3>
                  <div
                    class="flat-question-authoring__row-actions"
                    aria-label={`Position ${index() + 1} actions`}
                  >
                    <button
                      type="button"
                      class="quiet-action"
                      disabled={props.disabled || index() === 0}
                      onClick={() => {
                        props.onMoveItem(itemId, "earlier");
                        announce(`Moved item ${index() + 1} earlier in the correct order.`);
                      }}
                    >
                      Earlier
                    </button>
                    <button
                      type="button"
                      class="quiet-action"
                      disabled={props.disabled || index() === props.items().length - 1}
                      onClick={() => {
                        props.onMoveItem(itemId, "later");
                        announce(`Moved item ${index() + 1} later in the correct order.`);
                      }}
                    >
                      Later
                    </button>
                    <button
                      type="button"
                      class="quiet-action"
                      disabled={props.disabled || props.items().length <= MINIMUM_ITEMS}
                      onClick={() => props.onRemoveItem(itemId)}
                    >
                      Remove item
                    </button>
                  </div>
                </div>
                <label class="flat-question-authoring__field">
                  <span>Item text</span>
                  <textarea
                    value={item().text}
                    disabled={props.disabled}
                    aria-invalid={textError() !== undefined}
                    aria-describedby={
                      textError() === undefined ? undefined : `${itemId}-text-error`
                    }
                    onInput={(event) => props.onItemTextChange(itemId, event.currentTarget.value)}
                  />
                </label>
                <Show when={textError() !== undefined}>
                  <p
                    class="flat-question-authoring__error"
                    id={`${itemId}-text-error`}
                    role="alert"
                  >
                    {textError()}
                  </p>
                </Show>
                <details>
                  <summary>Advanced item identity</summary>
                  <p class="flat-question-authoring__identity">{itemId}</p>
                </details>
              </li>
            );
          }}
        </For>
      </ol>
      <div class="flat-question-authoring__actions">
        <button
          type="button"
          class="quiet-action"
          disabled={props.disabled || props.items().length >= MAXIMUM_ITEMS}
          onClick={props.onAddItem}
        >
          Add item
        </button>
        <p class="flat-question-authoring__help">
          {props.items().length} of {MAXIMUM_ITEMS} items; at least {MINIMUM_ITEMS} are required.
        </p>
      </div>
    </fieldset>
  );
}
