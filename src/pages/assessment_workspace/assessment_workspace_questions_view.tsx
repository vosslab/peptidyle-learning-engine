// Assessment Question Editor view for the current Assessment workspace.

import { A } from "@solidjs/router";
import { Show, createSignal, type Accessor, type JSX, type Setter } from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { QuestionPoolView } from "../../../generated/api/QuestionPoolView";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type { AssessmentBlueprintUpdateReview } from "../../api/assessment_release";
import type { QuestionLibraryBrowseRepository } from "../../api/question_library_projection";
import { PageFrame } from "../../components/page_frame/page_frame";
import { AssessmentEditor } from "../../features/assessment_content_picker/assessment_editor";
import type { ContentClassificationClient } from "../../api/content_classification";
import type { LibrarySearchRow } from "../../api/question_library_projection";
import { AssessmentPoolEntryEditor } from "./assessment_pool_entry_editor";
import { selectedAssessmentEntryContent } from "./assessment_workspace_selected_entry";
import {
  AssessmentWorkspaceIdentity,
  type AssessmentWorkspaceContextValue,
} from "./assessment_workspace_live_page";
import { assessmentWorkspacePath } from "./assessment_workspace_paths";
import { assessmentPointValueDraft } from "../../assessment_point_value";
import { type AssessmentContentPickerSelection } from "../../features/assessment_content_picker";
import { UnsavedChangesGuard } from "./unsaved_changes_guard";
import { trackEditorSave } from "../../components/unsaved_changes_guard";
import {
  AssessmentBlueprintContentSummary,
  currentBlueprintUpdateContent,
} from "./assessment_blueprint_update_review";
import "./assessment_questions_record_list.css";

export interface AssessmentWorkspaceQuestionsViewArgs {
  readonly workspace: AssessmentWorkspaceContextValue;
  readonly dirty: Accessor<boolean>;
  readonly save: () => Promise<boolean>;
  readonly message: Accessor<string>;
  readonly setMessage: Setter<string>;
  readonly busy: Accessor<boolean>;
  readonly needsReload: Accessor<boolean>;
  readonly reload: (discardLocalChanges?: boolean) => Promise<void>;
  readonly reviewBlueprintUpdate: () => Promise<void>;
  readonly applyBlueprintUpdate: () => Promise<void>;
  readonly blueprintReview: Accessor<AssessmentBlueprintUpdateReview | undefined>;
  readonly setBlueprintReview: Setter<AssessmentBlueprintUpdateReview | undefined>;
  readonly title: Accessor<string>;
  readonly setTitle: Setter<string>;
  readonly entries: Accessor<ReadonlyArray<AssessmentEntry>>;
  readonly bloomSortUnavailableReason: Accessor<string | undefined>;
  readonly sortByBloomClassification: () => void;
  readonly description: (publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple) => string;
  readonly questionTitle: (
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
  ) => string;
  readonly poolTitle: (questionPoolId: string) => string;
  readonly entryBlooms: Accessor<ReadonlyMap<AssessmentEntryId, BloomClassificationView>>;
  readonly move: (entryId: AssessmentEntryId, destinationEntryId: AssessmentEntryId) => void;
  readonly remove: (entryId: AssessmentEntryId) => void;
  readonly poolDetails: Accessor<ReadonlyMap<AssessmentEntryId, QuestionPoolView>>;
  readonly poolDetailLoadFailed: Accessor<boolean>;
  readonly assessmentContentRepository: QuestionLibraryBrowseRepository;
  readonly classificationClient: ContentClassificationClient;
  readonly included: (row: LibrarySearchRow) => boolean;
  readonly addDisabled: (row: LibrarySearchRow) => boolean;
  readonly duplicateAllowed: (row: LibrarySearchRow) => boolean;
  readonly updatePoolEntry: (
    entryId: AssessmentEntryId,
    changes: Partial<
      Pick<
        Extract<AssessmentEntry, { readonly kind: "questionPool" }>,
        "selectionCount" | "pointsPerItem" | "scoringRule"
      >
    >,
  ) => void;
  readonly remainingQuestionCapacity: Accessor<number>;
  readonly addQuestionsById: (value: string) => Promise<boolean>;
  readonly chooseAssessmentContent: (selection: AssessmentContentPickerSelection) => void;
}

function questionPoolEntry(
  entry: AssessmentEntry,
): Extract<AssessmentEntry, { readonly kind: "questionPool" }> | undefined {
  return entry.kind === "questionPool" ? entry : undefined;
}

/** Renders the Assessment Question Editor for the current Assessment. */
export function AssessmentWorkspaceQuestionsView(
  args: AssessmentWorkspaceQuestionsViewArgs,
): JSX.Element {
  const {
    workspace,
    dirty,
    save,
    message,
    setMessage,
    busy,
    needsReload,
    reload,
    reviewBlueprintUpdate,
    applyBlueprintUpdate,
    blueprintReview,
    setBlueprintReview,
    title,
    setTitle,
    entries,
    bloomSortUnavailableReason,
    sortByBloomClassification,
    description,
    questionTitle,
    poolTitle,
    entryBlooms,
    move,
    remove,
    poolDetails,
    poolDetailLoadFailed,
    assessmentContentRepository,
    classificationClient,
    included,
    addDisabled,
    duplicateAllowed,
    updatePoolEntry,
    remainingQuestionCapacity,
    addQuestionsById,
    chooseAssessmentContent,
  } = args;
  const [questionIdBatch, setQuestionIdBatch] = createSignal("");
  let questionIdBatchDisclosure: HTMLDetailsElement | undefined;
  const trackedSave = trackEditorSave(save);

  async function submitQuestionIdBatch(): Promise<void> {
    const added = await addQuestionsById(questionIdBatch());
    if (added) {
      setQuestionIdBatch("");
      return;
    }
    if (questionIdBatchDisclosure !== undefined) questionIdBatchDisclosure.open = true;
  }

  return (
    <PageFrame
      // Question Sequence editor body. PageFrame owns the stack.
      contentClass="assessment-workspace-questions"
      routeSurface="assessmentWorkspace"
    >
      <UnsavedChangesGuard
        dirty={dirty}
        save={trackedSave.save}
        inFlightSave={trackedSave.inFlightSave}
      />
      <AssessmentWorkspaceIdentity />
      <Show when={message()}>
        {(value) => (
          <p class="assessment-workspace-save-message" role="status">
            {value()}
          </p>
        )}
      </Show>
      <Show when={workspace.assessment().workspace.origin.kind === "adopted"}>
        <section class="assessment-editor-panel" aria-labelledby="blueprint-update-heading">
          <h2 id="blueprint-update-heading">Blueprint update</h2>
          <p>Existing Assessment content changes only when you review and apply an update.</p>
          <Show when={dirty()}>
            <p>
              Save your unsaved Question changes below, or explicitly discard them before reviewing.
            </p>
            <button type="button" disabled={busy()} onClick={() => void reload(true)}>
              Discard local changes and reload latest Assessment
            </button>
          </Show>
          <button
            type="button"
            disabled={busy() || dirty() || needsReload()}
            onClick={() => void reviewBlueprintUpdate()}
          >
            Review Blueprint update
          </button>
          <Show when={blueprintReview()}>
            {(review) => (
              <>
                <h3>
                  Review Blueprint Revision{" "}
                  {review().sourceBlueprintCourseRevisionTuple.revisionNumber}
                </h3>
                <p>
                  Apply replaces this Assessment's title, instructions, reusable settings, and
                  ordered Questions and Question Pools, including local customizations. Dates,
                  release status, and existing Student Work are preserved. New Attempts use the
                  updated content. A proposed Blueprint Pool copy is shown below before it becomes
                  an ordinary reusable Pool reference in this Assessment.
                </p>
                <div class="assessment-workspace-grid">
                  <AssessmentBlueprintContentSummary
                    heading="Current saved Assessment"
                    poolRole="assessmentOwned"
                    content={currentBlueprintUpdateContent(review().assessment)}
                    description={description}
                    questionTitle={questionTitle}
                    poolTitle={poolTitle}
                  />
                  <Show when={review().proposed}>
                    {(content) => (
                      <AssessmentBlueprintContentSummary
                        heading="Proposed Blueprint Assessment"
                        poolRole="librarySource"
                        content={content()}
                        description={description}
                        questionTitle={questionTitle}
                        poolTitle={poolTitle}
                      />
                    )}
                  </Show>
                </div>
                <Show when={review().cannotApplyReason !== null}>
                  <p role="alert">
                    {review().cannotApplyReason === "retainedSourceMissing"
                      ? "This Assessment's retained source is no longer in the parent Blueprint Revision. This update cannot be applied."
                      : "The parent Blueprint Assessment has a different Assessment Type. This update cannot be applied."}
                  </p>
                </Show>
                <p class="assessment-editor-actions">
                  <button
                    class="primary-action"
                    type="button"
                    disabled={
                      busy() ||
                      dirty() ||
                      review().cannotApplyReason !== null ||
                      review().proposed === null
                    }
                    onClick={() => void applyBlueprintUpdate()}
                  >
                    Apply Blueprint update
                  </button>
                  <button
                    type="button"
                    disabled={busy()}
                    onClick={() => {
                      setBlueprintReview(undefined);
                      setMessage("Blueprint review cancelled. No update was applied.");
                    }}
                  >
                    Cancel
                  </button>
                </p>
              </>
            )}
          </Show>
        </section>
      </Show>
      <label class="assessment-editor-field">
        Assessment title
        <input
          value={title()}
          disabled={busy() || needsReload()}
          onInput={(event) => {
            if (needsReload()) return;
            setTitle(event.currentTarget.value);
          }}
        />
      </label>
      <AssessmentEditor
        entries={entries()}
        entryId={(entry) => entry.id}
        entryKind={(entry) =>
          entry.kind === "fixedQuestion" ? "Published Question" : "Question Pool"
        }
        disabled={busy() || needsReload()}
        onMove={move}
        removeCommand={(entry) => ({
          label: entry.availability === "available" ? "Remove" : "Retained unavailable",
          disabled: busy() || needsReload() || entry.availability !== "available",
          run: () => remove(entry.id),
        })}
        content={(entry) =>
          selectedAssessmentEntryContent({
            entry,
            questionTitle,
            poolTitle,
            description,
            bloom: entryBlooms().get(entry.id),
          })
        }
        renderBody={(entry) => (
          <Show when={questionPoolEntry(entry())}>
            {(poolEntry) => (
              <>
                <AssessmentPoolEntryEditor
                  entry={poolEntry()}
                  pool={poolDetails().get(poolEntry().id)}
                  poolUnavailable={poolDetailLoadFailed()}
                  mutationsEnabled={!needsReload() && poolEntry().availability === "available"}
                  busy={busy()}
                  onSelectionCount={(selectionCount) =>
                    updatePoolEntry(poolEntry().id, { selectionCount })
                  }
                />
                <fieldset
                  disabled={busy() || needsReload() || poolEntry().availability !== "available"}
                >
                  <legend>Pool scoring in this Assessment</legend>
                  <label class="assessment-editor-field">
                    Points per selected Question
                    <input
                      inputmode="decimal"
                      value={poolEntry().pointsPerItem}
                      aria-invalid={
                        assessmentPointValueDraft(poolEntry().pointsPerItem) === undefined
                      }
                      onInput={(event) =>
                        updatePoolEntry(poolEntry().id, {
                          pointsPerItem: event.currentTarget.value,
                        })
                      }
                    />
                  </label>
                  <label class="assessment-editor-field">
                    Scoring
                    <select
                      value={poolEntry().scoringRule}
                      onChange={(event) =>
                        updatePoolEntry(poolEntry().id, {
                          scoringRule: event.currentTarget.value as AssessmentEntry["scoringRule"],
                        })
                      }
                    >
                      <option value="normal">Normal</option>
                      <option value="fullCredit">Full credit</option>
                      <option value="extraCredit">Extra credit</option>
                      <option value="excluded">Excluded</option>
                    </select>
                  </label>
                </fieldset>
              </>
            )}
          </Show>
        )}
        contentsTools={
          <div class="assessment-editor-contents-tools">
            <button
              type="button"
              disabled={
                busy() ||
                needsReload() ||
                entries().length < 2 ||
                bloomSortUnavailableReason() !== undefined
              }
              onClick={sortByBloomClassification}
            >
              Reorder Entries by Bloom Classification
            </button>
            <Show when={bloomSortUnavailableReason()}>
              {(reason) => <p class="assessment-editor-note">{reason()}</p>}
            </Show>
          </div>
        }
        repository={assessmentContentRepository}
        classificationClient={classificationClient}
        included={included}
        addDisabled={addDisabled}
        duplicateAllowed={duplicateAllowed}
        onAdd={chooseAssessmentContent}
        libraryTools={
          <>
            <Show when={remainingQuestionCapacity() === 0}>
              <p role="status">Assessment limit reached. Add controls are disabled.</p>
            </Show>
            <details ref={(element) => (questionIdBatchDisclosure = element)}>
              <summary>Add by Question ID</summary>
              <label class="assessment-editor-field">
                Question IDs (spaces, commas, or new lines)
                <textarea
                  rows="3"
                  value={questionIdBatch()}
                  disabled={busy() || needsReload() || remainingQuestionCapacity() === 0}
                  onInput={(event) => setQuestionIdBatch(event.currentTarget.value)}
                />
              </label>
              <button
                type="button"
                disabled={busy() || needsReload() || remainingQuestionCapacity() === 0}
                onClick={() => void submitQuestionIdBatch()}
              >
                Add Question IDs
              </button>
            </details>
          </>
        }
      />
      <p class="assessment-editor-actions">
        <button
          class="primary-action"
          type="button"
          disabled={busy() || needsReload()}
          onClick={() => void trackedSave.save()}
        >
          {busy() ? "Saving Questions..." : "Save Questions and order"}
        </button>
        <Show when={needsReload()}>
          <button type="button" onClick={() => void reload(dirty())}>
            {dirty()
              ? "Discard local changes and reload latest Assessment"
              : "Reload latest Assessment"}
          </button>
        </Show>
      </p>
      <p class="assessment-workspace-next-actions">
        <A
          class="quiet-link"
          href={assessmentWorkspacePath(
            workspace.courseInstanceId,
            workspace.assessmentId,
            "policies",
          )}
        >
          Review Assessment Properties
        </A>
      </p>
    </PageFrame>
  );
}
