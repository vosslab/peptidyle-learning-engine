// Private live proof that a host-published WebWork question reaches a learner through PLE.

import { configuredLiveWebworkInputs } from "../../../playwright.config";

import { expect, test, type Page } from "@playwright/test";

test.describe.configure({ mode: "serial" });

const QUESTION_ID = /^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u;
const CATALOG_PRIVATE_ID_KEYS = new Set(["problemId", "versionId"]);

function configuredQuestionId(): string {
  const questionId = process.env["PLE_WP_R2_WEBWORK_QUESTION_ID"];
  if (questionId === undefined || !QUESTION_ID.test(questionId)) {
    throw new Error("PLE_WP_R2_WEBWORK_QUESTION_ID must be one canonical Question ID");
  }
  return questionId;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function catalogPrivateIdentityKeys(value: unknown, found: string[] = []): string[] {
  if (Array.isArray(value)) {
    for (const item of value) catalogPrivateIdentityKeys(item, found);
  } else if (isRecord(value)) {
    for (const [key, item] of Object.entries(value)) {
      if (CATALOG_PRIVATE_ID_KEYS.has(key)) found.push(key);
      catalogPrivateIdentityKeys(item, found);
    }
  }
  return found;
}

function attachCatalogPayloadAudit(page: Page): { readonly finish: () => Promise<void> } {
  const hiddenIdentityKeys: string[] = [];
  const completed: Promise<void>[] = [];
  page.on("requestfinished", (request) => {
    if (!new URL(request.url()).pathname.startsWith("/api/problems")) return;
    completed.push(
      request
        .response()
        .then(async (response) => {
          const contentType = response?.headers()["content-type"]?.toLowerCase() ?? "";
          if (response === null || !contentType.includes("json")) return;
          hiddenIdentityKeys.push(
            ...catalogPrivateIdentityKeys(JSON.parse(await response.text()) as unknown),
          );
        })
        .catch(() => undefined),
    );
  });
  page.on("request", (request) => {
    if (!new URL(request.url()).pathname.startsWith("/api/problems")) return;
    const body = request.postData();
    if (body === null) return;
    try {
      hiddenIdentityKeys.push(...catalogPrivateIdentityKeys(JSON.parse(body) as unknown));
    } catch {
      // Catalog search uses a query string, so non-JSON bodies carry no catalog identity fields.
    }
  });
  return {
    async finish(): Promise<void> {
      await Promise.all(completed);
      expect(hiddenIdentityKeys).toEqual([]);
    },
  };
}

async function signInAndOpenLibrary(page: Page): Promise<void> {
  const live = configuredLiveWebworkInputs;
  if (live === undefined) throw new Error("private live WebWork inputs are unavailable");
  await page.goto(live.baseUrl);
  await page.getByLabel("Local development credential").fill(live.studentCredential);
  await page.getByRole("button", { name: "Sign in locally", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();
  await page.getByRole("link", { name: "Library", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Question library" })).toBeVisible();
}

test.describe("WP-R2 private host seed and WebWork renderer", () => {
  test.skip(
    configuredLiveWebworkInputs === undefined,
    "requires the private full-stack host-seed invocation",
  );

  test("shows the retained Question ID and renders its WebWork question through PLE", async ({
    page,
  }) => {
    test.slow();
    const questionId = configuredQuestionId();
    const audit = attachCatalogPayloadAudit(page);
    await signInAndOpenLibrary(page);
    const search = page.getByLabel("Search published questions");
    await search.fill(questionId);
    const row = page.locator(".catalog-row", {
      has: page.locator("code", { hasText: questionId }),
    });
    await expect(row).toBeVisible();
    await expect(row.locator("code")).toHaveText(questionId);
    await page.getByRole("link", { name: "Courses", exact: true }).click();
    const course = page.locator(".course-card").filter({
      has: page.getByRole("heading", { name: "PLE WebWork pilot E2E course" }),
    });
    await course.getByRole("link", { name: "Open course" }).click();
    const assignment = page.locator(".course-card").filter({
      has: page.getByRole("heading", { name: "PLE WebWork pilot E2E assignment" }),
    });
    await assignment.getByRole("link", { name: "Review assignment" }).click();
    await page.getByRole("button", { name: "Start or resume practice" }).click();
    await expect(
      page.getByRole("heading", {
        name: "Biochemistry: Identify hydrophobic compounds from formulas",
      }),
    ).toBeVisible();
    await expect(page.getByRole("radio")).toHaveCount(5);
    await expect(page.locator("body")).not.toContainText(
      /[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}/iu,
    );
    await audit.finish();
  });
});
