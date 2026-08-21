// UI-first ordinary session and boundary proof after the owner's visible Sysadmin setup child.

import { expect, test, type BrowserContext, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { liveDemoOriginReceiptPathFromEnvironment } from "../live_demo_live_config";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
} from "./real_stack_ui";

function writeOriginReceipt(pageOrigins: Set<string>, requestOrigins: Set<string>): void {
  writeFileSync(
    liveDemoOriginReceiptPathFromEnvironment(process.env),
    JSON.stringify({
      pageOrigins: [...pageOrigins].sort(),
      requestOrigins: [...requestOrigins].sort(),
    }),
    { encoding: "ascii", flag: "wx", mode: 0o600 },
  );
}

async function enterThenReenter(page: Page, name: RegExp, course: string): Promise<void> {
  await chooseSeededIdentity(page, name);
  await selectVisibleCourse(page, course);
  await expect(page.getByRole("main")).toBeVisible();
  await signOutVisible(page);
  await chooseSeededIdentity(page, name);
  await selectVisibleCourse(page, course);
}

test("authentication and authorization: sessions, approval, and course boundaries", async ({
  browser,
}) => {
  test.setTimeout(240_000);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("auth_authorization");
  expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-auth_authorization$/u);
  expect(scenarioInput.sysadminRequirement).toBe("claimed");
  expect(scenarioInput.sysadminOwnershipProof).toBeUndefined();

  const contexts: BrowserContext[] = [];
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();
  const contextOptions = { ignoreHTTPSErrors: true, viewport: { width: 1280, height: 800 } };
  try {
    const elenaContext = await browser.newContext(contextOptions);
    const maryContext = await browser.newContext(contextOptions);
    const morganContext = await browser.newContext(contextOptions);
    const averyContext = await browser.newContext(contextOptions);
    contexts.push(elenaContext, maryContext, morganContext, averyContext);
    for (const context of contexts) observeContextOrigins(context, pageOrigins, requestOrigins);
    const elena = await elenaContext.newPage();
    const mary = await maryContext.newPage();
    const morgan = await morganContext.newPage();
    const avery = await averyContext.newPage();

    await enterThenReenter(elena, /Elena Rivera/u, "Biochemistry Base Course");
    await enterThenReenter(mary, /Mary Okafor/u, "Biochemistry Base Course");

    await chooseSeededIdentity(morgan, /Morgan/u);
    await selectVisibleCourse(morgan, "Genetics Practice Course");
    await expect(morgan.getByRole("link", { name: "Teaching operations" })).toBeVisible();
    const geneticsPath = new URL(morgan.url()).pathname;
    await morgan.getByRole("link", { name: "Teaching operations" }).click();
    await expect(morgan.getByRole("heading", { name: "Instructor approval" })).toBeVisible();
    await morgan.getByLabel("Find an account by name").fill("Avery");
    await morgan.getByRole("button", { name: "Search accounts" }).click();
    await morgan
      .getByRole("listitem")
      .filter({ hasText: "Avery Singh" })
      .getByRole("button", { name: "Approve as instructor" })
      .click();
    await morgan.getByRole("dialog").getByRole("button", { name: "Approve as instructor" }).click();
    await expect(morgan.getByText(/Avery Singh.*approved/u)).toBeVisible();

    await elena.getByRole("link", { name: "Teaching operations" }).click();
    await expect(elena.getByRole("heading", { name: "Teaching team" })).toBeVisible();
    await elena.getByLabel("Find an approved colleague").fill("Avery");
    await elena.getByRole("button", { name: "Search eligible people" }).click();
    await elena
      .getByRole("listitem")
      .filter({ hasText: "Avery Singh" })
      .getByRole("button", { name: "Select" })
      .click();
    await elena.getByRole("button", { name: "Invite selected colleague" }).click();
    await expect(
      elena.getByRole("region", { name: "Teaching team" }).getByRole("status"),
    ).toHaveText("An invitation was created for Avery Singh.");

    await chooseSeededIdentity(avery, /Avery Singh/u);
    await selectVisibleCourse(avery, "Genetics Practice Course");
    await avery.getByRole("link", { name: "Invitations", exact: true }).click();
    await expect(
      avery.getByRole("heading", { name: "Pending teaching invitations" }),
    ).toBeVisible();
    await avery.getByRole("button", { name: "Accept" }).click();
    await avery.getByRole("dialog").getByRole("button", { name: "Accept invitation" }).click();
    await expect(avery.getByRole("main").getByRole("status")).toHaveText("Invitation accepted.");
    await signOutVisible(avery);
    await chooseSeededIdentity(avery, /Avery Singh/u);
    await selectVisibleCourse(avery, "Biochemistry Base Course");
    await avery.getByRole("link", { name: "Teaching operations" }).click();
    await expect(avery.getByRole("heading", { name: "Teaching team" })).toBeVisible();

    await chooseSeededIdentity(mary, /Mary Okafor/u);
    const navigationResponses: number[] = [];
    const protectedFollowOns: string[] = [];
    mary.on("response", (response) => {
      const path = new URL(response.url()).pathname;
      if (/^\/api\/navigation\/C-[1-9][0-9]*$/u.test(path))
        navigationResponses.push(response.status());
    });
    mary.on("request", (request) => {
      const path = new URL(request.url()).pathname;
      if (/^\/api\/(?:courses|assignments|runs)\//u.test(path)) protectedFollowOns.push(path);
    });
    await mary.goto(geneticsPath);
    await expect(mary.getByText("That page is not part of this learning space")).toBeVisible();
    expect(navigationResponses).toContain(404);
    expect(protectedFollowOns).toEqual([]);

    await selectVisibleCourse(mary, "Biochemistry Base Course");
    const basePath = new URL(mary.url()).pathname;
    await mary.goto(`/instructor${basePath}/teaching-operations`);
    await expect(mary.getByRole("alert")).toContainText("You do not manage this course.");
  } finally {
    try {
      await Promise.all(contexts.map((context) => context.close()));
    } finally {
      writeOriginReceipt(pageOrigins, requestOrigins);
    }
  }
});
