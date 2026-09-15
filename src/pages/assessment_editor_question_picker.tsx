// assessment_editor_question_picker.tsx - modal boundary for Assessment Question Picker selection.

import { A } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import { QuestionPicker } from "../features/question_picker";
import type { AssessmentEditorRepository } from "./assessment_editor_repository";
import type { AssessmentEditorPickerController } from "./assessment_editor_picker_controller";

export interface AssessmentEditorQuestionPickerProps {
  readonly repository: AssessmentEditorRepository;
  readonly controller: AssessmentEditorPickerController;
}

function pickerTitle(
  intent: NonNullable<ReturnType<AssessmentEditorPickerController["intent"]>>,
): string {
  if (intent.kind === "pool") return "Choose Questions for pool";
  return "Choose assessment questions";
}

function pickerConfirmLabel(
  intent: NonNullable<ReturnType<AssessmentEditorPickerController["intent"]>>,
): string {
  if (intent.kind === "pool") return "Add selected Questions to pool";
  return "Add selected questions";
}

/** Renders the native dialog only while one assessment destination is active. */
export function AssessmentEditorQuestionPicker(
  props: AssessmentEditorQuestionPickerProps,
): JSX.Element {
  return (
    <>
      <p class="assessment-picker-blueprint-course-link">
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
