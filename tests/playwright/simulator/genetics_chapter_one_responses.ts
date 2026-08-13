// genetics_chapter_one_responses.ts - visible teaching-content oracle for the Genetics pilot.

import { expect, type Locator, type Page } from "@playwright/test";

import {
  chooseVisibleResponseCandidate,
  continueFromVisibleFeedback,
  expectVisibleResponseControlsCleared,
  submitSelectedVisibleResponse,
  visibleResponseFamily,
  type VisibleResponseFamily,
  type VisibleResponseProgress,
} from "./chapter_question_responses";
import { tabTo } from "./keyboard_walkthrough";

interface DiagnosisRule {
  readonly diagnosis: string;
  readonly clues: readonly string[];
}

// This is an instructor-owned content oracle, not product grading data. The live journey reads only
// learner-visible prose and controls; it never imports authored source, calls an API, or inspects
// browser persistence. The clues express the biology a prepared learner is expected to recognize.
const DIAGNOSIS_RULES: readonly DiagnosisRule[] = [
  { diagnosis: "Achondroplasia", clues: ["growth of bone in the limbs"] },
  { diagnosis: "Beta-Thalassemia", clues: ["reduced hemoglobin"] },
  {
    diagnosis: "Cystic fibrosis",
    clues: ["glands that make mucus and sweat", "most critically the lungs"],
  },
  { diagnosis: "Duchenne muscular dystrophy", clues: ["muscle wasting"] },
  { diagnosis: "Fragile X syndrome", clues: ["changes in part of the x chromosome"] },
  { diagnosis: "Galactosemia", clues: ["sugar galactose"] },
  { diagnosis: "Hemophilia", clues: ["blood does not clot properly"] },
  { diagnosis: "Huntington's disease", clues: ["progressive breakdown degeneration"] },
  {
    diagnosis: "Maple syrup urine disease",
    clues: ["branched chain amino acids", "distinctive sweet odor"],
  },
  {
    diagnosis: "Marfan syndrome",
    clues: ["affects the connective tissue", "tall and thin with long arms"],
  },
  { diagnosis: "Phenylketonuria", clues: ["phenylalanine"] },
  { diagnosis: "Sickle-cell anemia", clues: ["stiff and sticky long and rigid"] },
  {
    diagnosis: "Tay-Sachs disease",
    clues: ["destruction of nerve cells", "beta hexosaminidase", "hexosaminidase a"],
  },
  {
    diagnosis: "Angelman syndrome",
    clues: ["duplications of the ube3a", "extra piece duplication of a specific gene"],
  },
  {
    diagnosis: "Prader-Willi syndrome",
    clues: [
      "loss of function of specific genes on chromosome 15",
      "deletion of a specific part of chromosome 15",
    ],
  },
  {
    diagnosis: "Cri du chat syndrome",
    clues: ["cat like cry", "chromosome 5 is missing", "specific part of chromosome 5"],
  },
  {
    diagnosis: "Philadelphia chromosome",
    clues: ["shortened version of human chromosome 22", "chromosome 9 and chromosome 22"],
  },
  {
    diagnosis: "DiGeorge syndrome",
    clues: ["chromosome 22 is missing", "specific part of chromosome 22"],
  },
  {
    diagnosis: "Down syndrome",
    clues: [
      "third copy of chromosome 21",
      "21st chromosome",
      "material from chromosome 21",
      "extra chromosome 21",
    ],
  },
  {
    diagnosis: "Edwards syndrome",
    clues: [
      "third copy of chromosome 18",
      "18th chromosome",
      "material from chromosome 18",
      "extra chromosome 18",
    ],
  },
  {
    diagnosis: "Klinefelter syndrome",
    clues: ["male to be born with an extra x chromosome", "47 xxy"],
  },
  {
    diagnosis: "Patau syndrome",
    clues: [
      "third copy of chromosome 13",
      "13th chromosome",
      "material from chromosome 13",
      "extra chromosome 13",
    ],
  },
  {
    diagnosis: "Triple X syndrome",
    clues: ["47 xxx", "female that has three x chromosomes", "three copies of the x chromosome"],
  },
  {
    diagnosis: "Turner syndrome",
    clues: ["one of their x chromosomes is missing", "monosomy x", "webbed neck"],
  },
  {
    diagnosis: "WAGR syndrome",
    clues: ["chromosome number 11", "chromosome 11 is missing", "specific part of chromosome 11"],
  },
  {
    diagnosis: "Wolf-Hirschhorn syndrome",
    clues: [
      "short arm of chromosome 4",
      "chromosome number 4",
      "chromosome 4 is missing",
      "specific part of chromosome 4",
    ],
  },
];

function normalizeVisibleText(value: string): string {
  return value
    .normalize("NFKD")
    .toLocaleLowerCase("en-US")
    .replace(/[^a-z0-9]+/gu, " ")
    .trim()
    .replace(/\s+/gu, " ");
}

/** Identifies one reviewed Genetics disorder from its learner-visible description. */
export function geneticsDiagnosisFromVisibleDescription(description: string): string {
  const normalized = normalizeVisibleText(description);
  const diagnoses = DIAGNOSIS_RULES.filter((rule) =>
    rule.clues.some((clue) => normalized.includes(normalizeVisibleText(clue))),
  ).map((rule) => rule.diagnosis);
  if (diagnoses.length !== 1) {
    throw new Error(
      `visible Genetics description resolved to ${diagnoses.length} reviewed diagnoses`,
    );
  }
  return diagnoses[0] as string;
}

function labelMatchesDiagnosis(label: string, diagnosis: string): boolean {
  return normalizeVisibleText(label).includes(normalizeVisibleText(diagnosis));
}

async function selectMultipleChoiceDiagnosis(page: Page, diagnosis: string): Promise<void> {
  const choices = page.locator(".choice-card:visible");
  const radios = page.locator('input[type="radio"]:visible');
  const count = await choices.count();
  expect(count).toBe(await radios.count());
  let selectedIndex = -1;
  for (let index = 0; index < count; index += 1) {
    const label = await choices.nth(index).innerText();
    if (labelMatchesDiagnosis(label, diagnosis)) selectedIndex = index;
  }
  if (selectedIndex < 0)
    throw new Error("visible multiple-choice options omit the diagnosed disorder");

  await expect(chooseVisibleResponseCandidate(page, selectedIndex)).resolves.toBe(
    "multiple-choice",
  );
}

async function visibleMatchingPrompt(group: Locator): Promise<string> {
  const prompt = group.locator(":scope > p").first();
  await expect(prompt).toBeVisible();
  return prompt.innerText();
}

async function matchingChoiceForDiagnosis(group: Locator, diagnosis: string): Promise<Locator> {
  const choices = group.getByRole("radio");
  for (let index = 0; index < (await choices.count()); index += 1) {
    const choice = choices.nth(index);
    const label = await choice.locator(".matching-choice-content > span").first().innerText();
    if (labelMatchesDiagnosis(label, diagnosis)) return choice;
  }
  throw new Error("visible matching options omit the diagnosed disorder");
}

async function selectMatchingDiagnoses(page: Page): Promise<void> {
  const groups = page.locator(".matching-group:visible");
  const count = await groups.count();
  expect(count).toBeGreaterThan(1);
  for (let index = 0; index < count; index += 1) {
    const group = groups.nth(index);
    const diagnosis = geneticsDiagnosisFromVisibleDescription(await visibleMatchingPrompt(group));
    const choice = await matchingChoiceForDiagnosis(group, diagnosis);
    await expect(choice).toHaveAttribute("aria-disabled", "false");
    await tabTo(page, choice);
    await expect(choice).toBeFocused();
    await page.keyboard.press("Space");
    await expect(choice).toHaveAttribute("aria-checked", "true");
  }
}

async function selectReviewedGeneticsResponse(page: Page): Promise<VisibleResponseFamily> {
  const family = await visibleResponseFamily(page);
  if (family === "multiple-choice") {
    const prompt = await page.locator(".prompt-copy:visible").innerText();
    await selectMultipleChoiceDiagnosis(page, geneticsDiagnosisFromVisibleDescription(prompt));
  } else {
    await selectMatchingDiagnoses(page);
  }
  return family;
}

/** Completes one reviewed Genetics question through visible prose, controls, and feedback only. */
export async function completeReviewedGeneticsQuestion(
  page: Page,
  progress: VisibleResponseProgress | undefined = undefined,
): Promise<VisibleResponseFamily> {
  await expectVisibleResponseControlsCleared(page);
  const family = await selectReviewedGeneticsResponse(page);
  const submitted = await submitSelectedVisibleResponse(page, family, progress);
  if (submitted.outcome !== "correct") {
    throw new Error("the reviewed visible Genetics reasoning did not receive Correct feedback");
  }
  await continueFromVisibleFeedback(page);
  return family;
}
