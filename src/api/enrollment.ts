// Passwordless account and course-roster browser contracts.

import type { CourseId } from "../../generated/api/CourseId";
import type { CourseReference } from "../../generated/api/CourseReference";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeField,
  decodeNonemptyString,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeSafeInteger,
  decodeStringEnum,
  decodeUuid,
} from "./decoder";
import { decodeCursor, requireOnlyFields } from "./decoders/shared";
import { parseCourseReference } from "../navigation/public_route";

export type CourseRosterRole = "student";
export type CourseRosterMemberStatus = "active" | "revoked";
export type CourseInvitationState = "pending" | "accepted" | "revoked" | "expired";
/** Coarse delivery state; never evidence that a recipient mailbox received mail. */
export type CourseInvitationEmailDelivery =
  "queued" | "sentToProvider" | "needsAttention" | "cancelled";
export type RosterImportRowStatus =
  "readyToInvite" | "alreadyMember" | "alreadyPending" | "duplicate" | "invalid";
/** Instructor-safe explanation that never repeats invalid CSV cells or account existence. */
export type RosterImportRowReason =
  "ready" | "alreadyOnRoster" | "invitationPending" | "duplicateInFile" | "correctEmailOrRosterId";

export interface ClaimedCourseInvitation {
  readonly courseId: CourseId;
  readonly courseReference: CourseReference;
  readonly membershipStatus: "active";
}

/** One normalized domain condition within a Course Invitation Email Rule. */
export interface CourseInvitationEmailDomain {
  readonly domain: string;
  readonly includeSubdomains: boolean;
}

export interface CourseRosterMember {
  readonly memberId: string;
  readonly displayName: string;
  readonly rosterEmail: string | null;
  readonly rosterId: string | null;
  readonly role: CourseRosterRole;
  readonly status: CourseRosterMemberStatus;
}

export interface PendingCourseInvitation {
  readonly invitationId: string;
  readonly email: string;
  readonly rosterId: string;
  readonly status: CourseInvitationState;
  readonly expiresAt: number;
}

interface CourseRosterPageBase {
  readonly members: ReadonlyArray<CourseRosterMember>;
  readonly nextCursor: string | null;
  readonly rosterRevision: number;
}

/** Course invitations are available only from a configured email-capable composition. */
export interface CourseInvitationRosterPage extends CourseRosterPageBase {
  readonly rosterMode: "courseInvitations";
  readonly pendingInvitations: ReadonlyArray<PendingCourseInvitation>;
  readonly allowedEmailDomains: ReadonlyArray<CourseInvitationEmailDomain>;
}

export type CourseRosterPage = CourseInvitationRosterPage;

export interface CourseInvitationAccepted {
  readonly invitation: PendingCourseInvitation;
  readonly redemptionPath: string;
  readonly emailDelivery: CourseInvitationEmailDelivery;
}

export interface CourseInvitationEmailRule {
  readonly allowedEmailDomains: ReadonlyArray<CourseInvitationEmailDomain>;
  readonly rosterRevision: number;
}

export interface RosterRevisionResult {
  readonly rosterRevision: number;
}

export interface RosterImportRow {
  readonly rowNumber: number;
  readonly email: string | null;
  readonly rosterId: string | null;
  readonly status: RosterImportRowStatus;
  readonly reason: RosterImportRowReason;
}

export interface RosterImportPreview {
  readonly importId: string;
  readonly state: "preview" | "committed";
  readonly expiresAt: number;
  readonly rosterRevision: number;
  readonly importRevision: number;
  readonly rows: ReadonlyArray<RosterImportRow>;
}

export interface RosterImportCommitResult {
  readonly importId: string;
  readonly importRevision: number;
  readonly rosterRevision: number;
  readonly invitationsCreated: number;
  readonly delivery: ReadonlyArray<RosterImportDelivery>;
}

/** Bulk result keyed only by CSV row and one coarse delivery outcome. */
export interface RosterImportDelivery {
  readonly rowNumber: number;
  readonly outcome: CourseInvitationEmailDelivery;
}

export interface CourseRosterClient {
  readonly redeemCourseInvitation: (token: string) => Promise<ClaimedCourseInvitation>;
  readonly listCourseRoster: (courseId: CourseId, cursor?: string) => Promise<CourseRosterPage>;
  readonly inviteCourseMember: (
    courseId: CourseId,
    email: string,
    rosterId: string,
    idempotencyKey: string,
  ) => Promise<CourseInvitationAccepted>;
  readonly revokeCourseInvitation: (
    courseId: CourseId,
    invitationId: string,
    rosterRevision: number,
  ) => Promise<RosterRevisionResult>;
  readonly revokeCourseMember: (
    courseId: CourseId,
    memberId: string,
    rosterRevision: number,
  ) => Promise<RosterRevisionResult>;
  readonly replaceCourseInvitationEmailRule: (
    courseId: CourseId,
    rule: Omit<CourseInvitationEmailRule, "rosterRevision">,
    rosterRevision: number,
  ) => Promise<CourseInvitationEmailRule>;
  readonly previewRosterImport: (
    courseId: CourseId,
    csv: Blob,
    rosterRevision: number,
    idempotencyKey: string,
  ) => Promise<RosterImportPreview>;
  readonly commitRosterImport: (
    courseId: CourseId,
    preview: Pick<RosterImportPreview, "importId" | "importRevision">,
    rowNumbers: ReadonlyArray<number>,
    idempotencyKey: string,
  ) => Promise<RosterImportCommitResult>;
}

function field(record: Record<string, unknown>, key: string, path: string): unknown {
  return decodeField(record, key, path);
}

function positiveRevision(value: unknown, path: string): number {
  const revision = decodePositiveInteger(value, path);
  if (revision > Number.MAX_SAFE_INTEGER) throw new DecodeError(path, "a safe positive revision");
  return revision;
}

export function decodeClaimedCourseInvitation(
  value: unknown,
  path = "response",
): ClaimedCourseInvitation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["courseId", "courseReference", "membershipStatus"]);
  const reference = field(record, "courseReference", path);
  if (typeof reference !== "string")
    throw new DecodeError(`${path}.courseReference`, "a C- reference");
  const courseReference = parseCourseReference(reference);
  if (courseReference === null) throw new DecodeError(`${path}.courseReference`, "a C- reference");
  return {
    courseId: decodeUuid(field(record, "courseId", path), `${path}.courseId`),
    courseReference,
    membershipStatus: decodeStringEnum(
      field(record, "membershipStatus", path),
      `${path}.membershipStatus`,
      ["active"],
    ),
  };
}

function decodeCourseInvitationEmailDomain(
  value: unknown,
  path: string,
): CourseInvitationEmailDomain {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["domain", "includeSubdomains"]);
  return {
    domain: decodeNonemptyString(field(record, "domain", path), `${path}.domain`),
    includeSubdomains: decodeBoolean(
      field(record, "includeSubdomains", path),
      `${path}.includeSubdomains`,
    ),
  };
}

function decodeRosterMember(value: unknown, path: string): CourseRosterMember {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "memberId",
    "displayName",
    "rosterEmail",
    "rosterId",
    "role",
    "status",
  ]);
  return {
    memberId: decodeUuid(field(record, "memberId", path), `${path}.memberId`),
    displayName: decodeNonemptyString(field(record, "displayName", path), `${path}.displayName`),
    rosterEmail: decodeNullable(
      field(record, "rosterEmail", path),
      `${path}.rosterEmail`,
      decodeNonemptyString,
    ),
    rosterId: decodeNullable(
      field(record, "rosterId", path),
      `${path}.rosterId`,
      decodeNonemptyString,
    ),
    role: decodeStringEnum(field(record, "role", path), `${path}.role`, ["student"]),
    status: decodeStringEnum(field(record, "status", path), `${path}.status`, [
      "active",
      "revoked",
    ]),
  };
}

function decodeInvitation(value: unknown, path: string): PendingCourseInvitation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["invitationId", "email", "rosterId", "status", "expiresAt"]);
  return {
    invitationId: decodeUuid(field(record, "invitationId", path), `${path}.invitationId`),
    email: decodeNonemptyString(field(record, "email", path), `${path}.email`),
    rosterId: decodeNonemptyString(field(record, "rosterId", path), `${path}.rosterId`),
    status: decodeStringEnum(field(record, "status", path), `${path}.status`, [
      "pending",
      "accepted",
      "revoked",
      "expired",
    ]),
    expiresAt: decodeSafeInteger(field(record, "expiresAt", path), `${path}.expiresAt`),
  };
}

function decodeRosterRevision(value: unknown, path: string): number {
  return positiveRevision(value, path);
}

export function decodeCourseRosterPage(value: unknown, path = "response"): CourseRosterPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "rosterMode",
    "members",
    "nextCursor",
    "rosterRevision",
    "pendingInvitations",
    "allowedEmailDomains",
  ]);
  const rosterMode = decodeStringEnum(field(record, "rosterMode", path), `${path}.rosterMode`, [
    "courseInvitations",
  ]);
  const members = decodeArray(
    field(record, "members", path),
    `${path}.members`,
    decodeRosterMember,
  );
  const nextCursor = decodeNullable(
    field(record, "nextCursor", path),
    `${path}.nextCursor`,
    decodeCursor,
  );
  const rosterRevision = decodeRosterRevision(
    field(record, "rosterRevision", path),
    `${path}.rosterRevision`,
  );
  const pendingInvitations = decodeArray(
    field(record, "pendingInvitations", path),
    `${path}.pendingInvitations`,
    decodeInvitation,
  );
  if (members.length + pendingInvitations.length > 100) {
    throw new DecodeError(path, "a roster page with at most 100 entries");
  }
  return {
    rosterMode,
    members,
    nextCursor,
    rosterRevision,
    pendingInvitations,
    allowedEmailDomains: decodeArray(
      field(record, "allowedEmailDomains", path),
      `${path}.allowedEmailDomains`,
      decodeCourseInvitationEmailDomain,
    ),
  };
}

export function decodeCourseInvitationAccepted(
  value: unknown,
  path = "response",
): CourseInvitationAccepted {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["invitation", "redemptionPath", "emailDelivery"]);
  const redemptionPath = decodeNonemptyString(
    field(record, "redemptionPath", path),
    `${path}.redemptionPath`,
  );
  if (!/^\/course-invitations\/redeem#token=[A-Za-z0-9_-]{43}$/u.test(redemptionPath)) {
    throw new DecodeError(`${path}.redemptionPath`, "a same-origin one-time invitation path");
  }
  return {
    invitation: decodeInvitation(field(record, "invitation", path), `${path}.invitation`),
    redemptionPath,
    emailDelivery: decodeStringEnum(field(record, "emailDelivery", path), `${path}.emailDelivery`, [
      "queued",
      "sentToProvider",
      "needsAttention",
      "cancelled",
    ]),
  };
}

export function decodeCourseInvitationEmailRule(
  value: unknown,
  path = "response",
): CourseInvitationEmailRule {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["allowedEmailDomains", "rosterRevision"]);
  return {
    allowedEmailDomains: decodeArray(
      field(record, "allowedEmailDomains", path),
      `${path}.allowedEmailDomains`,
      decodeCourseInvitationEmailDomain,
    ),
    rosterRevision: decodeRosterRevision(
      field(record, "rosterRevision", path),
      `${path}.rosterRevision`,
    ),
  };
}

export function decodeRosterRevisionResult(
  value: unknown,
  path = "response",
): RosterRevisionResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["rosterRevision"]);
  return {
    rosterRevision: decodeRosterRevision(
      field(record, "rosterRevision", path),
      `${path}.rosterRevision`,
    ),
  };
}

function decodeRosterImportRow(value: unknown, path: string): RosterImportRow {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["rowNumber", "email", "rosterId", "status", "reason"]);
  const decoded: RosterImportRow = {
    rowNumber: decodePositiveInteger(field(record, "rowNumber", path), `${path}.rowNumber`),
    email: decodeNullable(field(record, "email", path), `${path}.email`, decodeNonemptyString),
    rosterId: decodeNullable(
      field(record, "rosterId", path),
      `${path}.rosterId`,
      decodeNonemptyString,
    ),
    status: decodeStringEnum(field(record, "status", path), `${path}.status`, [
      "readyToInvite",
      "alreadyMember",
      "alreadyPending",
      "duplicate",
      "invalid",
    ]),
    reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
      "ready",
      "alreadyOnRoster",
      "invitationPending",
      "duplicateInFile",
      "correctEmailOrRosterId",
    ]),
  };
  const expectedReason: RosterImportRowReason =
    decoded.status === "readyToInvite"
      ? "ready"
      : decoded.status === "alreadyMember"
        ? "alreadyOnRoster"
        : decoded.status === "alreadyPending"
          ? "invitationPending"
          : decoded.status === "duplicate"
            ? "duplicateInFile"
            : "correctEmailOrRosterId";
  if (decoded.reason !== expectedReason) {
    throw new DecodeError(`${path}.reason`, "the safe category for its row status");
  }
  const withholdsInvalid =
    decoded.status === "invalid"
      ? decoded.email === null && decoded.rosterId === null
      : decoded.email !== null && decoded.rosterId !== null;
  if (!withholdsInvalid) {
    throw new DecodeError(path, "a row whose protected cells match its validation status");
  }
  return decoded;
}

export function decodeRosterImportPreview(value: unknown, path = "response"): RosterImportPreview {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "importId",
    "state",
    "expiresAt",
    "rosterRevision",
    "importRevision",
    "rows",
  ]);
  const rows = decodeArray(field(record, "rows", path), `${path}.rows`, decodeRosterImportRow);
  if (rows.length === 0 || rows.length > 500) {
    throw new DecodeError(`${path}.rows`, "1 through 500 roster rows");
  }
  return {
    importId: decodeUuid(field(record, "importId", path), `${path}.importId`),
    state: decodeStringEnum(field(record, "state", path), `${path}.state`, [
      "preview",
      "committed",
    ]),
    expiresAt: decodeSafeInteger(field(record, "expiresAt", path), `${path}.expiresAt`),
    rosterRevision: decodeRosterRevision(
      field(record, "rosterRevision", path),
      `${path}.rosterRevision`,
    ),
    importRevision: positiveRevision(
      field(record, "importRevision", path),
      `${path}.importRevision`,
    ),
    rows,
  };
}

export function decodeRosterImportCommitResult(
  value: unknown,
  path = "response",
): RosterImportCommitResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "importId",
    "importRevision",
    "rosterRevision",
    "invitationsCreated",
    "delivery",
  ]);
  const delivery = decodeArray(
    field(record, "delivery", path),
    `${path}.delivery`,
    decodeRosterImportDelivery,
  );
  return {
    importId: decodeUuid(field(record, "importId", path), `${path}.importId`),
    importRevision: positiveRevision(
      field(record, "importRevision", path),
      `${path}.importRevision`,
    ),
    rosterRevision: decodeRosterRevision(
      field(record, "rosterRevision", path),
      `${path}.rosterRevision`,
    ),
    invitationsCreated: decodeSafeInteger(
      field(record, "invitationsCreated", path),
      `${path}.invitationsCreated`,
    ),
    delivery,
  };
}

function decodeRosterImportDelivery(value: unknown, path: string): RosterImportDelivery {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["rowNumber", "outcome"]);
  return {
    rowNumber: decodePositiveInteger(field(record, "rowNumber", path), `${path}.rowNumber`),
    outcome: decodeStringEnum(field(record, "outcome", path), `${path}.outcome`, [
      "queued",
      "sentToProvider",
      "needsAttention",
      "cancelled",
    ]),
  };
}
