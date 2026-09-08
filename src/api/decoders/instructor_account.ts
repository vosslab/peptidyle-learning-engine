// Strict browser boundary for the deliberately small Instructor Accounts DTO.

import type { AccountReference } from "../../../generated/api/AccountReference";
import type {
  CreateInstructorAccountInput,
  DeactivateInstructorAccountInput,
  InstructorAccountState,
  InstructorAccountSummary,
} from "../instructor_account";
import {
  DecodeError,
  decodeArray,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
} from "../decoder";
import { field, requireOnlyFields } from "./shared";

const MAX_ACCOUNT_REFERENCE = 2_147_483_647;
const MAX_REASON_LENGTH = 1_000;

function accountReference(value: unknown, path: string): AccountReference {
  const decoded = decodeString(value, path);
  if (!/^U-[1-9][0-9]{0,9}$/u.test(decoded) || Number(decoded.slice(2)) > MAX_ACCOUNT_REFERENCE) {
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

function summary(value: unknown, path: string): InstructorAccountSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "state", "lastSuccessfulSignIn"]);
  return {
    reference: accountReference(field(record, "reference", path), `${path}.reference`),
    state: accountState(field(record, "state", path), `${path}.state`),
    lastSuccessfulSignIn: decodeNullable(
      field(record, "lastSuccessfulSignIn", path),
      `${path}.lastSuccessfulSignIn`,
      decodeSafeInteger,
    ),
  };
}

/** Rejects extra fields, including accidental Authentication Email disclosure. */
export function decodeInstructorAccounts(
  value: unknown,
  path = "response",
): ReadonlyArray<InstructorAccountSummary> {
  return decodeArray(value, path, summary);
}

export function decodeInstructorAccount(
  value: unknown,
  path = "response",
): InstructorAccountSummary {
  return summary(value, path);
}

/** Normalization is intentional; the server remains the final email-policy authority. */
export function decodeCreateInstructorAccountInput(
  value: unknown,
  path = "request",
): CreateInstructorAccountInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["normalizedEmail"]);
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
  return { normalizedEmail };
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
