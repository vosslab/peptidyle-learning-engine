import type { ProfileSettings, UpdateAccountSettingsInput } from "../profile_settings";
import { DecodeError, decodeRecord, decodeString } from "../decoder";
import { field, requireOnlyFields } from "./shared";

function decodeTimeZone(value: unknown, path: string): string {
  const timeZone = decodeString(value, path);
  if (timeZone.length === 0 || timeZone.length > 255 || timeZone.trim() !== timeZone) {
    throw new DecodeError(path, "a bounded exact IANA time-zone name");
  }
  try {
    new Intl.DateTimeFormat(undefined, { timeZone });
  } catch {
    throw new DecodeError(path, "a valid IANA time-zone name");
  }
  return timeZone;
}

/** Rejects any future profile fields until their product behavior is designed. */
export function decodeProfileSettings(value: unknown, path = "response"): ProfileSettings {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["timeZone"]);
  return { timeZone: decodeTimeZone(field(record, "timeZone", path), `${path}.timeZone`) };
}

/** Refuses Account or role fields before a settings write reaches the server. */
export function decodeUpdateAccountSettingsInput(
  value: unknown,
  path = "request",
): UpdateAccountSettingsInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["timeZone"]);
  return { timeZone: decodeTimeZone(field(record, "timeZone", path), `${path}.timeZone`) };
}
