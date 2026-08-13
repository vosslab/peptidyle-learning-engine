// instructor_catalog_binding.spec.ts - hostile retained-catalog ambiguity evidence for J13.

import { expect, test } from "@playwright/test";

import { catalogResultByVersion } from "./instructor_catalog_binding";

const CATALOG_TITLE = "Fake amino acid question 123e4567e89b";

function catalogArticle(title: string, displayId: string): string {
  return `<article><h3>${title}</h3><code>${displayId}</code><button type="button">Add published version</button></article>`;
}

test("selects either requested human version when both remain visible", async ({ page }) => {
  await page.setContent(
    `<div class="assignment-editor-catalog-results">${catalogArticle(CATALOG_TITLE, "P-1-v1")}${catalogArticle(CATALOG_TITLE, "P-1-v2")}</div>`,
  );
  await expect(catalogResultByVersion(page, CATALOG_TITLE, 1)).resolves.toHaveCount(1);
  await expect(catalogResultByVersion(page, CATALOG_TITLE, 2)).resolves.toHaveCount(1);
});

test("returns the one exact rendered catalog result", async ({ page }) => {
  await page.setContent(
    `<div class="assignment-editor-catalog-results">${catalogArticle(CATALOG_TITLE, "P-1-v2")}</div>`,
  );
  await expect(catalogResultByVersion(page, CATALOG_TITLE, 2)).resolves.toHaveCount(1);
});
