// question_json_editor_workspace.tsx - private draft fields, preview, and publish review.

import { For, Show, type Accessor, type JSX, type Setter } from "solid-js";

import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import { PleQuestionJsonFeedbackFields } from "./question_json_feedback_fields";
import { optionalPleManagedSupportText } from "./question_general_feedback_client";
import { PleQuestionJsonHintField } from "./question_json_hint_field";
import {
  setOutcomeFeedback,
  setPleQuestionJsonPrompt,
  setQuestionHint,
} from "./question_json_editor_model";
import { PleQuestionJsonMetadataFields } from "./question_json_metadata_fields";
import { pleQuestionJsonPublicPreview } from "./question_json_public_preview";
import {
  PleQuestionJsonPreview,
  type PleQuestionJsonInstructorAnswerCheck,
  type PleQuestionJsonPreviewProps,
} from "./question_json_preview";
import { PleQuestionJsonResponseFields } from "./question_json_response_fields";
import type { PleQuestionJsonEditorPageProps } from "./question_json_editor_types";
import type { PleQuestionJsonDocument } from "./question_json_source";
import type { PleQuestionJsonRecordMetadata } from "./question_json_defaults";
import { QuestionPublicationReviewFields } from "./question_publication_review_fields";

export type PleQuestionJsonPublishReview = {
  readonly draftQuestionEditNumber: string;
  readonly baseQuestion: "newQuestion";
  readonly questionTitle: string;
  readonly changed: ReadonlyArray<string>;
};

export interface PleQuestionJsonEditorWorkspaceProps {
  readonly source: Accessor<PleQuestionJsonDocument | null>;
  readonly metadata: Accessor<PleQuestionJsonRecordMetadata>;
  readonly metadataDirty: Accessor<boolean>;
  readonly metadataSaving: Accessor<boolean>;
  readonly currentSource: () => PleQuestionJsonDocument;
  readonly errors: () => Readonly<Record<string, string>>;
  readonly isLocked: () => boolean;
  readonly canSave: () => boolean;
  readonly isSaved: () => boolean;
  readonly canEditGeneralFeedback: () => boolean;
  readonly hasUnsavedGeneralFeedback: () => boolean;
  readonly numericAnswerLiteral: Accessor<string>;
  readonly hotspotPending: Accessor<boolean>;
  readonly generalFeedback: Accessor<string | null>;
  readonly hint: Accessor<string | null>;
  readonly workedSolution: Accessor<string | null>;
  readonly generalFeedbackSaving: Accessor<boolean>;
  readonly showInstructorCheck: Accessor<boolean>;
  readonly review: Accessor<PleQuestionJsonPublishReview | null>;
  readonly authorshipText: Accessor<string>;
  readonly disciplineUuid: Accessor<string | null>;
  readonly subjectUuid: Accessor<string | null>;
  readonly topicUuid: Accessor<string | null>;
  readonly subtopicUuid: Accessor<string | null>;
  readonly publishedQuestionId: Accessor<string | null>;
  readonly publishedSummary: Accessor<QuestionSummary | undefined>;
  readonly hotspotDraftQuestionImage: () => PleQuestionJsonPreviewProps["hotspotDraftQuestionImage"];
  readonly instructorAnswerCheck: (
    draft: PleQuestionJsonDocument,
  ) => PleQuestionJsonInstructorAnswerCheck | undefined;
  readonly classificationClient: PleQuestionJsonEditorPageProps["classificationClient"];
  readonly responseValidator: PleQuestionJsonEditorPageProps["responseValidator"];
  readonly draftPreviewPanel?: JSX.Element;
  readonly questionImagePreviewPath: (asset: string) => string;
  readonly onEdit: (source: PleQuestionJsonDocument) => void;
  readonly onMetadataChange: (metadata: PleQuestionJsonRecordMetadata) => void;
  readonly onSaveMetadata: () => void;
  readonly onNumericAnswerLiteralChange: (literal: string) => void;
  readonly onMoveChoice: (choiceId: string, direction: "up" | "down") => void;
  readonly onStatus: (status: string | null) => void;
  readonly onHotspotPendingChange: Setter<boolean>;
  readonly onHotspotLiteralValidityChange: Setter<boolean>;
  readonly onUpload: (file: File, signal: AbortSignal) => Promise<void>;
  readonly onGeneralFeedbackChange: Setter<string | null>;
  readonly onHintChange: Setter<string | null>;
  readonly onWorkedSolutionChange: Setter<string | null>;
  readonly onSaveGeneralFeedback: () => void;
  readonly onSave: () => void;
  readonly onInspectInstructorAnswer: () => void;
  readonly onOpenPublishReview: () => void;
  readonly onAuthorshipTextChange: Setter<string>;
  readonly onAuthorshipInput: (element: HTMLTextAreaElement) => void;
  readonly onDisciplineChange: (uuid: string | null) => void;
  readonly onSubjectChange: (uuid: string | null) => void;
  readonly onTopicChange: (uuid: string | null) => void;
  readonly onSubtopicChange: Setter<string | null>;
  readonly onPublish: () => void;
}

export function PleQuestionJsonEditorWorkspace(
  props: PleQuestionJsonEditorWorkspaceProps,
): JSX.Element {
  return (
    <>
      <Show when={props.source()}>
        {(_draft) => (
          <div class="editor-grid">
            <section class="editor-panel">
              <label class="ple-question-json-authoring__field">
                <span>Student-facing prompt</span>
                <textarea
                  value={props.currentSource().prompt}
                  disabled={props.isLocked()}
                  aria-invalid={props.errors()["prompt"] !== undefined}
                  onInput={(event) =>
                    props.onEdit(
                      setPleQuestionJsonPrompt(props.currentSource(), event.currentTarget.value),
                    )
                  }
                />
              </label>
              <PleQuestionJsonResponseFields
                source={props.currentSource}
                fieldErrors={props.errors()}
                disabled={props.isLocked()}
                numericAnswerLiteral={props.numericAnswerLiteral}
                onNumericAnswerLiteralChange={props.onNumericAnswerLiteralChange}
                onEdit={props.onEdit}
                onMoveChoice={props.onMoveChoice}
                onStatus={props.onStatus}
                selectedKind={() => props.currentSource().response.kind}
                onHotspotPendingChange={props.onHotspotPendingChange}
                hotspotPending={props.hotspotPending}
                onHotspotLiteralValidityChange={props.onHotspotLiteralValidityChange}
                onUpload={props.onUpload}
                questionImagePreviewPath={props.questionImagePreviewPath}
              />
              <PleQuestionJsonHintField
                value={props.currentSource().questionHint}
                fieldErrors={props.errors()}
                disabled={props.isLocked()}
                onChange={(questionHint) =>
                  props.onEdit(setQuestionHint(props.currentSource(), questionHint))
                }
              />
              <PleQuestionJsonFeedbackFields
                value={props.currentSource().feedback}
                fieldErrors={props.errors()}
                disabled={props.isLocked()}
                onChange={(patch) =>
                  props.onEdit(
                    setOutcomeFeedback(props.currentSource(), {
                      ...props.currentSource().feedback,
                      ...patch,
                    }),
                  )
                }
              />
              <fieldset>
                <legend>PLE-managed support</legend>
                <p class="ple-question-json-authoring__help">
                  Optional Hint, Question Feedback, and Worked Solution are stored with this Draft
                  and copied onto the next Published Question Revision. They stay separate from this
                  Question's source and from transient feedback generated by a Question Backend.
                </p>
                <label class="ple-question-json-authoring__field">
                  <span>Hint (optional)</span>
                  <textarea
                    value={props.hint() ?? ""}
                    disabled={!props.canEditGeneralFeedback()}
                    onInput={(event) =>
                      props.onHintChange(optionalPleManagedSupportText(event.currentTarget.value))
                    }
                  />
                </label>
                <label class="ple-question-json-authoring__field">
                  <span>General Feedback (optional)</span>
                  <textarea
                    value={props.generalFeedback() ?? ""}
                    disabled={!props.canEditGeneralFeedback()}
                    aria-describedby="ple-question-json-general-feedback-help"
                    onInput={(event) =>
                      props.onGeneralFeedbackChange(
                        optionalPleManagedSupportText(event.currentTarget.value),
                      )
                    }
                  />
                  <span
                    id="ple-question-json-general-feedback-help"
                    class="ple-question-json-authoring__help"
                  >
                    Save the Question source first when it has local edits. This text is not backend
                    source or captured backend feedback.
                  </span>
                </label>
                <label class="ple-question-json-authoring__field">
                  <span>Worked Solution (optional)</span>
                  <textarea
                    value={props.workedSolution() ?? ""}
                    disabled={!props.canEditGeneralFeedback()}
                    onInput={(event) =>
                      props.onWorkedSolutionChange(
                        optionalPleManagedSupportText(event.currentTarget.value),
                      )
                    }
                  />
                </label>
                <button
                  type="button"
                  class="quiet-action"
                  disabled={!props.canEditGeneralFeedback() || !props.hasUnsavedGeneralFeedback()}
                  onClick={() => void props.onSaveGeneralFeedback()}
                >
                  {props.generalFeedbackSaving() ? "Saving support text..." : "Save support text"}
                </button>
              </fieldset>
              <div class="editor-actions">
                <button
                  type="button"
                  class="primary-action"
                  disabled={!props.canSave() || props.isLocked()}
                  onClick={() => void props.onSave()}
                >
                  Save private draft
                </button>
                <button
                  type="button"
                  class="quiet-action"
                  disabled={!props.isSaved() || props.isLocked()}
                  onClick={props.onInspectInstructorAnswer}
                >
                  Check instructor answer
                </button>
              </div>
            </section>
            <aside class="editor-preview">
              <section class="editor-panel">
                <For each={[props.currentSource()]}>
                  {(draft) => (
                    <PleQuestionJsonPreview
                      preview={pleQuestionJsonPublicPreview(draft, props.metadata())}
                      hotspotDraftQuestionImage={props.hotspotDraftQuestionImage()}
                      validator={props.responseValidator}
                      instructorAnswerCheck={
                        props.showInstructorCheck() && props.isSaved()
                          ? props.instructorAnswerCheck(draft)
                          : undefined
                      }
                    />
                  )}
                </For>
              </section>
              {props.draftPreviewPanel}
              <PleQuestionJsonMetadataFields
                metadata={props.metadata()}
                disabled={props.isLocked()}
                onMetadataChange={props.onMetadataChange}
              />
              <div class="editor-actions">
                <button
                  type="button"
                  class="primary-action"
                  disabled={props.isLocked() || props.metadataSaving() || !props.metadataDirty()}
                  onClick={props.onSaveMetadata}
                >
                  {props.metadataSaving()
                    ? "Saving Question metadata..."
                    : "Save Question metadata"}
                </button>
                <Show when={props.metadataDirty()}>
                  <span role="status">Question metadata has unsaved changes.</span>
                </Show>
              </div>
              <section class="editor-panel" aria-labelledby="ple-question-json-publish-heading">
                <h2 id="ple-question-json-publish-heading">Publish review</h2>
                <p>Review the saved content before publishing a new Question ID.</p>
                <Show when={props.review() === null}>
                  <button
                    type="button"
                    class="primary-action"
                    disabled={!props.isSaved() || props.isLocked()}
                    onClick={() => void props.onOpenPublishReview()}
                  >
                    Review publication changes
                  </button>
                </Show>
                <Show when={props.review()}>
                  {(activeReview) => (
                    <QuestionPublicationReviewFields
                      questionTitle={activeReview().questionTitle}
                      changed={activeReview().changed}
                      isSaved={props.isSaved}
                      isLocked={props.isLocked}
                      authorshipText={props.authorshipText()}
                      disciplineUuid={props.disciplineUuid()}
                      subjectUuid={props.subjectUuid()}
                      topicUuid={props.topicUuid()}
                      subtopicUuid={props.subtopicUuid()}
                      classificationClient={props.classificationClient}
                      onAuthorshipTextChange={props.onAuthorshipTextChange}
                      onAuthorshipInput={props.onAuthorshipInput}
                      onDisciplineChange={props.onDisciplineChange}
                      onSubjectChange={props.onSubjectChange}
                      onTopicChange={props.onTopicChange}
                      onSubtopicChange={props.onSubtopicChange}
                      onPublish={props.onPublish}
                    />
                  )}
                </Show>
              </section>
            </aside>
          </div>
        )}
      </Show>
      <Show when={props.publishedQuestionId()}>
        {(libraryPath) => (
          <section class="editor-panel" role="status">
            <h2>Published</h2>
            <Show when={props.publishedSummary()} keyed>
              {(summary) => (
                <>
                  {/* ASVS 1.2.1: server-validated Question Library fields render as Solid text, not HTML. */}
                  <p>
                    <strong>Question:</strong> {props.metadata().questionTitle}
                  </p>
                  <p>
                    <strong>Question ID:</strong> <code>{summary.questionId}</code>
                  </p>
                  <p>
                    <strong>Published Revision:</strong>{" "}
                    {summary.publishedQuestionRevisionTuple.revisionNumber}
                  </p>
                  <p>
                    <strong>Published to:</strong> Question Library
                  </p>
                  <p>
                    <strong>Authors:</strong>{" "}
                    {summary.authorship.authors.map((author) => author.displayName).join(", ")}
                  </p>
                </>
              )}
            </Show>
            <a class="primary-action" href={libraryPath()}>
              Open published Question
            </a>
          </section>
        )}
      </Show>
    </>
  );
}
