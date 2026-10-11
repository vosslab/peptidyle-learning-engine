// QPM refresh contract: a fixed bank's seven response kinds remain usable in saved Draft previews.
// Failure plan: repair the QPM Native JSON engine or PLE decoder against NATIVE_JSON_SPEC.md.
// Selector contract: Import action in src/pages/question_drafts_page.tsx:299;
// fields, submit, status, results and links in src/pages/question_import_screen.tsx:185,193,217,224,243,254.
// Saved preview region, prompt and test action in
// src/features/question_draft_preview/draft_preview_panel.tsx:120,147,193.
import { expect, test } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  observeContextOrigins,
  requireScenarioInput,
  writeOriginReceipt,
} from "./real_stack_ui";

test("Instructor imports seven BBQ question kinds and opens every saved Draft preview", async ({
  browser,
}) => {
  test.setTimeout(120_000);
  expect(requireScenarioInput(configuredLiveDemoInputs).scenarioId).toBe("question_import");
  const bank = [
    "MC\tWhich molecule carries genetic information?\tDNA\tCorrect\tLipid\tIncorrect",
    "MA\tSelect the nucleic acids.\tDNA\tCorrect\tRNA\tCorrect\tProtein\tIncorrect",
    "FIB\tName the sugar in RNA.\tribose",
    "FIB_PLUS\tDNA contains [sugar] and [base].\tsugar\tdeoxyribose\t\tbase\tthymine",
    "NUM\tHow many strands form a DNA double helix?\t2\t0",
    "MAT\tMatch each molecule to its role.\tDNA\tGenetic storage\tEnzyme\tCatalysis",
    "ORD\tOrder the stages of gene expression.\tTranscription\tRNA processing\tTranslation",
  ];
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  observeContextOrigins(context, pageOrigins, requestOrigins);
  try {
    const page = await context.newPage();
    configureContextAndPage(context, page, 30_000);
    await chooseSeededIdentity(page, /Elena Rivera/u);
    await page.getByRole("link", { name: "Questions", exact: true }).click();
    await page.getByRole("link", { name: "My Draft Questions", exact: true }).click();
    await page.getByRole("button", { name: "Import questions", exact: true }).click();
    await page.getByLabel("Question bank file").setInputFiles({
      name: "seven_kinds.txt",
      mimeType: "text/plain",
      buffer: Buffer.from(`${bank.join("\n")}\n`),
    });
    await page.getByLabel("Question bank format").selectOption("bbq_text_upload");
    await page.getByRole("button", { name: "Import into Drafts", exact: true }).click();
    await expect(page.getByRole("status").filter({ hasText: "Import complete." })).toContainText(
      "7 of 7 questions saved as Drafts.",
    );
    const results = page.getByRole("region", { name: "Import results", exact: true });
    await expect(results.getByRole("listitem")).toHaveCount(7);
    for (const [index, line] of bank.entries()) {
      const opened = page.waitForEvent("popup");
      await results.getByRole("link").nth(index).click();
      const draft = await opened;
      const preview = draft.getByRole("region", { name: "Saved Draft preview and test" });
      await expect(
        preview.getByRole("article", { name: "Native Draft question prompt" }),
      ).toContainText(line.split("\t")[1]!);
      await expect(
        preview.getByRole("button", { name: "Test response", exact: true }),
      ).toBeVisible();
      await draft.close();
    }
  } finally {
    await context.close();
    writeOriginReceipt(pageOrigins, requestOrigins);
  }
});
