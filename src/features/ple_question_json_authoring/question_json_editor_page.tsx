// question_json_editor_page.tsx - private instructor surface for ple-question-json authoring.

import { Show, createEffect, onCleanup, onMount, type JSX } from "solid-js";

import { PageFrame } from "../../components/page_frame";
import { UnsavedChangesGuard } from "../../components/unsaved_changes_guard";
import { PLE_QUESTION_JSON_EDITOR_STYLES } from "./question_json_editor_styles";
import type { PleQuestionJsonPreviewProps } from "./question_json_preview";
import type { PleQuestionJsonEditorPageProps } from "./question_json_editor_types";
import { PleQuestionJsonEditorWorkspace } from "./question_json_editor_workspace";
import { DraftPreviewPanel } from "../question_draft_preview/draft_preview_panel";
import { DraftCorrectionPublishControl } from "../question_draft_preview/draft_correction_publish_control";
import { questionJsonAnswerCheck } from "./question_json_answer_check";
import { createQuestionJsonEditorController } from "./question_json_editor_controller";
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
  function hotspotDraftQuestionImage(): PleQuestionJsonPreviewProps["hotspotDraftQuestionImage"] {
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
  const headingId = "ple-question-json-authoring-heading";
  let authorshipInput: HTMLTextAreaElement | undefined;
  let headingFocusDelivered = false;

  function focusHeading(): void {
    const heading = document.getElementById(headingId) as HTMLHeadingElement | null;
    if (heading === null) return;
    heading.tabIndex = -1;
    heading.focus();
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
      if (document.getElementById(headingId) !== null) {
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
      <style>{PLE_QUESTION_JSON_EDITOR_STYLES}</style>
      <PageFrame
        eyebrow={
          editor.publishedQuestionId() ? "Publication complete" : "Private instructor authoring"
        }
        title={editor.publishedQuestionId() ? "Question published" : "PLE Question JSON"}
        lede={
          editor.publishedQuestionId()
            ? "Your authoring work is complete. This confirmation identifies the Question now available in the Question Library."
            : "Build a clear student question, save it privately, then review and publish it when it is ready."
        }
        routeSurface="pleQuestionJsonEditor"
        headingId={headingId}
      >
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
          disciplineUuid={editor.disciplineUuid}
          subjectUuid={editor.subjectUuid}
          topicUuid={editor.topicUuid}
          subtopicUuid={editor.subtopicUuid}
          publishedQuestionId={editor.publishedQuestionId}
          publishedSummary={editor.publishedSummary}
          hotspotDraftQuestionImage={hotspotDraftQuestionImage}
          instructorAnswerCheck={(draft) => questionJsonAnswerCheck(draft) ?? undefined}
          classificationClient={props.classificationClient}
          responseValidator={props.responseValidator}
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
              responseValidator={props.responseValidator}
              hotspotDraftQuestionImage={hotspotDraftQuestionImage()}
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
