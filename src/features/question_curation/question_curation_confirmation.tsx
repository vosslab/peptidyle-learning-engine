// Accessible native confirmation for named Question Curation deletion.

import type { JSX } from "solid-js";

import {
  questionCurationConfirmationPresentation,
  type QuestionCurationDeletion,
} from "./question_curation_model";

export interface QuestionCurationConfirmationProps {
  readonly deletion: QuestionCurationDeletion;
  readonly busy: boolean;
  readonly onCancel: () => void;
  readonly onConfirm: () => void;
}

/** Keeps Cancel first in reading, tab, and initial-focus order for destructive work. */
export function QuestionCurationConfirmationDialog(
  props: QuestionCurationConfirmationProps,
): JSX.Element {
  let cancelButton: HTMLButtonElement | undefined;
  const presentation = questionCurationConfirmationPresentation(props.deletion);

  return (
    <dialog
      class="confirmation-dialog"
      aria-labelledby={presentation.labelledBy}
      aria-describedby={presentation.describedBy}
      ref={(element) => {
        queueMicrotask(() => {
          element.showModal();
          cancelButton?.focus({ preventScroll: true });
        });
      }}
      onCancel={(event) => {
        event.preventDefault();
        props.onCancel();
      }}
    >
      <h2 id={presentation.labelledBy}>{presentation.heading}</h2>
      <p id={presentation.describedBy}>{presentation.consequence}</p>
      <div class="question-curation-actions">
        <button
          ref={(element) => {
            cancelButton = element;
          }}
          class="quiet-action"
          type="button"
          autofocus={presentation.actions[0].initial}
          disabled={props.busy}
          onClick={props.onCancel}
        >
          {presentation.actions[0].label}
        </button>
        <button
          class="primary-action"
          type="button"
          disabled={props.busy}
          onClick={props.onConfirm}
        >
          {presentation.actions[1].label}
        </button>
      </div>
    </dialog>
  );
}
