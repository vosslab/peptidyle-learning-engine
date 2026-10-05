// Current mixed-Library evidence with the Pool created beside this capture workflow.

import { decodeQuestionSearchPage } from "../../../src/api/decoders/question_library";

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
  followCaptureLink,
  openInstructorLibraryBrowse,
} from "./visible_workflows";

const SOURCE_QUESTION_TITLE = "Genetic disorders: Which one?";

async function loadDefaultLibrary(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Search", exact: true }).click();
}

async function createSearchPool(page: Page): Promise<string> {
  const poolTitle = `Library search screenshot ${Date.now()}`;
  const response: unknown = await page.evaluate(async (sourceQuestionTitle) => {
    const query = new URLSearchParams({
      kind: "questions",
      membership: "all",
      authorship: "any",
      sort: "titleAscending",
      page_size: "50",
      text: sourceQuestionTitle,
    });
    const searched = await fetch(`/api/questions/search?${query.toString()}`);
    if (!searched.ok) throw new Error(`Pilot Question search status ${searched.status}`);
    return (await searched.json()) as unknown;
  }, SOURCE_QUESTION_TITLE);
  const matches = decodeQuestionSearchPage(response).items.filter(
    (item) =>
      item.kind === "question" &&
      item.question.summary.metadata.questionTitle === SOURCE_QUESTION_TITLE,
  );
  const match = matches[0];
  if (matches.length !== 1 || match?.kind !== "question")
    throw new Error("Expected one exact Pilot Question for this capture.");
  const source = match.question.summary;
  await page.evaluate(
    async ({ title, questionId, revisionNumber }) => {
      const created = await fetch("/api/question-pools", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          title,
          description: "One exact Pilot Question for this Library search capture.",
          members: [{ questionId, revisionNumber }],
          interchangeabilityAttested: true,
        }),
      });
      if (created.status !== 201) throw new Error(`Pool creation status ${created.status}`);
    },
    {
      title: poolTitle,
      questionId: source.questionId,
      revisionNumber: source.publishedQuestionRevisionTuple.revisionNumber,
    },
  );
  return poolTitle;
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
    const poolTitle = await createSearchPool(page);
    // The Library intentionally remains idle until the visible Search action submits its
    // default query.
    await loadDefaultLibrary(page);
    const rows = publishedRows(page);
    await rows.filter({ hasText: poolTitle }).first().waitFor();
    await runtime.captureCheckpoint(session, "library_mixed_results");
    await page
      .getByRole("combobox", { name: "Question membership", exact: true })
      .selectOption("all");
    await rows.filter({ hasText: SOURCE_QUESTION_TITLE }).first().waitFor();
    await runtime.captureCheckpoint(session, "library_all_questions");
    await page.getByRole("combobox", { name: "Show", exact: true }).selectOption("pools");
    await rows.filter({ hasText: poolTitle }).first().waitFor();
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
    await page.getByRole("button", { name: /^Genetics/u }).click();
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
