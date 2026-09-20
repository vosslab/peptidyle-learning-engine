// Browser contract for the answer-free Assessment Workspace.

import type { AssessmentEditNumber } from "../../generated/api/AssessmentEditNumber";
import type { AssessmentEntry } from "../../generated/api/AssessmentEntry";
import type { AssessmentOrigin } from "../../generated/api/AssessmentOrigin";
import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { LocalDateAndTime } from "../../generated/api/LocalDateAndTime";
import type { LateWorkRule } from "../../generated/api/LateWorkRule";
import type { AccountTimeZone } from "../../generated/api/AccountTimeZone";
import type { AssessmentActivityRules } from "../../generated/api/AssessmentActivityRules";
import type { AssessmentType } from "../../generated/api/AssessmentType";
import type { StudentFeedbackReleaseRule } from "../../generated/api/StudentFeedbackReleaseRule";
import type { QuestionRevisionTuple } from "../../generated/api/QuestionRevisionTuple";
import type { BlueprintRevisionNumber } from "../../generated/api/BlueprintRevisionNumber";
import type { BlueprintCourseId } from "../../generated/api/BlueprintCourseId";
import type { BlueprintAssessmentDefaults } from "../../generated/api/BlueprintAssessmentDefaults";
import type { FixedQuestionAssessmentEntry } from "../../generated/api/FixedQuestionAssessmentEntry";
import type { QuestionPoolAssessmentEntry } from "../../generated/api/QuestionPoolAssessmentEntry";
import type { BloomClassificationView } from "../../generated/api/BloomClassificationView";

export type AssessmentBlueprintUpdateEntry =
  | ({ readonly kind: "fixedQuestion" } & Omit<FixedQuestionAssessmentEntry, "id" | "availability">)
  | ({ readonly kind: "questionPool" } & Omit<QuestionPoolAssessmentEntry, "id" | "availability">);

export interface AssessmentBlueprintUpdateContent {
  readonly assessmentType: AssessmentType;
  readonly title: string;
  readonly instructions: string;
  readonly defaults: BlueprintAssessmentDefaults;
  readonly entries: ReadonlyArray<AssessmentBlueprintUpdateEntry>;
}

export interface AssessmentBlueprintUpdateReview {
  readonly assessment: LiveAssessmentWorkspace;
  readonly sourceRevisionNumber: BlueprintRevisionNumber;
  readonly proposed: AssessmentBlueprintUpdateContent | null;
  readonly cannotApplyReason: "retainedSourceMissing" | "assessmentTypeMismatch" | null;
}

/** One current adopted Assessment correspondence; direct local Assessments are omitted. */
export interface CourseAssessmentBlueprintUpdateSummary {
  readonly assessmentId: AssessmentId;
  readonly title: string;
  readonly assessmentType: AssessmentType;
  readonly matchesSource: boolean;
  readonly cannotApplyReason: "retainedSourceMissing" | "assessmentTypeMismatch" | null;
}

/** Derived together from one parent Revision; adoptedRevisionNumber is the immutable creation pin. */
export interface CourseBlueprintUpdateReview {
  readonly blueprintCourseId: BlueprintCourseId;
  readonly adoptedRevisionNumber: BlueprintRevisionNumber;
  readonly sourceRevisionNumber: BlueprintRevisionNumber;
  readonly assessments: ReadonlyArray<CourseAssessmentBlueprintUpdateSummary>;
}

export interface ApplyAssessmentBlueprintUpdateInput {
  readonly expectedSourceRevisionNumber: BlueprintRevisionNumber;
  readonly expectedEditNumber: AssessmentEditNumber;
}

export type LiveAssessmentStatus = "unreleased" | "released" | "closed" | "archived";

export interface CourseAssessmentSummary {
  readonly id: AssessmentId;
  readonly assessmentType: AssessmentType;
  readonly title: string;
  readonly dueAt: LocalDateAndTime | null;
  /** Instructor zone governing this row's server-rendered local dueAt value. */
  readonly displayTimeZone: AccountTimeZone;
  readonly status: LiveAssessmentStatus;
  readonly editNumber: AssessmentEditNumber;
}

/** One Course-qualified Assessment due in the authenticated Instructor's rolling next-seven-days window. */
export interface DueSoonAssessmentSummary {
  readonly courseId: CourseInstanceId;
  readonly courseLongName: string;
  readonly assessmentId: AssessmentId;
  readonly assessmentType: AssessmentType;
  readonly assessmentTitle: string;
  readonly assessmentStatus: LiveAssessmentStatus;
  /** Stored UTC instant as Unix milliseconds; render it in displayTimeZone. */
  readonly dueAtMillis: number;
}

/** Bounded cross-Course Due Soon list with the Account-owned display zone. */
export interface DueSoonAssessments {
  readonly items: ReadonlyArray<DueSoonAssessmentSummary>;
  readonly nextCursor: null;
  readonly displayTimeZone: AccountTimeZone;
}

export interface SaveLiveAssessmentInlineInput {
  readonly title: string;
  readonly dueAt: LocalDateAndTime | null;
}

export interface AssessmentQuestionPickerEntry {
  /** Exact current Question Revision chosen by this picker row. */
  readonly questionRevisionTuple: QuestionRevisionTuple;
  readonly description: string;
  readonly bloom: BloomClassificationView | null;
}

/** Answer-free exact Question Revision pin shown in the Instructor workspace. */
export interface AuthoredAssessmentQuestion {
  readonly questionRevisionTuple: QuestionRevisionTuple;
  readonly description: string;
  readonly bloom: BloomClassificationView | null;
}

export interface LiveAssessmentWorkspace {
  readonly id: AssessmentId;
  readonly editNumber: AssessmentEditNumber;
  readonly status: LiveAssessmentStatus;
  /** Trusted server-derived direct or adopted origin; create requests never send it. */
  readonly origin: AssessmentOrigin;
  readonly assessmentType: AssessmentType;
  readonly title: string;
  readonly instructions: string;
  /** Zone-free wall-clock deadline resolved by the server in the Instructor zone. */
  readonly dueAt: LocalDateAndTime | null;
  readonly availableAt: LocalDateAndTime | null;
  readonly closesAt: LocalDateAndTime | null;
  readonly lateWorkRule: LateWorkRule;
  readonly assessmentAttemptTimeLimitSeconds: number | null;
  readonly attemptLimit: number | null;
  readonly activityRules: AssessmentActivityRules;
  readonly studentFeedbackReleaseRule: StudentFeedbackReleaseRule;
  /** Informational only; the server interprets dueAt in this authenticated Instructor zone. */
  readonly displayTimeZone: AccountTimeZone;
  /** Complete ordered normalized content for future Attempts. */
  readonly entries: ReadonlyArray<AssessmentEntry>;
  /** Answer-free exact pins for the existing preview and question list. */
  readonly questions: ReadonlyArray<AuthoredAssessmentQuestion>;
}

/** Current Assessment workspace plus the exact ETag required for its next mutation. */
export interface LiveAssessmentWorkspaceResponse {
  readonly workspace: LiveAssessmentWorkspace;
  /** Exact quoted strong ETag for the next save or release. */
  readonly etag: string;
}

export interface CreateLiveAssessmentInput {
  readonly assessmentType: AssessmentType;
  readonly title: string;
  readonly instructions: string;
}

export interface SaveLiveAssessmentInput {
  readonly title: string;
  readonly instructions: string;
  /** Ordered current Entry aggregates, each retaining exact Question Revision pins. */
  readonly entries: ReadonlyArray<AssessmentEntry>;
  readonly dueAt: LocalDateAndTime | null;
  readonly availableAt: LocalDateAndTime | null;
  readonly closesAt: LocalDateAndTime | null;
  readonly lateWorkRule: LateWorkRule;
  readonly assessmentAttemptTimeLimitSeconds: number | null;
  readonly attemptLimit: number | null;
  readonly activityRules: AssessmentActivityRules;
  readonly studentFeedbackReleaseRule: StudentFeedbackReleaseRule;
}

/** Closed policy-only write payload; it intentionally cannot carry title or Entries. */
export interface SaveBaseAssessmentPolicyInput {
  readonly instructions: string;
  readonly dueAt: LocalDateAndTime | null;
  readonly availableAt: LocalDateAndTime | null;
  readonly closesAt: LocalDateAndTime | null;
  readonly lateWorkRule: LateWorkRule;
  readonly assessmentAttemptTimeLimitSeconds: number | null;
  readonly attemptLimit: number | null;
  readonly activityRules: AssessmentActivityRules;
  readonly studentFeedbackReleaseRule: StudentFeedbackReleaseRule;
}

export interface AssessmentReleaseValidation {
  readonly canRelease: boolean;
  readonly issues: ReadonlyArray<
    | "noPublishedQuestions"
    | "questionCountExceeded"
    | "questionUnavailable"
    | "dueDateRequired"
    | "dueDateLessThan24HoursAhead"
    | "dueDateAfterCourseActiveUntil"
    | "availabilityAfterDueDate"
    | "dueDateAfterClose"
  >;
}

/** Released-only aggregate of Student Work removed by an Unrelease confirmation. */
export interface AssessmentUnreleaseImpact {
  /** The current title the Instructor must enter exactly. */
  readonly confirmationTitle: string;
  /** Current compare-and-swap value, retained for display and verification. */
  readonly editNumber: AssessmentEditNumber;
  readonly attemptCount: number;
  readonly submissionCount: number;
  readonly gradeCount: number;
}

/** Successful destructive transition, with only aggregate deletion facts. */
export interface UnreleasedLiveAssessment {
  readonly assessment: LiveAssessmentWorkspace;
  readonly deleted: AssessmentUnreleaseImpact;
}

/** Same-origin direct-Instructor Assessment Workspace boundary. */
export interface LiveAssessmentReleaseClient {
  readonly getCourseBlueprintUpdateReview: (
    course: CourseInstanceId,
  ) => Promise<CourseBlueprintUpdateReview>;
  readonly getAssessmentBlueprintUpdateReview: (
    course: CourseInstanceId,
    assessment: AssessmentId,
  ) => Promise<AssessmentBlueprintUpdateReview>;
  readonly applyAssessmentBlueprintUpdate: (
    course: CourseInstanceId,
    assessment: AssessmentId,
    input: ApplyAssessmentBlueprintUpdateInput,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  readonly listAssessmentsDueSoon: () => Promise<DueSoonAssessments>;
  readonly listCourseAssessments: (
    course: CourseInstanceId,
  ) => Promise<ReadonlyArray<CourseAssessmentSummary>>;
  /** Saves the mutable title and due date shown on the Course Assessment list row. */
  readonly saveLiveAssessmentInline: (
    course: CourseInstanceId,
    assessment: AssessmentId,
    input: SaveLiveAssessmentInlineInput,
    editNumber: AssessmentEditNumber,
  ) => Promise<CourseAssessmentSummary>;
  readonly listLiveAssessmentQuestionPicker: (
    course: CourseInstanceId,
  ) => Promise<ReadonlyArray<AssessmentQuestionPickerEntry>>;
  readonly createLiveAssessment: (
    course: CourseInstanceId,
    input: CreateLiveAssessmentInput,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  readonly getLiveAssessmentWorkspace: (
    course: CourseInstanceId,
    assessment: AssessmentId,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  readonly saveLiveAssessment: (
    course: CourseInstanceId,
    assessment: AssessmentId,
    input: SaveLiveAssessmentInput,
    etag: string,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  /** Saves only Base Assessment Policy fields with an exact Assessment Edit Number. */
  readonly saveBaseAssessmentPolicy: (
    course: CourseInstanceId,
    assessment: AssessmentId,
    input: SaveBaseAssessmentPolicyInput,
    etag: string,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  readonly validateLiveAssessmentRelease: (
    course: CourseInstanceId,
    assessment: AssessmentId,
  ) => Promise<AssessmentReleaseValidation>;
  readonly releaseLiveAssessment: (
    course: CourseInstanceId,
    assessment: AssessmentId,
    etag: string,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  /** Reads the aggregate confirmation facts for a currently Released Assessment. */
  readonly getLiveAssessmentUnreleaseImpact: (
    course: CourseInstanceId,
    assessment: AssessmentId,
  ) => Promise<AssessmentUnreleaseImpact>;
  /** Restores a Released Assessment to Unreleased after exact-title confirmation. */
  readonly unreleaseLiveAssessment: (
    course: CourseInstanceId,
    assessment: AssessmentId,
    confirmationTitle: string,
    etag: string,
  ) => Promise<{
    readonly result: UnreleasedLiveAssessment;
    readonly etag: string;
  }>;
}
