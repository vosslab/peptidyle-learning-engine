// Family-neutral visible UI helpers for real production-stack scenarios.
import { expect, type BrowserContext, type Locator, type Page } from "@playwright/test";
import type { BrowserScenarioInputV1 } from "../browser_suite_live_config";

export async function chooseSeededIdentity(page: Page, name: RegExp): Promise<void> {
  await page.goto("/sign-in");
  await expect(
    page.getByRole("heading", { level: 1, name: "Sign in to PLE", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: new RegExp(`Continue as .*${name.source}`, "i") }).click();
  await expect(page.getByRole("heading", { name: "Choose your course" })).toBeVisible();
}

export function courseChoice(page: Page, title: string): Locator {
  return page
    .getByRole("heading", { name: "Choose your course" })
    .locator("..")
    .getByRole("button")
    .filter({ hasText: title });
}

export async function selectVisibleCourse(page: Page, title: string): Promise<void> {
  const choice = courseChoice(page, title);
  await expect(choice).toHaveCount(1);
  await choice.click();
  await expect(page.getByRole("main")).toBeVisible();
}

export async function restoreViewportOrigin(page: Page): Promise<void> {
  await page.evaluate(() => window.scrollTo(0, 0));
  await expect.poll(() => page.evaluate(() => [window.scrollX, window.scrollY])).toEqual([0, 0]);
}

export async function signOutVisible(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: "Sign in to PLE", exact: true }),
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
