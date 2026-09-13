// Visible Instructor proof: Draft -> published Blueprint Revision -> Course -> current Assignment.
// Selector contract: accessible labels/headings/actions in the Blueprint, Course, and Assignment
// workspaces; the Question picker discovers its seeded published Question through its visible result.

import { chromium } from "playwright";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const runId = Date.now();
const blueprintTitle = `Browser current Blueprint ${runId}`;
const courseShortName = `Current-${runId}`;
const courseLongName = `Browser current Course ${runId}`;
const assignmentTitle = `Browser current Assignment ${runId}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

async function firstNonEmptyOptionValue(select) {
  const option = select.locator('option:not([value=""])').first();
  try {
    await option.waitFor({ state: "attached", timeout: 10_000 });
  } catch {
    throw new Error("the current Course did not offer a Blueprint Assignment source");
  }
  return await option.evaluate((element) => element.value);
}

async function assertAnswerFreePreview(target) {
  for (const name of [/response/i, /submit/i, /start assignment/i, /student attempt/i]) {
    if ((await target.getByRole("button", { name }).count()) !== 0) {
      throw new Error("Assignment delivery check exposed a Student work control");
    }
  }
}

async function assertRibbonTaskCurrent(tasks, control, destination) {
  const current = await tasks
    .locator(`[data-ribbon-control="${control}"]`)
    .getAttribute("aria-current");
  if (current !== "page") {
    throw new Error(`${destination} was not selected in the Ribbon`);
  }
}

async function assignmentEntryIds(page) {
  return page
    .getByRole("heading", { name: "Ordered Assignment Entries", exact: true })
    .locator("..")
    .locator("[data-assignment-entry]")
    .evaluateAll((entries) => entries.map((entry) => entry.getAttribute("data-assignment-entry")));
}

function hasExactEntryOrder(actual, expected) {
  return (
    actual.length === expected.length &&
    actual.every((entryId, index) => entryId === expected[index])
  );
}

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Elena Rivera" }).click();
  await page.waitForURL(`${origin}/library`);

  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true })
    .click();
  await page.waitForURL(`${origin}/`);
  await page.getByRole("link", { name: "My Blueprint Courses", exact: true }).click();
  await page.waitForURL(`${origin}/blueprint-courses`);
  await page.getByRole("button", { name: "Create Blueprint Course", exact: true }).click();
  await page.getByRole("heading", { name: "Create a Blueprint Course" }).waitFor();
  await page.getByLabel("Blueprint Course title").fill(blueprintTitle);
  await page.getByRole("button", { name: "Choose published Questions", exact: true }).click();
  const questionPicker = page.getByRole("dialog", {
    name: "Choose the first reusable Questions",
    exact: true,
  });
  await questionPicker.getByRole("button", { name: "Search questions", exact: true }).click();
  await questionPicker.getByRole("checkbox").first().check();
  await questionPicker.getByRole("button", { name: "Use selected Questions", exact: true }).click();
  await page.getByText("1 fixed Question selected in order.", { exact: true }).waitFor();
  await page
    .getByRole("dialog", { name: "Create a Blueprint Course", exact: true })
    .getByRole("button", { name: "Create Blueprint Course", exact: true })
    .click();
  await page.waitForURL(/\/blueprint-courses\/BP-[1-9][0-9]*$/u);
  await page.getByRole("heading", { name: blueprintTitle, exact: true }).waitFor();

  await page.getByRole("button", { name: "Publish Blueprint Revision", exact: true }).click();
  await page.getByText("Published Blueprint Revision 1.", { exact: true }).waitFor();

  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true })
    .click();
  await page.waitForURL(`${origin}/`);
  await page.getByRole("heading", { name: "Course Instances you teach", exact: true }).waitFor();
  await page.getByLabel("Blueprint Course Revision").selectOption({
    label: `${blueprintTitle} · Revision 1`,
  });
  await page.getByLabel("Course short name").fill(courseShortName);
  await page.getByLabel("Course long name").fill(courseLongName);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByRole("button", { name: "Create Course Instance", exact: true }).click();
  const createdCourse = page.locator("article").filter({
    has: page.getByRole("heading", { name: courseLongName, exact: true }),
  });
  await createdCourse.getByRole("link", { name: "Open Course Instance", exact: true }).click();
  await page.waitForURL(/\/courses\/C-[1-9][0-9]*$/u);
  await page.getByRole("heading", { name: courseLongName, exact: true }).waitFor();

  await page.getByRole("link", { name: "Create Assignment", exact: true }).click();
  await page.waitForURL(/\/instructor\/courses\/C-[1-9][0-9]*\/assignments\/new$/u);
  await page.getByRole("heading", { name: "Create an Assignment", exact: true }).waitFor();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  const assignmentSource = page.getByLabel("Blueprint Assignment source");
  await assignmentSource.selectOption(await firstNonEmptyOptionValue(assignmentSource));
  await page.getByRole("button", { name: "Create Assignment", exact: true }).click();
  await page.waitForURL(
    /\/instructor\/courses\/C-[1-9][0-9]*\/assignments\/A-[1-9][0-9]*\/questions$/u,
  );
  await page.getByRole("heading", { name: "Questions", exact: true }).waitFor();
  const availableQuestions = page
    .getByRole("heading", { name: "Available published Questions", exact: true })
    .locator("..");
  const initialAvailableQuestionCount = await availableQuestions
    .getByRole("button", { name: "Add Question", exact: true })
    .count();
  if (initialAvailableQuestionCount < 2) {
    throw new Error("the fixture did not offer two distinct published Questions to add");
  }
  await availableQuestions
    .getByRole("button", { name: "Add Question", exact: true })
    .first()
    .click();
  await availableQuestions
    .getByRole("button", { name: "Add Question", exact: true })
    .first()
    .click();
  await page.getByText(/exact revision pin/u).waitFor();
  const localEntryIds = await assignmentEntryIds(page);
  if (
    localEntryIds.length !== 2 ||
    localEntryIds.some((entryId) => entryId === null) ||
    new Set(localEntryIds).size !== localEntryIds.length
  ) {
    throw new Error(
      "adding two available Questions did not create two distinct Assignment Entries",
    );
  }
  await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
  await page
    .getByText("Questions and order saved. Review assignment policies when you are ready.", {
      exact: true,
    })
    .waitFor();
  const ribbonTasks = page.getByRole("navigation", { name: "Ribbon tasks", exact: true });
  await assertRibbonTaskCurrent(ribbonTasks, "assignmentQuestions", "Questions");
  await ribbonTasks.locator('[data-ribbon-control="assignmentPolicies"]').click();
  await page.getByRole("heading", { name: "Policies", exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assignmentPolicies", "Policies");

  await ribbonTasks.locator('[data-ribbon-control="assignmentQuestions"]').click();
  await page.getByRole("heading", { name: "Questions", exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assignmentQuestions", "Questions");
  const savedEntryIds = await assignmentEntryIds(page);
  if (
    savedEntryIds.length !== 2 ||
    savedEntryIds.some((entryId) => entryId === null) ||
    new Set(savedEntryIds).size !== savedEntryIds.length
  ) {
    throw new Error("the saved Assignment lacked two stable ordered Entries to reorder");
  }
  const moveDown = page.getByRole("button", { name: "Move down", exact: true }).first();
  if (await moveDown.isDisabled()) {
    throw new Error("the first saved Assignment Entry could not be moved down");
  }
  await moveDown.click();
  const reorderedEntryIds = await assignmentEntryIds(page);
  if (hasExactEntryOrder(reorderedEntryIds, savedEntryIds)) {
    throw new Error("the local structural Question reorder did not change the saved Entry order");
  }
  await ribbonTasks.locator('[data-ribbon-control="assignmentPolicies"]').click();
  await page
    .getByRole("heading", { name: "Save Assignment Question changes?", exact: true })
    .waitFor();
  await page.getByRole("button", { name: "Stay and keep editing", exact: true }).click();
  if (!hasExactEntryOrder(await assignmentEntryIds(page), reorderedEntryIds)) {
    throw new Error("Stay did not retain the exact local structural Question order");
  }
  await ribbonTasks.locator('[data-ribbon-control="assignmentPolicies"]').click();
  await page.getByRole("button", { name: "Discard and continue", exact: true }).click();
  await page.getByRole("heading", { name: "Policies", exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assignmentPolicies", "Policies");
  await ribbonTasks.locator('[data-ribbon-control="assignmentQuestions"]').click();
  await page.getByRole("heading", { name: "Questions", exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assignmentQuestions", "Questions");
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.getByRole("heading", { name: "Questions", exact: true }).waitFor();
  const reloadedEntryIds = await assignmentEntryIds(page);
  if (!hasExactEntryOrder(reloadedEntryIds, savedEntryIds)) {
    throw new Error("Discard did not restore the exact saved Assignment Entry order");
  }
  await ribbonTasks.locator('[data-ribbon-control="assignmentPolicies"]').click();
  await page.getByRole("heading", { name: "Policies", exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assignmentPolicies", "Policies");
  await page
    .getByRole("group", { name: "Due date and time", exact: true })
    .getByLabel(/Due date/u)
    .fill("2026-12-01");
  await page.getByLabel("Due time", { exact: true }).fill("12:00");
  await page.getByLabel("Time limit in seconds", { exact: true }).fill("1800");
  await page.getByLabel("Late-work rule").selectOption("mark_late");
  await page.getByText("Saved", { exact: true }).waitFor();

  let releaseHeldPolicyPut;
  let policyPutWasHeld;
  const heldPolicyPut = new Promise((resolve) => {
    releaseHeldPolicyPut = resolve;
  });
  const policyPutHeld = new Promise((resolve) => {
    policyPutWasHeld = resolve;
  });
  const holdPolicyPut = async (route) => {
    if (route.request().method() !== "PUT") {
      await route.continue();
      return;
    }
    policyPutWasHeld();
    await heldPolicyPut;
    await route.continue();
  };
  await page.route("**/policies", holdPolicyPut);
  let heldPolicyPutReleased = false;
  const releaseHeldPolicyPutOnce = () => {
    if (!heldPolicyPutReleased) {
      heldPolicyPutReleased = true;
      releaseHeldPolicyPut();
    }
  };
  try {
    await page.getByLabel("Late-work rule").selectOption("accept");
    await policyPutHeld;
    await page.getByText("Saving", { exact: true }).waitFor();
    if (
      !(await page.getByRole("button", { name: "Release assignment", exact: true }).isDisabled())
    ) {
      throw new Error("Release was available while the visible policy value was saving");
    }
    if (
      !(await page
        .getByRole("button", { name: "Check release readiness", exact: true })
        .isDisabled())
    ) {
      throw new Error("Release readiness was available while the visible policy value was saving");
    }
    releaseHeldPolicyPutOnce();
    await page.getByText("Saved", { exact: true }).waitFor();
  } finally {
    releaseHeldPolicyPutOnce();
    await page.unroute("**/policies", holdPolicyPut);
  }

  await page.reload({ waitUntil: "domcontentloaded" });
  await page.getByRole("heading", { name: "Policies", exact: true }).waitFor();
  const courseReference = new URL(page.url()).pathname.match(
    /^\/instructor\/courses\/(C-[1-9][0-9]*)\/assignments\//u,
  )?.[1];
  if (courseReference === undefined)
    throw new Error("the assignment workspace lacked a Course reference");
  if ((await page.getByLabel("Time limit in seconds", { exact: true }).inputValue()) !== "1800") {
    throw new Error("the autosaved time limit did not persist after reload");
  }
  if ((await page.getByLabel("Late-work rule").inputValue()) !== "accept") {
    throw new Error("the autosaved Late-work rule did not persist after reload");
  }
  await page.getByLabel("Time limit in seconds", { exact: true }).fill("0");
  await page.getByText("Invalid", { exact: true }).waitFor();
  if (!(await page.getByRole("button", { name: "Release assignment", exact: true }).isDisabled())) {
    throw new Error("Release was available while the visible policy value was invalid");
  }
  if (
    !(await page.getByRole("button", { name: "Check release readiness", exact: true }).isDisabled())
  ) {
    throw new Error("Release readiness was available while the visible policy value was invalid");
  }
  await page.getByLabel("Time limit in seconds", { exact: true }).fill("1800");
  await page.getByText("Saved", { exact: true }).waitFor();

  const deliveryCheckPage = context.waitForEvent("page");
  await page.getByRole("link", { name: "Check assignment delivery", exact: true }).click();
  const deliveryCheck = await deliveryCheckPage;
  await deliveryCheck
    .getByRole("heading", { name: "Assignment delivery check", exact: true })
    .waitFor();
  await deliveryCheck
    .getByText("Preview only - no Student work or grades are created.", { exact: true })
    .waitFor();
  await assertAnswerFreePreview(deliveryCheck);
  await deliveryCheck.close();

  await page.getByRole("button", { name: "Check release readiness", exact: true }).click();
  await page.getByRole("heading", { name: "Ready to release", exact: true }).waitFor();
  await page
    .getByText("Release readiness checked. This saved assignment is ready to release.", {
      exact: true,
    })
    .waitFor();
  await page.getByRole("button", { name: "Release assignment", exact: true }).click();
  await page.getByText(/^Assignment released\. Current edit number: [1-9][0-9]*\.$/u).waitFor();
  await page
    .getByRole("button", { name: "Release assignment", exact: true })
    .waitFor({ state: "hidden" });
  await page
    .getByRole("heading", { name: "Danger Zone: Unrelease assignment", exact: true })
    .waitFor();
  await ribbonTasks.locator('[data-ribbon-control="assignmentOverview"]').click();
  await page.getByRole("heading", { name: assignmentTitle, exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assignmentOverview", "Overview");
  const ribbonTabs = page.getByRole("navigation", { name: "Ribbon tabs", exact: true });
  await ribbonTabs.locator('[data-ribbon-control="assignments"]').click();
  await page.getByRole("heading", { name: courseLongName, exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTabs, "assignments", "Course Assignments");
  for (const unsupportedPath of [
    `/instructor/courses/${courseReference}/grade-settings`,
    `/instructor/courses/${courseReference}/teaching-operations`,
  ]) {
    await page.goto(`${origin}${unsupportedPath}`, { waitUntil: "domcontentloaded" });
    await page.locator('[data-route-surface="notFound"]').waitFor();
    await page
      .getByRole("heading", { name: "That page is not part of this learning space", exact: true })
      .waitFor();
  }
} finally {
  await context.close();
  await browser.close();
}
