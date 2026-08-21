import assert from "node:assert/strict";
import test from "node:test";

import { createCatalogRepository } from "../src/api/catalog_repository.ts";
import { EMPTY_CATALOG_QUERY, decodeCatalogBrowsePage } from "../src/pages/library_page_model.ts";

test("catalog browser rows use the public Question ID as their stable identity", async () => {
  const repository = createCatalogRepository({
    searchCatalog: async () => ({
      items: [
        {
          questionId: "7K3-M9QP",
          backend: "native",
          capabilities: [],
          metadata: {
            title: "Question",
            tags: [],
            taxonomy: [],
            license: { kind: "ccBy" },
            language: "en-US",
          },
          byline: { names: ["Fixture Instructor"] },
          scope: "public",
          lifecycle: { state: "published" },
          publishedAt: 1,
        },
      ],
      nextCursor: null,
      facets: {
        taxonomy: [],
        capabilities: [],
        licenses: [],
        statistics: { available: 0, unavailable: 1 },
      },
    }),
  });
  const page = decodeCatalogBrowsePage(await repository.search(EMPTY_CATALOG_QUERY, null));
  assert.equal(page.items[0]?.displayId, "7K3-M9QP");
  assert.equal(JSON.stringify(page).includes("problemId"), false);
});

test("catalog browser decoder rejects hidden identity fields", () => {
  assert.throws(() =>
    decodeCatalogBrowsePage({
      items: [
        {
          displayId: "7K3-M9QP",
          problemId: "hidden",
          title: "Question",
          summary: "Safe",
          taxonomy: [],
          capabilities: [],
          license: "ccBy",
        },
      ],
      nextCursor: null,
      aggregates: [],
    }),
  );
});
