// Instructor WeBWorK generated-example capture through the published Question Library.
// Selector contract: the search field and result rows are owned by library_page.tsx; the
// generated-example iframe is owned by question_detail_page.tsx and its document by WeBWorK.

import type { Page } from "playwright";

import type { ScenarioRuntime } from "./runtime";
import type { ScenarioDefinition } from "./scenario_types";
import { enterInstructor } from "./visible_workflows";

const ANSWER_KEY_TEXT = /answer key|correct answer|correct feedback|private source/iu;

interface GeneratedExample {
  readonly checkpoint: string;
  readonly title: string;
  readonly search: string;
  readonly prompt: string;
  readonly control: "input[type=radio]" | "select";
  readonly header?: string;
}

const GENERATED_EXAMPLES: ReadonlyArray<GeneratedExample> = [
  {
    checkpoint: "generated_example_laptop",
    title: "Genetic Disorders from Descriptions",
    search: "Genetic disorders",
    prompt: "",
    control: "input[type=radio]",
  },
  {
    checkpoint: "hla_genotype_laptop",
    title: "Offspring HLA Genotypes (2 Markers, Black)",
    search: "Offspring HLA",
    prompt: "The mother has",
    control: "input[type=radio]",
  },
  {
    checkpoint: "monohybrid_matching_laptop",
    title: "Matching Monohybrid Cross Genotypes to Phenotypes",
    search: "Matching Monohybrid",
    prompt: "monohybrid crosses",
    control: "select",
  },
  {
    checkpoint: "x_linked_counts_laptop",
    title: "Parent Genotypes in X-Linked Recessive Crosses",
    search: "Parent Genotypes in X-Linked",
    prompt: "red-eyed (wildtype)",
    control: "input[type=radio]",
    header: "phenotype",
  },
  {
    checkpoint: "dna_structure_laptop",
    title: "True/False Statements About DNA Structure",
    search: "DNA Structure",
    prompt: "DNA",
    control: "input[type=radio]",
  },
  {
    checkpoint: "meiosis_prophase_laptop",
    title: "Matching Meiosis Prophase I Stages to Descriptions",
    search: "Matching Meiosis Prophase",
    prompt: "stages of meiosis prophase I",
    control: "select",
  },
  {
    checkpoint: "chi_square_laptop",
    title: "True/False Statements About Chi-Square Tests",
    search: "Chi-Square Tests",
    prompt: "chi-square",
    control: "input[type=radio]",
  },
  {
    checkpoint: "chromosome_shapes_laptop",
    title: "Matching Chromosome Shapes to Descriptions",
    search: "Matching Chromosome Shapes",
    prompt: "categories of chromosome shape",
    control: "select",
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
  const selected = await document.locator("select").evaluateAll((controls) =>
    controls.some((control) => control.selectedIndex > 0),
  );
  if (selected) {
    throw new Error("WeBWorK generated example has a selected response");
  }
}

async function captureGeneratedExample(
  runtime: ScenarioRuntime,
  example: GeneratedExample,
): Promise<void> {
  const scenario = "instructor_webwork";
  const checkpoint = example.checkpoint;
  const session = await runtime.open(runtime.record(scenario, checkpoint));
  const page = session.page;
  try {
    await enterInstructor(page);
    await page.getByLabel("Search published questions", { exact: true }).fill(example.search);
    const result = page.locator("article.question-library-row").filter({
      has: page.getByRole("heading", { name: example.title, exact: true }),
    });
    await result.getByRole("link", { name: "Open question", exact: true }).click();
    await page.getByRole("heading", { level: 1, name: example.title, exact: true }).waitFor();
    await page.getByRole("region", { name: "Question prompt", exact: true }).waitFor();
    await renderedAnswerFreePreview(page, example);
    await runtime.capture(session, runtime.record(scenario, checkpoint));
  } finally {
    await runtime.close(session);
  }
}

async function instructorWebwork(runtime: ScenarioRuntime): Promise<void> {
  for (const example of GENERATED_EXAMPLES) await captureGeneratedExample(runtime, example);
}

export const INSTRUCTOR_WEBWORK_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "instructor_webwork",
    checkpoints: GENERATED_EXAMPLES.map((example) => example.checkpoint),
    run: instructorWebwork,
  },
];
