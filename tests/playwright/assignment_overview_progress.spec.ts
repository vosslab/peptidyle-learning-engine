// Built learner assignment-overview coverage for score-state disclosure copy.

import { expect, test, type Page, type Route } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";

const course = publishedProblemFixture.course;
const assignment = publishedProblemFixture.assignment;
const learnerAssignment = {
  id: assignment.id,
  reference: assignment.reference,
  title: assignment.title,
  items: assignment.items,
  selectionGroups: assignment.selectionGroups,
};
const assignmentPath = `/courses/${course.reference}/assignments/${assignment.reference}`;

type ScoreState = "noActivity" | "withheld" | "available";
type ClassStatistics =
  | undefined
  | { readonly state: "insufficientEvidence" }
  | {
      readonly state: "available";
      readonly completedLearnerCohortSize: number;
      readonly assignmentAverageScore: number;
    };

const learnerViewports = [
  { name: "laptop", width: 1280, height: 800 },
  { name: "tablet", width: 800, height: 1280 },
  { name: "phone", width: 393, height: 852 },
] as const;

const scoreStateCases = [
  {
    scoreState: "noActivity" as const,
    status: "No score yet. Submit a response to record scored progress.",
    scoreValues: [],
  },
  {
    scoreState: "withheld" as const,
    status: "Score is currently unavailable. 1 completed run recorded.",
    scoreValues: [],
  },
  {
    scoreState: "available" as const,
    status: "Score available: Current 75%, Latest 50%, Best 100%.",
    scoreValues: ["75%", "50%", "100%"],
  },
] as const;

function json(route: Route, value: unknown, headers: Record<string, string> = {}): Promise<void> {
  return route.fulfill({
    contentType: "application/json",
    headers,
    body: JSON.stringify(value),
  });
}

function learnerProgress(scoreState: ScoreState): object {
  switch (scoreState) {
    case "noActivity":
      return {
        scoreState,
        currentScore: null,
        bestScore: null,
        latestScore: null,
        completedRunCount: 0,
        totalQuestionAttempts: 0,
        lastActivityAt: 1786000000000,
      };
    case "withheld":
      return {
        scoreState,
        currentScore: null,
        bestScore: null,
        latestScore: null,
        completedRunCount: 1,
        totalQuestionAttempts: 1,
        lastActivityAt: 1786000000000,
      };
    case "available":
      return {
        scoreState,
        currentScore: 0.75,
        bestScore: 1,
        latestScore: 0.5,
        completedRunCount: 2,
        totalQuestionAttempts: 3,
        lastActivityAt: 1786000000000,
      };
  }
}

async function installLearnerRoutes(
  page: Page,
  scoreState: ScoreState,
  classStatistics: ClassStatistics = undefined,
): Promise<void> {
  await page.addInitScript(() =>
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    }),
  );
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === "/api/auth/session") {
      return await json(route, {
        authenticated: true,
        tenant: course.tenant,
        user: {
          id: "0198e000-0000-7000-8000-000000000114",
          displayName: "Learner",
          roles: ["student"],
        },
      });
    }
    if (path === `/api/navigation/${course.reference}`) {
      return await json(route, { kind: "course", courseId: course.id });
    }
    if (path === `/api/navigation/${assignment.reference}`) {
      return await json(route, {
        kind: "assignment",
        courseId: course.id,
        assignmentId: assignment.id,
      });
    }
    if (path === `/api/courses/${course.id}`) return await json(route, course);
    if (path === `/api/courses/${course.id}/appearance`) {
      return await json(
        route,
        { theme: "grass", revision: "1", banner: null },
        { "cache-control": "no-store", etag: '"1"' },
      );
    }
    if (path === `/api/assignments/${assignment.id}/learner`) {
      return await json(route, learnerAssignment);
    }
    if (path === `/api/assignments/${assignment.id}/summary`) {
      return await json(route, {
        ...learnerProgress(scoreState),
        ...(classStatistics === undefined ? {} : { classStatistics }),
      });
    }
    return await json(route, {
      error: `unexpected learner overview request ${request.method()} ${path}`,
    });
  });
}

async function openOverview(
  page: Page,
  scoreState: ScoreState,
  viewport: { readonly width: number; readonly height: number },
  classStatistics: ClassStatistics = undefined,
): Promise<void> {
  await installLearnerRoutes(page, scoreState, classStatistics);
  await page.setViewportSize(viewport);
  await page.goto("/");
  await page.evaluate((path) => {
    history.pushState({}, "", path);
    dispatchEvent(new PopStateEvent("popstate"));
  }, assignmentPath);
  await expect(page.locator('[data-route-surface="assignmentOverview"]')).toBeVisible();
  await expect(page.getByRole("heading", { name: assignment.title })).toBeVisible();
}

const classStatisticsCases = [
  {
    name: "omitted",
    scoreState: "withheld" as const,
    classStatistics: undefined,
    expected: undefined,
  },
  {
    name: "insufficient evidence",
    scoreState: "withheld" as const,
    classStatistics: { state: "insufficientEvidence" } as const,
    expected: "Not enough evidence to show class statistics.",
  },
  {
    name: "available aggregate while score is withheld",
    scoreState: "withheld" as const,
    classStatistics: {
      state: "available",
      completedLearnerCohortSize: 8,
      assignmentAverageScore: 0.625,
    } as const,
    expected: "Class average: 62.5%. Based on 8 completed learners.",
  },
] as const;

for (const statisticsCase of classStatisticsCases) {
  for (const viewport of learnerViewports) {
    test(`student overview renders ${statisticsCase.name} class statistics at ${viewport.name}`, async ({
      page,
    }) => {
      await openOverview(page, statisticsCase.scoreState, viewport, statisticsCase.classStatistics);

      const facts = page.locator(".assignment-facts");
      await expect(facts.getByRole("status")).toHaveText(
        "Score is currently unavailable. 1 completed run recorded.",
      );
      await expect(facts).not.toContainText(/afterSubmit|afterDue|afterClose|duringAttempt|never/u);
      for (const row of ["Current score", "Latest score", "Best score"]) {
        await expect(facts.getByText(row, { exact: true })).toHaveCount(0);
      }
      if (statisticsCase.expected === undefined) {
        await expect(facts.getByText("Class statistics", { exact: true })).toHaveCount(0);
      } else {
        await expect(facts.getByText("Class statistics", { exact: true })).toHaveCount(1);
        await expect(facts.getByText(statisticsCase.expected, { exact: true })).toHaveCount(1);
      }
      if (statisticsCase.name === "available aggregate while score is withheld") {
        await expect(facts.getByText(/^\d+(?:\.\d+)?%$/u)).toHaveCount(0);
      }
      if (viewport.name === "phone") {
        expect(
          await page.evaluate(
            () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
          ),
        ).toBe(true);
      }
    });
  }
}

for (const viewport of learnerViewports) {
  for (const scoreCase of scoreStateCases) {
    test(`student overview keeps ${scoreCase.scoreState} distinct at ${viewport.name} viewport`, async ({
      page,
    }) => {
      await openOverview(page, scoreCase.scoreState, viewport);

      const facts = page.locator(".assignment-facts");
      await expect(page.getByRole("heading", { name: assignment.title })).toBeVisible();
      await expect(facts.getByRole("status")).toHaveText(scoreCase.status);
      await expect(facts).not.toContainText(/afterSubmit|afterDue|afterClose|duringAttempt|never/u);

      for (const row of ["Current score", "Latest score", "Best score"]) {
        await expect(facts.getByText(row, { exact: true })).toHaveCount(
          scoreCase.scoreState === "available" ? 1 : 0,
        );
      }

      if (scoreCase.scoreState === "available") {
        for (const value of scoreCase.scoreValues) {
          await expect(facts.getByText(value, { exact: true })).toHaveCount(1);
        }
      } else {
        await expect(facts.getByText(/\d+%/u)).toHaveCount(0);
        await expect(facts.getByText("No score yet", { exact: false })).toHaveCount(
          scoreCase.scoreState === "noActivity" ? 1 : 0,
        );
      }
    });
  }
}
