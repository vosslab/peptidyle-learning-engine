// Strict browser decoders for generated Course teaching-operation DTOs.

import { MAX_ASSIGNMENT_ATTEMPT_LIMIT } from "../../../generated/api/MAX_ASSIGNMENT_ATTEMPT_LIMIT";
import { MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS } from "../../../generated/api/MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS";
import { MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS } from "../../../generated/api/MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS";
import { MAX_TEACHING_PAGE_SIZE } from "../../../generated/api/MAX_TEACHING_PAGE_SIZE";
import type { HypotheticalStudentViewScenarioModifiers } from "../../../generated/api/HypotheticalStudentViewScenarioModifiers";
import type { InstructorCourseInvitationCreateRequest } from "../../../generated/api/InstructorCourseInvitationCreateRequest";
import type { CourseInvitationTerminalActionRequest } from "../../../generated/api/CourseInvitationTerminalActionRequest";
import type { InstructorCourseInvitationsPage } from "../../../generated/api/InstructorCourseInvitationsPage";
import type { InstructorMembershipRemovalRequest } from "../../../generated/api/InstructorMembershipRemovalRequest";
import type { InstructorMembershipsPage } from "../../../generated/api/InstructorMembershipsPage";
import type { PendingCourseInvitationsPage } from "../../../generated/api/PendingCourseInvitationsPage";
import type { TeachingAccountView } from "../../../generated/api/TeachingAccountView";
import type { CourseInvitationTargetView } from "../../../generated/api/CourseInvitationTargetView";
import type { CourseInvitationTargetSearchPage } from "../../../generated/api/CourseInvitationTargetSearchPage";
import type { CourseInvitationTargetSearchRequest } from "../../../generated/api/CourseInvitationTargetSearchRequest";
import {
  DecodeError,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import {
  decodeBoundedArray,
  decodeCursor,
  decodeTimestamp,
  field,
  requireOnlyFields,
} from "./shared";

const MAX_ROUTE_REFERENCE = 2_147_483_647;
const MEMBERSHIP_STATUSES = ["active", "revoked"] as const;
const INVITATION_STATES = ["pending", "expired", "accepted", "declined", "revoked"] as const;

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

function boundedTrimmedText(value: unknown, path: string, maximum: number): string {
  const text = decodeString(value, path);
  if (text.trim() !== text || text.trim().length === 0 || Array.from(text).length > maximum) {
    throw new DecodeError(path, `trimmed nonblank text no longer than ${maximum} Unicode scalars`);
  }
  return text;
}

function reference(value: unknown, path: string, prefix: string): string {
  const parsed = decodeString(value, path);
  const pattern = new RegExp(`^${prefix}-[1-9][0-9]{0,9}$`, "u");
  if (!pattern.test(parsed)) throw new DecodeError(path, `a ${prefix}- prefixed route reference`);
  const number = Number(parsed.slice(prefix.length + 1));
  if (!Number.isSafeInteger(number) || number > MAX_ROUTE_REFERENCE) {
    throw new DecodeError(path, "a positive 31-bit route reference");
  }
  return parsed;
}

function canonicalPositivePostgresBigint(value: unknown, path: string): string {
  const parsed = decodeString(value, path);
  if (!/^[1-9][0-9]{0,18}$/u.test(parsed) || BigInt(parsed) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a canonical positive PostgreSQL bigint decimal");
  }
  return parsed;
}

function pageCursor(value: unknown, path: string): string | null {
  return decodeNullable(value, path, decodeCursor);
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
): HypotheticalStudentViewScenarioModifiers["adjustment"]["assignment_attempt_time_limit_seconds"] {
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
    "assignment_attempt_time_limit_seconds",
    "attempt_limit",
  ]);
  return {
    available_at: timePatch(record.available_at, `${path}.available_at`),
    due_at: timePatch(record.due_at, `${path}.due_at`),
    closes_at: timePatch(record.closes_at, `${path}.closes_at`),
    assignment_attempt_time_limit_seconds: limitPatch(
      record.assignment_attempt_time_limit_seconds,
      `${path}.assignment_attempt_time_limit_seconds`,
      MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS,
    ),
    attempt_limit: limitPatch(
      record.attempt_limit,
      `${path}.attempt_limit`,
      MAX_ASSIGNMENT_ATTEMPT_LIMIT,
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

function teachingAccount(value: unknown, path: string): TeachingAccountView {
  const record = closed(value, path, ["reference", "display"]);
  return {
    reference: reference(record.reference, `${path}.reference`, "U"),
    display: boundedTrimmedText(
      record.display,
      `${path}.display`,
      MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS,
    ),
  };
}

export function decodeInstructorMembershipsPage(
  value: unknown,
  path = "response",
): InstructorMembershipsPage {
  const record = closed(value, path, ["instructors", "nextCursor", "rosterChangeNumber"]);
  return {
    instructors: decodeBoundedArray(
      record.instructors,
      `${path}.instructors`,
      MAX_TEACHING_PAGE_SIZE,
      (entry, entryPath) => {
        const instructor = closed(entry, entryPath, ["membership", "account", "status"]);
        return {
          membership: reference(instructor.membership, `${entryPath}.membership`, "M"),
          account: teachingAccount(instructor.account, `${entryPath}.account`),
          status: decodeStringEnum(instructor.status, `${entryPath}.status`, MEMBERSHIP_STATUSES),
        };
      },
    ),
    nextCursor: pageCursor(record.nextCursor, `${path}.nextCursor`),
    rosterChangeNumber: canonicalPositivePostgresBigint(
      record.rosterChangeNumber,
      `${path}.rosterChangeNumber`,
    ),
  };
}

export function decodeInstructorCourseInvitationCreateRequest(
  value: unknown,
  path = "request",
): InstructorCourseInvitationCreateRequest {
  const record = closed(value, path, ["target"]);
  return {
    target: reference(record.target, `${path}.target`, "U"),
  };
}

function courseInvitationTarget(value: unknown, path: string): CourseInvitationTargetView {
  const record = closed(value, path, ["account"]);
  return {
    account: teachingAccount(record.account, `${path}.account`),
  };
}

/** Decode the bounded safe-picker search page without accepting account PII. */
export function decodeCourseInvitationTargetSearchPage(
  value: unknown,
  path = "response",
): CourseInvitationTargetSearchPage {
  const record = closed(value, path, ["targets", "nextCursor"]);
  return {
    targets: decodeBoundedArray(
      record.targets,
      `${path}.targets`,
      MAX_TEACHING_PAGE_SIZE,
      courseInvitationTarget,
    ),
    nextCursor: pageCursor(record.nextCursor, `${path}.nextCursor`),
  };
}

/** Decode the strict display-name-only target-search request before URL serialization. */
export function decodeCourseInvitationTargetSearchRequest(
  value: unknown,
  path = "request",
): CourseInvitationTargetSearchRequest {
  const record = closed(value, path, ["query", "after", "size"]);
  const query = boundedTrimmedText(record.query, `${path}.query`, 100);
  if (Array.from(query).length < 2) {
    throw new DecodeError(`${path}.query`, "a trimmed search query of 2 to 100 Unicode scalars");
  }
  return {
    query,
    after: pageCursor(record.after, `${path}.after`),
    size: positiveInteger(record.size, `${path}.size`, MAX_TEACHING_PAGE_SIZE),
  };
}

export function decodeInstructorCourseInvitationsPage(
  value: unknown,
  path = "response",
): InstructorCourseInvitationsPage {
  const record = closed(value, path, ["invitations", "nextCursor"]);
  return {
    invitations: decodeBoundedArray(
      record.invitations,
      `${path}.invitations`,
      MAX_TEACHING_PAGE_SIZE,
      (entry, entryPath) => {
        const invitation = closed(entry, entryPath, [
          "reference",
          "target",
          "state",
          "createdAt",
          "expiresAt",
          "state_precondition",
        ]);
        return {
          reference: reference(invitation.reference, `${entryPath}.reference`, "CI"),
          target: courseInvitationTarget(invitation.target, `${entryPath}.target`),
          state: decodeStringEnum(invitation.state, `${entryPath}.state`, INVITATION_STATES),
          createdAt: decodeTimestamp(invitation.createdAt, `${entryPath}.createdAt`),
          expiresAt: decodeTimestamp(invitation.expiresAt, `${entryPath}.expiresAt`),
          state_precondition: canonicalPositivePostgresBigint(
            invitation.state_precondition,
            `${entryPath}.state_precondition`,
          ),
        };
      },
    ),
    nextCursor: pageCursor(record.nextCursor, `${path}.nextCursor`),
  };
}

export function decodePendingCourseInvitationsPage(
  value: unknown,
  path = "response",
): PendingCourseInvitationsPage {
  const record = closed(value, path, ["invitations", "nextCursor"]);
  return {
    invitations: decodeBoundedArray(
      record.invitations,
      `${path}.invitations`,
      MAX_TEACHING_PAGE_SIZE,
      (entry, entryPath) => {
        const invitation = closed(entry, entryPath, [
          "reference",
          "courseLabel",
          "state",
          "expiresAt",
          "state_precondition",
        ]);
        return {
          reference: reference(invitation.reference, `${entryPath}.reference`, "CI"),
          courseLabel: boundedTrimmedText(
            invitation.courseLabel,
            `${entryPath}.courseLabel`,
            MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS,
          ),
          state: decodeStringEnum(invitation.state, `${entryPath}.state`, INVITATION_STATES),
          expiresAt: decodeTimestamp(invitation.expiresAt, `${entryPath}.expiresAt`),
          state_precondition: canonicalPositivePostgresBigint(
            invitation.state_precondition,
            `${entryPath}.state_precondition`,
          ),
        };
      },
    ),
    nextCursor: pageCursor(record.nextCursor, `${path}.nextCursor`),
  };
}

export function decodeCourseInvitationTerminalActionRequest(
  value: unknown,
  path = "request",
): CourseInvitationTerminalActionRequest {
  const record = closed(value, path, ["action"]);
  return {
    action: decodeStringEnum(record.action, `${path}.action`, ["accept", "decline"] as const),
  };
}

export function decodeInstructorMembershipRemovalRequest(
  value: unknown,
  path = "request",
): InstructorMembershipRemovalRequest {
  closed(value, path, []);
  return {};
}
