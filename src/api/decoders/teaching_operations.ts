// Strict browser decoders for generated WP-INST-T2 teaching-operation DTOs.

import { MAX_ASSIGNMENT_ATTEMPT_LIMIT } from "../../../generated/api/MAX_ASSIGNMENT_ATTEMPT_LIMIT";
import { MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS } from "../../../generated/api/MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS";
import { MAX_RETENTION_EXTENSION_DAYS } from "../../../generated/api/MAX_RETENTION_EXTENSION_DAYS";
import { MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS } from "../../../generated/api/MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS";
import { MAX_TEACHING_PAGE_SIZE } from "../../../generated/api/MAX_TEACHING_PAGE_SIZE";
import type { AccountApprovalView } from "../../../generated/api/AccountApprovalView";
import type { SysadminInstructorApprovalView } from "../../../generated/api/SysadminInstructorApprovalView";
import type { SysadminInstructorCandidateSearchPage } from "../../../generated/api/SysadminInstructorCandidateSearchPage";
import type { SysadminInstructorCandidateSearchRequest } from "../../../generated/api/SysadminInstructorCandidateSearchRequest";
import type { SysadminInstructorCandidateView } from "../../../generated/api/SysadminInstructorCandidateView";
import type { SyntheticPreviewAccommodationAdjustmentRequest } from "../../../generated/api/SyntheticPreviewAccommodationAdjustmentRequest";
import type { InstructorCourseInvitationCreateRequest } from "../../../generated/api/InstructorCourseInvitationCreateRequest";
import type { CourseInvitationTerminalActionRequest } from "../../../generated/api/CourseInvitationTerminalActionRequest";
import type { InstructorCourseInvitationsPage } from "../../../generated/api/InstructorCourseInvitationsPage";
import type { AccommodationAdjustmentUpdateRequest } from "../../../generated/api/AccommodationAdjustmentUpdateRequest";
import type { InstructorMembershipRemovalRequest } from "../../../generated/api/InstructorMembershipRemovalRequest";
import type { InstructorMembershipsPage } from "../../../generated/api/InstructorMembershipsPage";
import type { PendingCourseInvitationsPage } from "../../../generated/api/PendingCourseInvitationsPage";
import type { RetentionActionResponse } from "../../../generated/api/RetentionActionResponse";
import type { RetentionArchiveRequest } from "../../../generated/api/RetentionArchiveRequest";
import type { RetentionExtendRequest } from "../../../generated/api/RetentionExtendRequest";
import type { RetentionReadView } from "../../../generated/api/RetentionReadView";
import type { TeachingPreviewView } from "../../../generated/api/TeachingPreviewView";
import type { TeachingOperationRevisionResponse } from "../../../generated/api/TeachingOperationRevisionResponse";
import type { TeachingAccountView } from "../../../generated/api/TeachingAccountView";
import type { CourseInvitationTargetView } from "../../../generated/api/CourseInvitationTargetView";
import type { CourseInvitationTargetSearchPage } from "../../../generated/api/CourseInvitationTargetSearchPage";
import type { CourseInvitationTargetSearchRequest } from "../../../generated/api/CourseInvitationTargetSearchRequest";
import type { CourseStudentMembershipsPage } from "../../../generated/api/CourseStudentMembershipsPage";
import type { StudentMembershipView } from "../../../generated/api/StudentMembershipView";
import type { AssignmentPolicySource } from "../../../generated/api/AssignmentPolicySource";
import type { TeachingPreviewLimitField } from "../../../generated/api/TeachingPreviewLimitField";
import type { TeachingPreviewTimeField } from "../../../generated/api/TeachingPreviewTimeField";
import type { RetentionDispositionView } from "../../../generated/api/RetentionDispositionView";
import type { RetentionNotificationView } from "../../../generated/api/RetentionNotificationView";
import type { RetentionStateView } from "../../../generated/api/RetentionStateView";
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

function studentMembership(value: unknown, path: string): StudentMembershipView {
  const record = closed(value, path, ["reference", "display", "role", "status"]);
  return {
    reference: reference(record.reference, `${path}.reference`, "M"),
    display: boundedTrimmedText(
      record.display,
      `${path}.display`,
      MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS,
    ),
    role: decodeStringEnum(record.role, `${path}.role`, ["instructor", "student"] as const),
    status: decodeStringEnum(record.status, `${path}.status`, MEMBERSHIP_STATUSES),
  };
}

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

function revision(value: unknown, path: string): string {
  const parsed = decodeString(value, path);
  if (!/^[1-9][0-9]{0,18}$/u.test(parsed) || BigInt(parsed) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a canonical positive PostgreSQL bigint revision");
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

function courseTimeZone(value: unknown, path: string): string {
  const timeZone = boundedTrimmedText(value, path, 255);
  return timeZone;
}

/** Decode the common exact strong-revision response for accepted M2--M4 mutations. */
export function decodeTeachingOperationRevisionResponse(
  value: unknown,
  path = "response",
): TeachingOperationRevisionResponse {
  const record = closed(value, path, ["revision"]);
  return { revision: revision(record.revision, `${path}.revision`) };
}

function timePatch(
  value: unknown,
  path: string,
): SyntheticPreviewAccommodationAdjustmentRequest["adjustment"]["availableAt"] {
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
): SyntheticPreviewAccommodationAdjustmentRequest["adjustment"]["assignmentAttemptTimeLimitSeconds"] {
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
): SyntheticPreviewAccommodationAdjustmentRequest["adjustment"] {
  const record = closed(value, path, [
    "availableAt",
    "dueAt",
    "closesAt",
    "assignmentAttemptTimeLimitSeconds",
    "attemptLimit",
  ]);
  return {
    availableAt: timePatch(record.availableAt, `${path}.availableAt`),
    dueAt: timePatch(record.dueAt, `${path}.dueAt`),
    closesAt: timePatch(record.closesAt, `${path}.closesAt`),
    assignmentAttemptTimeLimitSeconds: limitPatch(
      record.assignmentAttemptTimeLimitSeconds,
      `${path}.assignmentAttemptTimeLimitSeconds`,
      MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS,
    ),
    attemptLimit: limitPatch(
      record.attemptLimit,
      `${path}.attemptLimit`,
      MAX_ASSIGNMENT_ATTEMPT_LIMIT,
    ),
  };
}

function decodeAccommodationAdjustmentWrite(
  value: unknown,
  path: string,
): SyntheticPreviewAccommodationAdjustmentRequest {
  const record = closed(value, path, ["mode", "adjustment"]);
  return {
    mode: decodeStringEnum(record.mode, `${path}.mode`, ["extendOnly", "replace"] as const),
    adjustment: accommodationAdjustment(record.adjustment, `${path}.adjustment`),
  };
}

export function decodeSyntheticPreviewAccommodationAdjustmentRequest(
  value: unknown,
  path = "request",
): SyntheticPreviewAccommodationAdjustmentRequest {
  return decodeAccommodationAdjustmentWrite(value, path);
}

export function decodeAccommodationAdjustmentUpdateRequest(
  value: unknown,
  path = "request",
): AccommodationAdjustmentUpdateRequest {
  return decodeAccommodationAdjustmentWrite(value, path);
}

function assignmentPolicySource(value: unknown, path: string): AssignmentPolicySource {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  switch (kind) {
    case "base":
      requireOnlyFields(record, path, ["kind", "label"]);
      return {
        kind,
        label: boundedTrimmedText(
          field(record, "label", path),
          `${path}.label`,
          MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS,
        ),
      };
    case "accommodation":
      requireOnlyFields(record, path, ["kind", "membership", "label"]);
      return {
        kind,
        membership: reference(field(record, "membership", path), `${path}.membership`, "M"),
        label: boundedTrimmedText(
          field(record, "label", path),
          `${path}.label`,
          MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS,
        ),
      };
    default:
      throw new DecodeError(`${path}.kind`, "a known Assignment Policy Source kind");
  }
}

function previewTimeField(value: unknown, path: string): TeachingPreviewTimeField {
  const record = closed(value, path, ["value", "source"]);
  return {
    value: decodeNullable(record.value, `${path}.value`, courseLocalDateAndTime),
    source: assignmentPolicySource(record.source, `${path}.source`),
  };
}

function previewLimitField(value: unknown, path: string): TeachingPreviewLimitField {
  const record = closed(value, path, ["value", "source"]);
  return {
    value: decodeNullable(record.value, `${path}.value`, (entry, entryPath) =>
      positiveInteger(entry, entryPath, MAX_ASSIGNMENT_ATTEMPT_TIME_LIMIT_SECONDS),
    ),
    source: assignmentPolicySource(record.source, `${path}.source`),
  };
}

function startVerdict(
  value: unknown,
  path: string,
): Extract<TeachingPreviewView, { active_student_course_membership: "allowed" }>["start"] {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  switch (kind) {
    case "mayStart":
      requireOnlyFields(record, path, ["kind", "late"]);
      return {
        kind,
        late: decodeStringEnum(field(record, "late", path), `${path}.late`, [
          "onTime",
          "acceptedLate",
          "markedLate",
        ] as const),
      };
    case "notYetAvailable":
    case "closed":
    case "attemptLimitReached":
    case "lateWorkRefused":
      requireOnlyFields(record, path, ["kind"]);
      return { kind };
    default:
      throw new DecodeError(`${path}.kind`, "a known start verdict");
  }
}

export function decodeTeachingPreviewView(value: unknown, path = "response"): TeachingPreviewView {
  const record = decodeRecord(value, path);
  const active_student_course_membership = decodeString(
    field(record, "active_student_course_membership", path),
    `${path}.active_student_course_membership`,
  );
  if (active_student_course_membership === "denied") {
    requireOnlyFields(record, path, ["active_student_course_membership", "reason"]);
    return {
      active_student_course_membership,
      reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
        "activeStudentCourseMembershipRequired",
      ] as const),
    };
  }
  if (active_student_course_membership !== "allowed")
    throw new DecodeError(`${path}.active_student_course_membership`, "allowed or denied");
  requireOnlyFields(record, path, [
    "active_student_course_membership",
    "timeZone",
    "start",
    "availableAt",
    "dueAt",
    "closesAt",
    "assignmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "lateWorkRule",
    "assignmentDeadlineRule",
  ]);
  const lateWorkRule = closed(field(record, "lateWorkRule", path), `${path}.lateWorkRule`, [
    "value",
    "source",
  ]);
  const assignmentDeadlineRule = closed(
    field(record, "assignmentDeadlineRule", path),
    `${path}.assignmentDeadlineRule`,
    ["value", "source"],
  );
  return {
    active_student_course_membership,
    timeZone: courseTimeZone(field(record, "timeZone", path), `${path}.timeZone`),
    start: startVerdict(field(record, "start", path), `${path}.start`),
    availableAt: previewTimeField(field(record, "availableAt", path), `${path}.availableAt`),
    dueAt: previewTimeField(field(record, "dueAt", path), `${path}.dueAt`),
    closesAt: previewTimeField(field(record, "closesAt", path), `${path}.closesAt`),
    assignmentAttemptTimeLimitSeconds: previewLimitField(
      field(record, "assignmentAttemptTimeLimitSeconds", path),
      `${path}.assignmentAttemptTimeLimitSeconds`,
    ),
    attemptLimit: previewLimitField(field(record, "attemptLimit", path), `${path}.attemptLimit`),
    lateWorkRule: {
      value: decodeStringEnum(lateWorkRule.value, `${path}.lateWorkRule.value`, [
        "accept",
        "markLate",
        "reject",
      ] as const),
      source: assignmentPolicySource(lateWorkRule.source, `${path}.lateWorkRule.source`),
    },
    assignmentDeadlineRule: {
      value: decodeStringEnum(
        assignmentDeadlineRule.value,
        `${path}.assignmentDeadlineRule.value`,
        ["autoSubmit"] as const,
      ),
      source: assignmentPolicySource(
        assignmentDeadlineRule.source,
        `${path}.assignmentDeadlineRule.source`,
      ),
    },
  };
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

export function decodeAccountApprovalView(value: unknown, path = "response"): AccountApprovalView {
  const record = closed(value, path, ["state", "revision"]);
  return {
    state: decodeStringEnum(record.state, `${path}.state`, ["approved", "revoked"] as const),
    revision: revision(record.revision, `${path}.revision`),
  };
}

function sysadminInstructorApproval(value: unknown, path: string): SysadminInstructorApprovalView {
  const record = closed(value, path, ["state", "revision"]);
  const state = decodeStringEnum(record.state, `${path}.state`, [
    "unapproved",
    "approved",
    "revoked",
  ] as const);
  const candidateRevision = decodeNullable(record.revision, `${path}.revision`, revision);
  if ((state === "unapproved") !== (candidateRevision === null)) {
    throw new DecodeError(
      path,
      "an unapproved state with no revision or a recorded state with a revision",
    );
  }
  return { state, revision: candidateRevision };
}

function sysadminInstructorCandidate(
  value: unknown,
  path: string,
): SysadminInstructorCandidateView {
  const record = closed(value, path, ["account", "approval"]);
  return {
    account: teachingAccount(record.account, `${path}.account`),
    approval: sysadminInstructorApproval(record.approval, `${path}.approval`),
  };
}

/** Decode the Sysadmin-only safe candidate page without accepting account identity or tenancy data. */
export function decodeSysadminInstructorCandidateSearchPage(
  value: unknown,
  path = "response",
): SysadminInstructorCandidateSearchPage {
  const record = closed(value, path, ["candidates", "nextCursor"]);
  const candidates = decodeBoundedArray(
    record.candidates,
    `${path}.candidates`,
    MAX_TEACHING_PAGE_SIZE,
    sysadminInstructorCandidate,
  );
  const references = candidates.map((candidate) => candidate.account.reference);
  if (new Set(references).size !== references.length) {
    throw new DecodeError(`${path}.candidates`, "unique account references");
  }
  return {
    candidates,
    nextCursor: pageCursor(record.nextCursor, `${path}.nextCursor`),
  };
}

/** Decode the sole bounded display-label search input before Sysadmin URL serialization. */
export function decodeSysadminInstructorCandidateSearchRequest(
  value: unknown,
  path = "request",
): SysadminInstructorCandidateSearchRequest {
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

export function decodeInstructorMembershipsPage(
  value: unknown,
  path = "response",
): InstructorMembershipsPage {
  const record = closed(value, path, ["instructors", "nextCursor", "rosterRevision"]);
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
    rosterRevision: revision(record.rosterRevision, `${path}.rosterRevision`),
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
  const record = closed(value, path, ["account", "approval"]);
  return {
    account: teachingAccount(record.account, `${path}.account`),
    approval: decodeAccountApprovalView(record.approval, `${path}.approval`),
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

/** Decode the bounded safe-picker roster page and preserve only student memberships. */
export function decodeCourseStudentMembershipsPage(
  value: unknown,
  path = "response",
): CourseStudentMembershipsPage {
  const record = closed(value, path, ["students", "nextCursor"]);
  const students = decodeBoundedArray(
    record.students,
    `${path}.students`,
    MAX_TEACHING_PAGE_SIZE,
    studentMembership,
  );
  if (!students.every((student) => student.role === "student")) {
    throw new DecodeError(`${path}.students`, "student membership rows");
  }
  return { students, nextCursor: pageCursor(record.nextCursor, `${path}.nextCursor`) };
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
          "revision",
        ]);
        return {
          reference: reference(invitation.reference, `${entryPath}.reference`, "CI"),
          target: courseInvitationTarget(invitation.target, `${entryPath}.target`),
          state: decodeStringEnum(invitation.state, `${entryPath}.state`, INVITATION_STATES),
          createdAt: decodeTimestamp(invitation.createdAt, `${entryPath}.createdAt`),
          expiresAt: decodeTimestamp(invitation.expiresAt, `${entryPath}.expiresAt`),
          revision: revision(invitation.revision, `${entryPath}.revision`),
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
          "revision",
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
          revision: revision(invitation.revision, `${entryPath}.revision`),
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

function retentionState(value: unknown, path: string): RetentionStateView {
  return decodeStringEnum(value, path, [
    "active",
    "notificationDue",
    "studentRecordsArchived",
    "studentRecordsDeleted",
  ] as const);
}

function retentionDisposition(value: unknown, path: string): RetentionDispositionView {
  return decodeStringEnum(value, path, ["retain", "delete"] as const);
}

function retentionNotification(value: unknown, path: string): RetentionNotificationView {
  const decoded = closed(value, path, ["intent", "createdAt", "copy"]);
  return {
    intent: decodeStringEnum(decoded.intent, `${path}.intent`, [
      "archive",
      "delete",
      "extend",
    ] as const),
    createdAt: decodeTimestamp(decoded.createdAt, `${path}.createdAt`),
    copy: decodeString(decoded.copy, `${path}.copy`),
  };
}

export function decodeRetentionReadView(value: unknown, path = "response"): RetentionReadView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["state", "assignmentDefinitions", "revision", "notification"]);
  const notification =
    record.notification === undefined
      ? undefined
      : retentionNotification(record.notification, `${path}.notification`);
  return {
    state: retentionState(field(record, "state", path), `${path}.state`),
    assignmentDefinitions: retentionDisposition(
      field(record, "assignmentDefinitions", path),
      `${path}.assignmentDefinitions`,
    ),
    revision: revision(field(record, "revision", path), `${path}.revision`),
    ...(notification === undefined ? {} : { notification }),
  };
}

export function decodeRetentionArchiveRequest(
  value: unknown,
  path = "request",
): RetentionArchiveRequest {
  const record = closed(value, path, ["assignmentDefinitions"]);
  return {
    assignmentDefinitions: retentionDisposition(
      record.assignmentDefinitions,
      `${path}.assignmentDefinitions`,
    ),
  };
}

export function decodeRetentionExtendRequest(
  value: unknown,
  path = "request",
): RetentionExtendRequest {
  const record = closed(value, path, ["additionalDays"]);
  return {
    additionalDays: positiveInteger(
      record.additionalDays,
      `${path}.additionalDays`,
      MAX_RETENTION_EXTENSION_DAYS,
    ),
  };
}

export function decodeRetentionActionResponse(
  value: unknown,
  path = "response",
): RetentionActionResponse {
  const record = closed(value, path, ["state", "assignmentDefinitions", "revision", "outcome"]);
  return {
    state: retentionState(record.state, `${path}.state`),
    assignmentDefinitions: retentionDisposition(
      record.assignmentDefinitions,
      `${path}.assignmentDefinitions`,
    ),
    revision: revision(record.revision, `${path}.revision`),
    outcome: decodeStringEnum(record.outcome, `${path}.outcome`, [
      "scheduled",
      "inProgress",
      "completed",
    ] as const),
  };
}
