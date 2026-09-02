// assignment_editor_question_picker.tsx - modal boundary for Assignment Question Picker selection.

import { A } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import { QuestionPicker } from "../features/question_picker";
import type { AssignmentEditorRepository } from "./assignment_editor_repository";
import type { AssignmentEditorPickerController } from "./assignment_editor_picker_controller";

export interface AssignmentEditorQuestionPickerProps {
  readonly repository: AssignmentEditorRepository;
  readonly controller: AssignmentEditorPickerController;
}

function pickerTitle(
  intent: NonNullable<ReturnType<AssignmentEditorPickerController["intent"]>>,
): string {
  if (intent.kind === "pool") return "Choose Questions for pool";
  return "Choose assignment questions";
}

function pickerConfirmLabel(
  intent: NonNullable<ReturnType<AssignmentEditorPickerController["intent"]>>,
): string {
  if (intent.kind === "pool") return "Add selected Questions to pool";
  return "Add selected questions";
}

/** Keeps the native dialog mounted only while one assignment destination is active. */
export function AssignmentEditorQuestionPicker(
  props: AssignmentEditorQuestionPickerProps,
): JSX.Element {
  return (
    <>
      <p class="assignment-picker-blueprint-course-link">
        <A class="quiet-link" href="/blueprint-courses">
          Open Blueprint Courses
        </A>
      </p>
      <Show when={props.controller.intent()} keyed>
        {(intent) => (
          <QuestionPicker
            repository={props.repository.questionPickerRepository}
            sources={props.controller.sources()}
            mode="many"
            maximumSelection={props.controller.maximum(intent)}
            trigger={props.controller.trigger()}
            title={pickerTitle(intent)}
            confirmLabel={pickerConfirmLabel(intent)}
            onConfirm={(selection) => void props.controller.useSelection(selection)}
            onCancel={props.controller.cancel}
          />
        )}
      </Show>
    </>
  );
}
