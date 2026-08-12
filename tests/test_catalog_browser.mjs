// MOD-UI-BROWSE mock-surface behavior: cursor boundedness and aggregate provenance.

import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

import {
  EMPTY_CATALOG_QUERY,
  CatalogBrowseSession,
  catalogVirtualWindow,
  createSyntheticCatalogRepository,
  decodeCatalogBrowsePage,
} from "../src/pages/library_page_model.ts";
import { createCatalogRepository } from "../src/api/catalog_repository.ts";
import { createMockApiClient } from "../src/api/mock/client.ts";

test("10,000-row catalog stays cursor-bounded and never full-fetches", async () => {
  const repository = createSyntheticCatalogRepository(10_000, 40);
  const states = [];
  const session = new CatalogBrowseSession(repository, (state) => states.push(state));

  await session.reset(EMPTY_CATALOG_QUERY);
  assert.equal(repository.requests.length, 1);
  assert.equal(session.state.kind, "ready");
  assert.equal(session.state.kind === "ready" ? session.state.rows.length : 0, 40);

  await Promise.all([session.loadNext(), session.loadNext(), session.loadNext()]);
  assert.equal(repository.requests.length, 2);
  assert.equal(session.state.kind === "ready" ? session.state.rows.length : 0, 80);
  assert.ok(repository.requests.length < 10_000 / 40);
  assert.ok(states.length > 0);
});

test("a 10,000-row result has a bounded virtual DOM slice at any scroll position", () => {
  const rows = Array.from({ length: 10_000 }, (_, index) => index);
  const initial = catalogVirtualWindow(rows, 0, 560, 116, 5);
  const middle = catalogVirtualWindow(rows, 580_000, 560, 116, 5);

  assert.ok(initial.rows.length <= 15);
  assert.ok(middle.rows.length <= 15);
  assert.ok(middle.offset > 0);
});

test("aggregate counts are supplied unchanged by the repository rather than derived from loaded rows", async () => {
  const repository = createSyntheticCatalogRepository(10_000, 25);
  const session = new CatalogBrowseSession(repository, () => undefined);
  await session.reset(EMPTY_CATALOG_QUERY);

  assert.equal(session.state.kind, "ready");
  if (session.state.kind !== "ready") {
    return;
  }
  assert.equal(session.state.rows.length, 25);
  assert.deepEqual(session.state.aggregates, [
    { group: "taxonomy", value: "Biochemistry", count: 10_000 },
    { group: "capability", value: "algorithmic", count: 10_000 },
    { group: "license", value: "CC BY 4.0", count: 10_000 },
    { group: "statistic", value: "k-anonymous", count: 10_000 },
  ]);
});

test("retry and repeated cursor pages retain stable, duplicate-free immutable rows", async () => {
  const page = {
    items: [
      {
        displayId: "P-1-v1",
        problemId: "problem-a",
        versionId: "version-a",
        title: "A",
        summary: "Summary",
        taxonomy: ["Bio"],
        capabilities: ["algorithmic"],
        license: "CC BY",
      },
    ],
    nextCursor: "next",
    aggregates: [{ group: "taxonomy", value: "Bio", count: 2 }],
  };
  let request = 0;
  const repository = {
    search: async () => {
      request += 1;
      if (request === 1) {
        return page;
      }
      return { ...page, nextCursor: null };
    },
  };
  const session = new CatalogBrowseSession(repository, () => undefined);
  await session.reset(EMPTY_CATALOG_QUERY);
  await session.loadNext();
  assert.equal(session.state.kind, "ready");
  assert.equal(session.state.kind === "ready" ? session.state.rows.length : 0, 1);
  await session.retry();
  assert.equal(session.state.kind === "ready" ? session.state.rows.length : 0, 1);
});

test("a failed load-more keeps the last good aggregates and retries only that cursor", async () => {
  const aggregates = [{ group: "taxonomy", value: "Bio", count: 2 }];
  const row = (suffix) => ({
    displayId: `P-${suffix === "one" ? 1 : 2}-v1`,
    problemId: `problem-${suffix}`,
    versionId: `version-${suffix}`,
    title: `Problem ${suffix}`,
    summary: "Summary",
    taxonomy: ["Bio"],
    capabilities: ["algorithmic"],
    license: "CC BY",
  });
  const calls = [];
  const repository = {
    search: async (_query, cursor) => {
      calls.push(cursor);
      if (calls.length === 1) {
        return { items: [row("one")], nextCursor: "second", aggregates };
      }
      if (calls.length === 2) {
        throw new Error("temporary catalog failure");
      }
      return { items: [row("two")], nextCursor: null, aggregates };
    },
  };
  const session = new CatalogBrowseSession(repository, () => undefined);
  await session.reset(EMPTY_CATALOG_QUERY);
  await session.loadNext();
  assert.equal(session.state.kind, "error");
  assert.deepEqual(session.state.kind === "error" ? session.state.aggregates : [], aggregates);
  assert.equal(session.state.kind === "error" ? session.state.nextCursor : null, "second");

  await session.retry();
  assert.deepEqual(calls, [null, "second", "second"]);
  assert.equal(session.state.kind, "ready");
  assert.equal(session.state.kind === "ready" ? session.state.rows.length : 0, 2);
  assert.deepEqual(session.state.kind === "ready" ? session.state.aggregates : [], aggregates);
});

test("the library facet accessor presents preserved aggregates while the session is in error", () => {
  const source = fs.readFileSync("src/pages/library_page.tsx", "utf8");
  assert.match(source, /return current\.aggregates;/);
  assert.doesNotMatch(source, /current\.kind === "error" \? \[\] : current\.aggregates/);
});

test("a query change during a request discards its late page and starts at a fresh cursor", async () => {
  const calls = [];
  let releaseFirst;
  const firstPage = new Promise((resolve) => {
    releaseFirst = resolve;
  });
  const repository = {
    search: async (query, cursor) => {
      calls.push({ query, cursor });
      if (calls.length === 1) {
        return firstPage;
      }
      return {
        items: [
          {
            displayId: "P-1-v1",
            problemId: "problem-fresh",
            versionId: "version-fresh",
            title: "Fresh result",
            summary: "Fresh summary",
            taxonomy: ["Bio"],
            capabilities: ["algorithmic"],
            license: "CC BY",
          },
        ],
        nextCursor: null,
        aggregates: [{ group: "taxonomy", value: "Bio", count: 1 }],
      };
    },
  };
  const session = new CatalogBrowseSession(repository, () => undefined);
  const initial = session.reset(EMPTY_CATALOG_QUERY);
  await session.reset({ ...EMPTY_CATALOG_QUERY, search: "fresh" });
  releaseFirst({ items: [], nextCursor: null, aggregates: [] });
  await initial;
  await new Promise((resolve) => setTimeout(resolve, 0));

  assert.equal(calls.length, 2);
  assert.equal(calls[1]?.cursor, null);
  assert.equal(calls[1]?.query.search, "fresh");
  assert.equal(session.state.kind, "ready");
  assert.equal(
    session.state.kind === "ready" ? session.state.rows[0]?.problemId : "",
    "problem-fresh",
  );
});

test("hostile catalog data is rejected before it reaches the library UI", () => {
  assert.throws(() =>
    decodeCatalogBrowsePage({ items: [], nextCursor: null, aggregates: [], answerKey: "secret" }),
  );
  assert.throws(() => decodeCatalogBrowsePage({ items: [], nextCursor: 9, aggregates: [] }));
  assert.throws(() =>
    decodeCatalogBrowsePage({
      items: Array.from({ length: 101 }, () => ({
        displayId: "P-1-v1",
        problemId: "p",
        versionId: "v",
        title: "t",
        summary: "s",
        taxonomy: [],
        capabilities: [],
        license: "l",
      })),
      nextCursor: null,
      aggregates: [],
    }),
  );
  assert.throws(() =>
    decodeCatalogBrowsePage({
      items: [],
      nextCursor: null,
      aggregates: [{ group: "taxonomy", value: "Bio", count: 1_000_000_001 }],
    }),
  );
});

test("production catalog repository uses the accepted mock search and immutable detail transport", async () => {
  const client = createMockApiClient();
  const repository = createCatalogRepository(client);
  const page = await repository.search(EMPTY_CATALOG_QUERY, null);
  const decoded = decodeCatalogBrowsePage(page);
  assert.equal(decoded.items.length, 1);
  assert.equal(decoded.items[0].displayId, "P-1-v1");
  assert.equal(decoded.nextCursor, null);
  assert.ok(decoded.aggregates.some((facet) => facet.group === "taxonomy"));
  assert.ok(decoded.aggregates.some((facet) => facet.group === "capability"));
  assert.ok(decoded.aggregates.some((facet) => facet.group === "license"));
  assert.equal(decoded.aggregates.filter((facet) => facet.group === "statistic").length, 2);
  const detail = await client.getCatalogProblemDetail(
    decoded.items[0].problemId,
    decoded.items[0].versionId,
  );
  assert.equal(detail.statistics, "unavailable");
  assert.equal("source" in detail, false);
  assert.equal("grading" in detail, false);
});
