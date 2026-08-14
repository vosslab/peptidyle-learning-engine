// instructor_catalog_binding.ts - one visible, exact catalog-result binding for J13.

import { expect, type Locator, type Page } from "@playwright/test";

/** Finds one exact human-readable current question without exposing opaque UUIDs. */
export async function catalogResultByQuestionId(
  page: Page,
  catalogSearchTitle: string,
  questionId?: string,
): Promise<Locator> {
  if (
    questionId !== undefined &&
    !/^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u.test(questionId)
  ) {
    throw new Error("catalog Question ID must be canonical");
  }
  let catalogRow = page.locator(".assignment-editor-catalog-results article", {
    has: page.getByRole("heading", { name: catalogSearchTitle, exact: true }),
  });
  if (questionId !== undefined) {
    catalogRow = catalogRow.filter({ has: page.locator("code", { hasText: questionId }) });
  }
  await expect(catalogRow).toHaveCount(1);
  await expect(catalogRow).toBeVisible();
  return catalogRow;
}
