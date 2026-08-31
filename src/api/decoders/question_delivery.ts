// Issued-question, response, and feedback decoders.

import type { AttemptResult } from "../../../generated/api/AttemptResult";
import type { DisclosedFeedback } from "../../../generated/api/DisclosedFeedback";
import type { DraftQuestionDefinition } from "../../../generated/api/DraftQuestionDefinition";
import type { QuestionDefinition } from "../../../generated/api/QuestionDefinition";
import type { QuestionEnvelope } from "../../../generated/api/QuestionEnvelope";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { ExternalToolLaunch, PublicationResult } from "../contracts";
import { isCanonicalExternalToolLaunchPath } from "../external_tool_launch";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeFiniteNumber,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import {
  decodeEnvelopeTitle,
  decodeIdentifier,
  decodePositiveQuestionVersionNumber,
  decodeQuestionId,
  decodeQuestionMetadata,
  decodeQuestionVersionReference,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import { decodeCatalogQuestionSummary } from "./catalog_course";
import {
  decodeAttemptPolicy,
  decodeContentBlock,
  decodeDraftQuestionSource,
  decodeGradingDefinition,
  decodeQuestionSource,
  decodeRandomization,
  decodeQuestionResponseFormat,
  decodeQuestionFormat,
  decodeQuestionType,
  decodeTimingPolicy,
  questionResponseFormatSupportsType,
} from "./question_model";

function decodeNormalizedCoordinate(value: unknown, path: string): number {
  const coordinate = decodeNonnegativeInteger(value, path);
  if (coordinate > 10_000) throw new DecodeError(path, "an integer from 0 through 10000");
  return coordinate;
}

function decodeQuestionContent(
  record: Record<string, unknown>,
  path: string,
): Omit<QuestionDefinition, "questionId" | "versionNumber"> {
  const response = decodeQuestionResponseFormat(
    field(record, "response", path),
    `${path}.response`,
    true,
  );
  const questionType = decodeQuestionType(
    field(record, "questionType", path),
    `${path}.questionType`,
  );
  if (!questionResponseFormatSupportsType(response, questionType)) {
    throw new DecodeError(`${path}.questionType`, "a type supported by the Question Response Format");
  }
  return {
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
    source: decodeQuestionSource(field(record, "source", path), `${path}.source`),
    questionFormat: decodeQuestionFormat(
      field(record, "questionFormat", path),
      `${path}.questionFormat`,
    ),
    prompt: decodeArray(field(record, "prompt", path), `${path}.prompt`, (block, blockPath) =>
      decodeContentBlock(block, blockPath, true),
    ),
    response,
    questionType,
    attemptPolicy: decodeAttemptPolicy(
      field(record, "attemptPolicy", path),
      `${path}.attemptPolicy`,
      true,
    ),
    timingPolicy: decodeTimingPolicy(
      field(record, "timingPolicy", path),
      `${path}.timingPolicy`,
      true,
    ),
    randomization: decodeRandomization(
      field(record, "randomization", path),
      `${path}.randomization`,
      true,
    ),
    grading: decodeGradingDefinition(field(record, "grading", path), `${path}.grading`, true),
    metadata: decodeQuestionMetadata(field(record, "metadata", path), `${path}.metadata`, true),
  } satisfies Omit<QuestionDefinition, "questionId" | "versionNumber">;
}

function decodeDraftQuestionContent(
  record: Record<string, unknown>,
  path: string,
): DraftQuestionDefinition {
  const response = decodeQuestionResponseFormat(
    field(record, "response", path),
    `${path}.response`,
    true,
  );
  const questionType = decodeQuestionType(
    field(record, "questionType", path),
    `${path}.questionType`,
  );
  if (!questionResponseFormatSupportsType(response, questionType)) {
    throw new DecodeError(`${path}.questionType`, "a type supported by the Question Response Format");
  }
  return {
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
    source: decodeDraftQuestionSource(field(record, "source", path), `${path}.source`),
    questionFormat: decodeQuestionFormat(
      field(record, "questionFormat", path),
      `${path}.questionFormat`,
    ),
    prompt: decodeArray(field(record, "prompt", path), `${path}.prompt`, (block, blockPath) =>
      decodeContentBlock(block, blockPath, true),
    ),
    response,
    questionType,
    attemptPolicy: decodeAttemptPolicy(
      field(record, "attemptPolicy", path),
      `${path}.attemptPolicy`,
      true,
    ),
    timingPolicy: decodeTimingPolicy(
      field(record, "timingPolicy", path),
      `${path}.timingPolicy`,
      true,
    ),
    randomization: decodeRandomization(
      field(record, "randomization", path),
      `${path}.randomization`,
      true,
    ),
    grading: decodeGradingDefinition(field(record, "grading", path), `${path}.grading`, true),
    metadata: decodeQuestionMetadata(field(record, "metadata", path), `${path}.metadata`, true),
  } satisfies DraftQuestionDefinition;
}

export function decodeQuestionDefinition(value: unknown, path = "response"): QuestionDefinition {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "questionId",
    "versionNumber",
    "workspace",
    "source",
    "questionFormat",
    "prompt",
    "response",
    "questionType",
    "attemptPolicy",
    "timingPolicy",
    "randomization",
    "grading",
    "metadata",
  ]);
  const decoded = {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    versionNumber: decodePositiveQuestionVersionNumber(
      field(record, "versionNumber", path),
      `${path}.versionNumber`,
    ),
    ...decodeQuestionContent(record, path),
  } satisfies QuestionDefinition;
  return decoded;
}

/** Strictly decodes editable content, which cannot carry published IDs. */
export function decodeDraftQuestionDefinition(
  value: unknown,
  path = "response",
): DraftQuestionDefinition {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "workspace",
    "source",
    "questionFormat",
    "prompt",
    "response",
    "questionType",
    "attemptPolicy",
    "timingPolicy",
    "randomization",
    "grading",
    "metadata",
  ]);
  return decodeDraftQuestionContent(record, path);
}

export function decodePublicationResult(value: unknown, path = "response"): PublicationResult {
  return { summary: decodeCatalogQuestionSummary(value, path, true) };
}

/** Strictly decodes the key-free rendered variant delivered for an attempt. */
export function decodeQuestionEnvelope(value: unknown, path = "response"): QuestionEnvelope {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionVersion", "seed", "title", "prompt", "response"]);
  const decoded = {
    questionVersion: decodeQuestionVersionReference(
      field(record, "questionVersion", path),
      `${path}.questionVersion`,
      true,
    ),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
    prompt: decodeArray(field(record, "prompt", path), `${path}.prompt`, (block, blockPath) =>
      decodeContentBlock(block, blockPath, true),
    ),
    response: decodeQuestionResponseFormat(field(record, "response", path), `${path}.response`, true),
  } satisfies QuestionEnvelope;
  return decoded;
}

/** Decodes the route-only external-tool broker projection. */
export function decodeExternalToolLaunch(
  value: unknown,
  path: string,
  courseId: string,
  assignmentId: string,
  attemptId: string,
): ExternalToolLaunch {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["launchUrl"]);
  const launchUrl = decodeNonemptyString(field(record, "launchUrl", path), `${path}.launchUrl`);
  if (
    !isCanonicalExternalToolLaunchPath(
      launchUrl,
      courseId,
      assignmentId,
      attemptId,
      "https://ple-invalid.example",
    )
  ) {
    throw new DecodeError(
      `${path}.launchUrl`,
      "the canonical same-origin broker path for this attempt",
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
      requireOnlyFields(record, path, ["kind", "points"]);
      return {
        kind: response,
        points: decodeArray(field(record, "points", path), `${path}.points`, (value, pointPath) => {
          const point = decodeRecord(value, pointPath);
          requireOnlyFields(point, pointPath, ["x", "y"]);
          return {
            x: decodeNormalizedCoordinate(field(point, "x", pointPath), `${pointPath}.x`),
            y: decodeNormalizedCoordinate(field(point, "y", pointPath), `${pointPath}.y`),
          };
        }),
      } satisfies StudentResponse;
    }
    case "fileUpload": {
      requireOnlyFields(record, path, ["kind", "objectKey"]);
      const decoded = {
        kind: response,
        objectKey: decodeNonemptyString(field(record, "objectKey", path), `${path}.objectKey`),
      } satisfies StudentResponse;
      return decoded;
    }
    case "externalTool":
      requireOnlyFields(record, path, ["kind"]);
      return { kind: response } satisfies StudentResponse;
    default:
      throw new DecodeError(`${path}.kind`, "a known student-response kind");
  }
}

export function decodeAttemptResult(value: unknown, path: string): AttemptResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["correct", "pointsEarned", "pointsPossible"]);
  const decoded = {
    correct: decodeBoolean(field(record, "correct", path), `${path}.correct`),
    pointsEarned: decodeFiniteNumber(field(record, "pointsEarned", path), `${path}.pointsEarned`),
    pointsPossible: decodeFiniteNumber(
      field(record, "pointsPossible", path),
      `${path}.pointsPossible`,
    ),
  } satisfies AttemptResult;
  return decoded;
}

/**
 * Decodes the server's already-redacted teaching projection.
 *
 * Every field is optional because absence is a security property: a client
 * must reject unknown properties rather than silently retaining a provider
 * transcript, key, or other server-private material.
 */
export function decodeDisclosedFeedback(value: unknown, path = "response"): DisclosedFeedback {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "correctness",
    "pointsEarned",
    "pointsPossible",
    "hint",
    "correctResponse",
    "rationale",
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
  const hint =
    "hint" in record
      ? decodeArray(field(record, "hint", path), `${path}.hint`, (block, blockPath) =>
          decodeContentBlock(block, blockPath, true),
        )
      : undefined;
  const correctResponse =
    "correctResponse" in record
      ? decodeArray(
          field(record, "correctResponse", path),
          `${path}.correctResponse`,
          (block, blockPath) => decodeContentBlock(block, blockPath, true),
        )
      : undefined;
  const rationale =
    "rationale" in record
      ? decodeArray(field(record, "rationale", path), `${path}.rationale`, (block, blockPath) =>
          decodeContentBlock(block, blockPath, true),
        )
      : undefined;
  return {
    ...(correctness === undefined ? {} : { correctness }),
    ...(pointsEarned === undefined ? {} : { pointsEarned }),
    ...(pointsPossible === undefined ? {} : { pointsPossible }),
    ...(hint === undefined ? {} : { hint }),
    ...(correctResponse === undefined ? {} : { correctResponse }),
    ...(rationale === undefined ? {} : { rationale }),
  } satisfies DisclosedFeedback;
}
