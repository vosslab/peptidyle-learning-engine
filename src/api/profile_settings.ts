import type { DisplayMode } from "../../generated/api/DisplayMode";
import type { Theme } from "../../generated/api/Theme";

/** Browser-safe, role-neutral settings returned for the authenticated Account. */
export interface ProfileSettings {
  readonly timeZone: string;
  /** Null follows the browser's current Light or Dark preference. */
  readonly displayModePreference: DisplayMode | null;
  /** Present only for an authenticated Instructor Account. */
  readonly personalTheme: Theme | null;
}

/** Closed browser input for the authenticated Account's sole settings preference. */
export interface UpdateAccountSettingsInput {
  readonly timeZone: string;
}

/** Closed Account-owned request for an explicit display form or browser-following clear. */
export interface UpdateDisplayModePreferenceInput {
  readonly displayModePreference: DisplayMode | null;
}

/** Closed Instructor-owned request for the global-page Theme. */
export interface UpdateInstructorPersonalThemeInput {
  readonly theme: Theme;
}
