// Assessment Attempt, gradebook, and validation decoders.

import type { AssessmentProgressRecord } from "../../../generated/api/AssessmentProgressRecord";
import type { AssessmentGrade } from "../../../generated/api/AssessmentGrade";
import type { AssessmentAttempt } from "../../../generated/api/AssessmentAttempt";
import type { AssessmentAttemptPolicySource } from "../../../generated/api/AssessmentAttemptPolicySource";
import type { BaseAssessmentPolicy } from "../../../generated/api/BaseAssessmentPolicy";
import type { AssessmentAttemptRouteReference } from "../../navigation/public_route";
import { parseAssessmentAttemptReference } from "../../navigation/public_route";

function decodeAssessmentAttemptReference(
  value: unknown,
  path: string,
): AssessmentAttemptRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "an Assessment Attempt UUID");
  const reference = parseAssessmentAttemptReference(value);
  if (reference === null) throw new DecodeError(path, "an Assessment Attempt UUID");
  return reference;
}
import type { StudentAssessmentLandingSummary } from "../../../generated/api/StudentAssessmentLandingSummary";
import type { QuestionAttemptState } from "../../../generated/api/QuestionAttemptState";
import type { QuestionAttemptTiming } from "../../../generated/api/QuestionAttemptTiming";
import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import type { IssuedAttemptCapability } from "../../../generated/api/IssuedAttemptCapability";
import type { StudentQuestionAttemptView } from "../../../generated/api/StudentQuestionAttemptView";
import type { QuestionResponse } from "../../../generated/api/QuestionResponse";
import type { StudentAssessmentProgress } from "../../../generated/api/StudentAssessmentProgress";
import type { AssessmentProgress } from "../../../generated/api/AssessmentProgress";
import type { StudentAssessmentGrade } from "../../../generated/api/StudentAssessmentGrade";
import type { ClassStatistics } from "../../../generated/api/ClassStatistics";
import type { AssessmentGradeScoreState } from "../../../generated/api/AssessmentGradeScoreState";
import type { AssessmentScoringState } from "../../../generated/api/AssessmentScoringState";
import type {
  AuthenticatedSession,
  CursorPage,
  StudentFeedbackReleaseResponse,
  StudentQuestionAttempt,
  QuestionPoolSelectionPosition,
  StudentIssuedQuestion,
  SignedOutResponse,
} from "../contracts";
import type { CapabilityViolation, QuestionAttemptTimingDecision } from "../../wasm/index";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeFiniteNumber,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
  decodeTrue,
} from "../decoder";
import {
  decodeCapability,
  decodeCursorPage,
  decodeIdentifier,
  decodeAssessmentTitle,
  decodeQuestionRevisionReference,
  decodeTimestamp,
  field,
  requireOnlyFields,
} from "./shared";
import { decodeStudentAssessmentLandingSummary } from "./question_library";
import { decodeStudentFeedbackReleaseRule } from "./assessment_policy";
import { decodeAssessmentActivityRules, decodeAssessmentInstructions } from "./assessment_release";
import { decodeGradingResult, decodeStudentResponse } from "./question_delivery";
import { decodeQuestionSummary, decodeCourseSummary } from "./question_library";

const ISSUED_ATTEMPT_CAPABILITIES = [
  "questionPresentation",
  "pleQuestionJsonPresentation",
  "webworkPresentation",
  "notApplicable",
] as const satisfies ReadonlyArray<IssuedAttemptCapability>;

// This fixed wire-contract minimum mirrors the server privacy floor. It only
// rejects unsafe API data; release-policy evaluation remains server-owned.
const CLASS_STATISTICS_WIRE_MINIMUM_COHORT_SIZE = 5;
const ASSIGNMENT_SCORING_STATES = [
  "current",
  "recalculating",
  "failed",
] as const satisfies ReadonlyArray<AssessmentScoringState>;

const QUESTION_ATTEMPT_FIELDS = [
  "id",
  "issuedQuestion",
  "finalizedResponse",
  "state",
  "timing",
  "issuedCapability",
] as const;

function decodeQuestionAttemptTiming(value: unknown, path: string): QuestionAttemptTiming {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["issuedAt", "deadline", "finalizedAt"]);
  const decoded = {
    issuedAt: decodeTimestamp(field(record, "issuedAt", path), `${path}.issuedAt`),
    deadline: decodeNullable(field(record, "deadline", path), `${path}.deadline`, decodeTimestamp),
    finalizedAt: decodeNullable(
      field(record, "finalizedAt", path),
      `${path}.finalizedAt`,
      decodeTimestamp,
    ),
  } satisfies QuestionAttemptTiming;
  return decoded;
}

export function decodeStudentIssuedQuestion(value: unknown, path: string): StudentIssuedQuestion {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "assessmentAttempt",
    "assessmentEntry",
    "assessmentContentEntryIndex",
    "issuedPosition",
    "reference",
    "questionStatisticsEligibility",
  ]);
  const reference = decodeRecord(field(record, "reference", path), `${path}.reference`);
  requireOnlyFields(reference, `${path}.reference`, ["questionId", "revisionNumber"]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    assessmentAttempt: decodeIdentifier(
      field(record, "assessmentAttempt", path),
      `${path}.assessmentAttempt`,
    ),
    assessmentEntry: decodeIdentifier(
      field(record, "assessmentEntry", path),
      `${path}.assessmentEntry`,
    ),
    assessmentContentEntryIndex: decodeNonnegativeInteger(
      field(record, "assessmentContentEntryIndex", path),
      `${path}.assessmentContentEntryIndex`,
    ),
    issuedPosition: decodePositiveInteger(
      field(record, "issuedPosition", path),
      `${path}.issuedPosition`,
    ),
    reference: decodeQuestionRevisionReference(reference, `${path}.reference`, true),
    questionStatisticsEligibility: decodeBoolean(
      field(record, "questionStatisticsEligibility", path),
      `${path}.questionStatisticsEligibility`,
    ),
  } satisfies StudentIssuedQuestion;
}

export function decodeStudentQuestionAttemptView(
  value: unknown,
  path = "response",
): StudentQuestionAttemptView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, QUESTION_ATTEMPT_FIELDS);
  const id = decodeIdentifier(field(record, "id", path), `${path}.id`);
  const decoded = {
    id,
    issuedQuestion: decodeIdentifier(
      field(record, "issuedQuestion", path),
      `${path}.issuedQuestion`,
    ),
    finalizedResponse: decodeNullable(
      field(record, "finalizedResponse", path),
      `${path}.finalizedResponse`,
      (response, responsePath) => decodeQuestionResponse(response, responsePath, id),
    ),
    state: decodeStringEnum(field(record, "state", path), `${path}.state`, [
      "open",
      "response_finalized",
      "closed_at_deadline",
    ] as const satisfies ReadonlyArray<QuestionAttemptState>),
    timing: decodeQuestionAttemptTiming(field(record, "timing", path), `${path}.timing`),
    issuedCapability: decodeStringEnum(
      field(record, "issuedCapability", path),
      `${path}.issuedCapability`,
      ISSUED_ATTEMPT_CAPABILITIES,
    ),
  } satisfies StudentQuestionAttemptView;
  return decoded;
}

function decodeQuestionResponse(
  value: unknown,
  path: string,
  expectedQuestionAttempt: string,
): QuestionResponse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "questionAttempt",
    "response",
    "finalizedAt",
    "gradingResult",
  ]);
  const questionAttempt = decodeIdentifier(
    field(record, "questionAttempt", path),
    `${path}.questionAttempt`,
  );
  if (questionAttempt !== expectedQuestionAttempt) {
    throw new DecodeError(`${path}.questionAttempt`, "the enclosing Question Attempt identity");
  }
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    questionAttempt,
    response: decodeStudentResponse(field(record, "response", path), `${path}.response`),
    finalizedAt: decodeTimestamp(field(record, "finalizedAt", path), `${path}.finalizedAt`),
    gradingResult: decodeNullable(
      field(record, "gradingResult", path),
      `${path}.gradingResult`,
      decodeGradingResult,
    ),
  } satisfies QuestionResponse;
}

export function decodeStudentQuestionAttempt(
  value: unknown,
  path = "response",
): StudentQuestionAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    ...QUESTION_ATTEMPT_FIELDS,
    "assessmentScoringState",
    "questionPoolSelectionPosition",
  ]);
  const { assessmentScoringState, questionPoolSelectionPosition, ...attempt } = record;
  const decoded = {
    ...decodeStudentQuestionAttemptView(attempt, path),
    assessmentScoringState: decodeStringEnum(
      assessmentScoringState,
      `${path}.assessmentScoringState`,
      ASSIGNMENT_SCORING_STATES,
    ),
    questionPoolSelectionPosition: decodeQuestionPoolSelectionPosition(
      questionPoolSelectionPosition,
      `${path}.questionPoolSelectionPosition`,
    ),
  } satisfies StudentQuestionAttempt;
  if (
    decoded.assessmentScoringState !== "current" &&
    decoded.finalizedResponse !== null &&
    decoded.finalizedResponse.gradingResult !== null
  ) {
    throw new DecodeError(
      `${path}.finalizedResponse.gradingResult`,
      "no numeric result while scoring is not current",
    );
  }
  return decoded;
}

function decodeQuestionPoolSelectionPosition(
  value: unknown,
  path: string,
): QuestionPoolSelectionPosition | null {
  if (value === null) return null;
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["selectedQuestionNumber", "selectedQuestionCount"]);
  const selectedQuestionNumber = decodePositiveInteger(
    field(record, "selectedQuestionNumber", path),
    `${path}.selectedQuestionNumber`,
  );
  const selectedQuestionCount = decodePositiveInteger(
    field(record, "selectedQuestionCount", path),
    `${path}.selectedQuestionCount`,
  );
  if (selectedQuestionNumber > selectedQuestionCount)
    throw new DecodeError(
      path,
      "a selected Question number no greater than its selected Question count",
    );
  return { selectedQuestionNumber, selectedQuestionCount };
}

export function decodeAssessmentAttempt(value: unknown, path = "response"): AssessmentAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "studentRecord",
    "assessment",
    "evidence",
    "attemptNumber",
    "startedAt",
    "submittedAt",
    "score",
  ]);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    studentRecord: decodeIdentifier(field(record, "studentRecord", path), `${path}.studentRecord`),
    assessment: decodeIdentifier(field(record, "assessment", path), `${path}.assessment`),
    evidence: decodeAssessmentAttemptEvidence(field(record, "evidence", path), `${path}.evidence`),
    attemptNumber: decodePositiveInteger(
      field(record, "attemptNumber", path),
      `${path}.attemptNumber`,
    ),
    startedAt: decodeTimestamp(field(record, "startedAt", path), `${path}.startedAt`),
    submittedAt: decodeNullable(
      field(record, "submittedAt", path),
      `${path}.submittedAt`,
      decodeTimestamp,
    ),
    score: decodeNullable(field(record, "score", path), `${path}.score`, decodeFiniteNumber),
  } satisfies AssessmentAttempt;
  return decoded;
}

function decodeAssessmentAttemptEvidence(
  value: unknown,
  path: string,
): AssessmentAttempt["evidence"] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "title",
    "instructions",
    "basePolicy",
    "activityRules",
    "studentFeedbackReleaseRule",
    "effectivePolicySources",
  ]);
  return {
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    instructions: decodeAssessmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
    basePolicy: decodeBaseAssessmentPolicy(field(record, "basePolicy", path), `${path}.basePolicy`),
    activityRules: decodeAssessmentActivityRules(
      field(record, "activityRules", path),
      `${path}.activityRules`,
    ),
    studentFeedbackReleaseRule: decodeStudentFeedbackReleaseRule(
      field(record, "studentFeedbackReleaseRule", path),
      `${path}.studentFeedbackReleaseRule`,
    ),
    effectivePolicySources: decodeAssessmentAttemptPolicySources(
      field(record, "effectivePolicySources", path),
      `${path}.effectivePolicySources`,
    ),
  };
}

function decodeBaseAssessmentPolicy(value: unknown, path: string): BaseAssessmentPolicy {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "available_at",
    "due_at",
    "closes_at",
    "assessment_attempt_time_limit_seconds",
    "attempt_limit",
    "late_work_rule",
  ]);
  return {
    available_at: decodeNullable(
      field(record, "available_at", path),
      `${path}.available_at`,
      decodeTimestamp,
    ),
    due_at: decodeNullable(field(record, "due_at", path), `${path}.due_at`, decodeTimestamp),
    closes_at: decodeNullable(
      field(record, "closes_at", path),
      `${path}.closes_at`,
      decodeTimestamp,
    ),
    assessment_attempt_time_limit_seconds: decodeNullable(
      field(record, "assessment_attempt_time_limit_seconds", path),
      `${path}.assessment_attempt_time_limit_seconds`,
      decodePositiveInteger,
    ),
    attempt_limit: decodeNullable(
      field(record, "attempt_limit", path),
      `${path}.attempt_limit`,
      decodePositiveInteger,
    ),
    late_work_rule: decodeStringEnum(
      field(record, "late_work_rule", path),
      `${path}.late_work_rule`,
      ["accept", "mark_late", "reject"],
    ),
  };
}

function decodeAssessmentAttemptPolicySource(
  value: unknown,
  path: string,
): AssessmentAttemptPolicySource {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "assessment") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind === "accommodation") {
    requireOnlyFields(record, path, ["kind", "accommodation"]);
    return {
      kind,
      accommodation: decodeIdentifier(
        field(record, "accommodation", path),
        `${path}.accommodation`,
      ),
    };
  }
  throw new DecodeError(`${path}.kind`, "a known Assessment Attempt policy source");
}

function decodeAssessmentAttemptPolicySources(
  value: unknown,
  path: string,
): AssessmentAttempt["evidence"]["effectivePolicySources"] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["schedule", "assessmentAttemptTimeLimit", "attemptLimit"]);
  return {
    schedule: decodeAssessmentAttemptPolicySource(
      field(record, "schedule", path),
      `${path}.schedule`,
    ),
    assessmentAttemptTimeLimit: decodeAssessmentAttemptPolicySource(
      field(record, "assessmentAttemptTimeLimit", path),
      `${path}.assessmentAttemptTimeLimit`,
    ),
    attemptLimit: decodeAssessmentAttemptPolicySource(
      field(record, "attemptLimit", path),
      `${path}.attemptLimit`,
    ),
  };
}

function decodeClassStatistics(value: unknown, path: string): ClassStatistics {
  const record = decodeRecord(value, path);
  const statisticsState = decodeStringEnum(field(record, "state", path), `${path}.state`, [
    "unavailable",
    "available",
  ] as const satisfies ReadonlyArray<ClassStatistics["state"]>);
  if (statisticsState === "unavailable") {
    requireOnlyFields(record, path, ["state"]);
    return { state: statisticsState };
  }
  requireOnlyFields(record, path, [
    "state",
    "completed_student_cohort_size",
    "assessment_average_score",
  ]);
  const assessmentAverageScore = decodeFiniteNumber(
    field(record, "assessment_average_score", path),
    `${path}.assessment_average_score`,
  );
  if (assessmentAverageScore < 0 || assessmentAverageScore > 1) {
    throw new DecodeError(
      `${path}.assessment_average_score`,
      "a normalized score from 0 through 1",
    );
  }
  const completedStudentCohortSize = decodePositiveInteger(
    field(record, "completed_student_cohort_size", path),
    `${path}.completed_student_cohort_size`,
  );
  if (completedStudentCohortSize < CLASS_STATISTICS_WIRE_MINIMUM_COHORT_SIZE) {
    throw new DecodeError(
      `${path}.completed_student_cohort_size`,
      `a cohort of at least ${CLASS_STATISTICS_WIRE_MINIMUM_COHORT_SIZE} completed Students`,
    );
  }
  return {
    state: statisticsState,
    completed_student_cohort_size: completedStudentCohortSize,
    assessment_average_score: assessmentAverageScore,
  };
}

export function decodeAssessmentProgressRecord(
  value: unknown,
  path = "response",
): AssessmentProgressRecord {
  const record = decodeRecord(value, path);
  const decoded = {
    student_record: decodeIdentifier(
      field(record, "student_record", path),
      `${path}.student_record`,
    ),
    assessment: decodeIdentifier(field(record, "assessment", path), `${path}.assessment`),
    completed_assessment_attempt_count: decodeNonnegativeInteger(
      field(record, "completed_assessment_attempt_count", path),
      `${path}.completed_assessment_attempt_count`,
    ),
    total_question_attempts: decodeNonnegativeInteger(
      field(record, "total_question_attempts", path),
      `${path}.total_question_attempts`,
    ),
    last_activity_at: decodeNullable(
      field(record, "last_activity_at", path),
      `${path}.last_activity_at`,
      decodeTimestamp,
    ),
  } satisfies AssessmentProgressRecord;
  return decoded;
}

export function decodeAssessmentGrade(value: unknown, path = "response"): AssessmentGrade {
  const record = decodeRecord(value, path);
  return {
    student_record: decodeIdentifier(
      field(record, "student_record", path),
      `${path}.student_record`,
    ),
    assessment: decodeIdentifier(field(record, "assessment", path), `${path}.assessment`),
    first_completed_at: decodeNullable(
      field(record, "first_completed_at", path),
      `${path}.first_completed_at`,
      decodeTimestamp,
    ),
    current_assessment_attempt: decodeNullable(
      field(record, "current_assessment_attempt", path),
      `${path}.current_assessment_attempt`,
      decodeIdentifier,
    ),
    current_score: decodeNullable(
      field(record, "current_score", path),
      `${path}.current_score`,
      decodeFiniteNumber,
    ),
    best_assessment_attempt: decodeNullable(
      field(record, "best_assessment_attempt", path),
      `${path}.best_assessment_attempt`,
      decodeIdentifier,
    ),
    best_score: decodeNullable(
      field(record, "best_score", path),
      `${path}.best_score`,
      decodeFiniteNumber,
    ),
    latest_assessment_attempt: decodeNullable(
      field(record, "latest_assessment_attempt", path),
      `${path}.latest_assessment_attempt`,
      decodeIdentifier,
    ),
    latest_score: decodeNullable(
      field(record, "latest_score", path),
      `${path}.latest_score`,
      decodeFiniteNumber,
    ),
  } satisfies AssessmentGrade;
}

/**
 * Decodes the Student-only disclosed Assessment Grade. Unlike the storage record,
 * this exact wire contract has no account or enrollment identifiers and sends no
 * score totals unless the current assessment settings permit their disclosure.
 */
function decodeStudentAssessmentGrade(value: unknown, path: string): StudentAssessmentGrade {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "score_state",
    "assessment_scoring_state",
    "current_score",
    "best_score",
    "latest_score",
    "class_statistics",
  ]);
  const scoreState = decodeStringEnum(field(record, "score_state", path), `${path}.score_state`, [
    "no_activity",
    "withheld",
    "available",
  ] as const satisfies ReadonlyArray<AssessmentGradeScoreState>);
  const classStatistics = Object.prototype.hasOwnProperty.call(record, "class_statistics")
    ? decodeClassStatistics(record["class_statistics"], `${path}.class_statistics`)
    : undefined;
  const decoded = {
    score_state: scoreState,
    assessment_scoring_state: decodeStringEnum(
      field(record, "assessment_scoring_state", path),
      `${path}.assessment_scoring_state`,
      ASSIGNMENT_SCORING_STATES,
    ),
    current_score: decodeNullable(
      field(record, "current_score", path),
      `${path}.current_score`,
      decodeFiniteNumber,
    ),
    best_score: decodeNullable(
      field(record, "best_score", path),
      `${path}.best_score`,
      decodeFiniteNumber,
    ),
    latest_score: decodeNullable(
      field(record, "latest_score", path),
      `${path}.latest_score`,
      decodeFiniteNumber,
    ),
    ...(classStatistics === undefined ? {} : { class_statistics: classStatistics }),
  } satisfies StudentAssessmentGrade;
  const scores = [decoded.current_score, decoded.best_score, decoded.latest_score];
  if (decoded.score_state !== "available" && scores.some((score) => score !== null)) {
    throw new DecodeError(`${path}.score_state`, "no score totals before scores are available");
  }
  return decoded;
}

function decodeAssessmentProgressActivity(value: unknown, path: string): AssessmentProgress {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "completed_assessment_attempt_count",
    "total_question_attempts",
    "last_activity_at",
  ]);
  return {
    completed_assessment_attempt_count: decodeNonnegativeInteger(
      field(record, "completed_assessment_attempt_count", path),
      `${path}.completed_assessment_attempt_count`,
    ),
    total_question_attempts: decodeNonnegativeInteger(
      field(record, "total_question_attempts", path),
      `${path}.total_question_attempts`,
    ),
    last_activity_at: decodeNullable(
      field(record, "last_activity_at", path),
      `${path}.last_activity_at`,
      decodeTimestamp,
    ),
  } satisfies AssessmentProgress;
}

/** Decodes the exact nested Student Assessment Progress response. */
export function decodeStudentAssessmentProgress(
  value: unknown,
  path = "response",
): StudentAssessmentProgress {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assessment_progress", "student_assessment_grade"]);
  const assessment_progress = decodeAssessmentProgressActivity(
    field(record, "assessment_progress", path),
    `${path}.assessment_progress`,
  );
  const student_assessment_grade = decodeStudentAssessmentGrade(
    field(record, "student_assessment_grade", path),
    `${path}.student_assessment_grade`,
  );
  if (
    student_assessment_grade.score_state === "no_activity" &&
    (assessment_progress.completed_assessment_attempt_count !== 0 ||
      assessment_progress.total_question_attempts !== 0)
  ) {
    throw new DecodeError(
      `${path}.student_assessment_grade.score_state`,
      "no submitted activity counters",
    );
  }
  return { assessment_progress, student_assessment_grade };
}

export function decodeStudentFeedbackReleaseResponse(
  value: unknown,
  path = "response",
): StudentFeedbackReleaseResponse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["released"]);
  return { released: decodeTrue(field(record, "released", path), `${path}.released`) };
}

export function decodeAuthenticatedSession(
  value: unknown,
  path = "response",
): AuthenticatedSession {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["authenticated", "account"]);
  const account = decodeRecord(field(record, "account", path), `${path}.account`);
  requireOnlyFields(account, `${path}.account`, ["id", "productRole"]);
  const decoded = {
    authenticated: decodeTrue(field(record, "authenticated", path), `${path}.authenticated`),
    account: {
      id: decodeIdentifier(field(account, "id", `${path}.account`), `${path}.account.id`),
      productRole: decodeStringEnum(
        field(account, "productRole", `${path}.account`),
        `${path}.account.productRole`,
        ["student", "instructor", "sysadmin"],
      ),
    },
  } satisfies AuthenticatedSession;
  return decoded;
}

export function decodeSignedOutResponse(value: unknown, path = "response"): SignedOutResponse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["authenticated"]);
  if (field(record, "authenticated", path) !== false) {
    throw new DecodeError(`${path}.authenticated`, "false");
  }
  return { authenticated: false };
}

export function decodeQuestionPage(value: unknown, path = "response"): CursorPage<QuestionSummary> {
  return decodeCursorPage(value, path, (item, itemPath) =>
    decodeQuestionSummary(item, itemPath, true),
  );
}

export function decodeCoursePage(value: unknown, path = "response"): CursorPage<CourseSummary> {
  return decodeCursorPage(value, path, decodeCourseSummary);
}

export function decodeStudentAssessmentPage(
  value: unknown,
  path = "response",
): CursorPage<StudentAssessmentLandingSummary> {
  return decodeCursorPage(value, path, (item, itemPath) =>
    decodeStudentAssessmentLandingSummary(item, itemPath, true),
  );
}

export function decodeAssessmentAttemptPage(
  value: unknown,
  path = "response",
): CursorPage<AssessmentAttempt> {
  return decodeCursorPage(value, path, decodeAssessmentAttempt);
}

export function decodeAttemptPage(
  value: unknown,
  path = "response",
): CursorPage<StudentQuestionAttempt> {
  return decodeCursorPage(value, path, decodeStudentQuestionAttempt);
}

export function decodeQuestionAttemptTimingDecision(
  value: unknown,
  path = "response",
): QuestionAttemptTimingDecision {
  return decodeStringEnum(value, path, [
    "untimed",
    "open",
    "gracePeriod",
    "finalizedOnTime",
    "finalizedWithinGrace",
    "timedOut",
  ]);
}

function decodeCapabilityViolation(value: unknown, path: string): CapabilityViolation {
  const record = decodeRecord(value, path);
  const decoded = {
    question: decodeQuestionRevisionReference(
      field(record, "question", path),
      `${path}.question`,
      true,
    ),
    capability: decodeCapability(field(record, "capability", path), `${path}.capability`),
  } satisfies CapabilityViolation;
  return decoded;
}

export function decodeCapabilityViolations(
  value: unknown,
  path = "response",
): ReadonlyArray<CapabilityViolation> {
  return decodeArray(value, path, decodeCapabilityViolation);
}
