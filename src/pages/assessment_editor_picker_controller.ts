// assessment_editor_picker_controller.ts - shared-picker composition for assessment destinations.

import { createSignal, type Accessor } from "solid-js";

import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseId } from "../../generated/api/CourseId";
import {
  appendFixedEntries,
  type AssessmentQuestionRow,
  type AssessmentEditorState,
} from "./assessment_editor_model";
import {
  assessmentPickerMaximum,
  type AssessmentPickerIntent,
} from "./assessment_editor_picker_model";
import type { AssessmentEditorRepository } from "./assessment_editor_repository";
import type { QuestionPickerSelection, QuestionPickerSource } from "../features/question_picker";

export type AssessmentPickerMode =
  /** A persisted workspace assessment whose Questions draft saves as complete Assessment Content. */
  { readonly kind: "workspace"; readonly assessmentId: AssessmentId };

export interface PendingPickerSelection {
  readonly intent: AssessmentPickerIntent;
  readonly questionIds: ReadonlyArray<string>;
}

export interface AssessmentEditorPickerControllerProps {
  readonly repository: AssessmentEditorRepository;
  readonly courseId: CourseId;
  readonly mode: AssessmentPickerMode;
  readonly currentDraft: () => AssessmentEditorState | undefined;
  readonly editorBusy: () => boolean;
  readonly setBusy: (value: boolean) => void;
  readonly onDraftChange: (draft: AssessmentEditorState) => void;
  readonly onMessage: (message: string) => void;
  readonly onError: (error: unknown, fallback: string) => void;
}

export interface AssessmentEditorPickerController {
  readonly sources: Accessor<ReadonlyArray<QuestionPickerSource>>;
  readonly intent: Accessor<AssessmentPickerIntent | undefined>;
  readonly pendingSelection: Accessor<PendingPickerSelection | undefined>;
  readonly trigger: () => HTMLButtonElement | undefined;
  readonly loadSources: () => Promise<void>;
  readonly open: (intent: AssessmentPickerIntent, trigger: HTMLButtonElement) => void;
  readonly useSelection: (selection: QuestionPickerSelection) => Promise<void>;
  readonly retryPendingSelection: () => Promise<void>;
  readonly cancel: () => void;
  readonly maximum: (intent: AssessmentPickerIntent) => number;
}

async function resolveRows(
  repository: AssessmentEditorRepository,
  questionIds: ReadonlyArray<string>,
): Promise<ReadonlyArray<AssessmentQuestionRow>> {
  return await Promise.all(
    questionIds.map(async (questionId) => await repository.resolvePublished(questionId)),
  );
}

export function createAssessmentEditorPickerController(
  props: AssessmentEditorPickerControllerProps,
): AssessmentEditorPickerController {
  const [sources, setSources] = createSignal<ReadonlyArray<QuestionPickerSource>>([]);
  const [intent, setIntent] = createSignal<AssessmentPickerIntent>();
  const [pendingSelection, setPendingSelection] = createSignal<PendingPickerSelection>();
  let pickerTrigger: HTMLButtonElement | undefined;

  function maximum(nextIntent: AssessmentPickerIntent): number {
    const draft = props.currentDraft();
    return draft === undefined ? 0 : assessmentPickerMaximum(draft, nextIntent);
  }

  function open(nextIntent: AssessmentPickerIntent, trigger: HTMLButtonElement): void {
    if (maximum(nextIntent) < 1) {
      props.onMessage(
        nextIntent.kind === "pool"
          ? "This pool has reached its entry limit. Remove a entry before choosing another."
          : "This assessment has reached its ordered-entry limit. Remove an entry before adding another question.",
      );
      return;
    }
    pickerTrigger = trigger;
    setIntent(nextIntent);
    setPendingSelection(undefined);
  }

  function addCreateRows(rows: ReadonlyArray<AssessmentQuestionRow>): void {
    const draft = props.currentDraft();
    if (draft === undefined) return;
    const nextDraft = appendFixedEntries(draft, rows);
    if (nextDraft === draft) {
      props.onMessage("Every selected Question ID is already in this assessment.");
      return;
    }
    props.onDraftChange(nextDraft);
    props.onMessage(
      `Added ${rows.length} selected question${rows.length === 1 ? "" : "s"} to this unsaved assessment.`,
    );
  }

  function addPoolRows(entryIndex: number, rows: ReadonlyArray<AssessmentQuestionRow>): void {
    const draft = props.currentDraft();
    const entry = draft?.entries[entryIndex];
    if (draft === undefined || entry === undefined || entry.kind !== "questionPool") return;
    const known = new Set(entry.items.map((item) => item.questionId));
    const addedPoolItems = [...entry.items, ...rows.filter((row) => !known.has(row.questionId))];
    if (addedPoolItems.length === entry.items.length) {
      props.onMessage("Every selected Question ID is already a Question Pool Item in this pool.");
      return;
    }
    const assessmentEntries = [...draft.entries];
    assessmentEntries[entryIndex] = { ...entry, items: addedPoolItems };
    props.onDraftChange({ ...draft, entries: assessmentEntries });
    const added = addedPoolItems.length - entry.items.length;
    props.onMessage(
      `${added} Question Pool Item ID${added === 1 ? "" : "s"} added to this Question Pool. Set its selection count, then save the Assessment.`,
    );
  }

  async function useSelection(selection: QuestionPickerSelection): Promise<void> {
    const currentIntent = intent();
    if (currentIntent === undefined || props.editorBusy()) return;
    props.setBusy(true);
    try {
      const rows = await resolveRows(props.repository, selection.questionIds);
      if (currentIntent.kind === "pool") addPoolRows(currentIntent.entryIndex, rows);
      else addCreateRows(rows);
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
        await props.repository.listQuestionPickerSources(props.courseId, props.mode.assessmentId),
      );
    } catch {
      setSources([{ kind: "library", label: "Question Library" }]);
      props.onMessage("The Question Library and direct Question ID entry are ready.");
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
