// Strict browser decoding for the audited Sysadmin Student-data projection.

import type { AccountId } from "../../../generated/api/AccountId";
import type { SysadminStudentDataAccess } from "../sysadmin_student_access";
import {
  DecodeError,
  decodeNonnegativeInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";
import { isCanonicalAccountId } from "./instructor_account";
import { decodeCourseInstanceId, field, requireOnlyFields } from "./shared";

function accountId(value: unknown, path: string): AccountId {
  const decoded = decodeString(value, path);
  if (!isCanonicalAccountId(decoded)) {
    throw new DecodeError(path, "a canonical Student Account ID");
  }
  return decoded;
}

function rosterId(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^[A-Za-z0-9._-]{1,64}$/u.test(decoded)) {
    throw new DecodeError(path, "a course-scoped roster identifier");
  }
  return decoded;
}

function rosterName(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (
    decoded === "" ||
    decoded !== decoded.trim() ||
    Array.from(decoded).length > 200 ||
    /[\uD800-\uDFFF]/u.test(decoded) ||
    /\p{Cc}/u.test(decoded)
  ) {
    throw new DecodeError(path, "a trimmed, nonempty Course roster name of at most 200 characters");
  }
  return decoded;
}

/** Decodes the exact roster record and same-transaction audit receipt returned by the server. */
export function decodeSysadminStudentDataAccess(
  value: unknown,
  path = "response",
): SysadminStudentDataAccess {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "courseInstanceId",
    "studentAccountId",
    "rosterId",
    "rosterName",
    "state",
    "audit",
  ]);
  const audit = decodeRecord(field(record, "audit", path), `${path}.audit`);
  requireOnlyFields(audit, `${path}.audit`, ["eventId", "occurredAt"]);
  const occurredAt = decodeNonnegativeInteger(
    field(audit, "occurredAt", `${path}.audit`),
    `${path}.audit.occurredAt`,
  );
  if (occurredAt > 8_640_000_000_000_000) {
    throw new DecodeError(
      `${path}.audit.occurredAt`,
      "a JavaScript Date timestamp in milliseconds",
    );
  }
  return {
    courseInstanceId: decodeCourseInstanceId(
      field(record, "courseInstanceId", path),
      `${path}.courseInstanceId`,
    ),
    studentAccountId: accountId(
      field(record, "studentAccountId", path),
      `${path}.studentAccountId`,
    ),
    rosterId: rosterId(field(record, "rosterId", path), `${path}.rosterId`),
    rosterName: rosterName(field(record, "rosterName", path), `${path}.rosterName`),
    state: decodeStringEnum(field(record, "state", path), `${path}.state`, [
      "invitationPending",
      "activeStudent",
      "removed",
    ] as const),
    audit: {
      eventId: decodeUuid(field(audit, "eventId", `${path}.audit`), `${path}.audit.eventId`),
      occurredAt,
    },
  };
}
