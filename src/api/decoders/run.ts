// Assignment-activity, gradebook, and validation decoders.

import type { AssignmentProgressRecord } from "../../../generated/api/AssignmentProgressRecord";
import type { AssignmentAttempt } from "../../../generated/api/AssignmentAttempt";
import type { IssuedQuestion } from "../../../generated/api/IssuedQuestion";
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
import type { AttemptProvenance } from "../../../generated/api/AttemptProvenance";
import type { AttemptStatus } from "../../../generated/api/AttemptStatus";
import type { QuestionAttemptTiming } from "../../../generated/api/QuestionAttemptTiming";
import type { CatalogProblemSummary } from "../../../generated/api/CatalogProblemSummary";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import type { IssuedAttemptCapabilityV1 } from "../../../generated/api/IssuedAttemptCapabilityV1";
import type { QuestionAttempt } from "../../../generated/api/QuestionAttempt";
import type { SourceArtifact } from "../../../generated/api/SourceArtifact";
import type { AssignmentProgress } from "../../../generated/api/AssignmentProgress";
import type { StudentClassStatistics } from "../../../generated/api/StudentClassStatistics";
import type { AssignmentProgressScoreState } from "../../../generated/api/AssignmentProgressScoreState";
import type { ScoringStatus } from "../../../generated/api/ScoringStatus";
import type { TaxonomyTerm } from "../../../generated/api/TaxonomyTerm";
import type {
  AuthenticatedSession,
  CursorPage,
  FeedbackReleaseResponse,
  StudentQuestionAttempt,
  PrefetchedNextQuestion,
  PoolSelection,
  AssignmentAttemptSummaryOutcome,
  AssignmentAttemptSummaryResponse,
  SignedOutResponse,
} from "../contracts";
import type {
  CapabilityViolation,
  ResponseFormatReport,
  ResponseFormatViolation,
  TimerVerdict,
} from "../../wasm/index";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeFiniteNumber,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
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
  decodeQuestionVersionReference,
  decodeSha256,
  decodeTaxonomyTerm,
  decodeTimestamp,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import { decodeStudentAssignmentLandingSummary } from "./catalog_course";
import { decodeGeneratorReference, decodeSelectionCardinality } from "./question_model";
import {
  decodeAttemptResult,
  decodeDisclosedFeedback,
  decodeStudentResponse,
} from "./question_delivery";
import { decodeIssuedPresentationEnvelope } from "./presentation_delivery";
import {
  decodeCatalogProblemSummary,
  decodeCourseRouteData,
  decodeCourseSummary,
} from "./catalog_course";

const ISSUED_ATTEMPT_CAPABILITIES = [
  "presentationEnvelope",
  "flatPresentation",
  "webworkPresentation",
  "notApplicable",
] as const satisfies ReadonlyArray<IssuedAttemptCapabilityV1>;

// This fixed wire-contract minimum mirrors the server privacy floor. It only
// rejects unsafe API data; release-policy evaluation remains server-owned.
const STUDENT_CLASS_STATISTICS_WIRE_MINIMUM_COHORT_SIZE = 5;
const SCORING_STATUSES = [
  "current",
  "recalculating",
  "failed",
] as const satisfies ReadonlyArray<ScoringStatus>;

const QUESTION_ATTEMPT_FIELDS = [
  "id",
  "issuedQuestion",
  "seed",
  "parameterHash",
  "response",
  "status",
  "result",
  "timing",
  "provenance",
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

export function decodeIssuedQuestion(value: unknown, path: string): IssuedQuestion {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "assignmentAttempt",
    "assignmentEntry",
    "definitionEntryIndex",
    "issuedPosition",
    "reference",
    "statisticsEligible",
    "questionPoolEntry",
    "selectionSeed",
  ]);
  const reference = decodeRecord(field(record, "reference", path), `${path}.reference`);
  requireOnlyFields(reference, `${path}.reference`, ["questionId", "versionNumber"]);
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
    definitionEntryIndex: decodeNonnegativeInteger(
      field(record, "definitionEntryIndex", path),
      `${path}.definitionEntryIndex`,
    ),
    issuedPosition: decodeNonnegativeInteger(
      field(record, "issuedPosition", path),
      `${path}.issuedPosition`,
    ),
    reference: decodeQuestionVersionReference(reference, `${path}.reference`, true),
    statisticsEligible: decodeBoolean(
      field(record, "statisticsEligible", path),
      `${path}.statisticsEligible`,
    ),
    questionPoolEntry: decodeNullable(
      field(record, "questionPoolEntry", path),
      `${path}.questionPoolEntry`,
      decodeIdentifier,
    ),
    selectionSeed: decodeNullable(
      field(record, "selectionSeed", path),
      `${path}.selectionSeed`,
      decodeNonnegativeInteger,
    ),
  } satisfies IssuedQuestion;
}

function decodeImplementationVersion(
  value: unknown,
  path: string,
): { id: string; version: string } {
  return decodeGeneratorReference(value, path, true);
}

function decodeSourceArtifact(value: unknown, path: string): SourceArtifact {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["object", "sha256"]);
  const decoded = {
    object: decodeIdentifier(field(record, "object", path), `${path}.object`),
    sha256: decodeSha256(field(record, "sha256", path), `${path}.sha256`),
  } satisfies SourceArtifact;
  return decoded;
}

function decodeAttemptProvenance(value: unknown, path: string): AttemptProvenance {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "adapter",
    "renderer",
    "generator",
    "sourceArtifact",
    "assetObjects",
    "grading",
    "renderedQuestionSha256",
  ]);
  const decoded = {
    adapter: decodeImplementationVersion(field(record, "adapter", path), `${path}.adapter`),
    renderer: decodeNullable(
      field(record, "renderer", path),
      `${path}.renderer`,
      decodeImplementationVersion,
    ),
    generator: decodeNullable(
      field(record, "generator", path),
      `${path}.generator`,
      (generator, generatorPath) => decodeGeneratorReference(generator, generatorPath, true),
    ),
    sourceArtifact: decodeNullable(
      field(record, "sourceArtifact", path),
      `${path}.sourceArtifact`,
      decodeSourceArtifact,
    ),
    assetObjects: decodeArray(
      field(record, "assetObjects", path),
      `${path}.assetObjects`,
      decodeIdentifier,
    ),
    grading: decodeImplementationVersion(field(record, "grading", path), `${path}.grading`),
    renderedQuestionSha256: decodeSha256(
      field(record, "renderedQuestionSha256", path),
      `${path}.renderedQuestionSha256`,
    ),
  } satisfies AttemptProvenance;
  return decoded;
}

export function decodeQuestionAttempt(value: unknown, path = "response"): QuestionAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, QUESTION_ATTEMPT_FIELDS);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    issuedQuestion: decodeIdentifier(
      field(record, "issuedQuestion", path),
      `${path}.issuedQuestion`,
    ),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    parameterHash: decodeSha256(field(record, "parameterHash", path), `${path}.parameterHash`),
    response: decodeNullable(
      field(record, "response", path),
      `${path}.response`,
      decodeStudentResponse,
    ),
    status: decodeStringEnum(field(record, "status", path), `${path}.status`, [
      "in_progress",
      "submitted",
      "auto_submitted",
      "cleared",
      "exempt",
    ] as const satisfies ReadonlyArray<AttemptStatus>),
    result: decodeNullable(field(record, "result", path), `${path}.result`, decodeAttemptResult),
    timing: decodeQuestionAttemptTiming(field(record, "timing", path), `${path}.timing`),
    provenance: decodeAttemptProvenance(field(record, "provenance", path), `${path}.provenance`),
    issuedCapability: decodeStringEnum(
      field(record, "issuedCapability", path),
      `${path}.issuedCapability`,
      ISSUED_ATTEMPT_CAPABILITIES,
    ),
  } satisfies QuestionAttempt;
  return decoded;
}

export function decodeStudentQuestionAttempt(
  value: unknown,
  path = "response",
): StudentQuestionAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [...QUESTION_ATTEMPT_FIELDS, "scoringStatus", "poolSelection"]);
  const { scoringStatus, poolSelection, ...attempt } = record;
  const decoded = {
    ...decodeQuestionAttempt(attempt, path),
    scoringStatus: decodeStringEnum(scoringStatus, `${path}.scoringStatus`, SCORING_STATUSES),
    poolSelection: decodePoolSelection(poolSelection, `${path}.poolSelection`),
  } satisfies StudentQuestionAttempt;
  if (decoded.scoringStatus !== "current" && decoded.result !== null) {
    throw new DecodeError(`${path}.result`, "no numeric result while scoring is not current");
  }
  return decoded;
}

function decodePoolSelection(value: unknown, path: string): PoolSelection | null {
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
    variation: decodeStringEnum(field(record, "variation", path), `${path}.variation`, [
      "newSeeds",
      "selectedProblemVariants",
      "fullRegeneration",
    ]),
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
    "attemptNumber",
    "startedAt",
    "completedAt",
    "score",
    "variation",
  ]);
  return decodeAssignmentAttempt(value, path);
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
 * Decodes the Student-only aggregate projection. Unlike the storage summary,
 * this exact wire contract has no account or enrollment identifiers and sends no
 * score totals unless the current assignment settings permit their disclosure.
 */
export function decodeAssignmentProgress(value: unknown, path = "response"): AssignmentProgress {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "score_state",
    "scoring_status",
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
    scoring_status: decodeStringEnum(
      field(record, "scoring_status", path),
      `${path}.scoring_status`,
      SCORING_STATUSES,
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
    "scoringStatus",
  ]);
  const decoded = {
    attempt: decodeIdentifier(field(record, "attempt", path), `${path}.attempt`),
    issuedQuestion: decodeIssuedQuestion(
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
      decodeDisclosedFeedback,
    ),
    scoringStatus: decodeStringEnum(
      field(record, "scoringStatus", path),
      `${path}.scoringStatus`,
      SCORING_STATUSES,
    ),
  } satisfies AssignmentAttemptSummaryOutcome;
  if (
    decoded.scoringStatus !== "current" &&
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
    course: decodeCourseRouteData(field(record, "course", path), `${path}.course`),
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

export function decodeFeedbackReleaseResponse(
  value: unknown,
  path = "response",
): FeedbackReleaseResponse {
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
    "poolSelection",
    "envelope",
  ]);
  const decoded = {
    predecessor: decodeIdentifier(field(record, "predecessor", path), `${path}.predecessor`),
    issuedQuestion: decodeIssuedQuestion(
      field(record, "issuedQuestion", path),
      `${path}.issuedQuestion`,
    ),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    renderedQuestionSha256: decodeSha256(
      field(record, "renderedQuestionSha256", path),
      `${path}.renderedQuestionSha256`,
    ),
    poolSelection: decodePoolSelection(
      field(record, "poolSelection", path),
      `${path}.poolSelection`,
    ),
    envelope: decodeIssuedPresentationEnvelope(field(record, "envelope", path), `${path}.envelope`),
  } satisfies PrefetchedNextQuestion;
  if (
    decoded.envelope.questionVersion.versionNumber !==
      decoded.issuedQuestion.reference.versionNumber ||
    decoded.envelope.questionVersion.questionId !== decoded.issuedQuestion.reference.questionId ||
    decoded.envelope.seed !== decoded.seed
  ) {
    throw new DecodeError(path, "a prefetch envelope bound to its descriptor");
  }
  return decoded;
}

export function decodeCatalogPage(
  value: unknown,
  path = "response",
): CursorPage<CatalogProblemSummary> {
  return decodeCursorPage(value, path, (item, itemPath) =>
    decodeCatalogProblemSummary(item, itemPath, true),
  );
}

export function decodeTaxonomyPage(value: unknown, path = "response"): CursorPage<TaxonomyTerm> {
  return decodeCursorPage(value, path, (item, itemPath) =>
    decodeTaxonomyTerm(item, itemPath, true),
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

function decodeResponseFormatViolation(value: unknown, path: string): ResponseFormatViolation {
  const record = decodeRecord(value, path);
  const violation = kind(record, path);
  switch (violation) {
    case "responseKindMismatch":
    case "numericNotFinite":
    case "orderingItemsMismatch":
    case "missingUploadReference":
      return { kind: violation };
    case "selectionCount": {
      const decoded = {
        kind: violation,
        expected: decodeSelectionCardinality(field(record, "expected", path), `${path}.expected`),
        actual: decodeNonnegativeInteger(field(record, "actual", path), `${path}.actual`),
      } satisfies ResponseFormatViolation;
      return decoded;
    }
    case "duplicateChoice":
    case "unknownChoice": {
      const decoded = {
        kind: violation,
        choice: decodeNonemptyString(field(record, "choice", path), `${path}.choice`),
      } satisfies ResponseFormatViolation;
      return decoded;
    }
    case "textTooLong": {
      const decoded = {
        kind: violation,
        maxLength: decodeNonnegativeInteger(field(record, "maxLength", path), `${path}.maxLength`),
        actualLength: decodeNonnegativeInteger(
          field(record, "actualLength", path),
          `${path}.actualLength`,
        ),
      } satisfies ResponseFormatViolation;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known response-format violation");
  }
}

export function decodeResponseFormatReport(
  value: unknown,
  path = "response",
): ResponseFormatReport {
  const record = decodeRecord(value, path);
  const decoded = {
    violations: decodeArray(
      field(record, "violations", path),
      `${path}.violations`,
      decodeResponseFormatViolation,
    ),
  } satisfies ResponseFormatReport;
  return decoded;
}

export function decodeTimerVerdict(value: unknown, path = "response"): TimerVerdict {
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
    question: decodeQuestionVersionReference(
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
