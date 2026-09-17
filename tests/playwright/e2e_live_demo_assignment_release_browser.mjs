// Visible Instructor proof: Blueprint Revision 1 -> Course -> current Assessment.
// Selector contract: accessible labels/headings/actions in the Blueprint, Course, and Assessment
// workspaces; assessment_workspace_policies_page.tsx renders the duration label with dynamic
// calculated-default helper text, so its stable accessible-name prefix is selected here. The
// Question picker discovers its seeded published Question through its visible result.

import { chromium } from "playwright";
import { liveDemoChromiumArgs } from "./helper_gateway_trust.mjs";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const runId = Date.now();
const blueprintTitle = `Browser current Blueprint ${runId}`;
const courseShortName = `Current-${runId}`;
const courseLongName = `Browser current Course ${runId}`;
const assessmentTitle = `Browser current Assessment ${runId}`;
const assessmentDurationOverrideLabel = /^Assessment duration override in minutes\b/u;
const browser = await chromium.launch({ headless: true, args: liveDemoChromiumArgs(origin) });
const context = await browser.newContext();
const page = await context.newPage();

async function assertAnswerFreePreview(target) {
  for (const name of [/response/i, /submit/i, /start assessment/i, /student attempt/i]) {
    if ((await target.getByRole("button", { name }).count()) !== 0) {
      throw new Error("Assessment Student View exposed a Student Work control");
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

async function assessmentEntryIds(page) {
  return page
    .getByRole("heading", { name: "Ordered Assessment Entries", exact: true })
    .locator("..")
    .locator("[data-assessment-entry]")
    .evaluateAll((entries) => entries.map((entry) => entry.getAttribute("data-assessment-entry")));
}

function hasExactEntryOrder(actual, expected) {
  return (
    actual.length === expected.length &&
    actual.every((entryId, index) => entryId === expected[index])
  );
}

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page
    .getByRole("button", { name: "Assume the role of Instructor Dr. Elena Rivera" })
    .click();
  await page.waitForURL(`${origin}/library`);

  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true })
    .click();
  await page.waitForURL(`${origin}/instructor`);
  await page.getByRole("link", { name: "My Blueprint Courses", exact: true }).click();
  await page.waitForURL(`${origin}/blueprint-courses`);
  await page.getByRole("button", { name: "Create Blueprint Course", exact: true }).click();
  await page.getByRole("heading", { name: "Create a Blueprint Course" }).waitFor();
  await page.getByLabel("Blueprint Course short name").fill(`Current BP ${runId}`);
  await page.getByLabel("Blueprint Course long name").fill(blueprintTitle);
  await page.getByLabel("Discipline (required)").selectOption({ index: 1 });
  await page.getByLabel("First Assessment Type").selectOption("practice_question_assignment");
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
  await page.waitForURL(/\/blueprint-courses\/BP[0-9A-HJKMNP-TV-Z]{8}$/u);
  await page.getByRole("heading", { name: blueprintTitle, exact: true }).waitFor();
  await page.getByRole("button", { name: "Publish Blueprint Course", exact: true }).click();
  await page
    .getByText(
      "Blueprint Course published. Instructors can now browse and adopt its current Revision.",
      { exact: true },
    )
    .waitFor();
  await page
    .getByRole("link", { name: "Create Course Instance from this Blueprint", exact: true })
    .click();
  await page.waitForURL(/\/instructor\?blueprint=BP[0-9A-HJKMNP-TV-Z]{8}#create-course-instance$/u);
  await page.getByRole("heading", { name: "Course Instances you teach", exact: true }).waitFor();
  await page
    .getByRole("combobox", { name: /^Blueprint Course/u })
    .selectOption({ label: `${blueprintTitle} · Revision 1` });
  await page.getByLabel("Course short name").fill(courseShortName);
  await page.getByLabel("Course long name").fill(courseLongName);
  await page.getByLabel("Discipline (required)").selectOption({ index: 1 });
  await page.getByLabel("Subject (optional)").selectOption({ index: 1 });
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page
    .locator("#create-course-instance")
    .getByRole("button", { name: "Create Course Instance", exact: true })
    .click();
  const createdCourse = page.locator("article").filter({
    has: page.getByRole("heading", { name: courseLongName, exact: true }),
  });
  await createdCourse.getByRole("link", { name: "Open Course Instance", exact: true }).click();
  await page.waitForURL(/\/courses\/CI[0-9A-HJKMNP-TV-Z]{8}$/u);
  await page.getByRole("heading", { name: courseLongName, exact: true }).waitFor();

  await page.getByRole("link", { name: "Create Assessment", exact: true }).click();
  await page.waitForURL(/\/instructor\/courses\/CI[0-9A-HJKMNP-TV-Z]{8}\/assessments\/new$/u);
  await page.getByRole("heading", { name: "Create an Assessment", exact: true }).waitFor();
  await page.getByLabel("Assessment title").fill(assessmentTitle);
  await page.getByLabel("Assessment Type").selectOption("practice_question_assignment");
  await page.getByRole("button", { name: "Create Assessment", exact: true }).click();
  await page.waitForURL(
    /\/instructor\/courses\/CI[0-9A-HJKMNP-TV-Z]{8}\/assessments\/A[0-9A-HJKMNP-TV-Z]{8}\/questions$/u,
  );
  await page.getByRole("heading", { name: "Assessment Question Editor", exact: true }).waitFor();
  const assessmentBreadcrumb = page.getByRole("navigation", { name: "Breadcrumb", exact: true });
  await assessmentBreadcrumb.getByRole("link", { name: assessmentTitle, exact: true }).waitFor();
  const assessmentIdentity = page.locator('dl[aria-label="Current Assessment"]');
  await assessmentIdentity.getByText(assessmentTitle, { exact: true }).waitFor();
  await assessmentIdentity.getByText("Unreleased", { exact: true }).waitFor();
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
  const localEntryIds = await assessmentEntryIds(page);
  if (
    localEntryIds.length !== 2 ||
    localEntryIds.some((entryId) => entryId === null) ||
    new Set(localEntryIds).size !== localEntryIds.length
  ) {
    throw new Error(
      "adding two available Questions did not create two distinct Assessment Entries",
    );
  }
  await page.getByRole("heading", { name: /^Entry 1 · /u }).waitFor();
  await page.getByRole("group", { name: "Entry 1 actions", exact: true }).waitFor();
  await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
  await page
    .getByText("Questions and order saved. Review Assessment Properties when you are ready.", {
      exact: true,
    })
    .waitFor();
  const ribbonTasks = page.getByRole("navigation", { name: "Ribbon tasks", exact: true });
  await assertRibbonTaskCurrent(ribbonTasks, "assessmentQuestions", "Questions");
  await ribbonTasks.locator('[data-ribbon-control="assessmentPolicies"]').click();
  await page.getByRole("heading", { name: "Assessment Properties Editor", exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assessmentPolicies", "Properties");
  await assessmentBreadcrumb.getByRole("link", { name: assessmentTitle, exact: true }).waitFor();
  await assessmentIdentity.getByText(assessmentTitle, { exact: true }).waitFor();
  await assessmentIdentity.getByText("Unreleased", { exact: true }).waitFor();

  await ribbonTasks.locator('[data-ribbon-control="assessmentQuestions"]').click();
  await page.getByRole("heading", { name: "Assessment Question Editor", exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assessmentQuestions", "Questions");
  const savedEntryIds = await assessmentEntryIds(page);
  if (
    savedEntryIds.length !== 2 ||
    savedEntryIds.some((entryId) => entryId === null) ||
    new Set(savedEntryIds).size !== savedEntryIds.length
  ) {
    throw new Error("the saved Assessment lacked two stable ordered Entries to reorder");
  }
  const moveLater = page.getByRole("button", { name: "Move later", exact: true }).first();
  if (await moveLater.isDisabled()) {
    throw new Error("the first saved Assessment Entry could not be moved later");
  }
  await moveLater.click();
  const reorderedEntryIds = await assessmentEntryIds(page);
  if (hasExactEntryOrder(reorderedEntryIds, savedEntryIds)) {
    throw new Error("the local structural Question reorder did not change the saved Entry order");
  }
  await ribbonTasks.locator('[data-ribbon-control="assessmentPolicies"]').click();
  await page
    .getByRole("heading", { name: "Save Assessment Question changes?", exact: true })
    .waitFor();
  await page.getByRole("button", { name: "Stay and keep editing", exact: true }).click();
  if (!hasExactEntryOrder(await assessmentEntryIds(page), reorderedEntryIds)) {
    throw new Error("Stay did not retain the exact local structural Question order");
  }
  await ribbonTasks.locator('[data-ribbon-control="assessmentPolicies"]').click();
  await page.getByRole("button", { name: "Discard and continue", exact: true }).click();
  await page.getByRole("heading", { name: "Assessment Properties Editor", exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assessmentPolicies", "Properties");
  await ribbonTasks.locator('[data-ribbon-control="assessmentQuestions"]').click();
  await page.getByRole("heading", { name: "Assessment Question Editor", exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assessmentQuestions", "Questions");
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.getByRole("heading", { name: "Assessment Question Editor", exact: true }).waitFor();
  const reloadedEntryIds = await assessmentEntryIds(page);
  if (!hasExactEntryOrder(reloadedEntryIds, savedEntryIds)) {
    throw new Error("Discard did not restore the exact saved Assessment Entry order");
  }
  await ribbonTasks.locator('[data-ribbon-control="assessmentPolicies"]').click();
  await page.getByRole("heading", { name: "Assessment Properties Editor", exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assessmentPolicies", "Properties");
  await page
    .getByRole("group", { name: "Due date and time", exact: true })
    .getByLabel(/Due date/u)
    .fill("2026-12-01");
  await page.getByLabel("Due time", { exact: true }).fill("12:00");
  await page.getByLabel(assessmentDurationOverrideLabel).fill("30");
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
      !(await page.getByRole("button", { name: "Release assessment", exact: true }).isDisabled())
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
  await page.getByRole("heading", { name: "Assessment Properties Editor", exact: true }).waitFor();
  const courseReference = new URL(page.url()).pathname.match(
    /^\/instructor\/courses\/(CI[0-9A-HJKMNP-TV-Z]{8})\/assessments\//u,
  )?.[1];
  if (courseReference === undefined)
    throw new Error("the Assessment workspace lacked a Course reference");
  if ((await page.getByLabel(assessmentDurationOverrideLabel).inputValue()) !== "30") {
    throw new Error("the autosaved time limit did not persist after reload");
  }
  if ((await page.getByLabel("Late-work rule").inputValue()) !== "accept") {
    throw new Error("the autosaved Late-work rule did not persist after reload");
  }
  await page.getByLabel(assessmentDurationOverrideLabel).fill("0");
  await page.getByText("Invalid", { exact: true }).waitFor();
  if (!(await page.getByRole("button", { name: "Release assessment", exact: true }).isDisabled())) {
    throw new Error("Release was available while the visible policy value was invalid");
  }
  if (
    !(await page.getByRole("button", { name: "Check release readiness", exact: true }).isDisabled())
  ) {
    throw new Error("Release readiness was available while the visible policy value was invalid");
  }
  await page.getByLabel(assessmentDurationOverrideLabel).fill("30");
  await page.getByText("Saved", { exact: true }).waitFor();

  await page.getByRole("link", { name: "Open Student View", exact: true }).click();
  await page.getByText("Student View preview", { exact: true }).waitFor();
  await assertAnswerFreePreview(page);
  await page.getByRole("link", { name: "Return to assessment", exact: true }).click();
  await ribbonTasks.locator('[data-ribbon-control="assessmentPolicies"]').click();
  await page.getByRole("heading", { name: "Assessment Properties Editor", exact: true }).waitFor();

  await page.getByRole("button", { name: "Check release readiness", exact: true }).click();
  await page.getByRole("heading", { name: "Ready to release", exact: true }).waitFor();
  await page
    .getByText("Release readiness checked. This saved assessment is ready to release.", {
      exact: true,
    })
    .waitFor();
  await page.getByRole("button", { name: "Release assessment", exact: true }).click();
  await page.getByText(/^Assessment released\. Current edit number: [1-9][0-9]*\.$/u).waitFor();
  await page
    .getByRole("button", { name: "Release assessment", exact: true })
    .waitFor({ state: "hidden" });
  await assessmentIdentity.getByText("Released", { exact: true }).waitFor();
  await page
    .getByRole("heading", { name: "Danger Zone: Unrelease assessment", exact: true })
    .waitFor();
  await ribbonTasks.locator('[data-ribbon-control="assessmentOverview"]').click();
  await page.getByRole("heading", { name: assessmentTitle, exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTasks, "assessmentOverview", "Overview");
  const ribbonTabs = page.getByRole("navigation", { name: "Ribbon tabs", exact: true });
  await ribbonTabs.locator('[data-ribbon-control="assessments"]').click();
  await page.getByRole("heading", { name: courseLongName, exact: true }).waitFor();
  await assertRibbonTaskCurrent(ribbonTabs, "assessments", "Course Assessments");
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
