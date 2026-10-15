// Current mixed-Library evidence with the Pool created beside this capture workflow.

import type { ScenarioRuntime } from "./runtime";
import type { Locator, Page } from "playwright";
import { catalogScreenshotFilename } from "./filenames";
import {
  viewportCoverage,
  type CaptureDeclaration,
  type ScenarioDefinition,
} from "./scenario_types";
import {
  enterInstructor,
  searchNativeMcLibraryQuestion,
  followCaptureLink,
  openInstructorLibraryBrowse,
} from "./visible_workflows";

async function loadDefaultLibrary(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Search", exact: true }).click();
}

async function createSearchPool(
  page: Page,
): Promise<{ poolTitle: string; sourceQuestionTitle: string }> {
  const source = await searchNativeMcLibraryQuestion(page);
  const sourceQuestionTitle = source.metadata.questionTitle;
  const poolTitle = `${sourceQuestionTitle} screenshot ${Date.now()}`;
  await page.evaluate(
    async ({ title, publishedQuestionRevisionTuple }) => {
      const created = await fetch("/api/question-pools", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          title,
          description: "One selected published Question for this Library search capture.",
          members: [publishedQuestionRevisionTuple],
        }),
      });
      if (created.status !== 201) {
        const responseBody = await created.text();
        throw new Error(`Pool creation status ${created.status}: ${responseBody}`);
      }
    },
    {
      title: poolTitle,
      publishedQuestionRevisionTuple: source.publishedQuestionRevisionTuple,
    },
  );
  return { poolTitle, sourceQuestionTitle };
}

function publishedRows(page: Page): Locator {
  return page
    .getByRole("region", { name: "Question Library results", exact: true })
    .getByRole("list", { name: "Question Library results", exact: true })
    .getByRole("listitem");
}

async function searchLibrary(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("library_mixed_results");
  const page = session.page;
  try {
    await enterInstructor(page);
    const { poolTitle, sourceQuestionTitle } = await createSearchPool(page);
    await page.getByLabel("Search Question Library", { exact: true }).fill("");
    await page
      .getByRole("combobox", { name: "Question membership", exact: true })
      .selectOption("inNoPool");
    await loadDefaultLibrary(page);
    const rows = publishedRows(page);
    await rows.filter({ hasText: poolTitle }).first().waitFor();
    await rows
      .filter({ has: page.getByRole("link", { name: "Open", exact: true }) })
      .first()
      .waitFor();
    await runtime.captureCheckpoint(session, "library_mixed_results");
    await page
      .getByRole("combobox", { name: "Question membership", exact: true })
      .selectOption("all");
    await rows
      .filter({
        hasText: sourceQuestionTitle,
        has: page.getByRole("link", { name: "Open", exact: true }),
      })
      .first()
      .waitFor();
    await runtime.captureCheckpoint(session, "library_all_questions");
    await page.getByRole("combobox", { name: "Show", exact: true }).selectOption("pools");
    await rows.filter({ hasText: poolTitle }).first().waitFor();
    if ((await rows.getByRole("link", { name: "Open", exact: true }).count()) !== 0) {
      throw new Error("Pools-only Library results include a Question.");
    }
    await runtime.captureCheckpoint(session, "library_pools_only");
    const displays = page.getByRole("group", { name: "Library result display", exact: true });
    for (const [mode, checkpoint] of [
      ["Compact", "library_compact"],
      ["List", "library_list"],
      ["Visual boxes", "library_visual"],
    ] as const) {
      await displays.getByRole("button", { name: mode, exact: true }).click();
      await runtime.captureCheckpoint(session, checkpoint);
    }
    await openInstructorLibraryBrowse(page);
    await page.getByRole("button", { name: /^Multiple choice/u }).click();
    await page.getByRole("link", { name: "Search within results", exact: true }).click();
    const leaveSearch = page.getByRole("dialog", { name: "Leave this search?", exact: true });
    await leaveSearch.waitFor();
    await leaveSearch.getByRole("button", { name: "Leave page", exact: true }).click();
    await page.getByLabel("Search Question Library", { exact: true }).fill(poolTitle);
    await page.getByLabel("Search Question Library", { exact: true }).press("Enter");
    await rows.filter({ hasText: poolTitle }).first().waitFor();
    const link = rows
      .filter({ hasText: poolTitle })
      .getByRole("link", { name: "Open Question Pool", exact: true });
    await followCaptureLink(page, link);
    await page.getByRole("heading", { name: poolTitle, exact: true }).waitFor();
    await runtime.captureCheckpoint(session, "library_pool_detail");
  } finally {
    await runtime.close(session);
  }
}

const captures = [
  ["library_mixed_results", "mixed default"],
  ["library_all_questions", "all Questions"],
  ["library_pools_only", "Pools only"],
  ["library_compact", "Pool compact display"],
  ["library_list", "Pool list display"],
  ["library_visual", "Pool visual-box display"],
  ["library_pool_detail", "Question Pool detail"],
] as const;

const libraryCaptures: ReadonlyArray<CaptureDeclaration> = [
  ...captures.map<CaptureDeclaration>(([checkpoint, state]) => ({
    checkpoint,
    filenameStem: catalogScreenshotFilename(
      "questions",
      checkpoint === "library_pool_detail" ? "browseQuestionLibrary" : "searchQuestionLibrary",
      checkpoint.replace(/^library_/u, ""),
    ),
    area: "question library",
    workflow: "Library discovery",
    state,
    viewport: "laptop",
    privacyProfile: "instructor_answer_free",
    caption: `Question Library ${state}`,
  })),
];

export const INSTRUCTOR_SHARED_SEARCH_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "instructor_library_results",
    role: "instructor",
    captures: libraryCaptures,
    viewportCoverage: viewportCoverage(["laptop"], {
      phone: {
        target: "library_mixed_results",
        reason: "Instructor laptop capture is the representative.",
      },
      tablet: {
        target: "library_mixed_results",
        reason: "The laptop result grid is the representative wide-layout capture.",
      },
      square: {
        target: "library_mixed_results",
        reason: "The laptop result grid is the representative wide-layout capture.",
      },
    }),
    run: async (runtime): Promise<void> => {
      await searchLibrary(runtime);
    },
  },
];
