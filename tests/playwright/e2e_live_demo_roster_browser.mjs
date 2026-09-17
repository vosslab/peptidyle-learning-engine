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
  await page.waitForURL(`${origin}/`);
  await page
    .getByRole("navigation", { name: "Ribbon tasks", exact: true })
    .getByRole("link", { name: "My Blueprint Courses", exact: true })
    .click();
  await page.waitForURL(`${origin}/blueprint-courses`);
  await page.getByRole("button", { name: "Create Blueprint Course" }).click();
  await page.getByRole("heading", { name: "Create a Blueprint Course" }).waitFor();
  await page.getByLabel("Blueprint Course short name").fill(`M9 BP ${runId}`);
  await page.getByLabel("Blueprint Course long name").fill(blueprintTitle);
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
  await page.waitForURL(/\/blueprint-courses\/BP[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$/u);
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true })
    .click();
  await page.waitForURL(`${origin}/`);
  const source = page.getByLabel("Blueprint Course");
  await source.selectOption({ label: `${blueprintTitle} · Revision 1` });
  await page.getByLabel("Course short name").fill(courseShortName);
  await page.getByLabel("Course long name").fill(courseLongName);
  await page.getByLabel("Course Term start date").fill("2026-09-01");
  await page.getByLabel("Course Term end date").fill("2026-12-18");
  await page.getByRole("button", { name: "Create Course Instance" }).click();
  await page.getByRole("heading", { name: courseLongName }).waitFor();
  await page.getByRole("link", { name: "Open Course Instance" }).first().click();
  await page.waitForURL(/\/courses\/CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$/u);
  await page.getByRole("link", { name: "Open Students" }).click();
  await page.waitForURL(/\/instructor\/courses\/CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}\/students$/u);
  await page.getByRole("heading", { name: "Students" }).waitFor();
  await page
    .getByLabel("Email, roster ID, Course roster name")
    .fill('mary.okafor@biology.roosevelt.edu,m9-browser-seeded,"Example, Synthetic Student"');
  await page.getByRole("button", { name: "Import roster" }).click();
  await page.getByText("Roster import recorded.", { exact: false }).waitFor();
  await page.getByText("Example, Synthetic Student", { exact: true }).waitFor();
  await page.getByText("Invitation pending").waitFor();
  await page.getByText("m9-browser-seeded").waitFor();
} finally {
  await context.close();
  await browser.close();
}
