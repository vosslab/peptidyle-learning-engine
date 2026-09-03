// Issued-question, response, and feedback decoders.

import type { GradingResult } from "../../../generated/api/GradingResult";
import type { StudentFeedback } from "../../../generated/api/StudentFeedback";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { ImathasQuestionBackendLaunch } from "../contracts";
import { isExpectedImathasQuestionBackendLaunchPath } from "../imathas_question_backend_launch";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeFiniteNumber,
  decodeNonemptyString,
  decodeRecord,
  decodeString,
} from "../decoder";
import { field, kind, requireOnlyFields } from "./shared";
import { decodeQuestionContentBlock } from "./question_model";

/** Decodes the route-only iMathAS Question Backend Transport response. */
export function decodeImathasQuestionBackendLaunch(
  value: unknown,
  path: string,
  courseId: string,
  assignmentId: string,
  attemptId: string,
): ImathasQuestionBackendLaunch {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["launchUrl"]);
  const launchUrl = decodeNonemptyString(field(record, "launchUrl", path), `${path}.launchUrl`);
  if (
    !isExpectedImathasQuestionBackendLaunchPath(
      launchUrl,
      courseId,
      assignmentId,
      attemptId,
      "https://ple-invalid.example",
    )
  ) {
    throw new DecodeError(
      `${path}.launchUrl`,
      "the expected same-origin iMathAS Question Backend Transport route for this attempt",
    );
  }
  return { launchUrl };
}

/** Strict outbound and inbound student-response boundary. */
export function decodeStudentResponse(value: unknown, path = "response"): StudentResponse {
  const record = decodeRecord(value, path);
  const response = kind(record, path);
  switch (response) {
    case "numeric": {
      requireOnlyFields(record, path, ["kind", "value"]);
      const decoded = {
        kind: response,
        value: decodeFiniteNumber(field(record, "value", path), `${path}.value`),
      } satisfies StudentResponse;
      return decoded;
    }
    case "multipleChoice": {
      requireOnlyFields(record, path, ["kind", "selected"]);
      const decoded = {
        kind: response,
        selected: decodeArray(
          field(record, "selected", path),
          `${path}.selected`,
          decodeNonemptyString,
        ),
      } satisfies StudentResponse;
      return decoded;
    }
    case "shortText": {
      requireOnlyFields(record, path, ["kind", "text"]);
      const decoded = {
        kind: response,
        text: decodeString(field(record, "text", path), `${path}.text`),
      } satisfies StudentResponse;
      return decoded;
    }
    case "multiBlank": {
      requireOnlyFields(record, path, ["kind", "answers"]);
      return {
        kind: response,
        answers: decodeArray(
          field(record, "answers", path),
          `${path}.answers`,
          (value, answerPath) => {
            const answer = decodeRecord(value, answerPath);
            requireOnlyFields(answer, answerPath, ["slot", "text"]);
            return {
              slot: decodeNonemptyString(field(answer, "slot", answerPath), `${answerPath}.slot`),
              text: decodeString(field(answer, "text", answerPath), `${answerPath}.text`),
            };
          },
        ),
      } satisfies StudentResponse;
    }
    case "matching": {
      requireOnlyFields(record, path, ["kind", "matches"]);
      return {
        kind: response,
        matches: decodeArray(
          field(record, "matches", path),
          `${path}.matches`,
          (value, pairPath) => {
            const pair = decodeRecord(value, pairPath);
            requireOnlyFields(pair, pairPath, ["prompt", "choice"]);
            return {
              prompt: decodeNonemptyString(field(pair, "prompt", pairPath), `${pairPath}.prompt`),
              choice: decodeNonemptyString(field(pair, "choice", pairPath), `${pairPath}.choice`),
            };
          },
        ),
      } satisfies StudentResponse;
    }
    case "ordering": {
      requireOnlyFields(record, path, ["kind", "order"]);
      const decoded = {
        kind: response,
        order: decodeArray(field(record, "order", path), `${path}.order`, decodeNonemptyString),
      } satisfies StudentResponse;
      return decoded;
    }
    case "hotspot": {
      requireOnlyFields(record, path, ["kind", "selections"]);
      return {
        kind: response,
        selections: decodeArray(
          field(record, "selections", path),
          `${path}.selections`,
          (value, selectionPath) => {
            const selection = decodeRecord(value, selectionPath);
            requireOnlyFields(selection, selectionPath, ["region"]);
            return {
              region: decodeNonemptyString(
                field(selection, "region", selectionPath),
                `${selectionPath}.region`,
              ),
            };
          },
        ),
      } satisfies StudentResponse;
    }
    case "imathasQuestionBackend":
      requireOnlyFields(record, path, ["kind"]);
      return { kind: response } satisfies StudentResponse;
    default:
      throw new DecodeError(`${path}.kind`, "a known student-response kind");
  }
}

export function decodeGradingResult(value: unknown, path: string): GradingResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["correct", "pointsEarned", "pointsPossible"]);
  const decoded = {
    correct: decodeBoolean(field(record, "correct", path), `${path}.correct`),
    pointsEarned: decodeFiniteNumber(field(record, "pointsEarned", path), `${path}.pointsEarned`),
    pointsPossible: decodeFiniteNumber(
      field(record, "pointsPossible", path),
      `${path}.pointsPossible`,
    ),
  } satisfies GradingResult;
  return decoded;
}

/**
 * Decodes the server's already-redacted Student Feedback.
 *
 * Every field is optional because absence is a security property: a client
 * must reject unknown properties rather than silently retaining iMathAS
 * transcript, Answer Key, or other server-private Question Grading Input.
 */
export function decodeStudentFeedback(value: unknown, path = "response"): StudentFeedback {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "correctness",
    "pointsEarned",
    "pointsPossible",
    "choiceFeedback",
    "correctFeedback",
    "incorrectFeedback",
    "questionAnswer",
    "questionAnswerExplanation",
  ]);
  const correctness =
    "correctness" in record
      ? decodeBoolean(field(record, "correctness", path), `${path}.correctness`)
      : undefined;
  const pointsEarned =
    "pointsEarned" in record
      ? decodeFiniteNumber(field(record, "pointsEarned", path), `${path}.pointsEarned`)
      : undefined;
  const pointsPossible =
    "pointsPossible" in record
      ? decodeFiniteNumber(field(record, "pointsPossible", path), `${path}.pointsPossible`)
      : undefined;
  const choiceFeedback =
    "choiceFeedback" in record
      ? decodeArray(
          field(record, "choiceFeedback", path),
          `${path}.choiceFeedback`,
          (block, blockPath) => decodeQuestionContentBlock(block, blockPath, true),
        )
      : undefined;
  const correctFeedback =
    "correctFeedback" in record
      ? decodeArray(
          field(record, "correctFeedback", path),
          `${path}.correctFeedback`,
          (block, blockPath) => decodeQuestionContentBlock(block, blockPath, true),
        )
      : undefined;
  const incorrectFeedback =
    "incorrectFeedback" in record
      ? decodeArray(
          field(record, "incorrectFeedback", path),
          `${path}.incorrectFeedback`,
          (block, blockPath) => decodeQuestionContentBlock(block, blockPath, true),
        )
      : undefined;
  const questionAnswer =
    "questionAnswer" in record
      ? decodeArray(
          field(record, "questionAnswer", path),
          `${path}.questionAnswer`,
          (block, blockPath) => decodeQuestionContentBlock(block, blockPath, true),
        )
      : undefined;
  const questionAnswerExplanation =
    "questionAnswerExplanation" in record
      ? decodeArray(
          field(record, "questionAnswerExplanation", path),
          `${path}.questionAnswerExplanation`,
          (block, blockPath) => decodeQuestionContentBlock(block, blockPath, true),
        )
      : undefined;
  return {
    ...(correctness === undefined ? {} : { correctness }),
    ...(pointsEarned === undefined ? {} : { pointsEarned }),
    ...(pointsPossible === undefined ? {} : { pointsPossible }),
    ...(choiceFeedback === undefined ? {} : { choiceFeedback }),
    ...(correctFeedback === undefined ? {} : { correctFeedback }),
    ...(incorrectFeedback === undefined ? {} : { incorrectFeedback }),
    ...(questionAnswer === undefined ? {} : { questionAnswer }),
    ...(questionAnswerExplanation === undefined ? {} : { questionAnswerExplanation }),
  } satisfies StudentFeedback;
}
