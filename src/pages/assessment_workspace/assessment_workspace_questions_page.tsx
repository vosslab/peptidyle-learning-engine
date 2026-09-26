import { createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { AssessmentQuestionPoolForkView } from "../../../generated/api/AssessmentQuestionPoolForkView";
import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { AssessmentBlueprintUpdateReview } from "../../api/assessment_release";
import { ApiRequestError } from "../../api/http_client/error";
import { useApplicationApi } from "../../api/application_api";
import { createQuestionLibraryRepository } from "../../api/question_library_repository";
import { LiveAssessmentWorkspaceConflictError } from "../../api/http_client/assessment_release";
import { AssessmentPoolForkConflictError } from "../../api/http_client/assessment_pool_fork";
import {
  questionLibraryPickerRepository,
  questionLibraryPickerSources,
  type QuestionPickerSelection,
} from "../../features/question_picker";
import type { QuestionPoolPickerSelection } from "../../features/question_pool_picker/question_pool_picker";
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
  const [pickedFacts, setPickedFacts] = createSignal<
    ReadonlyMap<
      string,
      { readonly description: string; readonly bloom: BloomClassificationView | null }
    >
  >(new Map());
  const [poolImport, setPoolImport] = createSignal<QuestionPoolPickerSelection>();
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [needsReload, setNeedsReload] = createSignal(false);
  const [dirty, setDirty] = createSignal(false);
  const [blueprintReview, setBlueprintReview] = createSignal<AssessmentBlueprintUpdateReview>();
  const [poolForks, setPoolForks] = createSignal<
    ReadonlyMap<AssessmentEntryId, AssessmentQuestionPoolForkView>
  >(new Map());
  const [poolForkLoadFailed, setPoolForkLoadFailed] = createSignal(false);
  const questionLibrary = createQuestionLibraryRepository(applicationApi.client);
  const myQuestions = createQuestionLibraryRepository(
    applicationApi.client,
    "authoredByCurrentAccount",
  );
  const pickerRepository = questionLibraryPickerRepository(questionLibrary, myQuestions);
  const pickerSources = questionLibraryPickerSources(true);
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
      known.set(questionRevisionKey(question.publishedQuestionRevisionTuple), question.description);
    for (const [key, fact] of pickedFacts()) known.set(key, fact.description);
    return known;
  });
  const fixedBlooms = createMemo(() => {
    const known = new Map<string, BloomClassificationView>();
    for (const question of workspace.assessment().workspace.questions) {
      if (question.bloom !== null)
        known.set(questionRevisionKey(question.publishedQuestionRevisionTuple), question.bloom);
    }
    for (const [key, fact] of pickedFacts()) {
      if (fact.bloom !== null) known.set(key, fact.bloom);
    }
    return known;
  });
  const entryBlooms = createMemo(() => {
    const known = new Map<AssessmentEntryId, BloomClassificationView>();
    for (const entry of entries()) {
      const bloom =
        entry.kind === "fixedQuestion"
          ? fixedBlooms().get(questionRevisionKey(entry.publishedQuestionRevisionTuple))
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
      return "Bloom sorting waits until all Questions and Question Pools have a Bloom Classification.";
    }
    return undefined;
  });
  const remainingQuestionCapacity = createMemo(() => {
    const questionCount = deliveredAssessmentQuestionCount(entries());
    return Math.max(
      0,
      Math.min(MAX_ASSIGNMENT_ENTRIES - entries().length, MAX_ASSESSMENT_QUESTIONS - questionCount),
    );
  });

  onMount(() => {
    void loadInitialPoolForks();
  });

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

  function description(publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple): string {
    return (
      descriptions().get(questionRevisionKey(publishedQuestionRevisionTuple)) ??
      "Published Question"
    );
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
          "Bloom sorting waits until all Questions and Question Pools have a Bloom Classification.",
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

  function rememberPickedQuestions(selection: QuestionPickerSelection): void {
    setPickedFacts((current) => {
      const next = new Map(current);
      for (const question of selection.questions) {
        next.set(questionRevisionKey(question.row.publishedQuestionRevisionTuple), {
          description: question.row.summary,
          bloom: question.row.bloom,
        });
      }
      return next;
    });
  }

  function addPublishedQuestions(selection: QuestionPickerSelection): void {
    if (busy()) return;
    if (needsReload()) {
      setMessage("Reload the latest Assessment before adding an entry.");
      return;
    }
    const candidates = selection.questions.filter(
      (question) =>
        !entries().some(
          (entry) =>
            entry.kind === "fixedQuestion" &&
            questionRevisionKey(entry.publishedQuestionRevisionTuple) ===
              questionRevisionKey(question.row.publishedQuestionRevisionTuple),
        ),
    );
    if (candidates.length === 0) {
      setMessage(
        "Those published Questions are already on this Assessment. No Questions were added.",
      );
      return;
    }
    if (candidates.length > remainingQuestionCapacity()) {
      setMessage(
        "An Assessment may deliver at most 250 Questions, counting each Pool's selected Questions. No Questions were added.",
      );
      return;
    }
    rememberPickedQuestions({
      ...selection,
      questions: candidates,
      questionIds: candidates.map((question) => question.questionId),
    });
    setEntries((current) => {
      let next = current;
      for (const candidate of candidates) {
        next = appendAvailableFixedQuestion(
          next,
          candidate.row.publishedQuestionRevisionTuple,
          entryId(),
        );
      }
      return next;
    });
    setDirty((current) => nextQuestionEditDirty(current, "add"));
    setMessage(
      candidates.length === 1
        ? "Available published Question added with its exact revision pin. Save Questions when ready."
        : `${candidates.length} published Questions added with their exact Revision pins. Save Questions when ready.`,
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
          expectedSourceBlueprintRevisionTuple: review.sourceBlueprintRevisionTuple,
          expectedAssessmentEditNumber: review.assessment.assessmentEditNumber,
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
        workspace.assessment().workspace.assessmentEditNumber,
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
    members: ReadonlyArray<PublishedQuestionRevisionTuple>,
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
          expectedQuestionPoolEditNumber: fork.questionPoolEditNumber,
          members,
          interchangeabilityAttested: true,
        },
        workspace.assessment().workspace.assessmentEditNumber,
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

  function choosePool(selection: QuestionPoolPickerSelection): void {
    setPoolImport(selection);
    setPoolSelectionCount("1");
    setMessage(
      `Question Pool ${selection.questionPoolId}, Edit ${selection.questionPoolEditNumber}, is ready to import. Existing Assessment Entries remain here.`,
    );
  }

  async function importPool(): Promise<void> {
    if (dirty() || needsReload()) return;
    const source = poolImport();
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
        workspace.assessment().workspace.assessmentEditNumber,
      );
      setPoolImport(undefined);
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
    pickerRepository,
    pickerSources,
    questionPoolClient: applicationApi.client,
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
  });
}
