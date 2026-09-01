// question_json_hint_field.tsx - author-only pre-response Question Hint control.

import { Show, type JSX } from "solid-js";

export interface PleQuestionJsonHintFieldProps {
  readonly value: string | null;
  readonly onChange: (value: string | null) => void;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
  readonly disabled?: boolean;
}

/** Keeps learner-requested help separate from feedback released after a response. */
export function PleQuestionJsonHintField(props: PleQuestionJsonHintFieldProps): JSX.Element {
  const error = (): string | undefined => props.fieldErrors?.questionHint;
  return (
    <fieldset>
      <legend>Question Hint</legend>
      <p class="ple-question-json-authoring__help">
        Students request this before selecting, entering, or submitting a response. It is separate
        from outcome feedback.
      </p>
      <label class="ple-question-json-authoring__field">
        <span>Question Hint (optional)</span>
        <textarea
          value={props.value ?? ""}
          disabled={props.disabled}
          aria-invalid={error() !== undefined}
          aria-describedby={error() === undefined ? undefined : "ple-question-json-hint-error"}
          onInput={(event) =>
            props.onChange(
              event.currentTarget.value.trim() === "" ? null : event.currentTarget.value,
            )
          }
        />
      </label>
      <Show when={error() !== undefined}>
        <p
          class="ple-question-json-authoring__error"
          id="ple-question-json-hint-error"
          role="alert"
        >
          {error()}
        </p>
      </Show>
    </fieldset>
  );
}
