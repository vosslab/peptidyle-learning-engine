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
const assessmentTypeLabel = "Unit Review Assignment";

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
  if (await courseHeading.isVisible()) return page;
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true })
    .click();
  await page.getByRole("heading", { name: "Your courses", exact: true }).waitFor();
  return selectVisibleCourse(page, courseLongName);
}

async function openAssessment(page) {
  const card = studentAssessmentCard(page);
  await expect(card).toHaveCount(1);
  const openedAssessment = page.waitForEvent("popup");
  await card.getByRole("link", { name: `Open ${assessmentTypeLabel}`, exact: true }).click();
  return await openedAssessment;
}

async function expectRosterRow(page, rosterId) {
  const row = page.getByRole("row").filter({ has: page.getByText(rosterId, { exact: true }) });
  await expect(row).toHaveCount(1);
  await expect(row.getByText("Active Student", { exact: true })).toBeVisible();
}

async function expectGradebookRow(page, rosterName, rosterId, progress, score, assessmentId) {
  const records = page.getByRole("table", { name: "Student progress and scores", exact: true });
  await expect(records).toHaveCount(1);
  const record = records
    .getByRole("row")
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
  const navigation = attempt.getByRole("navigation", { name: "Coursework questions", exact: true });
  await expect(navigation.getByRole("list").getByRole("button")).toHaveCount(4);
  await expect(navigation.locator('[aria-current="step"]')).toHaveCount(1);
  await expect(attempt.locator("article.question-card")).toHaveCount(1);
  await expect(attempt.getByRole("heading", { name: "Finish Attempt", exact: true })).toBeVisible();
}

async function resumeAttempt(page) {
  const openedAttempt = page.waitForEvent("popup");
  await studentAssessmentCard(page)
    .getByRole("link", { name: `Resume ${assessmentTypeLabel}`, exact: true })
    .click();
  const attemptPage = await openedAttempt;
  await expectAttempt(attemptPage);
  return attemptPage;
}

async function startAttempt(page) {
  const assessmentPage = await openAssessment(page);
  await expect(assessmentPage.locator('[data-route-surface="assessmentOverview"]')).toBeVisible();
  await assessmentPage
    .getByRole("button", { name: `Start ${assessmentTypeLabel}`, exact: true })
    .click();
  await expectAttempt(assessmentPage);
  return assessmentPage;
}

async function verifyElena(page) {
  await chooseSeededIdentity(page, /Elena Rivera/u);
  const coursePage = await selectVisibleCourse(page, courseLongName);
  const card = instructorAssessmentRow(coursePage);
  await expect(card).toHaveCount(1);
  await expect(card.getByText("Assessment 1", { exact: true })).toBeVisible();
  await expect(card.getByText("Released", { exact: true })).toBeVisible();
  const assessmentId = await discoverAssessmentId(coursePage);
  const openedAssessment = coursePage.waitForEvent("popup");
  await card.getByRole("link", { name: "Edit Assessment", exact: true }).click();
  const assessmentPage = await openedAssessment;
  await expect(
    assessmentPage.locator('.page-frame[data-route-surface="assessmentWorkspace"]'),
  ).toBeVisible();
  await expect(
    assessmentPage.getByRole("heading", { name: "Assessment Question Editor", exact: true }),
  ).toBeVisible();
  await expect(assessmentPage.getByLabel("Assessment title")).toHaveValue(assessmentTitle);
  await assessmentPage.close();
  await coursePage.getByRole("link", { name: "Open Students", exact: true }).click();
  await expect(
    coursePage.getByRole("heading", { level: 1, name: "Students", exact: true }),
  ).toBeVisible();
  await expectRosterRow(coursePage, "BIO301-MARY");
  await expectRosterRow(coursePage, "BIO301-JACK");
  await expectRosterRow(coursePage, "BIO301-AVERY");

  await coursePage.goBack();
  await coursePage.getByRole("link", { name: "Gradebook", exact: true }).click();
  await expect(coursePage.locator('[data-route-surface="gradebook"]')).toBeVisible();
  await expect(
    coursePage.getByRole("heading", { level: 1, name: "Gradebook", exact: true }),
  ).toBeVisible();
  await expectGradebookRow(
    coursePage,
    "Mary",
    "BIO301-MARY",
    "Completed and scored",
    "3 / 4",
    assessmentId,
  );
  await expectGradebookRow(coursePage, "Jack", "BIO301-JACK", "In progress", "-", assessmentId);
  await expectGradebookRow(coursePage, "Avery", "BIO301-AVERY", "Not started", "-", assessmentId);
  await signOutVisible(coursePage);
}

async function verifyMary(page) {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  const coursePage = await enterSeededStudentCourse(page);
  const card = studentAssessmentCard(coursePage);
  await expect(card).toHaveCount(1);
  await expect(card.getByText("Completed", { exact: true }).first()).toBeVisible();
  await signOutVisible(coursePage);
}

async function verifyJack(page) {
  await chooseSeededIdentity(page, /Jack Nguyen/u);
  const coursePage = await enterSeededStudentCourse(page);
  const card = studentAssessmentCard(coursePage);
  await expect(card).toHaveCount(1);
  await expect(card.getByText("In progress", { exact: true }).first()).toBeVisible();
  await expect(card.getByText(/questions graded/u)).toHaveCount(0);
  await expect(card.getByText(/Score/u)).toHaveCount(0);
  const attemptPage = await resumeAttempt(coursePage);
  await signOutVisible(attemptPage);
}

async function verifyAvery(page) {
  await chooseSeededIdentity(page, /Avery Thompson/u);
  const coursePage = await enterSeededStudentCourse(page);
  const card = studentAssessmentCard(coursePage);
  await expect(card).toHaveCount(1);
  await expect(card.getByText("Available", { exact: true }).first()).toBeVisible();
  await expect(card.getByText("Not started", { exact: true })).toBeVisible();
  const attemptPage = await startAttempt(coursePage);
  await signOutVisible(attemptPage);
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
