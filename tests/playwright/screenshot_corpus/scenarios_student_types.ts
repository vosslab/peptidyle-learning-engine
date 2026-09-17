// Actual enrolled Student delivery; preparation uses the ordinary Instructor workspace.

import type { Locator, Page } from "playwright";

import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { QuestionType } from "../../../generated/api/QuestionType";
import { decodeQuestionSearchPage } from "../../../src/api/decoders/question_library";
import { decodeStudentAssessmentAttemptPresentation } from "../../../src/api/decoders/assessment_attempt_navigation";
import type { StudentAssessmentAttemptPresentation } from "../../../src/api/assessment_attempt_navigation";
import type { CaptureSession, ScenarioRuntime } from "./runtime";
import type { ScenarioDefinition } from "./scenario_types";
import { authorHotspot, exerciseHotspot } from "./hotspot_workflow";
import {
  assignmentCard,
  choosePersona,
  enterInstructor,
  openInstructorCourse,
  openStudentCourse,
  scrollTop,
} from "./visible_workflows";

const SCENARIO = "student_question_types";
const ASSESSMENT_TITLE = "Cell biology response practice";
const PRACTICE_LABEL = "Practice Question Assignment";
const EXPECTED_QUESTION_COUNT = 9;

type ExampleSlug =
  "mc" | "ma" | "fib" | "multi_fib" | "num" | "match" | "order" | "hotspot" | "webwork";

interface Example {
  readonly slug: ExampleSlug;
  readonly title: string;
  readonly type: QuestionType;
  readonly backend: "ple" | "webwork";
  readonly format?: string;
  readonly prompt?: string;
}

// Meaningful published examples, not response-format fixture data or made-up deployment IDs.
const EXAMPLES: ReadonlyArray<Example> = [
  {
    slug: "mc",
    title: "Biochemistry Chapter 1: Charged functional groups",
    type: "multipleChoice",
    backend: "ple",
  },
  {
    slug: "ma",
    title: "Cell biology: membrane components",
    type: "multipleAnswer",
    backend: "ple",
    format: "multipleAnswer",
    prompt: "Select all components commonly found in a plasma membrane.",
  },
  {
    slug: "fib",
    title: "Cell biology: copying DNA",
    type: "fillInBlank",
    backend: "ple",
    format: "fillIn",
    prompt: "What process copies a cell's DNA before division? Enter the process name.",
  },
  {
    slug: "multi_fib",
    title: "Cell biology: gene expression steps",
    type: "multipleFillInBlank",
    backend: "ple",
    format: "multiFillIn",
    prompt:
      "Name the process that produces RNA from DNA, then the process that produces a polypeptide from an RNA template.",
  },
  {
    slug: "num",
    title: "Cell biology: dilution calculation",
    type: "numeric",
    backend: "ple",
    format: "numeric",
    prompt:
      "Dilute 2 mL of a 10 mM solution to a final volume of 20 mL. What is the final concentration in mM?",
  },
  {
    slug: "match",
    title: "Biochemistry Chapter 1: Functional group matching",
    type: "matching",
    backend: "ple",
  },
  {
    slug: "order",
    title: "Cell biology: mitotic stages",
    type: "ordering",
    backend: "ple",
    format: "ordering",
    prompt: "Arrange these stages of mitosis from earliest to latest.",
  },
  {
    slug: "hotspot",
    title: "Image region practice: one dot",
    type: "hotspot",
    backend: "ple",
    format: "hotspot",
    prompt: "Click the dot.",
  },
  {
    slug: "webwork",
    title: "Genetic Disorders from Descriptions",
    type: "multipleChoice",
    backend: "webwork",
  },
];

function checkpoint(slug: ExampleSlug, viewport: "laptop" | "phone"): string {
  return `question_unanswered_${slug}_${viewport}`;
}

async function questionLibrary(page: Page): Promise<void> {
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Questions", exact: true })
    .click();
  await page.getByRole("heading", { name: "Search Question Library", exact: true }).waitFor();
}

/** Observe answer-free metadata from a real visible search; no API writes or private-source reads. */
async function discover(page: Page, example: Example): Promise<QuestionSummary | undefined> {
  await questionLibrary(page);
  const loaded = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      url.origin === new URL(page.url()).origin &&
      url.pathname === "/api/questions/search" &&
      response.status() === 200
    );
  });
  await page.getByLabel("Search published questions").fill(example.title);
  const search = decodeQuestionSearchPage(await (await loaded).json());
  const matches = search.items
    .map((item) => item.summary)
    .filter(
      (summary) =>
        summary.metadata.questionTitle === example.title &&
        summary.backend === example.backend &&
        summary.questionType === example.type &&
        summary.availability.availability === "available",
    );
  if (matches.length > 1) throw new Error(`Ambiguous published example: ${example.title}`);
  const summary = matches[0];
  if (summary !== undefined) {
    await page.getByRole("heading", { name: example.title, exact: true }).waitFor();
  }
  return summary;
}

async function fillItems(fields: Locator, values: ReadonlyArray<string>): Promise<void> {
  if ((await fields.count()) !== values.length)
    throw new Error("Authoring item count did not match the visible example.");
  for (const [index, value] of values.entries()) await fields.nth(index).fill(value);
}

async function authorResponse(page: Page, example: Example): Promise<void> {
  switch (example.slug) {
    case "hotspot":
      await authorHotspot(page);
      break;
    case "ma":
      await fillItems(page.getByLabel("Choice text", { exact: true }), [
        "Phospholipids",
        "Membrane proteins",
        "Chromosomal DNA",
      ]);
      await page.getByLabel("Correct answer", { exact: true }).nth(1).check();
      break;
    case "fib":
      await page.getByLabel("Accepted answer 1", { exact: true }).fill("replication");
      break;
    case "multi_fib":
      await page.getByLabel("Visible blank label", { exact: true }).fill("DNA to RNA");
      await page.getByLabel("Accepted answer 1 for blank 1", { exact: true }).fill("transcription");
      await page.getByRole("button", { name: "Add blank", exact: true }).click();
      await page
        .getByLabel("Visible blank label", { exact: true })
        .nth(1)
        .fill("RNA to polypeptide");
      await page.getByLabel("Accepted answer 1 for blank 2", { exact: true }).fill("translation");
      break;
    case "num":
      await page.getByLabel("Accepted numeric value", { exact: true }).fill("1");
      await page.getByLabel("Unit (optional)").fill("mM");
      break;
    case "order":
      await fillItems(page.getByLabel("Ordering Item text", { exact: true }), [
        "Prophase",
        "Metaphase",
        "Anaphase",
      ]);
      break;
    default:
      throw new Error("Existing published examples must not be reauthored.");
  }
}

async function publish(page: Page, example: Example): Promise<void> {
  if (example.format === undefined || example.prompt === undefined)
    throw new Error(`Installed publication missing: ${example.title}`);
  await page.getByRole("link", { name: "My Draft Questions", exact: true }).click();
  await page.getByRole("heading", { name: "My Draft Questions", exact: true }).waitFor();
  await page.getByRole("button", { name: "New Draft Question", exact: true }).click();
  await page.getByLabel("Question Title", { exact: true }).fill(example.title);
  await page.getByLabel("Student-facing prompt", { exact: true }).fill(example.prompt);
  await page.getByLabel("Question License").selectOption("CC-BY-4.0");
  await page.getByLabel("Question format").selectOption(example.format);
  // Resize the visible list to the three membrane options before writing their text.
  if (example.slug === "ma") {
    while ((await page.getByLabel("Choice text", { exact: true }).count()) > 3) {
      await page
        .getByRole("group", { name: "Multiple-answer choices", exact: true })
        .getByRole("button", { name: "Remove", exact: true })
        .last()
        .click();
    }
    while ((await page.getByLabel("Choice text", { exact: true }).count()) < 3) {
      await page.getByRole("button", { name: "Add choice", exact: true }).click();
    }
  }
  await authorResponse(page, example);
  await page
    .getByLabel("Question Description for Instructors", { exact: true })
    .fill(example.title);
  await page.getByRole("button", { name: "Save private draft", exact: true }).click();
  await page.getByText("Private draft saved. It is not published.", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Review publication changes", exact: true }).click();
  await page.getByLabel("Question Authors").fill("Live Demo Instructor");
  await page
    .getByRole("combobox", { name: "Discipline (required)", exact: true })
    .selectOption({ label: "Biology" });
  await page
    .getByRole("combobox", { name: "Subject (required)", exact: true })
    .selectOption({ label: "Biochemistry" });
  await page.getByRole("button", { name: "Confirm and publish", exact: true }).click();
  await page.getByRole("heading", { name: "Published", exact: true }).waitFor();
}

async function prepare(runtime: ScenarioRuntime): Promise<ReadonlyMap<string, Example>> {
  // Preparation is a separate uncaptured session, so private authoring responses never enter a
  // Student capture's privacy monitor. ASVS 8.2.1: all writes remain role-gated visible actions.
  const session = await runtime.open(runtime.record(SCENARIO, checkpoint("mc", "laptop")));
  const selected: Array<{ readonly example: Example; readonly summary: QuestionSummary }> = [];
  try {
    const page = session.page;
    await enterInstructor(page);
    for (const example of EXAMPLES) {
      let summary = await discover(page, example);
      if (summary === undefined) {
        await publish(page, example);
        summary = await discover(page, example);
      }
      if (summary === undefined)
        throw new Error(`Publication did not become discoverable: ${example.title}`);
      selected.push({ example, summary });
    }
    await openInstructorCourse(page);
    await page.getByRole("link", { name: "Create Assessment", exact: true }).click();
    await page.getByRole("heading", { name: "Create an Assessment", exact: true }).waitFor();
    await page.getByLabel("Assessment title", { exact: true }).fill(ASSESSMENT_TITLE);
    await page
      .getByRole("combobox", { name: /^Assessment Type/u })
      .selectOption("practice_question_assignment");
    await page.getByRole("button", { name: "Create Assessment", exact: true }).click();
    await page.getByRole("heading", { name: "Assessment Question Editor", exact: true }).waitFor();
    const available = page.locator('section[aria-labelledby="available-questions-heading"]');
    for (const { summary } of selected) {
      // Exact discovered ID + immutable Revision, not first Add Question or title-only guessing.
      const identity = `${summary.questionId} * Revision ${summary.latestQuestionRevision.revisionNumber}:`;
      const row = available.getByRole("listitem").filter({ hasText: identity });
      await row.first().waitFor();
      if ((await row.count()) !== 1)
        throw new Error(
          `Exact published Revision is unavailable for selection: ${summary.metadata.questionTitle}`,
        );
      await row.getByRole("button", { name: "Add Question", exact: true }).click();
    }
    await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
    await page
      .getByText("Questions and order saved. Review Assessment Properties when you are ready.")
      .waitFor();
    await page.getByRole("link", { name: "Review Assessment Properties", exact: true }).click();
    await page
      .getByRole("heading", { name: "Assessment Properties Editor", exact: true })
      .waitFor();
    await page
      .getByRole("group", { name: "Due date and time", exact: true })
      .locator('input[type="date"]')
      .fill("2026-12-01");
    await page.getByLabel("Due time").fill("12:00");
    await page.getByText("Saved", { exact: true }).waitFor();
    await page.getByRole("button", { name: "Check release readiness", exact: true }).click();
    await page.getByRole("heading", { name: "Ready to release", exact: true }).waitFor();
    await page.getByRole("button", { name: "Release assessment", exact: true }).click();
    await page.getByText(/^Assessment released\. Current edit number: [1-9][0-9]*\.$/u).waitFor();
    return new Map(
      selected.map(({ example, summary }) => [
        `${summary.questionId}:${summary.latestQuestionRevision.revisionNumber}`,
        example,
      ]),
    );
  } finally {
    await runtime.close(session);
  }
}

async function readQuestion(
  page: Page,
  action: () => Promise<unknown>,
): Promise<StudentAssessmentAttemptPresentation> {
  const delivered = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      url.origin === new URL(page.url()).origin &&
      /^\/api\/assessment-attempts\/R-[1-9][0-9]*\/student-question$/u.test(url.pathname) &&
      response.status() === 200
    );
  });
  await action();
  return decodeStudentAssessmentAttemptPresentation(await (await delivered).json());
}

/** Follows the visible Student Course action without guessing from intermediate route timing. */
async function openOrResumePracticeAssignment(page: Page, card: Locator): Promise<void> {
  const action = card.getByRole("link", {
    name: /^(Open|Resume) Practice Question Assignment$/u,
  });
  const open = card.getByRole("link", { name: `Open ${PRACTICE_LABEL}`, exact: true });
  const resume = card.getByRole("link", { name: `Resume ${PRACTICE_LABEL}`, exact: true });
  await action.first().waitFor();
  const [canOpen, canResume] = await Promise.all([open.isVisible(), resume.isVisible()]);
  if (canOpen === canResume) {
    throw new Error("Student Course card must expose exactly one Open or Resume action.");
  }
  if (canOpen) {
    await open.click();
    await page.locator('[data-route-surface="assessmentOverview"]').waitFor();
    await page.getByRole("button", { name: `Start ${PRACTICE_LABEL}`, exact: true }).click();
  } else {
    await resume.click();
  }
  await page.locator('[data-route-surface="assessmentAttempt"]').waitFor();
}

async function waitForControl(session: CaptureSession, example: Example): Promise<void> {
  const page = session.page;
  if (example.backend === "webwork") {
    const document = page.frameLocator('iframe[title="Question document"]');
    await document.locator("form").first().waitFor();
    await document
      .locator(
        'input:not([type="hidden"]):not([type="submit"]):not([type="button"]):not([disabled]), select:not([disabled]), textarea:not([disabled])',
      )
      .first()
      .waitFor();
    return;
  }
  const response = page.locator("section.question-response-control");
  await response.waitFor();
  const selector: Record<Exclude<ExampleSlug, "webwork">, string> = {
    mc: 'input[type="radio"]',
    ma: 'input[type="checkbox"]',
    fib: 'input[type="text"], textarea',
    multi_fib: 'input[type="text"], textarea',
    num: "input",
    match: ".matching-bank button",
    order: "button",
    hotspot: ".hotspot-image-region",
  };
  await response
    .locator(selector[example.slug as Exclude<ExampleSlug, "webwork">])
    .first()
    .waitFor();
  if (example.slug === "multi_fib" && (await response.locator(selector.multi_fib).count()) < 2) {
    throw new Error("MULTI-FIB delivery did not provide independently identifiable blanks.");
  }
  if (example.slug === "hotspot") {
    await response.locator(".hotspot-image-surface img").waitFor();
    if ((await response.locator(".hotspot-image-region").count()) !== 1)
      throw new Error("HOTSPOT did not deliver the uploaded dot region.");
  }
}

async function captureTypes(runtime: ScenarioRuntime): Promise<void> {
  const provenance = await prepare(runtime);
  for (const viewport of ["laptop", "phone"] as const) {
    const session = await runtime.open(runtime.record(SCENARIO, checkpoint("mc", viewport)));
    try {
      const page = session.page;
      await choosePersona(page, "Avery Thompson");
      await openStudentCourse(page);
      const card = assignmentCard(page, ASSESSMENT_TITLE);
      let question = await readQuestion(page, () => openOrResumePracticeAssignment(page, card));
      const navigation = page.getByRole("navigation", {
        name: "Assessment questions",
        exact: true,
      });
      const covered = new Set<ExampleSlug>();
      const first = navigation.getByRole("button", { name: /^Question 1:/u });
      if (question.position !== 1) question = await readQuestion(page, () => first.click());
      else await first.click();
      for (let position = 1; position <= EXPECTED_QUESTION_COUNT; position += 1) {
        if (position > 1) {
          question = await readQuestion(page, () =>
            navigation.getByRole("button", { name: "Next question", exact: true }).click(),
          );
        }
        if (question.position !== position)
          throw new Error("Student navigation did not deliver the next issued Question position.");
        await page
          .getByText(`Question ${position} of ${EXPECTED_QUESTION_COUNT}`, { exact: true })
          .waitFor();
        const pin = question.presentation.questionRevision;
        const example = provenance.get(`${pin.questionId}:${pin.revisionNumber}`);
        if (example === undefined || covered.has(example.slug))
          throw new Error(
            "Student delivery did not match the exact selected published Revision set.",
          );
        await waitForControl(session, example);
        await scrollTop(page);
        await runtime.capture(
          session,
          runtime.record(SCENARIO, checkpoint(example.slug, viewport)),
        );
        covered.add(example.slug);
      }
      if (
        covered.size !== EXPECTED_QUESTION_COUNT ||
        EXAMPLES.some((example) => !covered.has(example.slug))
      )
        throw new Error("Student delivery type coverage is incomplete.");
    } finally {
      await runtime.close(session);
    }
  }
  // Interaction/submission proof is uncaptured and occurs after both unanswered captures.
  for (const input of ["pointer", "keyboard"] as const) {
    const session = await runtime.open(runtime.record(SCENARIO, checkpoint("hotspot", "laptop")));
    try {
      await choosePersona(session.page, input === "pointer" ? "Avery Thompson" : "Jack Nguyen");
      await openStudentCourse(session.page);
      await openOrResumePracticeAssignment(
        session.page,
        assignmentCard(session.page, ASSESSMENT_TITLE),
      );
      await exerciseHotspot(session.page, input);
    } finally {
      await runtime.close(session);
    }
  }
}

export const STUDENT_TYPE_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: SCENARIO,
    checkpoints: EXAMPLES.flatMap((example) => [
      checkpoint(example.slug, "laptop"),
      checkpoint(example.slug, "phone"),
    ]),
    run: captureTypes,
  },
];
