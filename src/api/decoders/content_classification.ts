// Strict wire decoding; display names never substitute for UUID identities.

import { DecodeError, decodeBoolean, decodeRecord, decodeString } from "../decoder";
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
    const decoded = decodeContentClassificationItem(value, itemPath);
    const uuid = decoded.uuid;
    if (seen.has(uuid)) throw new DecodeError(`${itemPath}.uuid`, "a distinct UUID");
    seen.add(uuid);
    return decoded;
  });
}

export function decodeContentClassificationItem(
  value: unknown,
  path = "response",
): ContentClassificationItem {
  const item = decodeRecord(value, path);
  requireOnlyFields(item, path, ["uuid", "name", "isRetired"]);
  const uuid = decodeClassificationUuid(field(item, "uuid", path), `${path}.uuid`);
  const name = decodeString(field(item, "name", path), `${path}.name`);
  if (name.length === 0 || name !== name.trim()) {
    throw new DecodeError(`${path}.name`, "a nonempty normalized name");
  }
  return {
    uuid,
    name,
    isRetired: decodeBoolean(field(item, "isRetired", path), `${path}.isRetired`),
  };
}

/** Mirrors the bounded normalized-name contract before it reaches the server. */
export function decodeContentDisciplineName(value: unknown, path = "request.name"): string {
  const name = decodeString(value, path).trim();
  if (name.length === 0 || name.length > 120 || /[\p{Cc}]/u.test(name)) {
    throw new DecodeError(path, "a nonempty control-free Discipline name within 120 characters");
  }
  return name;
}
