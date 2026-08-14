// Browser coverage for course-scoped themes and rendered contrast.

import { expect, test, type Page } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import {
  COURSE_THEME_CATALOG,
  courseThemeStyle,
} from "../../src/features/course_appearance/theme_catalog";
import {
  BANNER_ID,
  ASSIGNMENT_REFERENCE,
  COURSE_ID,
  COURSE_REFERENCE,
  appearanceHeaders,
  bannerBytes,
  json,
  session,
} from "./course_appearance_fixtures";

const SECONDARY_COURSE_REFERENCE = "C-2";
const RUN_REFERENCE = "R-4";

async function navigateWithinSpa(page: Page, pathname: string): Promise<void> {
  await page.evaluate((nextPath) => {
    history.pushState({}, "", nextPath);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, pathname);
}

function channel(value: number): number {
  const normalized = value / 255;
  return normalized <= 0.04045 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
}

function luminance(cssColor: string): number {
  const values = cssColor
    .match(/[0-9.]+/gu)
    ?.slice(0, 3)
    .map(Number);
  if (values === undefined || values.length !== 3) {
    throw new Error(`Expected a computed RGB color, received ${cssColor}`);
  }
  const channels = cssColor.startsWith("color(srgb") ? values.map((value) => value * 255) : values;
  const [red, green, blue] = channels;
  if (red === undefined || green === undefined || blue === undefined) {
    throw new Error(`Expected three computed RGB channels, received ${cssColor}`);
  }
  return 0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue);
}

function contrast(first: string, second: string): number {
  const lighter = Math.max(luminance(first), luminance(second));
  const darker = Math.min(luminance(first), luminance(second));
  return (lighter + 0.05) / (darker + 0.05);
}

test("course identity and readable palette surfaces stay scoped to learner routes", async ({
  page,
}) => {
  await page.goto("/");
  const header = page.locator("header.site-header");
  const globalHeader = await header.evaluate((element) => {
    const style = getComputedStyle(element);
    return { color: style.color, background: style.backgroundColor };
  });

  await navigateWithinSpa(page, `/courses/${COURSE_REFERENCE}`);
  const scope = page.locator('[data-course-theme="grass"]');
  await expect(scope).toBeVisible();
  await expect(scope).toHaveAttribute("data-course-reference", COURSE_REFERENCE);
  await expect(scope).toHaveCSS("background-color", "rgb(189, 222, 177)");
  const grassRail = await scope.evaluate((element) => getComputedStyle(element).backgroundImage);
  expect(grassRail).toContain("rgb(115, 193, 103)");
  expect(grassRail).toContain("rgb(0, 136, 82)");
  await expect(page.locator(".course-entry-banner")).toHaveCount(0);
  await expect(header).toHaveCSS("color", globalHeader.color);
  await expect(header).toHaveCSS("background-color", globalHeader.background);

  await navigateWithinSpa(page, `/runs/${RUN_REFERENCE}`);
  await expect(page.locator('[data-course-theme="grass"]')).toBeVisible();
  await expect(page.locator('[data-route-surface="runAttempt"]')).toBeVisible();

  await navigateWithinSpa(page, `/runs/${RUN_REFERENCE}/summary`);
  await expect(page.locator('[data-course-theme="grass"]')).toBeVisible();
  await expect(page.locator('[data-route-surface="runSummary"]')).toBeVisible();

  await navigateWithinSpa(page, `/courses/${SECONDARY_COURSE_REFERENCE}`);
  const oceanScope = page.locator('[data-course-theme="ocean"]');
  await expect(oceanScope).toBeVisible();
  await expect(oceanScope).toHaveAttribute("data-course-reference", SECONDARY_COURSE_REFERENCE);
  await expect(oceanScope).toHaveCSS("background-color", "rgb(221, 239, 245)");
  const oceanRail = await oceanScope.evaluate(
    (element) => getComputedStyle(element).backgroundImage,
  );
  expect(oceanRail).toContain("rgb(11, 108, 136)");
  expect(oceanRail).toContain("rgb(18, 60, 105)");
  await expect(page.locator('[data-course-theme="grass"]')).toHaveCount(0);

  await navigateWithinSpa(page, "/library");
  await expect(page.locator(".course-theme-scope")).toHaveCount(0);
  await expect(page.locator('[data-route-surface="library"]')).toBeVisible();
  await expect(header).toHaveCSS("color", globalHeader.color);
  await expect(header).toHaveCSS("background-color", globalHeader.background);
});

test("instructor editor, gradebook, and settings use the authorized course theme", async ({
  page,
}) => {
  const assignment = publishedProblemFixture.assignment;
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    if (path === `/api/navigation/${COURSE_REFERENCE}`) {
      return await json(route, { kind: "course", courseId: COURSE_ID });
    }
    if (path === `/api/navigation/${ASSIGNMENT_REFERENCE}`) {
      return await json(route, {
        kind: "assignment",
        courseId: COURSE_ID,
        assignmentId: assignment.id,
      });
    }
    if (path === `/api/courses/${COURSE_ID}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/courses/${COURSE_ID}/appearance`) {
      return await json(
        route,
        { theme: "grass", revision: "1", banner: null },
        200,
        appearanceHeaders("1"),
      );
    }
    if (path === `/api/assignments/${assignment.id}`) {
      return await json(route, assignment, 200, { etag: '"1"' });
    }
    if (path === `/api/courses/${COURSE_ID}/gradebook`) {
      return await json(route, { items: [], nextCursor: null });
    }
    return await json(route, { error: `unexpected instructor scope request ${path}` }, 500);
  });
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.goto("/");

  for (const [path, surface] of [
    [
      `/instructor/courses/${COURSE_REFERENCE}/assignments/${ASSIGNMENT_REFERENCE}/edit`,
      "assignmentEditor",
    ],
    [`/instructor/courses/${COURSE_REFERENCE}/gradebook`, "gradebook"],
    [`/instructor/courses/${COURSE_REFERENCE}/appearance`, "courseAppearance"],
  ] as const) {
    await navigateWithinSpa(page, path);
    const scope = page.locator('[data-course-theme="grass"]');
    await expect(scope).toBeVisible();
    await expect(scope).toHaveAttribute("data-course-reference", COURSE_REFERENCE);
    await expect(page.locator(`[data-route-surface="${surface}"]`)).toBeVisible();
  }
});

test("the authorized banner and text title render only at course entry", async ({ page }) => {
  test.setTimeout(2_000);
  const assignment = publishedProblemFixture.assignment;
  const appearance = {
    theme: "grass",
    revision: "1",
    banner: {
      id: BANNER_ID,
      alternativeText: {
        kind: "informative",
        text: "Green protein ribbons crossing a cell membrane",
      },
    },
  };
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;
    if (path === "/api/auth/session") return await json(route, session(["student"]));
    if (path === `/api/navigation/${COURSE_REFERENCE}`) {
      return await json(route, { kind: "course", courseId: COURSE_ID });
    }
    if (path === `/api/navigation/${ASSIGNMENT_REFERENCE}`) {
      return await json(route, {
        kind: "assignment",
        courseId: COURSE_ID,
        assignmentId: assignment.id,
      });
    }
    if (path === `/api/courses/${COURSE_ID}`) {
      return await json(route, publishedProblemFixture.course);
    }
    if (path === `/api/courses/${COURSE_ID}/appearance`) {
      return await json(route, appearance, 200, appearanceHeaders("1"));
    }
    if (path === `/api/courses/${COURSE_ID}/assignments`) {
      return await json(route, { items: [assignment], nextCursor: null });
    }
    if (path === `/api/assignments/${assignment.id}`) return await json(route, assignment);
    if (path === `/api/assets/${BANNER_ID}/delivery`) {
      expect(request.method()).toBe("POST");
      return await json(
        route,
        { url: `${url.origin}/api/assets/${BANNER_ID}?signed=course-entry` },
        200,
        { "cache-control": "no-store" },
      );
    }
    if (path === `/api/assets/${BANNER_ID}`) {
      expect(request.method()).toBe("GET");
      expect(url.searchParams.get("signed")).toBe("course-entry");
      return await route.fulfill({ status: 200, contentType: "image/png", body: bannerBytes });
    }
    return await json(route, { error: `unexpected entry-only request ${path}` }, 500);
  });
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.goto("/");

  await navigateWithinSpa(page, `/courses/${COURSE_REFERENCE}`);
  await expect(
    page.getByRole("heading", { name: publishedProblemFixture.course.title }),
  ).toBeVisible();
  await expect(
    page.getByRole("img", { name: "Green protein ribbons crossing a cell membrane" }),
  ).toHaveCount(1);
  await expect(page.getByRole("heading", { name: "Assignments" })).toBeVisible();

  await navigateWithinSpa(page, `/courses/${COURSE_REFERENCE}/assignments/${ASSIGNMENT_REFERENCE}`);
  await expect(page.locator('[data-route-surface="assignmentOverview"]')).toBeVisible();
  await expect(page.locator(".course-entry-banner")).toHaveCount(0);
  await expect(page.locator('[data-course-theme="grass"]')).toBeVisible();

  await navigateWithinSpa(page, "/library");
  await expect(page.locator(".course-entry-banner")).toHaveCount(0);
  await expect(page.locator(".course-theme-scope")).toHaveCount(0);
});

test("every catalog theme meets rendered text, action, active-navigation, and focus contrast", async ({
  page,
}) => {
  await page.goto("/");
  await navigateWithinSpa(page, `/courses/${COURSE_REFERENCE}`);
  const scope = page.locator(".course-theme-scope");
  await expect(scope).toBeVisible();

  for (const [themeId, tokens] of Object.entries(COURSE_THEME_CATALOG)) {
    const metrics = await scope.evaluate((element, styleText) => {
      element.setAttribute("style", styleText);
      const probe = document.createElement("div");
      probe.innerHTML = `
          <p data-probe="ink">Body text</p>
          <p class="page-lede" data-probe="muted">Supporting text</p>
          <a data-probe="link" href="#probe">Text link</a>
          <button class="primary-action" data-probe="action" type="button">Action</button>
          <span data-probe="hover" style="background: var(--ple-action-hover); color: var(--ple-on-action)">Hover</span>
          <span data-probe="secondary" style="background: var(--ple-theme-secondary); color: var(--ple-theme-on-secondary)">Active course navigation</span>
          <span data-probe="surface" style="background: var(--ple-card-surface); color: var(--ple-ink)">Card</span>
          <span data-probe="focus" style="border: 4px solid var(--ple-focus); background: var(--ple-card-surface)">Focus</span>
          <span data-probe="border" style="border: 4px solid var(--ple-border); background: var(--ple-card-surface)">Border</span>
        `;
      element.prepend(probe);
      const computed = (name: string): CSSStyleDeclaration =>
        getComputedStyle(probe.querySelector(`[data-probe="${name}"]`) as Element);
      const canvas = getComputedStyle(element).backgroundColor;
      const result = {
        canvas,
        ink: computed("ink").color,
        muted: computed("muted").color,
        link: computed("link").color,
        action: {
          foreground: computed("action").color,
          background: computed("action").backgroundColor,
        },
        hover: {
          foreground: computed("hover").color,
          background: computed("hover").backgroundColor,
        },
        secondary: {
          foreground: computed("secondary").color,
          background: computed("secondary").backgroundColor,
        },
        surface: {
          foreground: computed("surface").color,
          background: computed("surface").backgroundColor,
        },
        focus: computed("focus").borderTopColor,
        border: computed("border").borderTopColor,
      };
      probe.remove();
      return result;
    }, courseThemeStyle(tokens));

    for (const [name, foreground] of [
      ["ink", metrics.ink],
      ["muted", metrics.muted],
      ["link", metrics.link],
    ] as const) {
      const ratio = contrast(foreground, metrics.canvas);
      expect(ratio, `${themeId} ${name} on canvas`).toBeGreaterThanOrEqual(5.5);
      expect(ratio, `${themeId} ${name} standard-mode contrast ceiling`).toBeLessThanOrEqual(8.25);
    }
    expect(
      contrast(metrics.action.foreground, metrics.action.background),
      `${themeId} action`,
    ).toBeGreaterThanOrEqual(5.5);
    expect(
      contrast(metrics.hover.foreground, metrics.hover.background),
      `${themeId} hover`,
    ).toBeGreaterThanOrEqual(5.5);
    expect(
      contrast(metrics.secondary.foreground, metrics.secondary.background),
      `${themeId} active course navigation`,
    ).toBeGreaterThanOrEqual(5.5);
    expect(
      contrast(metrics.surface.foreground, metrics.surface.background),
      `${themeId} card text`,
    ).toBeGreaterThanOrEqual(5.5);
    expect(
      contrast(metrics.surface.foreground, metrics.surface.background),
      `${themeId} card text standard-mode contrast ceiling`,
    ).toBeLessThanOrEqual(8.25);
    for (const [name, foreground] of [["focus", metrics.focus]] as const) {
      expect(
        contrast(foreground, metrics.canvas),
        `${themeId} ${name} on canvas`,
      ).toBeGreaterThanOrEqual(3);
      expect(
        contrast(foreground, metrics.surface.background),
        `${themeId} ${name} on card`,
      ).toBeGreaterThanOrEqual(3);
    }
  }
});

test("the compact learner question keeps prompt, response, and timer in one adaptable flow", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await navigateWithinSpa(page, `/runs/${RUN_REFERENCE}`);

  const runHeader = page.locator(".run-header");
  const timer = page.getByRole("timer");
  const prompt = page.locator(".prompt-copy");
  const response = page.locator(".attempt-response");
  await expect(runHeader).toBeVisible();
  await expect(timer).toBeVisible();
  await expect(prompt).toBeVisible();
  await expect(response).toBeVisible();
  await expect(page.locator(".choice-card")).toHaveCount(3);

  const geometry = await page.evaluate(() => {
    const bounds = (selector: string): DOMRect => {
      const element = document.querySelector<HTMLElement>(selector);
      if (element === null) throw new Error(`missing learner layout element ${selector}`);
      return element.getBoundingClientRect();
    };
    const header = bounds(".run-header");
    const timerBox = bounds('[role="timer"]');
    const promptBox = bounds(".prompt-copy");
    const responseBox = bounds(".attempt-response");
    const skipLink = bounds(".skip-link");
    const main = document.querySelector<HTMLElement>('main[tabindex="-1"]');
    const siteHeader = document.querySelector<HTMLElement>("header.site-header");
    if (main === null) throw new Error("missing route focus target");
    if (siteHeader === null) throw new Error("missing site header");
    return {
      timerInsideHeader:
        timerBox.left >= header.left &&
        timerBox.right <= header.right &&
        timerBox.top >= header.top &&
        timerBox.bottom <= header.bottom,
      responseAfterPrompt: responseBox.top >= promptBox.bottom - 2,
      skipLinkHidden: skipLink.bottom <= 0,
      activeElement: document.activeElement?.className ?? document.activeElement?.tagName ?? "none",
      mainOutline: getComputedStyle(main).outlineStyle,
      siteHeaderHeight: Math.round(siteHeader.getBoundingClientRect().height),
      horizontalOverflow:
        document.documentElement.scrollWidth - document.documentElement.clientWidth,
    };
  });
  expect(geometry.timerInsideHeader).toBe(true);
  expect(geometry.responseAfterPrompt).toBe(true);
  expect(geometry.skipLinkHidden, JSON.stringify(geometry)).toBe(true);
  expect(geometry.mainOutline).toBe("none");
  expect(geometry.siteHeaderHeight).toBeLessThanOrEqual(64);
  expect(geometry.horizontalOverflow).toBe(0);
  await expect(page.locator("footer")).toHaveCount(0);

  if (process.env["PLE_CAPTURE_UI_DESIGN_VISUALS"] === "1") {
    const directory = resolve("generated/ui/ui_design");
    await mkdir(directory, { recursive: true });
    await page.screenshot({ path: resolve(directory, "student_question_390x844.png") });
    await page.setViewportSize({ width: 800, height: 1_280 });
    await expect(response).toBeVisible();
    const tabletOverflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(tabletOverflow).toBe(0);
    await page.screenshot({ path: resolve(directory, "student_question_800x1280.png") });
  }
});

test("increased contrast is account-selectable without replacing the course palette", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1_280, height: 800 });
  await page.goto("/");
  await navigateWithinSpa(page, "/account/security");
  const contrastSelect = page.getByLabel("Contrast level");
  await expect(contrastSelect).toHaveValue("standard");
  await contrastSelect.selectOption("increased");
  await expect(page.locator("html")).toHaveAttribute("data-contrast", "increased");

  await navigateWithinSpa(page, `/courses/${COURSE_REFERENCE}`);
  const scope = page.locator('[data-course-theme="grass"]');
  await expect(scope).toBeVisible();
  await expect(scope).toHaveCSS("background-color", "rgb(189, 222, 177)");
  const increased = await scope.evaluate((element) => {
    const styles = getComputedStyle(element);
    return {
      ink: styles.getPropertyValue("--ple-ink").trim(),
      secondary: styles.getPropertyValue("--ple-theme-secondary").trim(),
      accent: styles.getPropertyValue("--ple-theme-accent").trim(),
    };
  });
  expect(increased).toEqual({
    ink: "#111827",
    secondary: "#73c167",
    accent: "#008852",
  });

  if (process.env["PLE_CAPTURE_UI_DESIGN_VISUALS"] === "1") {
    const directory = resolve("generated/ui/ui_design");
    await mkdir(directory, { recursive: true });
    await page.screenshot({ path: resolve(directory, "increased_contrast_course_1280x800.png") });
  }

  await navigateWithinSpa(page, "/account/security");
  await expect(contrastSelect).toHaveValue("increased");
  await contrastSelect.selectOption("standard");
  await expect(page.locator("html")).toHaveAttribute("data-contrast", "standard");
});

test("the desktop gradebook uses human identity and composed on-demand history", async ({
  page,
}) => {
  const row = publishedProblemFixture.gradebook[0];
  if (row === undefined) throw new Error("gradebook fixture is missing");
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    if (path === "/api/auth/account/presentation") {
      return await json(route, { contrast: "standard" });
    }
    if (path === `/api/navigation/${COURSE_REFERENCE}`) {
      return await json(route, { kind: "course", courseId: COURSE_ID });
    }
    if (path === `/api/courses/${COURSE_ID}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/courses/${COURSE_ID}/appearance`) {
      return await json(
        route,
        { theme: "grass", revision: "1", banner: null },
        200,
        appearanceHeaders("1"),
      );
    }
    if (path === `/api/courses/${COURSE_ID}/gradebook`) {
      return await json(route, { items: [row], nextCursor: null });
    }
    if (path === `/api/enrollments/${row.enrollmentId}/runs`) {
      return await json(route, { items: publishedProblemFixture.runs, nextCursor: null });
    }
    return await json(route, { error: `unexpected gradebook layout request ${path}` }, 500);
  });
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.setViewportSize({ width: 1_280, height: 800 });
  await page.goto("/");
  await navigateWithinSpa(page, `/instructor/courses/${COURSE_REFERENCE}/gradebook`);

  await expect(page.getByRole("heading", { name: "Gradebook" })).toBeVisible();
  const courseNavigation = page.getByRole("navigation", { name: "Course management" });
  await expect(courseNavigation).toBeVisible();
  await expect(
    courseNavigation.getByRole("link", { name: "Assignments", exact: true }),
  ).toBeVisible();
  await expect(courseNavigation.getByRole("link", { name: "Gradebook" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  await expect(page.getByRole("cell", { name: row.learnerName })).toBeVisible();
  await expect(page.locator("body")).not.toContainText(row.studentId);
  await page.getByRole("button", { name: "View run history" }).click();
  await expect(
    page.getByRole("region", { name: `Run history for learner ${row.learnerName}` }),
  ).toBeVisible();
  await expect(page.locator(".run-history-list li")).toHaveCount(4);

  const tableWidth = await page
    .locator(".gradebook-table-wrap")
    .evaluate((element) => Math.round(element.getBoundingClientRect().width));
  expect(tableWidth).toBeGreaterThanOrEqual(1_100);
  await expect(page.locator("footer")).toHaveCount(0);

  if (process.env["PLE_CAPTURE_UI_DESIGN_VISUALS"] === "1") {
    const directory = resolve("generated/ui/ui_design");
    await mkdir(directory, { recursive: true });
    await page.screenshot({ path: resolve(directory, "gradebook_1280x800.png") });
  }
});

test("the desktop problem library gives the browse task the useful screen width", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1_280, height: 800 });
  await page.goto("/");
  await navigateWithinSpa(page, "/library");
  await expect(page.getByRole("heading", { name: "Question library" })).toBeVisible();
  await expect(page.locator(".catalog-row").first()).toBeVisible();

  const geometry = await page.evaluate(() => {
    const box = (selector: string): DOMRect => {
      const element = document.querySelector<HTMLElement>(selector);
      if (element === null) throw new Error(`missing library layout element ${selector}`);
      return element.getBoundingClientRect();
    };
    const controls = box(".catalog-controls");
    const windowBox = box(".catalog-window");
    const firstRow = box(".catalog-row");
    return {
      controlsWidth: Math.round(controls.width),
      windowWidth: Math.round(windowBox.width),
      windowHeight: Math.round(windowBox.height),
      loadedRows: document.querySelectorAll(".catalog-row").length,
      alignedLeft: Math.abs(controls.left - firstRow.left) <= 2,
      alignedRight: Math.abs(controls.right - firstRow.right) <= 2,
      horizontalOverflow:
        document.documentElement.scrollWidth - document.documentElement.clientWidth,
    };
  });
  expect(geometry.controlsWidth).toBeGreaterThanOrEqual(1_100);
  expect(geometry.windowWidth).toBeGreaterThanOrEqual(1_100);
  expect(geometry.windowHeight).toBeLessThanOrEqual(geometry.loadedRows * 106);
  expect(geometry.alignedLeft).toBe(true);
  expect(geometry.alignedRight).toBe(true);
  expect(geometry.horizontalOverflow).toBe(0);

  if (process.env["PLE_CAPTURE_UI_DESIGN_VISUALS"] === "1") {
    const directory = resolve("generated/ui/ui_design");
    await mkdir(directory, { recursive: true });
    await page.screenshot({ path: resolve(directory, "problem_library_1280x800.png") });
  }
});

test("the instructor workspace keeps global and course-level navigation distinct", async ({
  page,
}) => {
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    if (path === "/api/auth/account/presentation") {
      return await json(route, { contrast: "standard" });
    }
    if (path === "/api/courses") {
      return await json(route, {
        items: [{ ...publishedProblemFixture.course, role: "instructor" }],
        nextCursor: null,
      });
    }
    if (path === `/api/navigation/${COURSE_REFERENCE}`) {
      return await json(route, { kind: "course", courseId: COURSE_ID });
    }
    if (path === `/api/courses/${COURSE_ID}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/courses/${COURSE_ID}/appearance`) {
      return await json(
        route,
        { theme: "grass", revision: "1", banner: null },
        200,
        appearanceHeaders("1"),
      );
    }
    if (path === `/api/courses/${COURSE_ID}/assignments`) {
      return await json(route, {
        items: [publishedProblemFixture.assignment],
        nextCursor: null,
      });
    }
    return await json(route, { error: `unexpected instructor workspace request ${path}` }, 500);
  });
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.setViewportSize({ width: 1_280, height: 800 });
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Courses you teach" })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Primary navigation" })).toContainText(
    "Workspace",
  );
  await expect(page.getByRole("navigation", { name: "Course management" })).toHaveCount(0);
  if (process.env["PLE_CAPTURE_UI_DESIGN_VISUALS"] === "1") {
    const directory = resolve("generated/ui/ui_design");
    await mkdir(directory, { recursive: true });
    await page.screenshot({ path: resolve(directory, "instructor_courses_1280x800.png") });
  }

  await navigateWithinSpa(page, `/courses/${COURSE_REFERENCE}`);
  const courseNavigation = page.getByRole("navigation", { name: "Course management" });
  await expect(courseNavigation).toBeVisible();
  await expect(courseNavigation).toContainText("Assignments");
  await expect(courseNavigation).toContainText("New assignment");
  await expect(courseNavigation).toContainText("Students");
  await expect(courseNavigation).toContainText("Gradebook");
  await expect(courseNavigation).toContainText("Appearance");
  if (process.env["PLE_CAPTURE_UI_DESIGN_VISUALS"] === "1") {
    await page.screenshot({
      path: resolve("generated/ui/ui_design/instructor_course_1280x800.png"),
    });
  }
});
