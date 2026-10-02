// Answer-free inspection of one Question before it is added to an Assessment.

import { Show, type JSX } from "solid-js";

import { OpaqueWebworkPreviewFrame } from "../../components/opaque_webwork_preview_frame";
import { QuestionResponsePreviewControl } from "../../components/question_response_preview";
import {
  QuestionPromptRenderer,
  type QuestionImageUrlResolver,
} from "../../components/question_renderer";
import type { QuestionPickerInspectionView } from "./question_picker_model";

export function QuestionPickerInspection(props: {
  readonly view: QuestionPickerInspectionView;
  readonly previewDocumentUrl: string | null;
  readonly questionImageUrl: QuestionImageUrlResolver;
  readonly onClose: () => void;
}): JSX.Element {
  const tuple = (): QuestionPickerInspectionView["publishedQuestionRevisionTuple"] =>
    props.view.publishedQuestionRevisionTuple;
  return (
    <section class="question-picker-inspection" aria-label="Question inspection">
      <h3>Inspect before adding</h3>
      <p>{props.view.questionTitle}</p>
      <p>
        Question ID {props.view.questionId}, Revision {tuple().revisionNumber}. This inspection does
        not add the Question.
      </p>
      <Show
        when={props.view.backend === "webwork" && props.previewDocumentUrl !== null}
        fallback={
          <QuestionPromptRenderer
            blocks={props.view.prompt.blocks}
            publishedQuestionRevisionTuple={tuple()}
            questionImageUrl={props.questionImageUrl}
          />
        }
      >
        <OpaqueWebworkPreviewFrame
          class="question-picker-inspection-frame"
          src={props.previewDocumentUrl ?? ""}
          title={`Generated example for ${props.view.questionTitle}, Revision ${tuple().revisionNumber}`}
        />
      </Show>
      <Show when={props.view.backend !== "webwork" && props.view.responsePreview}>
        {(preview) => (
          <QuestionResponsePreviewControl
            preview={preview()}
            publishedQuestionRevisionTuple={tuple()}
            questionImageUrl={props.questionImageUrl}
          />
        )}
      </Show>
      <button class="quiet-action" type="button" onClick={() => props.onClose()}>
        Close inspection
      </button>
    </section>
  );
}
