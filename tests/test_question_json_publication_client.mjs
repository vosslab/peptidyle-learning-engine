import assert from "node:assert/strict";
import test from "node:test";

import {
  createPleQuestionJsonClient,
  PleQuestionJsonConflictError,
} from "../src/features/ple_question_json_authoring/question_json_client.ts";
import { createPleQuestionJsonRepository } from "../src/features/ple_question_json_authoring/question_json_repository.ts";
import { recordMetadata } from "./ple_question_json_authoring_support.mjs";

const draftQuestion = "0198e000-0000-7000-8000-000000000001";
const questionId = "7K3M-79QP";
const parent = { publishedQuestionId: questionId, revisionNumber: 4 };
const nextRevision = { publishedQuestionId: questionId, revisionNumber: 5 };
const classification = {
  disciplineUuid: "00000000-0000-4000-8000-000000000001",
  subjectUuid: "00000000-0000-4000-8000-000000000002",
  topicUuid: null,
  subtopicUuid: null,
};
const authorship = { authors: [{ displayName: "Ada Lovelace" }] };
const backendFormats = [
  { backend: "ple", questionFormat: "pleQuestionJson" },
  { backend: "webwork", questionFormat: "webworkPg" },
  { backend: "webwork", questionFormat: "webworkPgml" },
];

function summary({ backend, questionFormat }, revisionNumber) {
  return {
    questionId,
    publishedQuestionRevisionTuple: { publishedQuestionId: questionId, revisionNumber },
    parentPublishedQuestionRevisionTuple: null,
    backend,
    questionFormat,
    questionType: "multipleChoice",
    capabilities: ["serverGrading"],
    metadata: recordMetadata(),
    authorship: { authors: [{ accountId: null, displayName: "Ada Lovelace" }] },
    availability: { availability: "available" },
    publishedAt: 1786000000000,
    bloom: { cognitiveProcess: "Understand", knowledgeDimension: "Conceptual Knowledge" },
  };
}

function jsonResponse(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function lineage(summaryValue) {
  return {
    summary: summaryValue,
    viewerMayArchive: true,
    viewerMayEditMetadata: true,
  };
}

function makeClient(operation, publishedSummary, publicationStatus = 201) {
  const requests = [];
  const client = createPleQuestionJsonClient({
    fetch: async (input, init) => {
      const request = { input: String(input), init };
      requests.push(request);
      if (init.method === "POST") {
        if (operation === "publish") return jsonResponse({ questionId }, publicationStatus);
        return jsonResponse({ publishedQuestionRevisionTuple: nextRevision }, publicationStatus);
      }
      return jsonResponse(lineage(publishedSummary));
    },
  });
  return { client, requests };
}

test("/publish and /publish-revision return Native and WebWork summaries with unchanged request contracts", async () => {
  for (const backendFormat of backendFormats) {
    const createdSummary = summary(backendFormat, 1);
    const { client, requests } = makeClient("publish", createdSummary);
    const result = await client.publish(draftQuestion, { authorship, ...classification }, "7");

    assert.deepEqual(result, createdSummary);
    assert.equal(requests.length, 2);
    assert.equal(requests[0].input, `/api/authoring/drafts/${draftQuestion}/publish`);
    assert.equal(requests[0].init.method, "POST");
    assert.equal(requests[0].init.headers["if-match"], '"7"');
    assert.deepEqual(JSON.parse(requests[0].init.body), {
      authors: ["Ada Lovelace"],
      ...classification,
    });
    assert.equal(requests[1].input, `/api/questions/by-id/${questionId}`);

    const correctedSummary = summary(backendFormat, 5);
    const revisionClient = makeClient("publish-revision", correctedSummary, 200);
    const revisionResult = await revisionClient.client.publishRevision(
      draftQuestion,
      parent,
      "Correct the response explanation",
      "8",
    );

    assert.deepEqual(revisionResult, correctedSummary);
    assert.equal(revisionClient.requests.length, 2);
    assert.equal(
      revisionClient.requests[0].input,
      `/api/authoring/drafts/${draftQuestion}/publish-revision`,
    );
    assert.equal(revisionClient.requests[0].init.method, "POST");
    assert.equal(revisionClient.requests[0].init.headers["if-match"], '"8"');
    assert.deepEqual(JSON.parse(revisionClient.requests[0].init.body), {
      questionId,
      parentRevisionNumber: 4,
      reasonForEdit: "Correct the response explanation",
    });
    assert.equal(revisionClient.requests[1].input, `/api/questions/by-id/${questionId}`);
  }
});

test("malformed summaries are rejected after either publication operation", async () => {
  const malformedSummaries = [
    summary({ backend: "webwork", questionFormat: "pleQuestionJson" }, 1),
    summary({ backend: "imathas", questionFormat: "imathas" }, 1),
    { ...summary({ backend: "ple", questionFormat: "pleQuestionJson" }, 1), scope: "public" },
  ];
  for (const malformed of malformedSummaries) {
    for (const operation of ["publish", "publish-revision"]) {
      const { client } = makeClient(operation, malformed);
      if (operation === "publish") {
        await assert.rejects(client.publish(draftQuestion, { authorship, ...classification }, "1"));
      } else {
        await assert.rejects(client.publishRevision(draftQuestion, parent, "Correct a typo", "1"));
      }
    }
  }

  const archivedSummary = {
    ...summary({ backend: "ple", questionFormat: "pleQuestionJson" }, 1),
    availability: { availability: "archived" },
  };
  const { client } = makeClient("publish", archivedSummary);
  await assert.rejects(
    client.publish(draftQuestion, { authorship, ...classification }, "1"),
    /available Question Library summary/u,
  );
});

test("both publication operations reject a stale Draft Question Edit Number", async () => {
  for (const operation of ["publish", "publish-revision"]) {
    let calls = 0;
    const client = createPleQuestionJsonClient({
      fetch: async () => {
        calls += 1;
        return jsonResponse({ error: "stale" }, 412);
      },
    });
    const publish =
      operation === "publish"
        ? client.publish(draftQuestion, { authorship, ...classification }, "1")
        : client.publishRevision(draftQuestion, parent, "Correct a typo", "1");
    await assert.rejects(publish, (error) => {
      assert.ok(error instanceof PleQuestionJsonConflictError);
      assert.equal(error.status, 412);
      return true;
    });
    assert.equal(calls, 1);
  }
});

test("the repository sends its loaded Edit Number through the existing revision operation", async () => {
  const calls = [];
  const client = {
    async load() {
      return { source: {}, draftQuestionEditNumber: "9" };
    },
    async save() {
      return { draftQuestionEditNumber: "10" };
    },
    async publish() {
      return summary(backendFormats[0], 1);
    },
    async publishRevision(...args) {
      calls.push(args);
      return summary(backendFormats[2], 5);
    },
  };
  const repository = createPleQuestionJsonRepository(client);
  await repository.load(draftQuestion);
  const result = await repository.publishRevision(draftQuestion, parent, "Correct a typo");

  assert.equal(result.backend, "webwork");
  assert.deepEqual(calls, [[draftQuestion, parent, "Correct a typo", "9"]]);
});
