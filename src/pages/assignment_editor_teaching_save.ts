// assignment_editor_teaching_save.ts - one revisioned teaching-settings transaction.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { InstructorAssignmentTeachingSettingsLocal } from "../../generated/api/InstructorAssignmentTeachingSettingsLocal";

import type { AssignmentEditorDetail } from "../api/contracts";
import {
  AssignmentConflictError,
  AssignmentTeachingSettingsValidationError,
} from "../api/http_client";
import type { AssignmentEditorRepository } from "./assignment_editor_repository";

export interface AssignmentEditorTeachingSaveCallbacks {
  readonly onSaved: (saved: AssignmentEditorDetail) => void;
  readonly onValidation: (field: string, message: string) => void;
  readonly onConflictLatest: (latest: AssignmentEditorDetail) => void;
  readonly onMessage: (message: string) => void;
}

export async function saveAssignmentEditorTeachingSettings(
  repository: AssignmentEditorRepository,
  courseId: CourseId,
  assignmentId: AssignmentId,
  settings: InstructorAssignmentTeachingSettingsLocal,
  revision: string,
  refreshAssignments: () => Promise<void>,
  callbacks: AssignmentEditorTeachingSaveCallbacks,
): Promise<void> {
  try {
    const saved = await repository.saveTeachingSettings(courseId, assignmentId, settings, revision);
    await refreshAssignments();
    callbacks.onSaved(saved);
  } catch (error: unknown) {
    if (error instanceof AssignmentTeachingSettingsValidationError) {
      callbacks.onValidation(error.failure.field, error.failure.message);
      return;
    }
    if (error instanceof AssignmentConflictError) {
      try {
        callbacks.onConflictLatest(await repository.load(assignmentId));
      } catch {
        // Local teaching edits remain usable when a concurrent latest read is unavailable.
      }
    }
    callbacks.onMessage(
      error instanceof AssignmentConflictError
        ? "A newer teaching-settings revision was fetched. Your edits are still here; retry to save them, or adopt the latest teaching operations."
        : "Teaching operations were not saved. Correct the schedule or try again.",
    );
  }
}
