// Strict decoder for the closed Question Pool search-metadata command.

import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import { DecodeError, decodeArray, decodeRecord, decodeSafeInteger, decodeUuid } from "../decoder";
import type {
  QuestionPoolSearchMetadataUpdateRequest,
  QuestionPoolSearchMetadataUpdateResult,
} from "../question_pool_search_metadata";
import { decodeQuestionId, field, requireOnlyFields } from "./shared";

const MAX_TAG_CODE_POINTS = 120;

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

function decodeTag(value: unknown, path: string): string {
  if (
    typeof value !== "string" ||
    value.trim() !== value ||
    value.length === 0 ||
    [...value].length > MAX_TAG_CODE_POINTS ||
    hasControlCharacter(value)
  ) {
    throw new DecodeError(
      path,
      `trimmed, nonempty, control-free text within ${MAX_TAG_CODE_POINTS} Unicode code points`,
    );
  }
  return value;
}

function decodeTags(value: unknown, path: string): Array<string> {
  const tags = decodeArray(value, path, decodeTag);
  if (new Set(tags).size !== tags.length) throw new DecodeError(path, "unique tags");
  return tags;
}

function decodeCanonicalUuid(value: unknown, path: string): string {
  const uuid = decodeUuid(value, path);
  if (uuid !== uuid.toLowerCase()) throw new DecodeError(path, "a canonical lowercase UUID");
  return uuid;
}

function decodeOptionalUuid(value: unknown, path: string): string | null {
  if (value === null) return null;
  return decodeCanonicalUuid(value, path);
}

function decodeUpdateResult(value: unknown, path: string): QuestionPoolSearchMetadataUpdateResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolId", "questionPoolMetadataEditNumber"]);
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolMetadataEditNumber: decodePositiveSafeInteger(
      field(record, "questionPoolMetadataEditNumber", path),
      `${path}.questionPoolMetadataEditNumber`,
    ),
  };
}

/** ASVS 1.5.2 and 2.2.1: allow only the exact bounded Pool search-metadata result. */
export function decodeQuestionPoolSearchMetadataResult(
  value: unknown,
  path = "response",
): ReadonlyArray<QuestionPoolSearchMetadataUpdateResult> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["results"]);
  const results = field(record, "results", path);
  if (!Array.isArray(results)) throw new DecodeError(`${path}.results`, "an array");
  if (results.length === 0 || results.length > MAX_BULK_QUESTION_METADATA_ITEMS) {
    throw new DecodeError(`${path}.results`, `1 through ${MAX_BULK_QUESTION_METADATA_ITEMS} items`);
  }
  return results.map((item, index) => decodeUpdateResult(item, `${path}.results[${index}]`));
}

/** ASVS 2.2.1: Discipline and Subject are not fields of this command. */
export function validateQuestionPoolSearchMetadataRequest(
  value: unknown,
): QuestionPoolSearchMetadataUpdateRequest {
  const request = decodeRecord(value, "request");
  requireOnlyFields(request, "request", ["selection", "patch"]);
  const rawSelection = field(request, "selection", "request");
  if (!Array.isArray(rawSelection)) throw new DecodeError("request.selection", "an array");
  if (rawSelection.length === 0 || rawSelection.length > MAX_BULK_QUESTION_METADATA_ITEMS) {
    throw new DecodeError(
      "request.selection",
      `1 through ${MAX_BULK_QUESTION_METADATA_ITEMS} items`,
    );
  }
  const selection = rawSelection.map((item, index) => {
    const itemPath = `request.selection[${index}]`;
    const record = decodeRecord(item, itemPath);
    requireOnlyFields(record, itemPath, ["questionPoolId", "questionPoolMetadataEditNumber"]);
    return {
      questionPoolId: decodeQuestionId(
        field(record, "questionPoolId", itemPath),
        `${itemPath}.questionPoolId`,
      ),
      questionPoolMetadataEditNumber: decodePositiveSafeInteger(
        field(record, "questionPoolMetadataEditNumber", itemPath),
        `${itemPath}.questionPoolMetadataEditNumber`,
      ),
    };
  });
  const ids = selection.map((item) => item.questionPoolId);
  if (new Set(ids).size !== ids.length) {
    throw new DecodeError("request.selection", "distinct Question Pool IDs");
  }
  if (ids.some((id, index) => index > 0 && id <= (ids[index - 1] ?? ""))) {
    throw new DecodeError("request.selection", "canonical Question Pool ID order");
  }
  const rawPatch = decodeRecord(field(request, "patch", "request"), "request.patch");
  const keys = Object.keys(rawPatch);
  if (
    keys.length === 0 ||
    keys.length > 3 ||
    keys.some((key) => !["tags", "topicUuid", "subtopicUuid"].includes(key))
  ) {
    throw new DecodeError("request.patch", "Topic, Subtopic, or Tags");
  }
  const tags =
    "tags" in rawPatch
      ? decodeTags(field(rawPatch, "tags", "request.patch"), "request.patch.tags")
      : undefined;
  const topicUuid =
    "topicUuid" in rawPatch
      ? decodeOptionalUuid(field(rawPatch, "topicUuid", "request.patch"), "request.patch.topicUuid")
      : undefined;
  const subtopicUuid =
    "subtopicUuid" in rawPatch
      ? decodeOptionalUuid(
          field(rawPatch, "subtopicUuid", "request.patch"),
          "request.patch.subtopicUuid",
        )
      : undefined;
  return {
    selection,
    patch: {
      ...(tags === undefined ? {} : { tags }),
      ...(topicUuid === undefined ? {} : { topicUuid }),
      ...(subtopicUuid === undefined ? {} : { subtopicUuid }),
    },
  };
}
