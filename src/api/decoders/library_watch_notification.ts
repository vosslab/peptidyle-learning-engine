// Strict decoder for the self-only private Library Watch inbox.

import {
  DecodeError,
  decodeArray,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeStringEnum,
  decodeString,
  decodeUuid,
} from "../decoder";
import { decodeLibraryObjectId, field, requireOnlyFields } from "./shared";
import type {
  LibraryWatchEventKind,
  LibraryWatchNotification,
  LibraryWatchPage,
  LibraryWatchTargetKind,
} from "../library_watch_notification";

const TARGET_KINDS = [
  "question",
  "questionPool",
] as const satisfies ReadonlyArray<LibraryWatchTargetKind>;
const EVENT_KINDS = [
  "revision",
  "membersChanged",
  "fork",
] as const satisfies ReadonlyArray<LibraryWatchEventKind>;

function positiveInteger(value: unknown, path: string): number {
  const parsed = decodeSafeInteger(value, path);
  if (parsed < 1) throw new DecodeError(path, "a positive integer");
  return parsed;
}

function timestamp(value: unknown, path: string): number {
  const parsed = decodeSafeInteger(value, path);
  if (parsed < 0) throw new DecodeError(path, "a nonnegative millisecond timestamp");
  return parsed;
}

function notification(value: unknown, path: string): LibraryWatchNotification {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "targetKind",
    "eventId",
    "targetPublicId",
    "eventKind",
    "questionRevisionNumber",
    "questionPoolEditNumber",
    "forkedPublicId",
    "occurredAt",
  ]);
  const eventKind = decodeStringEnum(
    field(record, "eventKind", path),
    `${path}.eventKind`,
    EVENT_KINDS,
  );
  const questionRevisionNumber = decodeNullable(
    field(record, "questionRevisionNumber", path),
    `${path}.questionRevisionNumber`,
    positiveInteger,
  );
  const questionPoolEditNumber = decodeNullable(
    field(record, "questionPoolEditNumber", path),
    `${path}.questionPoolEditNumber`,
    positiveInteger,
  );
  const forkedPublicId = decodeNullable(
    field(record, "forkedPublicId", path),
    `${path}.forkedPublicId`,
    decodeLibraryObjectId,
  );
  const common = {
    eventId: decodeUuid(field(record, "eventId", path), `${path}.eventId`),
    targetKind: decodeStringEnum(
      field(record, "targetKind", path),
      `${path}.targetKind`,
      TARGET_KINDS,
    ),
    targetPublicId: decodeLibraryObjectId(
      field(record, "targetPublicId", path),
      `${path}.targetPublicId`,
    ),
    occurredAt: timestamp(field(record, "occurredAt", path), `${path}.occurredAt`),
  };
  switch (eventKind) {
    case "revision":
      if (
        common.targetKind !== "question" ||
        questionRevisionNumber === null ||
        questionPoolEditNumber !== null ||
        forkedPublicId !== null
      ) {
        throw new DecodeError(path, "a Question Revision event with Revision evidence only");
      }
      return {
        ...common,
        targetKind: "question",
        eventKind,
        questionRevisionNumber,
        questionPoolEditNumber,
        forkedPublicId,
      };
    case "membersChanged":
      if (
        common.targetKind !== "questionPool" ||
        questionRevisionNumber !== null ||
        questionPoolEditNumber === null ||
        forkedPublicId !== null
      ) {
        throw new DecodeError(path, "a Question Pool membership edit with its Edit Number only");
      }
      return {
        ...common,
        targetKind: "questionPool",
        eventKind,
        questionRevisionNumber,
        questionPoolEditNumber,
        forkedPublicId,
      };
    case "fork":
      if (forkedPublicId === null) {
        throw new DecodeError(path, "a fork event with source number and fork evidence");
      }
      if (
        common.targetKind === "question" &&
        questionRevisionNumber !== null &&
        questionPoolEditNumber === null
      ) {
        return {
          ...common,
          targetKind: "question",
          eventKind,
          questionRevisionNumber,
          questionPoolEditNumber,
          forkedPublicId,
        };
      }
      if (
        common.targetKind === "questionPool" &&
        questionRevisionNumber === null &&
        questionPoolEditNumber !== null
      ) {
        return {
          ...common,
          targetKind: "questionPool",
          eventKind,
          questionRevisionNumber,
          questionPoolEditNumber,
          forkedPublicId,
        };
      }
      throw new DecodeError(path, "a fork event with the source target's number and fork evidence");
  }
}

function facets(
  value: unknown,
  path: string,
  choices: ReadonlyArray<string>,
): LibraryWatchPage["targetKindFacets"] {
  return decodeArray(value, path, (item, itemPath) => {
    const record = decodeRecord(item, itemPath);
    requireOnlyFields(record, itemPath, ["value", "count"]);
    const key = decodeString(field(record, "value", itemPath), `${itemPath}.value`);
    if (!choices.includes(key)) throw new DecodeError(`${itemPath}.value`, "a Watch facet value");
    const count = decodeSafeInteger(field(record, "count", itemPath), `${itemPath}.count`);
    if (count < 0) throw new DecodeError(`${itemPath}.count`, "a nonnegative count");
    return { value: key, count };
  });
}

export function decodeLibraryWatchPage(value: unknown, path = "response"): LibraryWatchPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "items",
    "pageNumber",
    "pageSize",
    "totalCount",
    "targetKindFacets",
    "eventKindFacets",
  ]);
  const pageNumber = positiveInteger(field(record, "pageNumber", path), `${path}.pageNumber`);
  const pageSize = positiveInteger(field(record, "pageSize", path), `${path}.pageSize`);
  const totalCount = decodeSafeInteger(field(record, "totalCount", path), `${path}.totalCount`);
  if (pageSize > 100) throw new DecodeError(`${path}.pageSize`, "a page size no greater than 100");
  if (totalCount < 0) throw new DecodeError(`${path}.totalCount`, "a nonnegative count");
  const items = decodeArray(field(record, "items", path), `${path}.items`, notification);
  if (items.length > pageSize || (totalCount === 0 && pageNumber !== 1)) {
    throw new DecodeError(path, "a consistent numbered Watch page");
  }
  if (totalCount > 0 && (items.length === 0 || pageNumber > Math.ceil(totalCount / pageSize))) {
    throw new DecodeError(path, "a nonempty page within the Watch result count");
  }
  return {
    items,
    pageNumber,
    pageSize,
    totalCount,
    targetKindFacets: facets(
      field(record, "targetKindFacets", path),
      `${path}.targetKindFacets`,
      TARGET_KINDS,
    ),
    eventKindFacets: facets(
      field(record, "eventKindFacets", path),
      `${path}.eventKindFacets`,
      EVENT_KINDS,
    ),
  };
}
