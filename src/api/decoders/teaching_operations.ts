// Strict browser decoders for generated Course teaching-operation DTOs.

import { MAX_ASSESSMENT_ATTEMPT_LIMIT } from "../../../generated/api/MAX_ASSESSMENT_ATTEMPT_LIMIT";
import { MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS } from "../../../generated/api/MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS";
import type { HypotheticalStudentViewScenarioModifiers } from "../../../generated/api/HypotheticalStudentViewScenarioModifiers";
import {
  DecodeError,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { field, requireOnlyFields } from "./shared";

function closed(
  value: unknown,
  path: string,
  keys: ReadonlyArray<string>,
): Record<string, unknown> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, keys);
  for (const key of keys) field(record, key, path);
  return record;
}

function positiveInteger(value: unknown, path: string, maximum: number): number {
  const parsed = decodeSafeInteger(value, path);
  if (parsed < 1 || parsed > maximum) {
    throw new DecodeError(path, `a positive integer no larger than ${maximum}`);
  }
  return parsed;
}

function courseLocalDateAndTime(value: unknown, path: string): string {
  const text = decodeString(value, path);
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})\.(\d{3})$/u.exec(text);
  if (match === null)
    throw new DecodeError(path, "an exact valid YYYY-MM-DDTHH:MM:SS.sss local date-time");
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const hour = Number(match[4]);
  const minute = Number(match[5]);
  const second = Number(match[6]);
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const monthLengths = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  const monthLength = monthLengths[month - 1];
  if (
    year === 0 ||
    monthLength === undefined ||
    day < 1 ||
    day > monthLength ||
    hour > 23 ||
    minute > 59 ||
    second > 59
  ) {
    throw new DecodeError(path, "an exact valid YYYY-MM-DDTHH:MM:SS.sss local date-time");
  }
  return text;
}

function timePatch(
  value: unknown,
  path: string,
): HypotheticalStudentViewScenarioModifiers["adjustment"]["available_at"] {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  switch (kind) {
    case "inherit":
    case "unrestricted":
      requireOnlyFields(record, path, ["kind"]);
      return { kind };
    case "set":
      requireOnlyFields(record, path, ["kind", "value"]);
      return { kind, value: courseLocalDateAndTime(field(record, "value", path), `${path}.value`) };
    default:
      throw new DecodeError(`${path}.kind`, "one of inherit, set, unrestricted");
  }
}

function limitPatch(
  value: unknown,
  path: string,
  maximum: number,
): HypotheticalStudentViewScenarioModifiers["adjustment"]["assessment_attempt_time_limit_seconds"] {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  switch (kind) {
    case "inherit":
    case "unrestricted":
      requireOnlyFields(record, path, ["kind"]);
      return { kind };
    case "set":
      requireOnlyFields(record, path, ["kind", "value"]);
      return {
        kind,
        value: positiveInteger(field(record, "value", path), `${path}.value`, maximum),
      };
    default:
      throw new DecodeError(`${path}.kind`, "one of inherit, set, unrestricted");
  }
}

function accommodationAdjustment(
  value: unknown,
  path: string,
): HypotheticalStudentViewScenarioModifiers["adjustment"] {
  const record = closed(value, path, [
    "available_at",
    "due_at",
    "closes_at",
    "assessment_attempt_time_limit_seconds",
    "attempt_limit",
  ]);
  return {
    available_at: timePatch(record.available_at, `${path}.available_at`),
    due_at: timePatch(record.due_at, `${path}.due_at`),
    closes_at: timePatch(record.closes_at, `${path}.closes_at`),
    assessment_attempt_time_limit_seconds: limitPatch(
      record.assessment_attempt_time_limit_seconds,
      `${path}.assessment_attempt_time_limit_seconds`,
      MAX_ASSESSMENT_ATTEMPT_TIME_LIMIT_SECONDS,
    ),
    attempt_limit: limitPatch(
      record.attempt_limit,
      `${path}.attempt_limit`,
      MAX_ASSESSMENT_ATTEMPT_LIMIT,
    ),
  };
}

function decodeAccommodationAdjustmentWrite(
  value: unknown,
  path: string,
): HypotheticalStudentViewScenarioModifiers {
  const record = closed(value, path, ["mode", "adjustment"]);
  return {
    mode: decodeStringEnum(record.mode, `${path}.mode`, ["extend_only", "replace"] as const),
    adjustment: accommodationAdjustment(record.adjustment, `${path}.adjustment`),
  };
}

export function decodeHypotheticalStudentViewScenarioModifiers(
  value: unknown,
  path = "request",
): HypotheticalStudentViewScenarioModifiers {
  return decodeAccommodationAdjustmentWrite(value, path);
}
