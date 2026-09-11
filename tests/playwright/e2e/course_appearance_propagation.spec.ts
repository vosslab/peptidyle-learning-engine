// Production-stack proof that one Course Appearance persists independently and reaches its members.
// Selector contract: the admitted Appearance Ribbon task is owned by ribbon_catalog.ts; native
// theme radios and banner file input/save controls are owned by course_appearance_page.tsx; the
// enrolled Course-home identity banner is owned by course_entry_identity.tsx.
import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { COURSE_THEME_REGISTRY } from "../../../src/features/course_appearance/course_theme_registry";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  enterStudentCourse,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
  writeContextOriginReceipt,
} from "./real_stack_ui";

const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 120_000;
const seededCourseTitle = "Biochemistry 301: Proteins and Peptides";
const savedTheme = "forest";
const contextOptions = { ignoreHTTPSErrors: true, viewport: { width: 1280, height: 800 } };

// Complete, non-animated PNG. The server decodes these bytes and derives its WebP renditions.
const validPng = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAMAAAACCAYAAACddGYaAAAAIGNIUk0AAHomAACAhAAA+gAAAIDoAAB1MAAA6mAAADqYAAAXcJy6UTwAAAAGYktHRAD/AP8A/6C9p5MAAAATSURBVAjXY+RRsvjPAAVMDEgAAB+cAWl6WZF6AAAAAElFTkSuQmCC",
  "base64",
);

async function openAppearanceFromCourseActions(page: Page): Promise<void> {
  await page
    .getByRole("navigation", { name: "Course actions", exact: true })
    .getByRole("link", { name: "Appearance", exact: true })
    .click();
  await expect(page).toHaveURL(/\/instructor\/courses\/C-[1-9][0-9]*\/appearance$/u);
  await expect(page.locator("#main-content")).toBeFocused();
  await expect(page.locator('[data-route-surface="courseAppearance"]')).toBeVisible();
  await expect(
    page.getByRole("heading", { level: 1, name: "Course Appearance", exact: true }),
  ).toBeVisible();
  await expect(
    page
      .getByRole("navigation", { name: "Ribbon tasks", exact: true })
      .getByRole("link", { name: "Appearance", exact: true }),
  ).toHaveAttribute("aria-current", "page");
}

async function expectSavedTheme(page: Page): Promise<void> {
  const scope = page.locator(`.course-theme-scope[data-course-theme="${savedTheme}"]`);
  await expect(scope).toHaveCount(1);
  await expect(scope).toHaveCSS(
    "--ple-theme-secondary",
    COURSE_THEME_REGISTRY[savedTheme].anchors.secondary,
  );
}

async function createSecondCourseThroughVisibleControls(
  page: Page,
  shortName: string,
  longName: string,
): Promise<void> {
  await page.getByLabel("Blueprint Course Revision").selectOption({ index: 1 });
  await page.getByLabel("Course short name").fill(shortName);
  await page.getByLabel("Course long name").fill(longName);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByRole("button", { name: "Create Course Instance", exact: true }).click();
  const course = page.getByRole("article").filter({
    has: page.getByRole("heading", { name: longName, exact: true }),
  });
  await expect(course).toHaveCount(1);
  await course.getByRole("link", { name: "Open Course Instance", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: longName, exact: true })).toBeVisible();
}

test.describe("Course Appearance propagation on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("Instructor saves both properties and enrolled Student receives only that Course appearance", async ({
    browser,
  }) => {
    test.setTimeout(scenarioTimeoutMs);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("course_appearance_propagation");
    expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-course_appearance_propagation$/u);
    expect(scenarioInput.baselineReads).toEqual(["seeded_accounts", "base_course"]);
    expect(scenarioInput.visibleObservation).toBe(
      "instructor_saved_course_appearance_reloads_for_enrolled_student_only",
    );

    const contexts: BrowserContext[] = [];
    const origins = {
      instructor: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      student: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    };
    try {
      const instructorContext = await browser.newContext(contextOptions);
      contexts.push(instructorContext);
      observeContextOrigins(
        instructorContext,
        origins.instructor.pageOrigins,
        origins.instructor.requestOrigins,
      );
      const instructor = await instructorContext.newPage();
      configureContextAndPage(instructorContext, instructor, actionTimeoutMs);

      await test.step("Instructor opens the admitted Appearance task and saves the theme", async () => {
        await chooseSeededIdentity(instructor, /Elena Rivera/u);
        await selectVisibleCourse(instructor, seededCourseTitle);
        await openAppearanceFromCourseActions(instructor);
        await instructor.getByRole("radio", { name: /^Forest/u }).check();
        await instructor.getByRole("button", { name: "Save theme", exact: true }).click();
        await expect(
          instructor.locator(".course-appearance-form").first().getByRole("status"),
        ).toContainText("Theme saved.");
        await expectSavedTheme(instructor);
      });

      await test.step("Instructor uploads and saves a real PNG banner independently", async () => {
        await instructor.locator("[data-course-banner-file]").setInputFiles({
          name: "course-appearance-propagation.png",
          mimeType: "image/png",
          buffer: validPng,
        });
        await expect(instructor.locator("[data-course-banner-local-preview]")).toBeVisible();
        await instructor.getByRole("button", { name: "Save banner", exact: true }).click();
        await expect(
          instructor.locator(".course-appearance-form").last().getByRole("status"),
        ).toContainText("Banner saved.");
        await expect(instructor.locator("[data-course-banner-saved-preview]")).toBeVisible();
      });

      await test.step("Instructor reload proves the stored theme and saved banner", async () => {
        await instructor.reload();
        await expect(instructor.locator('[data-route-surface="courseAppearance"]')).toBeVisible();
        await expect(instructor.getByRole("radio", { name: /^Forest/u })).toBeChecked();
        await expectSavedTheme(instructor);
        await expect(instructor.locator("[data-course-banner-saved-preview] img")).toHaveCount(2);
      });

      await test.step("Enrolled Student opens the normal Course home and receives its saved appearance", async () => {
        const studentContext = await browser.newContext(contextOptions);
        contexts.push(studentContext);
        observeContextOrigins(
          studentContext,
          origins.student.pageOrigins,
          origins.student.requestOrigins,
        );
        const student = await studentContext.newPage();
        configureContextAndPage(studentContext, student, actionTimeoutMs);
        await chooseSeededIdentity(student, /Mary Okafor/u);
        await enterStudentCourse(student, seededCourseTitle);
        await expectSavedTheme(student);
        const banner = student.locator(".course-entry-banner");
        await expect(banner).toHaveCount(1);
        await expect(banner).toBeVisible();
        await expect(banner).toHaveJSProperty("complete", true);
        await expect
          .poll(() => banner.evaluate((image: HTMLImageElement) => image.naturalWidth))
          .toBeGreaterThan(0);
      });

      await test.step("A second Instructor Course retains the default appearance", async () => {
        await signOutVisible(instructor);
        await instructor.getByRole("button", { name: /Continue as .*Elena Rivera/iu }).click();
        await instructor.getByRole("link", { name: "Courses", exact: true }).click();
        await expect(
          instructor.getByRole("heading", { name: "Course Instances you teach", exact: true }),
        ).toBeVisible();
        const secondCourseShortName = "Appearance";
        const secondCourseLongName = `Appearance isolation ${scenarioInput.namespace}`;
        await createSecondCourseThroughVisibleControls(
          instructor,
          secondCourseShortName,
          secondCourseLongName,
        );
        await openAppearanceFromCourseActions(instructor);
        await expect(instructor.getByRole("radio", { name: /^Grass/u })).toBeChecked();
        await expect(instructor.locator("[data-course-banner-saved-preview]")).toHaveCount(0);
        await expect(
          instructor.getByRole("button", { name: "Remove banner", exact: true }),
        ).toHaveCount(0);
      });
    } finally {
      try {
        await Promise.all(contexts.map((context) => context.close()));
      } finally {
        writeContextOriginReceipt(origins);
      }
    }
  });
});
