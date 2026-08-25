// assignment_pool_preview_action.ts - save the visible pool definition before an authorized sample.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseReference } from "../../generated/api/CourseReference";

import type { PoolDrawPreview } from "../api/contracts";
import {
  assignmentEditorDraftFrom,
  assignmentInput,
  type AssignmentEditorDraft,
} from "./assignment_editor_model";
import type { AssignmentEditorRepository } from "./assignment_editor_repository";

export interface SavedPoolPreview {
  readonly draft: AssignmentEditorDraft;
  readonly preview: PoolDrawPreview;
}

export async function saveThenPreviewPoolDraw(
  repository: AssignmentEditorRepository,
  courseId: CourseId,
  assignmentId: AssignmentId,
  courseReference: CourseReference,
  draft: AssignmentEditorDraft,
  groupPosition: number,
): Promise<SavedPoolPreview> {
  const saved = await repository.save(
    courseId,
    assignmentId,
    assignmentInput(draft),
    draft.revision,
  );
  const savedDraft = assignmentEditorDraftFrom(saved);
  const group = savedDraft.entries.find(
    (entry) => entry.kind === "selectionGroup" && entry.position === groupPosition,
  );
  if (group === undefined || group.kind !== "selectionGroup") {
    throw new Error(
      "The saved assignment no longer contains that question pool. Reload to review it.",
    );
  }
  const preview = await repository.previewPoolDraw(
    courseReference,
    saved.reference,
    saved.revision,
    group.position,
  );
  return { draft: savedDraft, preview };
}
