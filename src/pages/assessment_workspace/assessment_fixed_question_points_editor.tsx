import {
  Show,
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  type Accessor,
  type JSX,
} from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentPointValue } from "../../../generated/api/AssessmentPointValue";
import { assessmentPointValueDraft } from "../../assessment_point_value";
import { UnsavedChangesGuard, trackEditorSave } from "../../components/unsaved_changes_guard";
import { RecordDetails } from "../../components/data_sheet/record_content";
import type { RecordContent } from "../../components/data_sheet/record_content";
import type { RecordField } from "../../components/data_sheet/record_fields";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { LiveAssessmentWorkspaceConflictError } from "../../api/http_client/assessment_release";
import { useAssessmentWorkspace } from "./assessment_workspace_live_page";
import {
  questionSaveInput,
  withFixedQuestionPointValues,
} from "./assessment_workspace_questions_model";

type FixedQuestionEntry = Extract<AssessmentEntry, { readonly kind: "fixedQuestion" }>;
type FixedQuestionSort = never;
type FixedQuestionFilter = "question_id" | "revision";

export interface AssessmentFixedQuestionPointsEditorProps {
  readonly disabled: boolean;
  readonly onEditingChange: (editing: boolean) => void;
  readonly reloadLatest: () => Promise<boolean>;
}

/** Focused full-aggregate editor for fixed-Question point values in Assessment Properties. */
export function AssessmentFixedQuestionPointsEditor(
  props: AssessmentFixedQuestionPointsEditorProps,
): JSX.Element {
  const workspace = useAssessmentWorkspace();
  const fixedEntries = createMemo(() =>
    workspace.assessment().workspace.entries.filter((entry) => entry.kind === "fixedQuestion"),
  );
  const [editing, setEditing] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [original, setOriginal] = createSignal<Readonly<Record<string, string>>>({});
  const [draft, setDraft] = createSignal<Readonly<Record<string, string>>>({});
  const [message, setMessage] = createSignal("");
  const [failed, setFailed] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);
  const [reloading, setReloading] = createSignal(false);
  const [selectedEntryId, setSelectedEntryId] = createSignal<string | null>(null);
  const selectedEntry = createMemo(() =>
    fixedEntries().find((entry) => entry.id === selectedEntryId()),
  );
  const detailsId = `fixed-question-point-details-${createUniqueId()}`;
  let detailsSection: HTMLElement | undefined;
  let editTrigger: HTMLButtonElement | undefined;
  let editorFieldset: HTMLFieldSetElement | undefined;

  createEffect(() => {
    if (selectedEntryId() !== null && selectedEntry() === undefined) setSelectedEntryId(null);
  });

  const dirty = (): boolean =>
    editing() && fixedEntries().some((entry) => draft()[entry.id] !== original()[entry.id]);
  // ASVS 2.2.1: the existing point parser gates all draft values before aggregate Save.
  const valid = (): boolean =>
    fixedEntries().every(
      (entry) => assessmentPointValueDraft(draft()[entry.id] ?? "") !== undefined,
    );

  function setEditorActive(active: boolean): void {
    setSelectedEntryId(null);
    setEditing(active);
    props.onEditingChange(active);
  }

  function beginEditing(): void {
    if (props.disabled || fixedEntries().length === 0) return;
    const values = Object.fromEntries(
      fixedEntries().map((entry) => [entry.id, entry.pointsPossible]),
    );
    setOriginal(values);
    setDraft(values);
    setMessage("");
    setFailed(false);
    setConflict(false);
    setEditorActive(true);
  }

  function cancel(): void {
    setDraft(original());
    setMessage("Point-value changes cancelled.");
    setFailed(false);
    setConflict(false);
    setEditorActive(false);
  }

  async function save(): Promise<boolean> {
    if (!editing() || !valid() || saving() || conflict()) return false;
    const parsed: Record<string, AssessmentPointValue> = {};
    for (const entry of fixedEntries()) {
      const value = assessmentPointValueDraft(draft()[entry.id] ?? "");
      if (value === undefined) return false;
      parsed[entry.id] = value;
    }
    setSaving(true);
    setMessage("");
    setFailed(false);
    setConflict(false);
    try {
      const current = workspace.assessment().workspace;
      const entries = withFixedQuestionPointValues(current.entries, parsed);
      await workspace.save(questionSaveInput(current, current.title, entries));
      setOriginal({ ...draft() });
      setMessage("Fixed Question point values saved.");
      setEditorActive(false);
      return true;
    } catch (error: unknown) {
      const stale = error instanceof LiveAssessmentWorkspaceConflictError;
      setFailed(true);
      setConflict(stale);
      setMessage(
        stale
          ? "This Assessment changed elsewhere. Reload the latest Assessment before saving again. Your typed point values remain here."
          : "Fixed Question point values were not saved. Your typed values remain here.",
      );
      return false;
    } finally {
      setSaving(false);
    }
  }
  const trackedSave = trackEditorSave(save);

  function fixedQuestionContent(entry: FixedQuestionEntry): RecordContent {
    const revision = entry.publishedQuestionRevisionTuple;
    const questionTitle = workspace
      .assessment()
      .workspace.questions.find(
        (question) =>
          question.publishedQuestionRevisionTuple.publishedQuestionId ===
            revision.publishedQuestionId &&
          question.publishedQuestionRevisionTuple.revisionNumber === revision.revisionNumber,
      )?.questionTitle;
    return {
      // ASVS 1.2.1: Question titles and draft values remain escaped shared record text.
      title: questionTitle ?? "Question",
      details: [
        {
          kind: "text",
          label: "Position",
          value: String(fixedQuestionPosition(entry)),
        },
        { kind: "text", label: "Question ID", value: revision.publishedQuestionId },
        { kind: "text", label: "Revision", value: String(revision.revisionNumber) },
        { kind: "text", label: "Points", value: draft()[entry.id] ?? "" },
      ],
      actions: [
        {
          id: "edit_point_value",
          kind: "command",
          label: "Edit point value",
          title: `Edit point value for entry ${fixedQuestionPosition(entry)}: ${questionTitle ?? "Question"}`,
          controls: detailsId,
          expanded: selectedEntryId() === entry.id,
          onClick: (event): void => {
            editTrigger = event.currentTarget;
            setSelectedEntryId(entry.id);
            queueMicrotask(() => detailsSection?.focus());
          },
        },
      ],
    };
  }

  function fixedQuestionPosition(entry: FixedQuestionEntry): number {
    return workspace.assessment().workspace.entries.findIndex((item) => item.id === entry.id) + 1;
  }

  const fields: ReadonlyArray<
    RecordField<FixedQuestionEntry, FixedQuestionSort, FixedQuestionFilter>
  > = [
    {
      id: "title",
      label: "Question",
      role: "identity",
      value: (entry) => fixedQuestionContent(entry).title,
      defaultColumn: { priority: 0, width: "20rem" },
    },
    {
      id: "question_id",
      label: "Question ID",
      filterKey: "question_id",
      value: (entry) => entry.publishedQuestionRevisionTuple.publishedQuestionId,
      defaultColumn: { priority: 2, width: "14rem" },
    },
    {
      id: "revision",
      label: "Revision",
      filterKey: "revision",
      value: (entry) => entry.publishedQuestionRevisionTuple.revisionNumber,
      defaultColumn: { priority: 3, width: "7rem" },
    },
    {
      id: "position",
      label: "Position",
      value: fixedQuestionPosition,
      defaultColumn: { priority: 4, width: "7rem" },
    },
    {
      id: "points",
      label: "Points",
      value: (entry) => draft()[entry.id] ?? "",
      defaultColumn: { priority: 1, width: "8rem" },
    },
  ];
  // Search projects the complete collection; the draft and aggregate Save retain every entry.
  const pointSource = (): ReturnType<typeof fixedEntries> => {
    draft();
    void workspace.assessment().workspace.questions;
    return fixedEntries();
  };
  const pointSearch = createLocalCompleteSearch(
    { rows: pointSource },
    {
      fields,
      rowId: (entry) => entry.id,
      keyword: (entry) => fields.map((field) => String(field.value(entry) ?? "")).join(" "),
      content: fixedQuestionContent,
    },
  );
  // Saved Assessment order is fixed; search and filters retain the original positions.
  const pointQuery = { kind: pointSearch.query.kind, filters: pointSearch.query.filters };

  function closeEntryDetails(): void {
    setSelectedEntryId(null);
    if (editTrigger?.isConnected) editTrigger.focus();
    else editorFieldset?.focus();
  }

  function renderInlinePointValue(entry: Accessor<FixedQuestionEntry>): JSX.Element {
    return <Show when={selectedEntryId() !== entry().id}>{renderPointValue(entry)}</Show>;
  }

  function renderPointValue(entry: Accessor<FixedQuestionEntry>): JSX.Element {
    const inputId = `fixed-question-points-${entry().id}`;
    const value = (): string => draft()[entry().id] ?? "";
    return (
      <label class="assessment-editor-field" for={inputId}>
        Point value
        <input
          id={inputId}
          inputmode="decimal"
          value={value()}
          aria-invalid={assessmentPointValueDraft(value()) === undefined}
          onInput={(event) => {
            setDraft((current) => ({
              ...current,
              [entry().id]: event.currentTarget.value,
            }));
            setMessage("");
            setFailed(false);
          }}
        />
      </label>
    );
  }

  async function reloadAfterConflict(): Promise<void> {
    if (!conflict() || reloading()) return;
    setReloading(true);
    try {
      if (!(await props.reloadLatest())) {
        setFailed(true);
        setMessage(
          "The latest Assessment could not load. Your typed point values remain here. Try reloading again.",
        );
        return;
      }
      const latestValues = Object.fromEntries(
        fixedEntries().map((entry) => [entry.id, entry.pointsPossible]),
      );
      setOriginal(latestValues);
      setDraft(latestValues);
      setConflict(false);
      setFailed(false);
      setMessage("Latest Assessment loaded. Point-value edits were discarded.");
      setEditorActive(false);
    } finally {
      setReloading(false);
    }
  }

  return (
    <section aria-labelledby="fixed-question-points-heading">
      <UnsavedChangesGuard
        dirty={dirty}
        save={trackedSave.save}
        inFlightSave={trackedSave.inFlightSave}
        copy={{
          heading: "Save fixed Question point values?",
          description: "Your fixed Question point-value changes have not been saved.",
          saveActionLabel: "Save point values and continue",
          savingActionLabel: "Saving point values...",
          saveFailureMessage:
            "Point values were not saved. Resolve the save error shown on the page, then try again or stay here.",
        }}
      />
      <h2 id="fixed-question-points-heading">Fixed Question point values</h2>
      <Show
        when={fixedEntries().length > 0}
        fallback={<p>No fixed Questions are currently selected for this Assessment.</p>}
      >
        <Show
          when={editing()}
          fallback={
            <button type="button" disabled={props.disabled} onClick={beginEditing}>
              Edit fixed Question point values
            </button>
          }
        >
          <fieldset
            ref={(element) => (editorFieldset = element)}
            tabindex="-1"
            class="assessment-editor-operation-boundary"
            disabled={saving() || reloading()}
          >
            <legend>Points assigned to each fixed Question</legend>
            <SearchBlock
              layout="above"
              controls={
                <SearchControls state={pointSearch.state} textLabel="Search fixed Questions" />
              }
              state={pointSearch.state}
              collection={{
                ariaLabel: "Fixed Question point values",
                initialMode: "list",
                fields,
                query: pointQuery,
                renderBody: renderInlinePointValue,
                disabled: () => saving() || reloading(),
                emptyState: { title: "No fixed Questions match your search." },
              }}
            />
            <Show when={selectedEntry()}>
              {(entry) => (
                <section
                  id={detailsId}
                  ref={(element) => (detailsSection = element)}
                  tabindex="-1"
                  aria-label="Edit fixed Question point value"
                >
                  <h3>
                    Entry {fixedQuestionPosition(entry())}: {fixedQuestionContent(entry()).title}
                  </h3>
                  <RecordDetails content={() => fixedQuestionContent(entry())} />
                  {renderPointValue(entry)}
                  <button type="button" class="quiet-action" onClick={closeEntryDetails}>
                    Close point-value details
                  </button>
                </section>
              )}
            </Show>
            <p class="assessment-editor-note">
              Enter zero or a positive point value with no more than four decimal places.
            </p>
            <p class="assessment-editor-actions">
              <button
                class="primary-action"
                type="button"
                disabled={!valid() || saving() || conflict()}
                onClick={() => void trackedSave.save()}
              >
                {saving() ? "Saving point values..." : "Save fixed Question point values"}
              </button>
              <button type="button" disabled={saving()} onClick={cancel}>
                Cancel
              </button>
            </p>
          </fieldset>
        </Show>
      </Show>
      <Show when={message()}>
        {(value) => (
          <p class="assessment-workspace-save-message" role={failed() ? "alert" : "status"}>
            {value()}
          </p>
        )}
      </Show>
      <Show when={conflict()}>
        <button type="button" disabled={reloading()} onClick={() => void reloadAfterConflict()}>
          {reloading()
            ? "Reloading latest Assessment..."
            : "Reload latest Assessment and discard point edits"}
        </button>
      </Show>
    </section>
  );
}
