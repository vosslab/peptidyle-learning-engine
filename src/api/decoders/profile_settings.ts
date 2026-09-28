import type {
  ProfileSettings,
  UpdateAccountSettingsInput,
  UpdateDisplayModePreferenceInput,
  UpdateInstructorPersonalThemeInput,
} from "../profile_settings";
import { DISPLAY_MODE_VALUES } from "../../../generated/api/DisplayMode";
import { THEME_VALUES } from "../../../generated/api/Theme";
import {
  DecodeError,
  decodeNullable,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
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

/** Rejects response fields outside the current Account Settings contract. */
export function decodeProfileSettings(value: unknown, path = "response"): ProfileSettings {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["timeZone", "displayModePreference", "personalTheme"]);
  return {
    timeZone: decodeTimeZone(field(record, "timeZone", path), `${path}.timeZone`),
    displayModePreference: decodeNullable(
      field(record, "displayModePreference", path),
      `${path}.displayModePreference`,
      (candidate, candidatePath) => decodeStringEnum(candidate, candidatePath, DISPLAY_MODE_VALUES),
    ),
    personalTheme: decodeNullable(
      field(record, "personalTheme", path),
      `${path}.personalTheme`,
      (candidate, candidatePath) => decodeStringEnum(candidate, candidatePath, THEME_VALUES),
    ),
  };
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

/** Refuses identity, role, and unrelated preferences before the mode write reaches the server. */
export function decodeUpdateDisplayModePreferenceInput(
  value: unknown,
  path = "request",
): UpdateDisplayModePreferenceInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["displayModePreference"]);
  return {
    displayModePreference: decodeNullable(
      field(record, "displayModePreference", path),
      `${path}.displayModePreference`,
      (candidate, candidatePath) => decodeStringEnum(candidate, candidatePath, DISPLAY_MODE_VALUES),
    ),
  };
}

/** Refuses account, role, and arbitrary color input before the Theme write reaches the server. */
export function decodeUpdateInstructorPersonalThemeInput(
  value: unknown,
  path = "request",
): UpdateInstructorPersonalThemeInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["theme"]);
  return {
    theme: decodeStringEnum(field(record, "theme", path), `${path}.theme`, THEME_VALUES),
  };
}
