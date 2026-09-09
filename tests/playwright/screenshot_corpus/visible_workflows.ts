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
  await page.getByRole("heading", { name: "Your courses", exact: true }).waitFor();
  const card = courseCard(page, title);
  await card.getByRole("link", { name: "Open assigned work", exact: true }).click();
  await page.getByRole("heading", { level: 1, name: title, exact: true }).waitFor();
}

export async function openStudentAssignment(page: Page): Promise<void> {
  const card = assignmentCard(page);
  await card.getByRole("link", { name: "Open Assignment", exact: true }).click();
  await page.locator('[data-route-surface="assignmentOverview"]').waitFor();
  await page.getByRole("button", { name: "Start Assignment", exact: true }).waitFor();
}

export async function startStudentAssignment(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Start Assignment", exact: true }).click();
  await page.getByRole("heading", { level: 2, name: "Questions", exact: true }).waitFor();
  await page.getByRole("heading", { name: /^Question 1:/u }).waitFor();
}

export async function enterStudentAssignment(
  page: Page,
  persona: Extract<SeededPersona, "Mary Okafor" | "Jack Nguyen" | "Avery Thompson">,
): Promise<void> {
  await choosePersona(page, persona);
  await openStudentCourse(page);
  await openStudentAssignment(page);
}

export async function enterInstructor(page: Page): Promise<void> {
  await choosePersona(page, "Elena Rivera");
  await page.getByRole("heading", { name: "Question library", exact: true }).waitFor();
}

export async function enterSysadmin(page: Page): Promise<void> {
  await choosePersona(page, "Morgan Delgado");
  await page.getByRole("heading", { name: "Your Course Instances", exact: true }).waitFor();
}
