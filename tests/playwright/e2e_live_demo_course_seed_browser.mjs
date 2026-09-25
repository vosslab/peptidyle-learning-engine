// Production-browser proof for the fixed realistic teaching course baseline.

import { readFileSync } from "node:fs";

import { expect } from "@playwright/test";
import { chromium } from "playwright";
import { liveDemoChromiumArgs } from "./helper_gateway_trust.mjs";

import { chooseSeededIdentity, selectVisibleCourse, signOutVisible } from "./e2e/real_stack_ui.ts";

const workspace = "local_stack_state/live_demo_browser/workspace";
const environment = readFileSync(workspace + "/env.local", "ascii");
const portEntry = environment
  .split(String.fromCharCode(10))
  .find((line) => line.startsWith("PLE_GATEWAY_HOST_PORT="));
const port = portEntry?.slice("PLE_GATEWAY_HOST_PORT=".length);
if (port === undefined || !/^[0-9]+$/u.test(port)) {
  throw new Error("fixed Live Demo gateway port is unavailable");
}

const origin = "https://localhost:" + port;
const courseLongName = "Biochemistry 301: Proteins and Peptides";
const assessmentTitle = "Chapter 1 Pilot Practice";
const assessmentTypeLabel = "Regular Assignment";

async function discoverAssessmentId(page) {
  const href = await instructorAssessmentRow(page)
    .getByRole("link", { name: "Edit Assessment", exact: true })
    .getAttribute("href");
  const match = href?.match(/\/assessments\/(A[0-9A-HJKMNP-TV-Z]{8})\/questions$/u);
  if (match?.[1] === undefined) throw new Error("Live Demo Assessment lacks a canonical public ID");
  return match[1];
}

function instructorAssessmentRow(page) {
  return page
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { name: assessmentTitle, exact: true }) });
}

function studentAssessmentCard(page) {
  return page
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { name: assessmentTitle, exact: true }) });
}

async function enterSeededStudentCourse(page) {
  const courseHeading = page.getByRole("heading", {
    level: 1,
    name: courseLongName,
    exact: true,
  });
  if (await courseHeading.isVisible()) return;
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true })
    .click();
  await page.getByRole("heading", { name: "Your courses", exact: true }).waitFor();
  const courseCard = page
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { name: courseLongName, exact: true }) });
  await expect(courseCard).toHaveCount(1);
  await courseCard.getByRole("link", { name: "Open Course", exact: true }).click();
  await expect(courseHeading).toBeVisible();
}

async function openAssessment(page) {
  const card = studentAssessmentCard(page);
  await expect(card).toHaveCount(1);
  await card.getByRole("link", { name: `Open ${assessmentTypeLabel}`, exact: true }).click();
}

async function expectRosterRow(page, rosterId) {
  const row = page.getByRole("row").filter({ has: page.getByText(rosterId, { exact: true }) });
  await expect(row).toHaveCount(1);
  await expect(row.getByText("Active Student", { exact: true })).toBeVisible();
}

async function expectGradebookRow(page, rosterName, rosterId, progress, score, assessmentId) {
  const evidence = page.getByRole("region", { name: "Student progress and scores" });
  const records = evidence.getByRole("list", { name: "Student progress and scores" });
  await expect(records).toHaveCount(1);
  const record = records
    .getByRole("listitem")
    .filter({ has: page.getByText(rosterId, { exact: true }) });
  await expect(record).toHaveCount(1);
  await expect(record.getByText(rosterName, { exact: true })).toBeVisible();
  await expect(record.getByText(rosterId, { exact: true })).toBeVisible();
  await expect(record.getByText(assessmentTitle, { exact: true })).toBeVisible();
  await expect(record.getByText(progress, { exact: true })).toBeVisible();
  await expect(record.getByText(score, { exact: true })).toBeVisible();
  await expect(record.getByText(assessmentId, { exact: true })).toHaveCount(0);
}

async function expectAttempt(page) {
  const attempt = page.locator('[data-route-surface="assessmentAttempt"]');
  await expect(attempt).toBeVisible();
  await expect(attempt.getByRole("heading", { level: 1, name: assessmentTitle })).toBeVisible();
  const navigation = attempt.getByRole("navigation", { name: "Assessment questions", exact: true });
  await expect(navigation.getByRole("list").getByRole("button")).toHaveCount(4);
  await expect(navigation.locator('[aria-current="step"]')).toHaveCount(1);
  await expect(attempt.locator("article.question-card")).toHaveCount(1);
  await expect(attempt.getByRole("heading", { name: "Finish Attempt", exact: true })).toBeVisible();
}

async function resumeAttempt(page) {
  await studentAssessmentCard(page)
    .getByRole("link", { name: `Resume ${assessmentTypeLabel}`, exact: true })
    .click();
  await expectAttempt(page);
}

async function startAttempt(page) {
  await openAssessment(page);
  await expect(page.locator('[data-route-surface="assessmentOverview"]')).toBeVisible();
  await page.getByRole("button", { name: `Start ${assessmentTypeLabel}`, exact: true }).click();
  await expectAttempt(page);
}

async function verifyElena(page) {
  await chooseSeededIdentity(page, /Elena Rivera/u);
  await selectVisibleCourse(page, courseLongName);
  const card = instructorAssessmentRow(page);
  await expect(card).toHaveCount(1);
  await expect(card.getByText(/Assessment 1 - Released/u)).toBeVisible();
  const assessmentId = await discoverAssessmentId(page);
  await card.getByRole("link", { name: "Edit Assessment", exact: true }).click();
  await expect(page.locator('[data-route-surface="assessmentWorkspace"]')).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Assessment Question Editor", exact: true }),
  ).toBeVisible();
  await expect(page.getByLabel("Assessment title")).toHaveValue(assessmentTitle);
  await page.goBack();
  await page.getByRole("link", { name: "Open Students", exact: true }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: "Students", exact: true }),
  ).toBeVisible();
  await expectRosterRow(page, "BIO301-MARY");
  await expectRosterRow(page, "BIO301-JACK");
  await expectRosterRow(page, "BIO301-AVERY");

  await page.getByRole("link", { name: "Gradebook", exact: true }).click();
  await expect(page.locator('[data-route-surface="gradebook"]')).toBeVisible();
  await expect(
    page.getByRole("heading", { level: 1, name: "Gradebook", exact: true }),
  ).toBeVisible();
  await expectGradebookRow(
    page,
    "Mary Okafor",
    "BIO301-MARY",
    "Completed and scored",
    "3 / 4",
    assessmentId,
  );
  await expectGradebookRow(page, "Jack Nguyen", "BIO301-JACK", "In progress", "-", assessmentId);
  await expectGradebookRow(
    page,
    "Avery Thompson",
    "BIO301-AVERY",
    "Not started",
    "-",
    assessmentId,
  );
  await signOutVisible(page);
}

async function verifyMary(page) {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await enterSeededStudentCourse(page);
  const card = studentAssessmentCard(page);
  await expect(card).toHaveCount(1);
  await expect(card.getByText("Completed", { exact: true }).first()).toBeVisible();
  await expect(
    card.getByText("4 of 4 questions graded · Assessment score 3 / 4", { exact: true }),
  ).toBeVisible();
  await signOutVisible(page);
}

async function verifyJack(page) {
  await chooseSeededIdentity(page, /Jack Nguyen/u);
  await enterSeededStudentCourse(page);
  const card = studentAssessmentCard(page);
  await expect(card).toHaveCount(1);
  await expect(card.getByText("In progress", { exact: true }).first()).toBeVisible();
  await expect(card.getByText("2 of 4 responses saved", { exact: true })).toBeVisible();
  await expect(card.getByText(/questions graded/u)).toHaveCount(0);
  await expect(card.getByText(/Score/u)).toHaveCount(0);
  await resumeAttempt(page);
  await signOutVisible(page);
}

async function verifyAvery(page) {
  await chooseSeededIdentity(page, /Avery Thompson/u);
  await enterSeededStudentCourse(page);
  const card = studentAssessmentCard(page);
  await expect(card).toHaveCount(1);
  await expect(card.getByText("Available", { exact: true }).first()).toBeVisible();
  await expect(card.getByText("Not started", { exact: true })).toBeVisible();
  await startAttempt(page);
  await signOutVisible(page);
}

const browser = await chromium.launch({ headless: true, args: liveDemoChromiumArgs(origin) });
const context = await browser.newContext({
  baseURL: origin,
});
const page = await context.newPage();

try {
  await verifyElena(page);
  await verifyMary(page);
  await verifyJack(page);
  await verifyAvery(page);
} finally {
  await context.close();
  await browser.close();
}
