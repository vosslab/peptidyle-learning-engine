// MOD-CLIENT catalog search/detail boundary tests: strict wire safety and exact URL behavior.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";
import { DecodeError } from "../src/api/decoder.ts";
import { decodeCatalogProblemDetail, decodeCatalogSearchPage } from "../src/api/decoders.ts";
import { ApiProtocolError, ApiRequestError, createHttpApiClient } from "../src/api/http_client.ts";
import { createMockApiClient } from "../src/api/mock/client.ts";
import { createMockFetch } from "../src/api/mock/handlers.ts";

const EMPTY_SEARCH = {
  text: null,
  taxonomy: [],
  capabilities: [],
  licenses: [],
  statistics: "any",
  cursor: null,
  pageSize: null,
};

async function mockJson(path) {
  const response = await createMockFetch()(path);
  assert.equal(response.status, 200);
  return response.json();
}

test("catalog decoders recursively reject hostile fields and unbounded response values", async () => {
  const search = await mockJson("/api/problems/search");
  assert.deepEqual(decodeCatalogSearchPage(search), search);

  assert.throws(() => decodeCatalogSearchPage({ ...search, answerKey: "secret" }), DecodeError);
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...search,
        items: search.items.map((item) => ({
          ...item,
          metadata: { ...item.metadata, source: "private/path.pg" },
        })),
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...search,
        items: Array.from({ length: 101 }, () => search.items[0]),
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...search,
        facets: {
          ...search.facets,
          taxonomy: Array.from({ length: 65 }, () => search.facets.taxonomy[0]),
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...search,
        facets: {
          ...search.facets,
          statistics: { available: Number.MAX_SAFE_INTEGER + 1, unavailable: 0 },
        },
      }),
    DecodeError,
  );
});

test("catalog detail decoder exposes only its safe immutable projection", async () => {
  const detail = await mockJson(
    `/api/problems/${publishedProblemFixture.publishedProblem.problem}/versions/${publishedProblemFixture.publishedProblem.version}/detail`,
  );
  assert.deepEqual(decodeCatalogProblemDetail(detail), detail);

  for (const forbidden of ["source", "response", "grading", "answerKey", "provider", "token"]) {
    assert.throws(
      () => decodeCatalogProblemDetail({ ...detail, [forbidden]: "secret" }),
      DecodeError,
    );
  }
  assert.doesNotMatch(
    JSON.stringify(detail),
    /"(?:source|response|grading|answerKey|provider|token)"\s*:/i,
  );
});

test("catalog search returns one immutable version for its copied human display ID", async () => {
  const summary = publishedProblemFixture.catalogProblem;
  const displayId = `P-${summary.publicId}-v${summary.versionNumber}`;
  const page = await mockJson(`/api/problems/search?text=${encodeURIComponent(displayId)}`);

  assert.equal(page.items.length, 1);
  assert.equal(page.items[0].publicId, summary.publicId);
  assert.equal(page.items[0].versionNumber, summary.versionNumber);
});

test("catalog detail decoder preserves scalar suppression and strictly decodes safe statistics", async () => {
  const detail = await mockJson(
    `/api/problems/${publishedProblemFixture.publishedProblem.problem}/versions/${publishedProblemFixture.publishedProblem.version}/detail`,
  );
  const available = {
    available: {
      cohortSize: 5,
      difficultyIndex: 0.7,
      attemptsMean: 1.2,
      timeMedianSecondsEstimate: 30,
      discriminationIndex: -0.4,
    },
  };
  const decoded = decodeCatalogProblemDetail({ ...detail, statistics: available });
  assert.deepEqual(decoded.statistics, available);
  assert.equal(decodeCatalogProblemDetail(detail).statistics, "unavailable");

  for (const statistics of [
    null,
    { available: null },
    { available: [] },
    { available: { ...available.available, cohortSize: 4 } },
    { available: { ...available.available, difficultyIndex: 1.01 } },
    { available: { ...available.available, attemptsMean: 0.9 } },
    { available: { ...available.available, timeMedianSecondsEstimate: -1 } },
    { available: { ...available.available, timeMedianSecondsEstimate: 0 } },
    { available: { ...available.available, timeMedianSecondsEstimate: 7 } },
    { available: { ...available.available, timeMedianSecondsEstimate: 86_401 } },
    { available: { ...available.available, discriminationIndex: null } },
    { available: { ...available.available, discriminationIndex: -1.01 } },
    { available: { ...available.available, privateAttemptCount: 5 } },
    { available: available.available, unavailable: true },
    "available",
  ]) {
    assert.throws(() => decodeCatalogProblemDetail({ ...detail, statistics }), DecodeError);
  }

  for (const privateField of [
    "tenant",
    "student",
    "run",
    "attempt",
    "source",
    "provider",
    "feedback",
    "answer",
    "key",
  ]) {
    assert.throws(
      () =>
        decodeCatalogProblemDetail({
          ...detail,
          statistics: { available: { ...available.available, [privateField]: "private" } },
        }),
      DecodeError,
    );
  }
});

test("HTTP catalog client serializes repeated filters, cursor, and page size on a same-origin search URL", async () => {
  const mockFetch = createMockFetch();
  const requests = [];
  async function captureFetch(input, init) {
    const request = new Request(new URL(input.toString(), "https://client.example.test"), init);
    requests.push(request);
    const url = new URL(request.url);
    return mockFetch(`${url.pathname}${url.search}`, init);
  }
  const client = createHttpApiClient({ fetch: captureFetch });
  const term = publishedProblemFixture.catalogProblem.metadata.taxonomy[0];
  assert.notEqual(term, undefined);
  const result = await client.searchCatalog({
    ...EMPTY_SEARCH,
    text: "peptide bond",
    taxonomy: [{ scheme: term.scheme, code: term.code }],
    capabilities: [publishedProblemFixture.catalogProblem.capabilities[0]],
    licenses: [publishedProblemFixture.catalogProblem.metadata.license.kind],
    statistics: "unavailable",
    cursor: "opaque+cursor",
    pageSize: 25,
  });
  assert.equal(result.items.length, 0, "the mock treats a cursor as the consumed first page");
  const requested = new URL(requests[0].url);
  assert.equal(requested.origin, "https://client.example.test");
  assert.equal(requested.pathname, "/api/problems/search");
  assert.deepEqual(requested.searchParams.getAll("taxonomy"), [`${term.scheme}:${term.code}`]);
  assert.deepEqual(requested.searchParams.getAll("capabilities"), [
    publishedProblemFixture.catalogProblem.capabilities[0],
  ]);
  assert.deepEqual(requested.searchParams.getAll("licenses"), [
    publishedProblemFixture.catalogProblem.metadata.license.kind,
  ]);
  assert.equal(requested.searchParams.get("cursor"), "opaque+cursor");
  assert.equal(requested.searchParams.get("pageSize"), "25");
  assert.equal(requested.searchParams.get("offset"), null);
});

test("catalog clients resolve one copyable exact version without exposing UUID input", async () => {
  const requests = [];
  const mockFetch = createMockFetch();
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(String(input), "https://client.example.test"), init);
      requests.push(new URL(request.url).pathname);
      return await mockFetch(
        `${new URL(request.url).pathname}${new URL(request.url).search}`,
        init,
      );
    },
  });
  const summary = await client.resolveCatalogProblem(" P-1-v1 ");
  assert.equal(summary.publicId, 1);
  assert.equal(summary.versionNumber, 1);
  assert.deepEqual(requests, ["/api/problems/by-id/P-1-v1"]);

  const mock = createMockApiClient();
  assert.equal((await mock.resolveCatalogProblem("P-1-v1")).problem, summary.problem);
  await assert.rejects(mock.resolveCatalogProblem("P-999-v1"), ApiRequestError);
  assert.throws(() => client.resolveCatalogProblem(" "), /problem reference must be 1 to 44/);
});

test("mock catalog display references preserve live 400 malformed and 404 unavailable semantics", async () => {
  const mockFetch = createMockFetch();
  const live = createHttpApiClient({ fetch: mockFetch });
  const mock = createMockApiClient({ fetch: mockFetch });

  assert.equal((await live.resolveCatalogProblem("P-1")).versionNumber, 1);
  for (const malformed of [
    "P-0",
    "P-1-v0",
    "P-one-v1",
    "P-1-v",
    "P-2147483648-v1",
    "not-a-reference",
  ]) {
    const response = await mockFetch(`/api/problems/by-id/${encodeURIComponent(malformed)}`);
    assert.equal(response.status, 400, `${malformed} must be rejected as malformed`);
    for (const client of [live, mock]) {
      await assert.rejects(
        client.resolveCatalogProblem(malformed),
        (error) => error instanceof ApiRequestError && error.status === 400,
      );
    }
  }

  for (const client of [live, mock]) {
    await assert.rejects(
      client.resolveCatalogProblem("P-999-v1"),
      (error) => error instanceof ApiRequestError && error.status === 404,
    );
  }
});

test("catalog HTTP client rejects invalid query bounds and mismatched detail identity", async () => {
  const client = createHttpApiClient({ fetch: createMockFetch() });
  assert.throws(
    () => client.searchCatalog({ ...EMPTY_SEARCH, pageSize: 101 }),
    /pageSize must be a safe integer between 1 and 100/,
  );
  assert.throws(
    () => client.searchCatalog({ ...EMPTY_SEARCH, text: " ", pageSize: 1 }),
    /catalog text must contain non-whitespace text/,
  );

  async function wrongIdentityFetch() {
    const detail = await mockJson(
      `/api/problems/${publishedProblemFixture.publishedProblem.problem}/versions/${publishedProblemFixture.publishedProblem.version}/detail`,
    );
    return new Response(
      JSON.stringify({
        ...detail,
        summary: { ...detail.summary, version: "0198e000-0000-7000-8000-000000000099" },
      }),
      { headers: { "content-type": "application/json" } },
    );
  }
  const wrongIdentity = createHttpApiClient({ fetch: wrongIdentityFetch });
  await assert.rejects(
    wrongIdentity.getCatalogProblemDetail(
      publishedProblemFixture.publishedProblem.problem,
      publishedProblemFixture.publishedProblem.version,
    ),
    ApiProtocolError,
  );
});

test("live and mock clients reject invalid catalog searches at the same shared boundary", () => {
  const live = createHttpApiClient({ fetch: createMockFetch() });
  const mock = createMockApiClient();
  const invalidQueries = [
    { ...EMPTY_SEARCH, pageSize: 101 },
    { ...EMPTY_SEARCH, text: "  " },
    { ...EMPTY_SEARCH, cursor: "" },
    { ...EMPTY_SEARCH, capabilities: ["not-a-capability"] },
  ];
  for (const query of invalidQueries) {
    let liveError;
    let mockError;
    assert.throws(
      () => live.searchCatalog(query),
      (error) => {
        liveError = error;
        return true;
      },
    );
    assert.throws(
      () => mock.searchCatalog(query),
      (error) => {
        mockError = error;
        return true;
      },
    );
    assert.equal(mockError?.message, liveError?.message);
  }
});

test("mock catalog client decodes hostile handler JSON instead of replacing it with a fixture", async () => {
  const hostileSearchFetch = async () =>
    new Response(
      JSON.stringify({ items: [], nextCursor: null, facets: {}, answerKey: "must-not-reach-UI" }),
      { headers: { "content-type": "application/json" } },
    );
  const hostileSearch = createMockApiClient({ fetch: hostileSearchFetch });
  await assert.rejects(hostileSearch.searchCatalog(EMPTY_SEARCH), DecodeError);

  const hostileDetailFetch = async () =>
    new Response(
      JSON.stringify({ summary: {}, prompt: [], statistics: "unavailable", source: "private.pg" }),
      { headers: { "content-type": "application/json" } },
    );
  const hostileDetail = createMockApiClient({ fetch: hostileDetailFetch });
  await assert.rejects(
    hostileDetail.getCatalogProblemDetail(
      publishedProblemFixture.publishedProblem.problem,
      publishedProblemFixture.publishedProblem.version,
    ),
    DecodeError,
  );
});

test("mock client and handlers use the real search/detail wire contract", async () => {
  const client = createMockApiClient();
  const search = await client.searchCatalog({ ...EMPTY_SEARCH, pageSize: 1 });
  assert.equal(search.items[0]?.problem, publishedProblemFixture.publishedProblem.problem);
  assert.equal(search.items[0]?.version, publishedProblemFixture.publishedProblem.version);
  assert.equal(search.facets.statistics.available, 0);
  assert.equal(search.facets.statistics.unavailable, 1);
  assert.equal(
    (await client.searchCatalog({ ...EMPTY_SEARCH, text: "does-not-match" })).items.length,
    0,
  );
  assert.equal(
    (await client.searchCatalog({ ...EMPTY_SEARCH, statistics: "available" })).items.length,
    0,
  );

  const detail = await client.getCatalogProblemDetail(
    publishedProblemFixture.publishedProblem.problem,
    publishedProblemFixture.publishedProblem.version,
  );
  assert.equal(detail.summary.problem, publishedProblemFixture.publishedProblem.problem);
  assert.equal(detail.summary.version, publishedProblemFixture.publishedProblem.version);
  const invalid = await createMockFetch()("/api/problems/search?offset=1");
  assert.equal(invalid.status, 400);
});
