// Closed student submission acknowledgement decoder.

import type { AssignmentAttemptCompletion } from "../../../generated/api/AssignmentAttemptCompletion";
import type { AssignmentScoringState } from "../../../generated/api/AssignmentScoringState";
import type {
  GradedQuestionSubmissionReceipt,
  NextIssuedAttempt,
  QuestionSubmissionAcknowledgement,
  QuestionSubmissionReceipt,
} from "../contracts";
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
import {
  decodeIssuedQuestion,
  decodeStudentQuestionAttemptView,
} from "./assignment_attempt";
import { decodeStudentFeedback } from "./question_delivery";
import { decodeIdentifier, decodeSha256, field, requireOnlyFields } from "./shared";

const ASSIGNMENT_SCORING_STATES = [
  "current",
  "recalculating",
  "failed",
] as const satisfies ReadonlyArray<AssignmentScoringState>;
const RUN_COMPLETION_STATUSES = [
  "inProgress",
  "completed",
] as const satisfies ReadonlyArray<AssignmentAttemptCompletion>;

export function decodeGradedQuestionSubmissionReceipt(
  value: unknown,
  path = "response",
): GradedQuestionSubmissionReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "accepted",
    "attempt",
    "feedback",
    "assignmentScoringState",
    "assignmentAttemptCompletion",
    "nextIssued",
    "nextPending",
  ]);
  const decoded = {
    accepted: decodeTrue(field(record, "accepted", path), `${path}.accepted`),
    attempt: decodeStudentQuestionAttemptView(field(record, "attempt", path), `${path}.attempt`),
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
    assignmentAttemptCompletion: decodeStringEnum(
      field(record, "assignmentAttemptCompletion", path),
      `${path}.assignmentAttemptCompletion`,
      RUN_COMPLETION_STATUSES,
    ),
    nextIssued: decodeNullable(
      field(record, "nextIssued", path),
      `${path}.nextIssued`,
      decodeNextIssuedAttempt,
    ),
    nextPending: decodeBoolean(field(record, "nextPending", path), `${path}.nextPending`),
  } satisfies Omit<GradedQuestionSubmissionReceipt, "attemptId">;
  const receipt = {
    ...decoded,
    attemptId: decoded.attempt.id,
  } satisfies GradedQuestionSubmissionReceipt;
  if (receipt.nextPending && receipt.nextIssued !== null) {
    throw new DecodeError(
      path,
      "a submission receipt with either an issued successor or a pending successor",
    );
  }
  if (
    receipt.assignmentAttemptCompletion === "completed" &&
    (receipt.nextIssued !== null || receipt.nextPending)
  ) {
    throw new DecodeError(path, "a completed run without successor delivery state");
  }
  if (
    receipt.assignmentScoringState !== "current" &&
    receipt.attempt.submission !== null &&
    receipt.attempt.submission.gradingResult !== null
  ) {
    throw new DecodeError(
      `${path}.attempt.submission.gradingResult`,
      "no numeric result while scoring is not current",
    );
  }
  if (
    receipt.assignmentScoringState !== "current" &&
    (receipt.feedback?.pointsEarned !== undefined || receipt.feedback?.pointsPossible !== undefined)
  ) {
    throw new DecodeError(`${path}.feedback`, "no numeric points while scoring is not current");
  }
  return receipt;
}

/** Decodes the accepted Question Submission Receipt and separate grading state. */
export function decodeQuestionSubmissionAcknowledgement(
  value: unknown,
  path = "response",
): QuestionSubmissionAcknowledgement {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["receipt", "gradingState", "nextAction"]);
  const gradingState = decodeStringEnum(
    field(record, "gradingState", path),
    `${path}.gradingState`,
    ["pending", "graded", "instructorAttention"] as const,
  );
  const receiptValue = field(record, "receipt", path);
  if (gradingState === "graded") {
    if ("nextAction" in record) {
      throw new DecodeError(path, "a graded acknowledgement without a next action");
    }
    return {
      receipt: decodeGradedQuestionSubmissionReceipt(receiptValue, `${path}.receipt`),
      gradingState,
    };
  }
  const receiptRecord = decodeRecord(receiptValue, `${path}.receipt`);
  requireOnlyFields(receiptRecord, `${path}.receipt`, ["accepted", "attemptId"]);
  const receipt = {
    accepted: decodeTrue(
      field(receiptRecord, "accepted", `${path}.receipt`),
      `${path}.receipt.accepted`,
    ),
    attemptId: decodeIdentifier(
      field(receiptRecord, "attemptId", `${path}.receipt`),
      `${path}.receipt.attemptId`,
    ),
  } satisfies QuestionSubmissionReceipt;
  const nextAction = decodeStringEnum(field(record, "nextAction", path), `${path}.nextAction`, [
    "check_status",
  ] as const);
  return { receipt, gradingState, nextAction };
}

export function decodeNextIssuedAttempt(value: unknown, path = "response"): NextIssuedAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "issuedQuestion",
    "seed",
    "deadline",
    "renderedQuestionSha256",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    issuedQuestion: decodeIssuedQuestion(
      field(record, "issuedQuestion", path),
      `${path}.issuedQuestion`,
    ),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    deadline: decodeNullable(
      field(record, "deadline", path),
      `${path}.deadline`,
      decodeFiniteNumber,
    ),
    renderedQuestionSha256: decodeSha256(
      field(record, "renderedQuestionSha256", path),
      `${path}.renderedQuestionSha256`,
    ),
  } satisfies NextIssuedAttempt;
}
