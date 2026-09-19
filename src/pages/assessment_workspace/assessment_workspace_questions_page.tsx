import { createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { AssessmentQuestionPoolForkView } from "../../../generated/api/AssessmentQuestionPoolForkView";
import type { QuestionPoolLibrarySummary } from "../../../generated/api/QuestionPoolLibrarySummary";
import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type {
  AssessmentQuestionPickerEntry,
  AssessmentBlueprintUpdateReview,
} from "../../api/assessment_release";
import { ApiRequestError } from "../../api/http_client/error";
import { useApplicationApi } from "../../api/application_api";
import { LiveAssessmentWorkspaceConflictError } from "../../api/http_client/assessment_release";
import { AssessmentPoolForkConflictError } from "../../api/http_client/assessment_pool_fork";
import { normalizeHumanEnteredQuestionId } from "../../question_id";
import { useAssessmentWorkspace } from "./assessment_workspace_live_page";
import {
  appendAvailableFixedQuestion,
  deliveredAssessmentQuestionCount,
  moveAssessmentEntry,
  nextQuestionEditDirty,
  questionSaveInput,
  questionRevisionKey,
  removeAssessmentEntry,
  sortAssessmentEntriesByBloom,
} from "./assessment_workspace_questions_model";
import { AssessmentWorkspaceQuestionsView } from "./assessment_workspace_questions_view";

const MAX_ASSIGNMENT_ENTRIES = 1024;
const MAX_ASSESSMENT_QUESTIONS = 250;

function entryId(): AssessmentEntryId {
  return crypto.randomUUID();
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
      known.set(questionRevisionKey(question.questionRevision), question.description);
    for (const question of available())
      known.set(questionRevisionKey(question.questionRevision), question.description);
    return known;
  });
  const fixedBlooms = createMemo(() => {
    const known = new Map<string, BloomClassificationView>();
    for (const question of workspace.assessment().workspace.questions) {
      if (question.bloom !== null)
        known.set(questionRevisionKey(question.questionRevision), question.bloom);
    }
    for (const question of available()) {
      if (question.bloom !== null)
        known.set(questionRevisionKey(question.questionRevision), question.bloom);
    }
    return known;
  });
  const entryBlooms = createMemo(() => {
    const known = new Map<AssessmentEntryId, BloomClassificationView>();
    for (const entry of entries()) {
      const bloom =
        entry.kind === "fixedQuestion"
          ? fixedBlooms().get(questionRevisionKey(entry.questionRevision))
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
            questionRevisionKey(entry.questionRevision) === questionRevisionKey(candidate.questionRevision),
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
        applicationApi.client.listLiveAssessmentQuestionPicker(workspace.courseInstanceId),
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
                workspace.courseInstanceId,
                workspace.assessmentId,
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

  function description(reference: AssessmentQuestionPickerEntry["questionRevision"]): string {
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
      const matches = availableToAdd().filter((candidate) => candidate.questionRevision.questionId === id);
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
        workspace.courseInstanceId,
        workspace.assessmentId,
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
        workspace.courseInstanceId,
        workspace.assessmentId,
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
        workspace.courseInstanceId,
        workspace.assessmentId,
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
    members: ReadonlyArray<AssessmentQuestionPickerEntry["questionRevision"]>,
  ): Promise<void> {
    const fork = poolForks().get(entry.id);
    if (dirty() || needsReload() || fork === undefined) return;
    setBusy(true);
    try {
      await applicationApi.client.appendAssessmentQuestionPoolForkMembers(
        workspace.courseInstanceId,
        workspace.assessmentId,
        entry.id,
        {
          expectedQuestionPoolEditNumber: String(fork.questionPoolEditNumber),
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
    const source = availablePools().find((pool) => pool.questionPoolId === poolToImport());
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
        workspace.courseInstanceId,
        workspace.assessmentId,
        {
          sourceQuestionPoolId: source.questionPoolId,
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

  return AssessmentWorkspaceQuestionsView({
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
  });
}
