// scenarios_sysadmin.ts - Sysadmin Courses, Account lifecycle, and scoped support states.
// Selector contract: role navigation is shared through visible_workflows.ts:80; Sysadmin surfaces
// are owned by src/pages/course_list_page.tsx:149, src/pages/instructor_accounts_page.tsx:126,
// and src/pages/support_roster_page.tsx:32.

import type { Locator, Page } from "playwright";

import type { CaptureSession, ScenarioRuntime } from "./runtime";
import type { ScenarioDefinition } from "./scenario_types";
import { enterInstructor, enterSysadmin, scrollTop } from "./visible_workflows";

const CREATED_EMAIL = "screenshot.instructor@live-demo.invalid";
const SEEDED_SYSADMIN_REFERENCE = "U-5";

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

function supportCapabilityId(
  value: unknown,
  expectedCourseReference: string,
  expectedSysadminReference: string,
): string {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("support capability receipt must be an object");
  }
  const receipt = value as Record<string, unknown>;
  const capabilityId = receipt["capabilityId"];
  const expectedFields = [
    "capabilityId",
    "courseReference",
    "expiresAt",
    "minimumProjection",
    "operationKind",
    "purpose",
    "revokedAt",
    "sysadminReference",
  ];
  if (
    Object.keys(receipt).sort().join(",") !== expectedFields.join(",") ||
    typeof capabilityId !== "string" ||
    !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u.test(capabilityId) ||
    receipt["courseReference"] !== expectedCourseReference ||
    receipt["sysadminReference"] !== expectedSysadminReference ||
    receipt["operationKind"] !== "course_roster_support" ||
    receipt["minimumProjection"] !== "course_roster" ||
    receipt["purpose"] !== "Review the screenshot corpus roster projection" ||
    typeof receipt["expiresAt"] !== "number" ||
    !Number.isSafeInteger(receipt["expiresAt"]) ||
    receipt["revokedAt"] !== null
  ) {
    throw new Error("support capability receipt does not match the requested scope");
  }
  return capabilityId;
}

async function issueSupportCapability(runtime: ScenarioRuntime): Promise<string> {
  // Capability issuance has no product UI. The existing same-origin contract prepares the token;
  // seeded Sysadmin U-5 still consumes it through the visible Scoped Support form below.
  const setupRecord = runtime.record("sysadmin_support", "support_entry");
  const setup = await runtime.open(setupRecord);
  const page = setup.page;
  try {
    await enterInstructor(page);
    await page.goto(
      new URL(`/instructor/courses/${runtime.references.course}/students`, runtime.entryUrl).href,
      { waitUntil: "commit" },
    );
    await page.getByRole("heading", { name: "Students", exact: true }).waitFor();
    await page
      .getByLabel("Email, roster ID")
      .fill("screenshot.support@live-demo.invalid,screenshot-support");
    await page.getByRole("button", { name: "Import roster", exact: true }).click();
    await page.getByText("screenshot-support", { exact: true }).waitFor();
    const response = await page.evaluate(
      async ({ courseReference, sysadminReference }) => {
        const result = await fetch(
          `/api/course-instances/${encodeURIComponent(courseReference)}/support-capabilities`,
          {
            method: "POST",
            headers: { "content-type": "application/json" },
            credentials: "same-origin",
            cache: "no-store",
            body: JSON.stringify({
              sysadminReference,
              purpose: "Review the screenshot corpus roster projection",
            }),
          },
        );
        return { ok: result.ok, status: result.status, body: (await result.json()) as unknown };
      },
      { courseReference: runtime.references.course, sysadminReference: SEEDED_SYSADMIN_REFERENCE },
    );
    if (!response.ok || response.status !== 201) {
      throw new Error(`support capability issuance failed with HTTP ${String(response.status)}`);
    }
    return supportCapabilityId(response.body, runtime.references.course, SEEDED_SYSADMIN_REFERENCE);
  } finally {
    await runtime.close(setup);
  }
}

async function sysadminSupport(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "sysadmin_support";
  const capabilityId = await issueSupportCapability(runtime);
  const session = await runtime.open(runtime.record(scenario, "support_entry"));
  const page = session.page;
  try {
    await enterSysadmin(page);
    await page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Scoped Support", exact: true })
      .click();
    await page
      .getByRole("heading", { name: "Scoped course roster support", exact: true })
      .waitFor();
    await captureCheckpoint(runtime, scenario, "support_entry", session);
    await page.getByLabel("Support capability ID").fill(capabilityId);
    await page.getByRole("button", { name: "Open scoped roster", exact: true }).click();
    const roster = page.getByRole("region", { name: "Scoped course roster", exact: true });
    await roster.getByText("Roster ID: screenshot-support", { exact: true }).waitFor();
    if ((await page.getByLabel("Support capability ID").inputValue()) !== "") {
      throw new Error("support capability remained in the visible credential field");
    }
    await captureCheckpoint(runtime, scenario, "support_roster", session);
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
  {
    id: "sysadmin_support",
    checkpoints: ["support_entry", "support_roster"],
    run: sysadminSupport,
  },
];
