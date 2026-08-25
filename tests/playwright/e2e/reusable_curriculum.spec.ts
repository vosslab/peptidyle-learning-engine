// Real-stack WP-PROF-B1 reusable curriculum journey.
//
// Selector contract:
// - src/features/reusable_curriculum/reusable_curriculum_workspace.tsx owns workspace and editor names.
// - src/features/reusable_curriculum/reusable_curriculum_create_dialog.tsx owns create-dialog controls.
// - src/features/problem_picker/problem_picker.tsx owns the shared picker dialog.
// - src/pages/assignment_editor_page.tsx owns ordinary assignment authoring controls.

import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { installVirtualAuthenticator, removeVirtualAuthenticator } from "../helper_live_demo";
import { CORPUS_VIEWPORT_SIZES } from "../ui_corpus_manifest";
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
const baseCourseTitle = "Biochemistry Base Course";

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

async function captureResponsive(
  page: Page,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
  target: Locator,
  artifactPrefix: string,
  anchor: Locator = target,
): Promise<void> {
  for (const viewport of ["laptop", "tablet", "iphone_pro", "square"] as const) {
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

async function signInWithPasskey(
  page: Page,
  namespace: string,
): Promise<Awaited<ReturnType<typeof installVirtualAuthenticator>>> {
  const authenticator = await installVirtualAuthenticator(page);
  await chooseSeededIdentity(page, /Elena Rivera/u);
  await selectVisibleCourse(page, baseCourseTitle);
  await page.getByRole("link", { name: "Account", exact: true }).click();
  const account = page.locator('[data-route-surface="accountSecurity"]');
  await expect(account).toBeVisible();
  await account.getByLabel("Passkey name").fill(`Elena B1 key ${namespace}`);
  await account.getByRole("button", { name: "Add passkey", exact: true }).click();
  await expect(account.getByRole("status")).toHaveText("Passkey added.");
  await signOutVisible(page);
  await page.getByRole("button", { name: "Sign in with a passkey", exact: true }).click();
  await selectVisibleCourse(page, baseCourseTitle);
  return authenticator;
}

async function chooseQuestion(
  page: Page,
  pickerName: string,
  sourceLabel: string,
  confirmLabel: string,
  questionTitle: string,
): Promise<void> {
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

async function createPublicQuestion(
  page: Page,
  questionTitle: string,
  namespace: string,
): Promise<string> {
  await page.getByRole("link", { name: "Workspace", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Draft, preview, and publish a learning question" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Create flat question", exact: true }).click();
  await page.getByLabel("Question title").fill(questionTitle);
  await page
    .getByLabel("Learner-facing prompt")
    .fill(`Which statements describe peptide-bond planarity? ${namespace}`);
  await page.getByLabel("Question format").selectOption("multipleAnswer");
  await page.getByLabel("Choice text").nth(0).fill(`Resonance restricts rotation ${namespace}`);
  await page
    .getByLabel("Choice text")
    .nth(1)
    .fill(`The peptide bond is usually planar ${namespace}`);
  await page.getByRole("button", { name: "Add choice", exact: true }).click();
  await page.getByLabel("Choice text").nth(2).fill(`Peptide bonds freely rotate ${namespace}`);
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
    const origins = { elena: elenaOrigins } satisfies Record<string, ObservedOrigins>;
    const curriculumWire: CurriculumWireValue[] = [];
    const pendingResponses: Promise<void>[] = [];
    const context = await browser.newContext({ ignoreHTTPSErrors: true });
    let authenticator: Awaited<ReturnType<typeof installVirtualAuthenticator>> | undefined;
    let originEvidenceVerified = false;
    try {
      observeContextOrigins(context, elenaOrigins.pageOrigins, elenaOrigins.requestOrigins);
      observeCurriculumWire(context, curriculumWire, pendingResponses);
      const page = await context.newPage();
      configureContextAndPage(context, page, actionTimeoutMs);
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

      await test.step("Elena completes ordinary passkey entry and creates the reusable definitions through the visible shared picker", async () => {
        authenticator = await signInWithPasskey(page, scenarioInput.namespace);
        publishedQuestionId = await createPublicQuestion(
          page,
          questionTitle,
          scenarioInput.namespace,
        );
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
        await captureResponsive(
          page,
          scenarioInput,
          alphaDetail,
          "reusable_curriculum_alpha_editor",
          alphaDetail.getByRole("heading", { level: 1, name: revisedAlphaTitle, exact: true }),
        );

        const remote = await context.newPage();
        configureContextAndPage(context, remote, actionTimeoutMs);
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
        await captureResponsive(
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

      await test.step("Elena reuses the Alpha definition through the ordinary assignment picker", async () => {
        await createCourse(page, courseTitle);
        await page.getByRole("link", { name: "Assignments", exact: true }).click();
        await page.getByRole("link", { name: "Create the first assignment", exact: true }).click();
        const editor = page.locator('[data-route-surface="assignmentEditor"]');
        await expect(editor).toBeVisible();
        await editor.getByLabel("Assignment title").fill(assignmentTitle);
        await editor.getByRole("button", { name: "Choose questions", exact: true }).click();
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
        await chooseQuestion(
          page,
          "Choose assignment questions",
          alphaSourceLabel,
          "Add selected questions",
          questionTitle,
        );
        await expect(editor).toContainText(questionTitle);
        await captureResponsive(
          page,
          scenarioInput,
          editor,
          "reusable_curriculum_alpha_reuse",
          editor.getByRole("heading", { name: "Assignment content", exact: true }),
        );
        await editor.getByRole("button", { name: "Create assignment", exact: true }).click();
        await expect(
          editor.getByRole("heading", { name: "Assignment created", exact: true }),
        ).toBeVisible();
      });

      await Promise.all(pendingResponses);
      expect(curriculumWire.some((value) => value.direction === "request")).toBe(true);
      expect(curriculumWire.some((value) => value.direction === "response")).toBe(true);
      expectObservedOrigin(elenaOrigins, new URL(scenarioInput.baseUrl).origin);
      originEvidenceVerified = true;
    } finally {
      try {
        if (authenticator !== undefined) await removeVirtualAuthenticator(authenticator);
      } finally {
        await context.close();
        if (originEvidenceVerified) writeContextOriginReceipt(origins);
      }
    }
  });
});
