// Strict decoder for the self-only private Library Watch inbox.

import {
  DecodeError,
  decodeArray,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";
import { decodeQuestionId, field, requireOnlyFields } from "./shared";
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
  "fork",
  "improvementThread",
  "impactNotice",
] as const satisfies ReadonlyArray<LibraryWatchEventKind>;

function positiveNullableInteger(value: unknown, path: string): number | null {
  const parsed = decodeNullable(value, path, decodeSafeInteger);
  if (parsed !== null && parsed < 1) throw new DecodeError(path, "a positive integer or null");
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
    "revisionNumber",
    "forkedPublicId",
    "activityId",
    "occurredAt",
  ]);
  const eventKind = decodeStringEnum(
    field(record, "eventKind", path),
    `${path}.eventKind`,
    EVENT_KINDS,
  );
  const revisionNumber = positiveNullableInteger(
    field(record, "revisionNumber", path),
    `${path}.revisionNumber`,
  );
  const forkedPublicId = decodeNullable(
    field(record, "forkedPublicId", path),
    `${path}.forkedPublicId`,
    decodeQuestionId,
  );
  const activityId = decodeNullable(
    field(record, "activityId", path),
    `${path}.activityId`,
    decodeUuid,
  );
  const common = {
    targetKind: decodeStringEnum(
      field(record, "targetKind", path),
      `${path}.targetKind`,
      TARGET_KINDS,
    ),
    targetPublicId: decodeQuestionId(
      field(record, "targetPublicId", path),
      `${path}.targetPublicId`,
    ),
    occurredAt: timestamp(field(record, "occurredAt", path), `${path}.occurredAt`),
  };
  switch (eventKind) {
    case "revision":
      if (revisionNumber === null || forkedPublicId !== null || activityId !== null) {
        throw new DecodeError(path, "a Revision event with Revision evidence only");
      }
      return { ...common, eventKind, revisionNumber, forkedPublicId, activityId };
    case "fork":
      if (revisionNumber === null || forkedPublicId === null || activityId !== null) {
        throw new DecodeError(path, "a fork event with source Revision and fork evidence only");
      }
      return { ...common, eventKind, revisionNumber, forkedPublicId, activityId };
    case "improvementThread":
      if (revisionNumber === null || forkedPublicId !== null || activityId === null) {
        throw new DecodeError(
          path,
          "an improvement-thread event with creation Revision and thread ID",
        );
      }
      return { ...common, eventKind, revisionNumber, forkedPublicId, activityId };
    case "impactNotice":
      if (forkedPublicId !== null || activityId === null) {
        throw new DecodeError(
          path,
          "an impact-notice event with its notice ID and no fork evidence",
        );
      }
      return { ...common, eventKind, revisionNumber, forkedPublicId, activityId };
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
