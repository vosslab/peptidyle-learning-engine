// qti_profile_import.spec.ts - routed acceptance for private QTI profile import review.
// Selector contract: WorkspaceEditorLivePage composes the native labels, headings, radios,
// checkboxes, and buttons from QtiProfileImportPage and FlatQuestionEditorPage.

import { expect, test, type Locator, type Page, type Route } from "@playwright/test";

const workspace = "00000000-0000-4000-8000-000000000010";
const tenant = "00000000-0000-4000-8000-000000000011";
const user = "00000000-0000-4000-8000-000000000012";
const workspacePath = "/workspace/W-1";
const flatSourcePath = `/api/workspaces/${workspace}/flat-question`;
const flatMediaType = "application/vnd.peptidyle.flat-question+json";

type UploadMode = "recognized" | "allRejected" | "unsupported";

interface RecordedRouteRequest {
  readonly method: string;
  readonly path: string;
}

interface QtiRouteHarness {
  readonly uploadPaths: Array<string>;
  readonly conversionRevisions: Array<string | null>;
  readonly recoveryRequests: Array<RecordedRouteRequest>;
  readonly conversionCount: () => number;
  readonly flatRefetchPending: () => boolean;
  readonly releaseConversion: () => void;
  readonly releaseFlatRefetch: () => void;
  readonly changeNextReport: () => void;
  readonly useUploadMode: (mode: UploadMode) => void;
}

interface QtiRouteOptions {
  readonly flatRefetchFailures?: number;
  readonly firstUploadIndeterminate?: boolean;
  readonly holdConversion?: boolean;
  readonly holdFlatRefetch?: boolean;
  readonly staleAfterFlatSave?: boolean;
  readonly uploadMode?: UploadMode;
}

interface FlatSource {
  readonly format: "pleFlatQuestion";
  readonly version: 2;
  readonly title: string;
  readonly prompt: string;
  readonly response: {
    readonly kind: "singleChoice";
    readonly choices: ReadonlyArray<{
      readonly id: string;
      readonly text: string;
      readonly feedback: string | null;
    }>;
    readonly correctChoice: string;
  };
  readonly feedback: { readonly correct: string | null; readonly incorrect: string | null };
  readonly points: number;
  readonly attemptPolicy: { readonly maxAttempts: null; readonly feedback: "immediateFull" };
  readonly timingPolicy: { readonly kind: "untimed" };
  readonly tags: ReadonlyArray<string>;
  readonly taxonomy: ReadonlyArray<never>;
  readonly license: { readonly kind: "ccBySa" };
  readonly language: "en-US";
}

function flatSource(title = "Original private draft"): FlatSource {
  return {
    format: "pleFlatQuestion",
    version: 2,
    title,
    prompt: "Which molecule stores hereditary information?",
    response: {
      kind: "singleChoice",
      choices: [
        { id: "dna", text: "DNA", feedback: null },
        { id: "lipid", text: "A lipid", feedback: null },
      ],
      correctChoice: "dna",
    },
    feedback: { correct: null, incorrect: null },
    points: 1,
    attemptPolicy: { maxAttempts: null, feedback: "immediateFull" },
    timingPolicy: { kind: "untimed" },
    tags: ["genetics"],
    taxonomy: [],
    license: { kind: "ccBySa" },
    language: "en-US",
  };
}

function publicDraft(source: FlatSource): unknown {
  return {
    workspace,
    source: { backend: "native", family: "flat_single_choice_v2" },
    prompt: [{ kind: "text", markdown: source.prompt }],
    response: {
      kind: "multipleChoice",
      choices: source.response.choices.map((choice) => ({
        id: choice.id,
        body: [{ kind: "text", markdown: choice.text }],
      })),
      selection: { kind: "exactlyOne" },
    },
    attemptPolicy: source.attemptPolicy,
    timingPolicy: source.timingPolicy,
    randomization: { kind: "static" },
    grading: { mode: "allOrNothing", points: source.points },
    metadata: {
      title: source.title,
      tags: source.tags,
      taxonomy: source.taxonomy,
      license: source.license,
      language: source.language,
    },
  };
}

function session(): unknown {
  return {
    authenticated: true,
    tenant,
    user: { id: user, displayName: "QTI instructor", roles: ["instructor"] },
  };
}

function qtiItem(status: "accepted" | "rejected"): unknown {
  if (status === "accepted") {
    return {
      sourceIdentifier: "metabolism-basics",
      title: "Metabolism basics",
      status,
      diagnostics: [],
      defaults: [
        {
          code: "PLE_DEFAULT_POINTS",
          location: "item/metabolism-basics",
          detail: "PLE will use one point because the package did not specify points.",
        },
      ],
      warnings: [
        {
          code: "QTI_UNUSED_METADATA",
          location: "item/metabolism-basics/metadata",
          detail: "One vendor metadata field will not be copied.",
        },
      ],
    };
  }
  return {
    sourceIdentifier: "essay-followup",
    title: "Essay follow-up",
    status,
    diagnostics: [
      {
        code: "QTI_UNSUPPORTED_INTERACTION",
        location: "item/essay-followup/interaction",
        detail: "This essay interaction is outside the recognized profile.",
      },
    ],
    defaults: [],
    warnings: [],
  };
}

function readyReport(importId: string, allRejected: boolean, changed: boolean): unknown {
  return {
    importId,
    state: "ready",
    profileId: "canvas-qti-1.2-static-single-choice/v1",
    profileLabel: "Canvas QTI 1.2 static single choice",
    profileVersion: "1.2",
    reportRevision: (changed ? "3" : "1").repeat(64),
    items: allRejected ? [qtiItem("rejected")] : [qtiItem("accepted"), qtiItem("rejected")],
    pleDefaults: [
      {
        code: "PLE_DEFAULT_ATTEMPTS",
        location: "assessment",
        detail: "PLE will allow unlimited attempts unless the instructor changes the draft.",
      },
    ],
    reviewToken: (changed ? "4" : "2").repeat(64),
  };
}

function json(
  route: Route,
  value: unknown,
  status = 200,
  headers: Record<string, string> = {},
): Promise<void> {
  return route.fulfill({
    status,
    headers: { "content-type": "application/json", ...headers },
    body: JSON.stringify(value),
  });
}

function qtiJson(route: Route, value: unknown, status: number): Promise<void> {
  return json(route, value, status, { "cache-control": "no-store" });
}

function importIdFrom(path: string): string | null {
  const prefix = `/api/workspaces/${workspace}/qti-imports/`;
  if (!path.startsWith(prefix)) return null;
  return path.slice(prefix.length).split("/", 1)[0] ?? null;
}

async function installQtiRouteHarness(
  page: Page,
  options: QtiRouteOptions = {},
): Promise<QtiRouteHarness> {
  let source = flatSource();
  let flatRevision = '"7"';
  let serverRevision = '"7"';
  let reportReads = 0;
  let conversionCount = 0;
  let failNextUpload = options.firstUploadIndeterminate === true;
  let uploadMode = options.uploadMode ?? "recognized";
  let changedNextReport = false;
  let flatReadCount = 0;
  let remainingFlatRefetchFailures = options.flatRefetchFailures ?? 0;
  let releasePendingConversion: (() => void) | null = null;
  let releasePendingFlatRefetch: (() => void) | null = null;
  const uploadPaths: Array<string> = [];
  const conversionRevisions: Array<string | null> = [];
  const recoveryRequests: Array<RecordedRouteRequest> = [];

  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    if (path === "/api/auth/session") return await json(route, session());
    if (path === "/api/courses") return await json(route, { items: [], nextCursor: null });
    if (path === flatSourcePath && method === "GET") {
      flatReadCount += 1;
      if (flatReadCount > 1) recoveryRequests.push({ method, path });
      if (flatReadCount > 1 && remainingFlatRefetchFailures > 0) {
        remainingFlatRefetchFailures -= 1;
        return await json(route, { error: "temporary converted draft read failure" }, 503);
      }
      if (options.holdFlatRefetch === true && flatReadCount > 1) {
        await new Promise<void>((resolve) => {
          releasePendingFlatRefetch = resolve;
        });
      }
      return await route.fulfill({
        status: 200,
        headers: { "content-type": flatMediaType, etag: flatRevision },
        body: JSON.stringify(source),
      });
    }
    if (path === flatSourcePath && method === "PUT") {
      const candidate: unknown = request.postDataJSON();
      source = candidate as FlatSource;
      flatRevision = '"8"';
      serverRevision = options.staleAfterFlatSave === true ? '"9"' : flatRevision;
      return await json(route, publicDraft(source), 200, { etag: flatRevision });
    }
    if (path === "/api/navigation/W-1" && method === "GET") {
      return await json(route, { kind: "workspace", workspaceId: workspace });
    }
    if (path === `/api/workspaces/${workspace}` && method === "GET") {
      return await json(route, publicDraft(source), 200, { etag: serverRevision });
    }

    const importId = importIdFrom(path);
    if (importId !== null && method === "PUT") {
      uploadPaths.push(path);
      reportReads = 0;
      if (failNextUpload) {
        failNextUpload = false;
        await route.abort("connectionreset");
        return;
      }
      if (uploadMode === "unsupported") {
        return await qtiJson(
          route,
          {
            importId,
            state: "unsupportedProfile",
            error: "No supported Canvas or Blackboard profile matched this package.",
          },
          422,
        );
      }
      return await qtiJson(route, { importId, state: "queued" }, 202);
    }
    if (importId !== null && method === "GET") {
      reportReads += 1;
      if (uploadMode === "recognized" && reportReads === 1) {
        return await qtiJson(route, { importId, state: "processing" }, 202);
      }
      const report = readyReport(importId, uploadMode === "allRejected", changedNextReport);
      changedNextReport = false;
      return await qtiJson(route, report, 200);
    }
    if (importId !== null && method === "POST" && path.endsWith("/convert-flat")) {
      conversionCount += 1;
      const requestedRevision = request.headers()["if-match"] ?? null;
      conversionRevisions.push(requestedRevision);
      if (requestedRevision !== serverRevision) {
        return await qtiJson(route, { error: "conflict" }, 409);
      }
      if (options.holdConversion === true) {
        await new Promise<void>((resolve) => {
          releasePendingConversion = resolve;
        });
      }
      source = flatSource("Imported metabolism basics");
      flatRevision = '"10"';
      serverRevision = flatRevision;
      return await json(route, publicDraft(source), 200, {
        "cache-control": "no-store",
        etag: flatRevision,
      });
    }
    return await json(route, { error: `Unexpected fixture request: ${method} ${path}` }, 500);
  });

  return {
    uploadPaths,
    conversionRevisions,
    recoveryRequests,
    conversionCount: () => conversionCount,
    flatRefetchPending: () => releasePendingFlatRefetch !== null,
    releaseConversion: (): void => {
      releasePendingConversion?.();
      releasePendingConversion = null;
    },
    releaseFlatRefetch: (): void => {
      releasePendingFlatRefetch?.();
      releasePendingFlatRefetch = null;
    },
    changeNextReport: (): void => {
      changedNextReport = true;
    },
    useUploadMode: (mode): void => {
      uploadMode = mode;
      reportReads = 0;
    },
  };
}

async function openWorkspace(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.goto("/");
  await page.evaluate((path: string) => {
    history.pushState({}, "", path);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, workspacePath);
  await expect(page.getByRole("heading", { name: "Flat question", exact: true })).toBeVisible();
  await expect(page.getByLabel("Question title")).toHaveValue("Original private draft");
}

async function chooseArchive(page: Page, name: string): Promise<void> {
  await page.getByLabel("QTI ZIP archive").setInputFiles({
    name,
    mimeType: "application/zip",
    buffer: Buffer.from([0x50, 0x4b, 0x03, 0x04, 0x00]),
  });
}

function importPanel(page: Page): Locator {
  return page.locator('[data-route-surface="qtiProfileImport"]');
}

function latestRecoveryRequest(harness: QtiRouteHarness): RecordedRouteRequest {
  const request = harness.recoveryRequests[harness.recoveryRequests.length - 1];
  if (request === undefined) throw new Error("Expected a protected flat-source recovery request.");
  return request;
}

async function refreshRecognizedReport(page: Page): Promise<void> {
  const panel = importPanel(page);
  await panel.getByRole("button", { name: "Refresh status" }).click();
  await expect(panel.getByRole("status")).toContainText("server is reviewing");
  await panel.getByRole("button", { name: "Refresh status" }).click();
  await expect(page.getByRole("heading", { name: "QTI import report" })).toBeFocused();
}

test("an indeterminate retry reuses its import identity and real conversion refetches the editor", async ({
  page,
}) => {
  const harness = await installQtiRouteHarness(page, {
    firstUploadIndeterminate: true,
    holdConversion: true,
    holdFlatRefetch: true,
  });
  await openWorkspace(page);
  const panel = importPanel(page);

  await chooseArchive(page, "chapter-one.zip");
  await panel.getByRole("button", { name: "Start import" }).click();
  await expect(panel.getByRole("alert")).toContainText("remains selected so you can retry");
  await panel.getByRole("button", { name: "Retry the same import" }).click();
  await expect(panel.getByRole("status")).toContainText("queued for server-side review");
  expect(harness.uploadPaths).toHaveLength(2);
  expect(harness.uploadPaths[1]).toBe(harness.uploadPaths[0]);

  await refreshRecognizedReport(page);
  await expect(page.getByText("Canvas QTI 1.2 static single choice")).toBeVisible();
  await expect(page.getByText(/allow unlimited attempts/u)).toBeVisible();
  await expect(page.getByText(/use one point/u)).toBeVisible();
  await expect(page.getByText(/vendor metadata field will not be copied/u)).toBeVisible();
  const rejectedItem = page
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { name: "Rejected: Essay follow-up" }) });
  await expect(rejectedItem.getByRole("radio")).toHaveCount(0);

  const renderedReport = `${await page.locator('[data-route-surface="qtiProfileImport"]').innerText()}\n${await page.locator('[data-route-surface="qtiProfileImport"]').innerHTML()}`;
  expect(renderedReport.toLowerCase()).not.toMatch(
    /chosen answer|correctchoice|correct_choice|private checksum|archive checksum|canonical checksum|object key|object id|grading marker|grader bytes|grader payload/u,
  );

  await panel.getByRole("radio", { name: "Select this item for conversion" }).check();
  await panel.getByRole("checkbox", { name: /I reviewed the profile/u }).check();
  const convert = panel.getByRole("button", { name: "Convert selected item" });
  await convert.click();
  await expect(convert).toBeDisabled();
  await convert.click({ force: true });
  expect(harness.conversionCount()).toBe(1);
  harness.releaseConversion();
  await expect.poll(harness.flatRefetchPending).toBe(true);
  const oldEditor = page.locator('[data-route-surface="flatQuestionEditor"]');
  const oldTitle = page.getByLabel("Question title");
  await expect(oldEditor).toHaveAttribute("inert", "");
  await expect(oldEditor).toHaveAttribute("aria-busy", "true");
  await oldTitle.evaluate((input) => input.focus());
  await expect(oldTitle).not.toBeFocused();
  await page.keyboard.type(" forbidden edit");
  await expect(oldTitle).toHaveValue("Original private draft");

  harness.releaseFlatRefetch();
  const importedTitle = page.getByLabel("Question title");
  await expect(importedTitle).toHaveValue("Imported metabolism basics");
  const importedEditor = page.locator('[data-route-surface="flatQuestionEditor"]');
  await expect(importedEditor).not.toHaveAttribute("inert", "");
  await expect(importedTitle).toBeEditable();
  await expect(page.getByRole("heading", { name: "Flat question", exact: true })).toBeFocused();
});

test("all-rejected and unsupported reports keep a clear chooser recovery path", async ({
  page,
}) => {
  const harness = await installQtiRouteHarness(page, { uploadMode: "allRejected" });
  await openWorkspace(page);
  const panel = importPanel(page);

  await chooseArchive(page, "all-rejected.zip");
  await panel.getByRole("button", { name: "Start import" }).click();
  await panel.getByRole("button", { name: "Refresh status" }).click();
  await expect(panel.getByRole("heading", { name: "No items can be converted" })).toBeVisible();
  await expect(panel.getByRole("radio")).toHaveCount(0);
  await expect(panel.getByRole("button", { name: "Convert selected item" })).toHaveCount(0);
  await panel.getByRole("button", { name: "Choose a different archive" }).click();
  await expect(page.getByLabel("QTI ZIP archive")).toBeFocused();

  harness.useUploadMode("unsupported");
  await chooseArchive(page, "unsupported-package.zip");
  await panel.getByRole("button", { name: "Start import" }).click();
  await expect(panel.getByRole("alert")).toContainText("does not match a supported Canvas");
  await expect(panel.getByText("unsupported-package.zip", { exact: false })).toBeVisible();
  await panel.getByRole("button", { name: "Choose a different archive" }).click();
  const chooser = page.getByLabel("QTI ZIP archive");
  await expect(chooser).toBeFocused();
  await chooseArchive(page, "replacement.zip");
  await expect(panel.getByRole("button", { name: "Start import" })).toBeEnabled();
});

test("a committed conversion keeps the stale editor locked until explicit reload succeeds", async ({
  page,
}) => {
  const harness = await installQtiRouteHarness(page, { flatRefetchFailures: 2 });
  await openWorkspace(page);
  const panel = importPanel(page);
  await chooseArchive(page, "converted-recovery.zip");
  await panel.getByRole("button", { name: "Start import" }).click();
  await refreshRecognizedReport(page);
  await panel.getByRole("radio", { name: "Select this item for conversion" }).check();
  await panel.getByRole("checkbox", { name: /I reviewed the profile/u }).check();
  await panel.getByRole("button", { name: "Convert selected item" }).click();

  await expect(
    panel.getByRole("heading", { name: "Converted draft reload required" }),
  ).toBeVisible();
  await expect(panel.getByRole("alert")).toContainText(
    "The previous editor remains locked; use Reload converted draft to try again.",
  );
  const staleEditor = page.locator('[data-route-surface="flatQuestionEditor"]');
  await expect(staleEditor).toHaveAttribute("inert", "");
  await expect(staleEditor).toHaveAttribute("aria-busy", "true");
  await expect(page.getByLabel("Question title")).toHaveValue("Original private draft");
  await expect(panel.getByLabel("QTI ZIP archive")).toHaveCount(0);
  await expect(panel.getByRole("button", { name: "Start import" })).toHaveCount(0);
  await expect(panel.getByRole("button", { name: "Retry the same import" })).toHaveCount(0);
  await expect(panel.getByRole("button", { name: "Choose a different archive" })).toHaveCount(0);
  expect(harness.conversionCount()).toBe(1);

  const requestsAfterAutomaticReload = harness.recoveryRequests.length;
  expect(latestRecoveryRequest(harness)).toEqual({ method: "GET", path: flatSourcePath });
  await panel.getByRole("button", { name: "Reload converted draft" }).click();
  await expect(panel.getByRole("alert")).toContainText(
    "The converted draft still could not load. The previous editor remains locked",
  );
  expect(harness.recoveryRequests.length).toBeGreaterThan(requestsAfterAutomaticReload);
  expect(latestRecoveryRequest(harness)).toEqual({ method: "GET", path: flatSourcePath });
  await expect(staleEditor).toHaveAttribute("inert", "");
  await expect(staleEditor).toHaveAttribute("aria-busy", "true");
  await expect(
    panel.getByRole("heading", { name: "Converted draft reload required" }),
  ).toBeVisible();
  expect(harness.conversionCount()).toBe(1);

  const requestsAfterFailedRecovery = harness.recoveryRequests.length;
  await panel.getByRole("button", { name: "Reload converted draft" }).click();
  await expect
    .poll(() => harness.recoveryRequests.length)
    .toBeGreaterThan(requestsAfterFailedRecovery);
  expect(latestRecoveryRequest(harness)).toEqual({ method: "GET", path: flatSourcePath });
  await expect(page.getByLabel("Question title")).toHaveValue("Imported metabolism basics");
  const importedEditor = page.locator('[data-route-surface="flatQuestionEditor"]');
  await expect(importedEditor).not.toHaveAttribute("inert", "");
  await expect(page.getByLabel("Question title")).toBeEditable();
  await expect(page.getByRole("heading", { name: "Flat question", exact: true })).toBeFocused();
  await expect(panel.getByRole("button", { name: "Reload converted draft" })).toHaveCount(0);
  expect(harness.conversionCount()).toBe(1);
});

test("changed review, dirty edits, and stale displayed revisions refuse replacement on mobile", async ({
  page,
}) => {
  await page.setViewportSize({ width: 375, height: 800 });
  const harness = await installQtiRouteHarness(page, { staleAfterFlatSave: true });
  await openWorkspace(page);
  const panel = importPanel(page);
  await chooseArchive(page, "stale-review.zip");
  await panel.getByRole("button", { name: "Start import" }).click();
  await refreshRecognizedReport(page);

  const accepted = panel.getByRole("radio", { name: "Select this item for conversion" });
  const acknowledged = panel.getByRole("checkbox", { name: /I reviewed the profile/u });
  await accepted.focus();
  await page.keyboard.press("Space");
  await acknowledged.focus();
  await page.keyboard.press("Space");
  harness.changeNextReport();
  const refresh = panel.getByRole("button", { name: "Refresh status" });
  await refresh.focus();
  await page.keyboard.press("Enter");
  await expect(accepted).not.toBeChecked();
  await expect(acknowledged).not.toBeChecked();

  await accepted.focus();
  await page.keyboard.press("Space");
  await acknowledged.focus();
  await page.keyboard.press("Space");
  const title = page.getByLabel("Question title");
  await title.fill("Locally reviewed genetics draft");
  const convert = panel.getByRole("button", { name: "Convert selected item" });
  await expect(convert).toBeDisabled();
  await expect(
    page.getByText(
      "Save or reload the current editor changes before replacing this private draft.",
    ),
  ).toBeVisible();
  await expect(title).toHaveValue("Locally reviewed genetics draft");
  expect(harness.conversionCount()).toBe(0);

  await page.getByRole("button", { name: "Save private draft" }).click();
  const savedStatus = page.getByRole("status", { name: "Private draft status" });
  await expect(savedStatus).toHaveText("Private draft saved. It is not published.");
  await expect(convert).toBeEnabled();
  await convert.focus();
  await page.keyboard.press("Enter");
  await expect(panel.getByRole("alert")).toContainText(
    "Refresh status, review the report again, and then retry conversion.",
  );
  expect(harness.conversionRevisions).toEqual(['"8"']);
  await expect(title).toHaveValue("Locally reviewed genetics draft");
  await expect(page.getByRole("heading", { name: "QTI import report" })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
});
