// UI-first independent seeded-role sessions and authorization-boundary proof.
//
// Selector contract:
// - src/pages/sign_in_page.tsx owns seeded-demo entry and course-choice headings.
// - src/pages/teaching_operations_page.tsx owns teaching-team controls.
// - src/pages/account_pending_invitations_page.tsx owns current invitation acceptance.
// - src/pages/course_list_page.tsx:330 owns the course heading and return-to-courses controls.

import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { BIOCHEMISTRY_COURSE_TITLE } from "./helper_course_titles";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
  writeOriginReceipt,
} from "./real_stack_ui";

async function enterThenReenter(page: Page, name: RegExp, course: string): Promise<void> {
  await chooseSeededIdentity(page, name);
  await selectVisibleCourse(page, course);
  await expect(page.getByRole("main")).toBeVisible();
  await signOutVisible(page);
  await chooseSeededIdentity(page, name);
  await selectVisibleCourse(page, course);
}

test("authentication and authorization: sessions and course boundaries", async ({ browser }) => {
  test.setTimeout(240_000);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("auth_authorization");
  expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-auth_authorization$/u);

  const contexts: BrowserContext[] = [];
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();
  const contextOptions = { ignoreHTTPSErrors: true, viewport: { width: 1280, height: 800 } };
  let basePath = "";
  let geneticsPath = "";
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
    await test.step("Elena enters her seeded Instructor session", async () => {
      await chooseSeededIdentity(elena, /Elena Rivera/u);
      await selectVisibleCourse(elena, BIOCHEMISTRY_COURSE_TITLE);
      await expect(elena.getByRole("link", { name: "Teaching operations" })).toBeVisible();
    });
    await test.step("Mary enters and reenters her installed Biochemistry course session", async () => {
      await enterThenReenter(mary, /Mary Okafor/u, BIOCHEMISTRY_COURSE_TITLE);
      await expect(mary.getByRole("heading", { name: BIOCHEMISTRY_COURSE_TITLE })).toBeVisible();
      basePath = new URL(mary.url()).pathname;
    });

    await test.step("Morgan accesses the Sysadmin-visible Genetics course", async () => {
      await chooseSeededIdentity(morgan, /Morgan/u);
      await selectVisibleCourse(morgan, "Genetics Practice Course");
      await expect(morgan.getByRole("link", { name: "Teaching operations" })).toBeVisible();
      geneticsPath = new URL(morgan.url()).pathname;
    });

    await test.step("Elena invites approved Avery through the teaching UI", async () => {
      await elena.getByRole("link", { name: "Teaching operations" }).click();
      await expect(elena.getByRole("heading", { name: "Teaching team" })).toBeVisible();
      const teachingTeam = elena.getByRole("region", { name: "Teaching team" });
      await teachingTeam.getByLabel("Find an approved colleague").fill("Avery");
      await teachingTeam.getByRole("button", { name: "Search eligible people" }).click();
      await teachingTeam
        .getByRole("listitem")
        .filter({ hasText: "Avery Singh" })
        .getByRole("button", { name: "Select" })
        .click();
      await teachingTeam.getByRole("button", { name: "Invite selected colleague" }).click();
      await expect(teachingTeam.getByRole("status")).toHaveText(
        "An invitation was created for Avery Singh.",
      );

    });

    await test.step("Avery accepts and observes a fresh authorized teaching session", async () => {
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
      await selectVisibleCourse(avery, BIOCHEMISTRY_COURSE_TITLE);
      await avery.getByRole("link", { name: "Teaching operations" }).click();
      await expect(avery.getByRole("heading", { name: "Teaching team" })).toBeVisible();
    });

    await test.step("Mary observes cross-course and role denial", async () => {
      const navigationResponses: number[] = [];
      const protectedFollowOns: string[] = [];
      mary.on("response", (response) => {
        const path = new URL(response.url()).pathname;
        if (/^\/api\/navigation\/C-[1-9][0-9]*$/u.test(path))
          navigationResponses.push(response.status());
      });
      mary.on("request", (request) => {
        const path = new URL(request.url()).pathname;
        if (/^\/api\/(?:courses|assignments|assignment-attempts)\//u.test(path)) {
          protectedFollowOns.push(path);
        }
      });
      await mary.goto(geneticsPath);
      await expect(mary.getByRole("alert")).toContainText("The learning space is still available");
      expect(navigationResponses).toContain(404);
      expect(protectedFollowOns).toEqual([]);

      await mary.getByRole("link", { name: "Return to courses" }).click();
      await expect(mary.getByRole("heading", { name: BIOCHEMISTRY_COURSE_TITLE })).toBeVisible();
      await mary.goto(`/instructor${basePath}/teaching-operations`);
      await expect(mary.getByRole("alert")).toContainText(
        "This page is available to instructors only",
      );
    });
  } finally {
    try {
      await Promise.all(contexts.map((context) => context.close()));
    } finally {
      writeOriginReceipt(pageOrigins, requestOrigins);
    }
  }
});
