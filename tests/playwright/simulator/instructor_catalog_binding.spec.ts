// instructor_catalog_binding.spec.ts - hostile retained-catalog ambiguity evidence for J13.

import { expect, test } from "@playwright/test";

import { catalogResultByQuestionId } from "./instructor_catalog_binding";

const CATALOG_TITLE = "Fake amino acid question 123e4567e89b";

function catalogArticle(title: string, displayId: string): string {
  return `<article><h3>${title}</h3><code>${displayId}</code><button type="button">Add question</button></article>`;
}

test("selects an exact Question ID when titles are duplicated", async ({ page }) => {
  await page.setContent(
    `<div class="assignment-editor-catalog-results">${catalogArticle(CATALOG_TITLE, "7K3-M9QP")}${catalogArticle(CATALOG_TITLE, "ABC-123T")}</div>`,
  );
  await expect(catalogResultByQuestionId(page, CATALOG_TITLE, "7K3-M9QP")).resolves.toHaveCount(1);
  await expect(catalogResultByQuestionId(page, CATALOG_TITLE, "ABC-123T")).resolves.toHaveCount(1);
});

test("returns the one exact rendered catalog result", async ({ page }) => {
  await page.setContent(
    `<div class="assignment-editor-catalog-results">${catalogArticle(CATALOG_TITLE, "7K3-M9QP")}</div>`,
  );
  await expect(catalogResultByQuestionId(page, CATALOG_TITLE)).resolves.toHaveCount(1);
});
