// Course-term browser-visible API DTO decoders.

import type { CourseTerm } from "../../../generated/api/CourseTerm";
import type { CourseTermValidationFailure } from "../../../generated/api/CourseTermValidationFailure";
import {
  DecodeError,
  decodeNonemptyString,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { field, requireOnlyFields } from "./shared";

function decodeCourseDate(value: unknown, path: string): string {
  const date = decodeString(value, path);
  const match = /^(\d{4})-(\d{2})-(\d{2})$/u.exec(date);
  if (match === null) throw new DecodeError(path, "an exact valid YYYY-MM-DD calendar date");
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const monthLengths = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  const monthLength = monthLengths[month - 1];
  if (year === 0 || monthLength === undefined || day < 1 || day > monthLength)
    throw new DecodeError(path, "an exact valid YYYY-MM-DD calendar date");
  return date;
}

/** Strict browser decoder for the course term shared by create inputs and course projections. */
export function decodeCourseTerm(value: unknown, path: string): CourseTerm {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["startDate", "endDate", "timeZone"]);
  const startDate = decodeCourseDate(field(record, "startDate", path), `${path}.startDate`);
  const endDate = decodeCourseDate(field(record, "endDate", path), `${path}.endDate`);
  if (endDate < startDate)
    throw new DecodeError(`${path}.endDate`, "a date on or after the course start date");
  const timeZone = decodeNonemptyString(field(record, "timeZone", path), `${path}.timeZone`);
  if (timeZone.length > 255 || timeZone.trim() !== timeZone)
    throw new DecodeError(`${path}.timeZone`, "a trimmed IANA time-zone name");
  return { startDate, endDate, timeZone } satisfies CourseTerm;
}

/** Strict bounded refusal decoder for course-term validation responses. */
export function decodeCourseTermValidationFailure(
  value: unknown,
  path = "response",
): CourseTermValidationFailure {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["error", "field", "reason", "message"]);
  const message = decodeNonemptyString(field(record, "message", path), `${path}.message`);
  if (message.length > 160) {
    throw new DecodeError(`${path}.message`, "at most 160 characters");
  }
  return {
    error: decodeStringEnum(field(record, "error", path), `${path}.error`, ["courseTermInvalid"]),
    field: decodeStringEnum(field(record, "field", path), `${path}.field`, [
      "term",
      "startDate",
      "endDate",
      "timeZone",
    ]),
    reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
      "required",
      "invalidCalendarDate",
      "endBeforeStart",
      "unknownCourseTimeZone",
    ]),
    message,
  } satisfies CourseTermValidationFailure;
}
