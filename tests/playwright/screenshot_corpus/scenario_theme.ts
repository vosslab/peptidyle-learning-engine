// Deterministic Course Appearance selection for screenshot-corpus scenarios.

import type { Browser } from "playwright";

import type { Theme } from "../../../generated/api/Theme";

import { enterInstructor, openInstructorCourse } from "./visible_workflows";
import { persistDisplayMode } from "./theme_capture_workflow";

/** Map a stable scenario identifier onto a reviewed persisted Course theme. */
export function scenarioCourseTheme(scenarioId: string): Theme {
  void scenarioId;
  return "grass";
}

/** Describe observed source-level palette variety without imposing a target count. */
export function reportScenarioThemeVariety(
  scenarios: ReadonlyArray<{ readonly id: string }>,
): string {
  const themes = new Set(scenarios.map((scenario) => scenarioCourseTheme(scenario.id)));
  return `Scenario Course Appearance palettes: ${String(themes.size)} default (${[...themes].sort().join(", ")}).`;
}

/**
 * Save the scenario palette through the Course Appearance screen, which exercises the persisted
 * product route instead of applying a page-local CSS override.
 */
export async function persistScenarioCourseTheme(
  browser: Browser,
  entryUrl: URL,
  scenarioId: string,
): Promise<void> {
  const theme = scenarioCourseTheme(scenarioId);
  const context = await browser.newContext({
    colorScheme: "light",
    deviceScaleFactor: 1,
    reducedMotion: "reduce",
    viewport: { width: 1280, height: 800 },
  });
  const page = await context.newPage();
  try {
    await page.goto(entryUrl.href, { waitUntil: "commit" });
    await page
      .getByRole("heading", {
        level: 1,
        name: "Explore Peptidyle Learning Engine",
        exact: true,
      })
      .waitFor();
    await enterInstructor(page);
    await openInstructorCourse(page);
    await page
      .getByRole("navigation", { name: "Course actions", exact: true })
      .getByRole("link", { name: "Appearance", exact: true })
      .click();
    await page.getByRole("heading", { level: 1, name: "Course Appearance", exact: true }).waitFor();
    await persistDisplayMode(page, "light");
    const selection = page.locator(`[data-course-theme-option="${theme}"]`);
    await selection.getByRole("radio").check();
    await page.getByRole("button", { name: "Save theme", exact: true }).click();
    await page.getByText("Theme saved.", { exact: true }).waitFor();
  } finally {
    await context.close();
  }
}
