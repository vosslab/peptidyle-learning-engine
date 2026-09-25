import { Show, createMemo, createSignal, type JSX } from "solid-js";

import type { AssessmentPointValue } from "../../../generated/api/AssessmentPointValue";
import { UnsavedChangesGuard } from "../../components/unsaved_changes_guard";
import { RecordList } from "../../components/record_list/record_list";
import type { RecordRegion } from "../../components/record_list/region_spec";
import { LiveAssessmentWorkspaceConflictError } from "../../api/http_client/assessment_release";
import { useAssessmentWorkspace } from "./assessment_workspace_live_page";
import {
  assessmentPointValueDraft,
  questionSaveInput,
  withFixedQuestionPointValues,
} from "./assessment_workspace_questions_model";

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

  const dirty = (): boolean =>
    editing() && fixedEntries().some((entry) => draft()[entry.id] !== original()[entry.id]);
  const valid = (): boolean =>
    fixedEntries().every(
      (entry) => assessmentPointValueDraft(draft()[entry.id] ?? "") !== undefined,
    );

  function setEditorActive(active: boolean): void {
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
        save={save}
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
          <fieldset class="assessment-editor-policy-panel" disabled={saving() || reloading()}>
            <legend>Points assigned to each fixed Question</legend>
            <RecordList
              rows={fixedEntries()}
              regions={
                [
                  {
                    id: "identity",
                    role: "identity",
                    priority: "required",
                    width: "minmax(0, 1fr)",
                    align: "start",
                    content: (entry): JSX.Element => {
                      const inputId = `fixed-question-points-${entry.id}`;
                      const value = (): string => draft()[entry.id] ?? "";
                      return (
                        <label class="assessment-editor-field" for={inputId}>
                          Question {entry.publishedQuestionRevisionTuple.publishedQuestionId}{" "}
                          Revision {entry.publishedQuestionRevisionTuple.revisionNumber}
                          <input
                            id={inputId}
                            inputmode="decimal"
                            value={value()}
                            aria-invalid={assessmentPointValueDraft(value()) === undefined}
                            onInput={(event) => {
                              setDraft((current) => ({
                                ...current,
                                [entry.id]: event.currentTarget.value,
                              }));
                              setMessage("");
                              setFailed(false);
                            }}
                          />
                        </label>
                      );
                    },
                  },
                ] satisfies ReadonlyArray<
                  RecordRegion<
                    typeof fixedEntries extends () => ReadonlyArray<infer Entry> ? Entry : never
                  >
                >
              }
              recordId={(entry) => entry.id}
              state={{ kind: "ready" }}
              ariaLabel="Fixed Question point values"
              emptyState={{
                title: "No fixed Questions are currently selected for this Assessment.",
              }}
            />
            <p class="assessment-editor-note">
              Enter zero or a positive point value with no more than four decimal places.
            </p>
            <p class="assessment-editor-actions">
              <button
                class="primary-action"
                type="button"
                disabled={!valid() || saving() || conflict()}
                onClick={() => void save()}
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
