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
  filterBrowseToMultipleChoice,
  searchNativeMcLibraryQuestion,
  followCaptureLink,
  openInstructorLibraryBrowse,
} from "./visible_workflows";

async function refreshLibrarySearch(page: Page): Promise<void> {
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

function resultRows(page: Page): Locator {
  return page.locator("[data-record-id]");
}

function titledRow(page: Page, title: string): Locator {
  return resultRows(page).filter({ has: page.getByText(title, { exact: true }) });
}

async function filterLibraryObjects(page: Page, kind: "questions" | "pools"): Promise<void> {
  const libraryObjectHeader = page.getByRole("columnheader", { name: /Library object/ });
  await libraryObjectHeader
    .getByRole("button", { name: /^Filter Library object(?:, \d+ selected)?$/ })
    .click();
  const dialog = page.getByRole("dialog", { name: "Filter Library object", exact: true });
  if (kind === "questions") {
    await dialog.getByRole("checkbox", { name: "Published Question" }).check();
  } else {
    await dialog.getByRole("checkbox", { name: "Published Question" }).uncheck();
    await dialog.getByRole("checkbox", { name: "Question Pool" }).check();
  }
  await dialog.getByRole("button", { name: "Close filter", exact: true }).click();
}

async function searchLibrary(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("library_mixed_results");
  const page = session.page;
  try {
    await enterInstructor(page);
    const displays = page.getByRole("group", { name: "Record display", exact: true });
    const { poolTitle, sourceQuestionTitle } = await createSearchPool(page);
    await refreshLibrarySearch(page);
    await page.getByRole("button", { name: "Sort by Title, ascending", exact: true }).click();
    const poolRow = titledRow(page, poolTitle);
    const questionRow = titledRow(page, sourceQuestionTitle)
      .filter({ has: page.getByRole("link", { name: "Open", exact: true }) })
      .first();
    await poolRow.getByRole("link", { name: "Open Question Pool", exact: true }).waitFor();
    await questionRow.getByRole("link", { name: "Open", exact: true }).waitFor();
    await runtime.captureCheckpoint(session, "library_mixed_results");
    await filterLibraryObjects(page, "questions");
    await poolRow.waitFor({ state: "hidden" });
    await questionRow.getByRole("link", { name: "Open", exact: true }).waitFor();
    await runtime.captureCheckpoint(session, "library_all_questions");
    await filterLibraryObjects(page, "pools");
    await questionRow.waitFor({ state: "hidden" });
    await poolRow.getByRole("link", { name: "Open Question Pool", exact: true }).waitFor();
    if ((await resultRows(page).getByRole("link", { name: "Open", exact: true }).count()) !== 0) {
      throw new Error("Pools-only Library results include a Question.");
    }
    await runtime.captureCheckpoint(session, "library_pools_only");
    const cardsButton = displays.getByRole("button", { name: "Cards", exact: true });
    await cardsButton.click();
    if ((await cardsButton.getAttribute("aria-pressed")) !== "true") {
      throw new Error("Cards is not the active Library record display.");
    }
    await poolRow.getByRole("link", { name: "Open Question Pool", exact: true }).waitFor();
    for (const [mode, checkpoint] of [
      ["Compact", "library_compact"],
      ["List", "library_list"],
      ["Visual Boxes", "library_visual"],
    ] as const) {
      await displays.getByRole("button", { name: mode, exact: true }).click();
      await runtime.captureCheckpoint(session, checkpoint);
    }
    await openInstructorLibraryBrowse(page);
    await filterBrowseToMultipleChoice(page);
    await page.getByRole("link", { name: "Search within results", exact: true }).click();
    await page.getByLabel("Search Question Library", { exact: true }).fill(poolTitle);
    await page.getByLabel("Search Question Library", { exact: true }).press("Enter");
    const link = titledRow(page, poolTitle).getByRole("link", {
      name: "Open Question Pool",
      exact: true,
    });
    await followCaptureLink(page, link);
    await page.getByRole("main", { name: poolTitle, exact: true }).waitFor();
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
