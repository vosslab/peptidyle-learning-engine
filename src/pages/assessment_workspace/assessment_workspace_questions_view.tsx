// Assessment Question Editor view for the current Assessment workspace.

import { A } from "@solidjs/router";
import { Show, createSignal, type Accessor, type JSX, type Setter } from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { AssessmentQuestionPoolForkView } from "../../../generated/api/AssessmentQuestionPoolForkView";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type { AssessmentBlueprintUpdateReview } from "../../api/assessment_release";
import type { QuestionPoolLibraryClient } from "../../api/question_pool_library";
import { PageFrame } from "../../components/page_frame";
import { RecordSequence } from "../../components/record_list/record_sequence";
import { AssessmentPoolEntryEditor } from "./assessment_pool_entry_editor";
import { selectedAssessmentEntryContent } from "./assessment_workspace_selected_entry";
import {
  AssessmentWorkspaceIdentity,
  type AssessmentWorkspaceContextValue,
} from "./assessment_workspace_live_page";
import { assessmentWorkspacePath } from "./assessment_workspace_paths";
import { nextQuestionEditDirty } from "./assessment_workspace_questions_model";
import {
  QuestionPicker,
  type QuestionPickerSelection,
  type QuestionPickerSource,
  type QuestionPickerSourceRepository,
} from "../../features/question_picker";
import {
  QuestionPoolPicker,
  type QuestionPoolPickerSelection,
} from "../../features/question_pool_picker/question_pool_picker";
import { UnsavedChangesGuard } from "./unsaved_changes_guard";
import {
  AssessmentBlueprintContentSummary,
  currentBlueprintUpdateContent,
} from "./assessment_blueprint_update_review";
import "./assessment_questions_record_list.css";

export interface AssessmentWorkspaceQuestionsViewArgs {
  readonly workspace: AssessmentWorkspaceContextValue;
  readonly dirty: Accessor<boolean>;
  readonly setDirty: Setter<boolean>;
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
  readonly entryBlooms: Accessor<ReadonlyMap<AssessmentEntryId, BloomClassificationView>>;
  readonly move: (index: number, offset: -1 | 1) => void;
  readonly remove: (index: number) => void;
  readonly poolForks: Accessor<ReadonlyMap<AssessmentEntryId, AssessmentQuestionPoolForkView>>;
  readonly poolForkLoadFailed: Accessor<boolean>;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly questionPoolClient: QuestionPoolLibraryClient;
  readonly updatePoolSelectionCount: (
    entry: Extract<AssessmentEntry, { readonly kind: "questionPool" }>,
    selectionCount: number,
  ) => Promise<void>;
  readonly replacePoolMembers: (
    entry: Extract<AssessmentEntry, { readonly kind: "questionPool" }>,
    members: ReadonlyArray<PublishedQuestionRevisionTuple>,
  ) => Promise<void>;
  readonly remainingQuestionCapacity: Accessor<number>;
  readonly addPublishedQuestions: (selection: QuestionPickerSelection) => void;
  readonly poolImport: Accessor<QuestionPoolPickerSelection | undefined>;
  readonly choosePool: (selection: QuestionPoolPickerSelection) => void;
  readonly poolSelectionCount: Accessor<string>;
  readonly setPoolSelectionCount: Setter<string>;
  readonly poolPointsPerItem: Accessor<string>;
  readonly setPoolPointsPerItem: Setter<string>;
  readonly poolSelectedQuestionOrder: Accessor<"questionPoolOrder" | "randomOrder">;
  readonly setPoolSelectedQuestionOrder: Setter<"questionPoolOrder" | "randomOrder">;
  readonly poolScoringRule: Accessor<"normal" | "fullCredit" | "extraCredit" | "excluded">;
  readonly setPoolScoringRule: Setter<"normal" | "fullCredit" | "extraCredit" | "excluded">;
  readonly importPool: () => Promise<void>;
}

type AssessmentQuestionRecord = {
  readonly entry: AssessmentEntry;
  readonly index: number;
};

function questionPoolEntry(
  entry: AssessmentEntry,
): Extract<AssessmentEntry, { readonly kind: "questionPool" }> | undefined {
  return entry.kind === "questionPool" ? entry : undefined;
}

function assessmentQuestionRecords(
  entries: ReadonlyArray<AssessmentEntry>,
): ReadonlyArray<AssessmentQuestionRecord> {
  return entries.map((entry, index) => ({ entry, index }));
}

function moveAssessmentEntryToDestination(
  moveEntry: (index: number, offset: -1 | 1) => void,
  sourceIndex: number,
  destinationIndex: number,
): void {
  if (sourceIndex === destinationIndex) return;
  const direction = destinationIndex > sourceIndex ? 1 : -1;
  for (
    let currentIndex = sourceIndex;
    currentIndex !== destinationIndex;
    currentIndex += direction
  ) {
    moveEntry(currentIndex, direction);
  }
}

/** Renders the Assessment Question Editor for the current Assessment. */
export function AssessmentWorkspaceQuestionsView(
  args: AssessmentWorkspaceQuestionsViewArgs,
): JSX.Element {
  const {
    workspace,
    dirty,
    setDirty,
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
    entryBlooms,
    move,
    remove,
    poolForks,
    poolForkLoadFailed,
    pickerRepository,
    pickerSources,
    questionPoolClient,
    updatePoolSelectionCount,
    replacePoolMembers,
    remainingQuestionCapacity,
    addPublishedQuestions,
    poolImport,
    choosePool,
    poolSelectionCount,
    setPoolSelectionCount,
    poolPointsPerItem,
    setPoolPointsPerItem,
    poolSelectedQuestionOrder,
    setPoolSelectedQuestionOrder,
    poolScoringRule,
    setPoolScoringRule,
    importPool,
  } = args;
  const [questionPickerOpen, setQuestionPickerOpen] = createSignal(false);
  const [poolPickerOpen, setPoolPickerOpen] = createSignal(false);
  let questionPickerTrigger: HTMLButtonElement | undefined;
  let poolPickerTrigger: HTMLButtonElement | undefined;

  return (
    <PageFrame
      // Question Sequence editor body. PageFrame owns the stack.
      contentClass="assessment-workspace-questions"
      routeSurface="assessmentWorkspace"
      headingId="assessment-questions-heading"
      eyebrow="Assessment workspace"
      title="Assessment Question Editor"
      lede="Every Entry retains its exact Question Revision and stable identity for future Attempts."
    >
      <UnsavedChangesGuard dirty={dirty} save={save} />
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
                  Review Blueprint Revision {review().sourceBlueprintRevisionTuple.revisionNumber}
                </h3>
                <p>
                  Apply replaces this Assessment's title, instructions, reusable settings, and
                  ordered Questions and Question Pools, including local customizations. Dates,
                  release status, and existing Student Work are preserved. New Attempts use the
                  updated content. For each proposed Library source Question Pool, Apply creates a
                  replacement Assessment-owned fork from the current source Pool shown below.
                </p>
                <div class="assessment-workspace-grid">
                  <AssessmentBlueprintContentSummary
                    heading="Current saved Assessment"
                    poolRole="assessmentOwned"
                    content={currentBlueprintUpdateContent(review().assessment)}
                    description={description}
                  />
                  <Show when={review().proposed}>
                    {(content) => (
                      <AssessmentBlueprintContentSummary
                        heading="Proposed Blueprint Assessment"
                        poolRole="librarySource"
                        content={content()}
                        description={description}
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
            setDirty((current) => nextQuestionEditDirty(current, "title"));
          }}
        />
      </label>
      <section class="assessment-editor-panel" aria-labelledby="selected-questions-heading">
        <h2 id="selected-questions-heading">Ordered Assessment Entries</h2>
        <p class="assessment-editor-actions">
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
            Sort by Bloom Classification
          </button>
        </p>
        <Show when={bloomSortUnavailableReason()}>
          {(reason) => <p class="assessment-editor-note">{reason()}</p>}
        </Show>
        <RecordSequence
          rows={assessmentQuestionRecords(entries())}
          content={(record) =>
            selectedAssessmentEntryContent({
              entry: record.entry,
              entryNumber: record.index + 1,
              description,
              bloom: entryBlooms().get(record.entry.id),
              removeDisabled: busy() || needsReload(),
              remove: () => remove(record.index),
            })
          }
          renderBody={(record) => (
            <Show when={questionPoolEntry(record().entry)}>
              {(poolEntry) => (
                <AssessmentPoolEntryEditor
                  entry={poolEntry()}
                  fork={poolForks().get(poolEntry().id)}
                  exactMembersUnavailable={poolForkLoadFailed()}
                  pickerRepository={pickerRepository}
                  pickerSources={pickerSources}
                  mutationsEnabled={
                    !dirty() && !needsReload() && poolEntry().availability === "available"
                  }
                  busy={busy()}
                  onSelectionCount={(selectionCount) =>
                    void updatePoolSelectionCount(poolEntry(), selectionCount)
                  }
                  onReplaceMembers={(members) => replacePoolMembers(poolEntry(), members)}
                />
              )}
            </Show>
          )}
          reorder={{
            onMove: (sourceIndex, destinationIndex) =>
              moveAssessmentEntryToDestination(move, sourceIndex, destinationIndex),
            recordLabel: (record) => `Entry ${record.index + 1}`,
            isDisabled: () => busy() || needsReload(),
          }}
          recordId={(record) => record.entry.id}
          state={{ kind: "ready" }}
          ariaLabel="Ordered Assessment Entries"
          emptyState={{ title: "No Entries are selected." }}
        />
      </section>
      <section class="assessment-editor-panel" aria-labelledby="available-questions-heading">
        <h2 id="available-questions-heading">Available published Questions</h2>
        <p class="assessment-editor-note">
          Search the Question Library and add a later page without loading the whole catalog. Each
          added Question keeps the exact Published Revision you select. Save Questions when ready.
        </p>
        <p role="status">
          Room for {remainingQuestionCapacity()} more Questions, counting each Pool's selected
          Questions.
        </p>
        <div class="assessment-editor-actions">
          <button
            type="button"
            class="primary-action"
            ref={(element) => (questionPickerTrigger = element)}
            disabled={busy() || needsReload() || remainingQuestionCapacity() === 0}
            onClick={() => setQuestionPickerOpen(true)}
          >
            Choose published Questions
          </button>
        </div>
        <Show when={questionPickerOpen() && remainingQuestionCapacity() > 0}>
          <QuestionPicker
            repository={pickerRepository}
            sources={pickerSources}
            mode="many"
            maximumSelection={remainingQuestionCapacity()}
            trigger={questionPickerTrigger}
            title="Choose published Questions"
            confirmLabel="Add selected Questions"
            instructions="Selected Questions are added to this Assessment in tray order and keep their exact Published Revisions. Cancel leaves the current Entries unchanged."
            onConfirm={(selection) => {
              setQuestionPickerOpen(false);
              addPublishedQuestions(selection);
            }}
            onCancel={() => setQuestionPickerOpen(false)}
          />
        </Show>
      </section>
      <section class="assessment-editor-panel" aria-labelledby="available-pools-heading">
        <h2 id="available-pools-heading">Import a reusable Question Pool</h2>
        <p class="assessment-editor-note">
          Importing creates an Assessment-owned fork. It does not change the reusable Question Pool.
          Later result pages stay available in the picker.
        </p>
        <div class="assessment-editor-actions">
          <button
            type="button"
            ref={(element) => (poolPickerTrigger = element)}
            disabled={busy() || dirty() || needsReload()}
            onClick={() => setPoolPickerOpen(true)}
          >
            Choose Question Pool
          </button>
        </div>
        <Show when={poolImport()}>
          {(selected) => (
            <p>
              Question Pool {selected().questionPoolId}, Edit {selected().questionPoolEditNumber},{" "}
              {selected().memberCount} published Questions.
            </p>
          )}
        </Show>
        <Show when={poolPickerOpen()}>
          <QuestionPoolPicker
            client={questionPoolClient}
            trigger={poolPickerTrigger}
            onConfirm={(selection) => {
              setPoolPickerOpen(false);
              choosePool(selection);
            }}
            onCancel={() => setPoolPickerOpen(false)}
          />
        </Show>
        <fieldset disabled={busy() || dirty() || needsReload() || poolImport() === undefined}>
          <label class="assessment-editor-field">
            Questions selected for each Attempt
            <input
              type="number"
              min="1"
              value={poolSelectionCount()}
              onInput={(event) => setPoolSelectionCount(event.currentTarget.value)}
            />
          </label>
          <label class="assessment-editor-field">
            Points per selected Question
            <input
              value={poolPointsPerItem()}
              onInput={(event) => setPoolPointsPerItem(event.currentTarget.value)}
            />
          </label>
          <label class="assessment-editor-field">
            Selected Question order
            <select
              value={poolSelectedQuestionOrder()}
              onChange={(event) =>
                setPoolSelectedQuestionOrder(
                  event.currentTarget.value as "questionPoolOrder" | "randomOrder",
                )
              }
            >
              <option value="randomOrder">Random order</option>
              <option value="questionPoolOrder">Question Pool order</option>
            </select>
          </label>
          <label class="assessment-editor-field">
            Scoring
            <select
              value={poolScoringRule()}
              onChange={(event) =>
                setPoolScoringRule(
                  event.currentTarget.value as "normal" | "fullCredit" | "extraCredit" | "excluded",
                )
              }
            >
              <option value="normal">Normal</option>
              <option value="fullCredit">Full credit</option>
              <option value="extraCredit">Extra credit</option>
              <option value="excluded">Excluded</option>
            </select>
          </label>
          <button
            type="button"
            disabled={poolImport() === undefined}
            onClick={() => void importPool()}
          >
            Import Question Pool
          </button>
        </fieldset>
      </section>
      <p class="assessment-editor-actions">
        <button
          class="primary-action"
          type="button"
          disabled={busy() || needsReload()}
          onClick={() => void save()}
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
