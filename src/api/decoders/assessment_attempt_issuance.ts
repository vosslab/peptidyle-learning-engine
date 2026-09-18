// Strict decoders for the bounded Student Assessment delivery surface.

import type { LiveAssessmentAccess, LiveAssessmentAttempt } from "../assessment_attempt_issuance";
import { ASSESSMENT_TYPE_VALUES } from "../../../generated/api/AssessmentType";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeFiniteNumber,
  decodeNonnegativeInteger,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import {
  decodeAssessmentId,
  decodeAssessmentTitle,
  field,
  requireOnlyFields,
} from "./shared";
import { decodeIssuedQuestionPresentation } from "./presentation_delivery";
import { parseAssessmentAttemptReference } from "../../navigation/public_route";
import { decodeStudentAssessmentDecision } from "./student_assessment_decision";

const MAX_QUESTIONS = 25;
const MAX_INSTRUCTIONS_UNICODE_SCALARS = 10_000;

function instructions(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (decoded.includes("\0") || Array.from(decoded).length > MAX_INSTRUCTIONS_UNICODE_SCALARS) {
    throw new DecodeError(path, "bounded plain-text Assessment Instructions");
  }
  return decoded;
}

function nonnegativeFiniteNumber(value: unknown, path: string): number {
  const decoded = decodeFiniteNumber(value, path);
  if (decoded < 0) throw new DecodeError(path, "a nonnegative finite number");
  return decoded;
}

function priorAttempt(
  value: unknown,
  path: string,
): import("../assessment_attempt_issuance").LiveAssessmentPreviousAttempt {
  const record = decodeRecord(value, path);
  const allowed = ["assessmentAttempt", "attemptNumber", "state", "score"];
  requireOnlyFields(record, path, allowed);
  const reference = field(record, "assessmentAttempt", path);
  if (typeof reference !== "string") {
    throw new DecodeError(`${path}.assessmentAttempt`, "an Assessment Attempt UUID");
  }
  const assessmentAttempt = parseAssessmentAttemptReference(reference);
  if (assessmentAttempt === null) {
    throw new DecodeError(`${path}.assessmentAttempt`, "an Assessment Attempt UUID");
  }
  const state = decodeString(field(record, "state", path), `${path}.state`);
  if (state !== "submitted" && state !== "closed") {
    throw new DecodeError(`${path}.state`, "a completed Assessment Attempt state");
  }
  const scoreValue = record.score;
  let score: { readonly pointsEarned: number; readonly pointsPossible: number } | undefined;
  if (scoreValue !== undefined) {
    const scoreRecord = decodeRecord(scoreValue, `${path}.score`);
    requireOnlyFields(scoreRecord, `${path}.score`, ["pointsEarned", "pointsPossible"]);
    const pointsEarned = nonnegativeFiniteNumber(
      field(scoreRecord, "pointsEarned", `${path}.score`),
      `${path}.score.pointsEarned`,
    );
    const pointsPossible = nonnegativeFiniteNumber(
      field(scoreRecord, "pointsPossible", `${path}.score`),
      `${path}.score.pointsPossible`,
    );
    if (pointsEarned > pointsPossible) {
      throw new DecodeError(`${path}.score`, "an ordered Assessment Attempt score");
    }
    score = { pointsEarned, pointsPossible };
  }
  return {
    assessmentAttempt,
    attemptNumber: decodePositiveInteger(
      field(record, "attemptNumber", path),
      `${path}.attemptNumber`,
    ),
    state,
    ...(score === undefined ? {} : { score }),
  };
}

export function decodeLiveAssessmentAccess(
  value: unknown,
  path = "response",
): LiveAssessmentAccess {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "decision",
    "activeAssessmentAttempt",
    "title",
    "assessmentType",
    "questionCount",
    "pointsPossible",
    "previousAttempts",
  ]);
  const activeAssessmentAttemptValue = field(record, "activeAssessmentAttempt", path);
  let activeAssessmentAttempt = null;
  if (activeAssessmentAttemptValue !== null) {
    if (typeof activeAssessmentAttemptValue !== "string") {
      throw new DecodeError(
        `${path}.activeAssessmentAttempt`,
        "an Assessment Attempt UUID or null",
      );
    }
    activeAssessmentAttempt = parseAssessmentAttemptReference(activeAssessmentAttemptValue);
    if (activeAssessmentAttempt === null) {
      throw new DecodeError(
        `${path}.activeAssessmentAttempt`,
        "an Assessment Attempt UUID or null",
      );
    }
  }
  return {
    decision: decodeStudentAssessmentDecision(field(record, "decision", path), `${path}.decision`),
    activeAssessmentAttempt,
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    assessmentType: decodeStringEnum(
      field(record, "assessmentType", path),
      `${path}.assessmentType`,
      ASSESSMENT_TYPE_VALUES,
    ),
    questionCount: decodeNonnegativeInteger(
      field(record, "questionCount", path),
      `${path}.questionCount`,
    ),
    pointsPossible: nonnegativeFiniteNumber(
      field(record, "pointsPossible", path),
      `${path}.pointsPossible`,
    ),
    previousAttempts: decodeArray(
      field(record, "previousAttempts", path),
      `${path}.previousAttempts`,
      priorAttempt,
    ),
  };
}

export function decodeLiveAssessmentAttempt(
  value: unknown,
  path = "response",
): LiveAssessmentAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentAttempt",
    "assessment",
    "attemptNumber",
    "resumed",
    "title",
    "instructions",
    "questions",
  ]);
  const assessmentAttemptValue = field(record, "assessmentAttempt", path);
  if (typeof assessmentAttemptValue !== "string")
    throw new DecodeError(`${path}.assessmentAttempt`, "an Assessment Attempt UUID");
  const assessmentAttempt = parseAssessmentAttemptReference(assessmentAttemptValue);
  if (assessmentAttempt === null)
    throw new DecodeError(`${path}.assessmentAttempt`, "an Assessment Attempt UUID");
  const questions = decodeArray(
    field(record, "questions", path),
    `${path}.questions`,
    decodeIssuedQuestionPresentation,
  );
  if (questions.length === 0 || questions.length > MAX_QUESTIONS) {
    throw new DecodeError(`${path}.questions`, "one to twenty-five issued Questions");
  }
  return {
    assessmentAttempt,
    assessment: decodeAssessmentId(field(record, "assessment", path), `${path}.assessment`),
    attemptNumber: decodePositiveInteger(
      field(record, "attemptNumber", path),
      `${path}.attemptNumber`,
    ),
    resumed: decodeBoolean(field(record, "resumed", path), `${path}.resumed`),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    instructions: instructions(field(record, "instructions", path), `${path}.instructions`),
    questions,
  };
}
