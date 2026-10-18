// question_json_editor_page.tsx - private instructor surface for ple-question-json authoring.

import { Show, createEffect, onCleanup, onMount, type JSX } from "solid-js";

import { PageFrame } from "../../components/page_frame/page_frame";
import { UnsavedChangesGuard } from "../../components/unsaved_changes_guard";
import { PLE_QUESTION_JSON_EDITOR_STYLES } from "./question_json_editor_styles";
import type { QuestionResponseControlBaseProps } from "../../components/question_response_controls/common";
import type { PleQuestionJsonEditorPageProps } from "./question_json_editor_types";
import { PleQuestionJsonEditorWorkspace } from "./question_json_editor_workspace";
import { DraftPreviewPanel } from "../question_draft_preview/draft_preview_panel";
import { DraftCorrectionPublishControl } from "../question_draft_preview/draft_correction_publish_control";
import { questionJsonAnswerCheck } from "./question_json_answer_check";
import { createQuestionJsonEditorController } from "./question_json_editor_controller";
import { QuestionPublicationReviewGuard } from "./question_publication_review_fields";
import {
  useClearRouteScopeLabels,
  usePublishRouteScopeLabels,
  useRouteScopePublication,
} from "../../ribbon/route_scope_context";

export type {
  PleQuestionJsonDraftDisplayState,
  PleQuestionJsonEditorPageProps,
} from "./question_json_editor_types";

/**
 * A purpose-built author surface. Student preview is a local answer-free PLE Question JSON Public Preview; this component
 * does not write Draft Question Content to URLs, browser storage, or diagnostics.
 */
export function PleQuestionJsonEditorPage(props: PleQuestionJsonEditorPageProps): JSX.Element {
  const routeScopePublication = useRouteScopePublication();
  const publishRouteScopeLabels = usePublishRouteScopeLabels();
  const clearRouteScopeLabels = useClearRouteScopeLabels();
  let publication: ReturnType<typeof routeScopePublication> | undefined;
  function publishQuestionTitle(title: string): void {
    if (publication !== undefined) publishRouteScopeLabels(publication, { questionTitle: title });
  }
  function draftQuestionImage(): QuestionResponseControlBaseProps["draftQuestionImage"] {
    const draftQuestion = props.draftQuestion;
    const client = props.questionImageClient;
    if (draftQuestion === undefined || client === undefined) return undefined;
    return {
      draftQuestion,
      questionImageUrl: (asset) =>
        new URL(
          client.questionImagePreviewPath(draftQuestion, asset.questionImageAssetId),
          window.location.origin,
        ),
    };
  }
  let authorshipInput: HTMLTextAreaElement | undefined;
  let headingFocusDelivered = false;

  function focusHeading(): void {
    document.getElementById("main-content")?.focus();
  }

  function clearQuestionTitle(): void {
    if (publication !== undefined) clearRouteScopeLabels(publication);
  }

  onMount(() => {
    publication = routeScopePublication();
  });
  const editor = createQuestionJsonEditorController(props, {
    publishQuestionTitle,
    clearQuestionTitle,
    focusHeading,
    focusAuthorship: () => authorshipInput?.focus(),
  });
  createEffect(() => {
    if (
      headingFocusDelivered ||
      props.focusHeadingOnMount !== true ||
      props.replacementPending === true
    ) {
      return;
    }
    headingFocusDelivered = true;
    queueMicrotask(() => {
      if (props.replacementPending === true) {
        headingFocusDelivered = false;
        return;
      }
      if (document.getElementById("main-content") !== null) {
        focusHeading();
        props.onHeadingFocusDelivered?.();
      }
    });
  });

  onCleanup(clearQuestionTitle);

  return (
    <div
      class="ple-question-json-authoring"
      inert={props.replacementPending === true}
      aria-busy={props.replacementPending === true}
    >
      <UnsavedChangesGuard
        autoSaveOnLeave
        dirty={() => !editor.isSaved()}
        save={editor.saveBeforeLeaving}
        copy={{
          heading: "Draft changes could not be saved",
          description: "Your latest Draft changes must save before you leave this editor.",
          saveActionLabel: "Retry save and continue",
          savingActionLabel: "Saving Draft...",
          saveFailureMessage:
            "Your Draft is still open. Resolve the save error, retry, or stay and keep editing.",
        }}
      />
      <QuestionPublicationReviewGuard dirty={editor.reviewDirty} />
      <style>{PLE_QUESTION_JSON_EDITOR_STYLES}</style>
      <PageFrame routeSurface="pleQuestionJsonEditor">
        <Show when={editor.status()}>
          {(message) => (
            <p
              role="status"
              aria-label={
                editor.publishedQuestionId() ? "Publication status" : "Private draft status"
              }
            >
              {message()}
            </p>
          )}
        </Show>
        <Show when={editor.errorMessage()}>
          {(message) => (
            <section class="ple-question-json-authoring__error" role="alert">
              <p>{message()}</p>
              <button type="button" class="quiet-action" onClick={editor.dismissError}>
                Dismiss
              </button>
            </section>
          )}
        </Show>
        <Show when={editor.isConflict()}>
          <section class="ple-question-json-authoring__error" role="alert">
            <p>A newer saved draft exists. Your local edits remain visible for comparison.</p>
            <button type="button" class="primary-action" onClick={() => void editor.reload(true)}>
              Keep local edits and load newest edit number
            </button>
            <button type="button" class="primary-action" onClick={() => void editor.reload()}>
              Discard local edits and reload newest draft
            </button>
          </section>
        </Show>
        <PleQuestionJsonEditorWorkspace
          source={editor.source}
          metadata={editor.metadata}
          metadataDirty={editor.hasUnsavedMetadata}
          metadataSaving={editor.metadataSaving}
          currentSource={editor.currentSource}
          errors={editor.errors}
          isLocked={editor.isLocked}
          canSave={editor.canSave}
          isSaved={editor.isSaved}
          canEditGeneralFeedback={editor.canEditGeneralFeedback}
          hasUnsavedGeneralFeedback={editor.hasUnsavedGeneralFeedback}
          numericAnswerLiteral={editor.numericAnswerLiteral}
          hotspotPending={editor.hotspotPending}
          generalFeedback={editor.generalFeedback}
          hint={editor.hint}
          workedSolution={editor.workedSolution}
          generalFeedbackSaving={editor.generalFeedbackSaving}
          showInstructorCheck={editor.showInstructorCheck}
          review={editor.review}
          authorshipText={editor.authorshipText}
          reviewDirty={editor.reviewDirty}
          onDiscardReviewChanges={editor.discardReviewChanges}
          disciplineUuid={editor.disciplineUuid}
          subjectUuid={editor.subjectUuid}
          topicUuid={editor.topicUuid}
          subtopicUuid={editor.subtopicUuid}
          publishedQuestionId={editor.publishedQuestionId}
          publishedSummary={editor.publishedSummary}
          instructorAnswerCheck={(draft) => questionJsonAnswerCheck(draft) ?? undefined}
          classificationClient={props.classificationClient}
          draftPreviewPanel={
            <DraftPreviewPanel
              draftQuestion={props.draftQuestion}
              draftQuestionEditNumber={editor.latestDraftQuestionEditNumber()}
              binding={{
                backend: "ple",
                format: "pleQuestionJson",
                mediaType: "application/vnd.peptidyle.question+json",
                webworkPgPath: null,
              }}
              isSaved={editor.isSaved()}
              draftQuestionImage={draftQuestionImage()}
              responseValidator={props.responseValidator}
            />
          }
          questionImagePreviewPath={(asset) =>
            asset === ""
              ? ""
              : (props.questionImageClient?.questionImagePreviewPath(props.draftQuestion, asset) ??
                "")
          }
          onEdit={editor.applyEdit}
          onMetadataChange={editor.applyMetadataEdit}
          onSaveMetadata={() => void editor.saveQuestionMetadata()}
          onNumericAnswerLiteralChange={editor.updateNumericAnswerLiteral}
          onMoveChoice={editor.moveChoice}
          onStatus={editor.setStatus}
          onHotspotPendingChange={editor.setHotspotPending}
          onHotspotLiteralValidityChange={editor.setHotspotLiteralsValid}
          onUpload={editor.uploadQuestionImage}
          onGeneralFeedbackChange={editor.applyGeneralFeedbackEdit}
          onHintChange={editor.applyHintEdit}
          onWorkedSolutionChange={editor.applyWorkedSolutionEdit}
          onSaveGeneralFeedback={() => void editor.saveGeneralFeedback()}
          onSave={() => void editor.save()}
          onInspectInstructorAnswer={editor.inspectInstructorAnswer}
          onOpenPublishReview={editor.openPublishReview}
          onAuthorshipTextChange={editor.applyAuthorshipTextEdit}
          selfAuthorPosition={editor.selfAuthorPosition}
          allowSelfAuthorship={props.allowSelfAuthorship}
          onSelfAuthorPositionChange={editor.setSelfAuthorPosition}
          onAuthorshipInput={(element) => {
            authorshipInput = element;
          }}
          onDisciplineChange={editor.setDisciplineUuid}
          onSubjectChange={editor.setSubjectUuid}
          onTopicChange={editor.setTopicUuid}
          onSubtopicChange={editor.setSubtopicUuid}
          onPublish={() => void editor.publish()}
        />
        <Show when={props.correctionParent}>
          {(parent) => (
            <DraftCorrectionPublishControl
              draftQuestion={props.draftQuestion}
              parent={parent()}
              draftQuestionEditNumber={editor.latestDraftQuestionEditNumber()}
              isSaved={editor.isSavedDraft()}
            />
          )}
        </Show>
      </PageFrame>
    </div>
  );
}
