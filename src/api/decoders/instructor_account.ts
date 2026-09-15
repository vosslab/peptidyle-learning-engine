// Strict browser boundary for the deliberately small Instructor Accounts DTO.

import type { AccountReference } from "../../../generated/api/AccountReference";
import type {
  CompleteInstructorIdentityVettingInput,
  CreateInstructorAccountInput,
  DeactivateInstructorAccountInput,
  InstructorAccountList,
  InstructorAccountState,
  InstructorAccountSummary,
  InstructorIdentityVettingReceipt,
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

const MAX_REASON_LENGTH = 1_000;
const ACCOUNT_REFERENCE_PATTERN = /^U[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$/u;
const VETTING_REFERENCE_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

function field(record: Record<string, unknown>, key: string, path: string): unknown {
  return decodeField(record, key, path);
}

function requireOnlyFields(
  record: Record<string, unknown>,
  path: string,
  allowed: ReadonlyArray<string>,
): void {
  for (const key of Object.keys(record)) {
    if (!allowed.includes(key)) {
      throw new DecodeError(`${path}.${key}`, "a field allowed by this response contract");
    }
  }
}

/** The Sysadmin-only Account reference is opaque, not a public route identity. */
export function isCanonicalAccountReference(value: string): value is AccountReference {
  return ACCOUNT_REFERENCE_PATTERN.test(value);
}

function accountReference(value: unknown, path: string): AccountReference {
  const decoded = decodeString(value, path);
  if (!isCanonicalAccountReference(decoded)) {
    throw new DecodeError(path, "a canonical Instructor Account public reference");
  }
  return decoded;
}

function accountState(value: unknown, path: string): InstructorAccountState {
  const decoded = decodeString(value, path);
  if (decoded !== "active" && decoded !== "deactivated" && decoded !== "closed") {
    throw new DecodeError(path, "a current Instructor Account State");
  }
  return decoded;
}

function providedAvatarId(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!PROVIDED_AVATAR_CATALOG.some((entry) => entry.id === decoded)) {
    throw new DecodeError(path, "a generated provided avatar id");
  }
  return decoded;
}

function summary(value: unknown, path: string): InstructorAccountSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "state",
    "lastSuccessfulSignIn",
    "providedAvatarId",
  ]);
  return {
    reference: accountReference(field(record, "reference", path), `${path}.reference`),
    state: accountState(field(record, "state", path), `${path}.state`),
    lastSuccessfulSignIn: decodeNullable(
      field(record, "lastSuccessfulSignIn", path),
      `${path}.lastSuccessfulSignIn`,
      decodeSafeInteger,
    ),
    providedAvatarId: decodeNullable(
      field(record, "providedAvatarId", path),
      `${path}.providedAvatarId`,
      providedAvatarId,
    ),
  };
}

function displayTimeZone(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (decoded.length === 0 || decoded.length > 100) {
    throw new DecodeError(path, "a bounded IANA display time zone");
  }
  try {
    new Intl.DateTimeFormat("en-US", { timeZone: decoded });
  } catch {
    throw new DecodeError(path, "an exact IANA display time zone");
  }
  return decoded;
}

/** Rejects extra fields, including accidental Authentication Email disclosure. */
export function decodeInstructorAccountList(
  value: unknown,
  path = "response",
): InstructorAccountList {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["accounts", "displayTimeZone"]);
  return {
    accounts: decodeArray(field(record, "accounts", path), `${path}.accounts`, summary),
    displayTimeZone: displayTimeZone(
      field(record, "displayTimeZone", path),
      `${path}.displayTimeZone`,
    ),
  };
}

export function decodeInstructorAccount(
  value: unknown,
  path = "response",
): InstructorAccountSummary {
  return summary(value, path);
}

/** Receipts remain opaque so the browser cannot use the audit ID as identity. */
export function decodeInstructorIdentityVettingReceipt(
  value: unknown,
  path = "response",
): InstructorIdentityVettingReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["vettingDecisionReference"]);
  const vettingDecisionReference = decodeString(
    field(record, "vettingDecisionReference", path),
    `${path}.vettingDecisionReference`,
  );
  if (!VETTING_REFERENCE_PATTERN.test(vettingDecisionReference)) {
    throw new DecodeError(`${path}.vettingDecisionReference`, "an opaque vetting decision receipt");
  }
  return { vettingDecisionReference };
}

export function decodeCompleteInstructorIdentityVettingInput(
  value: unknown,
  path = "request",
): CompleteInstructorIdentityVettingInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["normalizedEmail", "verifiedInstructorDisplayName"]);
  const normalizedEmail = decodeString(
    field(record, "normalizedEmail", path),
    `${path}.normalizedEmail`,
  );
  const verifiedInstructorDisplayName = decodeString(
    field(record, "verifiedInstructorDisplayName", path),
    `${path}.verifiedInstructorDisplayName`,
  );
  if (
    normalizedEmail.length < 3 ||
    normalizedEmail.length > 320 ||
    normalizedEmail !== normalizedEmail.trim().toLowerCase()
  ) {
    throw new DecodeError(
      `${path}.normalizedEmail`,
      "a trimmed lowercase normalized email within its bound",
    );
  }
  if (
    verifiedInstructorDisplayName !== verifiedInstructorDisplayName.trim() ||
    ![...verifiedInstructorDisplayName].length ||
    [...verifiedInstructorDisplayName].length > 200 ||
    /[\p{Cc}]/u.test(verifiedInstructorDisplayName)
  ) {
    throw new DecodeError(
      `${path}.verifiedInstructorDisplayName`,
      "a trimmed, control-free verified Instructor display name within its bound",
    );
  }
  return { normalizedEmail, verifiedInstructorDisplayName };
}

/** Normalization is intentional; the server remains the final email-policy authority. */
export function decodeCreateInstructorAccountInput(
  value: unknown,
  path = "request",
): CreateInstructorAccountInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["normalizedEmail", "vettingDecisionReference"]);
  const normalizedEmail = decodeString(
    field(record, "normalizedEmail", path),
    `${path}.normalizedEmail`,
  );
  if (
    normalizedEmail.length < 3 ||
    normalizedEmail.length > 320 ||
    normalizedEmail !== normalizedEmail.trim().toLowerCase()
  ) {
    throw new DecodeError(
      `${path}.normalizedEmail`,
      "a trimmed lowercase normalized email within its bound",
    );
  }
  const vettingDecisionReference = decodeString(
    field(record, "vettingDecisionReference", path),
    `${path}.vettingDecisionReference`,
  );
  if (!VETTING_REFERENCE_PATTERN.test(vettingDecisionReference)) {
    throw new DecodeError(`${path}.vettingDecisionReference`, "an opaque vetting decision receipt");
  }
  return { normalizedEmail, vettingDecisionReference };
}

export function decodeDeactivateInstructorAccountInput(
  value: unknown,
  path = "request",
): DeactivateInstructorAccountInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reason"]);
  const reason = decodeString(field(record, "reason", path), `${path}.reason`);
  if (reason.length === 0 || reason.length > MAX_REASON_LENGTH || reason !== reason.trim()) {
    throw new DecodeError(`${path}.reason`, "a trimmed deactivation reason within its bound");
  }
  return { reason };
}
