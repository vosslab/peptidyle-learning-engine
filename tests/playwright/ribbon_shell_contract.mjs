// Signed-in shell contracts: role-stable tier-one navigation and fixed geometry.
// Selector contract: ApplicationShell owns the breadcrumb/main rails; PageFrame owns the title;
// AppRibbon exposes stable tier-one control IDs.

import assert from "node:assert/strict";

import { RIBBON_ROUTE_MATERIALIZATIONS } from "../support/ribbon_model_fixtures.ts";
import { CANONICAL_VIEWPORTS } from "./screenshot_corpus/manifest.ts";
import { caseLocator, openRibbonShellEvidencePage, waitForPath } from "./ribbon_shell_helpers.mjs";

function tierOneControlIds(ribbon) {
  return ribbon
    .locator('nav[aria-label="Ribbon tabs"] [data-ribbon-control-id]')
    .evaluateAll((controls) =>
      controls.map((control) => control.getAttribute("data-ribbon-control-id")),
    );
}

async function shellGeometry(shellCase) {
  return shellCase.evaluate((root) => {
    const breadcrumb = root.querySelector(".ple-shell__breadcrumb-prelude");
    const mainContent = root.querySelector("#main-content");
    const pageTitle = root.querySelector(".page-frame__title");
    if (
      !(breadcrumb instanceof HTMLElement) ||
      !(mainContent instanceof HTMLElement) ||
      !(pageTitle instanceof HTMLElement)
    ) {
      throw new Error("Signed-in shell fixture is missing its breadcrumb, main content, or title.");
    }
    return {
      breadcrumbTop: breadcrumb.getBoundingClientRect().top,
      mainContentTop: mainContent.getBoundingClientRect().top,
      pageTitleLeft: pageTitle.getBoundingClientRect().left,
    };
  });
}

const MEMBERSHIP_SETTINGS_SENTENCE =
  "Identify the object and relevant context before an action that changes membership or stored settings, so users can recognize what they are accepting or changing.";
const FEEDBACK_OUTCOME_SENTENCE =
  "Match feedback wording and visual emphasis to the outcome: success, information, warning, or error. Make the result and any next action easy to recognize.";
const STUDENT_VIEW_SENTENCE =
  "Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.";
const VOCABULARY_WORKFLOW_SENTENCE =
  "Creating or selecting vocabulary should fit naturally into the classification workflow.";

async function outcomeBorder(locator) {
  return locator.evaluate((element) => {
    function token(name) {
      const probe = document.createElement("span");
      probe.style.color = `var(${name})`;
      element.append(probe);
      const color = getComputedStyle(probe).color;
      probe.remove();
      return color;
    }
    return {
      border: getComputedStyle(element).borderInlineStartColor,
      success: token("--ple-success"),
      danger: token("--ple-danger"),
      warning: token("--ple-warning"),
      information: token("--ple-accent"),
    };
  });
}

function assertOutcome(label, paint, tokenName) {
  assert.equal(
    paint.border,
    paint[tokenName],
    `${FEEDBACK_OUTCOME_SENTENCE} ${label} uses the ${tokenName} emphasis`,
  );
  for (const other of ["success", "danger", "warning", "information"]) {
    if (other === tokenName) continue;
    assert.notEqual(paint.border, paint[other], `${label} emphasis stays distinct from ${other}`);
  }
}

async function assertRosterRemovalNamesTheStudent(page) {
  await page.setViewportSize({
    width: CANONICAL_VIEWPORTS.laptop.width,
    height: CANONICAL_VIEWPORTS.laptop.height,
  });
  await page.evaluate(() => window.ribbonShell.releaseSession());
  await page
    .locator('[data-m10-case="current-production"]')
    .getByText("Preparing your learning space")
    .waitFor({ state: "hidden" });
  await page.evaluate(() => {
    window.ribbonShell.holdLiveCourseRoster();
    window.ribbonShell.currentNavigate("/instructor/courses/CI7K3M2QAZ/students");
  });
  const roster = page.locator(
    '[data-m10-case="current-production"] [data-route-surface="courseRoster"]',
  );
  const loading = roster.getByText("Loading course roster...", { exact: true });
  await loading.waitFor({ state: "visible" });
  assertOutcome("information", await outcomeBorder(loading), "information");
  await page.evaluate(() => window.ribbonShell.releaseLiveCourseRoster());
  await roster.getByRole("heading", { name: "Students", exact: true }).waitFor({
    state: "visible",
  });

  async function openRemoval(studentName) {
    await roster
      .getByRole("row", { name: new RegExp(studentName) })
      .getByRole("button", { name: "Remove course access" })
      .click();
    const dialog = page.getByRole("dialog");
    await dialog.waitFor({ state: "visible" });
    return dialog;
  }

  const invitation = await openRemoval("Morgan Lee");
  const invitationText = await invitation.innerText();
  assert.match(invitationText, /Morgan Lee/);
  assert.match(invitationText, /RU-002/);
  assert.deepEqual(await page.evaluate(() => window.ribbonShell.revokedRosterIds()), []);
  await invitation.getByRole("button", { name: "Keep it", exact: true }).click();
  await invitation.waitFor({ state: "hidden" });
  assert.deepEqual(await page.evaluate(() => window.ribbonShell.revokedRosterIds()), []);

  const removal = await openRemoval("Avery Thompson");
  const removalText = await removal.innerText();
  assert.match(removalText, /Avery Thompson/);
  assert.match(removalText, /RU-001/);
  assert.match(removalText, /immediately loses course access/);
  assertOutcome("warning", await outcomeBorder(removal), "warning");
  await removal.getByRole("button", { name: "Keep it", exact: true }).waitFor();
  await removal.getByRole("button", { name: "Revoke course access", exact: true }).waitFor();
  assert.deepEqual(
    await page.evaluate(() => window.ribbonShell.revokedRosterIds()),
    [],
    `${MEMBERSHIP_SETTINGS_SENTENCE} confirmation is visible before the membership change`,
  );
  await removal.getByRole("button", { name: "Revoke course access", exact: true }).click();
  await page.waitForFunction(() => window.ribbonShell.revokedRosterIds().length === 1);
  assert.deepEqual(await page.evaluate(() => window.ribbonShell.revokedRosterIds()), ["RU-001"]);
  await removal.waitFor({ state: "hidden" });
  const success = roster.getByText(
    "Course access was removed. Protected educational records remain under retention.",
    { exact: true },
  );
  await success.waitFor({ state: "visible" });
  assertOutcome("success", await outcomeBorder(success), "success");

  await page.evaluate(() => window.ribbonShell.failNextRosterRevoke());
  const refused = await openRemoval("Avery Thompson");
  await refused.getByRole("button", { name: "Revoke course access", exact: true }).click();
  const failure = roster.getByText(
    "This roster entry could not be changed. Reload and try again.",
    { exact: true },
  );
  await failure.waitFor({ state: "visible" });
  assertOutcome("error", await outcomeBorder(failure), "danger");
  assert.deepEqual(await page.evaluate(() => window.ribbonShell.revokedRosterIds()), ["RU-001"]);
}

async function assertStudentViewCreatesNoStudentWork(page) {
  await page.evaluate(() => {
    window.ribbonShell.clearStudentViewTransport();
    window.ribbonShell.currentNavigate(
      "/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/student-view",
    );
  });
  const preview = page.locator(
    '[data-m10-case="current-production"] [data-route-surface="assessmentWorkspace"]',
  );
  await preview
    .getByText(
      "This is the current Student-facing Assessment without answers. It creates no Student Work, Assessment Attempt, submission, or grade.",
      { exact: true },
    )
    .waitFor({ state: "visible" });
  await preview.getByText("Which atoms form a peptide bond?", { exact: true }).waitFor({
    state: "visible",
  });
  const responsePreview = preview.locator("fieldset.student-view-disabled-response");
  await responsePreview.waitFor({ state: "visible" });
  assert.notEqual(await responsePreview.getAttribute("disabled"), null);
  assert.equal(await responsePreview.locator("[inert][aria-disabled='true']").count(), 1);
  assert.equal(await preview.getByRole("button", { name: "Save response" }).count(), 0);
  assert.equal(await preview.getByRole("button", { name: "Submit" }).count(), 0);
  assert.equal(await preview.getByRole("button", { name: "Start attempt" }).count(), 0);
  assert.deepEqual(
    await page.evaluate(() => window.ribbonShell.studentViewTransport()),
    ["getLiveAssessmentWorkspace", "getInstructorStudentView", "getInstructorStudentViewQuestion"],
    STUDENT_VIEW_SENTENCE,
  );
}

async function selectedOptionText(select) {
  return select.evaluate((element) => element.selectedOptions[0]?.textContent?.trim() ?? "");
}

async function waitForSelectedText(select, expected) {
  await select.waitForFunction(
    (element, text) => element.selectedOptions[0]?.textContent?.trim() === text,
    expected,
  );
  assert.equal(await selectedOptionText(select), expected, VOCABULARY_WORKFLOW_SENTENCE);
}

async function assertClassificationWorkflowCreatesVocabulary(page) {
  await page.setViewportSize({
    width: CANONICAL_VIEWPORTS.laptop.width,
    height: CANONICAL_VIEWPORTS.laptop.height,
  });
  await page.evaluate(() => {
    window.ribbonShell.seedAuthoringClassification();
    window.ribbonShell.currentNavigate("/instructor");
  });
  const home = page.locator('[data-m10-case="current-production"]');
  await home.getByRole("heading", { name: "My Active Courses", exact: true }).waitFor({
    state: "visible",
  });
  await home.locator("button.course-create-disclosure").click();
  const classification = home.locator("fieldset.course-classification-fields");
  await classification.waitFor({ state: "visible" });
  const discipline = classification.getByRole("combobox", {
    name: "Discipline (required)",
    exact: true,
  });
  await discipline.locator("option", { hasText: /^Biology$/ }).waitFor({ state: "attached" });
  await discipline.selectOption({ label: "Biology" });
  await classification.getByLabel("New Subject name", { exact: true }).fill("Peptides");
  await classification.getByRole("button", { name: "Create Subject", exact: true }).click();
  const subject = classification.getByRole("combobox", {
    name: "Subject (optional)",
    exact: true,
  });
  await waitForSelectedText(subject, "Peptides");
  await classification.getByLabel("New Topic name", { exact: true }).fill("Bonds");
  await classification.getByRole("button", { name: "Create Topic", exact: true }).click();
  const topic = classification.getByRole("combobox", { name: "Topic (optional)", exact: true });
  await waitForSelectedText(topic, "Bonds");
  await classification.getByLabel("New Subtopic name", { exact: true }).fill("Amide");
  await classification.getByRole("button", { name: "Create Subtopic", exact: true }).click();
  const subtopic = classification.getByRole("combobox", {
    name: "Subtopic (optional)",
    exact: true,
  });
  await waitForSelectedText(subtopic, "Amide");
  await classification
    .getByLabel("New Subject name", { exact: true })
    .fill("Existing Biochemistry");
  await classification.getByRole("button", { name: "Create Subject", exact: true }).click();
  await classification
    .getByText("Existing Biochemistry already exists. Associate it with this Discipline?", {
      exact: true,
    })
    .waitFor({ state: "visible" });
  await classification
    .getByRole("button", { name: "Associate existing Subject", exact: true })
    .click();
  await waitForSelectedText(subject, "Existing Biochemistry");
  const visible = await classification.innerText();
  assert.equal(
    /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/.test(visible),
    false,
    VOCABULARY_WORKFLOW_SENTENCE,
  );
  assert.deepEqual(
    await page.evaluate(() => window.ribbonShell.vocabularyWrites()),
    [
      "seed:11111111-1111-4111-8111-111111111111",
      "createSubject:Peptides",
      "createTopic:Bonds",
      "createSubtopic:Amide",
      "createSubject:Existing Biochemistry",
      "acceptSubject:22222222-2222-4222-8222-222222222222",
    ],
    VOCABULARY_WORKFLOW_SENTENCE,
  );
}

const { browser, page, harnessServer, pageErrors, consoleErrors } =
  await openRibbonShellEvidencePage();
try {
  const fixtureCase = caseLocator(page, "fixture-shell");
  const shellOffsets = new Map();
  const studentTitleEdges = new Map();
  const tierOneSequences = new Map();

  for (const [viewportId, viewport] of Object.entries(CANONICAL_VIEWPORTS)) {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    for (const materialization of RIBBON_ROUTE_MATERIALIZATIONS) {
      const { userRole, pathname, route } = materialization;
      if (userRole !== "student" && viewportId !== "laptop") continue;

      await page.evaluate(
        ({ role, routeId }) => window.ribbonShell.fixtureNavigateRoute(role, routeId),
        { role: userRole, routeId: route.id },
      );
      await waitForPath(page, "fixture-shell", pathname);
      const ribbon = fixtureCase.getByRole("region", {
        name: "PLE application Ribbon",
        exact: true,
      });
      await ribbon.waitFor();
      await page.waitForFunction(
        ({ role }) =>
          document
            .querySelector('[data-m10-case="fixture-shell"] .ple-app-ribbon')
            ?.getAttribute("data-ribbon-user-role") === role,
        { role: userRole },
      );

      const { pageTitleLeft, ...offsets } = await shellGeometry(fixtureCase);
      const key = `${viewportId}/${userRole}`;
      const previousOffsets = shellOffsets.get(key);
      if (previousOffsets === undefined) shellOffsets.set(key, offsets);
      else
        assert.deepEqual(offsets, previousOffsets, `${key} shell geometry drifts at ${route.id}`);

      if (userRole === "student") {
        const previousTitleLeft = studentTitleEdges.get(viewportId);
        if (previousTitleLeft === undefined) studentTitleEdges.set(viewportId, pageTitleLeft);
        else assert.equal(pageTitleLeft, previousTitleLeft, `${key} PageFrame title origin drifts`);
      }

      const ids = await tierOneControlIds(ribbon);
      assert.notEqual(ids.length, 0, `${key}/${route.id} has tier-one controls`);
      const previousIds = tierOneSequences.get(key);
      if (previousIds === undefined) tierOneSequences.set(key, ids);
      else assert.deepEqual(ids, previousIds, `${key} tier-one navigation drifts at ${route.id}`);
    }
  }

  await assertRosterRemovalNamesTheStudent(page);
  await assertStudentViewCreatesNoStudentWork(page);
  await assertClassificationWorkflowCreatesVocabulary(page);
  assert.deepEqual(pageErrors, [], "signed-in route sweep raises no browser page errors");
  assert.deepEqual(consoleErrors, [], "signed-in route sweep raises no browser console errors");
  process.stdout.write(
    "Ribbon navigation and shell geometry remain stable across role-reachable routes.\n",
  );
} finally {
  await browser.close();
  await harnessServer.close();
}
