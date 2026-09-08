// Production-browser proof for the fixed realistic teaching course baseline.

import { readFileSync } from "node:fs";

import { expect } from "@playwright/test";
import { chromium } from "playwright";

import {
  chooseSeededIdentity,
  courseChoice,
  selectVisibleCourse,
  signOutVisible,
} from "./e2e/real_stack_ui.ts";

const workspace = "local_stack_state/live_demo_browser/workspace";
const environment = readFileSync(workspace + "/env.local", "ascii");
const portEntry = environment
  .split(String.fromCharCode(10))
  .find((line) => line.startsWith("PLE_GATEWAY_HOST_PORT="));
const port = portEntry?.slice("PLE_GATEWAY_HOST_PORT=".length);
if (port === undefined || !/^[0-9]+$/u.test(port)) {
  throw new Error("fixed Live Demo gateway port is unavailable");
}

const report = JSON.parse(readFileSync(workspace + "/live_demo_course_report.json", "ascii"));
const course = report.course_reference;
const assignment = report.assignment_reference;
if (!/^C-[1-9][0-9]{0,9}$/u.test(course) || !/^A-[1-9][0-9]{0,9}$/u.test(assignment)) {
  throw new Error("fixed Live Demo public references are unavailable");
}
const origin = "https://localhost:" + port;
const courseTitle = "Biochemistry 301: Proteins and Peptides";
const assignmentTitle = "Peptide Structure Practice";

async function openStudentCourse(page) {
  const choice = courseChoice(page, courseTitle);
  await expect(choice).toHaveCount(1);
  await choice.getByRole("link", { name: "Open assigned work", exact: true }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: courseTitle, exact: true }),
  ).toBeVisible();
}

function assignmentCard(page) {
  return page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
}

async function openAssignment(page) {
  const card = assignmentCard(page);
  await expect(card).toHaveCount(1);
  await card.getByRole("link", { name: "Open Assignment", exact: true }).click();
  await expect(page.locator('[data-route-surface="assignmentOverview"]')).toBeVisible();
}

async function expectRosterRow(page, rosterId, email) {
  const row = page.getByRole("row").filter({ has: page.getByText(rosterId, { exact: true }) });
  await expect(row).toHaveCount(1);
  await expect(row.getByText(email, { exact: true })).toBeVisible();
  await expect(row.getByText("Active Student", { exact: true })).toBeVisible();
}

async function expectGradebookRow(page, rosterId, progress, graded, score) {
  const row = page.getByRole("row").filter({ has: page.getByText(rosterId, { exact: true }) });
  await expect(row).toHaveCount(1);
  await expect(row.getByText(assignment, { exact: true })).toBeVisible();
  await expect(row.getByText(progress, { exact: true })).toBeVisible();
  await expect(row.getByText(graded, { exact: true })).toBeVisible();
  await expect(row.getByText(score, { exact: true })).toBeVisible();
}

async function startAttempt(page, expectedResumed) {
  await page.getByRole("button", { name: "Start Assignment", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: assignmentTitle })).toBeVisible();
  await expect(
    page.getByRole("heading", { level: 2, name: "Questions", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".question-presentation")).toHaveCount(4);
  const reopened = page.getByText("Your current Assignment Attempt has been reopened.", {
    exact: true,
  });
  await expect(reopened).toHaveCount(expectedResumed ? 1 : 0);
}

async function verifyElena(page) {
  await chooseSeededIdentity(page, /Elena Rivera/u);
  await selectVisibleCourse(page, courseTitle);
  const card = assignmentCard(page);
  await expect(card).toHaveCount(1);
  await expect(card.getByText("Released Assignment", { exact: true })).toBeVisible();
  await card.getByRole("link", { name: "Open Assignment Workspace", exact: true }).click();
  await expect(page.locator('[data-route-surface="assignmentReleaseWorkspace"]')).toBeVisible();
  await expect(page.getByLabel("Assignment title")).toHaveValue(assignmentTitle);
  await page.goBack();
  await page.getByRole("link", { name: "Open Students", exact: true }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: "Students", exact: true }),
  ).toBeVisible();
  await expectRosterRow(page, "BIO301-MARY", "mary.okafor@live-demo.invalid");
  await expectRosterRow(page, "BIO301-JACK", "jack.nguyen@live-demo.invalid");
  await expectRosterRow(page, "BIO301-AVERY", "avery.thompson@live-demo.invalid");

  await page.getByRole("link", { name: "Gradebook", exact: true }).click();
  await expect(page.locator('[data-route-surface="gradebook"]')).toBeVisible();
  await expect(
    page.getByRole("heading", { level: 1, name: "Gradebook", exact: true }),
  ).toBeVisible();
  await expectGradebookRow(page, "BIO301-MARY", "Completed and scored", "4 of 4", "2 / 4");
  await expectGradebookRow(page, "BIO301-JACK", "In progress", "2 of 4", "1 / 2");
  await expectGradebookRow(page, "BIO301-AVERY", "Not started", "0 of 4", "—");
  await signOutVisible(page);
}

async function verifyMary(page) {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await openStudentCourse(page);
  const card = assignmentCard(page);
  await expect(card).toHaveCount(1);
  await expect(card.getByText("Completed and scored", { exact: true })).toBeVisible();
  await expect(
    card.getByText("4 of 4 Questions graded · Score 2 / 4", { exact: true }),
  ).toBeVisible();
  await signOutVisible(page);
}

async function verifyJack(page) {
  await chooseSeededIdentity(page, /Jack Nguyen/u);
  await openStudentCourse(page);
  const card = assignmentCard(page);
  await expect(card).toHaveCount(1);
  await expect(card.getByText("In progress", { exact: true })).toBeVisible();
  await expect(
    card.getByText("2 of 4 Questions graded · Score so far 1 / 2", { exact: true }),
  ).toBeVisible();
  await openAssignment(page);
  await startAttempt(page, true);
  await signOutVisible(page);
}

async function verifyAvery(page) {
  await chooseSeededIdentity(page, /Avery Thompson/u);
  await openStudentCourse(page);
  const card = assignmentCard(page);
  await expect(card).toHaveCount(1);
  await expect(card.getByText("Not started", { exact: true })).toBeVisible();
  await openAssignment(page);
  await startAttempt(page, false);
  await signOutVisible(page);
}

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({
  baseURL: origin,
  ignoreHTTPSErrors: true,
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
