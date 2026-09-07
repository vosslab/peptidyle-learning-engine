// Strict decoders for the bounded M11 Student Assignment delivery surface.

import type {
  AssignmentStartDecision,
  LiveAssignmentAccess,
  LiveAssignmentAttempt,
  LiveNativePleSubmissionAcknowledgement,
} from "../assignment_attempt_issuance";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import {
  decodeAssignmentReference,
  decodeAssignmentTitle,
  field,
  requireOnlyFields,
} from "./shared";
import { decodeIssuedQuestionPresentation } from "./presentation_delivery";

const MAX_QUESTIONS = 25;
const MAX_INSTRUCTIONS_UNICODE_SCALARS = 10_000;

function decision(value: unknown, path: string): AssignmentStartDecision {
  const decoded = decodeString(value, path);
  if (
    decoded !== "may_start" &&
    decoded !== "not_yet_available" &&
    decoded !== "closed" &&
    decoded !== "attempt_limit_reached" &&
    decoded !== "late_work_refused"
  ) {
    throw new DecodeError(path, "a current Assignment Start Decision");
  }
  return decoded;
}

function instructions(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (decoded.includes("\0") || Array.from(decoded).length > MAX_INSTRUCTIONS_UNICODE_SCALARS) {
    throw new DecodeError(path, "bounded plain-text Assignment Instructions");
  }
  return decoded;
}

export function decodeLiveAssignmentAccess(
  value: unknown,
  path = "response",
): LiveAssignmentAccess {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["startDecision"]);
  return { startDecision: decision(field(record, "startDecision", path), `${path}.startDecision`) };
}

export function decodeLiveAssignmentAttempt(
  value: unknown,
  path = "response",
): LiveAssignmentAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assignment",
    "attemptNumber",
    "resumed",
    "title",
    "instructions",
    "questions",
  ]);
  const questions = decodeArray(
    field(record, "questions", path),
    `${path}.questions`,
    decodeIssuedQuestionPresentation,
  );
  if (questions.length === 0 || questions.length > MAX_QUESTIONS) {
    throw new DecodeError(`${path}.questions`, "one to twenty-five issued Questions");
  }
  return {
    assignment: decodeAssignmentReference(field(record, "assignment", path), `${path}.assignment`),
    attemptNumber: decodePositiveInteger(
      field(record, "attemptNumber", path),
      `${path}.attemptNumber`,
    ),
    resumed: decodeBoolean(field(record, "resumed", path), `${path}.resumed`),
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    instructions: instructions(field(record, "instructions", path), `${path}.instructions`),
    questions,
  };
}

/** Strictly decodes the M13 public acknowledgement, not the retained UUID attempt receipt. */
export function decodeLiveNativePleSubmissionAcknowledgement(
  value: unknown,
  path = "response",
): LiveNativePleSubmissionAcknowledgement {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["presentationNonce", "gradingState"]);
  const presentationNonce = decodeString(
    field(record, "presentationNonce", path),
    `${path}.presentationNonce`,
  );
  if (!/^[0-9a-f]{32}$/u.test(presentationNonce)) {
    throw new DecodeError(`${path}.presentationNonce`, "32 lowercase hexadecimal characters");
  }
  if (field(record, "gradingState", path) !== "pending") {
    throw new DecodeError(`${path}.gradingState`, "the pending grading state");
  }
  return { presentationNonce, gradingState: "pending" };
}
