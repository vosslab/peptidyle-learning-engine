// theme_capture_workflow.ts - visible persistence checks shared by Theme evidence scenarios.

import type { Page } from "playwright";

import type { DisplayMode } from "../../../generated/api/DisplayMode";
import type { Theme } from "../../../generated/api/Theme";
import { choosePersona, courseCard, COURSE_TITLE } from "./visible_workflows";

export interface DocumentAppearance {
  readonly theme: Theme;
  readonly mode: DisplayMode;
}

export async function assertDocumentAppearance(
  page: Page,
  expected: DocumentAppearance,
): Promise<void> {
  await page
    .locator(`html[data-theme="${expected.theme}"][data-display-mode="${expected.mode}"]`)
    .waitFor();
  const actual = await page.locator("html").evaluate((root) => ({
    theme: root.dataset.theme,
    mode: root.dataset.displayMode,
  }));
  if (actual.theme !== expected.theme || actual.mode !== expected.mode) {
    throw new Error(
      `document appearance is ${String(actual.theme)}/${String(actual.mode)}; ` +
        `expected ${expected.theme}/${expected.mode}`,
    );
  }
}

async function switchDisplayMode(page: Page, target: DisplayMode): Promise<void> {
  await page
    .getByRole("button", {
      name: target === "dark" ? "Switch to Dark" : "Switch to Light",
      exact: true,
    })
    .click();
  await page.locator(`html[data-display-mode="${target}"]`).waitFor();
}

/** Persist a concrete Light or Dark preference even when the current rendering already matches it. */
export async function persistDisplayMode(page: Page, target: DisplayMode): Promise<void> {
  const current = await page.locator("html").getAttribute("data-display-mode");
  if (current !== "light" && current !== "dark") {
    throw new Error(`document has no valid display mode before persistence: ${String(current)}`);
  }
  if (current === target) await switchDisplayMode(page, target === "light" ? "dark" : "light");
  await switchDisplayMode(page, target);
  await page.reload({ waitUntil: "commit" });
  await page.locator(`html[data-display-mode="${target}"]`).waitFor();
}

export async function openSeededCourseAppearance(page: Page): Promise<string> {
  await choosePersona(page, "Elena Rivera");
  const tabs = page.getByRole("navigation", { name: "Ribbon tabs", exact: true });
  await tabs.getByRole("link", { name: "Courses", exact: true }).click();
  await page.locator('[data-route-surface="courses"]').waitFor();
  await page.getByRole("heading", { name: "My Active Courses", exact: true }).waitFor();
  const course = courseCard(page, COURSE_TITLE);
  await course.waitFor({ state: "visible" });
  if ((await course.count()) !== 1) {
    throw new Error(`Expected one seeded Course Instance named ${COURSE_TITLE}.`);
  }
  const courseInstanceId = await course.getAttribute("data-record-id");
  if (courseInstanceId === null || courseInstanceId.length === 0) {
    throw new Error(`Seeded Course ${COURSE_TITLE} has no stable record ID.`);
  }
  await course.getByRole("link", { name: "Open Course", exact: true }).click();
  await page.locator('[data-route-surface="courseInstance"]').waitFor();
  await page.getByRole("heading", { level: 1, name: COURSE_TITLE, exact: true }).waitFor();
  await page
    .getByRole("navigation", { name: "Course actions", exact: true })
    .getByRole("link", { name: "Appearance", exact: true })
    .click();
  await page.locator('[data-route-surface="courseAppearance"]').waitFor();
  return courseInstanceId;
}

export async function persistCourseAppearance(
  page: Page,
  courseInstanceId: string,
  expected: DocumentAppearance,
): Promise<void> {
  await persistDisplayMode(page, "light");
  const option = page.locator(`[data-course-theme-option="${expected.theme}"]`);
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
  if (!saved.ok()) throw new Error(`Course Theme ${expected.theme} did not persist.`);
  const savedAppearance = (await saved.json()) as { readonly theme?: string };
  if (savedAppearance.theme !== expected.theme) {
    throw new Error(`Course saved ${String(savedAppearance.theme)} instead of ${expected.theme}.`);
  }
  await page.reload({ waitUntil: "commit" });
  await page.locator('[data-route-surface="courseAppearance"]').waitFor();
  if (
    !(await page
      .locator(`[data-course-theme-option="${expected.theme}"]`)
      .getByRole("radio")
      .isChecked())
  ) {
    throw new Error(`Course Theme ${expected.theme} did not persist across reload.`);
  }
  await assertDocumentAppearance(page, { theme: expected.theme, mode: "light" });
  await persistDisplayMode(page, expected.mode);
  await assertDocumentAppearance(page, expected);
}

export async function openSeededCourseWorkspace(
  page: Page,
  expected: DocumentAppearance,
): Promise<void> {
  const tabs = page.getByRole("navigation", { name: "Ribbon tabs", exact: true });
  await tabs.getByRole("link", { name: "Courses", exact: true }).click();
  await page.locator('[data-route-surface="courses"]').waitFor();
  const course = courseCard(page, COURSE_TITLE);
  await course.getByRole("link", { name: "Open Course", exact: true }).click();
  await page.locator('[data-route-surface="courseInstance"]').waitFor();
  await page.getByRole("heading", { level: 1, name: COURSE_TITLE, exact: true }).waitFor();
  await assertDocumentAppearance(page, expected);
}
