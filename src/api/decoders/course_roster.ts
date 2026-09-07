// Strict browser decoding for M9 Course Roster routes.

import type {
  CourseRosterEntry,
  CourseRosterImportEntry,
  CourseRosterImportInput,
} from "../course_roster";
import { DecodeError, decodeArray, decodeBoolean, decodeRecord, decodeString } from "../decoder";
import { field, requireOnlyFields } from "./shared";

function rosterId(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^[A-Za-z0-9._-]{1,64}$/u.test(decoded)) {
    throw new DecodeError(path, "a course-scoped roster identifier");
  }
  return decoded;
}

function email(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (decoded.length > 320 || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/u.test(decoded)) {
    throw new DecodeError(path, "a normalized course roster email");
  }
  return decoded;
}

function entry(value: unknown, path: string): CourseRosterEntry {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["rosterId", "rosterEmail", "state"]);
  const state = decodeString(field(record, "state", path), `${path}.state`);
  if (state !== "invitationPending" && state !== "activeStudent") {
    throw new DecodeError(`${path}.state`, "a current Course Roster state");
  }
  return {
    rosterId: rosterId(field(record, "rosterId", path), `${path}.rosterId`),
    rosterEmail: email(field(record, "rosterEmail", path), `${path}.rosterEmail`),
    state,
  };
}

/** Validates a bounded, reviewed Course Roster Import before it leaves the browser. */
export function decodeCourseRosterImportInput(
  value: unknown,
  path = "request",
): CourseRosterImportInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["entries"]);
  const entries = decodeArray(field(record, "entries", path), `${path}.entries`, (row, rowPath) => {
    const rowRecord = decodeRecord(row, rowPath);
    requireOnlyFields(rowRecord, rowPath, ["email", "rosterId"]);
    return {
      email: email(field(rowRecord, "email", rowPath), `${rowPath}.email`),
      rosterId: rosterId(field(rowRecord, "rosterId", rowPath), `${rowPath}.rosterId`),
    } satisfies CourseRosterImportEntry;
  });
  if (entries.length === 0 || entries.length > 50) {
    throw new DecodeError(`${path}.entries`, "between one and fifty roster rows");
  }
  const emails = new Set(entries.map((candidate) => candidate.email.toLowerCase()));
  const rosterIds = new Set(entries.map((candidate) => candidate.rosterId));
  if (emails.size !== entries.length || rosterIds.size !== entries.length) {
    throw new DecodeError(`${path}.entries`, "unique email and roster identifier values");
  }
  return { entries };
}

export function decodeCourseRoster(value: unknown, path = "response"): ReadonlyArray<CourseRosterEntry> {
  return decodeArray(value, path, entry);
}

export function decodeClaimedCourseInvitation(
  value: unknown,
  path = "response",
): { readonly activeStudentMembership: boolean } {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["activeStudentMembership"]);
  return {
    activeStudentMembership: decodeBoolean(
      field(record, "activeStudentMembership", path),
      `${path}.activeStudentMembership`,
    ),
  };
}
