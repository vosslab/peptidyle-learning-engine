// assignment_editor_picker_controller.ts - shared-picker composition for assignment destinations.

import { createSignal, type Accessor } from "solid-js";

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import {
  appendFixedEntries,
  type AssignmentQuestionRow,
  type AssignmentEditorDraft,
} from "./assignment_editor_model";
import {
  assignmentPickerMaximum,
  type AssignmentPickerIntent,
} from "./assignment_editor_picker_model";
import type { AssignmentEditorRepository } from "./assignment_editor_repository";
import type { QuestionPickerSelection, QuestionPickerSource } from "../features/question_picker";

export type AssignmentPickerMode =
  /** A persisted workspace assignment whose Questions draft saves as one focused replacement. */
  { readonly kind: "workspace"; readonly assignmentId: AssignmentId };

export interface PendingPickerSelection {
  readonly intent: AssignmentPickerIntent;
  readonly questionIds: ReadonlyArray<string>;
}

export interface AssignmentEditorPickerControllerProps {
  readonly repository: AssignmentEditorRepository;
  readonly courseId: CourseId;
  readonly mode: AssignmentPickerMode;
  readonly currentDraft: () => AssignmentEditorDraft | undefined;
  readonly editorBusy: () => boolean;
  readonly setBusy: (value: boolean) => void;
  readonly onDraftChange: (draft: AssignmentEditorDraft) => void;
  readonly onReplacementPrepared: (row: AssignmentQuestionRow, itemId: string) => void;
  readonly onMessage: (message: string) => void;
  readonly onError: (error: unknown, fallback: string) => void;
}

export interface AssignmentEditorPickerController {
  readonly sources: Accessor<ReadonlyArray<QuestionPickerSource>>;
  readonly intent: Accessor<AssignmentPickerIntent | undefined>;
  readonly pendingSelection: Accessor<PendingPickerSelection | undefined>;
  readonly trigger: () => HTMLButtonElement | undefined;
  readonly loadSources: () => Promise<void>;
  readonly open: (intent: AssignmentPickerIntent, trigger: HTMLButtonElement) => void;
  readonly useSelection: (selection: QuestionPickerSelection) => Promise<void>;
  readonly retryPendingSelection: () => Promise<void>;
  readonly cancel: () => void;
  readonly maximum: (intent: AssignmentPickerIntent) => number;
}

async function resolveRows(
  repository: AssignmentEditorRepository,
  questionIds: ReadonlyArray<string>,
): Promise<ReadonlyArray<AssignmentQuestionRow>> {
  return await Promise.all(
    questionIds.map(async (questionId) => await repository.resolvePublished(questionId)),
  );
}

export function createAssignmentEditorPickerController(
  props: AssignmentEditorPickerControllerProps,
): AssignmentEditorPickerController {
  const [sources, setSources] = createSignal<ReadonlyArray<QuestionPickerSource>>([]);
  const [intent, setIntent] = createSignal<AssignmentPickerIntent>();
  const [pendingSelection, setPendingSelection] = createSignal<PendingPickerSelection>();
  let pickerTrigger: HTMLButtonElement | undefined;

  function maximum(nextIntent: AssignmentPickerIntent): number {
    const draft = props.currentDraft();
    return draft === undefined ? 0 : assignmentPickerMaximum(draft, nextIntent);
  }

  function open(nextIntent: AssignmentPickerIntent, trigger: HTMLButtonElement): void {
    if (maximum(nextIntent) < 1) {
      props.onMessage(
        nextIntent.kind === "pool"
          ? "This pool has reached its entry limit. Remove a entry before choosing another."
          : "This assignment has reached its ordered-entry limit. Remove an entry before adding another question.",
      );
      return;
    }
    pickerTrigger = trigger;
    setIntent(nextIntent);
    setPendingSelection(undefined);
  }

  function addCreateRows(rows: ReadonlyArray<AssignmentQuestionRow>): void {
    const draft = props.currentDraft();
    if (draft === undefined) return;
    const nextDraft = appendFixedEntries(draft, rows);
    if (nextDraft === draft) {
      props.onMessage("Every selected Question ID is already in this assignment.");
      return;
    }
    props.onDraftChange(nextDraft);
    props.onMessage(
      `Added ${rows.length} selected question${rows.length === 1 ? "" : "s"} to this unsaved assignment.`,
    );
  }

  function addPoolRows(entryIndex: number, rows: ReadonlyArray<AssignmentQuestionRow>): void {
    const draft = props.currentDraft();
    const entry = draft?.entries[entryIndex];
    if (draft === undefined || entry === undefined || entry.kind !== "questionPool") return;
    const known = new Set(entry.entries.map((entry) => entry.questionId));
    const addedPoolEntries = [
      ...entry.entries,
      ...rows.filter((row) => !known.has(row.questionId)),
    ];
    if (addedPoolEntries.length === entry.entries.length) {
      props.onMessage("Every selected Question ID is already an entry in this pool.");
      return;
    }
    const assignmentEntries = [...draft.entries];
    assignmentEntries[entryIndex] = { ...entry, entries: addedPoolEntries };
    props.onDraftChange({ ...draft, entries: assignmentEntries });
    const added = addedPoolEntries.length - entry.entries.length;
    props.onMessage(
      `${added} entry Question ID${added === 1 ? "" : "s"} added to this Question Pool. Set its selection count, then save the Assignment.`,
    );
  }

  async function useSelection(selection: QuestionPickerSelection): Promise<void> {
    const currentIntent = intent();
    if (currentIntent === undefined || props.editorBusy()) return;
    props.setBusy(true);
    try {
      if (currentIntent.kind === "replacement") {
        const questionId = selection.questionIds[0];
        if (questionId === undefined) return;
        const row = await props.repository.resolvePublished(questionId);
        props.onReplacementPrepared(row, currentIntent.itemId);
      } else {
        const rows = await resolveRows(props.repository, selection.questionIds);
        if (currentIntent.kind === "pool") addPoolRows(currentIntent.entryIndex, rows);
        else addCreateRows(rows);
      }
      setIntent(undefined);
    } catch (error: unknown) {
      if (pendingSelection() === undefined) {
        setPendingSelection({ intent: currentIntent, questionIds: selection.questionIds });
      }
      props.onError(
        error,
        "The selected questions were not added. Your ordered Question IDs remain ready to retry.",
      );
    } finally {
      props.setBusy(false);
    }
  }

  async function retryPendingSelection(): Promise<void> {
    const pending = pendingSelection();
    if (pending === undefined) return;
    setIntent(pending.intent);
    await useSelection({ questionIds: pending.questionIds, questions: [] });
    if (pendingSelection()?.questionIds === pending.questionIds) setPendingSelection(undefined);
  }

  async function loadSources(): Promise<void> {
    try {
      setSources(
        await props.repository.listQuestionPickerSources(props.courseId, props.mode.assignmentId),
      );
    } catch {
      setSources([{ kind: "library", label: "Library" }]);
      props.onMessage(
        "Collections could not load. The Library and direct Question ID entry are ready.",
      );
    }
  }

  return {
    sources,
    intent,
    pendingSelection,
    trigger: () => pickerTrigger,
    loadSources,
    open,
    useSelection,
    retryPendingSelection,
    cancel: () => setIntent(undefined),
    maximum,
  };
}
