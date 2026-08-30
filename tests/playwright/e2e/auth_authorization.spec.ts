// UI-first independent seeded-role sessions and authorization-boundary proof.
//
// Selector contract:
// - src/pages/sign_in_page.tsx owns seeded-demo entry and course-choice headings.
// - src/pages/teaching_operations_page.tsx:95 and
//   src/pages/teaching_operations/course_groups_panel.tsx:340 own teaching-team, approval, and
//   group controls.
// - src/pages/account_pending_invitations_page.tsx owns current invitation acceptance.
// - src/pages/course_list_page.tsx:330 owns the course heading and return-to-courses controls.

import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { BIOCHEMISTRY_COURSE_TITLE } from "./helper_course_titles";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  requireScenarioInput,
  restoreViewportOrigin,
  selectVisibleCourse,
  signOutVisible,
  writeOriginReceipt,
} from "./real_stack_ui";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";

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

    await test.step("Morgan accesses Genetics and approves Avery", async () => {
      await chooseSeededIdentity(morgan, /Morgan/u);
      await selectVisibleCourse(morgan, "Genetics Practice Course");
      await expect(morgan.getByRole("link", { name: "Teaching operations" })).toBeVisible();
      geneticsPath = new URL(morgan.url()).pathname;
      await morgan.getByRole("link", { name: "Teaching operations" }).click();
      await expect(morgan.getByRole("heading", { name: "Instructor approval" })).toBeVisible();
      await morgan.getByLabel("Find an account by name").fill("Avery");
      await morgan.getByRole("button", { name: "Search accounts" }).click();
      await morgan
        .getByRole("listitem")
        .filter({ hasText: "Avery Singh" })
        .getByRole("button", { name: "Approve as instructor" })
        .click();
      await morgan
        .getByRole("dialog")
        .getByRole("button", { name: "Approve as instructor" })
        .click();
      await expect(morgan.getByText(/Avery Singh.*approved/u)).toBeVisible();
    });

    await test.step("Elena creates a course group and invites approved Avery through the teaching UI", async () => {
      await elena.getByRole("link", { name: "Teaching operations" }).click();
      await expect(elena.getByRole("heading", { name: "Teaching team" })).toBeVisible();
      const groupTitle = "Section A learners";
      const groups = elena.getByRole("region", { name: "Groups and sections" });
      await expect(groups).toBeVisible();
      await expect(groups.getByLabel("Group name")).toBeVisible();
      await groups.getByLabel("Group name").fill(groupTitle);
      await groups.getByRole("button", { name: "Create group" }).click();
      await expect(groups.getByRole("button", { name: groupTitle, exact: true })).toBeVisible();
      await restoreViewportOrigin(elena);
      await captureRealStackScreenshot(elena, scenarioInput, "auth_teaching_operations_groups");

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
      await teachingTeam.scrollIntoViewIfNeeded();
      await captureRealStackScreenshot(elena, scenarioInput, "auth_teaching_team_invited");

      const retention = elena.getByRole("heading", { name: "Record retention" }).locator("..");
      await expect(retention).toBeVisible();
      await retention.scrollIntoViewIfNeeded();
      await captureRealStackScreenshot(elena, scenarioInput, "auth_teaching_operations_retention");
    });

    await test.step("Avery accepts and observes a fresh authorized teaching session", async () => {
      await chooseSeededIdentity(avery, /Avery Singh/u);
      await selectVisibleCourse(avery, "Genetics Practice Course");
      await avery.getByRole("link", { name: "Invitations", exact: true }).click();
      await expect(
        avery.getByRole("heading", { name: "Pending teaching invitations" }),
      ).toBeVisible();
      await restoreViewportOrigin(avery);
      await captureRealStackScreenshot(avery, scenarioInput, "auth_pending_teaching_invitation");
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
        if (/^\/api\/(?:courses|assignments|runs)\//u.test(path)) protectedFollowOns.push(path);
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
      await mary.setViewportSize({ width: 1280, height: 800 });
      await restoreViewportOrigin(mary);
      await captureRealStackScreenshot(
        mary,
        scenarioInput,
        "auth_student_instructor_denial_laptop",
      );
      await mary.setViewportSize({ width: 800, height: 1280 });
      await restoreViewportOrigin(mary);
      await captureRealStackScreenshot(
        mary,
        scenarioInput,
        "auth_student_instructor_denial_tablet",
      );
      await mary.setViewportSize({ width: 393, height: 852 });
      await restoreViewportOrigin(mary);
      await captureRealStackScreenshot(
        mary,
        scenarioInput,
        "auth_student_instructor_denial_iphone_pro",
      );
      await mary.setViewportSize({ width: 800, height: 800 });
      await restoreViewportOrigin(mary);
      await captureRealStackScreenshot(
        mary,
        scenarioInput,
        "auth_student_instructor_denial_square",
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
