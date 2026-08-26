// Production-stack WP-PROF-B2 curriculum-adoption journey.
//
// Selector contract:
// - src/pages/editor_page.tsx:906 owns public question publication.
// - src/features/reusable_curriculum/reusable_curriculum_create_dialog.tsx:154 owns source creation.
// - src/features/curriculum_adoption/alpha_fork_action.tsx:180 owns independent-copy review.
// - src/features/curriculum_adoption/curriculum_adoption_page.tsx:80 owns the adoption workflow.
// - src/features/curriculum_adoption/curriculum_adoption_panels.tsx:319 owns receipt navigation.
// - src/pages/teaching_operations/sysadmin_instructor_approval_panel.tsx:241 owns approval search.
// - src/pages/teaching_team_panel.tsx:281 owns co-instructor invitation.
// - src/pages/account_pending_invitations_page.tsx:253 owns invitation acceptance.

import { expect, test, type Locator, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  expectObservedOrigin,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
  writeContextOriginReceipt,
  type ObservedOrigins,
} from "./real_stack_ui";

const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 600_000;
const baseCourseTitle = "Biochemistry Base Course";
const averyCourseTitle = "Genetics Practice Course";
const initialDstTerm = {
  startDate: "2027-03-14",
  endDate: "2027-04-30",
  timeZone: "America/Chicago",
};
const correctedDstTerm = { ...initialDstTerm, startDate: "2027-03-15" };
const shiftedTerm = {
  startDate: "2027-04-06",
  endDate: "2027-05-31",
  timeZone: "America/Chicago",
};
const rolloverTerm = {
  startDate: "2027-09-01",
  endDate: "2027-12-15",
  timeZone: "America/Chicago",
};

function assignmentCard(page: Page, title: string): Locator {
  return page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: title, exact: true }) });
}

async function createPublishedQuestion(page: Page, title: string): Promise<void> {
  await page.getByRole("link", { name: "Workspace", exact: true }).click();
  await page.getByRole("button", { name: "Create flat question", exact: true }).click();
  await page.getByLabel("Question title").fill(title);
  await page.getByLabel("Learner-facing prompt").fill(`Choose the supported statement: ${title}`);
  await page.getByLabel("Choice text").nth(0).fill(`Supported statement for ${title}`);
  await page.getByLabel("Choice text").nth(1).fill(`Alternative statement for ${title}`);
  await page
    .getByRole("radio", { name: new RegExp(`Mark choice 1 as correct: Supported statement`) })
    .check();
  await page.getByRole("button", { name: "Save private draft", exact: true }).click();
  await page.getByRole("button", { name: "Review publication changes", exact: true }).click();
  await page.getByLabel("Publication scope").selectOption({ label: "Public" });
  await page.getByLabel("Reviewed public byline").fill("Dr. Elena Rivera");
  await page.getByRole("button", { name: "Confirm and publish", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Published", exact: true })).toBeVisible();
}

async function createScheduledAlpha(
  page: Page,
  alphaTitle: string,
  questionTitle: string,
): Promise<string> {
  await page.getByRole("link", { name: "Curriculum", exact: true }).click();
  await page.getByRole("button", { name: "Create Alpha curriculum", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Create an Alpha curriculum", exact: true });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("Curriculum title").fill(alphaTitle);
  await dialog.getByRole("button", { name: "Choose published questions", exact: true }).click();
  const picker = page.getByRole("dialog", {
    name: "Choose the first reusable questions",
    exact: true,
  });
  await expect(picker).toBeVisible();
  await picker.getByLabel("Question source").selectOption({ label: "Public library" });
  await picker.getByLabel("Search questions").fill(questionTitle);
  await picker.getByRole("button", { name: "Search questions", exact: true }).click();
  await picker.getByRole("checkbox", { name: new RegExp(questionTitle, "u") }).check();
  await picker.getByRole("button", { name: "Use selected questions", exact: true }).click();
  await expect(picker).toHaveCount(0);
  await dialog.getByRole("button", { name: "Create live curriculum", exact: true }).click();
  const detail = page.locator('[data-route-surface="curriculumDetail"]');
  await expect(
    detail.getByRole("heading", { level: 1, name: alphaTitle, exact: true }),
  ).toBeVisible();
  const referenceMatch = new URL(page.url()).pathname.match(/^\/curriculum\/(AC-[1-9][0-9]*)$/u);
  if (referenceMatch?.[1] === undefined) {
    throw new Error("The created Alpha curriculum did not expose its public reference.");
  }
  await detail.getByLabel("Available relative moment").fill("0|02:30:00.000");
  await detail.getByLabel("Due relative moment").fill("1|03:00:00.000");
  await detail.getByLabel("Close relative moment").fill("2|03:00:00.000");
  await detail.getByRole("button", { name: "Save curriculum", exact: true }).click();
  await expect(detail.getByRole("status")).toContainText("Alpha curriculum saved");
  return referenceMatch[1];
}

async function reviseAlphaAssignment(
  page: Page,
  alphaTitle: string,
  alphaReference: string,
  assignmentTitle: string,
): Promise<void> {
  await page.getByRole("link", { name: "Curriculum", exact: true }).click();
  const workspace = page.locator('[data-route-surface="curriculum"]');
  await expect(workspace).toBeVisible();
  const alphaLink = workspace.locator(`a[href="/curriculum/${alphaReference}"]`);
  await expect(alphaLink).toContainText(alphaTitle);
  await alphaLink.click();
  const detail = page.locator('[data-route-surface="curriculumDetail"]');
  await expect(
    detail.getByRole("heading", { level: 1, name: alphaTitle, exact: true }),
  ).toBeVisible();
  await detail.getByLabel("Assignment title").fill(assignmentTitle);
  await detail.getByRole("button", { name: "Save curriculum", exact: true }).click();
  await expect(detail.getByRole("status")).toContainText("Alpha curriculum saved");
}

async function openCurriculumChanges(page: Page): Promise<void> {
  await page.getByRole("link", { name: "Curriculum changes", exact: true }).click();
  await expect(page.locator('[data-route-surface="curriculumAdoption"]')).toBeVisible();
  await expect(
    page.getByRole("heading", { level: 1, name: "Adopt reusable curriculum" }),
  ).toBeVisible();
}

async function setTargetTerm(
  page: Page,
  term: { readonly startDate: string; readonly endDate: string; readonly timeZone: string },
): Promise<void> {
  await page.getByLabel("Start date").fill(term.startDate);
  await page.getByLabel("End date").fill(term.endDate);
  await page.getByLabel("Time zone").fill(term.timeZone);
}

async function inspectImports(page: Page): Promise<Locator> {
  await page.getByRole("radio", { name: /Inspect curriculum imports/u }).check();
  await page.getByRole("button", { name: "Inspect imports", exact: true }).click();
  const evidence = page.getByRole("region", { name: "Curriculum import evidence" });
  await expect(evidence).toBeVisible();
  await expect(
    evidence.getByRole("heading", { name: "Imported curriculum evidence" }),
  ).toBeVisible();
  return evidence;
}

async function openCompletedDestination(page: Page, title: string): Promise<void> {
  const completion = page.getByRole("region", { name: "Completed curriculum adoption" });
  await expect(completion).toBeVisible();
  await completion.getByRole("link", { name: "Open course", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: title, exact: true })).toBeVisible();
}

async function approveAvery(page: Page): Promise<void> {
  await chooseSeededIdentity(page, /Morgan/u);
  await selectVisibleCourse(page, averyCourseTitle);
  await page.getByRole("link", { name: "Teaching operations", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Instructor approval" })).toBeVisible();
  await page.getByLabel("Find an account by name").fill("Avery");
  await page.getByRole("button", { name: "Search accounts" }).click();
  const candidate = page.getByRole("listitem").filter({ hasText: "Avery Singh" });
  await expect(candidate).toBeVisible();
  const approve = candidate.getByRole("button", { name: "Approve as instructor", exact: true });
  if ((await approve.count()) === 1) {
    await approve.click();
    await page.getByRole("dialog").getByRole("button", { name: "Approve as instructor" }).click();
    await expect(page.getByText(/Avery Singh.*eligible/u)).toBeVisible();
  } else {
    await expect(candidate).toContainText("Approved for invitations");
  }
}

async function inviteAveryToBaseCourse(page: Page): Promise<void> {
  await page.getByRole("link", { name: "Courses", exact: true }).click();
  const baseCourse = page.getByRole("article").filter({ hasText: baseCourseTitle });
  await expect(baseCourse).toBeVisible();
  await baseCourse.getByRole("link", { name: "Open course", exact: true }).click();
  await page.getByRole("link", { name: "Teaching operations", exact: true }).click();
  const teachingTeam = page.getByRole("region", { name: "Teaching team" });
  await expect(teachingTeam).toBeVisible();
  const activeAvery = teachingTeam
    .getByRole("region", { name: "Active instructors", exact: true })
    .getByRole("article")
    .filter({ hasText: "Avery Singh" });
  if ((await activeAvery.count()) === 1) {
    await expect(activeAvery).toContainText("Active direct instructor");
    return;
  }
  await teachingTeam.getByLabel("Find an approved colleague").fill("Avery");
  await teachingTeam.getByRole("button", { name: "Search eligible people" }).click();
  const eligibleAvery = teachingTeam
    .getByRole("list", { name: "Eligible co-instructor search results", exact: true })
    .getByRole("listitem")
    .filter({ hasText: "Avery Singh" });
  const pendingAvery = teachingTeam
    .getByRole("region", { name: "Pending invitations", exact: true })
    .getByRole("article")
    .filter({ hasText: "Avery Singh" });
  await expect(eligibleAvery.or(pendingAvery)).toBeVisible();
  if (await eligibleAvery.isVisible()) {
    await eligibleAvery.getByRole("button", { name: "Select", exact: true }).click();
    await teachingTeam.getByRole("button", { name: "Invite selected colleague" }).click();
    await expect(teachingTeam.getByRole("status")).toHaveText(
      "An invitation was created for Avery Singh.",
    );
  }
}

async function acceptBaseCourseInvitation(page: Page): Promise<void> {
  await chooseSeededIdentity(page, /Avery Singh/u);
  await selectVisibleCourse(page, averyCourseTitle);
  await page.getByRole("link", { name: "Invitations", exact: true }).click();
  const acceptInvitation = page
    .getByRole("article")
    .filter({ hasText: baseCourseTitle })
    .getByRole("button", { name: "Accept", exact: true });
  const noInvitations = page.getByRole("heading", { name: "No invitations waiting" });
  await expect(acceptInvitation.or(noInvitations)).toBeVisible();
  if (await acceptInvitation.isVisible()) {
    await acceptInvitation.click();
    await page.getByRole("dialog").getByRole("button", { name: "Accept invitation" }).click();
    await expect(page.getByRole("main").getByRole("status")).toHaveText("Invitation accepted.");
  }
  await signOutVisible(page);
  await chooseSeededIdentity(page, /Avery Singh/u);
  await selectVisibleCourse(page, baseCourseTitle);
}

async function forkAlphaAsAvery(
  page: Page,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
  alphaTitle: string,
  alphaReference: string,
): Promise<void> {
  await acceptBaseCourseInvitation(page);
  await page.getByRole("link", { name: "Curriculum", exact: true }).click();
  const workspace = page.locator('[data-route-surface="curriculum"]');
  await expect(workspace).toBeVisible();
  const alphaLink = workspace.locator(`a[href="/curriculum/${alphaReference}"]`);
  await expect(alphaLink).toContainText(alphaTitle);
  await alphaLink.click();
  const fork = page.getByRole("region", { name: "Create an independent Alpha copy" });
  await expect(fork).toBeVisible();
  await fork.getByRole("button", { name: "Create independent copy", exact: true }).click();
  const review = fork.getByRole("region", { name: "Alpha copy proposal" });
  await expect(review).toBeVisible();
  await expect(review.getByRole("heading", { name: "Review the independent copy" })).toBeVisible();
  await captureRealStackScreenshot(
    page,
    scenarioInput,
    "curriculum_adoption_alpha_fork_review_laptop",
  );
  await review.getByRole("button", { name: "Apply independent copy", exact: true }).click();
  await expect(fork.getByRole("region", { name: "Independent Alpha copy complete" })).toBeVisible();
}

test.describe("curriculum adoption on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("Elena adopts, shifts, updates, and rolls over Alpha curriculum while Avery keeps an independent copy", async ({
    browser,
  }) => {
    test.setTimeout(scenarioTimeoutMs);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("curriculum_adoption");
    expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-curriculum_adoption$/u);

    const elenaOrigins: ObservedOrigins = { pageOrigins: new Set(), requestOrigins: new Set() };
    const morganOrigins: ObservedOrigins = {
      pageOrigins: new Set(),
      requestOrigins: new Set(),
    };
    const averyOrigins: ObservedOrigins = { pageOrigins: new Set(), requestOrigins: new Set() };
    const freshElenaOrigins: ObservedOrigins = {
      pageOrigins: new Set(),
      requestOrigins: new Set(),
    };
    const origins = {
      elena: elenaOrigins,
      morgan: morganOrigins,
      avery: averyOrigins,
      fresh_elena: freshElenaOrigins,
    } satisfies Record<string, ObservedOrigins>;
    const elenaContext = await browser.newContext({ ignoreHTTPSErrors: true });
    const morganContext = await browser.newContext({ ignoreHTTPSErrors: true });
    const averyContext = await browser.newContext({ ignoreHTTPSErrors: true });
    const freshElenaContext = await browser.newContext({ ignoreHTTPSErrors: true });
    let originEvidenceVerified = false;

    try {
      observeContextOrigins(elenaContext, elenaOrigins.pageOrigins, elenaOrigins.requestOrigins);
      observeContextOrigins(morganContext, morganOrigins.pageOrigins, morganOrigins.requestOrigins);
      observeContextOrigins(averyContext, averyOrigins.pageOrigins, averyOrigins.requestOrigins);
      observeContextOrigins(
        freshElenaContext,
        freshElenaOrigins.pageOrigins,
        freshElenaOrigins.requestOrigins,
      );
      const elena = await elenaContext.newPage();
      const morgan = await morganContext.newPage();
      const avery = await averyContext.newPage();
      const freshElena = await freshElenaContext.newPage();
      configureContextAndPage(elenaContext, elena, actionTimeoutMs);
      configureContextAndPage(morganContext, morgan, actionTimeoutMs);
      configureContextAndPage(averyContext, avery, actionTimeoutMs);
      configureContextAndPage(freshElenaContext, freshElena, actionTimeoutMs);

      const tag = scenarioInput.namespace;
      const questionTitle = `Curriculum adoption question ${tag}`;
      const alphaTitle = `Alpha adoption source ${tag}`;
      const sourceRevisionTwoTitle = `Alpha assignment revision two ${tag}`;
      const sourceRevisionThreeTitle = `Alpha assignment revision three ${tag}`;
      const locallyDivergentTitle = `Local divergent assignment ${tag}`;
      const destinationCourseTitle = `Alpha adoption destination ${tag}`;
      const rolloverCourseTitle = `Alpha adoption rollover ${tag}`;
      let alphaReference = "";

      await test.step("Elena visibly publishes an Alpha source with a relative schedule", async () => {
        await chooseSeededIdentity(elena, /Elena Rivera/u);
        await selectVisibleCourse(elena, baseCourseTitle);
        await createPublishedQuestion(elena, questionTitle);
        alphaReference = await createScheduledAlpha(elena, alphaTitle, questionTitle);
      });

      await test.step(
        "Morgan approves Avery, Elena invites her, and Avery accepts " +
          "before applying an independent Alpha copy",
        async () => {
          await approveAvery(morgan);
          await inviteAveryToBaseCourse(elena);
          await forkAlphaAsAvery(avery, scenarioInput, alphaTitle, alphaReference);
        },
      );

      await test.step("Elena corrects a DST proposal before instantiating the Alpha course", async () => {
        await elena.getByRole("link", { name: "Courses", exact: true }).click();
        const baseCourse = elena.getByRole("article").filter({ hasText: baseCourseTitle });
        await expect(baseCourse).toBeVisible();
        await baseCourse.getByRole("link", { name: "Open course", exact: true }).click();
        await openCurriculumChanges(elena);
        await elena.getByRole("radio", { name: /Create a course from Alpha/u }).check();
        await elena
          .getByRole("radio", {
            name: new RegExp(`${alphaTitle} ${alphaReference} · revision`, "u"),
          })
          .check();
        await elena.getByLabel("New course title").fill(destinationCourseTitle);
        await setTargetTerm(elena, initialDstTerm);
        await elena.getByRole("button", { name: "Prepare proposal", exact: true }).click();
        const recovery = elena.getByRole("region", { name: "Proposal recovery" });
        await expect(recovery).toBeVisible();
        await expect(
          recovery.getByRole("heading", { name: "Resolve the proposal blocker" }),
        ).toBeVisible();
        await expect(recovery).toContainText("nonexistent daylight-saving local time");
        await captureRealStackScreenshot(
          elena,
          scenarioInput,
          "curriculum_adoption_dst_correction_laptop",
        );
        await setTargetTerm(elena, correctedDstTerm);
        await recovery.getByRole("button", { name: "Regenerate proposal", exact: true }).click();
        const proposal = elena.getByRole("region", { name: "Server-owned curriculum proposal" });
        await expect(proposal).toBeVisible();
        await expect(proposal).toContainText(destinationCourseTitle);
        await proposal.getByRole("button", { name: "Apply live change", exact: true }).click();
        await openCompletedDestination(elena, destinationCourseTitle);
      });

      await test.step(
        "Elena inspects imports and shifts the unissued course term " +
          "through its server proposal",
        async () => {
          await openCurriculumChanges(elena);
          const evidence = await inspectImports(elena);
          await expect(evidence).toContainText("Matches its imported baseline.");
          await evidence
            .getByRole("button", { name: "Return to course changes", exact: true })
            .click();
          await elena.getByRole("radio", { name: /Shift this course term/u }).check();
          await setTargetTerm(elena, shiftedTerm);
          await elena.getByRole("button", { name: "Prepare proposal", exact: true }).click();
          const proposal = elena.getByRole("region", { name: "Server-owned curriculum proposal" });
          await expect(proposal).toBeVisible();
          await proposal.getByRole("button", { name: "Apply live change", exact: true }).click();
          await openCompletedDestination(elena, destinationCourseTitle);
        },
      );

      await test.step("Elena fast-forwards the untouched import from the next Alpha revision", async () => {
        await reviseAlphaAssignment(elena, alphaTitle, alphaReference, sourceRevisionTwoTitle);
        await elena.getByRole("link", { name: "Courses", exact: true }).click();
        const destination = elena.getByRole("article").filter({ hasText: destinationCourseTitle });
        await expect(destination).toBeVisible();
        await destination.getByRole("link", { name: "Open course", exact: true }).click();
        await openCurriculumChanges(elena);
        const evidence = await inspectImports(elena);
        await evidence
          .getByRole("button", { name: "Preview controlled update", exact: true })
          .click();
        const decision = elena.getByRole("region", {
          name: "Server-owned controlled-update decision",
        });
        await expect(decision).toBeVisible();
        await expect(decision).toContainText("can safely fast-forward");
        await captureRealStackScreenshot(
          elena,
          scenarioInput,
          "curriculum_adoption_controlled_update_decision_laptop",
        );
        await decision
          .getByRole("button", { name: "Apply controlled update", exact: true })
          .click();
        await openCompletedDestination(elena, destinationCourseTitle);
      });

      await test.step(
        "Elena preserves a local divergence and creates a new source-derived draft " +
          "from revision three",
        async () => {
          await elena.getByRole("link", { name: "Assignments", exact: true }).click();
          const imported = assignmentCard(elena, sourceRevisionTwoTitle);
          await expect(imported).toHaveCount(1);
          await imported.getByRole("link", { name: "Edit assignment", exact: true }).click();
          await expect(elena.locator('[data-route-surface="assignmentEditor"]')).toBeVisible();
          await elena.getByLabel("Assignment title").fill(locallyDivergentTitle);
          await elena
            .getByRole("button", { name: "Save title, order, and settings", exact: true })
            .click();
          await expect(
            elena
              .getByRole("status")
              .filter({ hasText: "Assignment title, order, and settings saved." }),
          ).toBeVisible();
          await reviseAlphaAssignment(elena, alphaTitle, alphaReference, sourceRevisionThreeTitle);
          await elena.getByRole("link", { name: "Courses", exact: true }).click();
          const destination = elena
            .getByRole("article")
            .filter({ hasText: destinationCourseTitle });
          await expect(destination).toBeVisible();
          await destination.getByRole("link", { name: "Open course", exact: true }).click();
          await openCurriculumChanges(elena);
          const evidence = await inspectImports(elena);
          await expect(evidence).toContainText("Has diverged from its imported baseline");
          await evidence
            .getByRole("button", { name: "Preview controlled update", exact: true })
            .click();
          const decision = elena.getByRole("region", {
            name: "Server-owned controlled-update decision",
          });
          await expect(decision).toBeVisible();
          await expect(decision).toContainText("preserved the current assignment");
          await expect(
            decision.getByRole("button", {
              name: "Create new assignment from this source definition",
              exact: true,
            }),
          ).toBeVisible();
          await captureRealStackScreenshot(
            elena,
            scenarioInput,
            "curriculum_adoption_divergent_recovery_laptop",
          );
          await decision
            .getByRole("button", {
              name: "Create new assignment from this source definition",
              exact: true,
            })
            .click();
          const sourceDerived = elena.getByRole("region", {
            name: "Server-owned curriculum proposal",
          });
          await expect(sourceDerived).toBeVisible();
          await expect(sourceDerived).toContainText(sourceRevisionThreeTitle);
          await sourceDerived
            .getByRole("button", { name: "Apply live change", exact: true })
            .click();
          await openCompletedDestination(elena, destinationCourseTitle);
        },
      );

      await test.step(
        "Elena rolls over the unissued course and a fresh context observes " +
          "durable empty learner state",
        async () => {
          await openCurriculumChanges(elena);
          const evidence = await inspectImports(elena);
          await evidence
            .getByRole("button", { name: "Return to course changes", exact: true })
            .click();
          await elena.getByRole("radio", { name: /Rollover this course/u }).check();
          await elena.getByLabel("New course title").fill(rolloverCourseTitle);
          await setTargetTerm(elena, rolloverTerm);
          await elena.getByRole("button", { name: "Prepare proposal", exact: true }).click();
          const proposal = elena.getByRole("region", { name: "Server-owned curriculum proposal" });
          await expect(proposal).toBeVisible();
          await expect(proposal).toContainText(rolloverCourseTitle);
          await proposal.getByRole("button", { name: "Apply live change", exact: true }).click();
          await openCompletedDestination(elena, rolloverCourseTitle);

          await chooseSeededIdentity(freshElena, /Elena Rivera/u);
          await selectVisibleCourse(freshElena, rolloverCourseTitle);
          await openCurriculumChanges(freshElena);
          const destinationEvidence = await inspectImports(freshElena);
          await expect(destinationEvidence).toContainText("Origin: rollover");
          await expect(destinationEvidence).toContainText("Matches its imported baseline.");
          await freshElena.getByRole("link", { name: "Students", exact: true }).click();
          const roster = freshElena.locator('[data-route-surface="courseRoster"]');
          await expect(roster).toBeVisible();
          await expect(roster).toContainText("No active students are enrolled yet.");
          await openCurriculumChanges(freshElena);
          const reopenedEvidence = await inspectImports(freshElena);
          await expect(
            reopenedEvidence.getByRole("heading", { name: "Imported curriculum evidence" }),
          ).toBeVisible();
          await captureRealStackScreenshot(
            freshElena,
            scenarioInput,
            "curriculum_adoption_completed_destination_evidence_laptop",
          );
        },
      );

      const expectedOrigin = new URL(scenarioInput.baseUrl).origin;
      expectObservedOrigin(elenaOrigins, expectedOrigin);
      expectObservedOrigin(morganOrigins, expectedOrigin);
      expectObservedOrigin(averyOrigins, expectedOrigin);
      expectObservedOrigin(freshElenaOrigins, expectedOrigin);
      originEvidenceVerified = true;
    } finally {
      try {
        await Promise.all([
          elenaContext.close(),
          morganContext.close(),
          averyContext.close(),
          freshElenaContext.close(),
        ]);
      } finally {
        if (originEvidenceVerified) writeContextOriginReceipt(origins);
      }
    }
  });
});
