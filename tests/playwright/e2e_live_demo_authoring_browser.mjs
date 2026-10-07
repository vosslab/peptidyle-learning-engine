// Live Demo acceptance for ordinary unfinished Draft creation, autosave, reopen, and preview/test.
import { chromium } from "playwright";
import { liveDemoChromiumArgs } from "./helper_gateway_trust.mjs";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const validWebworkSource = readFileSync(
  new URL("../../OTHER_REPOS/webwork-pg-renderer/MultipleChoiceRadio.pg", import.meta.url),
  "utf8",
);

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true, args: liveDemoChromiumArgs(origin) });
const context = await browser.newContext();
const page = await context.newPage();

async function createDraft(format, path) {
  await page
    .getByRole("heading", { name: "Create a private Draft Question", exact: true })
    .waitFor();
  await page.getByLabel("Question source format", { exact: true }).selectOption(format);
  if (path !== undefined) {
    await page.getByLabel("Registered WebWork PG path", { exact: true }).fill(path);
  }
  await page.getByRole("button", { name: "Create Draft Question", exact: true }).click();
  await page.waitForURL(
    /\/authoring\/drafts\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u,
  );
}

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page
    .getByRole("button", { name: "Assume the role of Instructor Dr. Elena Rivera" })
    .click();
  await page.waitForURL(`${origin}/library`);
  await page.getByRole("link", { name: "My Draft Questions" }).click();
  await page.waitForURL(`${origin}/authoring/drafts`);

  // A semantically unfinished Native prompt is saved, reopened, and tested through the ordinary
  // structured editor. The source codec rejects a blank required prompt, so retain a partial one.
  await createDraft("pleQuestionJson");
  const prompt = page.getByLabel("Student-facing prompt", { exact: true });
  const unfinishedPrompt = "What explains";
  const previousNativeStatus = await page.evaluate(
    () =>
      document.querySelector('[aria-label="Private draft status"]')?.textContent?.trim() ?? null,
  );
  await prompt.fill(unfinishedPrompt);
  await page.waitForFunction(() =>
    /^(?:Draft changes are waiting to save|Saving private draft)\.\.\.$/u.test(
      [...document.querySelectorAll('[aria-label="Private draft status"]')]
        .map((element) => element.textContent?.trim() ?? "")
        .join(" "),
    ),
  );
  await page.waitForFunction((previous) => {
    const status = document
      .querySelector('[aria-label="Private draft status"]')
      ?.textContent?.trim();
    return status !== previous && status === "Private draft saved. It is not published.";
  }, previousNativeStatus);
  const nativeDraftId = /\/authoring\/drafts\/([0-9a-f-]+)$/u.exec(page.url())?.[1];
  assert.ok(nativeDraftId);
  const nativePreviewResponsePromise = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      response.request().method() === "GET" &&
      url.pathname === `/api/authoring/drafts/${nativeDraftId}/preview`
    );
  });
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.locator('[data-route-surface="pleQuestionJsonEditor"]').waitFor();
  const nativePreviewResponse = await nativePreviewResponsePromise;
  assert.equal(nativePreviewResponse.status(), 200);
  assert.match(nativePreviewResponse.headers()["content-type"] ?? "", /^application\/json/u);
  const savedNativeSource = await page.evaluate(async (path) => {
    const response = await fetch(path, {
      method: "GET",
      credentials: "same-origin",
      cache: "no-store",
      headers: { accept: "*/*" },
    });
    return { status: response.status, etag: response.headers.get("etag") };
  }, `/api/authoring/drafts/${nativeDraftId}/source`);
  assert.equal(savedNativeSource.status, 200);
  assert.match(savedNativeSource.etag ?? "", /^"[1-9][0-9]*"$/u);
  const savedNativeEditNumber = savedNativeSource.etag.slice(1, -1);
  const nativePreviewEditNumber = new URL(nativePreviewResponse.url()).searchParams.get(
    "draftQuestionEditNumber",
  );
  assert.equal(nativePreviewEditNumber, savedNativeEditNumber);
  assert.equal(
    await page.getByLabel("Student-facing prompt", { exact: true }).inputValue(),
    unfinishedPrompt,
  );
  await page
    .getByRole("status", { name: "Private draft status" })
    .getByText("Private draft saved. It is not published.", { exact: true })
    .waitFor();
  await page.getByRole("heading", { name: "Saved Draft preview and test", exact: true }).waitFor();
  await page.getByRole("radio").first().check();
  const nativeTestResponsePromise = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      response.request().method() === "POST" &&
      url.pathname === `/api/authoring/drafts/${nativeDraftId}/test`
    );
  });
  await page.getByRole("button", { name: "Test response", exact: true }).click();
  const nativeTestResponse = await nativeTestResponsePromise;
  assert.equal(nativeTestResponse.status(), 200);
  assert.equal(nativeTestResponse.request().headers()["if-match"], savedNativeSource.etag);
  await page
    .getByText(/(?:Correct|Partially correct|Not correct|did not score this response)/u)
    .waitFor();

  // Incomplete WebWork source is acknowledged and remains editable after reopening even when
  // backend preview/test cannot produce a response from it.
  for (const [format, label] of [
    ["webworkPg", "PG"],
    ["webworkPgml", "PGML"],
  ]) {
    await page.getByRole("link", { name: "My Draft Questions" }).click();
    await page.waitForURL(`${origin}/authoring/drafts`);
    const path = `live_demo/unfinished_${format}_${Date.now()}.pg`;
    await createDraft(format, path);
    const webworkDraftId = /\/authoring\/drafts\/([0-9a-f-]+)$/u.exec(page.url())?.[1];
    assert.ok(webworkDraftId);
    const source = page.getByLabel("Backend source", { exact: true });
    const savedStatus = page.getByText(/^Draft source is saved as Edit [1-9][0-9]*\.$/u);
    const previousStatus = await savedStatus.textContent();
    await source.fill("DOCUMENT(\n");
    await page.waitForFunction((previous) => {
      const status = document.getElementById("draft-source-save-status")?.textContent?.trim();
      return (
        status !== previous && /^Draft source is saved as Edit [1-9][0-9]*\.$/u.test(status ?? "")
      );
    }, previousStatus);
    await page.reload({ waitUntil: "domcontentloaded" });
    await page.getByRole("heading", { name: `${label} source`, exact: true }).waitFor();
    assert.equal(
      await page.getByLabel("Backend source", { exact: true }).inputValue(),
      "DOCUMENT(\n",
    );
    await page
      .getByRole("heading", { name: "Saved Draft preview and test", exact: true })
      .waitFor();
    await page.getByRole("button", { name: "Test this response", exact: true }).click();
    const testFeedback = page.locator(".opaque-webwork-preview-test [role='status']");
    await testFeedback
      .getByText("The Draft preview did not return a test response. Reload it and try again.", {
        exact: true,
      })
      .waitFor();

    // The checked-in renderer sample proves a valid preview and real Draft /test
    // request for each fixed source-format binding, through the visible editor.
    const beforeValidSourceStatus = await savedStatus.textContent();
    const priorEditNumber = beforeValidSourceStatus?.match(
      /^Draft source is saved as Edit ([1-9][0-9]*)\.$/u,
    )?.[1];
    assert.ok(priorEditNumber);
    const expectedValidEditNumber = String(Number(priorEditNumber) + 1);
    const webworkPreviewResponsePromise = page.waitForResponse((response) => {
      const url = new URL(response.url());
      return (
        response.request().method() === "GET" &&
        url.pathname === `/api/authoring/drafts/${webworkDraftId}/preview` &&
        url.searchParams.get("draftTest") === "true" &&
        url.searchParams.get("draftQuestionEditNumber") === expectedValidEditNumber
      );
    });
    await source.fill(validWebworkSource);
    await page.waitForFunction((previous) => {
      const status = document.getElementById("draft-source-save-status")?.textContent?.trim();
      return (
        status !== previous && /^Draft source is saved as Edit [1-9][0-9]*\.$/u.test(status ?? "")
      );
    }, beforeValidSourceStatus);
    const validSourceStatus = await savedStatus.textContent();
    const validSourceEditNumber = validSourceStatus?.match(
      /^Draft source is saved as Edit ([1-9][0-9]*)\.$/u,
    )?.[1];
    assert.ok(validSourceEditNumber);
    assert.equal(validSourceEditNumber, expectedValidEditNumber);
    const webworkPreviewResponse = await webworkPreviewResponsePromise;
    assert.equal(webworkPreviewResponse.status(), 200);
    assert.match(webworkPreviewResponse.headers()["content-type"] ?? "", /^text\/html/u);
    assert.match(await webworkPreviewResponse.text(), /Blue/u);

    const preview = page.frameLocator(`iframe[title="${label} Draft preview"]`);
    await preview.getByRole("radio", { name: "Blue", exact: true }).waitFor();
    await preview.getByRole("radio", { name: "Blue", exact: true }).check();
    const draftTestResponse = page.waitForResponse(
      (response) =>
        response.request().method() === "POST" &&
        new RegExp(`/api/authoring/drafts/[0-9a-f-]+/test$`, "u").test(response.url()),
    );
    await page.getByRole("button", { name: "Test this response", exact: true }).click();
    const response = await draftTestResponse;
    assert.equal(response.status(), 200);
    assert.equal(response.request().headers()["if-match"], `"${validSourceEditNumber}"`);
    assert.deepEqual(await response.json(), {
      kind: "evaluated",
      correct: true,
      normalizedCredit: 1,
    });
    await page
      .getByRole("status")
      .getByText("Correct. The Question Backend awarded full credit.", { exact: true })
      .waitFor();
  }
} finally {
  await context.close();
  await browser.close();
}
