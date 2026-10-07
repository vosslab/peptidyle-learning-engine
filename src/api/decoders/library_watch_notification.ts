// Strict decoder for the self-only private Library Watch inbox.

import {
  DecodeError,
  decodeArray,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeStringEnum,
} from "../decoder";
import { decodeLibraryObjectId, field, requireOnlyFields } from "./shared";
import type {
  LibraryWatchEventKind,
  LibraryWatchNotification,
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

export function decodeLibraryWatchNotifications(
  value: unknown,
  path = "response",
): ReadonlyArray<LibraryWatchNotification> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["notifications"]);
  return decodeArray(field(record, "notifications", path), `${path}.notifications`, notification);
}
