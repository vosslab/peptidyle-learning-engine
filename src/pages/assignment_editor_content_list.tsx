// assignment_editor_content_list.tsx - ordered fixed-question and pool definition controls.

import { For, type JSX } from "solid-js";

import { CopyableQuestionId } from "../components/copyable_question_id";
import {
  questionBackendLabel,
  type AssignmentCatalogRow,
  type AssignmentEditorEntry,
  type AssignmentEditorSelectionGroupEntry,
} from "./assignment_editor_model";
import { AssignmentPoolEditor } from "./assignment_pool_editor";
import type { PoolDrawPreview } from "../api/contracts";

export interface AssignmentEditorContentListProps {
  readonly entries: ReadonlyArray<AssignmentEditorEntry>;
  readonly createMode: boolean;
  readonly busy: boolean;
  readonly preview: PoolDrawPreview | undefined;
  readonly resolveCandidates: (
    questionIds: ReadonlyArray<string>,
  ) => Promise<ReadonlyArray<AssignmentCatalogRow>>;
  readonly onMove: (entryIndex: number, direction: -1 | 1) => void;
  readonly onReplace: (itemId: string) => void;
  readonly onRemoveFixed: (itemId: string) => void;
  readonly onPoolChange: (entryIndex: number, entry: AssignmentEditorSelectionGroupEntry) => void;
  readonly onRemovePool: (entryIndex: number) => void;
  readonly onMessage: (message: string) => void;
  readonly onPreviewPool: (groupPosition: number) => void;
  readonly onChoosePoolCandidates: (entryIndex: number, trigger: HTMLButtonElement) => void;
}

export function AssignmentEditorContentList(props: AssignmentEditorContentListProps): JSX.Element {
  function renderEntry(entry: AssignmentEditorEntry, entryIndex: number): JSX.Element {
    if (entry.kind === "selectionGroup") {
      return (
        <AssignmentPoolEditor
          entry={entry}
          entryIndex={entryIndex}
          entryCount={props.entries.length}
          resolveCandidates={props.resolveCandidates}
          onChange={(nextEntry) => props.onPoolChange(entryIndex, nextEntry)}
          onMove={(direction) => props.onMove(entryIndex, direction)}
          onRemove={() => props.onRemovePool(entryIndex)}
          onMessage={props.onMessage}
          preview={props.preview?.groupPosition === entry.position ? props.preview : undefined}
          previewBusy={props.busy}
          onPreview={() => props.onPreviewPool(entry.position)}
          onChooseCandidates={(trigger) => props.onChoosePoolCandidates(entryIndex, trigger)}
        />
      );
    }
    return (
      <li class="assignment-editor-row">
        <h3>{entry.title}</h3>
        <p>
          <CopyableQuestionId displayId={entry.questionId} /> {questionBackendLabel(entry.backend)}
        </p>
        <div class="assignment-editor-row-actions">
          <button
            class="quiet-action"
            type="button"
            disabled={entryIndex === 0}
            aria-label={`Move ${entry.title} earlier`}
            onClick={() => props.onMove(entryIndex, -1)}
          >
            &uarr;
          </button>
          <button
            class="quiet-action"
            type="button"
            disabled={entryIndex === props.entries.length - 1}
            aria-label={`Move ${entry.title} later`}
            onClick={() => props.onMove(entryIndex, 1)}
          >
            &darr;
          </button>
          <button
            class="quiet-action"
            disabled={props.createMode}
            aria-label={`Replace ${entry.title}`}
            onClick={() => props.onReplace(entry.id)}
          >
            Replace
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
    <ol class="assignment-editor-list">
      <For each={props.entries}>{(entry, index) => renderEntry(entry, index())}</For>
    </ol>
  );
}
