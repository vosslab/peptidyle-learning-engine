// WP-CA5 built-browser proof for course-only scope and rendered contrast.

import { expect, test, type Page } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import {
  COURSE_THEME_CATALOG,
  courseThemeStyle,
} from "../../src/features/course_appearance/theme_catalog";
import {
  BANNER_ID,
  COURSE_ID,
  appearanceHeaders,
  bannerBytes,
  json,
  session,
} from "./course_appearance_fixtures";

const SECONDARY_COURSE_ID = "0198e000-0000-7000-8000-000000000015";
const RUN_ID = "0198e000-0000-7000-8000-000000000023";

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
  const channels = cssColor
    .match(/[0-9.]+/gu)
    ?.slice(0, 3)
    .map(Number);
  if (channels === undefined || channels.length !== 3) {
    throw new Error(`Expected a computed RGB color, received ${cssColor}`);
  }
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

test("course identity rails scope learner routes without tinting the reading surface", async ({
  page,
}) => {
  await page.goto("/");
  const header = page.locator("header.site-header");
  const globalHeader = await header.evaluate((element) => {
    const style = getComputedStyle(element);
    return { color: style.color, background: style.backgroundColor };
  });

  await navigateWithinSpa(page, `/courses/${COURSE_ID}`);
  const scope = page.locator('[data-course-theme="grass"]');
  await expect(scope).toBeVisible();
  await expect(scope).toHaveAttribute("data-course-id", COURSE_ID);
  await expect(scope).toHaveCSS("background-color", "rgb(255, 255, 255)");
  const grassRail = await scope.evaluate((element) => getComputedStyle(element).backgroundImage);
  expect(grassRail).toContain("rgb(115, 193, 103)");
  expect(grassRail).toContain("rgb(0, 136, 82)");
  await expect(page.locator(".course-entry-banner")).toHaveCount(0);
  await expect(header).toHaveCSS("color", globalHeader.color);
  await expect(header).toHaveCSS("background-color", globalHeader.background);

  await navigateWithinSpa(page, `/runs/${RUN_ID}`);
  await expect(page.locator('[data-course-theme="grass"]')).toBeVisible();
  await expect(page.locator('[data-route-surface="runAttempt"]')).toBeVisible();

  await navigateWithinSpa(page, `/runs/${RUN_ID}/summary`);
  await expect(page.locator('[data-course-theme="grass"]')).toBeVisible();
  await expect(page.locator('[data-route-surface="runSummary"]')).toBeVisible();

  await navigateWithinSpa(page, `/courses/${SECONDARY_COURSE_ID}`);
  const oceanScope = page.locator('[data-course-theme="ocean"]');
  await expect(oceanScope).toBeVisible();
  await expect(oceanScope).toHaveAttribute("data-course-id", SECONDARY_COURSE_ID);
  await expect(oceanScope).toHaveCSS("background-color", "rgb(255, 255, 255)");
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
    [`/instructor/courses/${COURSE_ID}/assignments/${assignment.id}/edit`, "assignmentEditor"],
    [`/instructor/courses/${COURSE_ID}/gradebook`, "gradebook"],
    [`/instructor/courses/${COURSE_ID}/appearance`, "courseAppearance"],
  ] as const) {
    await navigateWithinSpa(page, path);
    const scope = page.locator('[data-course-theme="grass"]');
    await expect(scope).toBeVisible();
    await expect(scope).toHaveAttribute("data-course-id", COURSE_ID);
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

  await navigateWithinSpa(page, `/courses/${COURSE_ID}`);
  await expect(
    page.getByRole("heading", { name: publishedProblemFixture.course.title }),
  ).toBeVisible();
  await expect(
    page.getByRole("img", { name: "Green protein ribbons crossing a cell membrane" }),
  ).toHaveCount(1);
  await expect(page.getByRole("heading", { name: "Assignments" })).toBeVisible();

  await navigateWithinSpa(page, `/courses/${COURSE_ID}/assignments/${assignment.id}`);
  await expect(page.locator('[data-route-surface="assignmentOverview"]')).toBeVisible();
  await expect(page.locator(".course-entry-banner")).toHaveCount(0);
  await expect(page.locator('[data-course-theme="grass"]')).toBeVisible();

  await navigateWithinSpa(page, "/library");
  await expect(page.locator(".course-entry-banner")).toHaveCount(0);
  await expect(page.locator(".course-theme-scope")).toHaveCount(0);
});

test("every catalog theme meets rendered text, action, focus, and boundary contrast", async ({
  page,
}) => {
  await page.goto("/");
  await navigateWithinSpa(page, `/courses/${COURSE_ID}`);
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
          <span data-probe="hover" style="background: var(--ple-action-hover); color: white">Hover</span>
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
      expect(
        contrast(foreground, metrics.canvas),
        `${themeId} ${name} on canvas`,
      ).toBeGreaterThanOrEqual(5.5);
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
      contrast(metrics.surface.foreground, metrics.surface.background),
      `${themeId} card text`,
    ).toBeGreaterThanOrEqual(5.5);
    for (const [name, foreground] of [
      ["focus", metrics.focus],
      ["border", metrics.border],
    ] as const) {
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
