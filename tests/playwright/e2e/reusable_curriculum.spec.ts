// Real-stack WP-INST-B1 reusable curriculum journey.
//
// Selector contract:
// - src/features/reusable_curriculum/reusable_curriculum_workspace.tsx owns workspace and editor names.
// - src/features/reusable_curriculum/reusable_curriculum_create_dialog.tsx owns create-dialog controls.
// - src/features/problem_picker/problem_picker.tsx owns the shared picker dialog.
// - src/pages/assignment_workspace/ owns the focused assignment Questions workflow.

import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { CORPUS_VIEWPORT_SIZES } from "../ui_corpus_manifest";
import { BIOCHEMISTRY_COURSE_TITLE } from "./helper_course_titles";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  expectObservedOrigin,
  observeContextOrigins,
  relativeIsoDate,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
  writeContextOriginReceipt,
  type ObservedOrigins,
} from "./real_stack_ui";

const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 600_000;
interface CurriculumWireValue {
  readonly direction: "request" | "response";
  readonly path: string;
  readonly value: unknown;
}

async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  const dimensions = await page.evaluate(() => ({
    content: document.documentElement.scrollWidth,
    viewport: window.innerWidth,
  }));
  expect(dimensions.content).toBeLessThanOrEqual(dimensions.viewport);
}

async function captureLaptop(
  page: Page,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
  target: Locator,
  artifactPrefix: string,
  anchor: Locator = target,
): Promise<void> {
  for (const viewport of ["laptop"] as const) {
    await page.setViewportSize(CORPUS_VIEWPORT_SIZES[viewport]);
    await anchor.evaluate((element) =>
      element.scrollIntoView({ block: "start", inline: "nearest" }),
    );
    await expect(anchor).toBeVisible();
    await expect(target).toBeVisible();
    await expectNoHorizontalOverflow(page);
    await captureRealStackScreenshot(page, scenarioInput, `${artifactPrefix}_${viewport}`);
  }
  await page.setViewportSize(CORPUS_VIEWPORT_SIZES.laptop);
}

function curriculumPath(url: string): string | null {
  const path = new URL(url).pathname;
  return path.startsWith("/api/course-blueprints") || path.startsWith("/api/alpha-courses")
    ? path
    : null;
}

function observeCurriculumWire(
  context: BrowserContext,
  values: CurriculumWireValue[],
  pendingResponses: Promise<void>[],
): void {
  context.on("request", (request) => {
    const path = curriculumPath(request.url());
    const body = request.postData();
    if (path === null || body === null) return;
    values.push({ direction: "request", path, value: JSON.parse(body) });
  });
  context.on("response", (response) => {
    const path = curriculumPath(response.url());
    if (path === null || response.status() >= 400) return;
    pendingResponses.push(
      response.text().then((body) => {
        if (body !== "") values.push({ direction: "response", path, value: JSON.parse(body) });
      }),
    );
  });
}

async function enterSeededInstructorCourse(page: Page): Promise<void> {
  await chooseSeededIdentity(page, /Elena Rivera/u);
  await selectVisibleCourse(page, BIOCHEMISTRY_COURSE_TITLE);
}

async function selectQuestion(
  page: Page,
  pickerName: string,
  sourceLabel: string,
  questionTitle: string,
): Promise<Locator> {
  const picker = page.getByRole("dialog", { name: pickerName, exact: true });
  await expect(picker).toBeVisible();
  await picker.getByLabel("Question source").selectOption({ label: sourceLabel });
  await picker.getByLabel("Search questions").fill(questionTitle);
  await picker.getByRole("button", { name: "Search questions", exact: true }).click();
  const choice = picker
    .getByRole("region", { name: "Question results", exact: true })
    .getByRole("checkbox")
    .first();
  await expect(choice).toBeVisible();
  await choice.check();
  await expect(choice).toBeChecked();
  await expect(
    picker.getByRole("region", { name: "Selected questions", exact: true }),
  ).toContainText(questionTitle);
  return picker;
}

async function chooseQuestion(
  page: Page,
  pickerName: string,
  sourceLabel: string,
  confirmLabel: string,
  questionTitle: string,
): Promise<void> {
  const picker = await selectQuestion(page, pickerName, sourceLabel, questionTitle);
  await picker.getByRole("button", { name: confirmLabel, exact: true }).click();
  await expect(picker).toHaveCount(0);
}

async function createReusable(
  page: Page,
  kind: "blueprint" | "alpha",
  title: string,
  questionTitle: string,
  sourceLabel: string,
): Promise<void> {
  await page
    .getByRole("button", {
      name: kind === "blueprint" ? "Create blueprint" : "Create Alpha curriculum",
      exact: true,
    })
    .click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await dialog
    .getByLabel(kind === "blueprint" ? "Assignment title" : "Curriculum title")
    .fill(title);
  await dialog.getByRole("button", { name: "Choose published questions", exact: true }).click();
  await chooseQuestion(
    page,
    "Choose the first reusable questions",
    sourceLabel,
    "Use selected questions",
    questionTitle,
  );
  await dialog.getByRole("button", { name: "Create live curriculum", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: title,
      exact: true,
    }),
  ).toBeVisible();
}

async function createPublicQuestion(page: Page, questionTitle: string): Promise<string> {
  await page.getByRole("link", { name: "Workspace", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Draft, preview, and publish a learning question" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Create flat question", exact: true }).click();
  await page.getByLabel("Question title").fill(questionTitle);
  await page
    .getByLabel("Learner-facing prompt")
    .fill("Which statements describe peptide-bond planarity?");
  await page.getByLabel("Question format").selectOption("multipleAnswer");
  await page.getByLabel("Choice text").nth(0).fill("Resonance restricts rotation");
  await page.getByLabel("Choice text").nth(1).fill("The peptide bond is usually planar");
  await page.getByRole("button", { name: "Add choice", exact: true }).click();
  await page.getByLabel("Choice text").nth(2).fill("Peptide bonds freely rotate");
  const correctAnswers = page.getByRole("checkbox", { name: "Correct answer", exact: true });
  await correctAnswers.nth(0).check();
  await correctAnswers.nth(1).check();
  await page.getByRole("button", { name: "Save private draft", exact: true }).click();
  await expect(page.getByRole("status", { name: "Private draft status" })).toHaveText(
    "Private draft saved. It is not published.",
  );
  await page.getByRole("button", { name: "Review publication changes", exact: true }).click();
  await page.getByLabel("Publication scope").selectOption({ label: "Public" });
  await page.getByLabel("Reviewed public byline").fill("Dr. Elena Rivera");
  await page.getByRole("button", { name: "Confirm and publish", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Published", exact: true })).toBeVisible();
  const publicationResult = page.getByRole("status").filter({ hasText: questionTitle });
  await expect(publicationResult).toContainText("Question ID:");
  await expect(publicationResult).toContainText("Published to: Public library");
  await expect(publicationResult).toContainText("By: Dr. Elena Rivera");
  const questionId = (await publicationResult.locator("code").textContent())?.trim();
  if (questionId === undefined || !/^[A-Z0-9]+(?:-[A-Z0-9]+)+$/u.test(questionId)) {
    throw new Error("Published question has no canonical public Question ID");
  }
  return questionId;
}

async function createCourse(page: Page, title: string): Promise<void> {
  await page.getByRole("link", { name: "Courses", exact: true }).click();
  const courses = page.locator('[data-route-surface="courses"]');
  await expect(courses).toBeVisible();
  await courses.getByLabel("Course title").fill(title);
  await courses.getByLabel("Start date").fill(relativeIsoDate(-1));
  await courses.getByLabel("End date").fill(relativeIsoDate(30));
  await courses.getByLabel("Time zone (IANA)").fill("America/Chicago");
  await courses.getByRole("button", { name: "Create course", exact: true }).click();
  const course = courses.getByRole("article").filter({ hasText: title });
  await expect(course).toBeVisible();
  await course.getByRole("link", { name: "Open course", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: title, exact: true })).toBeVisible();
}

test.describe("reusable curriculum on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("Instructor revises a personal blueprint and public Alpha curriculum, then reuses Alpha questions in assignment authoring", async ({
    browser,
  }) => {
    test.setTimeout(scenarioTimeoutMs);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("reusable_curriculum");
    expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-reusable_curriculum$/u);

    const elenaOrigins: ObservedOrigins = {
      pageOrigins: new Set<string>(),
      requestOrigins: new Set<string>(),
    };
    const morganOrigins: ObservedOrigins = {
      pageOrigins: new Set<string>(),
      requestOrigins: new Set<string>(),
    };
    const averyOrigins: ObservedOrigins = {
      pageOrigins: new Set<string>(),
      requestOrigins: new Set<string>(),
    };
    const origins = {
      elena: elenaOrigins,
      morgan: morganOrigins,
      avery: averyOrigins,
    } satisfies Record<string, ObservedOrigins>;
    const curriculumWire: CurriculumWireValue[] = [];
    const pendingResponses: Promise<void>[] = [];
    const elenaContext = await browser.newContext({ ignoreHTTPSErrors: true });
    const morganContext = await browser.newContext({ ignoreHTTPSErrors: true });
    const averyContext = await browser.newContext({ ignoreHTTPSErrors: true });
    let originEvidenceVerified = false;
    try {
      observeContextOrigins(elenaContext, elenaOrigins.pageOrigins, elenaOrigins.requestOrigins);
      observeContextOrigins(morganContext, morganOrigins.pageOrigins, morganOrigins.requestOrigins);
      observeContextOrigins(averyContext, averyOrigins.pageOrigins, averyOrigins.requestOrigins);
      observeCurriculumWire(elenaContext, curriculumWire, pendingResponses);
      const page = await elenaContext.newPage();
      configureContextAndPage(elenaContext, page, actionTimeoutMs);
      const blueprintTitle = "Peptide-bond essentials blueprint";
      const alphaTitle = "Peptide-bond foundations";
      const questionTitle = "Peptide-bond planarity";
      const revisedBlueprintTitle = `${blueprintTitle} revised`;
      const revisedAlphaTitle = `${alphaTitle} revised`;
      const remoteAlphaTitle = `${alphaTitle} updated by another tab`;
      const localAlphaDraftTitle = `${alphaTitle} local draft`;
      const courseTitle = "Molecular structure seminar";
      const assignmentTitle = "Peptide-bond Alpha practice";
      let publishedQuestionId = "";

      await test.step("Elena enters the seeded Instructor session and creates reusable definitions through the visible shared picker", async () => {
        await enterSeededInstructorCourse(page);
        publishedQuestionId = await createPublicQuestion(page, questionTitle);
        await page.getByRole("link", { name: "Curriculum", exact: true }).click();
        const workspace = page.locator('[data-route-surface="curriculum"]');
        await expect(workspace).toBeVisible();
        const blueprintTrigger = workspace.getByRole("button", {
          name: "Create blueprint",
          exact: true,
        });
        await blueprintTrigger.click();
        await expect(page.getByRole("dialog", { name: "Create a blueprint" })).toBeVisible();
        await page.keyboard.press("Escape");
        await expect(page.getByRole("dialog", { name: "Create a blueprint" })).toHaveCount(0);
        await expect(blueprintTrigger).toBeFocused();
        await createReusable(page, "blueprint", blueprintTitle, questionTitle, "Current library");
        const blueprintDetail = page.locator('[data-route-surface="curriculumDetail"]');
        await page.getByLabel("Assignment title").fill(revisedBlueprintTitle);
        await expect(
          blueprintDetail.getByRole("button", { name: "Discard local changes", exact: true }),
        ).toBeVisible();
        await page.getByRole("button", { name: "Save curriculum", exact: true }).click();
        await expect(blueprintDetail.getByRole("status")).toContainText("Blueprint saved");
        await page.getByRole("link", { name: "Return to all curricula", exact: true }).click();
        await expect(workspace).toBeVisible();
        await createReusable(page, "alpha", alphaTitle, questionTitle, "Public library");
        const alphaDetail = page.locator('[data-route-surface="curriculumDetail"]');
        await page.getByLabel("Curriculum title").fill(revisedAlphaTitle);
        await page.getByRole("button", { name: "Save curriculum", exact: true }).click();
        await expect(alphaDetail.getByRole("status")).toContainText("Alpha curriculum saved");
        await captureLaptop(
          page,
          scenarioInput,
          alphaDetail,
          "reusable_curriculum_alpha_editor",
          alphaDetail.getByRole("heading", { level: 1, name: revisedAlphaTitle, exact: true }),
        );

        const remote = await elenaContext.newPage();
        configureContextAndPage(elenaContext, remote, actionTimeoutMs);
        await remote.goto(page.url());
        const remoteDetail = remote.locator('[data-route-surface="curriculumDetail"]');
        await expect(
          remoteDetail.getByRole("heading", { level: 1, name: revisedAlphaTitle, exact: true }),
        ).toBeVisible();
        await remote.getByLabel("Curriculum title").fill(remoteAlphaTitle);
        await remote.getByRole("button", { name: "Save curriculum", exact: true }).click();
        await expect(remoteDetail.getByRole("status")).toContainText("Alpha curriculum saved");
        await remote.close();

        await page.getByLabel("Curriculum title").fill(localAlphaDraftTitle);
        await page.getByRole("button", { name: "Save curriculum", exact: true }).click();
        await expect(alphaDetail.getByRole("alert")).toContainText("newer curriculum version");
        await expect(page.getByLabel("Curriculum title")).toHaveValue(localAlphaDraftTitle);
        await expect(
          alphaDetail.getByRole("button", { name: "Reload current version", exact: true }),
        ).toBeVisible();
        await alphaDetail.getByRole("button", { name: "Keep my draft", exact: true }).click();
        await expect(page.getByLabel("Curriculum title")).toHaveValue(localAlphaDraftTitle);
        await alphaDetail
          .getByRole("button", { name: "Discard local changes", exact: true })
          .click();
        await expect(
          alphaDetail.getByRole("heading", { level: 1, name: remoteAlphaTitle, exact: true }),
        ).toBeVisible();
      });

      await test.step("A reload preserves both revised current definitions", async () => {
        await page.reload();
        await expect(
          page.getByRole("heading", { level: 1, name: remoteAlphaTitle, exact: true }),
        ).toBeVisible();
        await page.getByRole("link", { name: "Return to all curricula", exact: true }).click();
        await expect(
          page.getByRole("link", { name: new RegExp(revisedBlueprintTitle, "u") }),
        ).toBeVisible();
        await expect(
          page.getByRole("link", { name: new RegExp(remoteAlphaTitle, "u") }),
        ).toBeVisible();
        const workspace = page.locator('[data-route-surface="curriculum"]');
        await captureLaptop(
          page,
          scenarioInput,
          workspace,
          "reusable_curriculum_workspace",
          workspace.getByRole("heading", {
            level: 1,
            name: "Build once, adapt for each course",
            exact: true,
          }),
        );
        await workspace.getByRole("link", { name: new RegExp(revisedBlueprintTitle, "u") }).click();
        const blueprintDetail = page.locator('[data-route-surface="curriculumDetail"]');
        await expect(
          blueprintDetail.getByRole("heading", {
            level: 1,
            name: revisedBlueprintTitle,
            exact: true,
          }),
        ).toBeVisible();
        await expect(blueprintDetail).toContainText(publishedQuestionId);
        await blueprintDetail
          .getByRole("link", { name: "Return to all curricula", exact: true })
          .click();
      });

      await test.step("Morgan confirms Avery's approval and Elena confirms her installed Biochemistry course teaching access", async () => {
        const morgan = await morganContext.newPage();
        configureContextAndPage(morganContext, morgan, actionTimeoutMs);
        await chooseSeededIdentity(morgan, /Morgan/u);
        await selectVisibleCourse(morgan, "Genetics Practice Course");
        await morgan.getByRole("link", { name: "Teaching operations" }).click();
        await expect(morgan.getByRole("heading", { name: "Instructor approval" })).toBeVisible();
        await morgan.getByLabel("Find an account by name").fill("Avery");
        await morgan.getByRole("button", { name: "Search accounts" }).click();
        const averyCandidate = morgan.getByRole("listitem").filter({ hasText: "Avery Singh" });
        await expect(averyCandidate).toBeVisible();
        const approveAvery = averyCandidate.getByRole("button", {
          name: "Approve as instructor",
          exact: true,
        });
        if ((await approveAvery.count()) === 1) {
          await approveAvery.click();
          await morgan
            .getByRole("dialog")
            .getByRole("button", { name: "Approve as instructor" })
            .click();
          await expect(morgan.getByText(/Avery Singh.*eligible/u)).toBeVisible();
        } else {
          await expect(averyCandidate).toContainText("Approved for invitations");
        }

        await page.getByRole("link", { name: "Courses", exact: true }).click();
        const baseCourse = page.getByRole("article").filter({ hasText: BIOCHEMISTRY_COURSE_TITLE });
        await expect(baseCourse).toBeVisible();
        await baseCourse.getByRole("link", { name: "Open course", exact: true }).click();
        await page.getByRole("link", { name: "Teaching operations", exact: true }).click();
        const teachingTeam = page.getByRole("region", { name: "Teaching team" });
        await expect(teachingTeam).toBeVisible();
        const activeInstructors = teachingTeam.getByRole("region", {
          name: "Active instructors",
          exact: true,
        });
        const activeAvery = activeInstructors
          .getByRole("article")
          .filter({ hasText: "Avery Singh" });
        if ((await activeAvery.count()) === 1) {
          await expect(activeAvery).toContainText("Active direct instructor");
        } else {
          await teachingTeam.getByLabel("Find an approved colleague").fill("Avery");
          await teachingTeam.getByRole("button", { name: "Search eligible people" }).click();
          const eligibleAvery = teachingTeam
            .getByRole("list", {
              name: "Eligible co-instructor search results",
              exact: true,
            })
            .getByRole("listitem")
            .filter({ hasText: "Avery Singh" });
          const pendingInvitations = teachingTeam.getByRole("region", {
            name: "Pending invitations",
            exact: true,
          });
          const pendingAvery = pendingInvitations
            .getByRole("article")
            .filter({ hasText: "Avery Singh" });
          await expect(eligibleAvery.or(pendingAvery)).toBeVisible();
          if (await eligibleAvery.isVisible()) {
            await eligibleAvery.getByRole("button", { name: "Select", exact: true }).click();
            await teachingTeam.getByRole("button", { name: "Invite selected colleague" }).click();
            await expect(teachingTeam.getByRole("status")).toHaveText(
              "An invitation was created for Avery Singh.",
            );
          } else {
            await expect(pendingAvery).toBeVisible();
          }
        }
      });

      await test.step("Avery accepts Elena's invitation and inspects the Alpha question set as an approved reader", async () => {
        const avery = await averyContext.newPage();
        configureContextAndPage(averyContext, avery, actionTimeoutMs);
        await chooseSeededIdentity(avery, /Avery Singh/u);
        await selectVisibleCourse(avery, "Genetics Practice Course");
        await avery.getByRole("link", { name: "Invitations", exact: true }).click();
        await expect(
          avery.getByRole("heading", { name: "Pending teaching invitations", exact: true }),
        ).toBeVisible();
        const acceptInvitation = avery
          .getByRole("article")
          .filter({ hasText: BIOCHEMISTRY_COURSE_TITLE })
          .getByRole("button", { name: "Accept", exact: true });
        const noInvitations = avery.getByRole("heading", { name: "No invitations waiting" });
        await expect(acceptInvitation.or(noInvitations)).toBeVisible();
        if (await acceptInvitation.isVisible()) {
          await acceptInvitation.click();
          await avery
            .getByRole("dialog")
            .getByRole("button", { name: "Accept invitation" })
            .click();
          await expect(avery.getByRole("main").getByRole("status")).toHaveText(
            "Invitation accepted.",
          );
        } else {
          await expect(noInvitations).toBeVisible();
        }
        await signOutVisible(avery);
        await chooseSeededIdentity(avery, /Avery Singh/u);
        await selectVisibleCourse(avery, BIOCHEMISTRY_COURSE_TITLE);
        await avery.getByRole("link", { name: "Curriculum", exact: true }).click();
        const readerWorkspace = avery.locator('[data-route-surface="curriculum"]');
        await expect(readerWorkspace).toBeVisible();
        await readerWorkspace
          .getByRole("link", { name: new RegExp(remoteAlphaTitle, "u") })
          .click();
        const readerDetail = avery.locator('[data-route-surface="curriculumDetail"]');
        await expect(
          readerDetail.getByRole("heading", { level: 1, name: remoteAlphaTitle, exact: true }),
        ).toBeVisible();
        const inspection = readerDetail.getByRole("region", {
          name: "Inspect and reuse this question set",
          exact: true,
        });
        await expect(
          inspection.getByRole("heading", { name: "Module 1 assignment", exact: true }),
        ).toBeVisible();
        await expect(inspection).toContainText(publishedQuestionId);
        await expect(readerDetail.getByLabel("Curriculum title")).toHaveCount(0);
        await expect(
          readerDetail.getByRole("button", { name: "Save curriculum", exact: true }),
        ).toHaveCount(0);
        await captureLaptop(
          avery,
          scenarioInput,
          inspection,
          "reusable_curriculum_alpha_reader_inspection",
          readerDetail.getByRole("heading", { level: 1, name: remoteAlphaTitle, exact: true }),
        );
      });

      await test.step("Elena reuses the Alpha definition through the ordinary assignment picker", async () => {
        await createCourse(page, courseTitle);
        await page.getByRole("link", { name: "Assignments", exact: true }).click();
        await page.getByRole("link", { name: "Create the first assignment", exact: true }).click();
        const createDraft = page.locator('[data-route-surface="assignmentCreate"]');
        await expect(createDraft).toBeVisible();
        await createDraft.getByLabel("Assignment title").fill(assignmentTitle);
        await createDraft
          .getByRole("button", { name: "Create assignment draft", exact: true })
          .click();
        const workspace = page.locator('[data-route-surface="assignmentWorkspace"]');
        await expect(
          workspace.getByRole("heading", { name: "Questions", exact: true }),
        ).toBeVisible();
        await workspace
          .getByRole("button", { name: "Search question library", exact: true })
          .click();
        const picker = page.getByRole("dialog", {
          name: "Choose assignment questions",
          exact: true,
        });
        const alphaSource = picker
          .getByLabel("Question source")
          .locator("option")
          .filter({ hasText: remoteAlphaTitle });
        await expect(alphaSource).toHaveCount(1);
        const alphaSourceLabel = await alphaSource.textContent();
        if (alphaSourceLabel === null) throw new Error("Alpha picker source has no label");
        await selectQuestion(page, "Choose assignment questions", alphaSourceLabel, questionTitle);
        await captureLaptop(
          page,
          scenarioInput,
          picker,
          "reusable_curriculum_alpha_reuse",
          picker.getByRole("heading", { name: "Choose assignment questions", exact: true }),
        );
        await picker.getByRole("button", { name: "Add selected questions", exact: true }).click();
        await expect(picker).toHaveCount(0);
        await expect(workspace).toContainText(questionTitle);
        await workspace
          .getByRole("button", { name: "Save questions and order", exact: true })
          .click();
        await expect(
          workspace.getByRole("status").filter({ hasText: "Questions and order saved." }),
        ).toBeVisible();
      });

      await Promise.all(pendingResponses);
      expect(curriculumWire.some((value) => value.direction === "request")).toBe(true);
      expect(curriculumWire.some((value) => value.direction === "response")).toBe(true);
      expectObservedOrigin(elenaOrigins, new URL(scenarioInput.baseUrl).origin);
      originEvidenceVerified = true;
    } finally {
      await Promise.all([elenaContext.close(), morganContext.close(), averyContext.close()]);
      if (originEvidenceVerified) writeContextOriginReceipt(origins);
    }
  });
});
