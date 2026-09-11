// visible_workflows.ts - reusable visible application actions for browser tests and captures.

import type { Locator, Page } from "playwright";

import {
  chooseSeededIdentityAtSignIn,
  courseChoice,
  restoreViewportOrigin,
} from "../e2e/real_stack_ui";

export const COURSE_TITLE = "Biochemistry 301: Proteins and Peptides";
export const ASSIGNMENT_TITLE = "Peptide Structure Practice";

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
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: title, exact: true }) });
}

export async function openInstructorCourse(
  page: Page,
  title: string = COURSE_TITLE,
): Promise<void> {
  const courses = page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true });
  await courses.click();
  await page.getByRole("heading", { name: "Course Instances you teach", exact: true }).waitFor();
  const card = courseCard(page, title);
  await card.getByRole("link", { name: "Open Course Instance", exact: true }).click();
  await page.getByRole("heading", { level: 1, name: title, exact: true }).waitFor();
}

export async function openStudentCourse(page: Page, title: string = COURSE_TITLE): Promise<void> {
  const entry = await waitForStudentCourseEntry(page, title);
  if (entry === "chooser") {
    const card = courseCard(page, title);
    await card.waitFor();
    await card.getByRole("link", { name: "Open assigned work", exact: true }).click();
    await studentCourseHeading(page, title).waitFor();
  }
}

/** Opens the visible chooser from the one-Course landing without bypassing normal navigation. */
export async function openStudentCourseChooser(page: Page): Promise<void> {
  const entry = await waitForStudentCourseEntry(page, COURSE_TITLE);
  if (entry === "course") {
    await page.getByRole("link", { name: "Your courses", exact: true }).click();
    await page.waitForURL((url) => url.pathname === "/" && url.searchParams.get("choose") === "1");
    await studentCourseChooserHeading(page).waitFor();
  }
}

function studentCourseHeading(page: Page, title: string): Locator {
  return page.getByRole("heading", { level: 1, name: title, exact: true });
}

function studentCourseChooserHeading(page: Page): Locator {
  return page.getByRole("heading", { name: "Your courses", exact: true });
}

/** Waits for a loaded current Student Course entry state after seeded sign-in. */
async function waitForStudentCourseEntry(page: Page, title: string): Promise<"course" | "chooser"> {
  await page.locator(".loading-state").waitFor({ state: "hidden" });
  const entry = await page.waitForFunction((expectedTitle) => {
    for (const heading of document.querySelectorAll("h1")) {
      const style = window.getComputedStyle(heading);
      if (
        heading.textContent?.trim() === expectedTitle &&
        style.visibility !== "hidden" &&
        style.display !== "none"
      ) {
        return "course";
      }
    }

    let visibleCardCount = 0;
    let expectedCardIsVisible = false;
    for (const card of document.querySelectorAll("article.course-card")) {
      const style = window.getComputedStyle(card);
      if (style.visibility === "hidden" || style.display === "none") continue;
      visibleCardCount += 1;
      for (const heading of card.querySelectorAll("h2")) {
        if (heading.textContent?.trim() === expectedTitle) expectedCardIsVisible = true;
      }
    }
    if (!expectedCardIsVisible || window.location.pathname !== "/") return null;

    const choosingCourses = new URLSearchParams(window.location.search).get("choose") === "1";
    return choosingCourses || visibleCardCount > 1 ? "chooser" : null;
  }, title);
  const state = await entry.jsonValue();
  if (state === "course" || state === "chooser") return state;
  throw new Error("Student Course entry did not reach a visible Course or chooser state.");
}

export async function openStudentAssignment(page: Page): Promise<void> {
  const card = assignmentCard(page);
  await card.getByRole("link", { name: "Open Assignment", exact: true }).click();
  await page.locator('[data-route-surface="assignmentOverview"]').waitFor();
  await page.getByRole("button", { name: "Start Assignment", exact: true }).waitFor();
}

/** Opens the active Attempt through its Student Course landing and Assignment card. */
export async function resumeStudentAssignmentAttempt(page: Page): Promise<void> {
  const card = assignmentCard(page);
  await card.getByRole("link", { name: "Open Assignment", exact: true }).click();
  await page.locator('[data-route-surface="assignmentAttempt"]').waitFor();
}

export async function enterInstructor(page: Page): Promise<void> {
  await choosePersona(page, "Elena Rivera");
  await page.getByRole("heading", { name: "Question library", exact: true }).waitFor();
}

export async function enterSysadmin(page: Page): Promise<void> {
  await choosePersona(page, "Morgan Delgado");
  await page.getByRole("heading", { name: "Your Course Instances", exact: true }).waitFor();
}
