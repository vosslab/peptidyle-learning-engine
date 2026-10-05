// Compact full-stack structural parity for WP-E6, WP-E2, and the production Library browse route.
// Selector contract: PageFrame mode/title (src/components/page_frame.tsx); Breadcrumb rail
// (src/application_shell.tsx); Student Coursework title and action
// (src/pages/student_course_landing_page.tsx); Library presentations/window and record copy
// (the shared SearchResults component).
import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import type { QuestionSearchPage } from "../../../generated/api/QuestionSearchPage";
import {
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSIONS,
} from "../../../src/api/decoders/bloom_classification";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  courseChoice,
  observeContextOrigins,
  requireScenarioInput,
  writeOriginReceipt,
} from "./real_stack_ui";

const laptop = { width: 1280, height: 800 };
const phone = { width: 393, height: 852 };
const courseTitle = "Biochemistry 301: Proteins and Peptides";
const parityQuestionId = "7K3M-79QP";
const parityPoolId = "3S8B-24DZ";

function parityQuestionLibraryPage(): QuestionSearchPage {
  return {
    items: [
      {
        kind: "question",
        ownerAccountId: "U00000009",
        question: {
          summary: {
            questionId: parityQuestionId,
            publishedQuestionRevisionTuple: {
              publishedQuestionId: parityQuestionId,
              revisionNumber: 1,
            },
            backend: "ple",
            questionFormat: "pleQuestionJson",
            questionType: "multipleChoice",
            capabilities: ["clientRendering"],
            metadata: {
              questionTitle: "Peptide bond",
              questionDescription: "Identify the atoms that form a peptide bond.",
              tags: ["protein"],
              questionLicense: "CC-BY-4.0",
              questionCitation: null,
              language: "en",
            },
            authorship: { authors: [{ displayName: "Ada Instructor", accountId: null }] },
            availability: { availability: "available" },
            publishedAt: 1_789_920_000_000,
            bloom: null,
          },
          disciplineName: "Biochemistry",
          disciplineIsRetired: false,
          evidence: { state: "unavailable" },
        },
      },
      {
        kind: "pool",
        pool: {
          questionPoolId: parityPoolId,
          ownerAccountId: "U00000009",
          questionType: "multipleChoice",
          backend: "ple",
          license: "CC-BY-4.0",
          questionPoolEditNumber: 1,
          questionPoolMetadataEditNumber: 1,
          metadata: {
            title: "Protein structure practice Pool",
            description: "Practice Questions about protein structure.",
            disciplineUuid: "00000000-0000-0000-0000-000000000001",
            disciplineName: "Biochemistry",
            disciplineIsRetired: false,
            subjectUuid: "00000000-0000-0000-0000-000000000002",
            topicUuid: null,
            subtopicUuid: null,
            tags: ["protein"],
          },
          memberCount: 1,
          bloom: null,
        },
      },
    ],
    nextCursor: null,
    facets: {
      categories: { questionsInNoPool: 1, questionsInPool: 0, pools: 1 },
      authorNames: [],
      authorNamesTruncated: false,
      backends: [],
      tags: [{ tag: "protein", count: 2 }],
      tagsTruncated: false,
      subjects: [],
      subjectsTruncated: false,
      topics: [],
      topicsTruncated: false,
      questionTypes: [],
      capabilities: [],
      questionLicenses: [],
      usedInMyCourses: { used: 0 },
      bloomCognitiveProcesses: BLOOM_COGNITIVE_PROCESSES.map((cognitiveProcess) => ({
        cognitiveProcess,
        count: 0,
      })),
      bloomKnowledgeDimensions: BLOOM_KNOWLEDGE_DIMENSIONS.map((knowledgeDimension) => ({
        knowledgeDimension,
        count: 0,
      })),
    },
  };
}

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
      let student = await studentContext.newPage();
      configureContextAndPage(studentContext, student, 30_000);

      await test.step("WP-E6 reading StudentCourseLandingPage uses its centered PageFrame rail", async () => {
        await chooseSeededIdentity(student, /Mary Okafor/u);
        const openedCourse = student.waitForEvent("popup");
        await courseChoice(student, courseTitle)
          .getByRole("link", { name: "Open Course", exact: true })
          .click();
        student = await openedCourse;
        configureContextAndPage(studentContext, student, 30_000);
        await student.waitForLoadState("domcontentloaded");
        await expect(
          student.getByRole("heading", { level: 1, name: courseTitle, exact: true }),
        ).toBeVisible();
        await expectPageFrameGeometry(student, "reading");
      });

      await test.step("Student Coursework keeps each Assessment title and action visible on laptop and phone", async () => {
        const courseworkTab = student
          .getByRole("navigation", { name: "Ribbon tabs", exact: true })
          .getByRole("link", { name: "Coursework", exact: true });
        await expect(courseworkTab).toBeVisible();
        const rows = student.locator(".student-coursework .record-list__row");
        await expect(rows.first()).toBeVisible();

        const assertCourseworkRows = async (): Promise<void> => {
          const structure = await rows.evaluateAll((elements) =>
            elements.map((row) => {
              const title = row.querySelector<HTMLElement>(".record-list__title");
              const action = row.querySelector<HTMLAnchorElement>(".record-list__actions a");
              if (title === null || action === null) return null;
              return {
                title: title.textContent?.trim() ?? "",
                titleVisible: title.getClientRects().length > 0,
                actionLabel: action.textContent?.trim() ?? "",
                actionVisible: action.getClientRects().length > 0,
              };
            }),
          );
          expect(structure.length).toBeGreaterThan(0);
          for (const row of structure) {
            expect(row).not.toBeNull();
            expect(row?.title.length).toBeGreaterThan(0);
            expect(row?.titleVisible).toBe(true);
            expect(row?.actionLabel.length).toBeGreaterThan(0);
            expect(row?.actionVisible).toBe(true);
          }
        };

        await assertCourseworkRows();
        await student.setViewportSize(phone);
        await assertCourseworkRows();
      });

      const instructorContext = await browser.newContext({ viewport: laptop });
      contexts.push(instructorContext);
      observeContextOrigins(instructorContext, pageOrigins, requestOrigins);
      let instructor = await instructorContext.newPage();
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
        return route.fulfill({ json: parityQuestionLibraryPage() });
      });

      await test.step("WP-E2 fullWidth Gradebook aligns its PageFrame with the main and breadcrumb rails", async () => {
        await chooseSeededIdentity(instructor, /Elena Rivera/u);
        const openedCourse = instructor.waitForEvent("popup");
        await courseChoice(instructor, courseTitle)
          .getByRole("link", { name: "Open Course", exact: true })
          .click();
        instructor = await openedCourse;
        configureContextAndPage(instructorContext, instructor, 30_000);
        await instructor.waitForLoadState("domcontentloaded");
        await expect(
          instructor.getByRole("heading", { level: 1, name: courseTitle, exact: true }),
        ).toBeVisible();
        await instructor
          .getByRole("navigation", { name: "Course actions", exact: true })
          .getByRole("link", { name: "Gradebook", exact: true })
          .click();
        await expect(
          instructor.getByRole("heading", { level: 1, name: "Gradebook", exact: true }),
        ).toBeVisible();
        await expectPageFrameGeometry(instructor, "fullWidth");
      });

      await test.step("Library browse presents Question and Pool results in production regions", async () => {
        await instructor.goto("/library/browse?tag=protein");
        await expect(
          instructor.getByRole("heading", { name: "Browse Question Library", exact: true }),
        ).toBeVisible();
        const listWindow = instructor.getByRole("region", {
          name: "Question Library results",
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
        const list = listWindow.getByRole("list", {
          name: "Question Library results",
          exact: true,
        });
        await expect(list).toHaveCount(1);
        const rows = list.locator(".record-list__row");
        await expect(rows).toHaveCount(2);
        const rowStructure = await rows.evaluateAll((elements) =>
          elements.map((row) => ({
            id: row.getAttribute("data-record-id"),
            title: row.querySelector(".record-list__title")?.textContent?.trim() ?? "",
            titleVisible:
              (row.querySelector(".record-list__title")?.getClientRects().length ?? 0) > 0,
            hasCopy:
              ((row.querySelector(".record-list__description")?.textContent?.trim().length ?? 0) >
                0 &&
                (row.querySelector(".record-list__description")?.getClientRects().length ?? 0) >
                  0) ||
              ((row.querySelector(".record-list__facts")?.textContent?.trim().length ?? 0) > 0 &&
                (row.querySelector(".record-list__facts")?.getClientRects().length ?? 0) > 0),
            openName: row.querySelector(".record-list__actions a")?.textContent?.trim() ?? "",
            openVisible:
              (row.querySelector(".record-list__actions a")?.getClientRects().length ?? 0) > 0,
          })),
        );
        expect(rowStructure.map((row) => row.id)).toEqual([parityQuestionId, parityPoolId]);
        for (const row of rowStructure) {
          expect(row.title.length).toBeGreaterThan(0);
          expect(row.titleVisible).toBe(true);
          expect(row.hasCopy).toBe(true);
          expect(row.openName).toBe(row.id === parityPoolId ? "Open Question Pool" : "Open");
          expect(row.openVisible).toBe(true);
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
