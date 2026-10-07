import assert from "node:assert/strict";
import test from "node:test";

import { createDraftQuestionPreviewClient } from "../src/features/question_draft_preview/draft_preview_client.ts";

const draftQuestion = "0190a0b0-c0d0-7e10-8a20-304050607080";

function sourceResponse({ backend, format, path, source }) {
  return new Response(source, {
    status: 200,
    headers: {
      "content-type":
        backend === "ple" ? "application/vnd.peptidyle.question+json" : "text/x-wework-pg",
      etag: '"8"',
      "x-ple-question-backend": backend,
      "x-ple-question-format": format,
      "x-ple-webwork-pg-path": path,
    },
  });
}

test("Draft source binds Native, PG, and PGML to their explicit stored formats", async () => {
  const cases = [
    {
      backend: "ple",
      format: "pleQuestionJson",
      path: "",
      source: "{}",
      expected: { backend: "ple", format: "pleQuestionJson", webworkPgPath: null },
    },
    {
      backend: "webwork",
      format: "webworkPg",
      path: "Library/Genetics/question.pg",
      source: "DOCUMENT();",
      expected: {
        backend: "webwork",
        format: "webworkPg",
        webworkPgPath: "Library/Genetics/question.pg",
      },
    },
    {
      backend: "webwork",
      format: "webworkPgml",
      path: "Library/Genetics/question.pgml",
      source: "DOCUMENT();",
      expected: {
        backend: "webwork",
        format: "webworkPgml",
        webworkPgPath: "Library/Genetics/question.pgml",
      },
    },
  ];

  for (const item of cases) {
    const client = createDraftQuestionPreviewClient({
      fetch: async () => sourceResponse(item),
    });
    const loaded = await client.loadSource(draftQuestion);
    assert.equal(loaded.source, item.source);
    assert.equal(loaded.draftQuestionEditNumber, "8");
    assert.equal(loaded.binding.backend, item.expected.backend);
    assert.equal(loaded.binding.format, item.expected.format);
    assert.equal(loaded.binding.webworkPgPath, item.expected.webworkPgPath);
  }
});

test("Draft WebWork binding rejects traversal paths before preview or test", async () => {
  const client = createDraftQuestionPreviewClient({
    fetch: async () =>
      sourceResponse({
        backend: "webwork",
        format: "webworkPg",
        path: "Library/%2e%2e/private.pg",
        source: "DOCUMENT();",
      }),
  });
  await assert.rejects(() => client.loadSource(draftQuestion), /invalid WebWork path/u);
});

test("Draft test sends only a Draft Edit Number and transient response payload", async () => {
  const requests = [];
  const client = createDraftQuestionPreviewClient({
    fetch: async (input, init) => {
      requests.push(new Request(new URL(String(input), "https://ple.example"), init));
      return new Response(
        JSON.stringify({ kind: "evaluated", correct: true, normalizedCredit: 1 }),
        {
          status: 200,
          headers: { "content-type": "application/json", "cache-control": "no-store" },
        },
      );
    },
  });
  const response = client.webworkResponse([["AnSwEr0001", "correct"]]);
  const result = await client.test(draftQuestion, "8", response, 27);
  const request = requests[0];
  const requestBody = await request.json();

  assert.deepEqual(result, { kind: "evaluated", correct: true, normalizedCredit: 1 });
  assert.equal(request.method, "POST");
  assert.equal(new URL(request.url).pathname, `/api/authoring/drafts/${draftQuestion}/test`);
  assert.equal(request.headers.get("if-match"), '"8"');
  assert.deepEqual(Object.keys(requestBody).sort(), ["response", "seed"]);
  assert.equal(requestBody.seed, 27);
  assert.deepEqual(requestBody.response, response);
  assert.equal(JSON.stringify(requestBody).includes("publishedQuestionRevisionTuple"), false);
  assert.equal(JSON.stringify(requestBody).includes("studentWork"), false);
});

test("WebWork preview binds a seed and Draft Edit Number without a Published tuple", () => {
  const client = createDraftQuestionPreviewClient();
  const preview = new URL(client.webworkPreviewPath(draftQuestion, "8", 27), "https://ple.example");

  assert.equal(preview.pathname, `/api/authoring/drafts/${draftQuestion}/preview`);
  assert.equal(preview.searchParams.get("draftQuestionEditNumber"), "8");
  assert.equal(preview.searchParams.get("seed"), "27");
  assert.equal(preview.searchParams.get("draftTest"), "true");
  assert.equal(preview.searchParams.has("publishedQuestionId"), false);
  assert.equal(preview.searchParams.has("revisionNumber"), false);
});

test("correction publication uses the existing revision route and exact Draft validator", async () => {
  const requests = [];
  const client = createDraftQuestionPreviewClient({
    fetch: async (input, init) => {
      requests.push(new Request(new URL(String(input), "https://ple.example"), init));
      return new Response(
        JSON.stringify({
          publishedQuestionRevisionTuple: {
            publishedQuestionId: "7K3M-79QP",
            revisionNumber: 5,
          },
        }),
        {
          status: 200,
          headers: { "content-type": "application/json", "cache-control": "no-store" },
        },
      );
    },
  });
  const parent = { publishedQuestionId: "7K3M-79QP", revisionNumber: 4 };
  const result = await client.publishRevision(
    draftQuestion,
    parent,
    "Correct the charge explanation",
    "8",
  );
  const request = requests[0];
  const body = await request.json();

  assert.equal(result.publishedQuestionId, parent.publishedQuestionId);
  assert.equal(result.revisionNumber, 5);
  assert.equal(
    new URL(request.url).pathname,
    `/api/authoring/drafts/${draftQuestion}/publish-revision`,
  );
  assert.equal(request.headers.get("if-match"), '"8"');
  assert.deepEqual(Object.keys(body).sort(), [
    "parentRevisionNumber",
    "questionId",
    "reasonForEdit",
  ]);
  assert.equal(body.questionId, parent.publishedQuestionId);
  assert.equal(body.parentRevisionNumber, parent.revisionNumber);
});
