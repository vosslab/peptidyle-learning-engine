import { A } from "@solidjs/router";
import { For, Show, createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { AssessmentQuestionPoolForkView } from "../../../generated/api/AssessmentQuestionPoolForkView";
import type { QuestionPoolLibrarySummary } from "../../../generated/api/QuestionPoolLibrarySummary";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type {
  AssessmentQuestionPickerEntry,
  AssessmentBlueprintUpdateReview,
} from "../../api/assessment_release";
import { ApiRequestError } from "../../api/http_client/error";
import { useApplicationApi } from "../../api/application_api";
import { LiveAssessmentWorkspaceConflictError } from "../../api/http_client/assessment_release";
import { AssessmentPoolForkConflictError } from "../../api/http_client/assessment_pool_fork";
import { AssessmentPoolEntryEditor } from "./assessment_pool_entry_editor";
import { SelectedAssessmentEntryIdentity } from "./assessment_workspace_selected_entry";
import { normalizeHumanEnteredQuestionId } from "../../question_id";
import {
  AssessmentWorkspaceIdentity,
  useAssessmentWorkspace,
} from "./assessment_workspace_live_page";
import { assessmentWorkspacePath } from "./assessment_workspace_paths";
import {
  appendAvailableFixedQuestion,
  deliveredAssessmentQuestionCount,
  moveAssessmentEntry,
  questionSaveInput,
  questionRevisionKey,
  removeAssessmentEntry,
  sortAssessmentEntriesByBloom,
} from "./assessment_workspace_questions_model";
import { UnsavedChangesGuard } from "./unsaved_changes_guard";
import {
  AssessmentBlueprintContentSummary,
  currentBlueprintUpdateContent,
} from "./assessment_blueprint_update_review";

const MAX_ASSIGNMENT_ENTRIES = 1024;
const MAX_ASSESSMENT_QUESTIONS = 250;

export type QuestionEditDirtyEvent =
  "title" | "move" | "sort" | "remove" | "add" | "saveSucceeded" | "saveFailed";

/** Keeps the leave guard active until the current structural edit was persisted successfully. */
export function nextQuestionEditDirty(current: boolean, event: QuestionEditDirtyEvent): boolean {
  if (event === "saveSucceeded") return false;
  if (event === "saveFailed") return current;
  return true;
}

function entryId(): AssessmentEntryId {
  return crypto.randomUUID();
}

function questionRevisionInspectionPath(reference: QuestionRevisionReference): string {
  // ASVS 1.2.2: encode the displayed Question identity before placing it in a route path.
  return `/library/${encodeURIComponent(reference.questionId)}?revision=${reference.revisionNumber}`;
}

function questionPoolEntry(
  entry: AssessmentEntry,
): Extract<AssessmentEntry, { readonly kind: "questionPool" }> | undefined {
  return entry.kind === "questionPool" ? entry : undefined;
}

/** Edits the complete normalized content owned by the current Assessment. */
export function AssessmentWorkspaceQuestionsPage(): JSX.Element {
  const workspace = useAssessmentWorkspace();
  const applicationApi = useApplicationApi();
  const initial = workspace.assessment().workspace;
  const [entries, setEntries] = createSignal<ReadonlyArray<AssessmentEntry>>(initial.entries);
  const [title, setTitle] = createSignal(initial.title);
  const [available, setAvailable] = createSignal<ReadonlyArray<AssessmentQuestionPickerEntry>>([]);
  const [questionIdsToAdd, setQuestionIdsToAdd] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [needsReload, setNeedsReload] = createSignal(false);
  const [dirty, setDirty] = createSignal(false);
  const [blueprintReview, setBlueprintReview] = createSignal<AssessmentBlueprintUpdateReview>();
  const [poolForks, setPoolForks] = createSignal<
    ReadonlyMap<AssessmentEntryId, AssessmentQuestionPoolForkView>
  >(new Map());
  const [poolForkLoadFailed, setPoolForkLoadFailed] = createSignal(false);
  const [availablePools, setAvailablePools] = createSignal<
    ReadonlyArray<QuestionPoolLibrarySummary>
  >([]);
  const [poolToImport, setPoolToImport] = createSignal("");
  const [poolSelectionCount, setPoolSelectionCount] = createSignal("1");
  const [poolPointsPerItem, setPoolPointsPerItem] = createSignal("1");
  const [poolSelectedQuestionOrder, setPoolSelectedQuestionOrder] = createSignal<
    "questionPoolOrder" | "randomOrder"
  >("randomOrder");
  const [poolScoringRule, setPoolScoringRule] = createSignal<
    "normal" | "fullCredit" | "extraCredit" | "excluded"
  >("normal");
  let poolForkLoadRequest = 0;

  const descriptions = createMemo(() => {
    const known = new Map<string, string>();
    for (const question of workspace.assessment().workspace.questions)
      known.set(questionRevisionKey(question.reference), question.description);
    for (const question of available())
      known.set(questionRevisionKey(question.reference), question.description);
    return known;
  });
  const fixedBlooms = createMemo(() => {
    const known = new Map<string, BloomClassificationView>();
    for (const question of workspace.assessment().workspace.questions) {
      if (question.bloom !== null)
        known.set(questionRevisionKey(question.reference), question.bloom);
    }
    for (const question of available()) {
      if (question.bloom !== null)
        known.set(questionRevisionKey(question.reference), question.bloom);
    }
    return known;
  });
  const entryBlooms = createMemo(() => {
    const known = new Map<AssessmentEntryId, BloomClassificationView>();
    for (const entry of entries()) {
      const bloom =
        entry.kind === "fixedQuestion"
          ? fixedBlooms().get(questionRevisionKey(entry.reference))
          : poolForks().get(entry.id)?.bloom;
      if (bloom !== undefined && bloom !== null) known.set(entry.id, bloom);
    }
    return known;
  });
  const bloomSortUnavailableReason = createMemo(() => {
    const missingPool = entries().some(
      (entry) => entry.kind === "questionPool" && !poolForks().has(entry.id),
    );
    if (missingPool && poolForkLoadFailed()) {
      return "Exact Bloom Classification for a Question Pool could not load. Reload the latest Assessment.";
    }
    if (missingPool)
      return "Loading exact Bloom Classification for Assessment-owned Question Pools.";
    if (entryBlooms().size !== entries().length) {
      return "Exact Bloom Classification for a fixed Question could not load. Reload the latest Assessment.";
    }
    return undefined;
  });
  const availableToAdd = createMemo(() =>
    available().filter(
      (candidate) =>
        !entries().some(
          (entry) =>
            entry.kind === "fixedQuestion" &&
            questionRevisionKey(entry.reference) === questionRevisionKey(candidate.reference),
        ),
    ),
  );
  const remainingQuestionCapacity = createMemo(() => {
    const questionCount = deliveredAssessmentQuestionCount(entries());
    return Math.max(
      0,
      Math.min(MAX_ASSIGNMENT_ENTRIES - entries().length, MAX_ASSESSMENT_QUESTIONS - questionCount),
    );
  });

  onMount(() => {
    void loadAvailable();
    void loadInitialPoolForks();
  });

  async function loadAvailable(): Promise<void> {
    try {
      const [questions, pools] = await Promise.all([
        applicationApi.client.listLiveAssessmentQuestionPicker(workspace.courseReference),
        applicationApi.client.listQuestionPools(),
      ]);
      setAvailable(questions);
      setAvailablePools(pools.items);
    } catch {
      setMessage(
        "Available published Questions or Question Pools could not load. Existing Assessment Entries remain here.",
      );
    }
  }

  async function loadInitialPoolForks(): Promise<void> {
    if ((await loadPoolForks(entries())) === "failed") {
      setMessage(
        "Exact Question Pool members could not load. Reload the Assessment and try again.",
      );
    }
  }

  async function loadPoolForks(
    currentEntries: ReadonlyArray<AssessmentEntry>,
  ): Promise<"loaded" | "failed" | "superseded"> {
    const request = ++poolForkLoadRequest;
    setPoolForkLoadFailed(false);
    const poolEntries = currentEntries.filter(
      (entry): entry is Extract<AssessmentEntry, { readonly kind: "questionPool" }> =>
        entry.kind === "questionPool",
    );
    try {
      const loaded = await Promise.all(
        poolEntries.map(
          async (entry) =>
            [
              entry.id,
              await applicationApi.client.getAssessmentQuestionPoolFork(
                workspace.courseReference,
                workspace.assessmentReference,
                entry.id,
              ),
            ] as const,
        ),
      );
      if (request !== poolForkLoadRequest) return "superseded";
      setPoolForks(new Map(loaded));
      setPoolForkLoadFailed(false);
      return "loaded";
    } catch {
      if (request !== poolForkLoadRequest) return "superseded";
      setPoolForkLoadFailed(true);
      return "failed";
    }
  }

  function description(reference: AssessmentQuestionPickerEntry["reference"]): string {
    return descriptions().get(questionRevisionKey(reference)) ?? "Published Question";
  }

  function move(index: number, offset: -1 | 1): void {
    if (needsReload()) {
      setMessage("Reload the latest Assessment before changing its entry order.");
      return;
    }
    setEntries((current) => moveAssessmentEntry(current, index, offset));
    setDirty((current) => nextQuestionEditDirty(current, "move"));
    setMessage("Entry order changed. Save Questions when ready.");
  }

  function sortByBloomClassification(): void {
    if (busy() || needsReload()) return;
    const current = entries();
    const sorted = sortAssessmentEntriesByBloom(current, entryBlooms());
    if (sorted === undefined) {
      setMessage(
        bloomSortUnavailableReason() ??
          "Exact Bloom Classification could not load. Reload the latest Assessment.",
      );
      return;
    }
    if (sorted === current) {
      setMessage("Entries already follow Bloom Classification guide order. No changes were made.");
      return;
    }
    setEntries(sorted);
    setDirty((dirtyState) => nextQuestionEditDirty(dirtyState, "sort"));
    setMessage("Entries sorted by Bloom Classification. Save Questions when ready.");
  }

  function remove(index: number): void {
    if (needsReload()) {
      setMessage("Reload the latest Assessment before removing an entry.");
      return;
    }
    if (entries()[index]?.availability !== "available") {
      setMessage("Retained unavailable Entries remain read-only evidence and cannot be removed.");
      return;
    }
    setEntries((current) => removeAssessmentEntry(current, index));
    setDirty((current) => nextQuestionEditDirty(current, "remove"));
    setMessage("Entry removed. Save Questions when ready.");
  }

  function appendAvailableQuestions(
    candidates: ReadonlyArray<AssessmentQuestionPickerEntry>,
  ): void {
    setEntries((current) => {
      let next = current;
      for (const candidate of candidates)
        next = appendAvailableFixedQuestion(next, candidate, entryId());
      return next;
    });
    setDirty((current) => nextQuestionEditDirty(current, "add"));
  }

  function add(candidate: AssessmentQuestionPickerEntry): void {
    if (busy()) return;
    if (needsReload()) {
      setMessage("Reload the latest Assessment before adding an entry.");
      return;
    }
    if (remainingQuestionCapacity() === 0) {
      setMessage(
        "An Assessment may deliver at most 250 Questions, counting each Pool's selected Questions.",
      );
      return;
    }
    appendAvailableQuestions([candidate]);
    setMessage(
      "Available published Question added with its exact revision pin. Save Questions when ready.",
    );
  }

  function addQuestionsById(): void {
    if (busy()) return;
    if (needsReload()) {
      setMessage("Reload the latest Assessment before adding an entry.");
      return;
    }
    const rawIds = questionIdsToAdd()
      .trim()
      .split(/[\s,]+/u)
      .filter(Boolean);
    const normalizedIds = rawIds.map((id) => normalizeHumanEnteredQuestionId(id));
    const invalidIds = rawIds.filter((_, index) => normalizedIds[index] === null);
    if (invalidIds.length > 0) {
      setMessage(
        `Invalid Question IDs: ${invalidIds.join(", ")}. Use the IDs shown below; no Questions were added.`,
      );
      return;
    }
    const ids = normalizedIds.map((id) => id!);
    if (ids.length === 0) return;
    setQuestionIdsToAdd(ids.join("\n"));
    if (new Set(ids).size !== ids.length) {
      setMessage(
        "Question IDs must appear only once. Remove duplicate IDs and try again; no Questions were added.",
      );
      return;
    }
    const candidates: AssessmentQuestionPickerEntry[] = [];
    const unresolved: string[] = [];
    // ASVS 2.2.1: resolve IDs only against the available Published summaries, never invent pins.
    for (const id of ids) {
      const matches = availableToAdd().filter((candidate) => candidate.reference.questionId === id);
      if (matches.length === 1) candidates.push(matches[0]!);
      else unresolved.push(id);
    }
    if (unresolved.length > 0) {
      setMessage(
        `These IDs do not identify one available Published Question: ${unresolved.join(", ")}. Check the list below or remove Questions already added; no Questions were added.`,
      );
      return;
    }
    // ASVS 2.2.1, 2.2.2: bound the whole local batch; the existing Save API remains authoritative.
    if (candidates.length > remainingQuestionCapacity()) {
      setMessage(
        "Enter fewer Question IDs. An Assessment may deliver at most 250 Questions, counting each Pool's selected Questions. No Questions were added.",
      );
      return;
    }
    appendAvailableQuestions(candidates);
    setQuestionIdsToAdd("");
    setMessage(
      `${candidates.length} published Questions added with their exact Revision pins. Save Questions when ready.`,
    );
  }

  async function save(): Promise<boolean> {
    setBlueprintReview(undefined);
    if (needsReload()) {
      setMessage("Reload the latest assessment before saving. Your current Entries remain here.");
      return false;
    }
    setBusy(true);
    try {
      await workspace.save(questionSaveInput(workspace.assessment().workspace, title(), entries()));
      setDirty((current) => nextQuestionEditDirty(current, "saveSucceeded"));
      setMessage("Questions and order saved. Review Assessment Properties when you are ready.");
      return true;
    } catch (error: unknown) {
      const conflict = error instanceof LiveAssessmentWorkspaceConflictError;
      setNeedsReload(conflict);
      setMessage(
        conflict
          ? "This assessment changed elsewhere. Reload latest assessment before saving; your current Entries remain here."
          : "Questions were not saved. Try again.",
      );
      setDirty((current) => nextQuestionEditDirty(current, "saveFailed"));
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function reload(discardLocalChanges = false): Promise<void> {
    setBlueprintReview(undefined);
    if (dirty() && !discardLocalChanges) {
      setMessage(
        "Your unsaved Assessment changes remain here. Discard them explicitly before reloading.",
      );
      return;
    }
    if (discardLocalChanges) setDirty(false);
    setBusy(true);
    try {
      const latest = await workspace.reloadAssessment();
      setEntries(latest.workspace.entries);
      setQuestionIdsToAdd("");
      setTitle(latest.workspace.title);
      if ((await loadPoolForks(latest.workspace.entries)) === "loaded") {
        setNeedsReload(false);
        setMessage("Latest assessment loaded. Review its complete ordered Entries.");
      } else {
        setNeedsReload(true);
        setMessage(
          "Latest Assessment loaded, but exact Question Pool members could not load. Reload again.",
        );
      }
    } catch {
      setMessage("The latest assessment could not load. Your current Entries remain here.");
    } finally {
      setBusy(false);
    }
  }

  async function reviewBlueprintUpdate(): Promise<void> {
    if (busy() || dirty() || needsReload()) return;
    setBusy(true);
    setBlueprintReview(undefined);
    setMessage("Loading Blueprint update review...");
    try {
      const review = await applicationApi.client.getAssessmentBlueprintUpdateReview(
        workspace.courseReference,
        workspace.assessmentReference,
      );
      setBlueprintReview(review);
      setMessage(
        "Review the saved current content and proposed Blueprint content before applying.",
      );
    } catch {
      setMessage(
        "Blueprint update review is unavailable. Your Assessment has not changed. Try reviewing again.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function applyBlueprintUpdate(): Promise<void> {
    const review = blueprintReview();
    if (
      busy() ||
      dirty() ||
      review === undefined ||
      review.cannotApplyReason !== null ||
      review.proposed === null
    )
      return;
    setBusy(true);
    try {
      // ASVS 2.3.1: apply only the source Revision and saved Edit Number explicitly reviewed.
      await applicationApi.client.applyAssessmentBlueprintUpdate(
        workspace.courseReference,
        workspace.assessmentReference,
        {
          expectedSourceRevision: review.sourceRevision,
          expectedEditNumber: review.assessment.editNumber,
        },
      );
      setBlueprintReview(undefined);
      await refreshAfterPoolMutation(
        "Blueprint update applied. Dates, release status, and existing Student Work were preserved.",
      );
    } catch (error: unknown) {
      const stale =
        error instanceof ApiRequestError && (error.status === 409 || error.status === 412);
      setBlueprintReview(undefined);
      setMessage(
        stale
          ? "The Assessment or parent Blueprint changed after this review. Review the Blueprint update again before applying. Nothing was applied."
          : "Blueprint update could not be applied. Review the update again before retrying. Your current Assessment remains here.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function refreshAfterPoolMutation(success: string): Promise<void> {
    try {
      const latest = await workspace.reloadAssessment();
      setEntries(latest.workspace.entries);
      setTitle(latest.workspace.title);
      if ((await loadPoolForks(latest.workspace.entries)) === "loaded") {
        setNeedsReload(false);
        setMessage(success);
      } else {
        setNeedsReload(true);
        setMessage(
          `${success} It was committed, but exact Question Pool members could not load. Reload again.`,
        );
      }
    } catch {
      setNeedsReload(true);
      setMessage(
        `${success} It was committed, but the latest Assessment could not load. Reload again.`,
      );
    }
  }

  async function updatePoolSelectionCount(
    entry: Extract<AssessmentEntry, { readonly kind: "questionPool" }>,
    selectionCount: number,
  ): Promise<void> {
    if (dirty() || needsReload()) return;
    setBusy(true);
    try {
      await applicationApi.client.updateAssessmentQuestionPoolSelectionCount(
        workspace.courseReference,
        workspace.assessmentReference,
        entry.id,
        selectionCount,
        workspace.assessment().etag,
      );
      await refreshAfterPoolMutation("Question Pool selection count updated.");
    } catch (error: unknown) {
      const conflict = error instanceof AssessmentPoolForkConflictError;
      setNeedsReload(conflict);
      setMessage(
        conflict
          ? "This Assessment changed elsewhere. Reload latest Assessment before changing its Question Pool."
          : "Question Pool selection count was not saved. Try again.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function replacePoolMembers(
    entry: Extract<AssessmentEntry, { readonly kind: "questionPool" }>,
    members: ReadonlyArray<AssessmentQuestionPickerEntry["reference"]>,
  ): Promise<void> {
    const fork = poolForks().get(entry.id);
    if (dirty() || needsReload() || fork === undefined) return;
    setBusy(true);
    try {
      await applicationApi.client.appendAssessmentQuestionPoolForkRevision(
        workspace.courseReference,
        workspace.assessmentReference,
        entry.id,
        {
          expectedPoolMetadataEtag: fork.poolMetadataEtag,
          members,
          interchangeabilityAttested: true,
        },
        workspace.assessment().etag,
      );
      await refreshAfterPoolMutation("Assessment-owned Question Pool membership updated.");
    } catch (error: unknown) {
      const conflict = error instanceof AssessmentPoolForkConflictError;
      setNeedsReload(conflict);
      setMessage(
        conflict
          ? "This Assessment changed elsewhere. Reload latest Assessment before changing its Question Pool."
          : "Question Pool membership was not saved. Try again.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function importPool(): Promise<void> {
    if (dirty() || needsReload()) return;
    const source = availablePools().find(
      (pool) => pool.questionPoolRevision.questionPoolId === poolToImport(),
    );
    const selectionCount = Number(poolSelectionCount());
    if (
      source === undefined ||
      !Number.isSafeInteger(selectionCount) ||
      selectionCount < 1 ||
      selectionCount > source.memberCount
    ) {
      setMessage("Choose a Question Pool and a selection count no greater than its member count.");
      return;
    }
    setBusy(true);
    try {
      await applicationApi.client.importAssessmentQuestionPoolFork(
        workspace.courseReference,
        workspace.assessmentReference,
        {
          sourceQuestionPoolId: source.questionPoolRevision.questionPoolId,
          authoredPosition: entries().length,
          selectionCount,
          pointsPerItem: poolPointsPerItem(),
          selectedQuestionOrder: poolSelectedQuestionOrder(),
          scoringRule: poolScoringRule(),
        },
        workspace.assessment().etag,
      );
      setPoolToImport("");
      await refreshAfterPoolMutation("Question Pool imported as an Assessment-owned fork.");
    } catch (error: unknown) {
      const conflict = error instanceof AssessmentPoolForkConflictError;
      setNeedsReload(conflict);
      setMessage(
        conflict
          ? "This Assessment changed elsewhere. Reload latest Assessment before importing a Question Pool."
          : "Question Pool import was not saved. Try again.",
      );
    } finally {
      setBusy(false);
    }
  }

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
                <h3>Review Blueprint Revision {review().sourceRevision}</h3>
                <p>
                  Apply replaces this Assessment's title, instructions, reusable settings, and
                  ordered Questions and Question Pools, including local customizations. Dates,
                  release status, and existing Student Work are preserved. New Attempts use the
                  updated content. For each proposed Library source Question Pool, Apply creates a
                  replacement Assessment-owned fork from the exact source Pool Revision shown below.
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
                  <strong>{candidate.reference.questionId}</strong> * Revision{" "}
                  {candidate.reference.revisionNumber}: {candidate.description}{" "}
                  <A href={questionRevisionInspectionPath(candidate.reference)}>Inspect</A>{" "}
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
                  <option value={pool.questionPoolRevision.questionPoolId}>
                    {pool.metadata.title} - {pool.questionPoolRevision.questionPoolId} Revision{" "}
                    {pool.questionPoolRevision.revisionNumber} ({pool.memberCount} Questions)
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
            workspace.courseReference,
            workspace.assessmentReference,
            "policies",
          )}
        >
          Review Assessment Properties
        </A>
      </p>
    </section>
  );
}
