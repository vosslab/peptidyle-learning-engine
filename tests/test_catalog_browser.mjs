import assert from "node:assert/strict";
import test from "node:test";

import { createCatalogRepository } from "../src/api/catalog_repository.ts";
import { EMPTY_CATALOG_QUERY, decodeCatalogBrowsePage } from "../src/pages/library_page_model.ts";

test("catalog browser rows use the public Question ID as their stable identity", async () => {
  const repository = createCatalogRepository({
    searchCatalog: async () => ({
      items: [
        {
          summary: {
            questionId: "7K3-M9QP",
            backend: "native",
            responseFamily: "numeric",
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
          evidence: {
            state: "available",
            formulaVersion: 1,
            observedCourseCount: 2,
            independentLearnerObservationCount: 12,
            difficultyIndex: 0.75,
            attemptsMean: 1.2,
            timeMedianSecondsEstimate: 30,
            discriminationIndex: null,
            evidenceAt: 1,
          },
        },
      ],
      nextCursor: null,
      facets: {
        bylines: [],
        backends: [],
        tags: [],
        responseFamilies: [],
        taxonomy: [],
        capabilities: [],
        licenses: [],
        evidence: { available: 1, unavailable: 0 },
        usedInMyCourses: { used: 0 },
      },
    }),
  });
  const page = decodeCatalogBrowsePage(await repository.search(EMPTY_CATALOG_QUERY, null));
  assert.equal(page.items[0]?.displayId, "7K3-M9QP");
  assert.equal(page.items[0]?.evidence.state, "available");
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
          evidence: { state: "insufficientEvidence" },
        },
      ],
      nextCursor: null,
      aggregates: [],
    }),
  );
});

test("catalog browser evidence requires comparable multi-course observations", () => {
  assert.throws(() =>
    decodeCatalogBrowsePage({
      items: [
        {
          displayId: "7K3-M9QP",
          title: "Question",
          summary: "Safe",
          byline: [],
          taxonomy: [],
          capabilities: [],
          license: "ccBy",
          evidence: {
            state: "available",
            observedCourseCount: 1,
            independentLearnerObservationCount: 1,
            difficultyIndex: 0.5,
            discriminationIndex: undefined,
          },
        },
      ],
      nextCursor: null,
      aggregates: [],
    }),
  );
});

test("catalog browser carries server-provided discovery facets into one resettable query", async () => {
  let receivedQuery;
  const repository = createCatalogRepository({
    searchCatalog: async (query) => {
      receivedQuery = query;
      return {
        items: [],
        nextCursor: null,
        facets: {
          bylines: [{ byline: "Fixture Instructor", count: 3 }],
          backends: [{ backend: "native", count: 3 }],
          tags: [{ tag: "protein structure", count: 2 }],
          responseFamilies: [{ responseFamily: "numeric", count: 2 }],
          taxonomy: [],
          capabilities: [],
          licenses: [],
          evidence: { available: 2, unavailable: 1 },
          usedInMyCourses: { used: 1 },
        },
      };
    },
  });
  const page = decodeCatalogBrowsePage(
    await repository.search(
      {
        ...EMPTY_CATALOG_QUERY,
        byline: "Fixture Instructor",
        backend: "native",
        tag: "protein structure",
        responseFamily: "numeric",
        usedInMyCourses: "used",
      },
      null,
    ),
  );
  assert.deepEqual(receivedQuery.bylines, ["Fixture Instructor"]);
  assert.deepEqual(receivedQuery.backends, ["native"]);
  assert.deepEqual(receivedQuery.tags, ["protein structure"]);
  assert.deepEqual(receivedQuery.responseFamilies, ["numeric"]);
  assert.equal(receivedQuery.usedInMyCourses, "used");
  assert.equal(receivedQuery.authorship, "any");
  assert.deepEqual(
    page.aggregates.map((facet) => facet.group),
    ["byline", "backend", "tag", "responseFamily", "evidence", "evidence", "usedInMyCourses"],
  );
});

test("catalog repository composes the closed Mine scope without an actor identifier", async () => {
  let receivedQuery;
  const repository = createCatalogRepository(
    {
      searchCatalog: async (query) => {
        receivedQuery = query;
        return {
          items: [],
          nextCursor: null,
          facets: {
            bylines: [],
            backends: [],
            tags: [],
            responseFamilies: [],
            taxonomy: [],
            capabilities: [],
            licenses: [],
            evidence: { available: 0, unavailable: 0 },
            usedInMyCourses: { used: 0 },
          },
        };
      },
    },
    "authoredByCurrentActor",
  );
  await repository.search(EMPTY_CATALOG_QUERY, null);
  assert.equal(receivedQuery.authorship, "authoredByCurrentActor");
  assert.equal(Object.hasOwn(receivedQuery, "actor"), false);
  assert.equal(Object.hasOwn(receivedQuery, "actorId"), false);
});
