// Issued-question, response, and feedback decoders.

import type { GradingResult } from "../../../generated/api/GradingResult";
import type { StudentFeedback } from "../../../generated/api/StudentFeedback";
import type { DraftQuestionDefinition } from "../../../generated/api/DraftQuestionDefinition";
import type { QuestionRevision } from "../../../generated/api/QuestionRevision";
import type { QuestionPresentation } from "../../../generated/api/QuestionPresentation";
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
  decodePositiveQuestionRevisionNumber,
  decodeQuestionId,
  decodeQuestionMetadata,
  decodeQuestionRevisionReference,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import { decodeQuestionSummary } from "./question_library";
import {
  decodeQuestionAttemptLimit,
  decodeContentBlock,
  decodeDraftQuestionSource,
  decodeQuestionGradingRule,
  decodeQuestionSource,
  decodeQuestionVariationDefinition,
  decodeQuestionResponseFormat,
  decodeQuestionFormat,
  decodeQuestionType,
  decodeQuestionAttemptTimeLimit,
  questionResponseFormatSupportsType,
} from "./question_model";

function decodeQuestionContent(
  record: Record<string, unknown>,
  path: string,
): Omit<QuestionRevision, "questionId" | "revisionNumber"> {
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
    throw new DecodeError(
      `${path}.questionType`,
      "a type supported by the Question Response Format",
    );
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
    questionAttemptLimit: decodeQuestionAttemptLimit(
      field(record, "questionAttemptLimit", path),
      `${path}.questionAttemptLimit`,
      true,
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
      true,
    ),
    questionVariationDefinition: decodeQuestionVariationDefinition(
      field(record, "questionVariationDefinition", path),
      `${path}.questionVariationDefinition`,
      true,
    ),
    grading: decodeQuestionGradingRule(field(record, "grading", path), `${path}.grading`, true),
    metadata: decodeQuestionMetadata(field(record, "metadata", path), `${path}.metadata`, true),
  } satisfies Omit<QuestionRevision, "questionId" | "revisionNumber">;
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
    throw new DecodeError(
      `${path}.questionType`,
      "a type supported by the Question Response Format",
    );
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
    questionAttemptLimit: decodeQuestionAttemptLimit(
      field(record, "questionAttemptLimit", path),
      `${path}.questionAttemptLimit`,
      true,
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
      true,
    ),
    questionVariationDefinition: decodeQuestionVariationDefinition(
      field(record, "questionVariationDefinition", path),
      `${path}.questionVariationDefinition`,
      true,
    ),
    grading: decodeQuestionGradingRule(field(record, "grading", path), `${path}.grading`, true),
    metadata: decodeQuestionMetadata(field(record, "metadata", path), `${path}.metadata`, true),
  } satisfies DraftQuestionDefinition;
}

export function decodeQuestionRevision(value: unknown, path = "response"): QuestionRevision {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "questionId",
    "revisionNumber",
    "workspace",
    "source",
    "questionFormat",
    "prompt",
    "response",
    "questionType",
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
    "questionVariationDefinition",
    "grading",
    "metadata",
  ]);
  const decoded = {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    revisionNumber: decodePositiveQuestionRevisionNumber(
      field(record, "revisionNumber", path),
      `${path}.revisionNumber`,
    ),
    ...decodeQuestionContent(record, path),
  } satisfies QuestionRevision;
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
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
    "questionVariationDefinition",
    "grading",
    "metadata",
  ]);
  return decodeDraftQuestionContent(record, path);
}

export function decodePublicationResult(value: unknown, path = "response"): PublicationResult {
  return { summary: decodeQuestionSummary(value, path, true) };
}

/** Strictly decodes the key-free rendered variant delivered for an attempt. */
export function decodeQuestionPresentation(
  value: unknown,
  path = "response",
): QuestionPresentation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["variation", "title", "prompt", "response"]);
  const variation = decodeRecord(field(record, "variation", path), `${path}.variation`);
  requireOnlyFields(variation, `${path}.variation`, ["questionRevision", "seed"]);
  const decoded = {
    variation: {
      questionRevision: decodeQuestionRevisionReference(
        field(variation, "questionRevision", `${path}.variation`),
        `${path}.variation.questionRevision`,
        true,
      ),
      seed: decodeNonnegativeInteger(
        field(variation, "seed", `${path}.variation`),
        `${path}.variation.seed`,
      ),
    },
    title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
    prompt: decodeArray(field(record, "prompt", path), `${path}.prompt`, (block, blockPath) =>
      decodeContentBlock(block, blockPath, true),
    ),
    response: decodeQuestionResponseFormat(
      field(record, "response", path),
      `${path}.response`,
      true,
    ),
  } satisfies QuestionPresentation;
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
    case "externalTool":
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
 * Decodes the server's already-redacted teaching projection.
 *
 * Every field is optional because absence is a security property: a client
 * must reject unknown properties rather than silently retaining a provider
 * transcript, key, or other server-private material.
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
          (block, blockPath) => decodeContentBlock(block, blockPath, true),
        )
      : undefined;
  const correctFeedback =
    "correctFeedback" in record
      ? decodeArray(
          field(record, "correctFeedback", path),
          `${path}.correctFeedback`,
          (block, blockPath) => decodeContentBlock(block, blockPath, true),
        )
      : undefined;
  const incorrectFeedback =
    "incorrectFeedback" in record
      ? decodeArray(
          field(record, "incorrectFeedback", path),
          `${path}.incorrectFeedback`,
          (block, blockPath) => decodeContentBlock(block, blockPath, true),
        )
      : undefined;
  const questionAnswer =
    "questionAnswer" in record
      ? decodeArray(
          field(record, "questionAnswer", path),
          `${path}.questionAnswer`,
          (block, blockPath) => decodeContentBlock(block, blockPath, true),
        )
      : undefined;
  const questionAnswerExplanation =
    "questionAnswerExplanation" in record
      ? decodeArray(
          field(record, "questionAnswerExplanation", path),
          `${path}.questionAnswerExplanation`,
          (block, blockPath) => decodeContentBlock(block, blockPath, true),
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
