// Strict decoding for current Question and Pool recognition titles.

import { DecodeError, decodeArray, decodeRecord } from "../decoder";
import { RECOGNITION_TITLE_BATCH_LIMIT, type RecognitionTitleMaps } from "../recognition_titles";
import { decodeQuestionId, decodeQuestionTitle, field, requireOnlyFields } from "./shared";

function decodeTitleItems(
  value: unknown,
  path: string,
  requested: ReadonlySet<string>,
): Map<string, string> {
  const items = decodeArray(value, path, (item, itemPath) => {
    const record = decodeRecord(item, itemPath);
    requireOnlyFields(record, itemPath, ["publicId", "title"]);
    return {
      publicId: decodeQuestionId(field(record, "publicId", itemPath), `${itemPath}.publicId`),
      title: decodeQuestionTitle(field(record, "title", itemPath), `${itemPath}.title`),
    };
  });
  if (items.length > RECOGNITION_TITLE_BATCH_LIMIT) {
    throw new DecodeError(path, `at most ${RECOGNITION_TITLE_BATCH_LIMIT} titles`);
  }
  const titles = new Map<string, string>();
  for (const [index, item] of items.entries()) {
    if (!requested.has(item.publicId) || titles.has(item.publicId)) {
      throw new DecodeError(`${path}[${index}].publicId`, "a requested public ID once");
    }
    titles.set(item.publicId, item.title);
  }
  return titles;
}

/** ASVS 1.5.2: allow only the titles requested for this batch. */
export function decodeRecognitionTitles(
  value: unknown,
  requestedQuestions: ReadonlySet<string>,
  requestedPools: ReadonlySet<string>,
  path = "response",
): RecognitionTitleMaps {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questions", "pools"]);
  return {
    questions: decodeTitleItems(
      field(record, "questions", path),
      `${path}.questions`,
      requestedQuestions,
    ),
    pools: decodeTitleItems(field(record, "pools", path), `${path}.pools`, requestedPools),
  };
}
