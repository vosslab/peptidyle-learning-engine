import type {
  InstructorProfile,
  InstructorProfileThumbnail,
  UpdateInstructorProfileInput,
} from "../instructor_profile";
import { DecodeError, decodeRecord, decodeString } from "../decoder";
import { field, requireOnlyFields } from "./shared";

function decodeTimeZone(value: unknown, path: string): string {
  const result = decodeString(value, path);
  if (result.length === 0 || result.length > 255 || result.trim() !== result)
    throw new DecodeError(path, "a bounded exact IANA time-zone name");
  return result;
}

export function decodeInstructorProfile(value: unknown, path = "response"): InstructorProfile {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["timeZone"]);
  return { timeZone: decodeTimeZone(field(record, "timeZone", path), `${path}.timeZone`) };
}

export function decodeUpdateInstructorProfileInput(
  value: unknown,
  path = "request",
): UpdateInstructorProfileInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["timeZone"]);
  return { timeZone: decodeTimeZone(field(record, "timeZone", path), `${path}.timeZone`) };
}

export function decodeInstructorProfileThumbnail(
  value: unknown,
  path = "response",
): InstructorProfileThumbnail {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference"]);
  const reference = field(record, "reference", path);
  if (reference === null) return { reference: null };
  const valueReference = decodeString(reference, `${path}.reference`);
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u.test(valueReference))
    throw new DecodeError(`${path}.reference`, "an opaque thumbnail reference");
  return { reference: valueReference };
}
