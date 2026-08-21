// Browser coverage for the instructor-owned course-grade editor and student route boundary.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page, type Route } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import {
  COURSE_ID,
  COURSE_REFERENCE,
  appearanceHeaders,
  json,
  session,
} from "./course_appearance_fixtures";

const GRADE_SETTINGS_PATH = `/instructor/courses/${COURSE_REFERENCE}/grade-settings`;
const ASSIGNMENT_ONE = publishedProblemFixture.assignment.id;
const ASSIGNMENT_TWO = "0198e000-0000-7000-8000-000000000032";
const REMOTE_CATEGORY = "0198e000-0000-7000-8000-000000000041";

type SchemeView = {
  scheme: {
    mode: "totalPoints" | "weightedCategories";
    rounding: "fourDecimalPlacesHalfAwayFromZero";
    categories: Array<{
      id: string;
      title: string;
      position: number;
      weightBasisPoints: number;
      dropLowest: number;
    }>;
    letterBands: Array<{ label: string; minimumBasisPoints: number }>;
  };
  assignments: Array<{
    assignment: string;
    title: string;
    included: boolean;
    category: string | null;
    position: number | null;
  }>;
};

function totalPointsView(): SchemeView {
  return {
    scheme: {
      mode: "totalPoints",
      rounding: "fourDecimalPlacesHalfAwayFromZero",
      categories: [],
      letterBands: [{ label: "C", minimumBasisPoints: 7_000 }],
    },
    assignments: [
      {
        assignment: ASSIGNMENT_ONE,
        title: "Enzyme kinetics lab",
        included: true,
        category: null,
        position: null,
      },
      {
        assignment: ASSIGNMENT_TWO,
        title: "Midterm examination",
        included: true,
        category: null,
        position: null,
      },
    ],
  };
}

function weightedView(title: string): SchemeView {
  return {
    scheme: {
      mode: "weightedCategories",
      rounding: "fourDecimalPlacesHalfAwayFromZero",
      categories: [
        {
          id: REMOTE_CATEGORY,
          title,
          position: 0,
          weightBasisPoints: 10_000,
          dropLowest: 0,
        },
      ],
      letterBands: [{ label: "C", minimumBasisPoints: 7_000 }],
    },
    assignments: totalPointsView().assignments.map((assignment, position) => ({
      ...assignment,
      category: REMOTE_CATEGORY,
      position,
    })),
  };
}

function totals(mode: SchemeView["scheme"]["mode"]): unknown {
  return {
    mode,
    rounding: "fourDecimalPlacesHalfAwayFromZero",
    rows: [
      {
        rosterId: ".student-01",
        displayName: "Avery Student",
        outcome:
          mode === "totalPoints"
            ? {
                status: "available",
                score: 0.75,
                letter: "C",
                droppedAssignmentIds: [],
                totalEarned: 30,
                totalPossible: 40,
              }
            : {
                status: "available",
                score: 0.75,
                letter: "C",
                droppedAssignmentIds: [ASSIGNMENT_TWO],
                totalEarned: null,
                totalPossible: null,
              },
      },
      {
        rosterId: "student-02",
        displayName: "Recalculating Student",
        outcome: { status: "unavailable", reason: "recalculating" },
      },
    ],
  };
}

async function openGradeSettings(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.goto(GRADE_SETTINGS_PATH);
}

async function commonInstructorRoute(route: Route): Promise<boolean> {
  const path = new URL(route.request().url()).pathname;
  if (path === "/api/auth/session") {
    await json(route, session(["instructor"]));
    return true;
  }
  if (path === `/api/navigation/${COURSE_REFERENCE}`) {
    await json(route, { kind: "course", courseId: COURSE_ID });
    return true;
  }
  if (path === `/api/courses/${COURSE_ID}`) {
    await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    return true;
  }
  if (path === `/api/courses/${COURSE_ID}/appearance`) {
    await json(route, { theme: "grass", revision: "1", banner: null }, 200, appearanceHeaders("1"));
    return true;
  }
  return false;
}

test("instructor configures weighted grades and downloads the server-owned export", async ({
  page,
}) => {
  let revision = 1;
  let current = totalPointsView();
  let savedBody: unknown;
  await page.route("**/api/**", async (route) => {
    if (await commonInstructorRoute(route)) return;
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === `/api/courses/${COURSE_ID}/grade-scheme`) {
      if (request.method() === "GET") {
        return await json(route, current, 200, {
          "cache-control": "no-store",
          etag: `"${revision}"`,
        });
      }
      expect(request.method()).toBe("PUT");
      expect(request.headers()["if-match"]).toBe(`"${revision}"`);
      savedBody = request.postDataJSON();
      const update = savedBody as Omit<SchemeView, "assignments"> & {
        assignments: Array<Omit<SchemeView["assignments"][number], "title">>;
      };
      current = {
        scheme: update.scheme,
        assignments: update.assignments.map((assignment) => ({
          ...assignment,
          title:
            assignment.assignment === ASSIGNMENT_ONE
              ? "Enzyme kinetics lab"
              : "Midterm examination",
        })),
      };
      revision += 1;
      return await json(route, current, 200, {
        "cache-control": "no-store",
        etag: `"${revision}"`,
      });
    }
    if (path === `/api/courses/${COURSE_ID}/gradebook-totals`) {
      return await json(route, totals(current.scheme.mode), 200, { "cache-control": "no-store" });
    }
    if (path === `/api/courses/${COURSE_ID}/grade-export.csv`) {
      expect(request.method()).toBe("POST");
      return await route.fulfill({
        status: 200,
        contentType: "text/csv",
        headers: {
          "cache-control": "no-store",
          "content-disposition": "attachment; filename=ple-course-grades.csv",
          "x-ple-course-grade-export-id": "0198e000-0000-7000-8000-000000000099",
        },
        body: "record_type,aggregation_mode\r\nmetadata,weightedCategories\r\n",
      });
    }
    return await json(route, { error: `unexpected grade-settings request ${path}` }, 500);
  });

  await page.setViewportSize({ width: 393, height: 852 });
  await openGradeSettings(page);
  await expect(page.getByRole("heading", { name: "Course grade settings" })).toBeVisible();
  const firstCourseLink = page.getByRole("link", { name: "Assignments", exact: true });
  const activeCourseLink = page.getByRole("link", { name: "Grade settings", exact: true });
  await expect(firstCourseLink).toBeVisible();
  await expect(activeCourseLink).toBeVisible();
  for (const link of [firstCourseLink, activeCourseLink]) {
    const box = await link.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(393);
  }
  await page.setViewportSize({ width: 800, height: 1_280 });
  await expect(page.getByRole("radio", { name: "Total points" })).toBeChecked();
  await expect(page.getByRole("row", { name: /Avery Student 75% C/u })).toBeVisible();
  await expect(page.getByRole("row", { name: /Recalculating Student Unavailable/u })).toBeVisible();

  await page.getByRole("radio", { name: "Weighted categories" }).check();
  await expect(page.getByText("Weight total: 100.00% of 100.00%.")).toBeVisible();
  const categorySelects = page.getByRole("combobox", { name: "Category" });
  for (let index = 0; index < (await categorySelects.count()); index += 1) {
    await categorySelects.nth(index).selectOption({ label: "Course work" });
    await expect(categorySelects.nth(index)).toHaveValue(/.+/u);
  }
  const saveButton = page.getByRole("button", { name: "Save grade settings" });
  const weightInput = page.getByRole("spinbutton", { name: "Weight (%)" });
  await weightInput.fill("100.001");
  expect(
    await weightInput.evaluate((element) => (element as HTMLInputElement).checkValidity()),
  ).toBe(false);
  await saveButton.click();
  expect(savedBody).toBeUndefined();
  await expect(page.getByText("Grade settings saved.")).toHaveCount(0);
  await weightInput.fill("100");
  expect(
    await weightInput.evaluate((element) => (element as HTMLInputElement).checkValidity()),
  ).toBe(true);
  const dropInput = page.getByRole("spinbutton", { name: "Drop lowest" });
  await dropInput.fill("1.5");
  expect(await dropInput.evaluate((element) => (element as HTMLInputElement).checkValidity())).toBe(
    false,
  );
  await saveButton.click();
  expect(savedBody).toBeUndefined();
  await dropInput.fill("0");
  expect(await dropInput.evaluate((element) => (element as HTMLInputElement).checkValidity())).toBe(
    true,
  );
  await saveButton.click();
  await expect(page.getByText("Grade settings saved.")).toBeVisible();
  expect(savedBody).toMatchObject({
    scheme: { mode: "weightedCategories" },
    assignments: [
      { assignment: ASSIGNMENT_ONE, included: true },
      { assignment: ASSIGNMENT_TWO, included: true },
    ],
  });
  expect(JSON.stringify(savedBody)).not.toContain("Enzyme kinetics lab");

  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export audited course grades CSV" }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe("ple-course-grades.csv");
  await expect(page.getByText("Audited course export is ready.")).toBeVisible();

  const accessibility = await new AxeBuilder({ page }).analyze();
  expect(
    accessibility.violations.filter(
      (violation) => violation.impact === "serious" || violation.impact === "critical",
    ),
  ).toEqual([]);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
});

test("a stale save preserves the local draft until the instructor adopts the latest scheme", async ({
  page,
}) => {
  let reads = 0;
  await page.route("**/api/**", async (route) => {
    if (await commonInstructorRoute(route)) return;
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === `/api/courses/${COURSE_ID}/grade-scheme` && request.method() === "GET") {
      reads += 1;
      const current = reads === 1 ? totalPointsView() : weightedView("Remote update");
      return await json(route, current, 200, {
        "cache-control": "no-store",
        etag: reads === 1 ? '"1"' : '"2"',
      });
    }
    if (path === `/api/courses/${COURSE_ID}/grade-scheme` && request.method() === "PUT") {
      return await json(route, { error: "changed" }, 412, { "cache-control": "no-store" });
    }
    if (path === `/api/courses/${COURSE_ID}/gradebook-totals`) {
      return await json(route, totals("totalPoints"), 200, { "cache-control": "no-store" });
    }
    return await json(route, { error: `unexpected conflict request ${path}` }, 500);
  });

  await openGradeSettings(page);
  await page.getByRole("radio", { name: "Weighted categories" }).check();
  const title = page.getByLabel("Title");
  await title.fill("Local draft");
  const categorySelects = page.getByRole("combobox", { name: "Category" });
  for (let index = 0; index < (await categorySelects.count()); index += 1) {
    await categorySelects.nth(index).selectOption({ label: "Local draft" });
    await expect(categorySelects.nth(index)).toHaveValue(/.+/u);
  }
  await page.getByRole("button", { name: "Save grade settings" }).click();
  await expect(page.getByText(/Your draft is preserved/u)).toBeVisible();
  await expect(title).toHaveValue("Local draft");
  await page.getByRole("button", { name: "Adopt latest settings" }).click();
  await expect(page.getByLabel("Title")).toHaveValue("Remote update");
  await expect(page.getByText(/Latest server settings adopted/u)).toBeVisible();
});

test("student direct navigation denies before any course-grade transport", async ({ page }) => {
  const requests: string[] = [];
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    requests.push(path);
    if (path === "/api/auth/session") return await json(route, session(["student"]));
    if (path === "/api/auth/account/presentation") {
      return await json(route, { contrast: "standard" });
    }
    return await json(route, { error: "student grade transport must not run" }, 500);
  });

  await openGradeSettings(page);
  const denial = page.locator('[data-route-surface="routeAccessDenied"]');
  await expect(denial).toHaveAttribute("data-denied-route", "courseGradeSettings");
  await expect(
    denial.getByRole("heading", { name: "This page is available to instructors only" }),
  ).toBeFocused();
  await expect(page.locator('[data-route-surface="courseGradeSettings"]')).toHaveCount(0);
  expect(requests.filter((path) => path.includes("grade-") || path.includes("gradebook"))).toEqual(
    [],
  );
});
