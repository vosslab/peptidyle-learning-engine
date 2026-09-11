// Scenario-neutral visible UI helpers for real production-stack scenarios.
import { writeFileSync } from "node:fs";

import { expect, type BrowserContext, type Locator, type Page } from "@playwright/test";

import type { BrowserScenarioInputV1 } from "../browser_suite_live_config";
import { liveDemoOriginReceiptPathFromEnvironment } from "../browser_suite_live_config";

export interface ObservedOrigins {
  readonly pageOrigins: Set<string>;
  readonly requestOrigins: Set<string>;
}

export function relativeIsoDate(offsetDays: number): string {
  const date = new Date();
  date.setDate(date.getDate() + offsetDays);
  return date.toISOString().slice(0, 10);
}

export function configureContextAndPage(
  context: BrowserContext,
  page: Page,
  timeoutMs: number,
): void {
  context.setDefaultTimeout(timeoutMs);
  context.setDefaultNavigationTimeout(timeoutMs);
  page.setDefaultTimeout(timeoutMs);
  page.setDefaultNavigationTimeout(timeoutMs);
}

export function expectObservedOrigin(origins: ObservedOrigins, expectedOrigin: string): void {
  expect([...origins.pageOrigins].sort()).toEqual([expectedOrigin]);
  expect([...origins.requestOrigins].sort()).toEqual([expectedOrigin]);
}

export function writeOriginReceipt(pageOrigins: Set<string>, requestOrigins: Set<string>): void {
  const value = {
    pageOrigins: [...pageOrigins].sort(),
    requestOrigins: [...requestOrigins].sort(),
  };
  writeOriginReceiptValue(value);
}

export function writeContextOriginReceipt(
  contexts: Readonly<Record<string, ObservedOrigins>>,
  includeContexts = true,
): void {
  for (const name of Object.keys(contexts)) {
    if (!/^[a-z][a-z0-9_]{0,31}$/u.test(name)) {
      throw new Error(`origin receipt context name is outside the canonical contract: ${name}`);
    }
  }
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();
  for (const origins of Object.values(contexts)) {
    for (const origin of origins.pageOrigins) pageOrigins.add(origin);
    for (const origin of origins.requestOrigins) requestOrigins.add(origin);
  }
  const contextValues = Object.fromEntries(
    Object.entries(contexts).map(([name, origins]) => [
      name,
      {
        pageOrigins: [...origins.pageOrigins].sort(),
        requestOrigins: [...origins.requestOrigins].sort(),
      },
    ]),
  );
  const value = includeContexts
    ? {
        pageOrigins: [...pageOrigins].sort(),
        requestOrigins: [...requestOrigins].sort(),
        contexts: contextValues,
      }
    : {
        pageOrigins: [...pageOrigins].sort(),
        requestOrigins: [...requestOrigins].sort(),
      };
  writeOriginReceiptValue(value);
}

function writeOriginReceiptValue(value: object): void {
  writeFileSync(liveDemoOriginReceiptPathFromEnvironment(process.env), JSON.stringify(value), {
    encoding: "ascii",
    flag: "wx",
    mode: 0o600,
  });
}

export async function chooseSeededIdentity(page: Page, name: RegExp): Promise<void> {
  await page.goto("/sign-in");
  await chooseSeededIdentityAtSignIn(page, name);
  await page.getByRole("link", { name: "Courses", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      name: /^(Course Instances you teach|Your Course Instances|Your courses)$/u,
    }),
  ).toBeVisible();
}

export async function chooseSeededIdentityAtSignIn(page: Page, name: RegExp): Promise<void> {
  await expect(
    page.getByRole("heading", { level: 1, name: "Explore Peptidyle Learning Engine", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: new RegExp(`Continue as .*${name.source}`, "i") }).click();
  await expect(page.getByRole("button", { name: "Sign out", exact: true })).toBeVisible();
}

export function courseChoice(page: Page, title: string): Locator {
  return page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: title, exact: true }) });
}

export async function selectVisibleCourse(page: Page, title: string): Promise<void> {
  const choice = courseChoice(page, title);
  await expect(choice).toHaveCount(1);
  await choice.getByRole("link", { name: "Open Course Instance", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
}

export type StudentCourseEntry = "course" | "chooser";

/**
 * Wait for a seeded Student's loaded Course entry state.
 *
 * A Student with one current Course enters that Course directly. The explicit
 * `/?choose=1` path retains the visible Course chooser. The initial `/` list
 * may render briefly while the role-owned Course state is loading, so callers
 * must wait for one of those settled states before interacting with it.
 */
export async function waitForStudentCourseEntry(
  page: Page,
  title: string,
): Promise<StudentCourseEntry> {
  await page.locator(".loading-state").waitFor({ state: "hidden" });
  const entry = await page.waitForFunction((expectedTitle) => {
    for (const heading of document.querySelectorAll("h1")) {
      const style = window.getComputedStyle(heading);
      if (
        heading.textContent?.trim() === expectedTitle &&
        style.visibility !== "hidden" &&
        style.display !== "none"
      ) {
        return "course";
      }
    }

    let visibleCardCount = 0;
    let expectedCardIsVisible = false;
    for (const card of document.querySelectorAll("article.course-card")) {
      const style = window.getComputedStyle(card);
      if (style.visibility === "hidden" || style.display === "none") continue;
      visibleCardCount += 1;
      for (const heading of card.querySelectorAll("h2")) {
        if (heading.textContent?.trim() === expectedTitle) expectedCardIsVisible = true;
      }
    }
    if (!expectedCardIsVisible || window.location.pathname !== "/") return null;

    const choosingCourses = new URLSearchParams(window.location.search).get("choose") === "1";
    return choosingCourses || visibleCardCount > 1 ? "chooser" : null;
  }, title);
  const state = await entry.jsonValue();
  if (state === "course" || state === "chooser") return state;
  throw new Error("Student Course entry did not reach a visible Course or chooser state.");
}

/** Enter a Student's current Course, using the chooser only when it is visible. */
export async function enterStudentCourse(page: Page, title: string): Promise<StudentCourseEntry> {
  const entry = await waitForStudentCourseEntry(page, title);
  if (entry === "chooser") {
    const choice = courseChoice(page, title);
    await expect(choice).toHaveCount(1);
    await choice.getByRole("link", { name: "Open assigned work", exact: true }).click();
    await expect(page.getByRole("heading", { level: 1, name: title, exact: true })).toBeVisible();
  }
  return entry;
}

export type RouteDataSurface = "assignmentOverview" | "assignmentAttempt";

export async function waitForRouteDataSurface(
  page: Page,
  surfaceName: RouteDataSurface,
): Promise<Locator> {
  const surface = page.locator(`[data-route-surface="${surfaceName}"]`);
  await surface.waitFor({ state: "visible" });
  return surface;
}

/**
 * Enter the current assignment's authoritative Assignment Attempt.
 *
 * Locator readiness uses the scenario-owned page timeout. Playwright assertions
 * have a separate five-second default, which is not the live-stack operation
 * boundary configured by `configureContextAndPage`.
 */
export async function startOrContinuePractice(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Start or continue practice", exact: true }).click();
  await waitForRouteDataSurface(page, "assignmentAttempt");
}

export async function restoreViewportOrigin(page: Page): Promise<void> {
  await page.evaluate(() => window.scrollTo(0, 0));
  await expect.poll(() => page.evaluate(() => [window.scrollX, window.scrollY])).toEqual([0, 0]);
}

export async function signOutVisible(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: "Explore Peptidyle Learning Engine", exact: true }),
  ).toBeVisible();
}

export function observeContextOrigins(
  context: BrowserContext,
  pageOrigins: Set<string>,
  requestOrigins: Set<string>,
): void {
  const record = (value: string, target: Set<string>): void => {
    if (value !== "about:blank") target.add(new URL(value).origin);
  };
  context.on("request", (request) => record(request.url(), requestOrigins));
  context.on("page", (page) =>
    page.on("framenavigated", (frame) => {
      if (frame === page.mainFrame()) record(frame.url(), pageOrigins);
    }),
  );
}

export function requireScenarioInput(
  input: BrowserScenarioInputV1 | undefined,
): BrowserScenarioInputV1 {
  if (input === undefined) throw new Error("connected browser scenario inputs were not configured");
  return input;
}
