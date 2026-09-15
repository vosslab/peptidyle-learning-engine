// scenarios_sysadmin.ts - Sysadmin Courses and Account lifecycle states.
// Selector contract: role navigation is shared through visible_workflows.ts:80; Sysadmin surfaces
// are owned by src/pages/course_list_page.tsx:149, src/pages/instructor_accounts_page.tsx:126,
// and src/pages/instructor_accounts_page.tsx:126.

import type { Locator, Page } from "playwright";

import type { CaptureSession, ScenarioRuntime } from "./runtime";
import type { ScenarioDefinition } from "./scenario_types";
import { enterSysadmin, scrollTop } from "./visible_workflows";

const CREATED_EMAIL = "screenshot.instructor@live-demo.invalid";

async function captureCheckpoint(
  runtime: ScenarioRuntime,
  scenario: string,
  checkpoint: string,
  session: CaptureSession,
): Promise<void> {
  await scrollTop(session.page);
  await runtime.capture(session, runtime.record(scenario, checkpoint));
}

async function sysadminCourses(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "sysadmin_courses";
  const session = await runtime.open(runtime.record(scenario, "course_list"));
  try {
    await enterSysadmin(session.page);
    await captureCheckpoint(runtime, scenario, "course_list", session);
  } finally {
    await runtime.close(session);
  }
}

async function openInstructorAccounts(page: Page): Promise<void> {
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Instructor Accounts", exact: true })
    .click();
  await page.getByRole("heading", { name: "Instructor Accounts", exact: true }).waitFor();
  await page
    .getByText("Loading Instructor Accounts...", { exact: true })
    .waitFor({ state: "hidden" });
}

function instructorAccount(page: Page, reference: string): Locator {
  return page
    .locator('section[aria-label="Instructor Accounts"] > .auth-panel')
    .filter({ has: page.getByRole("heading", { name: reference, exact: true }) });
}

async function reloadInstructorAccounts(page: Page): Promise<void> {
  await page.reload({ waitUntil: "commit" });
  await page.getByRole("heading", { name: "Instructor Accounts", exact: true }).waitFor();
  await page
    .getByText("Loading Instructor Accounts...", { exact: true })
    .waitFor({ state: "hidden" });
}

async function sysadminAccounts(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "sysadmin_accounts";
  const session = await runtime.open(runtime.record(scenario, "accounts_initial"));
  const page = session.page;
  try {
    await enterSysadmin(page);
    await openInstructorAccounts(page);
    await captureCheckpoint(runtime, scenario, "accounts_initial", session);
    await page.getByLabel("Instructor Authentication Email").fill("xx");
    await page.getByRole("button", { name: "Create Instructor Account", exact: true }).click();
    await page
      .getByText("Enter a normalized Instructor Authentication Email within its allowed length.", {
        exact: true,
      })
      .waitFor();
    await page.getByLabel("Instructor Authentication Email").fill("");
    await captureCheckpoint(runtime, scenario, "account_validation", session);
    await page.getByLabel("Instructor Authentication Email").fill(CREATED_EMAIL);
    await page.getByRole("button", { name: "Create Instructor Account", exact: true }).click();
    await page.getByText("Instructor Account created.", { exact: true }).waitFor();
    const newest = page.locator('section[aria-label="Instructor Accounts"] > .auth-panel').first();
    const reference = await newest.locator("h2").textContent();
    if (reference === null || !/^U-[1-9][0-9]{0,9}$/u.test(reference)) {
      throw new Error("created Instructor Account lacks a canonical public reference");
    }
    await reloadInstructorAccounts(page);
    let created = instructorAccount(page, reference);
    await created.waitFor();
    await captureCheckpoint(runtime, scenario, "account_created", session);
    await created.getByLabel("Deactivation reason").fill("Screenshot corpus lifecycle review");
    await created
      .getByRole("button", { name: "Deactivate Instructor Account", exact: true })
      .click();
    await created.getByText("State: Deactivated", { exact: true }).waitFor();
    await reloadInstructorAccounts(page);
    created = instructorAccount(page, reference);
    await created.getByText("State: Deactivated", { exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "account_deactivated", session);
    await created
      .getByRole("button", { name: "Reactivate Instructor Account", exact: true })
      .click();
    await created.getByText("State: Active", { exact: true }).waitFor();
    await reloadInstructorAccounts(page);
    created = instructorAccount(page, reference);
    await created.getByText("State: Active", { exact: true }).waitFor();
  } finally {
    await runtime.close(session);
  }
}

export const SYSADMIN_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  { id: "sysadmin_courses", checkpoints: ["course_list"], run: sysadminCourses },
  {
    id: "sysadmin_accounts",
    checkpoints: [
      "accounts_initial",
      "account_validation",
      "account_created",
      "account_deactivated",
    ],
    run: sysadminAccounts,
  },
];
