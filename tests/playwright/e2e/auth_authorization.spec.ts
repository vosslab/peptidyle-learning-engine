// UI-first independent seeded-session and no-record authorization-boundary proof.
//
// Selector contract:
// - src/pages/sign_in_page.tsx owns seeded-demo entry.
// - src/pages/course_list_page.tsx owns the truthful empty Course Instance states.
// - src/route_access_boundary.tsx owns Product Role denial before a protected route reads data.

import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  requireScenarioInput,
  signOutVisible,
  writeOriginReceipt,
} from "./real_stack_ui";

async function expectEmptyCourses(page: Page, heading: string, message: string): Promise<void> {
  await expect(page.getByRole("heading", { level: 1, name: heading, exact: true })).toBeVisible();
  await expect(page.getByRole("article")).toHaveCount(0);
  await expect(page.getByText(message, { exact: true })).toBeVisible();
}

async function enterThenReenterEmptyCourses(page: Page, name: RegExp): Promise<void> {
  await chooseSeededIdentity(page, name);
  await expectEmptyCourses(
    page,
    "Your Course Instances",
    "Course access begins when you hold an active Course Membership.",
  );
  await signOutVisible(page);
  await chooseSeededIdentity(page, name);
  await expectEmptyCourses(
    page,
    "Your Course Instances",
    "Course access begins when you hold an active Course Membership.",
  );
}

test("authentication and authorization: seeded sessions and no-record boundaries", async ({
  browser,
}) => {
  test.setTimeout(120_000);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("auth_authorization");
  expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-auth_authorization$/u);
  expect(scenarioInput.baselineReads).toEqual(["seeded_accounts"]);

  const contexts: BrowserContext[] = [];
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();
  const contextOptions = { ignoreHTTPSErrors: true, viewport: { width: 1280, height: 800 } };
  try {
    const publicContext = await browser.newContext(contextOptions);
    const elenaContext = await browser.newContext(contextOptions);
    const maryContext = await browser.newContext(contextOptions);
    const morganContext = await browser.newContext(contextOptions);
    contexts.push(publicContext, elenaContext, maryContext, morganContext);
    for (const context of contexts) observeContextOrigins(context, pageOrigins, requestOrigins);
    const publicPage = await publicContext.newPage();
    const elena = await elenaContext.newPage();
    const mary = await maryContext.newPage();
    const morgan = await morganContext.newPage();

    await test.step("A public no-record course path remains outside a signed-out session", async () => {
      await publicPage.goto("/courses/C-1");
      await expect(
        publicPage.getByRole("heading", {
          level: 1,
          name: "Your session needs to be renewed",
          exact: true,
        }),
      ).toBeVisible();
      await expect(
        publicPage.getByRole("link", { name: "Open sign-in", exact: true }),
      ).toBeVisible();
    });

    await test.step("Elena enters the seeded Instructor session with no Course Instance", async () => {
      await chooseSeededIdentity(elena, /Elena Instructor/u);
      await expectEmptyCourses(
        elena,
        "Course Instances you teach",
        "No Course Instances are teaching yet.",
      );
    });

    await test.step("Mary enters and reenters an independent seeded Student session", async () => {
      await enterThenReenterEmptyCourses(mary, /Mary Student/u);
    });

    await test.step("Morgan enters the seeded Sysadmin session without ambient Course access", async () => {
      await chooseSeededIdentity(morgan, /Morgan Sysadmin/u);
      await expectEmptyCourses(
        morgan,
        "Your Course Instances",
        "Course access begins when you hold an active Course Membership.",
      );
    });

    await test.step("Mary receives Product Role denial before a Question Library read", async () => {
      await mary.goto("/library");
      await expect(
        mary.getByRole("heading", {
          level: 1,
          name: "This page is not available to this account",
          exact: true,
        }),
      ).toBeVisible();
      await expect(
        mary.getByRole("link", { name: "Return to courses", exact: true }),
      ).toBeVisible();
    });
  } finally {
    try {
      await Promise.all(contexts.map((context) => context.close()));
    } finally {
      writeOriginReceipt(pageOrigins, requestOrigins);
    }
  }
});
