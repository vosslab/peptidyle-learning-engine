// Literal catalog wire and request serialization tests.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import { DecodeError } from "../src/api/decoder.ts";
import { decodeCatalogProblemDetail, decodeCatalogSearchPage } from "../src/api/decoders.ts";
import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";
import { jsonResponse } from "./http_client_test_support.mjs";

const EMPTY_SEARCH = {
  text: null,
  taxonomy: [],
  capabilities: [],
  licenses: [],
  statistics: "any",
  cursor: null,
  pageSize: null,
};

function searchPage() {
  return {
    items: [publishedProblemFixture.catalogProblem],
    nextCursor: null,
    facets: {
      taxonomy: [],
      capabilities: [],
      licenses: [],
      statistics: { available: 0, unavailable: 1 },
    },
  };
}

function detailPage() {
  return {
    summary: publishedProblemFixture.catalogProblem,
    prompt: publishedProblemFixture.publishedProblem.prompt,
    statistics: "unavailable",
  };
}

test("catalog decoders reject hostile fields in literal safe projections", () => {
  const page = searchPage();
  assert.deepEqual(decodeCatalogSearchPage(page), page);
  assert.throws(() => decodeCatalogSearchPage({ ...page, answerKey: "secret" }), DecodeError);
  const detail = detailPage();
  assert.deepEqual(decodeCatalogProblemDetail(detail), detail);
  assert.throws(() => decodeCatalogProblemDetail({ ...detail, source: "private.pg" }), DecodeError);
});

test("catalog search serializes repeated filters on a same-origin URL", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(String(input), "https://client.example.test"), init);
      requests.push(request);
      return jsonResponse(searchPage());
    },
  });
  const term = publishedProblemFixture.catalogProblem.metadata.taxonomy[0];
  assert.ok(term);
  await client.searchCatalog({
    ...EMPTY_SEARCH,
    text: "peptide bond",
    taxonomy: [{ scheme: term.scheme, code: term.code }],
    capabilities: ["serverGrading"],
    licenses: ["ccBy"],
    cursor: "opaque+cursor",
    pageSize: 25,
  });
  const requested = new URL(requests[0].url);
  assert.equal(requested.origin, "https://client.example.test");
  assert.equal(requested.searchParams.get("cursor"), "opaque+cursor");
  assert.equal(requested.searchParams.get("pageSize"), "25");
});

test("catalog client rejects invalid query bounds and mismatched detail identity", async () => {
  const client = createHttpApiClient({ fetch: async () => jsonResponse(searchPage()) });
  assert.throws(() => client.searchCatalog({ ...EMPTY_SEARCH, pageSize: 101 }));
  const wrongIdentity = createHttpApiClient({
    fetch: async () =>
      jsonResponse({
        ...detailPage(),
        summary: { ...publishedProblemFixture.catalogProblem, questionId: "7K4-M9QP" },
      }),
  });
  await assert.rejects(
    wrongIdentity.getCatalogProblemDetail(publishedProblemFixture.catalogProblem.questionId),
    ApiProtocolError,
  );
});
