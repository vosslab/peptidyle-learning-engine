// Strict decoders for the closed Published Question shared-metadata boundary.

import type { PublishedQuestionSharedMetadata } from "../../../generated/api/PublishedQuestionSharedMetadata";
import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import type {
  QuestionBulkMetadataUpdateResult,
  QuestionBulkMetadataUpdateRequest,
} from "../question_bulk_metadata";
import {
  DecodeError,
  decodeArray,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeUuid,
} from "../decoder";
import { decodeQuestionId, field, requireOnlyFields } from "./shared";

const MAX_SHARED_METADATA_VALUES = 64;
const MAX_SHARED_METADATA_TEXT_CODE_POINTS = 120;
function decodeClassificationUuid(value: unknown, path: string): string {
  const uuid = decodeUuid(value, path);
  if (uuid !== uuid.toLowerCase()) throw new DecodeError(path, "a canonical lowercase UUID");
  return uuid;
}
function hasControlCharacter(value: string): boolean {
  return [...value].some((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint <= 0x1f || (codePoint >= 0x7f && codePoint <= 0x9f);
  });
}

function decodePositiveSafeInteger(value: unknown, path: string): number {
  const decoded = decodeSafeInteger(value, path);
  if (decoded < 1) throw new DecodeError(path, "a positive safe integer");
  return decoded;
}

function decodeSharedMetadataText(value: unknown, path: string): string {
  if (
    typeof value !== "string" ||
    value.trim() !== value ||
    value.length === 0 ||
    [...value].length > MAX_SHARED_METADATA_TEXT_CODE_POINTS ||
    hasControlCharacter(value)
  ) {
    throw new DecodeError(
      path,
      `trimmed, nonempty, control-free text within ${MAX_SHARED_METADATA_TEXT_CODE_POINTS} Unicode code points`,
    );
  }
  return value;
}

function decodeTags(value: unknown, path: string): Array<string> {
  const tags = decodeArray(value, path, decodeSharedMetadataText);
  if (tags.length > MAX_SHARED_METADATA_VALUES) {
    throw new DecodeError(path, `at most ${MAX_SHARED_METADATA_VALUES} tags`);
  }
  if (new Set(tags).size !== tags.length) throw new DecodeError(path, "unique tags");
  return tags;
}

function decodeSharedMetadata(value: unknown, path: string): PublishedQuestionSharedMetadata {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "questionId",
    "metadataEditNumber",
    "tags",
    "disciplineUuid",
    "subjectUuid",
    "topicUuid",
    "subtopicUuid",
  ]);
  return {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    metadataEditNumber: decodePositiveSafeInteger(
      field(record, "metadataEditNumber", path),
      `${path}.metadataEditNumber`,
    ),
    tags: decodeTags(field(record, "tags", path), `${path}.tags`),
    disciplineUuid: decodeClassificationUuid(
      field(record, "disciplineUuid", path),
      `${path}.disciplineUuid`,
    ),
    subjectUuid: decodeClassificationUuid(
      field(record, "subjectUuid", path),
      `${path}.subjectUuid`,
    ),
    topicUuid: decodeNullable(
      field(record, "topicUuid", path),
      `${path}.topicUuid`,
      decodeClassificationUuid,
    ),
    subtopicUuid: decodeNullable(
      field(record, "subtopicUuid", path),
      `${path}.subtopicUuid`,
      decodeClassificationUuid,
    ),
  };
}

function decodeBoundedItems<T>(
  value: unknown,
  path: string,
  decoder: (item: unknown, itemPath: string) => T,
): Array<T> {
  const items = decodeArray(value, path, decoder);
  if (items.length === 0 || items.length > MAX_BULK_QUESTION_METADATA_ITEMS) {
    throw new DecodeError(path, `1 through ${MAX_BULK_QUESTION_METADATA_ITEMS} items`);
  }
  return items;
}

/** ASVS 1.5.2 and 2.2.1: allow only the exact bounded current-metadata response. */
export function decodeQuestionBulkMetadataCurrent(
  value: unknown,
  path = "response",
): ReadonlyArray<PublishedQuestionSharedMetadata> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items"]);
  return decodeBoundedItems(field(record, "items", path), `${path}.items`, decodeSharedMetadata);
}

function decodeUpdateResult(value: unknown, path: string): QuestionBulkMetadataUpdateResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionId", "metadataEditNumber"]);
  return {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    metadataEditNumber: decodePositiveSafeInteger(
      field(record, "metadataEditNumber", path),
      `${path}.metadataEditNumber`,
    ),
  };
}

/** ASVS 1.5.2 and 2.2.1: allow only the exact bounded whole-command result. */
export function decodeQuestionBulkMetadataResult(
  value: unknown,
  path = "response",
): ReadonlyArray<QuestionBulkMetadataUpdateResult> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["results"]);
  return decodeBoundedItems(field(record, "results", path), `${path}.results`, decodeUpdateResult);
}

export function validateQuestionBulkMetadataRequest(
  value: unknown,
): QuestionBulkMetadataUpdateRequest {
  const request = decodeRecord(value, "request");
  requireOnlyFields(request, "request", ["selection", "patch"]);
  const rawSelection = field(request, "selection", "request");
  if (!Array.isArray(rawSelection)) {
    throw new DecodeError("request.selection", "an array");
  }
  if (rawSelection.length === 0 || rawSelection.length > MAX_BULK_QUESTION_METADATA_ITEMS) {
    throw new DecodeError(
      "request.selection",
      `1 through ${MAX_BULK_QUESTION_METADATA_ITEMS} items`,
    );
  }
  const selection = rawSelection.map((value, index) => {
    const itemPath = `request.selection[${index}]`;
    const item = decodeRecord(value, itemPath);
    requireOnlyFields(item, itemPath, ["questionId", "metadataEditNumber"]);
    return {
      questionId: decodeQuestionId(field(item, "questionId", itemPath), `${itemPath}.questionId`),
      metadataEditNumber: decodePositiveSafeInteger(
        field(item, "metadataEditNumber", itemPath),
        `${itemPath}.metadataEditNumber`,
      ),
    };
  });
  const ids = selection.map((item) => item.questionId);
  if (new Set(ids).size !== ids.length) {
    throw new DecodeError("request.selection", "distinct Question IDs");
  }
  if (ids.some((id, index) => index > 0 && id <= (ids[index - 1] ?? ""))) {
    throw new DecodeError("request.selection", "canonical Question ID order");
  }

  const rawPatch = decodeRecord(field(request, "patch", "request"), "request.patch");
  const keys = Object.keys(rawPatch);
  if (
    keys.length === 0 ||
    keys.some(
      (key) =>
        !["tags", "disciplineUuid", "subjectUuid", "topicUuid", "subtopicUuid"].includes(key),
    )
  ) {
    throw new DecodeError("request.patch", "at least one closed shared-metadata field");
  }
  const patch: {
    tags?: Array<string>;
    disciplineUuid?: string;
    subjectUuid?: string;
    topicUuid?: string | null;
    subtopicUuid?: string | null;
  } = {};
  if ("tags" in rawPatch) {
    patch.tags = decodeTags(field(rawPatch, "tags", "request.patch"), "request.patch.tags");
  }
  for (const key of ["disciplineUuid", "subjectUuid"] as const) {
    if (key in rawPatch)
      patch[key] = decodeClassificationUuid(
        field(rawPatch, key, "request.patch"),
        `request.patch.${key}`,
      );
  }
  for (const key of ["topicUuid", "subtopicUuid"] as const) {
    if (key in rawPatch)
      patch[key] = decodeNullable(
        field(rawPatch, key, "request.patch"),
        `request.patch.${key}`,
        decodeClassificationUuid,
      );
  }
  return { selection, patch };
}
