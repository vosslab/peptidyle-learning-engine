// Attempt, run, enrollment, gradebook, and validation decoders.

import type { AssignmentEnrollment } from "../../../generated/api/AssignmentEnrollment";
import type { AssignmentRun } from "../../../generated/api/AssignmentRun";
import type { RunRouteReference } from "../../navigation/public_route";
import { parseRunReference } from "../../navigation/public_route";

function decodeRunReference(value: unknown, path: string): RunRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "an R- reference");
  const reference = parseRunReference(value);
  if (reference === null) throw new DecodeError(path, "an R- reference");
  return reference;
}
import type { StudentAssignmentLandingSummary } from "../../../generated/api/StudentAssignmentLandingSummary";
import type { AttemptProvenance } from "../../../generated/api/AttemptProvenance";
import type { AttemptStatus } from "../../../generated/api/AttemptStatus";
import type { AttemptTimerRecord } from "../../../generated/api/AttemptTimerRecord";
import type { CatalogProblemSummary } from "../../../generated/api/CatalogProblemSummary";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import type { IssuedAttemptCapabilityV1 } from "../../../generated/api/IssuedAttemptCapabilityV1";
import type { QuestionAttempt } from "../../../generated/api/QuestionAttempt";
import type { SourceArtifact } from "../../../generated/api/SourceArtifact";
import type { StudentAssignmentSummary } from "../../../generated/api/StudentAssignmentSummary";
import type { StudentAssignmentProgress } from "../../../generated/api/StudentAssignmentProgress";
import type { StudentClassStatistics } from "../../../generated/api/StudentClassStatistics";
import type { StudentScoreState } from "../../../generated/api/StudentScoreState";
import type { ScoringStatus } from "../../../generated/api/ScoringStatus";
import type { TaxonomyTerm } from "../../../generated/api/TaxonomyTerm";
import type {
  AuthSession,
  CursorPage,
  EnrollmentView,
  FeedbackReleaseResponse,
  StudentQuestionAttempt,
  PrefetchedNextQuestion,
  PoolSelection,
  RunSummaryOutcome,
  RunSummaryResponse,
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
  "tenant",
  "run",
  "problem",
  "questionVersion",
  "assignmentPosition",
  "seed",
  "parameterHash",
  "response",
  "status",
  "result",
  "timer",
  "provenance",
  "issuedCapability",
] as const;

function decodeAttemptTimer(value: unknown, path: string): AttemptTimerRecord {
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
  } satisfies AttemptTimerRecord;
  return decoded;
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
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    run: decodeIdentifier(field(record, "run", path), `${path}.run`),
    problem: decodeIdentifier(field(record, "problem", path), `${path}.problem`),
    questionVersion: decodeIdentifier(
      field(record, "questionVersion", path),
      `${path}.questionVersion`,
    ),
    assignmentPosition: decodeNonnegativeInteger(
      field(record, "assignmentPosition", path),
      `${path}.assignmentPosition`,
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
    timer: decodeAttemptTimer(field(record, "timer", path), `${path}.timer`),
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

export function decodeAssignmentRun(value: unknown, path = "response"): AssignmentRun {
  const record = decodeRecord(value, path);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeRunReference(field(record, "reference", path), `${path}.reference`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    enrollment: decodeIdentifier(field(record, "enrollment", path), `${path}.enrollment`),
    runNumber: decodePositiveInteger(field(record, "runNumber", path), `${path}.runNumber`),
    startedAt: decodeTimestamp(field(record, "startedAt", path), `${path}.startedAt`),
    completedAt: decodeNullable(
      field(record, "completedAt", path),
      `${path}.completedAt`,
      decodeTimestamp,
    ),
    score: decodeNullable(field(record, "score", path), `${path}.score`, decodeFiniteNumber),
    mode: decodeStringEnum(field(record, "mode", path), `${path}.mode`, ["assigned", "practice"]),
    variation: decodeStringEnum(field(record, "variation", path), `${path}.variation`, [
      "newSeeds",
      "selectedProblemVariants",
      "fullRegeneration",
    ]),
  } satisfies AssignmentRun;
  return decoded;
}

function decodeStrictAssignmentRun(value: unknown, path: string): AssignmentRun {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "reference",
    "tenant",
    "enrollment",
    "runNumber",
    "startedAt",
    "completedAt",
    "score",
    "mode",
    "variation",
  ]);
  return decodeAssignmentRun(value, path);
}

function decodeAssignmentEnrollment(value: unknown, path: string): AssignmentEnrollment {
  const record = decodeRecord(value, path);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    assignment: decodeIdentifier(field(record, "assignment", path), `${path}.assignment`),
    user: decodeIdentifier(field(record, "user", path), `${path}.user`),
    student: decodeIdentifier(field(record, "student", path), `${path}.student`),
    firstCompletedAt: decodeNullable(
      field(record, "firstCompletedAt", path),
      `${path}.firstCompletedAt`,
      decodeTimestamp,
    ),
    currentGradeRun: decodeNullable(
      field(record, "currentGradeRun", path),
      `${path}.currentGradeRun`,
      decodeIdentifier,
    ),
    bestGradeRun: decodeNullable(
      field(record, "bestGradeRun", path),
      `${path}.bestGradeRun`,
      decodeIdentifier,
    ),
  } satisfies AssignmentEnrollment;
  return decoded;
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

export function decodeStudentAssignmentSummary(
  value: unknown,
  path = "response",
): StudentAssignmentSummary {
  const record = decodeRecord(value, path);
  const decoded = {
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    enrollment: decodeIdentifier(field(record, "enrollment", path), `${path}.enrollment`),
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
    completedRunCount: decodeNonnegativeInteger(
      field(record, "completedRunCount", path),
      `${path}.completedRunCount`,
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
  } satisfies StudentAssignmentSummary;
  return decoded;
}

/**
 * Decodes the Student-only aggregate projection. Unlike the storage summary,
 * this exact wire contract has no tenant or enrollment identifiers and sends no
 * score totals unless the current assignment settings permit their disclosure.
 */
export function decodeStudentAssignmentProgress(
  value: unknown,
  path = "response",
): StudentAssignmentProgress {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "score_state",
    "scoring_status",
    "current_score",
    "best_score",
    "latest_score",
    "completed_run_count",
    "total_question_attempts",
    "last_activity_at",
    "class_statistics",
  ]);
  const scoreState = decodeStringEnum(field(record, "score_state", path), `${path}.score_state`, [
    "no_activity",
    "withheld",
    "available",
  ] as const satisfies ReadonlyArray<StudentScoreState>);
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
    completed_run_count: decodeNonnegativeInteger(
      field(record, "completed_run_count", path),
      `${path}.completed_run_count`,
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
  } satisfies StudentAssignmentProgress;
  const scores = [decoded.current_score, decoded.best_score, decoded.latest_score];
  if (decoded.score_state !== "available" && scores.some((score) => score !== null)) {
    throw new DecodeError(`${path}.score_state`, "no score totals before scores are available");
  }
  if (
    decoded.score_state === "no_activity" &&
    (decoded.completed_run_count !== 0 || decoded.total_question_attempts !== 0)
  ) {
    throw new DecodeError(`${path}.score_state`, "no submitted activity counters");
  }
  return decoded;
}

function decodeRunSummaryOutcome(value: unknown, path: string): RunSummaryOutcome {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "attempt",
    "assignmentPosition",
    "submittedAt",
    "response",
    "feedback",
    "scoringStatus",
  ]);
  const decoded = {
    attempt: decodeIdentifier(field(record, "attempt", path), `${path}.attempt`),
    assignmentPosition: decodeNonnegativeInteger(
      field(record, "assignmentPosition", path),
      `${path}.assignmentPosition`,
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
  } satisfies RunSummaryOutcome;
  if (
    decoded.scoringStatus !== "current" &&
    (decoded.feedback?.pointsEarned !== undefined || decoded.feedback?.pointsPossible !== undefined)
  ) {
    throw new DecodeError(`${path}.feedback`, "no numeric points while scoring is not current");
  }
  return decoded;
}

export function decodeRunSummaryResponse(value: unknown, path = "response"): RunSummaryResponse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["course", "run", "summary", "practiceAllowed", "outcomes"]);
  const outcomes = decodeRecord(field(record, "outcomes", path), `${path}.outcomes`);
  requireOnlyFields(outcomes, `${path}.outcomes`, ["items", "nextCursor"]);
  const decoded = {
    course: decodeCourseRouteData(field(record, "course", path), `${path}.course`),
    run: decodeStrictAssignmentRun(field(record, "run", path), `${path}.run`),
    summary: decodeStudentAssignmentProgress(field(record, "summary", path), `${path}.summary`),
    practiceAllowed: decodeBoolean(
      field(record, "practiceAllowed", path),
      `${path}.practiceAllowed`,
    ),
    outcomes: {
      items: decodeBoundedArray(
        field(outcomes, "items", `${path}.outcomes`),
        `${path}.outcomes.items`,
        MAX_CURSOR_PAGE_ITEMS,
        decodeRunSummaryOutcome,
      ),
      nextCursor: decodeNullable(
        field(outcomes, "nextCursor", `${path}.outcomes`),
        `${path}.outcomes.nextCursor`,
        decodeCursor,
      ),
    },
  } satisfies RunSummaryResponse;
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

export function decodeAuthSession(value: unknown, path = "response"): AuthSession {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["authenticated", "tenant", "user"]);
  const user = decodeRecord(field(record, "user", path), `${path}.user`);
  requireOnlyFields(user, `${path}.user`, ["id", "displayName", "roles"]);
  const decoded = {
    authenticated: decodeTrue(field(record, "authenticated", path), `${path}.authenticated`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    user: {
      id: decodeIdentifier(field(user, "id", `${path}.user`), `${path}.user.id`),
      displayName: decodeNonemptyString(
        field(user, "displayName", `${path}.user`),
        `${path}.user.displayName`,
      ),
      roles: decodeArray(field(user, "roles", `${path}.user`), `${path}.user.roles`, (role, p) =>
        decodeStringEnum(role, p, ["student", "instructor", "sysadmin"]),
      ),
    },
  } satisfies AuthSession;
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

export function decodeEnrollmentView(value: unknown, path = "response"): EnrollmentView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["enrollment", "summary"]);
  const decoded = {
    enrollment: decodeAssignmentEnrollment(field(record, "enrollment", path), `${path}.enrollment`),
    summary: decodeStudentAssignmentProgress(field(record, "summary", path), `${path}.summary`),
  } satisfies EnrollmentView;
  return decoded;
}

export function decodePrefetchedNextQuestion(
  value: unknown,
  path = "response",
): PrefetchedNextQuestion {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "predecessor",
    "run",
    "assignmentPosition",
    "questionVersion",
    "seed",
    "renderedQuestionSha256",
    "poolSelection",
    "envelope",
  ]);
  const decoded = {
    predecessor: decodeIdentifier(field(record, "predecessor", path), `${path}.predecessor`),
    run: decodeIdentifier(field(record, "run", path), `${path}.run`),
    assignmentPosition: decodeNonnegativeInteger(
      field(record, "assignmentPosition", path),
      `${path}.assignmentPosition`,
    ),
    questionVersion: decodeIdentifier(
      field(record, "questionVersion", path),
      `${path}.questionVersion`,
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
    decoded.envelope.version !== decoded.questionVersion ||
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

export function decodeRunPage(value: unknown, path = "response"): CursorPage<AssignmentRun> {
  return decodeCursorPage(value, path, decodeStrictAssignmentRun);
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
    question: decodeIdentifier(field(record, "question", path), `${path}.question`),
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
