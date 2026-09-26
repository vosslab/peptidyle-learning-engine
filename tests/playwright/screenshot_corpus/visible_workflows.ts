// visible_workflows.ts - reusable visible application actions for browser tests and captures.

import type { Locator, Page } from "playwright";

import { localDemoAuthenticationCode } from "./local_demo_authenticator";

import {
  chooseSeededIdentityAtSignIn,
  courseChoice,
  restoreViewportOrigin,
} from "../e2e/real_stack_ui";

export const COURSE_TITLE = "Biochemistry 301: Proteins and Peptides";
export const ASSIGNMENT_TITLE = "Chapter 1 Pilot Practice";
// The seeded Live Demo Assessment is a Practice Question Assignment; card links and the
// overview Start button render `{verb} {type label}`.
export const ASSESSMENT_TYPE_LABEL = "Practice Question Assignment";

export type SeededPersona =
  "Elena Rivera" | "Mary Okafor" | "Jack Nguyen" | "Avery Thompson" | "Morgan Delgado";

export async function choosePersona(page: Page, persona: SeededPersona): Promise<void> {
  await chooseSeededIdentityAtSignIn(page, new RegExp(persona, "u"));
}

export async function scrollTop(page: Page): Promise<void> {
  await restoreViewportOrigin(page);
}

export function courseCard(page: Page, title: string = COURSE_TITLE): Locator {
  return courseChoice(page, title);
}

export function assignmentCard(page: Page, title: string = ASSIGNMENT_TITLE): Locator {
  return page
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { name: title, exact: true }) });
}

export function courseworkRow(page: Page, title: string = ASSIGNMENT_TITLE): Locator {
  return page.locator(".record-list__row").filter({
    has: page.getByRole("heading", { name: title, exact: true }),
  });
}

export async function openAllStudentCoursework(page: Page): Promise<void> {
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Coursework", exact: true })
    .click();
  await page.locator('[data-route-surface="studentHome"]').waitFor();
  await courseworkRow(page).waitFor();
}

export async function openInstructorCourse(
  page: Page,
  title: string = COURSE_TITLE,
): Promise<void> {
  const courses = page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true });
  await courses.click();
  await page.getByRole("heading", { name: "My Active Courses", exact: true }).waitFor();
  const card = courseCard(page, title);
  await card.getByRole("link", { name: "Open Course", exact: true }).click();
  await page.getByRole("heading", { level: 1, name: title, exact: true }).waitFor();
}

export async function openStudentCourse(page: Page, title: string = COURSE_TITLE): Promise<void> {
  if (
    await studentCourseHeading(page, title)
      .isVisible()
      .catch(() => false)
  )
    return;
  await openStudentCourseList(page);
  const card = courseCard(page, title);
  await card.waitFor();
  await card.getByRole("link", { name: "Open Course", exact: true }).click();
  await studentCourseHeading(page, title).waitFor();
}

/** Opens the fixed Student Courses destination through Tier 1 navigation. */
export async function openStudentCourseList(page: Page): Promise<void> {
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true })
    .click();
  await page.waitForURL((url) => url.pathname === "/student/courses");
  await studentCourseListHeading(page).waitFor();
}

function studentCourseHeading(page: Page, title: string): Locator {
  return page.getByRole("heading", { level: 1, name: title, exact: true });
}

function studentCourseListHeading(page: Page): Locator {
  return page.getByRole("heading", { name: "Your courses", exact: true });
}

/** Matches the overview's entry button whether the Attempt is new or resumable. */
export const ASSESSMENT_ENTRY_BUTTON = new RegExp(`^(Start|Resume) ${ASSESSMENT_TYPE_LABEL}$`, "u");

export async function openStudentAssignment(page: Page): Promise<void> {
  const card = assignmentCard(page);
  // The card verb is Open, Resume, or Review depending on prior replays; the link is the same.
  await card.getByRole("link").first().click();
  await page.locator('[data-route-surface="assessmentOverview"]').waitFor();
  await page.getByRole("button", { name: ASSESSMENT_ENTRY_BUTTON }).waitFor();
}

/** Opens the active Attempt through its Student Course landing and Assignment card. */
export async function resumeStudentAssignmentAttempt(page: Page): Promise<void> {
  const card = assignmentCard(page);
  // The overview auto-resumes an active Attempt when its activeAttemptId loads, so the landing
  // action leads to the Attempt surface without requiring a transient overview button.
  await card.getByRole("link").first().click();
  await page.locator('[data-route-surface="assessmentAttempt"]').waitFor();
}

export async function enterInstructor(page: Page): Promise<void> {
  await choosePersona(page, "Elena Rivera");
  await page.getByRole("heading", { name: "Search Question Library", exact: true }).waitFor();
}

export async function enterSysadmin(page: Page): Promise<void> {
  const setupFile = process.env["PLE_LOCAL_DEMO_TOTP_SETUP_FILE"];
  if (setupFile === undefined && !process.argv.includes("--headed")) {
    throw new Error("Headless Sysadmin capture requires the owned local CLI authenticator path.");
  }
  if (setupFile !== undefined) {
    const entry = new URL(page.url());
    // ASVS 6.3.4, 14.2.3: this authenticator is only for the local demo entry.
    if (
      entry.protocol !== "https:" ||
      entry.hostname !== "localhost" ||
      entry.pathname !== "/sign-in" ||
      entry.search ||
      entry.hash ||
      entry.username ||
      entry.password
    ) {
      throw new Error("Automated demonstration MFA requires the local HTTPS sign-in entry.");
    }
  }
  const home = page.getByRole("heading", { name: "System administration", exact: true });
  const mfa = page.getByRole("heading", {
    level: 3,
    name: "Verify Morgan Delgado's administrator access",
    exact: true,
  });
  await page.getByRole("button", { name: /Assume the role of .*Morgan Delgado/u }).click();
  await Promise.race([home.waitFor(), mfa.waitFor()]);
  if (await home.isVisible()) return;
  if (setupFile !== undefined) {
    const code = await localDemoAuthenticationCode(setupFile);
    try {
      await page.getByLabel("Authentication code", { exact: true }).fill(code);
      await page
        .getByRole("button", {
          name: "Verify and open administrator tools",
          exact: true,
        })
        .click();
      await home.waitFor();
    } catch {
      // Playwright fill diagnostics may include a code; publish only this safe failure.
      throw new Error("Morgan's ordinary MFA form did not reach administrator tools.");
    }
    return;
  }
  try {
    await home.waitFor({ timeout: 120_000 });
  } catch {
    throw new Error(
      "Sysadmin screenshots require --headed and a prepared local authenticator; " +
        "enter the current Authentication code in the visible Morgan Delgado MFA form.",
    );
  }
}
