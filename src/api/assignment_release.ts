// Browser contract for the answer-free Assignment Workspace.

import type { AssignmentEditNumber } from "../../generated/api/AssignmentEditNumber";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { CourseLocalDateAndTime } from "../../generated/api/CourseLocalDateAndTime";
import type { LateWorkRule } from "../../generated/api/LateWorkRule";

export type LiveAssignmentStatus = "unreleased" | "released" | "closed" | "archived";

export interface CourseAssignmentSummary {
  readonly reference: AssignmentReference;
  readonly title: string;
  readonly status: LiveAssignmentStatus;
  readonly editNumber: AssignmentEditNumber;
}

export interface AssignmentQuestionPickerEntry {
  readonly questionId: QuestionId;
  readonly description: string;
}

export type AuthoredAssignmentQuestion = AssignmentQuestionPickerEntry;

export interface LiveAssignmentWorkspace {
  readonly reference: AssignmentReference;
  readonly editNumber: AssignmentEditNumber;
  readonly status: LiveAssignmentStatus;
  readonly title: string;
  readonly instructions: string;
  /** Course-local wall-clock deadline; the server resolves it with the Course Term. */
  readonly dueAt: CourseLocalDateAndTime | null;
  readonly lateWorkRule: LateWorkRule;
  readonly questions: ReadonlyArray<AuthoredAssignmentQuestion>;
}

export interface RevisionedLiveAssignmentWorkspace {
  readonly workspace: LiveAssignmentWorkspace;
  /** Exact quoted strong ETag for the next save or release. */
  readonly etag: string;
}

export interface CreateLiveAssignmentInput {
  readonly title: string;
  readonly instructions: string;
}

export interface SaveLiveAssignmentInput extends CreateLiveAssignmentInput {
  readonly questionIds: ReadonlyArray<QuestionId>;
  readonly dueAt: CourseLocalDateAndTime | null;
  readonly lateWorkRule: LateWorkRule;
}

export interface AssignmentReleaseValidation {
  readonly canRelease: boolean;
  readonly issues: ReadonlyArray<"noPublishedQuestions" | "questionUnavailable">;
}

/** Deliberately answer-free Instructor preview; it creates no Student delivery. */
export interface AssignmentPreview {
  readonly title: string;
  readonly instructions: string;
  readonly questions: ReadonlyArray<AuthoredAssignmentQuestion>;
}

export interface ReleasedLiveAssignment {
  readonly reference: AssignmentReference;
  readonly revisionNumber: number;
}

/** Same-origin direct-Instructor Assignment Workspace boundary. */
export interface LiveAssignmentReleaseClient {
  readonly listCourseAssignments: (
    course: CourseInstanceReference,
  ) => Promise<ReadonlyArray<CourseAssignmentSummary>>;
  readonly listLiveAssignmentQuestionPicker: (
    course: CourseInstanceReference,
  ) => Promise<ReadonlyArray<AssignmentQuestionPickerEntry>>;
  readonly createLiveAssignment: (
    course: CourseInstanceReference,
    input: CreateLiveAssignmentInput,
  ) => Promise<RevisionedLiveAssignmentWorkspace>;
  readonly getLiveAssignmentWorkspace: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
  ) => Promise<RevisionedLiveAssignmentWorkspace>;
  readonly saveLiveAssignment: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    input: SaveLiveAssignmentInput,
    etag: string,
  ) => Promise<RevisionedLiveAssignmentWorkspace>;
  readonly validateLiveAssignmentRelease: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
  ) => Promise<AssignmentReleaseValidation>;
  readonly getLiveAssignmentPreview: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
  ) => Promise<AssignmentPreview>;
  readonly releaseLiveAssignment: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    etag: string,
  ) => Promise<ReleasedLiveAssignment>;
}
