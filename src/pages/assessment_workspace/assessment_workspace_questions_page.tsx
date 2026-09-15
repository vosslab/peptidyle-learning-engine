import { A } from "@solidjs/router";
import { For, Show, createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentEntryId } from "../../../generated/api/AssessmentEntryId";
import type { AssessmentQuestionPoolForkView } from "../../../generated/api/AssessmentQuestionPoolForkView";
import type { QuestionPoolLibrarySummary } from "../../../generated/api/QuestionPoolLibrarySummary";
import type {
  AssessmentQuestionPickerEntry,
  LiveAssessmentWorkspace,
  SaveLiveAssessmentInput,
} from "../../api/assessment_release";
import { useApplicationApi } from "../../api/application_api";
import { LiveAssessmentWorkspaceConflictError } from "../../api/http_client/assessment_release";
import { AssessmentPoolForkConflictError } from "../../api/http_client/assessment_pool_fork";
import { AssessmentPoolEntryEditor } from "./assessment_pool_entry_editor";
import { useAssessmentWorkspace } from "./assessment_workspace_live_page";
import { assessmentWorkspacePath } from "./assessment_workspace_paths";
import {
  appendAvailableFixedQuestion,
  moveAssessmentEntry,
  questionRevisionKey,
  removeAssessmentEntry,
} from "./assessment_workspace_questions_model";
import { UnsavedChangesGuard } from "./unsaved_changes_guard";

const MAX_ASSIGNMENT_ENTRIES = 1024;

export type QuestionEditDirtyEvent =
  "title" | "move" | "remove" | "add" | "saveSucceeded" | "saveFailed";

/** Keeps the leave guard active until the current structural edit was persisted successfully. */
export function nextQuestionEditDirty(current: boolean, event: QuestionEditDirtyEvent): boolean {
  if (event === "saveSucceeded") return false;
  if (event === "saveFailed") return current;
  return true;
}

/** Builds the closed save payload from the editable Assessment workspace values only. */
export function questionSaveInput(
  current: LiveAssessmentWorkspace,
  title: string,
  entries: ReadonlyArray<AssessmentEntry>,
): SaveLiveAssessmentInput {
  return {
    title,
    instructions: current.instructions,
    entries,
    dueAt: current.dueAt,
    availableAt: current.availableAt,
    closesAt: current.closesAt,
    lateWorkRule: current.lateWorkRule,
    assessmentAttemptTimeLimitSeconds: current.assessmentAttemptTimeLimitSeconds,
    attemptLimit: current.attemptLimit,
    activityRules: current.activityRules,
    studentFeedbackReleaseRule: current.studentFeedbackReleaseRule,
  };
}

function entryId(): AssessmentEntryId {
  return crypto.randomUUID();
}

function questionPoolEntry(
  entry: AssessmentEntry,
): Extract<AssessmentEntry, { readonly kind: "questionPool" }> | undefined {
  return entry.kind === "questionPool" ? entry : undefined;
}

function AssessmentEntrySummary(props: {
  readonly entry: AssessmentEntry;
  readonly description: (reference: AssessmentQuestionPickerEntry["reference"]) => string;
}): JSX.Element {
  if (props.entry.kind === "questionPool") {
    return (
      <>
        <strong>Question Pool</strong> - {props.entry.selectionCount} selected from its exact pinned
        Pool Revision
      </>
    );
  }
  return (
    <>
      <strong>{props.entry.reference.questionId}</strong> * Revision{" "}
      {props.entry.reference.revisionNumber}: {props.description(props.entry.reference)} (
      {props.entry.availability})
    </>
  );
}

/** Edits the complete normalized content owned by the current Assessment. */
export function AssessmentWorkspaceQuestionsPage(): JSX.Element {
  const workspace = useAssessmentWorkspace();
  const applicationApi = useApplicationApi();
  const initial = workspace.assessment().workspace;
  const [entries, setEntries] = createSignal<ReadonlyArray<AssessmentEntry>>(initial.entries);
  const [title, setTitle] = createSignal(initial.title);
  const [available, setAvailable] = createSignal<ReadonlyArray<AssessmentQuestionPickerEntry>>([]);
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [needsReload, setNeedsReload] = createSignal(false);
  const [dirty, setDirty] = createSignal(false);
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

  function remove(index: number): void {
    if (needsReload()) {
      setMessage("Reload the latest Assessment before removing an entry.");
      return;
    }
    setEntries((current) => removeAssessmentEntry(current, index));
    setDirty((current) => nextQuestionEditDirty(current, "remove"));
    setMessage("Entry removed. Save Questions when ready.");
  }

  function add(candidate: AssessmentQuestionPickerEntry): void {
    if (needsReload()) {
      setMessage("Reload the latest Assessment before adding an entry.");
      return;
    }
    setEntries((current) => appendAvailableFixedQuestion(current, candidate, entryId()));
    setDirty((current) => nextQuestionEditDirty(current, "add"));
    setMessage(
      "Available published Question added with its exact revision pin. Save Questions when ready.",
    );
  }

  async function save(): Promise<boolean> {
    if (needsReload()) {
      setMessage("Reload the latest assessment before saving. Your current Entries remain here.");
      return false;
    }
    setBusy(true);
    try {
      await workspace.save(questionSaveInput(workspace.assessment().workspace, title(), entries()));
      setDirty((current) => nextQuestionEditDirty(current, "saveSucceeded"));
      setMessage("Questions and order saved. Review assessment policies when you are ready.");
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
        <Show when={entries().length > 0} fallback={<p>No Entries are selected.</p>}>
          <ol>
            <For each={entries()}>
              {(entry, index) => (
                <li data-assessment-entry={entry.id}>
                  <AssessmentEntrySummary entry={entry} description={description} />{" "}
                  <button
                    type="button"
                    disabled={busy() || needsReload() || index() === 0}
                    onClick={() => move(index(), -1)}
                  >
                    Move earlier
                  </button>{" "}
                  <button
                    type="button"
                    disabled={busy() || needsReload() || index() === entries().length - 1}
                    onClick={() => move(index(), 1)}
                  >
                    Move later
                  </button>{" "}
                  <button
                    type="button"
                    disabled={busy() || needsReload()}
                    aria-label={`Remove Assessment Entry ${index() + 1}`}
                    onClick={() => remove(index())}
                  >
                    Remove
                  </button>
                  <Show when={questionPoolEntry(entry)}>
                    {(poolEntry) => (
                      <AssessmentPoolEntryEditor
                        entry={poolEntry()}
                        fork={poolForks().get(poolEntry().id)}
                        exactMembersUnavailable={poolForkLoadFailed()}
                        availableQuestions={available()}
                        mutationsEnabled={!dirty() && !needsReload()}
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
          Adding a Question pins the exact Available revision shown here.
        </p>
        <Show
          when={availableToAdd().length > 0}
          fallback={<p>No additional Available Questions are ready to add.</p>}
        >
          <ul>
            <For each={availableToAdd()}>
              {(candidate) => (
                <li>
                  <strong>{candidate.reference.questionId}</strong> * Revision{" "}
                  {candidate.reference.revisionNumber}: {candidate.description}{" "}
                  <button
                    type="button"
                    disabled={busy() || needsReload() || entries().length >= MAX_ASSIGNMENT_ENTRIES}
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
                    {pool.questionPoolRevision.questionPoolId} Revision{" "}
                    {pool.questionPoolRevision.revisionNumber} ({pool.memberCount} Questions)
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
          Review assessment policies
        </A>
      </p>
    </section>
  );
}
