import assert from "node:assert/strict";
import test from "node:test";
import { createQuestionPoolLibraryClient } from "../src/api/http_client/question_pool_library.ts";
import { EMPTY_LIBRARY_CLASSIFICATION_FILTER } from "../src/api/library_classification_filter.ts";

const filter = {
  discipline_uuid: "00000000-0000-0000-0000-000000000001",
  subject_uuid: "00000000-0000-0000-0000-000000000002",
  topic_uuid: "00000000-0000-0000-0000-000000000003",
  subtopic_uuid: "00000000-0000-0000-0000-000000000004",
  cross_discipline: true,
  bloom_cognitive_process: "Analyze",
  bloom_knowledge_dimension: "Conceptual Knowledge",
};

const emptyBloomFacets = {
  cognitiveProcesses: ["Remember", "Understand", "Apply", "Analyze", "Evaluate", "Create"].map(
    (cognitiveProcess) => ({ cognitiveProcess, count: 0 }),
  ),
  knowledgeDimensions: [
    "Factual Knowledge",
    "Conceptual Knowledge",
    "Procedural Knowledge",
    "Metacognitive Knowledge",
  ].map((knowledgeDimension) => ({ knowledgeDimension, count: 0 })),
};

function emptyPage() {
  return { items: [], nextCursor: null, bloomFacets: emptyBloomFacets };
}

test("Pool discovery encodes the selected identity tuple on first and continuation reads", async () => {
  const requests = [];
  const client = createQuestionPoolLibraryClient({
    fetch: async (input, init) => {
      requests.push({ url: new URL(String(input), "https://example.test"), init });
      return new Response(JSON.stringify(emptyPage()), {
        headers: { "content-type": "application/json", "cache-control": "no-store" },
      });
    },
  });
  await client.listQuestionPools(undefined, 20, filter);
  await client.listQuestionPools("continuation&literal", 20, filter);
  for (const { url, init } of requests) {
    assert.equal(url.pathname, "/api/question-pools");
    for (const [field, value] of Object.entries(filter)) {
      assert.equal(url.searchParams.get(field), String(value));
    }
    assert.equal(url.searchParams.get("page_size"), "20");
    assert.equal(init.credentials, "same-origin");
    assert.equal(init.cache, "no-store");
  }
  assert.equal(requests[0].url.searchParams.has("cursor"), false);
  assert.equal(requests[1].url.searchParams.get("cursor"), "continuation&literal");
  await client.listQuestionPools();
  for (const field of Object.keys(filter))
    assert.equal(requests[2].url.searchParams.has(field), false);
});

test("Pool discovery rejects invalid classification chains before dispatch", async () => {
  let dispatched = false;
  const client = createQuestionPoolLibraryClient({
    fetch: async () => {
      dispatched = true;
      throw new Error("invalid input must not dispatch");
    },
  });
  for (const value of [
    { ...filter, discipline_uuid: "biology" },
    { ...filter, subject_uuid: null },
    { ...EMPTY_LIBRARY_CLASSIFICATION_FILTER, cross_discipline: true },
  ]) {
    await assert.rejects(client.listQuestionPools(undefined, 20, value));
  }
  assert.equal(dispatched, false);
});

test("Pool text and Tags keep their normalized query on continuation", async () => {
  const urls = [];
  const client = createQuestionPoolLibraryClient({
    fetch: async (input) => {
      urls.push(new URL(String(input), "https://example.test"));
      return new Response(JSON.stringify(emptyPage()), {
        headers: { "content-type": "application/json", "cache-control": "no-store" },
      });
    },
  });
  const query = {
    ...filter,
    text: '  TOPIC:"Cell   Division" -review ',
    tags: [" Review ", "CELL  division", "review"],
  };
  await client.listQuestionPools(undefined, 20, query);
  await client.listQuestionPools("continue", 20, query);
  for (const url of urls) {
    assert.equal(url.searchParams.get("text"), 'topic:"cell division" -review');
    assert.deepEqual(url.searchParams.getAll("tags"), ["cell division", "review"]);
    assert.equal(url.searchParams.get("subject_uuid"), filter.subject_uuid);
  }
});

test("Pool search bounds reject invalid text and Tags before dispatch", async () => {
  let dispatched = false;
  const client = createQuestionPoolLibraryClient({
    fetch: async () => {
      dispatched = true;
      throw new Error("unexpected");
    },
  });
  for (const extra of [
    { text: "x".repeat(257) },
    { tags: [" "] },
    { tags: ["x".repeat(257)] },
    { tags: Array(65).fill("review") },
    { bloom_cognitive_process: "analyze" },
    { bloom_cognitive_process: "" },
    { bloom_knowledge_dimension: "Strategic Knowledge" },
    { bloom_knowledge_dimension: "" },
  ]) {
    await assert.rejects(client.listQuestionPools(undefined, 20, { ...filter, ...extra }));
  }
  assert.equal(dispatched, false);
});

test("Pool exact detail uses the current Pool ID route", async () => {
  const requests = [];
  const client = createQuestionPoolLibraryClient({
    fetch: async (input) => {
      requests.push(new URL(String(input), "https://example.test"));
      return new Response(JSON.stringify({ error: "Question Pool unavailable" }), {
        status: 404,
        headers: { "content-type": "application/json", "cache-control": "no-store" },
      });
    },
  });
  await assert.rejects(client.getQuestionPool("3S8B-24DZ"));
  assert.equal(requests[0]?.pathname, "/api/question-pools/3S8B-24DZ");
});
