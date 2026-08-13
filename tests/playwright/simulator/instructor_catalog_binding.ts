// instructor_catalog_binding.ts - one visible, exact catalog-result binding for J13.

import { expect, type Locator, type Page } from "@playwright/test";

/** Finds one exact human-readable published version without opaque UUIDs. */
export async function catalogResultByVersion(
  page: Page,
  catalogSearchTitle: string,
  versionNumber: number,
): Promise<Locator> {
  if (!Number.isSafeInteger(versionNumber) || versionNumber < 1) {
    throw new Error("catalog version number must be a positive safe integer");
  }
  const displayIdSuffix = new RegExp(`-v${versionNumber}$`, "u");
  const catalogRow = page
    .locator(".assignment-editor-catalog-results article", {
      has: page.getByRole("heading", { name: catalogSearchTitle, exact: true }),
    })
    .filter({ has: page.locator("code", { hasText: displayIdSuffix }) });
  await expect(catalogRow).toHaveCount(1);
  await expect(catalogRow).toBeVisible();
  return catalogRow;
}
