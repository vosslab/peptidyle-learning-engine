// Browser contract for the answer-free Assessment Workspace.

import type { AssessmentEditNumber } from "../../generated/api/AssessmentEditNumber";
import type { AssessmentEntry } from "../../generated/api/AssessmentEntry";
import type { AssessmentReference } from "../../generated/api/AssessmentReference";
import type { BlueprintAssessmentReference } from "../../generated/api/BlueprintAssessmentReference";
import type { BlueprintCourseReference } from "../../generated/api/BlueprintCourseReference";
import type { BlueprintRevision } from "../../generated/api/BlueprintRevision";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { LocalDateAndTime } from "../../generated/api/LocalDateAndTime";
import type { LateWorkRule } from "../../generated/api/LateWorkRule";
import type { AccountTimeZone } from "../../generated/api/AccountTimeZone";
import type { AssessmentActivityRules } from "../../generated/api/AssessmentActivityRules";
import type { StudentFeedbackReleaseRule } from "../../generated/api/StudentFeedbackReleaseRule";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";

export type LiveAssessmentStatus = "unreleased" | "released" | "closed" | "archived";

export interface CourseAssessmentSummary {
  readonly reference: AssessmentReference;
  readonly title: string;
  readonly dueAt: LocalDateAndTime | null;
  /** Instructor zone governing this row's server-rendered local dueAt value. */
  readonly displayTimeZone: AccountTimeZone;
  readonly status: LiveAssessmentStatus;
  readonly editNumber: AssessmentEditNumber;
}

/** One Course-qualified Assessment due in the authenticated Instructor's rolling next-seven-days window. */
export interface DueSoonAssessmentSummary {
  readonly courseReference: CourseInstanceReference;
  readonly courseLongName: string;
  readonly assessmentReference: AssessmentReference;
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
  readonly reference: QuestionRevisionReference;
  readonly description: string;
}

/** One reusable Blueprint Assessment in this Course's exact pinned Blueprint Revision. */
export interface CourseAssessmentSourceChoice {
  readonly source: BlueprintAssessmentSource;
  readonly label: string;
}

/** Answer-free exact Question Revision pin shown in the Instructor workspace. */
export interface AuthoredAssessmentQuestion {
  readonly reference: QuestionRevisionReference;
  readonly description: string;
}

/** Immutable reusable Assessment provenance derived by PostgreSQL. */
export interface BlueprintAssessmentSource {
  readonly blueprint_revision: {
    readonly reference: BlueprintCourseReference;
    readonly revision: BlueprintRevision;
  };
  readonly blueprint_assessment_reference: BlueprintAssessmentReference;
}

export interface LiveAssessmentWorkspace {
  readonly reference: AssessmentReference;
  readonly editNumber: AssessmentEditNumber;
  readonly status: LiveAssessmentStatus;
  /** Database-derived reusable Blueprint source; requests never send it. */
  readonly source: BlueprintAssessmentSource;
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
  /** Stable Blueprint Assessment selected from the Course's pinned Blueprint Revision. */
  readonly blueprintAssessmentReference: BlueprintAssessmentReference;
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
    "noPublishedQuestions" | "questionUnavailable" | "timeLimitRequired"
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
  readonly listAssessmentsDueSoon: () => Promise<DueSoonAssessments>;
  readonly listCourseAssessments: (
    course: CourseInstanceReference,
  ) => Promise<ReadonlyArray<CourseAssessmentSummary>>;
  /** Saves the mutable title and due date shown on the Course Assessment list row. */
  readonly saveLiveAssessmentInline: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    input: SaveLiveAssessmentInlineInput,
    editNumber: AssessmentEditNumber,
  ) => Promise<CourseAssessmentSummary>;
  readonly listLiveAssessmentQuestionPicker: (
    course: CourseInstanceReference,
  ) => Promise<ReadonlyArray<AssessmentQuestionPickerEntry>>;
  /** Lists the Course-pinned reusable Blueprint Assessments available for creation. */
  readonly listCourseAssessmentSourceChoices: (
    course: CourseInstanceReference,
  ) => Promise<ReadonlyArray<CourseAssessmentSourceChoice>>;
  readonly createLiveAssessment: (
    course: CourseInstanceReference,
    input: CreateLiveAssessmentInput,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  readonly getLiveAssessmentWorkspace: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  readonly saveLiveAssessment: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    input: SaveLiveAssessmentInput,
    etag: string,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  /** Saves only Base Assessment Policy fields with an exact Assessment Edit Number. */
  readonly saveBaseAssessmentPolicy: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    input: SaveBaseAssessmentPolicyInput,
    etag: string,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  readonly validateLiveAssessmentRelease: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
  ) => Promise<AssessmentReleaseValidation>;
  readonly releaseLiveAssessment: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    etag: string,
  ) => Promise<LiveAssessmentWorkspaceResponse>;
  /** Reads the aggregate confirmation facts for a currently Released Assessment. */
  readonly getLiveAssessmentUnreleaseImpact: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
  ) => Promise<AssessmentUnreleaseImpact>;
  /** Restores a Released Assessment to Unreleased after exact-title confirmation. */
  readonly unreleaseLiveAssessment: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    confirmationTitle: string,
    etag: string,
  ) => Promise<{
    readonly result: UnreleasedLiveAssessment;
    readonly etag: string;
  }>;
}
