// instructor_catalog_binding.ts - one visible, exact catalog-result binding for J13.

import { expect, type Locator, type Page } from "@playwright/test";

/**
 * Finds the single rendered catalog result for the fresh public arrangement title.
 * A retained catalog with duplicate matching titles fails closed instead of selecting an arbitrary row.
 */
export async function exactCatalogResult(page: Page, catalogSearchTitle: string): Promise<Locator> {
  const catalogRow = page.locator(".assignment-editor-catalog-results article", {
    has: page.getByRole("heading", { name: catalogSearchTitle, exact: true }),
  });
  await expect(catalogRow).toHaveCount(1);
  await expect(catalogRow).toBeVisible();
  return catalogRow;
}
