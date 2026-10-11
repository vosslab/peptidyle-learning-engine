// Real-stack proof for deliberate, audited Sysadmin Student-data access.
// Selector contract: Instructor roster rows are rendered by course_roster_page.tsx; the
// Sysadmin confirmation and receipt are rendered by sysadmin_course_inspection_page.tsx.

import { expect, test, type BrowserContext } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { enterSysadmin } from "../screenshot_corpus/visible_workflows";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  selectVisibleCourse,
  writeContextOriginReceipt,
} from "./real_stack_ui";

const seededCourseTitle = "Biochemistry 301: Proteins and Peptides";
const contextOptions = { viewport: { width: 1280, height: 800 } };

test.describe("Sysadmin Student-data confirmation on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("Morgan confirms one visible Student roster record and receives its audit receipt", async ({
    browser,
  }) => {
    test.setTimeout(120_000);
    const contexts: BrowserContext[] = [];
    const origins = {
      instructor: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      sysadmin: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    };
    try {
      const instructorContext = await browser.newContext(contextOptions);
      const sysadminContext = await browser.newContext(contextOptions);
      contexts.push(instructorContext, sysadminContext);
      observeContextOrigins(
        instructorContext,
        origins.instructor.pageOrigins,
        origins.instructor.requestOrigins,
      );
      observeContextOrigins(
        sysadminContext,
        origins.sysadmin.pageOrigins,
        origins.sysadmin.requestOrigins,
      );
      const instructor = await instructorContext.newPage();
      const sysadmin = await sysadminContext.newPage();
      const protectedStudentDataRequests: string[] = [];
      sysadminContext.on("request", (request) => {
        const pathname = new URL(request.url()).pathname;
        if (pathname.endsWith("/student-data"))
          protectedStudentDataRequests.push(`${request.method()} ${pathname}`);
      });
      let instructorCoursePage = instructor;
      let sysadminCoursePage = sysadmin;

      await test.step("Elena reads Mary's roster ID from the visible seeded Course roster", async () => {
        await chooseSeededIdentity(instructor, /Elena Rivera/u);
        instructorCoursePage = await selectVisibleCourse(instructor, seededCourseTitle);
        await instructorCoursePage
          .getByRole("link", { name: "Open Students", exact: true })
          .click();
        await expect(
          instructorCoursePage.getByRole("heading", { level: 1, name: "Students", exact: true }),
        ).toBeVisible();
      });

      const maryRow = instructorCoursePage
        .getByRole("row")
        .filter({ has: instructorCoursePage.getByText("BIO301-MARY", { exact: true }) });
      await expect(maryRow).toHaveCount(1);
      const rosterId = (await maryRow.locator("small").textContent())?.trim();
      expect(rosterId).toMatch(/^[A-Za-z0-9._-]{1,64}$/u);

      await test.step("Morgan's unchecked and cancelled lookups disclose no Student data", async () => {
        await sysadmin.goto("/sign-in");
        await enterSysadmin(sysadmin);
        await sysadmin.getByRole("link", { name: "Find Courses", exact: true }).click();
        await expect(sysadmin.getByRole("heading", { name: "Courses", exact: true })).toBeVisible();
        const openedCoursePage = sysadmin.waitForEvent("popup");
        await sysadmin
          .getByRole("listitem")
          .filter({ has: sysadmin.getByRole("heading", { name: seededCourseTitle, exact: true }) })
          .getByRole("link", { name: "Inspect", exact: true })
          .click();
        sysadminCoursePage = await openedCoursePage;
        await expect(
          sysadminCoursePage.getByRole("heading", {
            name: "Access one Student roster record",
            exact: true,
          }),
        ).toBeVisible();

        const rosterInput = sysadminCoursePage.getByLabel("Student roster ID", { exact: true });
        const confirmation = sysadminCoursePage.getByLabel(
          "I confirm this access is needed for administrative work.",
          { exact: true },
        );
        const accessButton = sysadminCoursePage.getByRole("button", {
          name: "Access Student record",
          exact: true,
        });
        await rosterInput.fill(rosterId!);
        await expect(confirmation).not.toBeChecked();
        await expect(accessButton).toBeDisabled();
        await expect.poll(() => protectedStudentDataRequests).toEqual([]);
        await expect(
          sysadminCoursePage.getByRole("heading", { name: "Student roster record accessed" }),
        ).toHaveCount(0);

        await confirmation.check();
        await expect(accessButton).toBeEnabled();
        await sysadminCoursePage.getByRole("button", { name: "Cancel", exact: true }).click();
        await expect(rosterInput).toHaveValue("");
        await expect(confirmation).not.toBeChecked();
        await expect(accessButton).toBeDisabled();
        await expect.poll(() => protectedStudentDataRequests).toEqual([]);
        await expect(
          sysadminCoursePage.getByRole("heading", { name: "Student roster record accessed" }),
        ).toHaveCount(0);
      });

      await test.step("A fresh explicit confirmation shows its audit receipt", async () => {
        await sysadminCoursePage.getByLabel("Student roster ID", { exact: true }).fill(rosterId!);
        await sysadminCoursePage
          .getByLabel("I confirm this access is needed for administrative work.", { exact: true })
          .check();
        await sysadminCoursePage
          .getByRole("button", { name: "Access Student record", exact: true })
          .click();
        const receipt = sysadminCoursePage.getByRole("status").filter({
          has: sysadminCoursePage.getByRole("heading", {
            name: "Student roster record accessed",
            exact: true,
          }),
        });
        await expect(receipt).toBeVisible();
        await expect(receipt).toContainText(rosterId!);
        await expect(receipt).toContainText("Active Student");
        await expect(receipt.getByText("Audit event", { exact: true })).toBeVisible();
        await expect(receipt).not.toContainText("Mary Okafor");
      });
    } finally {
      try {
        await Promise.all(contexts.map((context) => context.close()));
      } finally {
        writeContextOriginReceipt(origins);
      }
    }
  });
});
