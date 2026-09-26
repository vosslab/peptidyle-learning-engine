// Production-browser proof for visible Course Roster Import and current roster projection.

import { chromium } from "playwright";
import { liveDemoChromiumArgs } from "./helper_gateway_trust.mjs";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const runId = Date.now();
const blueprintTitle = `Browser M9 Blueprint ${runId}`;
const courseShortName = `M9-${runId}`;
const courseLongName = `Browser M9 Course ${runId}`;
const browser = await chromium.launch({ headless: true, args: liveDemoChromiumArgs(origin) });
const context = await browser.newContext();
const page = await context.newPage();
let rosterImportRequests = 0;

page.on("request", (request) => {
  if (
    request.method() === "POST" &&
    /^\/api\/course-instances\/CI[0-9A-HJKMNP-TV-Z]{8}\/roster$/u.test(
      new URL(request.url()).pathname,
    )
  ) {
    rosterImportRequests += 1;
  }
});

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
  await page
    .getByRole("navigation", { name: "Ribbon tasks", exact: true })
    .getByRole("link", { name: "My Blueprint Courses", exact: true })
    .click();
  await page.waitForURL(`${origin}/blueprint-courses`);
  await page.getByRole("button", { name: "Create Blueprint Course" }).click();
  await page.getByRole("heading", { name: "Create a Blueprint Course" }).waitFor();
  await page.getByLabel("Blueprint Course short name").fill(`M9 BP ${runId}`);
  await page.getByLabel("Blueprint Course long name").fill(blueprintTitle);
  await page.getByLabel("Discipline (required)").selectOption({ index: 1 });
  await page.getByLabel("First Assessment Type").selectOption("practice_question_assignment");
  await page.getByRole("button", { name: "Choose published Questions" }).click();
  await page.getByRole("heading", { name: "Choose the first reusable Questions" }).waitFor();
  await page.getByRole("button", { name: "Search questions" }).click();
  await page
    .getByRole("dialog", { name: "Choose the first reusable Questions", exact: true })
    .getByRole("checkbox")
    .first()
    .check();
  await page.getByRole("button", { name: "Use selected Questions" }).click();
  await page.getByText("1 fixed Question selected in order.").waitFor();
  await page.getByRole("dialog").getByRole("button", { name: "Create Blueprint Course" }).click();
  await page.waitForURL(/\/blueprint-courses\/BP[0-9ABCDEFGHJKMNPQRSTVWXYZ]{8}$/u);
  await page.getByRole("button", { name: "Publish Blueprint Course", exact: true }).click();
  await page.getByRole("button", { name: "Return to Private", exact: true }).waitFor();
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true })
    .click();
  await page.waitForURL(`${origin}/instructor`);
  await page.getByRole("button", { name: "Create Course Instance", exact: true }).click();
  await page.getByLabel("Start with").selectOption("adopted");
  const source = page.locator("#course-blueprint-source");
  await source.selectOption({ label: `${blueprintTitle} · Revision 1` });
  await page.getByLabel("Course short name").fill(courseShortName);
  await page.getByLabel("Course long name").fill(courseLongName);
  await page.getByLabel("Discipline (required)").selectOption({ index: 1 });
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page
    .locator("#create-course-instance")
    .getByRole("button", { name: "Create Course Instance", exact: true })
    .click();
  await page.getByRole("heading", { name: courseLongName }).waitFor();
  await page.getByRole("link", { name: "Open Course" }).first().click();
  await page.waitForURL(/\/courses\/CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{8}$/u);
  await page.getByRole("link", { name: "Open Students" }).click();
  await page.waitForURL(/\/instructor\/courses\/CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{8}\/students$/u);
  await page.getByRole("heading", { name: "Students" }).waitFor();
  await page.getByRole("button", { name: "Import Students", exact: true }).click();

  await page
    .getByLabel("Email, roster ID, Course roster name")
    .fill("not-an-email,valid-id,Valid Name");
  const importsBeforeRejectedInput = rosterImportRequests;
  await page.getByRole("button", { name: "Import roster" }).click();
  const rejectedInput = page.getByRole("alert").filter({
    hasText: "Use 1-50 valid email,roster_id,roster_name rows.",
  });
  await rejectedInput.waitFor();
  if (!(await rejectedInput.evaluate((element) => element.classList.contains("inline-error")))) {
    throw new Error("rejected roster input must use error styling");
  }
  if (rosterImportRequests !== importsBeforeRejectedInput) {
    throw new Error("rejected local roster input must not request a roster import");
  }

  await page
    .getByLabel("Email, roster ID, Course roster name")
    .fill('mary.okafor@biology.roosevelt.edu,m9-browser-seeded,"Example, Synthetic Student"');
  await page.getByRole("button", { name: "Import roster" }).click();
  const imported = page.getByRole("status").filter({ hasText: "Roster import recorded." });
  await imported.waitFor();
  if (
    !(await imported.evaluate((element) => element.classList.contains("roster-feedback-success")))
  ) {
    throw new Error("recorded roster import must use success styling");
  }
  await imported
    .getByText("new Students remain pending until they claim their Course Invitation.")
    .waitFor();
  await page.getByText("Example, Synthetic Student", { exact: true }).waitFor();
  await page.getByText("Invitation pending").waitFor();
  await page.getByText("m9-browser-seeded").waitFor();
} finally {
  await context.close();
  await browser.close();
}
