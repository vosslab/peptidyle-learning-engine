// scenarios_sysadmin.ts - Sysadmin home and Instructor Account lifecycle states.
// Selector contract: role navigation is shared through visible_workflows.ts; Sysadmin surfaces are
// owned by src/pages/role_home_pages.tsx and src/pages/instructor_accounts_page.tsx.

import type { Locator, Page } from "playwright";

import { isCanonicalAccountId } from "../../../src/api/decoders/instructor_account";
import type { CaptureSession, ScenarioRuntime } from "./runtime";
import type { ScenarioDefinition } from "./scenario_types";
import { enterSysadmin, scrollTop } from "./visible_workflows";

// Unique per run so replays on the same stack can create a fresh Account.
const CREATED_EMAIL = `screenshot.instructor.${String(Date.now())}@live-demo.invalid`;
const VERIFIED_INSTRUCTOR_DISPLAY_NAME = "Screenshot Instructor";

async function captureCheckpoint(
  runtime: ScenarioRuntime,
  checkpoint: string,
  session: CaptureSession,
): Promise<void> {
  await scrollTop(session.page);
  await runtime.captureCheckpoint(session, checkpoint);
}

async function sysadminCourses(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("course_list");
  try {
    await enterSysadmin(session.page);
    await captureCheckpoint(runtime, "course_list", session);
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

function instructorAccount(page: Page, accountId: string): Locator {
  return page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: accountId, exact: true }) });
}

async function reloadInstructorAccounts(page: Page): Promise<void> {
  await page.reload({ waitUntil: "commit" });
  await page.getByRole("heading", { name: "Instructor Accounts", exact: true }).waitFor();
  await page
    .getByText("Loading Instructor Accounts...", { exact: true })
    .waitFor({ state: "hidden" });
}

async function sysadminAccounts(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("accounts_initial");
  const page = session.page;
  try {
    await enterSysadmin(page);
    await openInstructorAccounts(page);
    await captureCheckpoint(runtime, "accounts_initial", session);
    await page.getByLabel("Instructor Authentication Email").fill("xx");
    await page.getByRole("button", { name: "Create Instructor Account", exact: true }).click();
    await page
      .getByText("Enter a normalized Instructor Authentication Email within its allowed length.", {
        exact: true,
      })
      .waitFor();
    await page.getByLabel("Instructor Authentication Email").fill("");
    await captureCheckpoint(runtime, "account_validation", session);
    await page.getByLabel("Instructor Authentication Email").fill(CREATED_EMAIL);
    await page
      .getByLabel("Verified Instructor Display Name")
      .fill(VERIFIED_INSTRUCTOR_DISPLAY_NAME);
    await page.getByRole("button", { name: "Create Instructor Account", exact: true }).click();
    await page.getByText("Instructor Account created.", { exact: true }).waitFor();
    const newest = page.getByRole("article").first();
    const accountId = (await newest.getByRole("heading", { level: 2 }).innerText()).trim();
    if (!isCanonicalAccountId(accountId)) {
      throw new Error("created Instructor Account lacks a canonical public ID");
    }
    await reloadInstructorAccounts(page);
    let created = instructorAccount(page, accountId);
    await created.waitFor();
    await captureCheckpoint(runtime, "account_created", session);
    await created.getByLabel("Deactivation reason").fill("Screenshot corpus lifecycle review");
    await created
      .getByRole("button", { name: "Deactivate Instructor Account", exact: true })
      .click();
    await created.getByText("State: Deactivated", { exact: true }).waitFor();
    await reloadInstructorAccounts(page);
    created = instructorAccount(page, accountId);
    await created.getByText("State: Deactivated", { exact: true }).waitFor();
    await captureCheckpoint(runtime, "account_deactivated", session);
    await created
      .getByRole("button", { name: "Reactivate Instructor Account", exact: true })
      .click();
    await created.getByText("State: Active", { exact: true }).waitFor();
    await reloadInstructorAccounts(page);
    created = instructorAccount(page, accountId);
    await created.getByText("State: Active", { exact: true }).waitFor();
  } finally {
    await runtime.close(session);
  }
}

export const SYSADMIN_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "sysadmin_courses",
    role: "sysadmin",
    captures: [
      {
        checkpoint: "course_list",
        area: "system administration",
        workflow: "system administration",
        state: "system administration home",
        viewport: "laptop",
        privacyProfile: "sysadmin_account",
        caption: "System administration home",
        featured: true,
      },
    ],
    run: sysadminCourses,
  },
  {
    id: "sysadmin_accounts",
    role: "sysadmin",
    captures: [
      {
        checkpoint: "accounts_initial",
        area: "accounts",
        workflow: "instructor account lifecycle",
        state: "initial",
        viewport: "laptop",
        privacyProfile: "sysadmin_account",
        caption: "Instructor Accounts",
      },
      {
        checkpoint: "account_created",
        area: "accounts",
        workflow: "instructor account lifecycle",
        state: "created",
        viewport: "laptop",
        privacyProfile: "sysadmin_account",
        caption: "Created Instructor Account",
      },
      {
        checkpoint: "account_deactivated",
        area: "accounts",
        workflow: "instructor account lifecycle",
        state: "deactivated",
        viewport: "laptop",
        privacyProfile: "sysadmin_account",
        caption: "Deactivated Instructor Account",
      },
      {
        checkpoint: "account_validation",
        area: "accounts",
        workflow: "instructor account lifecycle",
        state: "creation validation",
        viewport: "laptop",
        privacyProfile: "sysadmin_account",
        caption: "Instructor Account creation validation",
      },
    ],
    run: sysadminAccounts,
  },
];
