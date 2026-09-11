// Browser contract for the answer-free Assignment Workspace.

import type { AssignmentEditNumber } from "../../generated/api/AssignmentEditNumber";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { LocalDateAndTime } from "../../generated/api/LocalDateAndTime";
import type { LateWorkRule } from "../../generated/api/LateWorkRule";
import type { AccountTimeZone } from "../../generated/api/AccountTimeZone";
import type { AssignmentActivityRules } from "../../generated/api/AssignmentActivityRules";
import type { StudentFeedbackReleaseRule } from "../../generated/api/StudentFeedbackReleaseRule";

export type LiveAssignmentStatus = "unreleased" | "released" | "closed" | "archived";

export interface CourseAssignmentSummary {
  readonly reference: AssignmentReference;
  readonly title: string;
  readonly dueAt: LocalDateAndTime | null;
  /** Instructor zone governing this row's server-rendered local dueAt value. */
  readonly displayTimeZone: AccountTimeZone;
  readonly status: LiveAssignmentStatus;
  readonly editNumber: AssignmentEditNumber;
}

/** One Course-qualified Assignment due in the authenticated Instructor's rolling next-seven-days window. */
export interface DueSoonAssignmentSummary {
  readonly courseReference: CourseInstanceReference;
  readonly courseTitle: string;
  readonly assignmentReference: AssignmentReference;
  readonly assignmentTitle: string;
  readonly assignmentStatus: LiveAssignmentStatus;
  /** Stored UTC instant as Unix milliseconds; render it in displayTimeZone. */
  readonly dueAtMillis: number;
}

/** Bounded cross-Course Due Soon list with the Account-owned display zone. */
export interface DueSoonAssignments {
  readonly items: ReadonlyArray<DueSoonAssignmentSummary>;
  readonly nextCursor: null;
  readonly displayTimeZone: AccountTimeZone;
}

export interface SaveLiveAssignmentInlineInput {
  readonly title: string;
  readonly dueAt: LocalDateAndTime | null;
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
  /** Zone-free wall-clock deadline resolved by the server in the Instructor zone. */
  readonly dueAt: LocalDateAndTime | null;
  readonly lateWorkRule: LateWorkRule;
  readonly assignmentAttemptTimeLimitSeconds: number | null;
  readonly attemptLimit: number | null;
  readonly activityRules: AssignmentActivityRules;
  readonly studentFeedbackReleaseRule: StudentFeedbackReleaseRule;
  /** Informational only; the server interprets dueAt in this authenticated Instructor zone. */
  readonly displayTimeZone: AccountTimeZone;
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
  readonly dueAt: LocalDateAndTime | null;
  readonly lateWorkRule: LateWorkRule;
  readonly assignmentAttemptTimeLimitSeconds: number | null;
  readonly attemptLimit: number | null;
  readonly activityRules: AssignmentActivityRules;
  readonly studentFeedbackReleaseRule: StudentFeedbackReleaseRule;
}

export interface AssignmentReleaseValidation {
  readonly canRelease: boolean;
  readonly issues: ReadonlyArray<
    "noPublishedQuestions" | "questionUnavailable" | "timeLimitRequired"
  >;
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
  readonly listAssignmentsDueSoon: () => Promise<DueSoonAssignments>;
  readonly listCourseAssignments: (
    course: CourseInstanceReference,
  ) => Promise<ReadonlyArray<CourseAssignmentSummary>>;
  /** Saves the mutable title and due date shown on the Course Assignment list row. */
  readonly saveLiveAssignmentInline: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    input: SaveLiveAssignmentInlineInput,
    editNumber: AssignmentEditNumber,
  ) => Promise<CourseAssignmentSummary>;
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
