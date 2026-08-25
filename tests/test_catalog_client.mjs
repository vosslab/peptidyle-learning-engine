// Literal catalog wire and request serialization tests.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import { catalogSearchPath } from "../src/api/catalog_query.ts";
import { DecodeError } from "../src/api/decoder.ts";
import { decodeCatalogProblemDetail, decodeCatalogSearchPage } from "../src/api/decoders.ts";
import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";
import { jsonResponse } from "./http_client_test_support.mjs";

const EMPTY_SEARCH = {
  text: null,
  bylines: [],
  backends: [],
  tags: [],
  responseFamilies: [],
  taxonomy: [],
  capabilities: [],
  licenses: [],
  publicationScopes: [],
  evidence: "any",
  usedInMyCourses: "any",
  authorship: "any",
  cursor: null,
  pageSize: null,
};

function catalogProblemSummary() {
  return { ...publishedProblemFixture.catalogProblem, responseFamily: "multipleChoice" };
}

function searchPage() {
  return {
    items: [
      {
        summary: catalogProblemSummary(),
        evidence: availableEvidence(),
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
  };
}

function availableEvidence() {
  return {
    state: "available",
    formulaVersion: 1,
    observedCourseCount: 2,
    independentLearnerObservationCount: 5,
    difficultyIndex: 0.7,
    attemptsMean: 1.2,
    timeMedianSecondsEstimate: 30,
    evidenceAt: 0,
  };
}

function usageDetail() {
  return {
    summary: {
      institutionCourseCount: 2,
      institutionAssignmentCount: 3,
      ownCourseCount: 1,
      ownAssignmentCount: 2,
    },
    ownCourses: [
      {
        course: "C-1",
        title: "Molecular Biology",
        assignmentCount: 2,
      },
    ],
    ownCoursesTruncated: false,
  };
}

function detailPage() {
  return {
    summary: catalogProblemSummary(),
    prompt: {
      kind: "static",
      blocks: publishedProblemFixture.publishedProblem.prompt,
    },
    evidence: availableEvidence(),
    usage: usageDetail(),
  };
}

function generatedDetailPage() {
  return {
    ...detailPage(),
    prompt: {
      kind: "generatedExample",
      blocks: [
        {
          kind: "text",
          markdown:
            "In the glycine peptide example, which bond has restricted rotation because resonance gives it partial double-bond character?",
        },
      ],
    },
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

test("catalog detail strictly decodes static and generated prompt projections", () => {
  const generated = generatedDetailPage();
  assert.deepEqual(decodeCatalogProblemDetail(generated), generated);
  assert.match(JSON.stringify(generated.prompt.blocks), /glycine/u);
  assert.doesNotMatch(JSON.stringify(generated.prompt.blocks), /\{\{residue\}\}/u);
  assert.throws(
    () =>
      decodeCatalogProblemDetail({
        ...generated,
        prompt: { ...generated.prompt, answerKey: "secret" },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogProblemDetail({
        ...generated,
        prompt: { ...generated.prompt, kind: "random" },
      }),
    DecodeError,
  );
});

test("catalog discovery decoders reject malformed evidence and non-public usage fields", () => {
  const page = searchPage();
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...page,
        items: [{ ...page.items[0], evidence: { state: "available", answerKey: "secret" } }],
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...page,
        items: [
          {
            ...page.items[0],
            evidence: { ...availableEvidence(), formulaVersion: 65_536 },
          },
        ],
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...page,
        items: [
          {
            ...page.items[0],
            evidence: { ...availableEvidence(), observedCourseCount: 1 },
          },
        ],
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...page,
        items: [
          {
            ...page.items[0],
            evidence: { ...availableEvidence(), independentLearnerObservationCount: 4 },
          },
        ],
      }),
    DecodeError,
  );
  const detail = detailPage();
  assert.throws(
    () =>
      decodeCatalogProblemDetail({
        ...detail,
        usage: {
          ...detail.usage,
          ownCourses: [{ ...detail.usage.ownCourses[0], accountId: "hidden" }],
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogProblemDetail({
        ...detail,
        usage: {
          ...detail.usage,
          ownCourses: [{ ...detail.usage.ownCourses[0], course: "C-2147483648" }],
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogProblemDetail({
        ...detail,
        usage: {
          ...detail.usage,
          ownCourses: [],
          ownCoursesTruncated: false,
        },
      }),
    DecodeError,
  );
});

test("catalog decoders strictly bind new public facets without course identities", () => {
  const page = searchPage();
  page.facets = {
    ...page.facets,
    bylines: [{ byline: "Dr. Ada Lovelace", count: 2 }],
    backends: [{ backend: "native", count: 2 }],
    tags: [{ tag: "Protein Structure", count: 2 }],
    responseFamilies: [{ responseFamily: "multipleChoice", count: 2 }],
    usedInMyCourses: { used: 1 },
  };
  assert.deepEqual(decodeCatalogSearchPage(page), page);
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...page,
        facets: { ...page.facets, bylines: [{ byline: "Ada", count: 1, accountId: "hidden" }] },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...page,
        items: [
          {
            ...page.items[0],
            summary: { ...page.items[0].summary, responseFamily: "essay" },
          },
        ],
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...page,
        facets: { ...page.facets, responseFamilies: [{ responseFamily: "essay", count: 1 }] },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCatalogSearchPage({
        ...page,
        facets: { ...page.facets, usedInMyCourses: { used: 1, course: "C-1" } },
      }),
    DecodeError,
  );
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
    bylines: ["  DR.  ADA Lovelace "],
    backends: ["native"],
    tags: [" Protein   Structure "],
    responseFamilies: ["multipleChoice"],
    taxonomy: [{ scheme: term.scheme, code: term.code }],
    capabilities: ["serverGrading"],
    licenses: ["ccBy"],
    publicationScopes: ["public"],
    evidence: "available",
    usedInMyCourses: "used",
    authorship: "authoredByCurrentActor",
    cursor: "opaque+cursor",
    pageSize: 25,
  });
  const requested = new URL(requests[0].url);
  assert.equal(requested.origin, "https://client.example.test");
  assert.equal(requested.searchParams.get("cursor"), "opaque+cursor");
  assert.equal(requested.searchParams.get("pageSize"), "25");
  assert.equal(requested.searchParams.get("evidence"), "available");
  assert.deepEqual(requested.searchParams.getAll("bylines"), ["dr. ada lovelace"]);
  assert.deepEqual(requested.searchParams.getAll("backends"), ["native"]);
  assert.deepEqual(requested.searchParams.getAll("tags"), ["protein structure"]);
  assert.deepEqual(requested.searchParams.getAll("responseFamilies"), ["multipleChoice"]);
  assert.equal(requested.searchParams.get("usedInMyCourses"), "used");
  assert.equal(requested.searchParams.get("authorship"), "authoredByCurrentActor");
  assert.deepEqual(requested.searchParams.getAll("publicationScopes"), ["public"]);
  assert.equal(requested.searchParams.get("statistics"), null);
  assert.equal(
    catalogSearchPath({
      ...EMPTY_SEARCH,
      bylines: ["DR. ADA LOVELACE"],
      tags: ["Protein Structure"],
    }),
    catalogSearchPath({
      ...EMPTY_SEARCH,
      bylines: ["  dr.  ada lovelace "],
      tags: [" protein   structure "],
    }),
  );
});

test("catalog client rejects invalid query bounds and mismatched detail identity", async () => {
  const client = createHttpApiClient({ fetch: async () => jsonResponse(searchPage()) });
  assert.equal(
    new URL(catalogSearchPath(EMPTY_SEARCH), "https://client.example.test").searchParams.get(
      "authorship",
    ),
    "any",
  );
  assert.throws(() => client.searchCatalog({ ...EMPTY_SEARCH, pageSize: 101 }));
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, statistics: "any" }), /unknown field/);
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, course: "C-1" }), /unknown field/);
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, bylines: Array(17).fill("Ada") }));
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, bylines: ["x".repeat(121)] }));
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, backends: Array(6).fill("native") }));
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, tags: Array(65).fill("protein") }));
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, tags: ["x".repeat(257)] }));
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, backends: ["legacy"] }));
  assert.throws(() =>
    catalogSearchPath({ ...EMPTY_SEARCH, responseFamilies: Array(10).fill("numeric") }),
  );
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, responseFamilies: ["essay"] }));
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, usedInMyCourses: "unused" }));
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, authorship: "anotherActor" }));
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, publicationScopes: ["private"] }));
  assert.throws(() => catalogSearchPath({ ...EMPTY_SEARCH, actorId: "hidden" }), /unknown field/);
  const wrongIdentity = createHttpApiClient({
    fetch: async () =>
      jsonResponse({
        ...detailPage(),
        summary: { ...catalogProblemSummary(), questionId: "7K4-M9QP" },
      }),
  });
  await assert.rejects(
    wrongIdentity.getCatalogProblemDetail(publishedProblemFixture.catalogProblem.questionId),
    ApiProtocolError,
  );
});
