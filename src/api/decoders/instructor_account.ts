// Strict browser boundary for the deliberately small Instructor Accounts DTO.
import type { AccountId } from "../../../generated/api/AccountId";
import type {
  CreatedInstructorAccount,
  CreateInstructorAccountInput,
  DeactivateInstructorAccountInput,
  InstructorAccountList,
  InstructorAccountState,
  InstructorAccountSummary,
} from "../instructor_account";
import { PROVIDED_AVATAR_CATALOG } from "../../features/profile_avatar/avatar_catalog_generated.ts";
import {
  DecodeError,
  decodeArray,
  decodeField,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
} from "../decoder.ts";
import { validateCanonicalPublicId } from "../../question_id.ts";
const MAX_REASON_LENGTH = 1_000;
function field(record: Record<string, unknown>, key: string, path: string): unknown {
  return decodeField(record, key, path);
}
function only(record: Record<string, unknown>, path: string, allowed: ReadonlyArray<string>): void {
  for (const key of Object.keys(record))
    if (!allowed.includes(key))
      throw new DecodeError(`${path}.${key}`, "a field allowed by this response contract");
}
export function isCanonicalAccountId(value: string): value is AccountId {
  return validateCanonicalPublicId("account", value) !== null;
}
function accountId(value: unknown, path: string): AccountId {
  const decoded = decodeString(value, path);
  if (!isCanonicalAccountId(decoded))
    throw new DecodeError(path, "a canonical Instructor Account ID");
  return decoded;
}
function state(value: unknown, path: string): InstructorAccountState {
  const decoded = decodeString(value, path);
  if (decoded !== "active" && decoded !== "deactivated" && decoded !== "closed")
    throw new DecodeError(path, "a current Instructor Account State");
  return decoded;
}
function avatar(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!PROVIDED_AVATAR_CATALOG.some((entry) => entry.id === decoded))
    throw new DecodeError(path, "a generated provided avatar id");
  return decoded;
}
function summary(value: unknown, path: string): InstructorAccountSummary {
  const record = decodeRecord(value, path);
  only(record, path, ["id", "state", "lastSuccessfulSignIn", "providedAvatarId"]);
  return {
    id: accountId(field(record, "id", path), `${path}.id`),
    state: state(field(record, "state", path), `${path}.state`),
    lastSuccessfulSignIn: decodeNullable(
      field(record, "lastSuccessfulSignIn", path),
      `${path}.lastSuccessfulSignIn`,
      decodeSafeInteger,
    ),
    providedAvatarId: decodeNullable(
      field(record, "providedAvatarId", path),
      `${path}.providedAvatarId`,
      avatar,
    ),
  };
}
function zone(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!decoded.length || decoded.length > 100)
    throw new DecodeError(path, "a bounded IANA display time zone");
  try {
    new Intl.DateTimeFormat("en-US", { timeZone: decoded });
  } catch {
    throw new DecodeError(path, "an exact IANA display time zone");
  }
  return decoded;
}
export function decodeInstructorAccountList(
  value: unknown,
  path = "response",
): InstructorAccountList {
  const record = decodeRecord(value, path);
  only(record, path, ["accounts", "displayTimeZone", "nextCursor"]);
  return {
    accounts: decodeArray(field(record, "accounts", path), `${path}.accounts`, summary),
    displayTimeZone: zone(field(record, "displayTimeZone", path), `${path}.displayTimeZone`),
    nextCursor: decodeNullable(field(record, "nextCursor", path), `${path}.nextCursor`, accountId),
  };
}
export function decodeInstructorAccount(
  value: unknown,
  path = "response",
): InstructorAccountSummary {
  return summary(value, path);
}
function text(
  record: Record<string, unknown>,
  path: string,
  key: "firstName" | "lastName" | "affiliation",
  maximum: number,
): string {
  const value = decodeString(field(record, key, path), `${path}.${key}`);
  if (
    !value.length ||
    value !== value.trim() ||
    [...value].length > maximum ||
    /[\p{Cc}]/u.test(value)
  )
    throw new DecodeError(
      `${path}.${key}`,
      "a trimmed, control-free Instructor setup field within its bound",
    );
  return value;
}
export function decodeCreateInstructorAccountInput(
  value: unknown,
  path = "request",
): CreateInstructorAccountInput {
  const record = decodeRecord(value, path);
  only(record, path, ["normalizedEmail", "firstName", "lastName", "affiliation"]);
  const normalizedEmail = decodeString(
    field(record, "normalizedEmail", path),
    `${path}.normalizedEmail`,
  );
  if (
    normalizedEmail.length < 3 ||
    normalizedEmail.length > 320 ||
    normalizedEmail !== normalizedEmail.trim().toLowerCase()
  )
    throw new DecodeError(
      `${path}.normalizedEmail`,
      "a trimmed lowercase normalized email within its bound",
    );
  return {
    normalizedEmail,
    firstName: text(record, path, "firstName", 100),
    lastName: text(record, path, "lastName", 100),
    affiliation: text(record, path, "affiliation", 300),
  };
}
export function decodeCreatedInstructorAccount(
  value: unknown,
  path = "response",
): CreatedInstructorAccount {
  const record = decodeRecord(value, path);
  only(record, path, ["account", "setupEmailSent"]);
  const setupEmailSent = field(record, "setupEmailSent", path);
  if (typeof setupEmailSent !== "boolean")
    throw new DecodeError(`${path}.setupEmailSent`, "a setup-email delivery result");
  return { account: summary(field(record, "account", path), `${path}.account`), setupEmailSent };
}
export function decodeInstructorSetupEmailResponse(value: unknown, path = "response"): boolean {
  const record = decodeRecord(value, path);
  only(record, path, ["setupEmailSent"]);
  const sent = field(record, "setupEmailSent", path);
  if (typeof sent !== "boolean")
    throw new DecodeError(`${path}.setupEmailSent`, "a setup-email delivery result");
  return sent;
}
export function decodeDeactivateInstructorAccountInput(
  value: unknown,
  path = "request",
): DeactivateInstructorAccountInput {
  const record = decodeRecord(value, path);
  only(record, path, ["reason"]);
  const reason = decodeString(field(record, "reason", path), `${path}.reason`);
  if (reason.length === 0 || reason.length > MAX_REASON_LENGTH || reason !== reason.trim())
    throw new DecodeError(`${path}.reason`, "a trimmed deactivation reason within its bound");
  return { reason };
}
