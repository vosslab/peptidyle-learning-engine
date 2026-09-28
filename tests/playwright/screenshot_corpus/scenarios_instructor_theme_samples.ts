// scenarios_instructor_theme_samples.ts - final persisted Course Theme comparisons.

import type { DisplayMode } from "../../../generated/api/DisplayMode";
import { THEME_VALUES, type Theme } from "../../../generated/api/Theme";

import type { ScenarioDefinition } from "./scenario_types";
import type { ScenarioRuntime } from "./runtime";
import {
  openSeededCourseAppearance,
  openSeededCourseWorkspace,
  persistCourseAppearance,
} from "./theme_capture_workflow";
import { scrollTop } from "./visible_workflows";

const DISPLAY_MODES = ["light", "dark"] as const satisfies ReadonlyArray<DisplayMode>;

function checkpoint(theme: Theme, mode: DisplayMode): string {
  return `theme_sample_${theme.replace(/-/gu, "_")}_${mode}`;
}

async function captureThemeSample(
  runtime: ScenarioRuntime,
  theme: Theme,
  mode: DisplayMode,
  expectedCourseInstanceId: string | undefined,
): Promise<string> {
  const session = await runtime.open(checkpoint(theme, mode));
  try {
    const courseInstanceId = await openSeededCourseAppearance(session.page);
    if (expectedCourseInstanceId !== undefined && courseInstanceId !== expectedCourseInstanceId) {
      throw new Error(
        `Course Theme captures changed Course record ID from ${expectedCourseInstanceId} to ${courseInstanceId}.`,
      );
    }
    const appearance = { theme, mode } as const;
    await persistCourseAppearance(session.page, courseInstanceId, appearance);
    await openSeededCourseWorkspace(session.page, appearance);
    await scrollTop(session.page);
    await runtime.captureCheckpoint(session, checkpoint(theme, mode));
    return courseInstanceId;
  } finally {
    await runtime.close(session);
  }
}

export const INSTRUCTOR_THEME_SAMPLE_SCENARIO: ScenarioDefinition = {
  id: "instructor_theme_samples",
  role: "instructor",
  captures: THEME_VALUES.flatMap((theme) =>
    DISPLAY_MODES.map((mode) => ({
      checkpoint: checkpoint(theme, mode),
      filenameStem: `theme_sample-${theme}-${mode}`,
      area: "courses",
      workflow: "Course Theme comparison",
      state: `theme sample ${theme} ${mode}`,
      viewport: "laptop" as const,
      privacyProfile: "instructor_answer_free" as const,
      displayMode: mode,
      expectedTheme: theme,
      caption: `Instructor Course workspace rendered with the ${theme} Theme in ${mode} mode`,
    })),
  ),
  viewportCoverage: {
    laptop: { status: "captured" },
    tablet: {
      status: "covered_by",
      target: checkpoint("forest", "light"),
      reason: "Course Theme comparisons use the canonical Instructor laptop workspace.",
    },
    phone: {
      status: "covered_by",
      target: checkpoint("forest", "light"),
      reason: "Course Theme comparisons use the canonical Instructor laptop workspace.",
    },
    square: {
      status: "covered_by",
      target: checkpoint("forest", "light"),
      reason: "Course Theme comparisons use the canonical Instructor laptop workspace.",
    },
  },
  run: async (runtime): Promise<void> => {
    let courseInstanceId: string | undefined;
    for (const theme of THEME_VALUES) {
      for (const mode of DISPLAY_MODES) {
        courseInstanceId = await captureThemeSample(runtime, theme, mode, courseInstanceId);
      }
    }
  },
};
