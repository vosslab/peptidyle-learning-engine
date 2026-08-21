// Selector contract: assignment_preview_page.tsx labels and data-route-surface define this journey.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page, type Route } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import { createMockAssignmentState } from "../../src/api/mock/handlers/authoring";
import {
  ASSIGNMENT_REFERENCE,
  COURSE_ID,
  COURSE_REFERENCE,
  appearanceHeaders,
  json,
  session,
} from "./course_appearance_fixtures";

const PATH = `/instructor/courses/${COURSE_REFERENCE}/assignments/${ASSIGNMENT_REFERENCE}/delivery-check`;
const REVISION = "7";
const moment = { value: "2026-08-25T09:00:00.000", timeZone: "America/Chicago" };

function schedule(source = "base"): unknown {
  return {
    availableAt: { value: "2026-08-24T09:00:00.000", source },
    dueAt: { value: "2026-08-26T09:00:00.000", source },
    closesAt: { value: "2026-08-27T09:00:00.000", source },
    timeLimitSeconds: { value: 3600, source },
    attemptLimit: { value: 2, source },
    lateSubmission: { value: "accept", source },
    deadlineBehavior: { value: "autoSubmit", source },
  };
}

function deliveryResponse(kind: "derived" | "synthetic"): unknown {
  return {
    evaluation: {
      kind: "allowed",
      subject: {
        kind,
        assignment: ASSIGNMENT_REFERENCE,
        revision: REVISION,
        selectedMoment: moment,
        groups: [{ purpose: "lab" }],
        policy: schedule("groupAccommodation"),
        priorRunCount: 0,
      },
      entitlement: "groupAudience",
      schedule: schedule("groupAccommodation"),
      disclosure: [
        {
          kind: "available",
          moment: "now",
          flags: {
            scoreShown: false,
            correctnessShown: false,
            feedbackShown: false,
            solutionShown: false,
            statisticsShown: false,
          },
        },
        {
          kind: "available",
          moment: "due",
          flags: {
            scoreShown: true,
            correctnessShown: true,
            feedbackShown: true,
            solutionShown: false,
            statisticsShown: false,
          },
        },
        { kind: "unavailable", moment: "close", reason: "boundaryMissing" },
      ],
    },
    accommodation: { before: schedule("base"), after: schedule("groupAccommodation") },
  };
}

function assignmentEditor(): unknown {
  const state = createMockAssignmentState();
  return {
    ...state.assignment,
    teachingSettings: state.teachingSettings,
    currentState: state.currentState,
  };
}

async function open(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
    });
  });
  await page.goto(PATH);
}

interface DeliveryFixtureState {
  derivedConflict: boolean;
}

async function instructorApi(route: Route, state: DeliveryFixtureState): Promise<boolean> {
  const request = route.request();
  const path = new URL(request.url()).pathname;
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
  if (path === `/api/navigation/${ASSIGNMENT_REFERENCE}`) {
    await json(route, {
      kind: "assignment",
      courseId: COURSE_ID,
      assignmentId: publishedProblemFixture.assignment.id,
    });
    return true;
  }
  if (path === `/api/assignments/${publishedProblemFixture.assignment.id}`) {
    await json(route, assignmentEditor(), 200, appearanceHeaders(REVISION));
    return true;
  }
  if (
    path === `/api/courses/${COURSE_REFERENCE}/assignments/${ASSIGNMENT_REFERENCE}/preview-schedule`
  ) {
    expect(request.headers()["if-match"]).toBe(`"${REVISION}"`);
    await json(
      route,
      {
        revision: REVISION,
        rows: [
          {
            kind: "granted",
            membership: "M-11",
            display: "Avery Student",
            entitlement: "groupAudience",
            schedule: schedule("groupAccommodation"),
          },
          { kind: "denied", membership: "M-12", display: "Jordan Student", reason: "notEntitled" },
        ],
        nextCursor: null,
      },
      200,
      { "cache-control": "no-store" },
    );
    return true;
  }
  if (path === `/api/courses/${COURSE_ID}/groups`) {
    await json(
      route,
      {
        groups: [
          {
            reference: "G-4",
            title: "Thursday lab",
            purpose: "lab",
            revision: REVISION,
            memberCount: 8,
          },
        ],
        nextCursor: null,
      },
      200,
      { "cache-control": "no-store" },
    );
    return true;
  }
  if (path.endsWith("/preview-subjects/derived") || path.endsWith("/preview-subjects/synthetic")) {
    expect(request.headers()["if-match"]).toBe(`"${REVISION}"`);
    const body = request.postDataJSON() as { membership?: string };
    if (body.membership !== undefined && state.derivedConflict) {
      state.derivedConflict = false;
      await json(route, { error: "stale assignment revision" }, 412, {
        "cache-control": "no-store",
      });
      return true;
    }
    await json(
      route,
      deliveryResponse(body.membership === undefined ? "synthetic" : "derived"),
      200,
      { "cache-control": "no-store" },
    );
    return true;
  }
  return false;
}

test("instructor checks assignment delivery with derived and synthetic subjects", async ({
  page,
}) => {
  const fixtureState: DeliveryFixtureState = { derivedConflict: true };
  await page.route("**/api/**", async (route) => {
    if (await instructorApi(route, fixtureState)) return;
    await json(
      route,
      { error: `unexpected request ${new URL(route.request().url()).pathname}` },
      500,
    );
  });
  await page.setViewportSize({ width: 1280, height: 800 });
  await open(page);
  await expect(page.getByRole("heading", { name: "Assignment delivery check" })).toBeVisible();
  await expect(
    page.getByText("Preview only - no learner work or grades are created."),
  ).toBeVisible();
  await expect(page.getByRole("row", { name: /Avery Student Group audience/u })).toBeVisible();
  await page.getByRole("combobox", { name: "Student membership reference" }).selectOption("M-11");
  await page.getByRole("button", { name: "Check assignment delivery" }).click();
  await expect(
    page.getByRole("button", { name: "Reload latest assignment revision" }),
  ).toBeVisible();
  await expect(page.getByRole("combobox", { name: "Student membership reference" })).toHaveValue(
    "M-11",
  );
  await page.getByRole("button", { name: "Reload latest assignment revision" }).click();
  await expect(
    page.getByText(
      "The latest assignment revision is loaded. Your hypothetical draft is preserved.",
    ),
  ).toBeVisible();
  await expect(page.getByRole("combobox", { name: "Student membership reference" })).toHaveValue(
    "M-11",
  );
  await page.getByRole("button", { name: "Check assignment delivery" }).click();
  await expect(page.getByRole("heading", { name: "Resolved delivery" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Accommodation effect" })).toBeVisible();
  await expect(page.getByText(/score withheld; correctness withheld/iu)).toBeVisible();
  const resolvedHeading = page.getByRole("heading", { name: "Resolved delivery" });
  const disclosureHeading = page.getByRole("heading", { name: "Disclosure" });
  const disclosureList = page.locator(".preview-disclosure-list");
  await expect(resolvedHeading).toBeFocused();
  for (const visibleResult of [resolvedHeading, disclosureHeading, disclosureList]) {
    const box = await visibleResult.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.y + box!.height).toBeLessThanOrEqual(800);
  }
  await page.getByRole("radio", { name: "Construct a synthetic group subject" }).check();
  await page.getByRole("checkbox", { name: /Thursday lab/u }).check();
  await page.getByRole("button", { name: "Check assignment delivery" }).click();
  await expect(page.getByText(/synthetic subject/iu)).toBeVisible();
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});

test("student direct navigation denies before preview transport", async ({ page }) => {
  let protectedRequests = 0;
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (
      path.startsWith(
        `/api/courses/${COURSE_REFERENCE}/assignments/${ASSIGNMENT_REFERENCE}/preview-`,
      )
    )
      protectedRequests += 1;
    if (path === "/api/auth/session") return await json(route, session(["student"]));
    await json(route, { error: "unexpected" }, 500);
  });
  await open(page);
  await expect(
    page.getByRole("heading", { name: "This page is available to instructors only" }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "This page is available to instructors only" }),
  ).toBeVisible();
  expect(protectedRequests).toBe(0);
});
