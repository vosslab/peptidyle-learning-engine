import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

const question = publishedQuestionFixture.publishedQuestion;

function noStoreJson(value, etag) {
  return new Response(JSON.stringify(value), {
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json",
      ...(etag === undefined ? {} : { etag }),
    },
  });
}

function details(revisionNumber) {
  return {
    summary: {
      ...question,
      latestQuestionRevision: { questionId: question.questionId, revisionNumber },
    },
    prompt: { kind: "static", blocks: [] },
    evidence: { state: "unavailable" },
    usage: {
      summary: {
        globalCourseCount: 0,
        globalAssignmentCount: 0,
        ownCourseCount: 0,
        ownAssignmentCount: 0,
      },
      ownCourses: [],
      ownCoursesTruncated: false,
    },
  };
}

test("Question availability client keeps current lineage transitions and exact revisions distinct", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      const path = new URL(request.url).pathname;
      if (path.endsWith("/revisions/2")) return noStoreJson(details(2), '"5"');
      if (path.endsWith("/archive")) {
        return noStoreJson({ availability: { availability: "archived" }, editNumber: "6" }, '"6"');
      }
      if (path.endsWith("/restore")) {
        return noStoreJson({ availability: { availability: "available" }, editNumber: "7" }, '"7"');
      }
      return noStoreJson(question, '"5"');
    },
  });

  const lineage = await client.getQuestionLineage(question.questionId);
  const exact = await client.getQuestionRevision({
    questionId: question.questionId,
    revisionNumber: 2,
  });
  const archived = await client.archiveQuestion(
    question.questionId,
    question.metadata.questionTitle,
    '"5"',
  );
  const restored = await client.restoreQuestion(question.questionId, archived.etag);

  assert.equal(lineage.availabilityEtag, '"5"');
  assert.equal(exact.summary.latestQuestionRevision.revisionNumber, 2);
  assert.equal(archived.availability, "archived");
  assert.equal(restored.availability, "available");
  assert.equal(requests[2].headers.get("if-match"), '"5"');
  assert.equal(requests[3].headers.get("if-match"), '"6"');
});

test("Question availability client rejects an ETag or exact revision identity mismatch", async () => {
  const client = createHttpApiClient({
    fetch: async (input) => {
      const path = new URL(input.toString(), "https://ple.example").pathname;
      if (path.endsWith("/revisions/2")) return noStoreJson(details(1), '"5"');
      return noStoreJson(question, '"05"');
    },
  });
  await assert.rejects(client.getQuestionLineage(question.questionId), ApiProtocolError);
  await assert.rejects(
    client.getQuestionRevision({ questionId: question.questionId, revisionNumber: 2 }),
    ApiProtocolError,
  );
});
