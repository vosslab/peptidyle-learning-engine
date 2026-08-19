// Browser coverage for instructor course appearance editing and banner recovery.

import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import {
  BANNER_ID,
  CANDIDATE_ID,
  COURSE_ID,
  SECOND_CANDIDATE_ID,
  appearanceHeaders,
  bannerBytes,
  json,
  openAppearance,
  resolveCourseReference,
  session,
} from "./course_appearance_fixtures";

test("instructor edits by keyboard, preserves a stale draft, then replaces and removes a banner", async ({
  page,
}) => {
  test.setTimeout(10_000);
  let appearance = { theme: "grass", revision: "1", banner: null } as {
    theme: string;
    revision: string;
    banner: null | {
      id: string;
      alternativeText: { kind: "decorative" } | { kind: "informative"; text: string };
    };
  };
  let candidateUploads = 0;
  let saves = 0;
  const mutations: Array<{
    readonly revision: string | null;
    readonly contentType: string | null;
    readonly body: unknown;
  }> = [];

  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    const navigation = resolveCourseReference(route, path);
    if (navigation !== null) return await navigation;
    if (path === `/api/courses/${COURSE_ID}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/assets/${BANNER_ID}/delivery`) {
      expect(request.method()).toBe("POST");
      return await json(
        route,
        { url: `${url.origin}/api/assets/${BANNER_ID}?signed=course-appearance` },
        200,
        { "cache-control": "no-store" },
      );
    }
    if (path === `/api/assets/${BANNER_ID}`) {
      expect(request.method()).toBe("GET");
      expect(url.searchParams.get("signed")).toBe("course-appearance");
      return await route.fulfill({ status: 200, contentType: "image/png", body: bannerBytes });
    }
    if (path === `/api/courses/${COURSE_ID}/appearance/banner-candidates`) {
      candidateUploads += 1;
      expect(request.method()).toBe("POST");
      expect(request.headers()["content-type"]).toBe("image/png");
      expect(request.postDataBuffer()?.length).toBeGreaterThan(0);
      const candidate = candidateUploads === 1 ? CANDIDATE_ID : SECOND_CANDIDATE_ID;
      return await json(route, { candidate }, 201, { "cache-control": "no-store" });
    }
    if (path === `/api/courses/${COURSE_ID}/appearance` && request.method() === "GET") {
      return await json(route, appearance, 200, appearanceHeaders(appearance.revision));
    }
    if (path === `/api/courses/${COURSE_ID}/appearance` && request.method() === "PUT") {
      saves += 1;
      const body = request.postDataJSON() as {
        theme: string;
        banner:
          | { kind: "remove" }
          | { kind: "keep"; alternativeText: unknown }
          | { kind: "replace"; candidate: string; alternativeText: unknown };
      };
      mutations.push({
        revision: request.headers()["if-match"] ?? null,
        contentType: request.headers()["content-type"] ?? null,
        body,
      });
      if (saves === 1) {
        appearance = { theme: "ocean", revision: "2", banner: null };
        return await json(
          route,
          { error: "course appearance changed; reload current settings" },
          412,
          { "cache-control": "no-store" },
        );
      }
      const revision = (Number(appearance.revision) + 1).toString();
      appearance = {
        theme: body.theme,
        revision,
        banner:
          body.banner.kind === "remove"
            ? null
            : body.banner.kind === "replace"
              ? { id: BANNER_ID, alternativeText: body.banner.alternativeText as never }
              : appearance.banner,
      };
      return await json(route, appearance, 200, appearanceHeaders(revision));
    }
    return await json(route, { error: `unexpected appearance request ${path}` }, 500);
  });

  await page.setViewportSize({ width: 480, height: 900 });
  await openAppearance(page);
  await expect(page.getByRole("heading", { name: "Course appearance" })).toBeVisible();

  const grass = page.getByRole("radio", { name: "Grass" });
  await expect(grass).toBeChecked();
  await grass.focus();
  await page.keyboard.press("ArrowLeft");
  await expect(page.getByRole("radio", { name: "Desert" })).toBeChecked();
  await expect(page.locator('[data-preview-theme="desert"]')).toBeVisible();

  const fileInput = page.getByLabel("Choose a banner image");
  await fileInput.setInputFiles({
    name: "genetics-banner.png",
    mimeType: "image/png",
    buffer: bannerBytes,
  });
  await expect(page.getByText("Selected: genetics-banner.png")).toBeVisible();
  const informative = page.getByRole("radio", { name: /Informative; describe/u });
  await informative.focus();
  await page.keyboard.press("Space");
  const altText = page.getByLabel("Banner alternative text");
  await altText.fill("A labeled chromosome spread");

  const previews = page.locator("img.course-appearance-banner");
  await expect(previews).toHaveCount(2);
  await expect(previews.first()).toHaveAttribute("alt", "A labeled chromosome spread");
  const sizes = await previews.evaluateAll((images) =>
    images.map((image) => {
      const bounds = image.getBoundingClientRect();
      return { width: bounds.width, height: bounds.height };
    }),
  );
  expect(sizes[0]?.width).toBeGreaterThan(sizes[1]?.width ?? Number.POSITIVE_INFINITY);
  for (const size of sizes) {
    expect(size.width / size.height).toBeCloseTo(1200 / 328, 1);
  }
  await expect(page.getByText(publishedProblemFixture.course.title).first()).toBeVisible();

  const save = page.getByRole("button", { name: "Save appearance" });
  await save.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: "A newer course appearance exists" }),
  ).toBeFocused();
  await expect(page.getByRole("radio", { name: "Desert" })).toBeChecked();
  await expect(altText).toHaveValue("A labeled chromosome spread");
  expect(await fileInput.evaluate((input: HTMLInputElement) => input.files?.length)).toBe(1);

  await page.getByRole("button", { name: "Review current appearance" }).click();
  await expect(page.getByRole("radio", { name: "Ocean" })).toBeChecked();
  await expect(page.getByText("Current course appearance loaded")).toBeVisible();
  expect(await fileInput.evaluate((input: HTMLInputElement) => input.files?.length)).toBe(0);

  await page.getByRole("radio", { name: "Forest" }).check();
  await fileInput.setInputFiles({
    name: "replacement.png",
    mimeType: "image/png",
    buffer: bannerBytes,
  });
  await page.getByRole("button", { name: "Save appearance" }).click();
  await expect(page.getByText("Course appearance saved.")).toBeVisible();
  await expect(page.locator(".course-theme-scope")).toHaveAttribute("data-course-theme", "forest");
  await expect(page.locator("img.course-appearance-banner")).toHaveCount(2);

  expect(mutations[1]).toEqual({
    revision: '"2"',
    contentType: "application/json",
    body: {
      theme: "forest",
      banner: {
        kind: "replace",
        candidate: SECOND_CANDIDATE_ID,
        alternativeText: { kind: "decorative" },
      },
    },
  });
  expect(JSON.stringify(mutations[1]?.body).includes("replacement.png")).toBe(false);

  await page.getByRole("button", { name: "Remove banner on save" }).click();
  await expect(page.getByText("Banner removal is ready but not saved.")).toBeVisible();
  await expect(page.locator("img.course-appearance-banner")).toHaveCount(0);
  await page.getByRole("button", { name: "Keep current banner instead" }).click();
  await expect(page.locator("img.course-appearance-banner")).toHaveCount(2);
  await page.getByRole("button", { name: "Remove banner on save" }).click();
  await page.getByRole("button", { name: "Save appearance" }).click();
  await expect(page.getByText("Course appearance saved.")).toBeVisible();
  await expect(page.locator("img.course-appearance-banner")).toHaveCount(0);
  expect(mutations[mutations.length - 1]).toEqual({
    revision: '"3"',
    contentType: "application/json",
    body: { theme: "forest", banner: { kind: "remove" } },
  });
});

test("student route exposes no appearance read or mutation transport", async ({ page }) => {
  const requests: string[] = [];
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    requests.push(`${route.request().method()} ${path}`);
    if (path === "/api/auth/session") return await json(route, session(["student"]));
    if (path === "/api/auth/account/presentation") {
      return await json(route, { contrast: "standard" });
    }
    return await json(route, { error: "student appearance transport must not run" }, 500);
  });
  await openAppearance(page);
  const denial = page.locator('[data-route-surface="routeAccessDenied"]');
  await expect(denial).toHaveAttribute("data-denied-route", "courseAppearance");
  await expect(
    denial.getByRole("heading", { name: "This page is available to instructors only" }),
  ).toBeVisible();
  await expect(page.locator('[data-route-surface="courseAppearance"]')).toHaveCount(0);
  expect(requests).toContain("GET /api/auth/session");
  expect(requests.filter((request) => request.includes("/appearance"))).toEqual([]);
  expect(requests.filter((request) => request.startsWith("PUT "))).toEqual([]);
});

test("settings remain keyboard-visible without horizontal overflow in narrow forced colors", async ({
  page,
}) => {
  const appearance = { theme: "grass", revision: "1", banner: null };
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    const navigation = resolveCourseReference(route, path);
    if (navigation !== null) return await navigation;
    if (path === `/api/courses/${COURSE_ID}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/courses/${COURSE_ID}/appearance`) {
      return await json(route, appearance, 200, appearanceHeaders("1"));
    }
    return await json(route, { error: "unexpected responsive request" }, 500);
  });
  await page.setViewportSize({ width: 320, height: 900 });
  await page.emulateMedia({ forcedColors: "active", reducedMotion: "reduce" });
  await openAppearance(page);

  await expect(page.getByRole("radio", { name: "Grass" })).toBeVisible();
  const dimensions = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    clientWidth: document.documentElement.clientWidth,
    overflowing: Array.from(document.querySelectorAll<HTMLElement>("body *"))
      .filter(
        (element) => element.getBoundingClientRect().right > document.documentElement.clientWidth,
      )
      .map((element) => ({
        tag: element.tagName.toLowerCase(),
        className: element.className,
        right: Math.round(element.getBoundingClientRect().right),
      })),
  }));
  expect(dimensions.scrollWidth, JSON.stringify(dimensions.overflowing)).toBeLessThanOrEqual(
    dimensions.clientWidth,
  );
  const themeCard = page.getByRole("radio", { name: "Grass" }).locator("xpath=ancestor::label");
  expect((await themeCard.boundingBox())?.height).toBeGreaterThanOrEqual(44);
  await page.getByRole("radio", { name: "Grass" }).focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByRole("radio", { name: "Arctic" })).toBeChecked();
  await expect(page.getByRole("button", { name: "Save appearance" })).toBeEnabled();
});

test("instructor appearance settings have no serious or critical axe violations", async ({
  page,
}) => {
  const appearance = { theme: "grass", revision: "1", banner: null };
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    const navigation = resolveCourseReference(route, path);
    if (navigation !== null) return await navigation;
    if (path === `/api/courses/${COURSE_ID}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/courses/${COURSE_ID}/appearance`) {
      return await json(route, appearance, 200, appearanceHeaders("1"));
    }
    return await json(route, { error: "unexpected accessibility request" }, 500);
  });
  await page.setViewportSize({ width: 768, height: 1_000 });
  await openAppearance(page);
  await expect(page.getByRole("heading", { name: "Course appearance" })).toBeVisible();

  const results = await new AxeBuilder({ page }).include("main").analyze();
  const blocking = results.violations.filter(
    (violation) => violation.impact === "critical" || violation.impact === "serious",
  );
  expect(blocking).toEqual([]);
});
