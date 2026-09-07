// Production-browser proof for the focused answer-free M15 Gradebook.

import { chromium } from "playwright";

const [port, course] = process.argv.slice(2);
if (!/^[0-9]+$/u.test(port ?? "") || !/^C-[1-9][0-9]{0,9}$/u.test(course ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port and a Course Instance reference");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Elena Instructor" }).click();
  await page.waitForURL(`${origin}/`);

  const gradebookResponse = page.waitForResponse((response) =>
    response.url().endsWith(`/api/course-instances/${course}/gradebook`),
  );
  await page.goto(`${origin}/instructor/courses/${course}/gradebook`, {
    waitUntil: "domcontentloaded",
  });
  const response = await gradebookResponse;
  if (!response.ok() || !response.headers()["cache-control"]?.includes("no-store")) {
    throw new Error("Gradebook did not retain its protected no-store transport boundary");
  }
  const projection = await response.json();
  if (
    projection === null ||
    typeof projection !== "object" ||
    Array.isArray(projection) ||
    Object.keys(projection).sort().join(",") !== "courseReference,gradedStudentWork" ||
    projection.courseReference !== course ||
    !Array.isArray(projection.gradedStudentWork) ||
    projection.gradedStudentWork.length === 0
  ) {
    throw new Error("Gradebook browser received an unexpected projection");
  }
  const first = projection.gradedStudentWork[0];
  if (
    first === null ||
    typeof first !== "object" ||
    Array.isArray(first) ||
    Object.keys(first).sort().join(",") !==
      "assignmentReference,gradedQuestionCount,pointsEarned,pointsPossible,rosterId"
  ) {
    throw new Error("Gradebook browser received non-answer-free Student Work evidence");
  }

  await page.locator('[data-route-surface="gradebook"]').waitFor();
  await page.getByRole("heading", { name: "Gradebook", exact: true }).waitFor();
  const evidence = page.getByRole("region", { name: "Gradebook evidence" });
  await evidence.getByText(first.rosterId, { exact: true }).waitFor();
  await evidence.getByText(first.assignmentReference, { exact: true }).waitFor();
  const rendered = (await evidence.textContent()) ?? "";
  if (/student response|answer key|source content|grader internals/i.test(rendered)) {
    throw new Error("Gradebook browser rendered non-answer-free evidence");
  }

  const foreignResponse = page.waitForResponse((candidate) =>
    candidate.url().endsWith("/api/course-instances/C-2147483647/gradebook"),
  );
  await page.goto(`${origin}/instructor/courses/C-2147483647/gradebook`, {
    waitUntil: "domcontentloaded",
  });
  if ((await foreignResponse).status() !== 404) {
    throw new Error("foreign Gradebook browser access was not concealed");
  }
  await page.getByRole("heading", { name: "Gradebook unavailable" }).waitFor();
} finally {
  await context.close();
  await browser.close();
}
