// Assessment Question Editor view for the current Assessment workspace.

import { A } from "@solidjs/router";
import { For, Show, type Accessor, type JSX, type Setter } from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { AssessmentQuestionPoolForkView } from "../../../generated/api/AssessmentQuestionPoolForkView";
import type { QuestionPoolLibrarySummary } from "../../../generated/api/QuestionPoolLibrarySummary";
import type { QuestionRevisionTuple } from "../../../generated/api/QuestionRevisionTuple";
import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type {
  AssessmentQuestionPickerEntry,
  AssessmentBlueprintUpdateReview,
} from "../../api/assessment_release";
import { AssessmentPoolEntryEditor } from "./assessment_pool_entry_editor";
import { SelectedAssessmentEntryIdentity } from "./assessment_workspace_selected_entry";
import {
  AssessmentWorkspaceIdentity,
  type AssessmentWorkspaceContextValue,
} from "./assessment_workspace_live_page";
import { assessmentWorkspacePath } from "./assessment_workspace_paths";
import { nextQuestionEditDirty } from "./assessment_workspace_questions_model";
import { UnsavedChangesGuard } from "./unsaved_changes_guard";
import {
  AssessmentBlueprintContentSummary,
  currentBlueprintUpdateContent,
} from "./assessment_blueprint_update_review";

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
  readonly description: (
    questionRevisionTuple: AssessmentQuestionPickerEntry["questionRevisionTuple"],
  ) => string;
  readonly entryBlooms: Accessor<ReadonlyMap<AssessmentEntryId, BloomClassificationView>>;
  readonly move: (index: number, offset: -1 | 1) => void;
  readonly remove: (index: number) => void;
  readonly poolForks: Accessor<ReadonlyMap<AssessmentEntryId, AssessmentQuestionPoolForkView>>;
  readonly poolForkLoadFailed: Accessor<boolean>;
  readonly available: Accessor<ReadonlyArray<AssessmentQuestionPickerEntry>>;
  readonly updatePoolSelectionCount: (
    entry: Extract<AssessmentEntry, { readonly kind: "questionPool" }>,
    selectionCount: number,
  ) => Promise<void>;
  readonly replacePoolMembers: (
    entry: Extract<AssessmentEntry, { readonly kind: "questionPool" }>,
    members: ReadonlyArray<QuestionRevisionTuple>,
  ) => Promise<void>;
  readonly availableToAdd: Accessor<ReadonlyArray<AssessmentQuestionPickerEntry>>;
  readonly remainingQuestionCapacity: Accessor<number>;
  readonly questionIdsToAdd: Accessor<string>;
  readonly setQuestionIdsToAdd: Setter<string>;
  readonly addQuestionsById: () => void;
  readonly add: (candidate: AssessmentQuestionPickerEntry) => void;
  readonly availablePools: Accessor<ReadonlyArray<QuestionPoolLibrarySummary>>;
  readonly poolToImport: Accessor<string>;
  readonly setPoolToImport: Setter<string>;
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

function questionRevisionInspectionPath(questionRevisionTuple: QuestionRevisionTuple): string {
  // ASVS 1.2.2: encode the displayed Question identity before placing it in a route path.
  return `/library/${encodeURIComponent(questionRevisionTuple.questionId)}?revision=${questionRevisionTuple.revisionNumber}`;
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
    available,
    updatePoolSelectionCount,
    replacePoolMembers,
    availableToAdd,
    remainingQuestionCapacity,
    questionIdsToAdd,
    setQuestionIdsToAdd,
    addQuestionsById,
    add,
    availablePools,
    poolToImport,
    setPoolToImport,
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

  return (
    <section class="assessment-workspace-questions" aria-labelledby="assessment-questions-heading">
      <UnsavedChangesGuard dirty={dirty} save={save} />
      <header class="assessment-workspace-header">
        <p class="eyebrow">Assessment workspace</p>
        <h1 id="assessment-questions-heading">Assessment Question Editor</h1>
        <AssessmentWorkspaceIdentity />
        <p class="page-lede">
          Every Entry retains its exact Question Revision and stable identity for future Attempts.
        </p>
      </header>
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
                <h3>Review Blueprint Revision {review().sourceRevisionNumber}</h3>
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
        <Show when={entries().length > 0} fallback={<p>No Entries are selected.</p>}>
          <ol class="assessment-editor-list">
            <For each={entries()}>
              {(entry, index) => (
                <li
                  classList={{
                    "assessment-editor-row": true,
                    "assessment-editor-pool": entry.kind === "questionPool",
                  }}
                  data-assessment-entry={entry.id}
                >
                  <SelectedAssessmentEntryIdentity
                    entry={entry}
                    entryNumber={index() + 1}
                    description={description}
                    bloom={entryBlooms().get(entry.id)}
                  />
                  <div
                    class="assessment-editor-row-actions"
                    role="group"
                    aria-label={`Entry ${index() + 1} actions`}
                  >
                    <button
                      class="quiet-action"
                      type="button"
                      disabled={busy() || needsReload() || index() === 0}
                      onClick={() => move(index(), -1)}
                    >
                      Move earlier
                    </button>
                    <button
                      class="quiet-action"
                      type="button"
                      disabled={busy() || needsReload() || index() === entries().length - 1}
                      onClick={() => move(index(), 1)}
                    >
                      Move later
                    </button>
                    <button
                      class="quiet-action"
                      type="button"
                      disabled={busy() || needsReload() || entry.availability !== "available"}
                      aria-label={
                        entry.availability === "available"
                          ? `Remove Assessment Entry ${index() + 1}`
                          : `Retained unavailable Assessment Entry ${index() + 1} cannot be removed`
                      }
                      onClick={() => remove(index())}
                    >
                      {entry.availability === "available" ? "Remove" : "Retained unavailable"}
                    </button>
                  </div>
                  <Show when={questionPoolEntry(entry)}>
                    {(poolEntry) => (
                      <AssessmentPoolEntryEditor
                        entry={poolEntry()}
                        fork={poolForks().get(poolEntry().id)}
                        exactMembersUnavailable={poolForkLoadFailed()}
                        availableQuestions={available()}
                        mutationsEnabled={
                          !dirty() && !needsReload() && poolEntry().availability === "available"
                        }
                        busy={busy()}
                        onSelectionCount={(selectionCount) =>
                          void updatePoolSelectionCount(poolEntry(), selectionCount)
                        }
                        onReplaceMembers={(members) =>
                          void replacePoolMembers(poolEntry(), members)
                        }
                      />
                    )}
                  </Show>
                </li>
              )}
            </For>
          </ol>
        </Show>
      </section>
      <section class="assessment-editor-panel" aria-labelledby="available-questions-heading">
        <h2 id="available-questions-heading">Available published Questions</h2>
        <p class="assessment-editor-note">
          Enter Question IDs to add them together in the order entered. Each keeps the exact
          Published Revision shown below; inspection is optional. Save Questions when ready.
        </p>
        <p>
          <A href="/library">Search Question Library</A> or{" "}
          <A href="/library/browse">Browse Question Library</A>.
        </p>
        <Show
          when={availableToAdd().length > 0}
          fallback={<p>No additional Available Questions are ready to add.</p>}
        >
          <p role="status">
            Room for {remainingQuestionCapacity()} more Questions, counting each Pool's selected
            Questions.
          </p>
          <label class="assessment-editor-field">
            Question IDs to add
            <textarea
              rows={3}
              value={questionIdsToAdd()}
              disabled={busy() || needsReload() || remainingQuestionCapacity() === 0}
              aria-describedby="bulk-question-id-help"
              onInput={(event) => setQuestionIdsToAdd(event.currentTarget.value)}
            />
          </label>
          <p id="bulk-question-id-help" class="assessment-editor-note">
            Separate IDs with commas, spaces, or new lines. Every ID must match a Question below;
            the whole batch is checked before any Questions are added.
          </p>
          <div class="assessment-editor-actions">
            <button
              type="button"
              class="primary-action"
              disabled={
                busy() ||
                needsReload() ||
                questionIdsToAdd().trim().length === 0 ||
                remainingQuestionCapacity() === 0
              }
              onClick={addQuestionsById}
            >
              Add Questions by ID
            </button>
          </div>
          <ul>
            <For each={availableToAdd()}>
              {(candidate) => (
                <li>
                  <strong>{candidate.questionRevisionTuple.questionId}</strong> * Revision{" "}
                  {candidate.questionRevisionTuple.revisionNumber}: {candidate.description}{" "}
                  <A href={questionRevisionInspectionPath(candidate.questionRevisionTuple)}>
                    Inspect
                  </A>{" "}
                  <Show when={candidate.bloom}>
                    {(bloom) => (
                      <span>
                        Bloom Cognitive Process: {bloom().cognitiveProcess}; Bloom Knowledge
                        Dimension: {bloom().knowledgeDimension}
                      </span>
                    )}
                  </Show>{" "}
                  <button
                    type="button"
                    disabled={busy() || needsReload() || remainingQuestionCapacity() === 0}
                    onClick={() => add(candidate)}
                  >
                    Add Question
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </section>
      <section class="assessment-editor-panel" aria-labelledby="available-pools-heading">
        <h2 id="available-pools-heading">Import a reusable Question Pool</h2>
        <p class="assessment-editor-note">
          Importing creates an Assessment-owned fork. It does not change the reusable Question Pool.
        </p>
        <fieldset disabled={busy() || dirty() || needsReload() || availablePools().length === 0}>
          <label class="assessment-editor-field">
            Published Question Pool
            <select
              value={poolToImport()}
              onChange={(event) => setPoolToImport(event.currentTarget.value)}
            >
              <option value="">Choose a Question Pool</option>
              <For each={availablePools()}>
                {(pool) => (
                  <option value={pool.questionPoolId}>
                    {pool.metadata.title} - {pool.questionPoolId} Edit {pool.questionPoolEditNumber}{" "}
                    ({pool.memberCount} Questions)
                    <Show when={pool.bloom}>
                      {(bloom) => ` - ${bloom().cognitiveProcess} / ${bloom().knowledgeDimension}`}
                    </Show>
                  </option>
                )}
              </For>
            </select>
          </label>
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
          <button type="button" disabled={poolToImport() === ""} onClick={() => void importPool()}>
            Import Question Pool
          </button>
        </fieldset>
        <Show when={availablePools().length === 0}>
          <p>No reusable published Question Pools are available to import.</p>
        </Show>
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
    </section>
  );
}
