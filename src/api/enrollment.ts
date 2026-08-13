// Passwordless account and course-roster browser contracts.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
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

export type CourseRosterRole = "student";
export type CourseRosterMemberStatus = "active" | "revoked";
export type CourseInvitationStatus = "pending" | "claimed" | "revoked" | "expired";
export type CourseSignupPosture = "invitationOnly" | "permittedDomains";
export type CourseInvitationEmailDelivery = "sent" | "notSent";
export type RosterImportRowStatus =
  "readyToInvite" | "alreadyMember" | "alreadyPending" | "duplicate" | "invalid";

export interface EmailAuthenticationAccepted {
  readonly accepted: true;
}

export interface AccountAuthenticated {
  readonly authenticated: true;
  readonly passkeyEnrollmentSuggested: boolean;
}

export interface AccountEmailChanged {
  readonly changed: true;
}

export interface AccountCourse {
  readonly courseId: CourseId;
  readonly title: string;
  readonly role: "student" | "instructor";
}

export interface AccountCoursePage {
  readonly courses: ReadonlyArray<AccountCourse>;
  readonly nextCursor: string | null;
}

export interface SelectedCourseSession {
  readonly authenticated: true;
  readonly courseId: CourseId;
  readonly role: AccountCourse["role"];
}

export interface ClaimedCourseInvitation {
  readonly courseId: CourseId;
  readonly membershipStatus: "active";
}

export interface PasskeySummary {
  readonly id: string;
  readonly label: string;
  readonly createdAtMillis: number;
  readonly lastUsedAtMillis: number | null;
}

export interface WebauthnStart {
  readonly ceremonyId: string;
  /** webauthn-rs owns this JSON; the browser adapter validates its binary members. */
  readonly options: Readonly<Record<string, unknown>>;
}

export interface PasskeyAuthenticated {
  readonly authenticated: true;
}

export interface AllowedEmailDomain {
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

/** A local-auth-only learner choice. The alias remains an implementation value. */
export interface LocalTeachingLearner {
  readonly alias: string;
  readonly displayName: string;
}

export interface PendingCourseInvitation {
  readonly invitationId: string;
  readonly email: string;
  readonly rosterId: string;
  readonly status: CourseInvitationStatus;
  readonly expiresAt: number;
}

interface CourseRosterPageBase {
  readonly members: ReadonlyArray<CourseRosterMember>;
  readonly nextCursor: string | null;
  readonly rosterRevision: number;
}

/** Email enrollment is available only from a configured email-capable composition. */
export interface EmailEnrollmentRosterPage extends CourseRosterPageBase {
  readonly rosterMode: "emailEnrollment";
  readonly pendingInvitations: ReadonlyArray<PendingCourseInvitation>;
  readonly allowedEmailDomains: ReadonlyArray<AllowedEmailDomain>;
  readonly signupPosture: CourseSignupPosture;
}

/** The no-email teaching composition exposes only configured local learner actions. */
export interface LocalTeachingRosterPage extends CourseRosterPageBase {
  readonly rosterMode: "localTeaching";
  readonly localTeachingLearners: ReadonlyArray<LocalTeachingLearner>;
}

export type CourseRosterPage = EmailEnrollmentRosterPage | LocalTeachingRosterPage;

export interface CourseInvitationAccepted {
  readonly invitation: PendingCourseInvitation;
  readonly redemptionPath: string;
  readonly emailDelivery: CourseInvitationEmailDelivery;
}

export interface LocalTeachingMemberAccepted {
  readonly member: CourseRosterMember;
  readonly rosterRevision: number;
}

export interface CourseEnrollmentPolicyResult {
  readonly allowedEmailDomains: ReadonlyArray<AllowedEmailDomain>;
  readonly signupPosture: CourseSignupPosture;
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
}

export interface ManualGradeExport {
  readonly assignmentId: AssignmentId;
  readonly exportId: string;
  readonly filename: string;
  readonly csv: Blob;
}

export interface CourseRosterClient {
  readonly startEmailAuthentication: (email: string) => Promise<EmailAuthenticationAccepted>;
  readonly completeEmailAuthentication: (
    token: string,
    displayName: string,
  ) => Promise<AccountAuthenticated>;
  readonly startAccountEmailChange: (email: string) => Promise<EmailAuthenticationAccepted>;
  readonly completeAccountEmailChange: (token: string) => Promise<AccountEmailChanged>;
  readonly listAccountCourses: () => Promise<AccountCoursePage>;
  readonly selectAccountCourse: (courseId: CourseId) => Promise<SelectedCourseSession>;
  readonly redeemCourseInvitation: (token: string) => Promise<ClaimedCourseInvitation>;
  readonly startPasskeyRegistration: () => Promise<WebauthnStart>;
  readonly completePasskeyRegistration: (
    ceremonyId: string,
    label: string,
    credential: RegistrationResponseJSON,
  ) => Promise<PasskeySummary>;
  readonly startPasskeyAuthentication: () => Promise<WebauthnStart>;
  readonly completePasskeyAuthentication: (
    ceremonyId: string,
    credential: AuthenticationResponseJSON,
  ) => Promise<PasskeyAuthenticated>;
  readonly listPasskeys: () => Promise<ReadonlyArray<PasskeySummary>>;
  readonly revokePasskey: (passkeyId: string) => Promise<void>;
  readonly listCourseRoster: (courseId: CourseId, cursor?: string) => Promise<CourseRosterPage>;
  readonly addLocalTeachingMember: (
    courseId: CourseId,
    learnerAlias: string,
  ) => Promise<LocalTeachingMemberAccepted>;
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
  readonly replaceCourseEnrollmentPolicy: (
    courseId: CourseId,
    policy: Omit<CourseEnrollmentPolicyResult, "rosterRevision">,
    rosterRevision: number,
  ) => Promise<CourseEnrollmentPolicyResult>;
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
  readonly createManualGradeExport: (
    courseId: CourseId,
    assignmentId: AssignmentId,
  ) => Promise<ManualGradeExport>;
}

function field(record: Record<string, unknown>, key: string, path: string): unknown {
  return decodeField(record, key, path);
}

function positiveRevision(value: unknown, path: string): number {
  const revision = decodePositiveInteger(value, path);
  if (revision > Number.MAX_SAFE_INTEGER) throw new DecodeError(path, "a safe positive revision");
  return revision;
}

function decodeTrueField(record: Record<string, unknown>, key: string, path: string): true {
  const value = decodeBoolean(field(record, key, path), `${path}.${key}`);
  if (!value) throw new DecodeError(`${path}.${key}`, "true");
  return true;
}

export function decodeEmailAuthenticationAccepted(
  value: unknown,
  path = "response",
): EmailAuthenticationAccepted {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["accepted"]);
  return { accepted: decodeTrueField(record, "accepted", path) };
}

export function decodeAccountAuthenticated(
  value: unknown,
  path = "response",
): AccountAuthenticated {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["authenticated", "passkeyEnrollmentSuggested"]);
  return {
    authenticated: decodeTrueField(record, "authenticated", path),
    passkeyEnrollmentSuggested: decodeBoolean(
      field(record, "passkeyEnrollmentSuggested", path),
      `${path}.passkeyEnrollmentSuggested`,
    ),
  };
}

export function decodeAccountEmailChanged(value: unknown, path = "response"): AccountEmailChanged {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["changed"]);
  if (!decodeBoolean(field(record, "changed", path), `${path}.changed`)) {
    throw new DecodeError(`${path}.changed`, "true");
  }
  return { changed: true };
}

function decodeAccountRole(value: unknown, path: string): AccountCourse["role"] {
  return decodeStringEnum(value, path, ["student", "instructor"]);
}

function decodeAccountCourse(value: unknown, path: string): AccountCourse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["courseId", "title", "role"]);
  return {
    courseId: decodeUuid(field(record, "courseId", path), `${path}.courseId`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    role: decodeAccountRole(field(record, "role", path), `${path}.role`),
  };
}

export function decodeAccountCoursePage(value: unknown, path = "response"): AccountCoursePage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["courses", "nextCursor"]);
  const courses = decodeArray(
    field(record, "courses", path),
    `${path}.courses`,
    decodeAccountCourse,
  );
  if (courses.length > 100) throw new DecodeError(`${path}.courses`, "at most 100 courses");
  return {
    courses,
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeCursor,
    ),
  };
}

export function decodeSelectedCourseSession(
  value: unknown,
  path = "response",
): SelectedCourseSession {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["authenticated", "courseId", "role"]);
  return {
    authenticated: decodeTrueField(record, "authenticated", path),
    courseId: decodeUuid(field(record, "courseId", path), `${path}.courseId`),
    role: decodeAccountRole(field(record, "role", path), `${path}.role`),
  };
}

export function decodeClaimedCourseInvitation(
  value: unknown,
  path = "response",
): ClaimedCourseInvitation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["courseId", "membershipStatus"]);
  return {
    courseId: decodeUuid(field(record, "courseId", path), `${path}.courseId`),
    membershipStatus: decodeStringEnum(
      field(record, "membershipStatus", path),
      `${path}.membershipStatus`,
      ["active"],
    ),
  };
}

export function decodeWebauthnStart(value: unknown, path = "response"): WebauthnStart {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["ceremonyId", "options"]);
  return {
    ceremonyId: decodeUuid(field(record, "ceremonyId", path), `${path}.ceremonyId`),
    options: decodeRecord(field(record, "options", path), `${path}.options`),
  };
}

export function decodePasskeySummary(value: unknown, path = "response"): PasskeySummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "label", "createdAtMillis", "lastUsedAtMillis"]);
  return {
    id: decodeUuid(field(record, "id", path), `${path}.id`),
    label: decodeNonemptyString(field(record, "label", path), `${path}.label`),
    createdAtMillis: decodeSafeInteger(
      field(record, "createdAtMillis", path),
      `${path}.createdAtMillis`,
    ),
    lastUsedAtMillis: decodeNullable(
      field(record, "lastUsedAtMillis", path),
      `${path}.lastUsedAtMillis`,
      decodeSafeInteger,
    ),
  };
}

export function decodePasskeyList(
  value: unknown,
  path = "response",
): ReadonlyArray<PasskeySummary> {
  const list = decodeArray(value, path, decodePasskeySummary);
  if (list.length > 100) throw new DecodeError(path, "at most 100 passkeys");
  return list;
}

export function decodePasskeyAuthenticated(
  value: unknown,
  path = "response",
): PasskeyAuthenticated {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["authenticated"]);
  return { authenticated: decodeTrueField(record, "authenticated", path) };
}

function decodeAllowedDomain(value: unknown, path: string): AllowedEmailDomain {
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

function decodeLocalTeachingLearner(value: unknown, path: string): LocalTeachingLearner {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["alias", "displayName"]);
  return {
    alias: decodeNonemptyString(field(record, "alias", path), `${path}.alias`),
    displayName: decodeNonemptyString(field(record, "displayName", path), `${path}.displayName`),
  };
}

export function decodeLocalTeachingMemberAccepted(
  value: unknown,
  path = "response",
): LocalTeachingMemberAccepted {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["member", "rosterRevision"]);
  return {
    member: decodeRosterMember(field(record, "member", path), `${path}.member`),
    rosterRevision: decodeRosterRevision(
      field(record, "rosterRevision", path),
      `${path}.rosterRevision`,
    ),
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
      "claimed",
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
  const baseFields = ["rosterMode", "members", "nextCursor", "rosterRevision"];
  const rosterMode = decodeStringEnum(field(record, "rosterMode", path), `${path}.rosterMode`, [
    "emailEnrollment",
    "localTeaching",
  ]);
  const allowedFields =
    rosterMode === "localTeaching"
      ? [...baseFields, "localTeachingLearners"]
      : [...baseFields, "pendingInvitations", "allowedEmailDomains", "signupPosture"];
  requireOnlyFields(record, path, allowedFields);
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
  if (rosterMode === "localTeaching") {
    return {
      rosterMode: "localTeaching",
      members,
      nextCursor,
      rosterRevision,
      localTeachingLearners: decodeArray(
        field(record, "localTeachingLearners", path),
        `${path}.localTeachingLearners`,
        decodeLocalTeachingLearner,
      ),
    };
  }
  const pendingInvitations = decodeArray(
    field(record, "pendingInvitations", path),
    `${path}.pendingInvitations`,
    decodeInvitation,
  );
  if (members.length + pendingInvitations.length > 100) {
    throw new DecodeError(path, "a roster page with at most 100 entries");
  }
  return {
    rosterMode: "emailEnrollment",
    members,
    nextCursor,
    rosterRevision,
    pendingInvitations,
    allowedEmailDomains: decodeArray(
      field(record, "allowedEmailDomains", path),
      `${path}.allowedEmailDomains`,
      decodeAllowedDomain,
    ),
    signupPosture: decodeStringEnum(field(record, "signupPosture", path), `${path}.signupPosture`, [
      "invitationOnly",
      "permittedDomains",
    ]),
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
      "sent",
      "notSent",
    ]),
  };
}

export function decodeCourseEnrollmentPolicyResult(
  value: unknown,
  path = "response",
): CourseEnrollmentPolicyResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["allowedEmailDomains", "signupPosture", "rosterRevision"]);
  return {
    allowedEmailDomains: decodeArray(
      field(record, "allowedEmailDomains", path),
      `${path}.allowedEmailDomains`,
      decodeAllowedDomain,
    ),
    signupPosture: decodeStringEnum(field(record, "signupPosture", path), `${path}.signupPosture`, [
      "invitationOnly",
      "permittedDomains",
    ]),
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
  requireOnlyFields(record, path, ["rowNumber", "email", "rosterId", "status"]);
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
  };
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
  ]);
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
  };
}
