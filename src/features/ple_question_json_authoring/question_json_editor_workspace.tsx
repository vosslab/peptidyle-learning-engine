// question_json_editor_workspace.tsx - private draft fields, preview, and publish review.

import { For, Show, batch, type Accessor, type JSX, type Setter } from "solid-js";

import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import { ContentClassificationSelect } from "../../components/content_classification_select";
import { PleQuestionJsonFeedbackFields } from "./question_json_feedback_fields";
import { PleQuestionJsonHintField } from "./question_json_hint_field";
import {
  setLanguage,
  setOutcomeFeedback,
  setPleQuestionJsonPrompt,
  setPleQuestionJsonQuestionTitle,
  setQuestionCitation,
  setQuestionDescription,
  setQuestionHint,
  setQuestionLicense,
  setTags,
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

export type PleQuestionJsonPublishReview = {
  readonly etag: string;
  readonly baseQuestion: "newQuestion";
  readonly questionTitle: string;
  readonly changed: ReadonlyArray<string>;
};

export interface PleQuestionJsonEditorWorkspaceProps {
  readonly source: Accessor<PleQuestionJsonDocument | null>;
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
  readonly hotspotDraftAsset: () => PleQuestionJsonPreviewProps["hotspotDraftAsset"];
  readonly instructorAnswerCheck: (
    draft: PleQuestionJsonDocument,
  ) => PleQuestionJsonInstructorAnswerCheck | undefined;
  readonly classificationClient: PleQuestionJsonEditorPageProps["classificationClient"];
  readonly responseValidator: PleQuestionJsonEditorPageProps["responseValidator"];
  readonly assetPreviewPath: (asset: string) => string;
  readonly onEdit: (source: PleQuestionJsonDocument) => void;
  readonly onNumericAnswerLiteralChange: (literal: string) => void;
  readonly onMoveChoice: (choiceId: string, direction: "up" | "down") => void;
  readonly onStatus: (status: string | null) => void;
  readonly onHotspotPendingChange: Setter<boolean>;
  readonly onHotspotLiteralValidityChange: Setter<boolean>;
  readonly onUpload: (file: File, signal: AbortSignal) => Promise<void>;
  readonly onGeneralFeedbackChange: Setter<string | null>;
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
                <span>Question Title</span>
                <input
                  value={props.currentSource().questionTitle}
                  disabled={props.isLocked()}
                  aria-invalid={props.errors()["questionTitle"] !== undefined}
                  onInput={(event) =>
                    props.onEdit(
                      setPleQuestionJsonQuestionTitle(
                        props.currentSource(),
                        event.currentTarget.value,
                      ),
                    )
                  }
                />
              </label>
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
                assetPreviewPath={props.assetPreviewPath}
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
                <legend>General Feedback</legend>
                <p class="ple-question-json-authoring__help">
                  PLE-managed general feedback is separate from this Question's source and from
                  transient feedback generated by a Question Backend during an interaction.
                </p>
                <label class="ple-question-json-authoring__field">
                  <span>General Feedback (optional)</span>
                  <textarea
                    value={props.generalFeedback() ?? ""}
                    disabled={!props.canEditGeneralFeedback()}
                    aria-describedby="ple-question-json-general-feedback-help"
                    onInput={(event) =>
                      props.onGeneralFeedbackChange(
                        event.currentTarget.value.trim() === "" ? null : event.currentTarget.value,
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
                <button
                  type="button"
                  class="quiet-action"
                  disabled={!props.canEditGeneralFeedback() || !props.hasUnsavedGeneralFeedback()}
                  onClick={() => void props.onSaveGeneralFeedback()}
                >
                  {props.generalFeedbackSaving()
                    ? "Saving general feedback..."
                    : "Save general feedback"}
                </button>
              </fieldset>
              <PleQuestionJsonMetadataFields
                questionDescription={props.currentSource().questionDescription}
                tags={props.currentSource().tags}
                questionLicense={props.currentSource().questionLicense}
                questionCitation={props.currentSource().questionCitation}
                language={props.currentSource().language}
                fieldErrors={props.errors()}
                disabled={props.isLocked()}
                onQuestionDescriptionChange={(questionDescription) =>
                  props.onEdit(setQuestionDescription(props.currentSource(), questionDescription))
                }
                onTagsChange={(tags) => props.onEdit(setTags(props.currentSource(), tags))}
                onQuestionLicenseChange={(questionLicense) =>
                  props.onEdit(setQuestionLicense(props.currentSource(), questionLicense))
                }
                onQuestionCitationChange={(questionCitation) =>
                  props.onEdit(setQuestionCitation(props.currentSource(), questionCitation))
                }
                onLanguageChange={(language) =>
                  props.onEdit(setLanguage(props.currentSource(), language))
                }
              />
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
                      preview={pleQuestionJsonPublicPreview(draft)}
                      hotspotDraftAsset={props.hotspotDraftAsset()}
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
                    <div class="ple-question-json-authoring__review">
                      <p>
                        <strong>Question:</strong> {activeReview().questionTitle}
                      </p>
                      <p>
                        This publication creates a new Question ID. Existing assessments keep their
                        assigned questions until an instructor deliberately replaces an item.
                      </p>
                      <h3>Changed sections</h3>
                      <ul>
                        <For each={activeReview().changed}>{(section) => <li>{section}</li>}</For>
                      </ul>
                      <label class="ple-question-json-authoring__field">
                        <span>Question Authors</span>
                        <textarea
                          ref={(element) => {
                            props.onAuthorshipInput(element);
                          }}
                          value={props.authorshipText()}
                          onInput={(event) =>
                            props.onAuthorshipTextChange(event.currentTarget.value)
                          }
                          aria-describedby="ple-question-json-authorship-help"
                          disabled={props.isLocked()}
                        />
                        <span
                          id="ple-question-json-authorship-help"
                          class="ple-question-json-authoring__help"
                        >
                          Enter one to sixteen distinct names, one per line. This reviewed text, not
                          account information, is published with the question.
                        </span>
                      </label>
                      <div class="publication-classification-fields">
                        <ContentClassificationSelect
                          label="Discipline"
                          required
                          value={props.disciplineUuid()}
                          disabled={props.isLocked()}
                          load={() => props.classificationClient.listDisciplinesIncludingRetired()}
                          onChange={(uuid) =>
                            batch(() => {
                              props.onDisciplineChange(uuid);
                              props.onSubjectChange(null);
                              props.onTopicChange(null);
                              props.onSubtopicChange(null);
                            })
                          }
                        />
                        <ContentClassificationSelect
                          label="Subject"
                          required
                          value={props.subjectUuid()}
                          parentUuid={props.disciplineUuid()}
                          disabled={props.isLocked()}
                          load={(uuid) => props.classificationClient.listSubjects(uuid)}
                          onChange={(uuid) =>
                            batch(() => {
                              props.onSubjectChange(uuid);
                              props.onTopicChange(null);
                              props.onSubtopicChange(null);
                            })
                          }
                        />
                        <ContentClassificationSelect
                          label="Topic"
                          value={props.topicUuid()}
                          parentUuid={props.subjectUuid()}
                          disabled={props.isLocked()}
                          load={(uuid) => props.classificationClient.listTopics(uuid)}
                          onChange={(uuid) =>
                            batch(() => {
                              props.onTopicChange(uuid);
                              props.onSubtopicChange(null);
                            })
                          }
                        />
                        <ContentClassificationSelect
                          label="Subtopic"
                          value={props.subtopicUuid()}
                          parentUuid={props.topicUuid()}
                          disabled={props.isLocked()}
                          load={(uuid) => props.classificationClient.listSubtopics(uuid)}
                          onChange={props.onSubtopicChange}
                        />
                      </div>
                      <p>Confirming publishes this saved private draft with a new Question ID.</p>
                      <button
                        type="button"
                        class="primary-action"
                        disabled={
                          !props.isSaved() ||
                          props.isLocked() ||
                          props.disciplineUuid() === null ||
                          props.subjectUuid() === null
                        }
                        onClick={() => void props.onPublish()}
                      >
                        Confirm and publish
                      </button>
                    </div>
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
                    <strong>Question:</strong> {summary.metadata.questionTitle}
                  </p>
                  <p>
                    <strong>Question ID:</strong> <code>{summary.questionId}</code>
                  </p>
                  <p>
                    <strong>Published Revision:</strong>{" "}
                    {summary.questionRevisionTuple.revisionNumber}
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
