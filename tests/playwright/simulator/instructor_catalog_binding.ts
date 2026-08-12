// instructor_catalog_binding.ts - one visible, exact catalog-result binding for J13.

import { expect, type Locator, type Page } from "@playwright/test";

/**
 * Finds the current v2 catalog result for a reviewed title.  Retained v1 rows
 * remain visible, while the current human-readable P-n-v2 identity selects the
 * reviewed retry-feedback revision without relying on an opaque UUID.
 */
export async function currentCatalogResult(page: Page, catalogSearchTitle: string): Promise<Locator> {
  const catalogRow = page.locator(".assignment-editor-catalog-results article", {
    has: page.getByRole("heading", { name: catalogSearchTitle, exact: true }),
  }).filter({ has: page.locator("code", { hasText: /-v2$/u }) });
  await expect(catalogRow).toHaveCount(1);
  await expect(catalogRow).toBeVisible();
  return catalogRow;
}
