// Instructor WeBWorK generated-example capture through the published Question Library.
// Selector contract: the Browse controls and result rows are owned by library_page.tsx; the
// generated-example iframe is owned by question_detail_page.tsx and its document by WeBWorK.

import type { Page } from "playwright";

import type { ScenarioRuntime } from "./runtime";
import { catalogScreenshotFilename } from "./filenames";
import {
  viewportCoverage,
  type CaptureDeclaration,
  type ScenarioDefinition,
} from "./scenario_types";
import {
  enterInstructor,
  followCaptureLink,
  openInstructorLibraryBrowse,
} from "./visible_workflows";

const ANSWER_KEY_TEXT = /answer key|correct answer|correct feedback|private source/iu;

interface GeneratedExample {
  readonly checkpoint: string;
  readonly title: string;
  readonly subject: string;
  readonly prompt: string;
  readonly control: "input[type=radio]" | "select";
  readonly header?: string;
  readonly state: string;
  readonly caption: string;
}

const GENERATED_EXAMPLES: ReadonlyArray<GeneratedExample> = [
  {
    checkpoint: "webwork_generated_example",
    title: "Genetic Disorders from Descriptions",
    subject: "Genetics",
    prompt: "",
    control: "input[type=radio]",
    state: "rendered WeBWorK example",
    caption: "Answer-free WeBWorK generated example",
  },
  {
    checkpoint: "webwork_hla_genotype",
    title: "Offspring HLA Genotypes (2 Markers, Black)",
    subject: "DNA Profiling",
    prompt: "The mother has",
    control: "input[type=radio]",
    state: "HLA haplotype inheritance example",
    caption: "HLA offspring genotype generated example",
  },
  {
    checkpoint: "webwork_monohybrid_matching",
    title: "Matching Monohybrid Cross Genotypes to Phenotypes",
    subject: "Genetics",
    prompt: "monohybrid crosses",
    control: "select",
    state: "monohybrid genotype matching example",
    caption: "Monohybrid genotype matching generated example",
  },
  {
    checkpoint: "webwork_x_linked_counts",
    title: "Parent Genotypes in X-Linked Recessive Crosses",
    subject: "Inheritance Genetics",
    prompt: "red-eyed (wildtype)",
    control: "input[type=radio]",
    header: "phenotype",
    state: "X-linked offspring count table example",
    caption: "X-linked offspring count table generated example",
  },
  {
    checkpoint: "webwork_dna_structure",
    title: "True/False Statements About DNA Structure",
    subject: "Molecular Biology",
    prompt: "DNA",
    control: "input[type=radio]",
    state: "True/False Statements About DNA Structure generated example",
    caption: "True/False Statements About DNA Structure generated example",
  },
  {
    checkpoint: "webwork_meiosis_prophase",
    title: "Matching Meiosis Prophase I Stages to Descriptions",
    subject: "Genetics",
    prompt: "stages of meiosis prophase I",
    control: "select",
    state: "Matching Meiosis Prophase I Stages to Descriptions generated example",
    caption: "Matching Meiosis Prophase I Stages to Descriptions generated example",
  },
  {
    checkpoint: "webwork_chi_square",
    title: "True/False Statements About Chi-Square Tests",
    subject: "Biostatistics",
    prompt: "chi-square",
    control: "input[type=radio]",
    state: "True/False Statements About Chi-Square Tests generated example",
    caption: "True/False Statements About Chi-Square Tests generated example",
  },
  {
    checkpoint: "webwork_chromosome_shapes",
    title: "Matching Chromosome Shapes to Descriptions",
    subject: "Genetics",
    prompt: "categories of chromosome shape",
    control: "select",
    state: "Matching Chromosome Shapes to Descriptions generated example",
    caption: "Matching Chromosome Shapes to Descriptions generated example",
  },
];

async function renderedAnswerFreePreview(page: Page, example: GeneratedExample): Promise<void> {
  const preview = page.locator("iframe.question-library-webwork-preview");
  await preview.waitFor();
  await preview.scrollIntoViewIfNeeded();
  const document = preview.contentFrame();
  // A renderer-owned form proves this is a rendered WeBWorK document, not the empty iframe shell.
  await document.locator("form").waitFor();
  await document.locator(example.control).first().waitFor();
  if (example.prompt !== "") {
    await document.getByText(example.prompt, { exact: false }).first().waitFor();
  }
  if (example.header !== undefined) {
    // niceTables renders a CSS table, not an HTML table; wait for its visible header.
    await document.getByText(example.header, { exact: true }).waitFor();
  }
  const visibleText = await document.locator("body").innerText();
  if (ANSWER_KEY_TEXT.test(visibleText)) {
    throw new Error("WeBWorK generated example exposed answer-key text");
  }
  if ((await document.locator("input[type=radio]:checked").count()) !== 0) {
    throw new Error("WeBWorK generated example has a selected response");
  }
  const selected = await document
    .locator("select")
    .evaluateAll((controls) =>
      controls.some((control) => control instanceof HTMLSelectElement && control.selectedIndex > 0),
    );
  if (selected) {
    throw new Error("WeBWorK generated example has a selected response");
  }
}

async function captureGeneratedExample(
  runtime: ScenarioRuntime,
  example: GeneratedExample,
): Promise<void> {
  const session = await runtime.open(example.checkpoint);
  const page = session.page;
  try {
    await enterInstructor(page);
    await openInstructorLibraryBrowse(page);
    await page.getByRole("button", { name: new RegExp(`^${example.subject}`, "u") }).click();
    await page.getByRole("link", { name: "Search within results", exact: true }).click();
    const leaveSearch = page.getByRole("dialog", { name: "Leave this search?", exact: true });
    await leaveSearch.waitFor();
    await leaveSearch.getByRole("button", { name: "Leave page", exact: true }).click();
    await page.getByRole("button", { name: "Remove Questions: in no Pool", exact: true }).click();
    const search = page.getByRole("searchbox", { name: "Search Question Library", exact: true });
    await search.fill(example.title);
    await page.getByRole("button", { name: "Search", exact: true }).click();
    const result = page.locator(".record-list__row").filter({
      has: page.getByRole("heading", { name: example.title, exact: true }),
    });
    await followCaptureLink(page, result.getByRole("link", { name: "Open", exact: true }));
    await page.getByRole("heading", { level: 1, name: example.title, exact: true }).waitFor();
    await page.getByRole("region", { name: "Question prompt", exact: true }).waitFor();
    await renderedAnswerFreePreview(page, example);
    await runtime.captureCheckpoint(session, example.checkpoint);
  } finally {
    await runtime.close(session);
  }
}

async function instructorWebwork(runtime: ScenarioRuntime): Promise<void> {
  for (const example of GENERATED_EXAMPLES) await captureGeneratedExample(runtime, example);
}

function generatedExampleCapture(example: GeneratedExample): CaptureDeclaration {
  return {
    checkpoint: example.checkpoint,
    filenameStem: catalogScreenshotFilename(
      "questions",
      "browseQuestionLibrary",
      example.checkpoint,
    ),
    area: "question library",
    workflow: "generated Question preview",
    state: example.state,
    viewport: "laptop",
    privacyProfile: "instructor_answer_free",
    caption: example.caption,
  };
}

export const INSTRUCTOR_WEBWORK_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "instructor_webwork",
    role: "instructor",
    captures: GENERATED_EXAMPLES.map(generatedExampleCapture),
    viewportCoverage: viewportCoverage(["laptop"], {
      tablet: {
        target: "webwork_generated_example",
        reason:
          "webwork_generated_example captures the rendered answer-free WeBWorK preview on the Instructor Question Library detail route as a laptop representative substitution. The tablet layout remains unverified until a tablet replay is captured.",
      },
      phone: {
        target: "webwork_generated_example",
        reason:
          "webwork_generated_example captures the rendered answer-free WeBWorK preview on the Instructor Question Library detail route as a laptop representative substitution. The phone layout remains unverified until a phone replay is captured.",
      },
      square: {
        target: "webwork_generated_example",
        reason:
          "webwork_generated_example captures the rendered answer-free WeBWorK preview on the Instructor Question Library detail route as a laptop representative substitution. The square layout remains unverified until a square replay is captured.",
      },
    }),
    run: instructorWebwork,
  },
];
