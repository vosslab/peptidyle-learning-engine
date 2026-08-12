// instructor_catalog_binding.spec.ts - hostile retained-catalog ambiguity evidence for J13.

import { expect, test } from "@playwright/test";

import { currentCatalogResult } from "./instructor_catalog_binding";

const CATALOG_TITLE = "Fake amino acid question 123e4567e89b";

function catalogArticle(title: string, displayId: string): string {
  return `<article><h3>${title}</h3><code>${displayId}</code><button type="button">Add published version</button></article>`;
}

test("selects the current v2 row while preserving retained v1 visibility", async ({
  page,
}) => {
  await page.setContent(
    `<div class="assignment-editor-catalog-results">${catalogArticle(CATALOG_TITLE, "P-1-v1")}${catalogArticle(CATALOG_TITLE, "P-1-v2")}</div>`,
  );
  await expect(currentCatalogResult(page, CATALOG_TITLE)).resolves.toHaveCount(1);
});

test("returns the one exact rendered catalog result", async ({ page }) => {
  await page.setContent(
    `<div class="assignment-editor-catalog-results">${catalogArticle(CATALOG_TITLE, "P-1-v2")}</div>`,
  );
  await expect(currentCatalogResult(page, CATALOG_TITLE)).resolves.toHaveCount(1);
});
