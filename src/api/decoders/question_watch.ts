// Strict decoder for the private one-boolean Question Watch projection.

import { decodeBoolean, decodeRecord } from "../decoder";
import { field, requireOnlyFields } from "./shared";
import type { QuestionWatchProjection } from "../question_watch";

/** Rejects any watcher count, identity, list, activity, or notification field. */
export function decodeQuestionWatchProjection(
  value: unknown,
  path = "response",
): QuestionWatchProjection {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["watching"]);
  return { watching: decodeBoolean(field(record, "watching", path), `${path}.watching`) };
}
