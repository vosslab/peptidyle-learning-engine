// Capture current PLE surfaces through the seeded Live Demo execution environment.

import path from "node:path";
import process from "node:process";
import { mkdir, readFile, stat } from "node:fs/promises";

import { chromium } from "playwright";

import { REPO_ROOT } from "./repo_root.mjs";

const screenshotsDirectory = path.join(REPO_ROOT, "docs/screenshots");
const manifestPath = path.join(screenshotsDirectory, "current_capture_manifest.json");
const laptopViewport = { width: 1280, height: 800 };
const tabletViewport = { width: 768, height: 1024 };
const phoneViewport = { width: 390, height: 844 };
const squareViewport = { width: 800, height: 800 };
const sensitiveScreenTerms = [
  /presentation nonce/iu,
  /source object/iu,
  /source checksum/iu,
  /answer key/iu,
  /correct answer/iu,
  /correct feedback/iu,
  /student record/iu,
  /access token/iu,
];
const protectedResponseKeys = new Set([
  "answerkey",
  "correctanswer",
  "correctfeedback",
  "solution",
  "sourceobject",
  "sourcechecksum",
  "questionattemptid",
  "accesstoken",
  "studentrecord",
]);
const studentOnlyProtectedResponseKeys = new Set([
  "answer",
  "correct",
  "correctness",
  "score",
  "scoretotal",
  "pointsearned",
  "pointpossible",
  "questionanswer",
  "questionanswerexplanation",
  "choicefeedback",
  "incorrectfeedback",
]);

function requireEntryUrl(argument) {
  if (argument === undefined) {
    throw new Error("the seeded PLE capture environment did not report its ready entry URL");
  }
  const url = new URL(argument);
  if (url.protocol !== "https:" || url.hostname !== "localhost" || url.pathname !== "/sign-in") {
    throw new Error("the seeded PLE capture entry must be its local HTTPS sign-in URL");
  }
  return url;
}

async function loadManifest() {
  const parsed = JSON.parse(await readFile(manifestPath, "utf8"));
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("the current screenshot manifest must be an object");
  }
  if (parsed.schemaVersion !== 1 || !Array.isArray(parsed.captures)) {
    throw new Error("the current screenshot manifest has an unsupported shape");
  }
  const captures = parsed.captures;
  const paths = new Set();
  for (const capture of captures) {
    if (
      capture === null ||
      typeof capture !== "object" ||
      Array.isArray(capture) ||
      typeof capture.path !== "string" ||
      !/^(?:public|instructor|student|sysadmin)(?:\/[a-z0-9_]+)+\.png$/u.test(capture.path) ||
      typeof capture.persona !== "string" ||
      typeof capture.viewport !== "string" ||
      typeof capture.surface !== "string" ||
      paths.has(capture.path)
    ) {
      throw new Error("the current screenshot manifest has an invalid or duplicate capture");
    }
    paths.add(capture.path);
  }
  return captures;
}

function captureFor(captures, artifactPath) {
  const capture = captures.find((candidate) => candidate.path === artifactPath);
  if (capture === undefined) throw new Error(`the manifest does not declare ${artifactPath}`);
  return capture;
}

async function verifyManifest(captures, producedPaths) {
  for (const capture of captures) {
    if (producedPaths !== undefined && !producedPaths.has(capture.path)) {
      throw new Error(`the capture run did not rebuild declared artifact: ${capture.path}`);
    }
    const target = path.join(screenshotsDirectory, capture.path);
    const [metadata, bytes] = await Promise.all([stat(target), readFile(target)]);
    if (
      !metadata.isFile() ||
      bytes.length < 8 ||
      !bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))
    ) {
      throw new Error(`the declared role capture is not a PNG: ${capture.path}`);
    }
  }
}

function sensitiveResponseKey(value, protectedKeys) {
  if (Array.isArray(value)) {
    for (const item of value) {
      const matched = sensitiveResponseKey(item, protectedKeys);
      if (matched !== undefined) return matched;
    }
    return undefined;
  }
  if (value === null || typeof value !== "object") return undefined;
  for (const [key, nested] of Object.entries(value)) {
    const normalized = key.replace(/[^a-z0-9]/giu, "").toLowerCase();
    if (protectedKeys.has(normalized)) return key;
    const matched = sensitiveResponseKey(nested, protectedKeys);
    if (matched !== undefined) return matched;
  }
  return undefined;
}

function monitorResponsePrivacy(page, entryUrl, persona) {
  const protectedKeys =
    persona === "student"
      ? new Set([...protectedResponseKeys, ...studentOnlyProtectedResponseKeys])
      : protectedResponseKeys;
  const inspections = [];
  const violations = [];
  page.on("response", (response) => {
    const responseUrl = new URL(response.url());
    const contentType = response.headers()["content-type"] ?? "";
    if (responseUrl.origin !== entryUrl.origin || !contentType.includes("application/json")) {
      return;
    }
    const inspection = response
      .json()
      .then((body) => {
        const key = sensitiveResponseKey(body, protectedKeys);
        if (key !== undefined) {
          violations.push(`${responseUrl.pathname} contains protected response key ${key}`);
        }
      })
      .catch(() => {
        violations.push(`${responseUrl.pathname} could not be inspected as JSON`);
      });
    inspections.push(inspection);
  });
  return {
    async assertSafe() {
      await Promise.all(inspections);
      if (violations.length > 0) {
        throw new Error(violations.join("; "));
      }
    },
  };
}

async function openPage(browser, entryUrl, viewport, persona, mobile = false) {
  const context = await browser.newContext({
    colorScheme: "light",
    deviceScaleFactor: 1,
    hasTouch: mobile,
    ignoreHTTPSErrors: true,
    isMobile: mobile,
    reducedMotion: "reduce",
    viewport,
  });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error));
  page.setDefaultTimeout(30_000);
  page.setDefaultNavigationTimeout(60_000);
  // The sign-in heading is the usable application readiness boundary. The
  // production WASM module can extend document parsing after that surface is
  // already available, so waiting for DOMContentLoaded only delays capture.
  await page.goto(entryUrl.href, { waitUntil: "commit" });
  await page
    .getByRole("heading", { level: 1, name: "Explore Peptidyle Learning Engine", exact: true })
    .waitFor();
  // Observe workflow responses after the application has completed its own
  // session bootstrap; capture evidence must not alter that startup boundary.
  const responsePrivacy = monitorResponsePrivacy(page, entryUrl, persona);
  return { context, page, pageErrors, responsePrivacy };
}

function requireNoPageErrors(pageErrors) {
  if (pageErrors.length > 0) {
    throw new AggregateError(
      pageErrors,
      "the seeded PLE capture environment raised browser page errors",
    );
  }
}

async function assertSafeCapture(page, captureRecord, responsePrivacy) {
  await responsePrivacy.assertSafe();
  const visibleText = await page.locator("body").innerText();
  const matched = sensitiveScreenTerms.find((term) => term.test(visibleText));
  if (matched !== undefined) {
    throw new Error(
      `${captureRecord.path} contains protected screen text matching ${matched.source}`,
    );
  }
  if (
    await page
      .locator('input[type="email"]')
      .evaluateAll((inputs) => inputs.some((input) => input.value.length > 0))
  ) {
    throw new Error(`${captureRecord.path} contains a filled Authentication Email field`);
  }
  if (
    captureRecord.persona === "student" &&
    ((await page.getByRole("heading", { name: /^(Response received|Graded)$/u }).count()) > 0 ||
      (await page.locator("input:checked").count()) > 0)
  ) {
    throw new Error(`${captureRecord.path} must show an unanswered Question presentation`);
  }
}

function expectedRibbonTab(captureRecord) {
  switch (captureRecord.persona) {
    case "instructor":
      return captureRecord.path.includes("question_library") ? "Question Library" : "Assignments";
    case "student":
      return "Assignments";
    case "sysadmin":
      return "Instructor Accounts";
    default:
      return undefined;
  }
}

async function assertNormalRibbon(page, captureRecord) {
  const tabName = expectedRibbonTab(captureRecord);
  if (tabName === undefined) return;
  const ribbon = page.getByRole("region", { name: "PLE application Ribbon", exact: true });
  await ribbon.waitFor();
  const tab = ribbon
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: tabName, exact: true });
  await tab.waitFor();
  if ((await tab.getAttribute("aria-current")) !== "page") {
    throw new Error(`${captureRecord.path} must show its selected ${tabName} Ribbon tab`);
  }
}

async function capture(capturePage, captures, artifactPath, producedPaths) {
  const { page, responsePrivacy } = capturePage;
  const captureRecord = captureFor(captures, artifactPath);
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.evaluate(async () => document.fonts.ready);
  await assertSafeCapture(page, captureRecord, responsePrivacy);
  await assertNormalRibbon(page, captureRecord);
  const target = path.join(screenshotsDirectory, captureRecord.path);
  await mkdir(path.dirname(target), { recursive: true });
  await page.screenshot({
    animations: "disabled",
    caret: "hide",
    path: target,
  });
  producedPaths.add(captureRecord.path);
  console.log(`Captured ${path.relative(REPO_ROOT, target)}`);
}

async function enterInstructor(page) {
  await page.getByRole("button", { name: /Continue as Elena Instructor/u }).click();
  await page.getByRole("heading", { name: "Question library", exact: true }).waitFor();
}

async function createReleasedAssignment(page, runId) {
  const courseTitle = `Connected Biochemistry ${runId}`;
  const blueprintTitle = `${courseTitle} Blueprint`;
  const assignmentTitle = `Peptide Bond Practice ${runId}`;
  await page.getByRole("link", { name: "Blueprint Courses", exact: true }).click();
  await page.getByRole("button", { name: "Create Blueprint Course", exact: true }).click();
  await page.getByLabel("Blueprint Course title").fill(blueprintTitle);
  await page.getByRole("button", { name: "Choose published Questions", exact: true }).click();
  await page.getByRole("button", { name: "Search questions", exact: true }).click();
  await page
    .getByRole("region", { name: "Question results", exact: true })
    .getByRole("checkbox")
    .first()
    .check();
  await page.getByRole("button", { name: "Use selected Questions", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Create Blueprint Course", exact: true })
    .click();
  await page.getByRole("link", { name: "Courses", exact: true }).click();
  await page
    .getByLabel("Blueprint Course Revision")
    .selectOption({ label: `${blueprintTitle} · Revision 1` });
  await page.getByLabel("Course Instance title").fill(courseTitle);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByLabel("Course Time Zone (IANA)").fill("America/Chicago");
  await page.getByRole("button", { name: "Create Course Instance", exact: true }).click();
  await page.getByRole("heading", { name: courseTitle, exact: true }).waitFor();
  await page.getByRole("link", { name: "Open Course Instance", exact: true }).first().click();
  await page.getByRole("link", { name: "Open Students", exact: true }).click();
  await page.getByLabel("Email, roster ID").fill("mary.student@live-demo.invalid,m20-student");
  await page.getByRole("button", { name: "Import roster", exact: true }).click();
  await page.getByText("Roster import recorded.").waitFor();
  await page.getByRole("link", { name: "Return to Course Instances", exact: true }).click();
  const courseCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: courseTitle, exact: true }) });
  await courseCard.getByRole("link", { name: "Open Course Instance", exact: true }).click();
  await page.getByRole("link", { name: "Open Assignments", exact: true }).click();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByLabel("Instructions").fill("Complete the selected published Question.");
  await page.getByRole("button", { name: "Create Assignment", exact: true }).click();
  await page.getByRole("heading", { name: "Assignment Workspace", exact: true }).waitFor();
  const available = page.getByRole("group", { name: "Available Published Questions" });
  await available.getByRole("checkbox").first().check();
  await page.getByLabel("Due date").fill("2026-12-01T12:00");
  await page.getByLabel("Late-work rule").selectOption("mark_late");
  await page.getByRole("button", { name: "Save Assignment", exact: true }).click();
  await page.getByText("Assignment saved. Validate it before release.", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Validate Assignment", exact: true }).click();
  await page
    .getByText(
      "Assignment validation passed. You can review the answer-free Assignment Preview or release it.",
      { exact: true },
    )
    .waitFor();
  await page.getByRole("button", { name: "Release Assignment", exact: true }).click();
  await page.getByText(/Released state · current edit [1-9][0-9]*/u).waitFor();
  return { courseTitle, assignmentTitle };
}

async function enterStudentAssignment(page, courseTitle, assignmentTitle, acceptInvitation) {
  await page.getByRole("button", { name: /Continue as .*Mary Student/iu }).click();
  await page.getByRole("heading", { name: "Your courses", exact: true }).waitFor();
  if (acceptInvitation) {
    await page.getByRole("link", { name: "Course invitations", exact: true }).click();
    const invitation = page
      .getByRole("article")
      .filter({ has: page.getByRole("heading", { name: courseTitle, exact: true }) });
    await invitation.getByRole("link", { name: "Review Course Invitation", exact: true }).click();
    await page.getByRole("button", { name: "Accept Course Invitation", exact: true }).click();
    await page.getByText("Course Invitation accepted.", { exact: true }).waitFor();
    await page.getByRole("link", { name: "Open assigned work", exact: true }).click();
  } else {
    const course = page
      .getByRole("article")
      .filter({ has: page.getByRole("heading", { name: courseTitle, exact: true }) });
    await course.getByRole("link", { name: "Open assigned work", exact: true }).click();
  }
  await page.getByRole("heading", { name: courseTitle, exact: true }).waitFor();
  const assignment = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignment.getByRole("link", { name: "Open Assignment", exact: true }).click();
  await page.getByRole("button", { name: "Start Assignment", exact: true }).click();
  await page.locator('[data-route-surface="assignmentOverview"]').waitFor();
  await page.getByRole("heading", { name: "Questions", exact: true }).waitFor();
}

async function enterSysadminInstructorAccounts(page) {
  await page.getByRole("button", { name: /Continue as Morgan Sysadmin/u }).click();
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Instructor Accounts", exact: true })
    .click();
  await page.getByRole("heading", { name: "Instructor Accounts", exact: true }).waitFor();
  await page
    .getByText("Loading Instructor Accounts...", { exact: true })
    .waitFor({ state: "hidden" });
}

async function captureCorpus(entryUrl, captures) {
  const browser = await chromium.launch();
  const producedPaths = new Set();
  const runId = "M20";
  let journey;
  try {
    const signIn = await openPage(browser, entryUrl, laptopViewport, "public");
    try {
      await capture(
        signIn,
        captures,
        "public/account/03_seeded_demo_sign_in_laptop.png",
        producedPaths,
      );
    } finally {
      requireNoPageErrors(signIn.pageErrors);
      await signIn.context.close();
    }

    const instructor = await openPage(browser, entryUrl, laptopViewport, "instructor");
    try {
      await enterInstructor(instructor.page);
      await capture(
        instructor,
        captures,
        "instructor/question_library_discovery/04_connected_question_library_laptop.png",
        producedPaths,
      );
      journey = await createReleasedAssignment(instructor.page, runId);
      await capture(
        instructor,
        captures,
        "instructor/assignment_workspace/03_released_assignment_workspace_laptop.png",
        producedPaths,
      );
    } finally {
      requireNoPageErrors(instructor.pageErrors);
      await instructor.context.close();
    }

    const laptopStudent = await openPage(browser, entryUrl, laptopViewport, "student");
    try {
      await enterStudentAssignment(
        laptopStudent.page,
        journey.courseTitle,
        journey.assignmentTitle,
        true,
      );
      await capture(
        laptopStudent,
        captures,
        "student/access/allowed_assignment_overview/03_connected_assignment_overview_laptop.png",
        producedPaths,
      );
    } finally {
      requireNoPageErrors(laptopStudent.pageErrors);
      await laptopStudent.context.close();
    }

    const tabletStudent = await openPage(browser, entryUrl, tabletViewport, "student", true);
    try {
      await enterStudentAssignment(
        tabletStudent.page,
        journey.courseTitle,
        journey.assignmentTitle,
        false,
      );
      await capture(
        tabletStudent,
        captures,
        "student/access/allowed_assignment_overview/02_connected_assignment_overview_tablet.png",
        producedPaths,
      );
    } finally {
      requireNoPageErrors(tabletStudent.pageErrors);
      await tabletStudent.context.close();
    }

    for (const [filename, viewport, mobile] of [
      ["student/delivery/09_unanswered_question_phone.png", phoneViewport, true],
      ["student/delivery/10_unanswered_question_square.png", squareViewport, false],
    ]) {
      const student = await openPage(browser, entryUrl, viewport, "student", mobile);
      try {
        await enterStudentAssignment(
          student.page,
          journey.courseTitle,
          journey.assignmentTitle,
          false,
        );
        await capture(student, captures, filename, producedPaths);
      } finally {
        requireNoPageErrors(student.pageErrors);
        await student.context.close();
      }
    }

    const sysadmin = await openPage(browser, entryUrl, laptopViewport, "sysadmin");
    try {
      await enterSysadminInstructorAccounts(sysadmin.page);
      await capture(
        sysadmin,
        captures,
        "sysadmin/instructor_accounts/01_instructor_accounts_laptop.png",
        producedPaths,
      );
    } finally {
      requireNoPageErrors(sysadmin.pageErrors);
      await sysadmin.context.close();
    }
  } finally {
    await browser.close();
  }
  await verifyManifest(captures, producedPaths);
}

const captures = await loadManifest();
if (process.argv[2] === "--verify") {
  await verifyManifest(captures);
  console.log("Current application screenshot manifest: PASS");
} else {
  await captureCorpus(requireEntryUrl(process.argv[2]), captures);
  console.log(`Screenshot capture complete: ${path.relative(REPO_ROOT, screenshotsDirectory)}`);
}
