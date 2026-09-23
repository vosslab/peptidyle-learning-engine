// Compact full-stack structural parity for WP-E6, WP-E2, and the production Library browse route.
// Selector contract: PageFrame mode/title (src/components/page_frame.tsx); Breadcrumb rail
// (src/application_shell.tsx); Student Coursework regions (src/pages/student_course_landing_page.tsx);
// Library presentations/window and record regions (src/pages/library_browse_rows.tsx).
import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import {
  FAST_UI_QUESTION_LIBRARY_IDS,
  fastUiQuestionLibraryPage,
} from "../../support/fast_ui_question_library_fixture";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  enterStudentCourse,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
  writeOriginReceipt,
} from "./real_stack_ui";

const laptop = { width: 1280, height: 800 };
const phone = { width: 393, height: 852 };
const courseTitle = "Biochemistry 301: Proteins and Peptides";

type RailGeometry = {
  readonly left: number;
  readonly right: number;
  readonly center: number;
};

async function railGeometry(page: Page, selector: string): Promise<RailGeometry> {
  return page.locator(selector).evaluate((element) => {
    const { left, right } = element.getBoundingClientRect();
    return { left, right, center: (left + right) / 2 };
  });
}

async function expectBreadcrumbAndMainRails(page: Page): Promise<RailGeometry> {
  const main = await railGeometry(page, "#main-content");
  const breadcrumbOrigin = await page
    .locator(".ple-shell__breadcrumb-prelude")
    .evaluate((element) => {
      const style = window.getComputedStyle(element);
      return element.getBoundingClientRect().left + Number.parseFloat(style.paddingLeft);
    });
  expect(breadcrumbOrigin).toBe(main.left);
  return main;
}

async function expectPageFrameGeometry(page: Page, layout: "reading" | "fullWidth"): Promise<void> {
  const frame = page.locator(`.page-frame[data-content-layout="${layout}"]`);
  await expect(frame).toHaveCount(1);
  const mainRail = await expectBreadcrumbAndMainRails(page);
  const [frameRail, titleLeft] = await Promise.all([
    railGeometry(page, ".page-frame"),
    page.locator(".page-frame__title").evaluate((title) => title.getBoundingClientRect().left),
  ]);
  expect(titleLeft).toBe(frameRail.left);
  if (layout === "reading") {
    expect(frameRail.center).toBe(mainRail.center);
  } else {
    expect(frameRail.left).toBe(mainRail.left);
    expect(frameRail.right).toBe(mainRail.right);
  }
}

test.describe("UI backbone compact parity on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("PageFrame, Student Coursework, and Library retain their named structures", async ({
    browser,
  }) => {
    test.setTimeout(120_000);
    requireScenarioInput(configuredLiveDemoInputs);

    const contexts: BrowserContext[] = [];
    const pageOrigins = new Set<string>();
    const requestOrigins = new Set<string>();
    try {
      const studentContext = await browser.newContext({ viewport: laptop });
      contexts.push(studentContext);
      observeContextOrigins(studentContext, pageOrigins, requestOrigins);
      const student = await studentContext.newPage();
      configureContextAndPage(studentContext, student, 30_000);

      await test.step("WP-E6 reading StudentCourseLandingPage uses its centered PageFrame rail", async () => {
        await chooseSeededIdentity(student, /Mary Okafor/u);
        await enterStudentCourse(student, courseTitle);
        await expect(
          student.getByRole("heading", { level: 1, name: courseTitle, exact: true }),
        ).toBeVisible();
        await expectPageFrameGeometry(student, "reading");
      });

      await test.step("Student Coursework keeps required regions and its action adjacent on laptop and phone", async () => {
        const courseworkTab = student
          .getByRole("navigation", { name: "Ribbon tabs", exact: true })
          .getByRole("link", { name: "Coursework", exact: true });
        await expect(courseworkTab).toBeVisible();
        const rows = student.locator(".student-coursework .record-list__row");
        await expect(rows.first()).toBeVisible();

        const assertCourseworkRows = async (): Promise<void> => {
          const structure = await rows.evaluateAll((elements) =>
            elements.map((row) => {
              const identity = row.querySelector<HTMLElement>(
                '[data-record-region-id="assessment"]',
              );
              const action = row.querySelector<HTMLElement>('[data-record-region-id="action"]');
              if (identity === null || action === null) return null;
              return {
                identityPriority: identity.dataset.recordListPriority,
                actionPriority: action.dataset.recordListPriority,
                identityVisible: identity.getClientRects().length > 0,
                actionVisible: action.getClientRects().length > 0,
                actionHasLink: action.querySelector("a") !== null,
              };
            }),
          );
          expect(structure.length).toBeGreaterThan(0);
          for (const row of structure) {
            expect(row).not.toBeNull();
            expect(row?.identityPriority).toBe("required");
            expect(row?.actionPriority).toBe("required");
            expect(row?.identityVisible).toBe(true);
            expect(row?.actionVisible).toBe(true);
            expect(row?.actionHasLink).toBe(true);
          }
        };

        await assertCourseworkRows();
        await student.setViewportSize(phone);
        await assertCourseworkRows();
      });

      const instructorContext = await browser.newContext({ viewport: laptop });
      contexts.push(instructorContext);
      observeContextOrigins(instructorContext, pageOrigins, requestOrigins);
      const instructor = await instructorContext.newPage();
      configureContextAndPage(instructorContext, instructor, 30_000);
      const interceptedSearchRequestUrls: string[] = [];
      const instructorQuestionRequestUrls: string[] = [];
      instructor.on("request", (request) => {
        const requestUrl = request.url();
        if (new URL(requestUrl).pathname.includes("/api/questions")) {
          instructorQuestionRequestUrls.push(requestUrl);
        }
      });
      await instructorContext.route("**/api/questions/search**", (route) => {
        interceptedSearchRequestUrls.push(route.request().url());
        return route.fulfill({ json: fastUiQuestionLibraryPage() });
      });

      await test.step("WP-E2 fullWidth Gradebook aligns its PageFrame with the main and breadcrumb rails", async () => {
        await chooseSeededIdentity(instructor, /Elena Rivera/u);
        await selectVisibleCourse(instructor, courseTitle);
        await instructor
          .getByRole("navigation", { name: "Ribbon tasks", exact: true })
          .getByRole("link", { name: "Gradebook", exact: true })
          .click();
        await expect(
          instructor.getByRole("heading", { level: 1, name: "Gradebook", exact: true }),
        ).toBeVisible();
        await expectPageFrameGeometry(instructor, "fullWidth");
      });

      await test.step("Library browse presents seeded results with production regions and windowing", async () => {
        await instructor.goto("/library/browse?tag=protein");
        await expect(
          instructor.getByRole("heading", { name: "Browse Question Library", exact: true }),
        ).toBeVisible();
        const listWindow = instructor.getByRole("region", {
          name: "Published questions",
          exact: true,
        });
        expect(
          interceptedSearchRequestUrls,
          `the browser should reach the production Question Library search endpoint; observed instructor /api/questions requests: ${JSON.stringify(instructorQuestionRequestUrls)}`,
        ).not.toHaveLength(0);
        expect(
          interceptedSearchRequestUrls.some((requestUrl) =>
            new URL(requestUrl).searchParams.getAll("tags").includes("protein"),
          ),
        ).toBe(true);
        await expect(listWindow.locator(".record-list__row").first()).toBeVisible();
        await expect(
          instructor.getByRole("group", { name: "Question result presentation", exact: true }),
        ).toBeVisible();
        await expect(instructor.getByRole("button", { name: "Scan", exact: true })).toHaveAttribute(
          "aria-pressed",
          "true",
        );
        await expect(
          instructor.getByRole("button", { name: "Preview", exact: true }),
        ).toBeVisible();
        await expect(listWindow.locator(".library-browse-record-list--scan")).toHaveCount(1);
        const rows = listWindow.locator(".record-list__row");
        const rowStructure = await rows.evaluateAll((elements) =>
          elements.map((row) => ({
            id: row.getAttribute("data-record-id"),
            title: row.querySelector("h2")?.textContent?.trim(),
            regions: [...row.querySelectorAll<HTMLElement>("[data-record-region-id]")].map(
              (region) => ({
                id: region.dataset.recordRegionId,
                priority: region.dataset.recordListPriority,
              }),
            ),
            identityVisible:
              (row.querySelector('[data-record-region-id="question"]')?.getClientRects().length ??
                0) > 0,
            actionVisible:
              (row.querySelector('[data-record-region-id="actions"]')?.getClientRects().length ??
                0) > 0,
          })),
        );
        expect(rowStructure.length).toBeGreaterThan(0);
        expect(rowStructure.length).toBeLessThan(FAST_UI_QUESTION_LIBRARY_IDS.length);
        expect(new Set(rowStructure.map((row) => row.id)).size).toBe(rowStructure.length);
        expect(rowStructure.map((row) => row.id)).toEqual(
          FAST_UI_QUESTION_LIBRARY_IDS.slice(0, rowStructure.length),
        );
        for (const row of rowStructure) {
          expect(row.title).toBeTruthy();
          expect(row.regions).toEqual([
            { id: "question", priority: "required" },
            { id: "classification", priority: "high" },
            { id: "authors", priority: "medium" },
            { id: "actions", priority: "required" },
          ]);
          expect(row.identityVisible).toBe(true);
          expect(row.actionVisible).toBe(true);
        }
      });
    } finally {
      try {
        await Promise.all(contexts.map((context) => context.close()));
      } finally {
        writeOriginReceipt(pageOrigins, requestOrigins);
      }
    }
  });
});
