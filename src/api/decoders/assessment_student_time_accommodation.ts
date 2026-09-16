// Strict decoder for the minimal Instructor accommodation projection.
import type { AssessmentStudentTimeAccommodation } from "../../../generated/api/AssessmentStudentTimeAccommodation";
import {
  DecodeError,
  decodeBoolean,
  decodeFiniteNumber,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../decoder";

export function decodeStudentTimeMultiplier(value: unknown, path: string): number | null {
  const multiplier = decodeNullable(value, path, decodeFiniteNumber);
  if (multiplier !== null && multiplier < 1) throw new DecodeError(path, "a multiplier >= 1");
  return multiplier;
}

export function decodeAccommodationEditNumber(value: unknown, path: string): string | null {
  const number = decodeNullable(value, path, decodeString);
  if (
    number !== null &&
    (!/^[1-9][0-9]*$/u.test(number) || BigInt(number) > 9_223_372_036_854_775_807n)
  ) {
    throw new DecodeError(path, "a canonical accommodation Edit Number");
  }
  return number;
}

export function decodeAssessmentStudentTimeAccommodation(
  value: unknown,
  path = "response",
): AssessmentStudentTimeAccommodation {
  const record = decodeRecord(value, path);
  const keys = [
    "rosterId",
    "timeMultiplier",
    "editNumber",
    "baseDurationSeconds",
    "effectiveDurationSeconds",
    "cappedAt24Hours",
  ];
  if (
    Object.keys(record).length !== keys.length ||
    Object.keys(record).some((key) => !keys.includes(key))
  ) {
    throw new DecodeError(path, "one closed Student time configuration");
  }
  const rosterId = decodeString(record.rosterId, `${path}.rosterId`);
  if (!/^[A-Za-z0-9._-]{1,64}$/u.test(rosterId)) throw new DecodeError(path, "a Course roster ID");
  const timeMultiplier = decodeStudentTimeMultiplier(
    record.timeMultiplier,
    `${path}.timeMultiplier`,
  );
  const editNumber = decodeAccommodationEditNumber(record.editNumber, `${path}.editNumber`);
  const baseDurationSeconds = decodeNullable(
    record.baseDurationSeconds,
    `${path}.baseDurationSeconds`,
    decodePositiveInteger,
  );
  const effectiveDurationSeconds = decodeNullable(
    record.effectiveDurationSeconds,
    `${path}.effectiveDurationSeconds`,
    decodePositiveInteger,
  );
  const cappedAt24Hours = decodeBoolean(record.cappedAt24Hours, `${path}.cappedAt24Hours`);
  if (
    (baseDurationSeconds === null) !== (effectiveDurationSeconds === null) ||
    (baseDurationSeconds !== null && baseDurationSeconds > 43_200) ||
    (effectiveDurationSeconds !== null &&
      (effectiveDurationSeconds > 86_400 ||
        effectiveDurationSeconds < (baseDurationSeconds ?? 1))) ||
    (editNumber === null && timeMultiplier !== null) ||
    (cappedAt24Hours && effectiveDurationSeconds !== 86_400)
  ) {
    throw new DecodeError(path, "a consistent finite-duration Student time configuration");
  }
  return {
    rosterId,
    timeMultiplier,
    editNumber,
    baseDurationSeconds,
    effectiveDurationSeconds,
    cappedAt24Hours,
  };
}
