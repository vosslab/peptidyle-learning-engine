// D2 browser curation contracts: strict answer-free decoding and revision recovery.

import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeProblemCollectionMemberPage,
  decodeProblemCollectionSummaryView,
  decodeSavedProblemSearchView,
} from "../src/api/decoders/problem_curation.ts";
import {
  ApiProtocolError,
  ProblemCurationConflictError,
  createHttpApiClient,
} from "../src/api/http_client.ts";

const FILTER = {
  text: "protein structure",
  bylines: [],
  backends: ["native"],
  tags: ["biochemistry"],
  response_families: ["multipleChoice"],
  taxonomy: [{ scheme: "Peptidyle", code: "BIOCHEM.PEPTIDE_BOND" }],
  capabilities: ["serverGrading"],
  licenses: ["ccBy"],
  evidence: "any",
  used_in_my_courses: "any",
  authorship: "any",
};

function collection(revision = "7") {
  return {
    reference: "PC-7",
    kind: "named",
    title: "Exam candidates",
    visibility: "private",
    revision,
    access: "owner",
  };
}

function savedSearch(revision = "3") {
  return { reference: "PS-3", title: "Peptide candidates", filter: FILTER, revision };
}

function noStoreJson(value, etag = undefined, status = 200) {
  const headers = { "cache-control": "no-store", "content-type": "application/json" };
  if (etag !== undefined) headers.etag = etag;
  return new Response(JSON.stringify(value), { status, headers });
}

function noStoreEmpty() {
  return new Response(null, { status: 204, headers: { "cache-control": "no-store" } });
}

test("D2 curation decoders accept only closed safe projections and current D1 filters", async () => {
  assert.deepEqual(decodeProblemCollectionSummaryView(collection()), collection());
  assert.deepEqual(decodeSavedProblemSearchView(savedSearch()), savedSearch());
  assert.throws(
    () => decodeSavedProblemSearchView({ ...savedSearch(), cursor: "private continuation" }),
    DecodeError,
  );
  const missingAuthorship = structuredClone(savedSearch());
  delete missingAuthorship.filter.authorship;
  assert.throws(() => decodeSavedProblemSearchView(missingAuthorship), DecodeError);
  const unknownAuthorship = structuredClone(savedSearch());
  unknownAuthorship.filter.authorship = "otherActor";
  assert.throws(() => decodeSavedProblemSearchView(unknownAuthorship), DecodeError);
  const retiredPublicationScope = structuredClone(savedSearch());
  retiredPublicationScope.filter.publication_scopes = ["private"];
  assert.throws(() => decodeSavedProblemSearchView(retiredPublicationScope), DecodeError);
  const camelCaseFilter = structuredClone(savedSearch());
  camelCaseFilter.filter.responseFamilies = ["multipleChoice"];
  assert.throws(() => decodeSavedProblemSearchView(camelCaseFilter), DecodeError);
  assert.throws(
    () => decodeProblemCollectionSummaryView({ ...collection(), owner: "private user" }),
    DecodeError,
  );
  const members = {
    items: [
      {
        questionId: publishedProblemFixture.catalogProblem.questionId,
        summary: publishedProblemFixture.catalogProblem,
        selectionAvailability: "available",
      },
    ],
    nextCursor: null,
  };
  assert.deepEqual(decodeProblemCollectionMemberPage(members), members);
  const mismatched = structuredClone(members);
  mismatched.items[0].questionId = "1A2-B3CD";
  assert.throws(() => decodeProblemCollectionMemberPage(mismatched), DecodeError);
  await assert.rejects(
    createHttpApiClient({
      fetch: () => Promise.resolve(noStoreJson(collection(), '"7"')),
    }).replaceProblemCollection("PC-7", { questionIds: ["7K3-M9QP", "7K3-M9QP"] }, '"7"'),
    DecodeError,
  );
});

test("D2 HTTP client sends exact ETags, keeps saved searches fresh, and preserves 412 recovery", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      if (request.method === "GET" && request.url.endsWith("/PC-7")) {
        return noStoreJson(collection(), '"7"');
      }
      if (request.method === "POST" && request.url.endsWith("/favorites")) {
        return noStoreJson(
          {
            ...collection("1"),
            reference: "PC-1",
            kind: "favorites",
            title: "Favorites",
          },
          '"1"',
        );
      }
      if (request.method === "PUT" && request.url.endsWith("/PC-7")) {
        return noStoreJson(collection("8"), '"8"');
      }
      if (request.method === "POST" && request.url.endsWith("/saved-problem-searches")) {
        return noStoreJson(savedSearch("4"), '"4"', 201);
      }
      if (request.method === "PUT" && request.url.endsWith("/PS-3")) {
        return noStoreJson(savedSearch("4"), '"4"');
      }
      if (request.method === "GET" && request.url.endsWith("/PS-3")) {
        return noStoreJson(savedSearch(), '"3"');
      }
      if (request.method === "DELETE") return noStoreEmpty();
      return noStoreJson({ items: [], nextCursor: null });
    },
  });

  const current = await client.getProblemCollection("PC-7");
  assert.equal(current.etag, '"7"');
  const favorites = await client.ensureFavorites();
  assert.equal(favorites.collection.kind, "favorites");
  const saved = await client.replaceProblemCollection(
    "PC-7",
    { questionIds: [publishedProblemFixture.catalogProblem.questionId] },
    current.etag,
  );
  assert.equal(saved.etag, '"8"');
  await client.createSavedProblemSearch({ title: "Peptide candidates", filter: FILTER });
  const currentSearch = await client.getSavedProblemSearch("PS-3");
  assert.equal(currentSearch.etag, '"3"');
  await client.replaceSavedProblemSearch(
    "PS-3",
    { title: "Peptide candidates", filter: FILTER },
    currentSearch.etag,
  );
  await client.deleteProblemCollection("PC-7", current.etag);
  await client.deleteSavedProblemSearch("PS-3", currentSearch.etag);
  const update = requests.find((request) => request.method === "PUT");
  assert.equal(update.headers.get("if-match"), '"7"');
  const searchCreate = requests.find(
    (request) => request.method === "POST" && request.url.endsWith("/saved-problem-searches"),
  );
  const body = JSON.parse(await searchCreate.text());
  assert.equal(Object.hasOwn(body.filter, "cursor"), false);
  assert.equal(Object.hasOwn(body.filter, "page_size"), false);
  assert.equal(Object.hasOwn(body.filter, "publication_scopes"), false);
  assert.equal(Object.hasOwn(body.filter, "publicationScopes"), false);
  const favoriteEnsure = requests.find(
    (request) => request.method === "POST" && request.url.endsWith("/favorites"),
  );
  assert.equal(favoriteEnsure.headers.get("content-type"), null);
  assert.deepEqual(
    requests
      .filter((request) => request.method === "PUT" || request.method === "DELETE")
      .map((request) => [
        request.method,
        new URL(request.url).pathname,
        request.headers.get("if-match"),
      ]),
    [
      ["PUT", "/api/problem-collections/PC-7", '"7"'],
      ["PUT", "/api/saved-problem-searches/PS-3", '"3"'],
      ["DELETE", "/api/problem-collections/PC-7", '"7"'],
      ["DELETE", "/api/saved-problem-searches/PS-3", '"3"'],
    ],
  );

  const conflict = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response(null, { status: 412, headers: { "cache-control": "no-store" } }),
      ),
  });
  await assert.rejects(
    conflict.replaceProblemCollection("PC-7", { questionIds: [] }, '"7"'),
    ProblemCurationConflictError,
  );
  await assert.rejects(
    client.replaceProblemCollection("PC-7", { questionIds: [] }, '"07"'),
    ApiProtocolError,
  );
});
