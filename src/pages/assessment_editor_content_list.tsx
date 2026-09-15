// assessment_editor_content_list.tsx - ordered Assessment Entry controls.

import { For, type JSX } from "solid-js";

import { CopyableQuestionId } from "../components/copyable_question_id";
import {
  assessmentQuestionLabel,
  questionBackendLabel,
  type AssessmentQuestionRow,
  type AssessmentEditorEntry,
  type AssessmentEditorQuestionPoolAssessmentEntry,
} from "./assessment_editor_model";
import { AssessmentPoolEditor } from "./assessment_pool_editor";
import type { QuestionPoolPreview } from "../api/contracts";

export interface AssessmentEditorContentListProps {
  readonly entries: ReadonlyArray<AssessmentEditorEntry>;
  readonly createMode: boolean;
  readonly busy: boolean;
  readonly preview: QuestionPoolPreview | undefined;
  readonly resolveQuestionPoolItems: (
    questionIds: ReadonlyArray<string>,
  ) => Promise<ReadonlyArray<AssessmentQuestionRow>>;
  readonly onMove: (entryIndex: number, direction: -1 | 1) => void;
  readonly onRemoveFixed: (itemId: string) => void;
  readonly onPoolChange: (
    entryIndex: number,
    entry: AssessmentEditorQuestionPoolAssessmentEntry,
  ) => void;
  readonly onRemovePool: (entryIndex: number) => void;
  readonly onMessage: (message: string) => void;
  readonly onPreviewPool: (assessmentEntryId: string) => void;
  readonly onChooseQuestionPoolItems: (entryIndex: number, trigger: HTMLButtonElement) => void;
}

export function AssessmentEditorContentList(props: AssessmentEditorContentListProps): JSX.Element {
  function renderEntry(entry: AssessmentEditorEntry, entryIndex: number): JSX.Element {
    if (entry.kind === "questionPool") {
      return (
        <AssessmentPoolEditor
          entry={entry}
          entryIndex={entryIndex}
          entryCount={props.entries.length}
          resolveQuestionPoolItems={props.resolveQuestionPoolItems}
          onChange={(nextEntry) => props.onPoolChange(entryIndex, nextEntry)}
          onMove={(direction) => props.onMove(entryIndex, direction)}
          onRemove={() => props.onRemovePool(entryIndex)}
          onMessage={props.onMessage}
          preview={props.preview?.assessmentEntryId === entry.id ? props.preview : undefined}
          previewBusy={props.busy}
          onPreview={() => {
            if (entry.id !== undefined) props.onPreviewPool(entry.id);
          }}
          onChooseQuestionPoolItems={(trigger) =>
            props.onChooseQuestionPoolItems(entryIndex, trigger)
          }
        />
      );
    }
    return (
      <li class="assessment-editor-row">
        <h3>{entry.questionTitle}</h3>
        <p>
          <CopyableQuestionId
            questionTitle={entry.questionTitle}
            displayId={assessmentQuestionLabel(entry)}
          />{" "}
          {questionBackendLabel(entry.backend)}
        </p>
        <div class="assessment-editor-row-actions">
          <button
            class="quiet-action"
            type="button"
            disabled={entryIndex === 0}
            aria-label={`Move ${entry.questionTitle} earlier`}
            onClick={() => props.onMove(entryIndex, -1)}
          >
            &uarr;
          </button>
          <button
            class="quiet-action"
            type="button"
            disabled={entryIndex === props.entries.length - 1}
            aria-label={`Move ${entry.questionTitle} later`}
            onClick={() => props.onMove(entryIndex, 1)}
          >
            &darr;
          </button>
          <button
            class="quiet-action"
            disabled={props.createMode || props.busy}
            onClick={() => props.onRemoveFixed(entry.id)}
          >
            Remove
          </button>
        </div>
      </li>
    );
  }

  return (
    <ol class="assessment-editor-list">
      <For each={props.entries}>{(entry, index) => renderEntry(entry, index())}</For>
    </ol>
  );
}
