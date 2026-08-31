// flat_fill_in_editor.tsx - compact private controls for accepted text responses.

import { Index, Show, type JSX } from "solid-js";

import {
  addFillInAnswer,
  removeFillInAnswer,
  setFillInAnswer,
  setFillInMatchMode,
  setFillInMaxLength,
  validateFillInResponse,
} from "./flat_fill_in_model";
import type {
  FlatQuestionFillInResponse,
  FlatQuestionTextResponseMatchRule,
} from "./flat_question_source";

const MATCH_MODE_HELP: Readonly<Record<FlatQuestionTextResponseMatchRule, string>> = {
  exact: "Students must use the same capitalization and spacing.",
  caseInsensitive: "Students may vary capitalization; spelling and spacing still need to match.",
  normalized: "Students may vary capitalization and ordinary spacing.",
};

function isTextResponseMatchRule(value: string): value is FlatQuestionTextResponseMatchRule {
  return value === "exact" || value === "caseInsensitive" || value === "normalized";
}

export interface FlatFillInEditorProps {
  readonly response: () => FlatQuestionFillInResponse;
  readonly onResponseChange: (response: FlatQuestionFillInResponse) => void;
  readonly disabled?: boolean;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
}

/** Each row preserves its exact authored text while showing concise, local recovery guidance. */
export function FlatFillInEditor(props: FlatFillInEditorProps): JSX.Element {
  const validation = (): ReturnType<typeof validateFillInResponse> =>
    validateFillInResponse(props.response());
  const errorFor = (field: string): string | undefined =>
    props.fieldErrors?.[`response.${field}`] ??
    props.fieldErrors?.[field] ??
    validation().issues[field];
  return (
    <fieldset class="flat-question-authoring__fill-in">
      <legend>Accepted text answers</legend>
      <p class="flat-question-authoring__help">
        Add the answer wording you will accept. Students see the prompt and response limit, not this
        list.
      </p>
      <Index each={props.response().answers}>
        {(answer, index) => {
          const errorId = `flat-fill-in-answer-${index}-error`;
          const error = (): string | undefined => errorFor(`answers.${index}`);
          return (
            <div class="flat-question-authoring__choice">
              <label class="flat-question-authoring__field">
                <span>Accepted answer {index + 1}</span>
                <textarea
                  value={answer()}
                  disabled={props.disabled}
                  aria-invalid={error() !== undefined}
                  aria-describedby={error() === undefined ? undefined : errorId}
                  onInput={(event) =>
                    props.onResponseChange(
                      setFillInAnswer(props.response(), index, event.currentTarget.value),
                    )
                  }
                />
              </label>
              <Show when={error() !== undefined}>
                <p class="flat-question-authoring__error" id={errorId} role="alert">
                  {error()}
                </p>
              </Show>
              <button
                type="button"
                class="quiet-action"
                disabled={props.disabled || props.response().answers.length <= 1}
                onClick={() => props.onResponseChange(removeFillInAnswer(props.response(), index))}
              >
                Remove answer
              </button>
            </div>
          );
        }}
      </Index>
      <div class="flat-question-authoring__actions">
        <button
          type="button"
          class="quiet-action"
          disabled={props.disabled}
          onClick={() => props.onResponseChange(addFillInAnswer(props.response()))}
        >
          Add accepted answer
        </button>
      </div>
      <label class="flat-question-authoring__field">
        <span>How should Student text match?</span>
        <select
          value={props.response().matchMode}
          disabled={props.disabled}
          onChange={(event) => {
            const matchMode = event.currentTarget.value;
            if (isTextResponseMatchRule(matchMode)) {
              props.onResponseChange(setFillInMatchMode(props.response(), matchMode));
            }
          }}
        >
          <option value="exact">Exact text</option>
          <option value="caseInsensitive">Ignore capitalization</option>
          <option value="normalized">Ignore capitalization and ordinary spacing</option>
        </select>
        <span class="flat-question-authoring__help">
          {MATCH_MODE_HELP[props.response().matchMode]}
        </span>
      </label>
      <label class="flat-question-authoring__field">
        <span>Maximum Student response length</span>
        <input
          type="number"
          min="1"
          max="16384"
          step="1"
          value={props.response().maxLength}
          disabled={props.disabled}
          aria-invalid={errorFor("maxLength") !== undefined}
          onInput={(event) =>
            props.onResponseChange(
              setFillInMaxLength(props.response(), Number(event.currentTarget.value)),
            )
          }
        />
        <span class="flat-question-authoring__help">
          Use the shortest practical limit for the response you expect.
        </span>
      </label>
      <Show when={errorFor("answers") !== undefined}>
        <p class="flat-question-authoring__error" role="alert">
          {errorFor("answers")}
        </p>
      </Show>
    </fieldset>
  );
}
