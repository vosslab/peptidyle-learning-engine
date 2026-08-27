// Closed learner submission acknowledgement decoder.

import type { RunCompletionStatus } from "../../../generated/api/RunCompletionStatus";
import type { ScoringStatus } from "../../../generated/api/ScoringStatus";
import type { LearnerSubmissionStatus, NextIssuedAttempt, SubmissionReceipt } from "../contracts";
import {
  DecodeError,
  decodeBoolean,
  decodeFiniteNumber,
  decodeNonnegativeInteger,
  decodeNullable,
  decodeRecord,
  decodeStringEnum,
  decodeTrue,
} from "../decoder";
import { decodeQuestionAttempt } from "./run";
import { decodeDisclosedFeedback } from "./question_delivery";
import { decodeIdentifier, decodeSha256, field, requireOnlyFields } from "./shared";

const SCORING_STATUSES = [
  "current",
  "recalculating",
  "failed",
] as const satisfies ReadonlyArray<ScoringStatus>;
const RUN_COMPLETION_STATUSES = [
  "inProgress",
  "completed",
] as const satisfies ReadonlyArray<RunCompletionStatus>;

export function decodeSubmissionReceipt(value: unknown, path = "response"): SubmissionReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "accepted",
    "attempt",
    "feedback",
    "scoringStatus",
    "runCompletionStatus",
    "nextIssued",
    "nextPending",
  ]);
  const decoded = {
    accepted: decodeTrue(field(record, "accepted", path), `${path}.accepted`),
    attempt: decodeQuestionAttempt(field(record, "attempt", path), `${path}.attempt`),
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
    runCompletionStatus: decodeStringEnum(
      field(record, "runCompletionStatus", path),
      `${path}.runCompletionStatus`,
      RUN_COMPLETION_STATUSES,
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
  if (
    decoded.runCompletionStatus === "completed" &&
    (decoded.nextIssued !== null || decoded.nextPending)
  ) {
    throw new DecodeError(path, "a completed run without successor delivery state");
  }
  if (decoded.scoringStatus !== "current" && decoded.attempt.result !== null) {
    throw new DecodeError(
      `${path}.attempt.result`,
      "no numeric result while scoring is not current",
    );
  }
  if (
    decoded.scoringStatus !== "current" &&
    (decoded.feedback?.pointsEarned !== undefined || decoded.feedback?.pointsPossible !== undefined)
  ) {
    throw new DecodeError(`${path}.feedback`, "no numeric points while scoring is not current");
  }
  return decoded;
}

/** Decodes only the three answer-free learner status alternatives frozen by the wire contract. */
export function decodeLearnerSubmissionStatus(
  value: unknown,
  path = "response",
): LearnerSubmissionStatus {
  const record = decodeRecord(value, path);
  const statusKind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "completed",
    "accepted_pending",
    "instructor_attention",
  ] as const);
  if (statusKind === "completed") {
    requireOnlyFields(record, path, [
      "kind",
      "accepted",
      "attempt",
      "feedback",
      "scoringStatus",
      "runCompletionStatus",
      "nextIssued",
      "nextPending",
    ]);
    const { kind: _kind, ...receipt } = record;
    return { kind: "completed", ...decodeSubmissionReceipt(receipt, path) };
  }
  requireOnlyFields(record, path, [
    "kind",
    "accepted",
    "attemptId",
    "automatedGradingStatus",
    "nextAction",
  ]);
  const accepted = decodeTrue(field(record, "accepted", path), `${path}.accepted`);
  const attemptId = decodeIdentifier(field(record, "attemptId", path), `${path}.attemptId`);
  const nextAction = decodeStringEnum(field(record, "nextAction", path), `${path}.nextAction`, [
    "check_status",
  ] as const);
  if (statusKind === "accepted_pending") {
    const automatedGradingStatus = decodeStringEnum(
      field(record, "automatedGradingStatus", path),
      `${path}.automatedGradingStatus`,
      ["pending"] as const,
    );
    return { kind: statusKind, accepted, attemptId, automatedGradingStatus, nextAction };
  }
  const automatedGradingStatus = decodeStringEnum(
    field(record, "automatedGradingStatus", path),
    `${path}.automatedGradingStatus`,
    ["instructor_attention"] as const,
  );
  return { kind: statusKind, accepted, attemptId, automatedGradingStatus, nextAction };
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
