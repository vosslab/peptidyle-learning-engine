// Assignment Attempt, gradebook, and validation decoders.

import type { AssignmentProgressRecord } from "../../../generated/api/AssignmentProgressRecord";
import type { AssignmentAttempt } from "../../../generated/api/AssignmentAttempt";
import type { AssignmentAttemptRouteReference } from "../../navigation/public_route";
import { parseAssignmentAttemptReference } from "../../navigation/public_route";

function decodeAssignmentAttemptReference(
  value: unknown,
  path: string,
): AssignmentAttemptRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "an Assignment Attempt R- reference");
  const reference = parseAssignmentAttemptReference(value);
  if (reference === null) throw new DecodeError(path, "an Assignment Attempt R- reference");
  return reference;
}
import type { StudentAssignmentLandingSummary } from "../../../generated/api/StudentAssignmentLandingSummary";
import type { QuestionAttemptState } from "../../../generated/api/QuestionAttemptState";
import type { QuestionAttemptTiming } from "../../../generated/api/QuestionAttemptTiming";
import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import type { IssuedAttemptCapability } from "../../../generated/api/IssuedAttemptCapability";
import type { StudentQuestionAttemptView } from "../../../generated/api/StudentQuestionAttemptView";
import type { QuestionSubmission } from "../../../generated/api/QuestionSubmission";
import type { AssignmentProgress } from "../../../generated/api/AssignmentProgress";
import type { StudentClassStatistics } from "../../../generated/api/StudentClassStatistics";
import type { AssignmentProgressScoreState } from "../../../generated/api/AssignmentProgressScoreState";
import type { AssignmentScoringState } from "../../../generated/api/AssignmentScoringState";
import type { QuestionClassification } from "../../../generated/api/QuestionClassification";
import type {
  AuthenticatedSession,
  CursorPage,
  StudentFeedbackReleaseResponse,
  StudentQuestionAttempt,
  PrefetchedNextQuestion,
  QuestionPoolSelectionPosition,
  StudentIssuedQuestion,
  AssignmentAttemptSummaryOutcome,
  AssignmentAttemptSummaryResponse,
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
  MAX_CURSOR_PAGE_ITEMS,
  decodeCapability,
  decodeBoundedArray,
  decodeCursor,
  decodeCursorPage,
  decodeIdentifier,
  decodeQuestionRevisionReference,
  decodeSha256,
  decodeQuestionClassification,
  decodeTimestamp,
  field,
  requireOnlyFields,
} from "./shared";
import { decodeStudentAssignmentLandingSummary } from "./question_library";
import {
  decodeGradingResult,
  decodeStudentFeedback,
  decodeStudentResponse,
} from "./question_delivery";
import { decodeIssuedQuestionPresentation } from "./presentation_delivery";
import {
  decodeAssignmentReference,
  decodeQuestionSummary,
  decodeCourseRouteView,
  decodeCourseSummary,
} from "./question_library";

const ISSUED_ATTEMPT_CAPABILITIES = [
  "questionPresentation",
  "pleQuestionJsonPresentation",
  "webworkPresentation",
  "qtiPresentation",
  "notApplicable",
] as const satisfies ReadonlyArray<IssuedAttemptCapability>;

// This fixed wire-contract minimum mirrors the server privacy floor. It only
// rejects unsafe API data; release-policy evaluation remains server-owned.
const STUDENT_CLASS_STATISTICS_WIRE_MINIMUM_COHORT_SIZE = 5;
const ASSIGNMENT_SCORING_STATES = [
  "current",
  "recalculating",
  "failed",
] as const satisfies ReadonlyArray<AssignmentScoringState>;

const QUESTION_ATTEMPT_FIELDS = [
  "id",
  "issuedQuestion",
  "seed",
  "submission",
  "state",
  "timing",
  "issuedCapability",
] as const;

function decodeQuestionAttemptTiming(value: unknown, path: string): QuestionAttemptTiming {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["issuedAt", "deadline", "submittedAt"]);
  const decoded = {
    issuedAt: decodeTimestamp(field(record, "issuedAt", path), `${path}.issuedAt`),
    deadline: decodeNullable(field(record, "deadline", path), `${path}.deadline`, decodeTimestamp),
    submittedAt: decodeNullable(
      field(record, "submittedAt", path),
      `${path}.submittedAt`,
      decodeTimestamp,
    ),
  } satisfies QuestionAttemptTiming;
  return decoded;
}

export function decodeStudentIssuedQuestion(value: unknown, path: string): StudentIssuedQuestion {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "assignmentAttempt",
    "assignmentEntry",
    "assignmentContentEntryIndex",
    "issuedPosition",
    "reference",
    "statisticsEligible",
  ]);
  const reference = decodeRecord(field(record, "reference", path), `${path}.reference`);
  requireOnlyFields(reference, `${path}.reference`, ["questionId", "revisionNumber"]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    assignmentAttempt: decodeIdentifier(
      field(record, "assignmentAttempt", path),
      `${path}.assignmentAttempt`,
    ),
    assignmentEntry: decodeIdentifier(
      field(record, "assignmentEntry", path),
      `${path}.assignmentEntry`,
    ),
    assignmentContentEntryIndex: decodeNonnegativeInteger(
      field(record, "assignmentContentEntryIndex", path),
      `${path}.assignmentContentEntryIndex`,
    ),
    issuedPosition: decodeNonnegativeInteger(
      field(record, "issuedPosition", path),
      `${path}.issuedPosition`,
    ),
    reference: decodeQuestionRevisionReference(reference, `${path}.reference`, true),
    statisticsEligible: decodeBoolean(
      field(record, "statisticsEligible", path),
      `${path}.statisticsEligible`,
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
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    submission: decodeNullable(
      field(record, "submission", path),
      `${path}.submission`,
      (submission, submissionPath) => decodeQuestionSubmission(submission, submissionPath, id),
    ),
    state: decodeStringEnum(field(record, "state", path), `${path}.state`, [
      "open",
      "submission_accepted",
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

function decodeQuestionSubmission(
  value: unknown,
  path: string,
  expectedQuestionAttempt: string,
): QuestionSubmission {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "questionAttempt",
    "response",
    "submittedAt",
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
    submittedAt: decodeTimestamp(field(record, "submittedAt", path), `${path}.submittedAt`),
    gradingResult: decodeNullable(
      field(record, "gradingResult", path),
      `${path}.gradingResult`,
      decodeGradingResult,
    ),
  } satisfies QuestionSubmission;
}

export function decodeStudentQuestionAttempt(
  value: unknown,
  path = "response",
): StudentQuestionAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    ...QUESTION_ATTEMPT_FIELDS,
    "assignmentScoringState",
    "questionPoolSelectionPosition",
  ]);
  const { assignmentScoringState, questionPoolSelectionPosition, ...attempt } = record;
  const decoded = {
    ...decodeStudentQuestionAttemptView(attempt, path),
    assignmentScoringState: decodeStringEnum(
      assignmentScoringState,
      `${path}.assignmentScoringState`,
      ASSIGNMENT_SCORING_STATES,
    ),
    questionPoolSelectionPosition: decodeQuestionPoolSelectionPosition(
      questionPoolSelectionPosition,
      `${path}.questionPoolSelectionPosition`,
    ),
  } satisfies StudentQuestionAttempt;
  if (
    decoded.assignmentScoringState !== "current" &&
    decoded.submission !== null &&
    decoded.submission.gradingResult !== null
  ) {
    throw new DecodeError(
      `${path}.submission.gradingResult`,
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
  requireOnlyFields(record, path, ["itemNumber", "itemCount"]);
  const itemNumber = decodePositiveInteger(field(record, "itemNumber", path), `${path}.itemNumber`);
  const itemCount = decodePositiveInteger(field(record, "itemCount", path), `${path}.itemCount`);
  if (itemNumber > itemCount)
    throw new DecodeError(path, "a pool item number no greater than its item count");
  return { itemNumber, itemCount };
}

export function decodeAssignmentAttempt(value: unknown, path = "response"): AssignmentAttempt {
  const record = decodeRecord(value, path);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeAssignmentAttemptReference(
      field(record, "reference", path),
      `${path}.reference`,
    ),
    studentRecord: decodeIdentifier(field(record, "studentRecord", path), `${path}.studentRecord`),
    assignment: decodeIdentifier(field(record, "assignment", path), `${path}.assignment`),
    assignmentRevision: decodeAssignmentRevisionReference(
      field(record, "assignmentRevision", path),
      `${path}.assignmentRevision`,
    ),
    attemptNumber: decodePositiveInteger(
      field(record, "attemptNumber", path),
      `${path}.attemptNumber`,
    ),
    startedAt: decodeTimestamp(field(record, "startedAt", path), `${path}.startedAt`),
    completedAt: decodeNullable(
      field(record, "completedAt", path),
      `${path}.completedAt`,
      decodeTimestamp,
    ),
    score: decodeNullable(field(record, "score", path), `${path}.score`, decodeFiniteNumber),
    questionPoolReuseRule: decodeStringEnum(
      field(record, "questionPoolReuseRule", path),
      `${path}.questionPoolReuseRule`,
      ["reuseSelection", "selectAgain"],
    ),
    questionVariationRule: decodeStringEnum(
      field(record, "questionVariationRule", path),
      `${path}.questionVariationRule`,
      ["reuseVariation", "newVariation"],
    ),
  } satisfies AssignmentAttempt;
  return decoded;
}

function decodeStrictAssignmentAttempt(value: unknown, path: string): AssignmentAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "reference",
    "studentRecord",
    "assignment",
    "assignmentRevision",
    "attemptNumber",
    "startedAt",
    "completedAt",
    "score",
    "questionPoolReuseRule",
    "questionVariationRule",
  ]);
  return decodeAssignmentAttempt(value, path);
}

function decodeAssignmentRevisionReference(
  value: unknown,
  path: string,
): AssignmentAttempt["assignmentRevision"] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assignment", "revision_number"]);
  const revisionNumber = decodeString(
    field(record, "revision_number", path),
    `${path}.revision_number`,
  );
  if (!/^[1-9][0-9]*$/u.test(revisionNumber))
    throw new DecodeError(`${path}.revision_number`, "a positive revision number");
  return {
    assignment: decodeAssignmentReference(field(record, "assignment", path), `${path}.assignment`),
    revision_number: revisionNumber,
  };
}

function decodeStudentClassStatistics(value: unknown, path: string): StudentClassStatistics {
  const record = decodeRecord(value, path);
  const statisticsState = decodeStringEnum(field(record, "state", path), `${path}.state`, [
    "insufficient_evidence",
    "available",
  ] as const satisfies ReadonlyArray<StudentClassStatistics["state"]>);
  if (statisticsState === "insufficient_evidence") {
    requireOnlyFields(record, path, ["state"]);
    return { state: statisticsState };
  }
  requireOnlyFields(record, path, [
    "state",
    "completed_student_cohort_size",
    "assignment_average_score",
  ]);
  const assignmentAverageScore = decodeFiniteNumber(
    field(record, "assignment_average_score", path),
    `${path}.assignment_average_score`,
  );
  if (assignmentAverageScore < 0 || assignmentAverageScore > 1) {
    throw new DecodeError(
      `${path}.assignment_average_score`,
      "a normalized score from 0 through 1",
    );
  }
  const completedStudentCohortSize = decodePositiveInteger(
    field(record, "completed_student_cohort_size", path),
    `${path}.completed_student_cohort_size`,
  );
  if (completedStudentCohortSize < STUDENT_CLASS_STATISTICS_WIRE_MINIMUM_COHORT_SIZE) {
    throw new DecodeError(
      `${path}.completed_student_cohort_size`,
      `a cohort of at least ${STUDENT_CLASS_STATISTICS_WIRE_MINIMUM_COHORT_SIZE} completed Students`,
    );
  }
  return {
    state: statisticsState,
    completed_student_cohort_size: completedStudentCohortSize,
    assignment_average_score: assignmentAverageScore,
  };
}

export function decodeAssignmentProgressRecord(
  value: unknown,
  path = "response",
): AssignmentProgressRecord {
  const record = decodeRecord(value, path);
  const decoded = {
    studentRecord: decodeIdentifier(field(record, "studentRecord", path), `${path}.studentRecord`),
    assignment: decodeIdentifier(field(record, "assignment", path), `${path}.assignment`),
    currentScore: decodeNullable(
      field(record, "currentScore", path),
      `${path}.currentScore`,
      decodeFiniteNumber,
    ),
    bestScore: decodeNullable(
      field(record, "bestScore", path),
      `${path}.bestScore`,
      decodeFiniteNumber,
    ),
    latestScore: decodeNullable(
      field(record, "latestScore", path),
      `${path}.latestScore`,
      decodeFiniteNumber,
    ),
    completedAssignmentAttemptCount: decodeNonnegativeInteger(
      field(record, "completedAssignmentAttemptCount", path),
      `${path}.completedAssignmentAttemptCount`,
    ),
    totalQuestionAttempts: decodeNonnegativeInteger(
      field(record, "totalQuestionAttempts", path),
      `${path}.totalQuestionAttempts`,
    ),
    lastActivityAt: decodeNullable(
      field(record, "lastActivityAt", path),
      `${path}.lastActivityAt`,
      decodeTimestamp,
    ),
  } satisfies AssignmentProgressRecord;
  return decoded;
}

/**
 * Decodes the Student-only Student Assignment Progress. Unlike the storage summary,
 * this exact wire contract has no account or enrollment identifiers and sends no
 * score totals unless the current assignment settings permit their disclosure.
 */
export function decodeAssignmentProgress(value: unknown, path = "response"): AssignmentProgress {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "score_state",
    "assignment_scoring_state",
    "current_score",
    "best_score",
    "latest_score",
    "completed_assignment_attempt_count",
    "total_question_attempts",
    "last_activity_at",
    "class_statistics",
  ]);
  const scoreState = decodeStringEnum(field(record, "score_state", path), `${path}.score_state`, [
    "no_activity",
    "withheld",
    "available",
  ] as const satisfies ReadonlyArray<AssignmentProgressScoreState>);
  const classStatistics = Object.prototype.hasOwnProperty.call(record, "class_statistics")
    ? decodeStudentClassStatistics(record["class_statistics"], `${path}.class_statistics`)
    : undefined;
  const decoded = {
    score_state: scoreState,
    assignment_scoring_state: decodeStringEnum(
      field(record, "assignment_scoring_state", path),
      `${path}.assignment_scoring_state`,
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
    completed_assignment_attempt_count: decodeNonnegativeInteger(
      field(record, "completed_assignment_attempt_count", path),
      `${path}.completed_assignment_attempt_count`,
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
    ...(classStatistics === undefined ? {} : { class_statistics: classStatistics }),
  } satisfies AssignmentProgress;
  const scores = [decoded.current_score, decoded.best_score, decoded.latest_score];
  if (decoded.score_state !== "available" && scores.some((score) => score !== null)) {
    throw new DecodeError(`${path}.score_state`, "no score totals before scores are available");
  }
  if (
    decoded.score_state === "no_activity" &&
    (decoded.completed_assignment_attempt_count !== 0 || decoded.total_question_attempts !== 0)
  ) {
    throw new DecodeError(`${path}.score_state`, "no submitted activity counters");
  }
  return decoded;
}

function decodeAssignmentAttemptSummaryOutcome(
  value: unknown,
  path: string,
): AssignmentAttemptSummaryOutcome {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "attempt",
    "issuedQuestion",
    "submittedAt",
    "response",
    "feedback",
    "assignmentScoringState",
  ]);
  const decoded = {
    attempt: decodeIdentifier(field(record, "attempt", path), `${path}.attempt`),
    issuedQuestion: decodeStudentIssuedQuestion(
      field(record, "issuedQuestion", path),
      `${path}.issuedQuestion`,
    ),
    submittedAt: decodeNullable(
      field(record, "submittedAt", path),
      `${path}.submittedAt`,
      decodeTimestamp,
    ),
    response: decodeNullable(
      field(record, "response", path),
      `${path}.response`,
      decodeStudentResponse,
    ),
    feedback: decodeNullable(
      field(record, "feedback", path),
      `${path}.feedback`,
      decodeStudentFeedback,
    ),
    assignmentScoringState: decodeStringEnum(
      field(record, "assignmentScoringState", path),
      `${path}.assignmentScoringState`,
      ASSIGNMENT_SCORING_STATES,
    ),
  } satisfies AssignmentAttemptSummaryOutcome;
  if (
    decoded.assignmentScoringState !== "current" &&
    (decoded.feedback?.pointsEarned !== undefined || decoded.feedback?.pointsPossible !== undefined)
  ) {
    throw new DecodeError(`${path}.feedback`, "no numeric points while scoring is not current");
  }
  return decoded;
}

export function decodeAssignmentAttemptSummaryResponse(
  value: unknown,
  path = "response",
): AssignmentAttemptSummaryResponse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["course", "assignmentAttempt", "summary", "outcomes"]);
  const outcomes = decodeRecord(field(record, "outcomes", path), `${path}.outcomes`);
  requireOnlyFields(outcomes, `${path}.outcomes`, ["items", "nextCursor"]);
  const decoded = {
    course: decodeCourseRouteView(field(record, "course", path), `${path}.course`),
    assignmentAttempt: decodeStrictAssignmentAttempt(
      field(record, "assignmentAttempt", path),
      `${path}.assignmentAttempt`,
    ),
    summary: decodeAssignmentProgress(field(record, "summary", path), `${path}.summary`),
    outcomes: {
      items: decodeBoundedArray(
        field(outcomes, "items", `${path}.outcomes`),
        `${path}.outcomes.items`,
        MAX_CURSOR_PAGE_ITEMS,
        decodeAssignmentAttemptSummaryOutcome,
      ),
      nextCursor: decodeNullable(
        field(outcomes, "nextCursor", `${path}.outcomes`),
        `${path}.outcomes.nextCursor`,
        decodeCursor,
      ),
    },
  } satisfies AssignmentAttemptSummaryResponse;
  return decoded;
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
  requireOnlyFields(account, `${path}.account`, ["id", "role"]);
  const decoded = {
    authenticated: decodeTrue(field(record, "authenticated", path), `${path}.authenticated`),
    account: {
      id: decodeIdentifier(field(account, "id", `${path}.account`), `${path}.account.id`),
      role: decodeStringEnum(field(account, "role", `${path}.account`), `${path}.account.role`, [
        "student",
        "instructor",
        "sysadmin",
      ]),
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

export function decodePrefetchedNextQuestion(
  value: unknown,
  path = "response",
): PrefetchedNextQuestion {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "predecessor",
    "issuedQuestion",
    "seed",
    "renderedQuestionSha256",
    "questionPoolSelectionPosition",
    "presentation",
  ]);
  const prefetchedQuestionPresentation = decodeIssuedQuestionPresentation(
    field(record, "presentation", path),
    `${path}.presentation`,
  );
  const decoded = {
    predecessor: decodeIdentifier(field(record, "predecessor", path), `${path}.predecessor`),
    issuedQuestion: decodeStudentIssuedQuestion(
      field(record, "issuedQuestion", path),
      `${path}.issuedQuestion`,
    ),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    renderedQuestionSha256: decodeSha256(
      field(record, "renderedQuestionSha256", path),
      `${path}.renderedQuestionSha256`,
    ),
    questionPoolSelectionPosition: decodeQuestionPoolSelectionPosition(
      field(record, "questionPoolSelectionPosition", path),
      `${path}.questionPoolSelectionPosition`,
    ),
    presentation: prefetchedQuestionPresentation,
  } satisfies PrefetchedNextQuestion;
  if (
    prefetchedQuestionPresentation.variation.questionRevision.revisionNumber !==
      decoded.issuedQuestion.reference.revisionNumber ||
    prefetchedQuestionPresentation.variation.questionRevision.questionId !==
      decoded.issuedQuestion.reference.questionId ||
    prefetchedQuestionPresentation.variation.seed !== decoded.seed
  ) {
    throw new DecodeError(path, "a Question Presentation bound to its descriptor");
  }
  return decoded;
}

export function decodeQuestionPage(value: unknown, path = "response"): CursorPage<QuestionSummary> {
  return decodeCursorPage(value, path, (item, itemPath) =>
    decodeQuestionSummary(item, itemPath, true),
  );
}

export function decodeQuestionClassificationPage(
  value: unknown,
  path = "response",
): CursorPage<QuestionClassification> {
  return decodeCursorPage(value, path, (item, itemPath) =>
    decodeQuestionClassification(item, itemPath, true),
  );
}

export function decodeCoursePage(value: unknown, path = "response"): CursorPage<CourseSummary> {
  return decodeCursorPage(value, path, decodeCourseSummary);
}

export function decodeStudentAssignmentPage(
  value: unknown,
  path = "response",
): CursorPage<StudentAssignmentLandingSummary> {
  return decodeCursorPage(value, path, (item, itemPath) =>
    decodeStudentAssignmentLandingSummary(item, itemPath, true),
  );
}

export function decodeAssignmentAttemptPage(
  value: unknown,
  path = "response",
): CursorPage<AssignmentAttempt> {
  return decodeCursorPage(value, path, decodeStrictAssignmentAttempt);
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
    "submittedOnTime",
    "submittedWithinGrace",
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
