// Ordinary visible HOTSPOT authoring and Student persistence/submission proof.
import { Buffer } from "node:buffer";
import type { Page } from "playwright";
import type { StudentAssessmentAttemptPresentation } from "../../../src/api/assessment_attempt_navigation";
import { decodeStudentAssessmentAttemptPresentation } from "../../../src/api/decoders/assessment_attempt_navigation";

const DESCRIPTION = "One dark dot in the center of a light image";
const EXPECTED_QUESTION_COUNT = 9;

async function navigateQuestion(
  page: Page,
  position: number,
  action: () => Promise<unknown>,
): Promise<StudentAssessmentAttemptPresentation> {
  const route = new URL(page.url());
  const attempt =
    /^\/assessment-attempts\/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/u.exec(
      route.pathname,
    )?.[1];
  if (attempt === undefined)
    throw new Error("Student navigation requires an Assessment Attempt route.");
  const delivered = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      url.origin === route.origin &&
      url.pathname === `/api/assessment-attempts/${attempt}/student-question` &&
      url.search === `?position=${position}` &&
      response.request().method() === "GET" &&
      response.status() === 200
    );
  });
  await action();
  // ASVS 2.2.1: validate the delivery shape and requested position; never log response bodies.
  const question = decodeStudentAssessmentAttemptPresentation(await (await delivered).json());
  if (question.position !== position)
    throw new Error("Student navigation did not deliver the requested Question position.");
  return question;
}

function hotspotRegionRole(question: StudentAssessmentAttemptPresentation): "radio" | "checkbox" {
  const response = question.presentation.response;
  if (response.kind !== "hotspot")
    throw new Error("HOTSPOT region readiness requires a public HOTSPOT presentation.");
  // Match the renderer's public selection bounds, never correct-answer cardinality.
  return response.minimum === 1 && response.maximum === 1 ? "radio" : "checkbox";
}

async function waitForDeliveredQuestionControl(
  page: Page,
  question: StudentAssessmentAttemptPresentation,
): Promise<void> {
  await waitForQuestionControl(page, question.position);
  const response = question.presentation.response;
  if (response.kind !== "hotspot") return;
  const pin = question.presentation.questionRevisionTuple;
  const assetPath = `/api/questions/${encodeURIComponent(pin.questionId)}/revisions/${pin.revisionNumber}/assets/${encodeURIComponent(response.surface.questionAssetTuple.questionAssetId)}`;
  const assetUrl = new URL(assetPath, page.url()).href;
  const control = page.locator("section.question-response-control");
  // The Save action is generic. Require the delivered immutable image and its loaded overlays.
  await control.locator(`.hotspot-image-surface img[src="${assetUrl}"]`).waitFor();
  await control.getByRole("img", { name: DESCRIPTION, exact: true }).waitFor();
  await control.getByRole(hotspotRegionRole(question), { name: "Dot", exact: true }).waitFor();
  // HOTSPOT mounts pointer regions only after this exact image's successful load event.
  await control
    .locator(".hotspot-image-surface")
    .getByRole("button", { name: "Dot", exact: true })
    .waitFor();
}

async function waitForQuestionControl(page: Page, position: number): Promise<void> {
  await page
    .getByText(`Question ${position} of ${EXPECTED_QUESTION_COUNT}`, { exact: true })
    .waitFor();
  // The summary changes before presentation delivery; wait for the actual response surface.
  await page
    .locator('section.question-response-control, iframe[title="Question document"]')
    .first()
    .waitFor();
  if (await page.locator("section.backend-owned-document").isVisible()) {
    await page.locator('iframe[title="Question document"]').waitFor();
    await page.frameLocator('iframe[title="Question document"]').locator("form").first().waitFor();
  } else {
    await page
      .locator("section.question-response-control")
      .getByRole("button", { name: "Save response", exact: true })
      .waitFor();
  }
}

async function waitForCurrentQuestionControl(page: Page): Promise<void> {
  const navigation = page.getByRole("navigation", { name: "Assessment questions", exact: true });
  const current = navigation.locator('button[aria-current="step"]');
  await current.waitFor();
  const label = await current.getAttribute("aria-label");
  const position = /^Question ([1-9][0-9]*):/u.exec(label ?? "")?.[1];
  if (position === undefined)
    throw new Error("Current Assessment Question navigation control lacks a valid position.");
  await waitForQuestionControl(page, Number(position));
}

async function returnToQuestion(
  page: Page,
  position: number,
): Promise<StudentAssessmentAttemptPresentation> {
  const navigation = page.getByRole("navigation", { name: "Assessment questions", exact: true });
  // Progress/navigation appears before its initial presentation. Do not activate until it renders.
  await waitForCurrentQuestionControl(page);
  const first = navigation.getByRole("button", { name: /^Question 1:/u });
  await first.waitFor();
  // Selecting the current position is a no-op. Move away so reset observes a real delivery GET.
  if ((await first.getAttribute("aria-current")) === "step") {
    const next = await navigateQuestion(page, 2, () =>
      navigation.getByRole("button", { name: "Next question", exact: true }).click(),
    );
    await waitForDeliveredQuestionControl(page, next);
  }
  let question = await navigateQuestion(page, 1, () => first.click());
  await waitForDeliveredQuestionControl(page, question);
  for (let candidate = 2; candidate <= position; candidate += 1) {
    question = await navigateQuestion(page, candidate, () =>
      navigation.getByRole("button", { name: "Next question", exact: true }).click(),
    );
    await waitForDeliveredQuestionControl(page, question);
  }
  return question;
}

export async function authorHotspot(page: Page): Promise<void> {
  // Construct a disposable raster, not a fake asset identity or a tracked fixture.
  const raster = await page.evaluate(() => {
    const canvas = document.createElement("canvas");
    canvas.width = 640;
    canvas.height = 320;
    const context = canvas.getContext("2d");
    if (context === null) throw new Error("Raster construction is unavailable.");
    context.fillStyle = "#edf4f2";
    context.fillRect(0, 0, 640, 320);
    context.fillStyle = "#202020";
    context.beginPath();
    context.arc(320, 160, 32, 0, Math.PI * 2);
    context.fill();
    return canvas.toDataURL("image/png").split(",")[1];
  });
  if (raster === undefined) throw new Error("Raster construction produced no PNG.");
  await page.getByLabel("Upload image").setInputFiles({
    name: "one-dot.png",
    mimeType: "image/png",
    buffer: Buffer.from(raster, "base64"),
  });
  await page.getByLabel("Image description", { exact: true }).fill(DESCRIPTION);
  await page.getByLabel("Region 1 label", { exact: true }).fill("Dot");
  for (const [coordinate, value] of Object.entries({ x: 4500, y: 4000, width: 1000, height: 2000 }))
    await page.getByLabel(`Region 1 ${coordinate}`, { exact: true }).fill(String(value));
  await page.getByLabel("Region 1 is correct", { exact: true }).check();
}

export async function exerciseHotspot(page: Page, input: "pointer" | "keyboard"): Promise<void> {
  const navigation = page.getByRole("navigation", { name: "Assessment questions", exact: true });
  let question = await returnToQuestion(page, 1);
  let position: number | undefined;
  for (let candidate = 1; candidate <= EXPECTED_QUESTION_COUNT; candidate += 1) {
    if (candidate > 1) {
      question = await navigateQuestion(page, candidate, () =>
        navigation.getByRole("button", { name: "Next question", exact: true }).click(),
      );
      await waitForDeliveredQuestionControl(page, question);
    }
    if (question.presentation.response.kind === "hotspot") {
      position = candidate;
      break;
    }
  }
  if (position === undefined) throw new Error("The released Assessment did not deliver HOTSPOT.");
  const pin = question.presentation.questionRevisionTuple;
  const control = page.locator("section.question-response-control");
  const image = control.getByRole("img", { name: DESCRIPTION, exact: true });
  await image.waitFor();
  // The delivered public bounds determine the control role, not the grading rule.
  const dot = control.getByRole(hotspotRegionRole(question), { name: "Dot", exact: true });
  if (await dot.isChecked()) throw new Error("HOTSPOT interaction requires an unanswered region.");
  if (input === "pointer") {
    await control
      .locator(".hotspot-image-surface")
      .getByRole("button", { name: "Dot", exact: true })
      .click();
  } else {
    await dot.focus();
    await page.keyboard.press("Space");
  }
  if (!(await dot.isChecked()))
    throw new Error(`${input} selection did not update the labeled region.`);
  async function saveAndReload(): Promise<void> {
    await control.getByRole("button", { name: "Save response", exact: true }).click();
    await page.getByText("Response saved.", { exact: true }).waitFor();
    await page.reload();
    // Reload may recommend another unanswered position; return through ordinary navigation.
    if (position === undefined) throw new Error("Missing HOTSPOT position after reload.");
    const restored = await returnToQuestion(page, position);
    const restoredPin = restored.presentation.questionRevisionTuple;
    if (
      restored.presentation.response.kind !== "hotspot" ||
      restoredPin.questionId !== pin.questionId ||
      restoredPin.revisionNumber !== pin.revisionNumber
    )
      throw new Error("Reload did not restore the exact issued HOTSPOT Question Revision.");
    await dot.waitFor();
    if (!(await dot.isChecked()))
      throw new Error("Saved HOTSPOT selection did not survive reload.");
  }
  await saveAndReload();
  await page.getByRole("button", { name: "Submit Assessment", exact: true }).click();
  await page.locator('[data-route-surface="assessmentAttemptSummary"]').waitFor();
  const recorded = page.getByRole("article").filter({
    has: page.getByRole("heading", { name: `Question ${position}`, exact: true }),
  });
  await recorded.getByText("Submitted.", { exact: true }).waitFor();
  await recorded.getByText("Marked correct.", { exact: true }).waitFor();
}
