// scenarios_instructor_theme_samples.ts - Persisted Course palette comparisons.

import { COURSE_THEME_VALUES, type CourseTheme } from "../../../generated/api/CourseTheme";

import type { ScenarioDefinition } from "./scenario_types";
import type { ScenarioRuntime } from "./runtime";
import { choosePersona, courseCard, COURSE_TITLE, scrollTop } from "./visible_workflows";

function checkpoint(theme: CourseTheme): string {
  return `theme_sample_${theme.replace(/-/gu, "_")}`;
}

async function captureThemeSample(
  runtime: ScenarioRuntime,
  theme: CourseTheme,
  expectedCourseInstanceId: string | undefined,
): Promise<string> {
  const session = await runtime.open(checkpoint(theme));
  try {
    const page = session.page;
    await choosePersona(page, "Elena Rivera");
    const tabs = page.getByRole("navigation", { name: "Ribbon tabs", exact: true });
    await tabs.getByRole("link", { name: "Courses", exact: true }).click();
    await page.waitForURL((url) => url.pathname === "/instructor");
    await page.locator('[data-route-surface="courses"]').waitFor();
    await page.getByRole("heading", { name: "My Active Courses", exact: true }).waitFor();

    const course = courseCard(page, COURSE_TITLE);
    await course.waitFor({ state: "visible" });
    const courseCount = await course.count();
    if (courseCount !== 1) {
      throw new Error(
        `Expected one seeded Course Instance named ${COURSE_TITLE}; found ${String(courseCount)}.`,
      );
    }
    const courseInstanceId = await course.getAttribute("data-record-id");
    if (courseInstanceId === null || courseInstanceId.length === 0) {
      throw new Error(`Seeded Course ${COURSE_TITLE} has no stable record ID.`);
    }
    if (expectedCourseInstanceId !== undefined && courseInstanceId !== expectedCourseInstanceId) {
      throw new Error(
        `Course theme captures changed Course record ID from ${expectedCourseInstanceId} ` +
          `to ${courseInstanceId}.`,
      );
    }
    await course.getByRole("heading", { name: COURSE_TITLE, exact: true }).waitFor();
    await course.getByRole("link", { name: "Open Course", exact: true }).click();
    await page.locator('[data-route-surface="courseInstance"]').waitFor();
    await page.getByRole("heading", { level: 1, name: COURSE_TITLE, exact: true }).waitFor();
    await page
      .getByRole("navigation", { name: "Course actions", exact: true })
      .getByRole("link", { name: "Appearance", exact: true })
      .click();
    await page.locator('[data-route-surface="courseAppearance"]').waitFor();

    const option = page.locator(`[data-course-theme-option="${theme}"]`);
    const radio = option.getByRole("radio");
    await radio.check();
    const savedThemeResponse = page.waitForResponse((response) => {
      const request = response.request();
      return (
        request.method() === "PUT" &&
        new URL(response.url()).pathname === `/api/course-instances/${courseInstanceId}/appearance`
      );
    });
    await page.getByRole("button", { name: "Save theme", exact: true }).click();
    const saved = await savedThemeResponse;
    if (!saved.ok()) {
      throw new Error(`Course theme ${theme} did not persist for ${courseInstanceId}.`);
    }
    const savedAppearance = (await saved.json()) as { readonly theme?: string };
    if (savedAppearance.theme !== theme) {
      throw new Error(
        `Course ${courseInstanceId} saved ${String(savedAppearance.theme)} instead of ${theme}.`,
      );
    }

    await page.reload({ waitUntil: "commit" });
    await page.locator('[data-route-surface="courseAppearance"]').waitFor();
    await radio.waitFor({ state: "visible" });
    if (!(await radio.isChecked())) {
      throw new Error(
        `Course ${courseInstanceId} did not reload with its persisted ${theme} theme.`,
      );
    }

    await tabs.getByRole("link", { name: "Courses", exact: true }).click();
    await page.waitForURL((url) => url.pathname === "/instructor");
    await page.locator('[data-route-surface="courses"]').waitFor();
    await page.getByRole("heading", { name: "My Active Courses", exact: true }).waitFor();
    const activeCourse = courseCard(page, COURSE_TITLE);
    await activeCourse.waitFor({ state: "visible" });
    await activeCourse.getByRole("link", { name: "Open Course", exact: true }).click();
    await page.locator('[data-route-surface="courseInstance"]').waitFor();
    await page.getByRole("heading", { level: 1, name: COURSE_TITLE, exact: true }).waitFor();
    await page
      .locator(
        `.course-theme-scope[data-course-theme="${theme}"][data-course-instance-id="${courseInstanceId}"]`,
      )
      .waitFor();
    await scrollTop(page);
    await runtime.captureCheckpoint(session, checkpoint(theme));
    return courseInstanceId;
  } finally {
    await runtime.close(session);
  }
}

export const INSTRUCTOR_THEME_SAMPLE_SCENARIO: ScenarioDefinition = {
  id: "instructor_theme_samples",
  role: "instructor",
  captures: COURSE_THEME_VALUES.map((theme) => ({
    checkpoint: checkpoint(theme),
    filenameStem: `theme_sample-${theme}`,
    area: "courses",
    workflow: "Course theme comparison",
    state: `theme sample ${theme}`,
    viewport: "laptop",
    privacyProfile: "instructor_answer_free",
    caption: `Instructor Course workspace rendered with the ${theme} theme`,
  })),
  viewportCoverage: {
    laptop: { status: "captured" },
    tablet: {
      status: "covered_by",
      target: "theme_sample_forest",
      reason: "Course theme comparisons use the canonical Instructor laptop workspace.",
    },
    phone: {
      status: "covered_by",
      target: "theme_sample_forest",
      reason: "Course theme comparisons use the canonical Instructor laptop workspace.",
    },
    square: {
      status: "covered_by",
      target: "theme_sample_forest",
      reason: "Course theme comparisons use the canonical Instructor laptop workspace.",
    },
  },
  run: async (runtime): Promise<void> => {
    let courseInstanceId: string | undefined;
    for (const theme of COURSE_THEME_VALUES) {
      courseInstanceId = await captureThemeSample(runtime, theme, courseInstanceId);
    }
  },
};
