// Production-browser proof for private Draft Question authoring and publication.
// Classification labels: src/components/content_classification_select.tsx:27.
// Publication controls: src/features/ple_question_json_authoring/question_json_editor_page.tsx:739.

import { chromium } from "playwright";
import assert from "node:assert/strict";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const questionTitle = `Browser publication path ${Date.now()}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

async function classificationUuid(path, key, name) {
  const response = await context.request.get(`${origin}${path}`);
  assert.equal(response.status(), 200, `Authoring classification selector ${key} failed`);
  const payload = await response.json();
  assert.ok(Array.isArray(payload[key]), `Classification selector ${key} did not return a list`);
  const matches = payload[key].filter((item) => item?.name === name);
  assert.equal(
    matches.length,
    1,
    `Authoring requires exactly one installed ${key} fixture named ${name}`,
  );
  const uuid = matches[0].uuid;
  // ASVS 2.2.1: selected installed identities must be canonical lowercase UUIDs.
  assert.match(uuid, /^[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$/u);
  return uuid;
}

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page
    .getByRole("button", { name: "Assume the role of Instructor Dr. Elena Rivera" })
    .click();
  await page.waitForURL(`${origin}/library`);
  const disciplineUuid = await classificationUuid(
    "/api/content-classification/disciplines",
    "disciplines",
    "Biology",
  );
  const subjectUuid = await classificationUuid(
    `/api/content-classification/subjects?disciplineUuid=${disciplineUuid}`,
    "subjects",
    "Genetics",
  );
  await page.getByRole("link", { name: "My Draft Questions" }).click();
  await page.waitForURL(`${origin}/authoring/drafts`);
  await page.getByRole("button", { name: "New Draft Question" }).click();
  await page.waitForURL(/\/authoring\/drafts\/D-[1-9][0-9]*$/u);
  await page.getByLabel("Question Title").fill(questionTitle);
  await page.getByLabel("Question License").selectOption("CC-BY-4.0");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await page.getByText("Private draft saved. It is not published.").waitFor();
  await page.getByRole("button", { name: "Review publication changes" }).click();
  const discipline = page.getByLabel("Discipline (required)", { exact: true });
  await discipline.locator(`option[value="${disciplineUuid}"]`).waitFor({ state: "attached" });
  await discipline.selectOption(disciplineUuid);
  const subject = page.getByLabel("Subject (required)", { exact: true });
  await subject.locator(`option[value="${subjectUuid}"]`).waitFor({ state: "attached" });
  await subject.selectOption(subjectUuid);
  await page.getByLabel("Question Authors").fill("Live Demo Instructor");
  await page.getByRole("button", { name: "Confirm and publish" }).click();
  await page.getByRole("heading", { name: "Published" }).waitFor();
  await page.getByRole("link", { name: "Open question library" }).click();
  await page.waitForURL(`${origin}/library`);
  await page.getByRole("heading", { name: questionTitle }).waitFor();
  await page
    .locator("article.question-library-row", {
      has: page.getByRole("heading", { name: questionTitle }),
    })
    .getByRole("link", { name: "Open question" })
    .click();
  await page.waitForURL(/\/library\/[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$/u);
  await page.getByRole("heading", { name: questionTitle }).waitFor();
} finally {
  await context.close();
  await browser.close();
}
