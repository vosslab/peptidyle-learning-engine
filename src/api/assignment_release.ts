// Browser contract for the answer-free Assignment Workspace.

import type { AssignmentEditNumber } from "../../generated/api/AssignmentEditNumber";
import type { AssignmentEntry } from "../../generated/api/AssignmentEntry";
import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { BlueprintAssignmentReference } from "../../generated/api/BlueprintAssignmentReference";
import type { BlueprintCourseReference } from "../../generated/api/BlueprintCourseReference";
import type { BlueprintRevision } from "../../generated/api/BlueprintRevision";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { LocalDateAndTime } from "../../generated/api/LocalDateAndTime";
import type { LateWorkRule } from "../../generated/api/LateWorkRule";
import type { AccountTimeZone } from "../../generated/api/AccountTimeZone";
import type { AssignmentActivityRules } from "../../generated/api/AssignmentActivityRules";
import type { StudentFeedbackReleaseRule } from "../../generated/api/StudentFeedbackReleaseRule";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";

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
  readonly courseLongName: string;
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
  /** Exact current Question Revision chosen by this picker row. */
  readonly reference: QuestionRevisionReference;
  readonly description: string;
}

/** One reusable Blueprint Assignment in this Course's exact pinned Blueprint Revision. */
export interface CourseAssignmentSourceChoice {
  readonly source: BlueprintAssignmentSource;
  readonly label: string;
}

/** Answer-free exact Question Revision pin shown in the Instructor workspace. */
export interface AuthoredAssignmentQuestion {
  readonly reference: QuestionRevisionReference;
  readonly description: string;
}

/** Immutable reusable Assignment provenance derived by PostgreSQL. */
export interface BlueprintAssignmentSource {
  readonly blueprint_revision: {
    readonly reference: BlueprintCourseReference;
    readonly revision: BlueprintRevision;
  };
  readonly blueprint_assignment_reference: BlueprintAssignmentReference;
}

export interface LiveAssignmentWorkspace {
  readonly reference: AssignmentReference;
  readonly editNumber: AssignmentEditNumber;
  readonly status: LiveAssignmentStatus;
  /** Database-derived reusable Blueprint source; requests never send it. */
  readonly source: BlueprintAssignmentSource;
  readonly title: string;
  readonly instructions: string;
  /** Zone-free wall-clock deadline resolved by the server in the Instructor zone. */
  readonly dueAt: LocalDateAndTime | null;
  readonly availableAt: LocalDateAndTime | null;
  readonly closesAt: LocalDateAndTime | null;
  readonly lateWorkRule: LateWorkRule;
  readonly assignmentAttemptTimeLimitSeconds: number | null;
  readonly attemptLimit: number | null;
  readonly activityRules: AssignmentActivityRules;
  readonly studentFeedbackReleaseRule: StudentFeedbackReleaseRule;
  /** Informational only; the server interprets dueAt in this authenticated Instructor zone. */
  readonly displayTimeZone: AccountTimeZone;
  /** Complete ordered normalized content for future Attempts. */
  readonly entries: ReadonlyArray<AssignmentEntry>;
  /** Answer-free exact pins for the existing preview and question list. */
  readonly questions: ReadonlyArray<AuthoredAssignmentQuestion>;
}

/** Current Assignment workspace plus the exact ETag required for its next mutation. */
export interface LiveAssignmentWorkspaceResponse {
  readonly workspace: LiveAssignmentWorkspace;
  /** Exact quoted strong ETag for the next save or release. */
  readonly etag: string;
}

export interface CreateLiveAssignmentInput {
  /** Stable Blueprint Assignment selected from the Course's pinned Blueprint Revision. */
  readonly blueprintAssignmentReference: BlueprintAssignmentReference;
  readonly title: string;
  readonly instructions: string;
}

export interface SaveLiveAssignmentInput {
  readonly title: string;
  readonly instructions: string;
  /** Ordered current Entry aggregates, each retaining exact Question Revision pins. */
  readonly entries: ReadonlyArray<AssignmentEntry>;
  readonly dueAt: LocalDateAndTime | null;
  readonly availableAt: LocalDateAndTime | null;
  readonly closesAt: LocalDateAndTime | null;
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

/** Released-only aggregate of Student Work removed by an Unrelease confirmation. */
export interface AssignmentUnreleaseImpact {
  /** The current title the Instructor must enter exactly. */
  readonly confirmationTitle: string;
  /** Current compare-and-swap value, retained for display and verification. */
  readonly editNumber: AssignmentEditNumber;
  readonly attemptCount: number;
  readonly submissionCount: number;
  readonly gradeCount: number;
}

/** Successful destructive transition, with only aggregate deletion facts. */
export interface UnreleasedLiveAssignment {
  readonly assignment: LiveAssignmentWorkspace;
  readonly deleted: AssignmentUnreleaseImpact;
}

/** Deliberately answer-free Instructor preview; it creates no Student delivery. */
export interface AssignmentPreview {
  readonly title: string;
  readonly instructions: string;
  readonly questions: ReadonlyArray<AuthoredAssignmentQuestion>;
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
  /** Lists the Course-pinned reusable Blueprint Assignments available for creation. */
  readonly listCourseAssignmentSourceChoices: (
    course: CourseInstanceReference,
  ) => Promise<ReadonlyArray<CourseAssignmentSourceChoice>>;
  readonly createLiveAssignment: (
    course: CourseInstanceReference,
    input: CreateLiveAssignmentInput,
  ) => Promise<LiveAssignmentWorkspaceResponse>;
  readonly getLiveAssignmentWorkspace: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
  ) => Promise<LiveAssignmentWorkspaceResponse>;
  readonly saveLiveAssignment: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    input: SaveLiveAssignmentInput,
    etag: string,
  ) => Promise<LiveAssignmentWorkspaceResponse>;
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
  ) => Promise<LiveAssignmentWorkspaceResponse>;
  /** Reads the aggregate confirmation facts for a currently Released Assignment. */
  readonly getLiveAssignmentUnreleaseImpact: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
  ) => Promise<AssignmentUnreleaseImpact>;
  /** Restores a Released Assignment to Unreleased after exact-title confirmation. */
  readonly unreleaseLiveAssignment: (
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    confirmationTitle: string,
    etag: string,
  ) => Promise<{
    readonly result: UnreleasedLiveAssignment;
    readonly etag: string;
  }>;
}
