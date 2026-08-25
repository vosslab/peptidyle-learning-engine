// assignment_editor_problem_picker.tsx - modal boundary for assignment question selection.

import { Show, type JSX } from "solid-js";

import { ProblemPicker } from "../features/problem_picker";
import type { AssignmentEditorRepository } from "./assignment_editor_repository";
import type { AssignmentEditorPickerController } from "./assignment_editor_picker_controller";

export interface AssignmentEditorProblemPickerProps {
  readonly repository: AssignmentEditorRepository;
  readonly controller: AssignmentEditorPickerController;
}

function pickerTitle(
  intent: NonNullable<ReturnType<AssignmentEditorPickerController["intent"]>>,
): string {
  if (intent.kind === "pool") return "Choose pool candidates";
  if (intent.kind === "replacement") return "Choose a replacement question";
  return "Choose assignment questions";
}

function pickerConfirmLabel(
  intent: NonNullable<ReturnType<AssignmentEditorPickerController["intent"]>>,
): string {
  if (intent.kind === "pool") return "Add selected candidates";
  if (intent.kind === "replacement") return "Use selected replacement";
  return "Add selected questions";
}

/** Keeps the native dialog mounted only while one assignment destination is active. */
export function AssignmentEditorProblemPicker(
  props: AssignmentEditorProblemPickerProps,
): JSX.Element {
  return (
    <Show when={props.controller.intent()} keyed>
      {(intent) => (
        <ProblemPicker
          repository={props.repository.problemPickerRepository}
          sources={props.controller.sources()}
          mode={intent.kind === "replacement" ? "one" : "many"}
          maximumSelection={props.controller.maximum(intent)}
          trigger={props.controller.trigger()}
          title={pickerTitle(intent)}
          confirmLabel={pickerConfirmLabel(intent)}
          onConfirm={(selection) => void props.controller.useSelection(selection)}
          onCancel={props.controller.cancel}
        />
      )}
    </Show>
  );
}
