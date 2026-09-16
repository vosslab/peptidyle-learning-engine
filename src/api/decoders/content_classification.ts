// Strict wire decoding; display names never substitute for UUID identities.

import { DecodeError, decodeRecord, decodeString } from "../decoder";
import type { ContentClassificationItem } from "../content_classification";
import { field, requireOnlyFields } from "./shared";

export type ContentClassificationListKey = "disciplines" | "subjects" | "topics" | "subtopics";

/** ASVS 2.2.1: match the server's canonical UUID boundary, without invented version rules. */
export function decodeClassificationUuid(value: unknown, path: string): string {
  const uuid = decodeString(value, path);
  if (!/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/.test(uuid)) {
    throw new DecodeError(path, "a canonical lowercase-hyphenated UUID");
  }
  return uuid;
}

export function decodeContentClassificationList(
  value: unknown,
  key: ContentClassificationListKey,
  path = "response",
): ReadonlyArray<ContentClassificationItem> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [key]);
  const items = field(record, key, path);
  if (!Array.isArray(items)) throw new DecodeError(`${path}.${key}`, "an array");
  const seen = new Set<string>();
  return items.map((value: unknown, index: number) => {
    const itemPath = `${path}.${key}[${index}]`;
    const item = decodeRecord(value, itemPath);
    requireOnlyFields(item, itemPath, ["uuid", "name"]);
    const uuid = decodeClassificationUuid(field(item, "uuid", itemPath), `${itemPath}.uuid`);
    if (seen.has(uuid)) throw new DecodeError(`${itemPath}.uuid`, "a distinct UUID");
    seen.add(uuid);
    const name = decodeString(field(item, "name", itemPath), `${itemPath}.name`);
    if (name.length === 0 || name !== name.trim()) {
      throw new DecodeError(`${itemPath}.name`, "a nonempty normalized name");
    }
    return { uuid, name };
  });
}
