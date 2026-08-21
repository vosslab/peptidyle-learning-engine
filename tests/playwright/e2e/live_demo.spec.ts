// Connected ordinary-stack proof. Selector contract: sign_in_page.tsx, course_roster_page.tsx,
// flat_question_editor_page.tsx, assignment_editor_page.tsx, course_assignments_page.tsx,
// run_page.tsx, and live_demo_sysadmin_setup_page.tsx.
// This file deliberately has no API/SQL setup: each state change is a visible user action.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { installVirtualAuthenticator } from "../helper_live_demo";
import { liveDemoOriginReceiptPathFromEnvironment } from "../live_demo_live_config";
import { tabTo } from "../simulator/keyboard_walkthrough";

const inputs = configuredLiveDemoInputs;
const maryEmail = "mary.okafor@live-demo.ple.example";
const connectedJourneyTimeoutMs = 600_000;
const connectedActionTimeoutMs = 30_000;

function requiredInputs(): NonNullable<typeof inputs> {
  if (inputs === undefined) throw new Error("connected live-demo inputs were not configured");
  return inputs;
}

function requiredSysadminOwnershipProof(): string {
  const value = requiredInputs();
  if (value.sysadminRequirement !== "unclaimed" || value.sysadminOwnershipProof === undefined) {
    throw new Error("the live-demo first-claim journey requires its private ownership proof");
  }
  return value.sysadminOwnershipProof;
}

function isoDate(offsetDays: number): string {
  const date = new Date();
  date.setDate(date.getDate() + offsetDays);
  return date.toISOString().slice(0, 10);
}

async function expectNoBlockingAxeViolations(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).include("main").analyze();
  expect(
    results.violations.filter(
      (violation) => violation.impact === "serious" || violation.impact === "critical",
    ),
  ).toEqual([]);
}

async function chooseSeededAccount(page: Page, name: RegExp): Promise<void> {
  await page.goto("/sign-in");
  await expect(
    page.getByRole("heading", { level: 1, name: "Sign in to PLE", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: new RegExp(`Continue as .*${name.source}`, "i") }).click();
  await expect(page.getByRole("heading", { name: "Choose your course" })).toBeVisible();
}

function courseChoice(page: Page, title: string): Locator {
  return page
    .getByRole("heading", { name: "Choose your course" })
    .locator("..")
    .getByRole("button")
    .filter({ hasText: title });
}

function courseChoices(page: Page): Locator {
  return page
    .getByRole("heading", { name: "Choose your course" })
    .locator("..")
    .getByRole("button");
}

async function selectCourse(page: Page, title: string): Promise<void> {
  const choice = courseChoice(page, title);
  await expect(choice).toHaveCount(1);
  await choice.click();
  await expect(page.getByRole("main")).toBeVisible();
}

async function openBaseCourse(page: Page): Promise<void> {
  await selectCourse(page, "Biochemistry Base Course");
}

async function signOut(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: "Sign in to PLE", exact: true }),
  ).toBeVisible();
}

async function closeContexts(contexts: readonly BrowserContext[]): Promise<void> {
  await Promise.all(contexts.map((context) => context.close()));
}

function configureConnectedJourneyContext(context: BrowserContext): void {
  context.setDefaultTimeout(connectedActionTimeoutMs);
  context.setDefaultNavigationTimeout(connectedActionTimeoutMs);
}

function configureConnectedJourneyPage(page: Page): void {
  page.setDefaultTimeout(connectedActionTimeoutMs);
  page.setDefaultNavigationTimeout(connectedActionTimeoutMs);
}

function recordOrigin(value: string, origins: Set<string>): void {
  if (value === "about:blank") return;
  const origin = new URL(value).origin;
  origins.add(origin);
}

function observeContextOrigins(
  context: BrowserContext,
  pageOrigins: Set<string>,
  requestOrigins: Set<string>,
): void {
  context.on("request", (request) => recordOrigin(request.url(), requestOrigins));
  context.on("page", (page) => {
    page.on("framenavigated", (frame) => {
      if (frame === page.mainFrame()) recordOrigin(frame.url(), pageOrigins);
    });
  });
}

function writeOriginReceipt(pageOrigins: Set<string>, requestOrigins: Set<string>): void {
  const receiptPath = liveDemoOriginReceiptPathFromEnvironment(process.env);
  const value = {
    pageOrigins: [...pageOrigins].sort(),
    requestOrigins: [...requestOrigins].sort(),
  };
  writeFileSync(receiptPath, JSON.stringify(value), { encoding: "ascii", flag: "wx", mode: 0o600 });
}

test.describe.configure({ mode: "serial" });

test("live demo: connected ordinary authoring, enrollment, WebAuthn, and teaching journey", async ({
  browser,
}) => {
  test.setTimeout(connectedJourneyTimeoutMs);
  const scenarioInput = requiredInputs();
  expect(scenarioInput.scenarioId).toBe("live_demo");
  const tag = scenarioInput.namespace;
  const questionTitle = `Connected single choice ${tag}`;
  const correctChoice = `Correct peptide bond ${tag}`;
  const courseTitle = `Connected course ${tag}`;
  const assignmentTitle = `Connected assignment ${tag}`;
  const contexts: BrowserContext[] = [];
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();

  try {
    const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };
    const elenaContext = await browser.newContext(contextOptions);
    const maryContext = await browser.newContext(contextOptions);
    const morganContext = await browser.newContext(contextOptions);
    const averyContext = await browser.newContext(contextOptions);
    contexts.push(elenaContext, maryContext, morganContext, averyContext);
    for (const context of contexts) {
      configureConnectedJourneyContext(context);
      observeContextOrigins(context, pageOrigins, requestOrigins);
    }
    const elena = await elenaContext.newPage();
    const mary = await maryContext.newPage();
    const morgan = await morganContext.newPage();
    const avery = await averyContext.newPage();
    for (const page of [elena, mary, morgan, avery]) configureConnectedJourneyPage(page);

    await chooseSeededAccount(elena, /Elena Rivera/u);
    await openBaseCourse(elena);
    await elena.getByRole("link", { name: "Workspace" }).click();
    await elena.getByRole("button", { name: "Create flat question" }).click();
    await elena.getByLabel("Question title").fill(questionTitle);
    await elena.getByLabel("Learner-facing prompt").fill(`Which statement is correct? ${tag}`);
    await elena.getByLabel("Choice text").nth(0).fill(correctChoice);
    await elena.getByLabel("Choice text").nth(1).fill(`Incorrect choice ${tag}`);
    await elena
      .getByRole("radio", { name: new RegExp(`Mark choice 1 as correct: ${correctChoice}`) })
      .check();
    await elena.getByRole("button", { name: "Save private draft" }).click();
    await elena.getByRole("button", { name: "Review publication changes" }).click();
    await elena.getByLabel("Reviewed public byline").fill("Dr. Elena Rivera");
    await elena.getByRole("button", { name: "Confirm and publish" }).click();
    await expect(elena.getByRole("heading", { name: "Published" })).toBeVisible();

    await elena.getByRole("link", { name: "Library", exact: true }).click();
    await elena.getByLabel("Search published questions").fill(questionTitle);
    const questionCard = elena
      .getByRole("region", { name: "Published questions" })
      .getByText(questionTitle)
      .locator("..");
    await expect(questionCard).toBeVisible();
    const questionId = await questionCard.locator("code").innerText();
    expect(questionId).toMatch(/^[A-Z0-9]{3}-[A-Z0-9]{4}$/u);

    await elena.getByRole("link", { name: "Courses" }).click();
    await elena.getByLabel("Course title").fill(courseTitle);
    await elena.getByLabel("Start date").fill(isoDate(-30));
    await elena.getByLabel("End date").fill(isoDate(365));
    await elena.getByLabel("Time zone (IANA)").fill("America/Chicago");
    await elena.getByRole("button", { name: "Create course" }).click();
    const createdCourseCard = elena
      .getByRole("article")
      .filter({ has: elena.getByRole("heading", { name: courseTitle, exact: true }) });
    await expect(createdCourseCard).toHaveCount(1);
    await createdCourseCard.getByRole("link", { name: "Open course", exact: true }).click();
    await elena.getByRole("link", { name: "Assignments" }).click();
    await elena.getByRole("link", { name: "Create the first assignment" }).click();
    await expect(elena.getByRole("heading", { name: "Create assignment" })).toBeVisible();
    await elena.getByLabel("Assignment title").fill(assignmentTitle);
    const addSeveralQuestionIds = elena.getByText("Add several Question IDs", { exact: true });
    await expect(addSeveralQuestionIds).toBeVisible();
    await tabTo(elena, addSeveralQuestionIds);
    await elena.keyboard.press("Enter");
    const questionIds = elena.getByLabel("Question IDs");
    await expect(questionIds).toBeVisible();
    await questionIds.fill(questionId);
    await elena.getByRole("button", { name: "Add questions by ID" }).click();
    await elena.getByRole("button", { name: "Create assignment" }).click();
    await expect(elena.getByText(`${assignmentTitle} now appears in this course.`)).toBeVisible();
    await elena.getByRole("link", { name: `Open ${assignmentTitle}` }).click();
    await elena.getByLabel("Lifecycle").selectOption("published");
    await elena.getByRole("button", { name: "Save teaching operations" }).click();
    await expect(elena.getByText("Published, open now.")).toBeVisible();

    await elena.getByRole("link", { name: "Students" }).click();
    await expect(elena.getByRole("heading", { name: "Students" })).toBeVisible();
    await elena.getByLabel("Institutional email").fill(maryEmail);
    await elena.getByLabel("Institutional student ID").fill(`mary-${tag}`);
    await elena.getByRole("button", { name: "Create invitation" }).click();
    const invitation = elena.getByLabel("Invitation link");
    await expect(invitation).toBeVisible();
    const invitationUrl = await invitation.inputValue();
    expect(invitationUrl).toMatch(/^https:\/\/localhost:\d+\/course-invitations\//u);
    await expectNoBlockingAxeViolations(elena);

    await chooseSeededAccount(mary, /Mary Okafor/u);
    await mary.goto(invitationUrl);
    await expect(mary.getByRole("heading", { name: "Join your PLE course" })).toBeVisible();
    await expectNoBlockingAxeViolations(mary);
    await mary.getByRole("button", { name: "Claim this course" }).click();
    await expect(mary.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
    await expect(mary.getByRole("heading", { name: "Assignments", exact: true })).toBeVisible();
    const maryAssignment = mary
      .getByRole("article")
      .filter({ has: mary.getByRole("heading", { name: assignmentTitle, exact: true }) });
    await expect(maryAssignment).toHaveCount(1);
    await maryAssignment.getByRole("link", { name: "Start assignment", exact: true }).click();
    await mary.getByRole("button", { name: "Start or continue practice" }).click();
    await mary.getByRole("radio", { name: correctChoice, exact: true }).check();
    await mary.getByRole("button", { name: "Submit answer" }).click();
    await expect(mary.getByRole("heading", { name: "Feedback" })).toBeVisible();
    await mary.getByRole("button", { name: "Continue" }).click();
    const maryRunSummary = mary.locator(".attempt-summary");
    await expect(
      maryRunSummary.getByRole("heading", { name: "Keep practicing with a fresh variation" }),
    ).toBeVisible();
    await expect(maryRunSummary.getByRole("region", { name: "Assignment score" })).toBeVisible();
    await expect(maryRunSummary.getByRole("heading", { name: "Feedback" })).toBeVisible();
    await expectNoBlockingAxeViolations(mary);
    await maryRunSummary.getByRole("button", { name: "Start another practice" }).click();
    await expect(mary.getByRole("button", { name: "Submit answer" })).toBeVisible();

    await installVirtualAuthenticator(morgan);
    await morgan.goto("/live-demo/sysadmin-setup");
    await expect(
      morgan.getByRole("heading", { name: "Set up administrator access" }),
    ).toBeVisible();
    await expectNoBlockingAxeViolations(morgan);
    await morgan.getByLabel("Administrator setup code").fill(requiredSysadminOwnershipProof());
    await morgan.getByLabel("Passkey name").fill(`Morgan passkey ${tag}`);
    await morgan.getByRole("button", { name: "Set up administrator passkey" }).click();
    const recoveredCourseHeading = morgan.getByRole("heading", { name: "Choose your course" });
    await expect(recoveredCourseHeading).toBeVisible();
    await expect(recoveredCourseHeading).toBeFocused();
    await expect(courseChoices(morgan)).toHaveCount(1);
    await expect(courseChoice(morgan, "Genetics Practice Course")).toHaveCount(1);
    await expect(courseChoice(morgan, "Biochemistry Base Course")).toHaveCount(0);
    await expect(courseChoice(morgan, courseTitle)).toHaveCount(0);
    await expectNoBlockingAxeViolations(morgan);
    const terminalSetup = await morganContext.newPage();
    configureConnectedJourneyPage(terminalSetup);
    await terminalSetup.goto("/live-demo/sysadmin-setup");
    await expect(terminalSetup.getByText("Administrator setup is already complete.")).toBeVisible();
    await terminalSetup.close();
    await selectCourse(morgan, "Genetics Practice Course");
    await morgan.getByRole("link", { name: "Teaching operations" }).click();
    await expect(morgan.getByRole("heading", { name: "Instructor approval" })).toBeVisible();
    await morgan.getByLabel("Find an account by name").fill("Avery");
    await morgan.getByRole("button", { name: "Search accounts" }).click();
    await morgan
      .getByRole("listitem")
      .filter({ hasText: "Avery Singh" })
      .getByRole("button", { name: "Approve as instructor" })
      .click();
    await morgan.getByRole("dialog").getByRole("button", { name: "Approve as instructor" }).click();
    await expect(morgan.getByText(/Avery Singh.*approved/u)).toBeVisible();

    await elena.getByRole("link", { name: "Teaching operations" }).click();
    await expect(elena.getByRole("heading", { name: "Teaching team" })).toBeVisible();
    await elena.getByLabel("Find an approved colleague").fill("Avery");
    await elena.getByRole("button", { name: "Search eligible people" }).click();
    await elena
      .getByRole("listitem")
      .filter({ hasText: "Avery Singh" })
      .getByRole("button", { name: "Select" })
      .click();
    await elena.getByRole("button", { name: "Invite selected colleague" }).click();
    const teachingTeam = elena.getByRole("region", { name: "Teaching team" });
    await expect(teachingTeam.getByRole("status")).toHaveText(
      "An invitation was created for Avery Singh.",
    );
    const pendingInvitations = teachingTeam
      .getByRole("heading", { name: "Pending invitations", exact: true })
      .locator("..");
    const averyInvitation = pendingInvitations
      .getByRole("article")
      .filter({ has: elena.locator("strong", { hasText: /^Avery Singh$/u }) });
    await expect(averyInvitation).toHaveCount(1);
    await expect(averyInvitation.locator("strong")).toHaveText("Avery Singh");
    await expect(averyInvitation.locator("p")).toContainText("Pending response.");

    await chooseSeededAccount(avery, /Avery Singh/u);
    await expect(courseChoices(avery)).toHaveCount(1);
    await expect(courseChoice(avery, "Genetics Practice Course")).toHaveCount(1);
    await expect(courseChoice(avery, "Biochemistry Base Course")).toHaveCount(0);
    await expect(courseChoice(avery, courseTitle)).toHaveCount(0);
    await selectCourse(avery, "Genetics Practice Course");
    await avery.getByRole("link", { name: "Invitations", exact: true }).click();
    await expect(
      avery.getByRole("heading", { name: "Pending teaching invitations" }),
    ).toBeVisible();
    await avery.getByRole("button", { name: "Accept" }).click();
    await avery.getByRole("dialog").getByRole("button", { name: "Accept invitation" }).click();
    const acceptedInvitationStatus = avery.getByRole("main").getByRole("status");
    const pendingInvitationHeading = avery.getByRole("heading", {
      name: "Pending teaching invitations",
    });
    await expect(acceptedInvitationStatus).toHaveText("Invitation accepted.");
    await expect(avery.getByRole("heading", { name: "No invitations waiting" })).toBeVisible();
    await expect(pendingInvitationHeading).toBeFocused();
    await signOut(avery);
    await chooseSeededAccount(avery, /Avery Singh/u);
    // This is the contract's primary persistence observation: Avery's fresh authorized
    // session sees the teaching-team role that the visible invitation acceptance created.
    await expect(courseChoice(avery, courseTitle)).toBeVisible();
    await selectCourse(avery, courseTitle);
    await avery.getByRole("link", { name: "Teaching operations" }).click();
    await expect(avery.getByRole("heading", { name: "Teaching operations" })).toBeVisible();
    await expect(avery.getByRole("heading", { name: "Teaching team" })).toBeVisible();

    await signOut(morgan);
    await morgan.getByRole("button", { name: "Sign in with a passkey" }).click();
    await expect(morgan.getByRole("heading", { name: "Choose your course" })).toBeVisible();
  } finally {
    try {
      await closeContexts(contexts);
    } finally {
      writeOriginReceipt(pageOrigins, requestOrigins);
    }
  }
});
