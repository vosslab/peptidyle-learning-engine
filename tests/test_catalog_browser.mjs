import assert from "node:assert/strict";
import test from "node:test";

import { createCatalogRepository } from "../src/api/catalog_repository.ts";
import { createMockApiClient } from "../src/api/mock/client.ts";
import { EMPTY_CATALOG_QUERY, decodeCatalogBrowsePage } from "../src/pages/library_page_model.ts";

test("catalog browser rows use the Question ID as their safe stable identity", async () => {
  const repository = createCatalogRepository(createMockApiClient());
  const page = await repository.search(EMPTY_CATALOG_QUERY, null);
  const decoded = decodeCatalogBrowsePage(page);
  const row = decoded.items[0];
  assert.ok(row);
  assert.equal(row.displayId, "7K3-M9QP");
  assert.equal(JSON.stringify(row).includes("problemId"), false);
  assert.equal(JSON.stringify(row).includes("versionId"), false);
});

test("catalog decoder rejects hidden identity fields", () => {
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
