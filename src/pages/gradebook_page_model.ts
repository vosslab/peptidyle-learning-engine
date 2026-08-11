// gradebook_page_model.ts - transport seam for the compact gradebook screen.

import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { CourseId } from "../../generated/api/CourseId";
import type { EnrollmentId } from "../../generated/api/EnrollmentId";
import type { GradebookSummaryRow } from "../../generated/api/GradebookSummaryRow";
import type { ApiClient } from "../api/client";
import type { CursorPage } from "../api/contracts";

/** The gradebook's one deliberately compact initial request. */
export function loadGradebookPage(
  client: ApiClient,
  courseId: CourseId,
  cursor?: string,
): Promise<CursorPage<GradebookSummaryRow>> {
  if (cursor !== undefined) return client.listGradebook(courseId, cursor);
  return client.listGradebook(courseId);
}

/** Run history is opt-in; it never contributes to the initial gradebook load. */
export function loadGradebookRunHistory(
  client: ApiClient,
  enrollmentId: EnrollmentId,
  cursor?: string,
): Promise<CursorPage<AssignmentRun>> {
  return client.listRuns(enrollmentId, cursor);
}
