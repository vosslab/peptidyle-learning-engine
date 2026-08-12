// chapter_question_responses.ts - visible, keyboard-only response families for Chapter 1 journeys.

import { expect, type Locator, type Page } from "@playwright/test";

import { tabTo } from "./keyboard_walkthrough";

export type VisibleResponseFamily = "multiple-choice" | "matching";
export type VisibleFeedbackOutcome = "correct" | "not-quite" | "undisclosed";

export interface SubmittedVisibleResponse {
  readonly family: VisibleResponseFamily;
  readonly outcome: VisibleFeedbackOutcome;
}

/** Optional milestones for a caller that needs redacted progress, not response content. */
export interface VisibleResponseProgress {
  readonly responseSelected?: () => void;
  readonly feedbackVisible?: () => void;
}

async function activateWithKeyboard(page: Page, control: Locator): Promise<void> {
  await tabTo(page, control);
  await expect(control).toBeFocused();
  await page.keyboard.press("Space");
}

/** Identifies a response family only from the learner-visible controls. */
export async function visibleResponseFamily(page: Page): Promise<VisibleResponseFamily> {
  if ((await page.locator(".matching-group:visible").count()) > 0) return "matching";
  if ((await page.locator('input[type="radio"]:visible').count()) > 0) return "multiple-choice";
  throw new Error("the visible question has no supported response controls");
}

/** Proves a resumed, unsubmitted question has no persisted visible choice. */
export async function expectVisibleResponseControlsCleared(page: Page): Promise<void> {
  await expect
    .poll(
      async () => {
        if ((await page.locator(".matching-group:visible").count()) > 0) return "matching";
        if ((await page.locator('input[type="radio"]:visible').count()) > 0) {
          return "multiple-choice";
        }
        return "pending";
      },
      { timeout: 30_000 },
    )
    .not.toBe("pending");
  const family = await visibleResponseFamily(page);
  if (family === "multiple-choice") {
    const controls = page.locator('input[type="radio"]:visible');
    expect(await controls.count()).toBeGreaterThan(0);
    for (const control of await controls.all()) await expect(control).not.toBeChecked();
    return;
  }

  const controls = page.locator('.matching-group:visible [role="radio"]');
  expect(await controls.count()).toBeGreaterThan(0);
  for (const control of await controls.all()) {
    await expect(control).toHaveAttribute("aria-checked", "false");
  }
}

function permutationAt(values: number, positions: number, ordinal: number): number[] {
  const available = Array.from({ length: values }, (_, index) => index);
  const result: number[] = [];
  let remainder = ordinal;
  for (let position = 0; position < positions; position += 1) {
    const selectedIndex = remainder % available.length;
    remainder = Math.floor(remainder / available.length);
    const selected = available.splice(selectedIndex, 1)[0];
    if (selected === undefined) throw new Error("visible matching permutation was incomplete");
    result.push(selected);
  }
  return result;
}

function responseCandidateCount(
  family: VisibleResponseFamily,
  choiceCount: number,
  groupCount = choiceCount,
): number {
  if (family === "multiple-choice") return choiceCount;
  let count = 1;
  for (let index = 0; index < groupCount; index += 1) count *= choiceCount - index;
  return count;
}

async function opaqueChoiceIdentities(choices: Locator): Promise<readonly string[]> {
  const identities: string[] = [];
  const count = await choices.count();
  for (let index = 0; index < count; index += 1) {
    const identity = await choices.nth(index).getAttribute("data-choice-id");
    if (identity === null || identity === "") {
      throw new Error("visible matching choice omitted its opaque identity");
    }
    if (identities.includes(identity)) {
      throw new Error("visible matching group repeated an opaque choice identity");
    }
    identities.push(identity);
  }
  return identities;
}

async function visibleResponseCandidateCount(
  page: Page,
  family: VisibleResponseFamily,
): Promise<number> {
  if (family === "multiple-choice") return page.locator('input[type="radio"]:visible').count();
  const groups = page.locator(".matching-group:visible");
  const groupCount = await groups.count();
  const choiceCount = await groups.first().getByRole("radio").count();
  if (choiceCount < groupCount) {
    throw new Error("visible matching controls cannot make a distinct selection for every prompt");
  }
  return responseCandidateCount(family, choiceCount, groupCount);
}

/**
 * Selects one plausible response in every visible response family without reading
 * an answer key. Matching selections are deliberately distinct when the rendered
 * choices permit it, which mirrors a student making a complete visible response.
 */
export async function chooseVisibleResponseCandidate(
  page: Page,
  candidateOrdinal = 0,
): Promise<VisibleResponseFamily> {
  const family = await visibleResponseFamily(page);
  if (family === "multiple-choice") {
    const radios = page.locator('input[type="radio"]:visible');
    expect(await radios.count()).toBeGreaterThan(1);
    const responseCount = await radios.count();
    if (candidateOrdinal < 0 || candidateOrdinal >= responseCount) {
      throw new Error("multiple-choice candidate ordinal exceeded visible responses");
    }
    const response = radios.nth(candidateOrdinal);
    await expect(response).not.toBeChecked();
    await activateWithKeyboard(page, response);
    await expect(response).toBeChecked();
    return family;
  }

  const groups = page.locator(".matching-group:visible");
  expect(await groups.count()).toBeGreaterThan(0);
  const groupCount = await groups.count();
  const firstChoices = groups.first().getByRole("radio");
  const choiceIdentities = await opaqueChoiceIdentities(firstChoices);
  const choiceCount = choiceIdentities.length;
  if (choiceCount < groupCount) {
    throw new Error("visible matching controls cannot make a distinct selection for every prompt");
  }
  const candidateCount = responseCandidateCount(family, choiceCount, groupCount);
  if (candidateOrdinal < 0 || candidateOrdinal >= candidateCount) {
    throw new Error("matching candidate ordinal exceeded visible response permutations");
  }
  const matchingIndices = permutationAt(choiceCount, groupCount, candidateOrdinal);
  const selectedValues = new Set<string>();
  for (let index = 0; index < groupCount; index += 1) {
    const choices = groups.nth(index).getByRole("radio");
    const groupIdentities = await opaqueChoiceIdentities(choices);
    expect(groupIdentities.length).toBe(choiceCount);
    expect([...groupIdentities].sort()).toEqual([...choiceIdentities].sort());
    const targetIndex = matchingIndices[index];
    if (targetIndex === undefined) throw new Error("matching candidate had no visible choice");
    const targetIdentity = choiceIdentities[targetIndex];
    if (targetIdentity === undefined) throw new Error("matching candidate had no opaque choice");
    const responseIndex = groupIdentities.indexOf(targetIdentity);
    if (responseIndex < 0) throw new Error("visible matching group omitted a candidate choice");
    selectedValues.add(targetIdentity);
    const response = choices.nth(responseIndex);
    await expect(response).toHaveAttribute("aria-checked", "false");
    await activateWithKeyboard(page, response);
    await expect(response).toHaveAttribute("aria-checked", "true");
  }
  expect(selectedValues.size).toBe(groupCount);
  return family;
}

/** Compatibility name for a first visible response without any answer-key knowledge. */
export async function choosePlausibleVisibleResponse(page: Page): Promise<VisibleResponseFamily> {
  return chooseVisibleResponseCandidate(page);
}

async function visibleFeedbackOutcome(page: Page): Promise<VisibleFeedbackOutcome> {
  const feedback = page.getByRole("region", { name: "Feedback" });
  await expect(feedback.getByRole("heading", { name: "Feedback", exact: true })).toBeVisible();
  await expect(feedback.getByRole("heading", { name: "Your response" })).toBeVisible();
  if (await feedback.getByRole("heading", { name: "Correct", exact: true }).isVisible()) {
    return "correct";
  }
  if (await feedback.getByRole("heading", { name: "Not quite", exact: true }).isVisible()) {
    return "not-quite";
  }
  return "undisclosed";
}

/** Selects and submits the displayed question, then proves learner-visible feedback. */
export async function submitVisibleResponseCandidate(
  page: Page,
  candidateOrdinal = 0,
  progress: VisibleResponseProgress | undefined = undefined,
): Promise<SubmittedVisibleResponse> {
  const family = await chooseVisibleResponseCandidate(page, candidateOrdinal);
  await expect(page.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );
  progress?.responseSelected?.();
  const submit = page.getByRole("button", { name: "Submit answer" });
  await activateWithKeyboard(page, submit);
  const outcome = await visibleFeedbackOutcome(page);
  progress?.feedbackVisible?.();
  return { family, outcome };
}

/** Submits the first complete visible response for general response-family coverage. */
export async function answerAndSubmitVisibleQuestion(page: Page): Promise<VisibleResponseFamily> {
  const submitted = await submitVisibleResponseCandidate(page);
  return submitted.family;
}

/** Leaves feedback through its rendered continuation control. */
export async function continueFromVisibleFeedback(page: Page): Promise<void> {
  const continueButton = page.getByRole("button", { name: "Continue" });
  await activateWithKeyboard(page, continueButton);
}

/**
 * Learns only from the outcome a student can read in the feedback panel. It tries each
 * complete visible response at most once and fails closed when policy withholds correctness.
 */
export async function completeVisibleQuestionThroughFeedback(
  page: Page,
  firstCandidateOrdinal = 0,
  progress: VisibleResponseProgress | undefined = undefined,
): Promise<VisibleResponseFamily> {
  // Continue leaves the run route mounted while its next server-issued question loads.
  // Wait for a cleared visible response before inspecting its family so that the route shell
  // cannot be mistaken for a ready question.
  await expectVisibleResponseControlsCleared(page);
  const family = await visibleResponseFamily(page);
  const candidateCount = await visibleResponseCandidateCount(page, family);
  for (
    let candidateOrdinal = firstCandidateOrdinal;
    candidateOrdinal < candidateCount;
    candidateOrdinal += 1
  ) {
    await expectVisibleResponseControlsCleared(page);
    const submitted = await submitVisibleResponseCandidate(page, candidateOrdinal, progress);
    if (submitted.outcome === "correct") {
      await continueFromVisibleFeedback(page);
      return submitted.family;
    }
    if (submitted.outcome === "undisclosed") {
      throw new Error(
        "the visible feedback policy withholds correctness, so the learner cannot safely retry to mastery",
      );
    }
    await continueFromVisibleFeedback(page);
    await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible({ timeout: 30_000 });
  }
  throw new Error("every complete visible response was rejected by the rendered question");
}
