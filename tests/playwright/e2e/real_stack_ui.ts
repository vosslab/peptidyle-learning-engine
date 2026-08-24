// Family-neutral visible UI helpers for real production-stack scenarios.
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
  await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
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
