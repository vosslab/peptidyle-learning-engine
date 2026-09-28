// appearance_context.ts - document-wide resolved appearance presentation.

import { createContext, useContext, type Accessor } from "solid-js";

import type { DisplayMode } from "../../generated/api/DisplayMode";
import type { Theme } from "../../generated/api/Theme";
import type { ProfileSettings } from "../api/profile_settings";
import type { ThemeTokens } from "./theme_registry";

export interface ResolvedAppearance {
  readonly theme: Theme;
  readonly mode: DisplayMode;
  readonly tokens: ThemeTokens;
}

export type AppearanceSettingsState = "idle" | "loading" | "ready" | "error";

/** Document-owned appearance state and the two self-service preference writes. */
export interface AppearanceContextValue {
  readonly appearance: Accessor<ResolvedAppearance>;
  /** True once an authenticated current Course and Account preference can no longer change this paint. */
  readonly initialAppearanceReady: Accessor<boolean>;
  readonly settings: Accessor<ProfileSettings | undefined>;
  readonly settingsState: Accessor<AppearanceSettingsState>;
  readonly busy: Accessor<boolean>;
  readonly error: Accessor<string | undefined>;
  readonly updateTimeZone: (timeZone: string) => Promise<void>;
  readonly updateDisplayModePreference: (preference: DisplayMode | null) => Promise<void>;
  readonly updatePersonalTheme: (theme: Theme) => Promise<void>;
  /** A Course Appearance editor holds this temporary selection until it saves or leaves. */
  readonly presentCourseTheme: (theme: Theme | undefined) => void;
}

export const AppearanceContext = createContext<AppearanceContextValue>();

/** Reads the one document appearance owner. */
export function useAppearance(): AppearanceContextValue {
  const value = useContext(AppearanceContext);
  if (value === undefined) throw new Error("AppearanceOwner is missing from the application shell");
  return value;
}
