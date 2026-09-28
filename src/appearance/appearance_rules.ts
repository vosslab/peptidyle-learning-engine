// appearance_rules.ts - pure Theme ownership and display-mode resolution.

import type { DisplayMode } from "../../generated/api/DisplayMode";
import type { Theme } from "../../generated/api/Theme";

export interface ThemeResolutionInput {
  readonly courseTheme?: Theme;
  readonly instructorPersonalTheme?: Theme;
  readonly signedInInstructor: boolean;
}

/** Resolves the one Theme for a page without inheritance or fallback chains. */
export function resolveTheme(input: ThemeResolutionInput): Theme {
  if (input.courseTheme !== undefined) return input.courseTheme;
  if (input.signedInInstructor && input.instructorPersonalTheme !== undefined)
    return input.instructorPersonalTheme;
  return "grass";
}

/** Uses the viewer preference when set and the live browser value when unset. */
export function resolveDisplayMode(
  preference: DisplayMode | null | undefined,
  browserMode: DisplayMode,
): DisplayMode {
  return preference ?? browserMode;
}
