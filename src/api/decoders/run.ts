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
import type { LearnerAssignmentSummary } from "../../../generated/api/LearnerAssignmentSummary";
import type { AttemptProvenance } from "../../../generated/api/AttemptProvenance";
import type { AttemptStatus } from "../../../generated/api/AttemptStatus";
import type { AttemptTimerRecord } from "../../../generated/api/AttemptTimerRecord";
import type { CatalogProblemSummary } from "../../../generated/api/CatalogProblemSummary";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import type { GradebookSummaryRow } from "../../../generated/api/GradebookSummaryRow";
import type { IssuedAttemptCapabilityV1 } from "../../../generated/api/IssuedAttemptCapabilityV1";
import type { QuestionAttempt } from "../../../generated/api/QuestionAttempt";
import type { SourceArtifact } from "../../../generated/api/SourceArtifact";
import type { StudentAssignmentSummary } from "../../../generated/api/StudentAssignmentSummary";
import type { LearnerAssignmentProgress } from "../../../generated/api/LearnerAssignmentProgress";
import type { LearnerClassStatistics } from "../../../generated/api/LearnerClassStatistics";
import type { LearnerScoreState } from "../../../generated/api/LearnerScoreState";
import type { TaxonomyTerm } from "../../../generated/api/TaxonomyTerm";
import type {
  AuthSession,
  CursorPage,
  EnrollmentView,
  FeedbackReleaseResponse,
  NextIssuedAttempt,
  PrefetchedNextQuestion,
  RunSummaryOutcome,
  RunSummaryResponse,
  SignedOutResponse,
  SubmissionReceipt,
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
import { decodeLearnerAssignmentSummary } from "./catalog_course";
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
const LEARNER_CLASS_STATISTICS_WIRE_MINIMUM_COHORT_SIZE = 5;

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
  requireOnlyFields(record, path, [
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
  ]);
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
      "needs_manual_grading",
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

function decodeLearnerClassStatistics(value: unknown, path: string): LearnerClassStatistics {
  const record = decodeRecord(value, path);
  const statisticsState = decodeStringEnum(field(record, "state", path), `${path}.state`, [
    "insufficientEvidence",
    "available",
  ] as const satisfies ReadonlyArray<LearnerClassStatistics["state"]>);
  if (statisticsState === "insufficientEvidence") {
    requireOnlyFields(record, path, ["state"]);
    return { state: statisticsState };
  }
  requireOnlyFields(record, path, [
    "state",
    "completedLearnerCohortSize",
    "assignmentAverageScore",
  ]);
  const assignmentAverageScore = decodeFiniteNumber(
    field(record, "assignmentAverageScore", path),
    `${path}.assignmentAverageScore`,
  );
  if (assignmentAverageScore < 0 || assignmentAverageScore > 1) {
    throw new DecodeError(`${path}.assignmentAverageScore`, "a normalized score from 0 through 1");
  }
  const completedLearnerCohortSize = decodePositiveInteger(
    field(record, "completedLearnerCohortSize", path),
    `${path}.completedLearnerCohortSize`,
  );
  if (completedLearnerCohortSize < LEARNER_CLASS_STATISTICS_WIRE_MINIMUM_COHORT_SIZE) {
    throw new DecodeError(
      `${path}.completedLearnerCohortSize`,
      `a cohort of at least ${LEARNER_CLASS_STATISTICS_WIRE_MINIMUM_COHORT_SIZE} completed learners`,
    );
  }
  return {
    state: statisticsState,
    completedLearnerCohortSize,
    assignmentAverageScore,
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
 * Decodes the learner-only aggregate projection.  Unlike the storage summary,
 * this exact wire contract has no tenant or enrollment identifiers and sends no
 * score totals unless the current assignment settings permit their disclosure.
 */
export function decodeLearnerAssignmentProgress(
  value: unknown,
  path = "response",
): LearnerAssignmentProgress {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "scoreState",
    "currentScore",
    "bestScore",
    "latestScore",
    "completedRunCount",
    "totalQuestionAttempts",
    "lastActivityAt",
    "classStatistics",
  ]);
  const scoreState = decodeStringEnum(field(record, "scoreState", path), `${path}.scoreState`, [
    "noActivity",
    "withheld",
    "available",
  ] as const satisfies ReadonlyArray<LearnerScoreState>);
  const classStatistics = Object.prototype.hasOwnProperty.call(record, "classStatistics")
    ? decodeLearnerClassStatistics(record["classStatistics"], `${path}.classStatistics`)
    : undefined;
  const decoded = {
    scoreState,
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
    ...(classStatistics === undefined ? {} : { classStatistics }),
  } satisfies LearnerAssignmentProgress;
  const scores = [decoded.currentScore, decoded.bestScore, decoded.latestScore];
  if (decoded.scoreState !== "available" && scores.some((score) => score !== null)) {
    throw new DecodeError(`${path}.scoreState`, "no score totals before scores are available");
  }
  if (
    decoded.scoreState === "noActivity" &&
    (decoded.completedRunCount !== 0 || decoded.totalQuestionAttempts !== 0)
  ) {
    throw new DecodeError(`${path}.scoreState`, "no submitted activity counters");
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
  ]);
  return {
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
  } satisfies RunSummaryOutcome;
}

export function decodeRunSummaryResponse(value: unknown, path = "response"): RunSummaryResponse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["course", "run", "summary", "practiceAllowed", "outcomes"]);
  const outcomes = decodeRecord(field(record, "outcomes", path), `${path}.outcomes`);
  requireOnlyFields(outcomes, `${path}.outcomes`, ["items", "nextCursor"]);
  const decoded = {
    course: decodeCourseRouteData(field(record, "course", path), `${path}.course`),
    run: decodeStrictAssignmentRun(field(record, "run", path), `${path}.run`),
    summary: decodeLearnerAssignmentProgress(field(record, "summary", path), `${path}.summary`),
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

/**
 * Decodes the gradebook's deliberately compact, tenant-owned projection.
 *
 * This boundary is exact because browser gradebook consumers must not silently
 * accept history, question content, or a cross-tenant record appended by a
 * future server regression.
 */
export function decodeGradebookSummaryRow(value: unknown, path = "response"): GradebookSummaryRow {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "tenant",
    "courseId",
    "enrollmentId",
    "studentId",
    "learnerName",
    "assignmentId",
    "assignmentTitle",
    "summary",
  ]);
  const tenant = decodeIdentifier(field(record, "tenant", path), `${path}.tenant`);
  const summary = decodeStudentAssignmentSummary(field(record, "summary", path), `${path}.summary`);
  const summaryRecord = decodeRecord(field(record, "summary", path), `${path}.summary`);
  requireOnlyFields(summaryRecord, `${path}.summary`, [
    "tenant",
    "enrollment",
    "currentScore",
    "bestScore",
    "latestScore",
    "completedRunCount",
    "totalQuestionAttempts",
    "lastActivityAt",
  ]);
  const enrollmentId = decodeIdentifier(
    field(record, "enrollmentId", path),
    `${path}.enrollmentId`,
  );
  if (summary.tenant !== tenant) {
    throw new DecodeError(`${path}.summary.tenant`, "the row tenant");
  }
  if (summary.enrollment !== enrollmentId) {
    throw new DecodeError(`${path}.summary.enrollment`, "the row enrollmentId");
  }
  const decoded = {
    tenant,
    courseId: decodeIdentifier(field(record, "courseId", path), `${path}.courseId`),
    enrollmentId,
    studentId: decodeIdentifier(field(record, "studentId", path), `${path}.studentId`),
    learnerName: decodeNonemptyString(field(record, "learnerName", path), `${path}.learnerName`),
    assignmentId: decodeIdentifier(field(record, "assignmentId", path), `${path}.assignmentId`),
    assignmentTitle: decodeNonemptyString(
      field(record, "assignmentTitle", path),
      `${path}.assignmentTitle`,
    ),
    summary,
  } satisfies GradebookSummaryRow;
  return decoded;
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
    summary: decodeLearnerAssignmentProgress(field(record, "summary", path), `${path}.summary`),
  } satisfies EnrollmentView;
  return decoded;
}

export function decodeSubmissionReceipt(value: unknown, path = "response"): SubmissionReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["accepted", "attempt", "feedback", "nextIssued", "nextPending"]);
  const decoded = {
    accepted: decodeTrue(field(record, "accepted", path), `${path}.accepted`),
    attempt: decodeQuestionAttempt(field(record, "attempt", path), `${path}.attempt`),
    feedback: decodeNullable(
      field(record, "feedback", path),
      `${path}.feedback`,
      decodeDisclosedFeedback,
    ),
    nextIssued: decodeNullable(
      field(record, "nextIssued", path),
      `${path}.nextIssued`,
      decodeNextIssuedAttempt,
    ),
    nextPending: decodeBoolean(field(record, "nextPending", path), `${path}.nextPending`),
  } satisfies SubmissionReceipt;
  if (decoded.nextPending && decoded.nextIssued !== null) {
    throw new DecodeError(
      path,
      "a submission receipt with either an issued successor or a pending successor",
    );
  }
  return decoded;
}

export function decodeNextIssuedAttempt(value: unknown, path = "response"): NextIssuedAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "run",
    "questionVersion",
    "seed",
    "deadline",
    "assignmentPosition",
    "renderedQuestionSha256",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    run: decodeIdentifier(field(record, "run", path), `${path}.run`),
    questionVersion: decodeIdentifier(
      field(record, "questionVersion", path),
      `${path}.questionVersion`,
    ),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    deadline: decodeNullable(
      field(record, "deadline", path),
      `${path}.deadline`,
      decodeFiniteNumber,
    ),
    assignmentPosition: decodeNonnegativeInteger(
      field(record, "assignmentPosition", path),
      `${path}.assignmentPosition`,
    ),
    renderedQuestionSha256: decodeSha256(
      field(record, "renderedQuestionSha256", path),
      `${path}.renderedQuestionSha256`,
    ),
  } satisfies NextIssuedAttempt;
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

export function decodeLearnerAssignmentPage(
  value: unknown,
  path = "response",
): CursorPage<LearnerAssignmentSummary> {
  return decodeCursorPage(value, path, (item, itemPath) =>
    decodeLearnerAssignmentSummary(item, itemPath, true),
  );
}

export function decodeRunPage(value: unknown, path = "response"): CursorPage<AssignmentRun> {
  return decodeCursorPage(value, path, decodeStrictAssignmentRun);
}

export function decodeAttemptPage(value: unknown, path = "response"): CursorPage<QuestionAttempt> {
  return decodeCursorPage(value, path, decodeQuestionAttempt);
}

export function decodeGradebookPage(
  value: unknown,
  path = "response",
): CursorPage<GradebookSummaryRow> {
  return decodeCursorPage(value, path, decodeGradebookSummaryRow);
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
