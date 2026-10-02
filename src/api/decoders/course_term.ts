// Course-term browser-visible API DTO decoders.

import type { CourseTerm } from "../../../generated/api/CourseTerm";
import { DecodeError, decodeRecord, decodeString } from "../decoder";
import { field, requireOnlyFields } from "./shared";

const ACTIVE_LIFETIME_MONTHS = 6;

function exactCalendarDate(value: string): string | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/u.exec(value);
  if (match === null) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const monthLength = monthLengths(year)[month - 1];
  if (year === 0 || monthLength === undefined || day < 1 || day > monthLength) return null;
  return value;
}

function monthLengths(year: number): readonly number[] {
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  return [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
}

function decodeCourseDate(value: unknown, path: string): string {
  const date = decodeString(value, path);
  const exact = exactCalendarDate(date);
  if (exact === null) throw new DecodeError(path, "an exact valid YYYY-MM-DD calendar date");
  return exact;
}

function activeLifetimeEnd(createdOn: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/u.exec(createdOn);
  if (match === null) return createdOn;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const shifted = month - 1 + ACTIVE_LIFETIME_MONTHS;
  const cutoffYear = year + Math.floor(shifted / 12);
  const cutoffMonth = (shifted % 12) + 1;
  const cutoffDay = Math.min(day, monthLengths(cutoffYear)[cutoffMonth - 1] ?? day);
  const cutoff = `${String(cutoffYear).padStart(4, "0")}-${String(cutoffMonth).padStart(2, "0")}-${String(cutoffDay).padStart(2, "0")}`;
  return exactCalendarDate(cutoff) ?? createdOn;
}

/** True when the inclusive end is on or before the UTC calendar date six months after creation. */
export function courseTermFitsActiveLifetime(endDate: string, createdOn: string): boolean {
  const end = exactCalendarDate(endDate);
  const created = exactCalendarDate(createdOn);
  if (end === null || created === null) return false;
  return end <= activeLifetimeEnd(created);
}

/** Strict browser decoder for the course term shared by create inputs and course projections. */
export function decodeCourseTerm(value: unknown, path: string): CourseTerm {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["startDate", "endDate"]);
  const startDate = decodeCourseDate(field(record, "startDate", path), `${path}.startDate`);
  const endDate = decodeCourseDate(field(record, "endDate", path), `${path}.endDate`);
  if (endDate < startDate)
    throw new DecodeError(`${path}.endDate`, "a date on or after the course start date");
  return { startDate, endDate } satisfies CourseTerm;
}
