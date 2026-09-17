// Stable search identity contracts. Failure means a selected filter was lost or broadened.
import assert from "node:assert/strict";
import test from "node:test";
import {
  libraryClassificationChange,
  libraryClassificationFilter,
} from "../src/api/library_classification_filter.ts";
import { questionSearchRequest } from "../src/api/question_library_repository.ts";
import { questionSearchPath } from "../src/api/question_search_query.ts";
import {
  recoverLibrarySearch,
  searchHandoffQuery,
  searchWithinResultsPath,
} from "../src/pages/library_search_parameters.ts";
import {
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  NO_QUESTION_LIBRARY_FACET_TRUNCATION,
  QuestionLibraryBrowseSession,
  normalizeQuestionLibraryBrowseQuery,
  saveQuestionLibraryReturnState,
  takeQuestionLibraryReturnState,
} from "../src/pages/library_page_model.ts";

const identities = {
  discipline_uuid: "00000000-0000-0000-0000-000000000001",
  subject_uuid: "00000000-0000-0000-0000-000000000002",
  topic_uuid: "00000000-0000-0000-0000-000000000003",
  subtopic_uuid: "00000000-0000-0000-0000-000000000004",
  cross_discipline: true,
};
function query() {
  return {
    ...EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
    ...identities,
    search: 'discipline:biology topic:"cell division" -mitosis',
    tag: "review",
    subjects: ["biochemistry"],
    topics: ["inheritance"],
    sort: "publishedNewest",
  };
}
function row() {
  return {
    displayId: "7K3M-X9QP",
    questionRevision: { questionId: "7K3M-X9QP", revisionNumber: 1 },
    questionTitle: "Cell division",
    summary: "Answer-free summary",
    questionFormat: "pleQuestionJson",
    authorNames: [],
    capabilities: [],
    questionLicense: "CC-BY-4.0",
    evidence: { state: "unavailable" },
  };
}

test("Library URL handoff and strict wire request retain hierarchy, filters, and sort", () => {
  const original = query();
  const restored = searchHandoffQuery(
    new URL(searchWithinResultsPath(original), "https://example.test").search,
  );
  assert.deepEqual(restored, original);
  assert.deepEqual(
    libraryClassificationFilter(normalizeQuestionLibraryBrowseQuery(restored)),
    identities,
  );
  const parameters = new URL(
    questionSearchPath(questionSearchRequest(restored, "next-page")),
    "https://example.test",
  ).searchParams;
  for (const [field, value] of Object.entries(identities))
    assert.equal(parameters.get(field), String(value));
  assert.equal(parameters.get("text"), original.search);
  assert.equal(parameters.get("tags"), "review");
  assert.equal(parameters.get("subjects"), "biochemistry");
  assert.equal(parameters.get("sort"), "publishedNewest");
  assert.equal(parameters.get("cursor"), "next-page");
  const empty = new URL(
    questionSearchPath(questionSearchRequest(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY, null)),
    "https://example.test",
  ).searchParams;
  for (const field of Object.keys(identities)) assert.equal(empty.has(field), false);
  assert.equal(empty.get("sort"), "titleAscending");
});

test("Library cascade clears descendants and cross mode without changing independent filters", () => {
  const original = query();
  const discipline = { ...original, ...libraryClassificationChange("discipline_uuid", null) };
  assert.deepEqual(
    libraryClassificationFilter(discipline),
    libraryClassificationFilter(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY),
  );
  const subject = { ...original, ...libraryClassificationChange("subject_uuid", null) };
  assert.equal(subject.topic_uuid, null);
  assert.equal(subject.subtopic_uuid, null);
  assert.equal(subject.cross_discipline, false);
  const topic = { ...original, ...libraryClassificationChange("topic_uuid", null) };
  assert.equal(topic.subtopic_uuid, null);
  assert.equal(topic.subject_uuid, original.subject_uuid);
  assert.equal(topic.cross_discipline, true);
  assert.equal(discipline.search, original.search);
  assert.equal(discipline.tag, original.tag);
});

test("Library request rejects malformed identities, incomplete chains, false booleans and unknown fields", () => {
  const request = questionSearchRequest(query(), null);
  for (const change of [
    { discipline_uuid: "biology" },
    { discipline_uuid: null },
    { subject_uuid: null },
    { topic_uuid: null },
    { cross_discipline: "false" },
    { hidden: true },
  ]) {
    assert.throws(() => questionSearchPath({ ...request, ...change }));
  }
  assert.throws(() => questionSearchPath({ ...request, sort: "unknown" }));
  assert.throws(() => searchHandoffQuery("?cross_discipline=1"));
  assert.throws(() => searchHandoffQuery("?sort=unknown"));
  assert.throws(() => searchHandoffQuery("?sort=titleAscending&sort=publishedNewest"));
});

test("Library malformed URL recovery removes only the rejected strict options", () => {
  const invalidSort = new URL(searchWithinResultsPath(query()), "https://example.test");
  invalidSort.searchParams.append("sort", "titleAscending");
  const recoveredSort = searchHandoffQuery(recoverLibrarySearch(invalidSort.search));
  assert.equal(recoveredSort.sort, "titleAscending");
  assert.equal(recoveredSort.tag, "review");
  assert.deepEqual(libraryClassificationFilter(recoveredSort), identities);

  const recoveredClassification = searchHandoffQuery(
    recoverLibrarySearch("?subject_uuid=bad&sort=publishedNewest&tag=review"),
  );
  assert.equal(recoveredClassification.sort, "publishedNewest");
  assert.equal(recoveredClassification.tag, "review");
  assert.deepEqual(
    libraryClassificationFilter(recoveredClassification),
    libraryClassificationFilter(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY),
  );
});

test("Library continuation, Retry and detail return preserve identity and sort", async () => {
  const requests = [];
  let failNext = true;
  const session = new QuestionLibraryBrowseSession(
    {
      search: async (value, cursor) => {
        requests.push({ value, cursor });
        if (cursor !== null && failNext) {
          failNext = false;
          throw new Error("temporary failure");
        }
        return {
          items: [row()],
          nextCursor: cursor === null ? "next-page" : null,
          aggregates: [],
          facetTruncation: NO_QUESTION_LIBRARY_FACET_TRUNCATION,
        };
      },
    },
    () => {},
  );
  await session.reset(query());
  await session.loadNext();
  assert.equal(session.state.kind, "error");
  await session.retry();
  assert.equal(session.state.kind, "ready");
  assert.deepEqual(
    requests.map((request) => libraryClassificationFilter(request.value)),
    [identities, identities, identities],
  );
  assert.deepEqual(
    requests.map((request) => request.cursor),
    [null, "next-page", "next-page"],
  );
  assert.deepEqual(
    requests.map((request) => request.value.sort),
    ["publishedNewest", "publishedNewest", "publishedNewest"],
  );
  const scope = {};
  const token = "00000000-0000-0000-0000-000000000099";
  saveQuestionLibraryReturnState(scope, "search", token, query(), session.state, 224);
  const returned = takeQuestionLibraryReturnState(scope, token);
  assert.deepEqual(returned.query, query());
  assert.equal(returned.scrollTop, 224);
  assert.deepEqual(returned.browseState, session.state);
});
