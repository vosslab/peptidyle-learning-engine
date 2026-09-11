// Strict decoders for the bounded Student Assignment delivery surface.

import type {
  AssignmentStartDecision,
  LiveAssignmentAccess,
  LiveAssignmentAttempt,
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
import { parseAssignmentAttemptReference } from "../../navigation/public_route";

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
  requireOnlyFields(record, path, ["startDecision", "activeAssignmentAttempt"]);
  const activeAssignmentAttemptValue = field(record, "activeAssignmentAttempt", path);
  let activeAssignmentAttempt = null;
  if (activeAssignmentAttemptValue !== null) {
    if (typeof activeAssignmentAttemptValue !== "string") {
      throw new DecodeError(
        `${path}.activeAssignmentAttempt`,
        "an Assignment Attempt R- reference or null",
      );
    }
    activeAssignmentAttempt = parseAssignmentAttemptReference(activeAssignmentAttemptValue);
    if (activeAssignmentAttempt === null) {
      throw new DecodeError(
        `${path}.activeAssignmentAttempt`,
        "an Assignment Attempt R- reference or null",
      );
    }
  }
  return {
    startDecision: decision(field(record, "startDecision", path), `${path}.startDecision`),
    activeAssignmentAttempt,
  };
}

export function decodeLiveAssignmentAttempt(
  value: unknown,
  path = "response",
): LiveAssignmentAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assignmentAttempt",
    "assignment",
    "attemptNumber",
    "resumed",
    "title",
    "instructions",
    "questions",
  ]);
  const assignmentAttemptValue = field(record, "assignmentAttempt", path);
  if (typeof assignmentAttemptValue !== "string")
    throw new DecodeError(`${path}.assignmentAttempt`, "an Assignment Attempt R- reference");
  const assignmentAttempt = parseAssignmentAttemptReference(assignmentAttemptValue);
  if (assignmentAttempt === null)
    throw new DecodeError(`${path}.assignmentAttempt`, "an Assignment Attempt R- reference");
  const questions = decodeArray(
    field(record, "questions", path),
    `${path}.questions`,
    decodeIssuedQuestionPresentation,
  );
  if (questions.length === 0 || questions.length > MAX_QUESTIONS) {
    throw new DecodeError(`${path}.questions`, "one to twenty-five issued Questions");
  }
  return {
    assignmentAttempt,
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
