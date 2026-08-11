// instructor_catalog_binding.spec.ts - hostile retained-catalog ambiguity evidence for J13.

import { expect, test } from "@playwright/test";

import { exactCatalogResult } from "./instructor_catalog_binding";

const CATALOG_TITLE = "Pilot retry corpus pilotref123e4567e89b12d3a456426614174000";

function catalogArticle(title: string): string {
  return `<article><h3>${title}</h3><button type="button">Add published version</button></article>`;
}

test("rejects retained catalog ambiguity instead of selecting a first matching public result", async ({
  page,
}) => {
  await page.setContent(
    `<div class="assignment-editor-catalog-results">${catalogArticle(CATALOG_TITLE)}${catalogArticle(CATALOG_TITLE)}</div>`,
  );
  await expect(exactCatalogResult(page, CATALOG_TITLE)).rejects.toThrow();
});

test("returns the one exact rendered catalog result", async ({ page }) => {
  await page.setContent(
    `<div class="assignment-editor-catalog-results">${catalogArticle(CATALOG_TITLE)}</div>`,
  );
  await expect(exactCatalogResult(page, CATALOG_TITLE)).resolves.toHaveCount(1);
});
