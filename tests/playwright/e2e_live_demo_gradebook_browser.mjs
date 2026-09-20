// Production-browser proof for the focused answer-free Gradebook.

import { chromium } from "playwright";
import { liveDemoChromiumArgs } from "./helper_gateway_trust.mjs";

const [port, course] = process.argv.slice(2);
if (!/^[0-9]+$/u.test(port ?? "") || !/^CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{8}$/u.test(course ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port and a Course Instance ID");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true, args: liveDemoChromiumArgs(origin) });
const context = await browser.newContext();
const page = await context.newPage();

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page
    .getByRole("button", { name: "Assume the role of Instructor Dr. Elena Rivera" })
    .click();
  await page.waitForURL(`${origin}/library`);

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
    Object.keys(projection).sort().join(",") !== "courseInstanceId,studentWork" ||
    projection.courseInstanceId !== course ||
    !Array.isArray(projection.studentWork) ||
    projection.studentWork.length === 0
  ) {
    throw new Error("Gradebook browser received an unexpected projection");
  }
  const first = projection.studentWork[0];
  if (
    first === null ||
    typeof first !== "object" ||
    Array.isArray(first) ||
    Object.keys(first).sort().join(",") !==
      "assessmentAttemptCompletion,assessmentId,assessmentTitle,expiredSubmitting,rosterId,rosterName,score"
  ) {
    throw new Error("Gradebook browser received non-answer-free Student Work evidence");
  }

  await page.locator('[data-route-surface="gradebook"]').waitFor();
  await page.getByRole("heading", { name: "Gradebook", exact: true }).waitFor();
  const evidence = page.getByRole("region", { name: "Student progress and scores" });
  await evidence.getByText(first.rosterId, { exact: true }).waitFor();
  await evidence.getByText(first.rosterName, { exact: true }).first().waitFor();
  await evidence.getByText(first.assessmentTitle, { exact: true }).first().waitFor();
  await evidence.locator("small").getByText(first.assessmentId, { exact: true }).first().waitFor();
  const rendered = (await evidence.textContent()) ?? "";
  if (/student response|answer key|source content|grader internals/i.test(rendered)) {
    throw new Error("Gradebook browser rendered non-answer-free evidence");
  }

  const foreignResponse = page.waitForResponse((candidate) =>
    candidate.url().endsWith("/api/course-instances/CI000000AQ/gradebook"),
  );
  await page.goto(`${origin}/instructor/courses/CI000000AQ/gradebook`, {
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
