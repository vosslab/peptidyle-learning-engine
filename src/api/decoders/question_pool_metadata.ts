// Strict wire decoding for ordinary current Question Pool metadata.

import type { QuestionPoolMetadataEditNumber } from "../../../generated/api/QuestionPoolMetadataEditNumber";
import {
  DecodeError,
  decodeArray,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeStringEnum,
} from "../decoder";
import type {
  CurrentQuestionPoolMetadata,
  QuestionPoolMetadataReplacement,
  SavedQuestionPoolMetadata,
  SaveQuestionPoolMetadataRequest,
} from "../question_pool_metadata";
import { BLOOM_COGNITIVE_PROCESSES, BLOOM_KNOWLEDGE_DIMENSIONS } from "./bloom_classification";
import { decodeQuestionId, field, requireOnlyFields } from "./shared";
import { decodeQuestionPoolText } from "./question_pool_summary";

function decodeEditNumber(value: unknown, path: string): QuestionPoolMetadataEditNumber {
  const result = decodeSafeInteger(value, path);
  if (result < 1) throw new DecodeError(path, "a positive Pool metadata Edit Number");
  return result;
}

function decodeClassificationUuid(value: unknown, path: string): string {
  if (
    typeof value !== "string" ||
    !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u.test(value)
  ) {
    throw new DecodeError(path, "a canonical lowercase classification UUID");
  }
  return value;
}

function decodeMetadata(value: unknown, path: string): QuestionPoolMetadataReplacement {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "title",
    "description",
    "topicUuid",
    "subtopicUuid",
    "tags",
    "bloomCognitiveProcess",
    "bloomKnowledgeDimension",
  ]);
  const tags = decodeArray(field(record, "tags", path), `${path}.tags`, (item, itemPath) =>
    decodeQuestionPoolText(item, itemPath, 120),
  );
  if (new Set(tags).size !== tags.length) {
    throw new DecodeError(`${path}.tags`, "unique Pool tags");
  }
  const topicUuid = decodeNullable(
    field(record, "topicUuid", path),
    `${path}.topicUuid`,
    decodeClassificationUuid,
  );
  const subtopicUuid = decodeNullable(
    field(record, "subtopicUuid", path),
    `${path}.subtopicUuid`,
    decodeClassificationUuid,
  );
  if (subtopicUuid !== null && topicUuid === null) {
    throw new DecodeError(path, "a Subtopic with its selected Topic");
  }
  return {
    title: decodeQuestionPoolText(field(record, "title", path), `${path}.title`, 512),
    description: decodeQuestionPoolText(
      field(record, "description", path),
      `${path}.description`,
      4000,
    ),
    topicUuid,
    subtopicUuid,
    tags,
    bloomCognitiveProcess: decodeNullable(
      field(record, "bloomCognitiveProcess", path),
      `${path}.bloomCognitiveProcess`,
      (item, itemPath) => decodeStringEnum(item, itemPath, BLOOM_COGNITIVE_PROCESSES),
    ),
    bloomKnowledgeDimension: decodeNullable(
      field(record, "bloomKnowledgeDimension", path),
      `${path}.bloomKnowledgeDimension`,
      (item, itemPath) => decodeStringEnum(item, itemPath, BLOOM_KNOWLEDGE_DIMENSIONS),
    ),
  };
}

const CURRENT_FIELDS = [
  "questionPoolId",
  "questionPoolMetadataEditNumber",
  "title",
  "description",
  "topicUuid",
  "subtopicUuid",
  "tags",
  "bloomCognitiveProcess",
  "bloomKnowledgeDimension",
] as const;

export function decodeCurrentQuestionPoolMetadata(
  value: unknown,
  path = "response",
): CurrentQuestionPoolMetadata {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, CURRENT_FIELDS);
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolMetadataEditNumber: decodeEditNumber(
      field(record, "questionPoolMetadataEditNumber", path),
      `${path}.questionPoolMetadataEditNumber`,
    ),
    ...decodeMetadata(
      Object.fromEntries(
        [
          "title",
          "description",
          "topicUuid",
          "subtopicUuid",
          "tags",
          "bloomCognitiveProcess",
          "bloomKnowledgeDimension",
        ].map((key) => [key, field(record, key, path)]),
      ),
      path,
    ),
  };
}

export function decodeSavedQuestionPoolMetadata(
  value: unknown,
  path = "response",
): SavedQuestionPoolMetadata {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolId", "questionPoolMetadataEditNumber"]);
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolMetadataEditNumber: decodeEditNumber(
      field(record, "questionPoolMetadataEditNumber", path),
      `${path}.questionPoolMetadataEditNumber`,
    ),
  };
}

export function validateQuestionPoolMetadataRequest(
  value: unknown,
): SaveQuestionPoolMetadataRequest {
  const record = decodeRecord(value, "request");
  requireOnlyFields(record, "request", [
    "questionPoolId",
    "expectedMetadataEditNumber",
    "metadata",
  ]);
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", "request"),
      "request.questionPoolId",
    ),
    expectedMetadataEditNumber: decodeEditNumber(
      field(record, "expectedMetadataEditNumber", "request"),
      "request.expectedMetadataEditNumber",
    ),
    metadata: decodeMetadata(field(record, "metadata", "request"), "request.metadata"),
  };
}
